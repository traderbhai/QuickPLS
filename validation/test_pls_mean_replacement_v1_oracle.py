from __future__ import annotations

import ast
import json
import math
from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

import pls_mean_replacement_v1_oracle as oracle


WORK_REPORT = (
    Path(__file__).resolve().parent
    / "results"
    / "method_factory"
    / "pls_algorithm_v1"
    / "work"
    / "pls_mean_replacement_v1_independent_oracle_work.json"
)


def _failure_code(callback: Callable[[], Any]) -> str:
    with pytest.raises(oracle.MeanReplacementOracleFailure) as raised:
        callback()
    return raised.value.code


def _single_indicator_rows(values: list[object]) -> dict[str, object]:
    return {
        "rows": [[value] for value in values],
        "row_ids": [f"r{index}" for index in range(len(values))],
        "variables": ["x"],
        "bindings": [oracle.OracleIndicatorBinding("indicator:x", "x")],
    }


def test_work_checks_are_deterministic_and_explicitly_non_promotional() -> None:
    first = oracle.run_work_checks()
    second = oracle.run_work_checks()

    assert first == second
    assert first["passed"]
    assert first["work_evidence_only"]
    assert not first["qualification_ready"]
    assert not first["promotion_requested"]
    assert not first["promotion_allowed"]
    assert not first["product_comparison_performed"]
    assert not first["receipt_attached"]
    assert not first["registry_or_manifest_mutation_requested"]
    assert all(first["checks"].values())
    assert all(first["typed_boundaries"].values())


def test_replacement_is_per_indicator_not_row_mean_or_listwise() -> None:
    result = oracle.mean_replace_continuous_raw_v1(
        rows=[
            [1.0, 10.0],
            [None, 20.0],
            [5.0, None],
            [9.0, 40.0],
        ],
        row_ids=["r1", "r2", "r3", "r4"],
        variables=["x", "y"],
        bindings=[
            oracle.OracleIndicatorBinding("indicator:x", "x"),
            oracle.OracleIndicatorBinding("indicator:y", "y"),
        ],
    )

    assert result["replacement_means"] == {
        "indicator:x": 5.0,
        "indicator:y": 70.0 / 3.0,
    }
    assert result["completed_rows"] == [
        [1.0, 10.0],
        [5.0, 20.0],
        [5.0, 70.0 / 3.0],
        [9.0, 40.0],
    ]
    assert result["used_observations"] == 4
    assert result["omitted_observations"] == 0
    assert result["total_missing_before"] == 2
    assert result["total_missing_after"] == 0
    assert result["missing_representation"] == "arrow_null_only"
    assert result["source_dataset_mutated"] is False


def test_no_complete_rows_and_a_fully_missing_modeled_row_are_retained() -> None:
    fixture = oracle.no_complete_row_fixture()
    source_rows = [list(row) for row in fixture["rows"]]
    result = oracle.mean_replace_continuous_raw_v1(**fixture)

    assert fixture["rows"] == source_rows
    assert result["completed_rows"] == [
        [1.0, 30.0],
        [3.0, 20.0],
        [5.0, 30.0],
        [3.0, 40.0],
        [3.0, 30.0],
    ]
    assert result["used_observations"] == 5
    assert result["omitted_observations"] == 0
    fully_missing = next(
        row for row in result["case_diagnostics"] if row["row_id"] == "fully-missing"
    )
    assert fully_missing["missing_before"] == 2
    assert fully_missing["replaced_count"] == 2
    assert fully_missing["missing_after"] == 0


def test_fractional_integer_mean_is_not_truncated_and_non_null_sentinel_is_observed() -> None:
    fractional = oracle.mean_replace_continuous_raw_v1(
        **_single_indicator_rows([1, None, 2])
    )
    assert fractional["completed_rows"] == [[1.0], [1.5], [2.0]]

    sentinel = oracle.mean_replace_continuous_raw_v1(
        **_single_indicator_rows([-99, None, 3, 9])
    )
    assert sentinel["non_null_sentinels_are_observed"]
    assert sentinel["replacement_means"]["indicator:x"] == -29.0
    assert sentinel["completed_rows"][0][0] == -99.0
    assert sentinel["completed_rows"][1][0] == -29.0


