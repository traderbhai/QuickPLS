"""Integrated v0.7 CB-SEM/CFA validation smoke fixtures."""

import argparse
import csv
import json
import random
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
OUTPUT = RESULTS / "cbsem_v07_reference_report.json"
CLI_READY = False


def ensure_cli():
    global CLI_READY
    if not CLI_READY:
        subprocess.run(["cargo", "build", "-p", "qpls-cli"], cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
        CLI_READY = True
    return CLI_EXE


def qpls(args, **kwargs):
    return subprocess.run([str(ensure_cli()), *args], cwd=ROOT, **kwargs)


def dataset_fingerprint(data_path, name):
    project_path = RESULTS / f"{name}.fingerprint.qpls"
    qpls(["import", str(data_path.relative_to(ROOT)), str(project_path.relative_to(ROOT)), "--name", name], check=True, stdout=subprocess.DEVNULL)
    payload = json.loads(qpls(["inspect", str(project_path.relative_to(ROOT)), "--json"], check=True, capture_output=True, text=True).stdout)
    return payload["datasets"][0]["fingerprint"]


def write_rows(path):
    rng = random.Random(20260721)
    fields = ["group", "x1", "x2", "x3", "m1", "m2", "m3", "y1", "y2", "y3"]
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for group, shift, beta_xy in [("A", 0.0, 0.55), ("B", 0.25, 0.35)]:
            for _ in range(90):
                x = rng.gauss(shift, 1.0)
                m = 0.45 * x + rng.gauss(0.0, 0.75)
                y = beta_xy * x + 0.50 * m + rng.gauss(0.0, 0.70)
                writer.writerow({
                    "group": group,
                    "x1": f"{x + rng.gauss(0.0, 0.18):.10f}",
                    "x2": f"{0.88 * x + rng.gauss(0.0, 0.20):.10f}",
                    "x3": f"{0.76 * x + rng.gauss(0.0, 0.22):.10f}",
                    "m1": f"{m + rng.gauss(0.0, 0.16):.10f}",
                    "m2": f"{0.86 * m + rng.gauss(0.0, 0.20):.10f}",
                    "m3": f"{0.74 * m + rng.gauss(0.0, 0.22):.10f}",
                    "y1": f"{y + rng.gauss(0.0, 0.16):.10f}",
                    "y2": f"{0.84 * y + rng.gauss(0.0, 0.20):.10f}",
                    "y3": f"{0.72 * y + rng.gauss(0.0, 0.22):.10f}",
                })


def recipe_payload(fingerprint, suffix, model_type, metadata):
    paths = [] if model_type == "cfa" else [{"source": "x", "target": "m"}, {"source": "x", "target": "y"}, {"source": "m", "target": "y"}]
    return {
        "schema_version": 2,
        "id": f"00000000-0000-0000-0000-000000007{suffix:03d}",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": f"00000000-0000-0000-0000-000000008{suffix:03d}",
            "name": "v0.7 CB-SEM validation",
            "constructs": [
                {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2", "x3"]},
                {"id": "m", "name": "M", "short_name": "M", "mode": "reflective", "indicators": ["m1", "m2", "m3"]},
                {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2", "y3"]},
            ],
            "paths": paths,
        },
        "settings": {
            "method": "cbsem",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260721,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {
            "fixture": "cbsem_v07_reference",
            "cbsem_model_type": model_type,
            "cbsem_estimator": "ml",
            "cbsem_input": "raw",
            "cbsem_mean_structure": "false",
            "cbsem_standardization": "std_all",
            **metadata,
        },
    }


def run_recipe(name, data_path, model_type, metadata):
    fingerprint = dataset_fingerprint(data_path, name)
    recipe = RESULTS / f"{name}.recipe.json"
    output = RESULTS / f"{name}_quickpls.json"
    suffix = sum(ord(ch) for ch in name) % 1000
    recipe.write_text(json.dumps(recipe_payload(fingerprint, suffix, model_type, metadata), indent=2), encoding="utf-8")
    qpls(["run", str(recipe.relative_to(ROOT)), "--data", str(data_path.relative_to(ROOT)), "--output", str(output.relative_to(ROOT)), "--allow-experimental"], check=True, stdout=subprocess.DEVNULL)
    return json.loads(output.read_text(encoding="utf-8"))


def cbsem_payload(payload):
    return payload["payload"]["estimation"]["cbsem"]


def finite_number(value):
    return isinstance(value, (int, float)) and value == value and value not in (float("inf"), float("-inf"))


def validate_cfa(data):
    payload = cbsem_payload(run_recipe("v07_cfa", data, "cfa", {}))
    assert payload["method_version"] == "cfa_ml_v1"
    assert payload["model_type"] == "cfa"
    assert payload["fit"]["method_version"] == "cbsem_fit_v1"
    assert len(payload["parameters"]) >= 12
    assert len(payload["standardized"]) == len(payload["parameters"])
    assert len(payload["implied_covariance"]) == 81
    return {"parameters": len(payload["parameters"]), "srmr": payload["fit"]["srmr"]}


def validate_sem(data):
    payload = cbsem_payload(run_recipe("v07_sem", data, "sem", {}))
    assert payload["method_version"] == "cbsem_ml_v1"
    paths = [item for item in payload["parameters"] if item["kind"] == "structural_path"]
    assert len(paths) == 3
    assert finite_number(payload["objective"])
    return {"paths": len(paths), "chi_square": payload["fit"]["chi_square"]}


def validate_fit(data):
    payload = cbsem_payload(run_recipe("v07_fit", data, "sem", {}))
    fit = payload["fit"]
    for key in ["chi_square", "srmr", "aic", "bic", "baseline_chi_square"]:
        assert finite_number(fit[key])
    assert fit["degrees_of_freedom"] >= 0
    assert fit["srmr"] >= 0
    return {"df": fit["degrees_of_freedom"], "cfi": fit["cfi"], "rmsea": fit["rmsea"]}


def validate_mi(data):
    payload = cbsem_payload(run_recipe("v07_mi", data, "sem", {}))
    assert payload["modification_indices"]
    assert payload["modification_indices"][0]["method_version"] == "cbsem_modification_indices_v1"
    return {"candidates": len(payload["modification_indices"]), "top_mi": payload["modification_indices"][0]["modification_index"]}


def validate_bootstrap(data):
    payload = cbsem_payload(run_recipe("v07_boot", data, "sem", {"cbsem_bootstrap_samples": "99"}))
    assert payload["bootstrap"]["method_version"] == "cbsem_bootstrap_v1"
    assert payload["bootstrap"]["usable_samples"] == 99
    assert payload["bootstrap"]["intervals"]
    return {"intervals": len(payload["bootstrap"]["intervals"])}


def validate_multigroup(data):
    payload = cbsem_payload(run_recipe("v07_mgrp", data, "sem", {"cbsem_group_column": "group", "cbsem_invariance_steps": "configural,metric,scalar", "cbsem_mean_structure": "true"}))
    assert payload["multigroup"]["method_version"] == "cbsem_multigroup_v1"
    assert len(payload["multigroup"]["groups"]) == 2
    steps = [step["step"] for step in payload["multigroup"]["invariance"]]
    assert steps == ["configural", "metric", "scalar"]
    return {"groups": len(payload["multigroup"]["groups"]), "steps": steps}


def validate_export(data):
    result_path = RESULTS / "v07_sem_quickpls.json"
    if not result_path.exists():
        validate_sem(data)
    export_path = RESULTS / "v07_cbsem_export.csv"
    qpls(["export", str(result_path.relative_to(ROOT)), "--format", "csv", "--output", str(export_path.relative_to(ROOT)), "--include-experimental"], check=True, stdout=subprocess.DEVNULL)
    text = export_path.read_text(encoding="utf-8")
    assert "cbsem_fit" in text
    assert "cbsem_parameter" in text
    return {"export_bytes": len(text)}


def validate_guard(data):
    fingerprint = dataset_fingerprint(data, "v07_guard")
    recipe = recipe_payload(fingerprint, 777, "sem", {})
    recipe["model"]["constructs"][0]["mode"] = "formative"
    path = RESULTS / "v07_guard.recipe.json"
    path.write_text(json.dumps(recipe, indent=2), encoding="utf-8")
    completed = qpls(["validate", str(path.relative_to(ROOT)), "--json"], capture_output=True, text=True)
    issues = json.loads(completed.stdout)
    codes = {issue["code"] for issue in issues}
    assert "cbsem.reflective_only" in codes
    return {"guard_code": "cbsem.reflective_only"}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--section", choices=["all", "cfa", "sem", "fit", "mi", "bootstrap", "multigroup"], default="all")
    args = parser.parse_args()
    RESULTS.mkdir(parents=True, exist_ok=True)
    data = RESULTS / "v07_cbsem.csv"
    write_rows(data)
    report = {"kind": "v07_cbsem_reference", "status": "passed", "sections": {}}
    try:
        if args.section in ("all", "cfa"):
            report["sections"]["cfa"] = validate_cfa(data)
        if args.section in ("all", "sem"):
            report["sections"]["sem"] = validate_sem(data)
        if args.section in ("all", "fit"):
            report["sections"]["fit"] = validate_fit(data)
        if args.section in ("all", "mi"):
            report["sections"]["mi"] = validate_mi(data)
        if args.section in ("all", "bootstrap"):
            report["sections"]["bootstrap"] = validate_bootstrap(data)
        if args.section in ("all", "multigroup"):
            report["sections"]["multigroup"] = validate_multigroup(data)
        if args.section == "all":
            report["sections"]["export"] = validate_export(data)
            report["sections"]["guard"] = validate_guard(data)
    except Exception as exc:
        report["status"] = "failed"
        report["error"] = str(exc)
        OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
        raise
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
