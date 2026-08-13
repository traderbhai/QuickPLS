#!/usr/bin/env python3
"""Independent numerical qualification for graph-defined PROCESS v2.

This generator never builds QuickPLS.  It executes only an already-built
release CLI, then recomputes the frozen graph with NumPy/SciPy and invokes the
validation-only R companion.  The reference intentionally does not share the
Rust graph builder, term builder, resampling stream, or Johnson-Neyman code.
"""

from __future__ import annotations

import copy
import csv
import hashlib
import json
import math
import os
import random
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Sequence

import numpy as np
from scipy.stats import norm, t as student_t

from r_runtime import find_rscript_optional


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
FIXTURE = RESULTS / "process_v2_reference_fixture.csv"
REPORT = RESULTS / "process_v2_reference_report.json"
R_SCRIPT = ROOT / "validation" / "process_v2_reference.R"
R_REPORT = RESULTS / "process_v2_r_reference.json"

FEATURE_ID = "qpls3.standalone.process"
METHOD_VERSION = "regression_process_v2"
BOOTSTRAP_METHOD_VERSION = "regression_process_bootstrap_v1"
WITNESS_VERSION = "regression_process_bootstrap_validation_witness_v1"
CATALOGUE_SNAPSHOT_DATE = "2026-08-12"
TARGET = "process_v2_independent_reference"
CONFIDENCE_LEVEL = 0.95
QUICKPLS_REPLICATES = 1_000
INDEPENDENT_REPLICATES = 2_000
SEED = 20_260_812
PYTHON_SEED = 20_260_813
WORKERS = (1, 4)
EXACT_TOLERANCE = 5e-9
ARITHMETIC_TOLERANCE = 5e-11
HC3_LEVERAGE_STABILITY_TOLERANCE = 1e-12
PROCESS_RELATIVE_RANK_TOLERANCE_MULTIPLIER = 100.0
PROCESS_JN_COEFFICIENT_TOLERANCE_MULTIPLIER = 64.0
PROCESS_JN_ROOT_DEDUP_TOLERANCE_MULTIPLIER = 128.0
HIGH_LEVERAGE_HC3_REASON = "high_leverage_hc3_instability"
INVALID_HC3_COVARIANCE_REASON = "invalid_hc3_covariance"
DEGENERATE_SIMPLE_SLOPE_REASON = "degenerate_simple_slope_variance"
BINARY_ENDOGENOUS_OUTCOME_REASON = "binary_process_equation_outcome"
COLLAPSED_PROBE_GRID_REASON = "collapsed_process_probe_grid"
JN_INVALID_COVARIANCE_MESSAGE = (
    "Johnson-Neyman conditional-effect variance must be finite and strictly positive "
    "across the tested moderator range."
)
REFERENCE_CONDITION = (
    "Continuous moderators are evaluated at their original complete-sample raw means "
    "(coded 0); binary moderators are evaluated at 0."
)

PREDICTORS = ["X", "M1", "M2", "M3", "M4", "W", "B"]
CONTROLS = ["C"]
OUTCOME = "Y"
PATHS = [
    ("X", "Y"),
    ("X", "M1"),
    ("M1", "M2"),
    ("M2", "Y"),
    ("X", "M3"),
    ("M3", "Y"),
    ("X", "M4"),
    ("M4", "Y"),
]
MODERATORS = [("W", "continuous"), ("B", "binary_0_1")]
MODERATIONS = [
    ("X", "Y", "W", "B"),
    ("X", "M3", "W", None),
    ("M4", "Y", "B", None),
]

REFERENCE_CHECK_NAMES = frozenset(
    {
        "typed_recipe_and_method_identity",
        "global_listwise_sample_and_profiles",
        "equation_order_terms_hc3_fit",
        "hc3_high_leverage_fail_closed",
        "hc3_covariance_diagonal_fail_closed",
        "simple_slope_variance_fail_closed",
        "scale_aware_solver_affine_unit_invariance_and_relative_collinearity",
        "johnson_neyman_root_solver_affine_stability",
        "johnson_neyman_invalid_covariance_fail_closed",
        "binary_endogenous_outcome_rejected_in_original_sample",
        "semantic_probe_grid_rejects_collapsed_f64_levels",
        "reference_effect_condition_policy",
        "point_metamorphic_invariance",
        "parallel_serial_effect_arithmetic",
        "continuous_binary_three_way_slopes",
        "first_second_stage_moderated_mediation",
        "johnson_neyman_roots_regions_curves",
        "conditional_plot_arithmetic",
        "bootstrap_witness_type7_bca_ratio",
        "bootstrap_index_partitions",
        "independent_python_resampling_distribution",
        "independent_r_equations_effects_resampling",
        "seed_worker_invariance",
        "dedicated_graph_result_not_generic_regression",
        "legacy_v1_not_current_evidence",
    }
)

COMPARISON_THRESHOLDS = {
    "mean_pooled_se_units": 0.40,
    "standard_error_relative": 0.35,
    "percentile_endpoint_pooled_se_units": 0.85,
}


def configured_cli() -> Path:
    configured = os.environ.get("QUICKPLS_CLI_PATH", "").strip()
    if configured:
        return Path(configured).resolve()
    return (ROOT / "target" / "release" / "qpls.exe").resolve()


CLI = configured_cli()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def artifact(path: Path) -> dict[str, Any]:
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "size": path.stat().st_size,
        "sha256": sha256_file(path),
    }


def require_release_cli() -> None:
    expected = (ROOT / "target" / "release" / "qpls.exe").resolve()
    if not CLI.is_file() or CLI != expected:
        raise SystemExit(
            f"PROCESS v2 qualification requires the coordinated release CLI {expected}; "
            f"received {CLI}. This script never invokes Cargo."
        )


def run(command: Sequence[str], *, timeout: int = 1_200) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        list(command), cwd=ROOT, capture_output=True, text=True, timeout=timeout, check=False
    )


def require_success(completed: subprocess.CompletedProcess[str], label: str) -> None:
    if completed.returncode:
        raise RuntimeError(
            f"{label} failed ({completed.returncode})\n"
            f"stdout={completed.stdout[-3000:]}\nstderr={completed.stderr[-3000:]}"
        )


def write_fixture(path: Path = FIXTURE) -> None:
    """Write a deterministic signal fixture with shared-sample missing rows."""

    rng = random.Random(20_260_812)
    rows: list[dict[str, float | None]] = []
    for index in range(180):
        x = 1.25 + rng.gauss(0.0, 1.0)
        w = -0.55 + rng.gauss(0.0, 0.85)
        b = float((index * 7 + 3) % 11 >= 5)
        c = 0.35 + rng.gauss(0.0, 0.75)
        m1 = 0.55 * x + 0.18 * c + rng.gauss(0.0, 0.52)
        m2 = 0.62 * m1 + 0.12 * c + rng.gauss(0.0, 0.48)
        m3 = (
            0.45 * x
            + 0.28 * w
            + 0.32 * (x - 1.25) * (w + 0.55)
            + 0.15 * c
            + rng.gauss(0.0, 0.50)
        )
        m4 = 0.50 * x + 0.12 * c + rng.gauss(0.0, 0.50)
        y = (
            0.25 * x
            + 0.42 * m2
            + 0.36 * m3
            + 0.33 * m4
            + 0.16 * w
            + 0.11 * b
            + 0.21 * (x - 1.25) * (w + 0.55)
            + 0.13 * (x - 1.25) * b
            + 0.10 * (w + 0.55) * b
            + 0.18 * (x - 1.25) * (w + 0.55) * b
            + 0.24 * m4 * b
            + 0.14 * c
            + rng.gauss(0.0, 0.58)
        )
        row: dict[str, float | None] = {
            "X": x, "M1": m1, "M2": m2, "M3": m3, "M4": m4,
            "W": w, "B": b, "C": c, "Y": y,
        }
        # Missingness is deliberately spread across graph roles. Every equation
        # must still use one global listwise-complete sampling frame.
        if index in {17, 83}:
            row["M2"] = None
        if index in {41, 139}:
            row["W"] = None
        if index == 127:
            row["C"] = None
        rows.append(row)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=[*PREDICTORS, *CONTROLS, OUTCOME])
        writer.writeheader()
        for row in rows:
            writer.writerow(
                {
                    key: "" if value is None else f"{value:.12f}"
                    for key, value in row.items()
                }
            )


def empty_model() -> dict[str, Any]:
    return {
        "id": "00000000-0000-0000-0000-000000032001",
        "name": "PROCESS v2 reference",
        "constructs": [], "paths": [], "controls": [],
        "higher_order_constructs": [], "interactions": [],
    }


def recipe_payload(fingerprint: str, *, workers: int, samples: int) -> dict[str, Any]:
    return {
        "schema_version": 3,
        "id": f"00000000-0000-0000-0000-{3200 + workers:012d}",
        "created_at": "2026-08-12T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": empty_model(),
        "settings": {
            "method": "regression", "weighting_scheme": "path",
            "tolerance": 1e-7, "max_iterations": 3000,
            "bootstrap_samples": samples, "studentized_inner_samples": 0,
            "permutation_samples": 0, "seed": SEED, "workers": workers,
            "confidence_level": CONFIDENCE_LEVEL,
            "preprocessing": "unstandardized",
            "missing_data": "listwise_deletion", "case_weight_column": None,
        },
        "method_config": {
            "kind": "regression", "outcome": OUTCOME,
            "predictors": PREDICTORS, "controls": CONTROLS,
            "model": {
                "type": "process",
                "relationship": {
                    "model": "graph", "focal_predictor": "X",
                    "paths": [{"from": source, "to": target} for source, target in PATHS],
                    "moderators": [
                        {"variable": variable, "scale": scale}
                        for variable, scale in MODERATORS
                    ],
                    "moderations": [
                        {
                            "from": source, "to": target, "moderator": moderator,
                            **({"conditioning_moderator": conditioning} if conditioning else {}),
                        }
                        for source, target, moderator, conditioning in MODERATIONS
                    ],
                    "continuous_product_centering": "equation_complete_case_mean_v1",
                },
            },
            "bootstrap": {
                "algorithm": "case_resampling", "intervals": ["percentile", "bca"]
            } if samples else None,
        },
        "metadata": {
            "status": (
                "candidate_regression_process_v2_plus_bootstrap_v1_bounded_scope"
                if samples else "candidate_regression_process_v2_bounded_scope"
            ),
            "fixture": "process_v2_reference",
        },
    }


def dataset_fingerprint(temporary: Path) -> str:
    project = temporary / "process-v2-fingerprint.qpls"
    completed = run([str(CLI), "import", str(FIXTURE), str(project), "--name", "PROCESS v2 reference"])
    require_success(completed, "fixture import")
    inspected = run([str(CLI), "inspect", str(project), "--json"])
    require_success(inspected, "fixture inspect")
    return json.loads(inspected.stdout)["datasets"][0]["fingerprint"]


