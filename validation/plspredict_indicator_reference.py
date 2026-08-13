"""Independent numerical reference for QuickPLS indicator prediction and CVPAT v2.

The fixture intentionally uses one reflective indicator per construct. In that
bounded shape the PLS construct scores are the train-standardized indicators,
so the complete recursive prediction, IA/LM benchmarks, fold plan, error
metrics, Q2_predict, and paired CVPAT statistics can be recomputed directly
without importing or translating the QuickPLS estimator.
"""

from __future__ import annotations

import csv
import hashlib
import json
import math
import subprocess
from collections import defaultdict
from pathlib import Path

import numpy as np
from scipy.stats import t as student_t


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "plspredict_indicator_reference.csv"
RECIPE = RESULTS / "plspredict_indicator_reference.recipe.json"
QUICKPLS = RESULTS / "plspredict_indicator_reference_quickpls.json"
REPEAT = RESULTS / "plspredict_indicator_reference_repeat.json"
REPORT = RESULTS / "plspredict_indicator_reference_report.json"
FINGERPRINT_PROJECT = RESULTS / "plspredict_indicator_reference.fingerprint.qpls"
CLI = ROOT / "target" / "debug" / "qpls.exe"

METHOD_VERSION = "plspredict_indicator_v2"
REPEATED_VERSION = "plspredict_repeated_kfold_indicator_v2"
CVPAT_VERSION = "cvpat_indicator_benchmarks_v2"
ASSIGNMENT = "seeded_sha256_source_row_order_round_robin_10_v1"
SEED = 20260811
FOLDS = 10
REPEATS = 10
CONFIDENCE = 0.95
TOLERANCE = 2e-9


def run_cli(*args: str, capture: bool = False) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(CLI), *args],
        cwd=ROOT,
        check=True,
        capture_output=capture,
        text=True,
    )


def ensure_cli() -> None:
    env = dict(__import__("os").environ)
    env.setdefault("CARGO_BUILD_JOBS", "1")
    subprocess.run(
        ["cargo", "build", "-p", "qpls-cli"],
        cwd=ROOT,
        env=env,
        check=True,
    )


def write_fixture() -> np.ndarray:
    RESULTS.mkdir(parents=True, exist_ok=True)
    rows: list[tuple[float, float, float]] = []
    with DATA.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle)
        writer.writerow(["x1", "m1", "y1"])
        for index in range(120):
            x = 1.0 + index / 12.0 + 0.08 * math.sin(index * 0.37)
            m = 2.0 + 0.72 * x + 0.34 * math.sin(index * 0.61)
            y = 3.0 + 0.81 * m + 0.22 * math.cos(index * 0.43)
            rows.append((x, m, y))
            writer.writerow([f"{x:.12f}", f"{m:.12f}", f"{y:.12f}"])
    return np.asarray(rows, dtype=float)


def dataset_fingerprint() -> str:
    run_cli(
        "import",
        str(DATA.relative_to(ROOT)),
        str(FINGERPRINT_PROJECT.relative_to(ROOT)),
        "--name",
        "PLSpredict indicator reference",
    )
    inspected = json.loads(
        run_cli(
            "inspect",
            str(FINGERPRINT_PROJECT.relative_to(ROOT)),
            "--json",
            capture=True,
        ).stdout
    )
    return inspected["datasets"][0]["fingerprint"]


def write_recipe(fingerprint: str) -> None:
    recipe = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000271",
        "created_at": "2026-08-11T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000272",
            "name": "Recursive indicator prediction reference",
            "constructs": [
                {
                    "id": "x",
                    "name": "X",
                    "short_name": "X",
                    "mode": "reflective",
                    "indicators": ["x1"],
                },
                {
                    "id": "m",
                    "name": "M",
                    "short_name": "M",
                    "mode": "reflective",
                    "indicators": ["m1"],
                },
                {
                    "id": "y",
                    "name": "Y",
                    "short_name": "Y",
                    "mode": "reflective",
                    "indicators": ["y1"],
                },
            ],
            "paths": [
                {"source": "x", "target": "m"},
                {"source": "m", "target": "y"},
            ],
        },
        "settings": {
            "method": "predict",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "studentized_inner_samples": 0,
            "permutation_samples": 0,
            "confidence_level": CONFIDENCE,
            "seed": SEED,
            "workers": 1,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {
            "fixture": "plspredict_indicator_reference_v2",
            "status": "validated_plspredict_indicator_v2_bounded_scope",
        },
    }
    RECIPE.write_text(json.dumps(recipe, indent=2), encoding="utf-8")


