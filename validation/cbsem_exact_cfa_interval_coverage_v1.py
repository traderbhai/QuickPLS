#!/usr/bin/env python3
"""Plan, execute, and aggregate validation-only exact-CFA coverage shards.

Only the B=500 conformance stage and the 20-dataset-per-DGP pilot are
executable. Final and paired B=10,000 confirmation cells are immutable plans;
this harness cannot promote, qualify, or expose a product capability.
"""

from __future__ import annotations

import argparse
import ctypes
from fractions import Fraction
from functools import lru_cache
import hashlib
import json
import math
import os
import platform
import re
import shutil
import struct
import subprocess
import tempfile
import time
import uuid
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence

import numpy as np
import scipy
from jsonschema import Draft202012Validator
from scipy.stats import beta as scipy_beta

try:
    from validation.cbsem_exact_cfa_interval_coverage_v1_oracle import (
        audit_engine_case,
        BCA_ACCELERATION_METHOD,
        BCA_ADJUSTED_PROBABILITY_METHOD,
        PERCENTILE_BIAS_BINDING_METHOD,
        PERCENTILE_MEAN_VALIDATION_METHOD,
        bonferroni_clopper_pearson_lower as oracle_bonferroni_lower,
        bonferroni_clopper_pearson_two_sided as oracle_bonferroni_two_sided,
        clopper_pearson_two_sided as oracle_clopper_pearson_two_sided,
        dataset_cluster_failure_summary as oracle_dataset_cluster_failure_summary,
    )
except ModuleNotFoundError:
    from cbsem_exact_cfa_interval_coverage_v1_oracle import (
        audit_engine_case,
        BCA_ACCELERATION_METHOD,
        BCA_ADJUSTED_PROBABILITY_METHOD,
        PERCENTILE_BIAS_BINDING_METHOD,
        PERCENTILE_MEAN_VALIDATION_METHOD,
        bonferroni_clopper_pearson_lower as oracle_bonferroni_lower,
        bonferroni_clopper_pearson_two_sided as oracle_bonferroni_two_sided,
        clopper_pearson_two_sided as oracle_clopper_pearson_two_sided,
        dataset_cluster_failure_summary as oracle_dataset_cluster_failure_summary,
    )


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST = ROOT / "validation/cbsem_exact_cfa_interval_coverage_v1_manifest.json"
MANIFEST_SCHEMA = ROOT / "validation/cbsem_exact_cfa_interval_coverage_v1_manifest.schema.json"
CASE_SCHEMA = ROOT / "validation/cbsem_exact_cfa_interval_coverage_v1_case.schema.json"
REPORT_SCHEMA = ROOT / "validation/cbsem_exact_cfa_interval_coverage_v1_report.schema.json"
DEFAULT_OUTPUT_ROOT = ROOT / "validation/results/cbsem_exact_cfa_interval_coverage_v1"
CASE_KIND = "cbsem_exact_cfa_interval_coverage_shard_case_v1"
REPORT_KIND = "cbsem_exact_cfa_interval_coverage_report_v1"
SEED_DOMAIN = b"quickpls_cbsem_exact_cfa_interval_coverage_v1\0"
SHA256_PATTERN = re.compile(r"^[0-9a-f]{64}$")
SAFE_ID_PATTERN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")
POINT_FAILURE_KINDS = {
    "moment_matrix_not_positive_definite",
    "non_convergence",
    "inadmissible_solution",
    "numerical_failure",
}
SE_UNAVAILABLE_REASONS = {
    "singular_information",
    "information_not_positive_definite",
    "invalid_information_variance_or_standard_error",
    "derivative_unavailable",
    "numerical_information_failure",
}
BCA_UNAVAILABLE_REASONS = {
    "base_inference_unavailable",
    "incomplete_delete_one_ledger",
    "bias_correction_probability_at_boundary",
    "degenerate_jackknife_acceleration",
    "nonfinite_jackknife_arithmetic",
    "singular_acceleration_adjustment",
    "invalid_adjusted_probability",
    "adjusted_probability_order_invalid",
    "nonfinite_or_reversed_interval",
}
METHODS = ("percentile", "studentized", "bca")
EXACT_BOOTSTRAP_METHOD = "cbsem_exact_case_bootstrap_v1"
EXACT_ESTIMATOR_METHOD = "cbsem_ml_exact_parameter_table_v3"
INDEX_DIGEST_METHOD = "sha256_source_fingerprint_and_ordered_u64_indices_v1"
IDENTITY_POSITIONS_DIGEST_METHOD = "sha256_complete_case_n_and_ordered_sampling_positions_v1"
SCHEDULE_POSITIONS_DIGEST_METHOD = "sha256_stream_seed_replicate_complete_case_n_and_ordered_sampling_positions_v1"
UNIVERSE_DIGEST_METHOD = "sha256_source_fingerprint_and_ordered_complete_case_u64_indices_v1"
STUDENTIZED_METHOD = "cbsem_exact_case_bootstrap_analytic_studentized_interval_v1"
SE_METHOD = "cbsem_exact_case_bootstrap_refit_standard_errors_v1"
INFORMATION_METHOD = "cbsem_ml_expected_information_delta_method_v1"
TYPED_JSON_DIGEST_METHOD = "sha256_typed_json_tree_v1"
DIGEST_VERIFICATION_KIND = "cbsem_exact_cfa_interval_coverage_digest_verification_receipt_v1"
DIGEST_VERIFICATION_METHOD = "bound_rust_exact_schedule_and_source_verifier_v1"
PROCESS_POLL_SECONDS = 0.02


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def scientific_runtime_versions() -> dict[str, str]:
    """Return the exact local runtime versions used by the scientific oracle."""

    return {
        "python": platform.python_version(),
        "numpy": np.__version__,
        "scipy": scipy.__version__,
    }


def _require_scientific_runtime(manifest: Mapping[str, Any]) -> dict[str, str]:
    observed = scientific_runtime_versions()
    expected = manifest.get("scientific_runtime")
    if observed != expected:
        raise ValueError(
            f"scientific runtime differs from frozen manifest: expected {expected}, observed {observed}"
        )
    return observed


def canonical_sha256(value: Any) -> str:
    payload = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def typed_json_sha256(value: Any) -> str:
    """Cross-language SHA-256 over a typed JSON tree.

    JSON text formatting is intentionally excluded. Object keys are UTF-8
    sorted, integers are signed decimal tokens, and floating-point values use
    their exact IEEE-754 binary64 bytes. The Rust example implements the same
    frozen method before emitting any scientific or base-point digest.
    """

    digest = hashlib.sha256()

    def update(current: Any) -> None:
        if current is None:
            digest.update(b"N")
        elif current is True:
            digest.update(b"T")
        elif current is False:
            digest.update(b"F")
        elif isinstance(current, int) and not isinstance(current, bool):
            token = str(current).encode("ascii")
            digest.update(b"I" + len(token).to_bytes(8, "little") + token)
        elif isinstance(current, float):
            if not math.isfinite(current):
                raise ValueError("typed JSON digest rejects nonfinite numbers")
            digest.update(b"D" + struct.pack("<d", current))
        elif isinstance(current, str):
            token = current.encode("utf-8")
            digest.update(b"S" + len(token).to_bytes(8, "little") + token)
        elif isinstance(current, list):
            digest.update(b"A" + len(current).to_bytes(8, "little"))
            for item in current:
                update(item)
        elif isinstance(current, Mapping):
            if any(not isinstance(key, str) for key in current):
                raise ValueError("typed JSON digest requires string object keys")
            keys = sorted(current, key=lambda key: key.encode("utf-8"))
            digest.update(b"O" + len(keys).to_bytes(8, "little"))
            for key in keys:
                token = key.encode("utf-8")
                digest.update(len(token).to_bytes(8, "little") + token)
                update(current[key])
        else:
            raise ValueError(f"typed JSON digest rejects {type(current).__name__}")

    update(value)
    return digest.hexdigest()


def compact_evidence_payload(
    scientific_result_sha256: str,
    data_sha256: str,
    source_dataset_fingerprint: str,
    ledger: Mapping[str, Any],
    methods: Sequence[Mapping[str, Any]],
) -> dict[str, Any]:
    """Return the exact raw-derived compact evidence authenticated by Rust.

    Coverage flags are deliberately excluded: aggregation recomputes them from
    the authenticated endpoints and frozen DGP truth instead of trusting a
    serialized covered/not-covered claim.
    """

    compact_methods = [
        {
            "method": row["method"],
            "available": row["available"],
            "parameters": [
                {
                    "parameter_id": parameter["parameter_id"],
                    "available": parameter["available"],
                    "interval_lower": parameter["interval_lower"],
                    "interval_upper": parameter["interval_upper"],
                }
                for parameter in row["parameters"]
            ],
        }
        for row in methods
    ]
    return {
        "scientific_result_sha256": scientific_result_sha256,
        "data_sha256": data_sha256,
        "source_dataset_fingerprint": source_dataset_fingerprint,
        "ledger": dict(ledger),
        "methods": compact_methods,
    }


def complete_case_universe_sha256(
    source_fingerprint: str, source_row_count: int, source_rows: Sequence[int]
) -> str:
    return _ordered_u64_digest(
        UNIVERSE_DIGEST_METHOD,
        source_fingerprint,
        source_row_count,
        source_rows,
    )


def sampling_positions_sha256(complete_case_count: int, positions: Sequence[int]) -> str:
    digest = hashlib.sha256()
    digest.update(IDENTITY_POSITIONS_DIGEST_METHOD.encode("ascii"))
    digest.update(b"\0")
    digest.update(complete_case_count.to_bytes(8, "little"))
    digest.update(len(positions).to_bytes(8, "little"))
    for position in positions:
        digest.update(position.to_bytes(8, "little"))
    return digest.hexdigest()


def source_rows_sha256(
    source_fingerprint: str, source_row_count: int, source_rows: Sequence[int]
) -> str:
    return _ordered_u64_digest(
        INDEX_DIGEST_METHOD,
        source_fingerprint,
        source_row_count,
        source_rows,
    )


def _ordered_u64_digest(
    method: str, source_fingerprint: str, source_row_count: int, values: Sequence[int]
) -> str:
    if not source_fingerprint or source_row_count < 0:
        raise ValueError("ordered-u64 digest preimage is invalid")
    digest = hashlib.sha256()
    token = source_fingerprint.encode("utf-8")
    digest.update(method.encode("ascii"))
    digest.update(b"\0")
    digest.update(len(token).to_bytes(8, "little"))
    digest.update(token)
    digest.update(source_row_count.to_bytes(8, "little"))
    digest.update(len(values).to_bytes(8, "little"))
    for value in values:
        if type(value) is not int or value < 0:
            raise ValueError("ordered-u64 digest values must be nonnegative integers")
        digest.update(value.to_bytes(8, "little"))
    return digest.hexdigest()


