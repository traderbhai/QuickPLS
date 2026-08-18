"""Transparent validation-only oracle for exact CB-SEM parameter-table semantics.

This module imports no QuickPLS product code and does not execute a product
binary.  It is deliberately small: its purpose is to freeze matrix placement
and fail-closed covariance semantics, not to qualify the production optimizer.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np


class OracleContractError(ValueError):
    """Typed validation-oracle failure."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


@dataclass(frozen=True)
class ExactMatrices:
    lambda_matrix: np.ndarray
    beta: np.ndarray
    psi: np.ndarray
    theta: np.ndarray
    phi: np.ndarray
    sigma: np.ndarray


def _square(name: str, matrix: np.ndarray, size: int) -> np.ndarray:
    value = np.asarray(matrix, dtype=float)
    if value.shape != (size, size):
        raise OracleContractError(
            f"{name}_shape_invalid", f"{name} must have shape {(size, size)}"
        )
    if not np.isfinite(value).all():
        raise OracleContractError(f"{name}_non_finite", f"{name} must be finite")
    if not np.array_equal(value, value.T):
        raise OracleContractError(f"{name}_not_symmetric", f"{name} must be symmetric")
    return value


def _positive_definite(name: str, matrix: np.ndarray) -> None:
    try:
        np.linalg.cholesky(matrix)
    except np.linalg.LinAlgError as error:
        raise OracleContractError(
            f"{name}_not_positive_definite", f"{name} must be positive definite"
        ) from error


def exact_implied_covariance(
    lambda_matrix: np.ndarray,
    beta: np.ndarray,
    psi: np.ndarray,
    theta: np.ndarray,
) -> ExactMatrices:
    """Construct Sigma = Lambda (I-B)^-1 Psi (I-B)^-T Lambda' + Theta."""

    lambda_value = np.asarray(lambda_matrix, dtype=float)
    if lambda_value.ndim != 2 or not np.isfinite(lambda_value).all():
        raise OracleContractError(
            "lambda_shape_invalid", "lambda must be a finite two-dimensional matrix"
        )
    indicators, factors = lambda_value.shape
    beta_value = np.asarray(beta, dtype=float)
    if beta_value.shape != (factors, factors) or not np.isfinite(beta_value).all():
        raise OracleContractError(
            "beta_shape_invalid", f"beta must have shape {(factors, factors)}"
        )
    psi_value = _square("psi", psi, factors)
    theta_value = _square("theta", theta, indicators)
    _positive_definite("psi", psi_value)
    _positive_definite("theta", theta_value)
    system = np.eye(factors) - beta_value
    if abs(float(np.linalg.det(system))) <= 1e-12:
        raise OracleContractError(
            "structural_system_singular", "I - beta must be invertible"
        )
    inverse = np.linalg.inv(system)
    phi = inverse @ psi_value @ inverse.T
    sigma = lambda_value @ phi @ lambda_value.T + theta_value
    _positive_definite("sigma", sigma)
    return ExactMatrices(
        lambda_matrix=lambda_value,
        beta=beta_value,
        psi=psi_value,
        theta=theta_value,
        phi=phi,
        sigma=sigma,
    )


def two_factor_cfa(
    *, latent_covariance: float | None = None, residual_covariance: float | None = None
) -> ExactMatrices:
    """Pre-registered two-factor hand case; absent covariance means exact zero."""

    loadings = np.array(
        [
            [1.0, 0.0],
            [0.7, 0.0],
            [0.7, 0.0],
            [0.0, 1.0],
            [0.0, 0.7],
            [0.0, 0.7],
        ]
    )
    psi = np.diag([1.0, 1.0])
    if latent_covariance is not None:
        psi[0, 1] = psi[1, 0] = latent_covariance
    theta = np.diag([0.5] * 6)
    if residual_covariance is not None:
        theta[0, 1] = theta[1, 0] = residual_covariance
    return exact_implied_covariance(loadings, np.zeros((2, 2)), psi, theta)


def disturbance_covariance_sem(
    *, disturbance_covariance: float | None = None
) -> ExactMatrices:
    """One exogenous factor predicts two endogenous factors."""

    loadings = np.eye(3)
    beta = np.zeros((3, 3))
    beta[1, 0] = 0.4
    beta[2, 0] = 0.5
    psi = np.diag([1.0, 0.6, 0.7])
    if disturbance_covariance is not None:
        psi[1, 2] = psi[2, 1] = disturbance_covariance
    theta = np.diag([0.2, 0.2, 0.2])
    return exact_implied_covariance(loadings, beta, psi, theta)


def shared_open_bound_value(
    *, starts: tuple[float, ...], lower: float | None, upper: float | None
) -> float:
    """Freeze equality-start and open-bound rules independently of Rust."""

    if not starts:
        raise OracleContractError("equality_label_singleton", "starts cannot be empty")
    if not all(np.isfinite(starts)):
        raise OracleContractError("parameter_non_finite", "starts must be finite")
    reference = starts[0]
    if any(
        abs(value - reference) > 1e-12 * max(abs(value), abs(reference), 1.0)
        for value in starts[1:]
    ):
        raise OracleContractError(
            "equality_start_conflict", "equality-constrained explicit starts differ"
        )
    if lower is not None and upper is not None and lower >= upper:
        raise OracleContractError("equality_bounds_empty", "open interval is empty")
    if (lower is not None and reference <= lower) or (
        upper is not None and reference >= upper
    ):
        raise OracleContractError(
            "parameter_start_outside_bounds", "start must lie in the open interval"
        )
    return reference


STABLE_PARAMETER_IDS = {
    "construct:f=~x1": "parameter:f:x1",
    "construct:f~~construct:f": "variance:f",
    "construct:f~~construct:g": "covariance:f:g",
    "x1~~x2": "residual_covariance:x1:x2",
}

