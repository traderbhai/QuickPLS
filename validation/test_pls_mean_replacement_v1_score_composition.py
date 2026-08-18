from __future__ import annotations

import copy

import pls_mean_replacement_v1_oracle as mean_oracle
import pls_score_execution_v2_oracle as score_oracle


def test_completed_matrix_composes_with_independent_pls_score_oracle() -> None:
    score_fixture = score_oracle.deterministic_fixture()
    rows = copy.deepcopy(score_fixture["rows"][:60])
    row_ids = list(score_fixture["row_ids"][:60])
    variables = list(score_fixture["variables"])
    missing_cells = ((1, 0), (7, 4), (13, 8), (23, 2))
    for row_index, column_index in missing_cells:
        rows[row_index][column_index] = None

    completed = mean_oracle.mean_replace_continuous_raw_v1(
        rows,
        row_ids,
        variables,
        [
            mean_oracle.OracleIndicatorBinding(f"indicator:{variable}", variable)
            for variable in variables
        ],
    )
    composed = score_oracle.estimate_score_execution_v2(
        completed["completed_rows"],
        row_ids,
        variables,
        score_fixture["blocks"],
        score_fixture["paths"],
    )

    manually_completed = copy.deepcopy(rows)
    for row_index, column_index in missing_cells:
        variable = variables[column_index]
        manually_completed[row_index][column_index] = completed["replacement_means"][
            f"indicator:{variable}"
        ]
    manual = score_oracle.estimate_score_execution_v2(
        manually_completed,
        row_ids,
        variables,
        score_fixture["blocks"],
        score_fixture["paths"],
    )
    listwise = score_oracle.estimate_score_execution_v2(
        rows,
        row_ids,
        variables,
        score_fixture["blocks"],
        score_fixture["paths"],
    )

    assert composed == manual
    assert completed["used_observations"] == 60
    assert completed["omitted_observations"] == 0
    assert completed["total_missing_before"] == len(missing_cells)
    assert composed["used_observations"] == 60
    assert composed["row_ids"] == row_ids
    assert listwise["used_observations"] == 60 - len(missing_cells)
    assert set(listwise["row_ids"]) == set(row_ids) - {
        row_ids[row_index] for row_index, _ in missing_cells
    }
    assert composed["paths"] != listwise["paths"]


def test_mean_replacement_and_score_composition_remains_point_only() -> None:
    fixture = mean_oracle.deterministic_fixture()
    try:
        mean_oracle.mean_replace_continuous_raw_v1(
            **fixture,
            settings=mean_oracle.OracleMeanReplacementSettings(bootstrap_samples=1),
        )
    except mean_oracle.MeanReplacementOracleFailure as error:
        assert error.code == "pls_mean_replacement_v1_resampling_unsupported"
    else:  # pragma: no cover - makes silent scope widening an explicit failure
        raise AssertionError("resampling scope was silently accepted")
