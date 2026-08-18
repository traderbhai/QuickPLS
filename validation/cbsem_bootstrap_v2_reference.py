"""Independent Python and lavaan full-refit references for CB-SEM bootstrap v2.

The Python reference implements the fixed three-factor recursive SEM covariance
likelihood directly with NumPy/SciPy and never calls QuickPLS or lavaan for a
refit. A second independent stream uses lavaan. Exact checks cover the QuickPLS
witness/accounting/Type-7 summary; distributional checks compare bootstrap
standard errors and percentile intervals from independent full ML refits. The
existing cbsem_ml_v1 lavaan gate continues to provide same-data point-estimate
parity at 1e-6 tolerances.
"""

from __future__ import annotations

import argparse
import json
import math
import subprocess
import tempfile
import textwrap
from pathlib import Path
from typing import Any

import cbsem_lavaan_reference as baseline
import numpy as np
from scipy.optimize import minimize


ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "target" / "debug" / "qpls.exe"
RESULT = ROOT / "validation" / "results" / "cbsem_bootstrap_v2_reference.json"
METHOD_VERSION = "cbsem_bootstrap_v2"
INTERVAL_METHOD = "percentile_type7_v1"
MINIMUM_REPLICATES = 1_000
PYTHON_REFERENCE_REPLICATES = 199
PYTHON_REFERENCE_SEED = 2026081403
PYTHON_MINIMUM_USABLE_FRACTION = 0.90
PYTHON_POINT_ABSOLUTE_TOLERANCE = 5e-4
PYTHON_SE_RELATIVE_TOLERANCE = 0.30
PYTHON_INTERVAL_ABSOLUTE_TOLERANCE = 0.30
SE_RELATIVE_TOLERANCE = 0.20
INTERVAL_ABSOLUTE_TOLERANCE = 0.20


def strict_json(path: Path) -> dict[str, Any]:
    return json.loads(
        path.read_text(encoding="utf-8"),
        parse_constant=lambda value: (_ for _ in ()).throw(
            ValueError(f"non-finite JSON token: {value}")
        ),
    )


def qpls(args: list[str]) -> subprocess.CompletedProcess[str]:
    if not CLI.is_file():
        raise FileNotFoundError("build the coordinated source-frozen debug qpls.exe first")
    return subprocess.run(
        [str(CLI), *args],
        cwd=ROOT,
        check=True,
        text=True,
        capture_output=True,
    )


def type7(values: list[float], probability: float) -> float:
    ordered = sorted(values)
    position = probability * (len(ordered) - 1)
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    fraction = position - lower
    return ordered[lower] + fraction * (ordered[upper] - ordered[lower])


def bootstrap_recipe(fingerprint: str, replicates: int, workers: int) -> dict[str, Any]:
    model = "latent_regression_sem"
    return {
        "schema_version": 3,
        "id": "00000000-0000-0000-0000-000000252501",
        "created_at": "2026-08-14T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000252502",
            "name": "CB-SEM bootstrap v2 independent reference",
            "constructs": baseline.constructs_for(model),
            "paths": baseline.paths_for(model),
            "controls": [],
            "higher_order_constructs": [],
            "interactions": [],
        },
        "settings": {
            "method": "cbsem",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3_000,
            "bootstrap_samples": 0,
            "studentized_inner_samples": 0,
            "permutation_samples": 0,
            "seed": 2026081401,
            "workers": workers,
            "confidence_level": 0.95,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
            "case_weight_column": None,
        },
        "method_config": {
            "kind": "cbsem",
            "model_type": "sem",
            "estimator": "ml",
            "input": "raw",
            "mean_structure": False,
            "bootstrap_samples": replicates,
        },
        "metadata": {"status": "cbsem_bootstrap_v2_independent_reference"},
    }


