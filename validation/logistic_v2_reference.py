#!/usr/bin/env python3
"""Independent qualification reference for bounded logistic regression v2.

The script intentionally does not build QuickPLS. Point ``QUICKPLS_CLI_PATH``
at an already-built CLI, or build the CLI in the coordinated release gate.
R and Python are validation-only references; neither is a runtime dependency.
"""

from __future__ import annotations

import csv
import json
import math
import os
import random
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

import numpy as np
from scipy.optimize import minimize
from scipy.stats import chi2, norm

from r_runtime import find_rscript_optional


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
FIXTURE = RESULTS / "logistic_v2_reference.csv"
RECIPE = RESULTS / "logistic_v2_reference.recipe.json"
QUICKPLS_OUTPUT = RESULTS / "logistic_v2_reference_quickpls.json"
REPORT = RESULTS / "logistic_v2_reference_report.json"
METHOD_VERSION = "regression_logistic_v2"
FEATURE_ID = "qpls3.standalone.logistic"
CATALOGUE_SNAPSHOT_DATE = "2026-08-12"
TOLERANCE = 2e-6


def configured_cli() -> Path:
    configured = os.environ.get("QUICKPLS_CLI_PATH", "").strip()
    if configured:
        return Path(configured).resolve()
    release = ROOT / "target" / "release" / "qpls.exe"
    return release if release.is_file() else ROOT / "target" / "debug" / "qpls.exe"


CLI = configured_cli()


def require_cli() -> None:
    if not CLI.is_file():
        raise SystemExit(
            "QuickPLS CLI is not built. Build it in the coordinated serial gate "
            "or set QUICKPLS_CLI_PATH; this validation script will not start Cargo."
        )


def run(command: list[str], *, timeout: int = 180) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )


def require_success(completed: subprocess.CompletedProcess[str], label: str) -> None:
    if completed.returncode != 0:
        raise RuntimeError(
            f"{label} failed with exit code {completed.returncode}\n"
            f"stdout={completed.stdout[-2000:]}\nstderr={completed.stderr[-2000:]}"
        )


def write_main_fixture(path: Path) -> None:
    rng = random.Random(20260812)
    rows: list[dict[str, str]] = []
    for index in range(1, 181):
        x = math.sin(index / 7.0) + (index % 11) * 0.075 + rng.gauss(0.0, 0.22)
        z = math.cos(index / 9.0) - (index % 5) * 0.055 + rng.gauss(0.0, 0.18)
        control = math.log(index + 2.0) * 0.16 + rng.gauss(0.0, 0.20)
        eta = -0.42 + 0.91 * x - 0.63 * z + 0.38 * control
        probability = 1.0 / (1.0 + math.exp(-eta))
        outcome = 1 if rng.random() < probability else 0
        row = {
            "outcome": str(outcome),
            "x": f"{x:.12f}",
            "z": f"{z:.12f}",
            "control": f"{control:.12f}",
        }
        # Missingness is deliberate. All references use exactly the same
        # listwise-complete rows and report the omitted-row count.
        if index in {17, 71, 143}:
            row["x"] = ""
        if index in {29, 109}:
            row["outcome"] = ""
        rows.append(row)
    write_rows(path, rows)


def write_rows(path: Path, rows: Iterable[dict[str, str]]) -> None:
    rows = list(rows)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=["outcome", "x", "z", "control"])
        writer.writeheader()
        writer.writerows(rows)


def load_complete_cases(path: Path) -> tuple[np.ndarray, np.ndarray, int]:
    complete: list[list[float]] = []
    total = 0
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            total += 1
            raw = [row["outcome"], row["x"], row["z"], row["control"]]
            if any(value is None or not value.strip() for value in raw):
                continue
            values = [float(value) for value in raw]
            if not all(math.isfinite(value) for value in values):
                continue
            complete.append(values)
    matrix = np.asarray(complete, dtype=float)
    y = matrix[:, 0]
    design = np.column_stack([np.ones(len(matrix)), matrix[:, 1:]])
    return y, design, total - len(matrix)


