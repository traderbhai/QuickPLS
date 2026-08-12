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


def artifact(path: str, purpose: str, required: bool = True, require_passed: bool = False) -> dict:
    source = ROOT / path
    report_passed = None
    if source.exists() and require_passed:
        try:
            report_passed = json.loads(source.read_text(encoding="utf-8")).get("passed") is True
        except (json.JSONDecodeError, OSError):
            report_passed = False
    return {
        "path": path,
        "purpose": purpose,
        "required": required,
        "present": source.exists(),
        "require_passed": require_passed,
        "passed": report_passed,
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
        "current_status": "validated_packaged_native_bounded_scope",
        "target_status": "validated_packaged_native_bounded_scope",
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
            "strict append/save/reopen and genuine packaged-native workflow",
            "known-difference note for sign orientation",
        ],
        "artifacts": [
            artifact("docs/methods/PCA_V1.md", "method specification"),
            artifact("validation/results/pca_method_promotion_audit.json", "v1.2 PCA promotion audit"),
            artifact("validation/results/v08_extended_methods_reference_report.json", "integrated NumPy/Python reference"),
            artifact("validation/results/extended_methods_publication_audit.json", "publication audit"),
            artifact("validation/results/v247_native_desktop_visual_acceptance.json", "responsive native setup evidence", require_passed=True),
            artifact("validation/results/v247_tauri_native_acceptance.json", "packaged native run/export/save/reopen evidence", require_passed=True),
            artifact("docs/KNOWN_DIFFERENCES.md", "known differences"),
        ],
        "blocking_questions": [],
        "scope_decisions": [
            "Standalone PCA is promoted for standardized numeric raw-data PCA with listwise deletion and deterministic sign orientation.",
            "Rotation methods, pairwise deletion, covariance/correlation-only input, nonnumeric variables, and inference remain excluded.",
            "The native PCA workflow is model-free and table-only; full scores are materialized for export rather than duplicated in the ordinary Results surface.",
        ],
    },
    {
        "id": "ols_regression",
        "family": "Regression",
        "method": "OLS regression",
        "current_status": "validated_packaged_native_bounded_scope",
        "target_status": "validated_packaged_native_bounded_scope",
        "promotion_batch": 1,
        "candidate_scope": (
            "Raw numeric OLS with intercept, optional controls, listwise deletion, fixed HC3 standard errors, "
            "two-sided 95% intervals, fit diagnostics, fitted values, residuals, and rank-deficiency guards."
        ),
        "required_evidence": [
            "independent Python reference",
            "R lm/sandwich/lmtest reference",
            "rank deficiency and collinearity guards",
            "robust-SE formula tests",
            "strict append/save/reopen contract",
            "responsive native setup and genuine packaged execution",
            "table-only fitted/residual XLSX export",
        ],
        "artifacts": [
            artifact("docs/methods/REGRESSION_OLS_V1.md", "method specification"),
            artifact("validation/results/ols_method_promotion_audit.json", "strict v1.2 OLS promotion audit", require_passed=True),
            artifact("validation/results/v08_extended_methods_reference_report.json", "integrated Python reference"),
            artifact("validation/results/extended_methods_publication_audit.json", "publication audit"),
            artifact("validation/results/v247_native_desktop_visual_acceptance.json", "responsive native OLS setup evidence", require_passed=True),
            artifact("validation/results/v247_tauri_native_acceptance.json", "packaged-native OLS evidence consumed by the strict audit", require_passed=True),
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
            artifact("docs/methods/PLSC_V2.md", "current method specification"),
            artifact("docs/methods/PLSC_V1.md", "legacy compatibility disclosure"),
            artifact("validation/results/plsc_method_promotion_audit.json", "v1.2.1 PLSc promotion audit"),
            artifact("validation/results/plsc_reference_report.json", "independent reference"),
            artifact("validation/results/plsc_unsupported_guard_report.json", "unsupported guard evidence"),
            artifact("validation/results/rho_a_csem_comparison.json", "rho_A second source"),
        ],
        "blocking_questions": [],
        "scope_decisions": [
            "Formative PLSc and PCA-weighting PLSc remain unsupported.",
            "plsc_v1 remains readable as a flagged legacy result; new analyses emit canonical plsc_v2.",
        ],
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
        "method": "Importance-Performance Map Analysis (IPMA)",
        "current_status": "ipma_v1_packaged_native_validated_bounded_scope",
        "target_status": "packaged_native_validated_bounded_scope",
        "promotion_batch": 2,
        "candidate_scope": "Bounded predecessor-only IPMA using PLS total effects as importance and observed-sample 0-100 performance from listwise-standardized scores.",
        "required_evidence": ["method spec", "independent Python reference", "target/unrelated exclusion checks", "exact provenance and scale", "strict project persistence", "native setup/results/export source contract", "packaged native run/export/save/reopen"],
        "artifacts": [
            artifact("docs/methods/IPMA_V1.md", "method specification"),
            artifact(
                "validation/results/ipma_method_promotion_audit.json",
                "v1.2.1 IPMA promotion audit",
                require_passed=True,
            ),
            artifact("validation/results/ipma_reference_report.json", "independent reference"),
            artifact(
                "validation/results/v247_tauri_native_acceptance.json",
                "packaged native acceptance artifact",
                require_passed=True,
            ),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Theoretical-range correction, alternate SmartPLS map representations, NCA integration, and cIPMA remain unsupported."],
    },
    {
        "id": "plspredict",
        "family": "Prediction",
        "method": "PLSpredict / CVPAT",
        "current_status": "packaged_native_validated_bounded_scope",
        "target_status": "packaged_native_validated_bounded_scope",
        "promotion_batch": 2,
        "candidate_scope": "Indicator-level endogenous prediction with train-only preprocessing, fixed seeded balanced 10-fold by 10-repeat complete-case validation, IA and LM benchmarks, one-sided 95% single-model CVPAT, supplementary construct scores, and a secondary deterministic modulo-4 holdout.",
        "required_evidence": ["current and legacy method specs", "independent indicator and CVPAT reference", "strict append/save/reopen/tamper tests", "native result and export enforcement", "genuine packaged-native run/export/reopen evidence"],
        "artifacts": [
            artifact("docs/methods/PLSPREDICT_INDICATOR_V2.md", "current bounded indicator prediction and CVPAT specification"),
            artifact("docs/methods/PLSPREDICT_HOLDOUT_V1.md", "legacy archive compatibility specification"),
            artifact("validation/results/plspredict_method_promotion_audit.json", "strict current prediction promotion audit", require_passed=True),
            artifact("validation/results/plspredict_indicator_reference_report.json", "independent indicator prediction and paired CVPAT reference", require_passed=True),
            artifact("validation/results/v247_tauri_native_acceptance.json", "packaged native run/export/save/reopen evidence", require_passed=True),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Fold assignment is fixed and seeded rather than SmartPLS-randomized. Separate saved-model comparison, formative endogenous targets, HOCs, interactions, weights, and non-listwise missing handling remain unsupported."],
    },
    {
        "id": "nca",
        "family": "NCA",
        "method": "CE-FDH and CR-FDH",
        "current_status": "packaged_native_validated_bounded_scope",
        "target_status": "packaged_native_validated_bounded_scope",
        "promotion_batch": 2,
        "candidate_scope": "Standalone observed numeric X/Y CE-FDH and CR-FDH NCA v2 with listwise deletion, seeded permutation p values, observed-range bottlenecks, strict archive persistence, no phantom model, bounded native results/export, and no full-parity claim.",
        "required_evidence": ["current and legacy method specs", "independent CE/CR Python reference", "recipe and data guards", "strict append/save/reopen/tamper tests", "no-model snapshot test", "native results/export enforcement", "genuine packaged-native run/export/reopen evidence"],
        "artifacts": [
            artifact("docs/methods/NCA_V2.md", "current method specification"),
            artifact("docs/methods/NCA_V1.md", "legacy compatibility and supersession policy"),
            artifact("validation/results/nca_method_promotion_audit.json", "strict NCA v2 promotion audit", require_passed=True),
            artifact("validation/results/v08_extended_methods_reference_report.json", "integrated independent CE-FDH/CR-FDH reference"),
            artifact("validation/results/v247_tauri_native_acceptance.json", "packaged-native evidence consumed by the strict audit"),
        ],
        "blocking_questions": [],
        "scope_decisions": [
            "nca_v1 is archive-readable only and is not current scientific evidence; new runs emit nca_v2.",
            "The qualified backend scope is one observed numeric X and one observed numeric Y with CE-FDH/CR-FDH, listwise deletion, and observed-range bottlenecks.",
            "Multiple conditions, latent-score NCA, cIPMA integration, categorical/ordinal variables, alternative ceilings, and full SmartPLS or R-package parity remain unsupported.",
            "Packaged-native promotion is accepted only because the NCA-specific audit now passes; mere artifact presence remains insufficient.",
        ],
    },
]


THIRD_BATCH = [
    {
        "id": "micom",
        "family": "Groups",
        "method": "MICOM measurement invariance",
        "current_status": "validated_micom_v2_bounded_scope",
        "target_status": "validated",
        "promotion_batch": 3,
        "candidate_scope": "MICOM v2 Steps 1-3 for two explicitly ordered groups, with group-specific PLS re-estimation and 5,000-10,000 usable deterministic label permutations.",
        "required_evidence": ["group-specific implementation", "method spec", "independent numerical reference", "invariant/non-invariant evidence", "strict persistence", "native results/export/reopen enforcement"],
        "artifacts": [
            artifact("docs/methods/MICOM_V2.md", "current bounded method specification"),
            artifact("docs/methods/MICOM_V1.md", "withdrawn legacy compatibility disclosure"),
            artifact("validation/results/micom_v2_reference_report.json", "independent v2 numerical comparison"),
            artifact("validation/results/micom_method_promotion_audit.json", "strict MICOM v2 promotion audit"),
            artifact("validation/results/v247_tauri_native_acceptance.json", "packaged native run, XLSX export, save, and reopen evidence"),
        ],
        "blocking_questions": [],
        "scope_decisions": [
            "Historical micom_v1 payloads remain legacy-only and are not validated evidence.",
            "MICOM v2 runs only with path weighting, standardized listwise data, explicit configural confirmation, and the joint permutation-MGA v2 plan.",
            "Failed Step 2 blocks an invariance-supported MGA interpretation for that composite; failed Step 3 blocks full-invariance and pooling claims.",
        ],
    },
    {
        "id": "mga_permutation",
        "family": "Groups",
        "method": "Two-group permutation MGA",
        "current_status": "validated_v2_bounded_scope",
        "target_status": "validated",
        "promotion_batch": 3,
        "candidate_scope": "Two observed groups with group-specific PLS re-estimation, deterministic label permutation, empirical p values for paths/loadings/weights, and the companion MICOM v2 hierarchy.",
        "required_evidence": ["v2 method spec", "independent reference", "integrated permutation fixture", "strict persistence", "native product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/PLS_MGA_PERMUTATION_V2.md", "current method specification"),
            artifact("docs/methods/PLS_MGA_PERMUTATION_V1.md", "legacy compatibility disclosure"),
            artifact("validation/results/micom_v2_reference_report.json", "joint independent MICOM/MGA v2 reference"),
            artifact("validation/results/mga_permutation_method_promotion_audit.json", "strict permutation-MGA v2 promotion audit"),
            artifact("validation/results/v247_tauri_native_acceptance.json", "packaged native run, XLSX export, save, and reopen evidence"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Approximate-normal diagnostics are omitted from the native report; inference uses the deterministic two-tailed permutation tables and must be interpreted with the MICOM hierarchy."],
    },
    {
        "id": "pls_pos",
        "family": "Groups",
        "method": "PLS-POS",
        "current_status": "experimental",
        "target_status": "validated",
        "promotion_batch": 3,
        "candidate_scope": "Deterministic PLS-POS with 2-5 segments, deterministic starts, minimum segment-share guard, objective history, memberships, segment paths, and segment R2.",
        "required_evidence": ["method spec", "2-5 segment recovery", "null screen", "metamorphic checks", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/PLS_POS_V1.md", "method specification"),
            artifact("validation/results/segmentation_recovery_simulation_report.json", "segmentation recovery and null-screen evidence"),
            artifact("validation/results/v06_group_methods_reference_report.json", "integrated POS fixture"),
            artifact("validation/results/pls_pos_method_promotion_audit.json", "v1.2.2 PLS-POS promotion audit"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Legacy pls_pos_bounded_v1 remains readable but is not the promoted generalized PLS-POS scope."],
    },
    {
        "id": "fimix_pls",
        "family": "Groups",
        "method": "FIMIX-PLS",
        "current_status": "experimental",
        "target_status": "validated",
        "promotion_batch": 3,
        "candidate_scope": "Bounded deterministic 2-3 class score-space segmentation with class probabilities, memberships, class paths/R2, information criteria, and entropy.",
        "required_evidence": ["method spec", "2/3-class recovery", "homogeneous-null screen", "start-order invariance", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/FIMIX_PLS_V1.md", "method specification"),
            artifact("validation/results/v06_group_methods_reference_report.json", "integrated FIMIX fixture"),
            artifact("validation/results/fimix_pls_method_promotion_audit.json", "v1.2.2 FIMIX-PLS promotion audit"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Promotion does not claim unrestricted EM/FIMIX parity."],
    },
    {
        "id": "logistic_regression",
        "family": "Regression",
        "method": "Logistic regression",
        "current_status": "experimental",
        "target_status": "validated",
        "promotion_batch": 3,
        "candidate_scope": "Binary 0/1 numeric complete-case logistic regression with deterministic IRLS, Wald SE/z/p, odds ratios, predicted probabilities, log-likelihood, pseudo-R2, AIC, and BIC.",
        "required_evidence": ["method spec", "Python IRLS reference", "R glm reference", "separation/rank guards", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/REGRESSION_LOGISTIC_V1.md", "method specification"),
            artifact("validation/results/v08_extended_methods_reference_report.json", "integrated Python reference"),
            artifact("validation/results/extended_methods_publication_audit.json", "publication audit"),
            artifact("validation/results/logistic_method_promotion_audit.json", "v1.2.2 logistic promotion audit"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Multinomial, ordinal, weighted, clustered, and Firth-corrected models remain unsupported."],
    },
    {
        "id": "process",
        "family": "Regression",
        "method": "PROCESS-style mediation/moderation",
        "current_status": "experimental",
        "target_status": "validated",
        "promotion_batch": 3,
        "candidate_scope": "Bounded mediation and moderation workflows generated from OLS component models.",
        "required_evidence": ["method spec", "Python OLS equations", "R base-lm parity", "mediation/moderation fixtures", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/PROCESS_V1.md", "method specification"),
            artifact("validation/results/v08_extended_methods_reference_report.json", "integrated Python reference"),
            artifact("validation/results/mediation_method_promotion_audit.json", "mediation evidence"),
            artifact("validation/results/moderation_method_promotion_audit.json", "moderation evidence"),
            artifact("validation/results/process_method_promotion_audit.json", "v1.2.2 PROCESS promotion audit"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Moderated mediation and the full Hayes PROCESS catalogue remain experimental."],
    },
]


FOURTH_BATCH = [
    {
        "id": "higher_order",
        "family": "PLS-SEM",
        "method": "Higher-order constructs",
        "current_status": "validated_bounded_scope",
        "target_status": "validated",
        "promotion_batch": 4,
        "candidate_scope": "Repeated-indicator, two-stage, and documented hybrid higher-order construct estimation for supported PLS recipes.",
        "required_evidence": ["independent references", "metamorphic checks", "invalid hybrid guards", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/PLS_HIGHER_ORDER_V1.md", "method specification"),
            artifact("docs/methods/HIGHER_ORDER_CONSTRUCTS_V1.md", "method alias"),
            artifact("validation/results/higher_order_method_promotion_audit.json", "v1.2.3 HOC promotion audit"),
            artifact("validation/results/higher_order_reference_report.json", "repeated-indicator reference"),
            artifact("validation/results/higher_order_metamorphic_report.json", "metamorphic evidence"),
            artifact("validation/results/higher_order_two_stage_reference_report.json", "two-stage reference"),
            artifact("validation/results/higher_order_hybrid_reference_report.json", "hybrid reference"),
            artifact("validation/results/higher_order_hybrid_guard_report.json", "invalid hybrid guard"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Unsupported HOC variants and invalid hybrid splits remain blocked."],
    },
    {
        "id": "nonlinear_effects",
        "family": "PLS-SEM",
        "method": "Nonlinear effects",
        "current_status": "validated_bounded_scope",
        "target_status": "validated",
        "promotion_batch": 4,
        "candidate_scope": "Centered squared-term fixed-score nonlinear diagnostic for supported PLS structural paths.",
        "required_evidence": ["independent Python reference", "OLS equation checks", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/PLS_NONLINEAR_EFFECTS_V1.md", "method specification"),
            artifact("docs/methods/NONLINEAR_EFFECTS_V1.md", "method alias"),
            artifact("validation/results/nonlinear_effects_method_promotion_audit.json", "v1.2.3 nonlinear promotion audit"),
            artifact("validation/results/nonlinear_effects_reference_report.json", "independent reference"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Broader nonlinear SEM estimation remains unsupported."],
    },
    {
        "id": "endogeneity",
        "family": "PLS-SEM",
        "method": "Gaussian-copula endogeneity diagnostics",
        "current_status": "validated_bounded_scope",
        "target_status": "validated",
        "promotion_batch": 4,
        "candidate_scope": "Gaussian-copula diagnostic with rankit inverse-normal copula terms for screenable nonnormal predictor scores.",
        "required_evidence": ["independent Python reference", "applicability warnings", "diagnostic-only product language"],
        "artifacts": [
            artifact("docs/methods/PLS_GAUSSIAN_COPULA_ENDOGENEITY_V1.md", "method specification"),
            artifact("docs/methods/GAUSSIAN_COPULA_ENDOGENEITY_V1.md", "method alias"),
            artifact("validation/results/endogeneity_method_promotion_audit.json", "v1.2.3 endogeneity promotion audit"),
            artifact("validation/results/endogeneity_reference_report.json", "independent reference"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["The diagnostic is not causal proof."],
    },
    {
        "id": "cca",
        "family": "PLS-SEM",
        "method": "CCA",
        "current_status": "validated_bounded_scope",
        "target_status": "validated",
        "promotion_batch": 4,
        "candidate_scope": "Descriptive composite correlation residual diagnostic for recursive standardized PLS path models.",
        "required_evidence": ["independent reference", "invalid-settings guard", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/PLS_CCA_V1.md", "method specification"),
            artifact("docs/methods/CCA_V1.md", "method alias"),
            artifact("validation/results/cca_method_promotion_audit.json", "v1.2.3 CCA promotion audit"),
            artifact("validation/results/cca_reference_report.json", "independent reference"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Bootstrap-based CCA decisions remain unsupported."],
    },
    {
        "id": "cta_pls",
        "family": "PLS-SEM",
        "method": "CTA-PLS",
        "current_status": "validated_bounded_scope",
        "target_status": "validated",
        "promotion_batch": 4,
        "candidate_scope": "Descriptive sample-covariance tetrad diagnostic for valid indicator blocks with four or more indicators.",
        "required_evidence": ["independent reference", "invalid-block guard", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/PLS_CTA_PLS_V1.md", "method specification"),
            artifact("docs/methods/CTA_PLS_V1.md", "method alias"),
            artifact("validation/results/cta_pls_method_promotion_audit.json", "v1.2.3 CTA-PLS promotion audit"),
            artifact("validation/results/cta_pls_reference_report.json", "independent reference"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Bootstrap/permutation tetrad decision rules remain unsupported."],
    },
    {
        "id": "moderated_mediation",
        "family": "PLS-SEM",
        "method": "Moderated mediation",
        "current_status": "validated_bounded_scope",
        "target_status": "validated",
        "promotion_batch": 4,
        "candidate_scope": "Two-stage conditional indirect-effect diagnostic with standardized moderator levels -1, 0, and +1 plus index of moderated mediation.",
        "required_evidence": ["independent reference", "invalid-recipe guard", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/PLS_MODERATED_MEDIATION_V1.md", "method specification"),
            artifact("docs/methods/MODERATED_MEDIATION_V1.md", "method alias"),
            artifact("validation/results/moderated_mediation_method_promotion_audit.json", "v1.2.3 moderated mediation promotion audit"),
            artifact("validation/results/moderated_mediation_reference_report.json", "independent reference"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["The full Hayes PROCESS catalogue remains unsupported."],
    },
]


FIFTH_BATCH = [
    {
        "id": "cbsem_cfa",
        "family": "CB-SEM",
        "method": "CFA and maximum-likelihood SEM",
        "current_status": "validated_bounded_scope",
        "target_status": "validated",
        "promotion_batch": 5,
        "candidate_scope": "Raw-data single-group reflective CFA/SEM ML with marker identification, standardized solutions, residuals, fit indices, and modification-index diagnostics.",
        "required_evidence": ["lavaan parity", "publication audit", "optimizer/inadmissibility guards", "product/export enforcement"],
        "artifacts": [
            artifact("docs/methods/CBSEM_ML_V1.md", "SEM method specification"),
            artifact("docs/methods/CFA_ML_V1.md", "CFA method specification"),
            artifact("validation/results/cbsem_cfa_method_promotion_audit.json", "v1.2.4 CB-SEM/CFA promotion audit"),
            artifact("validation/results/cbsem_lavaan_reference_report.json", "lavaan reference"),
            artifact("validation/results/v07_cbsem_evidence.json", "CB-SEM evidence index"),
            artifact("validation/results/cbsem_publication_audit.json", "publication audit"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Bootstrap, unrestricted multigroup/invariance, robust/ordinal/FIML, and broad constraints remain experimental or unsupported."],
    },
    {
        "id": "gsca",
        "family": "Components",
        "method": "GSCA",
        "current_status": "validated_bounded_scope",
        "target_status": "validated",
        "promotion_batch": 5,
        "candidate_scope": "Bounded gsca_als_v2 joint global least-squares ALS with standardized raw data, disjoint reflective/formative blocks, recursive paths, weights, loadings, paths, R2, objective, FIT/AFIT/local FIT, GFI, SRMR, and convergence.",
        "required_evidence": ["independent global-criterion reference", "strict persistence", "packaged native lifecycle/export/reopen"],
        "artifacts": [
            artifact("docs/methods/GSCA_ALS_V2.md", "current method specification"),
            artifact("docs/methods/GSCA_V1.md", "legacy preview boundary"),
            artifact("validation/results/gsca_method_promotion_audit.json", "GSCA ALS v2 promotion audit"),
            artifact("validation/results/gsca_als_v2_reference_report.json", "independent global-criterion reference"),
            artifact("validation/results/v247_native_desktop_visual_acceptance.json", "browser setup and viewport acceptance"),
            artifact("validation/results/v247_tauri_native_acceptance_gsca.json", "genuine packaged workflow evidence"),
        ],
        "blocking_questions": [],
        "scope_decisions": ["Historical gsca_v1 is legacy preview-only; score export, inference, multigroup, and unrestricted GSCA variants remain unsupported unless separately audited."],
    },
]


LATER_BATCHES = [
    {
        "id": "extended_pls_batch",
        "promotion_batch": 4,
        "methods": [],
        "main_missing_evidence": [
            "No remaining extended PLS diagnostics after v1.2.3; future variants must be added as new rows.",
        ],
    },
    {
        "id": "prediction_groups_batch",
        "promotion_batch": 4,
        "methods": [],
        "main_missing_evidence": [
            "No remaining group/prediction methods in this batch after v1.2.2; keep this placeholder for future scope expansions.",
        ],
    },
    {
        "id": "cbsem_gsca_batch",
        "promotion_batch": 5,
        "methods": [],
        "main_missing_evidence": [
            "No remaining CB-SEM/CFA or GSCA scopes after v1.2.4; advanced variants remain post-v1.2.",
        ],
    },
    {
        "id": "extended_methods_batch",
        "promotion_batch": 4,
        "methods": [],
        "main_missing_evidence": [
            "No remaining logistic/PROCESS methods in this batch after v1.2.2 except moderated mediation, tracked under extended PLS.",
        ],
    },
]


def summarize_row(row: dict) -> dict:
    missing = [item["path"] for item in row["artifacts"] if item["required"] and not item["present"]]
    failed = [
        item["path"]
        for item in row["artifacts"]
        if item["required"] and item.get("require_passed") and item.get("passed") is not True
    ]
    return {
        **row,
        "artifact_summary": {
            "required": sum(1 for item in row["artifacts"] if item["required"]),
            "present": sum(1 for item in row["artifacts"] if item["required"] and item["present"]),
            "missing": missing,
            "failed": failed,
        },
        "promotion_ready": not missing and not failed and not row["blocking_questions"],
    }


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    rows = [summarize_row(row) for row in FIRST_BATCH + SECOND_BATCH + THIRD_BATCH + FOURTH_BATCH + FIFTH_BATCH]
    report = {
        "schema_version": 2,
        "target": "v1_2_method_promotion_program",
        "passed": all(row["promotion_ready"] for row in rows),
        "first_batch_rows": sum(1 for row in rows if row["promotion_batch"] == 1),
        "first_batch_promotion_ready": sum(1 for row in rows if row["promotion_batch"] == 1 and row["promotion_ready"]),
        "second_batch_rows": sum(1 for row in rows if row["promotion_batch"] == 2),
        "second_batch_promotion_ready": sum(1 for row in rows if row["promotion_batch"] == 2 and row["promotion_ready"]),
        "third_batch_rows": sum(1 for row in rows if row["promotion_batch"] == 3),
        "third_batch_promotion_ready": sum(1 for row in rows if row["promotion_batch"] == 3 and row["promotion_ready"]),
        "fourth_batch_rows": sum(1 for row in rows if row["promotion_batch"] == 4),
        "fourth_batch_promotion_ready": sum(1 for row in rows if row["promotion_batch"] == 4 and row["promotion_ready"]),
        "fifth_batch_rows": sum(1 for row in rows if row["promotion_batch"] == 5),
        "fifth_batch_promotion_ready": sum(1 for row in rows if row["promotion_batch"] == 5 and row["promotion_ready"]),
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
        f"batch2_ready={report['second_batch_promotion_ready']}/{report['second_batch_rows']} | "
        f"batch3_ready={report['third_batch_promotion_ready']}/{report['third_batch_rows']} | "
        f"batch4_ready={report['fourth_batch_promotion_ready']}/{report['fourth_batch_rows']} | "
        f"batch5_ready={report['fifth_batch_promotion_ready']}/{report['fifth_batch_rows']}"
    )


if __name__ == "__main__":
    main()
