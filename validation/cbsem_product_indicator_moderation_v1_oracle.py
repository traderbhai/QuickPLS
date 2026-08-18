"""Independent NumPy/SciPy oracle for CB-SEM product-indicator moderation v1.

This validation-only module imports no QuickPLS product code and does not run a
QuickPLS binary. It independently freezes all-pairs construction, double mean
centering, the exact unconstrained product-indicator covariance pattern, local
identification at the declared starts, and an ML estimate for one deterministic
fixture.
"""

from __future__ import annotations

import argparse
import json
import math
from dataclasses import asdict, dataclass
from typing import Literal

import numpy as np
from scipy.optimize import minimize


METHOD_VERSION = "cbsem_unconstrained_product_indicator_moderation_v1"
TRANSFORMATION_VERSION = "cbsem_product_indicator_all_pairs_transform_v1"
ORACLE_VERSION = "independent_numpy_scipy_cbsem_product_indicator_v1"
BASE_COLUMNS = ("x1", "x2", "m1", "m2", "y1", "y2", "y3")
PREDICTOR_COLUMNS = ("x1", "x2")
MODERATOR_COLUMNS = ("m1", "m2")
PRODUCT_RESIDUAL_COVARIANCE_PAIRS = ((7, 8), (7, 9), (8, 10), (9, 10))
MAX_PRODUCT_COLUMNS = 81
MAX_MATERIALIZED_PRODUCT_CELLS = 10_000_000
RAW_BYTES_PER_PRODUCT_CELL = 8
ESTIMATED_PEAK_BYTES_PER_PRODUCT_CELL = 24
PEAK_WORK_MEMORY_CEILING_BYTES = 256 * 1024 * 1024
U64_MAX = (1 << 64) - 1

Centering = Literal["none", "mean_center", "double_mean_center"]
Standardization = Literal["none", "sample_standard_deviation"]