def empty_model() -> dict[str, Any]:
    return {
        "id": "00000000-0000-0000-0000-000000030101",
        "name": "Logistic v2 standalone reference",
        "constructs": [],
        "paths": [],
        "controls": [],
        "higher_order_constructs": [],
        "interactions": [],
    }


def recipe_payload(fingerprint: str, *, seed: int = 20260812) -> dict[str, Any]:
    return {
        "schema_version": 3,
        "id": "00000000-0000-0000-0000-000000030102",
        "created_at": "2026-08-12T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": empty_model(),
        "settings": {
            "method": "regression",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "studentized_inner_samples": 0,
            "permutation_samples": 0,
            "seed": seed,
            "workers": 1,
            "confidence_level": 0.95,
            "preprocessing": "unstandardized",
            "missing_data": "listwise_deletion",
            "case_weight_column": None,
        },
        "method_config": {
            "kind": "regression",
            "outcome": "outcome",
            "predictors": ["x", "z"],
            "controls": ["control"],
            "model": {"type": "logistic"},
        },
        "metadata": {"fixture": "logistic_v2_reference"},
    }


def dataset_fingerprint(data_path: Path, temporary: Path, name: str) -> str:
    project = temporary / f"{name}.qpls"
    imported = run([str(CLI), "import", str(data_path), str(project), "--name", name])
    require_success(imported, f"import {name}")
    inspected = run([str(CLI), "inspect", str(project), "--json"])
    require_success(inspected, f"inspect {name}")
    return json.loads(inspected.stdout)["datasets"][0]["fingerprint"]


def run_recipe(
    data_path: Path,
    temporary: Path,
    name: str,
    *,
    seed: int = 20260812,
    workers: int = 1,
    predictors: list[str] | None = None,
) -> tuple[subprocess.CompletedProcess[str], dict[str, Any] | None, dict[str, Any]]:
    fingerprint = dataset_fingerprint(data_path, temporary, name)
    payload = recipe_payload(fingerprint, seed=seed)
    payload["settings"]["workers"] = workers
    if predictors is not None:
        payload["method_config"]["predictors"] = predictors
        payload["method_config"]["controls"] = []
    recipe_path = temporary / f"{name}.recipe.json"
    output_path = temporary / f"{name}.result.json"
    recipe_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    completed = run(
        [
            str(CLI),
            "run",
            str(recipe_path),
            "--data",
            str(data_path),
            "--output",
            str(output_path),
        ]
    )
    result = json.loads(output_path.read_text(encoding="utf-8")) if output_path.is_file() else None
    return completed, result, payload


@dataclass(frozen=True)
class IndependentFit:
    beta: np.ndarray
    standard_error: np.ndarray
    statistic: np.ndarray
    p_value: np.ndarray
    probability: np.ndarray
    log_likelihood: float
    null_log_likelihood: float
    likelihood_ratio: float
    likelihood_ratio_p: float


def python_reference(y: np.ndarray, design: np.ndarray) -> IndependentFit:
    def objective(beta: np.ndarray) -> float:
        eta = np.clip(design @ beta, -40.0, 40.0)
        # logaddexp is a stable independent expression of Bernoulli NLL.
        return float(np.sum(np.logaddexp(0.0, eta) - y * eta))

    def gradient(beta: np.ndarray) -> np.ndarray:
        eta = np.clip(design @ beta, -40.0, 40.0)
        probability = 1.0 / (1.0 + np.exp(-eta))
        return design.T @ (probability - y)

    fitted = minimize(
        objective,
        np.zeros(design.shape[1]),
        jac=gradient,
        method="BFGS",
        options={"gtol": 1e-11, "maxiter": 1000},
    )
    if not fitted.success and np.max(np.abs(gradient(fitted.x))) > 1e-7:
        raise RuntimeError(f"independent SciPy fit did not converge: {fitted.message}")
    beta = np.asarray(fitted.x, dtype=float)
    eta = design @ beta
    probability = 1.0 / (1.0 + np.exp(-eta))
    information = design.T @ ((probability * (1.0 - probability))[:, None] * design)
    covariance = np.linalg.inv(information)
    standard_error = np.sqrt(np.diag(covariance))
    statistic = beta / standard_error
    p_value = 2.0 * norm.sf(np.abs(statistic))
    log_likelihood = -objective(beta)
    prevalence = float(np.mean(y))
    null_log_likelihood = float(
        np.sum(y * math.log(prevalence) + (1.0 - y) * math.log(1.0 - prevalence))
    )
    likelihood_ratio = max(0.0, 2.0 * (log_likelihood - null_log_likelihood))
    return IndependentFit(
        beta=beta,
        standard_error=standard_error,
        statistic=statistic,
        p_value=p_value,
        probability=probability,
        log_likelihood=log_likelihood,
        null_log_likelihood=null_log_likelihood,
        likelihood_ratio=likelihood_ratio,
        likelihood_ratio_p=float(chi2.sf(likelihood_ratio, design.shape[1] - 1)),
    )


