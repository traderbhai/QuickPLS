"""Fresh deterministic simulation matrix for the frozen standalone PCA v1 scope."""

from __future__ import annotations

import argparse
import math
from pathlib import Path
from typing import Any

import numpy as np

from pca_v1_factory_common import (
    REPORT_ROOT,
    WORK_ROOT,
    run_pca,
    write_csv,
    write_identity_report,
)


TOLERANCE = 1e-6
SOURCE = "validation/pca_simulation.py"


def orient(vectors: np.ndarray) -> np.ndarray:
    oriented = vectors.copy()
    for component in range(oriented.shape[1]):
        pivot = max(
            range(oriented.shape[0]),
            key=lambda index: (abs(oriented[index, component]), index),
        )
        if oriented[pivot, component] < 0:
            oriented[:, component] *= -1.0
    return oriented


def complete_matrix(rows: list[list[float | None]]) -> np.ndarray:
    complete = [row for row in rows if all(value is not None for value in row)]
    return np.asarray(complete, dtype=float)


def independent_reference(rows: list[list[float | None]]) -> dict[str, Any]:
    matrix = complete_matrix(rows)
    standardized = (matrix - matrix.mean(axis=0)) / matrix.std(axis=0, ddof=1)
    correlation = np.cov(standardized, rowvar=False, ddof=1)
    values, vectors = np.linalg.eigh(correlation)
    order = np.argsort(values)[::-1]
    values = values[order]
    vectors = orient(vectors[:, order])
    loadings = vectors * np.sqrt(np.maximum(values, 0.0))
    scores = standardized @ vectors
    explained = values / values.sum()
    return {
        "matrix": matrix,
        "values": values,
        "vectors": vectors,
        "loadings": loadings,
        "scores": scores,
        "explained": explained,
        "cumulative": np.cumsum(explained),
    }


def expected_retained(reference: dict[str, Any], rule: str, components: int | None, threshold: float | None) -> int:
    if rule == "fixed":
        assert components is not None
        return components
    if rule == "kaiser":
        return max(1, int(np.count_nonzero(reference["values"] >= 1.0)))
    assert rule == "variance_threshold" and threshold is not None
    return int(np.flatnonzero(reference["cumulative"] >= threshold)[0] + 1)


def compare(result: dict[str, Any], variables: list[str], reference: dict[str, Any], retained: int) -> dict[str, Any]:
    pca = result["pca"]
    components = pca["components"]
    reference = {
        key: value.copy() if isinstance(value, np.ndarray) else value
        for key, value in reference.items()
    }
    value_errors = [
        abs(components[index]["eigenvalue"] - float(reference["values"][index]))
        for index in range(retained)
    ]
    explained_errors = [
        abs(components[index]["explained_variance"] - float(reference["explained"][index]))
        for index in range(retained)
    ]
    cumulative_errors = [
        abs(components[index]["cumulative_variance"] - float(reference["cumulative"][index]))
        for index in range(retained)
    ]
    loading_map = {
        (row["variable"], row["component"]): (row["loading"], row["weight"])
        for row in pca["loadings"]
    }
    for component in range(retained):
        component_id = f"PC{component + 1}"
        quick_vector = np.asarray(
            [loading_map[(variable, component_id)][1] for variable in variables]
        )
        if float(np.dot(quick_vector, reference["vectors"][:, component])) < 0:
            reference["vectors"][:, component] *= -1.0
            reference["loadings"][:, component] *= -1.0
            reference["scores"][:, component] *= -1.0
    loading_errors: list[float] = []
    weight_errors: list[float] = []
    for component in range(retained):
        component_id = f"PC{component + 1}"
        for variable_index, variable in enumerate(variables):
            loading, weight = loading_map[(variable, component_id)]
            loading_errors.append(
                abs(loading - float(reference["loadings"][variable_index, component]))
            )
            weight_errors.append(
                abs(weight - float(reference["vectors"][variable_index, component]))
            )
    score_map = {
        (row["observation"], row["component"]): row["score"]
        for row in pca["scores"]
    }
    score_errors: list[float] = []
    for component in range(retained):
        component_id = f"PC{component + 1}"
        for observation in range(reference["scores"].shape[0]):
            score_errors.append(
                abs(
                    score_map[(observation, component_id)]
                    - float(reference["scores"][observation, component])
                )
            )
    quick_vectors = np.asarray(
        [
            [loading_map[(variable, f"PC{component + 1}")][1] for component in range(retained)]
            for variable in variables
        ]
    )
    reference_vectors = reference["vectors"][:, :retained]
    subspace_error = float(
        np.linalg.norm(
            quick_vectors @ quick_vectors.T
            - reference_vectors @ reference_vectors.T,
            ord=2,
        )
    )
    maxima = {
        "eigenvalue": max(value_errors, default=0.0),
        "explained_variance": max(explained_errors, default=0.0),
        "cumulative_variance": max(cumulative_errors, default=0.0),
        "loading": max(loading_errors, default=0.0),
        "weight": max(weight_errors, default=0.0),
        "score": max(score_errors, default=0.0),
        "subspace": subspace_error,
    }
    return {
        "passed": (
            result["passed"]
            and pca["retained_components"] == retained
            and pca["observations"] == reference["matrix"].shape[0]
            and all(value <= TOLERANCE for value in maxima.values())
        ),
        "expected_retained_components": retained,
        "observed_retained_components": pca["retained_components"],
        "observations": pca["observations"],
        "max_abs_errors": maxima,
        "minimum_adjacent_eigenvalue_gap": float(
            min(
                (abs(reference["values"][i] - reference["values"][i + 1]) for i in range(len(variables) - 1)),
                default=math.inf,
            )
        ),
        "output": result["output"],
        "output_sha256": result["output_sha256"],
    }