def run_quickpls(work: Path, replicates: int, workers: int) -> tuple[dict[str, Any], Path]:
    data = ROOT / "validation" / "results" / "lavaan_latent_regression_sem.csv"
    baseline.write_fixture(data, "latent_regression_sem")
    project = work / "bootstrap-reference.qpls"
    qpls(["import", str(data), str(project), "--name", "cbsem-bootstrap-reference"])
    inspected = json.loads(qpls(["inspect", str(project), "--json"]).stdout)
    fingerprint = inspected["datasets"][0]["fingerprint"]
    recipe = work / "bootstrap-reference.recipe.json"
    output = work / "bootstrap-reference.result.json"
    recipe.write_text(
        json.dumps(bootstrap_recipe(fingerprint, replicates, workers), indent=2) + "\n",
        encoding="utf-8",
    )
    qpls(
        [
            "run",
            str(recipe),
            "--data",
            str(data),
            "--output",
            str(output),
            "--allow-experimental",
        ]
    )
    result = strict_json(output)
    cbsem = result["payload"]["estimation"]["cbsem"]
    if cbsem.get("bootstrap") is not None:
        raise AssertionError("legacy cbsem_bootstrap_v1 appeared in a v2 result")
    return cbsem["bootstrap_v2"], data


def validate_quickpls_witness(payload: dict[str, Any]) -> dict[str, Any]:
    witness = payload["validation_witness"]
    names = witness["parameter_names"]
    rows = witness["successful_replicates"]
    intervals = payload["intervals"]
    confidence = float(payload["confidence_level"])
    alpha = (1.0 - confidence) / 2.0
    exact_rows = []
    for index, name in enumerate(names):
        values = [float(row["parameter_estimates"][index]) for row in rows]
        actual = next(row for row in intervals if row["parameter"] == name)
        mean = sum(values) / len(values)
        standard_error = math.sqrt(
            sum((value - mean) ** 2 for value in values) / (len(values) - 1)
        )
        checks = {
            "mean": abs(float(actual["bootstrap_mean"]) - mean) <= 1e-12,
            "bias": abs(float(actual["bias"]) - (mean - float(actual["original"])))
            <= 1e-12,
            "standard_error": abs(float(actual["standard_error"]) - standard_error)
            <= 1e-12,
            "lower_type7": abs(float(actual["percentile_lower"]) - type7(values, alpha))
            <= 1e-12,
            "upper_type7": abs(
                float(actual["percentile_upper"]) - type7(values, 1.0 - alpha)
            )
            <= 1e-12,
        }
        exact_rows.append({"parameter": name, "passed": all(checks.values()), **checks})
    requested = int(payload["requested_replicates"])
    usable = int(payload["usable_replicates"])
    failed_replicates = int(payload["failed_replicates"])
    attempted = int(payload["attempted_fits"])
    minimum_usable = int(payload["minimum_usable_replicates"])
    inference_status = payload.get("inference", {}).get("status")
    accounting = {
        "requested_equals_attempted": requested == attempted,
        "attempts_equal_success_plus_failures": attempted == usable + failed_replicates,
        "failure_rows_equal_failed_replicates": len(payload["failures"])
        == failed_replicates,
        "witness_rows_equal_usable": len(rows) == usable,
        "unique_requested_replicates": len({row["replicate_index"] for row in rows})
        == usable,
        "minimum_threshold_recorded": minimum_usable
        == max(MINIMUM_REPLICATES, math.ceil(0.90 * requested)),
        "threshold_status_exact": (inference_status == "available")
        == (usable >= minimum_usable),
        "interval_presence_exact": bool(intervals)
        == (inference_status == "available"),
    }
    identity = {
        "method_version": payload.get("method_version") == METHOD_VERSION,
        "interval_method": payload.get("interval_method") == INTERVAL_METHOD,
        "requested_range": MINIMUM_REPLICATES <= requested <= 10_000,
        "all_hashes_sha256": all(
            len(row.get("sample_indices_sha256", "")) == 64 for row in rows
        )
        and all(
            len(row.get("sample_indices_sha256", "")) == 64
            for row in payload["failures"]
        ),
        "base_identity_hashes_sha256": all(
            len(witness.get(key, "")) == 64
            for key in ("recipe_sha256", "base_result_sha256")
        ),
        "dataset_fingerprint_bound": bool(witness.get("dataset_fingerprint")),
    }
    return {
        "passed": all(identity.values())
        and all(accounting.values())
        and all(row["passed"] for row in exact_rows),
        "identity": identity,
        "accounting": accounting,
        "intervals": exact_rows,
    }


