"""Mediation metamorphic and deterministic simulation checks.

This is validation evidence for the experimental `pls_mediation_v1` slice. It
uses single-item constructs so the independent reference is plain standardized
OLS, then checks QuickPLS invariance under row order, positive affine scaling,
and construct order changes.
"""

import csv
import json
import math
import random
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "mediation_metamorphic_report.json"
TOLERANCE = 1e-10


def standardize(values):
    mean = sum(values) / len(values)
    centered = [value - mean for value in values]
    variance = sum(value * value for value in centered) / (len(values) - 1)
    scale = math.sqrt(variance)
    if not math.isfinite(scale) or scale <= 2.220446049250313e-16:
        raise ValueError("cannot standardize a zero-variance vector")
    return [value / scale for value in centered]


def covariance(left, right):
    return sum(a * b for a, b in zip(left, right)) / (len(left) - 1)


def slope(predictor, outcome):
    return covariance(predictor, outcome) / covariance(predictor, predictor)


def independent_effects(rows):
    x = standardize([row["x"] for row in rows])
    m = standardize([row["m"] for row in rows])
    y = standardize([row["y"] for row in rows])
    path_xm = slope(x, m)
    path_my = slope(m, y)
    indirect_xy = path_xm * path_my
    return {
        ("x", "m"): {"direct": path_xm, "indirect": 0.0, "total": path_xm, "vaf": 0.0},
        ("m", "y"): {"direct": path_my, "indirect": 0.0, "total": path_my, "vaf": 0.0},
        ("x", "y"): {"direct": 0.0, "indirect": indirect_xy, "total": indirect_xy, "vaf": 1.0},
    }


def generated_rows(seed=20260719, n=96):
    rng = random.Random(seed)
    rows = []
    for _ in range(n):
        x = rng.gauss(0.0, 1.0)
        m = 0.62 * x + rng.gauss(0.0, 0.55)
        y = 0.74 * m + rng.gauss(0.0, 0.50)
        rows.append({"x": x, "m": m, "y": y})
    return rows


def positive_affine(rows):
    return [
        {"x": 3.0 + 2.5 * row["x"], "m": -7.0 + 1.7 * row["m"], "y": 11.0 + 4.2 * row["y"]}
        for row in rows
    ]


def reversed_rows(rows):
    return list(reversed(rows))


def permuted_mediator(rows):
    shifted = [row["m"] for row in rows[17:]] + [row["m"] for row in rows[:17]]
    return [{"x": row["x"], "m": shifted[index], "y": row["y"]} for index, row in enumerate(rows)]


def write_csv(path, rows):
    RESULTS.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["x", "m", "y"])
        writer.writeheader()
        for row in rows:
            writer.writerow({key: f"{row[key]:.12f}" for key in ["x", "m", "y"]})


def dataset_fingerprint(csv_path, stem):
    project_path = RESULTS / f"{stem}.fingerprint.qpls"
    subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "import",
            str(csv_path.relative_to(ROOT)),
            str(project_path.relative_to(ROOT)),
            "--name",
            stem,
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    completed = subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "inspect",
            str(project_path.relative_to(ROOT)),
            "--json",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    project_path.unlink(missing_ok=True)
    return json.loads(completed.stdout)["datasets"][0]["fingerprint"]


