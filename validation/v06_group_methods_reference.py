"""Integrated v0.6 group-method validation smoke/recovery fixtures."""

import argparse
import csv
import json
import random
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
OUTPUT = RESULTS / "v06_group_methods_reference_report.json"
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


def write_group_rows(path):
    rng = random.Random(20260719)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["group", "x1", "x2", "z1", "z2", "y1", "y2"])
        writer.writeheader()
        for group, bx, bz in [("A", 0.85, 0.20), ("B", -0.35, 0.75)]:
            for _ in range(60):
                x = rng.gauss(0.0, 1.0)
                z = rng.gauss(0.0, 1.0)
                y = bx * x + bz * z + rng.gauss(0.0, 0.20)
                writer.writerow({
                    "group": group,
                    "x1": f"{x + rng.gauss(0.0, 0.05):.10f}",
                    "x2": f"{0.90 * x + rng.gauss(0.0, 0.06):.10f}",
                    "z1": f"{z + rng.gauss(0.0, 0.05):.10f}",
                    "z2": f"{0.90 * z + rng.gauss(0.0, 0.06):.10f}",
                    "y1": f"{y + rng.gauss(0.0, 0.05):.10f}",
                    "y2": f"{0.90 * y + rng.gauss(0.0, 0.06):.10f}",
                })


def write_segment_rows(path, classes=3):
    rng = random.Random(20260720 + classes)
    specs = [("A", 0.90), ("B", -0.65), ("C", 0.25)][:classes]
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["x1", "x2", "y1", "y2"])
        writer.writeheader()
        for _, beta in specs:
            for _ in range(56):
                x = rng.gauss(0.0, 1.0)
                y = beta * x + rng.gauss(0.0, 0.18)
                writer.writerow({
                    "x1": f"{x + rng.gauss(0.0, 0.04):.10f}",
                    "x2": f"{0.92 * x + rng.gauss(0.0, 0.05):.10f}",
                    "y1": f"{y + rng.gauss(0.0, 0.04):.10f}",
                    "y2": f"{0.92 * y + rng.gauss(0.0, 0.05):.10f}",
                })


def recipe_payload(fingerprint, method, metadata):
    return {
        "schema_version": 2,
        "id": "00000000-0000-0000-0000-000000000601",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000000602",
            "name": "v0.6 group methods validation",
            "constructs": [
                {"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2"]},
                *([] if method == "predict" else [{"id": "z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1", "z2"]}]),
                {"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2"]},
            ],
            "paths": [{"source": "x", "target": "y"}, *([] if method == "predict" else [{"source": "z", "target": "y"}])],
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
        "metadata": {"fixture": "v06_group_methods_reference", **metadata},
    }


def run_recipe(name, data_path, method, metadata):
    fingerprint = dataset_fingerprint(data_path, name)
    recipe = RESULTS / f"{name}.recipe.json"
    output = RESULTS / f"{name}_quickpls.json"
    recipe.write_text(json.dumps(recipe_payload(fingerprint, method, metadata), indent=2), encoding="utf-8")
    qpls(["run", str(recipe.relative_to(ROOT)), "--data", str(data_path.relative_to(ROOT)), "--output", str(output.relative_to(ROOT)), "--allow-experimental"], check=True, stdout=subprocess.DEVNULL)
    return json.loads(output.read_text(encoding="utf-8"))


def validate_groups():
    data = RESULTS / "v06_groups.csv"
    write_group_rows(data)
    payload = run_recipe("v06_groups", data, "mga", {
        "mga_group_column": "group",
        "group_methods": "micom,mga_permutation",
        "group_permutation_samples": "99",
    })
    estimation = payload["payload"]["estimation"]
    assert estimation["micom"]["method_version"] == "micom_v1"
    assert estimation["mga_permutation"]["method_version"] == "pls_mga_permutation_v1"
    assert estimation["mga_permutation"]["usable_permutations"] == 99
    differences = [abs(item["original_difference"]) for item in estimation["mga_permutation"]["comparisons"]]
    assert max(differences) > 0.5
    return {"micom_constructs": len(estimation["micom"]["constructs"]), "max_mga_difference": max(differences)}


def validate_pos():
    data = RESULTS / "v06_pos.csv"
    write_segment_rows(data, classes=3)
    payload = run_recipe("v06_pos", data, "predict", {
        "group_methods": "pls_pos",
        "segment_count": "3",
        "segment_starts": "6",
        "minimum_segment_share": "0.10",
    })
    segmentation = payload["payload"]["estimation"]["segmentation"]
    assert segmentation["method_version"] == "pls_pos_v1"
    assert segmentation["selected_segments"] == 3
    assert segmentation["objective_improvement"] > 0.25
    return {"segments": segmentation["selected_segments"], "objective_improvement": segmentation["objective_improvement"]}


def validate_fimix():
    data = RESULTS / "v06_fimix.csv"
    write_segment_rows(data, classes=3)
    payload = run_recipe("v06_fimix", data, "predict", {
        "group_methods": "fimix",
        "segment_count": "3",
        "fimix_classes": "3",
        "segment_starts": "6",
        "minimum_segment_share": "0.10",
    })
    fimix = payload["payload"]["estimation"]["fimix"]
    assert fimix["method_version"] == "fimix_pls_v1"
    assert fimix["classes"] == 3
    assert fimix["entropy"] >= 0.0
    return {"classes": fimix["classes"], "bic": fimix["bic"], "entropy": fimix["entropy"]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--section", choices=["all", "micom", "mga", "pos", "fimix"], default="all")
    args = parser.parse_args()
    RESULTS.mkdir(parents=True, exist_ok=True)
    report = {"kind": "v06_group_methods_reference", "sections": {}}
    if args.section in ("all", "micom", "mga"):
        report["sections"]["groups"] = validate_groups()
    if args.section in ("all", "pos"):
        report["sections"]["pos"] = validate_pos()
    if args.section in ("all", "fimix"):
        report["sections"]["fimix"] = validate_fimix()
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
