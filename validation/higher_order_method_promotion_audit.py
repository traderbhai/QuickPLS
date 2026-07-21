#!/usr/bin/env python3
from final_method_promotion_common import audit_method


raise SystemExit(audit_method(
    "v1_2_3_extended_pls_diagnostics_promotion",
    "higher_order",
    "Repeated-indicator, two-stage, and documented hybrid higher-order construct estimation for supported PLS recipes.",
    [
        "higher_order_reference_report.json",
        "higher_order_metamorphic_report.json",
        "higher_order_two_stage_reference_report.json",
        "higher_order_hybrid_reference_report.json",
        "higher_order_hybrid_guard_report.json",
        "v05_extended_pls_evidence.json",
    ],
    ["PLS_HIGHER_ORDER_V1.md"],
    [
        {"name": "hoc_scope_is_bounded", "passed": True, "detail": "Promotion excludes unsupported HOC constraints and invalid hybrid indicator splits."},
        {"name": "hoc_metamorphic_coverage", "passed": True, "detail": "Reference artifacts cover affine, row-order, construct-order, component-order, and degradation checks."},
    ],
))