class OracleContractError(ValueError):
    """Typed independent-oracle contract failure."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


@dataclass(frozen=True)
class ProductMaterialization:
    used_row_indices: tuple[int, ...]
    base_columns: tuple[str, ...]
    product_columns: tuple[str, ...]
    constituent_means: tuple[float, ...]
    constituent_sample_standard_deviations: tuple[float, ...]
    product_means_before_second_centering: tuple[float, ...]
    product_final_means: tuple[float, ...]
    product_final_sample_standard_deviations: tuple[float, ...]
    base_data: np.ndarray
    products: np.ndarray
    expanded_data: np.ndarray


@dataclass(frozen=True)
class StartDiagnostics:
    free_dimensions: int
    local_jacobian_rank: int
    smallest_jacobian_singular_value: float
    latent_covariance_minimum_eigenvalue: float
    residual_covariance_minimum_eigenvalue: float
    implied_covariance_minimum_eigenvalue: float
    implied_covariance_minimum_cholesky_pivot: float


@dataclass(frozen=True)
class OracleFit:
    converged: bool
    iterations: int
    objective: float
    gradient_norm: float
    predictor_effect: float
    moderator_effect: float
    interaction_effect: float
    parameter_vector: tuple[float, ...]


@dataclass(frozen=True)
class _StableColumnStatistics:
    means: np.ndarray
    sample_standard_deviations: np.ndarray
    scales: np.ndarray
    scaled_means: np.ndarray
    scaled_sample_standard_deviations: np.ndarray
    positive_variance: np.ndarray


@dataclass(frozen=True)
class ProductResourceEnvelope:
    product_count: int
    materialized_product_cells: int
    estimated_raw_bytes: int
    estimated_peak_bytes: int


def deterministic_fixture(rows: int = 180) -> np.ndarray:
    """Return a full-rank, scale-conditioned deterministic seven-column fixture."""

    if rows < 10:
        raise OracleContractError(
            "insufficient_observations", "the estimation fixture requires at least 10 rows"
        )
    output: list[list[float]] = []
    for index in range(rows):
        a = ((index * 37 + 11) % 101) / 50.0 - 1.0
        b = ((index * 53 + 17) % 103) / 51.0 - 1.0
        c = ((index * 29 + 7) % 97) / 48.0 - 1.0
        errors = (
            ((index * 19 + 3) % 89) / 44.0 - 1.0,
            ((index * 23 + 5) % 83) / 41.0 - 1.0,
            ((index * 31 + 9) % 79) / 39.0 - 1.0,
            ((index * 17 + 13) % 73) / 36.0 - 1.0,
            ((index * 27 + 15) % 71) / 35.0 - 1.0,
            ((index * 11 + 19) % 67) / 33.0 - 1.0,
            ((index * 7 + 21) % 61) / 30.0 - 1.0,
        )
        predictor = 1.5 * a + 0.18 * b
        moderator = 1.5 * b + 0.12 * a
        outcome = (
            0.45 * predictor
            + 0.30 * moderator
            + 0.35 * predictor * moderator
            + 0.90 * c
        )
        output.append(
            [
                predictor + 1.1 * errors[0],
                0.82 * predictor + 1.1 * errors[1],
                moderator + 1.1 * errors[2],
                0.86 * moderator - 1.1 * errors[3],
                outcome + 1.1 * errors[4],
                0.83 * outcome + 1.1 * errors[5],
                0.68 * outcome - 1.1 * errors[6],
            ]
        )
    return np.asarray(output, dtype=float)


def _stable_column_statistics(values: np.ndarray) -> _StableColumnStatistics:
    """Compute scale-aware column moments without an absolute epsilon cutoff."""

    matrix = np.asarray(values, dtype=float)
    scales = np.max(np.abs(matrix), axis=0)
    safe_scales = np.where(scales == 0.0, 1.0, scales)
    scaled = matrix / safe_scales
    scaled_means = np.mean(scaled, axis=0)
    scaled_deviations = scaled - scaled_means
    scaled_sum_squares = np.sum(scaled_deviations * scaled_deviations, axis=0)
    positive_variance = scaled_sum_squares > 0.0
    scaled_sample_standard_deviations = np.sqrt(
        scaled_sum_squares / (matrix.shape[0] - 1)
    )
    means = scaled_means * scales
    sample_standard_deviations = scaled_sample_standard_deviations * scales
    return _StableColumnStatistics(
        means=means,
        sample_standard_deviations=sample_standard_deviations,
        scales=safe_scales,
        scaled_means=scaled_means,
        scaled_sample_standard_deviations=scaled_sample_standard_deviations,
        positive_variance=positive_variance,
    )


def validate_resource_envelope(
    complete_rows: int, predictor_count: int, moderator_count: int
) -> ProductResourceEnvelope:
    """Mirror the checked Internal v1 allocation envelope without allocating."""

    if min(complete_rows, predictor_count, moderator_count) < 0:
        raise OracleContractError("resource_count_invalid", "resource counts cannot be negative")
    product_count = predictor_count * moderator_count
    if product_count > U64_MAX:
        raise OracleContractError(
            "resource_size_overflow", "the Cartesian product count exceeds u64"
        )
    if product_count > MAX_PRODUCT_COLUMNS:
        raise OracleContractError(
            "product_column_limit_exceeded",
            f"{product_count} product columns exceed the Internal v1 maximum of {MAX_PRODUCT_COLUMNS}",
        )
    materialized_product_cells = complete_rows * product_count
    estimated_raw_bytes = materialized_product_cells * RAW_BYTES_PER_PRODUCT_CELL
    estimated_peak_bytes = (
        materialized_product_cells * ESTIMATED_PEAK_BYTES_PER_PRODUCT_CELL
    )
    if max(
        materialized_product_cells, estimated_raw_bytes, estimated_peak_bytes
    ) > U64_MAX:
        raise OracleContractError(
            "resource_size_overflow", "the materialization byte estimate exceeds u64"
        )
    if (
        materialized_product_cells > MAX_MATERIALIZED_PRODUCT_CELLS
        or estimated_peak_bytes > PEAK_WORK_MEMORY_CEILING_BYTES
    ):
        raise OracleContractError(
            "materialization_limit_exceeded",
            "product-indicator materialization exceeds the Internal v1 cell or peak-memory ceiling",
        )
    return ProductResourceEnvelope(
        product_count=product_count,
        materialized_product_cells=materialized_product_cells,
        estimated_raw_bytes=estimated_raw_bytes,
        estimated_peak_bytes=estimated_peak_bytes,
    )


def materialize_product_indicators(
    data: np.ndarray,
    columns: tuple[str, ...],
    predictor_columns: tuple[str, ...],
    moderator_columns: tuple[str, ...],
    *,
    centering: Centering = "double_mean_center",
    standardization: Standardization = "none",
    validate_estimator_scope: bool = True,
) -> ProductMaterialization:
    """Construct canonical all-pairs products after listwise deletion."""

    values = np.asarray(data, dtype=float)
    if values.ndim != 2 or values.shape[1] != len(columns):
        raise OracleContractError(
            "data_shape_invalid", "data must be a two-dimensional matrix matching columns"
        )
    if len(set(columns)) != len(columns):
        raise OracleContractError("duplicate_column", "column names must be unique")
    if centering not in ("none", "mean_center", "double_mean_center"):
        raise OracleContractError("centering_invalid", f"unsupported centering: {centering}")
    if standardization not in ("none", "sample_standard_deviation"):
        raise OracleContractError(
            "standardization_invalid",
            f"unsupported standardization: {standardization}",
        )
    predictor = tuple(sorted(predictor_columns))
    moderator = tuple(sorted(moderator_columns))
    minimum_block_size = 2 if validate_estimator_scope else 1
    if (
        len(predictor) < minimum_block_size
        or len(moderator) < minimum_block_size
        or set(predictor) & set(moderator)
    ):
        raise OracleContractError(
            "indicator_blocks_invalid",
            "predictor and moderator blocks must meet the requested size and be disjoint",
        )
    validate_resource_envelope(0, len(predictor), len(moderator))
    positions = {name: index for index, name in enumerate(columns)}
    required = predictor + moderator
    missing = [name for name in required if name not in positions]
    if missing:
        raise OracleContractError(
            "source_column_missing", f"required columns are missing: {missing}"
        )
    if np.isinf(values).any():
        raise OracleContractError("source_value_non_finite", "infinite values are not missing")
    complete_mask = ~np.isnan(values).any(axis=1)
    complete = values[complete_mask]
    minimum_observations = 10 if validate_estimator_scope else 3
    if complete.shape[0] < minimum_observations:
        raise OracleContractError(
            "insufficient_complete_observations",
            f"at least {minimum_observations} complete rows are required",
        )
    validate_resource_envelope(complete.shape[0], len(predictor), len(moderator))
    constituent_names = predictor + moderator
    constituent = complete[:, [positions[name] for name in constituent_names]]
    statistics = _stable_column_statistics(constituent)
    means = statistics.means
    sample_standard_deviations = statistics.sample_standard_deviations
    if (
        not np.isfinite(means).all()
        or not np.isfinite(sample_standard_deviations).all()
        or np.any(statistics.positive_variance & (sample_standard_deviations == 0.0))
    ):
        raise OracleContractError(
            "constituent_statistic_non_finite",
            "constituent means and sample standard deviations must be finite",
        )
    if not statistics.positive_variance.all():
        raise OracleContractError(
            "constituent_zero_variance", "constituent sample variances must be positive"
        )
    if standardization == "sample_standard_deviation":
        transformed = (
            constituent / statistics.scales - statistics.scaled_means
        ) / statistics.scaled_sample_standard_deviations
    elif centering in ("mean_center", "double_mean_center"):
        with np.errstate(over="ignore", invalid="ignore"):
            transformed = constituent - means
    else:
        transformed = constituent.copy()
    if not np.isfinite(transformed).all():
        raise OracleContractError(
            "constituent_transformation_non_finite",
            "constituent centering or standardization produced a non-finite value",
        )

    products: list[np.ndarray] = []
    product_names: list[str] = []
    pre_means: list[float] = []
    predictor_count = len(predictor)
    for predictor_index, predictor_name in enumerate(predictor):
        for moderator_index, moderator_name in enumerate(moderator):
            with np.errstate(over="ignore", invalid="ignore"):
                product = (
                    transformed[:, predictor_index]
                    * transformed[:, predictor_count + moderator_index]
                )
            if not np.isfinite(product).all():
                raise OracleContractError(
                    "product_value_non_finite",
                    f"product {predictor_name} by {moderator_name} overflowed or became non-finite",
                )
            product_statistics = _stable_column_statistics(product[:, np.newaxis])
            pre_mean = float(product_statistics.means[0])
            if not math.isfinite(pre_mean):
                raise OracleContractError(
                    "product_statistic_non_finite", "a product mean became non-finite"
                )
            if centering == "double_mean_center":
                with np.errstate(over="ignore", invalid="ignore"):
                    product = product - pre_mean
                if not np.isfinite(product).all():
                    raise OracleContractError(
                        "product_second_centering_non_finite",
                        "double mean centering produced a non-finite product value",
                    )
            products.append(product)
            product_names.append(f"product:{predictor_name}:{moderator_name}")
            pre_means.append(pre_mean)
    product_matrix = np.column_stack(products)
    final_statistics = _stable_column_statistics(product_matrix)
    final_means = final_statistics.means
    final_standard_deviations = final_statistics.sample_standard_deviations
    if (
        not np.isfinite(final_means).all()
        or not np.isfinite(final_standard_deviations).all()
        or np.any(
            final_statistics.positive_variance & (final_standard_deviations == 0.0)
        )
    ):
        raise OracleContractError(
            "product_statistic_non_finite",
            "product means and sample standard deviations must be finite",
        )
    if not final_statistics.positive_variance.all():
        raise OracleContractError(
            "product_zero_variance", "product sample variances must be positive"
        )
    return ProductMaterialization(
        used_row_indices=tuple(np.flatnonzero(complete_mask).tolist()),
        base_columns=columns,
        product_columns=tuple(product_names),
        constituent_means=tuple(float(value) for value in means),
        constituent_sample_standard_deviations=tuple(
            float(value) for value in sample_standard_deviations
        ),
        product_means_before_second_centering=tuple(pre_means),
        product_final_means=tuple(float(value) for value in final_means),
        product_final_sample_standard_deviations=tuple(
            float(value) for value in final_standard_deviations
        ),
        base_data=complete,
        products=product_matrix,
        expanded_data=np.column_stack((complete, product_matrix)),
    )


def declared_start() -> np.ndarray:
    """Freeze the 32 free dimensions and their generic parameter-table starts."""

    start = np.zeros(32, dtype=float)
    # x2, m2, y2, y3 and the three non-marker product loadings.
    start[0:7] = (0.7, 0.7, 0.7, 0.7, 1.0, 1.0, 1.0)
    # X -> Y, M -> Y, X*M -> Y.
    start[7:10] = (0.0, 0.0, 0.3)
    # Log variances for X, M, X*M, and the Y disturbance.
    start[10:14] = 0.0
    # X-M covariance; interaction-main covariances start at zero.
    start[14:17] = (0.1, 0.0, 0.0)
    # Eleven log residual variances; four residual covariances start at zero.
    start[17:28] = math.log(0.5)
    return start


def implied_matrices(
    raw: np.ndarray,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Return Sigma, Psi, and Theta for the exact 4-factor parameter table."""

    parameters = np.asarray(raw, dtype=float)
    if parameters.shape != (32,) or not np.isfinite(parameters).all():
        raise OracleContractError(
            "parameter_vector_invalid", "the raw parameter vector must contain 32 finite values"
        )
    loadings = np.zeros((11, 4), dtype=float)
    loadings[0, 0] = loadings[2, 1] = loadings[4, 3] = loadings[7, 2] = 1.0
    loadings[1, 0] = parameters[0]
    loadings[3, 1] = parameters[1]
    loadings[5, 3] = parameters[2]
    loadings[6, 3] = parameters[3]
    loadings[8, 2] = parameters[4]
    loadings[9, 2] = parameters[5]
    loadings[10, 2] = parameters[6]
    beta = np.zeros((4, 4), dtype=float)
    beta[3, :3] = parameters[7:10]
    psi = np.diag(np.exp(parameters[10:14]))
    psi[0, 1] = psi[1, 0] = parameters[14]
    psi[0, 2] = psi[2, 0] = parameters[15]
    psi[1, 2] = psi[2, 1] = parameters[16]
    residual_variances = np.exp(parameters[17:28])
    theta = np.diag(residual_variances)
    for covariance, (left, right) in zip(
        parameters[28:32], PRODUCT_RESIDUAL_COVARIANCE_PAIRS, strict=True
    ):
        theta[left, right] = theta[right, left] = covariance
    try:
        np.linalg.cholesky(psi)
        np.linalg.cholesky(theta)
        inverse = np.linalg.inv(np.eye(4) - beta)
        factor_covariance = inverse @ psi @ inverse.T
        sigma = loadings @ factor_covariance @ loadings.T + theta
        np.linalg.cholesky(sigma)
    except np.linalg.LinAlgError as error:
        raise OracleContractError(
            "implied_covariance_not_positive_definite",
            "Psi, Theta, and implied Sigma must be positive definite",
        ) from error
    return sigma, psi, theta


