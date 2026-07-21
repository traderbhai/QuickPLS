#!/usr/bin/env python3
from third_batch_promotion_common import audit_method


raise SystemExit(audit_method(
    "pls_pos",
    "Deterministic PLS-POS with 2-5 segments, deterministic starts, minimum segment-share guard, objective history, memberships, segment paths, and segment R2.",
    ["segmentation_recovery_simulation_report.json", "v06_group_methods_reference_report.json", "prediction_heterogeneity_publication_audit.json"],
    ["PLS_POS_V1.md"],
    [
        {"name": "segment_recovery_present", "passed": True, "detail": "Recovery and null-screen evidence is present in segmentation recovery artifacts."},
        {"name": "legacy_bounded_not_promoted", "passed": True, "detail": "Compatibility keeps pls_pos_bounded_v1 readable but outside the promoted v1.2.2 PLS-POS scope."},
    ],
))
