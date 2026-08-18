#!/usr/bin/env python3
"""Independent qualification reference for regression bootstrap v1.

The script never invokes Cargo. Point ``QUICKPLS_CLI_PATH`` at an already
built CLI produced by the coordinated release gate. NumPy, SciPy, R, and the
R ``boot``/``jsonlite`` packages are validation-only dependencies; none is a
QuickPLS runtime dependency.

Qualification has two deliberately separate layers:

* exact arithmetic is recomputed from the archive-only validation witness,
  including Type-7 percentile, midrank BCa, the standard-normal bootstrap
  ratio, and odds-ratio-scale intervals; and
* independently seeded Python and R case-resampling runs compare the OLS and
  logistic sampling distributions without assuming QuickPLS RNG identity.
"""

from __future__ import annotations

import csv
import hashlib
import json
import math
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Callable, Sequence

import numpy as np
from scipy.optimize import minimize
from scipy.stats import norm

from r_runtime import find_rscript_optional


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
FIXTURE = RESULTS / "v08_extended_methods_fixture.csv"
REPORT = RESULTS / "regression_bootstrap_v1_reference_report.json"
R_SCRIPT = ROOT / "validation" / "regression_bootstrap_v1_reference.R"
R_OUTPUT = RESULTS / "regression_bootstrap_v1_r_reference.json"

FEATURE_ID = "qpls3.standalone.regression_bootstrap"
METHOD_VERSION = "regression_bootstrap_v1"
WITNESS_VERSION = "regression_bootstrap_validation_witness_v1"
CATALOGUE_SNAPSHOT_DATE = "2026-08-12"
TARGET = "regression_bootstrap_v1_reference"
BOOTSTRAP_SAMPLES = 1_000
PYTHON_REFERENCE_SAMPLES = 2_000
SEED = 20_260_812
PYTHON_REFERENCE_SEED = 20_260_813
WORKERS = (1, 4)
CONFIDENCE_LEVEL = 0.95
EXACT_TOLERANCE = 5e-11
POINT_TOLERANCE = 2e-6
COMPARISON_THRESHOLDS = {
    # Thresholds are expressed in pooled bootstrap-SE units except the SE
    # ratio. They cover Monte-Carlo error from independent RNG streams while
    # remaining far smaller than a substantively different distribution.
    "mean_pooled_se_units": 0.35,
    "standard_error_relative": 0.30,
    "percentile_endpoint_pooled_se_units": 0.75,
    "bca_endpoint_pooled_se_units": 0.90,
    "odds_ratio_log_endpoint_pooled_se_units": 0.75,
}

TERMS = ["intercept", "x", "z", "w"]
MODELS = {
    "ols": {
        "outcome": "y",
        "base_method_version": "regression_ols_v1",
        "provenance_method_version": "regression_ols_v1+regression_bootstrap_v1",
    },
    "logistic": {
        "outcome": "bin_y",
        "base_method_version": "regression_logistic_v2",
        "provenance_method_version": "regression_logistic_v2+regression_bootstrap_v1",
    },
}

REFERENCE_CHECK_NAMES = frozenset(
    {
        "frozen_supplied_type7_bca_normal_ratio_or",
        "quickpls_ols_exact_contract",
        "quickpls_logistic_exact_contract",
        "exact_witness_arithmetic_ols",
        "exact_witness_arithmetic_logistic",
        "witness_index_partitions_ols",
        "witness_index_partitions_logistic",
        "independent_python_ols_resampling",
        "independent_python_logistic_resampling",
        "external_r_ols_resampling",
        "external_r_logistic_resampling",
        "deterministic_worker_invariant_ols",
        "deterministic_worker_invariant_logistic",
    }
)


def configured_cli() -> Path:
    configured = os.environ.get("QUICKPLS_CLI_PATH", "").strip()
    if configured:
        return Path(configured).resolve()
    release = ROOT / "target" / "release" / "qpls.exe"
    return release if release.is_file() else ROOT / "target" / "debug" / "qpls.exe"


CLI = configured_cli()


def require_cli() -> None:
    if not CLI.is_file():
        raise SystemExit(
            "QuickPLS CLI is not built. Build it in the coordinated serial gate "
            "or set QUICKPLS_CLI_PATH; this validation script will not start Cargo."
        )
    expected = (ROOT / "target" / "release" / "qpls.exe").resolve()
    if CLI != expected:
        raise SystemExit(
            f"Regression bootstrap qualification must execute the coordinated release CLI {expected}; received {CLI}."
        )


def tested_cli_artifact() -> dict[str, Any]:
    contents = CLI.read_bytes()
    return {
        "path": CLI.relative_to(ROOT).as_posix(),
        "size": len(contents),
        "sha256": hashlib.sha256(contents).hexdigest(),
    }


def run(command: list[str], *, timeout: int = 600) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )


def require_success(completed: subprocess.CompletedProcess[str], label: str) -> None:
    if completed.returncode != 0:
        raise RuntimeError(
            f"{label} failed with exit code {completed.returncode}\n"
            f"stdout={completed.stdout[-3000:]}\nstderr={completed.stderr[-3000:]}"
        )


def empty_model() -> dict[str, Any]:
    return {
        "id": "00000000-0000-0000-0000-000000031001",
        "name": "Regression bootstrap v1 reference",
        "constructs": [],
        "paths": [],
        "controls": [],
        "higher_order_constructs": [],
        "interactions": [],
    }