def test_no_missing_is_identity_and_second_pass_is_idempotent() -> None:
    fixture = _single_indicator_rows([1.0, 3.0, 8.0, 12.0])
    first = oracle.mean_replace_continuous_raw_v1(**fixture)
    second = oracle.mean_replace_continuous_raw_v1(
        first["completed_rows"],
        fixture["row_ids"],
        fixture["variables"],
        fixture["bindings"],
    )

    assert first["completed_rows"] == fixture["rows"]
    assert first["total_missing_before"] == 0
    assert first["warnings"] == []
    assert second["completed_rows"] == first["completed_rows"]
    assert second["total_missing_before"] == 0
    assert second["prepared_matrix_sha256"] == first["prepared_matrix_sha256"]


def test_variance_attenuation_identity_is_explicit() -> None:
    result = oracle.mean_replace_continuous_raw_v1(
        **_single_indicator_rows([1.0, None, 5.0, 9.0, None])
    )
    diagnostic = result["indicator_diagnostics"][0]

    assert diagnostic["observed_sample_variance"] == 16.0
    assert diagnostic["completed_sample_variance"] == 8.0
    assert diagnostic["expected_completed_sample_variance"] == 8.0
    assert diagnostic["variance_identity_abs_difference"] == 0.0
    assert "pls_mean_replacement_v1_variance_attenuation" in result["warnings"]


def test_warning_thresholds_are_diagnostic_not_execution_gates() -> None:
    matrix = [[float(row + column * 100) for column in range(4)] for row in range(20)]
    matrix[0][1] = None
    for row in range(3):
        matrix[row][2] = None
    for row in range(4):
        matrix[row][3] = None
    result = oracle.mean_replace_continuous_raw_v1(
        matrix,
        [f"r{row}" for row in range(20)],
        ["none", "five", "fifteen", "twenty"],
        [
            oracle.OracleIndicatorBinding("indicator:none", "none"),
            oracle.OracleIndicatorBinding("indicator:five", "five"),
            oracle.OracleIndicatorBinding("indicator:fifteen", "fifteen"),
            oracle.OracleIndicatorBinding("indicator:twenty", "twenty"),
        ],
    )
    by_id = {row["indicator_id"]: row for row in result["indicator_diagnostics"]}

    assert by_id["indicator:none"]["warning_codes"] == []
    assert by_id["indicator:five"]["warning_codes"] == [
        "pls_mean_replacement_v1_indicator_missing_at_least_5_percent"
    ]
    assert by_id["indicator:fifteen"]["warning_codes"] == [
        "pls_mean_replacement_v1_indicator_missing_at_least_5_percent"
    ]
    assert by_id["indicator:twenty"]["warning_codes"] == [
        "pls_mean_replacement_v1_indicator_missing_at_least_5_percent",
        "pls_mean_replacement_v1_indicator_missing_above_15_percent",
    ]
    assert result["used_observations"] == 20
    assert result["total_missing_after"] == 0


def test_case_warning_is_strictly_above_fifteen_percent() -> None:
    variables = [f"v{index:02d}" for index in range(20)]
    bindings = [
        oracle.OracleIndicatorBinding(f"indicator:{name}", name) for name in variables
    ]
    rows = [[float(row + column) for column in range(20)] for row in range(5)]
    for column in range(3):
        rows[0][column] = None
    for column in range(4):
        rows[1][column] = None
    result = oracle.mean_replace_continuous_raw_v1(
        rows, [f"r{row}" for row in range(5)], variables, bindings
    )

    assert result["case_diagnostics"][0]["missing_fraction_before"] == 0.15
    assert result["case_diagnostics"][0]["warning_codes"] == []
    assert result["case_diagnostics"][1]["missing_fraction_before"] == 0.20
    assert result["case_diagnostics"][1]["warning_codes"] == [
        "pls_mean_replacement_v1_case_missing_above_15_percent"
    ]


