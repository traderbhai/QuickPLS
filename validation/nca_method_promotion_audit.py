#!/usr/bin/env python3
from second_batch_promotion_common import audit_method

raise SystemExit(audit_method(
    "nca",
    "Numeric X/Y NCA with CE-FDH and CR-FDH ceilings, deterministic permutation p values, and bottleneck tables.",
    [
        "v08_extended_methods_reference_report.json",
        "extended_methods_publication_audit.json",
    ],
    ["NCA_V1.md"],
))