def recipe_payload(
    fingerprint: str,
    model: str,
    *,
    workers: int,
    samples: int = BOOTSTRAP_SAMPLES,
) -> dict[str, Any]:
    spec = MODELS[model]
    model_config = (
        {"type": "ols", "robust_se": "hc3"}
        if model == "ols"
        else {"type": "logistic"}
    )
    return {
        "schema_version": 3,
        "id": (
            "00000000-0000-0000-0000-"
            f"{3100 + (0 if model == 'ols' else 10) + workers:012d}"
        ),
        "created_at": "2026-08-12T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": empty_model(),
        "settings": {
            "method": "regression",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": samples,
            "studentized_inner_samples": 0,
            "permutation_samples": 0,
            "seed": SEED,
            "workers": workers,
            "confidence_level": CONFIDENCE_LEVEL,
            "preprocessing": "unstandardized",
            "missing_data": "listwise_deletion",
            "case_weight_column": None,
        },
        "method_config": {
            "kind": "regression",
            "outcome": spec["outcome"],
            "predictors": ["x", "z"],
            "controls": ["w"],
            "model": model_config,
            "bootstrap": {
                "algorithm": "case_resampling",
                "intervals": ["percentile", "bca"],
            },
        },
        "metadata": {
            "status": "validated_regression_bootstrap_v1_bounded_scope",
            "fixture": "regression_bootstrap_v1_reference",
        },
    }


def dataset_fingerprint(temporary: Path) -> str:
    project = temporary / "regression-bootstrap-fingerprint.qpls"
    imported = run([str(CLI), "import", str(FIXTURE), str(project), "--name", "Bootstrap reference"])
    require_success(imported, "fixture import")
    inspected = run([str(CLI), "inspect", str(project), "--json"])
    require_success(inspected, "fixture inspect")
    return json.loads(inspected.stdout)["datasets"][0]["fingerprint"]


def run_quickpls(
    temporary: Path,
    fingerprint: str,
    model: str,
    workers: int,
) -> tuple[dict[str, Any], dict[str, Any]]:
    payload = recipe_payload(fingerprint, model, workers=workers)
    recipe = temporary / f"regression-bootstrap-{model}-w{workers}.recipe.json"
    output = temporary / f"regression-bootstrap-{model}-w{workers}.result.json"
    recipe.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    completed = run(
        [
            str(CLI),
            "run",
            str(recipe),
            "--data",
            str(FIXTURE),
            "--output",
            str(output),
            "--allow-experimental",
        ]
    )
    require_success(completed, f"QuickPLS {model} bootstrap workers={workers}")
    if not output.is_file():
        raise RuntimeError(f"QuickPLS {model} did not create {output}")
    return payload, json.loads(output.read_text(encoding="utf-8"))


def load_complete_cases(outcome: str) -> tuple[np.ndarray, np.ndarray, int]:
    rows: list[list[float]] = []
    total = 0
    with FIXTURE.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            total += 1
            raw = [row.get(outcome), row.get("x"), row.get("z"), row.get("w")]
            if any(value is None or not value.strip() for value in raw):
                continue
            values = [float(value) for value in raw]
            if all(math.isfinite(value) for value in values):
                rows.append(values)
    matrix = np.asarray(rows, dtype=float)
    y = matrix[:, 0]
    design = np.column_stack([np.ones(len(matrix)), matrix[:, 1:]])
    return y, design, total - len(matrix)


def fit_ols(y: np.ndarray, design: np.ndarray) -> np.ndarray | None:
    try:
        beta, _, rank, _ = np.linalg.lstsq(design, y, rcond=None)
    except np.linalg.LinAlgError:
        return None
    if rank != design.shape[1] or not np.all(np.isfinite(beta)):
        return None
    return np.asarray(beta, dtype=float)


def fit_logistic(y: np.ndarray, design: np.ndarray) -> np.ndarray | None:
    if len(np.unique(y)) != 2:
        return None

    def objective(beta: np.ndarray) -> float:
        eta = np.clip(design @ beta, -40.0, 40.0)
        return float(np.sum(np.logaddexp(0.0, eta) - y * eta))

    def gradient(beta: np.ndarray) -> np.ndarray:
        eta = np.clip(design @ beta, -40.0, 40.0)
        probability = 1.0 / (1.0 + np.exp(-eta))
        return design.T @ (probability - y)

    fitted = minimize(
        objective,
        np.zeros(design.shape[1]),
        jac=gradient,
        method="BFGS",
        options={"gtol": 1e-9, "maxiter": 300},
    )
    if not fitted.success and np.max(np.abs(gradient(fitted.x))) > 1e-6:
        return None
    beta = np.asarray(fitted.x, dtype=float)
    eta = design @ beta
    probability = 1.0 / (1.0 + np.exp(-np.clip(eta, -40.0, 40.0)))
    if (
        not np.all(np.isfinite(beta))
        or np.max(np.abs(beta)) > 100.0
        or np.min(probability) <= 1e-10
        or np.max(probability) >= 1.0 - 1e-10
    ):
        return None
    return beta


