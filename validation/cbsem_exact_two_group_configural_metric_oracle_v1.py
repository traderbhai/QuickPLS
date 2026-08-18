#!/usr/bin/env python3
"""Generate immutable engine-reference evidence for exact two-group CFA invariance.

This validation-only harness independently fits configural and metric covariance-
structure ML models, checks the quarantined Rust engine, and runs lavaan when an
Rscript installation is available. It never qualifies or exposes product support.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import importlib.metadata
import json
import math
import platform
import re
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Mapping, Sequence

import numpy as np
from jsonschema import Draft202012Validator
from scipy.optimize import minimize
from scipy.stats import chi2


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST = ROOT / "validation/cbsem_exact_two_group_configural_metric_manifest_v2.json"
MANIFEST_SCHEMA = ROOT / "validation/cbsem_exact_two_group_configural_metric_manifest_v2.schema.json"
REPORT_SCHEMA = ROOT / "validation/cbsem_exact_two_group_configural_metric_report_v2.schema.json"
DEFAULT_OUTPUT = (
    ROOT
    / "validation/results/cbsem_exact_two_group_configural_metric/reference-report-v2.json"
)
VARIABLES = ("x1", "x2", "x3", "y1", "y2", "y3")
GROUPS = ("A", "B")
SHA256_PATTERN = re.compile(r"^[0-9a-f]{64}$")
PARAMETER_ID_MAP = {
    "parameter_6631_7831": "loading_x1",
    "parameter_6631_7832": "loading_x2",
    "parameter_6631_7833": "loading_x3",
    "parameter_6632_7931": "loading_y1",
    "parameter_6632_7932": "loading_y2",
    "parameter_6632_7933": "loading_y3",
    "reference_factor_covariance_f1_f2": "factor_covariance_f1_f2",
    "variance_6631": "factor_variance_f1",
    "variance_6632": "factor_variance_f2",
    "residual_variance_7831": "residual_variance_x1",
    "residual_variance_7832": "residual_variance_x2",
    "residual_variance_7833": "residual_variance_x3",
    "residual_variance_7931": "residual_variance_y1",
    "residual_variance_7932": "residual_variance_y2",
    "residual_variance_7933": "residual_variance_y3",
}
PARAMETER_ORDER = (
    "loading_x1",
    "loading_x2",
    "loading_x3",
    "loading_y1",
    "loading_y2",
    "loading_y3",
    "factor_variance_f1",
    "factor_variance_f2",
    "factor_covariance_f1_f2",
    "residual_variance_x1",
    "residual_variance_x2",
    "residual_variance_x3",
    "residual_variance_y1",
    "residual_variance_y2",
    "residual_variance_y3",
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_sha256(value: Any) -> str:
    payload = json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8-sig") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain one JSON object")
    return value


def write_new_json(path: Path, value: Mapping[str, Any]) -> str:
    if path.exists():
        raise FileExistsError(f"append-only evidence path already exists: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = (
        json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n"
    ).encode("utf-8")
    with path.open("xb") as handle:
        handle.write(payload)
    return hashlib.sha256(payload).hexdigest()


def validate_manifest(path: Path) -> tuple[dict[str, Any], str]:
    if path.resolve() != DEFAULT_MANIFEST.resolve():
        raise ValueError("only the checked-in immutable manifest is accepted")
    schema = load_json(MANIFEST_SCHEMA)
    Draft202012Validator.check_schema(schema)
    manifest = load_json(path)
    Draft202012Validator(schema).validate(manifest)
    roles = [binding["role"] for binding in manifest["source_bindings"]]
    paths = [binding["path"] for binding in manifest["source_bindings"]]
    if len(roles) != len(set(roles)) or len(paths) != len(set(paths)):
        raise ValueError("source binding roles and paths must each be unique")
    for binding in manifest["source_bindings"]:
        bound_path = (ROOT / binding["path"]).resolve()
        if not bound_path.is_file():
            raise ValueError(f"bound source is absent: {bound_path}")
        observed = sha256(bound_path)
        if observed != binding["sha256"]:
            raise ValueError(f"immutable source binding drifted: {binding['path']}")
    superseded = manifest["supersedes"]
    superseded_path = (ROOT / superseded["path"]).resolve()
    if not superseded_path.is_file() or sha256(superseded_path) != superseded["sha256"]:
        raise ValueError("superseded evidence is absent or changed")
    return manifest, sha256(path)


def read_fixture(
    path: Path, expected_group_sizes: Mapping[str, int] | None = None
) -> dict[str, np.ndarray]:
    rows: dict[str, list[list[float]]] = {group: [] for group in GROUPS}
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        if tuple(reader.fieldnames or ()) != ("group", *VARIABLES):
            raise ValueError("fixture header differs from the frozen contract")
        for row_index, row in enumerate(reader):
            group = row.get("group")
            if group not in rows:
                raise ValueError(f"row {row_index} has undeclared group {group!r}")
            values = [float(row[variable]) for variable in VARIABLES]
            if not all(math.isfinite(value) for value in values):
                raise ValueError(f"row {row_index} contains a non-finite value")
            rows[group].append(values)
    arrays = {group: np.asarray(rows[group], dtype=np.float64) for group in GROUPS}
    expected = dict(expected_group_sizes or {"A": 180, "B": 220})
    if {group: len(array) for group, array in arrays.items()} != expected:
        raise ValueError(f"fixture group counts differ from the expected authority {expected}")
    return arrays


def validate_label_swap(
    primary: Mapping[str, np.ndarray], swapped: Mapping[str, np.ndarray]
) -> None:
    if not np.array_equal(primary["A"], swapped["B"]):
        raise ValueError("label-swap B rows are not byte-numeric copies of primary A")
    if not np.array_equal(primary["B"], swapped["A"]):
        raise ValueError("label-swap A rows are not byte-numeric copies of primary B")


def moments(rows: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    observed_means = rows.mean(axis=0)
    centered = rows - observed_means
    covariance_ml = centered.T @ centered / rows.shape[0]
    sign, _ = np.linalg.slogdet(covariance_ml)
    if sign != 1.0:
        raise ValueError("fixture group ML-N covariance is not positive definite")
    return covariance_ml, observed_means


def implied_covariance(loadings: np.ndarray, nuisance: np.ndarray) -> np.ndarray:
    factor_variances = np.exp(nuisance[:2])
    factor_correlation = math.tanh(float(nuisance[2]))
    factor_covariance = factor_correlation * math.sqrt(
        float(factor_variances[0] * factor_variances[1])
    )
    phi = np.asarray(
        [
            [factor_variances[0], factor_covariance],
            [factor_covariance, factor_variances[1]],
        ]
    )
    residual_variances = np.exp(nuisance[3:9])
    loading_matrix = np.asarray(
        [
            [1.0, 0.0],
            [loadings[0], 0.0],
            [loadings[1], 0.0],
            [0.0, 1.0],
            [0.0, loadings[2]],
            [0.0, loadings[3]],
        ]
    )
    return loading_matrix @ phi @ loading_matrix.T + np.diag(residual_variances)


def fml(sample_covariance: np.ndarray, sigma: np.ndarray) -> float:
    sigma_sign, sigma_logdet = np.linalg.slogdet(sigma)
    sample_sign, sample_logdet = np.linalg.slogdet(sample_covariance)
    if sigma_sign != 1.0 or sample_sign != 1.0:
        return math.inf
    value = (
        sigma_logdet
        + float(np.trace(np.linalg.solve(sigma, sample_covariance)))
        - sample_logdet
        - sample_covariance.shape[0]
    )
    return float(value) if math.isfinite(float(value)) else math.inf


def parameter_projection(loadings: np.ndarray, nuisance: np.ndarray) -> dict[str, float]:
    variances = np.exp(nuisance[:2])
    covariance = math.tanh(float(nuisance[2])) * math.sqrt(
        float(variances[0] * variances[1])
    )
    residuals = np.exp(nuisance[3:9])
    values = {
        "loading_x1": 1.0,
        "loading_x2": float(loadings[0]),
        "loading_x3": float(loadings[1]),
        "loading_y1": 1.0,
        "loading_y2": float(loadings[2]),
        "loading_y3": float(loadings[3]),
        "factor_variance_f1": float(variances[0]),
        "factor_variance_f2": float(variances[1]),
        "factor_covariance_f1_f2": float(covariance),
    }
    values.update(
        {
            f"residual_variance_{variable}": float(value)
            for variable, value in zip(VARIABLES, residuals, strict=True)
        }
    )
    return {name: values[name] for name in PARAMETER_ORDER}


def initial_group_vector() -> np.ndarray:
    return np.concatenate(
        (
            np.asarray([0.75, 0.65, 0.75, 0.65]),
            np.log(np.asarray([0.8, 0.8])),
            np.asarray([np.arctanh(0.2)]),
            np.log(np.asarray([0.4] * 6)),
        )
    )


def optimizer_receipt(result: Any) -> dict[str, Any]:
    gradient = np.asarray(result.jac, dtype=float)
    return {
        "method": "scipy_optimize_minimize_l_bfgs_b_transformed_open_domains_v1",
        "success": bool(result.success),
        "status": int(result.status),
        "message": str(result.message),
        "iterations": int(result.nit),
        "function_evaluations": int(result.nfev),
        "maximum_absolute_gradient": float(np.max(np.abs(gradient))),
    }


def fit_python_reference(groups: Mapping[str, np.ndarray]) -> dict[str, Any]:
    group_moments = {group: moments(groups[group]) for group in GROUPS}
    group_sizes = {group: int(groups[group].shape[0]) for group in GROUPS}
    configural_results: dict[str, Any] = {}
    configural_vectors: dict[str, np.ndarray] = {}
    for group in GROUPS:
        covariance = group_moments[group][0]

        def configural_objective(vector: np.ndarray) -> float:
            return fml(covariance, implied_covariance(vector[:4], vector[4:]))

        fit = minimize(
            configural_objective,
            initial_group_vector(),
            method="L-BFGS-B",
            options={"ftol": 1.0e-15, "gtol": 1.0e-10, "maxiter": 5000, "maxls": 100},
        )
        if not fit.success or not math.isfinite(float(fit.fun)):
            raise ValueError(f"independent configural optimizer failed for group {group}: {fit.message}")
        configural_vectors[group] = np.asarray(fit.x, dtype=float)
        sigma = implied_covariance(fit.x[:4], fit.x[4:])
        configural_results[group] = {
            "group": group,
            "sample_size": group_sizes[group],
            "objective": float(fit.fun),
            "chi_square": float(group_sizes[group] * fit.fun),
            "parameters": parameter_projection(fit.x[:4], fit.x[4:]),
            "implied_covariance": sigma.tolist(),
            "optimizer": optimizer_receipt(fit),
        }

    shared_start = np.mean(
        np.stack([configural_vectors[group][:4] for group in GROUPS]), axis=0
    )
    metric_start = np.concatenate(
        (
            shared_start,
            configural_vectors["A"][4:],
            configural_vectors["B"][4:],
        )
    )
    total_n = sum(group_sizes.values())

    def metric_objective(vector: np.ndarray) -> float:
        total = 0.0
        for index, group in enumerate(GROUPS):
            nuisance = vector[4 + index * 9 : 4 + (index + 1) * 9]
            total += group_sizes[group] * fml(
                group_moments[group][0], implied_covariance(vector[:4], nuisance)
            )
        return total / total_n

    metric_fit = minimize(
        metric_objective,
        metric_start,
        method="L-BFGS-B",
        options={"ftol": 1.0e-15, "gtol": 1.0e-10, "maxiter": 10000, "maxls": 100},
    )
    if not metric_fit.success or not math.isfinite(float(metric_fit.fun)):
        raise ValueError(f"independent metric optimizer failed: {metric_fit.message}")
    metric_results: dict[str, Any] = {}
    for index, group in enumerate(GROUPS):
        nuisance = metric_fit.x[4 + index * 9 : 4 + (index + 1) * 9]
        sigma = implied_covariance(metric_fit.x[:4], nuisance)
        objective = fml(group_moments[group][0], sigma)
        metric_results[group] = {
            "group": group,
            "sample_size": group_sizes[group],
            "objective": objective,
            "chi_square": group_sizes[group] * objective,
            "parameters": parameter_projection(metric_fit.x[:4], nuisance),
            "implied_covariance": sigma.tolist(),
        }

    configural_chi_square = sum(row["chi_square"] for row in configural_results.values())
    metric_chi_square = sum(row["chi_square"] for row in metric_results.values())
    lrt = metric_chi_square - configural_chi_square
    if lrt < -1.0e-10 * max(1.0, abs(configural_chi_square), abs(metric_chi_square)):
        raise ValueError("independent metric fit is unexpectedly better than configural")
    lrt = max(0.0, lrt)
    moment_projection = {
        group: {
            "group": group,
            "sample_size": group_sizes[group],
            "observed_means": group_moments[group][1].tolist(),
            "covariance_ml": group_moments[group][0].tolist(),
            "complete_rows_sha256": canonical_sha256(groups[group].tolist()),
        }
        for group in GROUPS
    }
    return {
        "runtime": {
            "python_version": platform.python_version(),
            "numpy_version": importlib.metadata.version("numpy"),
            "scipy_version": importlib.metadata.version("scipy"),
            "optimizer_contract": {
                "method": "L-BFGS-B",
                "ftol": 1.0e-15,
                "gtol": 1.0e-10,
                "configural_maxiter": 5000,
                "metric_maxiter": 10000,
                "maxls": 100,
                "parameterization": "marker_loadings_fixed_one_log_variances_tanh_factor_correlation",
            },
        },
        "moments": moment_projection,
        "configural": {
            "free_dimensions": 26,
            "observed_moments": 42,
            "degrees_of_freedom": 16,
            "objective": configural_chi_square / total_n,
            "chi_square": configural_chi_square,
            "groups": [configural_results[group] for group in GROUPS],
        },
        "metric": {
            "free_dimensions": 22,
            "observed_moments": 42,
            "degrees_of_freedom": 20,
            "objective": metric_chi_square / total_n,
            "chi_square": metric_chi_square,
            "groups": [metric_results[group] for group in GROUPS],
            "optimizer": optimizer_receipt(metric_fit),
        },
        "likelihood_ratio_test": {
            "statistic": lrt,
            "degrees_of_freedom": 4,
            "upper_tail_p_value": float(chi2.sf(lrt, 4)),
        },
    }


def run_engine(
    binary: Path, manifest: Mapping[str, Any]
) -> tuple[dict[str, Any], str, dict[str, Any]]:
    if not binary.is_file():
        raise ValueError(f"engine validation binary is absent: {binary}")
    binary_binding = next(
        binding
        for binding in manifest["source_bindings"]
        if binding["role"] == "release_validation_binary"
    )
    if binary.resolve() != (ROOT / binary_binding["path"]).resolve():
        raise ValueError("--engine-binary differs from the frozen release binary binding")
    command = [
        str(binary),
        manifest["fixture_contract"]["primary_path"],
        manifest["fixture_contract"]["label_swap_path"],
    ]
    raw_outputs: list[bytes] = []
    payloads: list[dict[str, Any]] = []
    canonical_digests: list[str] = []
    raw_digests: list[str] = []
    for _ in range(2):
        completed = subprocess.run(
            command,
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=False,
            timeout=manifest["execution_contract"]["engine_timeout_seconds"],
        )
        payload = json.loads(completed.stdout)
        if not isinstance(payload, dict):
            raise ValueError("engine output is not one JSON object")
        raw_outputs.append(completed.stdout)
        payloads.append(payload)
        raw_digests.append(hashlib.sha256(completed.stdout).hexdigest())
        canonical_digests.append(canonical_sha256(payload))
    raw_byte_identical = raw_outputs[0] == raw_outputs[1]
    canonical_identical = canonical_digests[0] == canonical_digests[1]
    if not raw_byte_identical or not canonical_identical or payloads[0] != payloads[1]:
        raise ValueError("two engine executions were not byte-identical and canonical-identical")
    reproducibility = {
        "status": "accepted",
        "execution_count": 2,
        "raw_stdout_byte_identical": raw_byte_identical,
        "canonical_json_identical": canonical_identical,
        "raw_stdout_sha256": raw_digests,
        "canonical_full_engine_json_sha256": canonical_digests,
    }
    return payloads[0], sha256(binary), reproducibility


def max_abs_difference(left: Any, right: Any) -> float:
    left_array = np.asarray(left, dtype=float)
    right_array = np.asarray(right, dtype=float)
    if left_array.shape != right_array.shape:
        return math.inf
    return float(np.max(np.abs(left_array - right_array))) if left_array.size else 0.0


def engine_parameters(group_fit: Mapping[str, Any]) -> dict[str, float]:
    output: dict[str, float] = {}
    fixed: dict[str, bool] = {}
    for row in group_fit["parameters"]:
        parameter_id = row["parameter_id"]
        if parameter_id not in PARAMETER_ID_MAP:
            raise ValueError(f"unexpected engine parameter id {parameter_id}")
        name = PARAMETER_ID_MAP[parameter_id]
        if name in output:
            raise ValueError(f"duplicate engine parameter id {parameter_id}")
        output[name] = float(row["estimate"])
        fixed[name] = bool(row["fixed"])
    if set(output) != set(PARAMETER_ORDER) or len(output) != len(PARAMETER_ORDER):
        raise ValueError("engine parameter identity drifted")
    if not fixed["loading_x1"] or not fixed["loading_y1"]:
        raise ValueError("engine marker loadings are not fixed")
    if output["loading_x1"] != 1.0 or output["loading_y1"] != 1.0:
        raise ValueError("engine marker loading value drifted")
    return {name: output[name] for name in PARAMETER_ORDER}


def project_and_compare_engine(
    engine: Mapping[str, Any],
    python: Mapping[str, Any],
    manifest: Mapping[str, Any],
    binary_sha256: str,
) -> tuple[dict[str, Any], dict[str, Any]]:
    expected = manifest["scientific_contract"]["expected_dimensions"]
    tolerances = manifest["acceptance_tolerances"]
    if engine.get("kind") != "cbsem_exact_two_group_configural_metric_engine_reference_v2":
        raise ValueError("engine reference kind drifted")
    if engine.get("schema_version") != 2:
        raise ValueError("engine reference schema version drifted")
    if engine.get("status") != "engine_reference_generated_product_qualification_blocked":
        raise ValueError("engine reference status drifted")
    primary = engine["primary"]
    if primary.get("method_version") != "cbsem_exact_two_group_configural_metric_prerequisite_v1":
        raise ValueError("engine method version drifted")
    if primary.get("qualification_status") != "engine_only_prerequisite_not_product_qualified":
        raise ValueError("engine qualification boundary drifted")
    source_binding = engine.get("source_binding", {})
    expected_uuids = manifest["fixture_contract"]["dataset_uuids"]
    if (
        source_binding.get("primary_dataset_uuid") != expected_uuids["primary"]
        or source_binding.get("label_swap_dataset_uuid") != expected_uuids["label_swap"]
        or source_binding.get("primary_dataset_uuid")
        == source_binding.get("label_swap_dataset_uuid")
    ):
        raise ValueError("engine dataset UUID authority drifted or collapsed")
    if (
        primary["data"]["dataset_id"] != expected_uuids["primary"]
        or engine["label_swap"]["data"]["dataset_id"] != expected_uuids["label_swap"]
    ):
        raise ValueError("compiled engine result did not preserve frozen dataset UUIDs")
    dimensions = {
        "configural_free_dimensions": primary["configural"]["free_dimensions"],
        "configural_degrees_of_freedom": primary["configural"]["degrees_of_freedom"],
        "metric_free_dimensions": primary["metric"]["free_dimensions"],
        "metric_degrees_of_freedom": primary["metric"]["degrees_of_freedom"],
        "delta_degrees_of_freedom": primary["nesting"]["delta_degrees_of_freedom"],
    }
    if dimensions != expected or engine["expected_dimensions"] != expected:
        raise ValueError("engine structural dimensions differ from the frozen contract")
    engine_groups = primary["data"]["groups"]
    if [row["canonical_value"] for row in engine_groups] != list(GROUPS):
        raise ValueError("engine group authority order drifted")
    python_groups = {row["group"]: row for row in python["moments"].values()}
    moment_differences: list[float] = []
    for engine_group in engine_groups:
        group = engine_group["canonical_value"]
        python_group = python_groups[group]
        if engine_group["complete_observations"] != python_group["sample_size"]:
            raise ValueError("engine group N differs from raw fixture authority")
        moment_differences.extend(
            [
                max_abs_difference(engine_group["observed_means"], python_group["observed_means"]),
                max_abs_difference(engine_group["covariance_ml"], python_group["covariance_ml"]),
            ]
        )
    max_moment_difference = max(moment_differences)

    parameter_differences: list[float] = []
    implied_differences: list[float] = []
    objective_differences: list[float] = []
    projected_fits: dict[str, Any] = {}
    for model in ("configural", "metric"):
        engine_fit = primary[model]
        python_fit = python[model]
        python_fit_groups = {row["group"]: row for row in python_fit["groups"]}
        group_projection = []
        for engine_group in engine_fit["groups"]:
            engine_authority = next(
                row for row in engine_groups if row["group_id"] == engine_group["group_id"]
            )
            group = engine_authority["canonical_value"]
            python_group = python_fit_groups[group]
            parameters = engine_parameters(engine_group)
            parameter_differences.append(
                max_abs_difference(
                    [parameters[name] for name in PARAMETER_ORDER],
                    [python_group["parameters"][name] for name in PARAMETER_ORDER],
                )
            )
            implied_differences.append(
                max_abs_difference(
                    engine_group["implied_covariance"], python_group["implied_covariance"]
                )
            )
            objective_differences.append(
                abs(float(engine_group["objective"]) - float(python_group["objective"]))
            )
            group_projection.append(
                {
                    "group": group,
                    "sample_size": engine_group["sample_size"],
                    "objective": engine_group["objective"],
                    "chi_square": engine_group["chi_square"],
                    "parameters": parameters,
                    "implied_covariance_sha256": canonical_sha256(
                        engine_group["implied_covariance"]
                    ),
                }
            )
        projected_fits[model] = {
            "free_dimensions": engine_fit["free_dimensions"],
            "observed_moments": engine_fit["observed_moments"],
            "degrees_of_freedom": engine_fit["degrees_of_freedom"],
            "objective": engine_fit["objective"],
            "chi_square": engine_fit["chi_square"],
            "gradient_norm": engine_fit["gradient_norm"],
            "groups": group_projection,
        }

    global_differences = {
        "configural_chi_square_absolute_difference": abs(
            float(primary["configural"]["chi_square"])
            - float(python["configural"]["chi_square"])
        ),
        "metric_chi_square_absolute_difference": abs(
            float(primary["metric"]["chi_square"])
            - float(python["metric"]["chi_square"])
        ),
        "lrt_absolute_difference": abs(
            float(primary["nesting"]["likelihood_ratio_statistic"])
            - float(python["likelihood_ratio_test"]["statistic"])
        ),
        "lrt_p_value_absolute_difference": abs(
            float(primary["nesting"]["p_value"])
            - float(python["likelihood_ratio_test"]["upper_tail_p_value"])
        ),
    }
    checks = {
        "maximum_moment_absolute_difference": max_moment_difference,
        "maximum_parameter_absolute_difference": max(parameter_differences),
        "maximum_implied_covariance_absolute_difference": max(implied_differences),
        "maximum_group_objective_absolute_difference": max(objective_differences),
        **global_differences,
        "label_swap_status": engine["label_swap_check"]["status"],
        "maximum_label_swap_chi_square_absolute_difference": max(
            float(engine["label_swap_check"][key])
            for key in (
                "configural_chi_square_absolute_difference",
                "metric_chi_square_absolute_difference",
                "lrt_absolute_difference",
            )
        ),
    }
    failures = []
    threshold_map = {
        "maximum_moment_absolute_difference": "moment_absolute",
        "maximum_parameter_absolute_difference": "parameter_absolute",
        "maximum_implied_covariance_absolute_difference": "implied_covariance_absolute",
        "maximum_group_objective_absolute_difference": "objective_absolute",
        "configural_chi_square_absolute_difference": "chi_square_absolute",
        "metric_chi_square_absolute_difference": "chi_square_absolute",
        "lrt_absolute_difference": "chi_square_absolute",
        "lrt_p_value_absolute_difference": "p_value_absolute",
        "maximum_label_swap_chi_square_absolute_difference": "label_swap_absolute",
    }
    for name, tolerance_name in threshold_map.items():
        if not math.isfinite(float(checks[name])) or float(checks[name]) > float(
            tolerances[tolerance_name]
        ):
            failures.append(name)
    if checks["label_swap_status"] != "accepted":
        failures.append("label_swap_status")
    parity = {
        "status": "accepted" if not failures else "rejected",
        "failures": sorted(failures),
        "tolerances": tolerances,
        "observed": checks,
    }
    projection = {
        "binary_sha256": binary_sha256,
        "scientific_result_sha256": canonical_sha256(engine),
        "method_version": primary["method_version"],
        "qualification_status": primary["qualification_status"],
        "data_authority": {
            "dataset_id": primary["data"]["dataset_id"],
            "dataset_fingerprint": primary["data"]["dataset_fingerprint"],
            "combined_authority_sha256": primary["data"]["combined_authority_sha256"],
            "shared_complete_case_observations": primary["data"][
                "shared_complete_case_observations"
            ],
            "group_sample_sizes": {
                row["canonical_value"]: row["complete_observations"] for row in engine_groups
            },
            "group_covariance_sha256": {
                row["canonical_value"]: row["canonical_ml_covariance_sha256"]
                for row in engine_groups
            },
            "group_observed_means_sha256": {
                row["canonical_value"]: row["canonical_observed_means_sha256"]
                for row in engine_groups
            },
        },
        "dataset_uuids": expected_uuids,
        "configural": projected_fits["configural"],
        "metric": projected_fits["metric"],
        "likelihood_ratio_test": {
            "statistic": primary["nesting"]["likelihood_ratio_statistic"],
            "degrees_of_freedom": primary["nesting"]["delta_degrees_of_freedom"],
            "upper_tail_p_value": primary["nesting"]["p_value"],
        },
        "label_swap_check": engine["label_swap_check"],
    }
    return projection, parity


def run_lavaan(
    manifest: Mapping[str, Any], python: Mapping[str, Any]
) -> dict[str, Any]:
    executable = shutil.which("Rscript")
    script_binding = next(
        row
        for row in manifest["source_bindings"]
        if row["role"] == "independent_lavaan_script"
    )
    if executable is None:
        return {
            "status": "unavailable_current_host",
            "reason": "Rscript_not_found",
            "script_sha256": script_binding["sha256"],
            "required_contract": manifest["scientific_contract"]["lavaan_contract"],
        }
    with tempfile.TemporaryDirectory(prefix="qpls-two-group-lavaan-") as raw:
        output_path = Path(raw) / "lavaan-reference.json"
        completed = subprocess.run(
            [
                executable,
                str(ROOT / script_binding["path"]),
                str(ROOT / manifest["fixture_contract"]["primary_path"]),
                str(output_path),
            ],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
            timeout=120,
        )
        payload = load_json(output_path)
        output_sha = sha256(output_path)
    expected = manifest["scientific_contract"]["expected_dimensions"]
    observed_dimensions = {
        "configural_free_dimensions": payload["configural"]["free_dimensions"],
        "configural_degrees_of_freedom": payload["configural"]["degrees_of_freedom"],
        "metric_free_dimensions": payload["metric"]["free_dimensions"],
        "metric_degrees_of_freedom": payload["metric"]["degrees_of_freedom"],
        "delta_degrees_of_freedom": payload["likelihood_ratio_test"]["degrees_of_freedom"],
    }
    tolerance = float(manifest["acceptance_tolerances"]["lavaan_chi_square_absolute"])
    differences = {
        "configural_chi_square_absolute_difference": abs(
            float(payload["configural"]["chi_square"])
            - float(python["configural"]["chi_square"])
        ),
        "metric_chi_square_absolute_difference": abs(
            float(payload["metric"]["chi_square"])
            - float(python["metric"]["chi_square"])
        ),
        "lrt_absolute_difference": abs(
            float(payload["likelihood_ratio_test"]["statistic"])
            - float(python["likelihood_ratio_test"]["statistic"])
        ),
    }
    accepted = observed_dimensions == expected and max(differences.values()) <= tolerance
    return {
        "status": "accepted" if accepted else "rejected",
        "script_sha256": script_binding["sha256"],
        "output_sha256": output_sha,
        "runtime": payload["runtime"],
        "contract": payload["contract"],
        "observed_options": payload["observed_options"],
        "group_sample_sizes": payload["group_sample_sizes"],
        "dimensions": observed_dimensions,
        "configural_chi_square": payload["configural"]["chi_square"],
        "metric_chi_square": payload["metric"]["chi_square"],
        "likelihood_ratio_test": payload["likelihood_ratio_test"],
        "parity": {"absolute_tolerance": tolerance, **differences},
    }


def fixture_projection(groups: Mapping[str, np.ndarray], manifest: Mapping[str, Any]) -> dict[str, Any]:
    fixture = manifest["fixture_contract"]
    return {
        "primary_path": fixture["primary_path"],
        "primary_sha256": fixture["primary_sha256"],
        "label_swap_path": fixture["label_swap_path"],
        "label_swap_sha256": fixture["label_swap_sha256"],
        "observed_variable_order": list(VARIABLES),
        "group_sample_sizes": {group: len(groups[group]) for group in GROUPS},
        "label_swap_validation": "exact_numeric_rows_preserved_group_text_values_exchanged",
        "dgp": fixture["dgp"],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--engine-binary", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--execute", action="store_true")
    args = parser.parse_args()
    manifest, manifest_sha = validate_manifest(args.manifest.resolve())
    if not args.execute:
        print(
            json.dumps(
                {
                    "status": "dry_run_no_fits_executed",
                    "manifest_sha256": manifest_sha,
                    "expected_dimensions": manifest["scientific_contract"][
                        "expected_dimensions"
                    ],
                    "qualification_status": "blocked_not_product_qualification",
                },
                indent=2,
            )
        )
        return 0
    if args.engine_binary is None:
        parser.error("--engine-binary is required with --execute")
    output = args.output.resolve()
    if not output.is_relative_to((ROOT / "validation/results").resolve()):
        parser.error("--output must remain under validation/results")
    primary = read_fixture(ROOT / manifest["fixture_contract"]["primary_path"])
    swapped = read_fixture(
        ROOT / manifest["fixture_contract"]["label_swap_path"],
        {"A": 220, "B": 180},
    )
    validate_label_swap(primary, swapped)
    python_reference = fit_python_reference(primary)
    engine_raw, binary_sha, reproducibility = run_engine(
        args.engine_binary.resolve(), manifest
    )
    engine_reference, parity = project_and_compare_engine(
        engine_raw, python_reference, manifest, binary_sha
    )
    if engine_reference["scientific_result_sha256"] != reproducibility[
        "canonical_full_engine_json_sha256"
    ][0]:
        raise ValueError("projected full-engine digest differs from repeat-run authority")
    engine_reference["repeat_run_reproducibility"] = reproducibility
    lavaan_reference = run_lavaan(manifest, python_reference)
    rejected = parity["status"] != "accepted" or lavaan_reference["status"] == "rejected"
    report = {
        "schema_version": 2,
        "kind": "cbsem_exact_two_group_configural_metric_engine_reference_report_v2",
        "status": (
            "engine_reference_evidence_v2_rejected_product_qualification_blocked"
            if rejected
            else "engine_reference_evidence_v2_passed_reproducible_product_qualification_blocked"
        ),
        "manifest_path": DEFAULT_MANIFEST.relative_to(ROOT).as_posix(),
        "manifest_sha256": manifest_sha,
        "source_bindings": manifest["source_bindings"],
        "supersedes": manifest["supersedes"],
        "fixture": fixture_projection(primary, manifest),
        "python_reference": python_reference,
        "engine_reference": engine_reference,
        "engine_python_parity": parity,
        "lavaan_reference": lavaan_reference,
        "qualification_boundary": manifest["qualification_boundary"],
    }
    report_schema = load_json(REPORT_SCHEMA)
    Draft202012Validator.check_schema(report_schema)
    Draft202012Validator(report_schema).validate(report)
    digest = write_new_json(output, report)
    print(
        json.dumps(
            {
                "status": report["status"],
                "output": str(output),
                "report_sha256": digest,
                "lavaan_status": lavaan_reference["status"],
            },
            indent=2,
        )
    )
    return 2 if rejected else 0


if __name__ == "__main__":
    raise SystemExit(main())
