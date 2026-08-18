#!/usr/bin/env python3
"""Conservatively reconcile active evidence claims with live derivation.

The default mode is read-only. ``--write`` atomically rewrites only the frozen
set of stale non-MICOM method manifests and Capability Registry V2. Evidence
artifacts under ``validation/results`` are historical records and are never
deleted or rewritten by this utility.
"""

from __future__ import annotations

import argparse
import copy
import json
import os
import shutil
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any

from capability_registry_v2 import (
    DEFAULT_REGISTRY_PATH,
    REPOSITORY_ROOT,
    derive_legacy_status,
    derive_row_projection,
    load_registry,
    validate_registry_document,
)
from method_promotion_manifest import (
    STATE_ORDER,
    validate_all,
    validate_manifest_document,
)


RECONCILIATION_ID = "quickpls.evidence_truth_reconciliation.v1"
RECONCILED_MANIFEST_NAMES = frozenset(
    {
        "cbsem_ml_v1.manifest.json",
        "cca_residuals_v1.manifest.json",
        "cta_pls_v1.manifest.json",
        "gaussian_copula_endogeneity_v1.manifest.json",
        "gsca_als_v2.manifest.json",
        "higher_order_v1.manifest.json",
        "ipma_v1.manifest.json",
        "logistic_regression_v2.manifest.json",
        "mediation_v1.manifest.json",
        "micom_permutation_mga_v4.manifest.json",
        "moderation_v1.manifest.json",
        "nca_v2.manifest.json",
        "ols_v1.manifest.json",
        "pca_v1.manifest.json",
        "pls_algorithm_v1.manifest.json",
        "pls_bootstrap_v4.manifest.json",
        "plsc_v2.manifest.json",
        "plspredict_cvpat_v2.manifest.json",
        "process_v2.manifest.json",
        "regression_bootstrap_v1.manifest.json",
        "structural_path_randomization_v1.manifest.json",
        "wpls_v1.manifest.json",
    }
)
EXPECTED_FINAL_ROW_EVIDENCE_COUNTS = {
    "absent": 28,
    "engine_only": 1,
    "archive_qualified": 3,
    "native_qualified": 2,
    "release_qualified": 11,
}
EXPECTED_FINAL_CELL_EVIDENCE_COUNTS = {
    "absent": 29,
    "engine_only": 2,
    "archive_qualified": 3,
    "native_qualified": 2,
    "release_qualified": 12,
}
REQUALIFIED_ARCHIVE_METHODS = frozenset(
    {"higher_order_v1", "mediation_v1", "moderation_v1"}
)
REQUALIFICATION_DIRECTORY = "evidence_truth_reconciliation_v1"
CLAIM_LANGUAGE_REPLACEMENTS = {
    "Every competitor-scope row maps to a frozen method-factory capability. Capabilities already governed by the current 17-capability parity ledger derive status from that evidence-backed ledger; all other capabilities derive status only from their exact validated method manifest. Editable row status and source-file references cannot promote a claim.": (
        "Capability Registry V2 is the current option-cell authority. Each "
        "method-manifest-linked option cell is fail-closed against the live state "
        "re-derived from its exact manifest; capability-cell contracts are separately "
        "validated by their exact qualification link. The historical parity ledger "
        "and generated legacy catalogue are non-authoritative records and projections. "
        "Editable row status, source-file references, and historical artifacts cannot "
        "promote a claim."
    ),
    "Preserve existing release-qualified evidence.": (
        "Rebuild all source-bound qualification evidence before preserving or "
        "promoting any evidence claim."
    ),
    "Promote current native and assessment capabilities to release-qualified evidence.": (
        "Rebuild source-bound engine, archive, native, and release evidence in sequence."
    ),
    "Maintain current scoped evidence; unsupported model shapes remain blocked and no numerical identity with SmartPLS is claimed.": (
        "Rebuild current source-bound qualification evidence; unsupported model shapes "
        "remain blocked and no numerical identity with SmartPLS is claimed."
    ),
    "Maintain current scoped evidence; broader survey designs and inference remain excluded.": (
        "Rebuild current source-bound qualification evidence; broader survey designs and "
        "inference remain excluded."
    ),
    "Maintain current scoped evidence; consistent resampling and MGA remain separate gaps.": (
        "Rebuild current source-bound qualification evidence; consistent resampling and "
        "MGA remain separate gaps."
    ),
    "Maintain current scoped evidence; rotations and inference remain excluded.": (
        "Rebuild current source-bound qualification evidence; rotations and inference "
        "remain excluded."
    ),
    "Maintain current scoped evidence; claims remain limited to supported PLS model shapes and documented defaults.": (
        "Rebuild current source-bound qualification evidence; any future claim remains "
        "limited to supported PLS model shapes and documented defaults."
    ),
    "The separately bounded fixed-score structural path randomization is release-qualified, but the shared official row also requires the currently in-progress two-group MICOM/permutation-MGA capability.": (
        "The separately bounded fixed-score structural path-randomization implementation "
        "has historical evidence, but its current source-bound manifest derives absent; "
        "the shared official row also requires the two-group MICOM/permutation-MGA capability."
    ),
    "Maintain current scoped evidence; separately saved competing-model comparison remains outside this bounded single-model benchmark workflow.": (
        "Rebuild current source-bound qualification evidence; separately saved "
        "competing-model comparison remains outside this bounded single-model benchmark workflow."
    ),
    "Maintain current scoped evidence; inferential CCA, resampled discrepancy tests, thresholds, and broad confirmatory claims remain excluded.": (
        "Rebuild current source-bound qualification evidence; inferential CCA, resampled "
        "discrepancy tests, thresholds, and broad confirmatory claims remain excluded."
    ),
    "Maintain current scoped evidence; saved-model comparison, formative targets, and broader shapes remain excluded.": (
        "Rebuild current source-bound qualification evidence; saved-model comparison, "
        "formative targets, and broader shapes remain excluded."
    ),
    "Maintain current scoped evidence; multiple outcomes, theoretical-range correction, and cIPMA remain excluded.": (
        "Rebuild current source-bound qualification evidence; multiple outcomes, "
        "theoretical-range correction, and cIPMA remain excluded."
    ),
    "Maintain current scoped point-estimation evidence; GSCA bootstrap and inference remain absent and cannot be claimed for the full official entry.": (
        "Rebuild current source-bound point-estimation evidence; GSCA bootstrap and "
        "inference remain absent and cannot be claimed for the full official entry."
    ),
    "Maintain current scoped evidence; latent-score, multiple-condition, theoretical-range, and cIPMA workflows remain excluded.": (
        "Rebuild current source-bound qualification evidence; latent-score, "
        "multiple-condition, theoretical-range, and cIPMA workflows remain excluded."
    ),
    "Maintain current scoped evidence; categorical helpers, weights, clustered SEs, mixed, and panel models remain excluded.": (
        "Rebuild current source-bound qualification evidence; categorical helpers, "
        "weights, clustered SEs, mixed, and panel models remain excluded."
    ),
    "Release-qualified evidence remains limited to single-group continuous reflective raw-data ML CFA or recursive SEM with listwise deletion. A separate CompiledCbsemPlanV2 raw/covariance/scaled-correlation Internal path now reaches cancellable Recipe V4 execution and canonical schema-6 append/readback, but its compatibility-only QualificationSpec V2 has zero receipts and it remains excluded from this cell's accepted data predicate, evidence state, and customer execution surface.": (
        "The intended qualification predicate remains single-group continuous reflective "
        "raw-data ML CFA or recursive SEM with listwise deletion, but its current "
        "source-bound manifest derives absent. A separate CompiledCbsemPlanV2 "
        "raw/covariance/scaled-correlation Internal path reaches cancellable Recipe V4 "
        "execution and canonical schema-6 append/readback, but its compatibility-only "
        "QualificationSpec V2 has zero receipts and it remains excluded from this cell's "
        "accepted data predicate, evidence state, and customer execution surface."
    ),
    "The bounded raw/covariance/scaled-correlation CompiledCbsemPlanV2 path has internal Recipe V4, cancellable-job, canonical-result, schema-6 append/readback, independent NumPy/SciPy work-oracle, and compatibility-only QualificationSpec V2 foundations. It has zero admitted receipts and no registered matrix-input option cell, Standard GUI, CLI, semantic export, packaged Windows, or independent scientific-review qualification; it is not part of the release-qualified claim.": (
        "The bounded raw/covariance/scaled-correlation CompiledCbsemPlanV2 path has "
        "Internal Recipe V4, cancellable-job, canonical-result, schema-6 append/readback, "
        "independent NumPy/SciPy work-oracle, and compatibility-only QualificationSpec V2 "
        "foundations. It has zero admitted receipts and no registered matrix-input option "
        "cell, Standard GUI, CLI, semantic export, packaged Windows, or independent "
        "scientific-review qualification; it creates no evidence claim for this absent cell."
    ),
    "Maintain current raw-data evidence; create a separate registered matrix-input option cell, complete independent product-oracle qualification, customer GUI/CLI activation, semantic export readback, and packaged Windows evidence before expanding any accepted predicate. Robust ML, ordinal/WLSMV, FIML, broad constraints, interactions, and HOCs remain separate gaps.": (
        "Rebuild current source-bound raw-data evidence; create a separate registered "
        "matrix-input option cell, complete independent product-oracle qualification, "
        "customer GUI/CLI activation, semantic export readback, and packaged Windows "
        "evidence before expanding any accepted predicate. Robust ML, ordinal/WLSMV, "
        "FIML, broad constraints, interactions, and HOCs remain separate gaps."
    ),
    "The release-qualified predicate remains the bounded raw-data workflow. Raw/covariance/scaled-correlation Recipe V4 and schema-6 execution is a separate unqualified Internal foundation with a compatibility-only QualificationSpec V2 and zero receipts, and is not available from this cell; robust ML, ordinal/WLSMV, FIML, broad constraints, interactions, and HOCs remain excluded.": (
        "The bounded raw-data workflow remains the intended qualification predicate, "
        "but its current source-bound manifest derives absent. Raw/covariance/"
        "scaled-correlation Recipe V4 and schema-6 execution is a separate unqualified "
        "Internal foundation with a compatibility-only QualificationSpec V2 and zero "
        "receipts, and is not available from this cell; robust ML, ordinal/WLSMV, FIML, "
        "broad constraints, interactions, and HOCs remain excluded."
    ),
    "Maintain current scoped evidence; robust, ordinal, FIML, mean-structure, and broader invariance scopes remain excluded.": (
        "Rebuild current source-bound qualification evidence; robust, ordinal, FIML, "
        "mean-structure, and broader invariance scopes remain excluded."
    ),
}


