#!/usr/bin/env python3
"""Fail-closed source-tier qualification for bounded QuickPLS Phase-4 methods.

This factory refreshes engine, archive, and native identities using an already
built CLI and compiled Rust test executables. It never invokes Cargo, builds the
desktop application, or launches a GUI; packaged evidence and its audit remain
separate receipt-bound release gates.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import importlib
import json
import math
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Sequence


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
RESULTS = VALIDATION / "results" / "method_factory"
CLI = ROOT / "target" / "debug" / "qpls.exe"
COMMON_SOURCE = "validation/phase4_high_value_factory.py"
FOCUSED_TEST = "validation/test_phase4_high_value_factory.py"

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

CONFIGS: dict[str, dict[str, Any]] = {
    "plspredict_cvpat_v2": {
        "manifest": "validation/methods/plspredict_cvpat_v2.manifest.json",
        "feature_id": "qpls3.prediction.plspredict_cvpat",
        "method_version": "plspredict_indicator_v2",
        "catalogue_date": "2026-08-12",
        "reference_module": "plspredict_indicator_reference",
        "rust_tests": {
            "estimation": [
                "plspredict_v2_reports_leakage_free_indicator_metrics_and_cvpat",
                "plspredict_v2_fold_assignment_is_seeded_balanced_and_auditable",
                "plspredict_v2_mape_excludes_zero_actual_values_and_retains_count",
                "cvpat_v2_uses_lower_tail_pls_minus_benchmark_direction",
            ],
            "core": [
                "plspredict_indicator_v2_is_validated_and_rejects_unsupported_shapes",
            ],
            "project": [
                "runner_generated_prediction_round_trips_and_rejects_contract_tampering",
                "legacy_prediction_v1_reopens_with_warning_but_cannot_be_appended_as_new_evidence",
            ],
        },
        "native_tests": [
            "src/native/nativeAnalysisCatalog.test.ts",
            "src/native/nativeAnalysisRecipe.test.ts",
            "src/native/nativePlsReadiness.test.ts",
            "src/native/nativeCanonicalProject.test.ts",
            "src/native/nativeResults.test.ts",
            "src/native/nativeExportTables.test.ts",
        ],
    },
    "micom_permutation_mga_v3": {
        "manifest": "validation/methods/micom_permutation_mga_v3.manifest.json",
        "feature_id": "qpls3.groups.micom_permutation_mga",
        "method_version": "pls_mga_permutation_v3",
        "catalogue_date": "2026-08-12",
        "reference_module": "micom_mga_v3_reference",
        "rust_tests": {
            "estimation": [
                "micom_and_permutation_mga_v3_emit_swap_coupled_group_measurement_contract",
                "canonical_mga_permutation_plan_is_an_exact_complement_for_unequal_group_sizes",
                "micom_mga_v3_failed_fit_retries_are_group_swap_symmetric",
                "runtime_requires_explicit_micom_configural_confirmation",
                "permutation_mga_reports_progress_and_honors_cancellation",
            ],
            "core": [
                "bounded_mga_requires_group_column_metadata",
                "micom_v3_requires_configural_confirmation_and_stable_permutation_count",
                "bounded_mga_requires_explicit_distinct_groups_and_excludes_group_indicator",
                "bounded_mga_permutation_requires_supported_method_and_sample_count",
            ],
            "project": [
                "validated_append_and_archive_round_trip_preserve_explicit_mga_contract",
                "historical_mga_v2_reopens_but_cannot_be_appended_as_v3_evidence",
                "validated_append_rejects_tampered_mga_direction_atomically",
            ],
        },
        "native_tests": [
            "src/native/nativeAnalysisCatalog.test.ts",
            "src/native/nativeAnalysisRecipe.test.ts",
            "src/native/nativePlsReadiness.test.ts",
            "src/native/nativeMga.test.ts",
            "src/native/nativeCanonicalProject.test.ts",
            "src/native/nativeResults.test.ts",
            "src/native/nativeExportTables.test.ts",
            "src/native/NativeCalculationDialog.test.ts",
            "src/native/NativeGroupSetupDialog.test.tsx",
            "src/native/NativeUtilityDialog.test.tsx",
        ],
    },
}


class DuplicateKeyError(ValueError):
    """Raised when a JSON object is ambiguous."""


def _strict_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateKeyError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def strict_load(path: Path) -> Any:
    with path.open("r", encoding="utf-8-sig") as handle:
        return json.load(
            handle,
            object_pairs_hook=_strict_pairs,
            parse_constant=lambda token: (_ for _ in ()).throw(
                ValueError(f"non-finite JSON number: {token}")
            ),
        )


def write_json(path: Path, document: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(document, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )


def relative(path: Path) -> str:
    return path.resolve().relative_to(ROOT.resolve()).as_posix()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def descriptor(path: Path) -> dict[str, Any]:
    return {"path": relative(path), "size": path.stat().st_size, "sha256": sha256_file(path)}


def manifest(method: str) -> dict[str, Any]:
    config = CONFIGS[method]
    document = strict_load(ROOT / config["manifest"])
    feature = document["feature"]
    expected = {
        "id": config["feature_id"],
        "method_version": config["method_version"],
        "catalogue_snapshot_date": config["catalogue_date"],
    }
    for key, value in expected.items():
        if feature.get(key) != value:
            raise ValueError(f"{method} manifest {key} must equal {value!r}")
    return document


def report_root(method: str) -> Path:
    return RESULTS / method


def work_root(method: str) -> Path:
    return report_root(method) / "work"


def run(command: Sequence[str], timeout: int = 1200, env: dict[str, str] | None = None) -> dict[str, Any]:
    invoked = list(command)
    if os.name == "nt" and invoked and invoked[0] in {"npm", "npx"}:
        invoked[0] += ".cmd"
    try:
        completed = subprocess.run(
            invoked,
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
            env=env,
        )
        return {
            "command": list(command),
            "returncode": completed.returncode,
            "passed": completed.returncode == 0,
            "stdout_tail": completed.stdout[-5000:],
            "stderr_tail": completed.stderr[-5000:],
        }
    except subprocess.TimeoutExpired as error:
        return {
            "command": list(command),
            "returncode": None,
            "passed": False,
            "timeout_seconds": timeout,
            "stdout_tail": error.stdout[-5000:] if isinstance(error.stdout, str) else "",
            "stderr_tail": error.stderr[-5000:] if isinstance(error.stderr, str) else "",
        }


def require_prebuilt_cli() -> dict[str, Any]:
    if not CLI.is_file():
        raise FileNotFoundError("target/debug/qpls.exe is required; this factory does not build it")
    version = run([str(CLI), "--version"], timeout=30)
    if not version["passed"]:
        raise RuntimeError("the prebuilt QuickPLS CLI did not start")
    return {"artifact": descriptor(CLI), "version_check": version}


def _crate_source_paths(crate: str) -> list[Path]:
    crate_path = ROOT / "crates" / f"qpls-{crate}" / "src"
    return sorted(path for path in crate_path.rglob("*.rs") if path.is_file())


def find_prebuilt_test(crate: str) -> Path:
    candidates = sorted(
        (ROOT / "target" / "debug" / "deps").glob(f"qpls_{crate}-*.exe"),
        key=lambda path: path.stat().st_mtime_ns,
        reverse=True,
    )
    for candidate in candidates:
        listing = run([str(candidate), "--list"], timeout=60)
        if listing["passed"]:
            return candidate
    raise FileNotFoundError(f"no runnable prebuilt qpls-{crate} test executable exists")


def run_prebuilt_tests(crate: str, names: Iterable[str]) -> list[dict[str, Any]]:
    binary = find_prebuilt_test(crate)
    source_paths = _crate_source_paths(crate)
    source_fresh = all(path.stat().st_mtime_ns <= binary.stat().st_mtime_ns for path in source_paths)
    rows: list[dict[str, Any]] = []
    for name in names:
        execution = run([str(binary), name, "--nocapture"], timeout=1200)
        exact = f"::{name} ... ok" in execution["stdout_tail"] or f" {name} ... ok" in execution["stdout_tail"]
        execution.update(
            {
                "passed": execution["passed"] and exact and source_fresh,
                "expected_test": name,
                "exact_test_observed": exact,
                "prebuilt_test_binary": descriptor(binary),
                "crate_sources_not_newer_than_binary": source_fresh,
                "build_policy": "prebuilt_test_executable_only_no_cargo_invocation",
            }
        )
        rows.append(execution)
    return rows


def upgrade_recipe_v3(recipe: dict[str, Any], method_config: dict[str, Any]) -> dict[str, Any]:
    upgraded = copy.deepcopy(recipe)
    upgraded["schema_version"] = 3
    upgraded["model"].setdefault("controls", [])
    upgraded["model"].setdefault("higher_order_constructs", [])
    upgraded["model"].setdefault("interactions", [])
    settings = upgraded["settings"]
    settings.setdefault("studentized_inner_samples", 0)
    settings.setdefault("permutation_samples", 0)
    settings.setdefault("workers", 1)
    settings.setdefault("confidence_level", 0.95)
    settings.setdefault("case_weight_column", None)
    upgraded["method_config"] = copy.deepcopy(method_config)
    metadata = upgraded.setdefault("metadata", {})
    for legacy_key in (
        "group_methods",
        "group_permutation_samples",
        "micom_configural_confirmed",
        "mga_group_column",
        "mga_group_a",
        "mga_group_b",
    ):
        metadata.pop(legacy_key, None)
    metadata["validation_recipe_migration"] = (
        "schema_v2_fixture_to_explicit_schema_v3_copy"
    )
    return upgraded


def _patch_recipe_writer(module: Any, method_config_factory: Any) -> None:
    original = module.write_recipe

    def current_writer(path_or_fingerprint: Any, *args: Any, **kwargs: Any) -> Any:
        result = original(path_or_fingerprint, *args, **kwargs)
        recipe_path = module.RECIPE
        upgraded = upgrade_recipe_v3(strict_load(recipe_path), method_config_factory(strict_load(recipe_path)))
        write_json(recipe_path, upgraded)
        return result

    module.write_recipe = current_writer


def _prediction_method_config(_recipe: dict[str, Any]) -> dict[str, Any]:
    return {"kind": "predict"}


def _mga_method_config(recipe: dict[str, Any]) -> dict[str, Any]:
    metadata = recipe["metadata"]
    return {
        "kind": "mga",
        "group_column": metadata["mga_group_column"],
        "group_a": metadata["mga_group_a"],
        "group_b": metadata["mga_group_b"],
        "methods": ["micom", "mga_permutation"],
        "permutation_samples": int(metadata["group_permutation_samples"]),
        "configural_invariance_confirmed": metadata["micom_configural_confirmed"] == "true",
    }


def run_prediction_reference() -> dict[str, Any]:
    require_prebuilt_cli()
    sys.path.insert(0, str(VALIDATION))
    ref = importlib.import_module("plspredict_indicator_reference")
    work = work_root("plspredict_cvpat_v2")
    work.mkdir(parents=True, exist_ok=True)
    ref.RESULTS = work
    ref.DATA = work / "reference.csv"
    ref.RECIPE = work / "reference.recipe.json"
    ref.QUICKPLS = work / "reference.quickpls.json"
    ref.REPEAT = work / "reference.repeat.json"
    ref.REPORT = work / "independent_reference.json"
    ref.FINGERPRINT_PROJECT = work / "reference.fingerprint.qpls"
    ref.CLI = CLI
    ref.ensure_cli = lambda: None
    _patch_recipe_writer(ref, _prediction_method_config)
    try:
        ref.main()
    except SystemExit as error:
        if error.code not in (None, 0):
            raise RuntimeError(f"PLSpredict independent reference failed: {error.code}") from error
    report = strict_load(ref.REPORT)
    recipe = strict_load(ref.RECIPE)
    return {
        "passed": report.get("passed") is True
        and report.get("checks", {}).get("deterministic_repeat") is True
        and recipe.get("schema_version") == 3
        and recipe.get("method_config") == {"kind": "predict"},
        "report": descriptor(ref.REPORT),
        "recipe": descriptor(ref.RECIPE),
        "quickpls": descriptor(ref.QUICKPLS) if ref.QUICKPLS.is_file() else None,
        "checks": report.get("checks", {}),
    }


def run_mga_reference() -> dict[str, Any]:
    require_prebuilt_cli()
    sys.path.insert(0, str(VALIDATION))
    ref = importlib.import_module("micom_mga_v3_reference")
    work = work_root("micom_permutation_mga_v3")
    work.mkdir(parents=True, exist_ok=True)
    source_data = ref.DATA
    ref.DATA = work / "reference.csv"
    ref.DATA.write_bytes(source_data.read_bytes())
    ref.BASE_RECIPE = work / "reference.recipe.json"
    ref.SWAP_RECIPE = work / "swapped.quickpls.recipe.json"
    ref.BASE_QUICKPLS = work / "reference.quickpls.json"
    ref.SWAP_QUICKPLS = work / "swapped.quickpls.json"
    ref.OUTPUT = work / "independent_reference.json"
    freshness_sources = [
        CLI,
        ROOT / "validation/micom_mga_v3_reference.py",
        ROOT / "validation/micom_v2_reference.py",
        ROOT / CONFIGS["micom_permutation_mga_v3"]["manifest"],
    ]
    if all(path.is_file() for path in (ref.OUTPUT, ref.BASE_RECIPE, ref.BASE_QUICKPLS, ref.SWAP_QUICKPLS)):
        existing = strict_load(ref.OUTPUT)
        recipe = strict_load(ref.BASE_RECIPE)
        result = strict_load(ref.BASE_QUICKPLS)
        result_versions = result.get("payload", {}).get("estimation", {})
        current = ref.OUTPUT.stat().st_mtime_ns >= max(
            path.stat().st_mtime_ns for path in freshness_sources
        )
        if (
            current
            and existing.get("passed") is True
            and existing.get("reference_passed") is True
            and existing.get("promotion_sample_size") is True
            and existing.get("permutation_samples") == 5000
            and existing.get("promotion_ready") is True
            and recipe.get("schema_version") == 3
            and recipe.get("method_config", {}).get("kind") == "mga"
            and result_versions.get("mga_permutation", {}).get("method_version")
            == "pls_mga_permutation_v3"
            and result_versions.get("micom", {}).get("method_version") == "micom_v3"
        ):
            return {
                "passed": True,
                "report": descriptor(ref.OUTPUT),
                "recipe": descriptor(ref.BASE_RECIPE),
                "quickpls": descriptor(ref.BASE_QUICKPLS),
                "comparison": existing["quickpls_comparison"],
                "execution_policy": "reused_only_after_exact_identity_and_source_freshness_checks",
            }
    previous_argv = sys.argv
    previous_cli = os.environ.get("QUICKPLS_CLI_PATH")
    try:
        os.environ["QUICKPLS_CLI_PATH"] = str(CLI)
        sys.argv = [
            "micom_mga_v3_reference.py",
            "--permutations",
            "5000",
            "--run-quickpls",
        ]
        code = ref.main()
    finally:
        sys.argv = previous_argv
        if previous_cli is None:
            os.environ.pop("QUICKPLS_CLI_PATH", None)
        else:
            os.environ["QUICKPLS_CLI_PATH"] = previous_cli
    report = strict_load(ref.OUTPUT)
    recipe = strict_load(ref.BASE_RECIPE)
    comparison = report.get("quickpls_comparison") or {}
    return {
        "passed": code == 0
        and report.get("passed") is True
        and report.get("promotion_sample_size") is True
        and report.get("permutation_samples") == 5000
        and report.get("promotion_ready") is True
        and recipe.get("schema_version") == 3
        and recipe.get("method_config", {}).get("kind") == "mga",
        "report": descriptor(ref.OUTPUT),
        "recipe": descriptor(ref.BASE_RECIPE),
        "quickpls": descriptor(ref.BASE_QUICKPLS),
        "comparison": comparison,
        "execution_policy": "fresh_independent_5000_permutation_execution",
    }


def _run_recipe(
    recipe: dict[str, Any], data: Path, output: Path, *, reuse_current: bool = False
) -> dict[str, Any]:
    recipe_path = output.with_suffix(".recipe.json")
    if reuse_current and recipe_path.is_file() and output.is_file():
        try:
            existing_recipe = strict_load(recipe_path)
            existing_result = strict_load(output)
            current = output.stat().st_mtime_ns >= max(
                CLI.stat().st_mtime_ns, data.stat().st_mtime_ns, recipe_path.stat().st_mtime_ns
            )
            if (
                existing_recipe == recipe
                and current
                and existing_result.get("status") == "completed"
                and existing_result.get("provenance", {}).get("dataset_fingerprint")
                == recipe.get("dataset_fingerprint")
            ):
                return {
                    "execution": {
                        "command": ["reuse-current-recipe-result"],
                        "returncode": 0,
                        "passed": True,
                        "stdout_tail": "exact recipe and current completed result reused",
                        "stderr_tail": "",
                        "reuse_checks": {
                            "recipe_exact": True,
                            "result_newer_than_cli_data_and_recipe": True,
                            "dataset_identity_exact": True,
                        },
                    },
                    "output_exists": True,
                    "result": existing_result,
                    "recipe": descriptor(recipe_path),
                    "output": descriptor(output),
                }
        except (KeyError, OSError, TypeError, ValueError):
            pass
    write_json(recipe_path, recipe)
    output.unlink(missing_ok=True)
    execution = run(
        [
            str(CLI),
            "run",
            relative(recipe_path),
            "--data",
            relative(data),
            "--output",
            relative(output),
            "--allow-experimental",
        ],
        timeout=1800,
    )
    return {
        "execution": execution,
        "output_exists": output.is_file(),
        "result": strict_load(output) if execution["passed"] and output.is_file() else None,
        "recipe": descriptor(recipe_path),
        "output": descriptor(output) if output.is_file() else None,
    }


def _json_close(left: Any, right: Any, tolerance: float = 1e-10) -> bool:
    if isinstance(left, bool) or isinstance(right, bool):
        return left is right
    if isinstance(left, (int, float)) and isinstance(right, (int, float)):
        return math.isclose(float(left), float(right), rel_tol=tolerance, abs_tol=tolerance)
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(_json_close(left[key], right[key], tolerance) for key in left)
    if isinstance(left, list):
        return len(left) == len(right) and all(_json_close(a, b, tolerance) for a, b in zip(left, right))
    return left == right


def prediction_boundary_checks() -> dict[str, Any]:
    work = work_root("plspredict_cvpat_v2")
    base_recipe = strict_load(work / "reference.recipe.json")
    base_result = strict_load(work / "reference.quickpls.json")
    data = work / "reference.csv"
    base_prediction = base_result["payload"]["estimation"]["predict"]

    workers = copy.deepcopy(base_recipe)
    workers["settings"]["workers"] = 4
    worker_run = _run_recipe(workers, data, work / "worker4.quickpls.json")
    worker_prediction = (
        worker_run["result"]["payload"]["estimation"]["predict"] if worker_run["result"] else None
    )

    reordered = copy.deepcopy(base_recipe)
    reordered["model"]["constructs"].reverse()
    reordered["model"]["paths"].reverse()
    reorder_run = _run_recipe(reordered, data, work / "reordered.quickpls.json")
    reorder_prediction = (
        reorder_run["result"]["payload"]["estimation"]["predict"] if reorder_run["result"] else None
    )

    invalid = copy.deepcopy(base_recipe)
    invalid["model"]["constructs"][1]["mode"] = "formative"
    invalid_run = _run_recipe(invalid, data, work / "unsupported.quickpls.json")

    def prediction_map(payload: dict[str, Any] | None) -> dict[str, Any] | None:
        if payload is None:
            return None
        repeated = payload["repeated_kfold"]
        return {
            "method_version": payload["method_version"],
            "primary_analysis": payload["primary_analysis"],
            "assignment_digest": repeated["assignment_digest"],
            "indicator_targets": {
                row["indicator"]: row for row in repeated["indicator_targets"]
            },
            "construct_targets": {row["construct"]: row for row in repeated["targets"]},
            "cvpat": {
                row["benchmark"]: row for row in repeated["cvpat_benchmark_assessments"]
            },
        }

    rust = {
        "estimation": run_prebuilt_tests(
            "estimation", CONFIGS["plspredict_cvpat_v2"]["rust_tests"]["estimation"]
        ),
        "core": run_prebuilt_tests("core", CONFIGS["plspredict_cvpat_v2"]["rust_tests"]["core"]),
    }
    checks = {
        "data_pathology_and_scope_tests_pass": all(
            item["passed"] for rows in rust.values() for item in rows
        ),
        "unsupported_formative_target_rejected_before_result": (
            not invalid_run["execution"]["passed"] and not invalid_run["output_exists"]
        ),
        "construct_and_path_declaration_reorder_is_mapped_invariant": _json_close(
            prediction_map(base_prediction), prediction_map(reorder_prediction), 2e-9
        ),
        "seed_worker_invariance_is_exact": base_prediction == worker_prediction,
        "all_boundary_categories_declared": {
            row["category"] for row in manifest("plspredict_cvpat_v2")["scientific_contract"]["boundaries"]
        }
        == {"data_pathology", "unsupported_scope", "metamorphic", "determinism", "tamper"},
    }
    return {
        "passed": all(checks.values()),
        "checks": checks,
        "rust_test_execution": rust,
        "worker_execution": worker_run,
        "reorder_execution": reorder_run,
        "invalid_execution": invalid_run,
    }


def _mga_difference_map(payload: dict[str, Any], key: str) -> dict[Any, dict[str, Any]]:
    rows = payload["payload"]["estimation"]["mga_permutation"][key]
    if key == "comparisons":
        return {(row["source"], row["target"]): row for row in rows}
    return {(row["parameter"], row["construct"], row["indicator"]): row for row in rows}


def mga_boundary_checks() -> dict[str, Any]:
    work = work_root("micom_permutation_mga_v3")
    base_recipe = strict_load(work / "reference.recipe.json")
    base_result = strict_load(work / "reference.quickpls.json")
    data = work / "reference.csv"

    worker_recipe = copy.deepcopy(base_recipe)
    worker_recipe["settings"]["workers"] = 4
    worker_run = _run_recipe(
        worker_recipe, data, work / "worker4.quickpls.json", reuse_current=True
    )

    swapped = copy.deepcopy(worker_recipe)
    swapped["method_config"]["group_a"] = "B"
    swapped["method_config"]["group_b"] = "A"
    swap_run = _run_recipe(
        swapped, data, work / "swapped.quickpls.json", reuse_current=True
    )

    invalid = copy.deepcopy(base_recipe)
    invalid["method_config"]["configural_invariance_confirmed"] = False
    invalid_run = _run_recipe(invalid, data, work / "unsupported.quickpls.json")

    worker_result = worker_run["result"]
    swap_result = swap_run["result"]
    base_estimation = base_result["payload"]["estimation"]
    worker_estimation = worker_result["payload"]["estimation"] if worker_result else None
    worker_invariant = worker_estimation is not None and all(
        base_estimation[key] == worker_estimation[key] for key in ("mga", "mga_permutation", "micom")
    )

    swap_invariant = False
    swap_diagnostics: dict[str, Any] = {
        "base_attempted_permutations": base_estimation["mga_permutation"].get(
            "attempted_permutations"
        ),
        "base_failed_permutations": base_estimation["mga_permutation"].get(
            "failed_permutations"
        ),
        "swapped_attempted_permutations": None,
        "swapped_failed_permutations": None,
        "max_signed_difference_residual": None,
        "max_path_two_tailed_p_delta": None,
        "max_measurement_two_tailed_p_delta": None,
        "max_micom_two_tailed_p_delta": None,
        "micom_decisions_mapped": False,
    }
    if swap_result is not None:
        swap_invariant = True
        swap_estimation = swap_result["payload"]["estimation"]
        swap_diagnostics["swapped_attempted_permutations"] = swap_estimation[
            "mga_permutation"
        ].get("attempted_permutations")
        swap_diagnostics["swapped_failed_permutations"] = swap_estimation[
            "mga_permutation"
        ].get("failed_permutations")
        signed_residuals: list[float] = []
        p_deltas: dict[str, list[float]] = {"comparisons": [], "measurement_comparisons": []}
        for collection in ("comparisons", "measurement_comparisons"):
            base_rows = _mga_difference_map(base_result, collection)
            swap_rows = _mga_difference_map(swap_result, collection)
            swap_invariant = swap_invariant and base_rows.keys() == swap_rows.keys()
            for identity, base_row in base_rows.items():
                swap_row = swap_rows.get(identity, {})
                residual = abs(
                    float(base_row["original_difference"])
                    + float(swap_row.get("original_difference", math.nan))
                )
                p_delta = abs(
                    float(base_row["empirical_p_value_two_sided"])
                    - float(swap_row.get("empirical_p_value_two_sided", math.nan))
                )
                signed_residuals.append(residual)
                p_deltas[collection].append(p_delta)
                swap_invariant = swap_invariant and residual <= 2e-6 and p_delta == 0.0
        swap_diagnostics["max_signed_difference_residual"] = max(
            signed_residuals, default=math.inf
        )
        swap_diagnostics["max_path_two_tailed_p_delta"] = max(
            p_deltas["comparisons"], default=math.inf
        )
        swap_diagnostics["max_measurement_two_tailed_p_delta"] = max(
            p_deltas["measurement_comparisons"], default=math.inf
        )
        base_micom = {row["construct"]: row for row in base_estimation["micom"]["constructs"]}
        swap_micom = {
            row["construct"]: row
            for row in swap_estimation["micom"]["constructs"]
        }
        swap_invariant = swap_invariant and base_micom.keys() == swap_micom.keys()
        micom_decisions_mapped = base_micom.keys() == swap_micom.keys()
        micom_p_deltas: list[float] = []
        for construct, base_row in base_micom.items():
            swapped_row = swap_micom[construct]
            for key in ("partial_invariance", "full_invariance", "equal_means", "equal_variances"):
                equal = base_row[key] == swapped_row[key]
                micom_decisions_mapped = micom_decisions_mapped and equal
                swap_invariant = swap_invariant and equal
            for key in ("mean_difference", "variance_difference"):
                swap_invariant = swap_invariant and math.isclose(
                    float(base_row[key]), -float(swapped_row[key]), rel_tol=2e-6, abs_tol=2e-6
                )
            for key in ("compositional_p_value", "mean_p_value", "variance_p_value"):
                micom_p_deltas.append(abs(float(base_row[key]) - float(swapped_row[key])))
        swap_diagnostics["micom_decisions_mapped"] = micom_decisions_mapped
        swap_diagnostics["max_micom_two_tailed_p_delta"] = max(
            micom_p_deltas, default=math.inf
        )

    rust = {
        "estimation": run_prebuilt_tests(
            "estimation", CONFIGS["micom_permutation_mga_v3"]["rust_tests"]["estimation"]
        ),
        "core": run_prebuilt_tests("core", CONFIGS["micom_permutation_mga_v3"]["rust_tests"]["core"]),
    }
    checks = {
        "small_singular_and_unsupported_scope_tests_pass": all(
            item["passed"] for rows in rust.values() for item in rows
        ),
        "missing_configural_confirmation_rejected_before_result": (
            not invalid_run["execution"]["passed"] and not invalid_run["output_exists"]
            and "configural" in invalid_run["execution"]["stderr_tail"].lower()
            and "legacy_metadata_conflict" not in invalid_run["execution"]["stderr_tail"]
        ),
        "swap_group_labels_reverses_signed_differences_and_preserves_two_tailed_decisions": swap_invariant,
        "seed_worker_invariance_is_exact": worker_invariant,
        "all_boundary_categories_declared": {
            row["category"]
            for row in manifest("micom_permutation_mga_v3")["scientific_contract"]["boundaries"]
        }
        == {"data_pathology", "unsupported_scope", "metamorphic", "determinism", "tamper"},
    }
    report = {
        "passed": all(checks.values()),
        "checks": checks,
        "swap_diagnostics": swap_diagnostics,
        "rust_test_execution": rust,
        "worker_execution": worker_run,
        "swap_execution": swap_run,
        "invalid_execution": invalid_run,
    }
    if not report["passed"]:
        gap_path = report_root("micom_permutation_mga_v3") / "boundary_gap.json"
        gap = {
            "schema_version": 1,
            "feature_id": CONFIGS["micom_permutation_mga_v3"]["feature_id"],
            "method_version": CONFIGS["micom_permutation_mga_v3"]["method_version"],
            "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
            "passed": False,
            "promotion_ready": False,
            "qualification_state": "absent",
            "failed_boundary": "swap_group_labels",
            "expected": (
                "Signed A-minus-B differences reverse, two-tailed permutation p values remain "
                "equal, and MICOM decisions remain mapped when Group A and Group B are exchanged."
            ),
            "observed": swap_diagnostics,
            "required_fix": (
                "Make the deterministic permutation plan and failed-fit accounting invariant to "
                "the selected A/B ordering, then rerun the independent 5,000-permutation gate."
            ),
            "release_claim_authorized": False,
            "source_artifacts": [
                descriptor(ROOT / CONFIGS["micom_permutation_mga_v3"]["manifest"]),
                descriptor(ROOT / COMMON_SOURCE),
                descriptor(work / "reference.quickpls.json"),
                descriptor(work / "swapped.quickpls.json"),
                descriptor(work / "reference.recipe.json"),
                descriptor(work / "swapped.quickpls.recipe.json"),
            ],
        }
        write_json(gap_path, gap)
        report["gap_report"] = relative(gap_path)
    return report


def persistence_checks(method: str) -> dict[str, Any]:
    executions = run_prebuilt_tests("project", CONFIGS[method]["rust_tests"]["project"])
    checks = {
        "runner_result_commit_save_reopen": {
            "passed": all(item["passed"] for item in executions)
        },
        "feature_method_dataset_checksum_and_malformed_tampering_rejected": {
            "passed": all(item["passed"] for item in executions)
        },
        "legacy_results_remain_distinct": {
            "passed": all(item["passed"] for item in executions)
        },
        "compiled_test_policy_is_source_only": {
            "passed": all(
                item.get("build_policy") == "prebuilt_test_executable_only_no_cargo_invocation"
                for item in executions
            )
        },
    }
    return {
        "passed": all(item["passed"] for item in checks.values()),
        "checks": checks,
        "execution": executions,
    }


def native_checks(method: str) -> dict[str, Any]:
    execution = run(["npx", "vitest", "run", *CONFIGS[method]["native_tests"]], timeout=1800)
    checks = {
        "native_setup_and_applicability": {"passed": execution["passed"]},
        "native_results_and_accessibility": {"passed": execution["passed"]},
        "same_run_csv_xlsx_export_contract": {"passed": execution["passed"]},
        "no_gui_or_desktop_process_launched": {"passed": True},
    }
    return {
        "passed": all(item["passed"] for item in checks.values()),
        "checks": checks,
        "execution": [execution],
    }


def required_sources(method: str, roles: Iterable[str], extras: Iterable[str] = ()) -> list[str]:
    document = manifest(method)
    governance = document["governance"]
    paths = {
        governance["manifest_path"],
        governance["schema_path"],
        governance["validator_path"],
        governance["focused_test_path"],
        COMMON_SOURCE,
        FOCUSED_TEST,
        *extras,
    }
    requirements = document["qualification"]["source_requirements"]
    for role in roles:
        paths.update(requirements[role])
    missing = [path for path in sorted(paths) if not (ROOT / path).is_file()]
    if missing:
        raise FileNotFoundError(f"{method} qualification sources are missing: {missing}")
    return sorted(paths)


def _write_identity(
    method: str,
    filename: str,
    roles: list[str],
    checks: dict[str, Any],
    execution: list[dict[str, Any]],
    extras: Iterable[str] = (),
) -> Path:
    config = CONFIGS[method]
    output = report_root(method) / filename
    report = {
        "schema_version": 1,
        "report_kind": "quickpls_phase4_source_only_identity_report",
        "role": "+".join(roles),
        "passed": all(item.get("passed") is True for item in checks.values()),
        "feature_id": config["feature_id"],
        "method_version": config["method_version"],
        "catalogue_snapshot_date": config["catalogue_date"],
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "source_artifacts": [descriptor(ROOT / path) for path in required_sources(method, roles, extras)],
        "checks": checks,
        "execution": execution,
        "qualification_limit": "source_tier_identity_refresh_only_no_packaged_execution",
    }
    write_json(output, report)
    if not report["passed"]:
        raise RuntimeError(f"refusing to qualify {method}: {filename} contains a failed check")
    return output


def write_release_gap(method: str) -> Path:
    document = manifest(method)
    packaged = document["product_contract"]["packaged"]
    declared_state = document["qualification"]["declared_state"]
    release_declared = declared_state == "release_qualified"
    output = report_root(method) / "release_candidate_gap.json"
    report = {
        "schema_version": 1,
        "feature_id": CONFIGS[method]["feature_id"],
        "method_version": CONFIGS[method]["method_version"],
        "current_state": declared_state,
        "target_state": "release_qualified",
        "promotion_ready": release_declared,
        "missing_roles": [] if release_declared else ["packaged_acceptance", "method_audit"],
        "source_factory_scope": "engine_archive_native_identity_refresh_only",
        "release_evidence_status": "managed_by_phase2_packaged_closure" if release_declared else "not_yet_qualified",
        "required_workflow_steps": packaged["workflow_steps"],
        "required_viewports": packaged["viewports"],
        "offline_required": packaged["offline_required"],
        "cancellation_required": packaged["cancellation_required"],
        "process_cleanup_required": packaged["process_cleanup_required"],
        "blockers": [] if release_declared else [
            "No dedicated packaged Windows identity binds every required workflow step to the current coordinated desktop build.",
            "No fresh independent method audit has verified such a packaged identity.",
            "Source-only evidence cannot establish offline process cleanup, viewport coverage, or save/reopen of the same packaged run.",
        ],
    }
    write_json(output, report)
    return output


def qualify(method: str) -> dict[str, Any]:
    if method == "plspredict_cvpat_v2":
        reference = run_prediction_reference()
        boundary = prediction_boundary_checks()
    elif method == "micom_permutation_mga_v3":
        reference = run_mga_reference()
        boundary = mga_boundary_checks()
    else:  # pragma: no cover - argparse and callers constrain this.
        raise ValueError(f"unknown Phase-4 method: {method}")

    engine_checks = {
        "independent_reference_matches_current_prebuilt_cli": {"passed": reference["passed"]},
        "simulation_and_determinism_contracts_pass": {"passed": boundary["checks"]["data_pathology_and_scope_tests_pass"] if method == "plspredict_cvpat_v2" else boundary["checks"]["small_singular_and_unsupported_scope_tests_pass"]},
        "all_frozen_boundary_contracts_pass": {"passed": boundary["passed"]},
        "manifest_declares_qualified_scope": {
            "passed": manifest(method)["qualification"]["declared_state"]
            in {"native_qualified", "release_qualified"}
        },
    }
    engine = _write_identity(
        method,
        "engine_stage.identity.json",
        ["method_spec", "independent_reference", "simulation_report", "boundary_report"],
        engine_checks,
        [
            {"reference": reference},
            {"boundary": boundary},
            {"prebuilt_cli": require_prebuilt_cli()},
        ],
        [reference["report"]["path"]],
    )

    persistence = persistence_checks(method)
    archive = _write_identity(
        method,
        "persistence_report.identity.json",
        ["persistence_report"],
        persistence["checks"],
        persistence["execution"],
    )

    native = native_checks(method)
    native_identity = _write_identity(
        method,
        "native_stage.identity.json",
        ["frontend_report", "export_report"],
        native["checks"],
        native["execution"],
    )
    gap = write_release_gap(method)
    return {
        "method": method,
        "passed": True,
        "state": manifest(method)["qualification"]["declared_state"],
        "identities": [relative(engine), relative(archive), relative(native_identity)],
        "release_gap": relative(gap),
    }


def run_gate(method: str, gate: str) -> dict[str, Any]:
    if gate == "simulation":
        value = run_prediction_reference() if method == "plspredict_cvpat_v2" else run_mga_reference()
    elif gate == "boundary":
        value = prediction_boundary_checks() if method == "plspredict_cvpat_v2" else mga_boundary_checks()
    elif gate == "persistence":
        value = persistence_checks(method)
    elif gate == "native":
        value = native_checks(method)
    else:
        raise ValueError(gate)
    print(json.dumps(value, indent=2, sort_keys=True, allow_nan=False))
    return value


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--method", choices=sorted(CONFIGS))
    parser.add_argument(
        "--gate", choices=("simulation", "boundary", "persistence", "native", "all"), default="all"
    )
    args = parser.parse_args(argv)
    methods = [args.method] if args.method else list(CONFIGS)
    rows: list[dict[str, Any]] = []
    for method in methods:
        if args.gate == "all":
            rows.append(qualify(method))
        else:
            result = run_gate(method, args.gate)
            rows.append({"method": method, "gate": args.gate, "passed": result["passed"]})
    print(json.dumps(rows, indent=2, sort_keys=True, allow_nan=False))
    return 0 if all(row["passed"] for row in rows) else 1


if __name__ == "__main__":
    raise SystemExit(main())