def python_parameter_names() -> list[str]:
    """Return the frozen free-parameter order for latent_regression_sem."""

    return [
        "x=~x2",
        "x=~x3",
        "m=~m2",
        "m=~m3",
        "y=~y2",
        "y=~y3",
        "m~x",
        "y~x",
        "y~m",
        "x~~x",
        "m~~m",
        "y~~y",
        "x1~~x1",
        "x2~~x2",
        "x3~~x3",
        "m1~~m1",
        "m2~~m2",
        "m3~~m3",
        "y1~~y1",
        "y2~~y2",
        "y3~~y3",
    ]


def python_implied_covariance(raw: np.ndarray) -> np.ndarray:
    """Construct marker-identified Sigma for the independent reference SEM."""

    if raw.shape != (21,):
        raise ValueError("the frozen Python reference requires 21 free parameters")
    loadings = np.zeros((9, 3), dtype=float)
    loadings[0, 0] = loadings[3, 1] = loadings[6, 2] = 1.0
    loadings[1, 0], loadings[2, 0] = raw[0], raw[1]
    loadings[4, 1], loadings[5, 1] = raw[2], raw[3]
    loadings[7, 2], loadings[8, 2] = raw[4], raw[5]

    beta = np.zeros((3, 3), dtype=float)
    beta[1, 0], beta[2, 0], beta[2, 1] = raw[6], raw[7], raw[8]
    disturbance = np.diag(np.exp(raw[9:12]))
    inverse = np.linalg.inv(np.eye(3) - beta)
    latent = inverse @ disturbance @ inverse.T
    residual = np.exp(raw[12:21])
    return loadings @ latent @ loadings.T + np.diag(residual)


def python_standardized_ml_covariance(rows: np.ndarray) -> np.ndarray:
    """Match the documented listwise-standardized ML covariance convention."""

    if rows.ndim != 2 or rows.shape[0] < 10 or rows.shape[1] != 9:
        raise ValueError("the Python reference requires at least 10 rows by 9 indicators")
    centered = rows - rows.mean(axis=0)
    standard_deviation = centered.std(axis=0, ddof=1)
    if np.any(~np.isfinite(standard_deviation)) or np.any(standard_deviation <= 0.0):
        raise ValueError("a resample contains a constant or non-finite indicator")
    standardized = centered / standard_deviation
    return standardized.T @ standardized / rows.shape[0]


def python_ml_fit(rows: np.ndarray, start: np.ndarray | None = None) -> tuple[np.ndarray, np.ndarray]:
    """Perform one complete independent covariance-ML refit."""

    sample = python_standardized_ml_covariance(rows)
    sample_sign, sample_logdet = np.linalg.slogdet(sample)
    if sample_sign <= 0.0 or not np.isfinite(sample_logdet):
        raise ValueError("sample covariance is not positive definite")
    if start is None:
        start = np.array(
            [0.80] * 6
            + [0.35, 0.30, 0.30]
            + [0.0, -0.35, -0.35]
            + [math.log(0.30)] * 9,
            dtype=float,
        )
    bounds = (
        [(-3.0, 3.0)] * 6
        + [(-2.0, 2.0)] * 3
        + [(-12.0, 6.0)] * 3
        + [(-12.0, 6.0)] * 9
    )

    def objective(raw: np.ndarray) -> float:
        try:
            implied = python_implied_covariance(raw)
            sign, logdet = np.linalg.slogdet(implied)
            if sign <= 0.0 or not np.isfinite(logdet):
                return 1e50
            trace = float(np.trace(np.linalg.solve(implied, sample)))
            value = logdet + trace - sample_logdet - sample.shape[0]
            return value if np.isfinite(value) else 1e50
        except np.linalg.LinAlgError:
            return 1e50

    fitted = minimize(
        objective,
        np.asarray(start, dtype=float),
        method="L-BFGS-B",
        bounds=bounds,
        options={"maxiter": 2_000, "maxls": 50, "ftol": 1e-12, "gtol": 1e-7},
    )
    if not fitted.success or not np.isfinite(fitted.fun):
        raise ValueError(f"independent Python ML fit did not converge: {fitted.message}")
    raw = np.asarray(fitted.x, dtype=float)
    transformed = raw.copy()
    transformed[9:12] = np.exp(transformed[9:12])
    transformed[12:21] = np.exp(transformed[12:21])
    if np.any(~np.isfinite(transformed)):
        raise ValueError("independent Python ML fit returned non-finite estimates")
    return raw, transformed


