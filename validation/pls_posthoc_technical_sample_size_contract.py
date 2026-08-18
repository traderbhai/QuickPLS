#!/usr/bin/env python3
"""Transparent independent oracle for the inference-aware post-hoc PLS result.

This development-only module deliberately has no product-code imports. It
implements the frozen arithmetic, identity matching, significance selection,
and typed unavailable states in straightforward Python.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Iterable


METHOD_VERSION = "inverse_square_root_posthoc_v2"
FORMULA_CONSTANT = 2.486
ALPHA = 0.05
POWER = 0.80
TEST_DIRECTION = "directional"
SIGNIFICANCE_ALPHA = 0.05
SIGNIFICANCE_ALTERNATIVE = "two_sided"
SIGNIFICANCE_SOURCE = "pls_bootstrap_normal_reference_two_sided"
SELECTION_RULE = "smallest_absolute_statistically_significant_structural_path"
MAX_SUPPORTED_SAMPLE_SIZE = (1 << 53) - 1


@dataclass(frozen=True)
class StructuralPath:
    source_id: str
    target_id: str
    coefficient: float


@dataclass(frozen=True)
class PathSignificance:
    source_id: str
    target_id: str
    p_value_two_sided: float | None


def technical_minimum_sample_size(
    paths: Iterable[StructuralPath],
    inference: Iterable[PathSignificance] | None,
    *,
    analytical_sample_size: int = 0,
) -> dict[str, object]:
    """Evaluate the v2 result without relying on QuickPLS product code."""

    ordered = list(paths)
    if any(
        not path.source_id.strip()
        or not path.target_id.strip()
        or not math.isfinite(path.coefficient)
        for path in ordered
    ):
        return _result(
            "inference_incomplete",
            ordered,
            analytical_sample_size,
            inference_present=inference is not None,
        )
    identities = {(path.source_id, path.target_id) for path in ordered}
    if len(identities) != len(ordered):
        return _result(
            "inference_incomplete", ordered, analytical_sample_size, inference_present=inference is not None
        )
    if not ordered:
        return _result("not_applicable_no_structural_path", ordered, analytical_sample_size)
    if inference is None:
        return _result("inference_unavailable", ordered, analytical_sample_size)

    probabilities: dict[tuple[str, str], float] = {}
    for row in inference:
        identity = (row.source_id, row.target_id)
        probability = row.p_value_two_sided
        if (
            identity not in identities
            or identity in probabilities
            or probability is None
            or not math.isfinite(probability)
            or not 0.0 <= probability <= 1.0
        ):
            return _result(
                "inference_incomplete", ordered, analytical_sample_size, inference_present=True
            )
        probabilities[identity] = probability
    if set(probabilities) != identities:
        return _result(
            "inference_incomplete", ordered, analytical_sample_size, inference_present=True
        )

    significant = sorted(
        (
            path
            for path in ordered
            if probabilities[(path.source_id, path.target_id)] <= SIGNIFICANCE_ALPHA
        ),
        key=lambda path: (
            abs(path.coefficient),
            path.source_id.encode("utf-8"),
            path.target_id.encode("utf-8"),
        ),
    )
    if not significant:
        return _result(
            "no_statistically_significant_path",
            ordered,
            analytical_sample_size,
            inference_present=True,
            significant_path_count=0,
        )

    driver = significant[0]
    probability = probabilities[(driver.source_id, driver.target_id)]
    absolute_driver = abs(driver.coefficient)
    if absolute_driver == 0.0:
        return _result(
            "undefined_zero_path",
            ordered,
            analytical_sample_size,
            inference_present=True,
            significant_path_count=len(significant),
            driver=driver,
            driver_p_value=probability,
        )
    ratio = FORMULA_CONSTANT / absolute_driver
    if not math.isfinite(ratio) or ratio > math.sqrt(MAX_SUPPORTED_SAMPLE_SIZE):
        return _result(
            "exceeds_supported_integer_range",
            ordered,
            analytical_sample_size,
            inference_present=True,
            significant_path_count=len(significant),
            driver=driver,
            driver_p_value=probability,
        )
    required = math.ceil(ratio * ratio)
    return _result(
        "available",
        ordered,
        analytical_sample_size,
        inference_present=True,
        significant_path_count=len(significant),
        driver=driver,
        driver_p_value=probability,
        required_sample_size=required,
    )


def _result(
    status: str,
    paths: list[StructuralPath],
    analytical_sample_size: int,
    *,
    inference_present: bool = False,
    significant_path_count: int | None = None,
    driver: StructuralPath | None = None,
    driver_p_value: float | None = None,
    required_sample_size: int | None = None,
) -> dict[str, object]:
    return {
        "method_version": METHOD_VERSION,
        "formula_constant": FORMULA_CONSTANT,
        "alpha": ALPHA,
        "power": POWER,
        "test_direction": TEST_DIRECTION,
        "selection_rule": SELECTION_RULE,
        "significance_alternative": SIGNIFICANCE_ALTERNATIVE if inference_present else None,
        "significance_source": SIGNIFICANCE_SOURCE if inference_present else None,
        "significance_alpha": SIGNIFICANCE_ALPHA if inference_present else None,
        "eligible_path_count": len(paths),
        "significant_path_count": significant_path_count,
        "status": status,
        "driver": driver,
        "driver_p_value_two_sided": driver_p_value,
        "minimum_absolute_path_coefficient": abs(driver.coefficient) if driver else None,
        "required_sample_size": required_sample_size,
        "analytical_sample_size": analytical_sample_size,
        "meets_technical_requirement": (
            analytical_sample_size >= required_sample_size
            if required_sample_size is not None
            else None
        ),
    }