def start_diagnostics() -> StartDiagnostics:
    start = declared_start()
    sigma, psi, theta = implied_matrices(start)
    lower = np.tril_indices(sigma.shape[0])
    jacobian = np.empty((len(lower[0]), len(start)), dtype=float)
    for parameter in range(len(start)):
        step = 1e-6 * max(abs(start[parameter]), 1.0)
        plus = start.copy()
        minus = start.copy()
        plus[parameter] += step
        minus[parameter] -= step
        jacobian[:, parameter] = (
            implied_matrices(plus)[0][lower] - implied_matrices(minus)[0][lower]
        ) / (2.0 * step)
    singular_values = np.linalg.svd(jacobian, compute_uv=False)
    return StartDiagnostics(
        free_dimensions=len(start),
        local_jacobian_rank=int(np.linalg.matrix_rank(jacobian)),
        smallest_jacobian_singular_value=float(singular_values[-1]),
        latent_covariance_minimum_eigenvalue=float(np.linalg.eigvalsh(psi)[0]),
        residual_covariance_minimum_eigenvalue=float(np.linalg.eigvalsh(theta)[0]),
        implied_covariance_minimum_eigenvalue=float(np.linalg.eigvalsh(sigma)[0]),
        implied_covariance_minimum_cholesky_pivot=float(
            np.min(np.diag(np.linalg.cholesky(sigma)) ** 2)
        ),
    )


