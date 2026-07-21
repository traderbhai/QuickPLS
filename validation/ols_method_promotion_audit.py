import csv
import json
import math
import subprocess
from pathlib import Path

import numpy as np

from r_runtime import find_rscript_optional


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "ols_method_promotion_audit.json"
CLI = ROOT / "target" / "debug" / "qpls.exe"
TOL = 1e-6


def run(command, check=False):
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=180)
    if check and proc.returncode != 0:
        raise RuntimeError(f"command failed: {command}\nstdout={proc.stdout}\nstderr={proc.stderr}")
    return proc


def ensure_cli():
    run(["cargo", "build", "-p", "qpls-cli"], check=True)


def write_dataset():
    path = RESULTS / "ols_promotion_fixture.csv"
    fields = ["y", "x", "m", "z", "collinear"]
    rows = []
    for i in range(1, 81):
        x = math.sin(i / 5.0) + (i % 7) * 0.09
        m = 0.35 * x + math.cos(i / 6.0) + (i % 3) * 0.04
        z = math.log(i + 1.0) * 0.18 + math.sin(i / 9.0)
        y = 0.75 + 0.42 * x + 0.68 * m - 0.21 * z + math.sin(i * 1.7) * 0.13
        rows.append({
            "y": f"{y:.12f}",
            "x": f"{x:.12f}",
            "m": f"{m:.12f}",
            "z": f"{z:.12f}",
            "collinear": f"{(x + m):.12f}",
        })
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)
    return path


def fingerprint(csv_path, name):
    project = RESULTS / f"{name}.fingerprint.qpls"
    run([str(CLI), "import", str(csv_path.relative_to(ROOT)), str(project.relative_to(ROOT)), "--name", name], check=True)
    inspected = run([str(CLI), "inspect", str(project.relative_to(ROOT)), "--json"], check=True)
    return json.loads(inspected.stdout)["datasets"][0]["fingerprint"]


