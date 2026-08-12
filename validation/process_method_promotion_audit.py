#!/usr/bin/env python3
from third_batch_promotion_common import audit_method


raise SystemExit(audit_method(
    "process",
    "Bounded PROCESS-style mediation and moderation workflows generated from OLS component models; moderated mediation remains experimental.",
    [
        {
            "name": "v08_process_reference_report.json",
            "required_true": ["checks.process.passed"],
            "required_values": {
                "schema_version": 2,
                "report_scope": "method_specific",
                "selected_section": "process",
                "checks.process.method_version": "regression_process_v1",
            },
            "source_paths": ["validation/v08_extended_methods_reference.py"],
            "companions": [{
                "path": "validation/results/v08_process_quickpls.json",
                "required_values": {
                    "status": "completed",
                    "provenance.method": "regression",
                    "provenance.method_version": "regression_process_v1",
                    "payload.estimation.method_version": "regression_process_v1",
                    "payload.estimation.regression.method_version": "regression_process_v1",
                },
            }],
        },
        {
            "name": "extended_methods_publication_audit.json",
            "required_true": ["method_coverage.process"],
            "source_paths": ["validation/extended_methods_publication_audit.py"],
        },
        {"name": "mediation_method_promotion_audit.json", "required_values": {"method_id": "mediation"}},
        {"name": "moderation_method_promotion_audit.json", "required_values": {"method_id": "moderation"}},
    ],
    [{
        "name": "PROCESS_V1.md",
        "required_phrases": [
            "regression_process_v1",
            "independent Python OLS equations",
            "Johnson-Neyman publication claims",
            "process_model = moderated_mediation",
        ],
    }],
))
