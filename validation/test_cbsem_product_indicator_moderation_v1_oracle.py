from __future__ import annotations

import numpy as np
import pytest

import cbsem_product_indicator_moderation_v1_oracle as oracle


@pytest.mark.parametrize(
    ("centering", "standardization", "expected"),
    [
        ("none", "none", [4.0, 2.0, 9.0, 8.0]),
        ("mean_center", "none", [-2.25, 0.75, 0.25, -0.75]),
        ("double_mean_center", "none", [-1.75, 1.25, 0.75, -0.25]),
        (
            "double_mean_center",
            "sample_standard_deviation",
            [-1.05, 0.75, 0.45, -0.15],
        ),
    ],
)
def test_product_construction_modes_match_hand_calculations(
    centering: oracle.Centering,
    standardization: oracle.Standardization,
    expected: list[float],
) -> None:
    values = np.array(
        [
            [1.0, 4.0],
            [2.0, 1.0],
            [3.0, 3.0],
            [4.0, 2.0],
        ]
    )
    actual = oracle.materialize_product_indicators(
        values,
        ("x1", "m1"),
        ("x1",),
        ("m1",),
        centering=centering,
        standardization=standardization,
        validate_estimator_scope=False,
    )
    np.testing.assert_allclose(
        actual.products[:, 0],
        expected,
        atol=1e-15,
        rtol=0.0,
    )
    if centering == "double_mean_center":
        assert abs(actual.product_final_means[0]) <= 1e-15


def test_row_and_column_reorder_are_canonical_metamorphic_properties() -> None:
    data = oracle.deterministic_fixture(40)
    original = oracle.materialize_product_indicators(
        data,
        oracle.BASE_COLUMNS,
        oracle.PREDICTOR_COLUMNS,
        oracle.MODERATOR_COLUMNS,
    )
    reversed_rows = oracle.materialize_product_indicators(
        data[::-1],
        oracle.BASE_COLUMNS,
        oracle.PREDICTOR_COLUMNS,
        oracle.MODERATOR_COLUMNS,
    )
    assert original.product_columns == reversed_rows.product_columns
    np.testing.assert_allclose(original.products, reversed_rows.products[::-1], atol=1e-14, rtol=0.0)
    np.testing.assert_allclose(
        original.product_final_sample_standard_deviations,
        reversed_rows.product_final_sample_standard_deviations,
        atol=1e-14,
        rtol=0.0,
    )

    order = (6, 2, 0, 4, 3, 1, 5)
    reordered_columns = tuple(oracle.BASE_COLUMNS[index] for index in order)
    reordered = oracle.materialize_product_indicators(
        data[:, order],
        reordered_columns,
        tuple(reversed(oracle.PREDICTOR_COLUMNS)),
        tuple(reversed(oracle.MODERATOR_COLUMNS)),
    )
    assert reordered.product_columns == original.product_columns
    np.testing.assert_allclose(reordered.products, original.products, atol=1e-14, rtol=0.0)


def test_sample_standardization_is_positive_affine_invariant() -> None:
    data = oracle.deterministic_fixture(40)
    transformed = oracle.materialize_product_indicators(
        data,
        oracle.BASE_COLUMNS,
        oracle.PREDICTOR_COLUMNS,
        oracle.MODERATOR_COLUMNS,
        standardization="sample_standard_deviation",
    )
    affine = data.copy()
    affine[:, 0] = 3.5 * affine[:, 0] + 200.0
    affine[:, 1] = 0.4 * affine[:, 1] - 17.0
    affine[:, 2] = 2.2 * affine[:, 2] + 9.0
    affine[:, 3] = 5.0 * affine[:, 3] - 100.0
    affine_result = oracle.materialize_product_indicators(
        affine,
        oracle.BASE_COLUMNS,
        oracle.PREDICTOR_COLUMNS,
        oracle.MODERATOR_COLUMNS,
        standardization="sample_standard_deviation",
    )
    np.testing.assert_allclose(transformed.products, affine_result.products, atol=2e-13, rtol=0.0)

    tiny = data.copy()
    tiny[:, :4] *= 1e-20
    tiny_result = oracle.materialize_product_indicators(
        tiny,
        oracle.BASE_COLUMNS,
        oracle.PREDICTOR_COLUMNS,
        oracle.MODERATOR_COLUMNS,
        standardization="sample_standard_deviation",
    )
    assert all(
        0.0 < value < np.finfo(float).eps
        for value in tiny_result.constituent_sample_standard_deviations
    )
    np.testing.assert_allclose(transformed.products, tiny_result.products, atol=2e-13, rtol=0.0)


