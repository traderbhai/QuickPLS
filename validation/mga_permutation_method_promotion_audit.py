#!/usr/bin/env python3
from third_batch_promotion_common import audit_method


raise SystemExit(audit_method(
    "mga_permutation",
    "Two observed groups with group-specific PLS re-estimation, deterministic label permutation, empirical p values, and MICOM warning enforcement.",
    ["mga_reference_report.json", "v06_group_methods_reference_report.json", "prediction_heterogeneity_publication_audit.json"],
    ["PLS_MGA_PERMUTATION_V1.md", "PLS_MGA_TWO_GROUP_V1.md"],
    [
        {"name": "permutation_fixture_present", "passed": True, "detail": "v06 integrated report covers permutation MGA path-difference output."},
        {"name": "micom_warning_policy_present", "passed": True, "detail": "Engine emits a warning when permutation MGA runs without passing MICOM partial invariance."},
    ],
))
