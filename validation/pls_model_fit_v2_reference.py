#!/usr/bin/env python3
"""Independent NumPy reference for the frozen PLS model-fit v2 equations.

This validation-only oracle intentionally does not import QuickPLS code.  It
uses direct dense linear algebra and is suitable for microcases and generated
positive-definite correlation matrices, not production execution.
"""

from __future__ import annotations

import argparse
import json
import math
from dataclasses import asdict, dataclass
from pathlib import Path

import numpy as np


@dataclass(frozen=True)
class FitValues:
    srmr: float
    d_uls: float
    d_g: float
    chi_square: float
    nfi: float | None


def _correlation_matrix(value: object, label: str) -> np.ndarray:
    matrix = np.asarray(value, dtype=float)
    if matrix.ndim != 2 or matrix.shape[0] == 0 or matrix.shape[0] != matrix.shape[1]:
        raise ValueError(f"{label} must be a nonempty square matrix")
    if not np.isfinite(matrix).all() or not np.allclose(matrix, matrix.T, rtol=0, atol=1e-12):
        raise ValueError(f"{label} must be finite and symmetric")
    if not np.allclose(np.diag(matrix), 1, rtol=0, atol=1e-12):
        raise ValueError(f"{label} must have a unit diagonal")
    if np.any(np.abs(matrix) > 1 + 1e-12):
        raise ValueError(f"{label} is not a correlation matrix")
    np.linalg.cholesky(matrix)
    return matrix


def d_uls(observed: np.ndarray, implied: np.ndarray) -> float:
    residual = observed - implied
    return float(0.5 * np.trace(residual @ residual))


def d_g(observed: np.ndarray, implied: np.ndarray) -> float:
    # eigvalsh on S^(-1/2) Sigma S^(-1/2) gives the real generalized
    # eigenvalues without relying on the nonsymmetric product inv(S) Sigma.
    lower = np.linalg.cholesky(observed)
    inverse_lower = np.linalg.inv(lower)
    whitened = inverse_lower @ implied @ inverse_lower.T
    eigenvalues = np.linalg.eigvalsh((whitened + whitened.T) / 2)
    if np.any(eigenvalues <= 0):
        raise ValueError("geodesic discrepancy requires positive eigenvalues")
    return float(0.5 * np.sum(np.log(eigenvalues) ** 2))


def maximum_likelihood_discrepancy(observed: np.ndarray, implied: np.ndarray) -> float:
    sign_observed, logdet_observed = np.linalg.slogdet(observed)
    sign_implied, logdet_implied = np.linalg.slogdet(implied)
    if sign_observed <= 0 or sign_implied <= 0:
        raise ValueError("ML discrepancy requires positive-definite matrices")
    value = float(
        np.trace(np.linalg.solve(implied, observed))
        - logdet_observed
        + logdet_implied
        - observed.shape[0]
    )
    if value < -1e-10 or not math.isfinite(value):
        raise ValueError("invalid ML discrepancy")
    return max(0.0, value)


def fit_values(
    observed_value: object,
    implied_value: object,
    sample_size: int,
    null_implied_value: object | None = None,
) -> FitValues:
    if sample_size < 2:
        raise ValueError("sample_size must be at least two")
    observed = _correlation_matrix(observed_value, "observed")
    implied = _correlation_matrix(implied_value, "implied")
    if observed.shape != implied.shape:
        raise ValueError("observed and implied dimensions differ")
    null_implied = _correlation_matrix(
        np.eye(observed.shape[0]) if null_implied_value is None else null_implied_value,
        "null_implied",
    )
    discrepancy = d_uls(observed, implied)
    chi_square = (sample_size - 1) * maximum_likelihood_discrepancy(observed, implied)
    null_chi_square = (sample_size - 1) * maximum_likelihood_discrepancy(observed, null_implied)
    return FitValues(
        srmr=math.sqrt(discrepancy / (observed.shape[0] * (observed.shape[0] + 1) / 2)),
        d_uls=discrepancy,
        d_g=d_g(observed, implied),
        chi_square=chi_square,
        nfi=None if null_chi_square <= np.finfo(float).eps else 1 - chi_square / null_chi_square,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, help="JSON with observed, implied, and sample_size")
    args = parser.parse_args()
    if args.input:
        payload = json.loads(args.input.read_text(encoding="utf-8"))
        result = fit_values(
            payload["observed"],
            payload["implied"],
            int(payload["sample_size"]),
            payload.get("null_implied"),
        )
    else:
        result = fit_values(
            [[1.0, 0.4], [0.4, 1.0]],
            [[1.0, 0.1], [0.1, 1.0]],
            100,
        )
    print(json.dumps(asdict(result), indent=2, sort_keys=True, allow_nan=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
