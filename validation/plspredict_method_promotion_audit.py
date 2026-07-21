#!/usr/bin/env python3
from second_batch_promotion_common import audit_method

raise SystemExit(audit_method(
    "plspredict",
    "Deterministic PLSpredict holdout, repeated k-fold, construct-score LM benchmark, Q2 predict, RMSE/MAE, and bounded CVPAT diagnostics.",
    [
        "plspredict_holdout_reference_report.json",
        "prediction_heterogeneity_publication_audit.json",
    ],
    ["PLSPREDICT_HOLDOUT_V1.md"],
))
