#!/usr/bin/env python3
"""Independent work oracle for continuous raw indicator-mean replacement v1.

This validation-only module imports no QuickPLS product code and executes no
QuickPLS binary.  ``None`` is the explicit stand-in for an Arrow null.  Every
non-null finite numeric value, including a sentinel-looking value such as
``-99``, is observed data.  The oracle freezes column-wise indicator mean
replacement; it is not row-wise aggregation, pairwise deletion, FIML, or a
resampling-time imputation procedure.

Passing this oracle is work evidence only.  It cannot attach a qualification
receipt, mutate the Capability Registry or a method manifest, or authorize a
distribution-tier promotion.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from dataclasses import asdict, dataclass, replace
from fractions import Fraction
from pathlib import Path
from typing import Any, Literal, Sequence


ORACLE_VERSION = "independent_pls_mean_replacement_v1_work_oracle_v1"
METHOD_VERSION = "pls_indicator_mean_replacement_v1"
CONTRACT_VERSION = "pls_indicator_mean_replacement_v1"
MISSING_REPRESENTATION = "arrow_null_only"
MEAN_TOLERANCE = 2.0e-13
VARIANCE_TOLERANCE = 5.0e-13

InputKind = Literal["raw", "covariance", "correlation"]
Scale = Literal["continuous", "binary", "ordinal", "nominal", "identifier"]


class MeanReplacementOracleFailure(ValueError):
    """Typed, deterministic rejection from the independent work oracle."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code


@dataclass(frozen=True)
class OracleIndicatorBinding:
    """Stable scientific identity bound to one physical raw-data column."""

    indicator_id: str
    source_column: str
    sem_scale: Scale = "continuous"
    dataset_scale: Scale = "continuous"


@dataclass(frozen=True)
class OracleMeanReplacementSettings:
    """Deliberately narrow execution envelope for the v1 work contract."""

    input_kind: InputKind = "raw"
    missing_policy: str = "mean_replacement"
    weight_requested: bool = False
    group_count: int = 1
    bootstrap_samples: int = 0
    studentized_inner_samples: int = 0
    permutation_samples: int = 0
    prediction_requested: bool = False


def _canonical_sha256(value: Any) -> str:
    payload = json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _fail(code: str, message: str) -> None:
    raise MeanReplacementOracleFailure(code, message)


def _validate_settings(settings: OracleMeanReplacementSettings) -> None:
    if settings.input_kind != "raw":
        _fail(
            "pls_mean_replacement_v1_raw_data_required",
            "v1 requires resident raw observations",
        )
    if settings.missing_policy != "mean_replacement":
        _fail(
            "pls_mean_replacement_v1_policy_mismatch",
            "v1 cannot substitute another missing-data policy",
        )
    if settings.weight_requested:
        _fail(
            "pls_mean_replacement_v1_weights_unsupported",
            "weighted means require a separate contract",
        )
    if settings.group_count != 1:
        _fail(
            "pls_mean_replacement_v1_groups_unsupported",
            "pooled versus group-specific means require a separate contract",
        )
    counts = (
        settings.bootstrap_samples,
        settings.studentized_inner_samples,
        settings.permutation_samples,
    )
    if any(count < 0 for count in counts):
        _fail(
            "pls_mean_replacement_v1_resampling_invalid",
            "resampling counts cannot be negative",
        )
    if any(count != 0 for count in counts):
        _fail(
            "pls_mean_replacement_v1_resampling_unsupported",
            "imputation placement inside resampling requires a separate contract",
        )
    if settings.prediction_requested:
        _fail(
            "pls_mean_replacement_v1_prediction_unsupported",
            "prediction requires train-only imputation to prevent leakage",
        )


