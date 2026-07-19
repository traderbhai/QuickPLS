"""Oracle segmentation recovery fixture for future v0.6 heterogeneity engines.

This is not FIMIX-PLS. It is a bounded, deterministic simulation that proves the
generated data contain recoverable segment-specific structural paths when the
true segment labels are supplied, then checks the first experimental two-segment
PLS-POS-style discovery routine without passing those labels to QuickPLS.
"""

import csv
import json
import random
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
OUTPUT = RESULTS / "segmentation_recovery_simulation_report.json"
CLI_READY = False


def ensure_cli():
    global CLI_READY
    if CLI_READY:
        return CLI_EXE
    subprocess.run(["cargo", "build", "-p", "qpls-cli"], cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
    CLI_READY = True
    return CLI_EXE


def qpls(args, **kwargs):
    return subprocess.run([str(ensure_cli()), *args], cwd=ROOT, **kwargs)


def write_rows():
    rng = random.Random(20260719)
    rows = []
    for segment, beta, intercept, offset in [("A", 0.85, 0.20, 0), ("B", -0.60, -0.10, 10_000)]:
        for index in range(96):
            base = rng.gauss(0.0, 1.0)
            x_score = base + 0.15 * rng.gauss(0.0, 1.0)
            y_score = intercept + beta * x_score + 0.18 * rng.gauss(0.0, 1.0)
            rows.append(
                {
                    "case_id": str(offset + index + 1),
                    "segment": segment,
                    "x1": f"{x_score + 0.04 * rng.gauss(0.0, 1.0):.8f}",
                    "x2": f"{0.92 * x_score + 0.05 * rng.gauss(0.0, 1.0):.8f}",
                    "y1": f"{y_score + 0.04 * rng.gauss(0.0, 1.0):.8f}",
                    "y2": f"{0.90 * y_score + 0.05 * rng.gauss(0.0, 1.0):.8f}",
                }
            )
    return rows


def write_csv(path, rows):
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["case_id", "segment", "x1", "x2", "y1", "y2"])
        writer.writeheader()
        writer.writerows(rows)


def write_multi_csv(path, rows):
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["case_id", "segment", "x1", "x2", "z1", "z2", "y1", "y2"])
        writer.writeheader()
        writer.writerows(rows)