def write_recipe(path, fingerprint, stem, construct_order):
    construct_defs = {
        "x": {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x"]},
        "m": {"id": "m", "name": "M", "short_name": "M", "mode": "reflective", "indicators": ["m"]},
        "y": {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y"]},
    }
    recipe = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000057",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000058",
            "name": stem,
            "constructs": [construct_defs[item] for item in construct_order],
            "paths": [{"source": "x", "target": "m"}, {"source": "m", "target": "y"}],
        },
        "settings": {
            "method": "pls_pm",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "studentized_inner_samples": 0,
            "permutation_samples": 0,
            "seed": 20260719,
            "workers": 1,
            "confidence_level": 0.95,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {"fixture": "mediation_metamorphic"},
    }
    path.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")


def run_quickpls(stem, rows, construct_order=("x", "m", "y")):
    csv_path = RESULTS / f"{stem}.csv"
    recipe_path = RESULTS / f"{stem}.recipe.json"
    result_path = RESULTS / f"{stem}_quickpls.json"
    write_csv(csv_path, rows)
    write_recipe(recipe_path, dataset_fingerprint(csv_path, stem), stem, construct_order)
    subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "run",
            str(recipe_path.relative_to(ROOT)),
            "--data",
            str(csv_path.relative_to(ROOT)),
            "--output",
            str(result_path.relative_to(ROOT)),
            "--allow-experimental",
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    result = json.loads(result_path.read_text(encoding="utf-8"))
    estimates = result["payload"]["estimation"]["mediation"]["estimates"]
    return {(row["source"], row["target"]): row for row in estimates}


def max_difference(reference, actual):
    differences = []
    for pair, expected in reference.items():
        row = actual[pair]
        for metric in ["direct", "indirect", "total"]:
            differences.append(abs(row[metric] - expected[metric]))
        differences.append(abs(row["variance_accounted_for"] - expected["vaf"]))
    return max(differences)


def pairwise_max_difference(left, right):
    differences = []
    for pair, row in left.items():
        other = right[pair]
        for metric in ["direct", "indirect", "total", "variance_accounted_for"]:
            differences.append(abs(row[metric] - other[metric]))
    return max(differences)


def main():
    base_rows = generated_rows()
    base = run_quickpls("mediation_metamorphic_base", base_rows)
    base_reference = independent_effects(base_rows)
    reference_delta = max_difference(base_reference, base)

    affine = run_quickpls("mediation_metamorphic_affine", positive_affine(base_rows))
    reversed_result = run_quickpls("mediation_metamorphic_reversed", reversed_rows(base_rows))
    reordered = run_quickpls("mediation_metamorphic_reordered", base_rows, ("y", "x", "m"))
    broken = run_quickpls("mediation_metamorphic_broken", permuted_mediator(base_rows))

    baseline_indirect = abs(base[("x", "y")]["indirect"])
    broken_indirect = abs(broken[("x", "y")]["indirect"])
    checks = {
        "independent_reference_max_delta": reference_delta,
        "positive_affine_max_delta": pairwise_max_difference(base, affine),
        "row_reversal_max_delta": pairwise_max_difference(base, reversed_result),
        "construct_reorder_max_delta": pairwise_max_difference(base, reordered),
        "baseline_indirect_abs": baseline_indirect,
        "permuted_mediator_indirect_abs": broken_indirect,
        "permuted_mediator_drop_ratio": broken_indirect / baseline_indirect,
    }
    passed = (
        checks["independent_reference_max_delta"] <= TOLERANCE
        and checks["positive_affine_max_delta"] <= TOLERANCE
        and checks["row_reversal_max_delta"] <= TOLERANCE
        and checks["construct_reorder_max_delta"] <= TOLERANCE
        and checks["baseline_indirect_abs"] >= 0.35
        and checks["permuted_mediator_drop_ratio"] <= 0.5
    )
    report = {
        "schema_version": 1,
        "kind": "pls_mediation_metamorphic_v1",
        "tolerance": TOLERANCE,
        "passed": passed,
        "checks": checks,
        "scenarios": [
            "base_generated_signal",
            "positive_affine_transform",
            "row_reversal",
            "construct_reorder",
            "permuted_mediator_degradation",
        ],
        "note": "Bounded deterministic simulation and metamorphic screen; not a full Monte Carlo qualification.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"wrote {OUTPUT} | passed={passed} | "
        f"reference_delta={checks['independent_reference_max_delta']:.3g} | "
        f"drop_ratio={checks['permuted_mediator_drop_ratio']:.3g}"
    )
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