def run_quickpls(
    temporary: Path, fingerprint: str, *, workers: int, samples: int
) -> tuple[dict[str, Any], dict[str, Any]]:
    recipe = recipe_payload(fingerprint, workers=workers, samples=samples)
    recipe_path = temporary / f"process-v2-w{workers}.recipe.json"
    result_path = temporary / f"process-v2-w{workers}.result.json"
    recipe_path.write_text(json.dumps(recipe, indent=2), encoding="utf-8")
    completed = run(
        [str(CLI), "run", str(recipe_path), "--data", str(FIXTURE), "--output", str(result_path)]
    )
    require_success(completed, f"PROCESS v2 workers={workers}")
    return recipe, json.loads(result_path.read_text(encoding="utf-8"))


def complete_case_columns(path: Path = FIXTURE) -> tuple[dict[str, np.ndarray], int]:
    variables = [*PREDICTORS, *CONTROLS, OUTCOME]
    complete: list[list[float]] = []
    total = 0
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            total += 1
            raw = [row.get(variable, "") for variable in variables]
            if any(value is None or not value.strip() for value in raw):
                continue
            values = [float(value) for value in raw]
            if all(math.isfinite(value) for value in values):
                complete.append(values)
    matrix = np.asarray(complete, dtype=float)
    return {variable: matrix[:, index] for index, variable in enumerate(variables)}, total


@dataclass(frozen=True)
class EquationSpec:
    outcome: str
    terms: tuple[tuple[str, str, tuple[str, ...]], ...]


class ProcessReferenceFitError(ValueError):
    """Independent fit failure carrying the frozen PROCESS reason code."""

    def __init__(self, reason_code: str, message: str) -> None:
        super().__init__(f"{reason_code}|{message}")
        self.reason_code = reason_code


def hc3_scaled_residuals(
    residual: np.ndarray,
    leverage: np.ndarray,
    *,
    equation_id: str,
) -> np.ndarray:
    """Apply HC3 without a leverage clamp and reject unstable equations."""

    denominator = 1.0 - np.asarray(leverage, dtype=float)
    unstable = np.flatnonzero(
        ~np.isfinite(denominator) | (denominator <= HC3_LEVERAGE_STABILITY_TOLERANCE)
    )
    if unstable.size:
        case_index = int(unstable[0])
        raise ProcessReferenceFitError(
            HIGH_LEVERAGE_HC3_REASON,
            f"equation={equation_id}|case_index={case_index}|one_minus_h={denominator[case_index]:.17g}",
        )
    return np.asarray(residual, dtype=float) / denominator


def hc3_high_leverage_boundary_check() -> bool:
    """Freeze the <= 1e-12 rejection boundary independently of the fixture."""

    stable = hc3_scaled_residuals(
        np.asarray([2.0]),
        np.asarray([1.0 - (HC3_LEVERAGE_STABILITY_TOLERANCE * 2.0)]),
        equation_id="equation:stable",
    )
    if not np.allclose(stable, [1.0e12], rtol=5e-5, atol=0.0):
        return False
    for leverage in (1.0 - HC3_LEVERAGE_STABILITY_TOLERANCE, 1.0, math.nan, math.inf):
        try:
            hc3_scaled_residuals(
                np.asarray([1.0]),
                np.asarray([leverage]),
                equation_id="equation:unstable",
            )
        except ProcessReferenceFitError as error:
            if error.reason_code != HIGH_LEVERAGE_HC3_REASON or not str(error).startswith(
                f"{HIGH_LEVERAGE_HC3_REASON}|"
            ):
                return False
        else:
            return False
    return True


def validate_hc3_covariance_diagonal(
    covariance: np.ndarray,
    *,
    equation_id: str,
) -> np.ndarray:
    """Reject nonfinite or nonpositive HC3 coefficient variances.

    PROCESS v2 never applies ``abs`` or ``max(0, variance)`` to make an
    invalid robust covariance appear usable.
    """

    matrix = np.asarray(covariance, dtype=float)
    if matrix.ndim != 2 or matrix.shape[0] != matrix.shape[1] or not np.all(np.isfinite(matrix)):
        raise ProcessReferenceFitError(
            INVALID_HC3_COVARIANCE_REASON,
            f"equation={equation_id}|reason=nonfinite_or_nonsquare_covariance",
        )
    diagonal = np.diag(matrix)
    invalid = np.flatnonzero(~np.isfinite(diagonal) | (diagonal <= 0.0))
    if invalid.size:
        index = int(invalid[0])
        raise ProcessReferenceFitError(
            INVALID_HC3_COVARIANCE_REASON,
            f"equation={equation_id}|term_index={index}|variance={diagonal[index]:.17g}",
        )
    return diagonal


def hc3_covariance_diagonal_boundary_check() -> bool:
    valid = validate_hc3_covariance_diagonal(
        np.asarray([[1.0, 0.25], [0.25, 2.0]]), equation_id="equation:valid"
    )
    if not np.array_equal(valid, np.asarray([1.0, 2.0])):
        return False
    for invalid in (0.0, -1e-12, math.nan, math.inf):
        try:
            validate_hc3_covariance_diagonal(
                np.asarray([[invalid]]), equation_id="equation:invalid"
            )
        except ProcessReferenceFitError as error:
            if error.reason_code != INVALID_HC3_COVARIANCE_REASON or not str(error).startswith(
                f"{INVALID_HC3_COVARIANCE_REASON}|"
            ):
                return False
        else:
            return False
    return True


def simple_slope_variance_boundary_check() -> bool:
    equation = {
        "coefficients": [{"estimate": 1.0}, {"estimate": 2.0}],
        "coefficient_covariance": [[1.0, 0.0], [0.0, 0.0]],
    }
    estimate, standard_error = linear_combination(
        equation, np.asarray([1.0, 0.0]), moderation_id="moderation:valid"
    )
    if estimate != 1.0 or standard_error != 1.0:
        return False
    for covariance in ([[0.0]], [[-1e-12]], [[math.nan]], [[math.inf]]):
        try:
            linear_combination(
                {"coefficients": [{"estimate": 1.0}], "coefficient_covariance": covariance},
                np.asarray([1.0]),
                moderation_id="moderation:invalid",
            )
        except ProcessReferenceFitError as error:
            if error.reason_code != DEGENERATE_SIMPLE_SLOPE_REASON or not str(error).startswith(
                f"{DEGENERATE_SIMPLE_SLOPE_REASON}|"
            ):
                return False
        else:
            return False
    return True


def column_location_scale(values: np.ndarray) -> tuple[float, float] | None:
    """Mirror the production Welford location/population-RMS normalization."""

    mean = 0.0
    centered_sum_squares = 0.0
    for index, raw in enumerate(np.asarray(values, dtype=float)):
        value = float(raw)
        if not math.isfinite(value):
            return None
        count = float(index + 1)
        delta = value - mean
        mean += delta / count
        centered_sum_squares += delta * (value - mean)
    if len(values) == 0:
        return None
    scale = math.sqrt(centered_sum_squares / len(values))
    return (mean, scale) if math.isfinite(mean) and math.isfinite(scale) and scale > 0.0 else None


def scale_aware_ols(
    design: np.ndarray,
    outcome: np.ndarray,
    *,
    equation_id: str,
) -> dict[str, np.ndarray | float]:
    """Solve a PROCESS equation through the frozen normalized thin-SVD seam."""

    matrix = np.asarray(design, dtype=float)
    values = np.asarray(outcome, dtype=float)
    if (
        matrix.ndim != 2
        or values.ndim != 1
        or matrix.shape[0] != values.shape[0]
        or matrix.shape[0] <= matrix.shape[1]
        or matrix.shape[1] == 0
        or not np.all(np.isfinite(matrix))
        or not np.all(np.isfinite(values))
        or not np.array_equal(matrix[:, 0], np.ones(matrix.shape[0]))
    ):
        raise ProcessReferenceFitError("rank_deficient", f"equation={equation_id}")
    rows, columns = matrix.shape
    centers = np.zeros(columns, dtype=float)
    scales = np.ones(columns, dtype=float)
    for column in range(1, columns):
        location_scale = column_location_scale(matrix[:, column])
        if location_scale is None:
            raise ProcessReferenceFitError("rank_deficient", f"equation={equation_id}")
        centers[column], scales[column] = location_scale
    normalized = matrix.copy()
    normalized[:, 1:] = (normalized[:, 1:] - centers[1:]) / scales[1:]
    left, singular_values, right_transpose = np.linalg.svd(normalized, full_matrices=False)
    maximum = float(singular_values[0])
    minimum = float(singular_values[-1])
    tolerance = (
        maximum
        * max(rows, columns)
        * sys.float_info.epsilon
        * PROCESS_RELATIVE_RANK_TOLERANCE_MULTIPLIER
    )
    if (
        not math.isfinite(maximum)
        or not math.isfinite(minimum)
        or maximum <= 0.0
        or minimum <= tolerance
    ):
        raise ProcessReferenceFitError("rank_deficient", f"equation={equation_id}")
    normalized_beta = right_transpose.T @ ((left.T @ values) / singular_values)
    fitted = normalized @ normalized_beta
    residual = values - fitted
    normalized_xtx_inverse = (
        right_transpose.T @ np.diag(1.0 / np.square(singular_values)) @ right_transpose
    )
    leverage = np.einsum("ij,jk,ik->i", normalized, normalized_xtx_inverse, normalized)
    scaled = hc3_scaled_residuals(residual, leverage, equation_id=equation_id)
    meat = (normalized * scaled[:, None]).T @ (normalized * scaled[:, None])
    normalized_covariance = normalized_xtx_inverse @ meat @ normalized_xtx_inverse
    raw_transform = np.zeros((columns, columns), dtype=float)
    raw_transform[0, 0] = 1.0
    for column in range(1, columns):
        raw_transform[0, column] = -centers[column] / scales[column]
        raw_transform[column, column] = 1.0 / scales[column]
    beta = raw_transform @ normalized_beta
    covariance = raw_transform @ normalized_covariance @ raw_transform.T
    covariance = (covariance + covariance.T) / 2.0
    if not np.all(np.isfinite(beta)) or not np.all(np.isfinite(covariance)):
        raise ProcessReferenceFitError("nonfinite_estimate", f"equation={equation_id}")
    return {
        "coefficients": beta,
        "fitted": fitted,
        "residuals": residual,
        "covariance": covariance,
        "minimum_singular_value": minimum,
        "maximum_singular_value": maximum,
        "rank_tolerance": tolerance,
    }


