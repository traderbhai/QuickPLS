"""Independent first-stage moderated mediation reference."""

import csv
import json
import math
import random
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "moderated_mediation_reference.csv"
RECIPE = RESULTS / "moderated_mediation_reference.recipe.json"
QUICKPLS = RESULTS / "moderated_mediation_reference_quickpls.json"
OUTPUT = RESULTS / "moderated_mediation_reference_report.json"
GUARD_RECIPE = RESULTS / "moderated_mediation_invalid.recipe.json"
GUARD_RESULT = RESULTS / "moderated_mediation_invalid_quickpls.json"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
TOLERANCE = 1e-10


def ensure_cli():
    if not CLI_EXE.exists():
        subprocess.run(
            ["cargo", "build", "-p", "qpls-cli"],
            cwd=ROOT,
            check=True,
            stdout=subprocess.DEVNULL,
        )
    return CLI_EXE


def qpls(args, **kwargs):
    return subprocess.run([str(ensure_cli()), *args], cwd=ROOT, **kwargs)


def standardize(values):
    mean = sum(values) / len(values)
    centered = [value - mean for value in values]
    variance = sum(value * value for value in centered) / (len(values) - 1)
    scale = math.sqrt(variance)
    if scale <= 2.220446049250313e-16 or not math.isfinite(scale):
        raise ValueError("zero variance")
    return [value / scale for value in centered]


def solve_linear(system, rhs):
    size = len(rhs)
    augmented = [list(row) + [rhs[index]] for index, row in enumerate(system)]
    for pivot in range(size):
        selected = max(range(pivot, size), key=lambda row: abs(augmented[row][pivot]))
        if abs(augmented[selected][pivot]) <= 1e-14:
            raise ValueError("singular reference regression")
        augmented[pivot], augmented[selected] = augmented[selected], augmented[pivot]
        divisor = augmented[pivot][pivot]
        for column in range(pivot, size + 1):
            augmented[pivot][column] /= divisor
        for row in range(size):
            if row == pivot:
                continue
            factor = augmented[row][pivot]
            for column in range(pivot, size + 1):
                augmented[row][column] -= factor * augmented[pivot][column]
    return [augmented[row][size] for row in range(size)]


def ols(predictors, outcome):
    xtx = [
        [sum(left * right for left, right in zip(predictors[i], predictors[j])) for j in range(len(predictors))]
        for i in range(len(predictors))
    ]
    xty = [sum(x * y for x, y in zip(predictor, outcome)) for predictor in predictors]
    return solve_linear(xtx, xty)


def generated_rows(seed=20260719, n=128):
    rng = random.Random(seed)
    rows = []
    for _ in range(n):
        x = rng.gauss(0.0, 1.0)
        w = 0.18 * x + rng.gauss(0.0, 1.0)
        m = 0.42 * x + 0.22 * w + 0.72 * x * w + rng.gauss(0.0, 0.20)
        y = 0.63 * m + 0.18 * x + rng.gauss(0.0, 0.18)
        rows.append({"x": x, "w": w, "m": m, "y": y})
    return rows


def write_csv(rows):
    RESULTS.mkdir(parents=True, exist_ok=True)
    with DATA.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["x", "w", "m", "y"])
        writer.writeheader()
        for row in rows:
            writer.writerow({field: f"{row[field]:.12f}" for field in ["x", "w", "m", "y"]})


