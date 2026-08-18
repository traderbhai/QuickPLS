#!/usr/bin/env python3
"""Fail-closed APIs for QuickPLS Capability Registry V2.

The V2 registry is authoritative.  The schema-v1 competitor catalogue is a
generated compatibility projection.  Normal validation is read-only; an
explicit command can atomically materialize the generated report after the
registry passes its fail-closed checks.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import re
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any, Iterable, Mapping


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_REGISTRY_PATH = REPOSITORY_ROOT / "validation/capabilities/capability_registry_v2.json"
DEFAULT_SCHEMA_PATH = REPOSITORY_ROOT / "validation/capabilities/capability_registry_v2.schema.json"
DEFAULT_LEGACY_CATALOGUE_PATH = REPOSITORY_ROOT / "validation/quickpls_3_competitor_catalogue.json"

REGISTRY_SCHEMA_VERSION = 2
EXPECTED_ROW_COUNT = 45
EXPECTED_ACTIVE_ROW_COUNT = 43
EXPECTED_COVERAGE_COUNTS = {
    "full": 0,
    "partial": 32,
    "absent": 11,
    "intentionally_excluded": 2,
}
EXPECTED_COVERAGE_STATES = tuple(EXPECTED_COVERAGE_COUNTS)
EXPECTED_EVIDENCE_STATES = (
    "absent",
    "engine_only",
    "archive_qualified",
    "native_qualified",
    "release_qualified",
)
EXPECTED_PRODUCT_AREAS = (
    "diagram",
    "data",
    "settings",
    "calculation",
    "results",
    "reporting",
)
EXPECTED_SURFACES = ("standard", "labs", "legacy", "internal")
QUALIFICATION_LINK_KEYS = frozenset(
    {"registry_schema_version", "capability_id", "cell_id", "capability_version"}
)
OPTION_CELL_KEYS = frozenset(
    {
        "capability_id",
        "cell_id",
        "capability_version",
        "coverage_state",
        "evidence_state",
        "surface",
        "supported_model_predicate",
        "supported_data_predicate",
        "settings_schema",
        "result_schema",
        "documentation_reference",
        "qualification_spec",
        "known_differences",
    }
)
EVIDENCE_MATURITY = {state: index for index, state in enumerate(EXPECTED_EVIDENCE_STATES)}
EXCLUDED_CAPABILITY_IDS = frozenset({"smartpls.blindfolding", "smartpls.gof"})
EXCLUSION_CELL_IDS = {
    "smartpls.blindfolding": frozenset({"qpls3.assessment.blindfolding_legacy"}),
    "smartpls.gof": frozenset({"qpls3.exclusion.gof"}),
}
LEGACY_STATUS_BY_EVIDENCE = {
    "absent": "absent",
    "engine_only": "engine-preview",
    "archive_qualified": "engine-preview",
    "native_qualified": "native-qualified",
    "release_qualified": "release-qualified",
}
_ID_RE = re.compile(r"^smartpls\.[a-z0-9_]+$")
_CELL_RE = re.compile(r"^qpls3\.[a-z0-9_.]+$")
_VERSION_RE = re.compile(r"^[a-z0-9][a-z0-9_.-]*$")


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key!r}")
        result[key] = value
    return result


def _reject_nonfinite(value: str) -> Any:
    raise ValueError(f"non-finite JSON constant: {value}")


def load_json(path: Path | str) -> dict[str, Any]:
    """Load strict JSON, rejecting duplicate keys and non-finite values."""

    source = Path(path)
    value = json.loads(
        source.read_text(encoding="utf-8"),
        object_pairs_hook=_reject_duplicate_keys,
        parse_constant=_reject_nonfinite,
    )
    if not isinstance(value, dict):
        raise ValueError(f"{source}: top-level JSON value must be an object")
    return value


def load_registry(path: Path | str = DEFAULT_REGISTRY_PATH) -> dict[str, Any]:
    return load_json(path)


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        allow_nan=False,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")


def canonical_sha256(value: Any) -> str:
    return hashlib.sha256(canonical_json_bytes(value)).hexdigest()


def _nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _string_list(value: Any, *, allow_empty: bool = False) -> bool:
    return (
        isinstance(value, list)
        and (allow_empty or bool(value))
        and all(_nonempty_string(item) for item in value)
        and len(value) == len(set(value))
    )


def _contained_path(repository_root: Path, reference: str) -> Path | None:
    if not _nonempty_string(reference) or "://" in reference:
        return None
    candidate = (repository_root / reference).resolve()
    try:
        candidate.relative_to(repository_root.resolve())
    except ValueError:
        return None
    return candidate


def qualification_link_identity(link: Mapping[str, Any]) -> tuple[int, str, str, str]:
    """Return the deterministic identity of an exact four-field V2 link."""

    if not isinstance(link, Mapping) or set(link) != QUALIFICATION_LINK_KEYS:
        raise ValueError(
            "qualification link must contain exactly registry_schema_version, "
            "capability_id, cell_id, capability_version"
        )
    version = link.get("registry_schema_version")
    capability_id = link.get("capability_id")
    cell_id = link.get("cell_id")
    capability_version = link.get("capability_version")
    if version != REGISTRY_SCHEMA_VERSION:
        raise ValueError("qualification link registry_schema_version must equal 2")
    if not isinstance(capability_id, str) or not _ID_RE.fullmatch(capability_id):
        raise ValueError("qualification link capability_id is invalid")
    if not isinstance(cell_id, str) or not _CELL_RE.fullmatch(cell_id):
        raise ValueError("qualification link cell_id is invalid")
    if not isinstance(capability_version, str) or not _VERSION_RE.fullmatch(capability_version):
        raise ValueError("qualification link capability_version is invalid")
    return version, capability_id, cell_id, capability_version


def lookup_capability(
    document: Mapping[str, Any], capability_id: str
) -> Mapping[str, Any] | None:
    for capability in document.get("capabilities", []):
        if isinstance(capability, Mapping) and capability.get("capability_id") == capability_id:
            return capability
    return None


def option_cell_identity(cell: Mapping[str, Any]) -> tuple[str, str, str]:
    """Return a cell identity without consulting any row-level projection."""

    capability_id = cell.get("capability_id")
    cell_id = cell.get("cell_id")
    capability_version = cell.get("capability_version")
    if not isinstance(capability_id, str) or not _ID_RE.fullmatch(capability_id):
        raise ValueError("option cell capability_id is invalid")
    if not isinstance(cell_id, str) or not _CELL_RE.fullmatch(cell_id):
        raise ValueError("option cell cell_id is invalid")
    if not isinstance(capability_version, str) or not _VERSION_RE.fullmatch(capability_version):
        raise ValueError("option cell capability_version is invalid")
    return capability_id, cell_id, capability_version


def lookup_option_cell(
    document: Mapping[str, Any], capability_id: str, cell_id: str
) -> Mapping[str, Any] | None:
    """Resolve an exact option cell within one official catalogue row."""

    row = lookup_capability(document, capability_id)
    if not isinstance(row, Mapping):
        return None
    matches = [
        cell
        for cell in row.get("option_cells", [])
        if isinstance(cell, Mapping) and cell.get("cell_id") == cell_id
    ]
    if len(matches) > 1:
        raise ValueError(f"duplicate option cell identity: {capability_id}::{cell_id}")
    return matches[0] if matches else None


def derive_row_projection(capability: Mapping[str, Any]) -> dict[str, str]:
    """Derive the conservative legacy row projection from authoritative cells."""

    cells = capability.get("option_cells")
    if not isinstance(cells, list) or not cells:
        raise ValueError("option_cells must be a non-empty list")
    coverage = [cell.get("coverage_state") for cell in cells if isinstance(cell, Mapping)]
    evidence = [cell.get("evidence_state") for cell in cells if isinstance(cell, Mapping)]
    surfaces = [cell.get("surface") for cell in cells if isinstance(cell, Mapping)]
    if len(coverage) != len(cells) or any(value not in EXPECTED_COVERAGE_STATES for value in coverage):
        raise ValueError("cannot derive row projection from invalid option-cell coverage")
    if len(evidence) != len(cells) or any(value not in EXPECTED_EVIDENCE_STATES for value in evidence):
        raise ValueError("cannot derive row projection from invalid option-cell evidence")
    if len(surfaces) != len(cells) or any(value not in EXPECTED_SURFACES for value in surfaces):
        raise ValueError("cannot derive row projection from invalid option-cell surface")

    if all(value == "intentionally_excluded" for value in coverage):
        projected_coverage = "intentionally_excluded"
    elif "intentionally_excluded" in coverage:
        raise ValueError("active and intentionally excluded option cells cannot share a row")
    elif "absent" in coverage:
        projected_coverage = "absent"
    elif "partial" in coverage:
        projected_coverage = "partial"
    else:
        projected_coverage = "full"

    projected_evidence = min(evidence, key=EVIDENCE_MATURITY.__getitem__)
    if all(value == "legacy" for value in surfaces):
        projected_surface = "legacy"
    elif all(value == "internal" for value in surfaces):
        projected_surface = "internal"
    elif all(value == "standard" for value in surfaces):
        projected_surface = "standard"
    else:
        projected_surface = "labs"
    return {
        "coverage_state": projected_coverage,
        "evidence_state": projected_evidence,
        "surface": projected_surface,
    }


def build_qualification_link_index(
    document: Mapping[str, Any],
) -> dict[tuple[int, str, str, str], Mapping[str, Any]]:
    """Index authoritative option-cell links, returning their owner rows."""

    index: dict[tuple[int, str, str, str], Mapping[str, Any]] = {}
    for capability in document.get("capabilities", []):
        if not isinstance(capability, Mapping):
            raise ValueError("capability entries must be objects")
        compatibility_links = capability.get("qualification_links")
        if not isinstance(compatibility_links, list):
            raise ValueError(f"{capability.get('capability_id')}: qualification_links must be a list")
        compatibility_identities = [qualification_link_identity(link) for link in compatibility_links]
        if len(compatibility_identities) != len(set(compatibility_identities)):
            raise ValueError(
                f"{capability.get('capability_id')}: duplicate compatibility qualification link identity"
            )
        cells = capability.get("option_cells")
        if not isinstance(cells, list) or not cells:
            raise ValueError(f"{capability.get('capability_id')}: option_cells must be a non-empty list")
        for cell in cells:
            if not isinstance(cell, Mapping):
                raise ValueError(f"{capability.get('capability_id')}: option cell must be an object")
            spec = cell.get("qualification_spec")
            if not isinstance(spec, Mapping) or not isinstance(spec.get("links"), list) or len(spec["links"]) != 1:
                raise ValueError(
                    f"{capability.get('capability_id')}: option-cell qualification_spec must contain exactly one link"
                )
            link = spec["links"][0]
            identity = qualification_link_identity(link)
            cell_identity = option_cell_identity(cell)
            if identity[1:] != cell_identity:
                raise ValueError(
                    f"{capability.get('capability_id')}: option-cell identity differs from qualification link"
                )
            if identity in index:
                raise ValueError(f"duplicate qualification link identity: {identity!r}")
            index[identity] = capability
        authoritative = {
            identity for identity, owner in index.items() if owner is capability
        }
        if authoritative != set(compatibility_identities):
            raise ValueError(
                f"{capability.get('capability_id')}: compatibility qualification_links drift from option cells"
            )
    return index


def lookup_qualification_link(
    document: Mapping[str, Any], link: Mapping[str, Any]
) -> Mapping[str, Any] | None:
    """Resolve an exact four-field link to its capability row."""

    return build_qualification_link_index(document).get(qualification_link_identity(link))


def cross_validate_qualification_links(
    document: Mapping[str, Any],
    links: Iterable[Mapping[str, Any]] | None = None,
) -> dict[str, Any]:
    """Cross-check contained or external links against the registry."""

    errors: list[str] = []
    try:
        index = build_qualification_link_index(document)
    except ValueError as exc:
        index = {}
        errors.append(str(exc))

    for capability in document.get("capabilities", []):
        if not isinstance(capability, Mapping):
            continue
        capability_id = capability.get("capability_id")
        linked_cells: set[str] = set()
        row_link_identities: set[tuple[int, str, str, str]] = set()
        for link in capability.get("qualification_links", []):
            try:
                identity = qualification_link_identity(link)
            except ValueError as exc:
                errors.append(f"{capability_id}: {exc}")
                continue
            if identity[1] != capability_id:
                errors.append(f"{capability_id}: qualification link capability_id mismatch")
            linked_cells.add(identity[2])
            row_link_identities.add(identity)
        option_link_identities: set[tuple[int, str, str, str]] = set()
        for cell in capability.get("option_cells", []):
            if not isinstance(cell, Mapping):
                errors.append(f"{capability_id}: option cell must be an object")
                continue
            try:
                cell_identity = option_cell_identity(cell)
                spec = cell.get("qualification_spec", {})
                spec_links = spec.get("links", []) if isinstance(spec, Mapping) else []
                if not isinstance(spec_links, list) or len(spec_links) != 1:
                    raise ValueError("option-cell qualification_spec must contain exactly one link")
                link_identity = qualification_link_identity(spec_links[0])
                if link_identity[1:] != cell_identity:
                    raise ValueError("option-cell identity differs from qualification link")
                option_link_identities.add(link_identity)
            except ValueError as exc:
                errors.append(f"{capability_id}: {exc}")
        if option_link_identities != row_link_identities:
            errors.append(
                f"{capability_id}: compatibility qualification_links drift from authoritative option cells"
            )
        legacy = capability.get("legacy_row", {})
        legacy_cells = set(legacy.get("quickpls_capability_ids", []))
        expected_cells = (
            set(EXCLUSION_CELL_IDS[str(capability_id)])
            if capability_id in EXCLUDED_CAPABILITY_IDS
            else legacy_cells
        )
        if linked_cells != expected_cells:
            errors.append(
                f"{capability_id}: linked cells {sorted(linked_cells)!r} "
                f"do not match expected {sorted(expected_cells)!r}"
            )

    external_count = 0
    if links is not None:
        seen: set[tuple[int, str, str, str]] = set()
        for external_count, link in enumerate(links, start=1):
            try:
                identity = qualification_link_identity(link)
            except ValueError as exc:
                errors.append(f"external link {external_count}: {exc}")
                continue
            if identity in seen:
                errors.append(f"external link {external_count}: duplicate {identity!r}")
            seen.add(identity)
            if identity not in index:
                errors.append(f"external link {external_count}: identity not found {identity!r}")

    return {
        "passed": not errors,
        "error_count": len(errors),
        "errors": errors,
        "registry_link_count": len(index),
        "external_link_count": external_count,
    }


def resolve_customer_visibility(
    document: Mapping[str, Any],
    capability: str | Mapping[str, Any],
    cell_id: str | None = None,
) -> dict[str, Any]:
    """Apply Standard/Labs routing to one cell, or a conservative row projection."""

    row = lookup_capability(document, capability) if isinstance(capability, str) else capability
    if not isinstance(row, Mapping):
        raise KeyError(f"capability not found: {capability!r}")
    if cell_id is None:
        state: Mapping[str, Any] = derive_row_projection(row)
    else:
        matches = [
            cell
            for cell in row.get("option_cells", [])
            if isinstance(cell, Mapping) and cell.get("cell_id") == cell_id
        ]
        if len(matches) != 1:
            raise KeyError(f"option cell not found: {row.get('capability_id')}::{cell_id}")
        state = matches[0]
    policy = document.get("customer_visibility_policy", {})
    standard = policy.get("standard_match", {})
    scoped_standard = policy.get("scoped_standard_match", {})
    is_standard = (
        (
            state.get("surface") == standard.get("surface")
            and state.get("coverage_state") == standard.get("coverage_state")
            and state.get("evidence_state") == standard.get("evidence_state")
        )
        or (
            state.get("surface") == scoped_standard.get("surface")
            and state.get("coverage_state") == scoped_standard.get("coverage_state")
            and state.get("evidence_state") == scoped_standard.get("evidence_state")
            and bool(scoped_standard.get("scope_statement_required"))
            and isinstance(row.get("scope_statement"), str)
            and bool(row.get("scope_statement", "").strip())
        )
    )
    coverage = state.get("coverage_state")
    evidence = state.get("evidence_state")
    declared_surface = state.get("surface")
    if is_standard:
        channel = "standard"
    elif declared_surface == "standard":
        channel = "hidden"
    else:
        channel = declared_surface
    executable_evidence = evidence in {
        "engine_only",
        "archive_qualified",
        "native_qualified",
        "release_qualified",
    }
    available = (
        coverage not in {"absent", "intentionally_excluded"}
        and executable_evidence
        and channel not in {"legacy", "internal", "hidden"}
    )
    return {
        "capability_id": row.get("capability_id"),
        "cell_id": cell_id,
        "channel": channel,
        "requires_opt_in": (
            bool(policy.get("labs_requires_opt_in")) if channel == "labs" else False
        ),
        "available": available,
        "evidence_label_visibility": policy.get("evidence_labels"),
    }


def derive_legacy_status(capability: Mapping[str, Any]) -> str:
    """Derive a schema-v1 status from authoritative option-cell state."""

    projection = derive_row_projection(capability)
    coverage = projection["coverage_state"]
    evidence = projection["evidence_state"]
    if coverage == "intentionally_excluded":
        return "deferred"
    if coverage == "absent":
        return "absent"
    try:
        return LEGACY_STATUS_BY_EVIDENCE[str(evidence)]
    except KeyError as exc:
        raise ValueError(
            f"cannot derive legacy status from coverage={coverage!r}, "
            f"evidence={evidence!r}"
        ) from exc


def generate_legacy_catalogue(document: Mapping[str, Any]) -> dict[str, Any]:
    """Generate the complete schema-v1 compatibility projection in memory."""

    contract = document.get("legacy_catalogue_contract")
    if not isinstance(contract, Mapping) or not isinstance(contract.get("header"), Mapping):
        raise ValueError("legacy_catalogue_contract.header must be an object")
    capabilities = document.get("capabilities")
    if not isinstance(capabilities, list):
        raise ValueError("capabilities must be a list")
    projected = copy.deepcopy(dict(contract["header"]))
    projected["methods"] = []
    for capability in capabilities:
        if not isinstance(capability, Mapping) or not isinstance(
            capability.get("legacy_row"), Mapping
        ):
            raise ValueError("each capability must contain a legacy_row object")
        legacy_row = copy.deepcopy(dict(capability["legacy_row"]))
        legacy_row["status"] = derive_legacy_status(capability)
        projected["methods"].append(legacy_row)
    return projected


def write_generated_legacy_catalogue(
    document: Mapping[str, Any], target: Path | str
) -> dict[str, Any]:
    """Atomically materialize the schema-v1 compatibility report.

    Callers must validate the authoritative registry before invoking this
    function.  The command-line entry point enforces that ordering and never
    replaces an existing report when validation fails.
    """

    destination = Path(target)
    destination.parent.mkdir(parents=True, exist_ok=True)
    projected = generate_legacy_catalogue(document)
    payload = (
        json.dumps(projected, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    ).encode("utf-8")
    temporary_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb",
            prefix=f".{destination.name}.",
            suffix=".tmp",
            dir=destination.parent,
            delete=False,
        ) as handle:
            temporary_path = Path(handle.name)
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary_path, destination)
        temporary_path = None
    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)

    materialized = load_json(destination)
    if materialized != projected:
        raise OSError("generated legacy catalogue failed read-back verification")
    return {
        "passed": True,
        "path": str(destination),
        "row_count": len(projected.get("methods", [])),
        "sha256": canonical_sha256(projected),
    }


def check_legacy_catalogue(
    document: Mapping[str, Any], legacy_document: Mapping[str, Any]
) -> dict[str, Any]:
    """Compare the deterministic projection with a supplied legacy catalogue."""

    generated = generate_legacy_catalogue(document)
    errors: list[str] = []
    expected_header = {key: value for key, value in generated.items() if key != "methods"}
    actual_header = {key: value for key, value in legacy_document.items() if key != "methods"}
    if canonical_json_bytes(expected_header) != canonical_json_bytes(actual_header):
        errors.append("legacy catalogue metadata/header differs from frozen projection")
    expected_rows = generated["methods"]
    actual_rows = legacy_document.get("methods")
    if not isinstance(actual_rows, list):
        errors.append("legacy catalogue methods must be a list")
        actual_rows = []
    if len(expected_rows) != len(actual_rows):
        errors.append(
            f"legacy row count differs: expected {len(expected_rows)}, found {len(actual_rows)}"
        )
    for position, (expected, actual) in enumerate(zip(expected_rows, actual_rows), start=1):
        if canonical_json_bytes(expected) != canonical_json_bytes(actual):
            errors.append(f"legacy row {position} differs ({expected.get('id', 'unknown')})")
    return {
        "passed": not errors,
        "error_count": len(errors),
        "errors": errors,
        "generated_sha256": canonical_sha256(generated),
        "actual_sha256": canonical_sha256(dict(legacy_document)),
        "row_count": len(expected_rows),
    }


def validate_schema_contract(schema: Mapping[str, Any]) -> dict[str, Any]:
    """Check frozen schema constants without requiring jsonschema."""

    errors: list[str] = []
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        errors.append("$schema must select JSON Schema draft 2020-12")
    properties = schema.get("properties", {})
    if properties.get("registry_schema_version", {}).get("const") != 2:
        errors.append("schema must freeze registry_schema_version at 2")
    rows = properties.get("capabilities", {})
    if rows.get("minItems") != 45 or rows.get("maxItems") != 45:
        errors.append("schema must freeze capabilities at exactly 45 rows")
    link = schema.get("$defs", {}).get("qualificationLink", {})
    if set(link.get("required", [])) != QUALIFICATION_LINK_KEYS:
        errors.append("schema qualificationLink required shape is not exact")
    if link.get("additionalProperties") is not False:
        errors.append("schema qualificationLink must reject additional properties")
    capability = schema.get("$defs", {}).get("capability", {})
    required = set(capability.get("required", []))
    explicit_v2_fields = {
        "surface",
        "supported_model_predicate",
        "supported_data_predicate",
        "settings_schema",
        "result_schema",
        "documentation_reference",
        "qualification_spec",
        "known_differences",
        "option_cells",
    }
    if not explicit_v2_fields.issubset(required):
        errors.append("schema capability omits explicit V2 contract fields")
    if capability.get("properties", {}).get("surface", {}).get("enum") != list(
        EXPECTED_SURFACES
    ):
        errors.append("schema surface enum must be standard/labs/legacy/internal")
    option_cell = schema.get("$defs", {}).get("optionCell", {})
    if set(option_cell.get("required", [])) != OPTION_CELL_KEYS:
        errors.append("schema optionCell required shape is not exact")
    if option_cell.get("additionalProperties") is not False:
        errors.append("schema optionCell must reject additional properties")
    return {"passed": not errors, "error_count": len(errors), "errors": errors}


def _validate_manifest_references(
    capability: Mapping[str, Any], repository_root: Path, errors: list[str]
) -> None:
    capability_id = capability.get("capability_id")
    for cell in capability.get("option_cells", []):
        if not isinstance(cell, Mapping):
            continue
        cell_id = cell.get("cell_id")
        expected_version = cell.get("capability_version")
        spec = cell.get("qualification_spec", {})
        references = spec.get("references", []) if isinstance(spec, Mapping) else []
        found = False
        for reference in references:
            path = _contained_path(repository_root, reference)
            if path is None:
                errors.append(f"{capability_id}::{cell_id}: invalid qualification reference {reference!r}")
                continue
            if not path.is_file():
                errors.append(f"{capability_id}::{cell_id}: missing qualification reference {reference!r}")
                continue
            if not reference.endswith(".manifest.json"):
                continue
            try:
                manifest = load_json(path)
            except (OSError, ValueError, json.JSONDecodeError) as exc:
                errors.append(f"{capability_id}::{cell_id}: cannot load {reference}: {exc}")
                continue
            feature = manifest.get("feature", {})
            if feature.get("id") != cell_id:
                errors.append(
                    f"{capability_id}::{cell_id}: {reference} feature.id "
                    f"{feature.get('id')!r} does not match"
                )
                continue
            found = True
            if feature.get("method_version") != expected_version:
                errors.append(
                    f"{capability_id}::{cell_id}: {reference} method_version "
                    f"{feature.get('method_version')!r} does not match {expected_version!r}"
                )
            if manifest.get("contract_kind") == "capability_cell_contract":
                for state_field in ("coverage_state", "evidence_state", "surface"):
                    if manifest.get(state_field) != cell.get(state_field):
                        errors.append(
                            f"{capability_id}::{cell_id}: {reference} {state_field} "
                            "does not match the authoritative option cell"
                        )
        if not found and cell_id != "qpls3.exclusion.gof":
            errors.append(f"{capability_id}::{cell_id}: no matching manifest reference")


def cross_validate_manifest_evidence(
    document: Mapping[str, Any], repository_root: Path | str = REPOSITORY_ROOT
) -> dict[str, Any]:
    """Fail closed when a registry cell outruns its live method manifest.

    Historical evidence reports are intentionally not interpreted here. The
    method-promotion validator re-derives each referenced manifest's state from
    the current source-bound artifacts on disk; the registry may claim no more
    maturity than that result. Capability-cell contracts (for example the
    post-hoc technical sample-size cell) retain their exact-state validation in
    :func:`_validate_manifest_references`.
    """

    from method_promotion_manifest import (  # imported lazily to avoid CLI coupling
        STATE_ORDER as MANIFEST_STATE_ORDER,
        validate_manifest,
    )

    root = Path(repository_root).resolve()
    maturity = {state: index for index, state in enumerate(MANIFEST_STATE_ORDER)}
    errors: list[str] = []
    manifest_cache: dict[Path, dict[str, Any]] = {}
    mappings: list[dict[str, Any]] = []
    derived_counts: Counter[str] = Counter()

    for capability in document.get("capabilities", []):
        if not isinstance(capability, Mapping):
            continue
        capability_id = capability.get("capability_id")
        for cell in capability.get("option_cells", []):
            if not isinstance(cell, Mapping):
                continue
            cell_id = cell.get("cell_id")
            spec = cell.get("qualification_spec", {})
            references = spec.get("references", []) if isinstance(spec, Mapping) else []
            results: list[dict[str, Any]] = []
            for reference in references:
                if not (
                    isinstance(reference, str)
                    and reference.startswith("validation/methods/")
                    and reference.endswith(".manifest.json")
                ):
                    continue
                path = _contained_path(root, reference)
                if path is None or not path.is_file():
                    continue
                if path not in manifest_cache:
                    manifest_cache[path] = validate_manifest(path, root)
                results.append(manifest_cache[path])
            if not results:
                continue

            derived_state = min(
                (str(result.get("derived_state", "absent")) for result in results),
                key=maturity.__getitem__,
            )
            registry_state = str(cell.get("evidence_state"))
            derived_counts[derived_state] += 1
            mapping = {
                "capability_id": capability_id,
                "cell_id": cell_id,
                "registry_evidence_state": registry_state,
                "derived_manifest_state": derived_state,
                "manifest_paths": sorted(
                    Path(result["path"]).resolve().relative_to(root).as_posix()
                    for result in results
                ),
            }
            mappings.append(mapping)
            if registry_state not in maturity:
                continue
            if maturity[registry_state] > maturity[derived_state]:
                errors.append(
                    f"{capability_id}::{cell_id}: registry evidence {registry_state} "
                    f"exceeds current manifest-derived evidence {derived_state}"
                )

    manifest_results = []
    for path, result in sorted(manifest_cache.items(), key=lambda item: item[0].as_posix()):
        relative = path.relative_to(root).as_posix()
        manifest_results.append(
            {
                "path": relative,
                "feature_id": result.get("feature_id"),
                "declared_state": result.get("declared_state"),
                "derived_state": result.get("derived_state", "absent"),
                "passed": bool(result.get("passed")),
                "error_count": len(result.get("errors", [])),
            }
        )
        if not result.get("passed"):
            errors.append(
                f"{relative}: referenced method manifest is invalid "
                f"({len(result.get('errors', []))} error(s)); run the method-manifest gate"
            )

    return {
        "passed": not errors,
        "error_count": len(errors),
        "errors": errors,
        "mapped_cell_count": len(mappings),
        "unique_manifest_count": len(manifest_results),
        "derived_cell_evidence_counts": {
            state: derived_counts.get(state, 0) for state in MANIFEST_STATE_ORDER
        },
        "mappings": mappings,
        "manifests": manifest_results,
    }


def _validate_option_cells(
    capability: Mapping[str, Any], errors: list[str]
) -> list[Mapping[str, Any]]:
    """Validate every independently governed cell and its exact link."""

    capability_id = capability.get("capability_id")
    lifecycle = capability.get("official_lifecycle")
    values = capability.get("option_cells")
    if not isinstance(values, list) or not values:
        errors.append(f"{capability_id}: option_cells must be a non-empty list")
        return []
    cells: list[Mapping[str, Any]] = []
    identities: list[tuple[str, str, str]] = []
    for index, value in enumerate(values):
        prefix = f"{capability_id}: option_cells[{index}]"
        if not isinstance(value, Mapping):
            errors.append(f"{prefix} must be an object")
            continue
        cells.append(value)
        if set(value) != OPTION_CELL_KEYS:
            errors.append(f"{prefix} must contain the exact option-cell fields")
        try:
            identity = option_cell_identity(value)
        except ValueError as exc:
            errors.append(f"{prefix}: {exc}")
            continue
        identities.append(identity)
        if identity[0] != capability_id:
            errors.append(f"{prefix}.capability_id must equal owner {capability_id}")

        coverage = value.get("coverage_state")
        evidence = value.get("evidence_state")
        surface = value.get("surface")
        if coverage not in EXPECTED_COVERAGE_STATES:
            errors.append(f"{prefix}: invalid coverage_state")
        if evidence not in EXPECTED_EVIDENCE_STATES:
            errors.append(f"{prefix}: invalid evidence_state")
        if surface not in EXPECTED_SURFACES:
            errors.append(f"{prefix}: invalid surface")
        if surface == "standard" and not (
            coverage in {"full", "partial"} and evidence == "release_qualified"
        ):
            errors.append(
                f"{prefix}: Standard requires documented coverage plus release-qualified evidence"
            )
        if coverage == "absent" and evidence != "absent":
            errors.append(f"{prefix}: absent coverage requires absent evidence")
        if coverage == "intentionally_excluded" and not (
            evidence == "absent" and surface == "legacy" and lifecycle == "legacy"
        ):
            errors.append(f"{prefix}: intentional exclusion requires a legacy owner and surface")
        if lifecycle == "legacy" and coverage != "intentionally_excluded":
            errors.append(f"{prefix}: legacy catalogue rows must remain intentionally excluded")
        if lifecycle == "active" and coverage == "intentionally_excluded":
            errors.append(f"{prefix}: active catalogue rows cannot contain intentional exclusions")

        for field in ("supported_model_predicate", "supported_data_predicate"):
            predicates = value.get(field)
            if (
                not isinstance(predicates, Mapping)
                or set(predicates) != {"official", "quickpls"}
                or not _string_list(predicates.get("official"))
                or not _string_list(predicates.get("quickpls"))
            ):
                errors.append(f"{prefix}: invalid {field}")
        for field in ("settings_schema", "result_schema"):
            reference_schema = value.get(field)
            if (
                not isinstance(reference_schema, Mapping)
                or set(reference_schema) != {"references"}
                or not _string_list(reference_schema.get("references"))
            ):
                errors.append(f"{prefix}: invalid {field}")
        if value.get("documentation_reference") != capability.get("official_url"):
            errors.append(f"{prefix}: documentation_reference must equal official_url")
        if not _string_list(value.get("known_differences")):
            errors.append(f"{prefix}: known_differences must be non-empty and unique")

        spec = value.get("qualification_spec")
        if not isinstance(spec, Mapping) or set(spec) != {"references", "links"}:
            errors.append(f"{prefix}: invalid qualification_spec")
            continue
        if not _string_list(spec.get("references")):
            errors.append(f"{prefix}: qualification_spec.references must be non-empty and unique")
        links = spec.get("links")
        if not isinstance(links, list) or len(links) != 1:
            errors.append(f"{prefix}: qualification_spec.links must contain exactly one link")
            continue
        try:
            link_identity = qualification_link_identity(links[0])
        except ValueError as exc:
            errors.append(f"{prefix}: {exc}")
        else:
            if link_identity[1:] != identity:
                errors.append(f"{prefix}: qualification link must exactly mirror the option-cell identity")
    if len(identities) != len(set(identities)):
        errors.append(f"{capability_id}: duplicate authoritative option-cell identity")
    return cells


def validate_registry_document(
    document: Mapping[str, Any],
    *,
    repository_root: Path | str = REPOSITORY_ROOT,
    check_references: bool = True,
    schema: Mapping[str, Any] | None = None,
) -> dict[str, Any]:
    """Validate frozen V2 invariants and return a deterministic report."""

    root = Path(repository_root).resolve()
    errors: list[str] = []
    if document.get("registry_schema_version") != 2:
        errors.append("registry_schema_version must equal 2")
    if document.get("registry_id") != "quickpls.capability_registry.v2":
        errors.append("registry_id is invalid")

    state = document.get("state_contract", {})
    if tuple(state.get("coverage_states", [])) != EXPECTED_COVERAGE_STATES:
        errors.append("coverage state order is not frozen")
    if tuple(state.get("evidence_states", [])) != EXPECTED_EVIDENCE_STATES:
        errors.append("evidence state order is not frozen")
    if state.get("baseline_counts") != EXPECTED_COVERAGE_COUNTS:
        errors.append("baseline_counts must be full=0, partial=32, absent=11, excluded=2")
    if state.get("active_row_count") != 43:
        errors.append("state_contract.active_row_count must equal 43")

    policy = document.get("customer_visibility_policy", {})
    if policy.get("policy_version") != 2:
        errors.append("customer visibility policy_version must equal 2")
    if policy.get("standard_match") != {
        "surface": "standard",
        "coverage_state": "full",
        "evidence_state": "release_qualified",
    }:
        errors.append("Standard must require surface=standard + full + release_qualified")
    if policy.get("scoped_standard_match") != {
        "surface": "standard",
        "coverage_state": "partial",
        "evidence_state": "release_qualified",
        "scope_statement_required": True,
    }:
        errors.append(
            "Scoped Standard must require surface=standard + partial + release_qualified + scope statement"
        )
    if policy.get("otherwise_channel") != "labs" or policy.get("labs_requires_opt_in") is not True:
        errors.append("failed Standard candidates must route to opt-in Labs")
    if policy.get("evidence_labels") != "internal_only":
        errors.append("evidence labels must remain internal_only")
    if policy.get("non_customer_surfaces") != ["legacy", "internal"]:
        errors.append("legacy/internal must be the non-customer surfaces")

    surface_contract = document.get("surface_contract", {})
    expected_surface_counts = surface_contract.get("baseline_counts")
    if surface_contract.get("allowed") != list(EXPECTED_SURFACES):
        errors.append("surface_contract.allowed is not the frozen V2 order")
    if not isinstance(expected_surface_counts, Mapping) or set(expected_surface_counts) != set(
        EXPECTED_SURFACES
    ) or any(
        not isinstance(expected_surface_counts.get(surface), int)
        or expected_surface_counts.get(surface) < 0
        for surface in EXPECTED_SURFACES
    ):
        errors.append("surface_contract.baseline_counts must contain nonnegative counts for every surface")
        expected_surface_counts = {}
    product_area_contract = document.get("product_area_contract", {})
    if product_area_contract.get("allowed") != list(EXPECTED_PRODUCT_AREAS):
        errors.append("product_area_contract.allowed is not the frozen order")

    capabilities = document.get("capabilities")
    if not isinstance(capabilities, list):
        errors.append("capabilities must be a list")
        capabilities = []
    if len(capabilities) != EXPECTED_ROW_COUNT:
        errors.append("capabilities must contain exactly 45 rows")

    positions: list[int] = []
    ids: list[str] = []
    catalogue_identities: list[tuple[Any, Any, Any]] = []
    coverage_counts: Counter[str] = Counter()
    evidence_counts: Counter[str] = Counter()
    surface_counts: Counter[str] = Counter()
    option_cell_coverage_counts: Counter[str] = Counter()
    option_cell_evidence_counts: Counter[str] = Counter()
    option_cell_surface_counts: Counter[str] = Counter()
    option_cell_count = 0
    active_count = 0
    for position, capability in enumerate(capabilities, start=1):
        if not isinstance(capability, Mapping):
            errors.append(f"row {position} must be an object")
            continue
        capability_id = capability.get("capability_id")
        ids.append(str(capability_id))
        positions.append(capability.get("catalogue_position"))
        catalogue_identities.append(
            (capability_id, capability.get("official_family"), capability.get("official_method"))
        )
        if capability.get("catalogue_position") != position:
            errors.append(f"{capability_id}: catalogue_position must equal {position}")

        expected_lifecycle = "legacy" if capability_id in EXCLUDED_CAPABILITY_IDS else "active"
        if capability.get("official_lifecycle") != expected_lifecycle:
            errors.append(f"{capability_id}: official_lifecycle must be {expected_lifecycle}")
        if capability.get("official_lifecycle") == "active":
            active_count += 1

        cells = _validate_option_cells(capability, errors)
        option_cell_count += len(cells)
        for cell in cells:
            option_cell_coverage_counts[str(cell.get("coverage_state"))] += 1
            option_cell_evidence_counts[str(cell.get("evidence_state"))] += 1
            option_cell_surface_counts[str(cell.get("surface"))] += 1
        try:
            projection = derive_row_projection(capability)
        except ValueError as exc:
            projection = {
                "coverage_state": capability.get("coverage_state"),
                "evidence_state": capability.get("evidence_state"),
                "surface": capability.get("surface"),
            }
            errors.append(f"{capability_id}: cannot derive compatibility row projection: {exc}")
        for field in ("coverage_state", "evidence_state", "surface"):
            if capability.get(field) != projection.get(field):
                errors.append(
                    f"{capability_id}: row {field} must equal the derived option-cell projection "
                    f"{projection.get(field)!r}"
                )

        coverage = projection.get("coverage_state")
        evidence = projection.get("evidence_state")
        coverage_counts[str(coverage)] += 1
        evidence_counts[str(evidence)] += 1
        if coverage not in EXPECTED_COVERAGE_STATES:
            errors.append(f"{capability_id}: invalid coverage_state")
        if evidence not in EXPECTED_EVIDENCE_STATES:
            errors.append(f"{capability_id}: invalid evidence_state")
        if coverage == "absent" and evidence != "absent":
            errors.append(f"{capability_id}: absent evidence must be absent")
        if coverage == "intentionally_excluded" and evidence != "absent":
            errors.append(f"{capability_id}: excluded evidence must be absent")

        surface = projection.get("surface")
        surface_counts[str(surface)] += 1
        if capability_id in EXCLUDED_CAPABILITY_IDS and surface != "legacy":
            errors.append(f"{capability_id}: excluded capability surface must be legacy")
        if capability_id not in EXCLUDED_CAPABILITY_IDS and surface in {"legacy", "internal"}:
            errors.append(f"{capability_id}: active capability cannot use {surface} surface")

        official_url = capability.get("official_url")
        if not isinstance(official_url, str) or not official_url.startswith(
            "https://smartpls.com/"
        ):
            errors.append(f"{capability_id}: official_url is not an official SmartPLS URL")
        product_areas = capability.get("product_areas")
        if not _string_list(product_areas) or any(
            item not in EXPECTED_PRODUCT_AREAS for item in product_areas
        ):
            errors.append(f"{capability_id}: invalid product_areas list")
        for field in ("model_predicates", "data_predicates"):
            predicates = capability.get(field)
            if (
                not isinstance(predicates, Mapping)
                or set(predicates) != {"official", "quickpls"}
                or not _string_list(predicates.get("official"))
                or not _string_list(predicates.get("quickpls"))
            ):
                errors.append(f"{capability_id}: invalid {field}")
        for field in (
            "settings_references",
            "result_references",
            "qualification_references",
            "known_differences",
        ):
            if not _string_list(capability.get(field)):
                errors.append(f"{capability_id}: invalid {field}")
        if capability.get("supported_model_predicate") != capability.get(
            "model_predicates"
        ):
            errors.append(f"{capability_id}: supported_model_predicate alias drift")
        if capability.get("supported_data_predicate") != capability.get(
            "data_predicates"
        ):
            errors.append(f"{capability_id}: supported_data_predicate alias drift")
        if capability.get("settings_schema") != {
            "references": capability.get("settings_references")
        }:
            errors.append(f"{capability_id}: settings_schema alias drift")
        if capability.get("result_schema") != {
            "references": capability.get("result_references")
        }:
            errors.append(f"{capability_id}: result_schema alias drift")
        if capability.get("documentation_reference") != official_url:
            errors.append(f"{capability_id}: documentation_reference alias drift")
        if capability.get("qualification_spec") != {
            "references": capability.get("qualification_references"),
            "links": capability.get("qualification_links"),
        }:
            errors.append(f"{capability_id}: qualification_spec alias drift")
        authoritative_references: list[str] = []
        for cell in cells:
            spec = cell.get("qualification_spec", {})
            if isinstance(spec, Mapping):
                for reference in spec.get("references", []):
                    if reference not in authoritative_references:
                        authoritative_references.append(reference)
            cell_settings = cell.get("settings_schema")
            cell_results = cell.get("result_schema")
            if not isinstance(cell_settings, Mapping) or official_url not in cell_settings.get("references", []):
                errors.append(f"{capability_id}::{cell.get('cell_id')}: settings schema omits official_url")
            if not isinstance(cell_results, Mapping) or official_url not in cell_results.get("references", []):
                errors.append(f"{capability_id}::{cell.get('cell_id')}: result schema omits official_url")
        if capability.get("qualification_references") != authoritative_references:
            errors.append(
                f"{capability_id}: compatibility qualification_references drift from option cells"
            )
        if official_url not in capability.get("settings_references", []):
            errors.append(f"{capability_id}: settings references omit official_url")
        if official_url not in capability.get("result_references", []):
            errors.append(f"{capability_id}: result references omit official_url")

        legacy = capability.get("legacy_row")
        if not isinstance(legacy, Mapping):
            errors.append(f"{capability_id}: legacy_row must be an object")
        else:
            checks = {
                "id": capability_id,
                "catalogue_position": position,
                "official_family": capability.get("official_family"),
                "official_method": capability.get("official_method"),
                "quickpls_scope": capability.get("scope_statement"),
            }
            for key, expected in checks.items():
                if legacy.get(key) != expected:
                    errors.append(f"{capability_id}: legacy_row.{key} mismatch")
            try:
                derived_legacy_status = derive_legacy_status(capability)
            except ValueError as exc:
                errors.append(f"{capability_id}: {exc}")
            else:
                if legacy.get("status") != derived_legacy_status:
                    errors.append(
                        f"{capability_id}: legacy_row.status must equal derived "
                        f"{derived_legacy_status!r}"
                    )

        if check_references:
            _validate_manifest_references(capability, root, errors)

    if positions != list(range(1, EXPECTED_ROW_COUNT + 1)):
        errors.append("catalogue positions must be contiguous 1..45")
    if len(ids) != len(set(ids)):
        errors.append("capability_id values must be unique")
    if len(catalogue_identities) != len(set(catalogue_identities)):
        duplicates = [
            item for item, count in Counter(catalogue_identities).items() if count > 1
        ]
        errors.append(f"duplicate full catalogue identities are forbidden: {duplicates!r}")
    actual_counts = {state: coverage_counts.get(state, 0) for state in EXPECTED_COVERAGE_STATES}
    if actual_counts != EXPECTED_COVERAGE_COUNTS:
        errors.append(f"coverage counts differ: {actual_counts!r}")
    if active_count != EXPECTED_ACTIVE_ROW_COUNT:
        errors.append(f"active row count must be 43, found {active_count}")
    actual_surface_counts = {
        surface: surface_counts.get(surface, 0) for surface in EXPECTED_SURFACES
    }
    if actual_surface_counts != expected_surface_counts:
        errors.append(f"surface counts differ: {actual_surface_counts!r}")

    exclusions = document.get("exclusions", [])
    exclusion_ids = {
        item.get("capability_id") for item in exclusions if isinstance(item, Mapping)
    }
    if exclusion_ids != EXCLUDED_CAPABILITY_IDS:
        errors.append("exclusions must be exactly Blindfolding and GoF")

    link_report = cross_validate_qualification_links(document)
    errors.extend(f"qualification: {item}" for item in link_report["errors"])

    manifest_evidence_report: dict[str, Any] = {
        "passed": True,
        "error_count": 0,
        "errors": [],
        "mapped_cell_count": 0,
        "unique_manifest_count": 0,
        "derived_cell_evidence_counts": {
            state: 0 for state in EXPECTED_EVIDENCE_STATES
        },
        "mappings": [],
        "manifests": [],
        "skipped": True,
    }
    if check_references:
        manifest_evidence_report = cross_validate_manifest_evidence(document, root)
        errors.extend(
            f"manifest evidence: {item}"
            for item in manifest_evidence_report["errors"]
        )

    try:
        projected = generate_legacy_catalogue(document)
    except ValueError as exc:
        projected = {}
        errors.append(f"legacy projection: {exc}")
    if projected and len(projected.get("methods", [])) != 45:
        errors.append("legacy projection must contain 45 rows")

    if schema is not None:
        schema_report = validate_schema_contract(schema)
        errors.extend(f"schema: {item}" for item in schema_report["errors"])

    return {
        "registry_schema_version": document.get("registry_schema_version"),
        "passed": not errors,
        "error_count": len(errors),
        "errors": errors,
        "capability_row_count": len(capabilities),
        "active_row_count": active_count,
        "coverage_counts": actual_counts,
        "evidence_counts": {
            state: evidence_counts.get(state, 0) for state in EXPECTED_EVIDENCE_STATES
        },
        "surface_counts": actual_surface_counts,
        "option_cell_count": option_cell_count,
        "option_cell_coverage_counts": {
            state: option_cell_coverage_counts.get(state, 0)
            for state in EXPECTED_COVERAGE_STATES
        },
        "option_cell_evidence_counts": {
            state: option_cell_evidence_counts.get(state, 0)
            for state in EXPECTED_EVIDENCE_STATES
        },
        "option_cell_surface_counts": {
            surface: option_cell_surface_counts.get(surface, 0)
            for surface in EXPECTED_SURFACES
        },
        "qualification_link_count": link_report["registry_link_count"],
        "manifest_evidence_check": manifest_evidence_report,
        "registry_sha256": canonical_sha256(dict(document)),
        "legacy_projection_sha256": canonical_sha256(projected) if projected else None,
    }


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry", type=Path, default=DEFAULT_REGISTRY_PATH)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA_PATH)
    parser.add_argument("--legacy", type=Path, default=DEFAULT_LEGACY_CATALOGUE_PATH)
    parser.add_argument("--check-legacy", action="store_true")
    parser.add_argument(
        "--write-legacy",
        nargs="?",
        type=Path,
        const=DEFAULT_LEGACY_CATALOGUE_PATH,
        help=(
            "atomically materialize the generated schema-v1 compatibility report; "
            "defaults to validation/quickpls_3_competitor_catalogue.json"
        ),
    )
    parser.add_argument(
        "--print-legacy",
        action="store_true",
        help="print the generated projection; never writes a file",
    )
    parser.add_argument("--skip-reference-check", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = _parse_args()
    try:
        registry = load_registry(args.registry)
        schema = load_json(args.schema)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(json.dumps({"passed": False, "errors": [str(exc)]}, indent=2))
        return 1
    if args.print_legacy:
        print(json.dumps(generate_legacy_catalogue(registry), ensure_ascii=False, indent=2))
        return 0
    report = validate_registry_document(
        registry,
        repository_root=REPOSITORY_ROOT,
        check_references=not args.skip_reference_check,
        schema=schema,
    )
    if args.write_legacy is not None:
        if report["passed"]:
            try:
                report["legacy_catalogue_write"] = write_generated_legacy_catalogue(
                    registry, args.write_legacy
                )
            except (OSError, ValueError, json.JSONDecodeError) as exc:
                report["legacy_catalogue_write"] = {
                    "passed": False,
                    "errors": [str(exc)],
                }
                report["passed"] = False
                report["error_count"] += 1
                report["errors"].append(f"legacy write: {exc}")
        else:
            report["legacy_catalogue_write"] = {
                "passed": False,
                "errors": ["authoritative registry validation failed; report was not written"],
            }
    if args.check_legacy:
        try:
            legacy_report = check_legacy_catalogue(registry, load_json(args.legacy))
        except (OSError, ValueError, json.JSONDecodeError) as exc:
            legacy_report = {"passed": False, "error_count": 1, "errors": [str(exc)]}
        report["legacy_catalogue_check"] = legacy_report
        if not legacy_report["passed"]:
            report["passed"] = False
            report["error_count"] += legacy_report["error_count"]
            report["errors"].extend(f"legacy: {item}" for item in legacy_report["errors"])
    print(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
