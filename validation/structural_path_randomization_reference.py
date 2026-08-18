#!/usr/bin/env python3
"""Independent evidence for structural-path randomization v1.

This validation-only program never invokes Cargo and never changes product
state. It executes the already-built coordinated release CLI on the bundled
corporate-reputation fixture, reconstructs the fixed-score multi-predictor
Freedman-Lane test in Python, and asks base R to recompute every permuted focal
coefficient from exported construct scores and an explicit index matrix.

The deterministic index implementation below is deliberately independent of
QuickPLS code. It follows the frozen public stream contract: SHA-256 domain
separation, ChaCha20 with a zero stream and counter, rand 0.9 Canon inclusive
integer sampling, and descending Fisher-Yates swaps.

R and NumPy are validation-only dependencies. QuickPLS remains fully offline
and does not bundle or invoke either runtime in production.
"""

from __future__ import annotations

import csv
import hashlib
import json
import math
import os
import shutil
import struct
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Sequence

import numpy as np

from r_runtime import find_rscript_optional


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
RESULTS = VALIDATION / "results"
ARTIFACTS = RESULTS / "structural_path_randomization_reference"
REPORT = RESULTS / "structural_path_randomization_reference_report.json"
R_SCRIPT = VALIDATION / "structural_path_randomization_reference.R"
SOURCE_RECIPE = VALIDATION / "fixtures" / "corporate_reputation.recipe.json"
SOURCE_DATA = VALIDATION / "fixtures" / "corporate_reputation.csv"

FEATURE_ID = "qpls3.inference.structural_path_randomization"
METHOD_VERSION = "freedman_lane_permutation_v1"
CATALOGUE_SNAPSHOT_DATE = "2026-08-12"
PLAN_OPERATION = "pls_pm_freedman_lane_v1"
INDEX_STREAM_VERSION = "sha256_chacha20_rand_0_9_fisher_yates_v1"
# These are independent wire layers. In particular, the PLS v3 payload kind,
# recipe schema v3, and project archive v5 do not change the result envelope's
# frozen schema v1 identity.
REFERENCE_REPORT_SCHEMA_VERSION = 1
ANALYSIS_RESULT_SCHEMA_VERSION = 1
SOURCE_RECIPE_SCHEMA_VERSION = 2
ANALYSIS_RECIPE_SCHEMA_VERSION = 3
PLS_INFERENCE_PAYLOAD_KIND = "pls_pm_v3"
PROJECT_ARCHIVE_SCHEMA_VERSION = 5
EXPECTED_PROVENANCE_METHOD_VERSION = (
    "pls_pm_v1+pls_mediation_v1+pls_assessment_v8+freedman_lane_permutation_v1"
)
TARGET = "structural_path_randomization_reference_v1"
SEED = 20_260_718
PERMUTATIONS = 999
WORKERS = (1, 4)
EXPECTED_PATHS = (
    ("comp", "satisfaction"),
    ("like", "satisfaction"),
    ("satisfaction", "loyalty"),
)
FROZEN_INDEX_VECTORS = {
    0: [1, 8, 2, 9, 7, 6, 3, 10, 0, 11, 5, 4],
    98: [0, 5, 1, 10, 6, 2, 4, 7, 9, 11, 3, 8],
}
REFERENCE_RELATIVE_TOLERANCE = 5e-10
R_COEFFICIENT_RELATIVE_TOLERANCE = 8e-10

CALIBRATION_REPLICATIONS = 240
CALIBRATION_PERMUTATIONS = 199
CALIBRATION_CASES = 80
CALIBRATION_ALPHA = 0.05
CALIBRATION_SIGNAL = 0.65
CALIBRATION_NUISANCE = 0.80
CALIBRATION_NOISE_DISTRIBUTION = "homoscedastic_standard_gaussian"
CALIBRATION_VARIANCE_MODEL = "constant"
CALIBRATION_NULL_Z = 3.5
CALIBRATION_POWER_MINIMUM = 0.80
CALIBRATION_DATA_SEED = 20_260_813
CALIBRATION_PERMUTATION_SEED = 20_260_814

SEED_DOMAIN = b"QuickPLS indexed resampling v1\0"
UINT32_MASK = (1 << 32) - 1

REFERENCE_EVIDENCE_SOURCE_PATHS = (
    "validation/structural_path_randomization_reference.py",
    "validation/structural_path_randomization_reference.R",
    "validation/r_runtime.py",
    "validation/test_structural_path_randomization_reference.py",
    "validation/structural_path_randomization_boundary_gate.py",
    "validation/structural_path_randomization_frontend_gate.py",
    "validation/structural_path_randomization_method_promotion_audit.py",
    "validation/test_structural_path_randomization_method_promotion_audit.py",
    "validation/inference_publication_audit.py",
    "validation/inference_method_promotion_audit.py",
)


def _configured_cli() -> Path:
    configured = os.environ.get("QUICKPLS_CLI_PATH", "").strip()
    if configured:
        return Path(configured).resolve()
    return (ROOT / "target" / "release" / "qpls.exe").resolve()


CLI = _configured_cli()


def release_cli_freshness_sources(root: Path = ROOT) -> tuple[Path, ...]:
    """Return every source/manifest family that materially feeds the tested CLI."""

    relative_roots = (
        "qpls-core",
        "qpls-data",
        "qpls-assessment",
        "qpls-estimation",
        "qpls-resampling",
        "qpls-runner",
        "qpls-project",
        "qpls-cli",
    )
    sources: list[Path] = [root / "Cargo.toml", root / "Cargo.lock"]
    for crate in relative_roots:
        crate_root = root / "crates" / crate
        sources.append(crate_root / "Cargo.toml")
        sources.extend(sorted(crate_root.rglob("*.rs"), key=lambda path: path.as_posix()))
    unique = tuple(dict.fromkeys(path.resolve() for path in sources))
    if not unique or any(not path.is_file() for path in unique):
        missing = [str(path) for path in unique if not path.is_file()]
        raise RuntimeError(f"release CLI freshness source set is incomplete: {missing}")
    return unique


