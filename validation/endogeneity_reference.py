"""Independent Gaussian-copula endogeneity diagnostic reference."""

import csv
import json
import math
import random
import subprocess
from pathlib import Path

import numpy as np

from higher_order_reference import estimate_pls


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "endogeneity_reference.csv"
RECIPE = RESULTS / "endogeneity_reference.recipe.json"
QUICKPLS = RESULTS / "endogeneity_reference_quickpls.json"
OUTPUT = RESULTS / "endogeneity_reference_report.json"
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


def generated_rows(seed=20260719, n=150):
    rng = random.Random(seed)
    rows = []
    for _ in range(n):
        omitted = rng.expovariate(1.0) - 1.0
        x = 0.72 * omitted + rng.expovariate(1.2) - 0.85
        z = 0.28 * x + rng.gauss(0.0, 0.85)
        y = 0.48 * x + 0.34 * z + 0.42 * omitted + rng.gauss(0.0, 0.42)
        rows.append(
            {
                "x1": x + rng.gauss(0.0, 0.08),
                "x2": 0.91 * x + rng.gauss(0.0, 0.10),
                "z1": z + rng.gauss(0.0, 0.09),
                "z2": 0.88 * z + rng.gauss(0.0, 0.11),
                "y1": y + rng.gauss(0.0, 0.08),
                "y2": 0.93 * y + rng.gauss(0.0, 0.10),
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
    project_path = RESULTS / "endogeneity_reference.fingerprint.qpls"
    qpls_cli(
        [
            "import",
            str(DATA.relative_to(ROOT)),
            str(project_path.relative_to(ROOT)),
            "--name",
            "endogeneity_reference",
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
        "id": "00000000-0000-0000-0000-000000000005",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000006",
            "name": "Gaussian-copula endogeneity reference",
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
            "method": "endogeneity",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260719,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {"fixture": "independent_gaussian_copula_reference"},
    }


def rankit_inverse_normal(values):
    ordered = sorted(enumerate(values), key=lambda pair: (pair[1], pair[0]))
    ranks = np.zeros(len(values), dtype=float)
    cursor = 0
    while cursor < len(ordered):
        start = cursor
        value = ordered[cursor][1]
        while cursor < len(ordered) and ordered[cursor][1] == value:
            cursor += 1
        average_rank = (start + 1 + cursor) / 2.0
        for index in range(start, cursor):
            ranks[ordered[index][0]] = average_rank
    probabilities = ranks / (len(values) + 1.0)
    transformed = np.asarray([normal_inverse_cdf(p) for p in probabilities], dtype=float)
    return transformed - float(np.mean(transformed))


def normal_inverse_cdf(probability):
    # Peter J. Acklam's rational approximation; absolute error is sufficient
    # for the 1e-6 end-to-end diagnostic tolerance used by this fixture.
    a = [-39.6968302866538, 220.946098424521, -275.928510446969, 138.357751867269, -30.6647980661472, 2.50662827745924]
    b = [-54.4760987982241, 161.585836858041, -155.698979859887, 66.8013118877197, -13.2806815528857]
    c = [-0.00778489400243029, -0.322396458041136, -2.40075827716184, -2.54973253934373, 4.37466414146497, 2.93816398269878]
    d = [0.00778469570904146, 0.32246712907004, 2.445134137143, 3.75440866190742]
    plow = 0.02425
    phigh = 1.0 - plow
    if probability < plow:
        q = math.sqrt(-2.0 * math.log(probability))
        return (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5]) / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)
    if probability <= phigh:
        q = probability - 0.5
        r = q * q
        return (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1.0)
    q = math.sqrt(-2.0 * math.log(1.0 - probability))
    return -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5]) / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1.0)


def regression_stats(predictors, outcome):
    x = np.column_stack([predictor - float(np.mean(predictor)) for predictor in predictors])
    y = outcome - float(np.mean(outcome))
    xtx = x.T @ x
    xty = x.T @ y
    beta = np.linalg.solve(xtx, xty)
    residuals = y - x @ beta
    df = len(y) - x.shape[1] - 1
    sigma2 = float(residuals @ residuals) / df
    covariance = sigma2 * np.linalg.inv(xtx)
    standard_errors = np.sqrt(np.diag(covariance))
    t_statistics = beta / standard_errors
    return beta, standard_errors, t_statistics


def skewness(values):
    centered = values - float(np.mean(values))
    sd = float(np.std(values, ddof=1))
    return float(np.mean((centered / sd) ** 3))


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
    copulas = {construct_id: rankit_inverse_normal(scores[construct_id]) for construct_id in construct_ids}
    expected_coefficients = {}
    expected_standard_errors = {}
    expected_t = {}
    expected_skewness = {}
    for target in construct_ids:
        predecessors = [path["source"] for path in recipe["model"]["paths"] if path["target"] == target]
        if not predecessors:
            continue
        predictors = [scores[source] for source in predecessors] + [copulas[source] for source in predecessors]
        beta, standard_errors, t_statistics = regression_stats(predictors, scores[target])
        for within, source in enumerate(predecessors):
            key = (source, target)
            copula_index = len(predecessors) + within
            expected_coefficients[key] = float(beta[copula_index])
            expected_standard_errors[key] = float(standard_errors[copula_index])
            expected_t[key] = float(t_statistics[copula_index])
            expected_skewness[key] = skewness(scores[source])

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
    analysis = estimation["endogeneity"]
    actual_coefficients = {(row["source"], row["target"]): row["copula_coefficient"] for row in analysis["estimates"]}
    actual_standard_errors = {(row["source"], row["target"]): row["standard_error"] for row in analysis["estimates"]}
    actual_t = {(row["source"], row["target"]): row["t_statistic"] for row in analysis["estimates"]}
    actual_skewness = {(row["source"], row["target"]): row["predictor_skewness"] for row in analysis["estimates"]}
    actual_p_values = [row["p_value_two_sided"] for row in analysis["estimates"]]
    checks = {
        "method_version": estimation["method_version"] == "gaussian_copula_endogeneity_v1",
        "payload_version": analysis["method_version"] == "gaussian_copula_endogeneity_v1",
        "coefficient_delta": max_delta(actual_coefficients, expected_coefficients),
        "standard_error_delta": max_delta(actual_standard_errors, expected_standard_errors),
        "t_delta": max_delta(actual_t, expected_t),
        "skewness_delta": max_delta(actual_skewness, expected_skewness),
        "p_values_in_range": all(0.0 <= value <= 1.0 for value in actual_p_values),
        "has_experimental_warning": any("experimental" in warning.lower() for warning in analysis["warnings"]),
    }
    passed = (
        checks["method_version"]
        and checks["payload_version"]
        and checks["coefficient_delta"] <= TOLERANCE
        and checks["standard_error_delta"] <= TOLERANCE
        and checks["t_delta"] <= TOLERANCE
        and checks["skewness_delta"] <= TOLERANCE
        and checks["p_values_in_range"]
        and checks["has_experimental_warning"]
    )
    report = {
        "schema_version": 1,
        "kind": "gaussian_copula_endogeneity_reference_v1",
        "passed": passed,
        "tolerance": TOLERANCE,
        "checks": checks,
        "estimates": analysis["estimates"],
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={passed} | max_delta={max(checks['coefficient_delta'], checks['standard_error_delta'], checks['t_delta'], checks['skewness_delta']):.3g}")
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
