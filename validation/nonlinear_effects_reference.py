"""Independent quadratic nonlinear-effects diagnostic reference."""

import csv
import json
import random
import subprocess
from pathlib import Path

import numpy as np

from higher_order_reference import estimate_pls


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "nonlinear_effects_reference.csv"
RECIPE = RESULTS / "nonlinear_effects_reference.recipe.json"
QUICKPLS = RESULTS / "nonlinear_effects_reference_quickpls.json"
OUTPUT = RESULTS / "nonlinear_effects_reference_report.json"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
TOLERANCE = 1e-6


def ensure_cli():
    if not CLI_EXE.exists():
        subprocess.run(
            ["cargo", "build", "-p", "qpls-cli"],
            cwd=ROOT,
            check=True,
            stdout=subprocess.DEVNULL,
        )
    return CLI_EXE


def qpls_cli(args, **kwargs):
    return subprocess.run([str(ensure_cli()), *args], cwd=ROOT, check=True, **kwargs)


def generated_rows(seed=20260719, n=140):
    rng = random.Random(seed)
    rows = []
    for _ in range(n):
        x = rng.gauss(0.0, 1.0)
        z = 0.25 * x + rng.gauss(0.0, 0.85)
        y = 0.38 * x + 0.28 * z + 0.42 * (x * x - 1.0) + rng.gauss(0.0, 0.35)
        rows.append(
            {
                "x1": x + rng.gauss(0.0, 0.07),
                "x2": 0.92 * x + rng.gauss(0.0, 0.09),
                "z1": z + rng.gauss(0.0, 0.08),
                "z2": 0.90 * z + rng.gauss(0.0, 0.10),
                "y1": y + rng.gauss(0.0, 0.07),
                "y2": 0.94 * y + rng.gauss(0.0, 0.09),
            }
        )
    return rows


def write_dataset(rows):
    RESULTS.mkdir(parents=True, exist_ok=True)
    fields = ["x1", "x2", "z1", "z2", "y1", "y2"]
    with DATA.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            writer.writerow({field: f"{row[field]:.12f}" for field in fields})