def maximum_delta(regression: dict[str, Any], reference: IndependentFit, y: np.ndarray) -> float:
    terms = ["intercept", "x", "z", "control"]
    coefficients = {row["term"]: row for row in regression["coefficients"]}
    critical = float(norm.ppf(0.975))
    differences: list[float] = []
    for index, term in enumerate(terms):
        row = coefficients[term]
        lower = reference.beta[index] - critical * reference.standard_error[index]
        upper = reference.beta[index] + critical * reference.standard_error[index]
        differences.extend(
            [
                abs(row["estimate"] - reference.beta[index]),
                abs(row["standard_error"] - reference.standard_error[index]),
                abs(row["statistic"] - reference.statistic[index]),
                abs(row["p_value_two_sided"] - reference.p_value[index]),
                abs(row["confidence_interval_lower"] - lower),
                abs(row["confidence_interval_upper"] - upper),
                abs(row["odds_ratio"] - math.exp(reference.beta[index])),
                abs(row["odds_ratio_confidence_interval_lower"] - math.exp(lower)),
                abs(row["odds_ratio_confidence_interval_upper"] - math.exp(upper)),
            ]
        )
    fit = regression["fit"]
    parameter_count = len(terms)
    expected_fit = {
        "log_likelihood": reference.log_likelihood,
        "null_log_likelihood": reference.null_log_likelihood,
        "deviance": -2.0 * reference.log_likelihood,
        "null_deviance": -2.0 * reference.null_log_likelihood,
        "pseudo_r_squared": 1.0 - reference.log_likelihood / reference.null_log_likelihood,
        "aic": -2.0 * reference.log_likelihood + 2.0 * parameter_count,
        "bic": -2.0 * reference.log_likelihood + math.log(len(y)) * parameter_count,
        "likelihood_ratio_chi_square": reference.likelihood_ratio,
        "likelihood_ratio_p_value": reference.likelihood_ratio_p,
    }
    differences.extend(abs(fit[key] - value) for key, value in expected_fit.items())
    for row, expected_probability, actual in zip(
        regression["predictions"], reference.probability, y, strict=True
    ):
        differences.extend(
            [
                abs(row["probability"] - expected_probability),
                abs(row["fitted"] - expected_probability),
                abs(row["residual"] - (actual - expected_probability)),
            ]
        )
    predicted = reference.probability >= 0.5
    positive = y == 1.0
    expected_classification = {
        "true_positive": int(np.sum(predicted & positive)),
        "true_negative": int(np.sum(~predicted & ~positive)),
        "false_positive": int(np.sum(predicted & ~positive)),
        "false_negative": int(np.sum(~predicted & positive)),
        "accuracy": float(np.mean(predicted == positive)),
        "sensitivity": float(np.sum(predicted & positive) / np.sum(positive)),
        "specificity": float(np.sum(~predicted & ~positive) / np.sum(~positive)),
    }
    classification = regression["logistic"]["classification"]
    for key, value in expected_classification.items():
        differences.append(abs(classification[key] - value))
    return max(differences)


