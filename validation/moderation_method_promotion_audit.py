#!/usr/bin/env python3
from second_batch_promotion_common import audit_method

raise SystemExit(audit_method(
    "moderation",
    "Two-stage PLS moderation with one generated product-score interaction, simple slopes at -1/0/+1, and validated bootstrap/permutation interpretation.",
    [
        "moderation_reference_report.json",
        "moderation_r_reference_report.json",
        "moderation_published_formula_report.json",
        "moderation_published_empirical_report.json",
        "moderation_simulation_report.json",
        "moderation_inference_report.json",
        "moderation_inference_qualification_report.json",
        "moderation_coverage_qualification_report.json",
        "extended_pls_publication_audit.json",
    ],
    ["PLS_TWO_STAGE_MODERATION_V1.md", "RESAMPLING_ENGINE_V4.md"],
))
