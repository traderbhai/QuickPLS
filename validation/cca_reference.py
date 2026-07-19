"""Independent CCA composite-correlation residual reference."""

import csv
import json
import random
import subprocess
from pathlib import Path

import numpy as np

from higher_order_reference import estimate_pls


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "cca_reference.csv"
RECIPE = RESULTS / "cca_reference.recipe.json"
QUICKPLS = RESULTS / "cca_reference_quickpls.json"
OUTPUT = RESULTS / "cca_reference_report.json"
GUARD_RECIPE = RESULTS / "cca_invalid.recipe.json"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
TOLERANCE = 1e-10
CLI_READY = False


def ensure_cli():
    global CLI_READY
    if CLI_READY:
        return CLI_EXE
    subprocess.run(
        ["cargo", "build", "-p", "qpls-cli"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    CLI_READY = True
    return CLI_EXE


def qpls(args, **kwargs):
    return subprocess.run([str(ensure_cli()), *args], cwd=ROOT, **kwargs)


def generated_rows(seed=20260719, n=132):
    rng = random.Random(seed)
    rows = []
    for _ in range(n):
        x = rng.gauss(0.0, 1.0)
        z = 0.45 * x + rng.gauss(0.0, 0.75)
        y = 0.35 * x + 0.55 * z + rng.gauss(0.0, 0.50)
        rows.append(
            {
                "x1": 0.91 * x + rng.gauss(0.0, 0.16),
                "x2": 0.84 * x + rng.gauss(0.0, 0.19),
                "z1": 0.88 * z + rng.gauss(0.0, 0.17),
                "z2": 0.80 * z + rng.gauss(0.0, 0.21),
                "y1": 0.93 * y + rng.gauss(0.0, 0.15),
                "y2": 0.79 * y + rng.gauss(0.0, 0.22),
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
    project_path = RESULTS / "cca_reference.fingerprint.qpls"
    qpls(
        ["import", str(DATA.relative_to(ROOT)), str(project_path.relative_to(ROOT)), "--name", "cca_reference"],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    payload = json.loads(
        qpls(
            ["inspect", str(project_path.relative_to(ROOT)), "--json"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    )
    return payload["datasets"][0]["fingerprint"]


def recipe_payload(fingerprint):
    return {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000017",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000018",
            "name": "CCA composite residual reference",
            "constructs": [
                {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2"]},
                {"id": "z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1", "z2"]},
                {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2"]},
            ],
            "paths": [
                {"source": "x", "target": "z"},
                {"source": "x", "target": "y"},
                {"source": "z", "target": "y"},
            ],
        },
        "settings": {
            "method": "cca",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260719,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {"fixture": "independent_cca_reference"},
    }


def correlation(left, right):
    return float(np.cov(np.asarray(left), np.asarray(right), ddof=1)[0, 1] / (np.std(left, ddof=1) * np.std(right, ddof=1)))


def reference_cca(rows, recipe):
    columns = {
        name: [row[name] for row in rows]
        for name in ["x1", "x2", "z1", "z2", "y1", "y2"]
    }
    pls = estimate_pls(columns, recipe)
    construct_ids = [construct["id"] for construct in recipe["model"]["constructs"]]
    index = {construct: position for position, construct in enumerate(construct_ids)}
    count = len(construct_ids)
    observed = np.eye(count)
    for row in range(count):
        for column in range(row):
            value = correlation(pls["scores"][construct_ids[row]], pls["scores"][construct_ids[column]])
            observed[row, column] = value
            observed[column, row] = value
    structural = np.zeros((count, count))
    for (source, target), coefficient in pls["paths"].items():
        structural[index[target], index[source]] = coefficient
    system = np.eye(count) - structural
    endogenous = {path["target"] for path in recipe["model"]["paths"]}
    residual_covariance = np.zeros((count, count))
    for row, construct_id in enumerate(construct_ids):
        if construct_id in endogenous:
            predecessors = [path["source"] for path in recipe["model"]["paths"] if path["target"] == construct_id]
            r2 = sum(pls["paths"][(source, construct_id)] * observed[index[source], row] for source in predecessors)
            residual_covariance[row, row] = max(0.0, 1.0 - r2)
        else:
            residual_covariance[row, row] = 1.0
            for column in range(row):
                if construct_ids[column] not in endogenous:
                    residual_covariance[row, column] = observed[row, column]
                    residual_covariance[column, row] = observed[row, column]
    inverse = np.linalg.solve(system, np.eye(count))
    reproduced = inverse @ residual_covariance @ inverse.T
    rows_out = {}
    for row in range(count):
        for column in range(row):
            key = (construct_ids[column], construct_ids[row])
            rows_out[key] = {
                "observed": float(observed[row, column]),
                "reproduced": float(reproduced[row, column]),
                "residual": float(observed[row, column] - reproduced[row, column]),
                "absolute_residual": abs(float(observed[row, column] - reproduced[row, column])),
            }
    return rows_out


def run_quickpls():
    QUICKPLS.unlink(missing_ok=True)
    qpls(
        [
            "run",
            str(RECIPE.relative_to(ROOT)),
            "--data",
            str(DATA.relative_to(ROOT)),
            "--output",
            str(QUICKPLS.relative_to(ROOT)),
            "--allow-experimental",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    return json.loads(QUICKPLS.read_text(encoding="utf-8"))


def check_guard(fingerprint):
    recipe = recipe_payload(fingerprint)
    recipe["settings"]["weighting_scheme"] = "pca"
    recipe["metadata"]["fixture"] = "invalid_cca_pca"
    GUARD_RECIPE.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")
    validation = qpls(
        ["validate", str(GUARD_RECIPE.relative_to(ROOT)), "--json"],
        capture_output=True,
        text=True,
    )
    codes = [issue["code"] for issue in json.loads(validation.stdout)]
    return {
        "passed": validation.returncode != 0 and "cca.pca_unsupported" in codes,
        "validation_codes": codes,
    }


def main():
    rows = generated_rows()
    write_dataset(rows)
    fingerprint = dataset_fingerprint()
    recipe = recipe_payload(fingerprint)
    RECIPE.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")
    quickpls = run_quickpls()
    estimation = quickpls["payload"]["estimation"]
    analysis = estimation["cca"]
    expected = reference_cca(rows, recipe)
    observed = {
        (row["left"], row["right"]): row
        for row in analysis["correlations"]
    }
    deltas = {}
    for key, values in expected.items():
        for field, value in values.items():
            deltas[f"{field}::{key[0]}::{key[1]}"] = abs(observed[key][field] - value)
    deltas["max_absolute_residual"] = abs(
        analysis["max_absolute_residual"] - max(row["absolute_residual"] for row in expected.values())
    )
    max_delta = max(deltas.values())
    guard = check_guard(fingerprint)
    checks = {
        "method_version": estimation["method_version"] == "cca_composite_residual_v1",
        "payload_version": analysis["method_version"] == "cca_composite_residual_v1",
        "correlation_count": len(analysis["correlations"]) == 3,
        "max_delta_within_tolerance": max_delta <= TOLERANCE,
        "guard": guard["passed"],
    }
    report = {
        "schema_version": 1,
        "kind": "cca_reference_v1",
        "passed": all(checks.values()),
        "tolerance": TOLERANCE,
        "max_delta": max_delta,
        "checks": checks,
        "deltas": deltas,
        "guard": guard,
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={report['passed']} | max_delta={max_delta:.3g}")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
