#!/usr/bin/env python3
from final_method_promotion_common import audit_method


raise SystemExit(audit_method(
    "v1_2_3_extended_pls_diagnostics_promotion",
    "moderated_mediation",
    "Two-stage conditional indirect-effect diagnostic with standardized moderator levels -1, 0, and +1 plus index of moderated mediation.",
    [
        {
            "name": "moderated_mediation_reference_report.json",
            "required_values": {"kind": "moderated_mediation_reference_v1"},
            "required_true": [
                "checks.has_experimental_warning",
                "checks.invalid_guard",
                "checks.method_version",
                "checks.payload_version",
                "checks.stage",
            ],
            "source_paths": ["validation/moderated_mediation_reference.py"],
            "companions": [{
                "path": "validation/results/moderated_mediation_reference_quickpls.json",
                "required_values": {
                    "status": "completed",
                    "provenance.method": "moderated_mediation",
                    "payload.estimation.method_version": "pls_moderated_mediation_v1",
                    "payload.estimation.moderated_mediation.method_version": "pls_moderated_mediation_v1",
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
                "where": {"file": "validation/results/moderated_mediation_reference_report.json"},
                "required_values": {"kind": "moderated_mediation_reference_v1"},
                "required_true": ["present", "passed"],
            }],
        },
    ],
    [{
        "name": "PLS_MODERATED_MEDIATION_V1.md",
        "required_phrases": [
            "pls_moderated_mediation_v1",
            "conditional indirect effects",
            "full Hayes PROCESS catalogue",
            "remain experimental or unsupported",
        ],
    }],
))
