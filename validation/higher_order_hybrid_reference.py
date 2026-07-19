"""Independent reference and metamorphic checks for hybrid HOCs.

The experimental QuickPLS hybrid contract follows the indicator-split approach:
each lower-order component keeps the first half of its indicators, while the
higher-order construct receives the remaining component indicators. This avoids
using the same manifest indicator twice in the execution model.
"""

import copy
import json

import higher_order_reference as ref


OUTPUT = ref.RESULTS / "higher_order_hybrid_reference_report.json"
TOLERANCE = 1e-6


def split_indicators(indicators):
    if len(indicators) < 2:
        raise ValueError("hybrid HOC components require at least two indicators")
    split = (len(indicators) + 1) // 2
    return indicators[:split], indicators[split:]


def expand_hybrid_hoc(recipe):
    expanded = copy.deepcopy(recipe)
    constructs = {construct["id"]: construct for construct in expanded["model"]["constructs"]}
    original = {
        construct["id"]: list(construct.get("indicators", []))
        for construct in recipe["model"]["constructs"]
    }
    for hoc in recipe["model"].get("higher_order_constructs", []):
        if hoc["method"] != "hybrid":
            continue
        hoc_indicators = []
        seen = set()
        for component in hoc["components"]:
            lower, higher = split_indicators(original[component])
            constructs[component]["indicators"] = lower
            for indicator in higher:
                if indicator not in seen:
                    seen.add(indicator)
                    hoc_indicators.append(indicator)
        constructs[hoc["id"]]["indicators"] = hoc_indicators
    return expanded


def write_recipe(path, fingerprint, stem, construct_order=("x", "z", "hoc", "y"), components=("x", "z")):
    construct_defs = {
        "x": {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2"]},
        "z": {"id": "z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1", "z2"]},
        "hoc": {"id": "hoc", "name": "HOC", "short_name": "HOC", "mode": "reflective", "indicators": []},
        "y": {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2"]},
    }
    recipe = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000093",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000094",
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
                    "method": "hybrid",
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
        "metadata": {"fixture": "higher_order_hybrid_reference"},
    }
    path.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")
    return recipe


def run_quickpls(stem, rows, construct_order=("x", "z", "hoc", "y"), components=("x", "z")):
    csv_path = ref.RESULTS / f"{stem}.csv"
    recipe_path = ref.RESULTS / f"{stem}.recipe.json"
    result_path = ref.RESULTS / f"{stem}_quickpls.json"
    ref.RESULTS.mkdir(parents=True, exist_ok=True)
    fields = ["x1", "x2", "z1", "z2", "y1", "y2"]
    with csv_path.open("w", newline="", encoding="utf-8") as handle:
        writer = ref.csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            writer.writerow({field: f"{row[field]:.12f}" for field in fields})
    project_path = ref.RESULTS / f"{stem}.fingerprint.qpls"
    ref.qpls_cli(
        ["import", str(csv_path.relative_to(ref.ROOT)), str(project_path.relative_to(ref.ROOT)), "--name", stem],
        stdout=ref.subprocess.DEVNULL,
    )
    completed = ref.qpls_cli(
        ["inspect", str(project_path.relative_to(ref.ROOT)), "--json"],
        capture_output=True,
        text=True,
    )
    project_path.unlink(missing_ok=True)
    fingerprint = json.loads(completed.stdout)["datasets"][0]["fingerprint"]
    recipe = write_recipe(recipe_path, fingerprint, stem, construct_order, components)
    ref.qpls_cli(
        [
            "run",
            str(recipe_path.relative_to(ref.ROOT)),
            "--data",
            str(csv_path.relative_to(ref.ROOT)),
            "--output",
            str(result_path.relative_to(ref.ROOT)),
            "--allow-experimental",
        ],
        stdout=ref.subprocess.DEVNULL,
    )
    result = json.loads(result_path.read_text(encoding="utf-8"))
    estimation = result["payload"]["estimation"]
    return {
        "paths": {
            (row["source"], row["target"]): row["coefficient"]
            for row in estimation["paths"]
        },
        "outer": {
            (row["construct"], row["indicator"]): {"loading": row["loading"], "weight": row["weight"]}
            for row in estimation["outer_estimates"]
        },
        "warnings": estimation["warnings"],
        "recipe": recipe,
    }