def exact_correlation_sample(n: int, correlation: np.ndarray, seed: int, *, non_gaussian: bool = False) -> np.ndarray:
    rng = np.random.default_rng(seed)
    raw = rng.normal(size=(n, correlation.shape[0]))
    if non_gaussian:
        raw = np.sign(raw) * np.abs(raw) ** 1.5
    raw -= raw.mean(axis=0)
    covariance = np.cov(raw, rowvar=False, ddof=1)
    values, vectors = np.linalg.eigh(covariance)
    whitened = raw @ vectors @ np.diag(1.0 / np.sqrt(values)) @ vectors.T
    return whitened @ np.linalg.cholesky(correlation).T


def scenario_rows(index: int, *, n: int, p: int, non_gaussian: bool, missing: bool, near_tie: bool) -> list[list[float | None]]:
    if near_tie:
        if p != 4:
            raise ValueError("the frozen near-tie scenario requires p=4")
        correlation = np.asarray(
            [
                [1.0, 0.65, 0.0, 0.0],
                [0.65, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.02],
                [0.0, 0.0, 0.02, 1.0],
            ]
        )
    else:
        loadings = np.asarray(
            [min(0.78, 0.25 + 0.07 * variable + 0.01 * (index % 3)) for variable in range(p)]
        )
        correlation = np.outer(loadings, loadings)
        np.fill_diagonal(correlation, 1.0)
    matrix = exact_correlation_sample(
        n,
        correlation,
        2026081300 + index,
        non_gaussian=non_gaussian,
    )
    rows: list[list[float | None]] = matrix.tolist()
    if missing:
        for row_index in range(3, len(rows), 11):
            rows[row_index][(row_index + index) % p] = None
    return rows


def run_simulation_matrix() -> dict[str, Any]:
    scenarios = [
        {"name": "small_fixed", "n": 30, "p": 2, "rule": "fixed", "components": 2, "threshold": None, "non_gaussian": False, "missing": False, "near_tie": False},
        {"name": "medium_kaiser", "n": 60, "p": 3, "rule": "kaiser", "components": None, "threshold": None, "non_gaussian": False, "missing": False, "near_tie": False},
        {"name": "near_tie_fixed", "n": 90, "p": 4, "rule": "fixed", "components": 3, "threshold": None, "non_gaussian": False, "missing": False, "near_tie": True},
        {"name": "missing_threshold", "n": 100, "p": 5, "rule": "variance_threshold", "components": None, "threshold": 0.80, "non_gaussian": False, "missing": True, "near_tie": False},
        {"name": "nongaussian_fixed", "n": 140, "p": 5, "rule": "fixed", "components": 3, "threshold": None, "non_gaussian": True, "missing": False, "near_tie": False},
        {"name": "wide_threshold", "n": 45, "p": 8, "rule": "variance_threshold", "components": None, "threshold": 0.95, "non_gaussian": False, "missing": False, "near_tie": False},
        {"name": "large_kaiser", "n": 240, "p": 6, "rule": "kaiser", "components": None, "threshold": None, "non_gaussian": True, "missing": True, "near_tie": False},
    ]
    rows_out: list[dict[str, Any]] = []
    for index, scenario in enumerate(scenarios):
        variables = [f"v{column + 1}" for column in range(scenario["p"])]
        rows = scenario_rows(
            index,
            n=scenario["n"],
            p=scenario["p"],
            non_gaussian=scenario["non_gaussian"],
            missing=scenario["missing"],
            near_tie=scenario["near_tie"],
        )
        csv_path = WORK_ROOT / f"simulation_{scenario['name']}.csv"
        write_csv(csv_path, variables, rows)
        reference = independent_reference(rows)
        retained = expected_retained(
            reference,
            scenario["rule"],
            scenario["components"],
            scenario["threshold"],
        )
        result = run_pca(
            name=f"factory_simulation_{scenario['name']}",
            csv_path=csv_path,
            variables=variables,
            rule=scenario["rule"],
            components=scenario["components"],
            threshold=scenario["threshold"],
        )
        comparison = compare(result, variables, reference, retained)
        rows_out.append({**scenario, **comparison})
    coverage = {
        "sample_sizes": sorted({row["n"] for row in scenarios}),
        "variable_counts": sorted({row["p"] for row in scenarios}),
        "retention_rules": sorted({row["rule"] for row in scenarios}),
        "includes_missing_rows": any(row["missing"] for row in scenarios),
        "includes_non_gaussian_data": any(row["non_gaussian"] for row in scenarios),
        "includes_near_tied_eigenvalues": any(row["near_tie"] for row in scenarios),
    }
    return {
        "passed": all(row["passed"] for row in rows_out),
        "tolerance": TOLERANCE,
        "scenario_count": len(rows_out),
        "coverage": coverage,
        "scenarios": rows_out,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--detail-only", action="store_true")
    args = parser.parse_args()
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    detail = run_simulation_matrix()
    if not args.detail_only:
        path = write_identity_report(
            "simulation_report",
            passed=detail["passed"],
            checks=detail,
            extras=[
                SOURCE,
                "crates/qpls-core/src/contract.rs",
                "crates/qpls-estimation/src/pls.rs",
                "crates/qpls-runner/src/lib.rs",
                "crates/qpls-cli/src/main.rs",
            ],
        )
        print(f"wrote {path} | passed={detail['passed']}")
    return 0 if detail["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
