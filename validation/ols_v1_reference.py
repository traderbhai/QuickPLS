"""Independent NumPy/SciPy reference equations for bounded QuickPLS OLS v1.

This module is development-validation-only. It deliberately does not import
QuickPLS code: product results are compared with separately expressed linear
algebra and Student-t equations.
"""

from __future__ import annotations

import math
from typing import Any, Iterable, Mapping

import numpy as np
from scipy import stats


def fit_ols_hc3(
    rows: Iterable[Mapping[str, Any]],
    outcome: str,
    predictors: list[str],
    controls: list[str] | None = None,
) -> dict[str, Any]:
    """Fit complete-case OLS with an intercept and HC3 covariance."""

    terms = [*predictors, *(controls or [])]
    selected = [outcome, *terms]
    complete: list[list[float]] = []
    total_rows = 0
    for row in rows:
        total_rows += 1
        values: list[float] = []
        for name in selected:
            try:
                number = float(row.get(name))
            except (TypeError, ValueError):
                break
            if not math.isfinite(number):
                break
            values.append(number)
        if len(values) == len(selected):
            complete.append(values)

    data = np.asarray(complete, dtype=float)
    if data.ndim != 2 or data.shape[0] < 3:
        raise ValueError("fewer than three complete observations")
    y = data[:, 0]
    x = np.column_stack([np.ones(data.shape[0]), data[:, 1:]])
    n, p = x.shape
    if n <= p:
        raise ValueError("positive residual degrees of freedom are required")
    if np.linalg.matrix_rank(x) != p:
        raise ValueError("rank-deficient design")

    xtx_inverse = np.linalg.inv(x.T @ x)
    beta = xtx_inverse @ x.T @ y
    fitted = x @ beta
    residuals = y - fitted
    leverage = np.sum((x @ xtx_inverse) * x, axis=1)
    if np.any(leverage >= 1.0 - 1e-12):
        raise ValueError("HC3 leverage is singular")
    scaled = residuals / (1.0 - leverage)
    hc3 = xtx_inverse @ (x.T @ ((scaled * scaled)[:, None] * x)) @ xtx_inverse
    covariance_diagonal = np.diag(hc3)
    if np.any(~np.isfinite(covariance_diagonal)) or np.any(covariance_diagonal <= 0.0):
        raise ValueError("HC3 covariance diagonal must be finite and strictly positive")
    standard_errors = np.sqrt(covariance_diagonal)
    statistics = np.divide(
        beta,
        standard_errors,
        out=np.full_like(beta, np.inf),
        where=standard_errors > 0.0,
    )
    degrees_freedom = n - p
    p_values = 2.0 * stats.t.sf(np.abs(statistics), degrees_freedom)
    critical = float(stats.t.ppf(0.975, degrees_freedom))

    rss = float(residuals @ residuals)
    centered = y - float(np.mean(y))
    tss = float(centered @ centered)
    r_squared = 1.0 - rss / tss
    adjusted = 1.0 - (1.0 - r_squared) * (n - 1) / degrees_freedom
    rmse = math.sqrt(rss / n)
    f_statistic = (r_squared / (p - 1)) / ((1.0 - r_squared) / degrees_freedom)
    # QuickPLS reports the common-constant-free Gaussian information criterion
    # used by its frozen OLS contract: n log(RSS/n) plus the model penalty.
    gaussian_deviance = n * math.log(rss / n)

    return {
        "terms": ["intercept", *terms],
        "observations": n,
        "omitted_observations": total_rows - n,
        "coefficients": beta.tolist(),
        "standard_errors": standard_errors.tolist(),
        "statistics": statistics.tolist(),
        "p_values": p_values.tolist(),
        "confidence_interval_lower": (beta - critical * standard_errors).tolist(),
        "confidence_interval_upper": (beta + critical * standard_errors).tolist(),
        "fitted": fitted.tolist(),
        "residuals": residuals.tolist(),
        "fit": {
            "r_squared": r_squared,
            "adjusted_r_squared": adjusted,
            "f_statistic": f_statistic,
            "rmse": rmse,
            "aic": gaussian_deviance + 2.0 * p,
            "bic": gaussian_deviance + math.log(n) * p,
        },
    }


def compare_quickpls(
    regression: Mapping[str, Any], reference: Mapping[str, Any]
) -> dict[str, Any]:
    """Return complete, bounded numerical parity diagnostics."""

    observed = {row["term"]: row for row in regression["coefficients"]}
    differences: dict[str, float] = {}
    for index, term in enumerate(reference["terms"]):
        row = observed[term]
        for output_key, reference_key in (
            ("estimate", "coefficients"),
            ("standard_error", "standard_errors"),
            ("statistic", "statistics"),
            ("p_value_two_sided", "p_values"),
            ("confidence_interval_lower", "confidence_interval_lower"),
            ("confidence_interval_upper", "confidence_interval_upper"),
        ):
            differences[f"coefficient.{term}.{output_key}"] = abs(
                float(row[output_key]) - float(reference[reference_key][index])
            )

    fit = regression["fit"]
    for key, expected in reference["fit"].items():
        differences[f"fit.{key}"] = abs(float(fit[key]) - float(expected))
    predictions = regression["predictions"]
    if len(predictions) != reference["observations"]:
        differences["prediction.count"] = float("inf")
    else:
        for index, row in enumerate(predictions):
            differences[f"prediction.{index}.fitted"] = abs(
                float(row["fitted"]) - float(reference["fitted"][index])
            )
            differences[f"prediction.{index}.residual"] = abs(
                float(row["residual"]) - float(reference["residuals"][index])
            )
    maximum = max(differences.values(), default=0.0)
    return {
        "passed": math.isfinite(maximum) and maximum <= 1e-7,
        "max_abs_difference": maximum,
        "difference_count": len(differences),
        "worst_field": max(differences, key=differences.get) if differences else None,
    }