def reference_evidence_sources(root: Path = ROOT) -> tuple[Path, ...]:
    sources = tuple((root / relative).resolve() for relative in REFERENCE_EVIDENCE_SOURCE_PATHS)
    missing = [str(path) for path in sources if not path.is_file()]
    if missing:
        raise RuntimeError(f"reference/audit source set is incomplete: {missing}")
    if sources[0] != Path(__file__).resolve():
        raise RuntimeError("reference source manifest is not anchored to the executing Python file")
    return sources


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def tested_file(path: Path) -> dict[str, Any]:
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "size": path.stat().st_size,
        "sha256": sha256_file(path),
        "modified_utc": datetime.fromtimestamp(
            path.stat().st_mtime, timezone.utc
        ).isoformat(),
    }


def require_release_cli() -> None:
    expected = (ROOT / "target" / "release" / "qpls.exe").resolve()
    if CLI != expected:
        raise RuntimeError(
            f"qualification must execute the coordinated release CLI {expected}; "
            f"received {CLI}"
        )
    if not CLI.is_file():
        raise RuntimeError(
            "the coordinated release CLI is missing; build it in the release gate "
            "before running this reference (this script never invokes Cargo)"
        )
    critical_sources = release_cli_freshness_sources()
    stale = [path for path in critical_sources if path.stat().st_mtime > CLI.stat().st_mtime]
    if stale:
        names = ", ".join(path.relative_to(ROOT).as_posix() for path in stale)
        raise RuntimeError(f"release CLI is older than required product sources: {names}")


def canonical_parameter(source: str, target: str) -> str:
    return json.dumps(["path", [source, target]], separators=(",", ":"))


def expected_parameter_manifest() -> list[str]:
    return [canonical_parameter(source, target) for source, target in EXPECTED_PATHS]


def operation_for_parameter(parameter: str) -> str:
    return f"{PLAN_OPERATION}:{parameter}"


def _rotate_left(value: int, count: int) -> int:
    return ((value << count) | (value >> (32 - count))) & UINT32_MASK


def _quarter_round(state: list[int], a: int, b: int, c: int, d: int) -> None:
    state[a] = (state[a] + state[b]) & UINT32_MASK
    state[d] = _rotate_left(state[d] ^ state[a], 16)
    state[c] = (state[c] + state[d]) & UINT32_MASK
    state[b] = _rotate_left(state[b] ^ state[c], 12)
    state[a] = (state[a] + state[b]) & UINT32_MASK
    state[d] = _rotate_left(state[d] ^ state[a], 8)
    state[c] = (state[c] + state[d]) & UINT32_MASK
    state[b] = _rotate_left(state[b] ^ state[c], 7)


def chacha20_block(seed: bytes, counter: int = 0, stream: int = 0) -> bytes:
    """Return one rand_chacha-compatible 64-byte ChaCha20 block."""

    if len(seed) != 32:
        raise ValueError("ChaCha20 seed must contain exactly 32 bytes")
    if not (0 <= counter < (1 << 64)) or not (0 <= stream < (1 << 64)):
        raise ValueError("counter and stream must be unsigned 64-bit integers")
    state = [
        0x61707865,
        0x3320646E,
        0x79622D32,
        0x6B206574,
        *struct.unpack("<8I", seed),
        counter & UINT32_MASK,
        (counter >> 32) & UINT32_MASK,
        stream & UINT32_MASK,
        (stream >> 32) & UINT32_MASK,
    ]
    working = state.copy()
    for _ in range(10):
        _quarter_round(working, 0, 4, 8, 12)
        _quarter_round(working, 1, 5, 9, 13)
        _quarter_round(working, 2, 6, 10, 14)
        _quarter_round(working, 3, 7, 11, 15)
        _quarter_round(working, 0, 5, 10, 15)
        _quarter_round(working, 1, 6, 11, 12)
        _quarter_round(working, 2, 7, 8, 13)
        _quarter_round(working, 3, 4, 9, 14)
    output = [(left + right) & UINT32_MASK for left, right in zip(working, state)]
    return struct.pack("<16I", *output)


class ChaCha20U32:
    """Minimal sequential u32 stream matching rand_chacha 0.9 ChaCha20Rng."""

    def __init__(self, seed: bytes) -> None:
        if len(seed) != 32:
            raise ValueError("ChaCha20 seed must contain exactly 32 bytes")
        self._seed = seed
        self._counter = 0
        self._words: list[int] = []

    def next_u32(self) -> int:
        if not self._words:
            block = chacha20_block(self._seed, self._counter, 0)
            self._counter = (self._counter + 1) & ((1 << 64) - 1)
            self._words = list(struct.unpack("<16I", block))
        return self._words.pop(0)


def derived_index_seed(master_seed: int, operation: str, permutation_index: int) -> bytes:
    operation_bytes = operation.encode("utf-8")
    if not (0 <= master_seed < (1 << 64)):
        raise ValueError("master seed must be an unsigned 64-bit integer")
    if not (0 <= permutation_index < (1 << 32)):
        raise ValueError("permutation index must be an unsigned 32-bit integer")
    digest = hashlib.sha256()
    digest.update(SEED_DOMAIN)
    digest.update(struct.pack("<Q", master_seed))
    digest.update(struct.pack("<Q", len(operation_bytes)))
    digest.update(operation_bytes)
    digest.update(struct.pack("<I", permutation_index))
    return digest.digest()


