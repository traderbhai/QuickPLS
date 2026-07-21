"""Generate the v1.2 method-promotion matrix.

The matrix is a working backlog for moving methods from experimental preview
output to researcher-ready validated output. It is intentionally conservative:
rows are promotion-ready only when all required artifact checks are present.
"""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "method_promotion_matrix_v1_2.json"
LEGACY_OUTPUT = RESULTS / "publication_promotion_matrix.json"


def artifact(path: str, purpose: str, required: bool = True) -> dict:
    return {
        "path": path,
        "purpose": purpose,
        "required": required,
        "present": (ROOT / path).exists(),
    }


FIRST_BATCH = [
    {
        "id": "pls_core_run_envelope",
        "family": "PLS-SEM",
        "method": "PLS core estimator output",
        "current_status": "validated_estimator_scope",
        "target_status": "validated",
        "promotion_batch": 1,
        "candidate_scope": (
            "Mode A/B PLS-PM with documented weighting schemes, preprocessing, scores, "
            "loadings, weights, paths, effects, R2, diagnostics, GUI/CLI parity, and estimator-only stable exports."
        ),
        "required_evidence": [
            "method specification",
            "two independent deterministic references",
            "published example",
            "metamorphic tests",
            "GUI/CLI serialized parity",
            "stable export parity",
            "known differences",
            "benchmark evidence",
        ],
        "artifacts": [
            artifact("docs/methods/PLS_PM_V1.md", "method specification"),
            artifact("validation/results/pls_csem_comparison.json", "cSEM reference"),
            artifact("validation/results/pls_plspm_comparison.json", "python-plspm reference"),
            artifact("validation/results/pls_pca_numpy_comparison.json", "NumPy PCA weighting reference"),
            artifact("validation/results/pls_csem_threecommonfactors_comparison.json", "published cSEM fixture"),
            artifact("validation/results/pls_publication_audit.json", "publication audit"),
            artifact("validation/results/pls_core_method_promotion_audit.json", "v1.2 PLS core promotion audit"),
            artifact("validation/results/stable_export_publication_audit.json", "stable export audit"),
            artifact("docs/KNOWN_DIFFERENCES.md", "known differences"),
        ],
        "blocking_questions": [],
        "scope_decisions": [
            "Stable PLS core output is estimator-only.",
            "Assessment and inference remain separate first-batch promotion rows.",
            "Full PLS run-envelope researcher-ready status is not claimed until assessment and inference are promoted.",
        ],
    },
    {
        "id": "assessment_metrics",
        "family": "Assessment",
        "method": "Reliability, validity, and quality metrics",
        "current_status": "experimental_family_label",
        "target_status": "validated",
        "promotion_batch": 1,
        "candidate_scope": (
            "Alpha, rho_A, rho_C, AVE, cross-loadings, Fornell-Larcker, HTMT/HTMT+, "
            "VIF, R2/adjusted R2, f2, Q2, SRMR, and d_ULS for documented PLS settings."
        ),
        "required_evidence": [
            "metric-by-metric formula matrix",
            "two-source references where available",
            "published fixtures",
            "degenerate applicability diagnostics",
            "export label and precision checks",
            "known HTMT/rho_A convention differences",
        ],
        "artifacts": [
            artifact("validation/results/assessment_publication_audit.json", "publication audit"),
            artifact("validation/results/assessment_method_promotion_audit.json", "v1.2 assessment promotion audit"),
            artifact("validation/results/assessment_publication_metric_matrix.json", "metric matrix"),
            artifact("validation/results/v04_assessment_evidence.json", "assessment evidence index"),
            artifact("validation/results/rho_a_primary_dijkstra_henseler_2015.json", "primary-paper rho_A evidence"),
            artifact("validation/results/rho_a_csem_comparison.json", "cSEM rho_A reference"),
            artifact("validation/results/htmt_csem_comparison.json", "cSEM HTMT reference"),
            artifact("validation/results/htmt_seminr_comparison.json", "seminr HTMT+ reference"),
            artifact("validation/results/assessment_csem_comparison.json", "cSEM assessment reference"),
            artifact("docs/KNOWN_DIFFERENCES.md", "known differences"),
        ],
        "blocking_questions": [],
        "scope_decisions": [
            "Validated assessment scope includes only the metrics listed in assessment_method_promotion_audit.json.",
            "d_G, NFI, and RMS_theta remain excluded until separately implemented and audited.",
            "Inference/resampling remains a separate first-batch promotion row.",
        ],
    },
    {
        "id": "inference_resampling",
        "family": "Inference",
        "method": "Bootstrap, BCa, studentized bootstrap, jackknife, and permutation",
        "current_status": "experimental_family_label",
        "target_status": "validated",
        "promotion_batch": 1,
        "candidate_scope": (
            "Documented PLS resampling settings with indexed ChaCha streams, percentile, BCa, "
            "studentized intervals, jackknife, and Freedman-Lane permutation where audited."
        ),
        "required_evidence": [
            "fixed-seed reproducibility",
            "worker-count invariance",
            "external matched-resample references",
            "Monte Carlo coverage/type-I evidence",
            "studentized release-stress benchmark",
            "cancellation and failed-replicate diagnostics",
            "stable export provenance",
        ],
        "artifacts": [
            artifact("validation/results/inference_publication_audit.json", "publication audit"),
            artifact("validation/results/inference_method_promotion_audit.json", "v1.2 inference promotion audit"),
            artifact("validation/results/inference_publication_matrix.json", "procedure matrix"),
            artifact("validation/results/monte_carlo_qualification.json", "percentile/BCa Monte Carlo qualification"),
            artifact("validation/results/monte_carlo_studentized_qualification.json", "studentized Monte Carlo qualification"),
            artifact("validation/results/studentized_worker_matrix.json", "worker invariance"),
            artifact("validation/results/studentized_release_stress.json", "release-stress benchmark"),
            artifact("validation/results/pls_bootstrap_external_reference.json", "cSEM matched resample reference"),
            artifact("validation/results/pls_bootstrap_plspm_external_reference.json", "python-plspm matched resample reference"),
        ],
        "blocking_questions": [],
        "scope_decisions": [
            "Percentile bootstrap, BCa, studentized/bootstrap-t, jackknife support, and Freedman-Lane path permutation are promoted together for documented PLS inference settings.",
            "Small-sample and non-normal claims are limited to the audited Monte Carlo qualification cells and documented diagnostics.",
            "Inference does not promote experimental base estimators or unsupported model shapes.",
        ],
    },
    {
        "id": "pca",
        "family": "Components",
        "method": "Standalone PCA",
        "current_status": "experimental",
        "target_status": "validated",
        "promotion_batch": 1,
        "candidate_scope": (
            "Standardized numeric raw-data PCA with deterministic sign orientation, eigenvalues, "
            "explained variance, loadings, weights, scores, and documented retention rules."
        ),
        "required_evidence": [
            "NumPy eigensystem reference",
            "second source or hand SVD fixture",
            "missing/constant/high-dimensional guards",
            "component sign and order metamorphic checks",
            "GUI/CLI/export parity",
            "known-difference note for sign orientation",
        ],
        "artifacts": [
            artifact("docs/methods/PCA_V1.md", "method specification"),
            artifact("validation/results/pca_method_promotion_audit.json", "v1.2 PCA promotion audit"),
            artifact("validation/results/v08_extended_methods_reference_report.json", "integrated NumPy/Python reference"),
            artifact("validation/results/extended_methods_publication_audit.json", "publication audit"),
            artifact("docs/KNOWN_DIFFERENCES.md", "known differences"),
        ],
        "blocking_questions": [],
        "scope_decisions": [
            "Standalone PCA is promoted for standardized numeric raw-data PCA with listwise deletion and deterministic sign orientation.",
            "Rotation methods, pairwise deletion, covariance/correlation-only input, nonnumeric variables, and inference remain excluded.",
            "Logistic regression, PROCESS, NCA, and GSCA remain separate v0.8 experimental methods.",
        ],
    },
    {
        "id": "ols_regression",
        "family": "Regression",
        "method": "OLS regression",
        "current_status": "experimental",
        "target_status": "validated",
        "promotion_batch": 1,
        "candidate_scope": (
            "OLS with intercept, controls, HC0/HC3/HC4 standard errors, confidence intervals, "
            "fit diagnostics, predictions, residual diagnostics, and documented rank-deficiency guards."
        ),
        "required_evidence": [
            "independent Python reference",
            "R lm/sandwich/lmtest reference",
            "rank deficiency and collinearity guards",
            "robust-SE formula tests",
            "bootstrap interval qualification if promoted",
            "GUI/CLI/export parity",
        ],
        "artifacts": [
            artifact("docs/methods/REGRESSION_OLS_V1.md", "method specification"),
            artifact("validation/results/ols_method_promotion_audit.json", "v1.2 OLS promotion audit"),
            artifact("validation/results/v08_extended_methods_reference_report.json", "integrated Python reference"),
            artifact("validation/results/extended_methods_publication_audit.json", "publication audit"),
        ],
        "blocking_questions": [],
        "scope_decisions": [
            "OLS is promoted for raw-data numeric complete-case regression with intercept, controls, HC3 standard errors, fit diagnostics, fitted values, and residuals.",
            "Logistic regression and PROCESS remain separate experimental methods.",
            "HC0 and HC4 public claims remain excluded until the engine honors robust_se selection.",
        ],
    },
]