def test_exact_binary_reference_resists_cancellation_and_row_order() -> None:
    forward = oracle.mean_replace_continuous_raw_v1(
        **_single_indicator_rows([1.0e16, 1.0, -1.0e16, None])
    )
    reversed_result = oracle.mean_replace_continuous_raw_v1(
        **_single_indicator_rows([-1.0e16, 1.0, 1.0e16, None])
    )

    expected = 1.0 / 3.0
    assert forward["replacement_means"]["indicator:x"] == expected
    assert reversed_result["replacement_means"]["indicator:x"] == expected
    assert forward["completed_rows"][-1][0] == expected
    assert reversed_result["completed_rows"][-1][0] == expected


def test_unused_columns_do_not_participate_and_are_left_unchanged() -> None:
    rows = [
        [1.0, None, "alpha"],
        [None, 20.0, None],
        [5.0, 40.0, "-99"],
    ]
    result = oracle.mean_replace_continuous_raw_v1(
        rows,
        ["r1", "r2", "r3"],
        ["x", "y", "unused"],
        [
            oracle.OracleIndicatorBinding("indicator:x", "x"),
            oracle.OracleIndicatorBinding("indicator:y", "y"),
        ],
    )

    assert result["completed_rows"] == [
        [1.0, 30.0, "alpha"],
        [3.0, 20.0, None],
        [5.0, 40.0, "-99"],
    ]
    assert result["total_missing_before"] == 2


@pytest.mark.parametrize("input_kind", ["covariance", "correlation"])
def test_matrix_inputs_fail_as_raw_data_required(input_kind: oracle.InputKind) -> None:
    fixture = _single_indicator_rows([1.0, None, 3.0])
    code = _failure_code(
        lambda: oracle.mean_replace_continuous_raw_v1(
            **fixture,
            settings=oracle.OracleMeanReplacementSettings(input_kind=input_kind),
        )
    )
    assert code == "pls_mean_replacement_v1_raw_data_required"


@pytest.mark.parametrize("scale", ["binary", "ordinal", "nominal", "identifier"])
def test_noncontinuous_scales_fail_closed(scale: oracle.Scale) -> None:
    fixture = _single_indicator_rows([1.0, None, 3.0])
    fixture["bindings"] = [
        oracle.OracleIndicatorBinding(
            "indicator:x", "x", sem_scale=scale, dataset_scale=scale
        )
    ]
    assert _failure_code(
        lambda: oracle.mean_replace_continuous_raw_v1(**fixture)
    ) == "pls_mean_replacement_v1_continuous_indicator_required"


@pytest.mark.parametrize(
    ("settings", "code"),
    [
        (
            oracle.OracleMeanReplacementSettings(weight_requested=True),
            "pls_mean_replacement_v1_weights_unsupported",
        ),
        (
            oracle.OracleMeanReplacementSettings(group_count=2),
            "pls_mean_replacement_v1_groups_unsupported",
        ),
        (
            oracle.OracleMeanReplacementSettings(bootstrap_samples=1),
            "pls_mean_replacement_v1_resampling_unsupported",
        ),
        (
            oracle.OracleMeanReplacementSettings(studentized_inner_samples=1),
            "pls_mean_replacement_v1_resampling_unsupported",
        ),
        (
            oracle.OracleMeanReplacementSettings(permutation_samples=1),
            "pls_mean_replacement_v1_resampling_unsupported",
        ),
        (
            oracle.OracleMeanReplacementSettings(prediction_requested=True),
            "pls_mean_replacement_v1_prediction_unsupported",
        ),
        (
            oracle.OracleMeanReplacementSettings(missing_policy="full_information_maximum_likelihood"),
            "pls_mean_replacement_v1_policy_mismatch",
        ),
    ],
)
def test_scope_drift_has_typed_failures(
    settings: oracle.OracleMeanReplacementSettings, code: str
) -> None:
    fixture = _single_indicator_rows([1.0, None, 3.0])
    assert _failure_code(
        lambda: oracle.mean_replace_continuous_raw_v1(
            **fixture, settings=settings
        )
    ) == code


