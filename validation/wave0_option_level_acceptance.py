#!/usr/bin/env python3
"""Generate and validate the Wave-0 option-level parity acceptance matrix.

The checked-in contract deliberately contains no inferred parity claims. Active
catalogue rows, exact cell identities, and primary official references are
expanded from Capability Registry V2. Missing option inventories and missing
cell-level states remain explicit ``open`` assessments. A structurally valid
baseline therefore passes the contract gate while failing the separate
finalization-ready gate.
"""

from __future__ import annotations

import argparse
import copy
import importlib.util
import json
from collections import Counter
from pathlib import Path
from typing import Any, Mapping
from urllib.parse import urlparse

from capability_registry_v2 import (
    canonical_sha256,
    load_json,
    qualification_link_identity,
)


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MATRIX_PATH = (
    REPOSITORY_ROOT / "validation/parity/wave0_option_level_acceptance_v1.json"
)
DEFAULT_SCHEMA_PATH = (
    REPOSITORY_ROOT / "validation/parity/wave0_option_level_acceptance_v1.schema.json"
)
DEFAULT_REGISTRY_PATH = (
    REPOSITORY_ROOT / "validation/capabilities/capability_registry_v2.json"
)
DEFAULT_REPORT_PATH = (
    REPOSITORY_ROOT / "validation/results/wave0_option_level_acceptance_v1.report.json"
)
FRAGMENT_ROOT = (REPOSITORY_ROOT / "validation/parity/fragments").resolve()

MATRIX_SCHEMA_VERSION = 1
MATRIX_ID = "quickpls.wave0.option_level_parity_acceptance.v1"
REGISTRY_ID = "quickpls.capability_registry.v2"
REQUIRED_DIMENSIONS = (
    "settings",
    "defaults",
    "model_shapes",
    "data_inputs",
    "preprocessing",
    "outputs",
    "charts",
    "failure_conditions",
    "workflows",
)
EXPECTED_EXCLUSIONS = frozenset({"smartpls.blindfolding", "smartpls.gof"})
ASSESSMENT_AXES = ("parity_obligation", "coverage", "evidence", "surface")
CAPTURED_STATE_FIELDS = {
    "parity_obligation": ("parity_role", frozenset({"active_parity", "beyond_parity"})),
    "coverage": ("coverage_state", frozenset({"full", "partial", "absent"})),
    "evidence": (
        "evidence_state",
        frozenset(
            {
                "absent",
                "engine_only",
                "archive_qualified",
                "native_qualified",
                "release_qualified",
            }
        ),
    ),
    "surface": ("surface", frozenset({"standard", "labs", "internal"})),
}
QUALIFICATION_LINK_KEYS = frozenset(
    {"registry_schema_version", "capability_id", "cell_id", "capability_version"}
)