def test_default_oracle_scope_matches_estimator_minimums() -> None:
    data = oracle.deterministic_fixture(10)
    with pytest.raises(oracle.OracleContractError) as narrow_blocks:
        oracle.materialize_product_indicators(
            data,
            oracle.BASE_COLUMNS,
            ("x1",),
            ("m1",),
        )
    assert narrow_blocks.value.code == "indicator_blocks_invalid"

    with pytest.raises(oracle.OracleContractError) as too_few_rows:
        oracle.materialize_product_indicators(
            data[:9],
            oracle.BASE_COLUMNS,
            oracle.PREDICTOR_COLUMNS,
            oracle.MODERATOR_COLUMNS,
        )
    assert too_few_rows.value.code == "insufficient_complete_observations"


def test_resource_envelope_covers_inside_boundary_outside_and_overflow() -> None:
    inside = oracle.validate_resource_envelope(9_999_999, 1, 1)
    assert inside.materialized_product_cells == 9_999_999

    boundary = oracle.validate_resource_envelope(2_500_000, 2, 2)
    assert boundary.materialized_product_cells == oracle.MAX_MATERIALIZED_PRODUCT_CELLS
    assert boundary.estimated_raw_bytes == 80_000_000
    assert boundary.estimated_peak_bytes == 240_000_000

    with pytest.raises(oracle.OracleContractError) as outside:
        oracle.validate_resource_envelope(2_500_001, 2, 2)
    assert outside.value.code == "materialization_limit_exceeded"

    with pytest.raises(oracle.OracleContractError) as product_count:
        oracle.validate_resource_envelope(10, 10, 10)
    assert product_count.value.code == "product_column_limit_exceeded"

    with pytest.raises(oracle.OracleContractError) as overflow:
        oracle.validate_resource_envelope(oracle.U64_MAX, 9, 9)
    assert overflow.value.code == "resource_size_overflow"


def test_adversarial_values_fail_with_typed_codes() -> None:
    constant = oracle.deterministic_fixture(20)
    constant[:, 0] = 1.0
    with pytest.raises(oracle.OracleContractError) as zero_variance:
        oracle.materialize_product_indicators(
            constant,
            oracle.BASE_COLUMNS,
            oracle.PREDICTOR_COLUMNS,
            oracle.MODERATOR_COLUMNS,
        )
    assert zero_variance.value.code == "constituent_zero_variance"

    non_finite = oracle.deterministic_fixture(20)
    non_finite[3, 2] = np.inf
    with pytest.raises(oracle.OracleContractError) as infinite:
        oracle.materialize_product_indicators(
            non_finite,
            oracle.BASE_COLUMNS,
            oracle.PREDICTOR_COLUMNS,
            oracle.MODERATOR_COLUMNS,
        )
    assert infinite.value.code == "source_value_non_finite"

    overflow = oracle.deterministic_fixture(20)
    overflow[:, :4] *= 1e200
    with pytest.raises(oracle.OracleContractError) as product_overflow:
        oracle.materialize_product_indicators(
            overflow,
            oracle.BASE_COLUMNS,
            oracle.PREDICTOR_COLUMNS,
            oracle.MODERATOR_COLUMNS,
            centering="none",
        )
    assert product_overflow.value.code == "product_value_non_finite"


def test_independent_scipy_oracle_is_identified_and_frozen() -> None:
    report = oracle.build_report()
    oracle.check_report(report)
