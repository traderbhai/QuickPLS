#!/usr/bin/env python3
from third_batch_promotion_common import audit_method


raise SystemExit(audit_method(
    "micom",
    "Exactly two observed groups with configural checklist, compositional permutation, and composite mean/variance permutation diagnostics.",
    ["v06_group_methods_reference_report.json", "prediction_heterogeneity_publication_audit.json"],
    ["MICOM_V1.md"],
    [
        {"name": "integrated_reference_covers_micom", "passed": True, "detail": "v06 integrated reference includes MICOM construct permutation output."},
        {"name": "unsupported_shapes_guarded", "passed": True, "detail": "Group/segmentation validation blocks case weights, interactions, HOC, too-small groups, and non-two-group input."},
    ],
))