def load_matrix_with_fragments(
    path: Path,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    """Load the matrix and merge immutable, disjoint official-source fragments.

    Fragment loading is intentionally outside ``build_acceptance_report`` so
    in-memory mutation tests remain pure. Paths are repository-relative,
    restricted to the parity fragment directory, and content-bound in the
    returned source bindings.
    """

    matrix = load_json(path)
    fragment_paths = matrix.get("override_fragments")
    if not isinstance(fragment_paths, list):
        raise ValueError("override_fragments must be a list")
    if len(fragment_paths) != len(set(fragment_paths)):
        raise ValueError("override_fragments must be unique")
    effective = copy.deepcopy(matrix)
    overrides = effective.get("overrides")
    if not isinstance(overrides, dict):
        raise ValueError("overrides must be an object")
    bindings: list[dict[str, Any]] = []
    seen_fragment_ids: set[str] = set()
    for position, raw_path in enumerate(fragment_paths, start=1):
        if not isinstance(raw_path, str) or not raw_path.strip():
            raise ValueError(f"override fragment {position} path must be nonempty")
        relative = Path(raw_path)
        resolved = (REPOSITORY_ROOT / relative).resolve()
        if (
            not resolved.is_relative_to(FRAGMENT_ROOT)
            or resolved.suffix.lower() != ".json"
        ):
            raise ValueError(
                f"override fragment path is outside validation/parity/fragments: {raw_path}"
            )
        fragment = load_json(resolved)
        allowed_keys = {
            "fragment_schema_version",
            "fragment_id",
            "frozen_on",
            "capability_ids",
            "cell_assessments",
            "dimension_assessments",
        }
        unknown_keys = set(fragment) - allowed_keys
        if unknown_keys:
            raise ValueError(
                f"override fragment {raw_path} has unknown fields: {sorted(unknown_keys)}"
            )
        if fragment.get("fragment_schema_version") != 1:
            raise ValueError(
                f"override fragment {raw_path} schema version must equal 1"
            )
        fragment_id = fragment.get("fragment_id")
        if not _nonempty_string(fragment_id) or fragment_id in seen_fragment_ids:
            raise ValueError(
                f"override fragment {raw_path} has a missing or duplicate fragment_id"
            )
        seen_fragment_ids.add(str(fragment_id))
        capabilities = fragment.get("capability_ids")
        if (
            not isinstance(capabilities, list)
            or not capabilities
            or len(capabilities) != len(set(capabilities))
            or not all(_nonempty_string(value) for value in capabilities)
        ):
            raise ValueError(
                f"override fragment {raw_path} capability_ids must be a nonempty unique string list"
            )
        cell_assessments = fragment.get("cell_assessments")
        dimension_assessments = fragment.get("dimension_assessments")
        if not isinstance(cell_assessments, list) or not isinstance(
            dimension_assessments, list
        ):
            raise ValueError(f"override fragment {raw_path} assessments must be lists")
        capability_set = set(capabilities)
        for assessment in cell_assessments:
            capability_id = (
                assessment.get("qualification_link", {}).get("capability_id")
                if isinstance(assessment, Mapping)
                else None
            )
            if capability_id not in capability_set:
                raise ValueError(
                    f"override fragment {raw_path} cell assessment is outside capability_ids"
                )
        for assessment in dimension_assessments:
            capability_id = (
                assessment.get("capability_id")
                if isinstance(assessment, Mapping)
                else None
            )
            if capability_id not in capability_set:
                raise ValueError(
                    f"override fragment {raw_path} dimension assessment is outside capability_ids"
                )
        overrides["cell_assessments"].extend(copy.deepcopy(cell_assessments))
        overrides["dimension_assessments"].extend(copy.deepcopy(dimension_assessments))
        bindings.append(
            {
                "path": relative.as_posix(),
                "fragment_id": fragment_id,
                "sha256": canonical_sha256(fragment),
            }
        )
    return effective, bindings


def _nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _is_official_url(value: Any, *, host: str = "smartpls.com") -> bool:
    if not _nonempty_string(value):
        return False
    parsed = urlparse(str(value))
    return parsed.scheme == "https" and parsed.hostname == host and bool(parsed.path)


def _registry_active_rows(registry: Mapping[str, Any]) -> list[Mapping[str, Any]]:
    return [
        row
        for row in registry.get("capabilities", [])
        if isinstance(row, Mapping) and row.get("official_lifecycle") == "active"
    ]


def _registry_exclusion_rows(registry: Mapping[str, Any]) -> list[Mapping[str, Any]]:
    return [
        row
        for row in registry.get("capabilities", [])
        if isinstance(row, Mapping)
        and row.get("official_lifecycle") == "legacy"
        and row.get("capability_id") in EXPECTED_EXCLUSIONS
    ]


def _official_references_for_row(row: Mapping[str, Any]) -> set[str]:
    candidates: list[Any] = [
        row.get("official_url"),
        row.get("documentation_reference"),
        *row.get("settings_references", []),
        *row.get("result_references", []),
    ]
    for cell in row.get("option_cells", []):
        if not isinstance(cell, Mapping):
            continue
        candidates.append(cell.get("documentation_reference"))
        settings_schema = cell.get("settings_schema")
        if isinstance(settings_schema, Mapping):
            candidates.extend(settings_schema.get("references", []))
        result_schema = cell.get("result_schema")
        if isinstance(result_schema, Mapping):
            candidates.extend(result_schema.get("references", []))
    return {str(value) for value in candidates if _is_official_url(value)}


def _link_as_dict(link: Mapping[str, Any]) -> dict[str, Any]:
    return {
        "registry_schema_version": link.get("registry_schema_version"),
        "capability_id": link.get("capability_id"),
        "cell_id": link.get("cell_id"),
        "capability_version": link.get("capability_version"),
    }


def _row_cell_records(
    row: Mapping[str, Any], *, errors: list[str]
) -> tuple[list[tuple[dict[str, Any], Mapping[str, Any] | None]], bool]:
    """Resolve exact identities, preferring authoritative option_cells.

    Capability Registry V2 originally exposed only row ``qualification_links``.
    During the cell-state migration both shapes may exist. When option_cells are
    present, their identities and states are authoritative and legacy links are
    used only as an equality cross-check.
    """

    capability_id = str(row.get("capability_id"))
    option_cells = row.get("option_cells")
    records: list[tuple[dict[str, Any], Mapping[str, Any] | None]] = []
    if isinstance(option_cells, list):
        if not option_cells:
            errors.append(f"{capability_id}: option_cells must not be empty")
            return records, True
        for position, cell in enumerate(option_cells, start=1):
            if not isinstance(cell, Mapping):
                errors.append(
                    f"{capability_id}: option_cell {position} must be an object"
                )
                continue
            link = {
                "registry_schema_version": 2,
                "capability_id": cell.get("capability_id"),
                "cell_id": cell.get("cell_id"),
                "capability_version": cell.get("capability_version"),
            }
            try:
                identity = qualification_link_identity(link)
            except ValueError as exc:
                errors.append(f"{capability_id}: option_cell {position}: {exc}")
                continue
            if identity[1] != capability_id:
                errors.append(
                    f"{capability_id}: option_cell {position} capability_id mismatch"
                )
                continue
            specification = cell.get("qualification_spec")
            if not isinstance(specification, Mapping):
                errors.append(
                    f"{capability_id}: option_cell {position} qualification_spec must be an object"
                )
                continue
            specification_links = specification.get("links")
            if (
                not isinstance(specification_links, list)
                or len(specification_links) != 1
            ):
                errors.append(
                    f"{capability_id}: option_cell {position} must contain exactly one qualification_spec link"
                )
                continue
            try:
                specification_identity = qualification_link_identity(
                    specification_links[0]
                )
            except ValueError as exc:
                errors.append(
                    f"{capability_id}: option_cell {position} qualification_spec: {exc}"
                )
                continue
            if specification_identity != identity:
                errors.append(
                    f"{capability_id}: option_cell {position} qualification_spec identity mismatch"
                )
                continue
            records.append((_link_as_dict(link), cell))

        legacy_links = row.get("qualification_links")
        if isinstance(legacy_links, list):
            legacy_identities: set[tuple[int, str, str, str]] = set()
            for position, link in enumerate(legacy_links, start=1):
                try:
                    legacy_identities.add(qualification_link_identity(link))
                except ValueError as exc:
                    errors.append(f"{capability_id}: legacy link {position}: {exc}")
            option_identities = {
                qualification_link_identity(link) for link, _cell in records
            }
            if legacy_identities != option_identities:
                errors.append(
                    f"{capability_id}: fallback qualification_links drift from authoritative option_cells"
                )
        return records, True

    links = row.get("qualification_links")
    if not isinstance(links, list) or not links:
        errors.append(f"{capability_id}: active row has no capability-cell identity")
        return records, False
    for position, link in enumerate(links, start=1):
        try:
            identity = qualification_link_identity(link)
        except ValueError as exc:
            errors.append(f"{capability_id}: cell {position}: {exc}")
            continue
        if identity[1] != capability_id:
            errors.append(f"{capability_id}: cell {position} capability_id mismatch")
            continue
        records.append((_link_as_dict(link), None))
    return records, False


def _validate_schema_instance(
    document: Mapping[str, Any], schema: Mapping[str, Any] | None
) -> tuple[bool, list[str]]:
    """Use Draft 2020-12 when available; manual validation remains mandatory."""

    if schema is None or importlib.util.find_spec("jsonschema") is None:
        return False, []
    import jsonschema

    try:
        jsonschema.Draft202012Validator.check_schema(schema)
    except jsonschema.SchemaError as exc:
        return True, [f"schema is invalid: {exc.message}"]
    errors = sorted(
        jsonschema.Draft202012Validator(schema).iter_errors(document),
        key=lambda error: tuple(str(part) for part in error.absolute_path),
    )
    return True, [f"matrix schema: {error.message}" for error in errors]


def _validate_static_contract(
    matrix: Mapping[str, Any], registry: Mapping[str, Any], errors: list[str]
) -> None:
    if matrix.get("matrix_schema_version") != MATRIX_SCHEMA_VERSION:
        errors.append("matrix_schema_version must equal 1")
    if matrix.get("matrix_id") != MATRIX_ID:
        errors.append(f"matrix_id must equal {MATRIX_ID!r}")

    binding = matrix.get("registry_binding")
    if not isinstance(binding, Mapping):
        errors.append("registry_binding must be an object")
        binding = {}
    expected_binding = {
        "path": "validation/capabilities/capability_registry_v2.json",
        "registry_schema_version": 2,
        "registry_id": REGISTRY_ID,
        "expected_catalogue_rows": 45,
        "expected_active_rows": 43,
        "expected_exclusions": 2,
    }
    for key, expected in expected_binding.items():
        if binding.get(key) != expected:
            errors.append(f"registry_binding.{key} must equal {expected!r}")

    if registry.get("registry_schema_version") != binding.get(
        "registry_schema_version"
    ):
        errors.append("bound registry_schema_version does not match the registry")
    if registry.get("registry_id") != binding.get("registry_id"):
        errors.append("bound registry_id does not match the registry")
    capabilities = registry.get("capabilities")
    if not isinstance(capabilities, list):
        errors.append("registry capabilities must be a list")
        capabilities = []
    if len(capabilities) != binding.get("expected_catalogue_rows"):
        errors.append("registry catalogue row count does not match the matrix binding")

    dimensions = matrix.get("dimension_contract")
    if not isinstance(dimensions, Mapping):
        errors.append("dimension_contract must be an object")
        dimensions = {}
    if tuple(dimensions.get("required_dimensions", [])) != REQUIRED_DIMENSIONS:
        errors.append(
            "required_dimensions must contain the frozen nine dimensions in order"
        )
    if dimensions.get("uncaptured_default") != {
        "capture_state": "open",
        "reason": "option_level_official_inventory_not_captured",
    }:
        errors.append("uncaptured dimension default must remain explicitly open")
    if dimensions.get("open_trace_cell_scope") != "all_registry_cells_for_row":
        errors.append(
            "open dimension cell trace must derive all registry cells for the row"
        )
    if dimensions.get("open_official_reference_scope") != "registry_official_url":
        errors.append(
            "open dimension reference trace must derive the registry official URL"
        )

    source_policy = matrix.get("official_source_policy")
    if not isinstance(source_policy, Mapping):
        errors.append("official_source_policy must be an object")
        source_policy = {}
    if source_policy.get("required_scheme") != "https":
        errors.append("official sources must require HTTPS")
    if source_policy.get("allowed_host") != "smartpls.com":
        errors.append("official sources must be restricted to smartpls.com")
    catalogue_url = registry.get("catalogue_snapshot", {}).get("official_catalogue_url")
    if not _is_official_url(catalogue_url):
        errors.append("registry catalogue URL is not an allowed official SmartPLS URL")

    cell_contract = matrix.get("cell_contract")
    if not isinstance(cell_contract, Mapping):
        errors.append("cell_contract must be an object")
        cell_contract = {}
    if tuple(cell_contract.get("identity_fields", [])) != (
        "registry_schema_version",
        "capability_id",
        "cell_id",
        "capability_version",
    ):
        errors.append(
            "cell identity fields must preserve the exact V2 qualification identity"
        )
    if (
        cell_contract.get("identity_source")
        != "registry_option_cells_with_qualification_links_fallback"
    ):
        errors.append(
            "cell identity source must prefer option_cells with legacy fallback"
        )
    if cell_contract.get("state_source_precedence") != [
        "registry_option_cells",
        "explicit_open_default",
    ]:
        errors.append("cell state precedence must prefer registry option_cells")
    if cell_contract.get("option_cell_states_are_authoritative") is not True:
        errors.append("registry option-cell states must be authoritative when present")
    defaults = cell_contract.get("uncaptured_defaults")
    expected_defaults = {axis: {"capture_state": "open"} for axis in ASSESSMENT_AXES}
    if defaults != expected_defaults:
        errors.append(
            "all unlisted cell assessment axes must default explicitly to open"
        )


def _validate_assessment(
    axis: str, assessment: Any, *, context: str, errors: list[str]
) -> None:
    if not isinstance(assessment, Mapping):
        errors.append(f"{context}.{axis} must be an object")
        return
    if assessment.get("capture_state") != "captured":
        errors.append(f"{context}.{axis}.capture_state must equal 'captured'")
        return
    field, allowed = CAPTURED_STATE_FIELDS[axis]
    if assessment.get(field) not in allowed:
        errors.append(f"{context}.{axis}.{field} is invalid")
    if set(assessment) != {"capture_state", field}:
        errors.append(f"{context}.{axis} contains unexpected or missing fields")


def _build_cell_assessments(
    matrix: Mapping[str, Any],
    active_rows: list[Mapping[str, Any]],
    errors: list[str],
) -> tuple[
    dict[tuple[int, str, str, str], dict[str, Any]],
    dict[str, list[tuple[int, str, str, str]]],
    dict[str, bool],
]:
    defaults = matrix.get("cell_contract", {}).get("uncaptured_defaults", {})
    resolved: dict[tuple[int, str, str, str], dict[str, Any]] = {}
    identities_by_row: dict[str, list[tuple[int, str, str, str]]] = {}
    option_cell_authority_by_row: dict[str, bool] = {}

    for row in active_rows:
        capability_id = str(row.get("capability_id"))
        identities: list[tuple[int, str, str, str]] = []
        records, uses_option_cells = _row_cell_records(row, errors=errors)
        option_cell_authority_by_row[capability_id] = uses_option_cells
        for link, option_cell in records:
            identity = qualification_link_identity(link)
            if identity in resolved:
                errors.append(f"duplicate active cell identity: {identity!r}")
                continue
            identities.append(identity)
            state_assessments = {
                axis: copy.deepcopy(defaults.get(axis)) for axis in ASSESSMENT_AXES
            }
            state_sources = {axis: "explicit_open_default" for axis in ASSESSMENT_AXES}
            if option_cell is not None:
                for axis in ("coverage", "evidence", "surface"):
                    field, allowed = CAPTURED_STATE_FIELDS[axis]
                    value = option_cell.get(field)
                    if value not in allowed:
                        errors.append(
                            f"{capability_id}: option cell {identity[2]} has invalid {field}"
                        )
                        continue
                    state_assessments[axis] = {
                        "capture_state": "captured",
                        field: value,
                    }
                    state_sources[axis] = "registry_option_cell"
            resolved[identity] = {
                "qualification_link": _link_as_dict(link),
                **state_assessments,
                "state_sources": state_sources,
            }
        identities_by_row[capability_id] = identities

    registry_identity_set = set(resolved)
    overrides = matrix.get("overrides", {}).get("cell_assessments", [])
    if not isinstance(overrides, list):
        errors.append("overrides.cell_assessments must be a list")
        overrides = []
    seen_overrides: set[tuple[int, str, str, str]] = set()
    for index, override in enumerate(overrides, start=1):
        context = f"cell override {index}"
        if not isinstance(override, Mapping):
            errors.append(f"{context} must be an object")
            continue
        try:
            identity = qualification_link_identity(
                override.get("qualification_link", {})
            )
        except ValueError as exc:
            errors.append(f"{context}: {exc}")
            continue
        if identity in seen_overrides:
            errors.append(f"{context}: duplicate override for {identity!r}")
            continue
        seen_overrides.add(identity)
        if identity not in registry_identity_set:
            errors.append(
                f"{context}: identity is not an active registry cell: {identity!r}"
            )
            continue
        _validate_assessment(
            "parity_obligation",
            override.get("parity_obligation"),
            context=context,
            errors=errors,
        )
        if set(override) != {"qualification_link", "parity_obligation"}:
            errors.append(
                f"{context}: only qualification_link and parity_obligation are allowed; "
                "coverage/evidence/surface come from registry option_cells"
            )
        if not any(error.startswith(context) for error in errors):
            resolved[identity]["parity_obligation"] = copy.deepcopy(
                override["parity_obligation"]
            )
            resolved[identity]["state_sources"]["parity_obligation"] = (
                "matrix_cell_assessment"
            )

    # Registry row-level states are intentionally not copied into these cells.
    # They cannot truthfully represent a row whose option cells have mixed states.
    return resolved, identities_by_row, option_cell_authority_by_row


def _validate_dimension_overrides(
    matrix: Mapping[str, Any],
    active_rows: list[Mapping[str, Any]],
    identities_by_row: Mapping[str, list[tuple[int, str, str, str]]],
    errors: list[str],
) -> dict[tuple[str, str], Mapping[str, Any]]:
    rows = {str(row.get("capability_id")): row for row in active_rows}
    overrides = matrix.get("overrides", {}).get("dimension_assessments", [])
    if not isinstance(overrides, list):
        errors.append("overrides.dimension_assessments must be a list")
        return {}
    index: dict[tuple[str, str], Mapping[str, Any]] = {}
    for position, override in enumerate(overrides, start=1):
        context = f"dimension override {position}"
        if not isinstance(override, Mapping):
            errors.append(f"{context} must be an object")
            continue
        capability_id = override.get("capability_id")
        dimension = override.get("dimension")
        key = (str(capability_id), str(dimension))
        if key in index:
            errors.append(f"{context}: duplicate override for {key!r}")
            continue
        if capability_id not in rows:
            errors.append(f"{context}: capability_id is not an active registry row")
            continue
        if dimension not in REQUIRED_DIMENSIONS:
            errors.append(f"{context}: dimension is not in the frozen contract")
            continue
        if override.get("capture_state") != "captured":
            errors.append(f"{context}: capture_state must equal 'captured'")
            continue
        items = override.get("acceptance_items")
        if not isinstance(items, list) or not items:
            errors.append(f"{context}: captured dimension requires acceptance_items")
            continue
        row = rows[str(capability_id)]
        row_cell_identities = set(identities_by_row[str(capability_id)])
        allowed_references = _official_references_for_row(row)
        item_ids: set[str] = set()
        before_item_errors = len(errors)
        for item_position, item in enumerate(items, start=1):
            item_context = f"{context} item {item_position}"
            if not isinstance(item, Mapping):
                errors.append(f"{item_context} must be an object")
                continue
            item_id = item.get("item_id")
            if not _nonempty_string(item_id):
                errors.append(f"{item_context}: item_id must be nonempty")
            elif str(item_id) in item_ids:
                errors.append(f"{item_context}: duplicate item_id {item_id!r}")
            else:
                item_ids.add(str(item_id))
            if not _nonempty_string(item.get("description")):
                errors.append(f"{item_context}: description must be nonempty")
            trace_cells = item.get("trace_cells")
            if not isinstance(trace_cells, list) or not trace_cells:
                errors.append(f"{item_context}: trace_cells must be nonempty")
            else:
                trace_identities: list[tuple[int, str, str, str]] = []
                for trace_position, trace_cell in enumerate(trace_cells, start=1):
                    try:
                        trace_identity = qualification_link_identity(trace_cell)
                    except ValueError as exc:
                        errors.append(
                            f"{item_context}: trace cell {trace_position}: {exc}"
                        )
                        continue
                    trace_identities.append(trace_identity)
                if len(trace_identities) != len(set(trace_identities)):
                    errors.append(f"{item_context}: trace_cells must be unique")
                elif not set(trace_identities).issubset(row_cell_identities):
                    errors.append(f"{item_context}: trace_cells contain a non-row cell")
            references = item.get("official_references")
            if not isinstance(references, list) or not references:
                errors.append(f"{item_context}: official_references must be nonempty")
            else:
                for reference in references:
                    if not _is_official_url(reference):
                        errors.append(
                            f"{item_context}: reference is not an official SmartPLS URL"
                        )
                    elif reference not in allowed_references:
                        errors.append(
                            f"{item_context}: official reference is not recorded on the registry row"
                        )
            criteria = item.get("acceptance_criteria")
            if (
                not isinstance(criteria, list)
                or not criteria
                or not all(_nonempty_string(value) for value in criteria)
            ):
                errors.append(f"{item_context}: acceptance_criteria must be nonempty")
        if len(errors) == before_item_errors:
            index[key] = override
    return index


def _resolved_assessment_value(assessment: Mapping[str, Any], axis: str) -> str:
    if assessment.get("capture_state") != "captured":
        return "open"
    return str(assessment.get(CAPTURED_STATE_FIELDS[axis][0]))


def build_acceptance_report(
    matrix: Mapping[str, Any],
    registry: Mapping[str, Any],
    *,
    schema: Mapping[str, Any] | None = None,
    fragment_bindings: list[Mapping[str, Any]] | None = None,
) -> dict[str, Any]:
    """Build a deterministic contract and finalization report."""

    errors: list[str] = []
    _schema_validation_performed, schema_errors = _validate_schema_instance(
        matrix, schema
    )
    errors.extend(schema_errors)
    _validate_static_contract(matrix, registry, errors)

    active_rows = _registry_active_rows(registry)
    exclusion_rows = _registry_exclusion_rows(registry)
    binding = matrix.get("registry_binding", {})
    if len(active_rows) != binding.get("expected_active_rows"):
        errors.append(f"active registry row count must be 43, found {len(active_rows)}")
    if len(exclusion_rows) != binding.get("expected_exclusions"):
        errors.append(
            f"registry exclusion row count must be 2, found {len(exclusion_rows)}"
        )

    positions = [
        row.get("catalogue_position") for row in registry.get("capabilities", [])
    ]
    if positions != list(range(1, 46)):
        errors.append(
            "registry catalogue rows must preserve ordered positions 1 through 45"
        )

    matrix_exclusions = matrix.get("exclusions")
    if not isinstance(matrix_exclusions, list):
        errors.append("exclusions must be a list")
        matrix_exclusions = []
    matrix_exclusion_ids = {
        item.get("capability_id")
        for item in matrix_exclusions
        if isinstance(item, Mapping)
    }
    registry_exclusion_ids = {row.get("capability_id") for row in exclusion_rows}
    if matrix_exclusion_ids != EXPECTED_EXCLUSIONS:
        errors.append("matrix exclusions must be exactly Blindfolding and GoF")
    if registry_exclusion_ids != EXPECTED_EXCLUSIONS:
        errors.append("registry exclusions must be exactly Blindfolding and GoF")
    for item in matrix_exclusions:
        if not isinstance(item, Mapping):
            errors.append("each exclusion must be an object")
            continue
        if item.get("decision") != "intentionally_excluded":
            errors.append(f"{item.get('capability_id')}: exclusion decision is invalid")

    resolved_cells, identities_by_row, option_cell_authority_by_row = (
        _build_cell_assessments(matrix, active_rows, errors)
    )
    dimension_overrides = _validate_dimension_overrides(
        matrix, active_rows, identities_by_row, errors
    )

    row_reports: list[dict[str, Any]] = []
    open_dimension_count = 0
    captured_dimension_count = 0
    open_axis_counts: Counter[str] = Counter()
    cell_state_counts: dict[str, Counter[str]] = {
        axis: Counter() for axis in ASSESSMENT_AXES
    }
    active_parity_cells = 0
    beyond_parity_cells = 0
    row_without_active_parity = 0
    nonfinal_active_parity_cells = 0
    traced_dimension_count = 0
    multi_cell_rows: list[dict[str, Any]] = []
    option_cell_authority_rows = 0
    fallback_identity_rows = 0
    release_without_full_cells: list[dict[str, Any]] = []

    for row in active_rows:
        capability_id = str(row.get("capability_id"))
        identities = identities_by_row.get(capability_id, [])
        if option_cell_authority_by_row.get(capability_id):
            option_cell_authority_rows += 1
        else:
            fallback_identity_rows += 1
        row_cells: list[dict[str, Any]] = []
        row_active_parity_cells = 0
        row_final = True
        for identity in identities:
            assessment = resolved_cells[identity]
            cell_report = {
                "qualification_link": assessment["qualification_link"],
                "state_sources": copy.deepcopy(assessment["state_sources"]),
            }
            for axis in ASSESSMENT_AXES:
                value = _resolved_assessment_value(assessment[axis], axis)
                cell_report[axis] = value
                cell_state_counts[axis][value] += 1
                if value == "open":
                    open_axis_counts[axis] += 1
                    row_final = False
            role = cell_report["parity_obligation"]
            if role == "active_parity":
                active_parity_cells += 1
                row_active_parity_cells += 1
                is_final_cell = (
                    cell_report["coverage"] == "full"
                    and cell_report["evidence"] == "release_qualified"
                    and cell_report["surface"] == "standard"
                )
                if not is_final_cell:
                    nonfinal_active_parity_cells += 1
                    row_final = False
            elif role == "beyond_parity":
                beyond_parity_cells += 1
            if (
                cell_report["evidence"] == "release_qualified"
                and cell_report["coverage"] != "full"
            ):
                release_without_full_cells.append(
                    copy.deepcopy(cell_report["qualification_link"])
                )
            row_cells.append(cell_report)
        if row_active_parity_cells == 0:
            row_without_active_parity += 1
            row_final = False

        official_url = row.get("official_url")
        if not _is_official_url(official_url):
            errors.append(
                f"{capability_id}: official_url is not an allowed SmartPLS URL"
            )
        dimension_reports: dict[str, Any] = {}
        for dimension in REQUIRED_DIMENSIONS:
            override = dimension_overrides.get((capability_id, dimension))
            if override is None:
                open_dimension_count += 1
                traced_cells = [
                    copy.deepcopy(resolved_cells[identity]["qualification_link"])
                    for identity in identities
                ]
                official_references = (
                    [official_url] if _is_official_url(official_url) else []
                )
                capture_state = "open"
                item_count = 0
                row_final = False
            else:
                captured_dimension_count += 1
                capture_state = "captured"
                items = override["acceptance_items"]
                traced_cell_index = {
                    qualification_link_identity(trace_cell): copy.deepcopy(trace_cell)
                    for item in items
                    for trace_cell in item["trace_cells"]
                }
                traced_cells = [
                    traced_cell_index[identity]
                    for identity in sorted(traced_cell_index)
                ]
                official_references = sorted(
                    {
                        reference
                        for item in items
                        for reference in item["official_references"]
                    }
                )
                item_count = len(items)
            trace_complete = bool(traced_cells) and bool(official_references)
            if trace_complete:
                traced_dimension_count += 1
            else:
                errors.append(f"{capability_id}.{dimension}: trace is incomplete")
                row_final = False
            dimension_reports[dimension] = {
                "capture_state": capture_state,
                "acceptance_item_count": item_count,
                "trace_cells": traced_cells,
                "official_references": official_references,
                "trace_complete": trace_complete,
            }

        if len(identities) > 1:
            multi_cell_rows.append(
                {
                    "capability_id": capability_id,
                    "catalogue_position": row.get("catalogue_position"),
                    "cell_ids": [identity[2] for identity in identities],
                    "limitation": (
                        "The registry row-level coverage/evidence/surface values cannot "
                        "represent mixed option-cell states; this matrix keeps all cell "
                        "assessments independent."
                    ),
                }
            )

        row_reports.append(
            {
                "catalogue_position": row.get("catalogue_position"),
                "capability_id": capability_id,
                "official_method": row.get("official_method"),
                "official_reference": official_url,
                "registry_row_state": {
                    "coverage_state": row.get("coverage_state"),
                    "evidence_state": row.get("evidence_state"),
                    "surface": row.get("surface"),
                    "use": "informational_only_not_inferred_into_cells",
                },
                "cells": row_cells,
                "dimensions": dimension_reports,
                "trace_complete": all(
                    item["trace_complete"] for item in dimension_reports.values()
                ),
                "finalization_ready": row_final,
            }
        )

    exclusion_reports: list[dict[str, Any]] = []
    for row in exclusion_rows:
        capability_id = str(row.get("capability_id"))
        links: list[dict[str, Any]] = []
        records, _uses_option_cells = _row_cell_records(row, errors=errors)
        for link, _option_cell in records:
            links.append(_link_as_dict(link))
        if not links:
            errors.append(f"{capability_id}: exclusion has no exact cell identity")
        official_url = row.get("official_url")
        if not _is_official_url(official_url):
            errors.append(f"{capability_id}: exclusion lacks an official SmartPLS URL")
        exclusion_reports.append(
            {
                "catalogue_position": row.get("catalogue_position"),
                "capability_id": capability_id,
                "decision": "intentionally_excluded",
                "official_reference": official_url,
                "cells": links,
            }
        )

    release_without_full = [
        str(row.get("capability_id"))
        for row in active_rows
        if row.get("evidence_state") == "release_qualified"
        and row.get("coverage_state") != "full"
    ]

    blockers: list[dict[str, Any]] = []
    if open_dimension_count:
        blockers.append(
            {
                "code": "OPEN_OPTION_DIMENSIONS",
                "count": open_dimension_count,
                "message": "Option-level acceptance inventories remain explicitly open.",
            }
        )
    for axis in ASSESSMENT_AXES:
        if open_axis_counts[axis]:
            blockers.append(
                {
                    "code": f"OPEN_CELL_{axis.upper()}",
                    "count": open_axis_counts[axis],
                    "message": f"Cell-level {axis.replace('_', ' ')} assessments remain open.",
                }
            )
    if row_without_active_parity:
        blockers.append(
            {
                "code": "ROWS_WITHOUT_CAPTURED_ACTIVE_PARITY_CELL",
                "count": row_without_active_parity,
                "message": "Each active official row must identify at least one active-parity cell.",
            }
        )
    if nonfinal_active_parity_cells:
        blockers.append(
            {
                "code": "ACTIVE_PARITY_CELL_NOT_FINAL",
                "count": nonfinal_active_parity_cells,
                "message": "Active-parity cells must be full, release-qualified, and Standard.",
            }
        )
    if fallback_identity_rows:
        blockers.append(
            {
                "code": "LEGACY_CELL_IDENTITY_FALLBACK",
                "count": fallback_identity_rows,
                "message": (
                    "Registry option_cells are not yet authoritative for every active row; "
                    "row qualification_links are being used only as an identity fallback."
                ),
            }
        )

    contract_passed = not errors
    finalization_ready = contract_passed and not blockers
    expected_trace_count = len(active_rows) * len(REQUIRED_DIMENSIONS)
    return {
        "report_schema_version": 1,
        "matrix_id": matrix.get("matrix_id"),
        "matrix_version": matrix.get("matrix_version"),
        "contract_passed": contract_passed,
        "finalization_ready": finalization_ready,
        "error_count": len(errors),
        "errors": errors,
        "blockers": blockers,
        "source_bindings": {
            "matrix_path": "validation/parity/wave0_option_level_acceptance_v1.json",
            "matrix_sha256": canonical_sha256(dict(matrix)),
            "registry_path": "validation/capabilities/capability_registry_v2.json",
            "registry_sha256": canonical_sha256(dict(registry)),
            "schema_path": "validation/parity/wave0_option_level_acceptance_v1.schema.json",
            "schema_sha256": canonical_sha256(dict(schema))
            if schema is not None
            else None,
            "schema_contract": "draft_2020_12_plus_fail_closed_manual_checks",
            "override_fragments": copy.deepcopy(fragment_bindings or []),
            "override_fragment_count": len(fragment_bindings or []),
        },
        "catalogue_counts": {
            "catalogue_rows": len(registry.get("capabilities", [])),
            "active_rows": len(active_rows),
            "explicit_exclusions": len(exclusion_rows),
            "active_cell_identities": len(resolved_cells),
            "active_multi_cell_rows": len(multi_cell_rows),
            "option_cell_authority_rows": option_cell_authority_rows,
            "legacy_identity_fallback_rows": fallback_identity_rows,
        },
        "dimension_counts": {
            "required_per_active_row": len(REQUIRED_DIMENSIONS),
            "expected_traces": expected_trace_count,
            "complete_traces": traced_dimension_count,
            "captured": captured_dimension_count,
            "open": open_dimension_count,
        },
        "cell_state_counts": {
            axis: dict(sorted(counts.items()))
            for axis, counts in cell_state_counts.items()
        },
        "cell_role_counts": {
            "active_parity": active_parity_cells,
            "beyond_parity": beyond_parity_cells,
            "open": open_axis_counts["parity_obligation"],
        },
        "coverage_evidence_independence": {
            "enforced": True,
            "row_states_are_informational_only": True,
            "evidence_never_infers_coverage": True,
            "registry_release_qualified_without_full_rows": release_without_full,
            "registry_release_qualified_without_full_count": len(release_without_full),
            "option_cells_release_qualified_without_full": release_without_full_cells,
            "option_cells_release_qualified_without_full_count": len(
                release_without_full_cells
            ),
        },
        "multi_cell_row_limitations": multi_cell_rows,
        "rows": row_reports,
        "exclusions": exclusion_reports,
    }


def project_saved_report(report: Mapping[str, Any]) -> dict[str, Any]:
    """Return the compact checked-in projection of the detailed report."""

    return {
        "report_schema_version": report.get("report_schema_version"),
        "matrix_id": report.get("matrix_id"),
        "matrix_version": report.get("matrix_version"),
        "contract_passed": report.get("contract_passed"),
        "finalization_ready": report.get("finalization_ready"),
        "error_count": report.get("error_count"),
        "errors": copy.deepcopy(report.get("errors", [])),
        "blockers": copy.deepcopy(report.get("blockers", [])),
        "source_bindings": copy.deepcopy(report.get("source_bindings", {})),
        "catalogue_counts": copy.deepcopy(report.get("catalogue_counts", {})),
        "dimension_counts": copy.deepcopy(report.get("dimension_counts", {})),
        "cell_state_counts": copy.deepcopy(report.get("cell_state_counts", {})),
        "cell_role_counts": copy.deepcopy(report.get("cell_role_counts", {})),
        "coverage_evidence_independence": copy.deepcopy(
            report.get("coverage_evidence_independence", {})
        ),
        "multi_cell_row_limitations": copy.deepcopy(
            report.get("multi_cell_row_limitations", [])
        ),
        "rows": [
            {
                "catalogue_position": row.get("catalogue_position"),
                "capability_id": row.get("capability_id"),
                "official_reference": row.get("official_reference"),
                "registry_row_state": copy.deepcopy(row.get("registry_row_state", {})),
                "cells": copy.deepcopy(row.get("cells", [])),
                "dimension_capture_states": {
                    dimension: value.get("capture_state")
                    for dimension, value in row.get("dimensions", {}).items()
                },
                "trace_complete": row.get("trace_complete"),
                "finalization_ready": row.get("finalization_ready"),
            }
            for row in report.get("rows", [])
        ],
        "exclusions": copy.deepcopy(report.get("exclusions", [])),
    }


def check_saved_report(
    expected_detailed: Mapping[str, Any], actual: Mapping[str, Any]
) -> dict[str, Any]:
    expected = project_saved_report(expected_detailed)
    errors: list[str] = []
    if expected != actual:
        errors.append(
            "saved acceptance report differs from the deterministic projection"
        )
    return {
        "passed": not errors,
        "error_count": len(errors),
        "errors": errors,
        "expected_sha256": canonical_sha256(dict(expected)),
        "actual_sha256": canonical_sha256(dict(actual)),
    }


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", type=Path, default=DEFAULT_MATRIX_PATH)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA_PATH)
    parser.add_argument("--registry", type=Path, default=DEFAULT_REGISTRY_PATH)
    parser.add_argument("--check-report", type=Path)
    parser.add_argument("--write-report", type=Path)
    parser.add_argument("--print-saved-report", action="store_true")
    parser.add_argument("--require-finalization-ready", action="store_true")
    parser.add_argument("--summary", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = _parse_args()
    try:
        matrix, fragment_bindings = load_matrix_with_fragments(args.matrix)
        registry = load_json(args.registry)
        schema = load_json(args.schema)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(json.dumps({"contract_passed": False, "errors": [str(exc)]}, indent=2))
        return 1

    report = build_acceptance_report(
        matrix,
        registry,
        schema=schema,
        fragment_bindings=fragment_bindings,
    )
    if args.write_report is not None:
        if not report["contract_passed"]:
            print(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True))
            return 1
        destination = args.write_report.resolve()
        allowed_results = (REPOSITORY_ROOT / "validation/results").resolve()
        if (
            not destination.is_relative_to(allowed_results)
            or destination.suffix.lower() != ".json"
        ):
            print(
                json.dumps(
                    {
                        "contract_passed": False,
                        "errors": [
                            "write-report must target a JSON file under validation/results"
                        ],
                    },
                    indent=2,
                )
            )
            return 1
        destination.parent.mkdir(parents=True, exist_ok=True)
        temporary = destination.with_name(f".{destination.name}.tmp")
        temporary.write_text(
            json.dumps(
                project_saved_report(report),
                ensure_ascii=False,
                indent=2,
                sort_keys=True,
            )
            + "\n",
            encoding="utf-8",
        )
        temporary.replace(destination)
    if args.print_saved_report:
        print(
            json.dumps(
                project_saved_report(report),
                ensure_ascii=False,
                indent=2,
                sort_keys=True,
            )
        )
        return 0 if report["contract_passed"] else 1

    if args.check_report is not None:
        try:
            saved = load_json(args.check_report)
            saved_check = check_saved_report(report, saved)
        except (OSError, ValueError, json.JSONDecodeError) as exc:
            saved_check = {"passed": False, "errors": [str(exc)]}
        report["saved_report_check"] = saved_check
        if not saved_check["passed"]:
            report["contract_passed"] = False
            report["finalization_ready"] = False

    if args.summary:
        independence = report["coverage_evidence_independence"]
        output = {
            "contract_passed": report["contract_passed"],
            "finalization_ready": report["finalization_ready"],
            "error_count": report["error_count"],
            "blocker_count": len(report["blockers"]),
            "catalogue_counts": report["catalogue_counts"],
            "dimension_counts": report["dimension_counts"],
            "cell_role_counts": report["cell_role_counts"],
            "coverage_evidence_independence": {
                "enforced": independence["enforced"],
                "evidence_never_infers_coverage": independence[
                    "evidence_never_infers_coverage"
                ],
                "row_states_are_informational_only": independence[
                    "row_states_are_informational_only"
                ],
                "registry_release_qualified_without_full_count": independence[
                    "registry_release_qualified_without_full_count"
                ],
                "option_cells_release_qualified_without_full_count": independence[
                    "option_cells_release_qualified_without_full_count"
                ],
            },
        }
        if "saved_report_check" in report:
            output["saved_report_check"] = report["saved_report_check"]
    else:
        output = report
    print(json.dumps(output, ensure_ascii=False, indent=2, sort_keys=True))

    if not report["contract_passed"]:
        return 1
    if args.require_finalization_ready and not report["finalization_ready"]:
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
