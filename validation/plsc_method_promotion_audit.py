#!/usr/bin/env python3
from second_batch_promotion_common import audit_method

raise SystemExit(audit_method(
    "plsc",
    "Reflective-only PLSc with path/factor weighting, corrected construct correlations, paths, loadings, R2, and rho_A attenuation correction.",
    [
        "plsc_reference_report.json",
        "plsc_unsupported_guard_report.json",
        "extended_pls_publication_audit.json",
        "rho_a_csem_comparison.json",
    ],
    ["PLSC_V1.md", "PLS_RHO_A_V1.md"],
))