def scale_aware_solver_boundary_check() -> dict[str, Any]:
    rows = 80
    x = np.asarray([index / 9.0 - 4.0 for index in range(rows)], dtype=float)
    design = np.column_stack([np.ones(rows), x])
    outcome = np.asarray(
        [1.25 + 0.75 * value + ((index * 7 % 13) - 6.0) / 100.0 for index, value in enumerate(x)],
        dtype=float,
    )
    base = scale_aware_ols(design, outcome, equation_id="Y")
    unit_scale, shift = 1.0e-9, 4.5
    transformed_design = np.column_stack([np.ones(rows), unit_scale * (x + shift)])
    transformed = scale_aware_ols(transformed_design, outcome, equation_id="Y")
    base_beta = np.asarray(base["coefficients"])
    transformed_beta = np.asarray(transformed["coefficients"])
    base_covariance = np.asarray(base["covariance"])
    transformed_covariance = np.asarray(transformed["covariance"])
    fitted_delta = float(np.max(np.abs(np.asarray(base["fitted"]) - np.asarray(transformed["fitted"]))))
    slope_delta = abs(float(transformed_beta[1] * unit_scale - base_beta[1]))
    intercept_delta = abs(float(transformed_beta[0] - (base_beta[0] - base_beta[1] * shift)))
    covariance_delta = abs(float(transformed_covariance[1, 1] * unit_scale**2 - base_covariance[1, 1]))
    statistic_delta = abs(float(
        transformed_beta[1] / math.sqrt(transformed_covariance[1, 1])
        - base_beta[1] / math.sqrt(base_covariance[1, 1])
    ))
    perturbation = np.asarray([((index * 11 % 17) - 8.0) * 1.0e-14 for index in range(rows)])
    near_collinear = np.column_stack([np.ones(rows), x, x + perturbation])
    rejected = False
    try:
        scale_aware_ols(near_collinear, outcome, equation_id="Y")
    except ProcessReferenceFitError as error:
        rejected = error.reason_code == "rank_deficient"
    return {
        "passed": (
            fitted_delta <= 1.0e-10
            and slope_delta <= 1.0e-10
            and intercept_delta <= 1.0e-9
            and covariance_delta <= 1.0e-10
            and statistic_delta <= 1.0e-9
            and rejected
        ),
        "normalization": "non_intercept_welford_mean_population_rms_v1",
        "rank_rule": "s_min_gt_s_max_times_max_n_p_times_epsilon_times_100",
        "fitted_maximum_absolute_difference": fitted_delta,
        "slope_back_transform_absolute_difference": slope_delta,
        "intercept_back_transform_absolute_difference": intercept_delta,
        "covariance_back_transform_absolute_difference": covariance_delta,
        "statistic_absolute_difference": statistic_delta,
        "relative_collinearity_rejected": rejected,
    }


EQUATION_SPECS = (
    EquationSpec("M1", (("path:X->M1", "path", ("X",)), ("control:C", "control", ("C",)))),
    EquationSpec("M2", (("path:M1->M2", "path", ("M1",)), ("control:C", "control", ("C",)))),
    EquationSpec(
        "M3",
        (("path:X->M3", "path", ("X",)), ("moderator:W", "moderator_main", ("W",)),
         ("interaction:X*W", "interaction", ("X", "W")), ("control:C", "control", ("C",))),
    ),
    EquationSpec(
        "M4",
        (("path:X->M4", "path", ("X",)), ("control:C", "control", ("C",))),
    ),
    EquationSpec(
        "Y",
        (("path:X->Y", "path", ("X",)), ("path:M2->Y", "path", ("M2",)),
         ("path:M3->Y", "path", ("M3",)), ("path:M4->Y", "path", ("M4",)),
         ("moderator:W", "moderator_main", ("W",)), ("moderator:B", "moderator_main", ("B",)),
         ("interaction:M4*B", "interaction", ("M4", "B")),
         ("interaction:W*B", "interaction", ("W", "B")),
         ("interaction:X*B", "interaction", ("X", "B")),
         ("interaction:X*W", "interaction", ("X", "W")),
         ("interaction:X*W*B", "interaction", ("X", "W", "B")),
         ("control:C", "control", ("C",))),
    ),
)


def variable_profiles(columns: dict[str, np.ndarray]) -> dict[str, dict[str, Any]]:
    moderator_names = {name for name, _ in MODERATORS}
    profiles: dict[str, dict[str, Any]] = {}
    for variable in [*PREDICTORS, *CONTROLS, OUTCOME]:
        values = columns[variable]
        scale = dict(MODERATORS).get(variable, "continuous")
        role = (
            "focal_predictor" if variable == "X" else
            "outcome" if variable == OUTCOME else
            "control" if variable in CONTROLS else
            "moderator" if variable in moderator_names else "mediator"
        )
        profiles[variable] = {
            "variable": variable, "role": role, "scale": scale,
            "raw_mean": float(np.mean(values)),
            "raw_sample_sd": float(np.std(values, ddof=1)),
            "raw_min": float(np.min(values)), "raw_max": float(np.max(values)),
            "levels": [0.0, 1.0] if scale == "binary_0_1" else [],
        }
    return profiles


def coded(values: np.ndarray, profile: dict[str, Any]) -> np.ndarray:
    return values if profile["scale"] == "binary_0_1" else values - profile["raw_mean"]


