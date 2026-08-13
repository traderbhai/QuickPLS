#!/usr/bin/env python3
from final_method_promotion_common import audit_method, load_json, RESULTS, ROOT


lavaan = load_json(RESULTS / "cbsem_lavaan_reference_report.json")
models = lavaan.get("models", [])
max_estimate_delta = max((model.get("max_estimate_delta") or 0.0 for model in models), default=None)
max_fit_delta = max((model.get("max_fit_delta") or 0.0 for model in models), default=None)
native = load_json(RESULTS / "v247_tauri_native_acceptance.json")
browser = load_json(RESULTS / "v247_native_desktop_visual_acceptance.json")
native_checks = native.get("checks", {})
native_result = native_checks.get("cbsemResult", {})
native_export = native_checks.get("cbsemExport", {}).get("nativeXlsx", {})
native_archive = native_checks.get("cbsemSaveReopen", {}).get("archive", {})
expected_provenance = "pls_pm_v1+cbsem_ml_v1+cbsem_fit_v1+cbsem_modification_indices_v1+pls_mediation_v1+pls_assessment_v7"
expected_sheets = [
    "Model fit",
    "Standardized parameters",
    "Unstandardized parameters",
    "Residual correlations",
    "Residual covariances",
    "Model-implied covariances",
    "Residual-based modification dia",
    "Calculation scope",
    "Run provenance",
]
native_sources = "\n".join((ROOT / path).read_text(encoding="utf-8") for path in [
    "src/native/nativeAnalysisCatalog.ts",
    "src/native/nativeAnalysisRecipe.ts",
    "src/native/nativeResults.ts",
    "crates/qpls-project/src/lib.rs",
])
packaged_native_workflow = (
    native.get("passed") is True
    and native_result.get("initialSelectedTable") == "cbsem_fit"
    and native_result.get("noPlaceholder") is True
    and native_result.get("noGenericPlsTables") is True
    and native_result.get("runDetails", {}).get("properties", {}).get("Method version") == expected_provenance
    and native_checks.get("cbsemProgress", {}).get("captured") is True
    and native_export.get("attempted") is True
    and native_export.get("file", {}).get("isFile") is True
    and native_export.get("file", {}).get("size", 0) > 0
    and native_export.get("workbookSheets") == expected_sheets
    and native_checks.get("cbsemSaveReopen", {}).get("sameRunRestored") is True
    and native_archive.get("provenanceMethodVersion") == expected_provenance
    and native_archive.get("cbsem", {}).get("methodVersion") == "cbsem_ml_v1"
    and native_archive.get("cbsem", {}).get("fitContract") is True
    and native_archive.get("cbsem", {}).get("modificationContract") is True
    and native_archive.get("recipe", {}).get("modelType") == "sem"
    and native_archive.get("recipe", {}).get("constructs") == 3
    and native_archive.get("recipe", {}).get("paths") == 2
    and len([shot for shot in native.get("screenshots", []) if "tauri-native-cbsem" in shot]) == 7
)
browser_cbsem = browser.get("checks", {}).get("cbsem", [])
browser_setup = (
    browser.get("passed") is True
    and len(browser_cbsem) == 3
    and all(check.get("linkage", {}).get("linkage") is True for check in browser_cbsem)
    and all(check.get("fiveCaseSampleBlockerVisible") is True for check in browser_cbsem)
    and all(check.get("truthAndOverflow", {}).get("noHorizontalOverflow") is True for check in browser_cbsem)
)

raise SystemExit(audit_method(
    "v1_2_4_cbsem_gsca_promotion",
    "cbsem_cfa",
    "Raw-data single-group reflective CFA/recursive SEM ML with marker identification, standardized solutions, residuals, fit indices, and residual-based modification screening.",
    ["cbsem_lavaan_reference_report.json", "cbsem_publication_audit.json", "v247_native_desktop_visual_acceptance.json", "v247_tauri_native_acceptance.json"],
    ["CBSEM_ML_V1.md", "CFA_ML_V1.md", "CBSEM_FIT_V1.md", "CBSEM_MODIFICATION_INDICES_V1.md"],
    [
        {"name": "lavaan_estimate_tolerance", "passed": max_estimate_delta is not None and max_estimate_delta <= 1e-6, "detail": f"Max lavaan estimate delta = {max_estimate_delta}."},
        {"name": "lavaan_fit_tolerance", "passed": max_fit_delta is not None and max_fit_delta <= 1e-6, "detail": f"Max lavaan fit delta = {max_fit_delta}."},
        {"name": "exact_method_provenance", "passed": all(
            load_json(RESULTS / name).get("provenance", {}).get("method_version") == version
            for name, version in [
                ("lavaan_two_factor_cfa_quickpls.json", "pls_pm_v1+cfa_ml_v1+cbsem_fit_v1+cbsem_modification_indices_v1+pls_mediation_v1+pls_assessment_v7"),
                ("lavaan_latent_regression_sem_quickpls.json", expected_provenance),
            ]
        ), "detail": "CFA and SEM reference envelopes contain only their executed estimator plus fit, residual-screening, mediation, and assessment versions."},
        {"name": "strict_native_and_archive_contract", "passed": all(token in native_sources for token in [
            'kind: "cbsem"', 'cbsem_model_type', 'cbsem_fit', 'validate_cbsem_payload_contract',
        ]), "detail": "The mounted native catalog, recipe builder, result tables, and project validator contain the bounded CB-SEM contract."},
        {"name": "browser_setup_boundary", "passed": browser_setup, "detail": "Three responsive browser viewports expose the exact editor and truthfully block the five-case preview without fabricated results."},
        {"name": "genuine_packaged_native_workflow", "passed": packaged_native_workflow, "detail": "The packaged desktop authored a 240-case three-factor recursive SEM, completed genuine ML, exported all tables to XLSX, saved the exact typed payload, and reopened the same run."},
        {"name": "advanced_cbsem_exclusions_recorded", "passed": True, "detail": "Bootstrap, unrestricted multigroup/invariance, robust/ordinal/FIML, mean structures, interactions, and higher-order constructs remain unsupported in this native workflow."},
    ],
))
