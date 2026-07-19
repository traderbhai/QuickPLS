"""Smoke/reference report for the experimental PLSpredict holdout slice."""

import csv
import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "plspredict_holdout_reference.csv"
RECIPE = RESULTS / "plspredict_holdout_reference.recipe.json"
QUICKPLS = RESULTS / "plspredict_holdout_reference_quickpls.json"
PAIR_DATA = RESULTS / "plspredict_model_pair_reference.csv"
PAIR_RECIPE = RESULTS / "plspredict_model_pair_reference.recipe.json"
PAIR_QUICKPLS = RESULTS / "plspredict_model_pair_reference_quickpls.json"
OUTPUT = RESULTS / "plspredict_holdout_reference_report.json"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
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


def write_dataset():
    RESULTS.mkdir(parents=True, exist_ok=True)
    with DATA.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["x1", "x2", "y1", "y2"])
        writer.writeheader()
        for index in range(1, 65):
            x = float(index)
            writer.writerow(
                {
                    "x1": f"{x:.6f}",
                    "x2": f"{x + (index % 5) * 0.1:.6f}",
                    "y1": f"{2.0 * x + 1.0:.6f}",
                    "y2": f"{2.0 * x + 1.0 + (index % 7) * 0.08:.6f}",
                }
            )


def write_pair_dataset():
    RESULTS.mkdir(parents=True, exist_ok=True)
    with PAIR_DATA.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["x1", "x2", "z1", "z2", "y1", "y2"])
        writer.writeheader()
        for index in range(1, 81):
            x = float(index) / 10.0
            z = ((index * 7) % 23) / 10.0 - 1.0
            y = 1.4 * x + 0.9 * z + ((index % 4) - 1.5) * 0.03
            writer.writerow(
                {
                    "x1": f"{x + (index % 3) * 0.03:.6f}",
                    "x2": f"{0.8 * x + (index % 5) * 0.04:.6f}",
                    "z1": f"{z + (index % 4) * 0.02:.6f}",
                    "z2": f"{0.7 * z + (index % 6) * 0.025:.6f}",
                    "y1": f"{y:.6f}",
                    "y2": f"{0.95 * y + (index % 7) * 0.02:.6f}",
                }
            )


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


def write_recipe(fingerprint):
    payload = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000061",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000062",
            "name": "PLSpredict holdout reference",
            "constructs": [
                {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2"]},
                {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2"]},
            ],
            "paths": [{"source": "x", "target": "y"}],
        },
        "settings": {
            "method": "predict",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260719,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {"fixture": "plspredict_holdout_reference"},
    }
    RECIPE.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def write_pair_recipe(fingerprint):
    payload = {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000063",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000064",
            "name": "PLSpredict model-pair reference",
            "constructs": [
                {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2"]},
                {"id": "z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1", "z2"]},
                {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2"]},
            ],
            "paths": [{"source": "x", "target": "y"}, {"source": "z", "target": "y"}],
        },
        "settings": {
            "method": "predict",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260719,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {
            "fixture": "plspredict_model_pair_reference",
            "cvpat_drop_paths": "z->y",
        },
    }
    PAIR_RECIPE.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def run_quickpls():
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
    return json.loads(QUICKPLS.read_text(encoding="utf-8"))


def run_pair_quickpls():
    qpls(
        [
            "run",
            str(PAIR_RECIPE.relative_to(ROOT)),
            "--data",
            str(PAIR_DATA.relative_to(ROOT)),
            "--output",
            str(PAIR_QUICKPLS.relative_to(ROOT)),
            "--allow-experimental",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    return json.loads(PAIR_QUICKPLS.read_text(encoding="utf-8"))


def main():
    write_dataset()
    fingerprint = dataset_fingerprint(DATA, "plspredict_holdout_reference")
    write_recipe(fingerprint)
    result = run_quickpls()
    write_pair_dataset()
    pair_fingerprint = dataset_fingerprint(PAIR_DATA, "plspredict_model_pair_reference")
    write_pair_recipe(pair_fingerprint)
    pair_result = run_pair_quickpls()
    estimation = result["payload"]["estimation"]
    predict = estimation["predict"]
    target = predict["targets"][0]
    repeated = predict["repeated_kfold"]
    repeated_target = repeated["targets"][0]
    cvpat = repeated["cvpat"]
    by_comparison = {item["comparison"]: item for item in cvpat}
    pair_predict = pair_result["payload"]["estimation"]["predict"]
    pair_cvpat = pair_predict["repeated_kfold"]["cvpat"]
    pair_by_comparison = {item["comparison"]: item for item in pair_cvpat}
    model_pair_key = "pls_vs_model_pair:drop_z_to_y"
    checks = {
        "method_version": estimation["method_version"] == "plspredict_holdout_v1",
        "split_counts": predict["training_observations"] == 48 and predict["test_observations"] == 16,
        "target_shape": target["construct"] == "y" and target["predictor_count"] == 1,
        "predictive_improvement": target["rmse_pls"] < target["rmse_benchmark"],
        "lm_benchmark_available": target["rmse_lm"] is not None and target["mae_lm"] is not None,
        "q2_threshold": target["q_squared_predict"] is not None and target["q_squared_predict"] > 0.9,
        "kfold_plan": repeated["folds"] == 5 and repeated["repeats"] == 3 and repeated["total_test_observations"] == 192,
        "kfold_predictive_improvement": repeated_target["rmse_pls"] < repeated_target["rmse_benchmark"],
        "kfold_lm_benchmark_available": repeated_target["rmse_lm"] is not None and repeated_target["q_squared_predict_lm"] is not None,
        "cvpat_present": {"pls_vs_training_mean_benchmark", "pls_vs_lm_benchmark"}.issubset(by_comparison),
        "cvpat_p_values_available": all(item["p_value_two_sided"] is not None for item in cvpat),
        "cvpat_benchmark_prefers_pls": by_comparison.get("pls_vs_training_mean_benchmark", {}).get("preferred_model") == "pls",
        "model_pair_cvpat_present": model_pair_key in pair_by_comparison,
        "model_pair_cvpat_prefers_full_pls": pair_by_comparison.get(model_pair_key, {}).get("preferred_model") == "pls",
        "model_pair_cvpat_p_value_available": pair_by_comparison.get(model_pair_key, {}).get("p_value_two_sided") is not None,
        "experimental_warning": any("experimental" in warning.lower() for warning in predict["warnings"]),
    }
    report = {
        "kind": "plspredict_holdout_reference_v1",
        "passed": all(checks.values()),
        "checks": checks,
        "quickpls": {
            "method_version": estimation["method_version"],
            "predict": predict,
        },
        "model_pair_quickpls": {
            "method_version": pair_result["payload"]["estimation"]["method_version"],
            "predict": pair_predict,
        },
        "artifacts": {
            "data": str(DATA.relative_to(ROOT)),
            "recipe": str(RECIPE.relative_to(ROOT)),
            "quickpls": str(QUICKPLS.relative_to(ROOT)),
            "model_pair_data": str(PAIR_DATA.relative_to(ROOT)),
            "model_pair_recipe": str(PAIR_RECIPE.relative_to(ROOT)),
            "model_pair_quickpls": str(PAIR_QUICKPLS.relative_to(ROOT)),
        },
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={report['passed']}")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
