"""Release-oriented coverage screen for two-stage moderation inference.

This is heavier than the always-on moderation inference qualification. It runs
the actual QuickPLS Freedman-Lane permutation pipeline across deterministic
signal and null datasets, records timing, and checks detection/false-positive
behavior. It is still bounded enough for a development workstation and should
not be represented as final publication-scale Monte Carlo validation.
"""

import argparse
import json
import random
import subprocess
import time
from pathlib import Path
from statistics import mean

import moderation_reference as ref
from moderation_inference import find_parameter, parameter_key
from moderation_simulation import dataset_fingerprint, ensure_cli


OUTPUT = ref.RESULTS / "moderation_coverage_qualification_report.json"
TOLERANCE = 1e-10


def generated_rows(seed, n, interaction_beta):
    rng = random.Random(seed)
    rows = []
    for index in range(n):
        x = rng.gauss(0.0, 1.0)
        m = 0.22 * x + rng.gauss(0.0, 1.0)
        y = 0.30 * x + 0.22 * m + interaction_beta * x * m + rng.gauss(0.0, 0.38)
        if index % 23 == 0:
            y += 0.012
        rows.append({"x": x, "m": m, "y": y})
    return rows


def run_quickpls(stem, rows, permutation_samples, workers):
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
    started = time.perf_counter()
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
            str(permutation_samples),
            "--workers",
            str(workers),
        ],
        cwd=ref.ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    elapsed_seconds = time.perf_counter() - started
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
        "elapsed_seconds": elapsed_seconds,
        "paths": paths,
        "permutation": permutation,
        "moderation": moderation,
        "warnings": result["payload"]["estimation"]["warnings"],
    }


def run_group(label, replicates, n, interaction_beta, permutation_samples, workers, seed_offset):
    rows = []
    for replicate in range(replicates):
        data = generated_rows(
            seed=2026072100 + seed_offset + replicate,
            n=n,
            interaction_beta=interaction_beta,
        )
        quickpls = run_quickpls(
            f"moderation_coverage_{label}_{replicate:02d}",
            data,
            permutation_samples,
            workers,
        )
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
            "elapsed_seconds": quickpls["elapsed_seconds"],
            "experimental_warning_present": any(
                "Two-stage moderation is validated" in warning
                for warning in quickpls["warnings"]
            ),
        })
    return rows


def summarize(rows):
    p_values = [row["permutation_p_value_two_sided"] for row in rows]
    interactions = [row["quickpls_interaction"] for row in rows]
    elapsed = [row["elapsed_seconds"] for row in rows]
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
        "mean_elapsed_seconds": mean(elapsed),
        "max_elapsed_seconds": max(elapsed),
    }


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--replicates", type=int, default=24)
    parser.add_argument("--sample-size", type=int, default=220)
    parser.add_argument("--permutation-samples", type=int, default=199)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--output", type=str, default=str(OUTPUT))
    return parser.parse_args()


def main():
    args = parse_args()
    started = time.perf_counter()
    signal = run_group(
        "signal",
        args.replicates,
        args.sample_size,
        0.55,
        args.permutation_samples,
        args.workers,
        0,
    )
    null = run_group(
        "null",
        args.replicates,
        args.sample_size,
        0.0,
        args.permutation_samples,
        args.workers,
        100_000,
    )
    signal_summary = summarize(signal)
    null_summary = summarize(null)
    elapsed_seconds = time.perf_counter() - started
    allowed_signal_misses = max(1, args.replicates // 12)
    allowed_null_flags = max(1, args.replicates // 8)
    checks = {
        "parameter": parameter_key("path", ["xm", "y"]),
        "signal_replicates": args.replicates,
        "null_replicates": args.replicates,
        "sample_size": args.sample_size,
        "permutation_samples": args.permutation_samples,
        "workers": args.workers,
        "elapsed_seconds": elapsed_seconds,
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
        "allowed_signal_misses": allowed_signal_misses,
        "allowed_null_flags": allowed_null_flags,
        "signal_min_interaction": signal_summary["min_interaction"],
        "null_mean_p_value_two_sided": null_summary["mean_p_value_two_sided"],
        "all_permutation_counts_match": all(
            row["permutation_samples"] == args.permutation_samples
            for row in [*signal, *null]
        ),
        "all_experimental_warnings_present": all(
            row["experimental_warning_present"] for row in [*signal, *null]
        ),
    }
    passed = (
        checks["max_reference_delta"] <= TOLERANCE
        and checks["max_moderation_effect_delta"] <= TOLERANCE
        and checks["signal_detected_count"] >= args.replicates - allowed_signal_misses
        and checks["null_flagged_count"] <= allowed_null_flags
        and checks["signal_min_interaction"] >= 0.35
        and checks["null_mean_p_value_two_sided"] >= 0.25
        and checks["all_permutation_counts_match"]
        and checks["all_experimental_warnings_present"]
    )
    report = {
        "schema_version": 1,
        "kind": "pls_two_stage_moderation_release_coverage_qualification_v1",
        "passed": passed,
        "tolerance": TOLERANCE,
        "checks": checks,
        "signal_summary": signal_summary,
        "null_summary": null_summary,
        "signal_replicates": signal,
        "null_replicates": null,
        "note": "Heavier deterministic release-oriented permutation coverage screen for two-stage moderation. This remains bounded development evidence, not final publication-scale Monte Carlo validation.",
    }
    output_path = Path(args.output)
    output_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"wrote {output_path} | passed={passed} | "
        f"signal_detected={checks['signal_detected_count']}/{args.replicates} | "
        f"null_flagged={checks['null_flagged_count']}/{args.replicates} | "
        f"elapsed={elapsed_seconds:.2f}s"
    )
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
