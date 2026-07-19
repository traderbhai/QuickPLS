"""Generate the v0.1-v0.8 publication promotion matrix.

This is an audit inventory, not evidence that methods are publication-ready.
Rows stay open until their required evidence artifacts exist and are reviewed.
"""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "publication_promotion_matrix.json"


METHODS = [
    {
        "id": "pls_pm",
        "release": "v0.3",
        "family": "PLS-SEM",
        "name": "PLS path modeling core",
        "current_status": "validated_estimator_scope",
        "promotion_status": "open",
        "required_evidence": [
            "stable run-envelope separation from experimental assessment/inference",
            "broader published example replication",
            "large-model benchmark",
            "known-difference register",
            "stable export tests",
        ],
        "existing_evidence": [
            "validation/results/pls_csem_comparison.json",
            "validation/results/pls_plspm_comparison.json",
            "validation/results/pls_pca_numpy_comparison.json",
            "validation/results/pls_csem_threecommonfactors_comparison.json",
        ],
    },
    {
        "id": "assessment",
        "release": "v0.4",
        "family": "Assessment",
        "name": "Reliability, validity, structural quality, and fit diagnostics",
        "current_status": "experimental",
        "promotion_status": "open",
        "required_evidence": [
            "metric-by-metric formula matrix",
            "cSEM/seminr/lavaan/hand references where applicable",
            "published worked examples for each metric",
            "degenerate-warning fixtures",
            "stable export label and precision tests",
        ],
        "existing_evidence": [
            "validation/results/v04_assessment_evidence.json",
            "validation/results/rho_a_csem_comparison.json",
            "validation/results/htmt_csem_comparison.json",
            "validation/results/htmt_seminr_comparison.json",
        ],
    },
    {
        "id": "inference",
        "release": "v0.4",
        "family": "Inference",
        "name": "Bootstrap, BCa, studentized bootstrap, jackknife, and permutation",
        "current_status": "experimental",
        "promotion_status": "open",
        "required_evidence": [
            "expanded Monte Carlo coverage across distribution families",
            "type-I error and power by procedure",
            "10,000-resample performance benchmark",
            "worker-count invariance for every stochastic payload",
            "stable export and reproducibility appendix tests",
        ],
        "existing_evidence": [
            "validation/results/monte_carlo_qualification.json",
            "validation/results/monte_carlo_studentized_qualification.json",
            "validation/results/studentized_worker_matrix.json",
            "validation/results/studentized_release_stress.json",
        ],
    },
    {
        "id": "extended_pls",
        "release": "v0.5",
        "family": "PLS-SEM",
        "name": "Mediation, moderation, HOC, PLSc, WPLS, CCA, CTA-PLS, endogeneity, nonlinear effects, controls",
        "current_status": "experimental",
        "promotion_status": "open",
        "required_evidence": [
            "method-specific two-reference matrix",
            "broader simulation/recovery studies",
            "inference coverage for promoted effects",
            "edge-case diagnostics",
            "known differences by method",
        ],
        "existing_evidence": [
            "validation/results/v05_extended_pls_evidence.json",
            "validation/results/mediation_reference_report.json",
            "validation/results/moderation_reference_report.json",
            "validation/results/higher_order_reference_report.json",
            "validation/results/plsc_reference_report.json",
            "validation/results/wpls_reference_report.json",
            "validation/results/cca_reference_report.json",
            "validation/results/cta_pls_reference_report.json",
        ],
    },
    {
        "id": "prediction_groups",
        "release": "v0.6",
        "family": "Prediction and groups",
        "name": "PLSpredict, CVPAT, IPMA, MICOM, MGA, FIMIX-PLS, PLS-POS",
        "current_status": "experimental",
        "promotion_status": "open",
        "required_evidence": [
            "larger external reference suite",
            "prediction leakage and CVPAT false-positive studies",
            "group-invariance simulation matrix",
            "segmentation recovery and null leakage matrix",
            "performance benchmark",
        ],
        "existing_evidence": [
            "validation/results/plspredict_holdout_reference_report.json",
            "validation/results/ipma_reference_report.json",
            "validation/results/mga_reference_report.json",
            "validation/results/v06_group_methods_reference_report.json",
        ],
    },
    {
        "id": "cbsem",
        "release": "v0.7",
        "family": "CB-SEM",
        "name": "CFA and maximum-likelihood SEM",
        "current_status": "experimental",
        "promotion_status": "open",
        "required_evidence": [
            "expanded lavaan parity suite",
            "second-source semopy or independent Python fixtures",
            "constrained multigroup refits or explicit stable blocking",
            "inadmissibility and optimizer failure tests",
            "fit/MI/SE/standardization tolerance matrix",
        ],
        "existing_evidence": [
            "validation/results/cbsem_v07_reference_report.json",
            "validation/results/cbsem_lavaan_reference_report.json",
        ],
    },
    {
        "id": "extended_methods",
        "release": "v0.8",
        "family": "Extended methods",
        "name": "PCA, OLS, logistic, PROCESS, NCA, GSCA",
        "current_status": "experimental",
        "promotion_status": "open",
        "required_evidence": [
            "R/Python second-source validation for PCA/OLS/logistic/NCA",
            "PROCESS model catalogue and bootstrap CI validation",
            "full independent GSCA ALS engine before GSCA promotion",
            "edge-case diagnostics",
            "performance evidence",
        ],
        "existing_evidence": [
            "validation/results/v08_extended_methods_reference_report.json",
        ],
    },
    {
        "id": "product_exports",
        "release": "cross-cutting",
        "family": "Product",
        "name": "GUI, SEM diagram, stable exports, manuals, benchmarks",
        "current_status": "experimental",
        "promotion_status": "open",
        "required_evidence": [
            "diagram editor screenshot QA",
            "GUI/CLI parity for every promoted method",
            "CSV/XLSX/HTML/PDF/SVG/PNG stable export tests",
            "methodology manual",
            "known-differences register",
            "benchmark report",
        ],
        "existing_evidence": [
            "docs/PUBLICATION_READY_AUDIT.md",
        ],
    },
]


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    rows = []
    for item in METHODS:
        existing = [
            {"path": path, "present": (ROOT / path).exists()}
            for path in item["existing_evidence"]
        ]
        rows.append({**item, "existing_evidence": existing})
    report = {
        "schema_version": 1,
        "target": "publication_ready_v0_1_to_v0_8",
        "passed": False,
        "row_count": len(rows),
        "rows": rows,
        "note": "All promotion_status values remain open until the required evidence is generated and reviewed.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