def write_new_json(path: Path, value: Any) -> str:
    if path.exists():
        raise FileExistsError(f"append-only output already exists: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n"
    path.write_text(payload, encoding="utf-8", newline="\n")
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def _bound_file(binding: Mapping[str, Any], path_key: str, digest_key: str) -> Path:
    path = (ROOT / str(binding[path_key])).resolve()
    if not path.is_file() or sha256(path) != binding[digest_key]:
        raise ValueError(f"immutable file binding failed for {path}")
    return path


def validate_manifest(
    path: Path,
    *,
    require_binary: bool = False,
    require_scientific_runtime: bool = False,
) -> tuple[dict[str, Any], str]:
    if path.resolve() != DEFAULT_MANIFEST.resolve():
        raise ValueError("only the checked-in predeclared manifest is accepted")
    manifest = load_json(path)
    schema = load_json(MANIFEST_SCHEMA)
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(manifest)
    for path_key, digest_key in (
        ("runner_path", "runner_sha256"),
        ("oracle_path", "oracle_sha256"),
        ("rust_example_path", "rust_example_sha256"),
        ("rust_kernel_path", "rust_kernel_sha256"),
        ("estimation_contract_path", "estimation_contract_sha256"),
        ("resampling_cargo_path", "resampling_cargo_sha256"),
        ("cargo_lock_path", "cargo_lock_sha256"),
        ("focused_test_path", "focused_test_sha256"),
    ):
        _bound_file(manifest["source_binding"], path_key, digest_key)
    for path_key, digest_key in (
        ("manifest_schema_path", "manifest_schema_sha256"),
        ("case_schema_path", "case_schema_sha256"),
        ("report_schema_path", "report_schema_sha256"),
    ):
        _bound_file(manifest["schema_binding"], path_key, digest_key)
    binary = (ROOT / manifest["binary_binding"]["path"]).resolve()
    frozen_binary_sha = manifest["binary_binding"]["sha256"]
    if require_binary:
        if frozen_binary_sha is None:
            raise ValueError("execution is blocked until the binary SHA-256 is frozen")
        if not binary.is_file() or sha256(binary) != frozen_binary_sha:
            raise ValueError("coverage binary binding failed")
    if require_scientific_runtime:
        _require_scientific_runtime(manifest)

    expected_dgps = {
        "dgp_a_one_factor_n150": (150, 3, 6),
        "dgp_b_two_factor_n300": (300, 6, 13),
    }
    parameter_count = 0
    for dgp in manifest["dgps"]:
        if expected_dgps.get(dgp["dgp_id"]) != (
            dgp["sample_size"],
            len(dgp["observed_columns"]),
            len(dgp["parameter_truth"]),
        ):
            raise ValueError(f"frozen DGP dimensions drifted: {dgp['dgp_id']}")
        fixture = {key: value for key, value in dgp.items() if key != "fixture_sha256"}
        if canonical_sha256(fixture) != dgp["fixture_sha256"]:
            raise ValueError(f"DGP fixture hash drifted: {dgp['dgp_id']}")
        ids = {row["parameter_id"] for row in dgp["parameter_truth"]}
        if len(ids) != len(dgp["parameter_truth"]):
            raise ValueError("DGP parameter identities must be unique within each DGP")
        parameter_count += len(ids)
        covariance = population_covariance(dgp)
        if not np.isfinite(covariance).all() or np.linalg.eigvalsh(covariance).min() <= 0.0:
            raise ValueError(f"DGP covariance is not positive definite: {dgp['dgp_id']}")

    coverage = manifest["coverage_contract"]
    if parameter_count * len(METHODS) != coverage["primary_cell_count"] or coverage["primary_cell_count"] != 57:
        raise ValueError("the primary coverage family is not the frozen 57 cells")
    expected_binomial_contract = {
        "binomial_endpoint_kernel": "scipy_beta_seed_exact_integer_binomial_tail_certified_tight_binary64_v2",
        "ordinary_interval": "clopper_pearson_two_sided_exact_certified_outward_binary64_v2",
        "simultaneous_interval": "bonferroni_57_cell_clopper_pearson_two_sided_exact_certified_outward_binary64_v2",
        "availability_interval": "bonferroni_6_cell_clopper_pearson_one_sided_lower_exact_certified_outward_binary64_v2",
        "ordinary_two_sided_tail": {"numerator": 1, "denominator": 40},
        "simultaneous_two_sided_tail": {"numerator": 1, "denominator": 2280},
        "availability_one_sided_tail": {"numerator": 1, "denominator": 120},
    }
    if any(coverage.get(key) != value for key, value in expected_binomial_contract.items()):
        raise ValueError("frozen exact-binomial arithmetic contract drifted")
    plans = {row["stage"]: row for row in manifest["stages"]}
    expected_stages = {
        "conformance": (1, 500, 12, 1, True),
        "pilot": (20, 5000, 12, 5, True),
        "final": (3200, 5000, 12, 100, False),
        "paired_confirmation": (20, 10000, 12, 20, False),
    }
    if set(plans) != set(expected_stages):
        raise ValueError("stage family drifted")
    for name, expected in expected_stages.items():
        row = plans[name]
        observed = (
            row["datasets_per_dgp"],
            row["replicates"],
            row["workers"],
            row["datasets_per_shard"],
            row["executable"],
        )
        if observed != expected or row["datasets_per_dgp"] % row["datasets_per_shard"]:
            raise ValueError(f"stage plan drifted: {name}")
    expected_seed_stages = {
        "conformance": "conformance",
        "pilot": "pilot",
        "final": "final",
        "paired_confirmation": "final",
    }
    if manifest["seed_contract"].get("derivation_stage_by_planned_stage") != expected_seed_stages:
        raise ValueError("paired confirmation must reuse every first-20 final dataset seed")
    if manifest["qualification_boundary"]["status"] != "blocked_not_product_qualification":
        raise ValueError("validation harness cannot declare product qualification")
    arithmetic = manifest["arithmetic_kernel_contract"]
    if (
        arithmetic["percentile_mean_validation_method"]
        != PERCENTILE_MEAN_VALIDATION_METHOD
        or arithmetic["percentile_bias_binding_method"]
        != PERCENTILE_BIAS_BINDING_METHOD
        or arithmetic["bca_acceleration_method"] != BCA_ACCELERATION_METHOD
        or arithmetic["bca_adjusted_probability_method"]
        != BCA_ADJUSTED_PROBABILITY_METHOD
    ):
        raise ValueError("frozen arithmetic kernel identities drifted")
    return manifest, sha256(path)


def dgp_by_id(manifest: Mapping[str, Any], dgp_id: str) -> Mapping[str, Any]:
    matches = [row for row in manifest["dgps"] if row["dgp_id"] == dgp_id]
    if len(matches) != 1:
        raise ValueError(f"unknown DGP {dgp_id}")
    return matches[0]


def stage_by_name(manifest: Mapping[str, Any], stage: str) -> Mapping[str, Any]:
    matches = [row for row in manifest["stages"] if row["stage"] == stage]
    if len(matches) != 1:
        raise ValueError(f"unknown stage {stage}")
    return matches[0]


def derive_seed(master_seed: int, dgp_id: str, stage: str, dataset_index: int, purpose: str) -> int:
    if type(master_seed) is not int or master_seed < 0 or type(dataset_index) is not int or dataset_index < 0:
        raise ValueError("seed inputs must be nonnegative integers")
    if purpose not in {"data", "bootstrap", "oracle"}:
        raise ValueError("seed purpose is outside the frozen domains")
    fields = (str(master_seed), dgp_id, stage, str(dataset_index), purpose)
    if any("\0" in field or not field for field in fields):
        raise ValueError("seed domain fields must be nonempty and NUL-free")
    digest = hashlib.sha256(SEED_DOMAIN + "\0".join(fields).encode("utf-8")).digest()
    return int.from_bytes(digest[:8], "little", signed=False)


def seed_derivation_stage(manifest: Mapping[str, Any], planned_stage: str) -> str:
    try:
        stage = manifest["seed_contract"]["derivation_stage_by_planned_stage"][planned_stage]
    except (KeyError, TypeError) as error:
        raise ValueError(f"seed stage is not frozen for {planned_stage}") from error
    if stage not in {"conformance", "pilot", "final"}:
        raise ValueError(f"invalid frozen seed stage {stage}")
    return stage


def population_covariance(dgp: Mapping[str, Any]) -> np.ndarray:
    loading = np.asarray(dgp["loading_matrix"], dtype=np.float64)
    factor = np.asarray(dgp["factor_covariance"], dtype=np.float64)
    residual = np.asarray(dgp["residual_variances"], dtype=np.float64)
    covariance = loading @ factor @ loading.T + np.diag(residual)
    if covariance.shape != (len(dgp["observed_columns"]),) * 2:
        raise ValueError("DGP covariance dimensions differ")
    return covariance


def generate_dataset_csv(dgp: Mapping[str, Any], seed: int) -> bytes:
    covariance = population_covariance(dgp)
    cholesky = np.linalg.cholesky(covariance)
    rng = np.random.Generator(np.random.PCG64DXSM(seed))
    normal = rng.standard_normal((dgp["sample_size"], len(dgp["observed_columns"])))
    observations = normal @ cholesky.T
    lines = [",".join(dgp["observed_columns"])]
    lines.extend(",".join(format(float(value), ".17g") for value in row) for row in observations)
    return ("\n".join(lines) + "\n").encode("utf-8")


def plan_shards(manifest: Mapping[str, Any], stage: str) -> list[dict[str, Any]]:
    plan = stage_by_name(manifest, stage)
    shards: list[dict[str, Any]] = []
    for dgp in manifest["dgps"]:
        size = plan["datasets_per_shard"]
        for ordinal, start in enumerate(range(0, plan["datasets_per_dgp"], size)):
            stop = min(start + size, plan["datasets_per_dgp"])
            shards.append(
                {
                    "shard_id": f"{stage}-{dgp['dgp_id']}-s{ordinal:02d}",
                    "stage": stage,
                    "dgp_id": dgp["dgp_id"],
                    "shard_ordinal": ordinal,
                    "dataset_indices": list(range(start, stop)),
                    "replicates": plan["replicates"],
                    "workers": plan["workers"],
                    "executable": plan["executable"],
                }
            )
    return shards


def _binomial_log_cdf(k: int, trials: int, probability: float) -> float:
    """Approximate a binomial CDF for endpoint seeding only.

    This log-space value is never accepted as a proof that an endpoint is
    conservative. Final endpoints are certified against the exact rational
    value of the binary64 probability below.
    """

    if k < 0:
        return -math.inf
    if k >= trials:
        return 0.0
    if probability <= 0.0:
        return 0.0
    if probability >= 1.0:
        return -math.inf
    logs = [
        math.lgamma(trials + 1)
        - math.lgamma(index + 1)
        - math.lgamma(trials - index + 1)
        + index * math.log(probability)
        + (trials - index) * math.log1p(-probability)
        for index in range(k + 1)
    ]
    maximum = max(logs)
    return maximum + math.log(math.fsum(math.exp(value - maximum) for value in logs))


def _solve_cdf_bracket(k: int, trials: int, target: float) -> tuple[float, float]:
    """Return an approximate bracket around a decreasing binomial-CDF root.

    The bracket is a fallback seed only. ``lgamma`` and floating log-sums do
    not prove an outward endpoint, so no reported confidence bound is taken
    from this function without exact-integer certification.
    """
    low, high = 0.0, 1.0
    log_target = math.log(target)
    for _ in range(160):
        midpoint = (low + high) / 2.0
        if _binomial_log_cdf(k, trials, midpoint) > log_target:
            low = midpoint
        else:
            high = midpoint
    return low, high


def _validate_tail_rational(numerator: int, denominator: int) -> None:
    if (
        type(numerator) is not int
        or type(denominator) is not int
        or numerator <= 0
        or denominator <= numerator
        or math.gcd(numerator, denominator) != 1
    ):
        raise ValueError("binomial tail must be a reduced rational strictly between zero and one")


def _tail_from_confidence(confidence: float, divisor: int) -> tuple[int, int]:
    if (
        isinstance(confidence, bool)
        or not isinstance(confidence, (int, float))
        or not math.isfinite(float(confidence))
        or not 0.0 < float(confidence) < 1.0
        or type(divisor) is not int
        or divisor <= 0
    ):
        raise ValueError("invalid exact-binomial confidence request")
    # Interpret the caller's serialized decimal confidence, not the result of
    # cancellation-prone binary64 subtraction such as ``1.0 - 0.95``.
    tail = (Fraction(1, 1) - Fraction(str(confidence))) / divisor
    _validate_tail_rational(tail.numerator, tail.denominator)
    return tail.numerator, tail.denominator


def _tail_from_manifest(value: Mapping[str, Any]) -> tuple[int, int]:
    if not isinstance(value, Mapping) or set(value) != {"numerator", "denominator"}:
        raise ValueError("manifest binomial tail rational is malformed")
    numerator = value["numerator"]
    denominator = value["denominator"]
    _validate_tail_rational(numerator, denominator)
    return numerator, denominator


def _exact_binomial_cdf_numerator(
    k: int, trials: int, probability: float
) -> tuple[int, int]:
    """Return the exact numerator/denominator of P(X <= k).

    ``float.as_integer_ratio`` makes the binary64 probability an exact
    rational. All subsequent operations use Python arbitrary-precision
    integers, so the comparison has no floating-point uncertainty.
    """

    probability_numerator, probability_denominator = probability.as_integer_ratio()
    complement_numerator = probability_denominator - probability_numerator
    denominator = pow(probability_denominator, trials)
    if k < 0:
        return 0, denominator
    if k >= trials:
        return denominator, denominator
    term = pow(complement_numerator, trials)
    total = term
    for index in range(k):
        dividend = term * (trials - index) * probability_numerator
        divisor = (index + 1) * complement_numerator
        term, remainder = divmod(dividend, divisor)
        if remainder:
            raise ArithmeticError("exact binomial CDF recurrence did not divide")
        total += term
    return total, denominator


def _exact_binomial_survival_numerator(
    k: int, trials: int, probability: float
) -> tuple[int, int]:
    """Return the exact numerator/denominator of P(X >= k)."""

    probability_numerator, probability_denominator = probability.as_integer_ratio()
    complement_numerator = probability_denominator - probability_numerator
    denominator = pow(probability_denominator, trials)
    if k <= 0:
        return denominator, denominator
    if k > trials:
        return 0, denominator
    term = pow(probability_numerator, trials)
    total = term
    for index in range(trials, k, -1):
        dividend = term * index * complement_numerator
        divisor = (trials - index + 1) * probability_numerator
        term, remainder = divmod(dividend, divisor)
        if remainder:
            raise ArithmeticError("exact binomial survival recurrence did not divide")
        total += term
    return total, denominator


def _binomial_endpoint_is_conservative(
    successes: int,
    trials: int,
    probability: float,
    tail_numerator: int,
    tail_denominator: int,
    endpoint: str,
) -> bool:
    """Prove the required one-sided tail inequality using exact integers."""

    _validate_binomial(successes, trials, 0.95)
    _validate_tail_rational(tail_numerator, tail_denominator)
    if endpoint not in {"lower", "upper"}:
        raise ValueError("binomial endpoint side is invalid")
    if not math.isfinite(probability) or not 0.0 <= probability <= 1.0:
        raise ValueError("binomial endpoint probability is invalid")
    if endpoint == "lower" and successes == 0:
        return probability == 0.0
    if endpoint == "upper" and successes == trials:
        return probability == 1.0
    if probability == 0.0:
        return endpoint == "lower"
    if probability == 1.0:
        return endpoint == "upper"

    if endpoint == "lower":
        # Prove P_p(X >= successes) <= tail. Choose the shorter exact sum;
        # the complementary comparison is algebraically identical.
        if trials - successes + 1 <= successes:
            numerator, denominator = _exact_binomial_survival_numerator(
                successes, trials, probability
            )
            return numerator * tail_denominator <= tail_numerator * denominator
        numerator, denominator = _exact_binomial_cdf_numerator(
            successes - 1, trials, probability
        )
        return (
            numerator * tail_denominator
            >= (tail_denominator - tail_numerator) * denominator
        )

    # Prove P_p(X <= successes) <= tail. Near high-coverage decision
    # boundaries the survival complement is much shorter than the CDF.
    if successes + 1 <= trials - successes:
        numerator, denominator = _exact_binomial_cdf_numerator(
            successes, trials, probability
        )
        return numerator * tail_denominator <= tail_numerator * denominator
    numerator, denominator = _exact_binomial_survival_numerator(
        successes + 1, trials, probability
    )
    return (
        numerator * tail_denominator
        >= (tail_denominator - tail_numerator) * denominator
    )


CERTIFIED_ENDPOINT_MAX_ULP_STEPS = 4096


def _tight_certified_endpoint(
    successes: int,
    trials: int,
    tail_numerator: int,
    tail_denominator: int,
    endpoint: str,
) -> float:
    tail = tail_numerator / tail_denominator
    if endpoint == "lower":
        seed = float(scipy_beta.ppf(tail, successes, trials - successes + 1))
        if not 0.0 < seed < 1.0:
            target = 1.0 - tail
            low, high = _solve_cdf_bracket(successes - 1, trials, target)
            seed = (low + high) / 2.0
        outward_target, inward_target = 0.0, 1.0
    elif endpoint == "upper":
        seed = float(scipy_beta.isf(tail, successes + 1, trials - successes))
        if not 0.0 < seed < 1.0:
            low, high = _solve_cdf_bracket(successes, trials, tail)
            seed = (low + high) / 2.0
        outward_target, inward_target = 1.0, 0.0
    else:
        raise ValueError("binomial endpoint side is invalid")

    endpoint_value = seed
    for _ in range(CERTIFIED_ENDPOINT_MAX_ULP_STEPS):
        if _binomial_endpoint_is_conservative(
            successes,
            trials,
            endpoint_value,
            tail_numerator,
            tail_denominator,
            endpoint,
        ):
            break
        adjacent = math.nextafter(endpoint_value, outward_target)
        if adjacent == endpoint_value:
            raise ArithmeticError("binomial endpoint could not move outward")
        endpoint_value = adjacent
    else:
        raise ArithmeticError("binomial endpoint seed exceeded outward certification limit")

    for _ in range(CERTIFIED_ENDPOINT_MAX_ULP_STEPS):
        adjacent = math.nextafter(endpoint_value, inward_target)
        if adjacent == endpoint_value or not _binomial_endpoint_is_conservative(
            successes,
            trials,
            adjacent,
            tail_numerator,
            tail_denominator,
            endpoint,
        ):
            return endpoint_value
        endpoint_value = adjacent
    raise ArithmeticError("binomial endpoint seed exceeded inward tightening limit")


@lru_cache(maxsize=None)
def _certified_clopper_pearson_bounds(
    successes: int, trials: int, tail_numerator: int, tail_denominator: int
) -> tuple[float, float]:
    _validate_binomial(successes, trials, 0.95)
    _validate_tail_rational(tail_numerator, tail_denominator)
    lower = (
        0.0
        if successes == 0
        else _tight_certified_endpoint(
            successes, trials, tail_numerator, tail_denominator, "lower"
        )
    )
    upper = (
        1.0
        if successes == trials
        else _tight_certified_endpoint(
            successes, trials, tail_numerator, tail_denominator, "upper"
        )
    )
    return lower, upper


def clopper_pearson_two_sided(
    successes: int, trials: int, confidence: float = 0.95
) -> tuple[float, float]:
    _validate_binomial(successes, trials, confidence)
    tail_numerator, tail_denominator = _tail_from_confidence(confidence, 2)
    return _certified_clopper_pearson_bounds(
        successes, trials, tail_numerator, tail_denominator
    )


def bonferroni_clopper_pearson_two_sided(
    successes: int, trials: int, cells: int, family_confidence: float = 0.95
) -> tuple[float, float]:
    _validate_binomial(successes, trials, family_confidence)
    if type(cells) is not int or cells <= 0:
        raise ValueError("cells must be positive")
    tail_numerator, tail_denominator = _tail_from_confidence(
        family_confidence, 2 * cells
    )
    return _certified_clopper_pearson_bounds(
        successes, trials, tail_numerator, tail_denominator
    )


def bonferroni_clopper_pearson_lower(
    successes: int, trials: int, cells: int, family_confidence: float = 0.95
) -> float:
    _validate_binomial(successes, trials, family_confidence)
    if type(cells) is not int or cells <= 0:
        raise ValueError("cells must be positive")
    if successes == 0:
        return 0.0
    tail_numerator, tail_denominator = _tail_from_confidence(
        family_confidence, cells
    )
    return _certified_clopper_pearson_bounds(
        successes, trials, tail_numerator, tail_denominator
    )[0]


def dataset_cluster_failure_summary(
    values: Sequence[float],
    seed: int,
    resamples: int,
    confidence: float,
) -> dict[str, float]:
    """Summarize one failure fraction per independent generated dataset.

    Resampling is over datasets, never over the B inner bootstrap draws. The
    fixed PCG64DXSM stream is consumed in bounded batches without changing the
    row-major draw sequence used by the independent SciPy oracle.
    """

    array = np.asarray(values, dtype=np.float64)
    if (
        array.ndim != 1
        or array.size < 2
        or not np.isfinite(array).all()
        or np.any((array < 0.0) | (array > 1.0))
        or type(seed) is not int
        or seed < 0
        or type(resamples) is not int
        or resamples <= 0
        or not 0.0 < confidence < 1.0
    ):
        raise ValueError("invalid dataset-cluster failure summary request")
    rng = np.random.Generator(np.random.PCG64DXSM(seed))
    means: list[np.ndarray] = []
    remaining = resamples
    while remaining:
        batch = min(256, remaining)
        indices = rng.integers(0, array.size, size=(batch, array.size))
        means.append(array[indices].mean(axis=1))
        remaining -= batch
    bootstrap_means = np.concatenate(means)
    tail = (1.0 - confidence) / 2.0
    quantiles = np.quantile(array, [0.05, 0.5, 0.95], method="linear")
    interval = np.quantile(bootstrap_means, [tail, 1.0 - tail], method="linear")
    return {
        "mean": float(np.mean(array)),
        "p05": float(quantiles[0]),
        "p50": float(quantiles[1]),
        "p95": float(quantiles[2]),
        "cluster_bootstrap_mean_ci_lower": float(interval[0]),
        "cluster_bootstrap_mean_ci_upper": float(interval[1]),
    }


def _validate_binomial(successes: int, trials: int, confidence: float) -> None:
    if (
        type(successes) is not int
        or type(trials) is not int
        or trials <= 0
        or successes < 0
        or successes > trials
        or not math.isfinite(confidence)
        or not 0.0 < confidence < 1.0
    ):
        raise ValueError("invalid exact-binomial request")


def _same_bounds(left: Sequence[float], right: Sequence[float]) -> bool:
    return all(math.isclose(float(a), float(b), rel_tol=1.0e-10, abs_tol=1.0e-12) for a, b in zip(left, right, strict=True))


def _failure_metric_summary(
    manifest: Mapping[str, Any],
    dgp_id: str,
    stage: str,
    metric: str,
    values: Sequence[float],
) -> dict[str, Any]:
    contract = manifest["failure_summary_contract"]
    dataset_index = contract["aggregate_seed_dataset_index_by_metric"][metric]
    seed = derive_seed(
        manifest["seed_contract"][contract["seed_master"]],
        dgp_id,
        stage,
        dataset_index,
        "oracle",
    )
    observed = dataset_cluster_failure_summary(
        values,
        seed,
        contract["cluster_bootstrap_resamples"],
        contract["cluster_bootstrap_confidence_level"],
    )
    expected = oracle_dataset_cluster_failure_summary(
        values,
        seed,
        contract["cluster_bootstrap_resamples"],
        contract["cluster_bootstrap_confidence_level"],
    )
    tolerance = contract["runner_oracle_endpoint_tolerance"]
    if any(
        not math.isclose(observed[key], expected[key], rel_tol=tolerance, abs_tol=tolerance)
        for key in observed
    ):
        raise ValueError(f"runner/oracle dataset-cluster failure summary differs: {dgp_id}:{metric}")
    return {
        "metric": metric,
        "dataset_count": len(values),
        "seed": seed,
        "resamples": contract["cluster_bootstrap_resamples"],
        "confidence_level": contract["cluster_bootstrap_confidence_level"],
        **observed,
    }


def _expected_digest_checks(
    requested_replicates: int, complete_case_count: int
) -> dict[str, int]:
    return {
        "scientific_result": 1,
        "base_point_result": 1,
        "source_dataset_fingerprint": 1,
        "compiler_analytical_identity": 1,
        "plan": 1,
        "model_scientific_identity": 1,
        "outer_recipe_analytical_identity": 1,
        "complete_case_universe": 1,
        "original_sampling_positions": 1,
        "original_source_rows": 1,
        "bootstrap_schedule_positions": requested_replicates,
        "bootstrap_source_rows": requested_replicates,
        "delete_one_sampling_positions": complete_case_count,
        "delete_one_source_rows": complete_case_count,
    }


def _validate_digest_authentication(
    receipt: Mapping[str, Any],
    document: Mapping[str, Any],
    data_sha256: str,
    source_dataset_fingerprint: str,
    requested_replicates: int,
    complete_case_count: int,
    compact_evidence: Mapping[str, Any],
) -> None:
    expected_checks = _expected_digest_checks(requested_replicates, complete_case_count)
    expected_keys = {
        "schema_version",
        "kind",
        "status",
        "method",
        "document_typed_sha256",
        "scientific_result_sha256",
        "compact_evidence_typed_sha256",
        "data_sha256",
        "source_dataset_fingerprint",
        "checks",
        "reasons",
    }
    if (
        not isinstance(receipt, Mapping)
        or set(receipt) != expected_keys
        or receipt.get("schema_version") != 1
        or receipt.get("kind") != DIGEST_VERIFICATION_KIND
        or receipt.get("status") != "accepted"
        or receipt.get("method") != DIGEST_VERIFICATION_METHOD
        or receipt.get("document_typed_sha256") != typed_json_sha256(document)
        or receipt.get("scientific_result_sha256")
        != compact_evidence.get("scientific_result_sha256")
        or receipt.get("compact_evidence_typed_sha256")
        != typed_json_sha256(compact_evidence)
        or receipt.get("data_sha256") != data_sha256
        or receipt.get("source_dataset_fingerprint") != source_dataset_fingerprint
        or receipt.get("checks") != expected_checks
        or receipt.get("reasons") != []
    ):
        raise ValueError("bound Rust digest verification receipt differs")


def validate_engine_document(
    document: Mapping[str, Any],
    manifest: Mapping[str, Any],
    stage: str,
    dgp: Mapping[str, Any],
    dataset_index: int,
    bootstrap_seed: int,
    data_sha256: str,
    digest_authentication: Mapping[str, Any],
    *,
    expected_workers: int | None = None,
) -> dict[str, Any]:
    reasons: list[str] = []
    plan = stage_by_name(manifest, stage)
    if document.get("schema_version") != 1 or document.get("kind") != "cbsem_exact_cfa_interval_coverage_engine_case_v1" or document.get("status") != "completed":
        raise ValueError("engine result envelope is not a completed coverage case")
    case = document.get("case", {})
    expected_case = {
        "dgp_id": dgp["dgp_id"],
        "dataset_index": dataset_index,
        "n_complete_cases": dgp["sample_size"],
        "v_observed_variables": len(dgp["observed_columns"]),
        "p_free_parameter_rows": len(dgp["parameter_truth"]),
        "d_optimizer_dimensions": len(dgp["parameter_truth"]),
        "requested_replicates": plan["replicates"],
        "workers": plan["workers"] if expected_workers is None else expected_workers,
        "bootstrap_seed": bootstrap_seed,
        "cancel_phase": None,
        "cancel_after": None,
    }
    if case != expected_case or document.get("fixture", {}).get("data_sha256") != data_sha256:
        raise ValueError("engine case or generated-data identity differs")
    if (
        document.get("qualification_status") != "validation_only_product_qualification_blocked"
        or document.get("digest_contract") != manifest["digest_contract"]
        or not _is_sha(document.get("scientific_result_sha256"))
    ):
        raise ValueError("engine qualification boundary or result digest is invalid")

    truth = document.get("truth")
    expected_truth = dgp["parameter_truth"]
    if not isinstance(truth, list) or len(truth) != len(expected_truth):
        raise ValueError("engine truth table has the wrong size")
    truth_by_id = {row["parameter_id"]: row for row in truth}
    expected_by_id = {row["parameter_id"]: row for row in expected_truth}
    if set(truth_by_id) != set(expected_by_id):
        raise ValueError("engine truth parameter identities differ")
    for parameter_id, expected in expected_by_id.items():
        observed = truth_by_id[parameter_id]
        if observed.get("target") != expected["target"] or not _same_float(observed.get("true_value"), expected["true_value"]):
            raise ValueError(f"engine truth drifted for {parameter_id}")

    original = document["original"]
    combined = document["bootstrap"]
    base = combined["base"]
    studentized = combined["studentized"]
    bca = document["bca"]
    point = original["refit"]
    scientific_payload = {"original": original, "bootstrap": combined, "bca": bca}
    if typed_json_sha256(scientific_payload) != document["scientific_result_sha256"]:
        raise ValueError("scientific result digest differs from the serialized scientific tree")
    if typed_json_sha256(point) != base.get("base_point_result_sha256"):
        raise ValueError("base-point digest differs from the serialized exact point result")
    expected_dataset_id = _expected_dataset_uuid(dgp["dgp_id"], dataset_index)
    identity_sha_fields = (
        "compiler_analytical_identity_sha256",
        "plan_sha256",
        "model_scientific_sha256",
        "complete_case_universe_sha256",
    )
    if (
        point.get("method_version") != EXACT_BOOTSTRAP_METHOD
        or point.get("estimator_method_version") != EXACT_ESTIMATOR_METHOD
        or point.get("source_dataset_id") != expected_dataset_id
        or not _is_sha(point.get("source_dataset_fingerprint"))
        or any(not _is_sha(point.get(field)) for field in identity_sha_fields)
        or point.get("source_row_count") != dgp["sample_size"]
        or point.get("complete_case_sample_size") != dgp["sample_size"]
        or point.get("complete_case_universe_digest_method") != UNIVERSE_DIGEST_METHOD
        or point.get("resampled_observations") != dgp["sample_size"]
        or point.get("covariance_denominator") != "maximum_likelihood_n"
        or point.get("sample_indices_digest_method") != INDEX_DIGEST_METHOD
        or point.get("sampling_positions_digest_method") != IDENTITY_POSITIONS_DIGEST_METHOD
        or not _is_sha(point.get("sampling_positions_sha256"))
        or not _is_sha(point.get("sample_indices_sha256"))
        or type(point.get("iterations")) is not int
        or point["iterations"] <= 0
        or not _finite(point.get("objective"))
        or not _finite(point.get("gradient_norm"))
    ):
        raise ValueError("original exact-refit, source, digest, or ML-N identity differs")
    complete_rows = list(range(dgp["sample_size"]))
    expected_universe = complete_case_universe_sha256(
        point["source_dataset_fingerprint"], dgp["sample_size"], complete_rows
    )
    expected_original_positions = sampling_positions_sha256(dgp["sample_size"], complete_rows)
    expected_original_rows = source_rows_sha256(
        point["source_dataset_fingerprint"], dgp["sample_size"], complete_rows
    )
    if (
        point["complete_case_universe_sha256"] != expected_universe
        or point["sampling_positions_sha256"] != expected_original_positions
        or point["sample_indices_sha256"] != expected_original_rows
    ):
        raise ValueError("original universe, sampling-position, or source-row digest differs")
    parameter_ids = [row["parameter_id"] for row in point["free_parameters"]]
    if set(parameter_ids) != set(expected_by_id) or len(parameter_ids) != len(set(parameter_ids)):
        raise ValueError("point-result parameter identity differs")
    if any(not _finite(row.get("estimate")) for row in original["refit"]["free_parameters"]):
        raise ValueError("point result has a nonfinite estimate")
    if base.get("parameter_ids") != parameter_ids or studentized.get("parameter_ids") != parameter_ids or bca.get("parameter_ids") != parameter_ids:
        raise ValueError("interval parameter order differs from point-result order")

    requested = plan["replicates"]
    minimum_usable = max(1000, math.ceil(0.9 * requested))
    if (
        base.get("method_version") != EXACT_BOOTSTRAP_METHOD
        or base.get("estimator_method_version") != EXACT_ESTIMATOR_METHOD
        or base.get("source_dataset_id") != point["source_dataset_id"]
        or base.get("source_dataset_fingerprint") != point["source_dataset_fingerprint"]
        or base.get("compiler_analytical_identity_sha256") != point["compiler_analytical_identity_sha256"]
        or base.get("plan_sha256") != point["plan_sha256"]
        or base.get("model_scientific_sha256") != point["model_scientific_sha256"]
        or not _is_sha(base.get("outer_recipe_analytical_identity_sha256"))
        or not _is_sha(base.get("base_point_result_sha256"))
        or base.get("complete_case_sample_size") != dgp["sample_size"]
        or base.get("complete_case_universe_digest_method") != UNIVERSE_DIGEST_METHOD
        or base.get("complete_case_universe_sha256") != point["complete_case_universe_sha256"]
        or base.get("covariance_denominator") != "maximum_likelihood_n"
        or base.get("sample_indices_digest_method") != INDEX_DIGEST_METHOD
        or base.get("sampling_positions_digest_method") != SCHEDULE_POSITIONS_DIGEST_METHOD
        or base.get("interval_method") != "percentile_type7_v1"
        or not _same_float(base.get("confidence_level"), 0.95)
        or not _same_float(base.get("minimum_usable_fraction"), 0.9)
        or base.get("minimum_usable_replicates") != minimum_usable
        or base.get("seed") != bootstrap_seed
        or base.get("stream_token") != "quickpls_cbsem_exact_cfa_ml_case_bootstrap_v1"
        or base.get("retry_policy") != "no_retry_fixed_preplanned_primary_draws_v1"
        or base.get("max_attempts_per_replicate") != 1
        or base.get("hypothesis_tests") is not None
    ):
        raise ValueError("base bootstrap scientific identity or fixed method contract differs")
    successes = base.get("successful_refits", [])
    failures = base.get("failed_refits", [])
    success_indices = [row.get("replicate_index") for row in successes]
    failure_indices = [row.get("replicate_index") for row in failures]
    if (
        base.get("requested_replicates") != requested
        or base.get("attempted_refits") != requested
        or base.get("usable_replicates") != len(successes)
        or base.get("failed_replicates") != len(failures)
        or success_indices != sorted(success_indices)
        or failure_indices != sorted(failure_indices)
        or set(success_indices).intersection(failure_indices)
        or set(success_indices + failure_indices) != set(range(requested))
        or base.get("retry_policy") != "no_retry_fixed_preplanned_primary_draws_v1"
        or base.get("max_attempts_per_replicate") != 1
    ):
        raise ValueError("point bootstrap ledger is not an exact one-attempt partition")
    for row in successes:
        if (
            len(row.get("parameter_estimates", [])) != len(parameter_ids)
            or any(not _finite(value) for value in row["parameter_estimates"])
            or type(row.get("iterations")) is not int
            or row["iterations"] <= 0
            or not _finite(row.get("objective"))
            or not _finite(row.get("gradient_norm"))
            or not _is_sha(row.get("sampling_positions_sha256"))
            or not _is_sha(row.get("sample_indices_sha256"))
        ):
            raise ValueError("point-success witness is malformed")
    for row in failures:
        if row.get("kind") not in POINT_FAILURE_KINDS or not str(row.get("message", "")).strip() or not _is_sha(row.get("sampling_positions_sha256")) or not _is_sha(row.get("sample_indices_sha256")):
            raise ValueError("point-failure witness is untyped or malformed")
    base_intervals = base.get("intervals", [])
    if base_intervals:
        if [row.get("parameter_id") for row in base_intervals] != parameter_ids:
            raise ValueError("percentile interval parameter order differs")
        point_estimates = [row["estimate"] for row in point["free_parameters"]]
        for row, estimate in zip(base_intervals, point_estimates, strict=True):
            if (
                not _same_float(row.get("original"), estimate)
                or any(
                    not _finite(row.get(field))
                    for field in (
                        "bootstrap_mean", "bias", "standard_error",
                        "percentile_lower", "percentile_upper",
                    )
                )
                or row["percentile_lower"] > row["percentile_upper"]
                or row.get("usable_replicates") != len(successes)
            ):
                raise ValueError("percentile interval identity or finite arithmetic differs")

    point_standard_errors = original.get("standard_errors", {})
    point_se_outcome = point_standard_errors.get("outcome", {})
    if point_standard_errors.get("method_version") != SE_METHOD:
        raise ValueError("point standard-error method identity differs")
    if point_se_outcome.get("status") == "available":
        point_se_rows = point_se_outcome.get("parameters", [])
        if (
            point_se_outcome.get("information_method") != INFORMATION_METHOD
            or [row.get("parameter_id") for row in point_se_rows] != parameter_ids
            or any(not _finite(row.get("standard_error")) or row["standard_error"] <= 0.0 for row in point_se_rows)
        ):
            raise ValueError("point standard-error information identity or rows differ")
    elif point_se_outcome.get("status") == "unavailable":
        if point_se_outcome.get("reason") not in SE_UNAVAILABLE_REASONS:
            raise ValueError("point standard-error unavailability is untyped")
    else:
        raise ValueError("point standard-error outcome is invalid")
    if (
        studentized.get("method_version") != STUDENTIZED_METHOD
        or studentized.get("standard_error_method_version") != SE_METHOD
        or studentized.get("expected_information_method") != INFORMATION_METHOD
        or studentized.get("pivot_method") != "outer_estimate_minus_point_estimate_over_outer_analytic_standard_error_v1"
        or studentized.get("quantile_method") != "percentile_type7_v1"
        or studentized.get("interval_method") != "reversed_type7_studentized_pivot_v1"
        or studentized.get("archive_validation_scope") != "ledger_and_arithmetic_only_no_raw_refit_or_expected_information_replay_v1"
        or not _same_float(studentized.get("confidence_level"), 0.95)
        or not _same_float(studentized.get("minimum_usable_fraction"), 0.9)
        or studentized.get("minimum_usable_replicates") != minimum_usable
        or studentized.get("point_standard_errors") != point_standard_errors
    ):
        raise ValueError("studentized scientific method or bound point-SE identity differs")
    receipts = studentized.get("refit_standard_errors", [])
    if [row.get("replicate_index") for row in receipts] != success_indices:
        raise ValueError("studentized receipt order differs from point successes")
    se_unavailable = 0
    for row in receipts:
        outcome = row.get("outcome", {})
        if outcome.get("status") == "available":
            values = outcome.get("standard_errors", [])
            if (
                outcome.get("information_method") != INFORMATION_METHOD
                or len(values) != len(parameter_ids)
                or any(not _finite(value) or value <= 0.0 for value in values)
            ):
                raise ValueError("available standard-error receipt is malformed")
        elif outcome.get("status") == "unavailable":
            se_unavailable += 1
            if outcome.get("reason") not in SE_UNAVAILABLE_REASONS:
                raise ValueError("standard-error unavailability is untyped")
        else:
            raise ValueError("standard-error receipt status is invalid")
    if studentized.get("studentized_usable_replicates") != len(successes) - se_unavailable:
        raise ValueError("studentized usable count differs from the receipt partition")
    student_interval_rows = studentized.get("intervals", [])
    if [row.get("parameter_id") for row in student_interval_rows] != parameter_ids:
        raise ValueError("studentized interval parameter order differs")
    for row in student_interval_rows:
        outcome = row.get("outcome", {})
        if outcome.get("status") == "available":
            if (
                any(not _finite(outcome.get(field)) for field in (
                    "point_estimate", "point_standard_error", "lower_pivot_quantile",
                    "upper_pivot_quantile", "interval_lower", "interval_upper",
                ))
                or outcome["point_standard_error"] <= 0.0
                or outcome["interval_lower"] > outcome["interval_upper"]
                or outcome.get("usable_replicates") != len(successes) - se_unavailable
            ):
                raise ValueError("studentized available interval is malformed")
        elif outcome.get("status") == "unavailable":
            if outcome.get("reason") not in {
                "point_standard_errors_unavailable",
                "insufficient_studentized_usable_replicates",
            }:
                raise ValueError("studentized interval unavailability is untyped")
        else:
            raise ValueError("studentized interval status is invalid")

    if (
        bca.get("method_version") != "cbsem_exact_case_bootstrap_bca_interval_v1"
        or bca.get("base_bootstrap_method_version") != EXACT_BOOTSTRAP_METHOD
        or bca.get("outer_recipe_analytical_identity_sha256") != base["outer_recipe_analytical_identity_sha256"]
        or bca.get("base_point_result_sha256") != base["base_point_result_sha256"]
        or bca.get("compiler_analytical_identity_sha256") != point["compiler_analytical_identity_sha256"]
        or bca.get("plan_sha256") != point["plan_sha256"]
        or bca.get("model_scientific_sha256") != point["model_scientific_sha256"]
        or bca.get("delete_one_refit_method_version") != "cbsem_exact_case_bootstrap_delete_one_refit_v1"
        or bca.get("bias_correction_method") != "midrank_less_plus_half_ties_no_clamp_v1"
        or bca.get("acceleration_method") != BCA_ACCELERATION_METHOD
        or bca.get("adjusted_probability_method") != BCA_ADJUSTED_PROBABILITY_METHOD
        or bca.get("quantile_method") != "percentile_type7_v1"
        or bca.get("retry_policy") != "no_retry_exactly_one_fit_per_omitted_case_v1"
        or not _same_float(bca.get("confidence_level"), 0.95)
        or bca.get("bootstrap_usable_replicates") != len(successes)
        or bca.get("minimum_bootstrap_usable_replicates") != minimum_usable
    ):
        raise ValueError("BCa scientific method, no-retry, or bound base identity differs")
    delete_successes = bca.get("successful_delete_one_refits", [])
    delete_failures = bca.get("failed_delete_one_refits", [])
    delete_positions = [row.get("omitted_complete_case_position") for row in delete_successes]
    delete_failure_positions = [row.get("omitted_complete_case_position") for row in delete_failures]
    if (
        bca.get("delete_one_case_count") != dgp["sample_size"]
        or delete_positions != sorted(delete_positions)
        or delete_failure_positions != sorted(delete_failure_positions)
        or set(delete_positions).intersection(delete_failure_positions)
        or set(delete_positions + delete_failure_positions) != set(range(dgp["sample_size"]))
    ):
        raise ValueError("delete-one ledger is not an exact N-case partition")
    for row in delete_successes:
        omitted_position = row.get("omitted_complete_case_position")
        retained = [position for position in complete_rows if position != omitted_position]
        if (
            type(row.get("omitted_source_row_index")) is not int
            or not 0 <= row["omitted_source_row_index"] < dgp["sample_size"]
            or row["omitted_source_row_index"] != omitted_position
            or row.get("retained_sampling_positions_sha256")
            != sampling_positions_sha256(dgp["sample_size"], retained)
            or row.get("retained_sample_indices_sha256")
            != source_rows_sha256(
                point["source_dataset_fingerprint"], dgp["sample_size"], retained
            )
            or len(row.get("parameter_estimates", [])) != len(parameter_ids)
            or any(not _finite(value) for value in row["parameter_estimates"])
            or type(row.get("iterations")) is not int
            or row["iterations"] <= 0
            or not _finite(row.get("objective"))
            or not _finite(row.get("gradient_norm"))
        ):
            raise ValueError("delete-one success witness is malformed")
    for row in delete_failures:
        omitted_position = row.get("omitted_complete_case_position")
        retained = [position for position in complete_rows if position != omitted_position]
        if (
            type(row.get("omitted_source_row_index")) is not int
            or not 0 <= row["omitted_source_row_index"] < dgp["sample_size"]
            or row["omitted_source_row_index"] != omitted_position
            or row.get("retained_sampling_positions_sha256")
            != sampling_positions_sha256(dgp["sample_size"], retained)
            or row.get("retained_sample_indices_sha256")
            != source_rows_sha256(
                point["source_dataset_fingerprint"], dgp["sample_size"], retained
            )
            or row.get("kind") not in POINT_FAILURE_KINDS
            or not str(row.get("message", "")).strip()
        ):
            raise ValueError("delete-one failure witness is untyped")
    bca_interval_rows = bca.get("intervals", [])
    if [row.get("parameter_id") for row in bca_interval_rows] != parameter_ids:
        raise ValueError("BCa interval parameter order differs")
    for row in bca_interval_rows:
        outcome = row.get("outcome", {})
        if outcome.get("status") == "available":
            if (
                any(not _finite(outcome.get(field)) for field in (
                    "point_estimate", "bias_correction", "acceleration",
                    "adjusted_lower_probability", "adjusted_upper_probability",
                    "interval_lower", "interval_upper",
                ))
                or not 0.0 <= outcome["adjusted_lower_probability"] <= outcome["adjusted_upper_probability"] <= 1.0
                or outcome["interval_lower"] > outcome["interval_upper"]
                or outcome.get("usable_replicates") != len(successes)
            ):
                raise ValueError("BCa available interval is malformed")
        elif outcome.get("status") == "unavailable":
            if outcome.get("reason") not in BCA_UNAVAILABLE_REASONS:
                raise ValueError("BCa interval unavailability is untyped")
        else:
            raise ValueError("BCa interval status is invalid")

    oracle = audit_engine_case(document)
    if oracle["status"] != "accepted":
        reasons.extend(f"oracle:{reason}" for reason in oracle["reasons"])
    conformance = stage == "conformance"
    if conformance:
        if base["inference"].get("status") != "unavailable" or base["inference"].get("reason_code") != "insufficient_usable_refits" or base.get("intervals"):
            reasons.append("conformance_percentile_did_not_preserve_insufficient_usable")
        if studentized["inference"].get("status") != "unavailable" or studentized["inference"].get("reason") != "insufficient_studentized_usable_replicates":
            reasons.append("conformance_studentized_did_not_preserve_insufficient_usable")
        if bca["inference"].get("status") != "unavailable" or bca["inference"].get("reason") != "base_inference_unavailable":
            reasons.append("conformance_bca_did_not_bind_base_unavailability")

    method_rows = _method_rows(document, expected_by_id, evaluate_coverage=not conformance)
    ledger = {
        "requested_replicates": requested,
        "point_successes": len(successes),
        "point_failures": len(failures),
        "point_failure_counts_by_kind": _counts(failures, "kind", POINT_FAILURE_KINDS),
        "studentized_usable": len(successes) - se_unavailable,
        "standard_error_unavailable": se_unavailable,
        "standard_error_unavailable_counts_by_reason": _counts(
            [row["outcome"] for row in receipts if row["outcome"].get("status") == "unavailable"],
            "reason",
            SE_UNAVAILABLE_REASONS,
        ),
        "delete_one_successes": len(delete_successes),
        "delete_one_failures": len(delete_failures),
        "delete_one_failure_counts_by_kind": _counts(delete_failures, "kind", POINT_FAILURE_KINDS),
        "failure_rate_inference_unit": "dataset_not_bootstrap_replicate",
    }
    compact_evidence = compact_evidence_payload(
        document["scientific_result_sha256"],
        data_sha256,
        point["source_dataset_fingerprint"],
        ledger,
        method_rows,
    )
    _validate_digest_authentication(
        digest_authentication,
        document,
        data_sha256,
        point["source_dataset_fingerprint"],
        requested,
        dgp["sample_size"],
        compact_evidence,
    )
    return {
        "status": "accepted" if not reasons else "rejected",
        "integrity_reasons": sorted(set(reasons)),
        "scientific_result_sha256": document["scientific_result_sha256"],
        "digest_authentication": dict(digest_authentication),
        "oracle": oracle,
        "ledger": ledger,
        "methods": method_rows,
    }


def _method_rows(
    document: Mapping[str, Any], truth_by_id: Mapping[str, Mapping[str, Any]], *, evaluate_coverage: bool
) -> list[dict[str, Any]]:
    base = document["bootstrap"]["base"]
    studentized = document["bootstrap"]["studentized"]
    bca = document["bca"]
    base_intervals = {
        row["parameter_id"]: (row["percentile_lower"], row["percentile_upper"])
        for row in base["intervals"]
    }
    student_intervals = {
        row["parameter_id"]: (
            (row["outcome"]["interval_lower"], row["outcome"]["interval_upper"])
            if row["outcome"].get("status") == "available"
            else None
        )
        for row in studentized["intervals"]
    }
    bca_intervals = {
        row["parameter_id"]: (
            (row["outcome"]["interval_lower"], row["outcome"]["interval_upper"])
            if row["outcome"].get("status") == "available"
            else None
        )
        for row in bca["intervals"]
    }
    maps = {"percentile": base_intervals, "studentized": student_intervals, "bca": bca_intervals}
    rows = []
    for method in METHODS:
        parameters = []
        for parameter_id, truth in truth_by_id.items():
            interval = maps[method].get(parameter_id)
            available = interval is not None
            covered = available and interval[0] <= truth["true_value"] <= interval[1]
            parameters.append(
                {
                    "parameter_id": parameter_id,
                    "available": available,
                    "interval_lower": None if interval is None else interval[0],
                    "interval_upper": None if interval is None else interval[1],
                    "covered_unconditional": None if not evaluate_coverage else bool(covered),
                    "covered_conditional": None if not evaluate_coverage or not available else bool(covered),
                }
            )
        rows.append(
            {
                "method": method,
                "available": all(row["available"] for row in parameters),
                "parameters": parameters,
            }
        )
    return rows


def _counts(rows: Iterable[Mapping[str, Any]], field: str, allowed: Iterable[str]) -> dict[str, int]:
    output = {value: 0 for value in sorted(allowed)}
    for row in rows:
        output[row[field]] += 1
    return output


def _finite(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(float(value))


def _same_float(left: Any, right: Any) -> bool:
    return _finite(left) and math.isclose(float(left), float(right), rel_tol=0.0, abs_tol=1.0e-15)


def _is_sha(value: Any) -> bool:
    return isinstance(value, str) and SHA256_PATTERN.fullmatch(value) is not None


def _expected_dataset_uuid(dgp_id: str, dataset_index: int) -> str:
    tag = 0xA if dgp_id == "dgp_a_one_factor_n150" else 0xB
    base = int("cb5ec0a0000000000000000000000000", 16)
    return str(uuid.UUID(int=base | (tag << 32) | dataset_index))


def engine_command(
    binary: Path,
    data_path: Path,
    output_path: Path,
    shard: Mapping[str, Any],
    dataset_index: int,
    bootstrap_seed: int,
    workers: int,
    cancel_phase: str | None = None,
    cancel_after: int | None = None,
) -> list[str]:
    command = [
        str(binary),
        "--repo-root", str(ROOT),
        "--data", str(data_path),
        "--output", str(output_path),
        "--dgp-id", shard["dgp_id"],
        "--dataset-index", str(dataset_index),
        "--replicates", str(shard["replicates"]),
        "--workers", str(workers),
        "--bootstrap-seed", str(bootstrap_seed),
        "--cancel-phase", cancel_phase or "none",
    ]
    if cancel_after is not None:
        command.extend(("--cancel-after", str(cancel_after)))
    return command


def execute_engine(command: Sequence[str], timeout_seconds: int) -> subprocess.CompletedProcess[str]:
    flags = getattr(subprocess, "CREATE_NO_WINDOW", 0)
    return subprocess.run(
        list(command),
        cwd=ROOT,
        text=True,
        capture_output=True,
        timeout=timeout_seconds,
        check=False,
        creationflags=flags,
    )


def _process_parent_snapshot() -> dict[int, int]:
    if os.name != "nt":
        parents: dict[int, int] = {}
        proc = Path("/proc")
        if not proc.is_dir():
            return parents
        for entry in proc.iterdir():
            if not entry.name.isdigit():
                continue
            try:
                fields = (entry / "stat").read_text(encoding="utf-8").split()
                parents[int(entry.name)] = int(fields[3])
            except (OSError, UnicodeError, ValueError, IndexError):
                continue
        return parents

    from ctypes import wintypes

    class PROCESSENTRY32W(ctypes.Structure):
        _fields_ = [
            ("dwSize", wintypes.DWORD),
            ("cntUsage", wintypes.DWORD),
            ("th32ProcessID", wintypes.DWORD),
            ("th32DefaultHeapID", ctypes.c_size_t),
            ("th32ModuleID", wintypes.DWORD),
            ("cntThreads", wintypes.DWORD),
            ("th32ParentProcessID", wintypes.DWORD),
            ("pcPriClassBase", wintypes.LONG),
            ("dwFlags", wintypes.DWORD),
            ("szExeFile", wintypes.WCHAR * 260),
        ]

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    snapshot = kernel32.CreateToolhelp32Snapshot(0x00000002, 0)
    if snapshot == ctypes.c_void_p(-1).value:
        return {}
    entry = PROCESSENTRY32W()
    entry.dwSize = ctypes.sizeof(entry)
    parents = {}
    try:
        if kernel32.Process32FirstW(snapshot, ctypes.byref(entry)):
            while True:
                parents[int(entry.th32ProcessID)] = int(entry.th32ParentProcessID)
                if not kernel32.Process32NextW(snapshot, ctypes.byref(entry)):
                    break
    finally:
        kernel32.CloseHandle(snapshot)
    return parents


def _process_tree_ids(root_pid: int) -> set[int]:
    parents = _process_parent_snapshot()
    descendants = {root_pid}
    changed = True
    while changed:
        changed = False
        for pid, parent in parents.items():
            if parent in descendants and pid not in descendants:
                descendants.add(pid)
                changed = True
    return descendants


def _process_exists(pid: int) -> bool:
    if os.name != "nt":
        return Path(f"/proc/{pid}").exists()
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    handle = kernel32.OpenProcess(0x1000, False, pid)
    if not handle:
        return False
    kernel32.CloseHandle(handle)
    return True


def execute_engine_observed(
    command: Sequence[str], timeout_seconds: int
) -> tuple[subprocess.CompletedProcess[str], int, bool]:
    """Execute while sampling the full process tree and observing orphans."""

    flags = getattr(subprocess, "CREATE_NO_WINDOW", 0)
    process = subprocess.Popen(
        list(command),
        cwd=ROOT,
        text=True,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        shell=False,
        creationflags=flags,
    )
    observed_pids: set[int] = set()
    deadline = time.monotonic() + timeout_seconds
    timed_out = False
    while process.poll() is None:
        observed_pids.update(_process_tree_ids(process.pid))
        if time.monotonic() >= deadline:
            timed_out = True
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
            break
        time.sleep(PROCESS_POLL_SECONDS)
    observed_pids.update(_process_tree_ids(process.pid))
    stdout, stderr = process.communicate()
    time.sleep(PROCESS_POLL_SECONDS)
    orphan_processes = sum(
        1 for pid in observed_pids if pid != process.pid and _process_exists(pid)
    )
    completed = subprocess.CompletedProcess(
        list(command), int(process.returncode), stdout, stderr
    )
    return completed, orphan_processes, timed_out


def verify_engine_digests(
    binary: Path, data_path: Path, document_path: Path, timeout_seconds: int
) -> dict[str, Any]:
    process = execute_engine(
        [
            str(binary),
            "--verify-document",
            str(document_path),
            "--data",
            str(data_path),
        ],
        timeout_seconds,
    )
    try:
        receipt = json.loads(process.stdout)
    except (json.JSONDecodeError, TypeError) as error:
        raise ValueError(
            f"digest verifier emitted invalid JSON (exit {process.returncode}): {process.stderr[-1000:]}"
        ) from error
    if not isinstance(receipt, dict):
        raise ValueError("digest verifier receipt is not an object")
    return receipt


def _dataset_seeds(manifest: Mapping[str, Any], dgp_id: str, stage: str, index: int) -> dict[str, int]:
    derivation_stage = seed_derivation_stage(manifest, stage)
    return {
        purpose: derive_seed(
            manifest["seed_contract"][f"{purpose}_master_seed"],
            dgp_id,
            derivation_stage,
            index,
            purpose,
        )
        for purpose in ("data", "bootstrap", "oracle")
    }


def run_dataset(
    binary: Path,
    manifest: Mapping[str, Any],
    manifest_sha256: str,
    shard: Mapping[str, Any],
    dataset_index: int,
    scratch: Path,
) -> tuple[dict[str, Any], dict[str, Any] | None, Path | None]:
    dgp = dgp_by_id(manifest, shard["dgp_id"])
    seeds = _dataset_seeds(manifest, shard["dgp_id"], shard["stage"], dataset_index)
    data_bytes = generate_dataset_csv(dgp, seeds["data"])
    data_sha = hashlib.sha256(data_bytes).hexdigest()
    data_path = scratch / f"dataset-{dataset_index:06d}.csv"
    data_path.write_bytes(data_bytes)
    raw_path = scratch / f"engine-{dataset_index:06d}.json"
    try:
        process = execute_engine(
            engine_command(
                binary,
                data_path,
                raw_path,
                shard,
                dataset_index,
                seeds["bootstrap"],
                shard["workers"],
            ),
            stage_by_name(manifest, shard["stage"])["per_dataset_timeout_seconds"],
        )
    except subprocess.TimeoutExpired as error:
        receipt = rejected_dataset_receipt(
            manifest_sha256,
            shard,
            dataset_index,
            seeds,
            data_sha,
            f"engine_timeout:{error.timeout}",
        )
        return receipt, None, raw_path if raw_path.exists() else None
    if process.returncode != 0 or not raw_path.is_file():
        receipt = rejected_dataset_receipt(
            manifest_sha256, shard, dataset_index, seeds, data_sha,
            f"engine_exit:{process.returncode}:{process.stderr[-2000:]}",
        )
        return receipt, None, raw_path if raw_path.exists() else None
    document = None
    digest_authentication = None
    try:
        document = load_json(raw_path)
        digest_authentication = verify_engine_digests(
            binary,
            data_path,
            raw_path,
            min(300, stage_by_name(manifest, shard["stage"])["per_dataset_timeout_seconds"]),
        )
        validation = validate_engine_document(
            document,
            manifest,
            shard["stage"],
            dgp,
            dataset_index,
            seeds["bootstrap"],
            data_sha,
            digest_authentication,
        )
        reasons = validation["integrity_reasons"]
    except Exception as error:
        validation = None
        reasons = [f"engine_validation:{type(error).__name__}:{error}"]
    receipt = {
        "dataset_index": dataset_index,
        "status": "accepted" if validation is not None and not reasons else "rejected",
        "manifest_sha256": manifest_sha256,
        "dgp_id": shard["dgp_id"],
        "stage": shard["stage"],
        "seeds": seeds,
        "data_sha256": data_sha,
        "scientific_result_sha256": (
            document.get("scientific_result_sha256") if isinstance(document, Mapping) else None
        ),
        "digest_authentication": digest_authentication,
        "integrity_reasons": reasons,
        "coverage_evaluated": shard["stage"] != "conformance",
        "ledger": None if validation is None else validation["ledger"],
        "methods": [] if validation is None else validation["methods"],
        "oracle": None if validation is None else validation["oracle"],
    }
    return receipt, document, raw_path


def rejected_dataset_receipt(
    manifest_sha256: str,
    shard: Mapping[str, Any],
    dataset_index: int,
    seeds: Mapping[str, int],
    data_sha256: str,
    reason: str,
) -> dict[str, Any]:
    return {
        "dataset_index": dataset_index,
        "status": "rejected",
        "manifest_sha256": manifest_sha256,
        "dgp_id": shard["dgp_id"],
        "stage": shard["stage"],
        "seeds": dict(seeds),
        "data_sha256": data_sha256,
        "scientific_result_sha256": None,
        "digest_authentication": None,
        "integrity_reasons": [reason],
        "coverage_evaluated": shard["stage"] != "conformance",
        "ledger": None,
        "methods": [],
        "oracle": None,
    }


def run_replay(
    binary: Path,
    manifest: Mapping[str, Any],
    shard: Mapping[str, Any],
    first_receipt: Mapping[str, Any],
    scratch: Path,
) -> tuple[dict[str, Any], Path | None]:
    index = first_receipt["dataset_index"]
    dgp = dgp_by_id(manifest, shard["dgp_id"])
    seeds = first_receipt["seeds"]
    data_bytes = generate_dataset_csv(dgp, seeds["data"])
    data_path = scratch / "replay.csv"
    data_path.write_bytes(data_bytes)
    raw_path = scratch / "replay.json"
    try:
        process = execute_engine(
            engine_command(binary, data_path, raw_path, shard, index, seeds["bootstrap"], 1),
            stage_by_name(manifest, shard["stage"])["per_dataset_timeout_seconds"],
        )
    except subprocess.TimeoutExpired as error:
        return {
            "status": "rejected",
            "dataset_index": index,
            "primary_workers": shard["workers"],
            "replay_workers": 1,
            "primary_scientific_result_sha256": first_receipt.get("scientific_result_sha256"),
            "replay_scientific_result_sha256": None,
            "digest_authentication": None,
            "reasons": [f"replay_engine_timeout:{error.timeout}"],
        }, raw_path if raw_path.is_file() else None
    reasons = []
    observed_sha = None
    digest_authentication = None
    if process.returncode != 0 or not raw_path.is_file():
        reasons.append(f"replay_engine_exit:{process.returncode}")
    else:
        try:
            document = load_json(raw_path)
            if document.get("case", {}).get("workers") != 1:
                raise ValueError("replay child did not declare the frozen W=1 envelope")
            observed_sha = document.get("scientific_result_sha256")
            digest_authentication = verify_engine_digests(
                binary,
                data_path,
                raw_path,
                min(300, stage_by_name(manifest, shard["stage"])["per_dataset_timeout_seconds"]),
            )
            validate_engine_document(
                document,
                manifest,
                shard["stage"],
                dgp,
                index,
                seeds["bootstrap"],
                first_receipt["data_sha256"],
                digest_authentication,
                expected_workers=1,
            )
        except Exception as error:
            reasons.append(f"replay_validation:{type(error).__name__}:{error}")
        if observed_sha != first_receipt.get("scientific_result_sha256"):
            reasons.append("worker_replay_scientific_digest_differs")
    return {
        "status": "accepted" if not reasons else "rejected",
        "dataset_index": index,
        "primary_workers": shard["workers"],
        "replay_workers": 1,
        "primary_scientific_result_sha256": first_receipt.get("scientific_result_sha256"),
        "replay_scientific_result_sha256": observed_sha,
        "digest_authentication": digest_authentication,
        "reasons": reasons,
    }, raw_path if raw_path.is_file() else None


def run_cancellation_probe(
    binary: Path,
    manifest: Mapping[str, Any],
    shard: Mapping[str, Any],
    scratch: Path,
) -> tuple[dict[str, Any], Path | None]:
    contract = manifest["cancellation_contract"]
    phase = contract["phase_by_dgp"][shard["dgp_id"]]
    dgp = dgp_by_id(manifest, shard["dgp_id"])
    index = contract["dataset_index"]
    seeds = _dataset_seeds(manifest, shard["dgp_id"], shard["stage"], index)
    data_path = scratch / f"cancel-{phase}.csv"
    data_bytes = generate_dataset_csv(dgp, seeds["data"])
    data_sha256 = hashlib.sha256(data_bytes).hexdigest()
    data_path.write_bytes(data_bytes)
    raw_path = scratch / f"cancel-{phase}.json"
    process, orphan_processes, timed_out = execute_engine_observed(
        engine_command(
            binary, data_path, raw_path, shard, index, seeds["bootstrap"], shard["workers"],
            phase, contract["cancel_after_completed"],
        ),
        contract["timeout_seconds"],
    )
    reasons = []
    latency = None
    if timed_out:
        reasons.append("cancellation_engine_timeout")
    if process.returncode != 0 or not raw_path.is_file():
        reasons.append(f"cancellation_engine_exit:{process.returncode}")
    else:
        try:
            document = load_json(raw_path)
            latency = document.get("cancellation", {}).get("terminal_latency_seconds")
            expected_case = {
                "dgp_id": dgp["dgp_id"],
                "dataset_index": index,
                "n_complete_cases": dgp["sample_size"],
                "v_observed_variables": len(dgp["observed_columns"]),
                "p_free_parameter_rows": len(dgp["parameter_truth"]),
                "d_optimizer_dimensions": len(dgp["parameter_truth"]),
                "requested_replicates": shard["replicates"],
                "workers": shard["workers"],
                "bootstrap_seed": seeds["bootstrap"],
                "cancel_phase": phase,
                "cancel_after": contract["cancel_after_completed"],
            }
            expected_keys = {
                "schema_version",
                "kind",
                "status",
                "qualification_status",
                "case",
                "elapsed_seconds",
                "cancellation",
                "fixture",
            }
            if (
                set(document) != expected_keys
                or document.get("schema_version") != 1
                or document.get("kind") != "cbsem_exact_cfa_interval_coverage_engine_case_v1"
                or document.get("status") != "cancelled_as_requested"
                or document.get("qualification_status")
                != "validation_only_product_qualification_blocked"
                or document.get("case") != expected_case
                or document.get("fixture") != {"data_sha256": data_sha256}
                or set(document.get("cancellation", {}))
                != {"phase", "terminal_latency_seconds"}
                or document.get("cancellation", {}).get("phase") != phase
                or not _finite(document.get("elapsed_seconds"))
                or document["elapsed_seconds"] < 0.0
                or not _finite(latency)
                or latency < 0.0
                or latency > contract["maximum_terminal_latency_seconds"]
            ):
                reasons.append("cancellation_receipt_or_latency_differs")
        except Exception as error:
            reasons.append(f"cancellation_validation:{type(error).__name__}:{error}")
    if orphan_processes > contract["maximum_orphan_processes"]:
        reasons.append("cancellation_orphan_process_limit_exceeded")
    return {
        "status": "accepted" if not reasons else "rejected",
        "dgp_id": dgp["dgp_id"],
        "dataset_index": index,
        "phase": phase,
        "bootstrap_seed": seeds["bootstrap"],
        "data_sha256": data_sha256,
        "terminal_latency_seconds": latency,
        "maximum_terminal_latency_seconds": contract["maximum_terminal_latency_seconds"],
        "orphan_processes": orphan_processes,
        "maximum_orphan_processes": contract["maximum_orphan_processes"],
        "reasons": reasons,
    }, raw_path if raw_path.is_file() else None


def run_shard(
    manifest: Mapping[str, Any],
    manifest_sha256: str,
    shard: Mapping[str, Any],
    run_directory: Path,
) -> dict[str, Any]:
    if not shard["executable"] or shard["stage"] not in {"conformance", "pilot"}:
        raise ValueError("only conformance and pilot shards are executable")
    scientific_runtime = _require_scientific_runtime(manifest)
    shard_directory = run_directory / "shards" / shard["shard_id"]
    if shard_directory.exists():
        raise FileExistsError(f"append-only shard already exists: {shard_directory}")
    shard_directory.mkdir(parents=True)
    binary = (ROOT / manifest["binary_binding"]["path"]).resolve()
    receipts = []
    retained = []

    def retain_artifact(
        artifact_kind: str,
        dataset_index: int,
        source: Path | None,
        diagnostic: Mapping[str, Any],
        filename: str,
    ) -> None:
        destination = shard_directory / "audit" / filename
        destination.parent.mkdir(parents=True, exist_ok=True)
        if destination.exists():
            raise FileExistsError(destination)
        if source is not None and source.is_file():
            shutil.copyfile(source, destination)
        else:
            write_new_json(
                destination,
                {
                    "schema_version": 1,
                    "kind": "cbsem_exact_cfa_interval_coverage_child_failure_diagnostic_v1",
                    "artifact_kind": artifact_kind,
                    "diagnostic": dict(diagnostic),
                },
            )
        retained.append(
            {
                "artifact_kind": artifact_kind,
                "dataset_index": dataset_index,
                "path": destination.relative_to(ROOT).as_posix(),
                "sha256": sha256(destination),
            }
        )

    with tempfile.TemporaryDirectory(prefix=f"qpls-coverage-{shard['shard_id']}-") as raw:
        scratch = Path(raw)
        for offset, dataset_index in enumerate(shard["dataset_indices"]):
            try:
                receipt, _document, raw_path = run_dataset(
                    binary, manifest, manifest_sha256, shard, dataset_index, scratch
                )
            except Exception as error:
                seeds = _dataset_seeds(manifest, shard["dgp_id"], shard["stage"], dataset_index)
                receipt = rejected_dataset_receipt(
                    manifest_sha256, shard, dataset_index, seeds, "0" * 64,
                    f"dataset_exception:{type(error).__name__}:{error}",
                )
                raw_path = None
            retain = offset == 0 or receipt["status"] == "rejected"
            if retain:
                retain_artifact(
                    "dataset",
                    dataset_index,
                    raw_path,
                    receipt,
                    f"dataset-{dataset_index:06d}.json",
                )
            receipts.append(receipt)

        replay = None
        cancellation = None
        if shard["shard_ordinal"] == 0 and receipts:
            replay, replay_raw_path = run_replay(binary, manifest, shard, receipts[0], scratch)
            if replay["status"] != "accepted":
                retain_artifact(
                    "worker_replay",
                    replay["dataset_index"],
                    replay_raw_path,
                    replay,
                    "worker-replay.json",
                )
            if shard["stage"] == "conformance":
                cancellation, cancellation_raw_path = run_cancellation_probe(
                    binary, manifest, shard, scratch
                )
                if cancellation["status"] != "accepted":
                    retain_artifact(
                        f"cancellation_{cancellation['phase']}",
                        cancellation["dataset_index"],
                        cancellation_raw_path,
                        cancellation,
                        f"cancellation-{cancellation['phase']}.json",
                    )

    accepted = all(row["status"] == "accepted" for row in receipts)
    contracts_accepted = (replay is None or replay["status"] == "accepted") and (
        cancellation is None or cancellation["status"] == "accepted"
    )
    case = {
        "schema_version": 1,
        "kind": CASE_KIND,
        "status": "accepted_validation_shard" if accepted and contracts_accepted else "rejected_validation_shard",
        "qualification_status": "blocked_not_product_qualification",
        "manifest_sha256": manifest_sha256,
        "binary_sha256": manifest["binary_binding"]["sha256"],
        "shard": dict(shard),
        "environment": {
            "os": platform.system(),
            "os_release": platform.release(),
            "architecture": platform.machine(),
            "logical_cores": os.cpu_count(),
            **scientific_runtime,
        },
        "dataset_receipts": receipts,
        "worker_replay": replay,
        "cancellation": cancellation,
        "retained_full_audit_results": retained,
        "retention_policy": manifest["retention_contract"],
    }
    schema = load_json(CASE_SCHEMA)
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(case)
    write_new_json(shard_directory / "case.json", case)
    return case


def _validate_compact_dataset_evidence(
    receipt: Mapping[str, Any],
    manifest: Mapping[str, Any],
    stage: str,
    dgp: Mapping[str, Any],
    requested_replicates: int,
) -> None:
    """Authenticate and reconcile one accepted compact dataset receipt."""

    ledger = receipt["ledger"]
    methods = receipt["methods"]
    authentication = receipt["digest_authentication"]
    oracle = receipt["oracle"]
    sample_size = dgp["sample_size"]
    parameter_truth = dgp["parameter_truth"]
    parameter_ids = [row["parameter_id"] for row in parameter_truth]

    if (
        not _is_sha(receipt.get("scientific_result_sha256"))
        or not isinstance(authentication, Mapping)
        or authentication.get("schema_version") != 1
        or authentication.get("kind") != DIGEST_VERIFICATION_KIND
        or authentication.get("status") != "accepted"
        or authentication.get("method") != DIGEST_VERIFICATION_METHOD
        or not _is_sha(authentication.get("document_typed_sha256"))
        or authentication.get("scientific_result_sha256")
        != receipt["scientific_result_sha256"]
        or authentication.get("data_sha256") != receipt["data_sha256"]
        or not _is_sha(authentication.get("source_dataset_fingerprint"))
        or authentication.get("checks")
        != _expected_digest_checks(requested_replicates, sample_size)
        or authentication.get("reasons") != []
    ):
        raise ValueError("accepted compact receipt lacks exact Rust authentication cross-links")

    point_failure_counts = ledger["point_failure_counts_by_kind"]
    standard_error_counts = ledger["standard_error_unavailable_counts_by_reason"]
    delete_one_failure_counts = ledger["delete_one_failure_counts_by_kind"]
    if (
        ledger["requested_replicates"] != requested_replicates
        or ledger["point_successes"] + ledger["point_failures"] != requested_replicates
        or set(point_failure_counts) != POINT_FAILURE_KINDS
        or sum(point_failure_counts.values()) != ledger["point_failures"]
        or ledger["studentized_usable"] + ledger["standard_error_unavailable"]
        != ledger["point_successes"]
        or set(standard_error_counts) != SE_UNAVAILABLE_REASONS
        or sum(standard_error_counts.values()) != ledger["standard_error_unavailable"]
        or ledger["delete_one_successes"] + ledger["delete_one_failures"]
        != sample_size
        or set(delete_one_failure_counts) != POINT_FAILURE_KINDS
        or sum(delete_one_failure_counts.values()) != ledger["delete_one_failures"]
        or ledger["failure_rate_inference_unit"]
        != "dataset_not_bootstrap_replicate"
    ):
        raise ValueError("accepted compact ledger is not an exact B/N partition")

    if [row["method"] for row in methods] != list(METHODS):
        raise ValueError("accepted compact method order differs")
    method_by_name: dict[str, Mapping[str, Any]] = {}
    available_parameter_counts: dict[str, int] = {}
    truth_by_id = {row["parameter_id"]: row["true_value"] for row in parameter_truth}
    for method in methods:
        parameters = method["parameters"]
        if [row["parameter_id"] for row in parameters] != parameter_ids:
            raise ValueError("accepted compact parameter identity or order differs")
        for parameter in parameters:
            available = parameter["available"]
            lower = parameter["interval_lower"]
            upper = parameter["interval_upper"]
            if available:
                if not _finite(lower) or not _finite(upper) or lower > upper:
                    raise ValueError("available compact interval endpoints are invalid")
            elif lower is not None or upper is not None:
                raise ValueError("unavailable compact interval emitted endpoints")

            if stage == "conformance":
                expected_unconditional = None
                expected_conditional = None
            else:
                covered = bool(
                    available
                    and lower <= truth_by_id[parameter["parameter_id"]] <= upper
                )
                expected_unconditional = covered
                expected_conditional = covered if available else None
            if (
                parameter["covered_unconditional"] is not expected_unconditional
                or parameter["covered_conditional"] is not expected_conditional
            ):
                raise ValueError("coverage flags differ from authenticated endpoints and truth")

        expected_method_available = all(row["available"] for row in parameters)
        if method["available"] is not expected_method_available:
            raise ValueError("compact method availability differs from parameter availability")
        method_by_name[method["method"]] = method
        available_parameter_counts[method["method"]] = sum(
            row["available"] for row in parameters
        )

    compact = compact_evidence_payload(
        receipt["scientific_result_sha256"],
        receipt["data_sha256"],
        authentication["source_dataset_fingerprint"],
        ledger,
        methods,
    )
    if authentication.get("compact_evidence_typed_sha256") != typed_json_sha256(compact):
        raise ValueError("compact receipt differs from Rust-authenticated raw evidence")

    minimum_usable = max(1000, math.ceil(0.9 * requested_replicates))
    expected_base_available = ledger["point_successes"] >= minimum_usable
    expected_studentized_available = method_by_name["studentized"]["available"]
    expected_bca_global_available = (
        expected_base_available and ledger["delete_one_failures"] == 0
    )
    if (
        not isinstance(oracle, Mapping)
        or oracle.get("schema_version") != 1
        or oracle.get("kind")
        != "cbsem_exact_cfa_interval_coverage_oracle_receipt_v1"
        or oracle.get("status") != "accepted"
        or oracle.get("reasons") != []
        or oracle.get("parameter_count") != len(parameter_ids)
        or oracle.get("requested_replicates") != requested_replicates
        or oracle.get("point_successes") != ledger["point_successes"]
        or oracle.get("point_failures") != ledger["point_failures"]
        or oracle.get("studentized_usable") != ledger["studentized_usable"]
        or oracle.get("delete_one_successes") != ledger["delete_one_successes"]
        or oracle.get("delete_one_failures") != ledger["delete_one_failures"]
        or oracle.get("base_inference_available") is not expected_base_available
        or oracle.get("studentized_inference_available")
        is not expected_studentized_available
        or oracle.get("bca_inference_available") is not expected_bca_global_available
        or oracle.get("endpoint_relative_tolerance")
        != manifest["coverage_contract"]["endpoint_relative_tolerance"]
        or oracle.get("percentile_mean_validation_method")
        != manifest["arithmetic_kernel_contract"]["percentile_mean_validation_method"]
        or oracle.get("percentile_bias_binding_method")
        != manifest["arithmetic_kernel_contract"]["percentile_bias_binding_method"]
        or oracle.get("bca_acceleration_method")
        != manifest["arithmetic_kernel_contract"]["bca_acceleration_method"]
        or oracle.get("bca_adjusted_probability_method")
        != manifest["arithmetic_kernel_contract"]["bca_adjusted_probability_method"]
    ):
        raise ValueError("oracle receipt does not exactly mirror authenticated compact evidence")

    if method_by_name["percentile"]["available"] is not expected_base_available:
        raise ValueError("percentile availability differs from the exact base ledger")
    expected_percentile_parameters = len(parameter_ids) if expected_base_available else 0
    if available_parameter_counts["percentile"] != expected_percentile_parameters:
        raise ValueError("percentile parameter availability differs from the base inference status")
    if available_parameter_counts["studentized"] not in {0, len(parameter_ids)}:
        raise ValueError("studentized parameter availability is not the global inference status")
    if (
        expected_studentized_available
        and ledger["studentized_usable"] < minimum_usable
    ):
        raise ValueError("studentized endpoints exist below the frozen usable threshold")
    if not expected_bca_global_available and available_parameter_counts["bca"] != 0:
        raise ValueError("BCa endpoints exist without a complete base/delete-one ledger")
    expected_endpoint_checks = 2 * sum(available_parameter_counts.values())
    expected_arithmetic_checks = (
        4 * available_parameter_counts["percentile"]
        + 4 * available_parameter_counts["studentized"]
        + 5 * available_parameter_counts["bca"]
    )
    if (
        oracle.get("endpoint_checks") != expected_endpoint_checks
        or oracle.get("arithmetic_checks") != expected_arithmetic_checks
    ):
        raise ValueError("oracle arithmetic/check counts differ from available compact rows")


def _rejected_aggregate_report(
    manifest: Mapping[str, Any],
    manifest_sha256: str,
    stage: str,
    cases: Sequence[Mapping[str, Any]],
    receipts: Sequence[Mapping[str, Any]],
) -> dict[str, Any]:
    diagnostics = []
    for case in cases:
        rejected_datasets = [
            {
                "dataset_index": row["dataset_index"],
                "integrity_reasons": row["integrity_reasons"],
            }
            for row in case["dataset_receipts"]
            if row["status"] == "rejected"
        ]
        replay = case["worker_replay"]
        cancellation = case["cancellation"]
        if (
            case["status"] == "rejected_validation_shard"
            or rejected_datasets
            or (replay is not None and replay["status"] == "rejected")
            or (cancellation is not None and cancellation["status"] == "rejected")
        ):
            diagnostics.append(
                {
                    "shard_id": case["shard"]["shard_id"],
                    "shard_status": case["status"],
                    "rejected_datasets": rejected_datasets,
                    "worker_replay_reasons": (
                        [] if replay is None or replay["status"] == "accepted" else replay["reasons"]
                    ),
                    "cancellation_reasons": (
                        []
                        if cancellation is None or cancellation["status"] == "accepted"
                        else cancellation["reasons"]
                    ),
                    "retained_full_audit_results": case["retained_full_audit_results"],
                }
            )
    return {
        "schema_version": 1,
        "kind": REPORT_KIND,
        "stage": stage,
        "status": f"{stage}_evidence_rejected_product_qualification_blocked",
        "qualification_status": "blocked_not_product_qualification",
        "manifest_sha256": manifest_sha256,
        "binary_sha256": manifest["binary_binding"]["sha256"],
        "shard_count": len(cases),
        "dataset_count": len(receipts),
        "all_predeclared_evidence_accepted": False,
        "coverage_decision": "not_evaluated_rejected_integrity_evidence",
        "primary_cells": [],
        "availability_cells": [],
        "failure_summary": [],
        "rejection_diagnostics": diagnostics,
        "qualification_boundary": manifest["qualification_boundary"],
    }


def aggregate_case_documents(
    manifest: Mapping[str, Any],
    manifest_sha256: str,
    stage: str,
    cases: Sequence[Mapping[str, Any]],
    *,
    evidence_root: Path = ROOT,
) -> dict[str, Any]:
    if stage not in {"conformance", "pilot"}:
        raise ValueError("only executable stages may be aggregated")
    _require_scientific_runtime(manifest)
    case_schema = load_json(CASE_SCHEMA)
    Draft202012Validator.check_schema(case_schema)
    validator = Draft202012Validator(case_schema)
    for case in cases:
        validator.validate(case)
    expected = plan_shards(manifest, stage)
    expected_by_id = {row["shard_id"]: row for row in expected}
    expected_ids = {row["shard_id"] for row in expected}
    try:
        observed_ids = [row["shard"]["shard_id"] for row in cases]
    except (KeyError, TypeError) as error:
        raise ValueError("aggregate shard envelope is malformed") from error
    if len(observed_ids) != len(set(observed_ids)) or set(observed_ids) != expected_ids:
        raise ValueError("aggregate shard family is duplicated or incomplete")
    evidence_root = evidence_root.resolve()
    for case in cases:
        shard = expected_by_id[case["shard"]["shard_id"]]
        if case["shard"] != shard:
            raise ValueError(f"aggregate shard differs from immutable plan: {shard['shard_id']}")
        if (
            case.get("manifest_sha256") != manifest_sha256
            or case.get("binary_sha256") != manifest["binary_binding"]["sha256"]
            or case.get("qualification_status") != "blocked_not_product_qualification"
            or case.get("retention_policy") != manifest["retention_contract"]
        ):
            raise ValueError("aggregate source, qualification, or retention binding differs")
        case_runtime = {
            key: case.get("environment", {}).get(key)
            for key in ("python", "numpy", "scipy")
        }
        if case_runtime != manifest["scientific_runtime"]:
            raise ValueError(
                f"aggregate scientific runtime differs for {shard['shard_id']}"
            )
        case_receipts = case["dataset_receipts"]
        if [row["dataset_index"] for row in case_receipts] != shard["dataset_indices"]:
            raise ValueError(f"aggregate receipt order differs for {shard['shard_id']}")
        dgp = dgp_by_id(manifest, shard["dgp_id"])
        for receipt in case_receipts:
            index = receipt["dataset_index"]
            expected_seeds = _dataset_seeds(manifest, shard["dgp_id"], stage, index)
            expected_data_sha = hashlib.sha256(
                generate_dataset_csv(dgp, expected_seeds["data"])
            ).hexdigest()
            if (
                receipt["manifest_sha256"] != manifest_sha256
                or receipt["dgp_id"] != shard["dgp_id"]
                or receipt["stage"] != stage
                or receipt["seeds"] != expected_seeds
                or receipt["data_sha256"] != expected_data_sha
                or receipt["coverage_evaluated"] != (stage != "conformance")
            ):
                raise ValueError(f"aggregate dataset identity differs: {shard['shard_id']}:{index}")
            if receipt["status"] == "accepted":
                if (
                    receipt["integrity_reasons"]
                    or receipt["ledger"] is None
                    or receipt["oracle"] is None
                    or receipt["digest_authentication"] is None
                ):
                    raise ValueError("accepted dataset receipt lacks accepted integrity evidence")
                _validate_compact_dataset_evidence(
                    receipt,
                    manifest,
                    stage,
                    dgp,
                    shard["replicates"],
                )
            elif not receipt["integrity_reasons"]:
                raise ValueError("rejected dataset receipt must preserve an integrity reason")

        expected_replay = shard["shard_ordinal"] == 0
        replay = case["worker_replay"]
        if expected_replay:
            first = case_receipts[0]
            if replay is None or (
                replay["dataset_index"] != first["dataset_index"]
                or replay["primary_workers"]
                != manifest["worker_replay_contract"]["primary_workers"]
                or replay["replay_workers"]
                != manifest["worker_replay_contract"]["replay_workers"]
                or replay["primary_scientific_result_sha256"]
                != first["scientific_result_sha256"]
            ):
                raise ValueError(f"worker replay contract differs for {shard['shard_id']}")
            if replay["status"] == "accepted" and (
                replay["reasons"]
                or replay["primary_scientific_result_sha256"]
                != replay["replay_scientific_result_sha256"]
                or replay["digest_authentication"] is None
                or replay["digest_authentication"].get("status") != "accepted"
                or replay["digest_authentication"].get("reasons") != []
                or replay["digest_authentication"].get("scientific_result_sha256")
                != replay["replay_scientific_result_sha256"]
                or replay["digest_authentication"].get("data_sha256")
                != first["digest_authentication"]["data_sha256"]
                or replay["digest_authentication"].get("source_dataset_fingerprint")
                != first["digest_authentication"]["source_dataset_fingerprint"]
                or replay["digest_authentication"].get("checks")
                != first["digest_authentication"]["checks"]
                or replay["digest_authentication"].get("compact_evidence_typed_sha256")
                != first["digest_authentication"]["compact_evidence_typed_sha256"]
            ):
                raise ValueError("accepted replay lacks exact W=1 digest evidence")
            if replay["status"] == "rejected" and not replay["reasons"]:
                raise ValueError("rejected replay must preserve a reason")
        elif replay is not None:
            raise ValueError(f"unexpected replay evidence for {shard['shard_id']}")

        expected_cancellation = stage == "conformance" and shard["shard_ordinal"] == 0
        cancellation = case["cancellation"]
        if expected_cancellation:
            contract = manifest["cancellation_contract"]
            phase = contract["phase_by_dgp"][shard["dgp_id"]]
            cancellation_seeds = _dataset_seeds(
                manifest, shard["dgp_id"], stage, contract["dataset_index"]
            )
            expected_cancel_data_sha = hashlib.sha256(
                generate_dataset_csv(dgp, cancellation_seeds["data"])
            ).hexdigest()
            if cancellation is None or (
                cancellation["dgp_id"] != shard["dgp_id"]
                or cancellation["dataset_index"] != contract["dataset_index"]
                or cancellation["phase"] != phase
                or cancellation["bootstrap_seed"] != cancellation_seeds["bootstrap"]
                or cancellation["data_sha256"] != expected_cancel_data_sha
                or cancellation["maximum_terminal_latency_seconds"]
                != contract["maximum_terminal_latency_seconds"]
                or cancellation["maximum_orphan_processes"]
                != contract["maximum_orphan_processes"]
            ):
                raise ValueError(f"cancellation contract differs for {shard['shard_id']}")
            if cancellation["status"] == "accepted" and (
                cancellation["reasons"]
                or not _finite(cancellation["terminal_latency_seconds"])
                or cancellation["terminal_latency_seconds"] < 0.0
                or cancellation["terminal_latency_seconds"]
                > contract["maximum_terminal_latency_seconds"]
                or cancellation["orphan_processes"] > contract["maximum_orphan_processes"]
            ):
                raise ValueError("accepted cancellation lacks observed terminal evidence")
            if cancellation["status"] == "rejected" and not cancellation["reasons"]:
                raise ValueError("rejected cancellation must preserve a reason")
        elif cancellation is not None:
            raise ValueError(f"unexpected cancellation evidence for {shard['shard_id']}")

        retained = case["retained_full_audit_results"]
        retained_keys = [(row["artifact_kind"], row["dataset_index"]) for row in retained]
        if len(retained_keys) != len(set(retained_keys)):
            raise ValueError(f"retained audit evidence is duplicated: {shard['shard_id']}")
        required_retained = {("dataset", shard["dataset_indices"][0])}
        required_retained.update(
            ("dataset", row["dataset_index"])
            for row in case_receipts
            if row["status"] == "rejected"
        )
        if replay is not None and replay["status"] == "rejected":
            required_retained.add(("worker_replay", replay["dataset_index"]))
        if cancellation is not None and cancellation["status"] == "rejected":
            required_retained.add(
                (f"cancellation_{cancellation['phase']}", cancellation["dataset_index"])
            )
        if not required_retained.issubset(set(retained_keys)):
            raise ValueError(f"required rejected/full audit evidence is missing: {shard['shard_id']}")
        for retained_row in retained:
            relative = Path(retained_row["path"])
            path = (evidence_root / relative).resolve()
            normalized = "/" + relative.as_posix().lstrip("/")
            required_fragment = f"/shards/{shard['shard_id']}/audit/"
            if (
                relative.is_absolute()
                or not path.is_relative_to(evidence_root)
                or required_fragment not in normalized
                or not path.is_file()
                or sha256(path) != retained_row["sha256"]
            ):
                raise ValueError(f"retained audit path/hash differs: {retained_row['path']}")

        evidence_accepted = all(row["status"] == "accepted" for row in case_receipts) and (
            replay is None or replay["status"] == "accepted"
        ) and (cancellation is None or cancellation["status"] == "accepted")
        expected_case_status = (
            "accepted_validation_shard" if evidence_accepted else "rejected_validation_shard"
        )
        if case["status"] != expected_case_status:
            raise ValueError(f"shard status is relabeled: {shard['shard_id']}")

    receipts = [receipt for case in cases for receipt in case["dataset_receipts"]]
    expected_dataset_keys = {
        (shard["dgp_id"], index) for shard in expected for index in shard["dataset_indices"]
    }
    observed_dataset_keys = [(row["dgp_id"], row["dataset_index"]) for row in receipts]
    if len(observed_dataset_keys) != len(set(observed_dataset_keys)) or set(observed_dataset_keys) != expected_dataset_keys:
        raise ValueError("aggregate dataset family is duplicated or incomplete")
    all_accepted = all(case["status"] == "accepted_validation_shard" for case in cases) and all(
        row["status"] == "accepted" for row in receipts
    )

    if not all_accepted:
        return _rejected_aggregate_report(
            manifest, manifest_sha256, stage, cases, receipts
        )

    primary_cells = []
    availability_cells = []
    failure_summary = []
    if stage != "conformance":
        contract = manifest["coverage_contract"]
        ordinary_tail = _tail_from_manifest(contract["ordinary_two_sided_tail"])
        simultaneous_tail = _tail_from_manifest(
            contract["simultaneous_two_sided_tail"]
        )
        availability_tail = _tail_from_manifest(
            contract["availability_one_sided_tail"]
        )
        for dgp in manifest["dgps"]:
            dgp_receipts = [row for row in receipts if row["dgp_id"] == dgp["dgp_id"]]
            for method in METHODS:
                method_rows = [
                    next(item for item in row["methods"] if item["method"] == method)
                    for row in dgp_receipts
                ]
                available_datasets = sum(row["available"] for row in method_rows)
                availability_lower = _certified_clopper_pearson_bounds(
                    available_datasets, len(method_rows), *availability_tail
                )[0]
                oracle_lower = oracle_bonferroni_lower(
                    available_datasets, len(method_rows), contract["availability_cell_count"], contract["family_confidence_level"]
                )
                if not math.isclose(availability_lower, oracle_lower, rel_tol=1.0e-10, abs_tol=1.0e-12):
                    raise ValueError("runner/oracle availability bound differs")
                availability_cells.append(
                    {
                        "dgp_id": dgp["dgp_id"],
                        "method": method,
                        "requested_datasets": len(method_rows),
                        "available_datasets": available_datasets,
                        "bonferroni_one_sided_lower": availability_lower,
                        "final_acceptance": (
                            availability_lower >= contract["minimum_availability_lower_bound"]
                            if stage == "final" else None
                        ),
                    }
                )
                for parameter in dgp["parameter_truth"]:
                    parameter_rows = [
                        next(item for item in row["parameters"] if item["parameter_id"] == parameter["parameter_id"])
                        for row in method_rows
                    ]
                    trials = len(parameter_rows)
                    available = sum(row["available"] for row in parameter_rows)
                    unconditional = sum(row["covered_unconditional"] is True for row in parameter_rows)
                    conditional = sum(row["covered_conditional"] is True for row in parameter_rows)
                    ordinary = _certified_clopper_pearson_bounds(
                        unconditional, trials, *ordinary_tail
                    )
                    simultaneous = _certified_clopper_pearson_bounds(
                        unconditional, trials, *simultaneous_tail
                    )
                    if not _same_bounds(ordinary, oracle_clopper_pearson_two_sided(unconditional, trials, contract["ordinary_confidence_level"])) or not _same_bounds(
                        simultaneous,
                        oracle_bonferroni_two_sided(unconditional, trials, contract["primary_cell_count"], contract["family_confidence_level"]),
                    ):
                        raise ValueError("runner/oracle coverage bounds differ")
                    conditional_bounds = (
                        None
                        if available == 0
                        else _certified_clopper_pearson_bounds(
                            conditional, available, *ordinary_tail
                        )
                    )
                    primary_cells.append(
                        {
                            "dgp_id": dgp["dgp_id"],
                            "parameter_id": parameter["parameter_id"],
                            "method": method,
                            "requested_datasets": trials,
                            "available_datasets": available,
                            "covered_unconditional": unconditional,
                            "covered_conditional": conditional,
                            "unconditional_rate": unconditional / trials,
                            "conditional_rate": None if available == 0 else conditional / available,
                            "ordinary_cp95_lower": ordinary[0],
                            "ordinary_cp95_upper": ordinary[1],
                            "ordinary_cp95_half_width": (ordinary[1] - ordinary[0]) / 2.0,
                            "simultaneous_bonferroni_cp_lower": simultaneous[0],
                            "simultaneous_bonferroni_cp_upper": simultaneous[1],
                            "conditional_cp95_lower": None if conditional_bounds is None else conditional_bounds[0],
                            "conditional_cp95_upper": None if conditional_bounds is None else conditional_bounds[1],
                            "final_acceptance": (
                                (ordinary[1] - ordinary[0]) / 2.0 <= contract["maximum_ordinary_cp95_half_width"]
                                and simultaneous[0] >= contract["simultaneous_acceptance_lower"]
                                and simultaneous[1] <= contract["simultaneous_acceptance_upper"]
                                if stage == "final" else None
                            ),
                        }
                    )
            point_rates = [row["ledger"]["point_failures"] / row["ledger"]["requested_replicates"] for row in dgp_receipts]
            se_rates = [row["ledger"]["standard_error_unavailable"] / row["ledger"]["requested_replicates"] for row in dgp_receipts]
            jackknife_rates = [row["ledger"]["delete_one_failures"] / dgp["sample_size"] for row in dgp_receipts]
            failure_summary.append(
                {
                    "dgp_id": dgp["dgp_id"],
                    "independent_cluster_unit": "generated_dataset",
                    "dataset_count": len(dgp_receipts),
                    "bootstrap_replicates_are_not_binomial_trials": True,
                    "descriptive_quantile_method": manifest["failure_summary_contract"]["descriptive_quantile_method"],
                    "cluster_bootstrap_method": manifest["failure_summary_contract"]["cluster_bootstrap_method"],
                    "point": _failure_metric_summary(
                        manifest, dgp["dgp_id"], stage, "point", point_rates
                    ),
                    "standard_error": _failure_metric_summary(
                        manifest, dgp["dgp_id"], stage, "standard_error", se_rates
                    ),
                    "delete_one": _failure_metric_summary(
                        manifest, dgp["dgp_id"], stage, "delete_one", jackknife_rates
                    ),
                }
            )

    status = {
        "conformance": "conformance_evidence_passed_coverage_not_evaluated_product_qualification_blocked",
        "pilot": "pilot_evidence_assembled_no_qualification_decision_product_qualification_blocked",
    }.get(stage, "planned_stage_not_executable_product_qualification_blocked")
    if not all_accepted:
        status = f"{stage}_evidence_rejected_product_qualification_blocked"
    return {
        "schema_version": 1,
        "kind": REPORT_KIND,
        "stage": stage,
        "status": status,
        "qualification_status": "blocked_not_product_qualification",
        "manifest_sha256": manifest_sha256,
        "binary_sha256": manifest["binary_binding"]["sha256"],
        "shard_count": len(cases),
        "dataset_count": len(receipts),
        "all_predeclared_evidence_accepted": all_accepted,
        "coverage_decision": "not_evaluated_conformance" if stage == "conformance" else "descriptive_pilot_only_no_qualification_decision",
        "primary_cells": primary_cells,
        "availability_cells": availability_cells,
        "failure_summary": failure_summary,
        "rejection_diagnostics": [],
        "qualification_boundary": manifest["qualification_boundary"],
    }


def aggregate_run(
    manifest: Mapping[str, Any], manifest_sha256: str, stage: str, run_directory: Path
) -> dict[str, Any]:
    if stage not in {"conformance", "pilot"}:
        raise ValueError("only executable stages may be aggregated")
    schema = load_json(CASE_SCHEMA)
    Draft202012Validator.check_schema(schema)
    cases = []
    for shard in plan_shards(manifest, stage):
        path = run_directory / "shards" / shard["shard_id"] / "case.json"
        if not path.is_file():
            raise ValueError(f"missing predeclared shard case {path}")
        case = load_json(path)
        Draft202012Validator(schema).validate(case)
        cases.append(case)
    report = aggregate_case_documents(manifest, manifest_sha256, stage, cases)
    report_schema = load_json(REPORT_SCHEMA)
    Draft202012Validator.check_schema(report_schema)
    Draft202012Validator(report_schema).validate(report)
    write_new_json(run_directory / f"{stage}-report.json", report)
    return report


def _safe_run_directory(output_root: Path, run_id: str) -> Path:
    if SAFE_ID_PATTERN.fullmatch(run_id) is None:
        raise ValueError("run id must be one path-safe identifier")
    output_root = output_root.resolve()
    allowed = (ROOT / "validation/results").resolve()
    if not output_root.is_relative_to(allowed):
        raise ValueError("output root must remain under validation/results")
    return output_root / run_id


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--output-root", type=Path, default=DEFAULT_OUTPUT_ROOT)
    subparsers = parser.add_subparsers(dest="command", required=True)
    plan_parser = subparsers.add_parser("plan")
    plan_parser.add_argument("--stage", choices=("conformance", "pilot", "final", "paired_confirmation"), required=True)
    shard_parser = subparsers.add_parser("run-shard")
    shard_parser.add_argument("--stage", choices=("conformance", "pilot"), required=True)
    shard_parser.add_argument("--shard-id", required=True)
    shard_parser.add_argument("--run-id", required=True)
    shard_parser.add_argument("--execute", action="store_true")
    aggregate_parser = subparsers.add_parser("aggregate")
    aggregate_parser.add_argument("--stage", choices=("conformance", "pilot"), required=True)
    aggregate_parser.add_argument("--run-id", required=True)
    aggregate_parser.add_argument("--execute", action="store_true")
    args = parser.parse_args()

    require_execution_bindings = args.command in {"run-shard", "aggregate"} and args.execute
    manifest, manifest_sha = validate_manifest(
        args.manifest.resolve(),
        require_binary=require_execution_bindings,
        require_scientific_runtime=require_execution_bindings,
    )
    if args.command == "plan":
        shards = plan_shards(manifest, args.stage)
        print(json.dumps({
            "status": "immutable_plan_only_no_workloads_executed",
            "stage": args.stage,
            "manifest_sha256": manifest_sha,
            "executable": stage_by_name(manifest, args.stage)["executable"],
            "shards": shards,
            "qualification_status": "blocked_not_product_qualification",
        }, indent=2))
        return 0

    if not args.execute:
        print(json.dumps({
            "status": "dry_run_no_workloads_or_outputs",
            "command": args.command,
            "stage": args.stage,
            "manifest_sha256": manifest_sha,
            "qualification_status": "blocked_not_product_qualification",
        }, indent=2))
        return 0
    run_directory = _safe_run_directory(args.output_root, args.run_id)
    if args.command == "run-shard":
        matches = [row for row in plan_shards(manifest, args.stage) if row["shard_id"] == args.shard_id]
        if len(matches) != 1:
            parser.error("shard id is not in the immutable plan")
        case = run_shard(manifest, manifest_sha, matches[0], run_directory)
        print(json.dumps({"status": case["status"], "shard_id": args.shard_id}, indent=2))
        return 0 if case["status"] == "accepted_validation_shard" else 2
    report = aggregate_run(manifest, manifest_sha, args.stage, run_directory)
    print(json.dumps({"status": report["status"], "run_directory": str(run_directory)}, indent=2))
    return 0 if report["all_predeclared_evidence_accepted"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
