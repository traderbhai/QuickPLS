#!/usr/bin/env python3
"""Verify v1.2.2 promoted group/prediction/regression scopes are enforced in product-facing code."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "validation" / "results" / "third_batch_product_enforcement_audit.json"


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def load_json(path: str) -> dict:
    value = ROOT / path
    return json.loads(value.read_text(encoding="utf-8")) if value.exists() else {}


def check(name: str, passed: bool, detail: str) -> dict:
    return {"name": name, "passed": bool(passed), "detail": detail}


def main() -> int:
    methods = text("crates/qpls-core/src/methods.rs")
    core_validation = text("crates/qpls-core/src/validation.rs")
    engine = text("crates/qpls-estimation/src/pls.rs")
    native_catalog = text("src/native/nativeAnalysisCatalog.ts")
    native_calculation_dialog = text("src/native/NativeCalculationDialog.tsx")
    native_recipe = text("src/native/nativeAnalysisRecipe.ts")
    native_results = text("src/native/nativeResults.ts")
    compat = text("docs/METHOD_COMPATIBILITY.md")
    packaged = load_json("validation/results/v247_tauri_native_acceptance.json")
    packaged_checks = packaged.get("checks", {})
    checks = [
        check("core_registry_promotes_entry_points", all(
            f'id: "{method}"' in methods and "status: MethodStatus::Validated" in methods.split(f'id: "{method}"', 1)[1].split("}", 1)[0]
            for method in ["mga", "predict", "regression"]
        ), "Core registry exposes the promoted entry points as validated while validation guards unsupported settings."),
        check("native_primary_catalog_exposes_truthful_micom_mga", all(snippet in native_catalog for snippet in [
            'kind: "mga"',
            'categoryLabel: "Groups"',
            "Assess MICOM measurement invariance",
            'groupMethods: "micom,mga_permutation"',
        ]) and "Confirm MICOM Step 1" in native_calculation_dialog
        and 'kind: "micom"' not in native_catalog, "The primary catalog exposes one bounded joint MICOM/MGA v2 workflow and no conflicting standalone MICOM kind."),
        check("native_recipe_enforces_explicit_groups", all(snippet in native_recipe for snippet in [
            'requiredText("groupColumn", settings.groupColumn)',
            'requiredText("groupAValue", settings.groupAValue)',
            'requiredText("groupBValue", settings.groupBValue)',
            'methodTokens(settings.groupMethods, ["micom", "mga_permutation"])',
            'group_methods: "micom,mga_permutation"',
            'micom_configural_confirmed: "true"',
        ]), "The native recipe requires explicit distinct Group A/B values and serializes the exact current MICOM/MGA v2 plan."),
        check("native_results_expose_complete_truthful_micom_mga_tables", all(snippet in native_results for snippet in [
            'id: "mga_group_summary"',
            'id: "micom_summary"',
            'id: "micom_configural"',
            'id: "micom_composition"',
            'id: "micom_means"',
            'id: "micom_variances"',
            'id: "mga_group_paths"',
            'id: "mga_group_r_squared"',
            'id: "mga_group_loadings"',
            'id: "mga_group_weights"',
            'id: "mga_path_differences"',
            'id: "mga_permutation"',
            '"mga_permutation_loadings"',
            '"mga_permutation_weights"',
        ]) and "if (result.mga)" in native_results,
        "Native Results derives the complete current group, measurement, permutation, and MICOM hierarchy without placeholder tables."),
        check("micom_v2_scope_is_enforced", all(phrase in core_validation for phrase in [
            "validated MICOM and permutation-MGA v2 scope requires path weighting",
            "group_permutation_samples between 5000 and 10000",
            "MICOM requires explicit confirmation",
        ]) and 'pub const MICOM_METHOD_VERSION: &str = "micom_v2"' in engine
        and 'pub const MICOM_METHOD_VERSION_V1: &str = "micom_v1"' in engine,
        "Core validation and estimation enforce the bounded current v2 contract while retaining v1 only as an explicit historical identifier."),
        check("engine_warnings_promoted_scope", all(phrase in engine for phrase in [
            "Logistic regression v1 is validated for the documented QuickPLS v1.2.2 binary numeric complete-case scope",
            "PROCESS-style regression v1 is validated for the documented QuickPLS v1.2.2 bounded mediation/moderation workflow scope",
            "PLS-POS v1 is validated for the documented QuickPLS v1.2.2 deterministic 2-5 segment",
            "Two-group MGA v2 reports Group A/Group B structural paths",
            "deterministic two-tailed group-label permutation",
            "MICOM v2 evaluates computational configural prerequisites",
            "FIMIX-PLS v1 is validated for the documented QuickPLS v1.2.2",
            "moderated mediation remains experimental",
        ]), "Generated warnings retain bounded claims for eligible third-batch methods and state the current joint MGA/MICOM v2 scope."),
        check("compatibility_documents_current_group_boundaries", all(phrase in compat for phrase in [
            "| Groups | MICOM and Two-Group Permutation MGA |",
            "5,000–10,000 usable label permutations",
            "MICOM Steps 1–3",
            "| Groups | Historical MICOM v1 | Withdrawn and execution-disabled",
        ]), "The compatibility matrix promotes only the bounded current v2 workflow and records micom_v1 as legacy-only."),
        check("packaged_micom_mga_v2_workflow", packaged.get("passed") is True
        and all(name in packaged_checks for name in ["mgaCalculationDialog", "mgaRunning", "mgaResult", "mgaExport", "mgaSaveReopen"])
        and packaged_checks.get("mgaExport", {}).get("nativeXlsx", {}).get("attempted") is True
        and packaged_checks.get("mgaSaveReopen", {}).get("attempted") is True,
        "The packaged desktop completed the current native setup, active lifecycle, Results, real XLSX export, explicit save, and same-run reopen sequence."),
    ]
    passed = all(item["passed"] for item in checks)
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps({
        "schema_version": 1,
        "target": "v1_2_2_group_prediction_regression_promotion",
        "passed": passed,
        "checks": checks,
    }, indent=2) + "\n", encoding="utf-8")
    if not passed:
        for item in checks:
            if not item["passed"]:
                print(f"FAIL {item['name']}: {item['detail']}")
        return 1
    print(f"wrote {OUTPUT} | passed=True")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
