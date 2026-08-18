from __future__ import annotations

import math

import pytest

import pls_model_comparison_v1_oracle as oracle


def test_frozen_independent_micro_report() -> None:
    report = oracle.build_report()
    oracle.check_report(report)


def test_oracle_rejects_invalid_formula_inputs() -> None:
    with pytest.raises(ValueError):
        oracle.prediction_oriented_bic(2, 1.0, 1)
    with pytest.raises(ValueError):
        oracle.prediction_oriented_bic(20, 0.0, 2)
    with pytest.raises(ValueError):
        oracle.two_candidate_weights(math.nan, 2.0)
    with pytest.raises(ValueError):
        oracle.paired_cvpat([1.0, 2.0], [1.0, 2.0])


def test_shared_fold_plan_is_seeded_balanced_and_complete() -> None:
    rows = list(range(23))
    first = oracle.shared_fold_plan(rows, folds=5, repeats=3, seed=47)
    repeated = oracle.shared_fold_plan(rows, folds=5, repeats=3, seed=47)
    changed = oracle.shared_fold_plan(rows, folds=5, repeats=3, seed=48)
    assert first == repeated
    assert first["assignment_digest"] != changed["assignment_digest"]
    ledger = first["ledger"]
    assert isinstance(ledger, list)
    for repeat in range(3):
        counts = [sum(row["repeat"] == repeat and row["fold"] == fold for row in ledger) for fold in range(5)]
        assert max(counts) - min(counts) <= 1
