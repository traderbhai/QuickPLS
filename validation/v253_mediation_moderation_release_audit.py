#!/usr/bin/env python3
"""One compact source/evidence audit for QuickPLS 2.53 mediation/moderation.

This audit intentionally checks integration identities and routing rather than
re-running earlier scientific matrices.  It can run source-only during the
consolidated diagnostic pass, then ingest the compact reference and packaged
reports at the promotion/release gate.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any, Iterable, Mapping


ROOT = Path(__file__).resolve().parents[1]
EXPECTED_PUBLIC_KINDS = (
    "pls_algorithm",
    "plsc",
    "wpls",
    "gsca",
    "cca",
    "cta_pls",
    "ipma",
    "cbsem",
    "pls_bootstrap",
    "plsc_bootstrap",
    "pls_permutation",
    "pls_posthoc_technical_minimum_sample_size",
    "pls_sample_size_power",
    "mga",
    "predict",
    "nca",
    "pca",
    "regression",
)

CELL_CONTRACTS = {
    "single_mediation_bootstrap": {
        "owner": "smartpls.mediation",
        "cell_id": "qpls3.pls.general_sem_single_mediation_bootstrap",
        "capability_version": "general_sem_pls_single_mediation_full_model_case_bootstrap_v1",
        "method_version": "general_sem_pls_single_mediation_full_model_case_bootstrap_v1",
        "operation_version": "general_sem_pls_single_mediation_case_bootstrap_v1",
        "method_manifest": "validation/methods/general_sem_pls_single_mediation_bootstrap_v1.manifest.json",
        "cell_manifest": "validation/capabilities/general_sem_pls_single_mediation_full_model_case_bootstrap_v1.cell.manifest.json",
    },
    "three_way_point": {
        "owner": "smartpls.moderation",
        "cell_id": "qpls3.pls.general_sem_three_way_moderation_point",
        "capability_version": "general_sem_pls_three_way_moderation_point_v1",
        "method_version": "qpls.general-sem-pls.three-way.point.v1",
        "method_manifest": "validation/methods/general_sem_pls_three_way_moderation_point_v1.manifest.json",
        "cell_manifest": "validation/capabilities/general_sem_pls_three_way_moderation_point_v1.cell.manifest.json",
    },
    "three_way_bootstrap": {
        "owner": "smartpls.moderation",
        "cell_id": "qpls3.pls.general_sem_three_way_moderation_bootstrap",
        "capability_version": "general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1",
        "method_version": "qpls.general-sem-pls.three-way.full-model-case-bootstrap.v1",
        "operation_version": "general_sem_pls_three_way_moderation_case_bootstrap_v1",
        "method_manifest": "validation/methods/general_sem_pls_three_way_moderation_bootstrap_v1.manifest.json",
        "cell_manifest": "validation/capabilities/general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1.cell.manifest.json",
    },
}

EVIDENCE_STATES = (
    "absent",
    "engine_only",
    "archive_qualified",
    "native_qualified",
    "release_qualified",
)

SOURCE_PATHS = (
    "src/native/nativeAnalysisCatalog.ts",
    "src/domain/moderationDiagramProjectionV1.ts",
    "src/domain/diagramGraph.ts",
    "src/components/ModelCanvas.tsx",
    "src/components/ModerationAnchorNode.tsx",
    "src/domain/unifiedSemCalculationV1.ts",
    "src/domain/generalSemCapabilityPreflightV1.ts",
    "src/domain/canonicalGeneralSemResultsV1.ts",
    "src/domain/canonicalResultNavigationV1.ts",
    "src/native/NativeModerationDialog.tsx",
    "src/native/NativeCalculationDialog.tsx",
    "src/native/NativeDesktopApp.tsx",
    "src/native/nativeResults.ts",
    "crates/qpls-core/src/general_sem_recipe_compiler_v1.rs",
    "crates/qpls-core/src/general_sem_pls_three_way_v1.rs",
    "crates/qpls-core/src/canonical_result_v2.rs",
    "crates/qpls-estimation/src/general_sem_pls_three_way_v1.rs",
    "crates/qpls-resampling/src/general_sem_pls_three_way_bootstrap_v1.rs",
    "crates/qpls-runner/examples/general_sem_v253_product_reference.rs",
    "crates/qpls-project/src/project_schema_v6.rs",
    "src-tauri/src/recipe_v4_general_sem_pls_jobs.rs",
    "src-tauri/src/recipe_v4_general_sem_canonical_result.rs",
    "validation/capabilities/capability_registry_v2.json",
    "validation/methods/general_sem_pls_single_mediation_bootstrap_v1.manifest.json",
    "validation/methods/general_sem_pls_three_way_moderation_point_v1.manifest.json",
    "validation/methods/general_sem_pls_three_way_moderation_bootstrap_v1.manifest.json",
    "validation/capabilities/general_sem_pls_single_mediation_full_model_case_bootstrap_v1.cell.manifest.json",
    "validation/capabilities/general_sem_pls_three_way_moderation_point_v1.cell.manifest.json",
    "validation/capabilities/general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1.cell.manifest.json",
    "validation/general_sem_v253_reference.py",
    "validation/run_v253_consolidated_diagnostics.ps1",
    "validation/run_v253_mediation_moderation_packaged_smoke.ps1",
    "validation/v253_mediation_moderation_packaged_smoke.mjs",
)


def _text(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def _json(relative: str) -> Any:
    return json.loads(_text(relative))


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _all_tokens(source: str, tokens: Iterable[str]) -> bool:
    return all(token in source for token in tokens)


def _catalogue_kinds(source: str) -> tuple[str, ...]:
    match = re.search(
        r"const\s+CATALOG_DRAFTS[^=]*=\s*\[(.*?)\]\s+as\s+const;",
        source,
        re.DOTALL,
    )
    if not match:
        return ()
    return tuple(re.findall(r'\bkind:\s*"([^"]+)"', match.group(1)))


def _registry_cells(registry: Mapping[str, Any]) -> list[Mapping[str, Any]]:
    rows: list[Mapping[str, Any]] = []
    for capability in registry.get("capabilities", []):
        for cell in capability.get("option_cells", []):
            rows.append(cell)
    return rows


def _valid_cell_lifecycle(surface: Any, evidence_state: Any) -> bool:
    return (
        surface == "labs" and evidence_state in {"absent", "engine_only"}
    ) or (
        surface == "standard" and evidence_state == "release_qualified"
    )


def _surface_matches(actual: Any, expected: str) -> bool:
    return expected == "either" or actual == expected


def _manifest_checks(contract: Mapping[str, str], expected_surface: str) -> dict[str, bool]:
    method = _json(contract["method_manifest"])
    cell = _json(contract["cell_manifest"])
    feature = method.get("feature", {})
    qualification = method.get("qualification", {})
    cell_feature = cell.get("feature", {})
    return {
        "method feature identity": feature.get("id") == contract["cell_id"],
        "method capability version": feature.get("method_version") == contract["capability_version"],
        "method evidence lifecycle is declared": qualification.get("declared_state") in EVIDENCE_STATES
        and qualification.get("target_state") in EVIDENCE_STATES,
        "cell owner identity": cell.get("owner_capability_id") == contract["owner"],
        "cell feature identity": cell_feature.get("id") == contract["cell_id"],
        "cell capability version": cell_feature.get("method_version") == contract["capability_version"],
        "cell analytical method identity": cell_feature.get("analytical_method_version") == contract["method_version"],
        "cell lifecycle is fail closed or independently promoted": _valid_cell_lifecycle(
            cell.get("surface"), cell.get("evidence_state")
        ),
        "cell surface matches requested gate": _surface_matches(cell.get("surface"), expected_surface),
    }


def _ingest_report(path: Path, kind: str) -> dict[str, Any]:
    absolute = path.resolve()
    result: dict[str, Any] = {
        "kind": kind,
        "path": absolute.relative_to(ROOT).as_posix() if absolute.is_relative_to(ROOT) else str(absolute),
        "exists": absolute.is_file(),
        "passed": False,
    }
    if not absolute.is_file():
        result["reason"] = "report is missing"
        return result
    try:
        payload = json.loads(absolute.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        result["reason"] = str(error)
        return result
    result["sha256"] = _sha256(absolute)
    result["reported_passed"] = payload.get("passed") is True
    if kind == "reference":
        identities = payload.get("identities", {})
        identity_ok = all(
            identities.get(name, {}).get("cell_id") == contract["cell_id"]
            and identities.get(name, {}).get("capability_version") == contract["capability_version"]
            and identities.get(name, {}).get("method_version") == contract["method_version"]
            and (
                "operation_version" not in contract
                or identities.get(name, {}).get("operation_version") == contract["operation_version"]
            )
            for name, contract in CELL_CONTRACTS.items()
        )
        result.update({
            "suite_id": payload.get("suite_id"),
            "identity_matrix_passed": identity_ok,
            "r_cross_check_status": payload.get("r_cross_check", {}).get("status"),
            "product_comparison_status": payload.get("product_comparison", {}).get("status"),
        })
        result["passed"] = (
            payload.get("passed") is True
            and identity_ok
            and payload.get("product_comparison", {}).get("status") == "passed"
        )
    elif kind == "packaged":
        screenshots = payload.get("screenshots", [])
        result.update({
            "complete": payload.get("complete") is True,
            "version": payload.get("version"),
            "screenshot_count": len(screenshots) if isinstance(screenshots, list) else 0,
        })
        result["passed"] = (
            payload.get("passed") is True
            and payload.get("complete") is True
            and payload.get("version") == "2.53.0"
            and isinstance(screenshots, list)
            and len(screenshots) >= 5
        )
    else:
        result["passed"] = payload.get("passed") is True
    return result


def _arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference-report", type=Path)
    parser.add_argument("--packaged-report", type=Path)
    parser.add_argument("--evidence-report", action="append", default=[], type=Path)
    parser.add_argument("--require-evidence", action="store_true")
    parser.add_argument(
        "--expect-surface",
        choices=("either", "labs", "standard"),
        default="either",
        help="Optionally bind the audit to its pre-promotion or post-promotion lifecycle gate.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / "validation/results/v253_mediation_moderation_release_audit.json",
    )
    return parser.parse_args()


def main() -> int:
    args = _arguments()
    checks: dict[str, bool] = {}
    missing = [relative for relative in SOURCE_PATHS if not (ROOT / relative).is_file()]
    checks["required integration sources exist"] = not missing

    catalogue = _catalogue_kinds(_text("src/native/nativeAnalysisCatalog.ts"))
    checks["Calculate catalogue remains exactly 18 methods"] = catalogue == EXPECTED_PUBLIC_KINDS

    unified = _text("src/domain/unifiedSemCalculationV1.ts")
    preflight = _text("src/domain/generalSemCapabilityPreflightV1.ts")
    projection = _text("src/domain/moderationDiagramProjectionV1.ts")
    graph = _text("src/domain/diagramGraph.ts")
    canvas = _text("src/components/ModelCanvas.tsx")
    anchor_node = _text("src/components/ModerationAnchorNode.tsx")
    dialog = _text("src/native/NativeModerationDialog.tsx")
    calculation = _text("src/native/NativeCalculationDialog.tsx")
    desktop = _text("src/native/NativeDesktopApp.tsx")
    navigation = _text("src/domain/canonicalResultNavigationV1.ts")
    canonical_ts = _text("src/domain/canonicalGeneralSemResultsV1.ts")
    compiler = _text("crates/qpls-core/src/general_sem_recipe_compiler_v1.rs")
    core_three_way = _text("crates/qpls-core/src/general_sem_pls_three_way_v1.rs")
    estimation = _text("crates/qpls-estimation/src/general_sem_pls_three_way_v1.rs")
    resampling = _text("crates/qpls-resampling/src/general_sem_pls_three_way_bootstrap_v1.rs")
    product_example = _text("crates/qpls-runner/examples/general_sem_v253_product_reference.rs")
    project = _text("crates/qpls-project/src/project_schema_v6.rs")
    tauri_jobs = _text("src-tauri/src/recipe_v4_general_sem_pls_jobs.rs")
    tauri_result = _text("src-tauri/src/recipe_v4_general_sem_canonical_result.rs")

    checks["mediation discovery uses substantive structural relations"] = _all_tokens(unified, (
        "scientificStructuralRelationsV1",
        '(relation.role ?? "structural") === "structural"',
        "generatedVariableIds",
        'variable.kind === "common_factor" || variable.kind === "composite"',
        "countSpecificIndirectPathsV1",
    ))
    checks["single and multiple mediation routes remain distinct"] = _all_tokens(unified + preflight, (
        "inventory.indirectPathCount === 1",
        "GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_CELL_V1",
        "initialIndirectPathCount === 1",
        "PLS_BOOTSTRAP_CELL",
    ))
    checks["three-way point and bootstrap route independently"] = _all_tokens(unified + preflight, (
        "inventory.threeWayInteractionCount > 0",
        "GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CELL_V1",
        "GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_CELL_V1",
        "hasThreeWayInteraction",
    ))
    checks["moderation anchors are explicitly visual only"] = _all_tokens(projection + graph, (
        "ModerationAnchorProjectionV1",
        "visualOnly: true",
        "moderation-anchor::",
        "moderation-connector::",
        "deriveModerationAnchorProjections",
    ))
    checks["native moderation offers pointer, keyboard, and explicit second-moderator alternatives"] = _all_tokens(dialog + canvas + anchor_node + desktop, (
        "parent_interaction",
        "Add three-way effect",
        "Add Second Moderator…",
        "Enter Delete",
        "Release to moderate",
    ))
    checks["Calculate presents concise two and three-way rows"] = _all_tokens(calculation, (
        "unifiedInteractionSummaryV1",
        'interaction.order === "three_way"',
        "Three-way moderation",
        "Moderation",
    ))
    checks["researcher-facing result navigation includes all effect families"] = _all_tokens(navigation, (
        'general_sem_interaction_effects: "moderation"',
        'general_sem_three_way_effect: "three_way_moderation"',
        'general_sem_specific_indirect_effects: "effects"',
    ))
    checks["core compiler declares all three new exact cells"] = _all_tokens(compiler + core_three_way, tuple(
        contract["cell_id"] for contract in CELL_CONTRACTS.values()
    ))
    checks["three-way point engine freezes hierarchy and fixed probes"] = _all_tokens(estimation, (
        "GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1",
        "ContinuousStandardized",
        "BinaryZeroOne",
        "scientific_rescaled_delta",
        "conditional_interaction_effects",
        "simple_slopes",
    ))
    checks["three-way bootstrap is one indexed complete-refit ledger"] = _all_tokens(resampling, (
        "run_bootstrap",
        "GENERAL_SEM_PLS_THREE_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1",
        "complete_model_reestimated_per_replicate: true",
        "all_three_way_targets_share_one_replicate_ledger: true",
        "minimum_usable",
    ))
    checks["compact product witness executes production and exposes exact indexed rows"] = _all_tokens(product_example, (
        "run_compiled_general_sem_pls_recipe_v1",
        "bootstrap_indices",
        '"continuous_continuous"',
        '"continuous_binary"',
        '"single_mediation_bootstrap"',
        '"replicate_positions"',
        '"target_intervals"',
    ))
    checks["canonical payload exposes additive three-way and single identities"] = _all_tokens(canonical_ts + tauri_result, (
        "three_way_interaction_effects",
        "three_way_conditional_interaction_effects",
        "three_way_simple_slopes",
        "three_way_moderation_bootstrap_receipt",
        "GENERAL_SEM_PLS_SINGLE_MEDIATION_CASE_BOOTSTRAP_METHOD_VERSION_V1",
    ))
    checks["Tauri execution routes all three exact cells"] = _all_tokens(tauri_jobs, (
        "pls_general_single_mediation_bootstrap_capability_cell_v1",
        "pls_general_three_way_moderation_point_capability_cell_v1",
        "pls_general_three_way_moderation_bootstrap_capability_cell_v1",
    ))
    checks["schema-6 validation binds new method identities"] = _all_tokens(project, (
        "GENERAL_SEM_PLS_SINGLE_MEDIATION_CASE_BOOTSTRAP_METHOD_VERSION_V1",
        "GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1",
        "GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1",
    ))

    registry = _json("validation/capabilities/capability_registry_v2.json")
    registry_rows = _registry_cells(registry)
    manifest_details: dict[str, dict[str, bool]] = {}
    registry_details: dict[str, dict[str, Any]] = {}
    for name, contract in CELL_CONTRACTS.items():
        matched = [
            row for row in registry_rows
            if row.get("capability_id") == contract["owner"]
            and row.get("cell_id") == contract["cell_id"]
            and row.get("capability_version") == contract["capability_version"]
        ]
        registry_details[name] = {
            "matches": len(matched),
            "evidence_state": matched[0].get("evidence_state") if len(matched) == 1 else None,
            "surface": matched[0].get("surface") if len(matched) == 1 else None,
        }
        checks[f"Registry exact cell: {name}"] = (
            len(matched) == 1
            and _valid_cell_lifecycle(
                matched[0].get("surface"), matched[0].get("evidence_state")
            )
            and _surface_matches(matched[0].get("surface"), args.expect_surface)
        )
        manifest_details[name] = _manifest_checks(contract, args.expect_surface)
        checks[f"Manifests exact and lifecycle-consistent: {name}"] = all(manifest_details[name].values())

    reports: list[dict[str, Any]] = []
    if args.reference_report:
        reports.append(_ingest_report(args.reference_report, "reference"))
    if args.packaged_report:
        reports.append(_ingest_report(args.packaged_report, "packaged"))
    reports.extend(_ingest_report(path, "additional") for path in args.evidence_report)
    evidence_kinds = {report["kind"] for report in reports if report.get("passed")}
    checks["provided evidence reports pass"] = all(report.get("passed") is True for report in reports)
    checks["required evidence reports supplied"] = (
        not args.require_evidence
        or {"reference", "packaged"}.issubset(evidence_kinds)
    )

    source_hashes = {
        relative: _sha256(ROOT / relative)
        for relative in SOURCE_PATHS
        if (ROOT / relative).is_file()
    }
    failed = [name for name, passed in checks.items() if not passed]
    payload = {
        "schema_version": 1,
        "audit_id": "quickpls_v253_mediation_moderation_release_audit_v1",
        "version": "2.53.0",
        "passed": not failed,
        "scope": "source integration, exact Registry routing, and supplied evidence ingestion",
        "expected_surface": args.expect_surface,
        "public_method_count": len(catalogue),
        "public_method_kinds": catalogue,
        "checks": checks,
        "failed": failed,
        "missing_sources": missing,
        "registry_cells": registry_details,
        "manifests": manifest_details,
        "evidence_reports": reports,
        "source_sha256": source_hashes,
        "excluded_repetition": [
            "historical two-way moderation qualification matrices",
            "historical multiple-mediation qualification matrices",
            "per-cell package and performance matrices",
        ],
    }
    encoded = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    output = args.output if args.output.is_absolute() else ROOT / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(encoded, encoding="utf-8")
    print(encoded, end="")
    return 0 if payload["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
