"""Independent two-stage moderation reference and metamorphic checks.

The fixture uses single-item predictor, moderator, outcome, and generated
product constructs. Under QuickPLS' current two-stage contract, stage-1 scores
for the predictor and moderator reduce to sample-standardized observed
variables. The stage-2 product indicator is the product of those scores, then
standardized before the final structural regression.
"""

import csv
import json
import math
import random
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "moderation_reference_report.json"
TOLERANCE = 1e-10
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"


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


def standardize(values):
    mean = sum(values) / len(values)
    centered = [value - mean for value in values]
    variance = sum(value * value for value in centered) / (len(values) - 1)
    scale = math.sqrt(variance)
    if not math.isfinite(scale) or scale <= 2.220446049250313e-16:
        raise ValueError("cannot standardize a zero-variance vector")
    return [value / scale for value in centered]


def transpose(matrix):
    return [list(column) for column in zip(*matrix)]


def solve_linear(system, rhs):
    size = len(rhs)
    augmented = [list(row) + [rhs[index]] for index, row in enumerate(system)]
    for pivot_index in range(size):
        pivot_row = max(
            range(pivot_index, size),
            key=lambda row: abs(augmented[row][pivot_index]),
        )
        if abs(augmented[pivot_row][pivot_index]) <= 1e-14:
            raise ValueError("singular independent reference regression")
        augmented[pivot_index], augmented[pivot_row] = (
            augmented[pivot_row],
            augmented[pivot_index],
        )
        pivot = augmented[pivot_index][pivot_index]
        for column in range(pivot_index, size + 1):
            augmented[pivot_index][column] /= pivot
        for row in range(size):
            if row == pivot_index:
                continue
            factor = augmented[row][pivot_index]
            for column in range(pivot_index, size + 1):
                augmented[row][column] -= factor * augmented[pivot_index][column]
    return [augmented[row][size] for row in range(size)]


def ols_coefficients(predictors, outcome):
    columns = predictors
    rows = transpose(columns)
    xtx = [
        [sum(row[left] * row[right] for row in rows) for right in range(len(columns))]
        for left in range(len(columns))
    ]
    xty = [sum(row[column] * y for row, y in zip(rows, outcome)) for column in range(len(columns))]
    return solve_linear(xtx, xty)


def generated_rows(seed=20260719, n=120):
    rng = random.Random(seed)
    rows = []
    for index in range(n):
        x = rng.gauss(0.0, 1.0)
        m = 0.20 * x + rng.gauss(0.0, 1.0)
        interaction = x * m
        y = 0.35 * x + 0.25 * m + 0.85 * interaction + rng.gauss(0.0, 0.18)
        if index % 11 == 0:
            y += 0.03
        rows.append({"x": x, "m": m, "y": y})
    return rows


def positive_affine(rows):
    return [
        {
            "x": 4.0 + 2.3 * row["x"],
            "m": -6.0 + 1.9 * row["m"],
            "y": 12.0 + 3.1 * row["y"],
        }
        for row in rows
    ]


def reversed_rows(rows):
    return list(reversed(rows))


def permuted_moderator(rows):
    shifted = [row["m"] for row in rows[23:]] + [row["m"] for row in rows[:23]]
    return [{"x": row["x"], "m": shifted[index], "y": row["y"]} for index, row in enumerate(rows)]


def with_missing_values(rows):
    output = [dict(row) for row in rows]
    output[7]["x"] = None
    output[41]["m"] = None
    output[89]["y"] = None
    return output


def complete_rows(rows):
    return [
        row
        for row in rows
        if row["x"] is not None and row["m"] is not None and row["y"] is not None
    ]


def write_csv(path, rows):
    RESULTS.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["x", "m", "y"])
        writer.writeheader()
        for row in rows:
            writer.writerow({
                key: "" if row[key] is None else f"{row[key]:.12f}"
                for key in ["x", "m", "y"]
            })