def execute(output: Path) -> dict:
    run_cli(
        "run",
        str(RECIPE.relative_to(ROOT)),
        "--data",
        str(DATA.relative_to(ROOT)),
        "--output",
        str(output.relative_to(ROOT)),
    )
    return json.loads(output.read_text(encoding="utf-8"))


def sample_standardize(values: np.ndarray) -> tuple[np.ndarray, float, float]:
    center = float(np.mean(values))
    scale = float(np.std(values, ddof=1))
    if not math.isfinite(scale) or scale <= np.finfo(float).eps:
        raise AssertionError("reference fixture unexpectedly contains a constant fold")
    return (values - center) / scale, center, scale


def centered_ols(train_x: np.ndarray, train_y: np.ndarray, test_x: np.ndarray) -> np.ndarray:
    train_x = np.asarray(train_x, dtype=float)
    test_x = np.asarray(test_x, dtype=float)
    if train_x.ndim == 1:
        train_x = train_x[:, None]
        test_x = test_x[:, None]
    means = np.mean(train_x, axis=0)
    outcome_mean = float(np.mean(train_y))
    coefficients, _, rank, _ = np.linalg.lstsq(train_x - means, train_y - outcome_mean, rcond=None)
    if rank != train_x.shape[1]:
        raise AssertionError("reference LM unexpectedly rank deficient")
    return outcome_mean + (test_x - means) @ coefficients


def fold_assignments(row_count: int, repeat: int) -> list[int]:
    ranked: list[tuple[bytes, int]] = []
    for source_row in range(row_count):
        material = f"{METHOD_VERSION}|{SEED}|{repeat}|{source_row}".encode("utf-8")
        ranked.append((hashlib.sha256(material).digest(), source_row))
    ranked.sort(key=lambda pair: (pair[0], pair[1]))
    assignments = [0] * row_count
    for position, (_, source_row) in enumerate(ranked):
        assignments[source_row] = position % FOLDS
    return assignments


def error_metrics(actual: np.ndarray, predicted: np.ndarray) -> dict:
    errors = actual - predicted
    squared_error_sum = float(np.sum(errors**2))
    absolute_error_sum = float(np.sum(np.abs(errors)))
    nonzero = np.abs(actual) > np.finfo(float).eps
    ape_sum = float(np.sum(np.abs(errors[nonzero] / actual[nonzero]))) if np.any(nonzero) else None
    mape_count = int(np.sum(nonzero))
    return {
        "observations": int(actual.size),
        "squared_error_sum": squared_error_sum,
        "absolute_error_sum": absolute_error_sum,
        "rmse": math.sqrt(squared_error_sum / actual.size),
        "mae": absolute_error_sum / actual.size,
        "absolute_percentage_error_sum": ape_sum,
        "mape_observations": mape_count,
        "mape_percent": (100.0 * ape_sum / mape_count) if ape_sum is not None else None,
    }


def predict_fold(data: np.ndarray, train_rows: np.ndarray, test_rows: np.ndarray) -> dict:
    train = data[train_rows]
    test = data[test_rows]
    train_z = np.empty_like(train)
    test_z = np.empty_like(test)
    transforms: list[tuple[float, float]] = []
    for column in range(data.shape[1]):
        train_z[:, column], center, scale = sample_standardize(train[:, column])
        test_z[:, column] = (test[:, column] - center) / scale
        transforms.append((center, scale))

    # Single-indicator reflective constructs have unit positive outer weights.
    b_x_m = float(np.dot(train_z[:, 0], train_z[:, 1]) / np.dot(train_z[:, 0], train_z[:, 0]))
    b_m_y = float(np.dot(train_z[:, 1], train_z[:, 2]) / np.dot(train_z[:, 1], train_z[:, 1]))
    predicted_m_z = b_x_m * test_z[:, 0]
    predicted_y_z = b_m_y * predicted_m_z
    predicted_z = {"m1": predicted_m_z, "y1": predicted_y_z}
    column_for = {"m1": 1, "y1": 2}
    indicator_rows = []
    for indicator in ("m1", "y1"):
        column = column_for[indicator]
        center, scale = transforms[column]
        predicted = predicted_z[indicator] * scale + center
        actual = test[:, column]
        ia = np.full(test_rows.size, center)
        lm_z = centered_ols(train_z[:, [0]], train_z[:, column], test_z[:, [0]])
        lm = lm_z * scale + center
        indicator_rows.append(
            {
                "construct": indicator[0],
                "indicator": indicator,
                "test_rows": test_rows,
                "actual": actual,
                "pls": predicted,
                "ia": ia,
                "lm": lm,
            }
        )
    construct_rows = [
        {
            "construct": "m",
            "test_rows": test_rows,
            "actual": test_z[:, 1],
            "pls": predicted_m_z,
            "ia": np.zeros(test_rows.size),
            "lm": centered_ols(train_z[:, [0]], train_z[:, 1], test_z[:, [0]]),
        },
        {
            "construct": "y",
            "test_rows": test_rows,
            "actual": test_z[:, 2],
            "pls": predicted_y_z,
            "ia": np.zeros(test_rows.size),
            "lm": centered_ols(train_z[:, [0]], train_z[:, 2], test_z[:, [0]]),
        },
    ]
    return {"indicators": indicator_rows, "constructs": construct_rows}


