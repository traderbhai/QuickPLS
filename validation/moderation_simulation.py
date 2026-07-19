"""Bounded simulation screen for two-stage moderation.

This validates deterministic recovery across multiple generated datasets. It is
not a full inferential coverage study; that remains a larger qualification
task because it requires many resampled simulations.
"""

import json
import random
import subprocess
from statistics import mean

import moderation_reference as ref


OUTPUT = ref.RESULTS / "moderation_simulation_report.json"
TOLERANCE = 1e-10
SIGNAL_REPLICATES = 20
NULL_REPLICATES = 20
N = 160
CLI_EXE = ref.ROOT / "target" / "debug" / ("qpls.exe")


def ensure_cli():
    if not CLI_EXE.exists():
        subprocess.run(
            ["cargo", "build", "-p", "qpls-cli"],
            cwd=ref.ROOT,
            check=True,
            stdout=subprocess.DEVNULL,
        )
    return CLI_EXE


def cli_json(args):
    completed = subprocess.run(
        [str(ensure_cli()), *args],
        cwd=ref.ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


def dataset_fingerprint(csv_path, stem):
    project_path = ref.RESULTS / f"{stem}.fingerprint.qpls"
    subprocess.run(
        [
            str(ensure_cli()),
            "import",
            str(csv_path.relative_to(ref.ROOT)),
            str(project_path.relative_to(ref.ROOT)),
            "--name",
            stem,
        ],
        cwd=ref.ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    completed = cli_json(["inspect", str(project_path.relative_to(ref.ROOT)), "--json"])
    project_path.unlink(missing_ok=True)
    return completed["datasets"][0]["fingerprint"]


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
    return {"paths": paths}


def generated_rows(seed, interaction_beta):
    rng = random.Random(seed)
    rows = []
    for index in range(N):
        x = rng.gauss(0.0, 1.0)
        m = 0.15 * x + rng.gauss(0.0, 1.0)
        y = (
            0.30 * x
            + 0.22 * m
            + interaction_beta * x * m
            + rng.gauss(0.0, 0.35)
        )
        if index % 17 == 0:
            y += 0.02
        rows.append({"x": x, "m": m, "y": y})
    return rows


def quantile(values, probability):
    values = sorted(values)
    if not values:
        raise ValueError("empty quantile input")
    if probability <= 0.0:
        return values[0]
    if probability >= 1.0:
        return values[-1]
    position = probability * (len(values) - 1)
    lower = int(position)
    upper = min(lower + 1, len(values) - 1)
    fraction = position - lower
    return values[lower] * (1.0 - fraction) + values[upper] * fraction


def run_group(label, replicates, interaction_beta, seed_offset):
    rows = []
    for replicate in range(replicates):
        data = generated_rows(2026071900 + seed_offset + replicate, interaction_beta)
        quickpls = run_quickpls(f"moderation_simulation_{label}_{replicate:02d}", data)
        reference = ref.independent_reference(data)
        interaction = quickpls["paths"][("xm", "y")]
        expected = reference[("xm", "y")]
        rows.append({
            "replicate": replicate,
            "interaction_beta": interaction_beta,
            "quickpls_interaction": interaction,
            "reference_interaction": expected,
            "abs_reference_delta": abs(interaction - expected),
            "x_path": quickpls["paths"][("x", "y")],
            "m_path": quickpls["paths"][("m", "y")],
        })
    return rows


def summarize(rows):
    interactions = [row["quickpls_interaction"] for row in rows]
    abs_interactions = [abs(value) for value in interactions]
    deltas = [row["abs_reference_delta"] for row in rows]
    return {
        "replicates": len(rows),
        "mean_interaction": mean(interactions),
        "min_interaction": min(interactions),
        "max_interaction": max(interactions),
        "mean_abs_interaction": mean(abs_interactions),
        "p90_abs_interaction": quantile(abs_interactions, 0.90),
        "max_abs_reference_delta": max(deltas),
    }


def main():
    signal = run_group("signal", SIGNAL_REPLICATES, 0.65, 0)
    null = run_group("null", NULL_REPLICATES, 0.0, 10_000)
    signal_summary = summarize(signal)
    null_summary = summarize(null)
    checks = {
        "signal_replicates": SIGNAL_REPLICATES,
        "null_replicates": NULL_REPLICATES,
        "signal_mean_interaction": signal_summary["mean_interaction"],
        "signal_min_interaction": signal_summary["min_interaction"],
        "null_mean_abs_interaction": null_summary["mean_abs_interaction"],
        "null_p90_abs_interaction": null_summary["p90_abs_interaction"],
        "max_reference_delta": max(
            signal_summary["max_abs_reference_delta"],
            null_summary["max_abs_reference_delta"],
        ),
        "signal_all_positive": all(row["quickpls_interaction"] > 0.45 for row in signal),
        "null_abs_under_threshold_count": sum(
            1 for row in null if abs(row["quickpls_interaction"]) <= 0.20
        ),
    }
    passed = (
        checks["max_reference_delta"] <= TOLERANCE
        and checks["signal_mean_interaction"] >= 0.70
        and checks["signal_min_interaction"] >= 0.45
        and checks["signal_all_positive"]
        and checks["null_mean_abs_interaction"] <= 0.10
        and checks["null_p90_abs_interaction"] <= 0.20
        and checks["null_abs_under_threshold_count"] >= NULL_REPLICATES - 1
    )
    report = {
        "schema_version": 1,
        "kind": "pls_two_stage_moderation_bounded_simulation_v1",
        "passed": passed,
        "tolerance": TOLERANCE,
        "sample_size": N,
        "checks": checks,
        "signal_summary": signal_summary,
        "null_summary": null_summary,
        "signal_replicates": signal,
        "null_replicates": null,
        "note": "Bounded deterministic recovery/null-signal screen; not a full bootstrap/permutation coverage qualification.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"wrote {OUTPUT} | passed={passed} | "
        f"signal_mean={checks['signal_mean_interaction']:.3g} | "
        f"null_p90_abs={checks['null_p90_abs_interaction']:.3g}"
    )
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
