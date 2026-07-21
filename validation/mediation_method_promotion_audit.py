#!/usr/bin/env python3
from second_batch_promotion_common import audit_method

raise SystemExit(audit_method(
    "mediation",
    "PLS mediation direct, indirect, total, VAF, classification, and validated bootstrap/permutation indirect-effect interpretation.",
    [
        "mediation_reference_report.json",
        "mediation_r_reference_report.json",
        "mediation_published_example_report.json",
        "mediation_metamorphic_report.json",
        "mediation_randomization_report.json",
        "extended_pls_publication_audit.json",
    ],
    ["PLS_MEDIATION_V1.md", "RESAMPLING_ENGINE_V4.md"],
))
