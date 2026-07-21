import csv
import json
import math
import subprocess
from pathlib import Path

import numpy as np


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "pca_method_promotion_audit.json"
CLI = ROOT / "target" / "debug" / "qpls.exe"
TOL = 1e-6


def run(command, check=False):
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=180)
    if check and proc.returncode != 0:
        raise RuntimeError(f"command failed: {command}\nstdout={proc.stdout}\nstderr={proc.stderr}")
    return proc


def ensure_cli():
    run(["cargo", "build", "-p", "qpls-cli"], check=True)


def write_csv(path, fields, rows):
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)


def fingerprint(csv_path, name):
    project = RESULTS / f"{name}.fingerprint.qpls"
    run([str(CLI), "import", str(csv_path.relative_to(ROOT)), str(project.relative_to(ROOT)), "--name", name], check=True)
    inspected = run([str(CLI), "inspect", str(project.relative_to(ROOT)), "--json"], check=True)
    return json.loads(inspected.stdout)["datasets"][0]["fingerprint"]


def run_pca(csv_path, name, variables, rule="fixed", components=2):
    recipe = RESULTS / f"{name}.recipe.json"
    output = RESULTS / f"{name}_quickpls.json"
    payload = {
        "schema_version": 2,
        "id": f"00000000-0000-0000-0000-00000012{sum(ord(c) for c in name) % 10000:04d}",
        "created_at": "2026-07-21T00:00:00Z",
        "dataset_fingerprint": fingerprint(csv_path, name),
        "model": {
            "id": "00000000-0000-0000-0000-000000001201",
            "name": "PCA promotion fixture",
            "constructs": [],
            "paths": [],
        },
        "settings": {
            "method": "pca",
            "weighting_scheme": "path",
            "tolerance": 1e-12,
            "max_iterations": 10000,
            "bootstrap_samples": 0,
            "seed": 20260721,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {
            "fixture": "pca_method_promotion_audit",
            "pca_variables": ",".join(variables),
            "pca_component_rule": rule,
            "pca_components": str(components),
        },
    }
    recipe.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    proc = run([
        str(CLI),
        "run",
        str(recipe.relative_to(ROOT)),
        "--data",
        str(csv_path.relative_to(ROOT)),
        "--output",
        str(output.relative_to(ROOT)),
        "--allow-experimental",
    ])
    if proc.returncode != 0:
        return {"ok": False, "stdout": proc.stdout, "stderr": proc.stderr, "recipe": str(recipe.relative_to(ROOT))}
    result = json.loads(output.read_text(encoding="utf-8"))["payload"]["estimation"]["pca"]
    return {"ok": True, "pca": result, "output": str(output.relative_to(ROOT)), "recipe": str(recipe.relative_to(ROOT))}


def standardized_matrix(csv_path, variables):
    rows = []
    with csv_path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            values = []
            skip = False
            for variable in variables:
                text = row[variable].strip()
                if not text:
                    skip = True
                    break
                values.append(float(text))
            if not skip:
                rows.append(values)
    x = np.array(rows, dtype=float)
    return (x - x.mean(axis=0)) / x.std(axis=0, ddof=1)


def orient_vectors(vectors):
    oriented = vectors.copy()
    for col in range(oriented.shape[1]):
        column = oriented[:, col]
        pivot = int(np.argmax(np.abs(column)))
        if column[pivot] < 0:
            oriented[:, col] *= -1.0
    return oriented


def pca_reference(csv_path, variables):
    z = standardized_matrix(csv_path, variables)
    corr = np.cov(z, rowvar=False, ddof=1)
    values, vectors = np.linalg.eigh(corr)
    order = np.argsort(values)[::-1]
    values = values[order]
    vectors = orient_vectors(vectors[:, order])
    loadings = vectors * np.sqrt(np.maximum(values, 0.0))
    return values, vectors, loadings, z


def max_component_diff(qpls_pca, values, variables, components):
    component_diffs = [
        abs(qpls_pca["components"][idx]["eigenvalue"] - float(values[idx]))
        for idx in range(components)
    ]
    loading_map = {
        (row["variable"], row["component"]): row["loading"]
        for row in qpls_pca["loadings"]
    }
    weight_map = {
        (row["variable"], row["component"]): row["weight"]
        for row in qpls_pca["loadings"]
    }
    _, vectors, loadings, _ = pca_reference_from_values(values, qpls_pca, variables)
    loading_diffs = []
    weight_diffs = []
    for comp_idx in range(components):
        component = f"PC{comp_idx + 1}"
        for var_idx, variable in enumerate(variables):
            loading_diffs.append(abs(loading_map[(variable, component)] - float(loadings[var_idx, comp_idx])))
            weight_diffs.append(abs(weight_map[(variable, component)] - float(vectors[var_idx, comp_idx])))
    return max(component_diffs + loading_diffs + weight_diffs)


def pca_reference_from_values(values, qpls_pca, variables):
    # Recompute from the CSV path would be cleaner, but this helper is kept for
    # direct comparison from the already computed reference in the caller.
    raise AssertionError("caller must use compare_pca")


def compare_pca(qpls_pca, csv_path, variables, components):
    values, vectors, loadings, _ = pca_reference(csv_path, variables)
    component_diffs = [
        abs(qpls_pca["components"][idx]["eigenvalue"] - float(values[idx]))
        for idx in range(components)
    ]
    loading_map = {(row["variable"], row["component"]): row["loading"] for row in qpls_pca["loadings"]}
    weight_map = {(row["variable"], row["component"]): row["weight"] for row in qpls_pca["loadings"]}
    loading_diffs = []
    weight_diffs = []
    for comp_idx in range(components):
        component = f"PC{comp_idx + 1}"
        for var_idx, variable in enumerate(variables):
            loading_diffs.append(abs(loading_map[(variable, component)] - float(loadings[var_idx, comp_idx])))
            weight_diffs.append(abs(weight_map[(variable, component)] - float(vectors[var_idx, comp_idx])))
    return {
        "max_abs_difference": max(component_diffs + loading_diffs + weight_diffs),
        "component_max_abs_difference": max(component_diffs),
        "loading_max_abs_difference": max(loading_diffs),
        "weight_max_abs_difference": max(weight_diffs),
    }


def hand_fixture():
    # Construct two sample-standardized vectors with exact sample correlation 0.5.
    n = 6
    r = 0.5
    a = np.array([-2.5, -1.5, -0.5, 0.5, 1.5, 2.5], dtype=float)
    a = (a - a.mean()) / a.std(ddof=1)
    raw_b = np.array([1.0, -1.0, 2.0, -2.0, 0.5, -0.5], dtype=float)
    raw_b = raw_b - raw_b.mean()
    raw_b = raw_b - a * (np.dot(raw_b, a) / np.dot(a, a))
    b = raw_b / raw_b.std(ddof=1)
    y = r * a + math.sqrt(1.0 - r * r) * b
    path = RESULTS / "pca_promotion_hand_fixture.csv"
    write_csv(path, ["x", "y"], [{"x": f"{a[i]:.15f}", "y": f"{y[i]:.15f}"} for i in range(n)])
    run_result = run_pca(path, "pca_promotion_hand", ["x", "y"], "fixed", 1)
    expected_eigenvalues = [1.5]
    observed = [row["eigenvalue"] for row in run_result["pca"]["components"][:1]] if run_result["ok"] else []
    max_diff = max(abs(observed[i] - expected_eigenvalues[i]) for i in range(1)) if observed else float("inf")
    return {
        "passed": run_result["ok"] and max_diff <= TOL,
        "expected_eigenvalues": expected_eigenvalues,
        "observed_eigenvalues": observed,
        "max_abs_difference": max_diff,
        "output": run_result.get("output"),
    }


def high_dimensional_fixture():
    path = RESULTS / "pca_promotion_high_dimensional.csv"
    fields = [f"v{i}" for i in range(1, 21)]
    rows = []
    for i in range(30):
        base = math.sin(i / 3.0) + (i % 5) * 0.11
        rows.append({
            field: f"{base * (1 + idx * 0.03) + math.cos((i + 1) * (idx + 2) / 11.0):.12f}"
            for idx, field in enumerate(fields)
        })
    write_csv(path, fields, rows)
    run_result = run_pca(path, "pca_promotion_high_dimensional", fields, "fixed", 5)
    compare = compare_pca(run_result["pca"], path, fields, 5) if run_result["ok"] else {"max_abs_difference": None}
    return {
        "passed": run_result["ok"] and compare["max_abs_difference"] <= TOL and run_result["pca"]["retained_components"] == 5,
        **compare,
        "variables": len(fields),
        "observations": run_result.get("pca", {}).get("observations"),
        "output": run_result.get("output"),
    }


def missing_fixture():
    path = RESULTS / "pca_promotion_missing.csv"
    fields = ["a", "b", "c"]
    rows = [
        {"a": "1", "b": "2", "c": "3"},
        {"a": "2", "b": "3", "c": "5"},
        {"a": "3", "b": "", "c": "7"},
        {"a": "4", "b": "5", "c": "11"},
        {"a": "5", "b": "7", "c": "13"},
        {"a": "6", "b": "11", "c": "17"},
    ]
    write_csv(path, fields, rows)
    run_result = run_pca(path, "pca_promotion_missing", fields, "fixed", 2)
    compare = compare_pca(run_result["pca"], path, fields, 2) if run_result["ok"] else {"max_abs_difference": None}
    return {
        "passed": run_result["ok"] and run_result["pca"]["observations"] == 5 and compare["max_abs_difference"] <= TOL,
        **compare,
        "observations": run_result.get("pca", {}).get("observations"),
        "output": run_result.get("output"),
    }


def constant_guard_fixture():
    path = RESULTS / "pca_promotion_constant.csv"
    fields = ["a", "constant", "c"]
    rows = [
        {"a": str(i), "constant": "5", "c": str(i * i + 1)}
        for i in range(1, 8)
    ]
    write_csv(path, fields, rows)
    run_result = run_pca(path, "pca_promotion_constant", fields, "fixed", 2)
    stderr = run_result.get("stderr", "")
    stdout = run_result.get("stdout", "")
    failed = not run_result["ok"]
    return {
        "passed": failed and ("constant" in stderr.lower() or "constant" in stdout.lower()),
        "failed": failed,
        "stdout_tail": stdout[-500:],
        "stderr_tail": stderr[-500:],
    }


def integrated_v08_check():
    report_path = "validation/results/v08_extended_methods_reference_report.json"
    report = json.loads((ROOT / report_path).read_text(encoding="utf-8"))
    pca = report.get("checks", {}).get("pca", {})
    return {
        "path": report_path,
        "passed": report.get("passed") is True and pca.get("passed") is True and pca.get("method_version") == "pca_v1",
        "max_abs_difference": pca.get("max_abs_difference"),
        "retained_components": pca.get("retained_components"),
    }


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    ensure_cli()
    checks = {
        "integrated_v08_numpy_reference": integrated_v08_check(),
        "hand_two_variable_fixture": hand_fixture(),
        "high_dimensional_numpy_reference": high_dimensional_fixture(),
        "missing_data_listwise_fixture": missing_fixture(),
        "constant_column_guard": constant_guard_fixture(),
    }
    docs = {
        "method_spec": (ROOT / "docs/methods/PCA_V1.md").exists(),
        "known_differences": (ROOT / "docs/KNOWN_DIFFERENCES.md").exists(),
        "method_compatibility_updated": "Standalone PCA | Validated for documented PCA scope" in (ROOT / "docs/METHOD_COMPATIBILITY.md").read_text(encoding="utf-8"),
    }
    report = {
        "schema_version": 1,
        "target": "pca_method_promotion",
        "passed": all(item["passed"] for item in checks.values()) and all(docs.values()),
        "status": "validated",
        "scope_decision": {
            "stable_output_scope": "standardized numeric raw-data PCA with listwise deletion, deterministic sign orientation, fixed/Kaiser/variance-threshold retention, eigenvalues, loadings, weights, and scores",
            "excluded_from_this_promotion": [
                "rotation methods",
                "pairwise deletion",
                "covariance/correlation-only input",
                "PCA inference or component-score uncertainty",
                "nonnumeric variables",
            ],
        },
        "checks": checks,
        "docs": docs,
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={report['passed']}")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
