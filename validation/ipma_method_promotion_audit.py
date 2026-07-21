#!/usr/bin/env python3
from second_batch_promotion_common import audit_method

raise SystemExit(audit_method(
    "ipma",
    "Bounded IPMA/cIPMA using PLS total effects as importance and 0-100 standardized score performance.",
    [
        "ipma_reference_report.json",
        "prediction_heterogeneity_publication_audit.json",
    ],
    ["IPMA_V1.md"],
))
