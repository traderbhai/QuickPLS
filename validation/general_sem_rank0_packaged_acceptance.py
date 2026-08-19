#!/usr/bin/env python3
"""Fail-closed packaged-Windows acceptance contract for General SEM Rank 0.

The contract deliberately separates the reusable package/viewport matrix from
the browser driver. Whole-Rank validation requires every exact capability cell;
cell-atomic validation requires only that cell's installed and portable rows.
Counts never substitute for exact identities or check sets.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any, Mapping, Sequence

try:
    from validation.general_sem_rank0_receipt_payload_v1 import (
        qualification_contract_sha256,
        unified_rank0_source_receipt as unified_rank0_source_receipt,
        validate_unified_rank0_source_receipt,
    )
except ModuleNotFoundError:
    from general_sem_rank0_receipt_payload_v1 import (
        qualification_contract_sha256,
        unified_rank0_source_receipt as unified_rank0_source_receipt,
        validate_unified_rank0_source_receipt,
    )


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONTRACT = (
    ROOT
    / "validation/capabilities/general_sem_rank0_packaged_acceptance_v1.manifest.json"
)
DEFAULT_REGISTRY = ROOT / "validation/capabilities/capability_registry_v2.json"
TOKEN = re.compile(r"^[a-z][a-z0-9_]*$")
CHECK_ID = re.compile(r"^[a-z][a-z0-9_]*$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
REQUIRED_ARTIFACT_KINDS = (
    "run_trace",
    "canonical_result",
    "exported_files_manifest",
    "accessibility_snapshot",
    "process_cleanup_trace",
    "close_reopen_trace",
)
EXPORT_FORMATS = ("csv", "xlsx", "html", "pdf", "svg", "png")
PACKAGE_PROVENANCE = {
    "evidence_kind": "windows_pe_package_identity_v1",
    "file_identity_source": "resolved_path_size_sha256",
    "version_identity_source": "System.Diagnostics.FileVersionInfo",
}
PACKAGED_TAURI_ORIGIN = "http://tauri.localhost"
PACKAGED_TAURI_IPC_ORIGIN = "http://ipc.localhost"
ALLOWED_OFFLINE_ORIGINS = {
    PACKAGED_TAURI_ORIGIN,
    PACKAGED_TAURI_IPC_ORIGIN,
    "null",
    None,
}
QUALIFICATION_SPEC_PATHS = {
    "mediation_point": "validation/qualification_v2/mediation_v1.qualification.json",
    "multiple_mediation_bootstrap": "validation/qualification_v2/general_sem_pls_multiple_mediation_bootstrap_v1.qualification.json",
    "multiple_two_way_moderation_point": "validation/qualification_v2/general_sem_pls_multiple_moderation_point_v1.qualification.json",
    "multiple_two_way_moderation_bootstrap": "validation/qualification_v2/general_sem_pls_multiple_moderation_bootstrap_v1.qualification.json",
}
MEDIATION_POINT_REFERENCE = (
    2,
    "smartpls.mediation",
    "qpls3.pls.mediation",
    "pls_mediation_v1",
)
MODERATION_POINT_REFERENCE = (
    2,
    "smartpls.moderation",
    "qpls3.pls.general_sem_multiple_two_way_moderation_point",
    "general_sem_pls_multiple_two_way_moderation_point_v1",
)
CANONICAL_AUTHORITIES = {
    "mediation_point": {
        "request": MEDIATION_POINT_REFERENCE,
        "primary": MEDIATION_POINT_REFERENCE,
        "supplemental": None,
        "method_version": "general_sem_effects_v1",
    },
    "multiple_mediation_bootstrap": {
        "request": (
            2,
            "smartpls.mediation",
            "qpls3.pls.general_sem_multiple_mediation_bootstrap",
            "general_sem_pls_full_model_case_bootstrap_v1",
        ),
        "primary": MEDIATION_POINT_REFERENCE,
        "supplemental": (
            2,
            "smartpls.mediation",
            "qpls3.pls.general_sem_multiple_mediation_bootstrap",
            "general_sem_pls_full_model_case_bootstrap_v1",
        ),
        "method_version": "general_sem_pls_full_model_case_bootstrap_v1",
    },
    "multiple_two_way_moderation_point": {
        "request": MODERATION_POINT_REFERENCE,
        "primary": MODERATION_POINT_REFERENCE,
        "supplemental": None,
        "method_version": "general_sem_pls_multiple_two_way_moderation_point_v1",
    },
    "multiple_two_way_moderation_bootstrap": {
        "request": (
            2,
            "smartpls.moderation",
            "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
            "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
        ),
        "primary": MODERATION_POINT_REFERENCE,
        "supplemental": (
            2,
            "smartpls.moderation",
            "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
            "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
        ),
        "method_version": "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
    },
}


class ContractError(ValueError):
    pass


def variant_canonical_authority(variant: Mapping[str, Any]) -> Mapping[str, Any]:
    authority = CANONICAL_AUTHORITIES.get(variant.get("variant_id"))
    if authority is None or authority["request"] != variant.get("reference"):
        raise ContractError(
            f"Rank 0 canonical authority is unavailable for {variant.get('variant_id')!r}"
        )
    return authority


def _strict_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ContractError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    try:
        return json.loads(
            path.read_text(encoding="utf-8-sig"),
            object_pairs_hook=_strict_object,
            parse_constant=lambda token: (_ for _ in ()).throw(
                ContractError(f"non-finite JSON value: {token}")
            ),
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise ContractError(str(error)) from error


def canonical_sha256(value: Any) -> str:
    payload = json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _package_fingerprint(packages: Sequence[Mapping[str, Any]]) -> str:
    lines: list[str] = []
    for expected_kind, package in zip(("installed", "portable"), packages):
        provenance = package.get("provenance")
        if (
            package.get("package_kind") != expected_kind
            or provenance != PACKAGE_PROVENANCE
        ):
            raise ContractError("package identities/provenance are not frozen")
        lines.extend(
            [
                f"package_kind={expected_kind}",
                f"resolved_path={package.get('resolved_path')}",
                f"size={package.get('size')}",
                f"sha256={package.get('sha256')}",
                f"product_version={package.get('product_version')}",
                f"file_version={package.get('file_version')}",
                f"provenance.evidence_kind={provenance['evidence_kind']}",
                f"provenance.file_identity_source={provenance['file_identity_source']}",
                f"provenance.version_identity_source={provenance['version_identity_source']}",
            ]
        )
    return hashlib.sha256("\n".join(lines).encode("utf-8")).hexdigest()


def _validate_package_identities(value: Any) -> dict[str, Mapping[str, Any]]:
    if not isinstance(value, list) or len(value) != 2:
        raise ContractError("report must bind exactly two package identities")
    normalized: dict[str, Mapping[str, Any]] = {}
    for expected_kind, raw in zip(("installed", "portable"), value):
        row = _exact_keys(
            raw,
            {
                "package_kind",
                "resolved_path",
                "size",
                "sha256",
                "product_version",
                "file_version",
                "provenance",
            },
            f"package_identities.{expected_kind}",
        )
        path_text = row.get("resolved_path")
        unresolved = Path(path_text) if isinstance(path_text, str) else Path()
        path = unresolved.resolve()
        if (
            row.get("package_kind") != expected_kind
            or not isinstance(path_text, str)
            or not Path(path_text).is_absolute()
            or unresolved.is_symlink()
            or not path.is_file()
            or row.get("size") != path.stat().st_size
            or row.get("sha256") != hashlib.sha256(path.read_bytes()).hexdigest()
            or not isinstance(row.get("sha256"), str)
            or not SHA256.fullmatch(row["sha256"])
            or not isinstance(row.get("product_version"), str)
            or not row["product_version"]
            or not isinstance(row.get("file_version"), str)
            or not row["file_version"]
            or row.get("provenance") != PACKAGE_PROVENANCE
        ):
            raise ContractError(
                f"{expected_kind} package identity differs from current bytes"
            )
        normalized[expected_kind] = row
    if (
        normalized["installed"]["resolved_path"].casefold()
        == normalized["portable"]["resolved_path"].casefold()
    ):
        raise ContractError("installed and portable package paths must be distinct")
    return normalized


def qualification_contract_authorities(
    context: Mapping[str, Any], repository_root: Path = ROOT
) -> list[dict[str, Any]]:
    """Bind each Registry cell to its immutable QualificationSpec projection."""

    authorities: list[dict[str, Any]] = []
    for variant in context["variants"]:
        variant_id = variant["variant_id"]
        relative = QUALIFICATION_SPEC_PATHS.get(variant_id)
        if relative is None:
            raise ContractError(
                f"QualificationSpec path is unavailable for {variant_id}"
            )
        path = (repository_root / relative).resolve()
        try:
            path.relative_to(repository_root.resolve())
        except ValueError as error:
            raise ContractError(
                "QualificationSpec path leaves the repository"
            ) from error
        if not path.is_file() or path.is_symlink():
            raise ContractError(
                f"QualificationSpec is missing or symlinked for {variant_id}"
            )
        specification = load_json(path)
        if not isinstance(specification, Mapping):
            raise ContractError(f"QualificationSpec root is invalid for {variant_id}")
        authorities.append(
            {
                "variant_id": variant_id,
                "capability_reference": {
                    "registry_schema_version": variant["reference"][0],
                    "capability_id": variant["reference"][1],
                    "cell_id": variant["reference"][2],
                    "capability_version": variant["reference"][3],
                },
                "qualification_spec_path": relative,
                "qualification_contract_sha256": qualification_contract_sha256(
                    specification
                ),
            }
        )
    return authorities


def _validate_qualification_contracts(
    value: Any, context: Mapping[str, Any], repository_root: Path
) -> dict[tuple[int, str, str, str], Mapping[str, Any]]:
    expected = qualification_contract_authorities(context, repository_root)
    if value != expected:
        raise ContractError(
            "qualification_contracts do not bind the exact current normalized contracts"
        )
    return {
        _reference(
            row["capability_reference"], "qualification_contracts.reference"
        ): row
        for row in expected
    }


def _validate_hardware_fingerprint(value: Any) -> Mapping[str, Any]:
    row = _exact_keys(
        value,
        {
            "os",
            "architecture",
            "cpu",
            "physical_cores",
            "logical_cores",
            "memory_bytes",
        },
        "hardware_fingerprint",
    )
    physical = row.get("physical_cores")
    logical = row.get("logical_cores")
    memory = row.get("memory_bytes")
    if (
        row.get("os") != "windows_11"
        or row.get("architecture") != "x86_64"
        or not isinstance(row.get("cpu"), str)
        or not row["cpu"].strip()
        or not isinstance(physical, int)
        or isinstance(physical, bool)
        or physical < 6
        or not isinstance(logical, int)
        or isinstance(logical, bool)
        or logical < physical
        or not isinstance(memory, int)
        or isinstance(memory, bool)
        or memory < 16 * 1024**3
    ):
        raise ContractError(
            "hardware_fingerprint does not satisfy standard_windows_6c16g"
        )
    return row


def _exact_keys(value: Any, expected: set[str], subject: str) -> Mapping[str, Any]:
    if not isinstance(value, Mapping) or set(value) != expected:
        raise ContractError(f"{subject} fields are not exact")
    return value


def _unique_tokens(values: Any, subject: str, minimum: int = 1) -> tuple[str, ...]:
    if (
        not isinstance(values, list)
        or len(values) < minimum
        or any(
            not isinstance(value, str) or not TOKEN.fullmatch(value) for value in values
        )
        or len(values) != len(set(values))
    ):
        raise ContractError(f"{subject} must be a unique token list")
    return tuple(values)


def _reference(value: Any, subject: str) -> tuple[int, str, str, str]:
    row = _exact_keys(
        value,
        {"registry_schema_version", "capability_id", "cell_id", "capability_version"},
        subject,
    )
    fields = (
        row.get("registry_schema_version"),
        row.get("capability_id"),
        row.get("cell_id"),
        row.get("capability_version"),
    )
    if fields[0] != 2 or any(
        not isinstance(value, str) or not value for value in fields[1:]
    ):
        raise ContractError(f"{subject} is invalid")
    return fields  # type: ignore[return-value]


def validate_contract(contract: Any, registry: Any) -> dict[str, Any]:
    root = _exact_keys(
        contract,
        {
            "schema_version",
            "contract_id",
            "contract_version",
            "target_surface",
            "packages",
            "viewports",
            "windows_scaling_percent",
            "common_required_check_ids",
            "bootstrap_required_check_ids",
            "variants",
        },
        "contract",
    )
    if root.get("schema_version") != 1:
        raise ContractError("schema_version must equal 1")
    if (
        root.get("contract_id")
        != "quickpls.general_sem.rank0.packaged_windows_acceptance.v1"
    ):
        raise ContractError("contract_id is invalid")
    if (
        not isinstance(root.get("contract_version"), str)
        or not root["contract_version"]
    ):
        raise ContractError("contract_version is invalid")
    if root.get("target_surface") != "standard":
        raise ContractError("target_surface must equal standard")
    packages = _unique_tokens(root.get("packages"), "packages", 2)
    if set(packages) != {"installed", "portable"}:
        raise ContractError("packages must contain installed and portable")
    viewport_values = root.get("viewports")
    if (
        not isinstance(viewport_values, list)
        or len(viewport_values) != 3
        or any(
            not isinstance(value, str)
            or not re.fullmatch(r"[1-9][0-9]{2,3}x[1-9][0-9]{2,3}", value)
            for value in viewport_values
        )
        or len(viewport_values) != len(set(viewport_values))
    ):
        raise ContractError("viewports must be three unique WIDTHxHEIGHT values")
    viewports = tuple(viewport_values)
    if set(viewports) != {"1024x700", "1280x720", "1440x900"}:
        raise ContractError("viewports do not match the Rank 0 matrix")
    scaling = root.get("windows_scaling_percent")
    if scaling != [100, 125, 150, 200]:
        raise ContractError("windows_scaling_percent must be 100/125/150/200")
    common = _unique_tokens(root.get("common_required_check_ids"), "common checks")
    bootstrap = _unique_tokens(
        root.get("bootstrap_required_check_ids"), "bootstrap checks", 0
    )
    if set(common) & set(bootstrap):
        raise ContractError("common and bootstrap check IDs must be disjoint")

    registry_root = _exact_keys(
        registry,
        set(registry) if isinstance(registry, Mapping) else set(),
        "registry",
    )
    if registry_root.get("registry_schema_version") != 2:
        raise ContractError("Registry schema version must equal 2")
    capabilities = registry_root.get("capabilities")
    if not isinstance(capabilities, list):
        raise ContractError("Registry capabilities are missing")
    registry_cells: dict[tuple[int, str, str, str], Mapping[str, Any]] = {}
    for capability in capabilities:
        if not isinstance(capability, Mapping):
            continue
        capability_id = capability.get("capability_id")
        for cell in capability.get("option_cells", []):
            if not isinstance(cell, Mapping):
                continue
            identity = (
                2,
                capability_id,
                cell.get("cell_id"),
                cell.get("capability_version"),
            )
            if all(isinstance(value, (int, str)) for value in identity):
                registry_cells[identity] = cell

    variants = root.get("variants")
    if not isinstance(variants, list) or len(variants) != 4:
        raise ContractError("Rank 0 must define exactly four variants")
    resolved: list[dict[str, Any]] = []
    variant_ids: list[str] = []
    references: list[tuple[int, str, str, str]] = []
    for index, value in enumerate(variants):
        variant = _exact_keys(
            value,
            {"variant_id", "bootstrap", "capability_reference"},
            f"variants[{index}]",
        )
        variant_id = variant.get("variant_id")
        if not isinstance(variant_id, str) or not TOKEN.fullmatch(variant_id):
            raise ContractError(f"variants[{index}].variant_id is invalid")
        if not isinstance(variant.get("bootstrap"), bool):
            raise ContractError(f"variants[{index}].bootstrap must be Boolean")
        reference = _reference(
            variant.get("capability_reference"),
            f"variants[{index}].capability_reference",
        )
        if reference not in registry_cells:
            raise ContractError(
                f"Rank 0 capability is not an exact active Registry cell: {reference}"
            )
        variant_ids.append(variant_id)
        references.append(reference)
        resolved.append(
            {
                "variant_id": variant_id,
                "bootstrap": variant["bootstrap"],
                "reference": reference,
                "registry_cell": registry_cells[reference],
                "required_checks": common + (bootstrap if variant["bootstrap"] else ()),
            }
        )
    if len(variant_ids) != len(set(variant_ids)):
        raise ContractError("variant IDs must be unique")
    if len(references) != len(set(references)):
        raise ContractError("variant capability references must be unique")
    if sum(row["bootstrap"] for row in resolved) != 2:
        raise ContractError("Rank 0 must define exactly two bootstrap variants")
    return {
        "contract_id": root["contract_id"],
        "contract_version": root["contract_version"],
        "contract_sha256": canonical_sha256(contract),
        "packages": packages,
        "viewports": viewports,
        "scaling": tuple(scaling),
        "variants": resolved,
    }


def _validate_artifact(value: Any, repository_root: Path, subject: str) -> Path:
    artifact = _exact_keys(value, {"kind", "path", "size", "sha256"}, subject)
    if not isinstance(artifact.get("kind"), str) or not TOKEN.fullmatch(
        artifact["kind"]
    ):
        raise ContractError(f"{subject}.kind is invalid")
    raw_path = artifact.get("path")
    if not isinstance(raw_path, str) or not raw_path or "\\" in raw_path:
        raise ContractError(f"{subject}.path must be repository-relative POSIX text")
    relative = Path(raw_path)
    if relative.is_absolute() or ".." in relative.parts:
        raise ContractError(f"{subject}.path is unsafe")
    path = repository_root / relative
    if not path.is_file():
        raise ContractError(f"{subject}.path is missing: {raw_path}")
    if artifact.get("size") != path.stat().st_size:
        raise ContractError(f"{subject}.size does not match {raw_path}")
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if artifact.get("sha256") != digest or not SHA256.fullmatch(digest):
        raise ContractError(f"{subject}.sha256 does not match {raw_path}")
    return path


def _bound_evidence(
    path: Path,
    *,
    package_kind: str,
    variant_id: str,
    evidence_kind: str,
    subject: str,
) -> Mapping[str, Any]:
    value = load_json(path)
    if not isinstance(value, Mapping):
        raise ContractError(f"{subject} must contain one JSON object")
    if value.get("schema_version") != 1 or value.get("evidence_kind") != evidence_kind:
        raise ContractError(f"{subject} evidence identity is invalid")
    if (
        value.get("package_kind") != package_kind
        or value.get("variant_id") != variant_id
    ):
        raise ContractError(
            f"{subject} package/variant identity does not match the report"
        )
    return value


def _validate_exported_files_manifest(
    value: Mapping[str, Any], repository_root: Path, subject: str
) -> None:
    _exact_keys(
        value,
        {
            "schema_version",
            "evidence_kind",
            "package_kind",
            "variant_id",
            "run_id",
            "document_id",
            "files",
        },
        subject,
    )
    files = value.get("files")
    if not isinstance(files, list) or len(files) != len(EXPORT_FORMATS):
        raise ContractError(f"{subject} must bind exactly six exported files")
    paths: set[str] = set()
    for index, (file_value, expected_format) in enumerate(zip(files, EXPORT_FORMATS)):
        row = _exact_keys(
            file_value,
            {"format", "path", "size", "sha256", "semantic_readback"},
            f"{subject}.files[{index}]",
        )
        if row.get("format") != expected_format:
            raise ContractError(f"{subject}.files[{index}] format/order is invalid")
        raw_path = row.get("path")
        if not isinstance(raw_path, str) or raw_path in paths:
            raise ContractError(f"{subject} export paths must be unique")
        paths.add(raw_path)
        _validate_artifact(
            {
                "kind": "exported_file",
                "path": row["path"],
                "size": row["size"],
                "sha256": row["sha256"],
            },
            repository_root,
            f"{subject}.files[{index}]",
        )
        if Path(raw_path).suffix.lower() != f".{expected_format}":
            raise ContractError(f"{subject}.files[{index}] extension is invalid")
        readback = _exact_keys(
            row.get("semantic_readback"),
            {
                "schema_version",
                "evidence_kind",
                "format",
                "document_id",
                "run_id",
                "method_version",
                "dataset_fingerprint",
                "semantic_sha256",
                "table_ids",
                "chart_ids",
                "canonical_values_sha256",
                "rendered_surface_match",
                "canonical_match",
                "passed",
            },
            f"{subject}.files[{index}].semantic_readback",
        )
        if (
            readback.get("schema_version") != 1
            or readback.get("evidence_kind")
            != "general_sem_rank0_export_semantic_readback"
            or readback.get("format") != expected_format
            or readback.get("document_id") != value.get("document_id")
            or readback.get("run_id") != value.get("run_id")
            or any(
                not isinstance(readback.get(field), str) or not readback[field]
                for field in ("method_version", "dataset_fingerprint")
            )
            or any(
                not isinstance(readback.get(field), str)
                or not SHA256.fullmatch(readback[field])
                for field in ("semantic_sha256", "canonical_values_sha256")
            )
            or not isinstance(readback.get("table_ids"), list)
            or not isinstance(readback.get("chart_ids"), list)
            or any(
                readback.get(field) is not True
                for field in ("rendered_surface_match", "canonical_match", "passed")
            )
        ):
            raise ContractError(
                f"{subject}.files[{index}] independent semantic readback did not pass"
            )


def _validate_role_evidence(
    paths: Mapping[str, Path],
    *,
    package_kind: str,
    variant: Mapping[str, Any],
    package_identity: Mapping[str, Any],
    repository_root: Path,
    subject: str,
) -> None:
    variant_id = variant["variant_id"]
    run = _bound_evidence(
        paths["run_trace"],
        package_kind=package_kind,
        variant_id=variant_id,
        evidence_kind="general_sem_rank0_run_trace",
        subject=f"{subject}.run_trace",
    )
    _exact_keys(
        run,
        {
            "schema_version",
            "evidence_kind",
            "package_kind",
            "variant_id",
            "capability_reference",
            "offline",
            "offline_observations",
            "steps",
            "run_id",
            "document_id",
            "project_archive",
            "cancellation_observation",
            "export_cancellation_observation",
        },
        f"{subject}.run_trace",
    )
    if (
        _reference(
            run.get("capability_reference"), f"{subject}.run_trace.capability_reference"
        )
        != variant["reference"]
    ):
        raise ContractError(f"{subject}.run_trace capability reference does not match")
    if run.get("offline") is not True or not isinstance(run.get("steps"), Mapping):
        raise ContractError(
            f"{subject}.run_trace did not prove its exact offline steps"
        )
    offline_observations = run.get("offline_observations")
    if not isinstance(offline_observations, list) or len(offline_observations) != 5:
        raise ContractError(
            f"{subject}.run_trace offline observation matrix is incomplete"
        )
    for index, observation in enumerate(offline_observations):
        offline = _exact_keys(
            observation,
            {
                "phase",
                "scale_percent",
                "observed_request_count",
                "external_request_count",
                "origins",
                "external_requests",
                "passed",
            },
            f"{subject}.run_trace.offline_observations[{index}]",
        )
        expected_phase = "execute" if index == 0 else "reopen"
        expected_scale = (100, 100, 125, 150, 200)[index]
        origins = offline.get("origins")
        observed_count = offline.get("observed_request_count")
        if (
            offline.get("phase") != expected_phase
            or offline.get("scale_percent") != expected_scale
            or not isinstance(observed_count, int)
            or isinstance(observed_count, bool)
            or observed_count <= 0
            or offline.get("external_request_count") != 0
            or not isinstance(origins, list)
            or not origins
            or len(origins) != len(set(origins))
            or any(origin not in ALLOWED_OFFLINE_ORIGINS for origin in origins)
            or not any(
                origin in {PACKAGED_TAURI_ORIGIN, PACKAGED_TAURI_IPC_ORIGIN}
                for origin in origins
            )
            or offline.get("external_requests") != []
            or offline.get("passed") is not True
        ):
            raise ContractError(
                f"{subject}.run_trace offline observation {index} is not positive and zero-external"
            )
    if tuple(run["steps"]) != variant["required_checks"] or any(
        value is not True for value in run["steps"].values()
    ):
        raise ContractError(f"{subject}.run_trace step set is incomplete")
    if not all(
        isinstance(run.get(field), str) and run[field]
        for field in ("run_id", "document_id")
    ):
        raise ContractError(f"{subject}.run_trace canonical identities are missing")
    project_archive = _exact_keys(
        run.get("project_archive"),
        {"path", "size", "sha256"},
        f"{subject}.run_trace.project_archive",
    )
    project_archive_path = _validate_artifact(
        {"kind": "project_archive", **project_archive},
        repository_root,
        f"{subject}.run_trace.project_archive",
    )
    if project_archive_path.suffix.lower() != ".qpls":
        raise ContractError(f"{subject}.run_trace project archive extension is invalid")
    cancellation = run.get("cancellation_observation")
    row = _exact_keys(
        cancellation,
        {
            "terminal_latency_seconds",
            "terminal_state",
            "job_completed_before_cancel",
            "no_partial_visible_result",
            "no_partial_committed_result",
            "archive_unchanged",
            "exact_same_settings_retry",
            "archive_before",
            "archive_after",
        },
        f"{subject}.run_trace.cancellation_observation",
    )
    latency = row.get("terminal_latency_seconds")
    if (
        not isinstance(latency, (int, float))
        or isinstance(latency, bool)
        or latency < 0
        or latency > 1
        or row.get("terminal_state") != "cancelled"
        or row.get("job_completed_before_cancel") is not False
        or any(
            row.get(field) is not True
            for field in (
                "no_partial_visible_result",
                "no_partial_committed_result",
                "archive_unchanged",
                "exact_same_settings_retry",
            )
        )
    ):
        raise ContractError(
            f"{subject}.run_trace cancellation did not satisfy the <=1.0s atomic gate"
        )
    archive_fields = {
        "byte_length",
        "sha256",
        "canonical_result_attachment_count",
    }
    before = _exact_keys(
        row.get("archive_before"),
        archive_fields,
        f"{subject}.run_trace.cancellation_observation.archive_before",
    )
    after = _exact_keys(
        row.get("archive_after"),
        archive_fields,
        f"{subject}.run_trace.cancellation_observation.archive_after",
    )
    if (
        before != after
        or not isinstance(before.get("byte_length"), int)
        or isinstance(before.get("byte_length"), bool)
        or before["byte_length"] <= 0
        or not isinstance(before.get("sha256"), str)
        or not SHA256.fullmatch(before["sha256"])
        or not isinstance(before.get("canonical_result_attachment_count"), int)
        or isinstance(before.get("canonical_result_attachment_count"), bool)
        or before["canonical_result_attachment_count"] < 0
    ):
        raise ContractError(
            f"{subject}.run_trace cancellation changed archive bytes or attachments"
        )
    export_cancellation = _exact_keys(
        run.get("export_cancellation_observation"),
        {
            "ui_control_cancellations",
            "save_dialog_destination_path",
            "save_dialog_cancelled",
            "semantic_readback_completed",
            "save_dialog_no_partial_file",
        },
        f"{subject}.run_trace.export_cancellation_observation",
    )
    save_path = export_cancellation.get("save_dialog_destination_path")
    ui_rows = export_cancellation.get("ui_control_cancellations")
    if not isinstance(ui_rows, list) or len(ui_rows) != 3:
        raise ContractError(
            f"{subject}.run_trace export cancellation matrix is incomplete"
        )
    for expected_format, ui in zip(("csv", "xlsx", "png"), ui_rows):
        row = _exact_keys(
            ui,
            {
                "format",
                "destination_path",
                "terminal_latency_seconds",
                "terminal_state",
                "cancel_control_activated",
                "native_dialog_observed",
                "no_partial_file",
                "temp_files_unchanged",
            },
            f"{subject}.run_trace.{expected_format}_export_cancellation",
        )
        ui_path = row.get("destination_path")
        latency = row.get("terminal_latency_seconds")
        if (
            row.get("format") != expected_format
            or not isinstance(ui_path, str)
            or not ui_path.endswith(f".{expected_format}")
            or Path(ui_path).is_absolute()
            or ".." in Path(ui_path).parts
            or not isinstance(latency, (int, float))
            or isinstance(latency, bool)
            or latency < 0
            or latency > 1
            or row.get("terminal_state") != "cancelled"
            or any(
                row.get(field) is not expected
                for field, expected in (
                    ("cancel_control_activated", True),
                    ("native_dialog_observed", False),
                    ("no_partial_file", True),
                    ("temp_files_unchanged", True),
                )
            )
            or (repository_root / ui_path).exists()
        ):
            raise ContractError(
                f"{subject}.run_trace {expected_format} cancellation did not prove zero publication"
            )
    if (
        not isinstance(save_path, str)
        or not save_path.endswith(".csv")
        or Path(save_path).is_absolute()
        or ".." in Path(save_path).parts
        or any(
            export_cancellation.get(field) is not expected
            for field, expected in (
                ("save_dialog_cancelled", True),
                ("semantic_readback_completed", True),
                ("save_dialog_no_partial_file", True),
            )
        )
        or (repository_root / ui_path).exists()
        or (repository_root / save_path).exists()
    ):
        raise ContractError(
            f"{subject}.run_trace export cancellations did not prove both zero-publication boundaries"
        )

    canonical = _bound_evidence(
        paths["canonical_result"],
        package_kind=package_kind,
        variant_id=variant_id,
        evidence_kind="general_sem_rank0_canonical_result",
        subject=f"{subject}.canonical_result",
    )
    _exact_keys(
        canonical,
        {
            "schema_version",
            "evidence_kind",
            "package_kind",
            "variant_id",
            "capability_reference",
            "primary_capability_reference",
            "supplemental_capability_reference",
            "method_version",
            "document_id",
            "run_id",
            "canonical_document_sha256",
            "canonical_document",
        },
        f"{subject}.canonical_result",
    )
    if (
        _reference(
            canonical.get("capability_reference"),
            f"{subject}.canonical_result.capability_reference",
        )
        != variant["reference"]
    ):
        raise ContractError(
            f"{subject}.canonical_result capability reference does not match"
        )
    authority = variant_canonical_authority(variant)
    primary = _reference(
        canonical.get("primary_capability_reference"),
        f"{subject}.canonical_result.primary_capability_reference",
    )
    supplemental_value = canonical.get("supplemental_capability_reference")
    supplemental = (
        _reference(
            supplemental_value,
            f"{subject}.canonical_result.supplemental_capability_reference",
        )
        if supplemental_value is not None
        else None
    )
    if (
        primary != authority["primary"]
        or supplemental != authority["supplemental"]
        or canonical.get("method_version") != authority["method_version"]
    ):
        raise ContractError(
            f"{subject}.canonical_result primary/supplemental authority is invalid"
        )
    document = canonical.get("canonical_document")
    provenance = document.get("provenance") if isinstance(document, Mapping) else None
    if (
        canonical.get("run_id") != run["run_id"]
        or canonical.get("document_id") != run["document_id"]
        or not isinstance(provenance, Mapping)
        or provenance.get("run_id") != run["run_id"]
        or _reference(
            provenance.get("capability_cell"),
            f"{subject}.canonical_result.provenance.capability_cell",
        )
        != authority["primary"]
        or provenance.get("method_version") != authority["method_version"]
    ):
        raise ContractError(
            f"{subject}.canonical_result does not reconcile to the exact run/primary cell"
        )
    inventory = (
        document.get("capability_cells") if isinstance(document, Mapping) else None
    )
    if not isinstance(inventory, list):
        raise ContractError(
            f"{subject}.canonical_result capability inventory is missing"
        )
    inventory_cells = [
        _reference(cell, f"{subject}.canonical_result.capability_cells[{index}]")
        for index, cell in enumerate(inventory)
    ]
    if inventory_cells.count(authority["primary"]) != 1 or (
        authority["supplemental"] is not None
        and inventory_cells.count(authority["supplemental"]) != 1
    ):
        raise ContractError(
            f"{subject}.canonical_result capability inventory omits its exact authority"
        )
    results = (
        document.get("general_sem_results") if isinstance(document, Mapping) else None
    )
    receipt = results.get("inference_receipt") if isinstance(results, Mapping) else None
    if authority["supplemental"] is None:
        if receipt is not None:
            raise ContractError(
                f"{subject}.canonical_result point authority cannot carry a supplemental receipt"
            )
    elif (
        not isinstance(receipt, Mapping)
        or _reference(
            receipt.get("capability_cell"),
            f"{subject}.canonical_result.inference_receipt.capability_cell",
        )
        != authority["supplemental"]
        or receipt.get("method_version") != authority["method_version"]
    ):
        raise ContractError(
            f"{subject}.canonical_result supplemental inference receipt is invalid"
        )
    digest = canonical.get("canonical_document_sha256")
    if (
        not isinstance(digest, str)
        or not SHA256.fullmatch(digest)
        or digest != canonical_sha256(document)
    ):
        raise ContractError(f"{subject}.canonical_result digest is invalid")

    exports = _bound_evidence(
        paths["exported_files_manifest"],
        package_kind=package_kind,
        variant_id=variant_id,
        evidence_kind="general_sem_rank0_exported_files_manifest",
        subject=f"{subject}.exported_files_manifest",
    )
    _validate_exported_files_manifest(
        exports, repository_root, f"{subject}.exported_files_manifest"
    )
    if (
        exports.get("run_id") != run["run_id"]
        or exports.get("document_id") != run["document_id"]
    ):
        raise ContractError(
            f"{subject}.exported_files_manifest is not bound to the exact run"
        )

    accessibility = _bound_evidence(
        paths["accessibility_snapshot"],
        package_kind=package_kind,
        variant_id=variant_id,
        evidence_kind="general_sem_rank0_accessibility_snapshot",
        subject=f"{subject}.accessibility_snapshot",
    )
    _exact_keys(
        accessibility,
        {
            "schema_version",
            "evidence_kind",
            "package_kind",
            "variant_id",
            "scales",
            "viewports",
            "cells",
            "keyboard_navigation",
            "accessible_table_and_chart",
            "passed",
        },
        f"{subject}.accessibility_snapshot",
    )
    cells = accessibility.get("cells")
    expected_cells = {
        (scale, viewport)
        for scale in (100, 125, 150, 200)
        for viewport in ("1024x700", "1280x720", "1440x900")
    }
    observed_cells = (
        {
            (cell.get("scale_percent"), cell.get("viewport"))
            for cell in cells
            if isinstance(cell, Mapping)
        }
        if isinstance(cells, list)
        else set()
    )
    if (
        accessibility.get("scales") != [100, 125, 150, 200]
        or accessibility.get("viewports") != ["1024x700", "1280x720", "1440x900"]
        or not isinstance(cells, list)
        or len(cells) != 12
    ):
        raise ContractError(f"{subject}.accessibility_snapshot matrix is incomplete")
    if observed_cells != expected_cells or any(
        not isinstance(cell, Mapping) or cell.get("passed") is not True
        for cell in cells
    ):
        raise ContractError(
            f"{subject}.accessibility_snapshot has a failed or duplicate cell"
        )
    for index, cell in enumerate(cells):
        row = _exact_keys(
            cell,
            {
                "scale_percent",
                "viewport",
                "origin",
                "tauri_runtime",
                "surface",
                "device_pixel_ratio",
                "actual_client_width",
                "actual_client_height",
                "no_horizontal_overflow",
                "table_count",
                "accessible_table_count",
                "chart_count",
                "accessible_chart_count",
                "keyboard_distinct_targets",
                "keyboard_reached_interactive_control",
                "passed",
            },
            f"{subject}.accessibility_snapshot.cells[{index}]",
        )
        scale = row.get("scale_percent")
        viewport = row.get("viewport")
        if not isinstance(scale, int) or not isinstance(viewport, str):
            raise ContractError(
                f"{subject}.accessibility_snapshot cell identity is invalid"
            )
        width, height = (int(value) for value in viewport.split("x"))
        dpr = row.get("device_pixel_ratio")
        if (
            row.get("origin") != PACKAGED_TAURI_ORIGIN
            or row.get("tauri_runtime") is not True
            or row.get("surface") != "model"
            or not isinstance(dpr, (int, float))
            or isinstance(dpr, bool)
            or abs(float(dpr) - scale / 100) > 0.08
            or row.get("actual_client_width") != width
            or row.get("actual_client_height") != height
            or row.get("no_horizontal_overflow") is not True
            or not isinstance(row.get("table_count"), int)
            or row["table_count"] <= 0
            or row.get("accessible_table_count") != row["table_count"]
            or not isinstance(row.get("chart_count"), int)
            or row["chart_count"] <= 0
            or row.get("accessible_chart_count") != row["chart_count"]
            or not isinstance(row.get("keyboard_distinct_targets"), int)
            or row["keyboard_distinct_targets"] < 4
            or row.get("keyboard_reached_interactive_control") is not True
            or row.get("passed") is not True
        ):
            raise ContractError(
                f"{subject}.accessibility_snapshot.cells[{index}] did not pass exact viewport/a11y checks"
            )
    if any(
        accessibility.get(field) is not True
        for field in ("keyboard_navigation", "accessible_table_and_chart", "passed")
    ):
        raise ContractError(f"{subject}.accessibility_snapshot did not pass")

    cleanup = _bound_evidence(
        paths["process_cleanup_trace"],
        package_kind=package_kind,
        variant_id=variant_id,
        evidence_kind="general_sem_rank0_process_cleanup_trace",
        subject=f"{subject}.process_cleanup_trace",
    )
    _exact_keys(
        cleanup,
        {
            "schema_version",
            "evidence_kind",
            "package_kind",
            "variant_id",
            "sessions",
            "orphan_process_ids",
            "temporary_or_partial_files",
            "passed",
        },
        f"{subject}.process_cleanup_trace",
    )
    sessions = cleanup.get("sessions")
    if (
        cleanup.get("passed") is not True
        or cleanup.get("orphan_process_ids") != []
        or cleanup.get("temporary_or_partial_files") != []
        or not isinstance(sessions, list)
        or len(sessions) != 5
    ):
        raise ContractError(f"{subject}.process_cleanup_trace is incomplete")
    expected_cleanup = (
        ("primary", "execute", 100),
        ("scale_100", "reopen", 100),
        ("scale_125", "reopen", 125),
        ("scale_150", "reopen", 150),
        ("scale_200", "reopen", 200),
    )
    for index, session in enumerate(sessions):
        row = _exact_keys(
            session,
            {
                "session_id",
                "phase",
                "scale_percent",
                "launched_pid",
                "launched_executable_path",
                "launched_executable_size",
                "launched_executable_sha256",
                "graceful_exit_confirmed",
                "forced_termination",
                "lingering_pids",
                "cdp_endpoint_closed",
                "passed",
            },
            f"{subject}.process_cleanup_trace.sessions[{index}]",
        )
        expected_session_id, expected_phase, expected_scale = expected_cleanup[index]
        if (
            row.get("session_id") != expected_session_id
            or row.get("phase") != expected_phase
            or row.get("scale_percent") != expected_scale
            or not isinstance(row.get("launched_pid"), int)
            or isinstance(row.get("launched_pid"), bool)
            or row["launched_pid"] <= 0
            or row.get("passed") is not True
            or row.get("graceful_exit_confirmed") is not True
            or row.get("forced_termination") is not False
            or row.get("lingering_pids") != []
            or row.get("cdp_endpoint_closed") is not True
            or row.get("launched_executable_path")
            != package_identity.get("resolved_path")
            or row.get("launched_executable_size") != package_identity.get("size")
            or row.get("launched_executable_sha256") != package_identity.get("sha256")
        ):
            raise ContractError(
                f"{subject}.process_cleanup_trace contains an unclean or mismatched session"
            )
    cleanup_pids = [session["launched_pid"] for session in sessions]
    if len(cleanup_pids) != len(set(cleanup_pids)):
        raise ContractError(
            f"{subject}.process_cleanup_trace reused a process identity"
        )

    reopen = _bound_evidence(
        paths["close_reopen_trace"],
        package_kind=package_kind,
        variant_id=variant_id,
        evidence_kind="general_sem_rank0_close_reopen_trace",
        subject=f"{subject}.close_reopen_trace",
    )
    _exact_keys(
        reopen,
        {
            "schema_version",
            "evidence_kind",
            "package_kind",
            "variant_id",
            "project_archive_sha256",
            "run_id",
            "document_id",
            "primary_pid",
            "reopen_sessions",
            "passed",
        },
        f"{subject}.close_reopen_trace",
    )
    reopen_sessions = reopen.get("reopen_sessions")
    if (
        reopen.get("passed") is not True
        or reopen.get("run_id") != run["run_id"]
        or reopen.get("document_id") != run["document_id"]
        or reopen.get("project_archive_sha256") != project_archive["sha256"]
        or reopen.get("primary_pid") != cleanup_pids[0]
        or not isinstance(reopen_sessions, list)
        or len(reopen_sessions) != 4
    ):
        raise ContractError(
            f"{subject}.close_reopen_trace did not prove exact fresh-process reopen"
        )
    for index, session in enumerate(reopen_sessions):
        row = _exact_keys(
            session,
            {
                "scale_percent",
                "process_id",
                "run_id",
                "document_id",
                "project_archive_sha256",
                "closed",
                "passed",
            },
            f"{subject}.close_reopen_trace.reopen_sessions[{index}]",
        )
        if (
            row.get("scale_percent") != (100, 125, 150, 200)[index]
            or row.get("process_id") != cleanup_pids[index + 1]
            or row.get("run_id") != run["run_id"]
            or row.get("document_id") != run["document_id"]
            or row.get("project_archive_sha256") != project_archive["sha256"]
            or row.get("closed") is not True
            or row.get("passed") is not True
        ):
            raise ContractError(
                f"{subject}.close_reopen_trace session {index} is not bound to the exact archive/process"
            )


def _validate_report_authority(
    report: Any, context: Mapping[str, Any], repository_root: Path
) -> tuple[
    Mapping[str, Any],
    str,
    str,
    dict[str, Mapping[str, Any]],
]:
    root = _exact_keys(
        report,
        {
            "schema_version",
            "report_kind",
            "contract_id",
            "contract_version",
            "contract_sha256",
            "build_fingerprint",
            "package_set_fingerprint",
            "package_identities",
            "hardware_fingerprint",
            "source_receipt",
            "qualification_contracts",
            "generated_at_utc",
            "results",
        },
        "report",
    )
    if root.get("schema_version") != 1:
        raise ContractError("report schema_version must equal 1")
    if root.get("report_kind") != "quickpls_general_sem_rank0_packaged_acceptance":
        raise ContractError("report_kind is invalid")
    for field in ("contract_id", "contract_version", "contract_sha256"):
        if root.get(field) != context[field]:
            raise ContractError(f"report {field} does not bind the contract")
    fingerprint = root.get("build_fingerprint")
    if not isinstance(fingerprint, str) or not SHA256.fullmatch(fingerprint):
        raise ContractError("build_fingerprint must be lowercase SHA-256")
    package_identities = _validate_package_identities(root.get("package_identities"))
    package_set = root.get("package_set_fingerprint")
    if not isinstance(package_set, str) or not SHA256.fullmatch(package_set):
        raise ContractError("package_set_fingerprint must be lowercase SHA-256")
    if _package_fingerprint(list(package_identities.values())) != package_set:
        raise ContractError(
            "package_set_fingerprint does not reproduce package identities"
        )
    if any(row["sha256"] != fingerprint for row in package_identities.values()):
        raise ContractError(
            "build_fingerprint must equal both installed and portable executable SHA-256 values"
        )
    _validate_hardware_fingerprint(root.get("hardware_fingerprint"))
    try:
        validate_unified_rank0_source_receipt(
            root.get("source_receipt"),
            repository_root,
            subject="report.source_receipt",
        )
    except ValueError as error:
        raise ContractError(str(error)) from error
    if (
        not isinstance(root.get("generated_at_utc"), str)
        or not root["generated_at_utc"]
    ):
        raise ContractError("generated_at_utc is invalid")
    return root, fingerprint, package_set, package_identities


def _validate_result_row(
    value: Any,
    *,
    subject: str,
    package: str,
    variant: Mapping[str, Any],
    package_identity: Mapping[str, Any],
    repository_root: Path,
    require_standard: bool,
) -> None:
    result = _exact_keys(
        value,
        {
            "package_kind",
            "variant_id",
            "capability_reference",
            "offline",
            "fresh_process_reopen",
            "checks",
            "artifacts",
        },
        subject,
    )
    if (
        result.get("package_kind") != package
        or result.get("variant_id") != variant["variant_id"]
    ):
        raise ContractError(f"{subject} package/variant identity does not match")
    if (
        _reference(
            result.get("capability_reference"), f"{subject}.capability_reference"
        )
        != variant["reference"]
    ):
        raise ContractError(f"{subject} capability reference does not match")
    if (
        result.get("offline") is not True
        or result.get("fresh_process_reopen") is not True
    ):
        raise ContractError(f"{subject} did not prove offline fresh-process use")
    checks = result.get("checks")
    required = variant["required_checks"]
    if not isinstance(checks, Mapping) or tuple(checks) != required:
        raise ContractError(f"{subject} check IDs/order are not exact")
    if any(check is not True for check in checks.values()):
        raise ContractError(f"{subject} contains a failed required check")
    artifacts = result.get("artifacts")
    if not isinstance(artifacts, list) or len(artifacts) < len(REQUIRED_ARTIFACT_KINDS):
        raise ContractError(f"{subject} must bind evidence artifacts")
    artifact_kinds = [
        artifact.get("kind") for artifact in artifacts if isinstance(artifact, Mapping)
    ]
    if any(artifact_kinds.count(kind) != 1 for kind in REQUIRED_ARTIFACT_KINDS):
        raise ContractError(
            f"{subject} must bind every distinct required evidence role exactly once"
        )
    artifact_paths = [
        artifact.get("path") for artifact in artifacts if isinstance(artifact, Mapping)
    ]
    if len(artifact_paths) != len(set(artifact_paths)):
        raise ContractError(
            f"{subject} evidence roles must use distinct artifact paths"
        )
    resolved_artifacts: dict[str, Path] = {}
    for artifact_index, artifact in enumerate(artifacts):
        artifact_path = _validate_artifact(
            artifact,
            repository_root,
            f"{subject}.artifacts[{artifact_index}]",
        )
        resolved_artifacts[artifact["kind"]] = artifact_path
    _validate_role_evidence(
        resolved_artifacts,
        package_kind=package,
        variant=variant,
        package_identity=package_identity,
        repository_root=repository_root,
        subject=subject,
    )
    if require_standard and (
        variant["registry_cell"].get("surface") != "standard"
        or variant["registry_cell"].get("evidence_state") != "release_qualified"
    ):
        raise ContractError(
            f"{variant['variant_id']} is not release-qualified Standard in Registry V2"
        )


def validate_report(
    report: Any,
    context: Mapping[str, Any],
    repository_root: Path,
    *,
    require_standard: bool,
) -> dict[str, Any]:
    root, fingerprint, package_set, package_identities = _validate_report_authority(
        report, context, repository_root
    )
    _validate_qualification_contracts(
        root.get("qualification_contracts"), context, repository_root
    )
    results = root.get("results")
    expected = {
        (package, variant["variant_id"]): variant
        for package in context["packages"]
        for variant in context["variants"]
    }
    if not isinstance(results, list) or len(results) != len(expected):
        raise ContractError("report must contain the exact package/variant matrix")
    observed: set[tuple[str, str]] = set()
    for index, value in enumerate(results):
        result = value if isinstance(value, Mapping) else {}
        key = (result.get("package_kind"), result.get("variant_id"))
        if key not in expected or key in observed:
            raise ContractError(
                f"results[{index}] has an unexpected or duplicate identity"
            )
        observed.add(key)
        variant = expected[key]
        _validate_result_row(
            value,
            subject=f"results[{index}]",
            package=str(key[0]),
            variant=variant,
            package_identity=package_identities[str(key[0])],
            repository_root=repository_root,
            require_standard=require_standard,
        )
    return {
        "passed": True,
        "result_count": len(results),
        "package_count": len(context["packages"]),
        "variant_count": len(context["variants"]),
        "build_fingerprint": fingerprint,
        "package_set_fingerprint": package_set,
    }


def validate_cell_report(
    report: Any,
    context: Mapping[str, Any],
    repository_root: Path,
    *,
    capability_reference: Mapping[str, Any],
    require_standard: bool,
) -> dict[str, Any]:
    """Validate only one cell's exact installed+portable evidence atomically."""

    root, fingerprint, package_set, package_identities = _validate_report_authority(
        report, context, repository_root
    )
    reference = _reference(capability_reference, "capability_reference")
    variants = [
        variant for variant in context["variants"] if variant["reference"] == reference
    ]
    if len(variants) != 1:
        raise ContractError(
            "capability_reference is not one exact Rank 0 packaged cell"
        )
    variant = variants[0]
    canonical_reference = {
        "registry_schema_version": reference[0],
        "capability_id": reference[1],
        "cell_id": reference[2],
        "capability_version": reference[3],
    }
    results = root.get("results")
    if not isinstance(results, list):
        raise ContractError("report results must be an array")
    selected = [
        (index, row)
        for index, row in enumerate(results)
        if isinstance(row, Mapping)
        and row.get("capability_reference") == canonical_reference
    ]
    if len(selected) != len(context["packages"]):
        raise ContractError(
            "cell report must contain exact installed and portable target rows"
        )
    observed_packages: set[str] = set()
    for index, row in selected:
        package = row.get("package_kind")
        if package not in context["packages"] or package in observed_packages:
            raise ContractError("cell report has a duplicate or unexpected package")
        observed_packages.add(str(package))
        _validate_result_row(
            row,
            subject=f"results[{index}]",
            package=str(package),
            variant=variant,
            package_identity=package_identities[str(package)],
            repository_root=repository_root,
            require_standard=require_standard,
        )
    if observed_packages != set(context["packages"]):
        raise ContractError("cell report does not cover installed and portable")
    expected_contracts = [
        row
        for row in qualification_contract_authorities(context, repository_root)
        if row["capability_reference"] == canonical_reference
    ]
    observed_contracts = (
        [
            row
            for row in root.get("qualification_contracts", [])
            if isinstance(root.get("qualification_contracts"), list)
            and isinstance(row, Mapping)
            and row.get("capability_reference") == canonical_reference
        ]
        if isinstance(root.get("qualification_contracts"), list)
        else []
    )
    if len(expected_contracts) != 1 or observed_contracts != expected_contracts:
        raise ContractError("normalized qualification contract is unavailable for cell")
    contract = expected_contracts[0]
    return {
        "passed": True,
        "result_count": len(selected),
        "package_count": len(observed_packages),
        "variant_count": 1,
        "build_fingerprint": fingerprint,
        "package_set_fingerprint": package_set,
        "source_set_sha256": root["source_receipt"]["source_set_sha256"],
        "qualification_contract_sha256": contract["qualification_contract_sha256"],
    }


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    parser.add_argument("--registry", type=Path, default=DEFAULT_REGISTRY)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--repository-root", type=Path, default=ROOT)
    parser.add_argument("--require-standard", action="store_true")
    args = parser.parse_args(argv)
    try:
        contract = load_json(args.contract)
        registry = load_json(args.registry)
        context = validate_contract(contract, registry)
        result: dict[str, Any] = {
            "passed": True,
            "contract_id": context["contract_id"],
            "contract_version": context["contract_version"],
            "contract_sha256": context["contract_sha256"],
            "package_count": len(context["packages"]),
            "variant_count": len(context["variants"]),
            "expected_result_count": len(context["packages"])
            * len(context["variants"]),
        }
        if args.report is not None:
            result["report"] = validate_report(
                load_json(args.report),
                context,
                args.repository_root.resolve(),
                require_standard=args.require_standard,
            )
    except (ContractError, OSError, UnicodeError) as error:
        result = {"passed": False, "errors": [str(error)]}
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
