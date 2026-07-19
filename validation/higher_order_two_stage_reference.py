"""Independent and metamorphic checks for two-stage higher-order constructs.

The fixture keeps lower-order components single-indicator so the independent
stage-one score calculation is explicit: each component score is the
standardized source indicator. Stage two then estimates the HOC using generated
component-score indicators, matching QuickPLS' public two-stage contract.
"""

import copy
import csv
import json
import random
from pathlib import Path
from subprocess import DEVNULL

import higher_order_reference as ref


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "higher_order_two_stage_reference_report.json"
TOLERANCE = 1e-6


def generated_rows(seed=20260719, n=120):
    rng = random.Random(seed)
    rows = []
    for _ in range(n):
        x_latent = rng.gauss(0.0, 1.0)
        z_latent = 0.20 * x_latent + rng.gauss(0.0, 1.0)
        y_latent = 0.44 * x_latent + 0.58 * z_latent + rng.gauss(0.0, 0.12)
        rows.append({
            "x1": x_latent + rng.gauss(0.0, 0.03),
            "z1": z_latent + rng.gauss(0.0, 0.03),
            "y1": y_latent + rng.gauss(0.0, 0.03),
        })
    return rows


def positive_affine(rows):
    return [
        {
            "x1": 5.0 + 2.4 * row["x1"],
            "z1": -4.0 + 1.8 * row["z1"],
            "y1": 9.0 + 3.1 * row["y1"],
        }
        for row in rows
    ]


def reversed_rows(rows):
    return list(reversed(rows))


def permuted_component(rows):
    shifted_z = [row["z1"] for row in rows[37:]] + [row["z1"] for row in rows[:37]]
    return [
        {
            "x1": row["x1"],
            "z1": shifted_z[index],
            "y1": row["y1"],
        }
        for index, row in enumerate(rows)
    ]


def write_csv(path, rows):
    RESULTS.mkdir(parents=True, exist_ok=True)
    fields = ["x1", "z1", "y1"]
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            writer.writerow({field: f"{row[field]:.12f}" for field in fields})


