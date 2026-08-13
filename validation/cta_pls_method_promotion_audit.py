#!/usr/bin/env python3
from final_method_promotion_common import audit_method


raise SystemExit(audit_method(
    "v1_2_3_extended_pls_diagnostics_promotion",
    "cta_pls",
    "Descriptive sample-covariance tetrad diagnostic for valid indicator blocks with four or more indicators.",
    [
        {
            "name": "cta_pls_reference_report.json",
            "required_values": {"kind": "cta_pls_reference_v1"},
            "required_true": [
                "checks.estimate_count",
                "checks.guard",
                "checks.max_absolute_summary",
                "checks.max_delta_within_tolerance",
                "checks.method_version",
                "checks.payload_version",
            ],
            "source_paths": ["validation/cta_pls_reference.py"],
            "companions": [{
                "path": "validation/results/cta_pls_reference_quickpls.json",
                "required_values": {
                    "status": "completed",
                    "provenance.method": "cta_pls",
                    "payload.estimation.method_version": "cta_pls_tetrad_v1",
                    "payload.estimation.cta_pls.method_version": "cta_pls_tetrad_v1",
                },
            }],
        },
        {
            "name": "v05_extended_pls_evidence.json",
            "required_true": [
                "promotion_ready",
                "all_listed_artifacts_present",
                "all_listed_artifacts_passed",
            ],
            "required_list_items": [{
                "path": "artifacts",
                "where": {"file": "validation/results/cta_pls_reference_report.json"},
                "required_values": {"kind": "cta_pls_reference_v1"},
                "required_true": ["present", "passed"],
            }],
        },
    ],
    [{
        "name": "PLS_CTA_PLS_V1.md",
        "required_phrases": [
            "cta_pls_tetrad_v1",
            "bootstrap/permutation tetrad decision rules",
            "remain unsupported",
        ],
    }],
))
