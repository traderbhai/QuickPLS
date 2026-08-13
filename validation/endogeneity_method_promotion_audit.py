#!/usr/bin/env python3
from final_method_promotion_common import audit_method


raise SystemExit(audit_method(
    "v1_2_3_extended_pls_diagnostics_promotion",
    "endogeneity",
    "Gaussian-copula diagnostic with rankit inverse-normal copula terms for screenable nonnormal predictor scores.",
    [
        {
            "name": "endogeneity_reference_report.json",
            "required_values": {"kind": "gaussian_copula_endogeneity_reference_v1"},
            "required_true": [
                "checks.has_experimental_warning",
                "checks.method_version",
                "checks.p_values_in_range",
                "checks.payload_version",
            ],
            "source_paths": ["validation/endogeneity_reference.py"],
            "companions": [{
                "path": "validation/results/endogeneity_reference_quickpls.json",
                "required_values": {
                    "status": "completed",
                    "provenance.method": "endogeneity",
                    "payload.estimation.method_version": "gaussian_copula_endogeneity_v1",
                    "payload.estimation.endogeneity.method_version": "gaussian_copula_endogeneity_v1",
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
                "where": {"file": "validation/results/endogeneity_reference_report.json"},
                "required_values": {"kind": "gaussian_copula_endogeneity_reference_v1"},
                "required_true": ["present", "passed"],
            }],
        },
    ],
    [{
        "name": "PLS_GAUSSIAN_COPULA_ENDOGENEITY_V1.md",
        "required_phrases": [
            "gaussian_copula_endogeneity_v1",
            "diagnostic screen for possible endogeneity risk",
            "not proof of causality",
        ],
    }],
))