def aggregate_rows(folds: list[dict], key: str, ids: tuple[str, ...]) -> dict[str, dict]:
    aggregated: dict[str, dict[str, list[float]]] = {
        row_id: {"actual": [], "pls": [], "ia": [], "lm": []} for row_id in ids
    }
    for fold in folds:
        for row in fold[key]:
            row_id = row["indicator"] if key == "indicators" else row["construct"]
            for metric in ("actual", "pls", "ia", "lm"):
                aggregated[row_id][metric].extend(np.asarray(row[metric], dtype=float).tolist())
    return {
        row_id: {
            metric: np.asarray(values, dtype=float) for metric, values in payload.items()
        }
        for row_id, payload in aggregated.items()
    }


def cvpat_row(pls_losses: np.ndarray, benchmark_losses: np.ndarray, benchmark: str) -> dict:
    differences = pls_losses - benchmark_losses
    n = differences.size
    mean_difference = float(np.mean(differences))
    se = float(np.std(differences, ddof=1) / math.sqrt(n))
    statistic = mean_difference / se
    p_value = float(student_t.cdf(statistic, df=n - 1))
    critical = float(student_t.ppf(0.5 + CONFIDENCE / 2.0, df=n - 1))
    return {
        "method_version": CVPAT_VERSION,
        "benchmark": benchmark,
        "mean_loss_pls": float(np.mean(pls_losses)),
        "mean_loss_benchmark": float(np.mean(benchmark_losses)),
        "mean_loss_difference": mean_difference,
        "loss_difference_sum_of_squares": float(np.sum(differences**2)),
        "standard_error": se,
        "t_statistic": statistic,
        "p_value_one_sided": p_value,
        "confidence_interval_lower": mean_difference - critical * se,
        "confidence_interval_upper": mean_difference + critical * se,
        "observations": n,
        "indicator_count": 2,
        "status": "available",
        "preferred_model": "pls_sem" if mean_difference < 0.0 and p_value < 0.05 else None,
    }