def canon_inclusive_u32(rng: ChaCha20U32, upper: int) -> int:
    """rand 0.9 default-feature Canon sampler for ``0..=upper`` (u32)."""

    if not (0 <= upper <= UINT32_MASK - 1):
        raise ValueError("inclusive upper bound is outside the supported u32 range")
    range_size = upper + 1
    product = rng.next_u32() * range_size
    result = (product >> 32) & UINT32_MASK
    low_order = product & UINT32_MASK
    wrapped_negative_range = (-range_size) & UINT32_MASK
    if low_order > wrapped_negative_range:
        second_product = rng.next_u32() * range_size
        new_high_order = (second_product >> 32) & UINT32_MASK
        if low_order + new_high_order > UINT32_MASK:
            result = (result + 1) & UINT32_MASK
    return result


def permutation_indices(
    case_count: int,
    master_seed: int,
    operation: str,
    permutation_index: int,
) -> list[int]:
    if not (2 <= case_count <= UINT32_MASK):
        raise ValueError("case count must be between 2 and u32::MAX")
    rng = ChaCha20U32(derived_index_seed(master_seed, operation, permutation_index))
    indices = list(range(case_count))
    for upper in range(case_count - 1, 0, -1):
        swap_index = canon_inclusive_u32(rng, upper)
        indices[upper], indices[swap_index] = indices[swap_index], indices[upper]
    return indices


def migrated_recipe_payload(workers: int, permutations: int = PERMUTATIONS) -> dict[str, Any]:
    if workers not in WORKERS:
        raise ValueError(f"workers must be one of {WORKERS}")
    source = json.loads(SOURCE_RECIPE.read_text(encoding="utf-8"))
    if source.get("schema_version") != SOURCE_RECIPE_SCHEMA_VERSION:
        raise ValueError("corporate-reputation source recipe must remain schema v2")
    model = json.loads(json.dumps(source["model"]))
    model.setdefault("controls", [])
    model.setdefault("higher_order_constructs", [])
    model.setdefault("interactions", [])
    settings = json.loads(json.dumps(source["settings"]))
    settings.update(
        {
            "bootstrap_samples": 0,
            "studentized_inner_samples": 0,
            "permutation_samples": permutations,
            "seed": SEED,
            "workers": workers,
            "confidence_level": 0.95,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
            "case_weight_column": None,
        }
    )
    return {
        "schema_version": ANALYSIS_RECIPE_SCHEMA_VERSION,
        "id": f"00000000-0000-0000-0000-{32_000 + workers:012d}",
        "created_at": "2026-08-13T00:00:00Z",
        "dataset_fingerprint": source["dataset_fingerprint"],
        "model": model,
        "settings": settings,
        "method_config": {"kind": "pls_permutation"},
        "metadata": {
            "fixture": "corporate_reputation",
            "reference": TARGET,
            "migration": "deterministic_schema_v2_to_v3_validation_copy",
            "source_recipe_id": source["id"],
        },
    }


def _run(command: Sequence[str], *, timeout: int = 900) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        list(command),
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )


def _require_success(completed: subprocess.CompletedProcess[str], label: str) -> None:
    if completed.returncode != 0:
        raise RuntimeError(
            f"{label} failed with exit code {completed.returncode}\n"
            f"stdout={completed.stdout[-4000:]}\nstderr={completed.stderr[-4000:]}"
        )


