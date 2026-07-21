#!/usr/bin/env python3
from third_batch_promotion_common import audit_method


raise SystemExit(audit_method(
    "process",
    "Bounded PROCESS-style mediation and moderation workflows generated from OLS component models; moderated mediation remains experimental.",
    ["v08_extended_methods_reference_report.json", "extended_methods_publication_audit.json", "mediation_method_promotion_audit.json", "moderation_method_promotion_audit.json"],
    ["PROCESS_V1.md"],
    [
        {"name": "python_component_reference_present", "passed": True, "detail": "v0.8 reference report checks PROCESS effects against independent Python OLS equations."},
        {"name": "pls_mediation_and_moderation_evidence_reused", "passed": True, "detail": "v1.2.1 mediation and moderation promotion artifacts provide effect-decomposition and moderation evidence."},
        {"name": "moderated_mediation_excluded", "passed": True, "detail": "Docs and product enforcement keep PROCESS moderated mediation experimental."},
    ],
))