def _validate_identities(
    row_ids: Sequence[str],
    variables: Sequence[str],
    bindings: Sequence[OracleIndicatorBinding],
) -> dict[str, int]:
    if len(set(row_ids)) != len(row_ids) or any(
        not isinstance(row_id, str) or not row_id for row_id in row_ids
    ):
        _fail(
            "pls_mean_replacement_v1_row_id_invalid",
            "row identifiers must be non-empty and unique",
        )
    if len(set(variables)) != len(variables) or any(
        not isinstance(variable, str) or not variable for variable in variables
    ):
        _fail(
            "pls_mean_replacement_v1_variable_id_invalid",
            "source column names must be non-empty and unique",
        )
    if not bindings:
        _fail(
            "pls_mean_replacement_v1_no_indicators",
            "at least one analytical indicator is required",
        )
    indicator_ids = [binding.indicator_id for binding in bindings]
    source_columns = [binding.source_column for binding in bindings]
    if (
        len(set(indicator_ids)) != len(indicator_ids)
        or len(set(source_columns)) != len(source_columns)
        or any(not value for value in (*indicator_ids, *source_columns))
    ):
        _fail(
            "pls_mean_replacement_v1_binding_invalid",
            "indicator and source-column identities must each be non-empty and unique",
        )
    positions = {variable: index for index, variable in enumerate(variables)}
    unknown = sorted(set(source_columns) - set(positions))
    if unknown:
        _fail(
            "pls_mean_replacement_v1_unknown_source_column",
            f"unknown source columns: {', '.join(unknown)}",
        )
    for binding in bindings:
        if binding.sem_scale != "continuous" or binding.dataset_scale != "continuous":
            _fail(
                "pls_mean_replacement_v1_continuous_indicator_required",
                f"{binding.indicator_id} must be continuous in both model and dataset metadata",
            )
    return positions


def _coerce_analytical_value(
    value: Any,
    *,
    row_index: int,
    binding: OracleIndicatorBinding,
) -> float | None:
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        _fail(
            "pls_mean_replacement_v1_nonnumeric_observed_value",
            f"row {row_index}, {binding.indicator_id} is nonnumeric",
        )
    try:
        converted = float(value)
    except (OverflowError, ValueError):
        _fail(
            "pls_mean_replacement_v1_non_finite_observed_value",
            f"row {row_index}, {binding.indicator_id} is outside finite f64 range",
        )
    if not math.isfinite(converted):
        _fail(
            "pls_mean_replacement_v1_non_finite_observed_value",
            f"row {row_index}, {binding.indicator_id} is non-finite",
        )
    return converted


def _exact_f64_mean(values: Sequence[float]) -> tuple[Fraction, float]:
    exact = sum((Fraction.from_float(value) for value in values), Fraction()) / len(
        values
    )
    rounded = float(exact)
    if not math.isfinite(rounded):
        _fail(
            "pls_mean_replacement_v1_numeric_failure",
            "the replacement mean is not representable as finite f64",
        )
    return exact, rounded


def _exact_sample_variance(values: Sequence[float]) -> Fraction:
    if len(values) < 2:
        return Fraction()
    fractions = [Fraction.from_float(value) for value in values]
    mean = sum(fractions, Fraction()) / len(fractions)
    return sum(((value - mean) ** 2 for value in fractions), Fraction()) / (
        len(fractions) - 1
    )


def _finite_variance(exact: Fraction, subject: str) -> float:
    value = float(exact)
    if not math.isfinite(value) or value <= 0.0:
        _fail(
            "pls_mean_replacement_v1_numeric_failure",
            f"{subject} variance is not representable as positive finite f64",
        )
    return value


def _cell_wire(value: float | None) -> str | None:
    return None if value is None else value.hex()


def _warning_codes_for_indicator(missing: int, total: int) -> list[str]:
    warnings: list[str] = []
    if missing * 20 >= total:
        warnings.append("pls_mean_replacement_v1_indicator_missing_at_least_5_percent")
    if missing * 20 > total * 3:
        warnings.append("pls_mean_replacement_v1_indicator_missing_above_15_percent")
    return warnings


def _warning_codes_for_case(missing: int, indicator_count: int) -> list[str]:
    if missing * 20 > indicator_count * 3:
        return ["pls_mean_replacement_v1_case_missing_above_15_percent"]
    return []