def run_python_bootstrap(
    data: Path,
    quickpls: dict[str, Any],
    replicates: int = PYTHON_REFERENCE_REPLICATES,
) -> dict[str, Any]:
    """Run an independent seeded raw-case bootstrap and compare distributions."""

    rows = np.loadtxt(data, delimiter=",", skiprows=1, dtype=float)
    point_raw, point = python_ml_fit(rows)
    generator = np.random.default_rng(PYTHON_REFERENCE_SEED)
    successful: list[np.ndarray] = []
    failures: list[dict[str, Any]] = []
    for replicate_index in range(replicates):
        sampled = rows[generator.integers(0, rows.shape[0], size=rows.shape[0])]
        try:
            _, estimate = python_ml_fit(sampled, point_raw)
            successful.append(estimate)
        except ValueError as error:
            failures.append(
                {"replicate_index": replicate_index, "message": str(error)}
            )

    required = math.ceil(PYTHON_MINIMUM_USABLE_FRACTION * replicates)
    names = python_parameter_names()
    quick_intervals = {row["parameter"]: row for row in quickpls["intervals"]}
    comparisons: list[dict[str, Any]] = []
    if len(successful) >= 2:
        matrix = np.vstack(successful)
        for index, name in enumerate(names):
            actual = quick_intervals.get(name)
            if actual is None:
                continue
            values = matrix[:, index].tolist()
            python_se = float(matrix[:, index].std(ddof=1))
            quick_se = float(actual["standard_error"])
            point_delta = abs(float(actual["original"]) - float(point[index]))
            se_relative_delta = abs(quick_se - python_se) / max(abs(python_se), 1e-8)
            lower_delta = abs(
                float(actual["percentile_lower"]) - type7(values, 0.025)
            )
            upper_delta = abs(
                float(actual["percentile_upper"]) - type7(values, 0.975)
            )
            passed = (
                point_delta <= PYTHON_POINT_ABSOLUTE_TOLERANCE
                and se_relative_delta <= PYTHON_SE_RELATIVE_TOLERANCE
                and lower_delta <= PYTHON_INTERVAL_ABSOLUTE_TOLERANCE
                and upper_delta <= PYTHON_INTERVAL_ABSOLUTE_TOLERANCE
            )
            comparisons.append(
                {
                    "parameter": name,
                    "passed": passed,
                    "point_absolute_delta": point_delta,
                    "standard_error_relative_delta": se_relative_delta,
                    "lower_interval_delta": lower_delta,
                    "upper_interval_delta": upper_delta,
                }
            )
    passed = (
        len(successful) >= required
        and len(successful) + len(failures) == replicates
        and len(comparisons) >= 10
        and all(row["passed"] for row in comparisons)
    )
    return {
        "passed": passed,
        "independence": (
            "NumPy/SciPy covariance-ML optimizer with an independently generated "
            "raw-case resampling stream; no QuickPLS or lavaan refit code is called"
        ),
        "model": "marker-identified three-factor recursive latent SEM",
        "seed": PYTHON_REFERENCE_SEED,
        "requested_replicates": replicates,
        "usable_replicates": len(successful),
        "failed_replicates": failures,
        "minimum_usable_replicates": required,
        "limits": {
            "point_estimate_absolute": PYTHON_POINT_ABSOLUTE_TOLERANCE,
            "bootstrap_standard_error_relative": PYTHON_SE_RELATIVE_TOLERANCE,
            "percentile_endpoint_absolute": PYTHON_INTERVAL_ABSOLUTE_TOLERANCE,
        },
        "comparisons": comparisons,
    }


def run_lavaan_bootstrap(data: Path, work: Path, replicates: int) -> dict[str, Any]:
    output = work / "lavaan-bootstrap.json"
    script = work / "lavaan-bootstrap.R"
    script.write_text(
        textwrap.dedent(
            f"""
            suppressPackageStartupMessages(library(lavaan))
            set.seed(2026081402)
            data <- read.csv({json.dumps(str(data))})
            data[names(data)] <- scale(data[names(data)])
            model <- {json.dumps(baseline.lavaan_syntax('latent_regression_sem'))}
            fit <- sem(model, data=data, meanstructure=FALSE, std.lv=FALSE,
                       auto.fix.first=TRUE, estimator="ML", missing="listwise",
                       fixed.x=FALSE, se="bootstrap", bootstrap={replicates})
            pe <- parameterEstimates(fit, boot.ci.type="perc", level=0.95)
            rows <- lapply(seq_len(nrow(pe)), function(i) as.list(
                pe[i, c("lhs","op","rhs","est","se","ci.lower","ci.upper")]
            ))
            payload <- list(converged=lavInspect(fit, "converged"), rows=rows)
            writeLines(jsonlite::toJSON(payload, auto_unbox=TRUE, pretty=TRUE,
                       na="null", digits=16), {json.dumps(str(output))})
            """
        ),
        encoding="utf-8",
    )
    subprocess.run([baseline.rscript_path(), str(script)], cwd=ROOT, check=True)
    return strict_json(output)


