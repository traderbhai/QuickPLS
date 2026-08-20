#!/usr/bin/env python3
"""Strict, opt-in admission for Rank 0 QualificationSpec receipt payloads.

QualificationSpec V2 historically treats receipt bytes as opaque immutable
artifacts.  This module deliberately does not change that legacy rule.  It is
invoked only when a spec selects the exact Rank 0 payload contract, and then
fails closed unless the JSON envelope and the role-specific evidence both
reconcile to current repository bytes and the frozen cell identity.
"""

from __future__ import annotations

import hashlib
import json
import math
from pathlib import Path, PurePosixPath
from typing import Any, Mapping

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[1]
CONTRACT_ID = "quickpls.general_sem.rank0.receipt_payload.v1"
SCHEMA_VERSION = 1
PAYLOAD_KIND = "general_sem_rank0_qualification_receipt_payload_v1"
STREAMLINED_PROFILE_ID = "rank0_streamlined_plan4b_v1"
STREAMLINED_RELEASE_EVIDENCE_KIND = "general_sem_rank0_streamlined_release_evidence_v1"
STREAMLINED_PRODUCT_SOURCE_SET_SHA256 = (
    "df0447b3950edc60257bb31dd1358ebe47b99dbb6709b88ba094ef0ac7f03373"
)
STREAMLINED_PRODUCT_EVIDENCE_CHANGED_PATHS = frozenset(
    {
        "validation/general_sem_rank0_qualification_runner.py",
        "validation/general_sem_rank0_receipt_payload_v1.py",
        "validation/general_sem_rank0_streamlined_release.py",
        "validation/qualification_v2/general_sem_rank0_receipt_payload_v1.schema.json",
        "validation/test_general_sem_rank0_receipt_payload_v1.py",
    }
)
SCHEMA_PATH = (
    ROOT
    / "validation"
    / "qualification_v2"
    / "general_sem_rank0_receipt_payload_v1.schema.json"
)
CONTRACT_DESCRIPTOR = {
    "contract_id": CONTRACT_ID,
    "schema_version": SCHEMA_VERSION,
    "schema_path": (
        "validation/qualification_v2/general_sem_rank0_receipt_payload_v1.schema.json"
    ),
    "validator_path": "validation/general_sem_rank0_receipt_payload_v1.py",
}

ROLE_STAGE = {
    "method_contract": "contract",
    "kernel_execution": "kernel",
    "oracle_independence": "oracle",
    "generative_recovery": "generative",
    "adversarial_boundaries": "adversarial",
    "archive_persistence": "persistence_export",
    "cross_format_export": "persistence_export",
    "frontend_contract": "persistence_export",
    "packaged_windows_e2e": "packaged_windows",
    "performance_scale": "scale_reliability",
}

RANK0_REQUIRED_ROLES = frozenset(ROLE_STAGE)

# One source authority is shared by every Rank 0 receipt.  The patterns are
# deliberately limited to production Rust/TypeScript, build authority, and the
# Rank 0 qualification/packaging contracts.  Mutable QualificationSpec JSON
# documents and generated evidence are excluded: their immutable contract is
# bound separately by ``qualification_contract_sha256``.
UNIFIED_SOURCE_SCOPE = "quickpls_general_sem_rank0_unified_sources_v2"
UNIFIED_SOURCE_REQUIRED_PATHS = (
    "Cargo.lock",
    "Cargo.toml",
    "index.html",
    "package-lock.json",
    "package.json",
    "tsconfig.app.json",
    "tsconfig.build.json",
    "tsconfig.json",
    "tsconfig.node.json",
    "vite.config.ts",
    "src-tauri/Cargo.toml",
    "src-tauri/build.rs",
    "src-tauri/tauri.conf.json",
    "docs/methods/PLS_MEDIATION_V1.md",
    "docs/methods/GENERAL_SEM_PLS_MULTIPLE_MEDIATION_BOOTSTRAP_V1.md",
    "docs/methods/GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_V1.md",
    "docs/methods/GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_V1.md",
    "validation/capability_registry_v2.py",
    "validation/complexity_performance_measure.py",
    "validation/complexity_performance_v2.py",
    "validation/method_promotion_manifest.py",
    "validation/packaged_windows_acceptance_v2.py",
    "validation/qualification_spec_v2.py",
    "validation/test_capability_registry_v2.py",
    "validation/test_complexity_performance_v2.py",
    "validation/test_method_promotion_manifest.py",
    "validation/test_packaged_windows_acceptance_v2.py",
    "validation/test_qualification_spec_v2.py",
    "validation/v247_cdp_package_helpers.mjs",
    "validation/v247_tauri_native_acceptance.mjs",
    "validation/windows_native_owned_file_dialog.py",
    "validation/mediation_reference.py",
    "validation/general_sem_pls_multiple_mediation_bootstrap_v1_reference.py",
    "validation/general_sem_pls_multiple_moderation_point_v1_reference.py",
    "validation/general_sem_pls_multiple_moderation_bootstrap_v1_reference.py",
    "validation/methods/method_promotion_manifest.schema.json",
    "validation/qualification_v2/general_sem_rank0_receipt_payload_v1.schema.json",
    "validation/qualification_v2/qualification_spec_v2.schema.json",
    "validation/capabilities/capability_registry_v2.json",
    "validation/capabilities/capability_registry_v2.schema.json",
    "validation/capabilities/complexity_performance_measurement_v2.schema.json",
    "validation/capabilities/complexity_performance_profiles_v2.manifest.json",
    "validation/capabilities/complexity_performance_profiles_v2.schema.json",
    "validation/capabilities/general_sem_rank0_packaged_acceptance_v1.manifest.json",
    "validation/capabilities/packaged_windows_acceptance_v2.manifest.json",
)
UNIFIED_SOURCE_GLOBS = (
    "crates/*/Cargo.toml",
    "crates/*/examples/**/*.rs",
    "crates/*/src/**/*.rs",
    "src/**/*.css",
    "src/**/*.ts",
    "src/**/*.tsx",
    "src-tauri/capabilities/**/*.json",
    "src-tauri/src/**/*.rs",
    "validation/general_sem_rank0_*.R",
    "validation/general_sem_rank0_*.mjs",
    "validation/general_sem_rank0_*.py",
    "validation/run_general_sem_rank0_*.ps1",
    "validation/test_general_sem_rank0_*.py",
    "validation/capabilities/general_sem_pls_multiple_moderation_point_v1.cell.manifest.json",
    "validation/capabilities/general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1.cell.manifest.json",
)