def fit_equation(
    spec: EquationSpec,
    columns: dict[str, np.ndarray],
    profiles: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    term_columns: list[np.ndarray] = []
    for _, _, variables in spec.terms:
        value = np.ones(len(columns[spec.outcome]))
        for variable in variables:
            raw = columns[variable]
            value *= coded(raw, profiles[variable]) if len(variables) > 1 else raw
        term_columns.append(value)
    design = np.column_stack([np.ones(len(columns[spec.outcome])), *term_columns])
    y = columns[spec.outcome]
    fitted_equation = scale_aware_ols(
        design,
        y,
        equation_id=f"equation:{spec.outcome}",
    )
    beta = np.asarray(fitted_equation["coefficients"])
    fitted = np.asarray(fitted_equation["fitted"])
    residual = np.asarray(fitted_equation["residuals"])
    covariance = np.asarray(fitted_equation["covariance"])
    covariance_diagonal = validate_hc3_covariance_diagonal(
        covariance,
        equation_id=f"equation:{spec.outcome}",
    )
    n, p = design.shape
    df = n - p
    distribution = student_t(df)
    critical = float(distribution.ppf(0.975))
    term_rows = [("intercept", "intercept", tuple()), *spec.terms]
    coefficients = []
    for index, (term_id, kind, variables) in enumerate(term_rows):
        se = float(math.sqrt(covariance_diagonal[index]))
        statistic = float(beta[index] / se)
        coefficients.append({
            "term_id": term_id, "kind": kind, "variables": list(variables),
            "estimate": float(beta[index]), "standard_error": se, "statistic": statistic,
            "p_value_two_sided": float(2.0 * distribution.sf(abs(statistic))),
            "confidence_interval_lower": float(beta[index] - critical * se),
            "confidence_interval_upper": float(beta[index] + critical * se),
        })
    rss = float(residual @ residual)
    tss = float(np.sum((y - np.mean(y)) ** 2))
    r2 = 0.0 if tss <= sys.float_info.epsilon else 1.0 - rss / tss
    sigma2 = max(rss / n, sys.float_info.min)
    return {
        "equation_id": f"equation:{spec.outcome}", "outcome": spec.outcome,
        "term_ids": [row["term_id"] for row in coefficients],
        "coefficients": coefficients, "coefficient_covariance": covariance.tolist(),
        "residual_degrees_of_freedom": df,
        "fit": {
            "observations": n,
            "parameter_count": p,
            "residual_sum_squares": rss,
            "total_sum_squares": tss,
            "r_squared": r2,
            "adjusted_r_squared": 1.0 - (1.0 - r2) * (n - 1) / df,
            "f_statistic": (r2 / (p - 1)) / max((1.0 - r2) / df, 1e-300),
            "aic": n * math.log(sigma2) + 2.0 * p,
            "bic": n * math.log(sigma2) + math.log(n) * p,
            "rmse": math.sqrt(rss / n),
        },
    }


def coefficient(equations: dict[str, dict[str, Any]], outcome: str, variables: Sequence[str]) -> dict[str, Any]:
    matches = [row for row in equations[outcome]["coefficients"] if row["variables"] == list(variables)]
    if len(matches) != 1:
        raise KeyError(f"coefficient {variables!r} in {outcome}")
    return matches[0]


def semantic_probe_levels(profile: dict[str, Any]) -> list[tuple[float, str]]:
    if profile["scale"] == "binary_0_1":
        return [(0.0, "binary_0"), (1.0, "binary_1")]
    levels = [
        profile["raw_mean"] - profile["raw_sample_sd"],
        profile["raw_mean"],
        profile["raw_mean"] + profile["raw_sample_sd"],
    ]
    if not all(math.isfinite(value) for value in levels) or not (
        levels[0] < levels[1] < levels[2]
    ):
        variable = profile.get("variable", "unknown")
        raise ProcessReferenceFitError(
            COLLAPSED_PROBE_GRID_REASON,
            f"PROCESS continuous moderator {variable} does not have three distinct finite "
            "mean-minus-SD, mean, and mean-plus-SD probes in f64",
        )
    return list(zip(levels, ("minus_1sd", "mean", "plus_1sd")))


def probe_values(profile: dict[str, Any]) -> list[float]:
    return [raw for raw, _ in semantic_probe_levels(profile)]


def moderator_value(variable: str, raw: float, profiles: dict[str, dict[str, Any]]) -> dict[str, float | str]:
    profile = profiles[variable]
    return {
        "variable": variable, "raw_value": float(raw),
        "coded_value": float(raw if profile["scale"] == "binary_0_1" else raw - profile["raw_mean"]),
    }


def semantic_probe_grid(
    moderation: tuple[str, str, str, str | None],
    raw_probe_profiles: dict[str, dict[str, Any]],
    coding_profiles: dict[str, dict[str, Any]],
) -> list[tuple[list[dict[str, Any]], str]]:
    _, _, primary, conditioning = moderation
    primary_levels = semantic_probe_levels(raw_probe_profiles[primary])
    conditioning_levels = (
        semantic_probe_levels(raw_probe_profiles[conditioning])
        if conditioning else [(None, "")]
    )
    return [
        (
            [moderator_value(primary, raw_primary, coding_profiles)]
            + ([moderator_value(conditioning, float(raw_conditioning), coding_profiles)] if conditioning else []),
            f"{primary}={primary_token}"
            + (f",{conditioning}={conditioning_token}" if conditioning else ""),
        )
        for raw_primary, primary_token in primary_levels
        for raw_conditioning, conditioning_token in conditioning_levels
    ]


def collapsed_probe_grid_boundary_check() -> dict[str, Any]:
    profile = {
        "variable": "W",
        "scale": "continuous",
        "raw_mean": 9_007_199_254_740_992.0,
        "raw_sample_sd": 0.25,
    }
    try:
        semantic_probe_levels(profile)
    except ProcessReferenceFitError as error:
        return {
            "passed": (
                error.reason_code == COLLAPSED_PROBE_GRID_REASON
                and str(error).startswith(f"{COLLAPSED_PROBE_GRID_REASON}|")
            ),
            "reason_code": error.reason_code,
            "message": str(error).split("|", 1)[1],
            "semantic_assignment": "canonical_grid_index_primary_outer_conditioning_inner",
        }
    return {"passed": False, "reason_code": None, "message": None}


def moderation_id(moderation: tuple[str, str, str, str | None]) -> str:
    source, target, primary, conditioning = moderation
    return f"moderation:{source}->{target}@{primary}" + (f"|{conditioning}" if conditioning else "")


def edge_slope(
    source: str,
    target: str,
    probes: Sequence[dict[str, Any]],
    equations: dict[str, dict[str, Any]],
) -> float:
    base = float(coefficient(equations, target, [source])["estimate"])
    moderation = next((row for row in MODERATIONS if row[0] == source and row[1] == target), None)
    if moderation is None:
        return base
    _, _, primary, conditioning = moderation
    by_variable = {row["variable"]: float(row["coded_value"]) for row in probes}
    primary_value = by_variable.get(primary, 0.0)
    slope = base + float(coefficient(equations, target, [source, primary])["estimate"]) * primary_value
    if conditioning:
        conditioning_value = by_variable.get(conditioning, 0.0)
        slope += float(coefficient(equations, target, [source, conditioning])["estimate"]) * conditioning_value
        slope += (
            float(coefficient(equations, target, [source, primary, conditioning])["estimate"])
            * primary_value * conditioning_value
        )
    return slope


SIMPLE_PATHS = [
    ["X", "M1", "M2", "Y"], ["X", "M3", "Y"], ["X", "M4", "Y"], ["X", "Y"]
]


def validate_original_endogenous_outcomes(columns: dict[str, np.ndarray]) -> None:
    """Reject exact 0/1 endogenous variables only at original-sample readiness."""

    for outcome in dict.fromkeys(target for _, target in PATHS):
        values = np.asarray(columns[outcome], dtype=float)
        if (
            np.any(values == 0.0)
            and np.any(values == 1.0)
            and np.all((values == 0.0) | (values == 1.0))
        ):
            raise ProcessReferenceFitError(
                BINARY_ENDOGENOUS_OUTCOME_REASON,
                f"variable={outcome}|scope=original_complete_sample",
            )


def binary_endogenous_outcome_boundary_check() -> dict[str, Any]:
    base = {name: np.linspace(0.1, 8.0, 80) for name in [*PREDICTORS, *CONTROLS, OUTCOME]}
    rejected: list[str] = []
    for outcome in ("M1", "Y"):
        candidate = {name: values.copy() for name, values in base.items()}
        candidate[outcome] = np.asarray([float(index % 2) for index in range(80)])
        try:
            validate_original_endogenous_outcomes(candidate)
        except ProcessReferenceFitError as error:
            if error.reason_code == BINARY_ENDOGENOUS_OUTCOME_REASON:
                rejected.append(outcome)
    continuous_accepted = True
    try:
        validate_original_endogenous_outcomes(base)
    except ProcessReferenceFitError:
        continuous_accepted = False
    return {
        "passed": rejected == ["M1", "Y"] and continuous_accepted,
        "reason_code": BINARY_ENDOGENOUS_OUTCOME_REASON,
        "rejected_outcomes": rejected,
        "original_sample_only": True,
        "continuous_fixture_accepted": continuous_accepted,
    }


def path_effect(path: Sequence[str], probes: Sequence[dict[str, Any]], equations: dict[str, dict[str, Any]]) -> float:
    result = 1.0
    for source, target in zip(path, path[1:]):
        result *= edge_slope(source, target, probes, equations)
    return result


def slope_weights(
    moderation: tuple[str, str, str, str | None],
    probes: Sequence[dict[str, Any]],
    equation: dict[str, Any],
) -> np.ndarray:
    source, _, primary, conditioning = moderation
    by_variable = {row["variable"]: float(row["coded_value"]) for row in probes}
    primary_value = by_variable.get(primary, 0.0)
    conditioning_value = by_variable.get(conditioning, 0.0) if conditioning else 0.0
    weights = []
    for row in equation["coefficients"]:
        variables = row["variables"]
        if variables == [source]:
            weight = 1.0
        elif variables == [source, primary]:
            weight = primary_value
        elif conditioning and variables == [source, conditioning]:
            weight = conditioning_value
        elif conditioning and variables == [source, primary, conditioning]:
            weight = primary_value * conditioning_value
        else:
            weight = 0.0
        weights.append(weight)
    return np.asarray(weights, dtype=float)


def linear_combination(
    equation: dict[str, Any],
    weights: np.ndarray,
    *,
    moderation_id: str,
) -> tuple[float, float]:
    beta = np.asarray([row["estimate"] for row in equation["coefficients"]])
    covariance = np.asarray(equation["coefficient_covariance"])
    estimate = float(weights @ beta)
    variance = float(weights @ covariance @ weights)
    if not math.isfinite(estimate) or not math.isfinite(variance) or variance <= 0.0:
        raise ProcessReferenceFitError(
            DEGENERATE_SIMPLE_SLOPE_REASON,
            f"moderation={moderation_id}|variance={variance:.17g}",
        )
    return estimate, math.sqrt(variance)


def reference_graph(
    columns: dict[str, np.ndarray],
    *,
    raw_probe_profiles: dict[str, dict[str, Any]] | None = None,
    include_diagnostics: bool = True,
    enforce_original_outcome_scope: bool = True,
) -> dict[str, Any]:
    if enforce_original_outcome_scope:
        validate_original_endogenous_outcomes(columns)
    profiles = variable_profiles(columns)
    # Bootstrap and delete-one equations are re-centered on their own fitted
    # sample, but every displayed/identified estimand is evaluated at the
    # original complete-sample raw probe grid. This is the scientific identity
    # that keeps conditional-effect IDs invariant across resamples.
    probe_profiles = raw_probe_profiles or profiles
    equations_list = [fit_equation(spec, columns, profiles) for spec in EQUATION_SPECS]
    equations = {row["outcome"]: row for row in equations_list}
    direct = path_effect(["X", "Y"], [], equations)
    indirect_rows = []
    for path in SIMPLE_PATHS[:-1]:
        indirect_rows.append({
            "effect_id": f"indirect:{'->'.join(path)}", "kind": "indirect",
            "path": path, "estimate": path_effect(path, [], equations),
        })
    total_indirect = sum(row["estimate"] for row in indirect_rows)
    reference_effects = [
        {"effect_id": "direct:X->Y", "kind": "direct", "path": ["X", "Y"], "estimate": direct},
        *indirect_rows,
        {"effect_id": "total_indirect:X->Y", "kind": "total_indirect", "path": ["X", "Y"], "estimate": total_indirect},
        {"effect_id": "total:X->Y", "kind": "total", "path": ["X", "Y"], "estimate": direct + total_indirect},
    ]
    conditional: list[dict[str, Any]] = []
    indices: list[dict[str, Any]] = []
    for path in SIMPLE_PATHS[:-1]:
        path_id = "->".join(path)
        moderation = next(
            (item for source, target in zip(path, path[1:]) for item in MODERATIONS if item[:2] == (source, target)),
            None,
        )
        if moderation is None:
            continue
        for probes, suffix in semantic_probe_grid(moderation, probe_profiles, profiles):
            conditional.append({
                "effect_id": f"indirect:{path_id}@{suffix}",
                "path_id": path_id, "moderator_values": probes,
                "estimate": path_effect(path, probes, equations),
            })
        source, target, primary, _ = moderation
        other = 1.0
        for edge_source, edge_target in zip(path, path[1:]):
            if (edge_source, edge_target) != (source, target):
                other *= edge_slope(edge_source, edge_target, [], equations)
        indices.append({
            "effect_id": f"index:{path_id}:{source}->{target}:{primary}",
            "path_id": path_id, "moderated_edge": f"{source}->{target}",
            "moderator": primary,
            "estimate": float(coefficient(equations, target, [source, primary])["estimate"]) * other,
        })
    simple_slopes: list[dict[str, Any]] = []
    for moderation in [("X", "M3", "W", None), ("X", "Y", "W", "B"), ("M4", "Y", "B", None)]:
        equation = equations[moderation[1]]
        distribution = student_t(equation["residual_degrees_of_freedom"])
        critical = float(distribution.ppf(0.975))
        mid = moderation_id(moderation)
        for probes, suffix in semantic_probe_grid(moderation, probe_profiles, profiles):
            estimate, se = linear_combination(
                equation,
                slope_weights(moderation, probes, equation),
                moderation_id=mid,
            )
            statistic = estimate / se
            simple_slopes.append({
                "effect_id": f"slope:{mid}@{suffix}",
                "moderation_id": mid, "moderator_values": probes,
                "estimate": estimate, "standard_error": se, "statistic": statistic,
                "p_value_two_sided": float(2.0 * distribution.sf(abs(statistic))),
                "confidence_interval_lower": estimate - critical * se,
                "confidence_interval_upper": estimate + critical * se,
            })
    plots = (
        [plot_reference(moderation, profiles, equations) for moderation in [("X", "M3", "W", None), ("X", "Y", "W", "B"), ("M4", "Y", "B", None)]]
        if include_diagnostics else []
    )
    johnson_neyman = (
        [
            result
            for moderation in [("X", "M3", "W", None), ("X", "Y", "W", "B"), ("M4", "Y", "B", None)]
            for result in johnson_neyman_reference(moderation, profiles, equations)
        ]
        if include_diagnostics else []
    )
    return {
        "policies": {
            "centering": "equation_complete_case_mean_v1", "covariance": "hc3_v1",
            "inference_reference": "student_t_residual_df_v1", "confidence_level": 0.95,
        },
        "complete_cases": len(columns[OUTCOME]),
        "variable_profiles": [profiles[name] for name in [*PREDICTORS, *CONTROLS, OUTCOME]],
        "paths": [{"path_id": f"{source}->{target}", "from": source, "to": target} for source, target in canonical_paths()],
        "moderations": [
            {
                "moderation_id": moderation_id(row), "from": row[0], "to": row[1],
                "moderator": row[2],
                **({"conditioning_moderator": row[3]} if row[3] is not None else {}),
            }
            for row in [("X", "M3", "W", None), ("X", "Y", "W", "B"), ("M4", "Y", "B", None)]
        ],
        "equations": equations_list, "reference_effects": reference_effects,
        "conditional_indirect_effects": conditional,
        "moderated_mediation_indices": indices, "simple_slopes": simple_slopes,
        "plots": plots, "johnson_neyman": johnson_neyman,
    }


def canonical_paths(paths: Sequence[tuple[str, str]] = PATHS) -> list[tuple[str, str]]:
    rank = {name: index for index, name in enumerate(["X", "M1", "M2", "M3", "M4", "Y"])}
    return sorted(paths, key=lambda edge: (rank[edge[1]], rank[edge[0]], edge[0], edge[1]))


def reference_condition_policy_check(graph: dict[str, Any]) -> dict[str, Any]:
    equations = {row["outcome"]: row for row in graph["equations"]}
    paths = SIMPLE_PATHS[:-1]
    direct = path_effect(["X", "Y"], [], equations)
    indirect = [path_effect(path, [], equations) for path in paths]
    expected = [direct, *indirect, sum(indirect), direct + sum(indirect)]
    observed = [float(row["estimate"]) for row in graph["reference_effects"]]
    maximum_delta = max((abs(left - right) for left, right in zip(expected, observed)), default=math.inf)
    return {
        "passed": len(observed) == 6 and maximum_delta <= ARITHMETIC_TOLERANCE,
        "column": "Reference condition",
        "value": REFERENCE_CONDITION,
        "continuous_coded_value": 0.0,
        "binary_raw_value": 0.0,
        "maximum_absolute_difference": maximum_delta,
    }


def point_metamorphic_invariance(columns: dict[str, np.ndarray]) -> dict[str, Any]:
    baseline = reference_graph(columns)
    reversed_rows = {name: values[::-1].copy() for name, values in columns.items()}
    with_irrelevant = {**columns, "irrelevant_validation_column": np.arange(len(columns[OUTCOME]), dtype=float)}
    reversed_delta = numeric_max_delta(baseline, reference_graph(reversed_rows))
    irrelevant_delta = numeric_max_delta(baseline, reference_graph(with_irrelevant))
    canonical_order = canonical_paths(PATHS)
    reversed_recipe_order = canonical_paths(list(reversed(PATHS)))
    return {
        "row_order_maximum_absolute_difference": reversed_delta,
        "irrelevant_column_maximum_absolute_difference": irrelevant_delta,
        "path_order_canonicalized": canonical_order == reversed_recipe_order,
        "passed": (
            reversed_delta <= ARITHMETIC_TOLERANCE
            and irrelevant_delta <= ARITHMETIC_TOLERANCE
            and canonical_order == reversed_recipe_order
        ),
    }


def plot_reference(
    moderation: tuple[str, str, str, str | None],
    profiles: dict[str, dict[str, Any]],
    equations: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    source, target, _, _ = moderation
    equation = equations[target]
    covariance = np.asarray(equation["coefficient_covariance"])
    critical = float(student_t(equation["residual_degrees_of_freedom"]).ppf(0.975))
    series = []
    for series_index, (probes, suffix) in enumerate(semantic_probe_grid(moderation, profiles, profiles)):
        raw_by_variable = {
            name: (0.0 if profile["scale"] == "binary_0_1" else profile["raw_mean"])
            for name, profile in profiles.items()
        }
        raw_by_variable.update({row["variable"]: row["raw_value"] for row in probes})
        points = []
        for index in range(25):
            x = profiles[source]["raw_min"] + (profiles[source]["raw_max"] - profiles[source]["raw_min"]) * index / 24
            raw_by_variable[source] = x
            design = []
            for row in equation["coefficients"]:
                if row["kind"] == "intercept":
                    design.append(1.0)
                    continue
                product = 1.0
                for variable in row["variables"]:
                    raw = float(raw_by_variable[variable])
                    product *= raw if len(row["variables"]) == 1 or profiles[variable]["scale"] == "binary_0_1" else raw - profiles[variable]["raw_mean"]
                design.append(product)
            vector = np.asarray(design)
            beta = np.asarray([row["estimate"] for row in equation["coefficients"]])
            predicted = float(vector @ beta)
            se = math.sqrt(max(0.0, float(vector @ covariance @ vector)))
            points.append({
                "predictor_raw": x, "predicted_raw": predicted,
                "confidence_interval_lower": predicted - critical * se,
                "confidence_interval_upper": predicted + critical * se,
            })
        series.append({
            "series_id": f"series:{series_index}:{suffix}",
            "moderator_values": probes, "points": points,
        })
    mid = moderation_id(moderation)
    return {"plot_id": f"plot:{mid}", "moderation_id": mid, "series": series}


def johnson_neyman_coded_roots(
    quadratic: float,
    linear: float,
    constant: float,
    coded_min: float,
    coded_max: float,
) -> list[float]:
    """Solve the frozen JN quadratic after affine domain normalization."""

    supplied = [quadratic, linear, constant, coded_min, coded_max]
    if not all(math.isfinite(value) for value in supplied) or coded_min > coded_max:
        return []
    midpoint = coded_min / 2.0 + coded_max / 2.0
    half_range = coded_max / 2.0 - coded_min / 2.0
    if not math.isfinite(midpoint) or not math.isfinite(half_range) or half_range <= 0.0:
        return []
    domain_quadratic = quadratic * half_range * half_range
    domain_linear = half_range * (2.0 * quadratic * midpoint + linear)
    domain_constant = quadratic * midpoint * midpoint + linear * midpoint + constant
    if not all(math.isfinite(value) for value in (domain_quadratic, domain_linear, domain_constant)):
        return []
    coefficient_scale = max(abs(domain_quadratic), abs(domain_linear), abs(domain_constant))
    if coefficient_scale == 0.0:
        return []
    a, b, c = (
        domain_quadratic / coefficient_scale,
        domain_linear / coefficient_scale,
        domain_constant / coefficient_scale,
    )
    coefficient_tolerance = PROCESS_JN_COEFFICIENT_TOLERANCE_MULTIPLIER * sys.float_info.epsilon
    normalized_roots: list[float] = []
    if abs(a) <= coefficient_tolerance:
        if abs(b) > coefficient_tolerance:
            normalized_roots.append(-c / b)
    else:
        discriminant_left = b * b
        discriminant_right = 4.0 * a * c
        discriminant = discriminant_left - discriminant_right
        discriminant_scale = max(
            abs(discriminant_left), abs(discriminant_right), sys.float_info.min
        )
        discriminant_tolerance = (
            PROCESS_JN_COEFFICIENT_TOLERANCE_MULTIPLIER
            * sys.float_info.epsilon
            * discriminant_scale
        )
        if discriminant >= -discriminant_tolerance:
            square_root = 0.0 if abs(discriminant) <= discriminant_tolerance else math.sqrt(discriminant)
            if square_root == 0.0:
                normalized_roots.append(-b / (2.0 * a))
            else:
                q = -0.5 * (b + math.copysign(square_root, b))
                if q == 0.0:
                    normalized_roots.append(-b / (2.0 * a))
                else:
                    normalized_roots.extend((q / a, c / q))
    domain_tolerance = (
        PROCESS_JN_ROOT_DEDUP_TOLERANCE_MULTIPLIER * sys.float_info.epsilon
    )
    normalized_roots = [
        min(max(root, -1.0), 1.0)
        for root in normalized_roots
        if math.isfinite(root)
        and root >= -1.0 - domain_tolerance
        and root <= 1.0 + domain_tolerance
    ]
    normalized_roots.sort()
    deduplicated_normalized: list[float] = []
    for root in normalized_roots:
        if deduplicated_normalized:
            root_scale = max(abs(deduplicated_normalized[-1]), abs(root), 1.0)
            if abs(root - deduplicated_normalized[-1]) <= (
                PROCESS_JN_ROOT_DEDUP_TOLERANCE_MULTIPLIER
                * sys.float_info.epsilon
                * root_scale
            ):
                continue
        deduplicated_normalized.append(root)
    roots = [
        coded_min + half_range * (root + 1.0)
        if root <= 0.0
        else coded_max - half_range * (1.0 - root)
        for root in deduplicated_normalized
    ]
    range_scale = max(
        abs(coded_min), abs(coded_max), abs(coded_max - coded_min), sys.float_info.min
    )
    range_tolerance = (
        PROCESS_JN_ROOT_DEDUP_TOLERANCE_MULTIPLIER
        * sys.float_info.epsilon
        * range_scale
    )
    roots = [
        min(max(root, coded_min), coded_max)
        for root in roots
        if math.isfinite(root)
        and root >= coded_min - range_tolerance
        and root <= coded_max + range_tolerance
    ]
    if len(roots) == 2 and quadratic != 0.0:
        mapped = list(roots)
        for target in range(2):
            other = mapped[1 - target]
            denominator = quadratic * other
            if math.isfinite(denominator) and denominator != 0.0:
                companion = constant / denominator
                at_boundary = roots[target] == coded_min or roots[target] == coded_max
                if (
                    at_boundary
                    and math.isfinite(companion)
                    and companion >= coded_min - range_tolerance
                    and companion <= coded_max + range_tolerance
                ):
                    roots[target] = min(max(companion, coded_min), coded_max)
    roots.sort()
    deduplicated: list[float] = []
    for root in roots:
        if deduplicated:
            root_scale = max(abs(deduplicated[-1]), abs(root), range_scale, sys.float_info.min)
            if abs(root - deduplicated[-1]) <= (
                PROCESS_JN_ROOT_DEDUP_TOLERANCE_MULTIPLIER
                * sys.float_info.epsilon
                * root_scale
            ):
                continue
        deduplicated.append(root)
    return deduplicated


def johnson_neyman_variance(v0: float, v1: float, v2: float, coded: float) -> float | None:
    variance = v0 + 2.0 * v1 * coded + v2 * coded * coded
    return variance if math.isfinite(variance) and variance > 0.0 else None


def johnson_neyman_variance_positive_across_range(
    v0: float,
    v1: float,
    v2: float,
    coded_min: float,
    coded_max: float,
) -> bool:
    if (
        johnson_neyman_variance(v0, v1, v2, coded_min) is None
        or johnson_neyman_variance(v0, v1, v2, coded_max) is None
    ):
        return False
    if v2 > 0.0:
        vertex = -v1 / v2
        if (
            coded_min < vertex < coded_max
            and johnson_neyman_variance(v0, v1, v2, vertex) is None
        ):
            return False
    return True


def johnson_neyman_root_solver_boundary_check() -> dict[str, Any]:
    base = johnson_neyman_coded_roots(1.0, -1.0, -2.0, -3.0, 3.0)
    transformed_deltas: list[float] = []
    transformed_lengths: list[int] = []
    for scale in (1.0e-10, 1.0e10):
        shift = 7.0 * scale
        transformed = johnson_neyman_coded_roots(
            1.0 / scale**2,
            -1.0 / scale - 2.0 * shift / scale**2,
            -2.0 + shift / scale + shift**2 / scale**2,
            shift - 3.0 * scale,
            shift + 3.0 * scale,
        )
        transformed_lengths.append(len(transformed))
        transformed_deltas.append(max(
            abs(transformed[0] - (shift - scale)),
            abs(transformed[1] - (shift + 2.0 * scale)),
        ) if len(transformed) == 2 else math.inf)
    exact_double = johnson_neyman_coded_roots(1.0, -2.0, 1.0, 0.0, 2.0)
    resolvable_near_double = johnson_neyman_coded_roots(
        1.0, -(2.0 + 1.0e-12), 1.0 + 1.0e-12, 0.0, 2.0
    )
    imbalanced = johnson_neyman_coded_roots(
        1.0, -(1.0e12 + 1.0e-12), 1.0, 0.0, 2.0e12
    )
    passed = (
        len(base) == 2
        and abs(base[0] + 1.0) <= 1.0e-12
        and abs(base[1] - 2.0) <= 1.0e-12
        and transformed_lengths == [2, 2]
        and transformed_deltas[0] <= 1.0e-22
        and transformed_deltas[1] <= 1.0e-2
        and len(exact_double) == 1
        and abs(exact_double[0] - 1.0) <= 1.0e-12
        and len(resolvable_near_double) == 2
        and abs(resolvable_near_double[0] - 1.0) <= 1.0e-12
        and abs(resolvable_near_double[1] - (1.0 + 1.0e-12)) <= 1.0e-12
        and len(imbalanced) == 2
        and abs(imbalanced[0] - 1.0e-12) <= 1.0e-24
        and abs(imbalanced[1] - 1.0e12) <= 1.0e-3
    )
    return {
        "passed": passed,
        "domain_normalization": "coded_range_to_minus_one_plus_one_v1",
        "coefficient_tolerance_multiplier": PROCESS_JN_COEFFICIENT_TOLERANCE_MULTIPLIER,
        "root_deduplication_tolerance_multiplier": PROCESS_JN_ROOT_DEDUP_TOLERANCE_MULTIPLIER,
        "stable_quadratic_formula": "q_formula_v1",
        "base_roots": base,
        "affine_transformed_maximum_absolute_differences": transformed_deltas,
        "exact_double_root_count": len(exact_double),
        "resolvable_near_double_root_count": len(resolvable_near_double),
        "imbalanced_roots": imbalanced,
    }


def johnson_neyman_invalid_covariance_boundary_check() -> dict[str, Any]:
    invalid = (
        not johnson_neyman_variance_positive_across_range(0.0, 0.0, 1.0, -1.0, 1.0)
        and not johnson_neyman_variance_positive_across_range(1.0, 1.1, 1.0, -2.0, 2.0)
    )
    valid = johnson_neyman_variance_positive_across_range(1.0, 0.1, 0.25, -1.0, 1.0)
    return {
        "passed": invalid and valid,
        "reason_code": INVALID_HC3_COVARIANCE_REASON,
        "message": JN_INVALID_COVARIANCE_MESSAGE,
        "variance_rule": "finite_and_strictly_positive_across_tested_range",
    }


def johnson_neyman_reference(
    moderation: tuple[str, str, str, str | None],
    profiles: dict[str, dict[str, Any]],
    equations: dict[str, dict[str, Any]],
) -> list[dict[str, Any]]:
    _, target, primary, conditioning = moderation
    mid = moderation_id(moderation)
    conditioning_grids = (
        [[moderator_value(conditioning, raw, profiles)] for raw in probe_values(profiles[conditioning])]
        if conditioning else [[]]
    )
    if profiles[primary]["scale"] == "binary_0_1":
        return [{
            "status": "unavailable", "moderation_id": mid, "solved_moderator": primary,
            "conditioning_values": values, "reason_code": "binary_solved_moderator",
            "message": "Johnson-Neyman regions require a continuous solved moderator.",
        } for values in conditioning_grids]
    equation = equations[target]
    covariance = np.asarray(equation["coefficient_covariance"])
    critical = float(student_t(equation["residual_degrees_of_freedom"]).ppf(0.975))
    profile = profiles[primary]
    results = []
    for conditioning_values in conditioning_grids:
        at_zero = [*conditioning_values, moderator_value(primary, profile["raw_mean"], profiles)]
        at_one = [*conditioning_values, moderator_value(primary, profile["raw_mean"] + 1.0, profiles)]
        w0 = slope_weights(moderation, at_zero, equation)
        w1 = slope_weights(moderation, at_one, equation)
        delta = w1 - w0
        beta = np.asarray([row["estimate"] for row in equation["coefficients"]])
        a, b = float(w0 @ beta), float(delta @ beta)
        v0, v1, v2 = float(w0 @ covariance @ w0), float(w0 @ covariance @ delta), float(delta @ covariance @ delta)
        coded_min = profile["raw_min"] - profile["raw_mean"]
        coded_max = profile["raw_max"] - profile["raw_mean"]
        if (
            not all(math.isfinite(value) for value in (a, b, v0, v1, v2))
            or not johnson_neyman_variance_positive_across_range(
                v0, v1, v2, coded_min, coded_max
            )
        ):
            results.append({
                "status": "unavailable",
                "moderation_id": mid,
                "solved_moderator": primary,
                "conditioning_values": conditioning_values,
                "reason_code": INVALID_HC3_COVARIANCE_REASON,
                "message": JN_INVALID_COVARIANCE_MESSAGE,
            })
            continue
        qa = b * b - critical * critical * v2
        qb = 2.0 * (a * b - critical * critical * v1)
        qc = a * a - critical * critical * v0
        coded_roots = johnson_neyman_coded_roots(qa, qb, qc, coded_min, coded_max)
        roots = [root + profile["raw_mean"] for root in coded_roots]
        boundaries = [profile["raw_min"], *roots, profile["raw_max"]]
        regions = []
        for lower, upper in zip(boundaries, boundaries[1:]):
            coded_value = (lower + upper) / 2.0 - profile["raw_mean"]
            effect = a + b * coded_value
            variance = johnson_neyman_variance(v0, v1, v2, coded_value)
            if variance is None:
                raise ProcessReferenceFitError(
                    INVALID_HC3_COVARIANCE_REASON,
                    JN_INVALID_COVARIANCE_MESSAGE,
                )
            margin = critical * math.sqrt(variance)
            status = "significant_negative" if effect + margin < 0.0 else "significant_positive" if effect - margin > 0.0 else "not_significant"
            regions.append({"lower": lower, "upper": upper, "status": status})
        curve = []
        for index in range(101):
            raw = profile["raw_min"] + (profile["raw_max"] - profile["raw_min"]) * index / 100
            value = raw - profile["raw_mean"]
            effect = a + b * value
            variance = johnson_neyman_variance(v0, v1, v2, value)
            if variance is None:
                raise ProcessReferenceFitError(
                    INVALID_HC3_COVARIANCE_REASON,
                    JN_INVALID_COVARIANCE_MESSAGE,
                )
            se = math.sqrt(variance)
            curve.append({
                "moderator_raw": raw, "effect": effect, "standard_error": se,
                "confidence_interval_lower": effect - critical * se,
                "confidence_interval_upper": effect + critical * se,
            })
        results.append({
            "status": "available", "moderation_id": mid, "solved_moderator": primary,
            "conditioning_values": conditioning_values, "raw_min": profile["raw_min"],
            "raw_max": profile["raw_max"], "roots": roots, "regions": regions,
            "curve_points": curve,
        })
    return results


def numeric_max_delta(expected: Any, observed: Any, *, ignored: frozenset[str] = frozenset()) -> float:
    deltas: list[float] = []
    def visit(left: Any, right: Any, path: str) -> None:
        key = path.rsplit(".", 1)[-1]
        if key in ignored:
            return
        if isinstance(left, bool) or isinstance(right, bool):
            if left != right:
                deltas.append(math.inf)
        elif isinstance(left, (int, float)) and isinstance(right, (int, float)):
            deltas.append(abs(float(left) - float(right)))
        elif isinstance(left, dict) and isinstance(right, dict):
            if set(left) != set(right):
                deltas.append(math.inf)
                return
            for child in left:
                visit(left[child], right[child], f"{path}.{child}")
        elif isinstance(left, list) and isinstance(right, list):
            if len(left) != len(right):
                deltas.append(math.inf)
                return
            for index, (child_left, child_right) in enumerate(zip(left, right)):
                visit(child_left, child_right, f"{path}[{index}]")
        elif left != right:
            deltas.append(math.inf)
    visit(expected, observed, "root")
    return max(deltas, default=0.0)


def type7(values: Sequence[float], probability: float) -> float:
    ordered = sorted(float(value) for value in values)
    position = (len(ordered) - 1) * probability
    lower, upper = math.floor(position), math.ceil(position)
    return ordered[lower] if lower == upper else ordered[lower] + (position - lower) * (ordered[upper] - ordered[lower])


def bca(values: Sequence[float], original: float, jackknife: Sequence[float]) -> dict[str, Any]:
    below = sum(value < original for value in values)
    tied = sum(value == original for value in values)
    probability = min(max((below + 0.5 * tied) / len(values), 0.5 / len(values)), 1.0 - 0.5 / len(values))
    z0 = float(norm.ppf(probability))
    mean = float(np.mean(jackknife))
    centered = [mean - value for value in jackknife]
    sum_squares = sum(value * value for value in centered)
    if sum_squares <= sys.float_info.epsilon:
        return {"status": "unavailable", "reason_code": "zero_jackknife_variance"}
    acceleration = sum(value ** 3 for value in centered) / (6.0 * sum_squares ** 1.5)
    adjusted = []
    for nominal in (0.025, 0.975):
        z = float(norm.ppf(nominal))
        denominator = 1.0 - acceleration * (z0 + z)
        if not math.isfinite(denominator) or abs(denominator) <= sys.float_info.epsilon:
            return {"status": "unavailable", "reason_code": "nonfinite_adjusted_probability"}
        probability = float(norm.cdf(z0 + (z0 + z) / denominator))
        if not math.isfinite(probability):
            return {"status": "unavailable", "reason_code": "nonfinite_adjusted_probability"}
        adjusted.append(min(max(probability, 0.0), 1.0))
    return {
        "status": "available", "bias_correction": z0, "acceleration": acceleration,
        "lower": type7(values, adjusted[0]), "upper": type7(values, adjusted[1]),
    }


def exact_bootstrap_check(graph: dict[str, Any]) -> dict[str, Any]:
    bootstrap = graph.get("bootstrap")
    if not isinstance(bootstrap, dict):
        return {"passed": False, "reason": "bootstrap missing"}
    witness = bootstrap.get("validation_witness", {})
    ids = witness.get("estimand_ids", [])
    expected_ids = [
        *(row["effect_id"] for row in graph["reference_effects"]),
        *(row["effect_id"] for row in graph["conditional_indirect_effects"]),
        *(row["effect_id"] for row in graph["moderated_mediation_indices"]),
        *(row["effect_id"] for row in graph["simple_slopes"]),
    ]
    bootstrap_rows = witness.get("successful_bootstrap", [])
    jackknife_rows = witness.get("successful_jackknife", [])
    failed_jackknife = witness.get("failed_jackknife", [])
    max_delta = 0.0
    identities = ids == expected_ids and [row["effect_id"] for row in bootstrap.get("estimands", [])] == ids
    for index, row in enumerate(bootstrap.get("estimands", [])):
        values = [float(item["estimates"][index]) for item in bootstrap_rows]
        jackknife = [float(item["estimates"][index]) for item in jackknife_rows]
        original = float(row["original"])
        mean = float(np.mean(values))
        se = float(np.std(values, ddof=1))
        expected = {
            "bootstrap_mean": mean, "bias": mean - original, "standard_error": se,
            "percentile_lower": type7(values, 0.025), "percentile_upper": type7(values, 0.975),
        }
        for field, value in expected.items():
            max_delta = max(max_delta, abs(float(row[field]) - value))
        test = row["test"]
        if se > 0.0:
            statistic = original / se
            max_delta = max(max_delta, abs(float(test["statistic"]) - statistic), abs(float(test["p_value_two_sided"]) - 2.0 * float(norm.sf(abs(statistic)))))
        else:
            identities &= test == {"status": "unavailable", "reason_code": "zero_bootstrap_standard_error", "message": test.get("message")}
        if failed_jackknife:
            identities &= row["bca"]["status"] == "unavailable" and row["bca"]["reason_code"] == "incomplete_jackknife"
        else:
            expected_bca = bca(values, original, jackknife)
            identities &= row["bca"]["status"] == expected_bca["status"]
            if expected_bca["status"] == "available":
                for field in ("bias_correction", "acceleration", "lower", "upper"):
                    max_delta = max(max_delta, abs(float(row["bca"][field]) - float(expected_bca[field])))
            else:
                identities &= row["bca"]["reason_code"] == expected_bca["reason_code"]
    successful_indexes = [row["replicate_index"] for row in bootstrap_rows]
    failed_indexes = [row["replicate_index"] for row in bootstrap.get("failed_replicates", [])]
    bootstrap_partition = sorted(successful_indexes + failed_indexes) == list(range(bootstrap["requested_replicates"])) and not set(successful_indexes) & set(failed_indexes)
    successful_cases = [row["omitted_case"] for row in jackknife_rows]
    failed_cases = [row["omitted_case"] for row in failed_jackknife]
    jackknife_partition = sorted(successful_cases + failed_cases) == list(range(bootstrap["jackknife_cases"])) and not set(successful_cases) & set(failed_cases)
    identity = (
        bootstrap.get("method_version") == BOOTSTRAP_METHOD_VERSION
        and bootstrap.get("algorithm") == "indexed_case_resampling_v1"
        and bootstrap.get("interval_policy") == "percentile_primary_bca_conditional_v1"
        and bootstrap.get("test_reference") == "standard_normal_bootstrap_ratio_v1"
        and bootstrap.get("stream_token") == "process_indexed_case_stream_v1"
        and bootstrap.get("minimum_usable_fraction") == 0.9
        and witness.get("method_version") == WITNESS_VERSION
    )
    return {
        "passed": identity and identities and bootstrap_partition and jackknife_partition and max_delta <= ARITHMETIC_TOLERANCE,
        "identity": identity, "estimand_identity": identities,
        "bootstrap_index_partition": bootstrap_partition,
        "jackknife_index_partition": jackknife_partition,
        "maximum_absolute_difference": max_delta,
    }


def estimand_vector(graph: dict[str, Any]) -> tuple[list[str], np.ndarray]:
    rows = [*graph["reference_effects"], *graph["conditional_indirect_effects"], *graph["moderated_mediation_indices"], *graph["simple_slopes"]]
    return [row["effect_id"] for row in rows], np.asarray([row["estimate"] for row in rows])


def independent_resampling(columns: dict[str, np.ndarray], *, samples: int) -> dict[str, Any]:
    rng = np.random.default_rng(PYTHON_SEED)
    original_profiles = variable_profiles(columns)
    ids, original = estimand_vector(reference_graph(columns, raw_probe_profiles=original_profiles))
    estimates: list[np.ndarray] = []
    for _ in range(samples):
        indices = rng.integers(0, len(columns[OUTCOME]), len(columns[OUTCOME]))
        resampled = {name: values[indices] for name, values in columns.items()}
        try:
            observed_ids, values = estimand_vector(reference_graph(
                resampled,
                raw_probe_profiles=original_profiles,
                include_diagnostics=False,
                enforce_original_outcome_scope=False,
            ))
        except (ValueError, np.linalg.LinAlgError):
            continue
        if observed_ids == ids and np.all(np.isfinite(values)):
            estimates.append(values)
    matrix = np.asarray(estimates)
    return {
        "estimand_ids": ids, "original": original.tolist(),
        "requested_replicates": samples, "usable_replicates": len(estimates),
        "mean": np.mean(matrix, axis=0).tolist(), "standard_error": np.std(matrix, axis=0, ddof=1).tolist(),
        "percentile_lower": [type7(matrix[:, index], 0.025) for index in range(matrix.shape[1])],
        "percentile_upper": [type7(matrix[:, index], 0.975) for index in range(matrix.shape[1])],
    }


def compare_distribution(bootstrap: dict[str, Any], reference: dict[str, Any]) -> dict[str, Any]:
    rows = bootstrap["estimands"]
    identities = [row["effect_id"] for row in rows] == reference["estimand_ids"]
    maxima = {name: 0.0 for name in COMPARISON_THRESHOLDS}
    terms = []
    for index, row in enumerate(rows):
        q_se = float(row["standard_error"])
        r_se = float(reference["standard_error"][index])
        pooled = max(math.sqrt((q_se * q_se + r_se * r_se) / 2.0), 1e-12)
        observed = {
            "mean_pooled_se_units": abs(float(row["bootstrap_mean"]) - float(reference["mean"][index])) / pooled,
            "standard_error_relative": abs(q_se - r_se) / pooled,
            "percentile_endpoint_pooled_se_units": max(
                abs(float(row["percentile_lower"]) - float(reference["percentile_lower"][index])),
                abs(float(row["percentile_upper"]) - float(reference["percentile_upper"][index])),
            ) / pooled,
        }
        for name, value in observed.items():
            maxima[name] = max(maxima[name], value)
        terms.append({"effect_id": row["effect_id"], **observed})
    threshold_checks = {name: maxima[name] <= threshold for name, threshold in COMPARISON_THRESHOLDS.items()}
    return {
        "passed": identities and reference["usable_replicates"] >= math.ceil(reference["requested_replicates"] * 0.9) and all(threshold_checks.values()),
        "identities": identities, "thresholds": COMPARISON_THRESHOLDS,
        "observed_maxima": maxima, "threshold_checks": threshold_checks, "estimands": terms,
    }


def compare_r_point(graph: dict[str, Any], reference: dict[str, Any]) -> dict[str, Any]:
    """Compare the deliberately compact R equation/estimand payload."""

    equations = reference.get("equations", [])
    expected_term_ids = [equation["term_ids"] for equation in equations]
    expected_estimates = [
        [coefficient["estimate"] for coefficient in equation["coefficients"]]
        for equation in equations
    ]
    expected_covariances = [equation["coefficient_covariance"] for equation in equations]
    expected_ids, expected_values = estimand_vector(reference)
    identities = (
        graph.get("equation_term_ids") == expected_term_ids
        and graph.get("estimand_ids") == expected_ids
    )
    maximum_absolute_difference = max(
        numeric_max_delta(expected_estimates, graph.get("equation_estimates")),
        numeric_max_delta(expected_covariances, graph.get("equation_covariances")),
        numeric_max_delta(expected_values.tolist(), graph.get("estimand_values")),
    )
    return {
        "passed": identities and maximum_absolute_difference <= EXACT_TOLERANCE,
        "identities": identities,
        "maximum_absolute_difference": maximum_absolute_difference,
    }


def r_reference() -> dict[str, Any]:
    found = find_rscript_optional()
    if found is None:
        return {"available": False, "passed": False, "reason": "Rscript not found"}
    rscript, version = found
    completed = run([rscript, str(R_SCRIPT), str(FIXTURE), str(R_REPORT), str(QUICKPLS_REPLICATES)], timeout=1_200)
    if completed.returncode or not R_REPORT.is_file():
        return {"available": True, "passed": False, "version": version, "stderr_tail": completed.stderr[-2000:]}
    payload = json.loads(R_REPORT.read_text(encoding="utf-8"))
    return {"available": True, "passed": payload.get("passed") is True, "version": version, "data": payload}


def normalized_graph(graph: dict[str, Any]) -> dict[str, Any]:
    value = copy.deepcopy(graph)
    if isinstance(value.get("bootstrap"), dict):
        value["bootstrap"].pop("workers", None)
    return value


def worker_invariance_comparison(
    serial_graph: dict[str, Any], parallel_graph: dict[str, Any]
) -> dict[str, Any]:
    """Compare exact scientific payloads while excluding declared worker metadata.

    The bounded mismatch rows make a failed runtime gate diagnosable without
    retaining a second multi-megabyte witness artifact. They do not relax the
    exact equality requirement.
    """

    serial_workers = serial_graph.get("bootstrap", {}).get("workers")
    parallel_workers = parallel_graph.get("bootstrap", {}).get("workers")
    serial = normalized_graph(serial_graph)
    parallel = normalized_graph(parallel_graph)
    mismatches: list[dict[str, Any]] = []
    mismatch_count = 0
    maximum_numeric_difference = 0.0

    def record(path: str, kind: str, left: Any, right: Any) -> None:
        nonlocal mismatch_count
        mismatch_count += 1
        if len(mismatches) < 20:
            mismatches.append({"path": path, "kind": kind, "serial": left, "parallel": right})

    def visit(left: Any, right: Any, path: str) -> None:
        nonlocal maximum_numeric_difference
        if isinstance(left, bool) or isinstance(right, bool):
            if left != right:
                record(path, "boolean", left, right)
        elif isinstance(left, (int, float)) and isinstance(right, (int, float)):
            if float(left) != float(right):
                difference = abs(float(left) - float(right))
                maximum_numeric_difference = max(maximum_numeric_difference, difference)
                record(path, "number", left, right)
        elif isinstance(left, dict) and isinstance(right, dict):
            for key in sorted(set(left) - set(right)):
                record(f"{path}.{key}", "missing_parallel_key", left[key], None)
            for key in sorted(set(right) - set(left)):
                record(f"{path}.{key}", "missing_serial_key", None, right[key])
            for key in sorted(set(left) & set(right)):
                visit(left[key], right[key], f"{path}.{key}")
        elif isinstance(left, list) and isinstance(right, list):
            if len(left) != len(right):
                record(path, "list_length", len(left), len(right))
            for index, (left_item, right_item) in enumerate(zip(left, right)):
                visit(left_item, right_item, f"{path}[{index}]")
        elif left != right:
            record(path, "value", left, right)

    visit(serial, parallel, "graph_v2")
    canonical = lambda value: json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False
    ).encode("utf-8")
    return {
        "passed": serial == parallel,
        "workers_compared": [serial_workers, parallel_workers],
        "serial_normalized_sha256": hashlib.sha256(canonical(serial)).hexdigest(),
        "parallel_normalized_sha256": hashlib.sha256(canonical(parallel)).hexdigest(),
        "mismatch_count": mismatch_count,
        "maximum_numeric_difference": maximum_numeric_difference,
        "first_mismatches": mismatches,
    }


def write_compact_artifacts(recipe: dict[str, Any], result: dict[str, Any]) -> dict[str, str]:
    recipe_path = RESULTS / "process_v2_reference.recipe.json"
    result_path = RESULTS / "process_v2_quickpls.json"
    recipe_path.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")
    result_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return {"recipe": recipe_path.relative_to(ROOT).as_posix(), "quickpls_result": result_path.relative_to(ROOT).as_posix()}


def main() -> int:
    require_release_cli()
    write_fixture()
    columns, total_rows = complete_case_columns()
    independent = reference_graph(columns)
    metamorphic = point_metamorphic_invariance(columns)
    numerical_boundaries = {
        "scale_aware_solver": scale_aware_solver_boundary_check(),
        "johnson_neyman_root_solver": johnson_neyman_root_solver_boundary_check(),
        "johnson_neyman_invalid_covariance": johnson_neyman_invalid_covariance_boundary_check(),
        "binary_endogenous_outcome": binary_endogenous_outcome_boundary_check(),
        "collapsed_probe_grid": collapsed_probe_grid_boundary_check(),
    }
    reference_condition = reference_condition_policy_check(independent)
    with tempfile.TemporaryDirectory(prefix="quickpls-process-v2-") as directory:
        temporary = Path(directory)
        fingerprint = dataset_fingerprint(temporary)
        runs: dict[int, tuple[dict[str, Any], dict[str, Any]]] = {
            workers: run_quickpls(temporary, fingerprint, workers=workers, samples=QUICKPLS_REPLICATES)
            for workers in WORKERS
        }
    recipe, result = runs[1]
    estimation = result["payload"]["estimation"]
    regression = estimation["regression"]
    process = regression["process"]
    graph = process["graph_v2"]
    parallel_graph = runs[4][1]["payload"]["estimation"]["regression"]["process"]["graph_v2"]
    independent["omitted_cases"] = total_rows - len(columns[OUTCOME])
    point_graph = {key: value for key, value in graph.items() if key != "bootstrap"}
    point_delta = numeric_max_delta(independent, point_graph)
    worker_invariance = worker_invariance_comparison(graph, parallel_graph)
    exact_bootstrap = exact_bootstrap_check(graph)
    python_reference = independent_resampling(columns, samples=INDEPENDENT_REPLICATES)
    python_comparison = compare_distribution(graph["bootstrap"], python_reference)
    r_result = r_reference()
    r_data = r_result.get("data", {}) if r_result.get("passed") else {}
    r_point = compare_r_point(r_data.get("graph", {}), independent) if r_data else {
        "passed": False,
        "identities": False,
        "maximum_absolute_difference": math.inf,
    }
    r_comparison = compare_distribution(graph["bootstrap"], r_data.get("resampling", {})) if r_data else {"passed": False}
    provenance = result.get("provenance", {})
    checks = {
        "typed_recipe_and_method_identity": (
            recipe["schema_version"] == 3 and process["method_version"] == METHOD_VERSION
            and process["model"] == "graph" and estimation["method_version"] == METHOD_VERSION
            and provenance.get("method_version") == f"{METHOD_VERSION}+{BOOTSTRAP_METHOD_VERSION}"
            and recipe.get("metadata", {}).get("status")
            == "candidate_regression_process_v2_plus_bootstrap_v1_bounded_scope"
            and "validated" not in recipe.get("metadata", {}).get("status", "")
        ),
        "global_listwise_sample_and_profiles": graph["complete_cases"] == len(columns[OUTCOME]) and graph["omitted_cases"] == total_rows - len(columns[OUTCOME]),
        "equation_order_terms_hc3_fit": point_delta <= EXACT_TOLERANCE,
        "hc3_high_leverage_fail_closed": hc3_high_leverage_boundary_check(),
        "hc3_covariance_diagonal_fail_closed": hc3_covariance_diagonal_boundary_check(),
        "simple_slope_variance_fail_closed": simple_slope_variance_boundary_check(),
        "scale_aware_solver_affine_unit_invariance_and_relative_collinearity": numerical_boundaries["scale_aware_solver"]["passed"],
        "johnson_neyman_root_solver_affine_stability": numerical_boundaries["johnson_neyman_root_solver"]["passed"],
        "johnson_neyman_invalid_covariance_fail_closed": numerical_boundaries["johnson_neyman_invalid_covariance"]["passed"],
        "binary_endogenous_outcome_rejected_in_original_sample": numerical_boundaries["binary_endogenous_outcome"]["passed"],
        "semantic_probe_grid_rejects_collapsed_f64_levels": numerical_boundaries["collapsed_probe_grid"]["passed"],
        "reference_effect_condition_policy": reference_condition["passed"],
        "point_metamorphic_invariance": metamorphic["passed"],
        "parallel_serial_effect_arithmetic": point_delta <= EXACT_TOLERANCE and len(graph["reference_effects"]) == 6,
        "continuous_binary_three_way_slopes": point_delta <= EXACT_TOLERANCE and len(graph["simple_slopes"]) == 11,
        "first_second_stage_moderated_mediation": point_delta <= EXACT_TOLERANCE and len(graph["conditional_indirect_effects"]) == 5 and len(graph["moderated_mediation_indices"]) == 2,
        "johnson_neyman_roots_regions_curves": point_delta <= EXACT_TOLERANCE and len(graph["johnson_neyman"]) == 4,
        "conditional_plot_arithmetic": point_delta <= EXACT_TOLERANCE and len(graph["plots"]) == 3,
        "bootstrap_witness_type7_bca_ratio": exact_bootstrap["passed"],
        "bootstrap_index_partitions": exact_bootstrap["bootstrap_index_partition"] and exact_bootstrap["jackknife_index_partition"],
        "independent_python_resampling_distribution": python_comparison["passed"],
        "independent_r_equations_effects_resampling": (
            r_result.get("passed") is True
            and r_point["passed"]
            and r_comparison["passed"]
            and r_data.get("numerical_boundaries", {}).get("passed") is True
            and r_data.get("reference_condition", {}).get("passed") is True
        ),
        "seed_worker_invariance": worker_invariance["passed"],
        "dedicated_graph_result_not_generic_regression": (
            regression.get("method_version") == METHOD_VERSION
            and regression.get("regression_type") == "process"
            and regression.get("outcome") == OUTCOME
            and regression.get("predictors") == PREDICTORS
            and regression.get("controls") == CONTROLS
            and regression.get("observations") == graph.get("complete_cases")
            and regression.get("coefficients") == []
            and regression.get("fit") is None
            and regression.get("predictions") == []
            and "logistic" not in regression
            and "bootstrap" not in regression
            and estimation.get("mediation") is None
            and estimation.get("moderation") is None
        ),
        "legacy_v1_not_current_evidence": process["effects"] == [] and process["simple_slopes"] == [] and graph is not None,
    }
    if frozenset(checks) != REFERENCE_CHECK_NAMES:
        raise RuntimeError("PROCESS v2 reference check identity drifted")
    artifacts = {
        "fixture": artifact(FIXTURE), "r_script": artifact(R_SCRIPT),
        "tested_cli": artifact(CLI), **write_compact_artifacts(recipe, result),
        "r_report": artifact(R_REPORT) if R_REPORT.is_file() else None,
    }
    report = {
        "schema_version": 1, "target": TARGET, "feature_id": FEATURE_ID,
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "method_version": METHOD_VERSION, "bootstrap_method_version": BOOTSTRAP_METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "passed": all(checks.values()), "checks": checks,
        "scope": {
            "graph_defined_not_numbered_templates": True,
            "families": ["multiple_equations", "parallel_mediation", "serial_mediation", "continuous_moderation", "binary_0_1_moderation", "three_way_moderated_moderation", "first_stage_moderated_mediation", "second_stage_moderated_mediation", "simple_slopes", "johnson_neyman", "percentile_bca_bootstrap"],
            "outcome_family": "continuous_ols_only", "missing_data": "global_listwise_deletion",
            "covariance": "hc3_v1", "inference": "fixed_two_sided_95_percent_student_t",
            "equation_solver": {
                "algorithm": "scale_aware_thin_svd_v1",
                "normalization": "non_intercept_welford_mean_population_rms_v1",
                "rank_tolerance_multiplier": PROCESS_RELATIVE_RANK_TOLERANCE_MULTIPLIER,
            },
            "johnson_neyman_solver": {
                "domain_normalization": "coded_range_to_minus_one_plus_one_v1",
                "coefficient_tolerance_multiplier": PROCESS_JN_COEFFICIENT_TOLERANCE_MULTIPLIER,
                "root_deduplication_tolerance_multiplier": PROCESS_JN_ROOT_DEDUP_TOLERANCE_MULTIPLIER,
                "invalid_covariance_reason": INVALID_HC3_COVARIANCE_REASON,
                "invalid_covariance_message": JN_INVALID_COVARIANCE_MESSAGE,
            },
            "reference_condition": REFERENCE_CONDITION,
            "binary_endogenous_outcome_policy": "reject_exact_0_1_in_original_complete_sample_only",
            "semantic_probe_grid": {
                "assignment": "canonical_grid_index_primary_outer_conditioning_inner",
                "collapsed_reason": COLLAPSED_PROBE_GRID_REASON,
            },
            "hc3_instability_policy": {
                "minimum_one_minus_leverage_exclusive": HC3_LEVERAGE_STABILITY_TOLERANCE,
                "reason_code": HIGH_LEVERAGE_HC3_REASON,
                "clamp": False,
            },
            "hc3_covariance_policy": {
                "coefficient_variance": "finite_and_strictly_positive",
                "simple_slope_variance": "finite_and_strictly_positive",
                "absolute_value": False,
                "zero_clamp": False,
                "reason_codes": [INVALID_HC3_COVARIANCE_REASON, DEGENERATE_SIMPLE_SLOPE_REASON],
            },
            "quickpls_replicates": QUICKPLS_REPLICATES,
            "python_reference_replicates": INDEPENDENT_REPLICATES,
            "workers_compared": list(WORKERS),
            "capacity": {"top_level_predictors_maximum": 8, "controls_maximum": 1, "equation_non_intercept_terms_maximum": 50},
            "recipe_status": "candidate_regression_process_v2_plus_bootstrap_v1_bounded_scope",
        },
        "point_reference": {"maximum_absolute_difference": point_delta, "reference": independent},
        "point_metamorphic": metamorphic,
        "worker_invariance": worker_invariance,
        "numerical_boundaries": numerical_boundaries,
        "reference_condition": reference_condition,
        "bootstrap_exact_arithmetic": exact_bootstrap,
        "independent_python": {"comparison": python_comparison, "resampling": python_reference},
        "external_r": {
            **{key: value for key, value in r_result.items() if key != "data"},
            "point_comparison": r_point,
            "comparison": r_comparison,
            "numerical_boundaries": r_data.get("numerical_boundaries"),
            "reference_condition": r_data.get("reference_condition"),
        },
        "comparison_thresholds": COMPARISON_THRESHOLDS, "artifacts": artifacts,
        "runtime_boundary": "Python, NumPy, SciPy, and R are validation-only and are not bundled or required at runtime.",
    }
    REPORT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