def lavaan_name(row: dict[str, Any]) -> str | None:
    if row["op"] in {"=~", "~", "~~"}:
        return f"{row['lhs']}{row['op']}{row['rhs']}"
    return None


def compare_lavaan(
    quickpls: dict[str, Any], lavaan: dict[str, Any]
) -> dict[str, Any]:
    reference = {
        name: row
        for row in lavaan["rows"]
        if (name := lavaan_name(row)) is not None
    }
    comparisons = []
    for actual in quickpls["intervals"]:
        expected = reference.get(actual["parameter"])
        if expected is None or expected.get("se") is None:
            continue
        quick_se = float(actual["standard_error"])
        lavaan_se = float(expected["se"])
        scale = max(abs(lavaan_se), 1e-8)
        se_relative_delta = abs(quick_se - lavaan_se) / scale
        lower_delta = abs(float(actual["percentile_lower"]) - float(expected["ci.lower"]))
        upper_delta = abs(float(actual["percentile_upper"]) - float(expected["ci.upper"]))
        original_delta = abs(float(actual["original"]) - float(expected["est"]))
        passed = (
            original_delta <= 1e-6
            and se_relative_delta <= SE_RELATIVE_TOLERANCE
            and lower_delta <= INTERVAL_ABSOLUTE_TOLERANCE
            and upper_delta <= INTERVAL_ABSOLUTE_TOLERANCE
        )
        comparisons.append(
            {
                "parameter": actual["parameter"],
                "passed": passed,
                "original_delta": original_delta,
                "standard_error_relative_delta": se_relative_delta,
                "lower_interval_delta": lower_delta,
                "upper_interval_delta": upper_delta,
            }
        )
    return {
        "passed": lavaan.get("converged") is True
        and len(comparisons) >= 10
        and all(row["passed"] for row in comparisons),
        "independence": "lavaan 0.7.2 full raw-case ML refits with an independent seed stream",
        "limits": {
            "point_estimate_absolute": 1e-6,
            "bootstrap_standard_error_relative": SE_RELATIVE_TOLERANCE,
            "percentile_endpoint_absolute": INTERVAL_ABSOLUTE_TOLERANCE,
        },
        "comparisons": comparisons,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--replicates", type=int, default=MINIMUM_REPLICATES)
    parser.add_argument("--workers", type=int, default=4)
    args = parser.parse_args()
    if not MINIMUM_REPLICATES <= args.replicates <= 10_000:
        parser.error("replicates must be between 1000 and 10000")
    with tempfile.TemporaryDirectory(prefix="cbsem_bootstrap_v2_reference_") as directory:
        work = Path(directory)
        quickpls, data = run_quickpls(work, args.replicates, args.workers)
        exact = validate_quickpls_witness(quickpls)
        python_reference = run_python_bootstrap(data, quickpls)
        lavaan = run_lavaan_bootstrap(data, work, args.replicates)
        independent = compare_lavaan(quickpls, lavaan)
    report = {
        "kind": "cbsem_bootstrap_v2_independent_reference_v1",
        "passed": exact["passed"]
        and python_reference["passed"]
        and independent["passed"],
        "method_version": METHOD_VERSION,
        "requested_replicates": args.replicates,
        "quickpls_exact_checks": exact,
        "independent_python": python_reference,
        "independent_lavaan": independent,
        "legacy_v1_eligible": False,
    }
    RESULT.write_text(
        json.dumps(report, indent=2, allow_nan=False) + "\n", encoding="utf-8"
    )
    print(json.dumps({"passed": report["passed"], "report": str(RESULT)}, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
