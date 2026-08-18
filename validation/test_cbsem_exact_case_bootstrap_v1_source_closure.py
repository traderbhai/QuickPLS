from __future__ import annotations

from validation.cbsem_exact_case_bootstrap_v1_contract_matrix import (
    EXPECTED,
    bca_interval,
    report,
    studentized_interval,
)
from validation.cbsem_exact_case_bootstrap_v1_source_gate import run_gate


def test_frozen_compact_independent_matrix_passes_exactly() -> None:
    observed = report()
    assert observed["passed"] is True
    assert observed["observed"] == EXPECTED
    assert observed["observed"]["metamorphic"] == {
        "estimate_reorder_invariant": True,
        "worker_index_projection_invariant": True,
    }


def test_studentized_and_bca_fail_closed_on_invalid_boundary_inputs() -> None:
    try:
        studentized_interval(1.0, 0.0, [0.9, 1.1], [0.1, 0.1])
    except ValueError as error:
        assert "positive standard errors" in str(error)
    else:
        raise AssertionError("zero point SE must fail closed")

    try:
        bca_interval(1.0, [1.1, 1.2], [0.9, 1.1])
    except ValueError as error:
        assert "probability boundary" in str(error)
    else:
        raise AssertionError("boundary BCa bias correction must fail closed")


def test_current_source_bridge_and_integrity_scope_are_bound() -> None:
    gate = run_gate()
    assert gate["passed"] is True, gate["checks"]
    assert gate["authentication_exclusion"] == (
        "malicious coordinated semantic rewrite and trusted-origin authentication are not claimed"
    )