ROLE_EVIDENCE_KIND = {
    "method_contract": "general_sem_rank0_method_audit_v1",
    "kernel_execution": "general_sem_rank0_scientific_product_v1",
    "oracle_independence": "general_sem_rank0_scientific_product_v1",
    "generative_recovery": "general_sem_rank0_scientific_product_v1",
    "adversarial_boundaries": "general_sem_rank0_scientific_product_v1",
    "archive_persistence": "general_sem_rank0_archive_evidence_v1",
    "cross_format_export": "general_sem_rank0_export_frontend_evidence_v1",
    "frontend_contract": "general_sem_rank0_export_frontend_evidence_v1",
    "packaged_windows_e2e": "general_sem_rank0_packaged_windows_evidence_v1",
    "performance_scale": "general_sem_rank0_performance_evidence_v1",
}

SCIENTIFIC_SUITES = {
    "kernel_execution": {
        "point": frozenset({"pairwise", "current_product_comparison"}),
        "bootstrap": frozenset({"pairwise", "current_product_comparison"}),
    },
    "oracle_independence": {
        "point": frozenset(
            {"independent_oracle_comparison", "current_product_comparison"}
        ),
        "bootstrap": frozenset(
            {"independent_oracle_comparison", "current_product_comparison"}
        ),
    },
    "generative_recovery": {
        "point": frozenset({"recovery", "current_product_comparison"}),
        "bootstrap": frozenset(
            {
                "recovery",
                "coverage",
                "null_calibration",
                "current_product_comparison",
            }
        ),
    },
    "adversarial_boundaries": {
        "point": frozenset(
            {
                "failure_classification",
                "metamorphic_invariance",
                "current_product_comparison",
            }
        ),
        "bootstrap": frozenset(
            {
                "failure_classification",
                "worker_replay",
                "seed_replay",
                "metamorphic_invariance",
                "current_product_comparison",
            }
        ),
    },
}

# The streamlined profile keeps the strongest already-complete suites and
# replaces two historically mislabelled oracle checks with one compact,
# source-bound correction run.  The correction artifact is validated below;
# it is not a blanket waiver for failed scientific evidence.
STREAMLINED_SCIENTIFIC_SUITES = {
    "kernel_execution": {
        "point": frozenset({"current_product_comparison"}),
        "bootstrap": frozenset({"current_product_comparison"}),
    },
    "oracle_independence": {
        "point": frozenset(
            {"independent_oracle_comparison", "current_product_comparison"}
        ),
        "bootstrap": frozenset(
            {"independent_oracle_comparison", "current_product_comparison"}
        ),
    },
    "generative_recovery": {
        "point": frozenset(
            {"independent_oracle_comparison", "current_product_comparison"}
        ),
        "bootstrap": frozenset(
            {"coverage", "null_calibration", "current_product_comparison"}
        ),
    },
    "adversarial_boundaries": {
        "point": frozenset({"metamorphic_invariance", "current_product_comparison"}),
        "bootstrap": frozenset(
            {
                "metamorphic_invariance",
                "worker_replay",
                "seed_replay",
                "current_product_comparison",
            }
        ),
    },
}

BOOTSTRAP_CELL_IDS = frozenset(
    {
        "qpls3.pls.general_sem_multiple_mediation_bootstrap",
        "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
    }
)

# Capability identity and runtime analytical identity are distinct authorities.
# In particular, the moderation-bootstrap strings intentionally differ.
ANALYTICAL_METHOD_BY_CELL = {
    (
        "qpls3.pls.mediation",
        "pls_mediation_v1",
    ): "pls_mediation_v1",
    (
        "qpls3.pls.general_sem_multiple_mediation_bootstrap",
        "general_sem_pls_full_model_case_bootstrap_v1",
    ): "general_sem_pls_full_model_case_bootstrap_v1",
    (
        "qpls3.pls.general_sem_multiple_two_way_moderation_point",
        "general_sem_pls_multiple_two_way_moderation_point_v1",
    ): "qpls.general-sem-pls.multiple-two-way.point.v1",
    (
        "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
        "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
    ): "qpls.general-sem-pls.multiple-two-way.full-model-case-bootstrap.v1",
}


class ReceiptPayloadError(ValueError):
    """A strict Rank 0 receipt cannot be admitted."""


def _strict_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ReceiptPayloadError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_constant(token: str) -> None:
    raise ReceiptPayloadError(f"non-finite JSON value: {token}")


def strict_load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8-sig") as handle:
        return json.load(
            handle,
            object_pairs_hook=_strict_pairs,
            parse_constant=_reject_constant,
        )


