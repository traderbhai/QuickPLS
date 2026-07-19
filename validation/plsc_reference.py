"""Independent PLSc attenuation-correction reference.

The fixture estimates ordinary PLS with the local Python reference
implementation, then applies the PLSc reflective correction independently:
rho_A reliability, attenuation-corrected construct correlations, corrected
structural paths, corrected R2, and corrected outer loadings.
"""

import csv
import json
import math
import random
import subprocess
from pathlib import Path

import numpy as np

from higher_order_reference import correlation, estimate_pls


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "plsc_reference.csv"
RECIPE = RESULTS / "plsc_reference.recipe.json"
QUICKPLS = RESULTS / "plsc_reference_quickpls.json"
OUTPUT = RESULTS / "plsc_reference_report.json"
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


def generated_rows(seed=20260719, n=120):
    rng = random.Random(seed)
    rows = []
    for _ in range(n):
        x = rng.gauss(0.0, 1.0)
        z = 0.32 * x + rng.gauss(0.0, 0.92)
        y = 0.30 * x + 0.26 * z + rng.gauss(0.0, 0.70)
        rows.append(
            {
                "x1": x + rng.gauss(0.0, 0.10),
                "x2": 0.91 * x + rng.gauss(0.0, 0.12),
                "z1": z + rng.gauss(0.0, 0.11),
                "z2": 0.89 * z + rng.gauss(0.0, 0.13),
                "y1": y + rng.gauss(0.0, 0.10),
                "y2": 0.94 * y + rng.gauss(0.0, 0.11),
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
    project_path = RESULTS / "plsc_reference.fingerprint.qpls"
    qpls_cli(
        [
            "import",
            str(DATA.relative_to(ROOT)),
            str(project_path.relative_to(ROOT)),
            "--name",
            "plsc_reference",
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
        "id": "00000000-0000-0000-0000-00000000plsc".replace("plsc", "0003"),
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000004",
            "name": "PLSc reflective reference",
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
            "method": "plsc",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260719,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {"fixture": "independent_plsc_reference"},
    }


def rho_a(columns, weights):
    count = len(columns)
    indicator_correlation = np.eye(count)
    for left in range(count):
        for right in range(left + 1, count):
            value = correlation(columns[left], columns[right])
            indicator_correlation[left, right] = value
            indicator_correlation[right, left] = value
    weights = np.asarray(weights, dtype=float)
    numerator = float(weights @ indicator_correlation @ weights) ** 2
    error_correlation = indicator_correlation.copy()
    squared_weight_sum = float(np.sum(weights * weights))
    for index in range(count):
        error_correlation[index, index] = 1.0 - squared_weight_sum
    denominator = numerator + float(weights @ error_correlation @ weights)
    return numerator / denominator


def solve_paths(corrected, construct_ids, paths):
    index = {construct_id: position for position, construct_id in enumerate(construct_ids)}
    estimates = {}
    r_squared = {}
    for target in construct_ids:
        predecessors = [path["source"] for path in paths if path["target"] == target]
        if not predecessors:
            continue
        predictor_indices = [index[source] for source in predecessors]
        target_index = index[target]
        system = corrected[np.ix_(predictor_indices, predictor_indices)]
        rhs = corrected[predictor_indices, target_index]
        coefficients = np.linalg.solve(system, rhs)
        r2 = float(np.dot(coefficients, rhs))
        r_squared[target] = max(0.0, min(1.0, r2))
        for source, coefficient in zip(predecessors, coefficients):
            estimates[(source, target)] = float(coefficient)
    return estimates, r_squared


def max_delta(actual, expected):
    deltas = []
    for key, expected_value in expected.items():
        deltas.append(abs(actual[key] - expected_value))
    return max(deltas) if deltas else 0.0


def main():
    rows = generated_rows()
    write_dataset(rows)
    fingerprint = dataset_fingerprint()
    recipe = recipe_payload(fingerprint)
    RECIPE.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")

    columns = {name: np.asarray([float(row[name]) for row in rows], dtype=float) for name in rows[0]}
    reference = estimate_pls(columns, recipe)
    construct_ids = [construct["id"] for construct in recipe["model"]["constructs"]]
    reliabilities = {}
    for construct in recipe["model"]["constructs"]:
        block_columns = [columns[indicator] for indicator in construct["indicators"]]
        block_weights = [reference["weights"][construct["id"]][indicator] for indicator in construct["indicators"]]
        reliabilities[construct["id"]] = rho_a(block_columns, block_weights)

    corrected = np.eye(len(construct_ids))
    for left, left_id in enumerate(construct_ids):
        for right in range(left + 1, len(construct_ids)):
            right_id = construct_ids[right]
            original = correlation(reference["scores"][left_id], reference["scores"][right_id])
            value = original / math.sqrt(reliabilities[left_id] * reliabilities[right_id])
            corrected[left, right] = max(-1.0, min(1.0, value))
            corrected[right, left] = corrected[left, right]

    expected_paths, expected_r2 = solve_paths(corrected, construct_ids, recipe["model"]["paths"])
    expected_loadings = {}
    for (construct, indicator), outer in reference["outer"].items():
        expected_loadings[(construct, indicator)] = max(
            -1.0,
            min(1.0, outer["loading"] / math.sqrt(reliabilities[construct])),
        )

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
    plsc = estimation["plsc"]
    actual_paths = {(path["source"], path["target"]): path["coefficient"] for path in estimation["paths"]}
    actual_r2 = estimation["r_squared"]
    actual_rho = {entry["construct"]: entry["rho_a"] for entry in plsc["reliabilities"]}
    actual_loadings = {
        (entry["construct"], entry["indicator"]): entry["loading"]
        for entry in plsc["corrected_outer_loadings"]
    }
    checks = {
        "method_version": estimation["method_version"] == "plsc_v1",
        "plsc_payload_version": plsc["method_version"] == "plsc_v1",
        "path_delta": max_delta(actual_paths, expected_paths),
        "r2_delta": max_delta(actual_r2, expected_r2),
        "rho_a_delta": max_delta(actual_rho, reliabilities),
        "loading_delta": max_delta(actual_loadings, expected_loadings),
    }
    passed = (
        checks["method_version"]
        and checks["plsc_payload_version"]
        and checks["path_delta"] <= TOLERANCE
        and checks["r2_delta"] <= TOLERANCE
        and checks["rho_a_delta"] <= TOLERANCE
        and checks["loading_delta"] <= TOLERANCE
    )
    report = {
        "schema_version": 1,
        "kind": "plsc_reference_v1",
        "passed": passed,
        "tolerance": TOLERANCE,
        "checks": checks,
        "quickpls": {
            "method_version": estimation["method_version"],
            "paths": [
                {"source": source, "target": target, "coefficient": coefficient}
                for (source, target), coefficient in sorted(actual_paths.items())
            ],
            "r_squared": actual_r2,
            "rho_a": actual_rho,
        },
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={passed} | max_delta={max(checks[k] for k in ['path_delta', 'r2_delta', 'rho_a_delta', 'loading_delta']):.3g}")
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