def dataset_fingerprint(csv_path, stem):
    project_path = RESULTS / f"{stem}.fingerprint.qpls"
    ref.qpls_cli(
        [
            "import",
            str(csv_path.relative_to(ROOT)),
            str(project_path.relative_to(ROOT)),
            "--name",
            stem,
        ],
        stdout=DEVNULL,
    )
    completed = ref.qpls_cli(
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


def write_recipe(path, fingerprint, stem, construct_order, components):
    construct_defs = {
        "x": {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1"]},
        "z": {"id": "z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1"]},
        "hoc": {"id": "hoc", "name": "HOC", "short_name": "HOC", "mode": "reflective", "indicators": []},
        "y": {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1"]},
    }
    recipe = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000083",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000084",
            "name": stem,
            "constructs": [construct_defs[item] for item in construct_order],
            "paths": [{"source": "hoc", "target": "y"}],
            "higher_order_constructs": [
                {
                    "id": "hoc",
                    "components": list(components),
                    "method": "two_stage",
                    "stage_one_recipe": None,
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
        "metadata": {"fixture": "higher_order_two_stage_reference"},
    }
    path.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")
    return recipe


def run_quickpls(stem, rows, construct_order=("x", "z", "hoc", "y"), components=("x", "z")):
    csv_path = RESULTS / f"{stem}.csv"
    recipe_path = RESULTS / f"{stem}.recipe.json"
    result_path = RESULTS / f"{stem}_quickpls.json"
    write_csv(csv_path, rows)
    recipe = write_recipe(
        recipe_path,
        dataset_fingerprint(csv_path, stem),
        stem,
        construct_order,
        components,
    )
    ref.qpls_cli(
        [
            "run",
            str(recipe_path.relative_to(ROOT)),
            "--data",
            str(csv_path.relative_to(ROOT)),
            "--output",
            str(result_path.relative_to(ROOT)),
            "--allow-experimental",
        ],
        stdout=DEVNULL,
    )
    result = json.loads(result_path.read_text(encoding="utf-8"))
    estimation = result["payload"]["estimation"]
    assessment = result["payload"].get("assessment", {})
    return {
        "paths": {
            (row["source"], row["target"]): row["coefficient"]
            for row in estimation["paths"]
        },
        "hoc_outer": {
            row["indicator"]: {"loading": row["loading"], "weight": row["weight"]}
            for row in estimation["outer_estimates"]
            if row["construct"] == "hoc"
        },
        "warnings": estimation["warnings"],
        "assessment_constructs": [
            row["construct"] for row in assessment.get("construct_quality", [])
        ],
        "recipe": recipe,
    }


def generated_indicator(hoc_id, component_id):
    return f"__qpls_hoc_{hoc_id}_{component_id}"


def stage_two_reference(rows, recipe):
    columns = {name: [row[name] for row in rows] for name in rows[0]}
    expanded_columns = dict(columns)
    for component in recipe["model"]["higher_order_constructs"][0]["components"]:
        indicator = next(
            construct["indicators"][0]
            for construct in recipe["model"]["constructs"]
            if construct["id"] == component
        )
        expanded_columns[generated_indicator("hoc", component)] = ref.standardize(columns[indicator])

    stage_two = copy.deepcopy(recipe)
    constructs = {construct["id"]: construct for construct in stage_two["model"]["constructs"]}
    constructs["hoc"]["indicators"] = [
        generated_indicator("hoc", component)
        for component in recipe["model"]["higher_order_constructs"][0]["components"]
    ]
    stage_two["model"]["constructs"] = [
        construct for construct in stage_two["model"]["constructs"]
        if construct["id"] in {"hoc", "y"}
    ]
    stage_two["model"]["higher_order_constructs"] = []
    return ref.estimate_pls(expanded_columns, stage_two)


def max_reference_delta(rows, result, recipe):
    reference = stage_two_reference(rows, recipe)
    deltas = []
    for pair, coefficient in reference["paths"].items():
        deltas.append(abs(coefficient - result["paths"][pair]))
    for (construct, indicator), metrics in reference["outer"].items():
        if construct != "hoc":
            continue
        other = result["hoc_outer"][indicator]
        deltas.append(abs(metrics["loading"] - other["loading"]))
        deltas.append(abs(metrics["weight"] - other["weight"]))
    return max(deltas)


def max_result_delta(left, right):
    deltas = []
    for pair, coefficient in left["paths"].items():
        deltas.append(abs(coefficient - right["paths"][pair]))
    for indicator, metrics in left["hoc_outer"].items():
        other = right["hoc_outer"][indicator]
        deltas.append(abs(metrics["loading"] - other["loading"]))
        deltas.append(abs(metrics["weight"] - other["weight"]))
    return max(deltas)


def main():
    base_rows = generated_rows()
    base = run_quickpls("higher_order_two_stage_base", base_rows)
    affine = run_quickpls("higher_order_two_stage_affine", positive_affine(base_rows))
    reversed_result = run_quickpls("higher_order_two_stage_reversed", reversed_rows(base_rows))
    reordered = run_quickpls(
        "higher_order_two_stage_reordered",
        base_rows,
        ("y", "hoc", "z", "x"),
    )
    component_reordered = run_quickpls(
        "higher_order_two_stage_component_reordered",
        base_rows,
        components=("z", "x"),
    )
    broken = run_quickpls("higher_order_two_stage_broken", permuted_component(base_rows))

    baseline_hoc_to_y = abs(base["paths"][("hoc", "y")])
    broken_hoc_to_y = abs(broken["paths"][("hoc", "y")])
    checks = {
        "independent_reference_max_delta": max_reference_delta(base_rows, base, base["recipe"]),
        "positive_affine_max_delta": max_result_delta(base, affine),
        "row_reversal_max_delta": max_result_delta(base, reversed_result),
        "construct_reorder_max_delta": max_result_delta(base, reordered),
        "component_reorder_max_delta": max_result_delta(base, component_reordered),
        "baseline_hoc_to_y_abs": baseline_hoc_to_y,
        "permuted_component_hoc_to_y_abs": broken_hoc_to_y,
        "permuted_component_drop_ratio": broken_hoc_to_y / baseline_hoc_to_y,
        "experimental_warning_present": any(
            "Two-stage higher-order constructs are experimental" in warning
            for warning in base["warnings"]
        ),
        "assessment_hoc_present": "hoc" in base["assessment_constructs"],
    }
    passed = (
        checks["independent_reference_max_delta"] <= TOLERANCE
        and checks["positive_affine_max_delta"] <= TOLERANCE
        and checks["row_reversal_max_delta"] <= TOLERANCE
        and checks["construct_reorder_max_delta"] <= TOLERANCE
        and checks["component_reorder_max_delta"] <= TOLERANCE
        and checks["baseline_hoc_to_y_abs"] >= 0.80
        and checks["permuted_component_drop_ratio"] <= 0.90
        and checks["experimental_warning_present"]
        and checks["assessment_hoc_present"]
    )
    report = {
        "schema_version": 1,
        "kind": "two_stage_hoc_reference_metamorphic_v1",
        "tolerance": TOLERANCE,
        "passed": passed,
        "checks": checks,
        "scenarios": [
            "base_generated_signal",
            "positive_affine_transform",
            "row_reversal",
            "construct_reorder",
            "component_reorder",
            "permuted_component_degradation",
        ],
        "note": "Bounded deterministic two-stage HOC screen; not a full Monte Carlo qualification.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"wrote {OUTPUT} | passed={passed} | "
        f"reference_delta={checks['independent_reference_max_delta']:.3g} | "
        f"drop_ratio={checks['permuted_component_drop_ratio']:.3g}"
    )
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
