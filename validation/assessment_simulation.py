import csv
import json
import math
import subprocess
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "assessment_simulation.csv"
BROKEN_DATA = RESULTS / "assessment_simulation_broken.csv"
RECIPE = RESULTS / "assessment_simulation.recipe.json"
BROKEN_RECIPE = RESULTS / "assessment_simulation_broken.recipe.json"
QUICKPLS = RESULTS / "assessment_simulation_quickpls.json"
BROKEN_QUICKPLS = RESULTS / "assessment_simulation_broken_quickpls.json"
OUTPUT = RESULTS / "assessment_simulation_report.json"


def standardized(values):
    values = np.asarray(values, dtype=float)
    centered = values - np.mean(values)
    sd = np.std(centered, ddof=1)
    if not math.isfinite(sd) or sd <= np.finfo(float).eps:
        raise ValueError("simulation produced a zero-variance column")
    return centered / sd


def generate_latents(seed=20260718, n=240):
    rng = np.random.default_rng(seed)
    image = standardized(rng.normal(0.0, 1.0, n))
    value = standardized(0.30 * image + rng.normal(0.0, 0.95, n))
    satisfaction = standardized(0.55 * image + 0.45 * value + rng.normal(0.0, 0.45, n))
    loyalty = standardized(0.72 * satisfaction + rng.normal(0.0, 0.55, n))
    return {
        "image": image,
        "value": value,
        "satisfaction": satisfaction,
        "loyalty": loyalty,
    }


def make_indicators(latents, seed=20260719):
    rng = np.random.default_rng(seed)
    columns = {}
    loadings = {
        "image": [0.88, 0.84, 0.80],
        "value": [0.86, 0.82, 0.78],
        "satisfaction": [0.90, 0.85, 0.81],
        "loyalty": [0.89, 0.83, 0.79],
    }
    names = {
        "image": ["IMG1", "IMG2", "IMG3"],
        "value": ["VAL1", "VAL2", "VAL3"],
        "satisfaction": ["SAT1", "SAT2", "SAT3"],
        "loyalty": ["LOY1", "LOY2", "LOY3"],
    }
    for construct, latent in latents.items():
        for name, loading in zip(names[construct], loadings[construct]):
            error_sd = math.sqrt(1.0 - loading**2)
            columns[name] = standardized(loading * latent + rng.normal(0.0, error_sd, len(latent)))
    return columns


def write_csv(path, columns):
    names = list(columns)
    row_count = len(next(iter(columns.values())))
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=names)
        writer.writeheader()
        for row_index in range(row_count):
            writer.writerow({name: f"{columns[name][row_index]:.12f}" for name in names})


def dataset_fingerprint(data_path):
    project_path = data_path.with_suffix(".fingerprint.qpls")
    subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "qpls-cli",
            "--",
            "import",
            str(data_path.relative_to(ROOT)),
            str(project_path.relative_to(ROOT)),
            "--name",
            data_path.stem,
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
    report = json.loads(completed.stdout)
    return report["datasets"][0]["fingerprint"]


def write_recipe(path, fingerprint):
    recipe = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000401",
        "created_at": "2026-07-18T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000402",
            "name": "Assessment simulation reference",
            "constructs": [
                {
                    "id": "image",
                    "name": "Image",
                    "short_name": "IMG",
                    "mode": "reflective",
                    "indicators": ["IMG1", "IMG2", "IMG3"],
                },
                {
                    "id": "value",
                    "name": "Value",
                    "short_name": "VAL",
                    "mode": "reflective",
                    "indicators": ["VAL1", "VAL2", "VAL3"],
                },
                {
                    "id": "satisfaction",
                    "name": "Satisfaction",
                    "short_name": "SAT",
                    "mode": "reflective",
                    "indicators": ["SAT1", "SAT2", "SAT3"],
                },
                {
                    "id": "loyalty",
                    "name": "Loyalty",
                    "short_name": "LOY",
                    "mode": "reflective",
                    "indicators": ["LOY1", "LOY2", "LOY3"],
                },
            ],
            "paths": [
                {"source": "image", "target": "satisfaction"},
                {"source": "value", "target": "satisfaction"},
                {"source": "satisfaction", "target": "loyalty"},
            ],
        },
        "settings": {
            "method": "pls_pm",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260718,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {
            "fixture": "assessment_simulation",
            "source": "Deterministic generated reflective PLS model with known structural signal",
        },
    }
    path.write_text(json.dumps(recipe, indent=2) + "\n", encoding="utf-8")


def run_quickpls(recipe_path, data_path, output_path):
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
            str(data_path.relative_to(ROOT)),
            "--output",
            str(output_path.relative_to(ROOT)),
        ],
        cwd=ROOT,
        check=True,
    )
    return json.loads(output_path.read_text(encoding="utf-8"))["payload"]["assessment"]


def by_construct(rows, key="construct"):
    return {row[key]: row for row in rows}


def finite_nonnegative(value):
    return value is not None and math.isfinite(value) and value >= 0.0