def dataset_fingerprint():
    project_path = RESULTS / "moderated_mediation_reference.fingerprint.qpls"
    qpls(
        [
            "import",
            str(DATA.relative_to(ROOT)),
            str(project_path.relative_to(ROOT)),
            "--name",
            "moderated_mediation_reference",
        ],
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


def recipe_payload(fingerprint, include_interaction=True):
    model = {
        "id": "00000000-0000-0000-0000-000000000010",
        "name": "First-stage moderated mediation reference",
        "constructs": [
            {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x"]},
            {"id": "w", "name": "W", "short_name": "W", "mode": "reflective", "indicators": ["w"]},
            {"id": "xw", "name": "X by W", "short_name": "XW", "mode": "formative", "indicators": []},
            {"id": "m", "name": "M", "short_name": "M", "mode": "reflective", "indicators": ["m"]},
            {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y"]},
        ],
        "paths": [
            {"source": "x", "target": "m"},
            {"source": "w", "target": "m"},
            {"source": "xw", "target": "m"},
            {"source": "m", "target": "y"},
            {"source": "x", "target": "y"},
        ],
    }
    if include_interaction:
        model["interactions"] = [
            {
                "id": "x_by_w_to_m",
                "predictor": "x",
                "moderator": "w",
                "product_construct": "xw",
                "outcome": "m",
                "method": "two_stage_product_score",
            }
        ]
    return {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000009",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": model,
        "settings": {
            "method": "moderated_mediation",
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
        "metadata": {"fixture": "independent_moderated_mediation_reference"},
    }


def expected(rows):
    x = standardize([row["x"] for row in rows])
    w = standardize([row["w"] for row in rows])
    m = standardize([row["m"] for row in rows])
    y = standardize([row["y"] for row in rows])
    product = standardize([left * right for left, right in zip(x, w)])
    a_x, _a_w, a_int = ols([x, w, product], m)
    b_m, _b_x = ols([m, x], y)
    conditional = {
        level: (a_x + a_int * level) * b_m
        for level in [-1.0, 0.0, 1.0]
    }
    return {
        "index": a_int * b_m,
        "conditional": conditional,
        "first_stage": {level: a_x + a_int * level for level in [-1.0, 0.0, 1.0]},
        "second_stage": b_m,
    }


def max_delta(actual, expected_values):
    deltas = [abs(actual["index_of_moderated_mediation"] - expected_values["index"])]
    for row in actual["conditional_indirect_effects"]:
        level = row["moderator_score"]
        deltas.append(abs(row["indirect_effect"] - expected_values["conditional"][level]))
        deltas.append(abs(row["first_stage_effect"] - expected_values["first_stage"][level]))
        deltas.append(abs(row["second_stage_effect"] - expected_values["second_stage"]))
    return max(deltas)


def invalid_guard(fingerprint):
    GUARD_RESULT.unlink(missing_ok=True)
    GUARD_RECIPE.write_text(
        json.dumps(recipe_payload(fingerprint, include_interaction=False), indent=2) + "\n",
        encoding="utf-8",
    )
    validation = qpls(
        ["validate", str(GUARD_RECIPE.relative_to(ROOT)), "--json"],
        capture_output=True,
        text=True,
    )
    issues = json.loads(validation.stdout)
    codes = [issue["code"] for issue in issues]
    run = qpls(
        [
            "run",
            str(GUARD_RECIPE.relative_to(ROOT)),
            "--data",
            str(DATA.relative_to(ROOT)),
            "--output",
            str(GUARD_RESULT.relative_to(ROOT)),
            "--allow-experimental",
        ],
        capture_output=True,
        text=True,
    )
    return {
        "validation_exit_nonzero": validation.returncode != 0,
        "validation_code_present": "moderated_mediation.interaction_required" in codes,
        "run_exit_nonzero": run.returncode != 0,
        "run_error_mentions_code": "moderated_mediation.interaction_required" in run.stderr
        or "moderated_mediation.interaction_required" in run.stdout,
        "result_not_written": not GUARD_RESULT.exists(),
    }


def main():
    rows = generated_rows()
    write_csv(rows)
    fingerprint = dataset_fingerprint()
    RECIPE.write_text(json.dumps(recipe_payload(fingerprint), indent=2) + "\n", encoding="utf-8")
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
    quickpls = json.loads(QUICKPLS.read_text(encoding="utf-8"))
    estimation = quickpls["payload"]["estimation"]
    analysis = estimation["moderated_mediation"]
    estimate = analysis["estimates"][0]
    expected_values = expected(rows)
    guard_checks = invalid_guard(fingerprint)
    checks = {
        "method_version": estimation["method_version"] == "pls_moderated_mediation_v1",
        "payload_version": analysis["method_version"] == "pls_moderated_mediation_v1",
        "stage": estimate["moderated_stage"] == "first_stage",
        "reference_delta": max_delta(estimate, expected_values),
        "has_experimental_warning": any("experimental" in warning.lower() for warning in analysis["warnings"]),
        "invalid_guard": all(guard_checks.values()),
    }
    passed = (
        checks["method_version"]
        and checks["payload_version"]
        and checks["stage"]
        and checks["reference_delta"] <= TOLERANCE
        and checks["has_experimental_warning"]
        and checks["invalid_guard"]
    )
    report = {
        "schema_version": 1,
        "kind": "moderated_mediation_reference_v1",
        "passed": passed,
        "tolerance": TOLERANCE,
        "checks": checks,
        "guard_checks": guard_checks,
        "estimate": estimate,
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={passed} | reference_delta={checks['reference_delta']:.3g}")
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
