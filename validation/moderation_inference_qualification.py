"""Bounded inferential qualification for two-stage moderation.

This is a deliberately small always-on screen. It runs the actual QuickPLS
Freedman-Lane permutation pipeline over several generated signal and null
moderation datasets, then checks that signal datasets are detected and null
datasets are not systematically flagged. It is not a final publication-scale
Monte Carlo qualification.
"""

import json
import random
import subprocess
from statistics import mean

import moderation_reference as ref
from moderation_inference import find_parameter, parameter_key
from moderation_simulation import dataset_fingerprint, ensure_cli


OUTPUT = ref.RESULTS / "moderation_inference_qualification_report.json"
PERMUTATION_SAMPLES = 119
SIGNAL_REPLICATES = 6
NULL_REPLICATES = 6
N = 180
TOLERANCE = 1e-10


def generated_rows(seed, interaction_beta):
    rng = random.Random(seed)
    rows = []
    for index in range(N):
        x = rng.gauss(0.0, 1.0)
        m = 0.18 * x + rng.gauss(0.0, 1.0)
        y = 0.32 * x + 0.24 * m + interaction_beta * x * m + rng.gauss(0.0, 0.28)
        if index % 19 == 0:
            y -= 0.015
        rows.append({"x": x, "m": m, "y": y})
    return rows


def run_quickpls(stem, rows):
    csv_path = ref.RESULTS / f"{stem}.csv"
    recipe_path = ref.RESULTS / f"{stem}.recipe.json"
    result_path = ref.RESULTS / f"{stem}_quickpls.json"
    ref.write_csv(csv_path, rows)
    ref.write_recipe(
        recipe_path,
        dataset_fingerprint(csv_path, stem),
        stem,
        ("x", "m", "xm", "y"),
    )
    subprocess.run(
        [
            str(ensure_cli()),
            "run",
            str(recipe_path.relative_to(ref.ROOT)),
            "--data",
            str(csv_path.relative_to(ref.ROOT)),
            "--output",
            str(result_path.relative_to(ref.ROOT)),
            "--allow-experimental",
            "--permutation-samples",
            str(PERMUTATION_SAMPLES),
            "--workers",
            "4",
        ],
        cwd=ref.ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    result = json.loads(result_path.read_text(encoding="utf-8"))
    paths = {
        (row["source"], row["target"]): row["coefficient"]
        for row in result["payload"]["estimation"]["paths"]
    }
    permutation = find_parameter(
        result["payload"]["permutation"]["parameters"],
        "path",
        ["xm", "y"],
    )
    moderation = result["payload"]["estimation"]["moderation"]["estimates"][0]
    return {
        "paths": paths,
        "permutation": permutation,
        "moderation": moderation,
        "warnings": result["payload"]["estimation"]["warnings"],
    }


def run_group(label, replicates, interaction_beta, seed_offset):
    rows = []
    for replicate in range(replicates):
        data = generated_rows(2026072000 + seed_offset + replicate, interaction_beta)
        quickpls = run_quickpls(f"moderation_inference_qualification_{label}_{replicate:02d}", data)
        reference = ref.independent_reference(data)
        interaction = quickpls["paths"][("xm", "y")]
        expected = reference[("xm", "y")]
        permutation = quickpls["permutation"]
        rows.append({
            "replicate": replicate,
            "interaction_beta": interaction_beta,
            "quickpls_interaction": interaction,
            "reference_interaction": expected,
            "abs_reference_delta": abs(interaction - expected),
            "permutation_p_value_two_sided": permutation["p_value_two_sided"],
            "permutation_exceedances": permutation["exceedances"],
            "permutation_samples": permutation["permutations"],
            "moderation_effect_delta": abs(
                quickpls["moderation"]["interaction_effect"] - interaction
            ),
            "experimental_warning_present": any(
                "Two-stage moderation is experimental" in warning
                for warning in quickpls["warnings"]
            ),
        })
    return rows


def summarize(rows):
    p_values = [row["permutation_p_value_two_sided"] for row in rows]
    interactions = [row["quickpls_interaction"] for row in rows]
    return {
        "replicates": len(rows),
        "mean_interaction": mean(interactions),
        "min_interaction": min(interactions),
        "max_interaction": max(interactions),
        "mean_p_value_two_sided": mean(p_values),
        "significant_at_0_05": sum(value <= 0.05 for value in p_values),
        "non_significant_at_0_05": sum(value > 0.05 for value in p_values),
        "max_abs_reference_delta": max(row["abs_reference_delta"] for row in rows),
        "max_moderation_effect_delta": max(row["moderation_effect_delta"] for row in rows),
    }


def main():
    signal = run_group("signal", SIGNAL_REPLICATES, 0.70, 0)
    null = run_group("null", NULL_REPLICATES, 0.0, 10_000)
    signal_summary = summarize(signal)
    null_summary = summarize(null)
    checks = {
        "parameter": parameter_key("path", ["xm", "y"]),
        "signal_replicates": SIGNAL_REPLICATES,
        "null_replicates": NULL_REPLICATES,
        "permutation_samples": PERMUTATION_SAMPLES,
        "max_reference_delta": max(
            signal_summary["max_abs_reference_delta"],
            null_summary["max_abs_reference_delta"],
        ),
        "max_moderation_effect_delta": max(
            signal_summary["max_moderation_effect_delta"],
            null_summary["max_moderation_effect_delta"],
        ),
        "signal_detected_count": signal_summary["significant_at_0_05"],
        "null_flagged_count": null_summary["significant_at_0_05"],
        "signal_min_interaction": signal_summary["min_interaction"],
        "null_mean_p_value_two_sided": null_summary["mean_p_value_two_sided"],
        "all_permutation_counts_match": all(
            row["permutation_samples"] == PERMUTATION_SAMPLES
            for row in [*signal, *null]
        ),
        "all_experimental_warnings_present": all(
            row["experimental_warning_present"] for row in [*signal, *null]
        ),
    }
    passed = (
        checks["max_reference_delta"] <= TOLERANCE
        and checks["max_moderation_effect_delta"] <= TOLERANCE
        and checks["signal_detected_count"] >= SIGNAL_REPLICATES - 1
        and checks["null_flagged_count"] <= 1
        and checks["signal_min_interaction"] >= 0.50
        and checks["null_mean_p_value_two_sided"] >= 0.20
        and checks["all_permutation_counts_match"]
        and checks["all_experimental_warnings_present"]
    )
    report = {
        "schema_version": 1,
        "kind": "pls_two_stage_moderation_bounded_inference_qualification_v1",
        "passed": passed,
        "tolerance": TOLERANCE,
        "sample_size": N,
        "checks": checks,
        "signal_summary": signal_summary,
        "null_summary": null_summary,
        "signal_replicates": signal,
        "null_replicates": null,
        "note": "Bounded always-on permutation decision screen; not a final publication-scale Monte Carlo coverage qualification.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"wrote {OUTPUT} | passed={passed} | "
        f"signal_detected={checks['signal_detected_count']}/{SIGNAL_REPLICATES} | "
        f"null_flagged={checks['null_flagged_count']}/{NULL_REPLICATES}"
    )
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