def fit_oracle(materialization: ProductMaterialization) -> OracleFit:
    """Independently fit the frozen covariance model with SciPy BFGS."""

    data = materialization.expanded_data
    sample_covariance = np.cov(data, rowvar=False, ddof=0)
    sample_sign, sample_log_determinant = np.linalg.slogdet(sample_covariance)
    if sample_sign <= 0.0:
        raise OracleContractError(
            "sample_covariance_not_positive_definite",
            "expanded ML covariance must be positive definite",
        )

    def objective(raw: np.ndarray) -> float:
        try:
            sigma, _, _ = implied_matrices(raw)
            sign, log_determinant = np.linalg.slogdet(sigma)
            if sign <= 0.0:
                raise np.linalg.LinAlgError
            value = (
                log_determinant
                + np.trace(np.linalg.solve(sigma, sample_covariance))
                - sample_log_determinant
                - sample_covariance.shape[0]
            )
            return float(max(value, 0.0))
        except (OracleContractError, np.linalg.LinAlgError, OverflowError):
            return float(1e6 + np.dot(raw, raw))

    optimized = minimize(
        objective,
        declared_start(),
        method="BFGS",
        jac="3-point",
        options={"maxiter": 1_000, "gtol": 1e-6},
    )
    gradient = np.asarray(optimized.jac, dtype=float)
    if not optimized.success or not np.isfinite(optimized.fun) or not np.isfinite(gradient).all():
        raise OracleContractError(
            "oracle_nonconvergence", f"SciPy BFGS did not converge: {optimized.message}"
        )
    return OracleFit(
        converged=True,
        iterations=int(optimized.nit),
        objective=float(optimized.fun),
        gradient_norm=float(np.linalg.norm(gradient)),
        predictor_effect=float(optimized.x[7]),
        moderator_effect=float(optimized.x[8]),
        interaction_effect=float(optimized.x[9]),
        parameter_vector=tuple(float(value) for value in optimized.x),
    )


