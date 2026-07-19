"""Integrated v0.8 extended-method validation fixtures.

These checks are preview gates, not publication validation. They prove that the
experimental v0.8 product boundary emits typed payloads and matches independent
NumPy/Python calculations for bounded fixtures.
"""

import argparse
import csv
import json
import math
import random
import subprocess
from pathlib import Path

import numpy as np


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "v08_extended_methods_fixture.csv"
OUTPUT = RESULTS / "v08_extended_methods_reference_report.json"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
CLI_READY = False
TOL = 1e-6


def ensure_cli():
    global CLI_READY
    if not CLI_READY:
        subprocess.run(["cargo", "build", "-p", "qpls-cli"], cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
        CLI_READY = True
    return CLI_EXE


def qpls(args, **kwargs):
    return subprocess.run([str(ensure_cli()), *args], cwd=ROOT, **kwargs)


def write_dataset():
    RESULTS.mkdir(parents=True, exist_ok=True)
    rng = random.Random(20260719)
    fields = ["x", "m", "w", "y", "z", "bin_y", "g1", "g2", "g3", "h1", "h2"]
    with DATA.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for index in range(1, 141):
            x = rng.gauss(0.0, 1.0)
            w = rng.gauss(0.0, 1.0)
            z = rng.gauss(0.0, 1.0)
            m = 0.62 * x + 0.22 * z + rng.gauss(0.0, 0.45)
            y = 0.30 * x + 0.68 * m + 0.24 * w + 0.18 * x * w + rng.gauss(0.0, 0.42)
            logit = -0.15 + 1.05 * x - 0.55 * z + 0.35 * w
            p = 1.0 / (1.0 + math.exp(-logit))
            bin_y = 1 if rng.random() < p else 0
            writer.writerow({
                "x": f"{x:.12f}",
                "m": f"{m:.12f}",
                "w": f"{w:.12f}",
                "y": f"{y:.12f}",
                "z": f"{z:.12f}",
                "bin_y": str(bin_y),
                "g1": f"{x + rng.gauss(0.0, 0.08):.12f}",
                "g2": f"{0.86 * x + rng.gauss(0.0, 0.10):.12f}",
                "g3": f"{0.74 * x + rng.gauss(0.0, 0.12):.12f}",
                "h1": f"{y + rng.gauss(0.0, 0.08):.12f}",
                "h2": f"{0.88 * y + rng.gauss(0.0, 0.10):.12f}",
            })


def rows():
    with DATA.open(newline="", encoding="utf-8") as handle:
        return [{key: float(value) for key, value in row.items()} for row in csv.DictReader(handle)]


def dataset_fingerprint(name):
    project_path = RESULTS / f"{name}.fingerprint.qpls"
    qpls(["import", str(DATA.relative_to(ROOT)), str(project_path.relative_to(ROOT)), "--name", name], check=True, stdout=subprocess.DEVNULL)
    payload = json.loads(qpls(["inspect", str(project_path.relative_to(ROOT)), "--json"], check=True, capture_output=True, text=True).stdout)
    return payload["datasets"][0]["fingerprint"]


def base_settings(method):
    return {
        "method": method,
        "weighting_scheme": "path",
        "tolerance": 1e-9,
        "max_iterations": 3000,
        "bootstrap_samples": 0,
        "seed": 20260719,
        "preprocessing": "standardized",
        "missing_data": "listwise_deletion",
    }


def empty_model():
    return {
        "id": "00000000-0000-0000-0000-000000008001",
        "name": "v0.8 standalone method validation",
        "constructs": [],
        "paths": [],
    }


def gsca_model():
    return {
        "id": "00000000-0000-0000-0000-000000008002",
        "name": "v0.8 GSCA validation",
        "constructs": [
            {"id": "g", "name": "G", "short_name": "G", "mode": "reflective", "indicators": ["g1", "g2", "g3"]},
            {"id": "h", "name": "H", "short_name": "H", "mode": "reflective", "indicators": ["h1", "h2"]},
        ],
        "paths": [{"source": "g", "target": "h"}],
    }


def run_recipe(name, method, metadata, model=None):
    fingerprint = dataset_fingerprint(name)
    recipe = RESULTS / f"{name}.recipe.json"
    output = RESULTS / f"{name}_quickpls.json"
    payload = {
        "schema_version": 2,
        "id": f"00000000-0000-0000-0000-00000008{sum(ord(ch) for ch in name) % 10000:04d}",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": model or empty_model(),
        "settings": base_settings(method),
        "metadata": {"fixture": "v08_extended_methods_reference", **metadata},
    }
    recipe.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    qpls(["run", str(recipe.relative_to(ROOT)), "--data", str(DATA.relative_to(ROOT)), "--output", str(output.relative_to(ROOT)), "--allow-experimental"], check=True, stdout=subprocess.DEVNULL)
    return json.loads(output.read_text(encoding="utf-8"))["payload"]["estimation"]


def matrix(columns):
    data = rows()
    return np.array([[row[column] for column in columns] for row in data], dtype=float)


def standardize(values):
    return (values - values.mean(axis=0)) / values.std(axis=0, ddof=1)


def ols_fit(y, predictors):
    x = np.column_stack([np.ones(len(y)), predictors])
    beta = np.linalg.lstsq(x, y, rcond=None)[0]
    fitted = x @ beta
    resid = y - fitted
    ssr = float(np.sum(resid ** 2))
    sst = float(np.sum((y - y.mean()) ** 2))
    return beta, 1.0 - ssr / sst


def check_pca():
    estimation = run_recipe("v08_pca", "pca", {
        "pca_variables": "x,m,w,y,z",
        "pca_component_rule": "fixed",
        "pca_components": "3",
    })
    pca = estimation["pca"]
    z = standardize(matrix(["x", "m", "w", "y", "z"]))
    corr = np.cov(z, rowvar=False, ddof=1)
    vals = np.linalg.eigh(corr)[0][::-1]
    diffs = [abs(pca["components"][i]["eigenvalue"] - float(vals[i])) for i in range(3)]
    return {
        "passed": max(diffs) <= TOL,
        "method_version": pca["method_version"],
        "max_abs_difference": max(diffs),
        "retained_components": pca["retained_components"],
    }


def check_ols():
    estimation = run_recipe("v08_regression_ols", "regression", {
        "regression_type": "ols",
        "regression_outcome": "y",
        "regression_predictors": "x,m",
        "regression_controls": "z",
        "robust_se": "hc3",
    })
    reg = estimation["regression"]
    data = matrix(["y", "x", "m", "z"])
    beta, r2 = ols_fit(data[:, 0], data[:, 1:])
    by_term = {row["term"]: row["estimate"] for row in reg["coefficients"]}
    diffs = [
        abs(by_term["intercept"] - float(beta[0])),
        abs(by_term["x"] - float(beta[1])),
        abs(by_term["m"] - float(beta[2])),
        abs(by_term["z"] - float(beta[3])),
        abs(reg["fit"]["r_squared"] - float(r2)),
    ]
    return {"passed": max(diffs) <= TOL, "method_version": reg["method_version"], "max_abs_difference": max(diffs)}


def logistic_reference(y, predictors):
    x = np.column_stack([np.ones(len(y)), predictors])
    beta = np.zeros(x.shape[1])
    for _ in range(80):
        eta = x @ beta
        p = 1.0 / (1.0 + np.exp(-np.clip(eta, -30, 30)))
        w = np.maximum(p * (1.0 - p), 1e-9)
        z = eta + (y - p) / w
        xtw = x.T * w
        next_beta = np.linalg.solve(xtw @ x, xtw @ z)
        if np.max(np.abs(next_beta - beta)) < 1e-10:
            beta = next_beta
            break
        beta = next_beta
    return beta


def check_logistic():
    estimation = run_recipe("v08_regression_logistic", "regression", {
        "regression_type": "logistic",
        "regression_outcome": "bin_y",
        "regression_predictors": "x,z,w",
    })
    reg = estimation["regression"]
    data = matrix(["bin_y", "x", "z", "w"])
    beta = logistic_reference(data[:, 0], data[:, 1:])
    by_term = {row["term"]: row["estimate"] for row in reg["coefficients"]}
    diffs = [
        abs(by_term["intercept"] - float(beta[0])),
        abs(by_term["x"] - float(beta[1])),
        abs(by_term["z"] - float(beta[2])),
        abs(by_term["w"] - float(beta[3])),
    ]
    return {"passed": max(diffs) <= 1e-5, "method_version": reg["method_version"], "max_abs_difference": max(diffs)}


def check_process():
    estimation = run_recipe("v08_process", "regression", {
        "regression_type": "process",
        "regression_outcome": "y",
        "regression_predictors": "x,m",
        "process_model": "mediation",
        "process_x": "x",
        "process_m": "m",
    })
    reg = estimation["regression"]
    process = reg["process"]
    data = matrix(["x", "m", "y"])
    a, _ = ols_fit(data[:, 1], data[:, [0]])
    b_model, _ = ols_fit(data[:, 2], data[:, [0, 1]])
    indirect = float(a[1] * b_model[2])
    reported = {row["effect"]: row["estimate"] for row in process["effects"]}
    diff = abs(reported["indirect"] - indirect)
    return {"passed": diff <= TOL, "method_version": process["method_version"], "max_abs_difference": diff}


def nca_effect(x, y):
    order = np.lexsort((y, x))
    xs = x[order]
    ys = y[order]
    min_y = float(np.min(y))
    max_y = float(np.max(y))
    scope = max(float((np.max(x) - np.min(x)) * (max_y - min_y)), np.finfo(float).eps)
    area = 0.0
    for i in range(len(xs) - 1):
        x0 = float(xs[i])
        x1 = float(xs[i + 1])
        ceiling_y = max(float(value) for value, x_value in zip(ys, xs) if float(x_value) >= x0)
        area += max(0.0, x1 - x0) * max(0.0, max_y - ceiling_y)
    return min(1.0, max(0.0, area / scope))


def check_nca():
    estimation = run_recipe("v08_nca", "nca", {
        "nca_x": "x",
        "nca_y": "y",
        "nca_ceiling": "both",
        "nca_permutation_samples": "99",
    })
    nca = estimation["nca"]
    data = matrix(["x", "y"])
    ref = nca_effect(data[:, 0], data[:, 1])
    fdh = next(row for row in nca["ceilings"] if row["ceiling"] == "ce_fdh")
    diff = abs(fdh["effect_size"] - ref)
    monotonic = all(nca["bottlenecks"][i]["required_x_percent"] <= nca["bottlenecks"][i + 1]["required_x_percent"] + 1e-9 for i in range(len(nca["bottlenecks"]) - 1))
    return {"passed": diff <= TOL and monotonic, "method_version": nca["method_version"], "max_abs_difference": diff, "bottleneck_monotonic": monotonic}


def check_gsca():
    estimation = run_recipe("v08_gsca", "gsca", {}, model=gsca_model())
    gsca = estimation["gsca"]
    finite = all(math.isfinite(path["coefficient"]) for path in gsca["paths"])
    present = len(gsca["weights"]) == 5 and len(gsca["paths"]) == 1 and gsca["method_version"] == "gsca_v1"
    return {
        "passed": finite and present,
        "method_version": gsca["method_version"],
        "paths": len(gsca["paths"]),
        "weights": len(gsca["weights"]),
        "fit": gsca["fit"],
    }


CHECKS = {
    "pca": check_pca,
    "ols": check_ols,
    "logistic": check_logistic,
    "process": check_process,
    "nca": check_nca,
    "gsca": check_gsca,
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--section", choices=[*CHECKS.keys(), "all"], default="all")
    args = parser.parse_args()
    write_dataset()
    selected = CHECKS.keys() if args.section == "all" else [args.section]
    checks = {name: CHECKS[name]() for name in selected}
    report = {
        "passed": all(item["passed"] for item in checks.values()),
        "schema_version": 1,
        "target": "v0.8 extended methods experimental preview",
        "selected_section": args.section,
        "tolerance": TOL,
        "checks": checks,
        "note": "v0.8 methods are experimental and watermarked; this is bounded reference evidence, not publication validation.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