def dataset_fingerprint():
    project_path = RESULTS / "nonlinear_effects_reference.fingerprint.qpls"
    qpls_cli(
        [
            "import",
            str(DATA.relative_to(ROOT)),
            str(project_path.relative_to(ROOT)),
            "--name",
            "nonlinear_effects_reference",
        ],
        stdout=subprocess.DEVNULL,
    )
    payload = json.loads(
        subprocess.run(
            [str(ensure_cli()), "inspect", str(project_path.relative_to(ROOT)), "--json"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    )
    return payload["datasets"][0]["fingerprint"]


def recipe_payload(fingerprint):
    return {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000007",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000008",
            "name": "Quadratic nonlinear-effects reference",
            "constructs": [
                {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2"]},
                {"id": "z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1", "z2"]},
                {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2"]},
            ],
            "paths": [
                {"source": "x", "target": "y"},
                {"source": "z", "target": "y"},
            ],
        },
        "settings": {
            "method": "nonlinear_effects",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260719,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {"fixture": "independent_quadratic_nonlinear_reference"},
    }


def centered_square(values):
    centered = values - float(np.mean(values))
    squared = centered * centered
    return squared - float(np.mean(squared))


def regression_stats(predictors, outcome):
    x = np.column_stack([predictor - float(np.mean(predictor)) for predictor in predictors])
    y = outcome - float(np.mean(outcome))
    xtx = x.T @ x
    beta = np.linalg.solve(xtx, x.T @ y)
    residuals = y - x @ beta
    df = len(y) - x.shape[1] - 1
    sigma2 = float(residuals @ residuals) / df
    covariance = sigma2 * np.linalg.inv(xtx)
    standard_errors = np.sqrt(np.diag(covariance))
    t_statistics = beta / standard_errors
    r_squared = 1.0 - float(residuals @ residuals) / float(y @ y)
    return beta, standard_errors, t_statistics, max(0.0, min(1.0, r_squared))


def max_delta(actual, expected):
    return max(abs(actual[key] - expected[key]) for key in expected)


def main():
    rows = generated_rows()
    write_dataset(rows)
    fingerprint = dataset_fingerprint()
    recipe = recipe_payload(fingerprint)
    RECIPE.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")
    columns = {name: np.asarray([float(row[name]) for row in rows], dtype=float) for name in rows[0]}
    reference = estimate_pls(columns, recipe)
    construct_ids = [construct["id"] for construct in recipe["model"]["constructs"]]
    scores = {construct_id: np.asarray(reference["scores"][construct_id], dtype=float) for construct_id in construct_ids}
    expected_coefficients = {}
    expected_standard_errors = {}
    expected_t = {}
    expected_linear_r2 = {}
    expected_augmented_r2 = {}
    expected_delta_r2 = {}
    for target in construct_ids:
        predecessors = [path["source"] for path in recipe["model"]["paths"] if path["target"] == target]
        if not predecessors:
            continue
        linear_predictors = [scores[source] for source in predecessors]
        _, _, _, linear_r2 = regression_stats(linear_predictors, scores[target])
        predictors = linear_predictors + [centered_square(scores[source]) for source in predecessors]
        beta, standard_errors, t_statistics, augmented_r2 = regression_stats(predictors, scores[target])
        for within, source in enumerate(predecessors):
            key = (source, target)
            quadratic_index = len(predecessors) + within
            expected_coefficients[key] = float(beta[quadratic_index])
            expected_standard_errors[key] = float(standard_errors[quadratic_index])
            expected_t[key] = float(t_statistics[quadratic_index])
            expected_linear_r2[key] = linear_r2
            expected_augmented_r2[key] = augmented_r2
            expected_delta_r2[key] = max(0.0, augmented_r2 - linear_r2)

    qpls_cli(
        [
            "run",
            str(RECIPE.relative_to(ROOT)),
            "--data",
            str(DATA.relative_to(ROOT)),
            "--output",
            str(QUICKPLS.relative_to(ROOT)),
            "--allow-experimental",
        ],
        stdout=subprocess.DEVNULL,
    )
    quickpls = json.loads(QUICKPLS.read_text(encoding="utf-8"))
    estimation = quickpls["payload"]["estimation"]
    analysis = estimation["nonlinear_effects"]
    actual_coefficients = {(row["source"], row["target"]): row["quadratic_coefficient"] for row in analysis["estimates"]}
    actual_standard_errors = {(row["source"], row["target"]): row["standard_error"] for row in analysis["estimates"]}
    actual_t = {(row["source"], row["target"]): row["t_statistic"] for row in analysis["estimates"]}
    actual_linear_r2 = {(row["source"], row["target"]): row["linear_r_squared"] for row in analysis["estimates"]}
    actual_augmented_r2 = {(row["source"], row["target"]): row["augmented_r_squared"] for row in analysis["estimates"]}
    actual_delta_r2 = {(row["source"], row["target"]): row["delta_r_squared"] for row in analysis["estimates"]}
    actual_p_values = [row["p_value_two_sided"] for row in analysis["estimates"]]
    checks = {
        "method_version": estimation["method_version"] == "pls_quadratic_nonlinear_effects_v1",
        "payload_version": analysis["method_version"] == "pls_quadratic_nonlinear_effects_v1",
        "coefficient_delta": max_delta(actual_coefficients, expected_coefficients),
        "standard_error_delta": max_delta(actual_standard_errors, expected_standard_errors),
        "t_delta": max_delta(actual_t, expected_t),
        "linear_r2_delta": max_delta(actual_linear_r2, expected_linear_r2),
        "augmented_r2_delta": max_delta(actual_augmented_r2, expected_augmented_r2),
        "delta_r2_delta": max_delta(actual_delta_r2, expected_delta_r2),
        "p_values_in_range": all(0.0 <= value <= 1.0 for value in actual_p_values),
        "has_experimental_warning": any("experimental" in warning.lower() for warning in analysis["warnings"]),
    }
    passed = (
        checks["method_version"]
        and checks["payload_version"]
        and checks["coefficient_delta"] <= TOLERANCE
        and checks["standard_error_delta"] <= TOLERANCE
        and checks["t_delta"] <= TOLERANCE
        and checks["linear_r2_delta"] <= TOLERANCE
        and checks["augmented_r2_delta"] <= TOLERANCE
        and checks["delta_r2_delta"] <= TOLERANCE
        and checks["p_values_in_range"]
        and checks["has_experimental_warning"]
    )
    report = {
        "schema_version": 1,
        "kind": "quadratic_nonlinear_effects_reference_v1",
        "passed": passed,
        "tolerance": TOLERANCE,
        "checks": checks,
        "estimates": analysis["estimates"],
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    max_observed = max(
        checks["coefficient_delta"],
        checks["standard_error_delta"],
        checks["t_delta"],
        checks["linear_r2_delta"],
        checks["augmented_r2_delta"],
        checks["delta_r2_delta"],
    )
    print(f"wrote {OUTPUT} | passed={passed} | max_delta={max_observed:.3g}")
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