def positive_affine(rows):
    return [
        {
            "x1": 3.0 + 2.1 * row["x1"],
            "x2": -2.0 + 1.7 * row["x2"],
            "z1": 5.0 + 2.4 * row["z1"],
            "z2": -1.0 + 1.8 * row["z2"],
            "y1": 8.0 + 2.9 * row["y1"],
            "y2": -4.0 + 2.2 * row["y2"],
        }
        for row in rows
    ]


def permuted_hoc_half(rows):
    shifted_x2 = [row["x2"] for row in rows[19:]] + [row["x2"] for row in rows[:19]]
    return [
        {**row, "x2": shifted_x2[index]}
        for index, row in enumerate(rows)
    ]


def reference_for(rows, recipe):
    columns = {name: [row[name] for row in rows] for name in rows[0]}
    return ref.estimate_pls(columns, expand_hybrid_hoc(recipe))


def max_reference_delta(rows, result):
    reference = reference_for(rows, result["recipe"])
    deltas = []
    for pair, coefficient in reference["paths"].items():
        deltas.append(abs(coefficient - result["paths"][pair]))
    for key, metrics in reference["outer"].items():
        if key[0] != "hoc":
            continue
        other = result["outer"][key]
        deltas.append(abs(metrics["loading"] - other["loading"]))
        deltas.append(abs(metrics["weight"] - other["weight"]))
    return max(deltas)


def max_result_delta(left, right):
    deltas = []
    for pair, coefficient in left["paths"].items():
        deltas.append(abs(coefficient - right["paths"][pair]))
    for key, metrics in left["outer"].items():
        if key[0] != "hoc":
            continue
        other = right["outer"][key]
        deltas.append(abs(metrics["loading"] - other["loading"]))
        deltas.append(abs(metrics["weight"] - other["weight"]))
    return max(deltas)


def main():
    rows = ref.generated_rows(seed=20260721, n=110)
    base = run_quickpls("higher_order_hybrid_base", rows)
    affine = run_quickpls("higher_order_hybrid_affine", positive_affine(rows))
    reversed_result = run_quickpls("higher_order_hybrid_reversed", list(reversed(rows)))
    component_reordered = run_quickpls(
        "higher_order_hybrid_component_reordered",
        rows,
        components=("z", "x"),
    )
    broken = run_quickpls("higher_order_hybrid_broken", permuted_hoc_half(rows))
    baseline = abs(base["paths"][("hoc", "y")])
    broken_value = abs(broken["paths"][("hoc", "y")])
    checks = {
        "independent_reference_max_delta": max_reference_delta(rows, base),
        "positive_affine_max_delta": max_result_delta(base, affine),
        "row_reversal_max_delta": max_result_delta(base, reversed_result),
        "component_reorder_max_delta": max_result_delta(base, component_reordered),
        "baseline_hoc_to_y_abs": baseline,
        "permuted_hoc_half_hoc_to_y_abs": broken_value,
        "permuted_hoc_half_drop_ratio": broken_value / baseline,
        "hoc_indicators": [indicator for construct, indicator in base["outer"] if construct == "hoc"],
        "experimental_warning_present": any(
            "hybrid higher-order construct" in warning.lower()
            or "hybrid higher-order construct metadata is experimental" in warning.lower()
            for warning in base["warnings"]
        ),
    }
    passed = (
        checks["independent_reference_max_delta"] <= TOLERANCE
        and checks["positive_affine_max_delta"] <= TOLERANCE
        and checks["row_reversal_max_delta"] <= TOLERANCE
        and checks["component_reorder_max_delta"] <= TOLERANCE
        and checks["baseline_hoc_to_y_abs"] >= 0.70
        and checks["permuted_hoc_half_drop_ratio"] <= 0.90
        and checks["hoc_indicators"] == ["x2", "z2"]
        and checks["experimental_warning_present"]
    )
    report = {
        "schema_version": 1,
        "kind": "hybrid_hoc_reference_metamorphic_v1",
        "tolerance": TOLERANCE,
        "passed": passed,
        "checks": checks,
        "scenarios": [
            "base_generated_signal",
            "positive_affine_transform",
            "row_reversal",
            "component_reorder",
            "permuted_hoc_indicator_half_degradation",
        ],
        "note": "Bounded deterministic hybrid HOC indicator-split screen; not a full Monte Carlo qualification.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"wrote {OUTPUT} | passed={passed} | "
        f"reference_delta={checks['independent_reference_max_delta']:.3g} | "
        f"drop_ratio={checks['permuted_hoc_half_drop_ratio']:.3g}"
    )
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
