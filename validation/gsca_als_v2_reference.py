"""Independent numerical reference for the bounded QuickPLS GSCA ALS v2 scope.

The oracle minimizes the published GSCA global least-squares criterion with
SciPy SLSQP and explicit unit-component constraints.  It does not reuse or
translate the QuickPLS ALS updates.  The model combines one formative and one
reflective block and one recursive structural path.
"""

import csv
import json
import math
import subprocess
from pathlib import Path

import numpy as np
from scipy.optimize import minimize


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
DATA = RESULTS / "v08_extended_methods_fixture.csv"
RECIPE = RESULTS / "gsca_als_v2.recipe.json"
QUICKPLS = RESULTS / "gsca_als_v2_quickpls.json"
REPORT = RESULTS / "gsca_als_v2_reference_report.json"
CLI = ROOT / "target" / "debug" / "qpls.exe"
METHOD_VERSION = "gsca_als_v2"
ALGORITHM_VERSION = "alternating_least_squares_v1"
TOLERANCE = 2e-6
INDICATORS = ["g1", "g2", "g3", "h1", "h2"]


def load_unit_columns():
    with DATA.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    complete = [
        [float(row[indicator]) for indicator in INDICATORS]
        for row in rows
        if all(row[indicator].strip() for indicator in INDICATORS)
    ]
    raw = np.asarray(complete, dtype=float)
    standardized = (raw - raw.mean(axis=0)) / raw.std(axis=0, ddof=1)
    return standardized / math.sqrt(raw.shape[0] - 1), raw.shape[0]


def normalized_ones(block):
    candidate = np.ones(block.shape[1], dtype=float)
    return candidate / np.linalg.norm(block @ candidate)


def unpack(parameters, z):
    w_g = parameters[0:3]
    w_h = parameters[3:5]
    c_h = parameters[5:7]
    path = float(parameters[7])
    gamma_g = z[:, 0:3] @ w_g
    gamma_h = z[:, 3:5] @ w_h
    return w_g, w_h, c_h, path, gamma_g, gamma_h


def objective(parameters, z):
    _, _, c_h, path, gamma_g, gamma_h = unpack(parameters, z)
    measurement = float(np.square(z[:, 0:3]).sum())
    measurement += float(np.square(z[:, 3:5] - np.outer(gamma_h, c_h)).sum())
    structural = float(np.square(gamma_g).sum())
    structural += float(np.square(gamma_h - path * gamma_g).sum())
    return measurement + structural


def unit_constraints(parameters, z):
    *_, gamma_g, gamma_h = unpack(parameters, z)
    return np.asarray(
        [float(gamma_g @ gamma_g - 1.0), float(gamma_h @ gamma_h - 1.0)]
    )