def r_reference(data_path: Path, temporary: Path) -> dict[str, Any]:
    found = find_rscript_optional()
    if found is None:
        return {"available": False, "passed": False, "reason": "Rscript not found"}
    rscript, version = found
    script = temporary / "logistic_v2_reference.R"
    output = temporary / "logistic_v2_r_reference.json"
    script.write_text(
        f"""
data <- read.csv({json.dumps(str(data_path))}, na.strings=c("", "NA"))
fit <- glm(outcome ~ x + z + control, data=data, family=binomial(link="logit"), na.action=na.omit)
summary_fit <- summary(fit)
beta <- coef(fit)
se <- summary_fit$coefficients[, "Std. Error"]
z <- summary_fit$coefficients[, "z value"]
p <- summary_fit$coefficients[, "Pr(>|z|)"]
critical <- qnorm(0.975)
probability <- as.numeric(predict(fit, type="response"))
actual <- model.response(model.frame(fit))
predicted <- probability >= 0.5
tp <- sum(predicted & actual == 1)
tn <- sum(!predicted & actual == 0)
fp <- sum(predicted & actual == 0)
fn <- sum(!predicted & actual == 1)
lr <- fit$null.deviance - fit$deviance
payload <- list(
  version=R.version.string,
  terms=names(beta),
  coefficients=as.numeric(beta),
  standard_error=as.numeric(se),
  statistic=as.numeric(z),
  p_value=as.numeric(p),
  ci_lower=as.numeric(beta - critical * se),
  ci_upper=as.numeric(beta + critical * se),
  odds_ratio=as.numeric(exp(beta)),
  odds_ratio_ci_lower=as.numeric(exp(beta - critical * se)),
  odds_ratio_ci_upper=as.numeric(exp(beta + critical * se)),
  probability=probability,
  log_likelihood=as.numeric(logLik(fit)),
  null_log_likelihood=-fit$null.deviance / 2,
  deviance=fit$deviance,
  null_deviance=fit$null.deviance,
  aic=AIC(fit),
  bic=BIC(fit),
  likelihood_ratio_chi_square=lr,
  likelihood_ratio_p_value=pchisq(lr, df=fit$df.null-fit$df.residual, lower.tail=FALSE),
  classification=list(
    true_positive=tp, true_negative=tn, false_positive=fp, false_negative=fn,
    accuracy=(tp+tn)/length(actual), sensitivity=tp/(tp+fn), specificity=tn/(tn+fp)
  )
)
writeLines(jsonlite::toJSON(payload, auto_unbox=TRUE, digits=16), {json.dumps(str(output))})
""",
        encoding="utf-8",
    )
    completed = run([rscript, str(script)])
    if completed.returncode != 0 or not output.is_file():
        return {
            "available": True,
            "passed": False,
            "version": version,
            "stderr_tail": completed.stderr[-1000:],
        }
    return {
        "available": True,
        "passed": True,
        "version": version,
        "data": json.loads(output.read_text(encoding="utf-8")),
    }


def maximum_r_delta(regression: dict[str, Any], reference: dict[str, Any]) -> float:
    terms = ["intercept" if value == "(Intercept)" else value for value in reference["terms"]]
    coefficients = {row["term"]: row for row in regression["coefficients"]}
    differences: list[float] = []
    fields = [
        ("estimate", "coefficients"),
        ("standard_error", "standard_error"),
        ("statistic", "statistic"),
        ("p_value_two_sided", "p_value"),
        ("confidence_interval_lower", "ci_lower"),
        ("confidence_interval_upper", "ci_upper"),
        ("odds_ratio", "odds_ratio"),
        ("odds_ratio_confidence_interval_lower", "odds_ratio_ci_lower"),
        ("odds_ratio_confidence_interval_upper", "odds_ratio_ci_upper"),
    ]
    for index, term in enumerate(terms):
        for quickpls_field, reference_field in fields:
            differences.append(
                abs(coefficients[term][quickpls_field] - reference[reference_field][index])
            )
    fit = regression["fit"]
    for field in [
        "log_likelihood",
        "null_log_likelihood",
        "deviance",
        "null_deviance",
        "aic",
        "bic",
        "likelihood_ratio_chi_square",
        "likelihood_ratio_p_value",
    ]:
        differences.append(abs(fit[field] - reference[field]))
    differences.extend(
        abs(row["probability"] - expected)
        for row, expected in zip(regression["predictions"], reference["probability"], strict=True)
    )
    classification = regression["logistic"]["classification"]
    differences.extend(
        abs(classification[field] - reference["classification"][field])
        for field in reference["classification"]
    )
    return max(differences)