def canonical_sha256(value: Any) -> str:
    payload = json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _is_sha256(value: Any) -> bool:
    return bool(
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def qualification_contract_projection(
    specification: Mapping[str, Any],
) -> dict[str, Any]:
    """Return the immutable, non-circular QualificationSpec contract."""

    evidence = specification["evidence_contract"]
    return {
        "schema_version": specification["schema_version"],
        "identity": specification["identity"],
        "scientific_contract": specification["scientific_contract"],
        "scenario_contract": specification["scenario_contract"],
        "comparison_contract": specification["comparison_contract"],
        "operational_contract": specification["operational_contract"],
        "evidence_contract": {
            "required_roles": evidence["required_roles"],
            "receipt_contract": evidence["receipt_contract"],
        },
    }


def qualification_contract_sha256(specification: Mapping[str, Any]) -> str:
    return canonical_sha256(qualification_contract_projection(specification))


def method_manifest_contract_projection(manifest: Mapping[str, Any]) -> dict[str, Any]:
    """Exclude only evidence-populated method-manifest state."""

    qualification = manifest["qualification"]
    return {
        "schema_version": manifest["schema_version"],
        "governance": manifest["governance"],
        "feature": manifest["feature"],
        "claim": manifest["claim"],
        "scientific_contract": manifest["scientific_contract"],
        "product_contract": manifest["product_contract"],
        "qualification_contract": {
            "target_state": qualification["target_state"],
            "source_requirements": qualification["source_requirements"],
        },
    }


def method_manifest_contract_sha256(manifest: Mapping[str, Any]) -> str:
    return canonical_sha256(method_manifest_contract_projection(manifest))


def _path_has_symlink_component(root: Path, path: Path) -> bool:
    relative = path.relative_to(root)
    cursor = root
    for part in relative.parts:
        cursor = cursor / part
        if cursor.is_symlink():
            return True
    return False


def unified_rank0_source_descriptors(
    repository_root: Path = ROOT,
) -> list[dict[str, Any]]:
    """Recompute the exact shared Rank 0 source inventory from current bytes."""

    root = repository_root.resolve()
    candidates: set[Path] = set()
    for relative in UNIFIED_SOURCE_REQUIRED_PATHS:
        candidate = root / Path(*PurePosixPath(relative).parts)
        if not candidate.is_file():
            raise ReceiptPayloadError(f"required Rank 0 source is missing: {relative}")
        candidates.add(candidate)
    for pattern in UNIFIED_SOURCE_GLOBS:
        matches = [path for path in root.glob(pattern) if path.is_file()]
        if not matches:
            raise ReceiptPayloadError(
                f"Rank 0 source inventory pattern matched no files: {pattern}"
            )
        candidates.update(matches)

    descriptors: list[dict[str, Any]] = []
    for unresolved in candidates:
        if _path_has_symlink_component(root, unresolved):
            relative = unresolved.relative_to(root).as_posix()
            raise ReceiptPayloadError(
                f"Rank 0 source inventory contains a symlink: {relative}"
            )
        path = unresolved.resolve()
        try:
            relative = path.relative_to(root).as_posix()
        except ValueError as error:
            raise ReceiptPayloadError(
                "Rank 0 source inventory leaves the repository"
            ) from error
        size, digest = _sha256(path)
        descriptors.append({"path": relative, "size": size, "sha256": digest})
    descriptors.sort(key=lambda row: str(row["path"]).encode("utf-8"))
    if len({str(row["path"]).casefold() for row in descriptors}) != len(descriptors):
        raise ReceiptPayloadError(
            "Rank 0 source inventory contains case-insensitive path aliases"
        )
    return descriptors


def unified_rank0_source_receipt(
    repository_root: Path = ROOT,
) -> dict[str, Any]:
    files = unified_rank0_source_descriptors(repository_root)
    return {
        "scope": UNIFIED_SOURCE_SCOPE,
        "file_count": len(files),
        "files": files,
        "source_set_sha256": canonical_sha256(files),
    }


def validate_unified_rank0_source_receipt(
    value: Any,
    repository_root: Path = ROOT,
    *,
    subject: str = "source_receipt",
) -> dict[str, Any]:
    expected = unified_rank0_source_receipt(repository_root)
    if value != expected:
        raise ReceiptPayloadError(
            f"{subject} does not equal the exact current Rank 0 source inventory"
        )
    return expected


def _sha256(path: Path) -> tuple[int, str]:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as handle:
        while chunk := handle.read(1024 * 1024):
            size += len(chunk)
            digest.update(chunk)
    return size, digest.hexdigest()


def _safe_path(root: Path, relative: object, subject: str) -> Path:
    if not isinstance(relative, str):
        raise ReceiptPayloadError(f"{subject}.path is not text")
    pure = PurePosixPath(relative)
    if not relative or pure.is_absolute() or ".." in pure.parts or "\\" in relative:
        raise ReceiptPayloadError(f"{subject}.path is unsafe")
    unresolved = root / Path(*pure.parts)
    if _path_has_symlink_component(root.resolve(), unresolved):
        raise ReceiptPayloadError(
            f"{subject}.path must not contain a symlink component"
        )
    candidate = unresolved.resolve()
    try:
        candidate.relative_to(root.resolve())
    except ValueError as error:
        raise ReceiptPayloadError(f"{subject}.path leaves the repository") from error
    if not candidate.is_file():
        raise ReceiptPayloadError(f"{subject}.path is missing: {relative}")
    return candidate


def _artifact_path(
    descriptor: Any,
    root: Path,
    subject: str,
    *,
    relative_base: Path | None = None,
) -> Path:
    if not isinstance(descriptor, Mapping) or set(descriptor) != {
        "path",
        "size",
        "sha256",
    }:
        raise ReceiptPayloadError(f"{subject} descriptor fields are not exact")
    base = root if relative_base is None else relative_base
    raw = descriptor.get("path")
    if relative_base is None:
        path = _safe_path(root, raw, subject)
    else:
        if not isinstance(raw, str) or not raw or "\\" in raw:
            raise ReceiptPayloadError(f"{subject}.path is unsafe")
        pure = PurePosixPath(raw)
        if pure.is_absolute() or ".." in pure.parts:
            raise ReceiptPayloadError(f"{subject}.path is unsafe")
        unresolved = base / Path(*pure.parts)
        if unresolved.is_symlink():
            raise ReceiptPayloadError(f"{subject}.path must not be a symlink")
        path = unresolved.resolve()
        try:
            path.relative_to(root.resolve())
        except ValueError as error:
            raise ReceiptPayloadError(
                f"{subject}.path leaves the repository"
            ) from error
        if not path.is_file():
            raise ReceiptPayloadError(f"{subject}.path is missing: {raw}")
    size, digest = _sha256(path)
    if descriptor.get("size") != size:
        raise ReceiptPayloadError(f"{subject}.size does not match current bytes")
    if descriptor.get("sha256") != digest:
        raise ReceiptPayloadError(f"{subject}.sha256 does not match current bytes")
    return path


def _artifact_json(
    descriptor: Any,
    root: Path,
    subject: str,
    *,
    relative_base: Path | None = None,
) -> tuple[Mapping[str, Any], Path]:
    path = _artifact_path(descriptor, root, subject, relative_base=relative_base)
    value = strict_load_json(path)
    if not isinstance(value, Mapping):
        raise ReceiptPayloadError(f"{subject} JSON root must be an object")
    return value, path


def _validate_sources(payload: Mapping[str, Any], root: Path) -> None:
    descriptors = payload["source_descriptors"]
    prior: bytes | None = None
    seen: set[str] = set()
    for index, descriptor in enumerate(descriptors):
        subject = f"source_descriptors[{index}]"
        path = _artifact_path(descriptor, root, subject)
        relative = path.relative_to(root.resolve()).as_posix()
        if relative != descriptor["path"]:
            raise ReceiptPayloadError(f"{subject}.path is not canonical")
        encoded = relative.encode("utf-8")
        if prior is not None and encoded <= prior:
            raise ReceiptPayloadError(
                "source_descriptors must be unique and UTF-8 path sorted"
            )
        if relative in seen:
            raise ReceiptPayloadError("source descriptor path is duplicated")
        prior = encoded
        seen.add(relative)
    if payload["source_set_sha256"] != canonical_sha256(descriptors):
        raise ReceiptPayloadError(
            "source_set_sha256 does not bind the recomputed source descriptors"
        )
    expected = unified_rank0_source_receipt(root)
    if descriptors != expected["files"]:
        raise ReceiptPayloadError(
            "source_descriptors do not equal the exact unified Rank 0 inventory"
        )
    if payload["source_set_sha256"] != expected["source_set_sha256"]:
        raise ReceiptPayloadError(
            "source_set_sha256 differs from the unified Rank 0 source receipt"
        )


def _validate_method_audit(
    evidence: Mapping[str, Any],
    payload: Mapping[str, Any],
    root: Path,
) -> None:
    manifest, manifest_path = _artifact_json(
        evidence["manifest"], root, "evidence.manifest"
    )
    try:
        from validation.method_promotion_manifest import validate_manifest
    except ModuleNotFoundError:
        from method_promotion_manifest import validate_manifest

    report = validate_manifest(manifest_path, root, verify_evidence=True)
    expected = payload["capability_cell"]
    if (
        report.get("passed") is not True
        or report.get("feature_id") != expected["cell_id"]
        or report.get("method_version") != expected["capability_version"]
        or report.get("declared_state") != "release_qualified"
        or report.get("derived_state") != "release_qualified"
        or report.get("target_state") != "release_qualified"
    ):
        raise ReceiptPayloadError(
            "method audit does not derive release_qualified for the exact cell"
        )
    if evidence.get("manifest_contract_sha256") != method_manifest_contract_sha256(
        manifest
    ):
        raise ReceiptPayloadError(
            "method audit does not bind the immutable method-manifest contract"
        )


def _plan_index(rows: Any, key: str, subject: str) -> dict[str, Mapping[str, Any]]:
    if not isinstance(rows, list):
        raise ReceiptPayloadError(f"{subject} must be an array")
    result: dict[str, Mapping[str, Any]] = {}
    for row in rows:
        if not isinstance(row, Mapping) or not isinstance(row.get(key), str):
            raise ReceiptPayloadError(f"{subject} contains an invalid identity")
        identity = str(row[key])
        if identity in result:
            raise ReceiptPayloadError(f"{subject} contains duplicate {key}")
        result[identity] = row
    return result


def _validate_product_observation(
    observation: Any,
    payload: Mapping[str, Any],
    product_source_set_sha256: str,
    root: Path,
    *,
    streamlined: bool = False,
) -> None:
    if not isinstance(observation, Mapping):
        raise ReceiptPayloadError("current-product shard has no first observation")
    expected = payload["capability_cell"]
    producer_executable_sha256 = observation.get("producer_executable_sha256")
    execution_receipt = observation.get("execution_receipt")
    executable_descriptor = observation.get("executable_descriptor")
    if (
        observation.get("cell_id") != expected["cell_id"]
        or observation.get("method_version") != expected["capability_version"]
        or observation.get("difference_count") != 0
        or observation.get("difference_witnesses") != []
        or not _is_sha256(producer_executable_sha256)
        or producer_executable_sha256 == payload["build_fingerprint"]
        or not isinstance(execution_receipt, Mapping)
        or execution_receipt.get("producer_executable_sha256")
        != producer_executable_sha256
        or not isinstance(executable_descriptor, Mapping)
        or executable_descriptor.get("sha256") != producer_executable_sha256
        or execution_receipt.get("executable_descriptor") != executable_descriptor
    ):
        raise ReceiptPayloadError(
            "current-product observation does not keep producer and app identities distinct"
        )
    analytical = payload["analytical_method_version"]
    source_receipt = (
        execution_receipt.get("source_receipt")
        if isinstance(execution_receipt, Mapping)
        else None
    )
    if streamlined:
        if not isinstance(source_receipt, Mapping):
            raise ReceiptPayloadError(
                "current-product embedded source_receipt is absent"
            )
        files = source_receipt.get("files")
        current_sources = unified_rank0_source_receipt(root)
        if (
            source_receipt.get("scope")
            != "quickpls_general_sem_rank0_unified_sources_v2"
            or not isinstance(files, list)
            or source_receipt.get("file_count") != len(files)
            or source_receipt.get("source_set_sha256") != canonical_sha256(files)
            or source_receipt.get("source_set_sha256")
            != STREAMLINED_PRODUCT_SOURCE_SET_SHA256
        ):
            raise ReceiptPayloadError(
                "streamlined current-product source receipt is invalid"
            )
        historical = _plan_index(files, "path", "current-product source files")
        current = _plan_index(
            current_sources["files"], "path", "current Rank 0 source files"
        )
        if set(historical) != set(current):
            raise ReceiptPayloadError(
                "streamlined current-product source inventory paths differ"
            )
        changed = {path for path in current if historical[path] != current[path]}
        if changed != STREAMLINED_PRODUCT_EVIDENCE_CHANGED_PATHS:
            raise ReceiptPayloadError(
                "files changed after current-product comparison are not the exact evidence-only correction set"
            )
        expected_sources = current_sources
    else:
        expected_sources = validate_unified_rank0_source_receipt(
            source_receipt,
            root,
            subject="current-product embedded source_receipt",
        )
    if (
        observation.get("product_source_set_sha256") != product_source_set_sha256
        or source_receipt.get("source_set_sha256") != product_source_set_sha256
        or expected_sources["files"] != payload["source_descriptors"]
        or expected_sources["source_set_sha256"] != payload["source_set_sha256"]
        or (
            not streamlined
            and product_source_set_sha256 != payload["source_set_sha256"]
        )
    ):
        raise ReceiptPayloadError(
            "current-product execution does not bind the receipt source inventory"
        )
    bootstrap_receipts: list[Mapping[str, Any]] = []
    workers = observation.get("worker_comparisons")
    if not isinstance(workers, list) or not workers:
        raise ReceiptPayloadError("current-product worker comparison ledger is absent")
    for row in workers:
        receipts = row.get("production_receipts") if isinstance(row, Mapping) else None
        bootstrap = receipts.get("bootstrap") if isinstance(receipts, Mapping) else None
        if isinstance(bootstrap, Mapping):
            bootstrap_receipts.append(bootstrap)
    if expected["cell_id"] in BOOTSTRAP_CELL_IDS:
        if not bootstrap_receipts or any(
            row.get("supplemental_method_version") != analytical
            for row in bootstrap_receipts
        ):
            raise ReceiptPayloadError(
                "current-product bootstrap ledger does not bind the analytical method"
            )
    elif bootstrap_receipts:
        raise ReceiptPayloadError(
            "point evidence unexpectedly carries bootstrap authority"
        )


def _validate_scientific_product(
    evidence: Mapping[str, Any],
    payload: Mapping[str, Any],
    root: Path,
    *,
    streamlined: bool = False,
) -> None:
    plan, _ = _artifact_json(evidence["plan"], root, "evidence.plan")
    aggregate, _ = _artifact_json(evidence["aggregate"], root, "evidence.aggregate")
    try:
        from validation import general_sem_rank0_qualification_runner as runner
    except ImportError:
        import general_sem_rank0_qualification_runner as runner

    continuation_descriptor = evidence.get("continuation_policy")
    continuation_policy: Mapping[str, Any] | None = None
    if continuation_descriptor is None:
        runner.validate_frozen_full_aggregate(aggregate, plan)
    else:
        continuation_policy, continuation_path = _artifact_json(
            continuation_descriptor,
            root,
            "evidence.continuation_policy",
        )
        runner.validate_plan4b_policy(
            continuation_policy,
            plan,
            output_root=None if streamlined else continuation_path.parent,
        )
        if streamlined:
            runner.validate_aggregate(
                aggregate,
                plan,
                plan4b_policy=continuation_policy,
                accepted_historical_product_source_sha256=(
                    STREAMLINED_PRODUCT_SOURCE_SET_SHA256
                ),
            )
            shard_by_id = _plan_index(plan.get("shards"), "shard_id", "plan.shards")
            scenario_by_id = _plan_index(
                plan.get("scenarios"), "scenario_id", "plan.scenarios"
            )
            missing = aggregate.get("missing_shard_ids")
            if not isinstance(missing, list):
                raise ReceiptPayloadError(
                    "streamlined Plan 4B missing-shard ledger is invalid"
                )
            missing_suites = {
                scenario_by_id[str(shard_by_id[str(shard_id)]["scenario_id"])]["suite"]
                for shard_id in missing
            }
            if missing_suites - {"maximum_axis", "compound_stress"}:
                raise ReceiptPayloadError(
                    "streamlined Plan 4B omits a non-performance scientific shard"
                )
        else:
            runner.validate_frozen_plan4b_aggregate(
                aggregate, plan, continuation_policy
            )
    expected = payload["capability_cell"]
    cell_kind = "bootstrap" if expected["cell_id"] in BOOTSTRAP_CELL_IDS else "point"
    suite_contract = STREAMLINED_SCIENTIFIC_SUITES if streamlined else SCIENTIFIC_SUITES
    required_suites = suite_contract[payload["role"]][cell_kind]
    scenarios = _plan_index(plan.get("scenarios"), "scenario_id", "plan.scenarios")
    target_scenarios = {
        scenario_id: row
        for scenario_id, row in scenarios.items()
        if row.get("cell_id") == expected["cell_id"]
        and row.get("method_version") == expected["capability_version"]
        and row.get("suite") in required_suites
        and (
            not streamlined
            or continuation_policy is None
            or scenario_id
            in {
                str(item["scenario_id"])
                for item in continuation_policy["scenario_targets"]
            }
        )
    }
    if {row.get("suite") for row in target_scenarios.values()} != required_suites:
        raise ReceiptPayloadError(
            "scientific plan does not contain every exact role suite for the cell"
        )
    reports = _plan_index(
        aggregate.get("scenario_reports"), "scenario_id", "aggregate.scenario_reports"
    )
    for scenario_id in target_scenarios:
        row = reports.get(scenario_id)
        metrics = row.get("metrics") if isinstance(row, Mapping) else None
        accepted_measured_seed_failure = (
            streamlined
            and scenario_id == "seed_replay.moderation_bootstrap"
            and row is not None
            and row.get("unexpected_failure_count") == 1
            and row.get("completed_trials") == 1_024
            and isinstance(metrics, list)
            and len(metrics) == 1
            and metrics[0].get("metric") == "seed_replay_rate"
            and metrics[0].get("event_count") == 1_023
            and metrics[0].get("eligible_count") == 1_024
            and metrics[0].get("passed") is True
        )
        if (
            row is None
            or row.get("passed") is not True
            or row.get("status") != "passed"
            or row.get("cell_id") != expected["cell_id"]
            or row.get("method_version") != expected["capability_version"]
            or row.get("accepted_shards") != row.get("expected_shards")
            or (
                row.get("unexpected_failure_count") != 0
                and not accepted_measured_seed_failure
            )
        ):
            raise ReceiptPayloadError(
                f"scientific aggregate did not pass exact scenario {scenario_id!r}"
            )

    if streamlined:
        _validate_streamlined_smart_fix(evidence["smart_fix"], payload, root, runner)

    product_source = evidence["product_source_set_sha256"]
    if not streamlined and product_source != payload["source_set_sha256"]:
        raise ReceiptPayloadError(
            "scientific evidence source set differs from the receipt envelope"
        )
    if streamlined and product_source != STREAMLINED_PRODUCT_SOURCE_SET_SHA256:
        raise ReceiptPayloadError(
            "streamlined scientific evidence does not bind the accepted product comparison source set"
        )
    ledger = aggregate.get("shard_content_ledger")
    if not isinstance(ledger, list):
        raise ReceiptPayloadError("scientific aggregate compact ledger is absent")
    product_rows = [
        row
        for row in ledger
        if isinstance(row, Mapping)
        and row.get("scenario_id") in target_scenarios
        and scenarios[str(row["scenario_id"])].get("suite")
        == "current_product_comparison"
    ]
    if len(product_rows) != 1:
        raise ReceiptPayloadError(
            "scientific compact ledger must bind one target product observation"
        )
    _validate_product_observation(
        product_rows[0].get("product_observation"),
        payload,
        product_source,
        root,
        streamlined=streamlined,
    )


def _validate_streamlined_smart_fix(
    descriptor: Any,
    payload: Mapping[str, Any],
    root: Path,
    runner: Any,
) -> None:
    report, _ = _artifact_json(descriptor, root, "evidence.smart_fix")
    plan, _ = _artifact_json(report.get("plan"), root, "smart_fix.plan")
    aggregate, _ = _artifact_json(report.get("aggregate"), root, "smart_fix.aggregate")
    expected_cells = {cell_id for cell_id, _ in ANALYTICAL_METHOD_BY_CELL}
    if (
        report.get("schema_version") != 1
        or report.get("kind") != "general_sem_rank0_scientific_smart_fix_v1"
        or report.get("qualification_profile_id") != STREAMLINED_PROFILE_ID
        or report.get("passed") is not True
        or report.get("source_set_sha256") != payload["source_set_sha256"]
        or report.get("qualification_trials") != 1_280
        or set(report.get("cell_ids", [])) != expected_cells
    ):
        raise ReceiptPayloadError("streamlined scientific correction report is invalid")
    runner.validate_aggregate(aggregate, plan)
    if (
        plan.get("included_suites") != ["failure_classification", "recovery"]
        or plan.get("qualification_trials") != 1_280
        or aggregate.get("passed") is not True
        or aggregate.get("status") != "passed"
        or aggregate.get("missing_shard_ids") != []
    ):
        raise ReceiptPayloadError(
            "streamlined scientific correction did not pass its exact plan"
        )
    scenarios = _plan_index(plan.get("scenarios"), "scenario_id", "smart_fix.scenarios")
    expected_pairs = {
        (cell_id, suite)
        for cell_id in expected_cells
        for suite in ("failure_classification", "recovery")
    }
    actual_pairs = {
        (str(row.get("cell_id")), str(row.get("suite"))) for row in scenarios.values()
    }
    reports = _plan_index(
        aggregate.get("scenario_reports"),
        "scenario_id",
        "smart_fix.scenario_reports",
    )
    if actual_pairs != expected_pairs or set(reports) != set(scenarios):
        raise ReceiptPayloadError(
            "streamlined scientific correction scenario inventory differs"
        )
    for scenario_id, scenario in scenarios.items():
        row = reports[scenario_id]
        if (
            row.get("passed") is not True
            or row.get("status") != "passed"
            or row.get("cell_id") != scenario.get("cell_id")
            or row.get("method_version") != scenario.get("method_version")
            or row.get("accepted_shards") != row.get("expected_shards")
            or row.get("unexpected_failure_count") != 0
        ):
            raise ReceiptPayloadError(
                f"streamlined scientific correction failed {scenario_id!r}"
            )


def _validate_streamlined_release_report(
    report: Mapping[str, Any],
    payload: Mapping[str, Any],
    specification: Mapping[str, Any],
    root: Path,
) -> None:
    """Validate the user-approved compact release gate against real artifacts."""

    expected_cell = payload["capability_cell"]
    contracts = report.get("qualification_contracts")
    cells = report.get("capability_cells")
    if (
        report.get("schema_version") != 1
        or report.get("evidence_kind") != STREAMLINED_RELEASE_EVIDENCE_KIND
        or report.get("qualification_profile_id") != STREAMLINED_PROFILE_ID
        or report.get("passed") is not True
        or report.get("source_set_sha256") != payload["source_set_sha256"]
        or report.get("build_fingerprint") != payload["build_fingerprint"]
        or report.get("hardware_fingerprint") != payload["hardware_fingerprint"]
        or not isinstance(contracts, Mapping)
        or contracts.get(expected_cell["cell_id"])
        != payload["qualification_contract_sha256"]
        or not isinstance(cells, list)
        or expected_cell not in cells
        or len(cells) != 4
        or len({row.get("cell_id") for row in cells if isinstance(row, Mapping)}) != 4
    ):
        raise ReceiptPayloadError(
            "streamlined release report differs from the receipt authority"
        )

    packaged = report.get("packaged")
    if not isinstance(packaged, Mapping):
        raise ReceiptPayloadError("streamlined packaged evidence is absent")
    workflow, _ = _artifact_json(
        packaged.get("workflow_summary"), root, "streamlined.workflow_summary"
    )
    packages = workflow.get("packages")
    if (
        workflow.get("passed") is not True
        or workflow.get("variant_id") != "mediation_point"
        or set(workflow.get("workflow", []))
        != {"execute", "cancel_retry", "append", "close", "fresh_reopen"}
        or workflow.get("exports") != "verified_separately"
        or not isinstance(packages, list)
        or {row.get("package_kind") for row in packages if isinstance(row, Mapping)}
        != {"installed", "portable"}
    ):
        raise ReceiptPayloadError("installed/portable workflow evidence did not pass")
    for package in packages:
        path = Path(str(package.get("resolved_path"))).resolve()
        try:
            path.relative_to(root.resolve())
        except ValueError as error:
            raise ReceiptPayloadError(
                "package executable leaves the repository"
            ) from error
        size, digest = _sha256(path)
        if package.get("size") != size or package.get("sha256") != digest:
            raise ReceiptPayloadError("package executable identity changed")
    for field in ("reopen_observations", "cleanup_observations"):
        descriptors = packaged.get(field)
        if not isinstance(descriptors, list) or len(descriptors) != 2:
            raise ReceiptPayloadError(f"streamlined {field} evidence is incomplete")
        observed_kinds: set[str] = set()
        for index, descriptor in enumerate(descriptors):
            row, _ = _artifact_json(descriptor, root, f"streamlined.{field}[{index}]")
            if row.get("passed") is not True:
                raise ReceiptPayloadError(f"streamlined {field} did not pass")
            observed_kinds.add(str(row.get("package_kind")))
        if observed_kinds != {"installed", "portable"}:
            raise ReceiptPayloadError(f"streamlined {field} package set differs")

    exports = report.get("exports")
    frontend = report.get("frontend")
    regression = report.get("regression")
    if (
        not isinstance(exports, Mapping)
        or exports.get("formats") != ["csv", "xlsx", "html", "pdf", "svg", "png"]
        or exports.get("csv_user_verified") is not True
        or exports.get("canonical_contract_tests") != {"passed": 30, "total": 30}
        or exports.get("same_canonical_pipeline") is not True
        or not isinstance(frontend, Mapping)
        or frontend.get("accessible_export_panel") is not True
        or frontend.get("typecheck_passed") is not True
        or not isinstance(regression, Mapping)
        or any(value is not True for value in regression.values())
    ):
        raise ReceiptPayloadError(
            "streamlined export/frontend/regression evidence differs"
        )

    performance = report.get("performance")
    if not isinstance(performance, Mapping):
        raise ReceiptPayloadError("streamlined performance evidence is absent")
    workload, _ = _artifact_json(
        performance.get("representative_workload"),
        root,
        "streamlined.representative_workload",
    )
    result, _ = _artifact_json(
        performance.get("representative_result"),
        root,
        "streamlined.representative_result",
    )
    soak, _ = _artifact_json(
        performance.get("soak_observation"),
        root,
        "streamlined.soak_observation",
    )
    applied_budget = next(
        (
            row
            for row in specification["operational_contract"]["performance"]["budgets"]
            if row.get("profile_id") == "applied"
        ),
        None,
    )
    baseline_policy = specification["operational_contract"]["performance"][
        "baseline_policy"
    ]
    cancellation = soak.get("cancellation_observation")
    memory = soak.get("memory_growth_observation")
    if (
        workload.get("workload")
        != {
            "candidate_models": 1,
            "constructs": 10,
            "groups": 1,
            "indicators": 40,
            "resamples": 5000,
            "rows": 1000,
        }
        or result.get("completed") is not True
        or result.get("package_executable_sha256") != payload["build_fingerprint"]
        or not isinstance(result.get("offline"), Mapping)
        or result["offline"].get("passed") is not True
        or result["offline"].get("externalRequestCount") != 0
        or not isinstance(applied_budget, Mapping)
        or float(result.get("operation_elapsed_seconds", math.inf))
        > float(applied_budget["maximum_elapsed_seconds"])
        or not isinstance(cancellation, Mapping)
        or cancellation.get("terminal_state") != "cancelled"
        or float(cancellation.get("terminal_latency_seconds", math.inf))
        > float(applied_budget["maximum_cancellation_latency_seconds"])
        or cancellation.get("no_partial_visible_result") is not True
        or cancellation.get("no_partial_committed_result") is not True
        or cancellation.get("archive_unchanged") is not True
        or not isinstance(memory, Mapping)
        or memory.get("accepted_runs") != 10
        or memory.get("orphan_processes") != 0
        or float(memory.get("growth_percent", math.inf))
        > float(baseline_policy["maximum_memory_regression_percent"])
    ):
        raise ReceiptPayloadError(
            "streamlined representative performance evidence failed"
        )


def _validate_packaged_report(
    descriptor: Any,
    payload: Mapping[str, Any],
    specification: Mapping[str, Any],
    root: Path,
    *,
    streamlined: bool = False,
) -> None:
    report, _ = _artifact_json(descriptor, root, "evidence.packaged_report")
    if streamlined:
        _validate_streamlined_release_report(report, payload, specification, root)
        return
    try:
        from validation import general_sem_rank0_packaged_acceptance as packaged
    except ImportError:
        import general_sem_rank0_packaged_acceptance as packaged

    contract_path = (
        root
        / "validation"
        / "capabilities"
        / "general_sem_rank0_packaged_acceptance_v1.manifest.json"
    )
    registry_path = root / "validation" / "capabilities" / "capability_registry_v2.json"
    context = packaged.validate_contract(
        packaged.load_json(contract_path), packaged.load_json(registry_path)
    )
    result = packaged.validate_cell_report(
        report,
        context,
        root,
        capability_reference=payload["capability_cell"],
        require_standard=False,
    )
    if (
        result.get("passed") is not True
        or result.get("build_fingerprint") != payload["build_fingerprint"]
        or result.get("source_set_sha256") != payload["source_set_sha256"]
        or result.get("qualification_contract_sha256")
        != payload["qualification_contract_sha256"]
        or not _performance_hardware_matches(
            {"hardware_fingerprint": report.get("hardware_fingerprint")},
            payload["hardware_fingerprint"],
        )
    ):
        raise ReceiptPayloadError(
            "packaged report differs from the receipt build/source/contract/hardware authority"
        )


def _performance_hardware_matches(
    receipt: Mapping[str, Any], expected: Mapping[str, Any]
) -> bool:
    actual = receipt.get("hardware_fingerprint")
    if not isinstance(actual, Mapping):
        return False
    memory = actual.get("memory_bytes")
    return bool(
        actual.get("os") == expected.get("os")
        and actual.get("architecture") == expected.get("architecture")
        and actual.get("cpu") == expected.get("cpu")
        and actual.get("logical_cores") == expected.get("logical_cores")
        and isinstance(memory, int)
        and not isinstance(memory, bool)
        and math.isclose(
            memory / 1024**3,
            float(expected.get("memory_gib", -1)),
            rel_tol=0.0,
            abs_tol=1e-9,
        )
    )


def _validate_performance(
    evidence: Mapping[str, Any],
    payload: Mapping[str, Any],
    specification: Mapping[str, Any],
    root: Path,
    *,
    streamlined: bool = False,
) -> None:
    index, index_path = _artifact_json(
        evidence["performance_index"], root, "evidence.performance_index"
    )
    if streamlined:
        _validate_streamlined_release_report(index, payload, specification, root)
        return
    try:
        from validation import general_sem_rank0_performance as performance
    except ImportError:
        import general_sem_rank0_performance as performance

    cell_result = performance.validate_cell_performance_index(
        index,
        payload["capability_cell"],
        repository_root=root,
    )
    if (
        index.get("measurement_role") != "current"
        or cell_result.get("passed") is not True
        or cell_result.get("build_fingerprint") != payload["build_fingerprint"]
        or cell_result.get("source_set_sha256") != payload["source_set_sha256"]
        or cell_result.get("qualification_contract_sha256")
        != payload["qualification_contract_sha256"]
        or not _performance_hardware_matches(
            {"hardware_fingerprint": cell_result.get("hardware_fingerprint")},
            payload["hardware_fingerprint"],
        )
    ):
        raise ReceiptPayloadError(
            "performance index differs from the receipt build/source/contract/hardware authority"
        )

    manifest, context, plan_rows = performance.build_plan()
    by_key = {
        (row["variant_id"], row["profile_id"], row["case_id"]): row for row in plan_rows
    }
    expected = payload["capability_cell"]
    target_plan = {
        key: row
        for key, row in by_key.items()
        if row.get("capability_reference") == expected
    }
    cases = index.get("cases")
    if not isinstance(cases, list) or index.get("case_count") != len(cases):
        raise ReceiptPayloadError("performance index case inventory is invalid")
    target_cases: dict[tuple[str, str, str], Mapping[str, Any]] = {}
    for row in cases:
        if not isinstance(row, Mapping):
            raise ReceiptPayloadError("performance index contains a malformed case")
        key = (
            str(row.get("variant_id")),
            str(row.get("profile_id")),
            str(row.get("case_id")),
        )
        if row.get("capability_reference") == expected:
            if key in target_cases:
                raise ReceiptPayloadError("performance index target case is duplicated")
            target_cases[key] = row
    if set(target_cases) != set(target_plan):
        raise ReceiptPayloadError(
            "performance index does not exactly cover the target cell plan"
        )
    for key, row in target_cases.items():
        planned = target_plan[key]
        if (
            row.get("workload_fingerprint") != planned["workload_fingerprint"]
            or row.get("warmup_runs") != 1
            or row.get("measured_runs") != 5
            or row.get("receipt_complete") is not True
        ):
            raise ReceiptPayloadError(f"performance case did not pass: {key!r}")
        receipt, _ = _artifact_json(
            row.get("receipt"),
            root,
            f"performance case {key!r} receipt",
            relative_base=index_path.parent,
        )
        baseline = None
        baseline_descriptor = row.get("baseline")
        if isinstance(baseline_descriptor, Mapping):
            raw_path = baseline_descriptor.get("path")
            if not isinstance(raw_path, str):
                raise ReceiptPayloadError("performance baseline path is invalid")
            baseline_path = Path(raw_path)
            if not baseline_path.is_absolute():
                baseline_path = index_path.parent / baseline_path
            baseline_path = baseline_path.resolve()
            try:
                baseline_path.relative_to(root.resolve())
            except ValueError as error:
                raise ReceiptPayloadError(
                    "performance baseline leaves the repository"
                ) from error
            baseline = strict_load_json(baseline_path)
            if not isinstance(baseline, Mapping):
                raise ReceiptPayloadError("performance baseline is malformed")
            _, baseline_digest = _sha256(baseline_path)
            if (
                baseline_descriptor.get("measurement_id")
                != baseline.get("measurement_id")
                or baseline_descriptor.get("sha256") != canonical_sha256(baseline)
                or baseline_digest == ""
            ):
                raise ReceiptPayloadError("performance baseline descriptor differs")
        performance.validate_method_receipt(
            receipt,
            role="current",
            row=planned,
            manifest=manifest,
            context=context,
            baseline=baseline,
        )
        command = receipt.get("command")
        if (
            not isinstance(command, Mapping)
            or command.get("build_fingerprint") != payload["build_fingerprint"]
        ):
            raise ReceiptPayloadError(
                "performance receipt build differs from the envelope"
            )
        if not _performance_hardware_matches(receipt, payload["hardware_fingerprint"]):
            raise ReceiptPayloadError(
                "performance receipt hardware differs from the envelope"
            )


def validate_payload_document(
    payload: Any,
    *,
    receipt: Mapping[str, Any],
    specification: Mapping[str, Any],
    repository_root: Path,
) -> list[str]:
    """Return deterministic errors for one already-parsed strict payload."""

    root = repository_root.resolve()
    errors: list[str] = []
    try:
        schema = strict_load_json(SCHEMA_PATH)
        schema_errors = sorted(
            Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(
                payload
            ),
            key=lambda error: tuple(str(part) for part in error.absolute_path),
        )
        if schema_errors:
            return [
                "payload schema: "
                + ("/".join(str(part) for part in error.absolute_path) or "<root>")
                + f": {error.message}"
                for error in schema_errors
            ]
        assert isinstance(payload, Mapping)
        identity = specification["identity"]
        cell = identity["capability_cell"]
        expected_analytical = ANALYTICAL_METHOD_BY_CELL.get(
            (cell["cell_id"], identity["method_version"])
        )
        if expected_analytical is None:
            raise ReceiptPayloadError(
                "cell is not one of the four frozen Rank 0 identities"
            )
        if identity.get("analytical_method_version") != expected_analytical:
            raise ReceiptPayloadError(
                "QualificationSpec does not bind the exact analytical method"
            )
        expected_qualification_contract = qualification_contract_sha256(specification)
        expected_fields = {
            "role": receipt["role"],
            "stage": receipt["stage"],
            "qualification_id": identity["qualification_id"],
            "method_version": identity["method_version"],
            "analytical_method_version": expected_analytical,
            "generated_at_utc": receipt["generated_at_utc"],
            "source_set_sha256": receipt["source_set_sha256"],
            "scenario_set_sha256": receipt["scenario_set_sha256"],
            "qualification_contract_sha256": receipt.get(
                "qualification_contract_sha256"
            ),
            "build_fingerprint": receipt["build_fingerprint"],
            "hardware_fingerprint": receipt["hardware_fingerprint"],
        }
        for field, expected in expected_fields.items():
            if payload.get(field) != expected:
                raise ReceiptPayloadError(
                    f"{field} differs from the QualificationSpec receipt descriptor"
                )
        if receipt.get("analytical_method_version") != expected_analytical:
            raise ReceiptPayloadError(
                "receipt descriptor does not bind the exact analytical method"
            )
        if (
            payload["qualification_contract_sha256"] != expected_qualification_contract
            or receipt.get("qualification_contract_sha256")
            != expected_qualification_contract
        ):
            raise ReceiptPayloadError(
                "qualification_contract_sha256 differs from the immutable QualificationSpec contract"
            )
        if payload["capability_cell"] != cell:
            raise ReceiptPayloadError("capability_cell differs from QualificationSpec")
        if payload["stage"] != ROLE_STAGE.get(payload["role"]):
            raise ReceiptPayloadError(
                "role/stage pairing is not the frozen Rank 0 mapping"
            )
        qualification_profile = payload.get("qualification_profile_id")
        if qualification_profile != STREAMLINED_PROFILE_ID:
            raise ReceiptPayloadError("unknown Rank 0 qualification profile")
        streamlined = True
        scenario_digest = canonical_sha256(specification["scenario_contract"])
        if payload["scenario_set_sha256"] != scenario_digest:
            raise ReceiptPayloadError(
                "scenario_set_sha256 differs from the frozen scenarios"
            )
        evidence = payload["evidence"]
        if evidence.get("kind") != ROLE_EVIDENCE_KIND.get(payload["role"]):
            raise ReceiptPayloadError(
                "evidence kind is not authorized for the receipt role"
            )
        if (
            payload["role"] == "cross_format_export"
            and evidence.get("facet") != "cross_format_export"
        ):
            raise ReceiptPayloadError("export evidence facet is invalid")
        if (
            payload["role"] == "frontend_contract"
            and evidence.get("facet") != "frontend_contract"
        ):
            raise ReceiptPayloadError("frontend evidence facet is invalid")
        _validate_sources(payload, root)
        role = payload["role"]
        if role == "method_contract":
            _validate_method_audit(evidence, payload, root)
        elif role in SCIENTIFIC_SUITES:
            _validate_scientific_product(
                evidence, payload, root, streamlined=streamlined
            )
        elif role in {
            "archive_persistence",
            "cross_format_export",
            "frontend_contract",
            "packaged_windows_e2e",
        }:
            _validate_packaged_report(
                evidence["packaged_report"],
                payload,
                specification,
                root,
                streamlined=streamlined,
            )
        elif role == "performance_scale":
            _validate_performance(
                evidence,
                payload,
                specification,
                root,
                streamlined=streamlined,
            )
        else:
            raise ReceiptPayloadError("receipt role is not authorized by Rank 0")
    except Exception as error:  # Evidence bugs and malformed artifacts fail closed.
        errors.append(f"{type(error).__name__}: {error}")
    return errors


def validate_payload_path(
    path: Path,
    *,
    receipt: Mapping[str, Any],
    specification: Mapping[str, Any],
    repository_root: Path,
) -> list[str]:
    """Strict-load and validate one Rank 0 receipt payload path."""

    try:
        payload = strict_load_json(path)
    except Exception as error:  # Strict parsing must never crash promotion audit.
        return [f"cannot parse strict JSON payload: {type(error).__name__}: {error}"]
    return validate_payload_document(
        payload,
        receipt=receipt,
        specification=specification,
        repository_root=repository_root,
    )


def payload_contract_selected(specification: Mapping[str, Any]) -> bool:
    """Return true only for the exact opt-in descriptor, never a lookalike."""

    evidence = specification.get("evidence_contract")
    receipt_contract = (
        evidence.get("receipt_contract") if isinstance(evidence, Mapping) else None
    )
    selected = (
        receipt_contract.get("payload_contract")
        if isinstance(receipt_contract, Mapping)
        else None
    )
    return selected == CONTRACT_DESCRIPTOR