def type7_quantile(values: Sequence[float], probability: float) -> float:
    if not values:
        raise ValueError("Type-7 quantile requires at least one value")
    ordered = sorted(float(value) for value in values)
    position = (len(ordered) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    return ordered[lower] + (position - lower) * (ordered[upper] - ordered[lower])


def bca_interval(
    bootstrap_values: Sequence[float],
    original: float,
    jackknife_values: Sequence[float],
    confidence_level: float = CONFIDENCE_LEVEL,
) -> dict[str, Any] | None:
    if len(bootstrap_values) < 2 or len(jackknife_values) < 3:
        return None
    values = [float(value) for value in bootstrap_values]
    jackknife = [float(value) for value in jackknife_values]
    below = sum(value < original for value in values)
    tied = sum(value == original for value in values)
    probability = (below + 0.5 * tied) / len(values)
    probability = min(max(probability, 0.5 / len(values)), 1.0 - 0.5 / len(values))
    bias_correction = float(norm.ppf(probability))
    jackknife_mean = sum(jackknife) / len(jackknife)
    centered = [jackknife_mean - value for value in jackknife]
    sum_squares = sum(value * value for value in centered)
    if not math.isfinite(sum_squares) or sum_squares <= sys.float_info.epsilon:
        return None
    acceleration = sum(value**3 for value in centered) / (6.0 * sum_squares**1.5)
    if not math.isfinite(acceleration):
        return None

    tail = (1.0 - confidence_level) / 2.0

    def adjusted_probability(nominal: float) -> float | None:
        z_value = float(norm.ppf(nominal))
        denominator = 1.0 - acceleration * (bias_correction + z_value)
        if not math.isfinite(denominator) or abs(denominator) <= sys.float_info.epsilon:
            return None
        adjusted = float(
            norm.cdf(
                bias_correction
                + (bias_correction + z_value) / denominator
            )
        )
        return min(max(adjusted, 0.0), 1.0) if math.isfinite(adjusted) else None

    lower_probability = adjusted_probability(tail)
    upper_probability = adjusted_probability(1.0 - tail)
    if (
        lower_probability is None
        or upper_probability is None
        or lower_probability > upper_probability
    ):
        return None
    return {
        "bias_correction": bias_correction,
        "acceleration": acceleration,
        "lower": type7_quantile(values, lower_probability),
        "upper": type7_quantile(values, upper_probability),
    }


def summarize_supplied(
    terms: Sequence[str],
    original: Sequence[float],
    bootstrap: Sequence[Sequence[float]],
    jackknife: Sequence[Sequence[float]],
    *,
    expected_jackknife_cases: int,
    logistic: bool,
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    jackknife_complete = len(jackknife) == expected_jackknife_cases
    tail = (1.0 - CONFIDENCE_LEVEL) / 2.0
    for index, term in enumerate(terms):
        values = sorted(float(row[index]) for row in bootstrap)
        point = float(original[index])
        mean = sum(values) / len(values)
        standard_error = math.sqrt(
            sum((value - mean) ** 2 for value in values) / (len(values) - 1)
        )
        replicate_max_abs = max(abs(value) for value in values)
        test_tolerance = (
            64.0
            * sys.float_info.epsilon
            * max(1.0, abs(point), replicate_max_abs)
        )
        if math.isfinite(standard_error) and standard_error > test_tolerance:
            statistic = point / standard_error
            test: dict[str, Any] = {
                "status": "available",
                "statistic": statistic,
                "p_value_two_sided": float(2.0 * norm.sf(abs(statistic))),
            }
        else:
            test = {
                "status": "unavailable",
                "reason_code": "degenerate_bootstrap_standard_error",
            }
        jackknife_values = [float(row[index]) for row in jackknife]
        if not jackknife_complete:
            bca: dict[str, Any] = {
                "status": "unavailable",
                "reason_code": "incomplete_jackknife",
            }
        else:
            calculated_bca = bca_interval(values, point, jackknife_values)
            bca = (
                {"status": "available", **calculated_bca}
                if calculated_bca is not None
                else {
                    "status": "unavailable",
                    "reason_code": "degenerate_jackknife_acceleration",
                }
            )
        odds_ratio = None
        if logistic:
            transformed = [math.exp(value) for value in values]
            transformed_jackknife = [math.exp(value) for value in jackknife_values]
            if not jackknife_complete:
                odds_bca: dict[str, Any] = {
                    "status": "unavailable",
                    "reason_code": "incomplete_jackknife",
                }
            else:
                calculated_odds_bca = bca_interval(
                    transformed,
                    math.exp(point),
                    transformed_jackknife,
                )
                odds_bca = (
                    {"status": "available", **calculated_odds_bca}
                    if calculated_odds_bca is not None
                    else {
                        "status": "unavailable",
                        "reason_code": "degenerate_jackknife_acceleration",
                    }
                )
            odds_ratio = {
                "original": math.exp(point),
                "percentile_lower": type7_quantile(transformed, tail),
                "percentile_upper": type7_quantile(transformed, 1.0 - tail),
                "bca": odds_bca,
            }
        rows.append(
            {
                "term": term,
                "original": point,
                "bootstrap_mean": mean,
                "bias": mean - point,
                "standard_error": standard_error,
                "replicate_max_abs": replicate_max_abs,
                "test_tolerance": test_tolerance,
                "test": test,
                "percentile_lower": type7_quantile(values, tail),
                "percentile_upper": type7_quantile(values, 1.0 - tail),
                "usable_replicates": len(values),
                "bca": bca,
                "odds_ratio": odds_ratio,
            }
        )
    return rows


FROZEN_SUPPLIED = {
    "terms": ["intercept", "x"],
    "original": [1.0, -0.5],
    "bootstrap": [[0.8, -0.6], [1.1, -0.4], [0.9, -0.55], [1.2, -0.45]],
    "jackknife": [[0.95, -0.52], [1.05, -0.48], [0.98, -0.51], [1.02, -0.49]],
}

FROZEN_EXPECTED = [
    {
        "bootstrap_mean": 1.0,
        "standard_error": 0.18257418583505536,
        "percentile_lower": 0.8075,
        "percentile_upper": 1.1925,
        "statistic": 5.477225575051661,
        "p_value_two_sided": 4.320463057827508e-08,
        "bca_lower": 0.8075000000000001,
        "bca_upper": 1.1925,
        "odds_ratio_percentile_lower": 2.243095592192304,
        "odds_ratio_percentile_upper": 3.2964206053272886,
        "odds_ratio_bca_lower": 2.2427039555816957,
        "odds_ratio_bca_upper": 3.295887168113359,
    },
    {
        "bootstrap_mean": -0.5,
        "standard_error": 0.09128709291752768,
        "percentile_lower": -0.59625,
        "percentile_upper": -0.40375000000000005,
        "statistic": -5.477225575051661,
        "p_value_two_sided": 4.320463057827508e-08,
        "bca_lower": -0.59625,
        "bca_upper": -0.40375000000000005,
        "odds_ratio_percentile_lower": 0.550921999165511,
        "odds_ratio_percentile_upper": 0.6678681539545994,
        "odds_ratio_bca_lower": 0.5509085316685839,
        "odds_ratio_bca_upper": 0.6678524667223256,
    },
]


def frozen_supplied_check() -> dict[str, Any]:
    rows = summarize_supplied(
        FROZEN_SUPPLIED["terms"],
        FROZEN_SUPPLIED["original"],
        FROZEN_SUPPLIED["bootstrap"],
        FROZEN_SUPPLIED["jackknife"],
        expected_jackknife_cases=len(FROZEN_SUPPLIED["jackknife"]),
        logistic=True,
    )
    differences: list[float] = []
    for row, expected in zip(rows, FROZEN_EXPECTED, strict=True):
        observed = {
            "bootstrap_mean": row["bootstrap_mean"],
            "standard_error": row["standard_error"],
            "percentile_lower": row["percentile_lower"],
            "percentile_upper": row["percentile_upper"],
            "statistic": row["test"]["statistic"],
            "p_value_two_sided": row["test"]["p_value_two_sided"],
            "bca_lower": row["bca"]["lower"],
            "bca_upper": row["bca"]["upper"],
            "odds_ratio_percentile_lower": row["odds_ratio"]["percentile_lower"],
            "odds_ratio_percentile_upper": row["odds_ratio"]["percentile_upper"],
            "odds_ratio_bca_lower": row["odds_ratio"]["bca"]["lower"],
            "odds_ratio_bca_upper": row["odds_ratio"]["bca"]["upper"],
        }
        differences.extend(abs(observed[key] - expected[key]) for key in expected)

    degenerate = summarize_supplied(
        ["x"],
        [2.0],
        [[2.0], [2.0], [2.0]],
        [[2.0], [2.0], [2.0]],
        expected_jackknife_cases=3,
        logistic=False,
    )[0]
    incomplete = summarize_supplied(
        ["x"],
        [1.0],
        [[0.8], [1.0], [1.2]],
        [[0.9], [1.0], [1.1]],
        expected_jackknife_cases=4,
        logistic=False,
    )[0]
    maximum = max(differences, default=0.0)
    passed = (
        maximum <= EXACT_TOLERANCE
        and degenerate["test"]["status"] == "unavailable"
        and degenerate["test"]["reason_code"]
        == "degenerate_bootstrap_standard_error"
        and degenerate["bca"]["status"] == "unavailable"
        and degenerate["bca"]["reason_code"]
        == "degenerate_jackknife_acceleration"
        and incomplete["bca"]["status"] == "unavailable"
        and incomplete["bca"]["reason_code"] == "incomplete_jackknife"
    )
    return {
        "passed": passed,
        "maximum_absolute_difference": maximum,
        "tolerance": EXACT_TOLERANCE,
        "rows": rows,
        "degenerate_status": {
            "test": degenerate["test"],
            "bca": degenerate["bca"],
        },
        "incomplete_jackknife_status": incomplete["bca"],
    }


def validate_witness_partition(bootstrap: dict[str, Any]) -> dict[str, Any]:
    witness = bootstrap.get("validation_witness", {})
    successful = witness.get("successful_bootstrap", [])
    failed = bootstrap.get("failed_replicates", [])
    successful_jackknife = witness.get("successful_jackknife", [])
    failed_jackknife = witness.get("failed_jackknife", [])
    bootstrap_indices = [row.get("replicate_index") for row in successful]
    failed_indices = [row.get("replicate_index") for row in failed]
    jackknife_indices = [row.get("omitted_case") for row in successful_jackknife]
    failed_jackknife_indices = [row.get("omitted_case") for row in failed_jackknife]
    terms = witness.get("terms")
    width = len(terms) if isinstance(terms, list) else -1

    def strictly_ascending(values: Sequence[Any]) -> bool:
        return all(
            isinstance(value, int) and not isinstance(value, bool)
            for value in values
        ) and all(first < second for first, second in zip(values, values[1:]))

    vectors_valid = all(
        isinstance(row.get("coefficients"), list)
        and len(row["coefficients"]) == width
        and all(
            isinstance(value, (int, float))
            and not isinstance(value, bool)
            and math.isfinite(float(value))
            for value in row["coefficients"]
        )
        for row in [*successful, *successful_jackknife]
    )
    bootstrap_partition = sorted([*bootstrap_indices, *failed_indices])
    jackknife_partition = sorted([*jackknife_indices, *failed_jackknife_indices])
    passed = (
        witness.get("method_version") == WITNESS_VERSION
        and terms == TERMS
        and 1 <= width <= 51
        and len(successful) == bootstrap.get("usable_replicates")
        and len(successful_jackknife) == bootstrap.get("usable_jackknife_cases")
        and strictly_ascending(bootstrap_indices)
        and strictly_ascending(failed_indices)
        and strictly_ascending(jackknife_indices)
        and strictly_ascending(failed_jackknife_indices)
        and bootstrap_partition
        == list(range(int(bootstrap.get("requested_replicates", -1))))
        and jackknife_partition
        == list(range(int(bootstrap.get("jackknife_cases", -1))))
        and vectors_valid
        and all(
            isinstance(row.get("reason_code"), str)
            and bool(row["reason_code"].strip())
            and isinstance(row.get("message"), str)
            and bool(row["message"].strip())
            for row in [*failed, *failed_jackknife]
        )
    )
    return {
        "passed": passed,
        "witness_version": witness.get("method_version"),
        "terms": terms,
        "bootstrap_successes": len(successful),
        "bootstrap_failures": len(failed),
        "bootstrap_partition_size": len(bootstrap_partition),
        "jackknife_successes": len(successful_jackknife),
        "jackknife_failures": len(failed_jackknife),
        "jackknife_partition_size": len(jackknife_partition),
        "coefficient_vectors_valid": vectors_valid,
    }


def _compare_bca(
    actual: dict[str, Any],
    expected: dict[str, Any],
    differences: list[float],
) -> bool:
    if actual.get("status") != expected.get("status"):
        return False
    if expected.get("status") == "available":
        for field in ("bias_correction", "acceleration", "lower", "upper"):
            value = actual.get(field)
            reference = expected.get(field)
            if not isinstance(value, (int, float)) or not isinstance(
                reference, (int, float)
            ):
                return False
            differences.append(abs(float(value) - float(reference)))
        return True
    return actual.get("reason_code") == expected.get("reason_code")


def exact_witness_arithmetic(
    regression: dict[str, Any],
    *,
    logistic: bool,
) -> dict[str, Any]:
    bootstrap = regression.get("bootstrap", {})
    witness = bootstrap.get("validation_witness", {})
    supplied_bootstrap = [row["coefficients"] for row in witness.get("successful_bootstrap", [])]
    supplied_jackknife = [row["coefficients"] for row in witness.get("successful_jackknife", [])]
    original = [row["estimate"] for row in regression.get("coefficients", [])]
    expected = summarize_supplied(
        witness.get("terms", []),
        original,
        supplied_bootstrap,
        supplied_jackknife,
        expected_jackknife_cases=bootstrap.get("jackknife_cases", 0),
        logistic=logistic,
    )
    actual = bootstrap.get("coefficients", [])
    differences: list[float] = []
    identities_valid = len(actual) == len(expected)
    for observed, reference in zip(actual, expected):
        identities_valid = identities_valid and observed.get("term") == reference["term"]
        for field in (
            "original",
            "bootstrap_mean",
            "bias",
            "standard_error",
            "replicate_max_abs",
            "test_tolerance",
            "percentile_lower",
            "percentile_upper",
        ):
            differences.append(abs(float(observed[field]) - float(reference[field])))
        identities_valid = (
            identities_valid
            and observed.get("usable_replicates") == reference["usable_replicates"]
            and observed.get("test", {}).get("status")
            == reference["test"]["status"]
        )
        if reference["test"]["status"] == "available":
            differences.extend(
                [
                    abs(
                        float(observed["test"]["statistic"])
                        - float(reference["test"]["statistic"])
                    ),
                    abs(
                        float(observed["test"]["p_value_two_sided"])
                        - float(reference["test"]["p_value_two_sided"])
                    ),
                ]
            )
        else:
            identities_valid = (
                identities_valid
                and observed["test"].get("reason_code")
                == reference["test"].get("reason_code")
            )
        identities_valid = identities_valid and _compare_bca(
            observed.get("bca", {}), reference["bca"], differences
        )
        if logistic:
            observed_odds = observed.get("odds_ratio")
            reference_odds = reference["odds_ratio"]
            if not isinstance(observed_odds, dict) or not isinstance(reference_odds, dict):
                identities_valid = False
            else:
                for field in ("original", "percentile_lower", "percentile_upper"):
                    differences.append(
                        abs(float(observed_odds[field]) - float(reference_odds[field]))
                    )
                identities_valid = identities_valid and _compare_bca(
                    observed_odds.get("bca", {}),
                    reference_odds["bca"],
                    differences,
                )
        else:
            identities_valid = identities_valid and observed.get("odds_ratio") is None
    maximum = max(differences, default=math.inf)
    return {
        "passed": identities_valid and maximum <= EXACT_TOLERANCE,
        "identities_valid": identities_valid,
        "maximum_absolute_difference": maximum,
        "tolerance": EXACT_TOLERANCE,
    }


def exact_quickpls_contract(
    result: dict[str, Any],
    recipe: dict[str, Any],
    model: str,
) -> dict[str, Any]:
    spec = MODELS[model]
    provenance = result.get("provenance", {})
    estimation = result.get("payload", {}).get("estimation", {})
    regression = estimation.get("regression", {})
    bootstrap = regression.get("bootstrap", {})
    expected_model = (
        {"type": "ols", "robust_se": "hc3"}
        if model == "ols"
        else {"type": "logistic"}
    )
    warnings = bootstrap.get("warnings", [])
    contract = {
        "completed": result.get("status") == "completed",
        "provenance": (
            provenance.get("method") == "regression"
            and provenance.get("method_version") == spec["provenance_method_version"]
            and provenance.get("seed") == SEED
            and provenance.get("settings", {}).get("bootstrap_samples")
            == BOOTSTRAP_SAMPLES
        ),
        "nested_versions": (
            estimation.get("method_version") == spec["base_method_version"]
            and regression.get("method_version") == spec["base_method_version"]
            and bootstrap.get("method_version") == METHOD_VERSION
        ),
        "regression_identity": (
            regression.get("regression_type") == model
            and regression.get("outcome") == spec["outcome"]
            and regression.get("predictors") == ["x", "z"]
            and regression.get("controls") == ["w"]
            and [row.get("term") for row in regression.get("coefficients", [])]
            == TERMS
        ),
        "bootstrap_identity": (
            bootstrap.get("algorithm") == "indexed_case_resampling_v1"
            and bootstrap.get("stream_token") == "quickpls_indexed_resampling_v1"
            and bootstrap.get("interval_policy")
            == "percentile_primary_bca_conditional_v1"
            and bootstrap.get("test_reference")
            == "standard_normal_bootstrap_ratio_v1"
            and bootstrap.get("test_tolerance_policy")
            == "64eps_max_1_original_replicates_v1"
            and bootstrap.get("alternative") == "two_sided"
            and bootstrap.get("confidence_level") == CONFIDENCE_LEVEL
            and bootstrap.get("requested_replicates") == BOOTSTRAP_SAMPLES
            and bootstrap.get("minimum_usable_fraction") == 0.9
            and bootstrap.get("seed") == SEED
            and bootstrap.get("workers") == recipe["settings"]["workers"]
            and bootstrap.get("usable_replicates", 0)
            + len(bootstrap.get("failed_replicates", []))
            == BOOTSTRAP_SAMPLES
            and bootstrap.get("usable_replicates", 0) >= math.ceil(0.9 * BOOTSTRAP_SAMPLES)
        ),
        "recipe": (
            recipe.get("schema_version") == 3
            and recipe.get("settings", {}).get("method") == "regression"
            and recipe.get("settings", {}).get("preprocessing") == "unstandardized"
            and recipe.get("settings", {}).get("missing_data") == "listwise_deletion"
            and recipe.get("settings", {}).get("confidence_level") == CONFIDENCE_LEVEL
            and recipe.get("method_config")
            == {
                "kind": "regression",
                "outcome": spec["outcome"],
                "predictors": ["x", "z"],
                "controls": ["w"],
                "model": expected_model,
                "bootstrap": {
                    "algorithm": "case_resampling",
                    "intervals": ["percentile", "bca"],
                },
            }
            and recipe.get("metadata", {}).get("status")
            == "validated_regression_bootstrap_v1_bounded_scope"
        ),
        "warnings": (
            len(warnings) >= 2
            and "deterministic indexed case resampling" in warnings[0]
            and "two-sided standard-normal reference" in warnings[1]
        ),
        "assessment_not_applicable": result.get("payload", {})
        .get("assessment", {})
        .get("method_version")
        == "assessment_not_applicable_v1",
        "generic_pls_bootstrap_absent": result.get("payload", {}).get("bootstrap") is None,
    }
    return {"passed": all(contract.values()), "checks": contract}


def normalized_worker_scientific_payload(result: dict[str, Any]) -> dict[str, Any]:
    regression = json.loads(
        json.dumps(result["payload"]["estimation"]["regression"])
    )
    regression["bootstrap"].pop("workers", None)
    return regression


def independent_resampling(
    y: np.ndarray,
    design: np.ndarray,
    fit: Callable[[np.ndarray, np.ndarray], np.ndarray | None],
    *,
    samples: int,
    seed: int,
    logistic: bool,
) -> dict[str, Any]:
    original = fit(y, design)
    if original is None:
        raise RuntimeError("independent point fit failed")
    rng = np.random.Generator(np.random.PCG64(seed))
    replicates: list[list[float]] = []
    failures = 0
    for _ in range(samples):
        indices = rng.integers(0, len(y), len(y), endpoint=False)
        estimate = fit(y[indices], design[indices])
        if estimate is None:
            failures += 1
        else:
            replicates.append(estimate.tolist())
    jackknife: list[list[float]] = []
    jackknife_failures = 0
    for omitted in range(len(y)):
        keep = np.arange(len(y)) != omitted
        estimate = fit(y[keep], design[keep])
        if estimate is None:
            jackknife_failures += 1
        else:
            jackknife.append(estimate.tolist())
    if len(replicates) < math.ceil(0.9 * samples):
        raise RuntimeError(
            f"independent {('logistic' if logistic else 'OLS')} reference had "
            f"only {len(replicates)} usable replicates"
        )
    rows = summarize_supplied(
        TERMS,
        original.tolist(),
        replicates,
        jackknife,
        expected_jackknife_cases=len(y),
        logistic=logistic,
    )
    return {
        "algorithm": "numpy_pcg64_independent_case_resampling_with_replacement_v1",
        "seed": seed,
        "requested_replicates": samples,
        "usable_replicates": len(replicates),
        "failed_replicates": failures,
        "jackknife_cases": len(y),
        "failed_jackknife": jackknife_failures,
        "original": original.tolist(),
        "coefficients": rows,
    }


def compare_distributions(
    quickpls: dict[str, Any],
    reference: dict[str, Any],
    *,
    logistic: bool,
) -> dict[str, Any]:
    quickpls_rows = {row["term"]: row for row in quickpls["coefficients"]}
    reference_rows = {row["term"]: row for row in reference["coefficients"]}
    metrics: dict[str, list[float]] = {
        "mean_pooled_se_units": [],
        "standard_error_relative": [],
        "percentile_endpoint_pooled_se_units": [],
        "bca_endpoint_pooled_se_units": [],
        "odds_ratio_log_endpoint_pooled_se_units": [],
    }
    term_rows: list[dict[str, Any]] = []
    identities_valid = set(quickpls_rows) == set(reference_rows) == set(TERMS)
    for term in TERMS:
        observed = quickpls_rows[term]
        expected = reference_rows[term]
        pooled = max(
            1e-12,
            math.sqrt(
                (float(observed["standard_error"]) ** 2
                + float(expected["standard_error"]) ** 2)
                / 2.0
            ),
        )
        mean_units = abs(
            float(observed["bootstrap_mean"])
            - float(expected["bootstrap_mean"])
        ) / pooled
        se_relative = abs(
            float(observed["standard_error"])
            - float(expected["standard_error"])
        ) / pooled
        percentile_units = max(
            abs(
                float(observed["percentile_lower"])
                - float(expected["percentile_lower"])
            ),
            abs(
                float(observed["percentile_upper"])
                - float(expected["percentile_upper"])
            ),
        ) / pooled
        metrics["mean_pooled_se_units"].append(mean_units)
        metrics["standard_error_relative"].append(se_relative)
        metrics["percentile_endpoint_pooled_se_units"].append(percentile_units)

        bca_units = 0.0
        if (
            observed.get("bca", {}).get("status") == "available"
            and expected.get("bca", {}).get("status") == "available"
        ):
            bca_units = max(
                abs(float(observed["bca"]["lower"]) - float(expected["bca"]["lower"])),
                abs(float(observed["bca"]["upper"]) - float(expected["bca"]["upper"])),
            ) / pooled
        else:
            identities_valid = False
            bca_units = math.inf
        metrics["bca_endpoint_pooled_se_units"].append(bca_units)

        odds_units = 0.0
        if logistic:
            observed_odds = observed.get("odds_ratio")
            expected_odds = expected.get("odds_ratio")
            if not isinstance(observed_odds, dict) or not isinstance(expected_odds, dict):
                identities_valid = False
                odds_units = math.inf
            else:
                odds_units = max(
                    abs(
                        math.log(float(observed_odds["percentile_lower"]))
                        - math.log(float(expected_odds["percentile_lower"]))
                    ),
                    abs(
                        math.log(float(observed_odds["percentile_upper"]))
                        - math.log(float(expected_odds["percentile_upper"]))
                    ),
                ) / pooled
            metrics["odds_ratio_log_endpoint_pooled_se_units"].append(odds_units)
        term_rows.append(
            {
                "term": term,
                "mean_pooled_se_units": mean_units,
                "standard_error_relative": se_relative,
                "percentile_endpoint_pooled_se_units": percentile_units,
                "bca_endpoint_pooled_se_units": bca_units,
                **(
                    {"odds_ratio_log_endpoint_pooled_se_units": odds_units}
                    if logistic
                    else {}
                ),
            }
        )

    observed_maxima = {
        name: max(values, default=0.0)
        for name, values in metrics.items()
        if values
    }
    threshold_checks = {
        name: observed_maxima.get(name, 0.0) <= threshold
        for name, threshold in COMPARISON_THRESHOLDS.items()
        if logistic or name != "odds_ratio_log_endpoint_pooled_se_units"
    }
    return {
        "passed": identities_valid and all(threshold_checks.values()),
        "identities_valid": identities_valid,
        "thresholds": {
            name: threshold
            for name, threshold in COMPARISON_THRESHOLDS.items()
            if logistic or name != "odds_ratio_log_endpoint_pooled_se_units"
        },
        "observed_maxima": observed_maxima,
        "threshold_checks": threshold_checks,
        "terms": term_rows,
    }


def r_reference(samples: int) -> dict[str, Any]:
    found = find_rscript_optional()
    if found is None:
        return {"available": False, "passed": False, "reason": "Rscript not found"}
    rscript, version = found
    completed = run(
        [rscript, str(R_SCRIPT), str(FIXTURE), str(R_OUTPUT), str(samples)],
        timeout=900,
    )
    if completed.returncode != 0 or not R_OUTPUT.is_file():
        return {
            "available": True,
            "passed": False,
            "version": version,
            "stderr_tail": completed.stderr[-2000:],
        }
    payload = json.loads(R_OUTPUT.read_text(encoding="utf-8"))
    return {
        "available": True,
        "passed": payload.get("passed") is True,
        "version": version,
        "data": payload,
    }


def r_distribution(model_payload: dict[str, Any]) -> dict[str, Any]:
    rows = []
    for row in model_payload["coefficients"]:
        bca = row.get("bca")
        odds = row.get("odds_ratio")
        rows.append(
            {
                "term": row["term"],
                "original": row["original"],
                "bootstrap_mean": row["bootstrap_mean"],
                "standard_error": row["standard_error"],
                "percentile_lower": row["percentile_lower"],
                "percentile_upper": row["percentile_upper"],
                "bca": (
                    {"status": "available", **bca}
                    if isinstance(bca, dict) and bca.get("available") is True
                    else {"status": "unavailable"}
                ),
                "odds_ratio": (
                    {
                        "percentile_lower": odds["percentile_lower"],
                        "percentile_upper": odds["percentile_upper"],
                    }
                    if isinstance(odds, dict)
                    else None
                ),
            }
        )
    return {"coefficients": rows}


def maximum_point_delta(regression: dict[str, Any], reference: dict[str, Any]) -> float:
    by_term = {row["term"]: row["estimate"] for row in regression["coefficients"]}
    return max(
        abs(float(by_term[term]) - float(reference["original"][index]))
        for index, term in enumerate(TERMS)
    )


def write_compact_artifacts(
    model: str,
    recipe: dict[str, Any],
    result: dict[str, Any],
) -> dict[str, str]:
    recipe_path = RESULTS / f"regression_bootstrap_v1_{model}.recipe.json"
    result_path = RESULTS / f"regression_bootstrap_v1_{model}_quickpls.json"
    recipe_path.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")
    result_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return {
        "recipe": recipe_path.relative_to(ROOT).as_posix(),
        "quickpls_result": result_path.relative_to(ROOT).as_posix(),
    }


def main() -> int:
    require_cli()
    RESULTS.mkdir(parents=True, exist_ok=True)
    if not FIXTURE.is_file():
        raise SystemExit(f"Required frozen fixture is missing: {FIXTURE}")

    supplied = frozen_supplied_check()
    with tempfile.TemporaryDirectory(prefix="quickpls-regression-bootstrap-v1-") as directory:
        temporary = Path(directory)
        fingerprint = dataset_fingerprint(temporary)
        quickpls_runs: dict[str, dict[int, dict[str, Any]]] = {}
        recipes: dict[str, dict[int, dict[str, Any]]] = {}
        for model in MODELS:
            quickpls_runs[model] = {}
            recipes[model] = {}
            for workers in WORKERS:
                recipe, result = run_quickpls(temporary, fingerprint, model, workers)
                recipes[model][workers] = recipe
                quickpls_runs[model][workers] = result

    references: dict[str, dict[str, Any]] = {}
    exact_contracts: dict[str, dict[str, Any]] = {}
    partitions: dict[str, dict[str, Any]] = {}
    exact_arithmetic: dict[str, dict[str, Any]] = {}
    python_comparisons: dict[str, dict[str, Any]] = {}
    worker_invariance: dict[str, bool] = {}
    point_deltas: dict[str, float] = {}
    artifacts: dict[str, Any] = {
        "fixture": FIXTURE.relative_to(ROOT).as_posix(),
        "r_script": R_SCRIPT.relative_to(ROOT).as_posix(),
        "tested_cli": tested_cli_artifact(),
    }
    for index, model in enumerate(MODELS):
        result = quickpls_runs[model][1]
        regression = result["payload"]["estimation"]["regression"]
        bootstrap = regression["bootstrap"]
        logistic = model == "logistic"
        y, design, omitted = load_complete_cases(MODELS[model]["outcome"])
        reference = independent_resampling(
            y,
            design,
            fit_logistic if logistic else fit_ols,
            samples=PYTHON_REFERENCE_SAMPLES,
            seed=PYTHON_REFERENCE_SEED + index,
            logistic=logistic,
        )
        reference["omitted_cases"] = omitted
        references[model] = reference
        exact_contracts[model] = exact_quickpls_contract(
            result, recipes[model][1], model
        )
        partitions[model] = validate_witness_partition(bootstrap)
        exact_arithmetic[model] = exact_witness_arithmetic(
            regression, logistic=logistic
        )
        python_comparisons[model] = compare_distributions(
            bootstrap, reference, logistic=logistic
        )
        worker_invariance[model] = normalized_worker_scientific_payload(
            quickpls_runs[model][1]
        ) == normalized_worker_scientific_payload(quickpls_runs[model][4])
        point_deltas[model] = maximum_point_delta(regression, reference)
        artifacts[model] = write_compact_artifacts(
            model, recipes[model][1], result
        )

    r_result = r_reference(BOOTSTRAP_SAMPLES)
    r_comparisons: dict[str, dict[str, Any]] = {}
    if r_result.get("passed") is True:
        for model in MODELS:
            bootstrap = quickpls_runs[model][1]["payload"]["estimation"]["regression"]["bootstrap"]
            r_comparisons[model] = compare_distributions(
                bootstrap,
                r_distribution(r_result["data"][model]),
                logistic=model == "logistic",
            )
    else:
        r_comparisons = {
            model: {"passed": False, "reason": r_result.get("reason", "R reference failed")}
            for model in MODELS
        }

    checks = {
        "frozen_supplied_type7_bca_normal_ratio_or": supplied["passed"],
        "quickpls_ols_exact_contract": exact_contracts["ols"]["passed"],
        "quickpls_logistic_exact_contract": exact_contracts["logistic"]["passed"],
        "exact_witness_arithmetic_ols": exact_arithmetic["ols"]["passed"],
        "exact_witness_arithmetic_logistic": exact_arithmetic["logistic"]["passed"],
        "witness_index_partitions_ols": partitions["ols"]["passed"],
        "witness_index_partitions_logistic": partitions["logistic"]["passed"],
        "independent_python_ols_resampling": (
            python_comparisons["ols"]["passed"]
            and point_deltas["ols"] <= POINT_TOLERANCE
        ),
        "independent_python_logistic_resampling": (
            python_comparisons["logistic"]["passed"]
            and point_deltas["logistic"] <= POINT_TOLERANCE
        ),
        "external_r_ols_resampling": r_comparisons["ols"]["passed"],
        "external_r_logistic_resampling": r_comparisons["logistic"]["passed"],
        "deterministic_worker_invariant_ols": worker_invariance["ols"],
        "deterministic_worker_invariant_logistic": worker_invariance["logistic"],
    }
    if frozenset(checks) != REFERENCE_CHECK_NAMES:
        raise RuntimeError("reference check identity drifted")

    maximum_exact_delta = max(
        supplied["maximum_absolute_difference"],
        exact_arithmetic["ols"]["maximum_absolute_difference"],
        exact_arithmetic["logistic"]["maximum_absolute_difference"],
    )
    report = {
        "schema_version": 1,
        "target": TARGET,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "passed": all(checks.values()),
        "checks": checks,
        "scope": {
            "base_methods": ["regression_ols_v1", "regression_logistic_v2"],
            "algorithm": "case_resampling_with_replacement",
            "intervals": ["percentile_type7_primary", "bca_midrank_conditional"],
            "test_reference": "standard_normal_bootstrap_ratio_v1",
            "alternative": "two_sided",
            "confidence_level": CONFIDENCE_LEVEL,
            "quickpls_replicates": BOOTSTRAP_SAMPLES,
            "python_reference_replicates": PYTHON_REFERENCE_SAMPLES,
            "workers_compared": list(WORKERS),
            "missing_data": "listwise_deletion",
            "term_limit_including_intercept": 51,
            "process_bootstrap": False,
            "studentized_or_custom_alpha": False,
        },
        "exact_arithmetic": {
            "tolerance": EXACT_TOLERANCE,
            "maximum_absolute_difference": maximum_exact_delta,
            "frozen_supplied": supplied,
            "ols": exact_arithmetic["ols"],
            "logistic": exact_arithmetic["logistic"],
        },
        "quickpls_contracts": exact_contracts,
        "witness_partitions": partitions,
        "worker_invariance": worker_invariance,
        "independent_python": {
            "implementation": "NumPy PCG64 indices with independent NumPy OLS and SciPy BFGS logistic fits",
            "point_tolerance": POINT_TOLERANCE,
            "point_maximum_absolute_difference": point_deltas,
            "distribution_comparisons": python_comparisons,
            "reference_runs": references,
        },
        "external_r": {
            **{key: value for key, value in r_result.items() if key != "data"},
            "distribution_comparisons": r_comparisons,
            "artifact": R_OUTPUT.relative_to(ROOT).as_posix()
            if R_OUTPUT.is_file()
            else None,
        },
        "comparison_thresholds": COMPARISON_THRESHOLDS,
        "artifacts": artifacts,
        "runtime_boundary": (
            "Python, NumPy, SciPy, R, boot, and jsonlite are validation-only "
            "references and are not bundled or required by QuickPLS."
        ),
    }
    REPORT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