def solve_reference(z):
    w_g = normalized_ones(z[:, 0:3])
    w_h = normalized_ones(z[:, 3:5])
    gamma_g = z[:, 0:3] @ w_g
    gamma_h = z[:, 3:5] @ w_h
    c_h = z[:, 3:5].T @ gamma_h
    path = float(gamma_g @ gamma_h)
    initial = np.concatenate([w_g, w_h, c_h, [path]])
    starts = [initial]
    rng = np.random.default_rng(20260812)
    for _ in range(7):
        candidate_g = normalized_ones(z[:, 0:3] * rng.choice([-1.0, 1.0], 3))
        candidate_g *= rng.choice([-1.0, 1.0], 3)
        candidate_h = normalized_ones(z[:, 3:5] * rng.choice([-1.0, 1.0], 2))
        candidate_h *= rng.choice([-1.0, 1.0], 2)
        score_g = z[:, 0:3] @ candidate_g
        score_h = z[:, 3:5] @ candidate_h
        starts.append(
            np.concatenate(
                [
                    candidate_g,
                    candidate_h,
                    z[:, 3:5].T @ score_h,
                    [float(score_g @ score_h)],
                ]
            )
        )
    solutions = []
    for start in starts:
        solution = minimize(
            objective,
            start,
            args=(z,),
            method="SLSQP",
            constraints={"type": "eq", "fun": unit_constraints, "args": (z,)},
            options={"ftol": 1e-13, "maxiter": 10_000, "disp": False},
        )
        if solution.success and np.max(np.abs(unit_constraints(solution.x, z))) < 1e-8:
            solutions.append(solution)
    if not solutions:
        raise RuntimeError("independent GSCA constrained optimizer did not converge")
    solution = min(solutions, key=lambda item: item.fun)
    w_g, w_h, c_h, path, gamma_g, gamma_h = unpack(solution.x, z)
    if w_g.sum() < 0:
        w_g = -w_g
        gamma_g = -gamma_g
        path = -path
    if w_h.sum() < 0:
        w_h = -w_h
        gamma_h = -gamma_h
        c_h = -c_h
        path = -path
    loadings = np.concatenate([z[:, 0:3].T @ gamma_g, z[:, 3:5].T @ gamma_h])
    structural_residual = float(np.square(gamma_g).sum())
    structural_residual += float(np.square(gamma_h - path * gamma_g).sum())
    measurement_residual = float(np.square(z[:, 0:3]).sum())
    measurement_residual += float(
        np.square(z[:, 3:5] - np.outer(gamma_h, c_h)).sum()
    )
    fitted_h = path * gamma_g
    r_squared = 1.0 - float(np.square(gamma_h - fitted_h).sum())
    return {
        "weights": np.concatenate([w_g, w_h]),
        "loadings": loadings,
        "path": path,
        "r_squared": r_squared,
        "scores": np.column_stack([gamma_g, gamma_h]),
        "measurement_coefficients": c_h,
        "measurement_residual": measurement_residual,
        "structural_residual": structural_residual,
        "objective": measurement_residual + structural_residual,
        "optimizer_iterations": int(solution.nit),
        "optimizer_message": str(solution.message),
    }


def covariance_fit(z, reference):
    gamma = reference["scores"]
    c_h = reference["measurement_coefficients"]
    path = reference["path"]
    observed = z.shape[1]
    constructs = gamma.shape[1]
    width = observed + constructs
    coefficients = np.zeros((constructs, width), dtype=float)
    coefficients[1, 3:5] = c_h
    coefficients[0, observed + 1] = path
    residual = np.zeros((z.shape[0], width), dtype=float)
    residual[:, 0:3] = z[:, 0:3]
    residual[:, 3:5] = z[:, 3:5] - np.outer(gamma[:, 1], c_h)
    residual[:, observed] = gamma[:, 0]
    residual[:, observed + 1] = gamma[:, 1] - path * gamma[:, 0]
    residual_covariance = np.zeros((width, width), dtype=float)
    residual_covariance[:observed, :observed] = residual[:, :observed].T @ residual[:, :observed]
    residual_covariance[observed:, observed:] = residual[:, observed:].T @ residual[:, observed:]
    transition = np.zeros((width, width), dtype=float)
    transition[observed:, :] = coefficients
    inverse = np.linalg.inv(np.eye(width) - transition)
    implied = inverse.T @ residual_covariance @ inverse
    sample = z.T @ z
    difference = sample - implied[:observed, :observed]
    discrepancy = float(np.square(difference).sum())
    sample_total = float(np.square(sample).sum())
    standardized_lower = 0.0
    for row in range(observed):
        for column in range(row + 1):
            denominator = math.sqrt(abs(sample[row, row] * sample[column, column]))
            standardized_lower += float((difference[row, column] / denominator) ** 2)
    return {
        "gfi": 1.0 - discrepancy / sample_total,
        "srmr": math.sqrt(2.0 * standardized_lower / (observed * (observed + 1))),
        "covariance_discrepancy": discrepancy,
        "covariance_sample_total": sample_total,
        "standardized_residual_sum": standardized_lower,
    }


def run_quickpls():
    if not CLI.exists():
        raise FileNotFoundError(
            f"{CLI} is missing; build qpls-cli once before running this reference"
        )
    QUICKPLS.unlink(missing_ok=True)
    subprocess.run(
        [
            str(CLI),
            "run",
            str(RECIPE.relative_to(ROOT)),
            "--data",
            str(DATA.relative_to(ROOT)),
            "--output",
            str(QUICKPLS.relative_to(ROOT)),
            "--allow-internal-qualification",
        ],
        cwd=ROOT,
        check=True,
        stdout=subprocess.DEVNULL,
    )
    return json.loads(QUICKPLS.read_text(encoding="utf-8"))