def metric_checks(correct, broken):
    correct_q2 = by_construct(correct["blindfolding"]["constructs"])
    broken_q2 = by_construct(broken["blindfolding"]["constructs"])
    quality = by_construct(correct["structural_quality"])
    broken_quality = by_construct(broken["structural_quality"])
    f2_rows = {
        (row["source_construct"], row["target_construct"]): row
        for row in correct["f_squared"]
    }
    checks = [
        {
            "id": "satisfaction_r2_signal",
            "value": quality["satisfaction"]["r_squared"],
            "threshold": "> 0.55",
            "passed": quality["satisfaction"]["r_squared"] > 0.55,
        },
        {
            "id": "loyalty_r2_signal",
            "value": quality["loyalty"]["r_squared"],
            "threshold": "> 0.45",
            "passed": quality["loyalty"]["r_squared"] > 0.45,
        },
        {
            "id": "satisfaction_q2_positive",
            "value": correct_q2["satisfaction"]["q_squared"],
            "threshold": "> 0.20",
            "passed": correct_q2["satisfaction"]["q_squared"] > 0.20,
        },
        {
            "id": "loyalty_q2_positive",
            "value": correct_q2["loyalty"]["q_squared"],
            "threshold": "> 0.20",
            "passed": correct_q2["loyalty"]["q_squared"] > 0.20,
        },
        {
            "id": "broken_satisfaction_r2_degrades",
            "value": quality["satisfaction"]["r_squared"] - broken_quality["satisfaction"]["r_squared"],
            "threshold": "> 0.35",
            "passed": quality["satisfaction"]["r_squared"] - broken_quality["satisfaction"]["r_squared"] > 0.35,
        },
        {
            "id": "broken_satisfaction_q2_degrades",
            "value": correct_q2["satisfaction"]["q_squared"] - broken_q2["satisfaction"]["q_squared"],
            "threshold": "> 0.35",
            "passed": correct_q2["satisfaction"]["q_squared"] - broken_q2["satisfaction"]["q_squared"] > 0.35,
        },
        {
            "id": "true_predictor_f2_present",
            "value": min(
                f2_rows[("image", "satisfaction")]["f_squared"],
                f2_rows[("value", "satisfaction")]["f_squared"],
                f2_rows[("satisfaction", "loyalty")]["f_squared"],
            ),
            "threshold": "> 0.10",
            "passed": min(
                f2_rows[("image", "satisfaction")]["f_squared"],
                f2_rows[("value", "satisfaction")]["f_squared"],
                f2_rows[("satisfaction", "loyalty")]["f_squared"],
            )
            > 0.10,
        },
        {
            "id": "fit_indices_finite_nonnegative",
            "value": {
                "estimated_srmr": correct["model_fit"]["estimated"]["srmr"],
                "estimated_d_uls": correct["model_fit"]["estimated"]["d_uls"],
                "saturated_srmr": correct["model_fit"]["saturated"]["srmr"],
                "saturated_d_uls": correct["model_fit"]["saturated"]["d_uls"],
            },
            "threshold": "all finite and >= 0",
            "passed": all(
                finite_nonnegative(correct["model_fit"][variant][metric])
                for variant in ("estimated", "saturated")
                for metric in ("srmr", "d_uls")
            ),
        },
    ]
    return checks


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    latents = generate_latents()
    columns = make_indicators(latents)
    broken_columns = {name: np.asarray(values).copy() for name, values in columns.items()}
    permutation = np.random.default_rng(20260720).permutation(len(latents["image"]))
    for name in ("IMG1", "IMG2", "IMG3", "VAL1", "VAL2", "VAL3"):
        broken_columns[name] = broken_columns[name][permutation]
    write_csv(DATA, columns)
    write_csv(BROKEN_DATA, broken_columns)
    write_recipe(RECIPE, dataset_fingerprint(DATA))
    write_recipe(BROKEN_RECIPE, dataset_fingerprint(BROKEN_DATA))

    correct = run_quickpls(RECIPE, DATA, QUICKPLS)
    broken = run_quickpls(BROKEN_RECIPE, BROKEN_DATA, BROKEN_QUICKPLS)
    checks = metric_checks(correct, broken)
    report = {
        "status": "passed" if all(check["passed"] for check in checks) else "failed",
        "reference": "deterministic generated-data simulation with known reflective measurement and structural signal; broken-input comparator permutes exogenous blocks to verify degradation",
        "seed": 20260718,
        "artifacts": {
            "data": str(DATA.relative_to(ROOT)),
            "broken_data": str(BROKEN_DATA.relative_to(ROOT)),
            "recipe": str(RECIPE.relative_to(ROOT)),
            "broken_recipe": str(BROKEN_RECIPE.relative_to(ROOT)),
            "quickpls": str(QUICKPLS.relative_to(ROOT)),
            "broken_quickpls": str(BROKEN_QUICKPLS.relative_to(ROOT)),
        },
        "checks": checks,
    }
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        raise SystemExit(f"assessment simulation failed; see {OUTPUT}")


if __name__ == "__main__":
    main()
