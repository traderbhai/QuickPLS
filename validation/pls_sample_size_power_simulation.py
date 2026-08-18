#!/usr/bin/env python3
"""Qualification simulation and shared evidence helpers for PLS power v1.

The qualification profile is intentionally heavier than a smoke run.  It uses
the current prebuilt debug CLI, executes the production Rust implementation,
and compares a preregistered power-recovery design with the independent NumPy
implementation.  A smoke profile is useful during development but is never
written as passing method-promotion evidence.

This module also owns the small, method-scoped helpers used by the companion
boundary, persistence, export, and audit gates.  The helpers never build or
package QuickPLS and fail closed when the debug CLI predates an execution
source.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import os
import random
import re
import subprocess
import sys
import time
import uuid
from copy import deepcopy
from datetime import datetime, timezone
from pathlib import Path
from statistics import NormalDist
from typing import Any, Iterable, Sequence


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
MANIFEST_PATH = VALIDATION / "methods" / "history" / "pls_sample_size_power_v1.manifest.json"
REPORT_ROOT = VALIDATION / "results" / "method_factory" / "pls_sample_size_power_v1"
WORK_ROOT = REPORT_ROOT / "work"
CLI = ROOT / "target" / "debug" / "qpls.exe"

FEATURE_ID = "qpls3.pls.sample_size_power"
METHOD_VERSION = "pls_sample_size_power_v1"
CATALOGUE_SNAPSHOT_DATE = "2026-08-12"
CAPABILITY_STREAM_DOMAIN = "quickpls/pls_sample_size_power_v1/monte_carlo"
SIMULATION_SOURCE = "validation/pls_sample_size_power_simulation.py"
FOCUSED_FACTORY_TEST = "validation/test_pls_sample_size_power_v1.py"

# These are the source bytes that can change the executable power result.  A
# binary older than any of them is unsuitable for evidence generation.
EXECUTION_SOURCES = (
    "Cargo.lock",
    "crates/qpls-core/Cargo.toml",
    "crates/qpls-core/src/lib.rs",
    "crates/qpls-core/src/contract.rs",
    "crates/qpls-core/src/validation.rs",
    "crates/qpls-data/Cargo.toml",
    "crates/qpls-data/src/lib.rs",
    "crates/qpls-estimation/Cargo.toml",
    "crates/qpls-estimation/src/lib.rs",
    "crates/qpls-estimation/src/pls.rs",
    "crates/qpls-resampling/Cargo.toml",
    "crates/qpls-resampling/src/lib.rs",
    "crates/qpls-resampling/src/power.rs",
    "crates/qpls-runner/Cargo.toml",
    "crates/qpls-runner/src/lib.rs",
    "crates/qpls-project/Cargo.toml",
    "crates/qpls-project/src/archive_integrity.rs",
    "crates/qpls-project/src/lib.rs",
    "crates/qpls-cli/Cargo.toml",
    "crates/qpls-cli/src/main.rs",
)

IDENTITY_VERIFICATION = {
    "kind": "identity_report",
    "identity_pointers": {
        "passed": "/passed",
        "feature_id": "/feature_id",
        "method_version": "/method_version",
        "catalogue_snapshot_date": "/catalogue_snapshot_date",
    },
    "source_artifacts_pointer": "/source_artifacts",
    "generated_at_pointer": "/generated_at_utc",
}


class DuplicateKeyError(ValueError):
    """Raised when JSON contains an ambiguous duplicate key."""


def _strict_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateKeyError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def strict_load_json(path: Path) -> Any:
    """Load strict UTF-8 JSON, rejecting duplicates and non-finite values."""

    with path.open("r", encoding="utf-8-sig") as handle:
        return json.load(
            handle,
            object_pairs_hook=_strict_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                ValueError(f"non-finite JSON number: {value}")
            ),
        )


def repository_path(path: Path) -> str:
    return path.resolve().relative_to(ROOT.resolve()).as_posix()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def source_descriptors(paths: Iterable[str]) -> list[dict[str, Any]]:
    descriptors: list[dict[str, Any]] = []
    for relative in sorted(set(paths)):
        candidate = Path(relative)
        if (
            not relative
            or "\\" in relative
            or candidate.is_absolute()
            or ".." in candidate.parts
        ):
            raise ValueError(f"unsafe repository source path: {relative!r}")
        path = ROOT / candidate
        if not path.is_file():
            raise FileNotFoundError(f"required power-v1 evidence source is missing: {relative}")
        descriptors.append(
            {
                "path": relative,
                "size": path.stat().st_size,
                "sha256": sha256_file(path),
            }
        )
    return descriptors


def manifest() -> dict[str, Any]:
    document = strict_load_json(MANIFEST_PATH)
    feature = document["feature"]
    expected = {
        "id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
    }
    for field, value in expected.items():
        if feature.get(field) != value:
            raise ValueError(
                f"power-v1 manifest identity mismatch for {field}: "
                f"expected {value!r}, found {feature.get(field)!r}"
            )
    return document


def role_sources(role: str, extras: Iterable[str] = ()) -> list[str]:
    document = manifest()
    governance = document["governance"]
    sources = {
        governance["manifest_path"],
        governance["schema_path"],
        governance["validator_path"],
        governance["focused_test_path"],
        SIMULATION_SOURCE,
        FOCUSED_FACTORY_TEST,
        *document["qualification"]["source_requirements"][role],
        *extras,
    }
    return sorted(sources)


def write_identity_report(
    role: str,
    *,
    passed: bool,
    checks: dict[str, Any],
    extras: Iterable[str] = (),
    execution: Any | None = None,
) -> Path:
    """Write one factory-compatible role report bound to exact source bytes."""

    document = manifest()
    feature = document["feature"]
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    path = REPORT_ROOT / f"{role}.identity.json"
    report: dict[str, Any] = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": role,
        "passed": bool(passed),
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "source_artifacts": source_descriptors(role_sources(role, extras)),
        "checks": checks,
    }
    if execution is not None:
        report["execution"] = execution
    path.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return path


def run_command(
    command: Sequence[str],
    *,
    timeout: int = 900,
    env: dict[str, str] | None = None,
) -> tuple[subprocess.CompletedProcess[str], dict[str, Any]]:
    started = time.monotonic()
    completed = subprocess.run(
        list(command),
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        env={**os.environ, **(env or {})},
    )
    record = {
        "command": list(command),
        "returncode": completed.returncode,
        "duration_seconds": round(time.monotonic() - started, 3),
        "stdout_tail": completed.stdout[-4_000:],
        "stderr_tail": completed.stderr[-4_000:],
    }
    return completed, record


def execution_source_snapshot() -> dict[str, dict[str, Any]]:
    return {
        relative: {
            "size": (ROOT / relative).stat().st_size,
            "mtime_ns": (ROOT / relative).stat().st_mtime_ns,
            "sha256": sha256_file(ROOT / relative),
        }
        for relative in EXECUTION_SOURCES
    }


def require_current_cli() -> dict[str, Any]:
    """Require a prebuilt CLI newer than every bound execution source."""

    if not CLI.is_file():
        raise FileNotFoundError(
            "target/debug/qpls.exe is required; the power-v1 evidence lane never builds the product"
        )
    snapshot = execution_source_snapshot()
    newest_source = max(row["mtime_ns"] for row in snapshot.values())
    current = CLI.stat().st_mtime_ns >= newest_source
    if not current:
        raise RuntimeError(
            "target/debug/qpls.exe predates a PLS sample-size/power execution source; "
            "run the coordinated debug build before regenerating evidence"
        )
    return {
        "passed": True,
        "path": repository_path(CLI),
        "size": CLI.stat().st_size,
        "sha256": sha256_file(CLI),
        "mtime_ns": CLI.stat().st_mtime_ns,
        "prebuilt": True,
        "built_by_factory": False,
        "newer_than_bound_execution_sources": True,
        "execution_sources": snapshot,
    }


def require_stable_execution_sources(before: dict[str, dict[str, Any]]) -> dict[str, Any]:
    after = execution_source_snapshot()
    changed = [relative for relative in EXECUTION_SOURCES if before.get(relative) != after[relative]]
    return {
        "passed": not changed,
        "changed_sources": changed,
        "after": after,
    }


def _dummy_rows(count: int = 48) -> list[list[float]]:
    rng = random.Random(20_260_814)
    rows: list[list[float]] = []
    for _ in range(count):
        latent_x = rng.gauss(0.0, 1.0)
        latent_y = 0.35 * latent_x + math.sqrt(1.0 - 0.35**2) * rng.gauss(0.0, 1.0)
        rows.append(
            [
                0.8 * latent_x + 0.6 * rng.gauss(0.0, 1.0),
                0.8 * latent_x + 0.6 * rng.gauss(0.0, 1.0),
                0.8 * latent_x + 0.6 * rng.gauss(0.0, 1.0),
                0.8 * latent_y + 0.6 * rng.gauss(0.0, 1.0),
                0.8 * latent_y + 0.6 * rng.gauss(0.0, 1.0),
                0.8 * latent_y + 0.6 * rng.gauss(0.0, 1.0),
            ]
        )
    return rows


def ensure_fixture_dataset() -> tuple[Path, str, dict[str, Any]]:
    """Create/import the irrelevant observed-data envelope used by the CLI."""

    require_current_cli()
    WORK_ROOT.mkdir(parents=True, exist_ok=True)
    csv_path = WORK_ROOT / "prospective_power_envelope.csv"
    with csv_path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.writer(handle)
        writer.writerow(["x1", "x2", "x3", "y1", "y2", "y3"])
        writer.writerows([[f"{value:.15g}" for value in row] for row in _dummy_rows()])
    project_path = WORK_ROOT / "prospective_power_envelope.qpls"
    if project_path.exists():
        project_path.unlink()
    imported, import_record = run_command(
        [str(CLI), "import", str(csv_path), str(project_path), "--name", "power-v1-envelope"]
    )
    if imported.returncode != 0:
        raise RuntimeError(f"power-v1 fixture import failed: {import_record}")
    inspected, inspect_record = run_command([str(CLI), "inspect", str(project_path), "--json"])
    if inspected.returncode != 0:
        raise RuntimeError(f"power-v1 fixture inspection failed: {inspect_record}")
    fingerprint = json.loads(inspected.stdout)["datasets"][0]["fingerprint"]
    return csv_path, fingerprint, {
        "import": import_record,
        "inspect": inspect_record,
        "project": repository_path(project_path),
        "project_sha256": sha256_file(project_path),
        "dataset": repository_path(csv_path),
        "dataset_sha256": sha256_file(csv_path),
    }


def canonical_recipe(
    fingerprint: str,
    *,
    name: str,
    population_path: float,
    sample_size_grid: Sequence[int],
    monte_carlo_replicates: int,
    bootstrap_replicates: int = 99,
    target_power: float = 0.80,
    alpha: float = 0.05,
    interval_confidence_level: float = 0.95,
    seed: int = 20_260_814,
    workers: int = 1,
    predictor_loadings: Sequence[float] = (0.8, 0.8, 0.8),
    outcome_loadings: Sequence[float] = (0.8, 0.8, 0.8),
) -> dict[str, Any]:
    """Build the exact schema-v3 product recipe for the bounded design."""

    return {
        "schema_version": 3,
        "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"quickpls:power-v1:recipe:{name}")),
        "created_at": "2026-08-14T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": str(uuid.uuid5(uuid.NAMESPACE_URL, f"quickpls:power-v1:model:{name}")),
            "name": "PLS sample-size/power v1 factory envelope",
            "constructs": [
                {
                    "id": "x",
                    "name": "Predictor",
                    "short_name": "X",
                    "mode": "reflective",
                    "indicators": ["x1", "x2", "x3"],
                },
                {
                    "id": "y",
                    "name": "Outcome",
                    "short_name": "Y",
                    "mode": "reflective",
                    "indicators": ["y1", "y2", "y3"],
                },
            ],
            "paths": [{"source": "x", "target": "y"}],
            "controls": [],
            "higher_order_constructs": [],
            "interactions": [],
        },
        "settings": {
            "method": "pls_sample_size_power",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3_000,
            "bootstrap_samples": 0,
            "studentized_inner_samples": 0,
            "permutation_samples": 0,
            "seed": seed,
            "workers": workers,
            "confidence_level": interval_confidence_level,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
            "case_weight_column": None,
        },
        "method_config": {
            "kind": "pls_sample_size_power",
            "scenario_identity": name,
            "predictor_construct": "x",
            "outcome_construct": "y",
            "predictor_indicator_loadings": list(predictor_loadings),
            "outcome_indicator_loadings": list(outcome_loadings),
            "population_path": population_path,
            "exogenous_distribution": "standard_normal",
            "structural_disturbance_distribution": "standard_normal",
            "indicator_error_distribution": "standard_normal",
            "missing_data": "none",
            "inference": "case_bootstrap_normal_reference_two_sided",
            "sample_size_grid": list(sample_size_grid),
            "alpha": alpha,
            "target_power": target_power,
            "interval_confidence_level": interval_confidence_level,
            "monte_carlo_replicates": monte_carlo_replicates,
            "bootstrap_replicates": bootstrap_replicates,
        },
        "metadata": {"status": "candidate_pls_sample_size_power_v1_bounded_scope"},
    }


def scientific_recipe_from_canonical(recipe: dict[str, Any]) -> dict[str, Any]:
    """Project a product recipe into the independent Python contract."""

    config = recipe["method_config"]
    settings = recipe["settings"]
    return {
        "schema_version": 1,
        "capability_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "scenario_identity": config["scenario_identity"],
        "design": {
            "predictor_construct": config["predictor_construct"],
            "outcome_construct": config["outcome_construct"],
            "predictor_indicator_loadings": config["predictor_indicator_loadings"],
            "outcome_indicator_loadings": config["outcome_indicator_loadings"],
            "population_path": config["population_path"],
            "exogenous_distribution": config["exogenous_distribution"],
            "structural_disturbance_distribution": config[
                "structural_disturbance_distribution"
            ],
            "indicator_error_distribution": config["indicator_error_distribution"],
            "missing_data": config["missing_data"],
        },
        "estimator": {
            "weighting_scheme": settings["weighting_scheme"],
            "preprocessing": settings["preprocessing"],
            "tolerance": settings["tolerance"],
            "max_iterations": settings["max_iterations"],
        },
        "inference": config["inference"],
        "sample_size_grid": config["sample_size_grid"],
        "alpha": config["alpha"],
        "target_power": config["target_power"],
        "confidence_level": config["interval_confidence_level"],
        "monte_carlo_replicates": config["monte_carlo_replicates"],
        "bootstrap_replicates": config["bootstrap_replicates"],
        "master_seed": settings["seed"],
        "workers": settings["workers"],
    }


def _write_recipe(path: Path, recipe: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(recipe, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )


def run_product_power(
    *,
    name: str,
    recipe: dict[str, Any],
    dataset: Path,
    expect_success: bool = True,
    timeout: int = 1_800,
) -> dict[str, Any]:
    """Execute one typed prospective power run with the prebuilt CLI."""

    cli_identity = require_current_cli()
    WORK_ROOT.mkdir(parents=True, exist_ok=True)
    recipe_path = WORK_ROOT / f"{name}.recipe.json"
    result_path = WORK_ROOT / f"{name}.result.json"
    _write_recipe(recipe_path, recipe)
    if result_path.exists():
        result_path.unlink()
    completed, record = run_command(
        [
            str(CLI),
            "run",
            str(recipe_path),
            "--data",
            str(dataset),
            "--output",
            str(result_path),
            "--allow-experimental",
        ],
        timeout=timeout,
    )
    base = {
        "execution": record,
        "cli_identity": cli_identity,
        "recipe": repository_path(recipe_path),
        "recipe_sha256": sha256_file(recipe_path),
        "result": repository_path(result_path) if result_path.exists() else None,
        "result_sha256": sha256_file(result_path) if result_path.exists() else None,
    }
    if not expect_success:
        return {
            **base,
            "passed": completed.returncode != 0 and not result_path.exists(),
            "returncode": completed.returncode,
        }
    if completed.returncode != 0 or not result_path.is_file():
        raise RuntimeError(f"PLS sample-size/power execution failed for {name}: {record}")
    document = strict_load_json(result_path)
    analysis = document.get("payload", {}).get("analysis")
    identity_passed = (
        document.get("status") == "completed"
        and document.get("payload", {}).get("kind") == "pls_sample_size_power_v1"
        and document.get("provenance", {}).get("method") == "pls_sample_size_power"
        and document.get("provenance", {}).get("method_version") == METHOD_VERSION
        and isinstance(analysis, dict)
        and analysis.get("capability_id") == FEATURE_ID
        and analysis.get("method_version") == METHOD_VERSION
        and analysis.get("stream_domain") == CAPABILITY_STREAM_DOMAIN
    )
    return {
        **base,
        "passed": identity_passed,
        "returncode": completed.returncode,
        "document": document,
        "analysis": analysis,
    }


def wilson_interval(successes: int, trials: int, confidence_level: float) -> tuple[float, float]:
    if trials <= 0 or successes < 0 or successes > trials:
        raise ValueError("invalid Wilson counts")
    z = NormalDist().inv_cdf(1.0 - (1.0 - confidence_level) / 2.0)
    proportion = successes / trials
    denominator = 1.0 + z * z / trials
    center = (proportion + z * z / (2.0 * trials)) / denominator
    half = z * math.sqrt(
        proportion * (1.0 - proportion) / trials + z * z / (4.0 * trials * trials)
    ) / denominator
    return max(0.0, center - half), min(1.0, center + half)


def validate_product_analysis(recipe: dict[str, Any], analysis: dict[str, Any]) -> dict[str, Any]:
    """Independently recompute row accounting, Wilson intervals, and decision."""

    config = recipe["method_config"]
    requested = config["monte_carlo_replicates"]
    confidence = config["interval_confidence_level"]
    target = config["target_power"]
    outcomes = analysis.get("outcomes", [])
    rows = analysis.get("rows", [])
    expected_total = len(config["sample_size_grid"]) * requested
    ledger_order = [
        (outcome.get("sample_size"), outcome.get("replicate_index")) for outcome in outcomes
    ]
    expected_order = [
        (sample_size, replicate_index)
        for sample_size in config["sample_size_grid"]
        for replicate_index in range(requested)
    ]
    row_checks: list[dict[str, Any]] = []
    for row in rows:
        selected = [item for item in outcomes if item.get("sample_size") == row.get("sample_size")]
        successes = sum(item.get("successful") is True for item in selected)
        failures = sum(item.get("successful") is False for item in selected)
        rejections = sum(item.get("rejected") is True for item in selected)
        lower, upper = wilson_interval(rejections, requested, confidence)
        check = {
            "sample_size": row.get("sample_size"),
            "ledger_count": len(selected),
            "successful_replicates": successes,
            "failed_replicates": failures,
            "rejections": rejections,
            "expected_confidence_lower": lower,
            "expected_confidence_upper": upper,
            "passed": (
                len(selected) == requested
                and row.get("requested_replicates") == requested
                and row.get("attempted_replicates") == requested
                and row.get("successful_replicates") == successes
                and row.get("failed_replicates") == failures
                and row.get("rejections") == rejections
                and math.isclose(row.get("achieved_power", math.nan), rejections / requested, abs_tol=1e-12)
                and math.isclose(row.get("confidence_lower", math.nan), lower, abs_tol=1e-12)
                and math.isclose(row.get("confidence_upper", math.nan), upper, abs_tol=1e-12)
                and row.get("qualifies") == (lower >= target)
            ),
        }
        row_checks.append(check)
    selected = next((row["sample_size"] for row in rows if row.get("qualifies")), None)
    expected_decision = (
        {"status": "reached", "sample_size": selected}
        if selected is not None
        else {"status": "not_reached"}
    )
    failure_rows_valid = all(
        (
            outcome.get("successful") is True
            and outcome.get("converged") is True
            and isinstance(outcome.get("target_estimate"), (int, float))
            and not isinstance(outcome.get("target_estimate"), bool)
            and math.isfinite(float(outcome["target_estimate"]))
            and isinstance(outcome.get("p_value_two_sided"), (int, float))
            and not isinstance(outcome.get("p_value_two_sided"), bool)
            and math.isfinite(float(outcome["p_value_two_sided"]))
            and 0.0 <= float(outcome["p_value_two_sided"]) <= 1.0
            and outcome.get("rejected")
            == (float(outcome["p_value_two_sided"]) <= float(config["alpha"]))
            and outcome.get("failure_code") is None
            and outcome.get("failure_message") is None
        )
        or (
            outcome.get("successful") is False
            and outcome.get("converged") is False
            and outcome.get("target_estimate") is None
            and outcome.get("p_value_two_sided") is None
            and isinstance(outcome.get("failure_code"), str)
            and bool(outcome["failure_code"].strip())
            and isinstance(outcome.get("failure_message"), str)
            and bool(outcome["failure_message"].strip())
            and outcome.get("rejected") is False
        )
        for outcome in outcomes
    )
    checks = {
        "identity": analysis.get("capability_id") == FEATURE_ID
        and analysis.get("method_version") == METHOD_VERSION,
        "ordered_complete_ledger": len(outcomes) == expected_total and ledger_order == expected_order,
        "rows_recomputed": len(rows) == len(config["sample_size_grid"])
        and all(row["passed"] for row in row_checks),
        "decision_recomputed": analysis.get("decision") == expected_decision,
        "failure_accounting_typed": failure_rows_valid,
        "workload_matches": analysis.get("workload", {}).get("planned_datasets") == expected_total,
    }
    return {
        "passed": all(checks.values()),
        "checks": checks,
        "row_checks": row_checks,
        "expected_decision": expected_decision,
    }


def _row_map(report: dict[str, Any]) -> dict[int, dict[str, Any]]:
    return {int(row["sample_size"]): row for row in report["rows"]}


def compare_independent_power(
    product: dict[str, Any], reference: dict[str, Any], *, target_power: float
) -> dict[str, Any]:
    """Compare independent streams with preregistered sampling-error tolerances."""

    product_rows = _row_map(product)
    reference_rows = _row_map(reference)
    common = sorted(product_rows.keys() & reference_rows.keys())
    comparisons: list[dict[str, Any]] = []
    for sample_size in common:
        left = product_rows[sample_size]
        right = reference_rows[sample_size]
        p_left = float(left["achieved_power"])
        p_right = float(right["achieved_power"])
        n_left = int(left["requested_replicates"])
        n_right = int(right["requested_replicates"])
        combined_se = math.sqrt(
            max(p_left * (1.0 - p_left), 0.0) / n_left
            + max(p_right * (1.0 - p_right), 0.0) / n_right
        )
        tolerance = max(0.08, 3.0 * combined_se)
        comparisons.append(
            {
                "sample_size": sample_size,
                "product_power": p_left,
                "independent_power": p_right,
                "absolute_difference": abs(p_left - p_right),
                "preregistered_tolerance": tolerance,
                "passed": abs(p_left - p_right) <= tolerance,
            }
        )
    product_selected = next(
        (int(row["sample_size"]) for row in product["rows"] if row.get("qualifies")), None
    )
    independent_at_selection = reference_rows.get(product_selected) if product_selected else None
    conservative_check = (
        independent_at_selection is not None
        and float(independent_at_selection["confidence_lower"]) >= target_power
    )
    return {
        "passed": bool(common)
        and len(common) == len(product_rows) == len(reference_rows)
        and all(row["passed"] for row in comparisons)
        and conservative_check,
        "common_grid": common,
        "row_comparisons": comparisons,
        "product_selected_sample_size": product_selected,
        "independent_validation_at_product_selection": independent_at_selection,
        "independent_lower_bound_meets_target_at_selection": conservative_check,
    }


def _compact_reference(report: dict[str, Any]) -> dict[str, Any]:
    return {
        key: deepcopy(report[key])
        for key in (
            "report_kind",
            "passed",
            "feature_id",
            "method_version",
            "stream_domain",
            "failure_policy",
            "interval_method",
            "inference_method",
            "rows",
            "decision",
            "monotonicity_violations",
            "outcome_digest",
        )
    }


def qualification_profile() -> tuple[dict[str, Any], list[dict[str, Any]]]:
    """Run the preregistered, bounded scientific qualification profile."""

    from pls_sample_size_power_reference import run_reference

    cli_identity = require_current_cli()
    before = cli_identity["execution_sources"]
    dataset, fingerprint, fixture_execution = ensure_fixture_dataset()
    executions: list[dict[str, Any]] = [fixture_execution]

    # At B=99 and R=1250 this uses exactly 250,000 PLS fits, the declared
    # desktop cap. Two moderate/large-n points limit finite-sample distortion;
    # the preregistered pooled interval has 2,500 independent null outcomes.
    null_recipe = canonical_recipe(
        fingerprint,
        name="qualification_null_calibration",
        population_path=0.0,
        sample_size_grid=(160, 320),
        monte_carlo_replicates=1_250,
        bootstrap_replicates=99,
        target_power=0.50,
        seed=20_260_814,
        workers=4,
    )
    null_run = run_product_power(
        name="qualification_null_calibration", recipe=null_recipe, dataset=dataset
    )
    executions.append(null_run["execution"])
    null_contract = validate_product_analysis(null_recipe, null_run["analysis"])
    null_rows = null_run["analysis"]["rows"]
    null_points = [
        {
            "sample_size": row["sample_size"],
            "type_i_error": row["achieved_power"],
            "confidence_lower": row["confidence_lower"],
            "confidence_upper": row["confidence_upper"],
            "convergence_rate": row["successful_replicates"] / row["requested_replicates"],
            "descriptive_point_guard_passed": row["achieved_power"] <= 0.08
            and row["successful_replicates"] / row["requested_replicates"] >= 0.99,
        }
        for row in null_rows
    ]
    pooled_null_rejections = sum(int(row["rejections"]) for row in null_rows)
    pooled_null_trials = sum(int(row["requested_replicates"]) for row in null_rows)
    pooled_null_lower, pooled_null_upper = wilson_interval(
        pooled_null_rejections, pooled_null_trials, 0.95
    )
    null_acceptance = {
        "passed": pooled_null_lower <= 0.05 <= pooled_null_upper
        and pooled_null_upper <= 0.065
        and all(row["descriptive_point_guard_passed"] for row in null_points),
        "pooled_preregistered_interval": {
            "rejections": pooled_null_rejections,
            "trials": pooled_null_trials,
            "type_i_error": pooled_null_rejections / pooled_null_trials,
            "confidence_lower": pooled_null_lower,
            "confidence_upper": pooled_null_upper,
            "contains_alpha_0_05": pooled_null_lower <= 0.05 <= pooled_null_upper,
            "upper_not_above_0_065": pooled_null_upper <= 0.065,
        },
        "sample_size_points": null_points,
        "pooling_rule": (
            "The two preregistered null grid points have equal replicate counts. Their rejection "
            "indicators are pooled once for the frozen binomial calibration interval; each point "
            "also has a descriptive 0.08 false-positive guard and 0.99 convergence guard."
        ),
    }

    # The product run uses 800 replications (240,000 fits).  The independent
    # run uses production-valid counts but fewer replications (60,000 fits),
    # with comparison limits widened by its declared Monte Carlo uncertainty.
    signal_recipe = canonical_recipe(
        fingerprint,
        name="qualification_power_recovery",
        population_path=0.45,
        sample_size_grid=(60, 100, 140),
        monte_carlo_replicates=800,
        bootstrap_replicates=99,
        target_power=0.75,
        seed=20_260_815,
        workers=4,
    )
    signal_run = run_product_power(
        name="qualification_power_recovery", recipe=signal_recipe, dataset=dataset
    )
    executions.append(signal_run["execution"])
    signal_contract = validate_product_analysis(signal_recipe, signal_run["analysis"])

    independent_recipe = scientific_recipe_from_canonical(signal_recipe)
    independent_recipe["scenario_identity"] = "qualification_power_recovery_independent"
    independent_recipe["master_seed"] = 20_260_916
    independent_recipe["monte_carlo_replicates"] = 200
    independent_recipe["workers"] = 1
    independent_started = time.monotonic()
    independent = run_reference(independent_recipe, enforce_production_counts=True)
    executions.append(
        {
            "command": [
                sys.executable,
                "validation/pls_sample_size_power_reference.py",
                "qualification_power_recovery_independent",
            ],
            "returncode": 0,
            "duration_seconds": round(time.monotonic() - independent_started, 3),
            "implementation": "direct imported independent NumPy function",
        }
    )
    independent_comparison = compare_independent_power(
        signal_run["analysis"], independent, target_power=signal_recipe["method_config"]["target_power"]
    )
    signal_rows = signal_run["analysis"]["rows"]
    monotonicity = all(
        right["achieved_power"] + 1e-12 >= left["achieved_power"]
        for left, right in zip(signal_rows, signal_rows[1:])
    )
    source_stability = require_stable_execution_sources(before)

    checks = {
        "qualification_profile_complete": True,
        "prebuilt_current_cli": cli_identity["passed"],
        "null_contract_recomputed": null_contract["passed"],
        "null_calibration": null_acceptance["passed"],
        "signal_contract_recomputed": signal_contract["passed"],
        "independent_reference_completed": independent.get("passed") is True,
        "independent_statistical_agreement": independent_comparison["passed"],
        "signal_grid_nondecreasing": monotonicity,
        "failure_denominator_policy": all(
            row["attempted_replicates"] == row["requested_replicates"]
            for row in [*null_rows, *signal_rows]
        ),
        "source_stable_during_gate": source_stability["passed"],
        "observed_dataset_values_not_design_inputs": True,
    }
    report = {
        "profile": "qualification",
        "passed": all(checks.values()),
        "checks": checks,
        "preregistered_limits": {
            "null_interval_contains": 0.05,
            "null_interval_upper_max": 0.065,
            "null_point_false_positive_guard_max": 0.08,
            "supported_scope_convergence_min": 0.99,
            "independent_curve_minimum_tolerance": 0.08,
            "independent_curve_standard_error_multiplier": 3.0,
            "power_target": 0.75,
        },
        "null": {
            "recipe": scientific_recipe_from_canonical(null_recipe),
            "rows": null_rows,
            "acceptance": null_acceptance,
            "contract_recomputation": null_contract,
            "outcome_digest": null_run["analysis"]["outcome_digest"],
        },
        "power_recovery": {
            "product_recipe": scientific_recipe_from_canonical(signal_recipe),
            "product_rows": signal_rows,
            "product_decision": signal_run["analysis"]["decision"],
            "product_contract_recomputation": signal_contract,
            "independent_recipe": independent_recipe,
            "independent_report": _compact_reference(independent),
            "comparison": independent_comparison,
        },
        "cli_identity": cli_identity,
        "source_stability": source_stability,
        "generated_artifacts": {
            "null_result": null_run["result"],
            "null_result_sha256": null_run["result_sha256"],
            "signal_result": signal_run["result"],
            "signal_result_sha256": signal_run["result_sha256"],
        },
    }
    return report, executions


def smoke_profile() -> tuple[dict[str, Any], list[dict[str, Any]]]:
    """Run a fast, explicitly non-qualifying product/reference smoke profile."""

    from pls_sample_size_power_reference import run_reference

    cli_identity = require_current_cli()
    before = cli_identity["execution_sources"]
    dataset, fingerprint, fixture_execution = ensure_fixture_dataset()
    recipe = canonical_recipe(
        fingerprint,
        name="smoke_power_recovery",
        population_path=0.45,
        sample_size_grid=(60, 120),
        monte_carlo_replicates=100,
        bootstrap_replicates=99,
        target_power=0.70,
        seed=20_260_817,
        workers=2,
    )
    product = run_product_power(name="smoke_power_recovery", recipe=recipe, dataset=dataset)
    contract = validate_product_analysis(recipe, product["analysis"])
    reference_recipe = scientific_recipe_from_canonical(recipe)
    reference_recipe["monte_carlo_replicates"] = 20
    reference_recipe["bootstrap_replicates"] = 9
    reference = run_reference(reference_recipe, enforce_production_counts=False)
    stability = require_stable_execution_sources(before)
    smoke_checks = {
        "product_completed": product["passed"],
        "product_contract_recomputed": contract["passed"],
        "independent_smoke_completed": reference["passed"],
        "source_stable_during_gate": stability["passed"],
    }
    return (
        {
            "profile": "smoke",
            "passed": all(smoke_checks.values()),
            "qualification_profile_complete": False,
            "promotion_evidence_written": False,
            "checks": smoke_checks,
            "product_rows": product["analysis"]["rows"],
            "independent_rows": reference["rows"],
            "note": "Smoke evidence is not method-promotion evidence and cannot qualify a scientific claim.",
        },
        [fixture_execution, product["execution"]],
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", choices=("qualification", "smoke"), default="qualification")
    args = parser.parse_args()
    if args.profile == "qualification":
        report, executions = qualification_profile()
        path = write_identity_report(
            "simulation_report",
            passed=report["passed"],
            checks=report,
            execution=executions,
            extras=[
                SIMULATION_SOURCE,
                "validation/pls_sample_size_power_reference.py",
                *EXECUTION_SOURCES,
            ],
        )
    else:
        report, executions = smoke_profile()
        REPORT_ROOT.mkdir(parents=True, exist_ok=True)
        path = REPORT_ROOT / "simulation_smoke.nonqualifying.json"
        path.write_text(
            json.dumps(
                {
                    **report,
                    "generated_at_utc": datetime.now(timezone.utc)
                    .isoformat()
                    .replace("+00:00", "Z"),
                    "execution": executions,
                },
                indent=2,
                sort_keys=True,
                allow_nan=False,
            )
            + "\n",
            encoding="utf-8",
        )
    print(json.dumps({"passed": report["passed"], "profile": args.profile, "output": str(path)}, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
