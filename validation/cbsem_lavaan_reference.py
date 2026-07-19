"""Development-only lavaan parity checks for the QuickPLS CB-SEM optimizer."""

import argparse
import csv
import json
import math
import os
import random
import subprocess
import textwrap
from pathlib import Path

from r_runtime import find_rscript_optional


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
CLI_EXE = ROOT / "target" / "debug" / "qpls.exe"
OUTPUT = RESULTS / "cbsem_lavaan_reference_report.json"
USER_RSCRIPT = Path(r"C:\Users\mohd.naved\AppData\Local\Programs\R\R-4.6.1\bin\x64\Rscript.exe")
CLI_READY = False


def ensure_cli():
    global CLI_READY
    if not CLI_READY:
        subprocess.run(["cargo", "build", "-p", "qpls-cli"], cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
        CLI_READY = True
    return CLI_EXE


def qpls(args, **kwargs):
    return subprocess.run([str(ensure_cli()), *args], cwd=ROOT, **kwargs)


def rscript_path():
    configured = os.environ.get("QPLS_RSCRIPT")
    if configured and Path(configured).exists():
        return configured
    if USER_RSCRIPT.exists():
        return str(USER_RSCRIPT)
    found = find_rscript_optional()
    if found:
        return found[0]
    raise SystemExit("Rscript.exe was not found; set QPLS_RSCRIPT.")


def write_fixture(path, model):
    rng = random.Random(20260722 + sum(ord(ch) for ch in model))
    columns = ["x1", "x2", "x3"]
    if model != "one_factor_cfa":
        columns += ["m1", "m2", "m3"]
    if model in ("three_factor_cfa", "latent_regression_sem", "latent_mediation_sem", "correlated_exogenous_sem"):
        columns += ["y1", "y2", "y3"]
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=columns)
        writer.writeheader()
        for _ in range(240):
            x = rng.gauss(0.0, 1.0)
            z = 0.35 * x + rng.gauss(0.0, 0.94)
            if model == "latent_mediation_sem":
                m = 0.55 * x + rng.gauss(0.0, 0.75)
                y = 0.30 * x + 0.50 * m + rng.gauss(0.0, 0.65)
            elif model == "latent_regression_sem":
                m = 0.60 * x + rng.gauss(0.0, 0.70)
                y = 0.45 * x + 0.35 * m + rng.gauss(0.0, 0.70)
            elif model == "correlated_exogenous_sem":
                m = z
                y = 0.45 * x + 0.40 * m + rng.gauss(0.0, 0.70)
            else:
                m = z
                y = 0.25 * x + 0.30 * m + rng.gauss(0.0, 0.85)
            row = {
                "x1": x + rng.gauss(0.0, 0.25),
                "x2": 0.82 * x + rng.gauss(0.0, 0.28),
                "x3": 0.74 * x + rng.gauss(0.0, 0.30),
            }
            if "m1" in columns:
                row.update({
                    "m1": m + rng.gauss(0.0, 0.24),
                    "m2": 0.84 * m + rng.gauss(0.0, 0.27),
                    "m3": 0.72 * m + rng.gauss(0.0, 0.30),
                })
            if "y1" in columns:
                row.update({
                    "y1": y + rng.gauss(0.0, 0.24),
                    "y2": 0.82 * y + rng.gauss(0.0, 0.27),
                    "y3": 0.70 * y + rng.gauss(0.0, 0.30),
                })
            writer.writerow({key: f"{value:.10f}" for key, value in row.items()})


def constructs_for(model):
    constructs = [{"id": "x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1", "x2", "x3"]}]
    if model != "one_factor_cfa":
        constructs.append({"id": "m", "name": "M", "short_name": "M", "mode": "reflective", "indicators": ["m1", "m2", "m3"]})
    if model in ("three_factor_cfa", "latent_regression_sem", "latent_mediation_sem", "correlated_exogenous_sem"):
        constructs.append({"id": "y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1", "y2", "y3"]})
    return constructs


def paths_for(model):
    if model == "latent_regression_sem":
        return [{"source": "x", "target": "m"}, {"source": "x", "target": "y"}, {"source": "m", "target": "y"}]
    if model == "latent_mediation_sem":
        return [{"source": "x", "target": "m"}, {"source": "x", "target": "y"}, {"source": "m", "target": "y"}]
    if model == "correlated_exogenous_sem":
        return [{"source": "x", "target": "y"}, {"source": "m", "target": "y"}]
    return []


def lavaan_syntax(model):
    lines = ["x =~ x1 + x2 + x3"]
    if model != "one_factor_cfa":
        lines.append("m =~ m1 + m2 + m3")
    if model in ("three_factor_cfa", "latent_regression_sem", "latent_mediation_sem", "correlated_exogenous_sem"):
        lines.append("y =~ y1 + y2 + y3")
    if model in ("latent_regression_sem", "latent_mediation_sem"):
        lines += ["m ~ x", "y ~ x + m"]
    if model == "correlated_exogenous_sem":
        lines += ["y ~ x + m", "x ~~ m"]
    return "\n".join(lines)


