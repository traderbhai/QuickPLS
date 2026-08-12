#!/usr/bin/env python3
from third_batch_promotion_common import audit_method


raise SystemExit(audit_method(
    "fimix_pls",
    "Bounded deterministic 2-3 class FIMIX-PLS score-space segmentation with probabilities, memberships, class paths/R2, log-likelihood, information criteria, entropy, and convergence diagnostics.",
    [
        {
            "name": "v06_group_methods_reference_report.json",
            "pass_paths": ["passed"],
            "required_values": {
                "kind": "v06_group_methods_reference",
                "sections.fimix.method_version": "fimix_pls_v1",
            },
            "source_paths": ["validation/v06_group_methods_reference.py"],
            "companions": [{
                "path": "validation/results/v06_fimix_quickpls.json",
                "required_values": {
                    "status": "completed",
                    "provenance.method": "predict",
                    "payload.estimation.fimix.method_version": "fimix_pls_v1",
                },
            }],
        },
        {
            "name": "prediction_heterogeneity_publication_audit.json",
            "required_true": ["coverage.fimix"],
            "source_paths": ["validation/prediction_heterogeneity_publication_audit.py"],
        },
    ],
    [{
        "name": "FIMIX_PLS_V1.md",
        "required_phrases": [
            "fimix_pls_v1",
            "random-start EM qualification",
            "not blanket full EM/FIMIX parity",
        ],
    }],
))