def failure_case(
    temporary: Path,
    name: str,
    rows: list[dict[str, str]],
    expected_text: str,
    *,
    predictors: list[str] | None = None,
) -> dict[str, Any]:
    path = temporary / f"{name}.csv"
    write_rows(path, rows)
    completed, result, _ = run_recipe(path, temporary, name, predictors=predictors)
    combined = f"{completed.stdout}\n{completed.stderr}".lower()
    return {
        "passed": completed.returncode != 0 and result is None and expected_text in combined,
        "exit_code": completed.returncode,
        "expected_text": expected_text,
        "stderr_tail": completed.stderr[-500:],
    }


def negative_guards(temporary: Path) -> dict[str, Any]:
    base = [
        {
            "outcome": str(index % 2),
            "x": str(index / 10.0),
            "z": str(math.sin(index)),
            "control": str(math.cos(index)),
        }
        for index in range(1, 41)
    ]
    invalid = [dict(row) for row in base]
    invalid[7]["outcome"] = "2"
    single = [dict(row, outcome="0") for row in base]
    collinear = [dict(row, z=row["x"]) for row in base]
    separated = [dict(row, outcome="1" if float(row["x"]) > 2.0 else "0") for row in base]
    workers_path = temporary / "logistic_workers_two.csv"
    write_rows(workers_path, base)
    workers_completed, workers_result, _ = run_recipe(
        workers_path,
        temporary,
        "logistic_workers_two",
        workers=2,
    )
    workers_text = f"{workers_completed.stdout}\n{workers_completed.stderr}".lower()
    return {
        "non_binary": failure_case(
            temporary,
            "logistic_non_binary",
            invalid,
            "exact numeric 0 and 1",
        ),
        "single_class": failure_case(
            temporary,
            "logistic_single_class",
            single,
            "both 0 and 1",
        ),
        "rank_deficient": failure_case(
            temporary,
            "logistic_rank_deficient",
            collinear,
            "rank",
        ),
        "separation": failure_case(
            temporary,
            "logistic_separation",
            separated,
            "separation",
        ),
        "workers_fixed": {
            "passed": (
                workers_completed.returncode != 0
                and workers_result is None
                and "worker" in workers_text
            ),
            "exit_code": workers_completed.returncode,
            "expected_text": "worker",
            "stderr_tail": workers_completed.stderr[-500:],
        },
    }


