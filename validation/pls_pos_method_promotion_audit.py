#!/usr/bin/env python3
from third_batch_promotion_common import audit_method


raise SystemExit(audit_method(
    "pls_pos",
    "Deterministic PLS-POS with 2-5 segments, deterministic starts, minimum segment-share guard, objective history, memberships, segment paths, and segment R2.",
    [
        {
            "name": "segmentation_recovery_simulation_report.json",
            "required_values": {"kind": "segmentation_recovery_simulation_v1"},
            "required_true": [
                "checks.discovery_payload_version",
                "checks.discovery_memberships_complete",
                "checks.multi_discovery_memberships_complete",
                "checks.bounded_inferential_screen_null_below_signal",
            ],
            "source_paths": ["validation/segmentation_recovery_simulation.py"],
        },
        {
            "name": "v06_group_methods_reference_report.json",
            "pass_paths": ["passed"],
            "required_values": {
                "kind": "v06_group_methods_reference",
                "sections.pos.method_version": "pls_pos_v1",
            },
            "source_paths": ["validation/v06_group_methods_reference.py"],
            "companions": [{
                "path": "validation/results/v06_pos_quickpls.json",
                "required_values": {
                    "status": "completed",
                    "provenance.method": "predict",
                    "payload.estimation.segmentation.method_version": "pls_pos_v1",
                },
            }],
        },
        {
            "name": "prediction_heterogeneity_publication_audit.json",
            "required_true": ["coverage.pls_pos"],
            "source_paths": ["validation/prediction_heterogeneity_publication_audit.py"],
        },
    ],
    [{
        "name": "PLS_POS_V1.md",
        "required_phrases": [
            "pls_pos_v1",
            "stable memberships",
            "backward-compatible for older metadata",
        ],
    }],
))
