#!/usr/bin/env python3
"""Audit the truthful native/product boundary for the v1.2.1 method batch."""

from __future__ import annotations

import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "validation" / "results" / "second_batch_product_enforcement_audit.json"

PRIMARY_NATIVE_KINDS = {
    "pls_algorithm",
    "plsc",
    "wpls",
    "cca",
    "ipma",
    "pls_bootstrap",
    "pls_permutation",
    "mga",
    "predict",
    "nca",
}

# These methods have bounded numerical/recipe contracts but are not selectable
# in the current primary native workbench. Engine support is not desktop
# workflow acceptance.
BOUNDED_ENGINE_NATIVE_BACKLOG = {
    "cta_pls",
    "endogeneity",
    "nonlinear_effects",
    "moderated_mediation",
    "cbsem",
    "pca",
    "gsca",
    "regression",
}


def text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def load_json(path: str) -> dict:
    source = ROOT / path
    return json.loads(source.read_text(encoding="utf-8")) if source.exists() else {}


def source_section(source: str, start: str, end: str) -> str:
    if start not in source or end not in source:
        return ""
    return source.split(start, 1)[1].split(end, 1)[0]


def check(name: str, passed: bool, detail: str) -> dict:
    return {"name": name, "passed": bool(passed), "detail": detail}


def catalog_kinds(source: str) -> set[str]:
    drafts = source_section(source, "const CATALOG_DRAFTS", "] as const;")
    return set(re.findall(r'\bkind:\s*"([^"]+)"', drafts))


def recipe_descriptors(source: str) -> dict[str, tuple[str, str, str]]:
    block = source_section(source, "NATIVE_ANALYSIS_RECIPE_DESCRIPTORS = [", "] as const")
    matches = re.findall(
        r'\{\s*kind:\s*"([^"]+)".*?engineMethod:\s*"([^"]+)".*?'
        r'scopeStatus:\s*"([^"]+)".*?scopeMetadata:\s*"([^"]+)"\s*\}',
        block,
    )
    return {kind: (method, status, metadata) for kind, method, status, metadata in matches}