def dataset_fingerprint(csv_path, stem):
    project_path = RESULTS / f"{stem}.fingerprint.qpls"
    qpls_cli(
        [
            "import",
            str(csv_path.relative_to(ROOT)),
            str(project_path.relative_to(ROOT)),
            "--name",
            stem,
        ],
        stdout=subprocess.DEVNULL,
    )
    completed = qpls_cli(
        [
            "inspect",
            str(project_path.relative_to(ROOT)),
            "--json",
        ],
        capture_output=True,
        text=True,
    )
    project_path.unlink(missing_ok=True)
    return json.loads(completed.stdout)["datasets"][0]["fingerprint"]


def write_recipe(path, fingerprint, stem, construct_order):
    construct_defs = {
        "x": {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x"]},
        "m": {"id": "m", "name": "M", "short_name": "M", "mode": "reflective", "indicators": ["m"]},
        "xm": {"id": "xm", "name": "X by M", "short_name": "XM", "mode": "formative", "indicators": []},
        "y": {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y"]},
    }
    recipe = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000059",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000060",
            "name": stem,
            "constructs": [construct_defs[item] for item in construct_order],
            "paths": [
                {"source": "x", "target": "y"},
                {"source": "m", "target": "y"},
                {"source": "xm", "target": "y"},
            ],
            "interactions": [
                {
                    "id": "x_by_m_to_y",
                    "predictor": "x",
                    "moderator": "m",
                    "product_construct": "xm",
                    "outcome": "y",
                    "method": "two_stage_product_score",
                }
            ],
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
        "metadata": {"fixture": "moderation_reference"},
    }
    path.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")


def run_quickpls(stem, rows, construct_order=("x", "m", "xm", "y")):
    csv_path = RESULTS / f"{stem}.csv"
    recipe_path = RESULTS / f"{stem}.recipe.json"
    result_path = RESULTS / f"{stem}_quickpls.json"
    write_csv(csv_path, rows)
    write_recipe(recipe_path, dataset_fingerprint(csv_path, stem), stem, construct_order)
    qpls_cli(
        [
            "run",
            str(recipe_path.relative_to(ROOT)),
            "--data",
            str(csv_path.relative_to(ROOT)),
            "--output",
            str(result_path.relative_to(ROOT)),
            "--allow-experimental",
        ],
        stdout=subprocess.DEVNULL,
    )
    result = json.loads(result_path.read_text(encoding="utf-8"))
    paths = {
        (row["source"], row["target"]): row["coefficient"]
        for row in result["payload"]["estimation"]["paths"]
    }
    moderation = result["payload"]["estimation"]["moderation"]["estimates"]
    estimation = result["payload"]["estimation"]
    warnings = [diagnostic["message"] for diagnostic in result["diagnostics"]]
    warnings.extend(result["payload"]["estimation"]["warnings"])
    return {
        "paths": paths,
        "moderation": moderation,
        "warnings": warnings,
        "used_observations": estimation["used_observations"],
        "omitted_observations": estimation["omitted_observations"],
    }


def independent_reference(rows):
    rows = complete_rows(rows)
    x = standardize([row["x"] for row in rows])
    m = standardize([row["m"] for row in rows])
    y = standardize([row["y"] for row in rows])
    product = standardize([x_value * m_value for x_value, m_value in zip(x, m)])
    coefficients = ols_coefficients([x, m, product], y)
    return {("x", "y"): coefficients[0], ("m", "y"): coefficients[1], ("xm", "y"): coefficients[2]}


def max_path_difference(reference, actual):
    return max(abs(expected - actual[pair]) for pair, expected in reference.items())


def pairwise_max_path_difference(left, right):
    return max(abs(coefficient - right[pair]) for pair, coefficient in left.items())


def simple_slopes(reference):
    main = reference[("x", "y")]
    interaction = reference[("xm", "y")]
    return {-1.0: main - interaction, 0.0: main, 1.0: main + interaction}


def simple_slope_max_difference(reference, actual):
    actual_estimate = actual["moderation"][0]
    actual_slopes = {
        row["moderator_score"]: row["effect"] for row in actual_estimate["simple_slopes"]
    }
    return max(
        abs(expected - actual_slopes[level])
        for level, expected in simple_slopes(reference).items()
    )


def main():
    base_rows = generated_rows()
    base = run_quickpls("moderation_reference_base", base_rows)
    reference = independent_reference(base_rows)
    reference_delta = max_path_difference(reference, base["paths"])

    affine = run_quickpls("moderation_reference_affine", positive_affine(base_rows))
    reversed_result = run_quickpls("moderation_reference_reversed", reversed_rows(base_rows))
    reordered = run_quickpls(
        "moderation_reference_reordered",
        base_rows,
        ("y", "xm", "m", "x"),
    )
    missing_rows = with_missing_values(base_rows)
    missing = run_quickpls("moderation_reference_missing", missing_rows)
    broken = run_quickpls("moderation_reference_broken", permuted_moderator(base_rows))

    baseline_interaction = abs(base["paths"][("xm", "y")])
    broken_interaction = abs(broken["paths"][("xm", "y")])
    checks = {
        "independent_reference_max_delta": reference_delta,
        "simple_slope_reference_max_delta": simple_slope_max_difference(reference, base),
        "positive_affine_max_delta": pairwise_max_path_difference(base["paths"], affine["paths"]),
        "row_reversal_max_delta": pairwise_max_path_difference(base["paths"], reversed_result["paths"]),
        "construct_reorder_max_delta": pairwise_max_path_difference(base["paths"], reordered["paths"]),
        "missing_data_reference_max_delta": max_path_difference(independent_reference(missing_rows), missing["paths"]),
        "missing_data_used_observations": missing["used_observations"],
        "missing_data_omitted_observations": missing["omitted_observations"],
        "baseline_interaction_abs": baseline_interaction,
        "permuted_moderator_interaction_abs": broken_interaction,
        "permuted_moderator_drop_ratio": broken_interaction / baseline_interaction,
        "experimental_warning_present": any(
            "Two-stage moderation is experimental" in warning for warning in base["warnings"]
        ),
    }
    passed = (
        checks["independent_reference_max_delta"] <= TOLERANCE
        and checks["simple_slope_reference_max_delta"] <= TOLERANCE
        and checks["positive_affine_max_delta"] <= TOLERANCE
        and checks["row_reversal_max_delta"] <= TOLERANCE
        and checks["construct_reorder_max_delta"] <= TOLERANCE
        and checks["missing_data_reference_max_delta"] <= TOLERANCE
        and checks["missing_data_used_observations"] == 117
        and checks["missing_data_omitted_observations"] == 3
        and checks["baseline_interaction_abs"] >= 0.50
        and checks["permuted_moderator_drop_ratio"] <= 0.65
        and checks["experimental_warning_present"]
    )
    report = {
        "schema_version": 1,
        "kind": "pls_two_stage_moderation_reference_v1",
        "tolerance": TOLERANCE,
        "passed": passed,
        "checks": checks,
        "reference_paths": {f"{source}->{target}": value for (source, target), value in reference.items()},
        "quickpls_paths": {f"{source}->{target}": value for (source, target), value in base["paths"].items()},
        "scenarios": [
            "base_generated_interaction_signal",
            "positive_affine_transform",
            "row_reversal",
            "construct_reorder",
            "missing_data_listwise_row_mapping",
            "permuted_moderator_degradation",
        ],
        "note": "Independent single-item two-stage moderation fixture and bounded metamorphic screen; not a full publication-validation suite.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"wrote {OUTPUT} | passed={passed} | "
        f"reference_delta={checks['independent_reference_max_delta']:.3g} | "
        f"drop_ratio={checks['permuted_moderator_drop_ratio']:.3g}"
    )
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