def dataset_fingerprint(data_path, name):
    project_path = RESULTS / f"{name}.fingerprint.qpls"
    qpls(["import", str(data_path.relative_to(ROOT)), str(project_path.relative_to(ROOT)), "--name", name], check=True, stdout=subprocess.DEVNULL)
    payload = json.loads(qpls(["inspect", str(project_path.relative_to(ROOT)), "--json"], check=True, capture_output=True, text=True).stdout)
    return payload["datasets"][0]["fingerprint"]


def quickpls_run(model, data_path):
    fingerprint = dataset_fingerprint(data_path, f"lavaan_{model}")
    recipe = {
        "schema_version": 2,
        "id": f"00000000-0000-0000-0000-000000071{len(model):03d}",
        "created_at": "2026-07-19T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": f"00000000-0000-0000-0000-000000072{len(model):03d}",
            "name": f"lavaan {model}",
            "constructs": constructs_for(model),
            "paths": paths_for(model),
        },
        "settings": {
            "method": "cbsem",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "seed": 20260722,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
        },
        "metadata": {
            "cbsem_model_type": "sem" if paths_for(model) else "cfa",
            "cbsem_estimator": "ml",
            "cbsem_input": "raw",
        },
    }
    recipe_path = RESULTS / f"lavaan_{model}.recipe.json"
    output_path = RESULTS / f"lavaan_{model}_quickpls.json"
    recipe_path.write_text(json.dumps(recipe, indent=2), encoding="utf-8")
    qpls(["run", str(recipe_path.relative_to(ROOT)), "--data", str(data_path.relative_to(ROOT)), "--output", str(output_path.relative_to(ROOT)), "--allow-experimental"], check=True, stdout=subprocess.DEVNULL)
    return json.loads(output_path.read_text(encoding="utf-8"))["payload"]["estimation"]["cbsem"]


def lavaan_run(model, data_path):
    output = RESULTS / f"lavaan_{model}_reference.json"
    script = RESULTS / f"lavaan_{model}.R"
    script.write_text(textwrap.dedent(f"""
        suppressPackageStartupMessages(library(lavaan))
        data <- read.csv({json.dumps(str(data_path))})
        numeric_cols <- names(data)
        data[numeric_cols] <- scale(data[numeric_cols])
        model <- {json.dumps(lavaan_syntax(model))}
        fit <- sem(model, data=data, meanstructure=FALSE, std.lv=FALSE, auto.fix.first=TRUE,
                   estimator="ML", missing="listwise", fixed.x=FALSE)
        pe <- parameterEstimates(fit, standardized=TRUE)
        fitm <- fitMeasures(fit, c("chisq","df","pvalue","cfi","tli","rmsea","srmr","aic","bic"))
        rows <- lapply(seq_len(nrow(pe)), function(i) as.list(pe[i, c("lhs","op","rhs","est","se","z","pvalue","std.lv","std.all")]))
        payload <- list(parameters=rows, fit=as.list(fitm), syntax=model)
        writeLines(jsonlite::toJSON(payload, auto_unbox=TRUE, pretty=TRUE, na="null", digits=16), {json.dumps(str(output))})
    """), encoding="utf-8")
    subprocess.run([rscript_path(), str(script)], cwd=ROOT, check=True)
    return json.loads(output.read_text(encoding="utf-8"))


def quick_parameter_map(cbsem):
    return {row["name"]: row for row in cbsem["parameters"]}


def quick_standardized_map(cbsem):
    return {row["name"]: row for row in cbsem["standardized"]}


def lavaan_parameter_map(payload):
    mapped = {}
    for row in payload["parameters"]:
        op = row["op"]
        if op == "=~":
            name = f"{row['lhs']}=~{row['rhs']}"
        elif op == "~":
            name = f"{row['lhs']}~{row['rhs']}"
        elif op == "~~":
            name = f"{row['lhs']}~~{row['rhs']}"
        else:
            continue
        mapped[name] = row
    return mapped


def abs_delta(left, right):
    if left is None or right is None:
        return None
    return abs(float(left) - float(right))