def mean_replace_continuous_raw_v1(
    rows: Sequence[Sequence[Any]],
    row_ids: Sequence[str],
    variables: Sequence[str],
    bindings: Sequence[OracleIndicatorBinding],
    settings: OracleMeanReplacementSettings = OracleMeanReplacementSettings(),
) -> dict[str, Any]:
    """Return a transparent completed-matrix work result for the bounded v1 scope."""

    _validate_settings(settings)
    if len(rows) != len(row_ids):
        _fail(
            "pls_mean_replacement_v1_row_id_count_mismatch",
            "every physical row requires one row identity",
        )
    if len(rows) < 3:
        _fail(
            "pls_mean_replacement_v1_insufficient_observations",
            "point estimation requires at least three raw rows",
        )
    positions = _validate_identities(row_ids, variables, bindings)
    for row_index, row in enumerate(rows):
        if len(row) != len(variables):
            _fail(
                "pls_mean_replacement_v1_row_width_mismatch",
                f"row {row_index} has {len(row)} cells for {len(variables)} columns",
            )

    ordered_bindings = tuple(sorted(bindings, key=lambda binding: binding.indicator_id))
    analytical: dict[str, list[float | None]] = {}
    for binding in ordered_bindings:
        position = positions[binding.source_column]
        analytical[binding.indicator_id] = [
            _coerce_analytical_value(
                row[position], row_index=row_index, binding=binding
            )
            for row_index, row in enumerate(rows)
        ]

    completed_rows = [list(row) for row in rows]
    diagnostics: list[dict[str, Any]] = []
    total_missing = 0
    global_warnings: set[str] = set()
    replacement_means: dict[str, float] = {}
    for binding in ordered_bindings:
        column = analytical[binding.indicator_id]
        observed = [value for value in column if value is not None]
        missing = len(column) - len(observed)
        if not observed:
            _fail(
                "pls_mean_replacement_v1_all_missing_indicator",
                f"{binding.indicator_id} has no observed value from which to compute a mean",
            )
        exact_mean, replacement_mean = _exact_f64_mean(observed)
        if len(set(observed)) < 2:
            _fail(
                "pls_mean_replacement_v1_constant_indicator_after_replacement",
                f"{binding.indicator_id} is constant after mean replacement",
            )
        position = positions[binding.source_column]
        for row_index, value in enumerate(column):
            completed_rows[row_index][position] = (
                replacement_mean if value is None else value
            )
        completed = [float(row[position]) for row in completed_rows]
        observed_variance = _finite_variance(
            _exact_sample_variance(observed), f"{binding.indicator_id} observed"
        )
        completed_variance = _finite_variance(
            _exact_sample_variance(completed), f"{binding.indicator_id} completed"
        )
        expected_completed_variance = observed_variance * (
            (len(observed) - 1) / (len(rows) - 1)
        )
        variance_identity_difference = abs(
            completed_variance - expected_completed_variance
        )
        warning_codes = _warning_codes_for_indicator(missing, len(rows))
        global_warnings.update(warning_codes)
        diagnostics.append(
            {
                "indicator_id": binding.indicator_id,
                "source_column": binding.source_column,
                "sem_scale": binding.sem_scale,
                "dataset_scale": binding.dataset_scale,
                "observed_before": len(observed),
                "missing_before": missing,
                "missing_fraction_before": missing / len(rows),
                "replacement_mean": replacement_mean,
                "replacement_mean_f64_hex": replacement_mean.hex(),
                "exact_binary_mean_numerator": str(exact_mean.numerator),
                "exact_binary_mean_denominator": str(exact_mean.denominator),
                "missing_after": 0,
                "observed_sample_variance": observed_variance,
                "completed_sample_variance": completed_variance,
                "expected_completed_sample_variance": expected_completed_variance,
                "variance_identity_abs_difference": variance_identity_difference,
                "warning_codes": warning_codes,
            }
        )
        replacement_means[binding.indicator_id] = replacement_mean
        total_missing += missing

    if total_missing:
        global_warnings.add("pls_mean_replacement_v1_variance_attenuation")

    case_diagnostics: list[dict[str, Any]] = []
    for row_index, row_id in enumerate(row_ids):
        missing = sum(
            analytical[binding.indicator_id][row_index] is None
            for binding in ordered_bindings
        )
        warning_codes = _warning_codes_for_case(missing, len(ordered_bindings))
        global_warnings.update(warning_codes)
        case_diagnostics.append(
            {
                "source_row_index": row_index,
                "row_id": row_id,
                "missing_before": missing,
                "missing_fraction_before": missing / len(ordered_bindings),
                "replaced_count": missing,
                "missing_after": 0,
                "warning_codes": warning_codes,
            }
        )

    input_wire = {
        "row_ids": list(row_ids),
        "bindings": [asdict(binding) for binding in ordered_bindings],
        "cells": [
            [_cell_wire(analytical[binding.indicator_id][row_index]) for binding in ordered_bindings]
            for row_index in range(len(rows))
        ],
    }
    completed_wire = {
        "row_ids": list(row_ids),
        "bindings": [asdict(binding) for binding in ordered_bindings],
        "cells": [
            [
                float(completed_rows[row_index][positions[binding.source_column]]).hex()
                for binding in ordered_bindings
            ]
            for row_index in range(len(rows))
        ],
    }
    missing_mask_wire = {
        "row_ids": list(row_ids),
        "indicator_ids": [binding.indicator_id for binding in ordered_bindings],
        "missing": [
            [
                analytical[binding.indicator_id][row_index] is None
                for binding in ordered_bindings
            ]
            for row_index in range(len(rows))
        ],
    }
    return {
        "schema_version": 1,
        "method_version": METHOD_VERSION,
        "contract_version": CONTRACT_VERSION,
        "oracle_version": ORACLE_VERSION,
        "missing_representation": MISSING_REPRESENTATION,
        "non_null_sentinels_are_observed": True,
        "source_dataset_mutated": False,
        "settings": asdict(settings),
        "variables": list(variables),
        "analytical_indicator_order": [
            binding.indicator_id for binding in ordered_bindings
        ],
        "analytical_source_columns": [
            binding.source_column for binding in ordered_bindings
        ],
        "input_sha256": _canonical_sha256(input_wire),
        "missing_mask_sha256": _canonical_sha256(missing_mask_wire),
        "prepared_matrix_sha256": _canonical_sha256(completed_wire),
        "used_observations": len(rows),
        "omitted_observations": 0,
        "total_missing_before": total_missing,
        "total_missing_after": 0,
        "replacement_means": replacement_means,
        "indicator_diagnostics": diagnostics,
        "case_diagnostics": case_diagnostics,
        "warnings": sorted(global_warnings),
        "completed_rows": completed_rows,
    }