def main() -> int:
    native_catalog = text("src/native/nativeAnalysisCatalog.ts")
    native_mode = text("src/native/nativeCalculationMode.ts")
    native_recipe = text("src/native/nativeAnalysisRecipe.ts")
    native_results = text("src/native/nativeResults.ts")
    core_validation = text("crates/qpls-core/src/validation.rs")
    engine = text("crates/qpls-estimation/src/pls.rs")
    compatibility = text("docs/METHOD_COMPATIBILITY.md")
    promotion_program = text("docs/METHOD_PROMOTION_PROGRAM_V1_2.md")
    native_redesign = text("docs/NATIVE_DESKTOP_REDESIGN.md")
    ipma_spec = text("docs/methods/IPMA_V1.md")
    nca_spec = text("docs/methods/NCA_V2.md")
    micom_spec = text("docs/methods/MICOM_V2.md")
    prediction_spec = text("docs/methods/PLSPREDICT_INDICATOR_V2.md")
    ipma_promotion = load_json("validation/results/ipma_method_promotion_audit.json")
    nca_promotion = load_json("validation/results/nca_method_promotion_audit.json")
    prediction_promotion = load_json("validation/results/plspredict_method_promotion_audit.json")

    observed_catalog_kinds = catalog_kinds(native_catalog)
    descriptors = recipe_descriptors(native_recipe)
    production_shell_doc = source_section(native_redesign, "## Production Shell", "## Primary Workflow")
    primary_workflow_doc = source_section(native_redesign, "## Primary Workflow", "## State Truthfulness")
    expected_second_batch_descriptors = {
        "plsc": ("plsc", "validated", "validated_v1_2_1_plsc_bounded_scope"),
        "wpls": ("wpls", "validated", "validated_v1_2_1_wpls_bounded_scope"),
        "predict": ("predict", "validated", "validated_plspredict_indicator_v2_and_cvpat_indicator_benchmarks_v2_bounded_scope"),
        "ipma": ("ipma", "validated", "validated_v1_2_1_ipma_bounded_scope"),
        "nca": ("nca", "validated", "validated_nca_v2_bounded_scope"),
    }

    checks = [
        check(
            "primary_native_catalog_is_exact_and_bounded",
            observed_catalog_kinds == PRIMARY_NATIVE_KINDS
            and 'description: "Map each structural predecessor\'s total importance against observed-range construct performance for one endogenous target."' in native_catalog
            and 'description: "Inspect descriptive residuals between observed and model-reproduced composite correlations."' in native_catalog
            and "NATIVE_PREDICTION_SCOPE_DESCRIPTION" in native_catalog
            and 'NATIVE_PREDICTION_METHOD_LABEL = "PLSpredict / CVPAT"' in native_mode,
            "The default workbench exposes exactly the ten accepted model-bound and model-free native workflows, including bounded MICOM/MGA v2 and NCA v2.",
        ),
        check(
            "native_second_batch_recipes_are_bounded",
            all(descriptors.get(kind) == expected for kind, expected in expected_second_batch_descriptors.items())
            and 'if (["plsc", "wpls", "cca", "cta_pls", "endogeneity", "nonlinear_effects", "moderated_mediation"].includes(kind) && weightingScheme === "pca")' in native_recipe
            and 'kind === "wpls" && preprocessing !== "standardized"' in native_recipe
            and 'requiredText("caseWeightColumn", source.caseWeightColumn)' in native_recipe
            and 'kind === "ipma" && weightingScheme !== "path"' in native_recipe
            and 'kind === "ipma" && preprocessing !== "standardized"' in native_recipe
            and 'ipma_targets: requiredSingleTarget(settings.ipmaTargets)' in native_recipe,
            "Native recipe descriptors promote only documented PLSc, WPLS, prediction, IPMA, and NCA scopes, with explicit method-specific guards.",
        ),
        check(
            "native_results_cover_second_batch_and_effect_workflows",
            all(snippet in native_results for snippet in [
                'id: "plsc_reliability"',
                'id: "plsc_correlations"',
                'id: "wpls_weights"',
                '"plspredict_indicator_summary"',
                '"cvpat_benchmark_assessment"',
                '"plspredict_validation_plan"',
                '"plspredict_holdout_indicator_summary"',
                '"ipma_constructs"',
                '"ipma_indicators"',
                '"ipma_scope"',
                'const hasMediation = Boolean(result.mediation && specificIndirectEffects.effects.length);',
                'addTableGroup(groups, "mediation", "Mediation", MEDIATION_IDS, byId)',
                'addTableGroup(groups, "moderation", "Moderation", MODERATION_IDS, byId)',
                'addModerationInferenceTables(tables, run, moderationEstimates, constructLabel)',
                'tables.push({ ...draft, status: draft.status ?? "validated", rows });',
            ])
            and "nativeIpmaPredecessorIds" in native_results
            and "row.performance >= 0" in native_results
            and "row.performance <= 100" in native_results
            and 'if (!isCompletedResultRun(run)) return [];' in native_results
            and 'if (!rows.length) return;' in native_results
            and '"N/A"' not in native_results
            and {"mediation", "moderation"}.isdisjoint(observed_catalog_kinds),
            "Capability-derived native Results expose PLSc, WPLS, bounded prediction, predecessor-only 0-100 IPMA, mediation, and moderation tables only when genuine payload rows exist.",
        ),
        check(
            "cca_and_micom_mga_are_current_native_workflows",
            descriptors.get("cca") == ("cca", "validated", "validated_v1_2_3_cca_bounded_scope")
            and descriptors.get("mga") == ("mga", "validated", "validated_micom_v2_and_permutation_mga_v2_bounded_scope")
            and all(snippet in native_results for snippet in [
                '"cca_residual_summary"',
                '"cca_composite_residuals"',
                '"mga_group_summary"',
                '"mga_group_paths"',
                '"mga_group_r_squared"',
                '"mga_group_loadings"',
                '"mga_group_weights"',
                '"mga_path_differences"',
                '"mga_permutation"',
                '"micom_summary"',
                '"micom_composition"',
            ])
            and "packaged native run/XLSX export/save/reopen acceptance pass" in compatibility
            and "MICOM and Two-Group Permutation MGA | Validated for the documented v2 scope" in compatibility,
            "CCA is a packaged-accepted descriptive residual workflow and MICOM/MGA v2 is the bounded explicit-A/B measurement-invariance and permutation workflow.",
        ),
        check(
            "engine_messages_retain_bounded_claims",
            all(phrase in engine for phrase in [
                "PLS mediation effect decomposition is validated for the documented QuickPLS v1.2.1 scope when paired with validated bootstrap or permutation intervals",
                "Two-stage moderation is validated for the documented QuickPLS v1.2.1 single-interaction scope",
                "PLSc is validated for the documented QuickPLS v1.2.1 reflective path/factor-weighting scope",
                "WPLS is validated for the documented QuickPLS v1.2.1 positive case-weighted reflective path/factor-weighting scope",
                "PLSpredict indicator v2 is limited to the documented QuickPLS bounded scope",
                "It does not compare separately saved models",
                "IPMA v1 reports direct and indirect structural predecessors only",
                "observed sample range of listwise-standardized scores on a 0-100 scale",
                "Theoretical-range performance and cIPMA are unsupported",
                "CCA is validated for the documented QuickPLS v1.2.3 descriptive composite residual scope; bootstrap-based CCA decisions remain unsupported",
                "NCA v2 is limited to the documented numeric X/Y CE-FDH and CR-FDH scope with observed-range bottlenecks",
                "FIMIX-PLS v1 is validated for the documented QuickPLS v1.2.2 bounded deterministic 2-3 class score-space segmentation scope; full unrestricted EM/FIMIX parity is not claimed",
                "PROCESS-style regression v1 is validated for the documented QuickPLS v1.2.2 bounded mediation/moderation workflow scope; moderated mediation and the full Hayes model catalogue remain experimental",
            ]),
            "Estimator messages describe the exact validated numerical slices and preserve exclusions instead of labeling every later family experimental or implying full-method parity.",
        ),
        check(
            "plspredict_v2_is_packaged_indicator_first_native",
            descriptors.get("predict") == ("predict", "validated", "validated_plspredict_indicator_v2_and_cvpat_indicator_benchmarks_v2_bounded_scope")
            and prediction_promotion.get("passed") is True
            and all(token in native_results for token in ['"plspredict_indicator_summary"', '"cvpat_benchmark_assessment"', '"plspredict_validation_plan"'])
            and "plspredict_indicator_v2" in prediction_spec
            and "not a comparison of saved models" in prediction_spec,
            "PLSpredict/CVPAT v2 is accepted only after independent indicator/CVPAT reference evidence and a genuine packaged run/XLSX/save/reopen pass; saved-model comparison remains separate.",
        ),
        check(
            "nca_v2_is_packaged_model_free_native",
            descriptors.get("nca") == ("nca", "validated", "validated_nca_v2_bounded_scope")
            and "nca" in observed_catalog_kinds
            and all(token in native_results for token in ['"nca_ceiling_effects"', '"nca_cr_line"', '"nca_bottlenecks"', '"nca_scope"'])
            and "packaged-native setup/results/XLSX/save/reopen acceptance" in nca_spec
            and "packaged native setup/results/XLSX/export/reopen" in compatibility
            and nca_promotion.get("passed") is True,
            "NCA v2 is a packaged-accepted model-free native workflow while broader NCA variants remain excluded.",
        ),
        check(
            "micom_v2_is_owned_by_the_joint_third_batch_workflow",
            "micom" not in observed_catalog_kinds
            and descriptors.get("mga") == ("mga", "validated", "validated_micom_v2_and_permutation_mga_v2_bounded_scope")
            and "validated MICOM and permutation-MGA v2 scope requires path weighting" in core_validation
            and 'pub const MICOM_METHOD_VERSION: &str = "micom_v2"' in engine
            and "Status: validated for the bounded scope below" in micom_spec
            and "| Groups | Historical MICOM v1 | Withdrawn and execution-disabled" in compatibility,
            "MICOM v2 is exposed only through the bounded joint group workflow; micom_v1 remains legacy-only.",
        ),
        check(
            "fimix_and_regression_stay_bounded_engine_only",
            {"fimix", "regression"}.isdisjoint(observed_catalog_kinds)
            and not any(token in native_results for token in ["result.fimix", "result.regression", '"fimix_', '"regression_'])
            and 'groupMethods: mode === "predict" ? null : settings.groupMethods' in native_mode
            and 'methodTokens(settings.groupMethods, ["pls_pos", "fimix"])' in native_recipe
            and 'NATIVE_ANALYSIS_RECIPE_BOUNDS.fimixClassCount.minimum' in native_recipe
            and 'NATIVE_ANALYSIS_RECIPE_BOUNDS.fimixClassCount.maximum' in native_recipe
            and '{ group_methods: "fimix" }' in native_recipe
            and '{ fimix_classes: String(segmentCount) }' in native_recipe
            and descriptors.get("regression") == ("regression", "validated", "validated_v1_2_2_regression_bounded_scope")
            and 'assertEnum("regressionType", regressionType, ["ols", "logistic", "process"] as const)' in native_recipe
            and 'settings.robustSe !== "hc3"' in native_recipe
            and 'if (processModel === "moderated_mediation")' in native_recipe
            and "PROCESS-style moderated mediation remains outside the validated native regression scope." in native_recipe,
            "FIMIX is an explicit bounded Predict-recipe metadata path and Regression has bounded recipe guards, but compact Prediction clears hidden segmentation and neither workflow is claimed in the primary catalog or native Results.",
        ),
        check(
            "bounded_engine_backlog_is_not_overclaimed_as_native",
            BOUNDED_ENGINE_NATIVE_BACKLOG.isdisjoint(observed_catalog_kinds)
            and all(descriptors.get(kind, (None, None, None))[1] == "validated" for kind in BOUNDED_ENGINE_NATIVE_BACKLOG)
            and all(phrase in compatibility for phrase in [
                "bootstrap/permutation tetrad decision rules remain unsupported",
                "diagnostic only, not causal proof",
                "broader nonlinear SEM remains unsupported",
                "full Hayes PROCESS catalogue remains unsupported",
                "bootstrap, unrestricted multigroup/invariance, robust/ordinal/FIML estimators",
                "unrestricted GSCA variants remain unsupported",
                "unrestricted EM/FIMIX parity is not claimed",
                "moderated mediation, the full Hayes catalogue",
            ])
            and "Existing bounded engine support for any of those families is not evidence that its redesigned desktop workflow is finished." in native_redesign,
            "Later bounded engines remain documented with explicit exclusions and stay outside the primary native catalog until their desktop workflows are accepted.",
        ),
        check(
            "current_docs_and_evidence_accept_bounded_ipma",
            ipma_promotion.get("passed") is True
            and any(
                item.get("name") == "packaged_native_workflow" and item.get("passed") is True
                for item in ipma_promotion.get("checks", [])
            )
            and "Bounded IPMA using predecessor total effects and observed-range standardized-score performance" in promotion_program
            and "packaged execution, XLSX export, and save/reopen contracts pass" in promotion_program
            and "Native setup/results/map, strict persistence, genuine packaged execution, XLSX export, explicit save, and same-run reopen accepted" in compatibility
            and "Native-workflow promotion includes a current packaged-Tauri run, XLSX export, explicit save, and same-run reopen artifact" in ipma_spec
            and "It is not cIPMA" in ipma_spec
            and "source tests or synthetic completed-result fixtures are not substitutes" in ipma_spec,
            "IPMA promotion is accepted only because the strict promotion artifact proves the genuine packaged run/export/save/reopen workflow; bounded-scope and cIPMA exclusions remain mandatory.",
        ),
        check(
            "native_workbench_docs_name_ten_accepted_workflows",
            "ten packaged-accepted calculation workflows" in native_redesign
            and "Importance-Performance Map Analysis" in production_shell_doc
            and "Importance-Performance Map Analysis" in primary_workflow_doc
            and "ten-method calculation browser" in native_redesign
            and "CCA, IPMA, prediction, and NCA retain their narrower documented scopes" in native_redesign
            and "bounded Necessary Condition Analysis" in native_redesign,
            "The native workbench documentation records IPMA and model-free NCA alongside the ten accepted workflows while preserving bounded scopes.",
        ),
    ]

    passed = all(item["passed"] for item in checks)
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps({
        "schema_version": 1,
        "target": "v1_2_1_second_batch_method_promotion",
        "passed": passed,
        "semantics": {
            "primary_native_catalog": sorted(observed_catalog_kinds),
            "native_result_capabilities": [
                "PLSc",
                "WPLS",
                "CCA descriptive residuals",
                "IPMA predecessor importance/performance",
                "MGA explicit Group A/B permutation",
                "MICOM v2 measurement invariance",
                "PLSpredict / CVPAT v2",
                "Necessary Condition Analysis v2",
                "Mediation",
                "Moderation",
            ],
            "bounded_engine_contracts_not_primary_native": sorted(BOUNDED_ENGINE_NATIVE_BACKLOG),
            "numeric_validated_native_backlog": [],
            "packaged_native_promoted": ["cca", "ipma", "mga", "micom_v2", "nca", "plspredict_indicator_v2"],
            "bounded_metadata_workflows_not_primary_native": ["fimix", "pls_pos"],
            "legacy_only": ["micom_v1", "nca_v1"],
        },
        "checks": checks,
        "note": "Validated numerical or recipe support is not treated as packaged native acceptance. CCA, IPMA, MICOM/MGA v2, NCA v2, and indicator-level PLSpredict/CVPAT v2 require current packaged evidence; historical construct-score prediction v1, micom_v1, and nca_v1 remain legacy-only.",
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