SECOND_BATCH = [
    {
        "id": "mediation",
        "family": "PLS-SEM",
        "method": "PLS mediation effect decomposition",
        "current_status": "experimental",
        "target_status": "validated",
        "promotion_batch": 2,
        "candidate_scope": "Direct, indirect, total, VAF, classification, and validated bootstrap/permutation indirect-effect interpretation for documented PLS path models.",
        "required_evidence": ["independent Python reference", "R base-lm reference", "published example", "metamorphic checks", "randomization screen", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/MEDIATION_V1.md", "method specification alias"),
            artifact("validation/results/mediation_method_promotion_audit.json", "v1.2.1 mediation promotion audit"),
            artifact("validation/results/mediation_reference_report.json", "independent Python reference"),
            artifact("validation/results/mediation_r_reference_report.json", "R base-lm reference"),
            artifact("validation/results/mediation_published_example_report.json", "published example"),
            artifact("validation/results/mediation_metamorphic_report.json", "metamorphic evidence"),
            artifact("validation/results/mediation_randomization_report.json", "randomization evidence"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["PROCESS-style mediation and moderated mediation remain experimental."],
    },
    {
        "id": "moderation",
        "family": "PLS-SEM",
        "method": "Two-stage moderation",
        "current_status": "experimental",
        "target_status": "validated",
        "promotion_batch": 2,
        "candidate_scope": "Single-interaction two-stage moderation with generated product-score construct and simple slopes at standardized -1/0/+1.",
        "required_evidence": ["independent Python reference", "R base-lm reference", "published fixtures", "simulation", "inference qualification", "coverage screen"],
        "artifacts": [
            artifact("docs/methods/TWO_STAGE_MODERATION_V1.md", "method specification alias"),
            artifact("validation/results/moderation_method_promotion_audit.json", "v1.2.1 moderation promotion audit"),
            artifact("validation/results/moderation_reference_report.json", "independent reference"),
            artifact("validation/results/moderation_r_reference_report.json", "R reference"),
            artifact("validation/results/moderation_published_formula_report.json", "published formula fixture"),
            artifact("validation/results/moderation_published_empirical_report.json", "published empirical fixture"),
            artifact("validation/results/moderation_simulation_report.json", "simulation evidence"),
            artifact("validation/results/moderation_inference_qualification_report.json", "inference qualification"),
            artifact("validation/results/moderation_coverage_qualification_report.json", "coverage qualification"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Moderated mediation and broader interaction systems remain experimental."],
    },
    {
        "id": "plsc",
        "family": "PLS-SEM",
        "method": "Consistent PLS",
        "current_status": "experimental",
        "target_status": "validated",
        "promotion_batch": 2,
        "candidate_scope": "Reflective-only PLSc with path/factor weighting, rho_A attenuation correction, corrected paths/loadings/R2.",
        "required_evidence": ["method spec", "independent Python reference", "unsupported guards", "rho_A second-source evidence"],
        "artifacts": [
            artifact("docs/methods/PLSC_V1.md", "method specification"),
            artifact("validation/results/plsc_method_promotion_audit.json", "v1.2.1 PLSc promotion audit"),
            artifact("validation/results/plsc_reference_report.json", "independent reference"),
            artifact("validation/results/plsc_unsupported_guard_report.json", "unsupported guard evidence"),
            artifact("validation/results/rho_a_csem_comparison.json", "rho_A second source"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Formative PLSc and PCA-weighting PLSc remain unsupported."],
    },
    {
        "id": "wpls",
        "family": "PLS-SEM",
        "method": "Weighted PLS",
        "current_status": "experimental",
        "target_status": "validated",
        "promotion_batch": 2,
        "candidate_scope": "Positive case-weighted reflective WPLS with standardized preprocessing and path/factor weighting.",
        "required_evidence": ["method spec", "independent weighted-PLS reference", "invalid weight guards", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/PLS_WPLS_V1.md", "method specification"),
            artifact("validation/results/wpls_method_promotion_audit.json", "v1.2.1 WPLS promotion audit"),
            artifact("validation/results/wpls_reference_report.json", "independent reference and guards"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["WPLS inference, formative blocks, generated interactions/HOC, and PCA weighting remain unsupported."],
    },
    {
        "id": "ipma",
        "family": "Prediction",
        "method": "IPMA / cIPMA",
        "current_status": "experimental",
        "target_status": "validated",
        "promotion_batch": 2,
        "candidate_scope": "Bounded IPMA using PLS total effects as importance and 0-100 standardized-score performance.",
        "required_evidence": ["method spec", "independent Python reference", "target-selection checks", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/IPMA_V1.md", "method specification"),
            artifact("validation/results/ipma_method_promotion_audit.json", "v1.2.1 IPMA promotion audit"),
            artifact("validation/results/ipma_reference_report.json", "independent reference"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Broader cIPMA extensions remain unsupported."],
    },
    {
        "id": "plspredict",
        "family": "Prediction",
        "method": "PLSpredict / CVPAT",
        "current_status": "experimental",
        "target_status": "validated",
        "promotion_batch": 2,
        "candidate_scope": "Deterministic holdout, repeated k-fold, construct-score LM benchmark, Q2 predict, RMSE/MAE, and bounded CVPAT diagnostics.",
        "required_evidence": ["method spec", "leakage-free independent reference", "CVPAT checks", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/PLSPREDICT_V1.md", "method specification alias"),
            artifact("validation/results/plspredict_method_promotion_audit.json", "v1.2.1 PLSpredict promotion audit"),
            artifact("validation/results/plspredict_holdout_reference_report.json", "independent reference"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Separate saved-model CVPAT and indicator-level PLSpredict remain unsupported."],
    },
    {
        "id": "nca",
        "family": "NCA",
        "method": "CE-FDH and CR-FDH",
        "current_status": "experimental",
        "target_status": "validated",
        "promotion_batch": 2,
        "candidate_scope": "Numeric X/Y CE-FDH and CR-FDH NCA with deterministic permutation p values and bottleneck tables.",
        "required_evidence": ["method spec", "independent Python reference", "constant/nonnumeric guards", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/NCA_V1.md", "method specification"),
            artifact("validation/results/nca_method_promotion_audit.json", "v1.2.1 NCA promotion audit"),
            artifact("validation/results/v08_extended_methods_reference_report.json", "integrated independent reference"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Broader NCA ceiling variants and nonnumeric variables remain unsupported."],
    },
]


LATER_BATCHES = [
    {
        "id": "extended_pls_batch",
        "promotion_batch": 2,
        "methods": [
            "moderated mediation",
            "CCA",
            "CTA-PLS",
            "higher-order constructs",
            "endogeneity diagnostics",
            "nonlinear effects",
        ],
        "main_missing_evidence": [
            "method-specific second-source matrix",
            "larger simulation/recovery studies",
            "inference coverage for promoted effects",
            "edge-case diagnostics",
        ],
    },
    {
        "id": "prediction_groups_batch",
        "promotion_batch": 3,
        "methods": ["MICOM", "MGA", "FIMIX-PLS", "PLS-POS"],
        "main_missing_evidence": [
            "prediction leakage false-positive screens",
            "group-invariance simulation matrix",
            "segmentation null/recovery matrix",
            "larger performance benchmark",
        ],
    },
    {
        "id": "cbsem_gsca_batch",
        "promotion_batch": 4,
        "methods": ["CB-SEM/CFA", "GSCA"],
        "main_missing_evidence": [
            "broader lavaan parity",
            "second-source SEM validation",
            "robust inadmissibility and optimizer tests",
            "full independent GSCA ALS validation",
        ],
    },
    {
        "id": "extended_methods_batch",
        "promotion_batch": 3,
        "methods": ["logistic regression", "PROCESS-style workflows"],
        "main_missing_evidence": [
            "R/Python second-source split reports",
            "bootstrap/permutation qualification",
            "edge cases and performance evidence",
        ],
    },
]


def summarize_row(row: dict) -> dict:
    missing = [item["path"] for item in row["artifacts"] if item["required"] and not item["present"]]
    return {
        **row,
        "artifact_summary": {
            "required": sum(1 for item in row["artifacts"] if item["required"]),
            "present": sum(1 for item in row["artifacts"] if item["required"] and item["present"]),
            "missing": missing,
        },
        "promotion_ready": not missing and not row["blocking_questions"],
    }


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    rows = [summarize_row(row) for row in FIRST_BATCH + SECOND_BATCH]
    report = {
        "schema_version": 2,
        "target": "v1_2_method_promotion_program",
        "passed": False,
        "first_batch_rows": sum(1 for row in rows if row["promotion_batch"] == 1),
        "first_batch_promotion_ready": sum(1 for row in rows if row["promotion_batch"] == 1 and row["promotion_ready"]),
        "second_batch_rows": sum(1 for row in rows if row["promotion_batch"] == 2),
        "second_batch_promotion_ready": sum(1 for row in rows if row["promotion_batch"] == 2 and row["promotion_ready"]),
        "rows": rows,
        "later_batches": LATER_BATCHES,
        "decision_rule": (
            "A method can be promoted only after its row has no missing required artifacts, "
            "all blocking questions are resolved into docs or tests, and product/export labels "
            "are updated to expose validated output only for the promoted scope."
        ),
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    # Keep the old path as a compatibility alias for existing publication scripts.
    LEGACY_OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(
        f"wrote {OUTPUT} | rows={len(rows)} | "
        f"batch1_ready={report['first_batch_promotion_ready']}/{report['first_batch_rows']} | "
        f"batch2_ready={report['second_batch_promotion_ready']}/{report['second_batch_rows']}"
    )


if __name__ == "__main__":
    main()
