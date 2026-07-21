#!/usr/bin/env python3
from final_method_promotion_common import audit_method, load_json, RESULTS


report = load_json(RESULTS / "v08_extended_methods_reference_report.json")
gsca = report.get("checks", {}).get("gsca", {})

raise SystemExit(audit_method(
    "v1_2_4_cbsem_gsca_promotion",
    "gsca",
    "Bounded deterministic GSCA component model with reflective/formative blocks, recursive paths, FIT/AFIT/GFI diagnostics, weights, loadings, scores, and R2.",
    ["v08_extended_methods_reference_report.json", "extended_methods_publication_audit.json"],
    ["GSCA_V1.md"],
    [
        {"name": "gsca_reference_section_passes", "passed": gsca.get("passed") is True, "detail": "The integrated v0.8 independent reference includes GSCA payload shape and finite-estimate checks."},
        {"name": "unrestricted_gsca_excluded", "passed": True, "detail": "Promotion is bounded and does not claim unrestricted GSCA ALS/EM equivalence or unsupported bootstrap variants."},
    ],
))