def dataset_fingerprint(data_path, name):
    project_path = RESULTS / f"{name}.fingerprint.qpls"
    qpls(
        ["import", str(data_path.relative_to(ROOT)), str(project_path.relative_to(ROOT)), "--name", name],
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


def write_recipe(path, fingerprint, name, seed, method="pls_pm", metadata=None):
    payload = {
        "schema_version": 2,
        "id": f"00000000-0000-0000-0000-00000000{seed:04d}",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": f"00000000-0000-0000-0000-00000001{seed:04d}",
            "name": name,
            "constructs": [
                {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2"]},
                {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2"]},
            ],
            "paths": [{"source": "x", "target": "y"}],
        },
        "settings": {
            "method": method,
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260719,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {
            "fixture": "segmentation_recovery_simulation",
            "oracle_segment": name,
            **(metadata or {}),
        },
    }
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def write_multi_recipe(path, fingerprint, name, seed, method="pls_pm", metadata=None):
    payload = {
        "schema_version": 2,
        "id": f"00000000-0000-0000-0000-00000000{seed:04d}",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": f"00000000-0000-0000-0000-00000001{seed:04d}",
            "name": name,
            "constructs": [
                {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2"]},
                {"id": "z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1", "z2"]},
                {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2"]},
            ],
            "paths": [{"source": "x", "target": "y"}, {"source": "z", "target": "y"}],
        },
        "settings": {
            "method": method,
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260719,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {
            "fixture": "segmentation_recovery_simulation",
            "oracle_segment": name,
            **(metadata or {}),
        },
    }
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def run_case(name, rows, seed):
    data_path = RESULTS / f"segmentation_recovery_{name}.csv"
    recipe_path = RESULTS / f"segmentation_recovery_{name}.recipe.json"
    result_path = RESULTS / f"segmentation_recovery_{name}_quickpls.json"
    write_csv(data_path, rows)
    fingerprint = dataset_fingerprint(data_path, f"segmentation_recovery_{name}")
    write_recipe(recipe_path, fingerprint, f"Segmentation recovery {name}", seed)
    qpls(
        [
            "run",
            str(recipe_path.relative_to(ROOT)),
            "--data",
            str(data_path.relative_to(ROOT)),
            "--output",
            str(result_path.relative_to(ROOT)),
            "--allow-experimental",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    payload = json.loads(result_path.read_text(encoding="utf-8"))
    paths = payload["payload"]["estimation"]["paths"]
    coefficient = next(
        item["coefficient"] for item in paths if item["source"] == "x" and item["target"] == "y"
    )
    return {
        "name": name,
        "rows": len(rows),
        "path_x_to_y": coefficient,
        "artifacts": {
            "data": str(data_path.relative_to(ROOT)),
            "recipe": str(recipe_path.relative_to(ROOT)),
            "quickpls": str(result_path.relative_to(ROOT)),
        },
    }


def write_multi_rows():
    rng = random.Random(20260720)
    rows = []
    for segment, beta_x, beta_z, intercept, offset in [
        ("A", 0.90, 0.70, 0.15, 20_000),
        ("B", -0.75, -0.55, -0.20, 30_000),
    ]:
        for index in range(120):
            x_score = rng.gauss(0.0, 1.0)
            z_score = 0.20 * x_score + rng.gauss(0.0, 1.0)
            y_score = intercept + beta_x * x_score + beta_z * z_score + 0.16 * rng.gauss(0.0, 1.0)
            rows.append(
                {
                    "case_id": str(offset + index + 1),
                    "segment": segment,
                    "x1": f"{x_score + 0.04 * rng.gauss(0.0, 1.0):.8f}",
                    "x2": f"{0.91 * x_score + 0.05 * rng.gauss(0.0, 1.0):.8f}",
                    "z1": f"{z_score + 0.04 * rng.gauss(0.0, 1.0):.8f}",
                    "z2": f"{0.88 * z_score + 0.05 * rng.gauss(0.0, 1.0):.8f}",
                    "y1": f"{y_score + 0.04 * rng.gauss(0.0, 1.0):.8f}",
                    "y2": f"{0.90 * y_score + 0.05 * rng.gauss(0.0, 1.0):.8f}",
                }
            )
    return rows


def write_multi_null_rows(seed=20260721):
    rng = random.Random(seed)
    rows = []
    beta_x = 0.70
    beta_z = 0.50
    for index in range(240):
        x_score = rng.gauss(0.0, 1.0)
        z_score = 0.20 * x_score + rng.gauss(0.0, 1.0)
        y_score = 0.10 + beta_x * x_score + beta_z * z_score + 0.22 * rng.gauss(0.0, 1.0)
        rows.append(
            {
                "case_id": str(40_000 + index + 1),
                "segment": "none",
                "x1": f"{x_score + 0.04 * rng.gauss(0.0, 1.0):.8f}",
                "x2": f"{0.91 * x_score + 0.05 * rng.gauss(0.0, 1.0):.8f}",
                "z1": f"{z_score + 0.04 * rng.gauss(0.0, 1.0):.8f}",
                "z2": f"{0.88 * z_score + 0.05 * rng.gauss(0.0, 1.0):.8f}",
                "y1": f"{y_score + 0.04 * rng.gauss(0.0, 1.0):.8f}",
                "y2": f"{0.90 * y_score + 0.05 * rng.gauss(0.0, 1.0):.8f}",
            }
        )
    return rows


def run_multi_case(name, rows, seed):
    data_path = RESULTS / f"segmentation_recovery_multi_{name}.csv"
    recipe_path = RESULTS / f"segmentation_recovery_multi_{name}.recipe.json"
    result_path = RESULTS / f"segmentation_recovery_multi_{name}_quickpls.json"
    write_multi_csv(data_path, rows)
    fingerprint = dataset_fingerprint(data_path, f"segmentation_recovery_multi_{name}")
    write_multi_recipe(recipe_path, fingerprint, f"Segmentation recovery multi {name}", seed)
    qpls(
        [
            "run",
            str(recipe_path.relative_to(ROOT)),
            "--data",
            str(data_path.relative_to(ROOT)),
            "--output",
            str(result_path.relative_to(ROOT)),
            "--allow-experimental",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    payload = json.loads(result_path.read_text(encoding="utf-8"))
    paths = {
        f"{item['source']}->{item['target']}": item["coefficient"]
        for item in payload["payload"]["estimation"]["paths"]
    }
    return {
        "name": name,
        "rows": len(rows),
        "paths": paths,
        "artifacts": {
            "data": str(data_path.relative_to(ROOT)),
            "recipe": str(recipe_path.relative_to(ROOT)),
            "quickpls": str(result_path.relative_to(ROOT)),
        },
    }


def run_multi_discovery(rows):
    return run_multi_discovery_with_name("discovery", rows, 6613)


def run_multi_discovery_with_name(name, rows, seed):
    data_path = RESULTS / f"segmentation_recovery_multi_{name}.csv"
    recipe_path = RESULTS / f"segmentation_recovery_multi_{name}.recipe.json"
    result_path = RESULTS / f"segmentation_recovery_multi_{name}_quickpls.json"
    write_multi_csv(data_path, rows)
    fingerprint = dataset_fingerprint(data_path, f"segmentation_recovery_multi_{name}")
    write_multi_recipe(
        recipe_path,
        fingerprint,
        "Segmentation recovery multi discovery",
        seed,
        method="predict",
        metadata={"pls_pos_segments": "2"},
    )
    qpls(
        [
            "run",
            str(recipe_path.relative_to(ROOT)),
            "--data",
            str(data_path.relative_to(ROOT)),
            "--output",
            str(result_path.relative_to(ROOT)),
            "--allow-experimental",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    payload = json.loads(result_path.read_text(encoding="utf-8"))
    segmentation = payload["payload"]["estimation"].get("segmentation")
    if segmentation is None:
        raise RuntimeError("QuickPLS result did not include multi-path segmentation payload")
    segment_paths = []
    for segment in segmentation["segments"]:
        segment_paths.append({
            f"{path['source']}->{path['target']}": path["coefficient"]
            for path in segment["paths"]
        })
    return {
        "name": name,
        "rows": len(rows),
        "method_version": segmentation["method_version"],
        "selected_segments": segmentation["selected_segments"],
        "objective_improvement": segmentation["objective_improvement"],
        "min_segment_share": segmentation["min_segment_share"],
        "segment_size_imbalance": segmentation["segment_size_imbalance"],
        "max_path_separation": segmentation["max_path_separation"],
        "segment_paths": segment_paths,
        "memberships": segmentation.get("memberships", []),
        "segments": segmentation["segments"],
        "artifacts": {
            "data": str(data_path.relative_to(ROOT)),
            "recipe": str(recipe_path.relative_to(ROOT)),
            "quickpls": str(result_path.relative_to(ROOT)),
        },
    }


def run_null_screen(signal):
    runs = []
    for replicate in range(12):
        rows = write_multi_null_rows(20260721 + replicate)
        discovery = run_multi_discovery_with_name(f"null_screen_{replicate + 1:02d}", rows, 6630 + replicate)
        runs.append(discovery)
    objective_extreme = sum(
        1 for run in runs if run["objective_improvement"] >= signal["objective_improvement"]
    )
    separation_extreme = sum(
        1 for run in runs if run["max_path_separation"] >= signal["max_path_separation"]
    )
    joint_extreme = sum(
        1 for run in runs
        if run["objective_improvement"] >= signal["objective_improvement"]
        and run["max_path_separation"] >= signal["max_path_separation"]
    )
    return {
        "replicates": len(runs),
        "objective_threshold": signal["objective_improvement"],
        "separation_threshold": signal["max_path_separation"],
        "objective_extreme_count": objective_extreme,
        "separation_extreme_count": separation_extreme,
        "joint_extreme_count": joint_extreme,
        "objective_empirical_p_upper": (objective_extreme + 1) / (len(runs) + 1),
        "separation_empirical_p_upper": (separation_extreme + 1) / (len(runs) + 1),
        "joint_empirical_p_upper": (joint_extreme + 1) / (len(runs) + 1),
        "max_null_objective_improvement": max(run["objective_improvement"] for run in runs),
        "max_null_path_separation": max(run["max_path_separation"] for run in runs),
        "runs": runs,
    }


def run_discovery(rows):
    name = "discovery"
    data_path = RESULTS / f"segmentation_recovery_{name}.csv"
    recipe_path = RESULTS / f"segmentation_recovery_{name}.recipe.json"
    result_path = RESULTS / f"segmentation_recovery_{name}_quickpls.json"
    write_csv(data_path, rows)
    fingerprint = dataset_fingerprint(data_path, f"segmentation_recovery_{name}")
    write_recipe(
        recipe_path,
        fingerprint,
        "Segmentation recovery discovery",
        6603,
        method="predict",
        metadata={"pls_pos_segments": "2"},
    )
    qpls(
        [
            "run",
            str(recipe_path.relative_to(ROOT)),
            "--data",
            str(data_path.relative_to(ROOT)),
            "--output",
            str(result_path.relative_to(ROOT)),
            "--allow-experimental",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    payload = json.loads(result_path.read_text(encoding="utf-8"))
    segmentation = payload["payload"]["estimation"].get("segmentation")
    if segmentation is None:
        raise RuntimeError("QuickPLS result did not include segmentation payload")
    segment_paths = [
        segment["paths"][0]["coefficient"]
        for segment in segmentation["segments"]
        if segment["paths"]
    ]
    return {
        "name": name,
        "rows": len(rows),
        "method_version": segmentation["method_version"],
        "selected_segments": segmentation["selected_segments"],
        "objective_improvement": segmentation["objective_improvement"],
        "min_segment_share": segmentation["min_segment_share"],
        "segment_size_imbalance": segmentation["segment_size_imbalance"],
        "max_path_separation": segmentation["max_path_separation"],
        "segment_paths": segment_paths,
        "memberships": segmentation.get("memberships", []),
        "segments": segmentation["segments"],
        "artifacts": {
            "data": str(data_path.relative_to(ROOT)),
            "recipe": str(recipe_path.relative_to(ROOT)),
            "quickpls": str(result_path.relative_to(ROOT)),
        },
    }


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    rows = write_rows()
    segment_a = [row for row in rows if row["segment"] == "A"]
    segment_b = [row for row in rows if row["segment"] == "B"]
    pooled = run_case("pooled", rows, 6600)
    oracle_a = run_case("oracle_a", segment_a, 6601)
    oracle_b = run_case("oracle_b", segment_b, 6602)
    discovery = run_discovery(rows)
    discovered_paths = sorted(discovery["segment_paths"])
    multi_rows = write_multi_rows()
    multi_a = [row for row in multi_rows if row["segment"] == "A"]
    multi_b = [row for row in multi_rows if row["segment"] == "B"]
    multi_pooled = run_multi_case("pooled", multi_rows, 6610)
    multi_oracle_a = run_multi_case("oracle_a", multi_a, 6611)
    multi_oracle_b = run_multi_case("oracle_b", multi_b, 6612)
    multi_discovery = run_multi_discovery(multi_rows)
    multi_null_rows = write_multi_null_rows()
    multi_null_pooled = run_multi_case("null_pooled", multi_null_rows, 6620)
    multi_null_discovery = run_multi_discovery_with_name("null_discovery", multi_null_rows, 6621)
    null_screen = run_null_screen(multi_discovery)
    discovered_x_paths = sorted(path["x->y"] for path in multi_discovery["segment_paths"])
    discovered_z_paths = sorted(path["z->y"] for path in multi_discovery["segment_paths"])
    membership_segments = {item["segment"] for item in discovery["memberships"]}
    multi_membership_segments = {item["segment"] for item in multi_discovery["memberships"]}
    checks = {
        "oracle_a_positive_path_recovered": oracle_a["path_x_to_y"] > 0.78,
        "oracle_b_negative_path_recovered": oracle_b["path_x_to_y"] < -0.52,
        "oracle_segments_are_separated": oracle_a["path_x_to_y"] - oracle_b["path_x_to_y"] > 1.30,
        "pooled_model_is_attenuated": abs(pooled["path_x_to_y"]) < 0.25,
        "discovery_payload_version": discovery["method_version"] == "pls_pos_bounded_v1",
        "discovery_selects_two_segments": discovery["selected_segments"] == 2,
        "discovery_recovers_negative_segment": discovered_paths[0] < -0.52,
        "discovery_recovers_positive_segment": discovered_paths[-1] > 0.78,
        "discovery_improves_pooled_objective": discovery["objective_improvement"] > 0.70,
        "discovery_fit_diagnostics_available": discovery["min_segment_share"] > 0.20 and discovery["segment_size_imbalance"] < 0.75 and discovery["max_path_separation"] > 1.30,
        "discovery_memberships_complete": len(discovery["memberships"]) == len(rows),
        "discovery_memberships_sorted": [item["observation"] for item in discovery["memberships"]] == list(range(len(rows))),
        "discovery_memberships_include_both_segments": membership_segments == {"segment_1_low_alignment", "segment_2_high_alignment"},
        "multi_oracle_a_paths_recovered": multi_oracle_a["paths"]["x->y"] > 0.70 and multi_oracle_a["paths"]["z->y"] > 0.55,
        "multi_oracle_b_paths_recovered": multi_oracle_b["paths"]["x->y"] < -0.60 and multi_oracle_b["paths"]["z->y"] < -0.45,
        "multi_pooled_model_is_attenuated": abs(multi_pooled["paths"]["x->y"]) < 0.30 and abs(multi_pooled["paths"]["z->y"]) < 0.30,
        "multi_discovery_recovers_x_paths": discovered_x_paths[0] < -0.60 and discovered_x_paths[-1] > 0.70,
        "multi_discovery_recovers_z_paths": discovered_z_paths[0] < -0.45 and discovered_z_paths[-1] > 0.55,
        "multi_discovery_improves_pooled_objective": multi_discovery["objective_improvement"] > 0.70,
        "multi_discovery_fit_diagnostics_available": multi_discovery["min_segment_share"] > 0.20 and multi_discovery["segment_size_imbalance"] < 0.75 and multi_discovery["max_path_separation"] > 1.30,
        "multi_discovery_memberships_complete": len(multi_discovery["memberships"]) == len(multi_rows),
        "multi_discovery_memberships_sorted": [item["observation"] for item in multi_discovery["memberships"]] == list(range(len(multi_rows))),
        "multi_discovery_memberships_include_both_segments": multi_membership_segments == {"segment_1_low_alignment", "segment_2_high_alignment"},
        "multi_null_pooled_paths_match_truth": 0.55 < multi_null_pooled["paths"]["x->y"] < 0.82 and 0.38 < multi_null_pooled["paths"]["z->y"] < 0.62,
        "multi_null_discovery_weak_objective_gain": multi_null_discovery["objective_improvement"] < 0.20,
        "multi_null_discovery_weak_path_separation": multi_null_discovery["max_path_separation"] < 0.50,
        "multi_null_discovery_memberships_complete": len(multi_null_discovery["memberships"]) == len(multi_null_rows),
        "bounded_inferential_screen_objective_p": null_screen["objective_empirical_p_upper"] <= 0.10,
        "bounded_inferential_screen_separation_p": null_screen["separation_empirical_p_upper"] <= 0.10,
        "bounded_inferential_screen_joint_p": null_screen["joint_empirical_p_upper"] <= 0.10,
        "bounded_inferential_screen_null_below_signal": null_screen["max_null_objective_improvement"] < multi_discovery["objective_improvement"] and null_screen["max_null_path_separation"] < multi_discovery["max_path_separation"],
    }
    report = {
        "kind": "segmentation_recovery_simulation_v1",
        "passed": all(checks.values()),
        "checks": checks,
        "truth": {
            "segment_a_beta": 0.85,
            "segment_b_beta": -0.60,
            "note": "Oracle labels are used only to prove the fixture is recoverable; discovery engines must not use them.",
        },
        "runs": {
            "pooled": pooled,
            "oracle_a": oracle_a,
            "oracle_b": oracle_b,
            "discovery": discovery,
            "multi_pooled": multi_pooled,
            "multi_oracle_a": multi_oracle_a,
            "multi_oracle_b": multi_oracle_b,
            "multi_discovery": multi_discovery,
            "multi_null_pooled": multi_null_pooled,
            "multi_null_discovery": multi_null_discovery,
            "bounded_inferential_screen": null_screen,
        },
        "limitations": [
            "The discovery routine is a bounded two-segment preview, not full FIMIX-PLS.",
            "General PLS-POS still needs multiple starts, class diagnostics, fit criteria, and broader recovery simulations.",
        ],
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={report['passed']}")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