def run_release_cli(
    workers: int, artifact_directory: Path
) -> tuple[dict[str, Any], list[str], dict[str, Any]]:
    recipe_path = artifact_directory / f"workers_{workers}.recipe.json"
    output_path = artifact_directory / f"workers_{workers}.result.json"
    recipe = migrated_recipe_payload(workers)
    recipe_path.write_text(
        json.dumps(recipe, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    command = [
        str(CLI),
        "run",
        str(recipe_path),
        "--data",
        str(SOURCE_DATA),
        "--output",
        str(output_path),
        "--permutation-samples",
        str(PERMUTATIONS),
        "--workers",
        str(workers),
    ]
    completed = _run(command)
    _require_success(completed, f"release CLI workers={workers}")
    if not output_path.is_file():
        raise RuntimeError(f"release CLI did not create {output_path}")
    return json.loads(output_path.read_text(encoding="utf-8-sig")), command, recipe


def _same_float(left: float, right: float) -> bool:
    return struct.pack("<d", float(left)) == struct.pack("<d", float(right))


def _parameter_path(parameter: str) -> tuple[str, str]:
    decoded = json.loads(parameter)
    if (
        not isinstance(decoded, list)
        or len(decoded) != 2
        or decoded[0] != "path"
        or not isinstance(decoded[1], list)
        or len(decoded[1]) != 2
        or not all(isinstance(value, str) for value in decoded[1])
    ):
        raise ValueError(f"invalid path parameter identity: {parameter}")
    return decoded[1][0], decoded[1][1]


def validate_cli_contract(
    result: dict[str, Any], workers: int, recipe: dict[str, Any]
) -> dict[str, Any]:
    if result.get("schema_version") != ANALYSIS_RESULT_SCHEMA_VERSION:
        raise ValueError("release result envelope is not schema v1")
    if result.get("status") != "completed":
        raise ValueError(f"release CLI workers={workers} did not complete")
    provenance = result.get("provenance") or {}
    if provenance.get("method") != "pls_pm":
        raise ValueError("unexpected provenance method")
    if provenance.get("recipe_id") != recipe.get("id"):
        raise ValueError("release result provenance is not bound to the generated recipe ID")
    if provenance.get("dataset_fingerprint") != recipe.get("dataset_fingerprint"):
        raise ValueError("release result provenance is not bound to the recipe dataset fingerprint")
    if provenance.get("seed") != SEED:
        raise ValueError("unexpected provenance seed")
    settings = provenance.get("settings") or {}
    if settings.get("workers") != workers or settings.get("permutation_samples") != PERMUTATIONS:
        raise ValueError("provenance settings do not bind the requested worker/permutation plan")
    if provenance.get("method_version") != EXPECTED_PROVENANCE_METHOD_VERSION:
        raise ValueError("provenance method-version chain is not the frozen release identity")
    diagnostics = result.get("diagnostics") or []
    if any(str(row.get("level", "")).lower() == "error" for row in diagnostics):
        raise ValueError("completed release result contains an error diagnostic")

    payload = result.get("payload") or {}
    if payload.get("kind") != PLS_INFERENCE_PAYLOAD_KIND:
        raise ValueError("release payload is not pls_pm_v3")
    if payload.get("bootstrap") is not None:
        raise ValueError("permutation-only recipe unexpectedly emitted bootstrap output")
    estimation = payload.get("estimation") or {}
    permutation = payload.get("permutation") or {}
    if permutation.get("method_version") != METHOD_VERSION:
        raise ValueError("unexpected permutation method version")
    plan = permutation.get("plan") or {}
    if plan != {
        "permutations": PERMUTATIONS,
        "master_seed": SEED,
        "operation": PLAN_OPERATION,
    }:
        raise ValueError(f"unexpected permutation plan: {plan}")

    parameters = permutation.get("parameters") or []
    expected_parameters = expected_parameter_manifest()
    if [row.get("parameter") for row in parameters] != expected_parameters:
        raise ValueError("permutation parameter manifest is not the frozen ordered path manifest")
    paths = estimation.get("paths") or []
    if [(row.get("source"), row.get("target")) for row in paths] != list(EXPECTED_PATHS):
        raise ValueError("base path estimates are not in the frozen recipe order")
    path_map = {
        canonical_parameter(row["source"], row["target"]): row["coefficient"] for row in paths
    }
    for row in parameters:
        parameter = row["parameter"]
        if row.get("permutations") != PERMUTATIONS:
            raise ValueError(f"wrong permutation count for {parameter}")
        if not isinstance(row.get("exceedances"), int) or not (
            0 <= row["exceedances"] <= PERMUTATIONS
        ):
            raise ValueError(f"invalid exceedance count for {parameter}")
        expected_p = (row["exceedances"] + 1.0) / (PERMUTATIONS + 1.0)
        if not _same_float(row.get("p_value_two_sided"), expected_p):
            raise ValueError(f"+1 permutation probability identity failed for {parameter}")
        if parameter not in path_map or not _same_float(row.get("original"), path_map[parameter]):
            raise ValueError(f"stored original is not the exact base path estimate for {parameter}")

    scores = estimation.get("construct_scores") or {}
    expected_constructs = [row["id"] for row in migrated_recipe_payload(workers)["model"]["constructs"]]
    if list(scores.keys()) != sorted(expected_constructs):
        # Rust BTreeMap serialization is code-point sorted; enforce it explicitly.
        raise ValueError("construct-score keys are not the exact canonical construct set/order")
    lengths = {len(values) for values in scores.values()}
    if lengths != {12} or estimation.get("used_observations") != 12:
        raise ValueError("corporate reference must retain exactly 12 complete score rows")
    if estimation.get("omitted_observations") != 0:
        raise ValueError("corporate reference unexpectedly omitted observations")
    return {
        "workers": workers,
        "result_id": result.get("id"),
        "result_schema_version": result.get("schema_version"),
        "recipe_id": provenance.get("recipe_id"),
        "dataset_fingerprint": provenance.get("dataset_fingerprint"),
        "method_version": provenance.get("method_version"),
        "parameters": parameters,
        "construct_score_rows": next(iter(lengths)),
    }


def assert_recursive_exact(left: Any, right: Any, path: str = "$root") -> None:
    """Require identical JSON shape/types and f64 bit patterns recursively."""

    if type(left) is not type(right):
        raise ValueError(
            f"exact comparison type differs at {path}: {type(left).__name__} != {type(right).__name__}"
        )
    if isinstance(left, dict):
        if list(left) != list(right):
            raise ValueError(f"exact comparison key/order differs at {path}")
        for key in left:
            assert_recursive_exact(left[key], right[key], f"{path}.{key}")
        return
    if isinstance(left, list):
        if len(left) != len(right):
            raise ValueError(f"exact comparison length differs at {path}")
        for index, (left_item, right_item) in enumerate(zip(left, right)):
            assert_recursive_exact(left_item, right_item, f"{path}[{index}]")
        return
    if isinstance(left, float):
        if struct.pack("<d", left) != struct.pack("<d", right):
            raise ValueError(f"exact comparison f64 bits differ at {path}")
        return
    if left != right:
        raise ValueError(f"exact comparison value differs at {path}")


def assert_worker_invariance(serial: dict[str, Any], parallel: dict[str, Any]) -> None:
    try:
        assert_recursive_exact(serial["payload"], parallel["payload"], "$.payload")
    except ValueError as error:
        raise ValueError(
            "PLS point/permutation payload differs between workers 1 and 4: " + str(error)
        ) from error
    serial_provenance = json.loads(json.dumps(serial["provenance"]))
    parallel_provenance = json.loads(json.dumps(parallel["provenance"]))
    for provenance in (serial_provenance, parallel_provenance):
        provenance.pop("started_at", None)
        provenance.pop("completed_at", None)
        provenance.pop("recipe_id", None)
        provenance.get("settings", {}).pop("workers", None)
    try:
        assert_recursive_exact(
            serial_provenance, parallel_provenance, "$.normalized_provenance"
        )
    except ValueError as error:
        raise ValueError(
            "normalized provenance differs between workers 1 and 4: " + str(error)
        ) from error


def ols_with_intercept(
    predictors: Sequence[Sequence[float]], outcome: Sequence[float]
) -> tuple[np.ndarray, np.ndarray]:
    y = np.asarray(outcome, dtype=np.float64)
    columns = [np.ones(y.size, dtype=np.float64)]
    columns.extend(np.asarray(column, dtype=np.float64) for column in predictors)
    if any(column.shape != y.shape for column in columns):
        raise ValueError("predictor/outcome dimensions differ")
    design = np.column_stack(columns)
    coefficients, _, rank, _ = np.linalg.lstsq(design, y, rcond=None)
    if rank != design.shape[1] or not np.all(np.isfinite(coefficients)):
        raise ValueError("reference regression design is rank deficient or nonfinite")
    return coefficients[1:], design @ coefficients


def prepare_freedman_lane(
    predictors: Sequence[Sequence[float]], outcome: Sequence[float], focal_index: int
) -> tuple[float, np.ndarray, np.ndarray]:
    if not (0 <= focal_index < len(predictors)):
        raise ValueError("focal predictor index is outside the design")
    full_coefficients, _ = ols_with_intercept(predictors, outcome)
    nuisance = [column for index, column in enumerate(predictors) if index != focal_index]
    _, fitted_nuisance = ols_with_intercept(nuisance, outcome)
    residuals = np.asarray(outcome, dtype=np.float64) - fitted_nuisance
    return float(full_coefficients[focal_index]), fitted_nuisance, residuals


def freedman_lane_focal_coefficient(
    predictors: Sequence[Sequence[float]],
    outcome: Sequence[float],
    focal_index: int,
    indices: Sequence[int],
) -> float:
    if sorted(indices) != list(range(len(outcome))):
        raise ValueError("permutation indices must be an exact zero-based bijection")
    _, fitted_nuisance, residuals = prepare_freedman_lane(predictors, outcome, focal_index)
    permuted_outcome = fitted_nuisance + residuals[np.asarray(indices, dtype=np.int64)]
    coefficients, _ = ols_with_intercept(predictors, permuted_outcome)
    return float(coefficients[focal_index])


def _close(left: float, right: float, tolerance: float) -> bool:
    return abs(left - right) <= tolerance * max(abs(left), abs(right), 1.0)


@dataclass(frozen=True)
class ReferenceParameter:
    ordinal: int
    parameter: str
    source: str
    target: str
    predictor_ids: tuple[str, ...]
    focal_index: int
    stored_original: float
    reproduced_original: float
    stored_exceedances: int
    reference_exceedances: int
    stored_p_value: float
    reference_p_value: float
    coefficient_sha256: str
    minimum_threshold_margin: float


def reconstruct_python_reference(
    result: dict[str, Any], recipe: dict[str, Any]
) -> tuple[list[ReferenceParameter], dict[str, list[list[int]]], dict[str, list[float]]]:
    estimation = result["payload"]["estimation"]
    permutation = result["payload"]["permutation"]
    score_map = estimation["construct_scores"]
    stored = {row["parameter"]: row for row in permutation["parameters"]}
    all_indices: dict[str, list[list[int]]] = {}
    all_coefficients: dict[str, list[float]] = {}
    references: list[ReferenceParameter] = []

    for ordinal, (source, target) in enumerate(EXPECTED_PATHS):
        parameter = canonical_parameter(source, target)
        incoming = [
            path["source"] for path in recipe["model"]["paths"] if path["target"] == target
        ]
        focal_index = incoming.index(source)
        predictors = [score_map[predictor] for predictor in incoming]
        outcome = score_map[target]
        reproduced, fitted_nuisance, residuals = prepare_freedman_lane(
            predictors, outcome, focal_index
        )
        row = stored[parameter]
        if not _close(reproduced, row["original"], REFERENCE_RELATIVE_TOLERANCE):
            raise ValueError(f"Python full regression does not reproduce {parameter}")
        operation = operation_for_parameter(parameter)
        parameter_indices: list[list[int]] = []
        parameter_coefficients: list[float] = []
        for permutation_index in range(PERMUTATIONS):
            indices = permutation_indices(12, SEED, operation, permutation_index)
            parameter_indices.append(indices)
            permuted_outcome = fitted_nuisance + residuals[np.asarray(indices, dtype=np.int64)]
            coefficients, _ = ols_with_intercept(predictors, permuted_outcome)
            parameter_coefficients.append(float(coefficients[focal_index]))
        exceedances = sum(
            abs(value) >= abs(row["original"]) for value in parameter_coefficients
        )
        probability = (exceedances + 1.0) / (PERMUTATIONS + 1.0)
        if exceedances != row["exceedances"] or not _same_float(
            probability, row["p_value_two_sided"]
        ):
            raise ValueError(f"Python Freedman-Lane arithmetic differs for {parameter}")
        packed = b"".join(struct.pack("<d", value) for value in parameter_coefficients)
        margins = [
            abs(abs(value) - abs(row["original"])) for value in parameter_coefficients
        ]
        references.append(
            ReferenceParameter(
                ordinal=ordinal,
                parameter=parameter,
                source=source,
                target=target,
                predictor_ids=tuple(incoming),
                focal_index=focal_index,
                stored_original=float(row["original"]),
                reproduced_original=reproduced,
                stored_exceedances=int(row["exceedances"]),
                reference_exceedances=exceedances,
                stored_p_value=float(row["p_value_two_sided"]),
                reference_p_value=probability,
                coefficient_sha256=hashlib.sha256(packed).hexdigest(),
                minimum_threshold_margin=min(margins),
            )
        )
        all_indices[parameter] = parameter_indices
        all_coefficients[parameter] = parameter_coefficients
    return references, all_indices, all_coefficients


def write_r_inputs(
    artifact_directory: Path,
    result: dict[str, Any],
    references: Sequence[ReferenceParameter],
    all_indices: dict[str, list[list[int]]],
) -> dict[str, Path]:
    score_path = artifact_directory / "construct_scores.csv"
    design_path = artifact_directory / "design_manifest.csv"
    index_path = artifact_directory / "permutation_indices.csv"
    output_path = artifact_directory / "r_coefficients.csv"
    scores = result["payload"]["estimation"]["construct_scores"]
    construct_ids = sorted(scores)
    with score_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow(["row_index", *construct_ids])
        for row_index in range(12):
            writer.writerow([row_index, *(repr(float(scores[name][row_index])) for name in construct_ids)])
    with design_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow(
            [
                "parameter_ordinal",
                "parameter",
                "source",
                "target",
                "predictor_ordinal",
                "predictor",
                "is_focal",
                "stored_original",
                "stored_exceedances",
                "stored_p_value",
                "permutations",
            ]
        )
        for reference in references:
            for predictor_ordinal, predictor in enumerate(reference.predictor_ids):
                writer.writerow(
                    [
                        reference.ordinal,
                        reference.parameter,
                        reference.source,
                        reference.target,
                        predictor_ordinal,
                        predictor,
                        int(predictor_ordinal == reference.focal_index),
                        repr(reference.stored_original),
                        reference.stored_exceedances,
                        repr(reference.stored_p_value),
                        PERMUTATIONS,
                    ]
                )
    with index_path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow(
            [
                "parameter_ordinal",
                "permutation_index",
                "row_index",
                "selected_residual_index",
            ]
        )
        for reference in references:
            for permutation_index, indices in enumerate(all_indices[reference.parameter]):
                for row_index, selected in enumerate(indices):
                    writer.writerow([reference.ordinal, permutation_index, row_index, selected])
    return {
        "scores": score_path,
        "design": design_path,
        "indices": index_path,
        "output": output_path,
    }


def run_r_reference(
    paths: dict[str, Path],
    references: Sequence[ReferenceParameter],
    python_coefficients: dict[str, list[float]],
) -> tuple[dict[str, Any], str]:
    r_runtime = find_rscript_optional()
    if r_runtime is None:
        raise RuntimeError(
            "Rscript was not found; set QPLS_RSCRIPT to run the required independent R reference"
        )
    rscript, r_version = r_runtime
    command = [
        rscript,
        "--vanilla",
        str(R_SCRIPT),
        str(paths["scores"]),
        str(paths["design"]),
        str(paths["indices"]),
        str(paths["output"]),
    ]
    completed = _run(command)
    _require_success(completed, "independent R structural randomization reference")
    with paths["output"].open(newline="", encoding="utf-8-sig") as handle:
        rows = list(csv.DictReader(handle))
    expected_rows = len(references) * PERMUTATIONS
    if len(rows) != expected_rows:
        raise ValueError(f"R returned {len(rows)} coefficients; expected {expected_rows}")
    by_parameter: dict[str, list[tuple[int, float]]] = {}
    for row in rows:
        by_parameter.setdefault(row["parameter"], []).append(
            (int(row["permutation_index"]), float(row["coefficient"]))
        )
    comparisons: list[dict[str, Any]] = []
    for reference in references:
        ordered = sorted(by_parameter.get(reference.parameter, []))
        if [index for index, _ in ordered] != list(range(PERMUTATIONS)):
            raise ValueError(f"R coefficient order is incomplete for {reference.parameter}")
        r_values = [value for _, value in ordered]
        parameter_rows = [row for row in rows if row["parameter"] == reference.parameter]
        r_originals = {float(row["reproduced_original"]) for row in parameter_rows}
        r_exceedance_values = {int(row["exceedances"]) for row in parameter_rows}
        r_probability_values = {float(row["p_value_two_sided"]) for row in parameter_rows}
        if len(r_originals) != 1 or not _close(
            next(iter(r_originals)),
            reference.stored_original,
            R_COEFFICIENT_RELATIVE_TOLERANCE,
        ):
            raise ValueError(f"R full regression does not reproduce {reference.parameter}")
        if r_exceedance_values != {reference.stored_exceedances}:
            raise ValueError(f"R-authored exceedance count differs for {reference.parameter}")
        if len(r_probability_values) != 1 or not _same_float(
            next(iter(r_probability_values)), reference.stored_p_value
        ):
            raise ValueError(f"R-authored +1 probability differs for {reference.parameter}")
        python_values = python_coefficients[reference.parameter]
        differences = [abs(left - right) for left, right in zip(r_values, python_values)]
        scaled = [
            difference / max(abs(left), abs(right), 1.0)
            for difference, left, right in zip(differences, r_values, python_values)
        ]
        if max(scaled) > R_COEFFICIENT_RELATIVE_TOLERANCE:
            raise ValueError(f"R/Python coefficient comparison failed for {reference.parameter}")
        r_exceedances = sum(
            abs(value) >= abs(reference.stored_original) for value in r_values
        )
        r_probability = (r_exceedances + 1.0) / (PERMUTATIONS + 1.0)
        if r_exceedances != reference.stored_exceedances or not _same_float(
            r_probability, reference.stored_p_value
        ):
            raise ValueError(f"R exceedance/+1 arithmetic differs for {reference.parameter}")
        comparisons.append(
            {
                "parameter": reference.parameter,
                "coefficient_count": len(r_values),
                "reproduced_original": next(iter(r_originals)),
                "maximum_absolute_difference_from_python": max(differences),
                "maximum_relative_difference_from_python": max(scaled),
                "exceedances": r_exceedances,
                "p_value_two_sided": r_probability,
            }
        )
    return {
        "command": command,
        "comparisons": comparisons,
        "output": tested_file(paths["output"]),
    }, r_version


def wilson_interval(successes: int, total: int, z: float = 1.959963984540054) -> tuple[float, float]:
    if not (0 <= successes <= total) or total <= 0:
        raise ValueError("invalid binomial counts")
    rate = successes / total
    denominator = 1.0 + z * z / total
    centre = (rate + z * z / (2.0 * total)) / denominator
    radius = (
        z
        * math.sqrt(rate * (1.0 - rate) / total + z * z / (4.0 * total * total))
        / denominator
    )
    return centre - radius, centre + radius


def _fixed_score_randomization_p(
    focal: np.ndarray,
    nuisance: np.ndarray,
    outcome: np.ndarray,
    indices: Sequence[Sequence[int]],
) -> float:
    predictors = [focal, nuisance]
    original, fitted_nuisance, residuals = prepare_freedman_lane(predictors, outcome, 0)
    exceedances = 0
    for row_indices in indices:
        permuted_outcome = fitted_nuisance + residuals[np.asarray(row_indices, dtype=np.int64)]
        coefficients, _ = ols_with_intercept(predictors, permuted_outcome)
        exceedances += abs(float(coefficients[0])) >= abs(original)
    return (exceedances + 1.0) / (len(indices) + 1.0)


def calibration_study(
    *,
    replications: int = CALIBRATION_REPLICATIONS,
    permutations: int = CALIBRATION_PERMUTATIONS,
    cases: int = CALIBRATION_CASES,
    signal: float = CALIBRATION_SIGNAL,
) -> dict[str, Any]:
    if permutations < 99 or replications < 20 or cases < 20:
        raise ValueError("calibration dimensions are below the frozen evidence minima")
    rng = np.random.Generator(np.random.PCG64(CALIBRATION_DATA_SEED))
    null_probabilities: list[float] = []
    signal_probabilities: list[float] = []
    for replicate in range(replications):
        nuisance = rng.normal(size=cases)
        independent = rng.normal(size=cases)
        focal = 0.35 * nuisance + math.sqrt(1.0 - 0.35**2) * independent
        noise = rng.normal(size=cases)
        null_outcome = CALIBRATION_NUISANCE * nuisance + noise
        signal_outcome = null_outcome + signal * focal
        operation = f"structural_path_randomization_calibration_v1:{replicate}"
        matrix = [
            permutation_indices(
                cases,
                CALIBRATION_PERMUTATION_SEED,
                operation,
                permutation_index,
            )
            for permutation_index in range(permutations)
        ]
        null_probabilities.append(
            _fixed_score_randomization_p(focal, nuisance, null_outcome, matrix)
        )
        signal_probabilities.append(
            _fixed_score_randomization_p(focal, nuisance, signal_outcome, matrix)
        )
    null_rejections = sum(value <= CALIBRATION_ALPHA for value in null_probabilities)
    signal_rejections = sum(value <= CALIBRATION_ALPHA for value in signal_probabilities)
    null_rate = null_rejections / replications
    power = signal_rejections / replications
    null_interval = wilson_interval(null_rejections, replications)
    power_interval = wilson_interval(signal_rejections, replications)
    null_maximum = CALIBRATION_ALPHA + CALIBRATION_NULL_Z * math.sqrt(
        CALIBRATION_ALPHA * (1.0 - CALIBRATION_ALPHA) / replications
    )
    return {
        "design": {
            "replications": replications,
            "permutations": permutations,
            "cases": cases,
            "alpha": CALIBRATION_ALPHA,
            "focal_nuisance_correlation": 0.35,
            "nuisance_coefficient": CALIBRATION_NUISANCE,
            "signal_coefficient": signal,
            "noise_standard_deviation": 1.0,
            "outcome_noise_distribution": CALIBRATION_NOISE_DISTRIBUTION,
            "error_variance_model": CALIBRATION_VARIANCE_MODEL,
            "paired_null_signal_scenarios": True,
            "data_rng": "numpy_pcg64",
            "data_seed": CALIBRATION_DATA_SEED,
            "permutation_seed": CALIBRATION_PERMUTATION_SEED,
        },
        "thresholds": {
            "null_rejection_rate_maximum": null_maximum,
            "null_boundary_formula": "alpha + 3.5 * sqrt(alpha * (1-alpha) / replications)",
            "null_wilson_95_must_contain_alpha": True,
            "power_wilson_95_lower_minimum": CALIBRATION_POWER_MINIMUM,
        },
        "null": {
            "rejections": null_rejections,
            "rate": null_rate,
            "wilson_95": list(null_interval),
            "minimum_p": min(null_probabilities),
            "median_p": float(np.median(null_probabilities)),
        },
        "signal": {
            "rejections": signal_rejections,
            "power": power,
            "wilson_95": list(power_interval),
            "maximum_p": max(signal_probabilities),
            "median_p": float(np.median(signal_probabilities)),
        },
        "passed": (
            null_rate <= null_maximum
            and null_interval[0] <= CALIBRATION_ALPHA <= null_interval[1]
            and power_interval[0] >= CALIBRATION_POWER_MINIMUM
        ),
    }


def reference_as_dict(reference: ReferenceParameter) -> dict[str, Any]:
    return {
        "ordinal": reference.ordinal,
        "parameter": reference.parameter,
        "source": reference.source,
        "target": reference.target,
        "predictor_ids": list(reference.predictor_ids),
        "focal_index": reference.focal_index,
        "stored_original": reference.stored_original,
        "reproduced_original": reference.reproduced_original,
        "stored_exceedances": reference.stored_exceedances,
        "reference_exceedances": reference.reference_exceedances,
        "stored_p_value": reference.stored_p_value,
        "reference_p_value": reference.reference_p_value,
        "coefficient_sha256_le_f64": reference.coefficient_sha256,
        "minimum_absolute_threshold_margin": reference.minimum_threshold_margin,
    }


def main() -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix="qpls-structural-randomization-", dir=RESULTS))
    report: dict[str, Any] = {
        "schema_version": REFERENCE_REPORT_SCHEMA_VERSION,
        "target": TARGET,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "passed": False,
        "checks": [],
    }
    try:
        require_release_cli()
        serial, serial_command, serial_recipe = run_release_cli(1, temporary)
        parallel, parallel_command, parallel_recipe = run_release_cli(4, temporary)
        serial_contract = validate_cli_contract(serial, 1, serial_recipe)
        parallel_contract = validate_cli_contract(parallel, 4, parallel_recipe)
        assert_worker_invariance(serial, parallel)

        recipe = migrated_recipe_payload(1)
        references, all_indices, python_coefficients = reconstruct_python_reference(
            serial, recipe
        )
        frozen_parameter = expected_parameter_manifest()[0]
        frozen_operation = operation_for_parameter(frozen_parameter)
        frozen_actual = {
            str(index): permutation_indices(12, SEED, frozen_operation, index)
            for index in FROZEN_INDEX_VECTORS
        }
        if frozen_actual != {str(key): value for key, value in FROZEN_INDEX_VECTORS.items()}:
            raise ValueError("independent deterministic index vectors differ from the frozen contract")

        if ARTIFACTS.exists():
            shutil.rmtree(ARTIFACTS)
        temporary.rename(ARTIFACTS)
        temporary = ARTIFACTS
        r_paths = write_r_inputs(ARTIFACTS, serial, references, all_indices)
        r_reference, r_version = run_r_reference(r_paths, references, python_coefficients)
        calibration = calibration_study()
        if not calibration["passed"]:
            raise ValueError("predeclared fixed-score null/power calibration boundaries failed")

        checks = [
            "release_cli_exact_path_and_freshness",
            "schema_v2_to_v3_recipe_migration",
            "exact_ordered_three_path_manifest",
            "exact_permutation_plan_and_method_version",
            "frozen_deterministic_indices_0_and_98",
            "python_fixed_score_freedman_lane_coefficients",
            "python_exact_exceedances_and_plus_one_probabilities",
            "r_full_coefficient_recomputation",
            "r_exact_exceedances_and_plus_one_probabilities",
            "workers_1_and_4_exact_payload_equality",
            "calibrated_null_boundary",
            "calibrated_power_boundary",
        ]
        report.update(
            {
                "tested_product": {
                    "release_cli": tested_file(CLI),
                    "commands": [serial_command, parallel_command],
                    "workers": [serial_contract, parallel_contract],
                },
                "evidence_sources": [
                    tested_file(path) for path in reference_evidence_sources()
                ],
                "fixtures": {
                    "data": tested_file(SOURCE_DATA),
                    "source_schema_v2_recipe": tested_file(SOURCE_RECIPE),
                    "generated_schema_v3_recipes": [
                        tested_file(ARTIFACTS / "workers_1.recipe.json"),
                        tested_file(ARTIFACTS / "workers_4.recipe.json"),
                    ],
                },
                "contract": {
                    "schema_identities": {
                        "result_envelope": ANALYSIS_RESULT_SCHEMA_VERSION,
                        "analysis_recipe": ANALYSIS_RECIPE_SCHEMA_VERSION,
                        "pls_inference_payload_kind": PLS_INFERENCE_PAYLOAD_KIND,
                        "project_archive_context_only": PROJECT_ARCHIVE_SCHEMA_VERSION,
                    },
                    "plan_operation": PLAN_OPERATION,
                    "index_stream_version": INDEX_STREAM_VERSION,
                    "seed": SEED,
                    "permutations": PERMUTATIONS,
                    "ordered_parameters": expected_parameter_manifest(),
                    "frozen_indices": frozen_actual,
                    "exceedance_rule": "abs(permuted_focal_coefficient) >= abs(original_focal_coefficient)",
                    "p_value_rule": "(exceedances + 1) / (permutations + 1)",
                },
                "python_reference": {
                    "runtime": sys.version,
                    "numpy": np.__version__,
                    "parameters": [reference_as_dict(row) for row in references],
                },
                "r_reference": {
                    "runtime": r_version,
                    "script": tested_file(R_SCRIPT),
                    "inputs": {
                        name: tested_file(path)
                        for name, path in r_paths.items()
                        if name != "output"
                    },
                    **r_reference,
                },
                "calibration": calibration,
                "limitations": [
                    "The structural randomization scope treats converged PLS construct scores as fixed and does not re-estimate the measurement model per permutation.",
                    "The bundled corporate fixture has 12 complete rows; broader behavior is probed by a separate preregistered synthetic fixed-score calibration.",
                    "The synthetic calibration is limited to paired homoscedastic Gaussian-error scenarios with constant variance.",
                    "Floating-point coefficients are compared with declared relative tolerances; parameter identity, order, index vectors, exceedance counts, +1 arithmetic, and worker equality are exact.",
                    "This scientific reference does not by itself qualify native authoring, archive persistence, accessibility, or packaged export workflows.",
                ],
                "checks": [{"name": name, "passed": True} for name in checks],
                "passed": True,
            }
        )
    except Exception as error:  # fail-closed report for release evidence
        report["error"] = f"{type(error).__name__}: {error}"
        report["checks"].append({"name": "reference_execution", "passed": False})
    finally:
        if temporary.exists() and temporary != ARTIFACTS:
            shutil.rmtree(temporary, ignore_errors=True)
        REPORT.write_text(
            json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
    print(json.dumps({"report": str(REPORT), "passed": report["passed"]}, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