def oracle(data: np.ndarray) -> dict:
    folds: list[dict] = []
    assignment_text = []
    case_losses: dict[int, dict[str, list[float]]] = defaultdict(
        lambda: {"pls": [], "indicator_average": [], "linear_model": []}
    )
    for repeat in range(REPEATS):
        assignments = fold_assignments(data.shape[0], repeat)
        for source_row, fold in enumerate(assignments):
            assignment_text.append(f"{repeat}|{source_row}|{fold}\n")
        for fold in range(FOLDS):
            train_rows = np.asarray([index for index, value in enumerate(assignments) if value != fold])
            test_rows = np.asarray([index for index, value in enumerate(assignments) if value == fold])
            predicted = predict_fold(data, train_rows, test_rows)
            folds.append(predicted)
            for position, source_row in enumerate(test_rows):
                indicator_rows = predicted["indicators"]
                case_losses[int(source_row)]["pls"].append(
                    float(np.mean([(row["actual"][position] - row["pls"][position]) ** 2 for row in indicator_rows]))
                )
                case_losses[int(source_row)]["indicator_average"].append(
                    float(np.mean([(row["actual"][position] - row["ia"][position]) ** 2 for row in indicator_rows]))
                )
                case_losses[int(source_row)]["linear_model"].append(
                    float(np.mean([(row["actual"][position] - row["lm"][position]) ** 2 for row in indicator_rows]))
                )

    indicators = aggregate_rows(folds, "indicators", ("m1", "y1"))
    constructs = aggregate_rows(folds, "constructs", ("m", "y"))
    indicator_metrics = {}
    for indicator, values in indicators.items():
        pls = error_metrics(values["actual"], values["pls"])
        ia = error_metrics(values["actual"], values["ia"])
        lm = error_metrics(values["actual"], values["lm"])
        indicator_metrics[indicator] = {
            "pls": pls,
            "indicator_average": ia,
            "linear_model": lm,
            "q_squared_predict": 1.0 - pls["squared_error_sum"] / ia["squared_error_sum"],
        }
    construct_metrics = {}
    for construct, values in constructs.items():
        pls = error_metrics(values["actual"], values["pls"])
        ia = error_metrics(values["actual"], values["ia"])
        lm = error_metrics(values["actual"], values["lm"])
        construct_metrics[construct] = {
            "pls": pls,
            "indicator_average": ia,
            "linear_model": lm,
            "q_squared_predict": 1.0 - pls["squared_error_sum"] / ia["squared_error_sum"],
        }
    pls_case = np.asarray([np.mean(case_losses[row]["pls"]) for row in range(data.shape[0])])
    ia_case = np.asarray([np.mean(case_losses[row]["indicator_average"]) for row in range(data.shape[0])])
    lm_case = np.asarray([np.mean(case_losses[row]["linear_model"]) for row in range(data.shape[0])])
    assignment_digest = "sha256:" + hashlib.sha256("".join(assignment_text).encode("utf-8")).hexdigest()
    return {
        "assignment_digest": assignment_digest,
        "indicators": indicator_metrics,
        "constructs": construct_metrics,
        "cvpat": {
            "indicator_average": cvpat_row(pls_case, ia_case, "indicator_average"),
            "linear_model": cvpat_row(pls_case, lm_case, "linear_model"),
        },
    }


def close(left, right, tolerance: float = TOLERANCE) -> bool:
    if left is None or right is None:
        return left is None and right is None
    return math.isclose(float(left), float(right), rel_tol=tolerance, abs_tol=tolerance)


def metrics_match(actual: dict, expected: dict) -> bool:
    exact = ("observations", "mape_observations")
    numeric = (
        "squared_error_sum",
        "absolute_error_sum",
        "rmse",
        "mae",
        "absolute_percentage_error_sum",
        "mape_percent",
    )
    return all(actual.get(key) == expected.get(key) for key in exact) and all(
        close(actual.get(key), expected.get(key)) for key in numeric
    )