@pytest.mark.parametrize(
    ("values", "code"),
    [
        ([1.0, math.nan, 3.0], "pls_mean_replacement_v1_non_finite_observed_value"),
        ([1.0, math.inf, 3.0], "pls_mean_replacement_v1_non_finite_observed_value"),
        ([1.0, "-99", 3.0], "pls_mean_replacement_v1_nonnumeric_observed_value"),
        ([None, None, None], "pls_mean_replacement_v1_all_missing_indicator"),
        ([2.0, None, 2.0], "pls_mean_replacement_v1_constant_indicator_after_replacement"),
    ],
)
def test_invalid_cells_and_degenerate_columns_fail_closed(
    values: list[object], code: str
) -> None:
    fixture = _single_indicator_rows(values)
    assert _failure_code(
        lambda: oracle.mean_replace_continuous_raw_v1(**fixture)
    ) == code


def test_identity_shape_and_minimum_n_boundaries_fail_closed() -> None:
    fixture = _single_indicator_rows([1.0, None, 3.0])
    assert _failure_code(
        lambda: oracle.mean_replace_continuous_raw_v1(
            fixture["rows"], ["same", "same", "third"], ["x"], fixture["bindings"]
        )
    ) == "pls_mean_replacement_v1_row_id_invalid"
    assert _failure_code(
        lambda: oracle.mean_replace_continuous_raw_v1(
            [[1.0], [2.0, 3.0], [4.0]],
            fixture["row_ids"],
            ["x"],
            fixture["bindings"],
        )
    ) == "pls_mean_replacement_v1_row_width_mismatch"
    assert _failure_code(
        lambda: oracle.mean_replace_continuous_raw_v1(
            [[1.0], [2.0]], ["r1", "r2"], ["x"], fixture["bindings"]
        )
    ) == "pls_mean_replacement_v1_insufficient_observations"


def test_oracle_import_graph_is_independent_of_quickpls_product_code() -> None:
    source_path = Path(oracle.__file__).resolve()
    syntax = ast.parse(source_path.read_text(encoding="utf-8"))
    imported_roots: set[str] = set()
    for node in ast.walk(syntax):
        if isinstance(node, ast.Import):
            imported_roots.update(alias.name.split(".")[0] for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module is not None:
            imported_roots.add(node.module.split(".")[0])

    assert imported_roots <= {
        "__future__",
        "argparse",
        "hashlib",
        "json",
        "math",
        "dataclasses",
        "fractions",
        "pathlib",
        "typing",
    }
    assert not any(root.startswith("qpls") for root in imported_roots)


def test_work_report_is_strict_json_and_remains_unqualified(tmp_path: Path) -> None:
    path = tmp_path / "pls_mean_replacement_v1_work.json"
    expected = oracle.write_work_report(path)
    observed = json.loads(
        path.read_text(encoding="utf-8"),
        parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)),
    )

    assert observed == expected
    assert not observed["qualification_spec_v2_alignment"]["admissible_receipt"]
    assert observed["qualification_spec_v2_alignment"]["candidate_oracle_kind"] == (
        "independent_implementation"
    )
    assert observed["qualification_spec_v2_alignment"]["runtime_policy"] == (
        "development_validation_only"
    )


def test_repository_work_report_is_fresh_and_cannot_claim_qualification() -> None:
    observed = json.loads(
        WORK_REPORT.read_text(encoding="utf-8"),
        parse_constant=lambda value: (_ for _ in ()).throw(ValueError(value)),
    )

    assert observed == oracle.run_work_checks()
    assert observed["work_evidence_only"]
    assert not observed["qualification_ready"]
    assert not observed["product_comparison_performed"]
    assert not observed["receipt_attached"]