def compare_model(model):
    data_path = RESULTS / f"lavaan_{model}.csv"
    write_fixture(data_path, model)
    quick = quickpls_run(model, data_path)
    lavaan = lavaan_run(model, data_path)
    quick_parameters = quick_parameter_map(quick)
    quick_standardized = quick_standardized_map(quick)
    lavaan_parameters = lavaan_parameter_map(lavaan)
    comparisons = []
    standardized_comparisons = []
    max_estimate_delta = 0.0
    max_standard_error_delta = 0.0
    max_z_delta = 0.0
    max_p_delta = 0.0
    max_standardized_delta = 0.0
    max_fit_delta = 0.0
    for name, quick_row in quick_parameters.items():
        lavaan_row = lavaan_parameters.get(name)
        if not lavaan_row:
            continue
        delta = abs_delta(quick_row["estimate"], lavaan_row["est"])
        if delta is not None:
            max_estimate_delta = max(max_estimate_delta, delta)
        se_delta = z_delta = p_delta = None
        if not quick_row.get("fixed"):
            se_delta = abs_delta(quick_row.get("standard_error"), lavaan_row.get("se"))
            z_delta = abs_delta(quick_row.get("z_statistic"), lavaan_row.get("z"))
            p_delta = abs_delta(quick_row.get("p_value_two_sided"), lavaan_row.get("pvalue"))
            if se_delta is not None:
                max_standard_error_delta = max(max_standard_error_delta, se_delta)
            if z_delta is not None:
                max_z_delta = max(max_z_delta, z_delta)
            if p_delta is not None:
                max_p_delta = max(max_p_delta, p_delta)
        comparisons.append({
            "parameter": name,
            "quickpls": quick_row["estimate"],
            "lavaan": lavaan_row["est"],
            "abs_delta": delta,
            "standard_error_delta": se_delta,
            "z_delta": z_delta,
            "p_delta": p_delta,
        })
        quick_std = quick_standardized.get(name)
        if quick_std:
            std_lv_delta = abs_delta(quick_std.get("std_lv"), lavaan_row.get("std.lv"))
            std_all_delta = abs_delta(quick_std.get("std_all"), lavaan_row.get("std.all"))
            if std_lv_delta is not None:
                max_standardized_delta = max(max_standardized_delta, std_lv_delta)
            if std_all_delta is not None:
                max_standardized_delta = max(max_standardized_delta, std_all_delta)
            standardized_comparisons.append({
                "parameter": name,
                "quickpls_std_lv": quick_std.get("std_lv"),
                "lavaan_std_lv": lavaan_row.get("std.lv"),
                "std_lv_delta": std_lv_delta,
                "quickpls_std_all": quick_std.get("std_all"),
                "lavaan_std_all": lavaan_row.get("std.all"),
                "std_all_delta": std_all_delta,
            })
    fit_keys = {
        "chi_square": "chisq",
        "degrees_of_freedom": "df",
        "p_value": "pvalue",
        "cfi": "cfi",
        "tli": "tli",
        "rmsea": "rmsea",
        "srmr": "srmr",
        # lavaan includes the full normal-theory log-likelihood constant in AIC/BIC.
        # QuickPLS v0.7.1 reports discrepancy-based information criteria, so those
        # are recorded in ordinary result exports but excluded from parity gating.
    }
    fit_comparisons = []
    for quick_key, lavaan_key in fit_keys.items():
        delta = abs_delta(quick["fit"].get(quick_key), lavaan["fit"].get(lavaan_key))
        if delta is not None:
            max_fit_delta = max(max_fit_delta, delta)
        fit_comparisons.append({
            "metric": quick_key,
            "quickpls": quick["fit"].get(quick_key),
            "lavaan": lavaan["fit"].get(lavaan_key),
            "abs_delta": delta,
        })
    return {
        "model": model,
        "quickpls_converged": quick["converged"],
        "matched_parameters": len(comparisons),
        "max_estimate_delta": max_estimate_delta,
        "max_standard_error_delta": max_standard_error_delta,
        "max_z_delta": max_z_delta,
        "max_p_delta": max_p_delta,
        "max_standardized_delta": max_standardized_delta,
        "max_fit_delta": max_fit_delta,
        "passed": quick["converged"]
        and max_estimate_delta <= 1e-6
        and max_fit_delta <= 1e-6
        and max_standardized_delta <= 1e-5
        and max_standard_error_delta <= 1e-4
        and max_z_delta <= 1e-4
        and max_p_delta <= 1e-4,
        "parameters": comparisons,
        "standardized": standardized_comparisons,
        "fit": fit_comparisons,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--section", choices=["all", "cfa", "sem"], default="all")
    args = parser.parse_args()
    RESULTS.mkdir(parents=True, exist_ok=True)
    models = []
    if args.section in ("all", "cfa"):
        models += ["one_factor_cfa", "two_factor_cfa", "three_factor_cfa"]
    if args.section in ("all", "sem"):
        models += ["latent_regression_sem", "latent_mediation_sem", "correlated_exogenous_sem"]
    report = {
        "kind": "cbsem_lavaan_reference_v1",
        "rscript": rscript_path(),
        "lavaan": "0.7.2",
        "models": [],
    }
    for model in models:
        report["models"].append(compare_model(model))
    report["status"] = "passed" if all(item["passed"] for item in report["models"]) else "failed"
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({
        "status": report["status"],
        "models": [{key: item[key] for key in ("model", "matched_parameters", "max_estimate_delta", "max_standard_error_delta", "max_z_delta", "max_p_delta", "max_standardized_delta", "max_fit_delta", "passed")} for item in report["models"]],
    }, indent=2))
    if report["status"] != "passed":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