def max_delta(left, right):
    return float(np.max(np.abs(np.asarray(left, dtype=float) - np.asarray(right, dtype=float))))


def main():
    z, observations = load_unit_columns()
    reference = solve_reference(z)
    covariance = covariance_fit(z, reference)
    payload = run_quickpls()
    estimation = payload["payload"]["estimation"]
    gsca = estimation["gsca"]
    quick_weights = [row["weight"] for row in gsca["weights"]]
    quick_loadings = [row["loading"] for row in gsca["loadings"]]
    quick_path = gsca["paths"][0]["coefficient"]
    quick_r_squared = gsca["r_squared"]["h"]
    observed_total = float(z.shape[1])
    component_total = 2.0
    expected_fit = 1.0 - reference["objective"] / (observed_total + component_total)
    expected_measurement_fit = 1.0 - reference["measurement_residual"] / observed_total
    expected_structural_fit = 1.0 - reference["structural_residual"] / component_total
    free_parameters = (3 - 1) + (2 - 1) + 2 + 1
    null_degrees = observations * z.shape[1]
    expected_adjusted_fit = 1.0 - (1.0 - expected_fit) * null_degrees / (
        null_degrees - free_parameters
    )
    deltas = {
        "weights": max_delta(quick_weights, reference["weights"]),
        "loadings": max_delta(quick_loadings, reference["loadings"]),
        "path": abs(quick_path - reference["path"]),
        "r_squared": abs(quick_r_squared - reference["r_squared"]),
        "objective": abs(gsca["objective"] - reference["objective"]),
        "fit": abs(gsca["fit"] - expected_fit),
        "measurement_fit": abs(gsca["measurement_fit"] - expected_measurement_fit),
        "structural_fit": abs(gsca["structural_fit"] - expected_structural_fit),
        "adjusted_fit": abs(gsca["adjusted_fit"] - expected_adjusted_fit),
        "gfi": abs(gsca["gfi"] - covariance["gfi"]),
        "srmr": abs(gsca["srmr"] - covariance["srmr"]),
        "covariance_discrepancy": abs(
            gsca["covariance_discrepancy"] - covariance["covariance_discrepancy"]
        ),
        "covariance_sample_total": abs(
            gsca["covariance_sample_total"] - covariance["covariance_sample_total"]
        ),
        "standardized_residual_sum": abs(
            gsca["standardized_residual_sum"] - covariance["standardized_residual_sum"]
        ),
    }
    contract_checks = {
        "method": payload["provenance"]["method"] == "gsca",
        "provenance": payload["provenance"]["method_version"] == METHOD_VERSION,
        "outer_version": estimation["method_version"] == METHOD_VERSION,
        "nested_version": gsca["method_version"] == METHOD_VERSION,
        "algorithm": gsca["algorithm"] == ALGORITHM_VERSION,
        "assessment_not_applicable": payload["payload"]["assessment"]
        == {
            "method_version": "assessment_not_applicable_v1",
            "warnings": [
                "PLS assessment is not applicable to GSCA ALS component-model estimation."
            ],
        },
        "no_fabricated_intervals": gsca["bootstrap_intervals"] == [],
        "no_pls_effects": estimation["effects"] == []
        and estimation["mediation"]["estimates"] == [],
        "converged": estimation["converged"] is True
        and gsca["converged"] is True
        and gsca["final_change"] <= 1e-7,
        "observations": gsca["observations"] == observations,
        "free_parameters": gsca["free_parameters"] == free_parameters,
    }
    report = {
        "schema_version": 1,
        "method_version": METHOD_VERSION,
        "reference": "independent_scipy_slsqp_global_gsca_criterion_v1",
        "fixture": str(DATA.relative_to(ROOT)).replace("\\", "/"),
        "recipe": str(RECIPE.relative_to(ROOT)).replace("\\", "/"),
        "observations": observations,
        "optimizer_iterations": reference["optimizer_iterations"],
        "optimizer_message": reference["optimizer_message"],
        "tolerance": TOLERANCE,
        "deltas": deltas,
        "contract_checks": contract_checks,
        "passed": max(deltas.values()) <= TOLERANCE and all(contract_checks.values()),
    }
    REPORT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