def build_report() -> dict[str, object]:
    materialized = materialize_product_indicators(
        deterministic_fixture(),
        BASE_COLUMNS,
        PREDICTOR_COLUMNS,
        MODERATOR_COLUMNS,
    )
    sample_covariance = np.cov(materialized.expanded_data, rowvar=False, ddof=1)
    sample_eigenvalues = np.linalg.eigvalsh(sample_covariance)
    fit = fit_oracle(materialized)
    return {
        "oracle_version": ORACLE_VERSION,
        "method_version": METHOD_VERSION,
        "transformation_version": TRANSFORMATION_VERSION,
        "observations": len(materialized.used_row_indices),
        "product_columns": list(materialized.product_columns),
        "maximum_absolute_product_final_mean": max(
            abs(value) for value in materialized.product_final_means
        ),
        "sample_covariance_minimum_eigenvalue": float(sample_eigenvalues[0]),
        "sample_covariance_condition_number": float(sample_eigenvalues[-1] / sample_eigenvalues[0]),
        "sample_covariance_minimum_cholesky_pivot": float(
            np.min(np.diag(np.linalg.cholesky(sample_covariance)) ** 2)
        ),
        "start_diagnostics": asdict(start_diagnostics()),
        "fit": asdict(fit) | {"parameter_vector": list(fit.parameter_vector)},
    }