def main() -> int:
    require_cli()
    RESULTS.mkdir(parents=True, exist_ok=True)
    write_main_fixture(FIXTURE)
    with tempfile.TemporaryDirectory(prefix="quickpls-logistic-v2-") as directory:
        temporary = Path(directory)
        first, first_result, first_recipe = run_recipe(FIXTURE, temporary, "logistic_v2_main")
        require_success(first, "QuickPLS logistic v2 main run")
        assert first_result is not None
        second, second_result, _ = run_recipe(
            FIXTURE,
            temporary,
            "logistic_v2_seed_repeat",
            seed=987654321,
        )
        require_success(second, "QuickPLS logistic v2 deterministic repeat")
        assert second_result is not None
        regression = first_result["payload"]["estimation"]["regression"]
        repeated_regression = second_result["payload"]["estimation"]["regression"]
        y, design, omitted = load_complete_cases(FIXTURE)
        independent = python_reference(y, design)
        python_delta = maximum_delta(regression, independent, y)
        r_result = r_reference(FIXTURE, temporary)
        r_delta = (
            maximum_r_delta(regression, r_result["data"])
            if r_result.get("passed") is True
            else None
        )
        guards = negative_guards(temporary)

        # Retain only compact, reproducible evidence—not disposable projects.
        RECIPE.write_text(json.dumps(first_recipe, indent=2), encoding="utf-8")
        QUICKPLS_OUTPUT.write_text(json.dumps(first_result, indent=2), encoding="utf-8")

    estimation = first_result["payload"]["estimation"]
    diagnostics = regression["logistic"]
    profile = diagnostics["outcome_profile"]
    convergence = diagnostics["convergence"]
    classification = diagnostics["classification"]
    method_contract = (
        first_result.get("status") == "completed"
        and first_result.get("provenance", {}).get("method") == "regression"
        and first_result.get("provenance", {}).get("method_version") == METHOD_VERSION
        and estimation.get("method_version") == METHOD_VERSION
        and regression.get("method_version") == METHOD_VERSION
        and regression.get("regression_type") == "logistic"
        and regression.get("outcome") == "outcome"
        and regression.get("predictors") == ["x", "z"]
        and regression.get("controls") == ["control"]
        and regression.get("observations") == len(y)
        and profile
        == {
            "outcome": "outcome",
            "coding": "numeric_0_1_exact_v1",
            "complete_cases": len(y),
            "omitted_cases": omitted,
            "zero_count": int(np.sum(y == 0.0)),
            "one_count": int(np.sum(y == 1.0)),
            "invalid_count": 0,
            "prevalence": float(np.mean(y)),
            "readiness": "ready",
        }
        and convergence.get("algorithm") == "deterministic_newton_irls_v1"
        and convergence.get("converged") is True
        and convergence.get("max_iterations") == 100
        and convergence.get("tolerance") == 1e-8
        and convergence.get("separation_probability_tolerance") == 1e-9
        and classification.get("threshold") == 0.5
        and regression.get("fit", {}).get("pseudo_r_squared_method") == "mcfadden_v1"
        and regression.get("fit", {}).get("likelihood_ratio_degrees_of_freedom") == 3
    )
    checks = {
        "quickpls_v2_exact_contract": method_contract,
        "independent_python_full_arithmetic": python_delta <= TOLERANCE,
        "external_r_glm_full_arithmetic": r_delta is not None and r_delta <= TOLERANCE,
        "seed_invariant_deterministic_result": regression == repeated_regression,
        "non_binary_outcome_rejected": guards["non_binary"]["passed"],
        "single_class_outcome_rejected": guards["single_class"]["passed"],
        "rank_deficiency_rejected": guards["rank_deficient"]["passed"],
        "complete_separation_rejected": guards["separation"]["passed"],
        "nondefault_worker_count_rejected": guards["workers_fixed"]["passed"],
    }
    report = {
        "schema_version": 1,
        "target": "binary_logistic_regression_v2_reference",
        "feature_id": FEATURE_ID,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "method_version": METHOD_VERSION,
        "tolerance": TOLERANCE,
        "passed": all(checks.values()),
        "checks": checks,
        "scope": {
            "outcome_coding": "exact numeric 0 and 1",
            "predictors_and_controls": "numeric observed variables",
            "intercept": True,
            "missing_data": "listwise_deletion",
            "confidence": "two_sided_95_percent_fixed",
            "classification_threshold": 0.5,
            "workers": 1,
            "multinomial_or_ordinal": False,
            "automatic_categorical_encoding": False,
            "weights_or_clusters": False,
            "firth_or_penalized_fit": False,
        },
        "fixture": {
            "rows": len(y) + omitted,
            "complete_cases": len(y),
            "omitted_cases": omitted,
            "zero_count": int(np.sum(y == 0.0)),
            "one_count": int(np.sum(y == 1.0)),
        },
        "maximum_absolute_difference_python": python_delta,
        "maximum_absolute_difference_r": r_delta,
        "tolerance": TOLERANCE,
        "r_reference": {key: value for key, value in r_result.items() if key != "data"},
        "negative_guards": guards,
        "artifacts": {
            "fixture": FIXTURE.relative_to(ROOT).as_posix(),
            "recipe": RECIPE.relative_to(ROOT).as_posix(),
            "quickpls_result": QUICKPLS_OUTPUT.relative_to(ROOT).as_posix(),
        },
        "runtime_boundary": (
            "Python, SciPy, and R glm are validation-only references and are not bundled or "
            "required by the QuickPLS desktop application."
        ),
    }
    REPORT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
