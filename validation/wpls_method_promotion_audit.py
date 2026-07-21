#!/usr/bin/env python3
from second_batch_promotion_common import audit_method

raise SystemExit(audit_method(
    "wpls",
    "Positive case-weighted reflective WPLS with standardized preprocessing and path/factor weighting.",
    [
        "wpls_reference_report.json",
        "extended_pls_publication_audit.json",
    ],
    ["PLS_WPLS_V1.md"],
))