def _load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8-sig") as handle:
        document = json.load(handle)
    if not isinstance(document, dict):
        raise ValueError(f"{path}: JSON root must be an object")
    return document


def _json_bytes(document: dict[str, Any]) -> bytes:
    return (json.dumps(document, ensure_ascii=False, indent=2) + "\n").encode("utf-8")


def _atomic_write(path: Path, payload: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def _evidence_counts(registry: dict[str, Any]) -> dict[str, dict[str, int]]:
    rows = Counter(
        str(capability.get("evidence_state"))
        for capability in registry.get("capabilities", [])
        if isinstance(capability, dict)
    )
    cells = Counter(
        str(cell.get("evidence_state"))
        for capability in registry.get("capabilities", [])
        if isinstance(capability, dict)
        for cell in capability.get("option_cells", [])
        if isinstance(cell, dict)
    )
    return {
        "rows": {state: rows.get(state, 0) for state in STATE_ORDER},
        "cells": {state: cells.get(state, 0) for state in STATE_ORDER},
    }


def _replace_claim_language(value: Any) -> tuple[Any, int]:
    if isinstance(value, dict):
        result: dict[str, Any] = {}
        count = 0
        for key, child in value.items():
            result[key], child_count = _replace_claim_language(child)
            count += child_count
        return result, count
    if isinstance(value, list):
        result_list = []
        count = 0
        for child in value:
            replaced, child_count = _replace_claim_language(child)
            result_list.append(replaced)
            count += child_count
        return result_list, count
    if isinstance(value, str) and value in CLAIM_LANGUAGE_REPLACEMENTS:
        return CLAIM_LANGUAGE_REPLACEMENTS[value], 1
    return value, 0


def build_reconciliation_plan(
    repository_root: Path = REPOSITORY_ROOT,
) -> dict[str, Any]:
    """Build and validate the conservative in-memory reconciliation plan."""

    root = repository_root.resolve()
    registry_path = (root / DEFAULT_REGISTRY_PATH.relative_to(REPOSITORY_ROOT)).resolve()
    registry = load_registry(registry_path)
    manifest_report = validate_all(repository_root=root)
    results_by_name = {
        Path(result["path"]).name: result for result in manifest_report["manifests"]
    }
    errors: list[str] = []

    unexpected_failures = sorted(
        name
        for name, result in results_by_name.items()
        if not result["passed"] and name not in RECONCILED_MANIFEST_NAMES
    )
    if unexpected_failures:
        errors.append(
            "refusing to touch manifests outside the frozen reconciliation set: "
            + ", ".join(unexpected_failures)
        )
    missing = sorted(RECONCILED_MANIFEST_NAMES - set(results_by_name))
    if missing:
        errors.append("frozen reconciliation manifests are missing: " + ", ".join(missing))

    maturity = {state: index for index, state in enumerate(STATE_ORDER)}
    manifest_documents: dict[Path, dict[str, Any]] = {}
    manifest_changes: list[dict[str, Any]] = []
    archive_requalification_methods: list[str] = []
    derived_by_path: dict[Path, str] = {}
    for name in sorted(RECONCILED_MANIFEST_NAMES):
        result = results_by_name.get(name)
        if result is None:
            continue
        path = Path(result["path"]).resolve()
        derived = str(result.get("derived_state", "absent"))
        document = _load_json(path)
        updated = copy.deepcopy(document)
        qualification = updated["qualification"]
        qualification["declared_state"] = derived
        for state in STATE_ORDER[1:]:
            if maturity[state] > maturity[derived]:
                qualification["evidence"][state] = []
        method_name = name.removesuffix(".manifest.json")
        if method_name in REQUALIFIED_ARCHIVE_METHODS:
            if derived != "archive_qualified":
                errors.append(
                    f"{name}: expected the retained lower stages to derive archive_qualified, "
                    f"found {derived}"
                )
            for state, filename in (
                ("engine_only", "engine_evidence.identity.json"),
                ("archive_qualified", "persistence_report.identity.json"),
            ):
                artifacts = qualification["evidence"].get(state, [])
                if len(artifacts) != 1:
                    errors.append(f"{name}: expected exactly one retained {state} artifact")
                    continue
                artifacts[0]["path"] = (
                    f"validation/results/method_factory/{method_name}/"
                    f"{REQUALIFICATION_DIRECTORY}/{filename}"
                )
            if updated != document:
                archive_requalification_methods.append(method_name)
        validation = validate_manifest_document(
            updated,
            root,
            manifest_path=path,
            verify_evidence=method_name not in archive_requalification_methods,
        )
        if not validation["passed"]:
            errors.append(
                f"{name}: reconciled manifest would remain invalid: "
                + "; ".join(validation["errors"][:5])
            )
        manifest_documents[path] = updated
        derived_by_path[path] = derived
        if updated != document:
            manifest_changes.append(
                {
                    "path": path.relative_to(root).as_posix(),
                    "declared_before": document["qualification"]["declared_state"],
                    "declared_after": derived,
                    "cleared_stages": [
                        state
                        for state in STATE_ORDER[1:]
                        if document["qualification"]["evidence"].get(state)
                        and not updated["qualification"]["evidence"].get(state)
                    ],
                }
            )

    updated_registry, claim_language_change_count = _replace_claim_language(registry)
    cell_changes: list[dict[str, Any]] = []
    legacy_evidence_pointer_clear_count = 0
    for capability in updated_registry.get("capabilities", []):
        if not isinstance(capability, dict):
            continue
        for cell in capability.get("option_cells", []):
            if not isinstance(cell, dict):
                continue
            references = cell.get("qualification_spec", {}).get("references", [])
            derived_states = []
            for reference in references:
                if not isinstance(reference, str):
                    continue
                path = (root / reference).resolve()
                if path in derived_by_path:
                    derived_states.append(derived_by_path[path])
            if not derived_states:
                continue
            derived = min(derived_states, key=maturity.__getitem__)
            current = str(cell.get("evidence_state"))
            desired = min((current, derived), key=maturity.__getitem__)
            if desired != current:
                cell_changes.append(
                    {
                        "capability_id": capability.get("capability_id"),
                        "cell_id": cell.get("cell_id"),
                        "before": current,
                        "after": desired,
                    }
                )
                cell["evidence_state"] = desired
        projection = derive_row_projection(capability)
        capability["coverage_state"] = projection["coverage_state"]
        capability["evidence_state"] = projection["evidence_state"]
        capability["surface"] = projection["surface"]
        if (
            projection["evidence_state"] == "absent"
            and capability["legacy_row"].get("implementation_evidence")
        ):
            capability["legacy_row"]["implementation_evidence"] = []
            legacy_evidence_pointer_clear_count += 1
        capability["legacy_row"]["status"] = derive_legacy_status(capability)

    before_counts = _evidence_counts(registry)
    after_counts = _evidence_counts(updated_registry)
    if after_counts["rows"] != EXPECTED_FINAL_ROW_EVIDENCE_COUNTS:
        errors.append(f"unexpected final row evidence counts: {after_counts['rows']!r}")
    if after_counts["cells"] != EXPECTED_FINAL_CELL_EVIDENCE_COUNTS:
        errors.append(f"unexpected final cell evidence counts: {after_counts['cells']!r}")
    registry_validation = validate_registry_document(
        updated_registry,
        repository_root=root,
        check_references=False,
    )
    if not registry_validation["passed"]:
        errors.append(
            "reconciled registry fails structural validation: "
            + "; ".join(registry_validation["errors"][:10])
        )

    return {
        "reconciliation_id": RECONCILIATION_ID,
        "passed": not errors,
        "errors": errors,
        "before_evidence_counts": before_counts,
        "after_evidence_counts": after_counts,
        "manifest_changes": manifest_changes,
        "cell_changes": cell_changes,
        "claim_language_change_count": claim_language_change_count,
        "legacy_evidence_pointer_clear_count": legacy_evidence_pointer_clear_count,
        "archive_requalification_methods": archive_requalification_methods,
        "historical_artifacts_modified": False,
        "registry_path": registry_path,
        "registry_document": updated_registry,
        "manifest_documents": manifest_documents,
    }


def _public_report(plan: dict[str, Any], *, written: bool) -> dict[str, Any]:
    return {
        "reconciliation_id": plan["reconciliation_id"],
        "passed": plan["passed"],
        "written": written,
        "error_count": len(plan["errors"]),
        "errors": plan["errors"],
        "before_evidence_counts": plan["before_evidence_counts"],
        "after_evidence_counts": plan["after_evidence_counts"],
        "manifest_change_count": len(plan["manifest_changes"]),
        "manifest_changes": plan["manifest_changes"],
        "cell_change_count": len(plan["cell_changes"]),
        "cell_changes": plan["cell_changes"],
        "claim_language_change_count": plan["claim_language_change_count"],
        "legacy_evidence_pointer_clear_count": plan[
            "legacy_evidence_pointer_clear_count"
        ],
        "archive_requalification_methods": plan["archive_requalification_methods"],
        "historical_artifacts_modified": False,
    }


def _run_archive_requalification(methods: list[str], root: Path) -> list[Path]:
    """Rerun only retained engine/archive gates into new evidence directories."""

    if not methods:
        return []
    import phase3_workflow_factory as factory

    old_report_root = factory.report_root
    old_work_root = factory.work_root
    created_roots: list[Path] = []

    def reconciliation_root(method: str) -> Path:
        return (
            factory.RESULTS
            / "method_factory"
            / method
            / REQUALIFICATION_DIRECTORY
        )

    for method in methods:
        destination = reconciliation_root(method).resolve()
        expected_parent = (
            root / "validation/results/method_factory" / method
        ).resolve()
        if destination.parent != expected_parent or destination.name != REQUALIFICATION_DIRECTORY:
            raise ValueError(f"unsafe archive requalification destination: {destination}")
        if destination.exists():
            raise FileExistsError(
                f"refusing to overwrite retained qualification evidence: {destination}"
            )

    factory.report_root = reconciliation_root
    factory.work_root = lambda method: reconciliation_root(method) / "work"
    try:
        for method in methods:
            destination = reconciliation_root(method)
            destination.mkdir(parents=True, exist_ok=False)
            created_roots.append(destination)
            engine = factory.engine_gate(method)
            boundary = factory.boundary_gate(method)
            engine_passed = engine["passed"] and boundary["passed"]
            factory.write_identity(
                method,
                "engine_evidence",
                [
                    "method_spec",
                    "independent_reference",
                    "simulation_report",
                    "boundary_report",
                ],
                {
                    "passed": engine_passed,
                    "engine_gate": engine,
                    "boundary_gate": boundary,
                },
                [engine["path"], boundary["path"]],
            )
            if not engine_passed:
                raise ValueError(f"{method}: retained engine qualification did not pass")
            persistence = factory.persistence_gate(method)
            factory.write_identity(
                method,
                "persistence_report",
                ["persistence_report"],
                persistence,
                [persistence["path"], persistence["archive"]],
            )
            if not persistence["passed"]:
                raise ValueError(f"{method}: retained archive qualification did not pass")
    except Exception:
        for destination in reversed(created_roots):
            shutil.rmtree(destination)
        raise
    finally:
        factory.report_root = old_report_root
        factory.work_root = old_work_root
    return created_roots


def apply_reconciliation(plan: dict[str, Any], repository_root: Path) -> None:
    """Write the prevalidated plan and roll back if the final gates fail."""

    if not plan["passed"]:
        raise ValueError("cannot write an invalid reconciliation plan")
    root = repository_root.resolve()
    payloads = {
        **{
            path: _json_bytes(document)
            for path, document in plan["manifest_documents"].items()
        },
        plan["registry_path"]: _json_bytes(plan["registry_document"]),
    }
    originals = {path: path.read_bytes() for path in payloads}
    created_evidence_roots: list[Path] = []
    try:
        manifest_paths = set(plan["manifest_documents"])
        for path, payload in payloads.items():
            if path not in manifest_paths:
                continue
            if payload != originals[path]:
                _atomic_write(path, payload)
        created_evidence_roots = _run_archive_requalification(
            plan["archive_requalification_methods"], root
        )
        manifest_report = validate_all(repository_root=root)
        if not manifest_report["passed"]:
            failures = [
                f"{Path(item['path']).name}:{len(item.get('errors', []))}"
                for item in manifest_report["manifests"]
                if not item["passed"]
            ]
            raise ValueError("post-write manifest gate failed: " + ", ".join(failures))
        registry_payload = payloads[plan["registry_path"]]
        if registry_payload != originals[plan["registry_path"]]:
            _atomic_write(plan["registry_path"], registry_payload)
        registry_report = validate_registry_document(
            load_registry(plan["registry_path"]),
            repository_root=root,
            check_references=True,
        )
        if not registry_report["passed"]:
            raise ValueError(
                "post-write registry gate failed: "
                + "; ".join(registry_report["errors"][:10])
            )
    except Exception:
        for path, payload in originals.items():
            _atomic_write(path, payload)
        for destination in reversed(created_evidence_roots):
            if (
                destination.name == REQUALIFICATION_DIRECTORY
                and "validation" in destination.parts
                and "method_factory" in destination.parts
            ):
                shutil.rmtree(destination)
        raise


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)
    root = REPOSITORY_ROOT.resolve()
    plan = build_reconciliation_plan(root)
    written = False
    if args.write and plan["passed"]:
        try:
            apply_reconciliation(plan, root)
            written = True
        except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
            plan["passed"] = False
            plan["errors"].append(str(error))
    print(json.dumps(_public_report(plan, written=written), indent=2, ensure_ascii=False))
    return 0 if plan["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
