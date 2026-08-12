#!/usr/bin/env python3
from final_method_promotion_common import audit_method


raise SystemExit(audit_method(
    "v1_2_3_extended_pls_diagnostics_promotion",
    "nonlinear_effects",
    "Centered squared-term fixed-score nonlinear diagnostic for supported PLS structural paths.",
    [
        {
            "name": "nonlinear_effects_reference_report.json",
            "required_values": {"kind": "quadratic_nonlinear_effects_reference_v1"},
            "required_true": [
                "checks.has_experimental_warning",
                "checks.method_version",
                "checks.p_values_in_range",
                "checks.payload_version",
            ],
            "source_paths": ["validation/nonlinear_effects_reference.py"],
            "companions": [{
                "path": "validation/results/nonlinear_effects_reference_quickpls.json",
                "required_values": {
                    "status": "completed",
                    "provenance.method": "nonlinear_effects",
                    "payload.estimation.method_version": "pls_quadratic_nonlinear_effects_v1",
                    "payload.estimation.nonlinear_effects.method_version": "pls_quadratic_nonlinear_effects_v1",
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
                "where": {"file": "validation/results/nonlinear_effects_reference_report.json"},
                "required_values": {"kind": "quadratic_nonlinear_effects_reference_v1"},
                "required_true": ["present", "passed"],
            }],
        },
    ],
    [{
        "name": "PLS_NONLINEAR_EFFECTS_V1.md",
        "required_phrases": [
            "pls_quadratic_nonlinear_effects_v1",
            "fixed-score quadratic diagnostic scope",
            "broader nonlinear SEM estimation remains unsupported",
        ],
    }],
))