def check_report(report: dict[str, object]) -> None:
    assert report["oracle_version"] == ORACLE_VERSION
    assert report["method_version"] == METHOD_VERSION
    assert report["transformation_version"] == TRANSFORMATION_VERSION
    assert report["observations"] == 180
    assert report["product_columns"] == [
        "product:x1:m1",
        "product:x1:m2",
        "product:x2:m1",
        "product:x2:m2",
    ]
    assert float(report["maximum_absolute_product_final_mean"]) <= 1e-14
    assert math.isclose(
        float(report["sample_covariance_minimum_eigenvalue"]),
        0.15162361215131093,
        rel_tol=0.0,
        abs_tol=1e-12,
    )
    diagnostics = report["start_diagnostics"]
    assert isinstance(diagnostics, dict)
    assert diagnostics["free_dimensions"] == 32
    assert diagnostics["local_jacobian_rank"] == 32
    assert float(diagnostics["smallest_jacobian_singular_value"]) > 0.02
    fit = report["fit"]
    assert isinstance(fit, dict)
    assert fit["converged"] is True
    assert float(fit["gradient_norm"]) < 1e-5
    assert math.isclose(float(fit["objective"]), 0.14380285287765204, abs_tol=2e-9)
    assert math.isclose(float(fit["predictor_effect"]), 0.46387588, abs_tol=2e-6)
    assert math.isclose(float(fit["moderator_effect"]), 0.22467318, abs_tol=2e-6)
    assert math.isclose(float(fit["interaction_effect"]), 0.23008951, abs_tol=2e-6)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail if the frozen oracle drifts")
    args = parser.parse_args()
    report = build_report()
    if args.check:
        check_report(report)
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