def compare(result: dict, expected: dict, repeat_result: dict) -> dict:
    estimation = result["payload"]["estimation"]
    prediction = estimation["predict"]
    repeated = prediction["repeated_kfold"]
    actual_indicators = {row["indicator"]: row for row in repeated["indicator_targets"]}
    actual_constructs = {row["construct"]: row for row in repeated["targets"]}
    actual_cvpat = {row["benchmark"]: row for row in repeated["cvpat_benchmark_assessments"]}

    indicator_checks = {}
    for indicator, oracle_row in expected["indicators"].items():
        row = actual_indicators.get(indicator, {})
        indicator_checks[indicator] = (
            row.get("predictor_scope") == "earliest_antecedent_indicators"
            and row.get("predictor_count") == 1
            and metrics_match(row.get("pls", {}), oracle_row["pls"])
            and metrics_match(row.get("indicator_average", {}), oracle_row["indicator_average"])
            and row.get("linear_model", {}).get("status") == "available"
            and metrics_match(row.get("linear_model", {}).get("metrics", {}), oracle_row["linear_model"])
            and close(row.get("q_squared_predict"), oracle_row["q_squared_predict"])
        )

    construct_checks = {}
    for construct, oracle_row in expected["constructs"].items():
        row = actual_constructs.get(construct, {})
        construct_checks[construct] = (
            row.get("predictor_count") == 1
            and close(row.get("rmse_pls"), oracle_row["pls"]["rmse"])
            and close(row.get("mae_pls"), oracle_row["pls"]["mae"])
            and close(row.get("rmse_benchmark"), oracle_row["indicator_average"]["rmse"])
            and close(row.get("mae_benchmark"), oracle_row["indicator_average"]["mae"])
            and close(row.get("rmse_lm"), oracle_row["linear_model"]["rmse"])
            and close(row.get("mae_lm"), oracle_row["linear_model"]["mae"])
            and close(row.get("q_squared_predict"), oracle_row["q_squared_predict"])
        )

    cvpat_checks = {}
    for benchmark, oracle_row in expected["cvpat"].items():
        row = actual_cvpat.get(benchmark, {})
        numeric_keys = (
            "mean_loss_pls",
            "mean_loss_benchmark",
            "mean_loss_difference",
            "loss_difference_sum_of_squares",
            "standard_error",
            "t_statistic",
            "p_value_one_sided",
            "confidence_interval_lower",
            "confidence_interval_upper",
        )
        cvpat_checks[benchmark] = (
            row.get("method_version") == CVPAT_VERSION
            and row.get("comparison_kind") == "benchmark_assessment"
            and row.get("target_scope") == "all_endogenous_indicators"
            and row.get("loss") == "mean_squared_error_across_indicators_per_observation"
            and row.get("alternative") == "pls_loss_less_than_benchmark"
            and close(row.get("confidence_level"), CONFIDENCE)
            and row.get("observations") == 120
            and row.get("indicator_count") == 2
            and row.get("status") == oracle_row["status"]
            and row.get("preferred_model") == oracle_row["preferred_model"]
            and all(close(row.get(key), oracle_row[key]) for key in numeric_keys)
        )

    checks = {
        "current_method_versions": estimation.get("method_version") == METHOD_VERSION
        and prediction.get("method_version") == METHOD_VERSION
        and prediction.get("primary_analysis") == REPEATED_VERSION
        and repeated.get("method_version") == REPEATED_VERSION,
        "seeded_balanced_plan": repeated.get("folds") == FOLDS
        and repeated.get("repeats") == REPEATS
        and repeated.get("seed") == SEED
        and repeated.get("assignment") == ASSIGNMENT
        and repeated.get("assignment_digest") == expected["assignment_digest"]
        and repeated.get("total_test_observations") == 1200,
        "indicator_prediction_matches_independent_oracle": all(indicator_checks.values())
        and set(actual_indicators) == {"m1", "y1"},
        "recursive_construct_prediction_matches_independent_oracle": all(construct_checks.values())
        and set(actual_constructs) == {"m", "y"},
        "cvpat_matches_independent_paired_test": all(cvpat_checks.values())
        and set(actual_cvpat) == {"indicator_average", "linear_model"},
        "legacy_rows_are_not_relabelled": repeated.get("cvpat") == []
        and repeated.get("paired_loss_diagnostics") == [],
        "bounded_scope_is_explicit": any(
            "does not compare separately saved models" in warning.lower()
            for warning in prediction.get("warnings", []) + repeated.get("warnings", [])
        ),
        "deterministic_repeat": repeat_result["payload"]["estimation"]["predict"] == prediction,
    }
    return {
        "checks": checks,
        "indicator_checks": indicator_checks,
        "construct_checks": construct_checks,
        "cvpat_checks": cvpat_checks,
        "passed": all(checks.values()),
    }


def main() -> None:
    ensure_cli()
    data = write_fixture()
    write_recipe(dataset_fingerprint())
    result = execute(QUICKPLS)
    repeat_result = execute(REPEAT)
    expected = oracle(data)
    comparison = compare(result, expected, repeat_result)
    report = {
        "kind": "plspredict_indicator_reference_v2",
        "passed": comparison["passed"],
        "checks": comparison["checks"],
        "details": {
            "indicator_checks": comparison["indicator_checks"],
            "construct_checks": comparison["construct_checks"],
            "cvpat_checks": comparison["cvpat_checks"],
        },
        "quickpls": result["payload"]["estimation"]["predict"],
        "oracle": expected,
        "artifacts": {
            "data": str(DATA.relative_to(ROOT)),
            "recipe": str(RECIPE.relative_to(ROOT)),
            "quickpls": str(QUICKPLS.relative_to(ROOT)),
            "repeat": str(REPEAT.relative_to(ROOT)),
        },
    }
    REPORT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"wrote {REPORT} | passed={report['passed']}")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