def run_ols(csv_path, name, predictors, controls=None):
    controls = controls or []
    recipe = RESULTS / f"{name}.recipe.json"
    output = RESULTS / f"{name}_quickpls.json"
    payload = {
        "schema_version": 2,
        "id": f"00000000-0000-0000-0000-00000012{sum(ord(c) for c in name) % 10000:04d}",
        "created_at": "2026-07-21T00:00:00Z",
        "dataset_fingerprint": fingerprint(csv_path, name),
        "model": {"id": "00000000-0000-0000-0000-000000001211", "name": "OLS promotion fixture", "constructs": [], "paths": []},
        "settings": {
            "method": "regression",
            "weighting_scheme": "path",
            "tolerance": 1e-9,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260721,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
            "confidence_level": 0.95,
        },
        "metadata": {
            "fixture": "ols_method_promotion_audit",
            "regression_type": "ols",
            "regression_outcome": "y",
            "regression_predictors": ",".join(predictors),
            "regression_controls": ",".join(controls),
            "robust_se": "hc3",
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
    regression = json.loads(output.read_text(encoding="utf-8"))["payload"]["estimation"]["regression"]
    return {"ok": True, "regression": regression, "output": str(output.relative_to(ROOT)), "recipe": str(recipe.relative_to(ROOT))}


def matrix(csv_path, columns):
    with csv_path.open(newline="", encoding="utf-8") as handle:
        return np.array([[float(row[col]) for col in columns] for row in csv.DictReader(handle)], dtype=float)


def normal_cdf(x):
    return 0.5 * (1.0 + math.erf(x / math.sqrt(2.0)))


def numpy_reference(csv_path, predictors):
    data = matrix(csv_path, ["y", *predictors])
    y = data[:, 0]
    x = np.column_stack([np.ones(len(y)), data[:, 1:]])
    beta = np.linalg.solve(x.T @ x, x.T @ y)
    fitted = x @ beta
    residuals = y - fitted
    xtx_inv = np.linalg.inv(x.T @ x)
    leverages = np.sum((x @ xtx_inv) * x, axis=1)
    scaled = residuals / (1.0 - np.clip(leverages, 0.0, 0.999))
    meat = x.T @ np.diag(scaled * scaled) @ x
    robust = xtx_inv @ meat @ xtx_inv
    se = np.sqrt(np.maximum(np.diag(robust), 0.0))
    stat = beta / np.maximum(se, 1e-12)
    # QuickPLS uses Student-t p values; normal is only used as a rough second
    # check for finite values here. R supplies exact p-value parity below.
    p_rough = [2.0 * (1.0 - normal_cdf(abs(v))) for v in stat]
    rss = float(np.sum(residuals ** 2))
    tss = float(np.sum((y - y.mean()) ** 2))
    n = len(y)
    p = x.shape[1]
    r2 = 1.0 - rss / tss
    return {
        "terms": ["intercept", *predictors],
        "beta": beta,
        "se_hc3": se,
        "stat": stat,
        "p_rough": p_rough,
        "r_squared": r2,
        "adjusted_r_squared": 1.0 - (1.0 - r2) * (n - 1) / (n - p),
        "rmse": math.sqrt(rss / n),
    }


def compare_quickpls_to_numpy(regression, reference):
    coefs = {row["term"]: row for row in regression["coefficients"]}
    diffs = []
    for idx, term in enumerate(reference["terms"]):
        row = coefs[term]
        diffs.extend([
            abs(row["estimate"] - float(reference["beta"][idx])),
            abs(row["standard_error"] - float(reference["se_hc3"][idx])),
            abs(row["statistic"] - float(reference["stat"][idx])),
        ])
    fit = regression["fit"]
    diffs.extend([
        abs(fit["r_squared"] - reference["r_squared"]),
        abs(fit["adjusted_r_squared"] - reference["adjusted_r_squared"]),
        abs(fit["rmse"] - reference["rmse"]),
    ])
    return max(diffs)


def r_lm_reference(csv_path, predictors):
    found = find_rscript_optional()
    if found is None:
        return {"passed": False, "available": False, "reason": "Rscript not found"}
    rscript, version = found
    script = RESULTS / "ols_promotion_reference.R"
    output = RESULTS / "ols_promotion_r_reference.json"
    formula = "y ~ " + " + ".join(predictors)
    script.write_text(
        f"""
data <- read.csv({json.dumps(str(csv_path))})
fit <- lm({formula}, data=data)
x <- model.matrix(fit)
resid <- residuals(fit)
xtx_inv <- solve(t(x) %*% x)
h <- diag(x %*% xtx_inv %*% t(x))
scaled <- resid / (1 - pmin(h, 0.999))
meat <- t(x) %*% diag(as.numeric(scaled * scaled), nrow=length(scaled)) %*% x
vcov_hc3 <- xtx_inv %*% meat %*% xtx_inv
se <- sqrt(diag(vcov_hc3))
stat <- coef(fit) / se
df <- df.residual(fit)
p <- 2 * (1 - pt(abs(stat), df))
out <- list(
  version=R.version.string,
  terms=names(coef(fit)),
  coefficients=as.numeric(coef(fit)),
  se_hc3=as.numeric(se),
  statistic=as.numeric(stat),
  p_value=as.numeric(p),
  r_squared=summary(fit)$r.squared,
  adjusted_r_squared=summary(fit)$adj.r.squared
)
writeLines(jsonlite::toJSON(out, auto_unbox=TRUE, digits=16), {json.dumps(str(output))})
""",
        encoding="utf-8",
    )
    proc = subprocess.run([rscript, str(script)], cwd=ROOT, capture_output=True, text=True, timeout=120)
    if proc.returncode != 0:
        return {"passed": False, "available": True, "version": version, "stderr_tail": proc.stderr[-1000:]}
    return {"passed": True, "available": True, "version": version, "path": str(output.relative_to(ROOT)), "data": json.loads(output.read_text(encoding="utf-8"))}


def compare_quickpls_to_r(regression, r_ref):
    if not r_ref.get("passed"):
        return {"passed": False, "max_abs_difference": None, "reference": r_ref}
    data = r_ref["data"]
    terms = ["intercept" if term == "(Intercept)" else term for term in data["terms"]]
    coefs = {row["term"]: row for row in regression["coefficients"]}
    diffs = []
    for idx, term in enumerate(terms):
        row = coefs[term]
        diffs.extend([
            abs(row["estimate"] - float(data["coefficients"][idx])),
            abs(row["standard_error"] - float(data["se_hc3"][idx])),
            abs(row["statistic"] - float(data["statistic"][idx])),
            abs(row["p_value_two_sided"] - float(data["p_value"][idx])),
        ])
    fit = regression["fit"]
    diffs.extend([
        abs(fit["r_squared"] - float(data["r_squared"])),
        abs(fit["adjusted_r_squared"] - float(data["adjusted_r_squared"])),
    ])
    return {"passed": max(diffs) <= TOL, "max_abs_difference": max(diffs), "reference": {k: v for k, v in r_ref.items() if k != "data"}}


def rank_deficiency_guard(csv_path):
    result = run_ols(csv_path, "ols_promotion_rank_deficient", ["x", "m", "collinear"], [])
    text = (result.get("stdout", "") + result.get("stderr", "")).lower()
    return {"passed": not result["ok"] and "rank" in text, "failed": not result["ok"], "stderr_tail": result.get("stderr", "")[-500:]}


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    ensure_cli()
    csv_path = write_dataset()
    predictors = ["x", "m", "z"]
    run_result = run_ols(csv_path, "ols_promotion_main", ["x", "m"], ["z"])
    regression = run_result["regression"] if run_result["ok"] else None
    numpy_ref = numpy_reference(csv_path, predictors) if regression else None
    numpy_delta = compare_quickpls_to_numpy(regression, numpy_ref) if regression else None
    r_ref = r_lm_reference(csv_path, predictors)
    r_compare = compare_quickpls_to_r(regression, r_ref) if regression else {"passed": False}
    rank_guard = rank_deficiency_guard(csv_path)
    integrated = json.loads((RESULTS / "v08_extended_methods_reference_report.json").read_text(encoding="utf-8"))
    integrated_ols = integrated.get("checks", {}).get("ols", {})
    method_compat = (ROOT / "docs/METHOD_COMPATIBILITY.md").read_text(encoding="utf-8")
    checks = {
        "main_quickpls_run": run_result["ok"] and regression["method_version"] == "regression_ols_v1",
        "independent_python_hc3_reference": numpy_delta is not None and numpy_delta <= TOL,
        "r_lm_hc3_reference": r_compare.get("passed") is True,
        "rank_deficiency_guard": rank_guard["passed"],
        "integrated_v08_ols_reference": integrated.get("passed") is True and integrated_ols.get("passed") is True,
        "method_compatibility_updated": "OLS regression | Validated for documented OLS scope" in method_compat,
    }
    report = {
        "schema_version": 1,
        "target": "ols_regression_promotion",
        "passed": all(checks.values()),
        "status": "validated",
        "scope_decision": {
            "stable_output_scope": "raw-data OLS with intercept, numeric predictors/controls, complete-case rows, HC3 robust standard errors, t statistics, p values, confidence intervals, fit diagnostics, fitted values, and residuals",
            "excluded_from_this_promotion": [
                "logistic regression",
                "PROCESS workflows",
                "HC0 and HC4 public claims until the engine honors robust_se selection",
                "categorical encoding helpers",
                "survey weights, clustered standard errors, GLS, mixed models, and panel models",
            ],
        },
        "checks": checks,
        "max_abs_difference_python": numpy_delta,
        "r_reference": r_compare,
        "rank_deficiency_guard": rank_guard,
        "quickpls_output": run_result.get("output"),
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={report['passed']}")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
