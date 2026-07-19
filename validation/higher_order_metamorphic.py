"""Metamorphic checks for repeated-indicator higher-order constructs."""

import csv
import json
from pathlib import Path
from subprocess import DEVNULL

import higher_order_reference as ref


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "higher_order_metamorphic_report.json"
TOLERANCE = 1e-6


def positive_affine(rows):
    return [
        {
            "x1": 4.0 + 2.1 * row["x1"],
            "x2": -3.0 + 1.7 * row["x2"],
            "z1": 8.0 + 2.4 * row["z1"],
            "z2": -5.0 + 1.9 * row["z2"],
            "y1": 11.0 + 3.2 * row["y1"],
            "y2": -7.0 + 2.8 * row["y2"],
        }
        for row in rows
    ]


def reversed_rows(rows):
    return list(reversed(rows))


def permuted_component(rows):
    shifted_z1 = [row["z1"] for row in rows[19:]] + [row["z1"] for row in rows[:19]]
    shifted_z2 = [row["z2"] for row in rows[31:]] + [row["z2"] for row in rows[:31]]
    return [
        {
            "x1": row["x1"],
            "x2": row["x2"],
            "z1": shifted_z1[index],
            "z2": shifted_z2[index],
            "y1": row["y1"],
            "y2": row["y2"],
        }
        for index, row in enumerate(rows)
    ]


def write_csv(path, rows):
    RESULTS.mkdir(parents=True, exist_ok=True)
    fields = ["x1", "x2", "z1", "z2", "y1", "y2"]
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
        "x": {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2"]},
        "z": {"id": "z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1", "z2"]},
        "hoc": {"id": "hoc", "name": "HOC", "short_name": "HOC", "mode": "reflective", "indicators": []},
        "y": {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2"]},
    }
    recipe = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000073",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000074",
            "name": stem,
            "constructs": [construct_defs[item] for item in construct_order],
            "paths": [
                {"source": "x", "target": "hoc"},
                {"source": "z", "target": "hoc"},
                {"source": "hoc", "target": "y"},
            ],
            "higher_order_constructs": [
                {
                    "id": "hoc",
                    "components": list(components),
                    "method": "repeated_indicators",
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
        "metadata": {"fixture": "higher_order_metamorphic"},
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
        "recipe": recipe,
    }


def max_result_delta(left, right):
    deltas = []
    for pair, coefficient in left["paths"].items():
        deltas.append(abs(coefficient - right["paths"][pair]))
    for indicator, metrics in left["hoc_outer"].items():
        other = right["hoc_outer"][indicator]
        deltas.append(abs(metrics["loading"] - other["loading"]))
        deltas.append(abs(metrics["weight"] - other["weight"]))
    return max(deltas)


def max_reference_delta(rows, result, recipe):
    columns = {name: [row[name] for row in rows] for name in rows[0]}
    reference = ref.estimate_pls(columns, ref.expand_repeated_indicator_hoc(recipe))
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


def main():
    base_rows = ref.generated_rows()
    base = run_quickpls("higher_order_metamorphic_base", base_rows)
    affine = run_quickpls("higher_order_metamorphic_affine", positive_affine(base_rows))
    reversed_result = run_quickpls("higher_order_metamorphic_reversed", reversed_rows(base_rows))
    reordered = run_quickpls(
        "higher_order_metamorphic_reordered",
        base_rows,
        ("y", "hoc", "z", "x"),
    )
    component_reordered = run_quickpls(
        "higher_order_metamorphic_component_reordered",
        base_rows,
        components=("z", "x"),
    )
    broken = run_quickpls("higher_order_metamorphic_broken", permuted_component(base_rows))

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
            "Repeated-indicator higher-order constructs are experimental" in warning
            for warning in base["warnings"]
        ),
    }
    passed = (
        checks["independent_reference_max_delta"] <= TOLERANCE
        and checks["positive_affine_max_delta"] <= TOLERANCE
        and checks["row_reversal_max_delta"] <= TOLERANCE
        and checks["construct_reorder_max_delta"] <= TOLERANCE
        and checks["component_reorder_max_delta"] <= TOLERANCE
        and checks["baseline_hoc_to_y_abs"] >= 0.80
        and checks["permuted_component_drop_ratio"] <= 0.95
        and checks["experimental_warning_present"]
    )
    report = {
        "schema_version": 1,
        "kind": "repeated_indicator_hoc_metamorphic_v1",
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
        "note": "Bounded deterministic HOC metamorphic screen; not a full Monte Carlo qualification.",
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