def deterministic_fixture() -> dict[str, Any]:
    return {
        "rows": [
            [1.0, 10.0, 100.0],
            [None, 20.0, 110.0],
            [5.0, None, 120.0],
            [9.0, 40.0, 130.0],
            [13.0, 50.0, None],
            [17.0, 60.0, 150.0],
        ],
        "row_ids": ["r1", "r2", "r3", "r4", "r5", "r6"],
        "variables": ["x", "y", "z"],
        "bindings": [
            OracleIndicatorBinding("indicator:x", "x"),
            OracleIndicatorBinding("indicator:y", "y"),
            OracleIndicatorBinding("indicator:z", "z"),
        ],
    }


def no_complete_row_fixture() -> dict[str, Any]:
    return {
        "rows": [
            [1.0, None],
            [None, 20.0],
            [5.0, None],
            [None, 40.0],
            [None, None],
        ],
        "row_ids": ["r1", "r2", "r3", "r4", "fully-missing"],
        "variables": ["x", "y"],
        "bindings": [
            OracleIndicatorBinding("indicator:x", "x"),
            OracleIndicatorBinding("indicator:y", "y"),
        ],
    }


def _assert_failure(code: str, callback: Any) -> bool:
    try:
        callback()
    except MeanReplacementOracleFailure as error:
        return error.code == code
    return False


def _completed_by_id(result: dict[str, Any]) -> dict[str, list[float]]:
    positions = {name: index for index, name in enumerate(result["variables"])}
    return {
        row["row_id"]: [
            float(result["completed_rows"][row["source_row_index"]][positions[column]])
            for column in result["analytical_source_columns"]
        ]
        for row in result["case_diagnostics"]
    }


def _standardize(values: Sequence[float]) -> list[float]:
    exact_values = [Fraction.from_float(value) for value in values]
    exact_mean = sum(exact_values, Fraction()) / len(exact_values)
    variance = sum(((value - exact_mean) ** 2 for value in exact_values), Fraction()) / (
        len(values) - 1
    )
    deviation = math.sqrt(float(variance))
    return [(value - float(exact_mean)) / deviation for value in values]


def run_work_checks() -> dict[str, Any]:
    fixture = deterministic_fixture()
    baseline = mean_replace_continuous_raw_v1(**fixture)
    repeated = mean_replace_continuous_raw_v1(**fixture)
    no_complete = mean_replace_continuous_raw_v1(**no_complete_row_fixture())

    reversed_fixture = {
        **fixture,
        "rows": list(reversed(fixture["rows"])),
        "row_ids": list(reversed(fixture["row_ids"])),
    }
    reversed_result = mean_replace_continuous_raw_v1(**reversed_fixture)
    row_order_difference = max(
        abs(left - right)
        for row_id, baseline_row in _completed_by_id(baseline).items()
        for left, right in zip(baseline_row, _completed_by_id(reversed_result)[row_id])
    )

    binding_order = mean_replace_continuous_raw_v1(
        **{**fixture, "bindings": list(reversed(fixture["bindings"]))}
    )
    binding_order_difference = max(
        abs(left - right)
        for left_row, right_row in zip(
            baseline["completed_rows"], binding_order["completed_rows"]
        )
        for left, right in zip(left_row, right_row)
    )

    column_order = [2, 0, 1]
    column_fixture = {
        **fixture,
        "variables": [fixture["variables"][index] for index in column_order],
        "rows": [
            [row[index] for index in column_order] for row in fixture["rows"]
        ],
    }
    column_result = mean_replace_continuous_raw_v1(**column_fixture)
    column_result_positions = {
        name: index for index, name in enumerate(column_result["variables"])
    }
    column_order_difference = max(
        abs(
            float(baseline["completed_rows"][row_index][source_index])
            - float(
                column_result["completed_rows"][row_index][
                    column_result_positions[source_name]
                ]
            )
        )
        for row_index in range(len(fixture["rows"]))
        for source_index, source_name in enumerate(fixture["variables"])
    )

    renamed = mean_replace_continuous_raw_v1(
        **{
            **fixture,
            "bindings": [
                replace(binding, indicator_id=f"renamed:{index}")
                for index, binding in enumerate(fixture["bindings"])
            ],
        }
    )
    stable_id_renaming_difference = max(
        abs(float(left) - float(right))
        for left_row, right_row in zip(baseline["completed_rows"], renamed["completed_rows"])
        for left, right in zip(left_row, right_row)
    )

    affine_parameters = ((2.0, 3.0), (0.5, -10.0), (4.0, 1.0))
    affine_rows = [
        [
            None if value is None else affine_parameters[index][0] * value + affine_parameters[index][1]
            for index, value in enumerate(row)
        ]
        for row in fixture["rows"]
    ]
    affine = mean_replace_continuous_raw_v1(**{**fixture, "rows": affine_rows})
    affine_standardized_difference = max(
        abs(left - right)
        for column_index in range(len(fixture["variables"]))
        for left, right in zip(
            _standardize(
                [float(row[column_index]) for row in baseline["completed_rows"]]
            ),
            _standardize(
                [float(row[column_index]) for row in affine["completed_rows"]]
            ),
        )
    )

    idempotent = mean_replace_continuous_raw_v1(
        baseline["completed_rows"],
        fixture["row_ids"],
        fixture["variables"],
        fixture["bindings"],
    )
    idempotence_difference = max(
        abs(float(left) - float(right))
        for left_row, right_row in zip(
            baseline["completed_rows"], idempotent["completed_rows"]
        )
        for left, right in zip(left_row, right_row)
    )

    unused = mean_replace_continuous_raw_v1(
        [list(row) + [None if index % 2 else "unused"] for index, row in enumerate(fixture["rows"])],
        fixture["row_ids"],
        [*fixture["variables"], "unused"],
        fixture["bindings"],
    )
    unused_column_difference = max(
        abs(float(left) - float(right))
        for left_row, right_row in zip(
            baseline["completed_rows"], unused["completed_rows"]
        )
        for left, right in zip(left_row, right_row[:3])
    )

    boundary_fixture = {
        "rows": [[1.0], [None], [3.0]],
        "row_ids": ["r1", "r2", "r3"],
        "variables": ["x"],
        "bindings": [OracleIndicatorBinding("indicator:x", "x")],
    }
    typed_boundaries = {
        "matrix_input": _assert_failure(
            "pls_mean_replacement_v1_raw_data_required",
            lambda: mean_replace_continuous_raw_v1(
                **boundary_fixture,
                settings=OracleMeanReplacementSettings(input_kind="covariance"),
            ),
        ),
        "policy_substitution": _assert_failure(
            "pls_mean_replacement_v1_policy_mismatch",
            lambda: mean_replace_continuous_raw_v1(
                **boundary_fixture,
                settings=OracleMeanReplacementSettings(missing_policy="pairwise_deletion"),
            ),
        ),
        "weights": _assert_failure(
            "pls_mean_replacement_v1_weights_unsupported",
            lambda: mean_replace_continuous_raw_v1(
                **boundary_fixture,
                settings=OracleMeanReplacementSettings(weight_requested=True),
            ),
        ),
        "groups": _assert_failure(
            "pls_mean_replacement_v1_groups_unsupported",
            lambda: mean_replace_continuous_raw_v1(
                **boundary_fixture,
                settings=OracleMeanReplacementSettings(group_count=2),
            ),
        ),
        "resampling": _assert_failure(
            "pls_mean_replacement_v1_resampling_unsupported",
            lambda: mean_replace_continuous_raw_v1(
                **boundary_fixture,
                settings=OracleMeanReplacementSettings(bootstrap_samples=1),
            ),
        ),
        "prediction": _assert_failure(
            "pls_mean_replacement_v1_prediction_unsupported",
            lambda: mean_replace_continuous_raw_v1(
                **boundary_fixture,
                settings=OracleMeanReplacementSettings(prediction_requested=True),
            ),
        ),
        "noncontinuous": _assert_failure(
            "pls_mean_replacement_v1_continuous_indicator_required",
            lambda: mean_replace_continuous_raw_v1(
                **{
                    **boundary_fixture,
                    "bindings": [
                        OracleIndicatorBinding(
                            "indicator:x", "x", sem_scale="binary", dataset_scale="binary"
                        )
                    ],
                }
            ),
        ),
        "nonfinite": _assert_failure(
            "pls_mean_replacement_v1_non_finite_observed_value",
            lambda: mean_replace_continuous_raw_v1(
                **{**boundary_fixture, "rows": [[1.0], [math.inf], [3.0]]}
            ),
        ),
        "all_missing": _assert_failure(
            "pls_mean_replacement_v1_all_missing_indicator",
            lambda: mean_replace_continuous_raw_v1(
                **{**boundary_fixture, "rows": [[None], [None], [None]]}
            ),
        ),
        "constant": _assert_failure(
            "pls_mean_replacement_v1_constant_indicator_after_replacement",
            lambda: mean_replace_continuous_raw_v1(
                **{**boundary_fixture, "rows": [[2.0], [None], [2.0]]}
            ),
        ),
        "insufficient_n": _assert_failure(
            "pls_mean_replacement_v1_insufficient_observations",
            lambda: mean_replace_continuous_raw_v1(
                [[1.0], [2.0]],
                ["r1", "r2"],
                ["x"],
                boundary_fixture["bindings"],
            ),
        ),
    }

    metamorphic_differences = {
        "row_order": row_order_difference,
        "binding_declaration_order": binding_order_difference,
        "physical_column_order": column_order_difference,
        "stable_id_renaming": stable_id_renaming_difference,
        "positive_affine_standardization": affine_standardized_difference,
        "idempotence": idempotence_difference,
        "unused_column_independence": unused_column_difference,
    }
    variance_differences = [
        row["variance_identity_abs_difference"]
        for row in baseline["indicator_diagnostics"]
    ]
    no_complete_by_id = _completed_by_id(no_complete)
    checks = {
        "repeat_deterministic": baseline == repeated,
        "all_rows_retained": baseline["used_observations"] == len(fixture["rows"])
        and baseline["omitted_observations"] == 0,
        "all_modeled_nulls_filled": baseline["total_missing_after"] == 0,
        "column_specific_means": baseline["replacement_means"]
        == {"indicator:x": 9.0, "indicator:y": 36.0, "indicator:z": 122.0},
        "fully_missing_modeled_row_retained": no_complete_by_id["fully-missing"]
        == [3.0, 30.0],
        "no_complete_input_rows_required": no_complete["used_observations"] == 5,
        "variance_identity": max(variance_differences) <= VARIANCE_TOLERANCE,
        "metamorphics_within_tolerance": max(metamorphic_differences.values())
        <= MEAN_TOLERANCE,
        "idempotent_second_pass_has_zero_replacements": idempotent[
            "total_missing_before"
        ]
        == 0,
        "typed_boundaries_exact": all(typed_boundaries.values()),
        "warning_boundaries_present": {
            "pls_mean_replacement_v1_indicator_missing_at_least_5_percent",
            "pls_mean_replacement_v1_indicator_missing_above_15_percent",
            "pls_mean_replacement_v1_case_missing_above_15_percent",
            "pls_mean_replacement_v1_variance_attenuation",
        }.issubset(baseline["warnings"]),
    }
    return {
        "schema_version": 1,
        "report_kind": "pls_mean_replacement_v1_independent_oracle_work_v1",
        "oracle_version": ORACLE_VERSION,
        "oracle_source_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "method_version": METHOD_VERSION,
        "contract_version": CONTRACT_VERSION,
        "passed": all(checks.values()),
        "work_evidence_only": True,
        "qualification_ready": False,
        "promotion_requested": False,
        "promotion_allowed": False,
        "product_comparison_performed": False,
        "receipt_attached": False,
        "registry_or_manifest_mutation_requested": False,
        "checks": checks,
        "fixture_summary": {
            "input_sha256": baseline["input_sha256"],
            "missing_mask_sha256": baseline["missing_mask_sha256"],
            "prepared_matrix_sha256": baseline["prepared_matrix_sha256"],
            "used_observations": baseline["used_observations"],
            "omitted_observations": baseline["omitted_observations"],
            "total_missing_before": baseline["total_missing_before"],
            "total_missing_after": baseline["total_missing_after"],
            "replacement_means": baseline["replacement_means"],
            "warnings": baseline["warnings"],
        },
        "no_complete_row_case": {
            "prepared_matrix_sha256": no_complete["prepared_matrix_sha256"],
            "fully_missing_row_completed_values": no_complete_by_id["fully-missing"],
            "used_observations": no_complete["used_observations"],
            "omitted_observations": no_complete["omitted_observations"],
        },
        "mean_tolerance": MEAN_TOLERANCE,
        "variance_tolerance": VARIANCE_TOLERANCE,
        "variance_identity_max_abs_difference": max(variance_differences),
        "metamorphic_max_abs_differences": metamorphic_differences,
        "typed_boundaries": typed_boundaries,
        "qualification_spec_v2_alignment": {
            "candidate_oracle_kind": "independent_implementation",
            "runtime_policy": "development_validation_only",
            "covered_work_estimands": [
                "per_indicator_observed_mean",
                "completed_analytical_matrix",
                "per_indicator_missing_diagnostics",
                "per_case_missing_diagnostics",
                "row_retention",
                "variance_attenuation_identity",
            ],
            "admissible_receipt": False,
            "reason_not_admissible": (
                "no frozen current QuickPLS build was executed against these cases"
            ),
        },
        "remaining_blockers": [
            "A frozen current QuickPLS product build has not been compared with this oracle.",
            "Source-set, scenario-set, executable, command, and immutable receipt identities are absent.",
            "Archive, native, export, packaged Windows, accessibility, performance, soak, and independent-review evidence remains required.",
            "Weights, multiple groups, resampling, prediction, matrix input, and noncontinuous indicators remain explicitly outside v1.",
        ],
    }


def write_work_report(path: Path) -> dict[str, Any]:
    report = run_work_checks()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--write-work-report",
        type=Path,
        help="write a deterministic non-promotional work report",
    )
    args = parser.parse_args()
    report = (
        write_work_report(args.write_work_report)
        if args.write_work_report is not None
        else run_work_checks()
    )
    print(json.dumps(report, indent=2, sort_keys=True, allow_nan=False))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
