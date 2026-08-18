#!/usr/bin/env python3
"""Generate current-byte, method-specific OLS v1 promotion evidence.

Only engine, archive, and native-source stages are implemented here. The
runner never launches the desktop and cannot produce packaged acceptance.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import os
import subprocess
import tempfile
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, Iterable, Sequence

import numpy as np

from ols_v1_reference import compare_quickpls, fit_ols_hc3


ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = "validation/methods/ols_v1.manifest.json"
MANIFEST = ROOT / MANIFEST_PATH
OUTPUT = ROOT / "validation/results/method_factory/ols_v1"
FEATURE_ID = "qpls3.standalone.ols"
METHOD_VERSION = "regression_ols_v1"
CATALOGUE_DATE = "2026-08-12"
FACTORY_SOURCE = "validation/ols_v1_factory.py"
REFERENCE_SOURCE = "validation/ols_v1_reference.py"
FOCUSED_TEST = "validation/test_ols_v1_factory.py"
NATIVE_TEST_FILES = [
    "src/native/nativeOls.test.ts",
    "src/native/nativeAnalysisRecipe.test.ts",
    "src/native/NativeCalculationDialog.test.ts",
    "src/native/nativeResults.test.ts",
    "src/native/nativeExportTables.test.ts",
]


def cli_source_paths() -> list[str]:
    """Return the complete local Rust source closure used by the release CLI."""

    paths = {"Cargo.lock", "Cargo.toml", "crates/qpls-cli/Cargo.toml"}
    paths.update(
        repository_path(path)
        for path in (ROOT / "crates/qpls-cli/src").rglob("*.rs")
        if path.is_file()
    )
    for crate in (
        "qpls-assessment",
        "qpls-core",
        "qpls-data",
        "qpls-estimation",
        "qpls-project",
        "qpls-resampling",
        "qpls-runner",
    ):
        crate_root = ROOT / "crates" / crate
        paths.add(repository_path(crate_root / "Cargo.toml"))
        paths.update(
            repository_path(path)
            for path in (crate_root / "src").rglob("*.rs")
            if path.is_file()
        )
    development_slices = ROOT / "validation/development_slices.json"
    if development_slices.is_file():
        paths.add(repository_path(development_slices))
    return sorted(paths)


def strict_json(path: Path) -> dict[str, Any]:
    def pairs(rows: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in rows:
            if key in result:
                raise ValueError(f"duplicate JSON key: {key}")
            result[key] = value
        return result

    with path.open("r", encoding="utf-8-sig") as handle:
        value = json.load(
            handle,
            object_pairs_hook=pairs,
            parse_constant=lambda token: (_ for _ in ()).throw(
                ValueError(f"non-finite JSON value: {token}")
            ),
        )
    if not isinstance(value, dict):
        raise ValueError(f"JSON root must be an object: {path}")
    return value


def repository_path(path: Path) -> str:
    return path.resolve().relative_to(ROOT.resolve()).as_posix()


def descriptor(path: Path) -> dict[str, Any]:
    payload = path.read_bytes()
    return {
        "path": repository_path(path),
        "size": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }


def executable(name: str) -> str:
    return f"{name}.cmd" if os.name == "nt" and name in {"npx", "npm"} else name


def run(command: Sequence[str], timeout: int = 900) -> subprocess.CompletedProcess[str]:
    invoked = [executable(command[0]), *command[1:]]
    return subprocess.run(
        invoked,
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
        env={**os.environ, "CARGO_BUILD_JOBS": "1"},
    )


def execution(completed: subprocess.CompletedProcess[str], command: Sequence[str]) -> dict[str, Any]:
    return {
        "command": list(command),
        "returncode": completed.returncode,
        "stdout_tail": completed.stdout[-4000:],
        "stderr_tail": completed.stderr[-4000:],
    }


def manifest() -> dict[str, Any]:
    document = strict_json(MANIFEST)
    feature = document["feature"]
    expected = {"id": FEATURE_ID, "method_version": METHOD_VERSION, "catalogue_snapshot_date": CATALOGUE_DATE}
    for key, value in expected.items():
        if feature.get(key) != value:
            raise ValueError(f"OLS manifest {key} mismatch: {feature.get(key)!r}")
    return document


def source_paths(roles: Iterable[str], extras: Iterable[str] = ()) -> list[str]:
    document = manifest()
    governance = document["governance"]
    paths = {
        governance["manifest_path"], governance["schema_path"],
        governance["validator_path"], governance["focused_test_path"],
        FACTORY_SOURCE, REFERENCE_SOURCE, FOCUSED_TEST, *extras,
    }
    requirements = document["qualification"]["source_requirements"]
    for role in roles:
        paths.update(requirements[role])
    missing = [path for path in sorted(paths) if not (ROOT / path).is_file()]
    if missing:
        raise FileNotFoundError(f"OLS factory source files are missing: {missing}")
    return sorted(paths)


def write_identity(
    filename: str,
    roles: list[str],
    checks: dict[str, Any],
    *,
    extras: Iterable[str] = (),
    command_evidence: list[dict[str, Any]] | None = None,
) -> Path:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    path = OUTPUT / filename
    payload = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": "+".join(roles),
        "passed": all(bool(value.get("passed")) for value in checks.values()),
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_DATE,
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "source_artifacts": [descriptor(ROOT / source) for source in source_paths(roles, extras)],
        "checks": checks,
        "execution": command_evidence or [],
    }
    path.write_text(json.dumps(payload, indent=2, sort_keys=True, allow_nan=False) + "\n", encoding="utf-8")
    return path


def cli_path() -> Path:
    configured = os.environ.get("QUICKPLS_CLI_PATH")
    candidates = [
        Path(configured) if configured else None,
        ROOT / "target/release/qpls.exe",
        ROOT / "target/debug/qpls.exe",
    ]
    for candidate in candidates:
        if candidate is not None and candidate.is_file():
            candidate.resolve().relative_to(ROOT.resolve())
            newer = [
                relative
                for relative in cli_source_paths()
                if (ROOT / relative).stat().st_mtime_ns > candidate.stat().st_mtime_ns
            ]
            if newer:
                continue
            return candidate
    raise FileNotFoundError("a coordinated QuickPLS CLI is required; this runner never builds it")


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0]))
        writer.writeheader()
        writer.writerows(rows)


def fingerprint(cli: Path, csv_path: Path, directory: Path, name: str) -> str:
    project = directory / f"{name}.qpls"
    imported = run([str(cli), "import", str(csv_path), str(project), "--name", name])
    if imported.returncode != 0:
        raise RuntimeError(f"dataset import failed: {imported.stderr}")
    inspected = run([str(cli), "inspect", str(project), "--json"])
    if inspected.returncode != 0:
        raise RuntimeError(f"dataset inspection failed: {inspected.stderr}")
    return json.loads(inspected.stdout)["datasets"][0]["fingerprint"]


def recipe(fingerprint_value: str, name: str, predictors: list[str], controls: list[str]) -> dict[str, Any]:
    namespace = uuid.UUID("86912d57-d57f-4a51-94be-1bb7658763d0")
    return {
        "schema_version": 3,
        "id": str(uuid.uuid5(namespace, f"recipe:{name}")),
        "created_at": "2026-08-13T00:00:00Z",
        "dataset_fingerprint": fingerprint_value,
        "model": {
            "id": str(uuid.uuid5(namespace, f"model:{name}")),
            "name": "OLS v1 factory fixture", "constructs": [], "paths": [],
            "controls": [], "higher_order_constructs": [], "interactions": [],
        },
        "settings": {
            "method": "regression", "weighting_scheme": "path", "tolerance": 1e-9,
            "max_iterations": 3000, "bootstrap_samples": 0,
            "studentized_inner_samples": 0, "permutation_samples": 0,
            "seed": 20260813, "workers": 1, "confidence_level": 0.95,
            "preprocessing": "unstandardized", "missing_data": "listwise_deletion",
            "case_weight_column": None,
        },
        "method_config": {
            "kind": "regression", "outcome": "y", "predictors": predictors,
            "controls": controls, "model": {"type": "ols", "robust_se": "hc3"},
        },
        "metadata": {"status": "validated_regression_ols_v1_bounded_scope", "fixture": name},
    }


def run_ols(
    cli: Path,
    directory: Path,
    name: str,
    rows: list[dict[str, Any]],
    predictors: list[str],
    controls: list[str],
    mutate: Callable[[dict[str, Any]], None] | None = None,
) -> dict[str, Any]:
    csv_path = directory / f"{name}.csv"
    recipe_path = directory / f"{name}.recipe.json"
    result_path = directory / f"{name}.quickpls.json"
    write_csv(csv_path, rows)
    document = recipe(fingerprint(cli, csv_path, directory, name), name, predictors, controls)
    if mutate is not None:
        mutate(document)
    recipe_path.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")
    command = [
        str(cli), "run", str(recipe_path), "--data", str(csv_path),
        "--output", str(result_path), "--allow-experimental",
    ]
    completed = run(command)
    result = strict_json(result_path) if completed.returncode == 0 and result_path.is_file() else None
    return {
        "passed": completed.returncode == 0,
        "result": result,
        "command": execution(completed, command),
        "message": (completed.stdout + "\n" + completed.stderr).strip(),
    }


def generated_rows(seed: int, n: int, correlation: float, heteroskedastic: bool, missing: bool) -> list[dict[str, Any]]:
    rng = np.random.default_rng(seed)
    x1 = rng.normal(size=n)
    x2 = correlation * x1 + math.sqrt(1.0 - correlation * correlation) * rng.normal(size=n)
    control = rng.normal(size=n)
    scale = 0.22 * (1.0 + 0.8 * np.abs(x1)) if heteroskedastic else np.full(n, 0.28)
    y = 0.8 + 0.45 * x1 - 0.3 * x2 + 0.2 * control + rng.normal(size=n) * scale
    rows: list[dict[str, Any]] = []
    for index in range(n):
        row: dict[str, Any] = {
            "y": f"{y[index]:.15g}", "x1": f"{x1[index]:.15g}",
            "x2": f"{x2[index]:.15g}", "control": f"{control[index]:.15g}",
            "label": f"case-{index}", "weight": "1",
        }
        if missing and index % 13 == 0:
            row["x2"] = ""
        if missing and index % 19 == 0:
            row["y"] = ""
        rows.append(row)
    return rows


def regression_payload(result: dict[str, Any]) -> dict[str, Any]:
    return result["payload"]["estimation"]["regression"]


def engine_checks(cli: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    commands: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="quickpls-ols-v1-") as temporary:
        directory = Path(temporary)
        hand_rows = [
            {
                "y": 1.25 + 0.75 * x - 0.5 * z + 0.1 * x * z,
                "x1": x, "x2": z, "control": 0.0, "label": "hand", "weight": 1.0,
            }
            for x in (-3.0, -1.0, 1.0, 3.0)
            for z in (-1.0, 1.0)
        ]
        hand = run_ols(cli, directory, "hand", hand_rows, ["x1", "x2"], [])
        commands.append(hand["command"])
        hand_regression = regression_payload(hand["result"]) if hand["passed"] else {}
        hand_estimates = {row["term"]: row["estimate"] for row in hand_regression.get("coefficients", [])}
        hand_reference = fit_ols_hc3(hand_rows, "y", ["x1", "x2"])
        hand_parity = compare_quickpls(hand_regression, hand_reference) if hand["passed"] else {"passed": False}

        specs = [
            ("baseline", 64, 0.15, False, False),
            ("heteroskedastic", 96, 0.25, True, False),
            ("correlated", 128, 0.82, True, False),
            ("listwise_missing", 90, 0.55, True, True),
        ]
        parity_rows: list[dict[str, Any]] = []
        errors_by_term: dict[str, list[float]] = {term: [] for term in ("intercept", "x1", "x2", "control")}
        coverage: list[bool] = []
        truth = {"intercept": 0.8, "x1": 0.45, "x2": -0.3, "control": 0.2}
        for scenario_index, (scenario, n, corr, hetero, missing) in enumerate(specs):
            for replicate in range(8):
                rows = generated_rows(910_000 + scenario_index * 100 + replicate, n, corr, hetero, missing)
                observed = run_ols(cli, directory, f"simulation_{scenario}_{replicate}", rows, ["x1", "x2"], ["control"])
                commands.append(observed["command"])
                if not observed["passed"]:
                    parity_rows.append({"scenario": scenario, "replicate": replicate, "passed": False})
                    continue
                regression = regression_payload(observed["result"])
                reference = fit_ols_hc3(rows, "y", ["x1", "x2"], ["control"])
                parity_rows.append({"scenario": scenario, "replicate": replicate, **compare_quickpls(regression, reference)})
                for coefficient in regression["coefficients"]:
                    term = coefficient["term"]
                    errors_by_term[term].append(coefficient["estimate"] - truth[term])
                    coverage.append(
                        coefficient["confidence_interval_lower"] <= truth[term]
                        <= coefficient["confidence_interval_upper"]
                    )

        max_bias = max(abs(float(np.mean(values))) for values in errors_by_term.values())
        coverage_rate = sum(coverage) / len(coverage)
        boundary_rows = generated_rows(930_001, 36, 0.2, False, False)
        rank_rows = [dict(row, x2=str(2.0 * float(row["x1"]))) for row in boundary_rows]
        high_leverage_rows = [
            {
                "y": y,
                "x1": x,
                "x2": 0.0,
                "control": 0.0,
                "label": f"high-leverage-{index}",
                "weight": 1.0,
            }
            for index, (y, x) in enumerate(
                [(0.0, 0.0), (0.2, 0.0), (-0.1, 0.0), (1.0, 1.0)]
            )
        ]
        constant_predictor_rows = [dict(row, x1="1") for row in boundary_rows]
        residual_df_rows = [
            {
                "y": y,
                "x1": x1,
                "x2": x2,
                "control": 0.0,
                "label": f"residual-df-{index}",
                "weight": 1.0,
            }
            for index, (y, x1, x2) in enumerate(
                [(0.0, 0.0, 0.0), (1.0, 1.0, 0.0), (2.0, 0.0, 1.0)]
            )
        ]
        boundary_cases: dict[str, dict[str, Any]] = {}

        def rejected(
            name: str,
            rows: list[dict[str, Any]],
            predictors: list[str],
            controls: list[str],
            mutate: Callable[[dict[str, Any]], None] | None,
            tokens: tuple[str, ...],
        ) -> None:
            outcome = run_ols(cli, directory, f"boundary_{name}", rows, predictors, controls, mutate)
            commands.append(outcome["command"])
            lowered = outcome["message"].lower()
            boundary_cases[name] = {
                "passed": not outcome["passed"] and any(token in lowered for token in tokens),
                "returncode": outcome["command"]["returncode"],
                "matched_tokens": [token for token in tokens if token in lowered],
                "message_tail": outcome["message"][-600:],
            }

        rejected("rank_deficient", rank_rows, ["x1", "x2"], [], None, ("rank", "singular"))
        rejected("duplicate_role", boundary_rows, ["x1", "x1"], [], None, ("duplicate", "distinct"))
        rejected("nonnumeric", boundary_rows, ["label"], [], None, ("numeric", "number"))
        rejected(
            "unsupported_hc0", boundary_rows, ["x1", "x2"], [],
            lambda doc: doc["method_config"]["model"].update(robust_se="hc0"),
            ("hc3", "unknown variant", "robust"),
        )
        rejected(
            "case_weight", boundary_rows, ["x1", "x2"], [],
            lambda doc: doc["settings"].update(case_weight_column="weight"),
            ("case weight", "case_weight", "weighted"),
        )
        rejected(
            "resampling", boundary_rows, ["x1", "x2"], [],
            lambda doc: doc["settings"].update(bootstrap_samples=100),
            ("bootstrap", "resampling"),
        )
        rejected("insufficient_rows", boundary_rows[:2], ["x1"], [], None, ("degrees", "observations", "rows", "sample"))
        rejected(
            "high_leverage_hc3",
            high_leverage_rows,
            ["x1"],
            [],
            None,
            ("hc3", "1-h", "leverage"),
        )
        rejected(
            "constant_predictor",
            constant_predictor_rows,
            ["x1"],
            [],
            None,
            ("constant", "rank", "singular"),
        )
        rejected(
            "nonpositive_residual_df",
            residual_df_rows,
            ["x1", "x2"],
            [],
            None,
            ("positive residual degrees of freedom", "residual degrees"),
        )

        missing_rows = [dict(row) for row in boundary_rows[:8]]
        missing_rows[1]["y"] = ""
        missing_rows[5]["x2"] = ""
        missing = run_ols(
            cli,
            directory,
            "listwise_complete_case_counts",
            missing_rows,
            ["x1", "x2"],
            ["control"],
        )
        commands.append(missing["command"])
        missing_estimation = (
            missing["result"]["payload"]["estimation"] if missing["passed"] else {}
        )
        missing_regression = regression_payload(missing["result"]) if missing["passed"] else {}
        missing_reference = fit_ols_hc3(
            missing_rows, "y", ["x1", "x2"], ["control"]
        )
        missing_count_contract = {
            "passed": missing["passed"]
            and missing_estimation.get("used_observations") == 6
            and missing_estimation.get("omitted_observations") == 2
            and missing_regression.get("observations") == 6
            and len(missing_regression.get("predictions", [])) == 6
            and missing_reference["observations"] == 6
            and missing_reference["omitted_observations"] == 2,
            "quickpls_used": missing_estimation.get("used_observations"),
            "quickpls_omitted": missing_estimation.get("omitted_observations"),
            "reference_used": missing_reference["observations"],
            "reference_omitted": missing_reference["omitted_observations"],
        }

        baseline = run_ols(cli, directory, "metamorphic_baseline", boundary_rows, ["x1", "x2"], ["control"])
        repeat = run_ols(cli, directory, "metamorphic_repeat", boundary_rows, ["x1", "x2"], ["control"])
        reordered = run_ols(cli, directory, "metamorphic_reordered", boundary_rows, ["x2", "x1"], ["control"])
        row_reversed = run_ols(cli, directory, "metamorphic_rows_reversed", list(reversed(boundary_rows)), ["x1", "x2"], ["control"])
        commands.extend(row["command"] for row in (baseline, repeat, reordered, row_reversed))
        regressions = [regression_payload(row["result"]) if row["passed"] else {} for row in (baseline, repeat, reordered, row_reversed)]

        def coefficient_map(regression: dict[str, Any]) -> dict[str, tuple[float, float]]:
            return {row["term"]: (row["estimate"], row["standard_error"]) for row in regression.get("coefficients", [])}

        def mapped_delta(left: dict[str, Any], right: dict[str, Any]) -> float:
            left_map, right_map = coefficient_map(left), coefficient_map(right)
            if set(left_map) != set(right_map) or not left_map:
                return math.inf
            values = [abs(left_map[term][index] - right_map[term][index]) for term in left_map for index in (0, 1)]
            values.extend(abs(left["fit"][key] - right["fit"][key]) for key in ("r_squared", "adjusted_r_squared", "f_statistic", "rmse"))
            return max(values)

        repeat_exact = regressions[0] == regressions[1]
        reorder_delta = mapped_delta(regressions[0], regressions[2])
        row_delta = mapped_delta(regressions[0], regressions[3])

    max_reference_delta = max((row.get("max_abs_difference", math.inf) for row in parity_rows), default=math.inf)
    checks = {
        "current_cli_bound": {
            "passed": True,
            "tested_cli": descriptor(cli),
            "build_policy": "prebuilt coordinated CLI; factory did not build or mutate product sources",
        },
        "hand_calculated_fixture": {
            "passed": hand["passed"]
            and abs(hand_estimates.get("intercept", math.inf) - 1.25) <= 1e-10
            and abs(hand_estimates.get("x1", math.inf) - 0.75) <= 1e-10
            and abs(hand_estimates.get("x2", math.inf) + 0.5) <= 1e-10
            and hand_parity.get("passed") is True,
            "estimates": hand_estimates,
            "independent_parity": hand_parity,
        },
        "independent_numpy_scipy_reference": {
            "passed": bool(parity_rows) and all(row.get("passed") is True for row in parity_rows),
            "scenario_replicates": len(parity_rows),
            "max_abs_difference": max_reference_delta,
            "runtime_policy": "development_validation_only",
            "independence": (
                "The reference imports only NumPy/SciPy and re-expresses the "
                "published matrix and Student-t equations; it does not import QuickPLS code."
            ),
        },
        "bounded_recovery_and_coverage": {
            "passed": max_bias <= 0.12 and 0.75 <= coverage_rate <= 1.0,
            "replicates": len(parity_rows),
            "coefficient_mean_bias": {key: float(np.mean(values)) for key, values in errors_by_term.items()},
            "maximum_absolute_mean_bias": max_bias,
            "hc3_95_percent_coverage": coverage_rate,
            "acceptance": {"max_absolute_mean_bias": 0.12, "coverage_interval": [0.75, 1.0]},
        },
        "failure_boundaries": {"passed": all(row["passed"] for row in boundary_cases.values()), "cases": boundary_cases},
        "listwise_complete_case_accounting": missing_count_contract,
        "determinism_and_metamorphic_invariance": {
            "passed": repeat_exact and reorder_delta <= 1e-7 and row_delta <= 1e-7,
            "repeat_payload_exact": repeat_exact,
            "predictor_reorder_max_abs_difference": reorder_delta,
            "row_reorder_max_abs_difference": row_delta,
        },
        "method_identity": {
            "passed": hand_regression.get("method_version") == METHOD_VERSION
            and hand_regression.get("regression_type") == "ols"
            and hand_regression.get("process") is None,
            "method_version": hand_regression.get("method_version"),
            "regression_type": hand_regression.get("regression_type"),
        },
    }
    return checks, commands


def generate_engine() -> Path:
    cli = cli_path()
    checks, commands = engine_checks(cli)
    return write_identity(
        "engine_stage.identity.json",
        ["method_spec", "independent_reference", "simulation_report", "boundary_report"],
        checks,
        extras=[repository_path(cli), *cli_source_paths()],
        command_evidence=commands,
    )


def generate_archive() -> Path:
    command = [
        "cargo", "test", "-p", "qpls-project",
        "runner_generated_ols_v1_commits_saves_reopens_and_rejects_contract_tampering",
        "--", "--nocapture",
    ]
    completed = run(command, timeout=1800)
    combined = completed.stdout + "\n" + completed.stderr
    project_source = (ROOT / "crates/qpls-project/src/lib.rs").read_text(encoding="utf-8")
    source_contract = all(
        token in project_source
        for token in (
            "runner_generated_ols_v1_commits_saves_reopens_and_rejects_contract_tampering",
            "tampered_version",
            "tampered_statistic",
            "tampered_fit",
            "tampered_prediction",
            "mismatched_recipe",
            "tampered_for_save",
        )
    )
    checks = {
        "current_project_archive_contract": {
            "passed": completed.returncode == 0
            and "runner_generated_ols_v1_commits_saves_reopens_and_rejects_contract_tampering ... ok" in combined
            and "1 passed" in combined
            and "0 failed" in combined
            and source_contract,
            "focused_test": "runner_generated_ols_v1_commits_saves_reopens_and_rejects_contract_tampering",
            "proves": [
                "runner-generated OLS result commit", "save and reopen",
                "method-version, statistic, fit, prediction, recipe-pairing, and save-time tamper rejection",
            ],
            "source_contract_tokens_present": source_contract,
        }
    }
    return write_identity(
        "persistence_report.identity.json", ["persistence_report"], checks,
        command_evidence=[execution(completed, command)],
    )


def generate_native() -> Path:
    vitest_command = ["npx", "vitest", "run", *NATIVE_TEST_FILES, "--reporter=json"]
    vitest = run(vitest_command, timeout=1200)
    try:
        result = json.loads(vitest.stdout)
    except (json.JSONDecodeError, UnicodeError):
        result = {}
    raw_results = result.get("testResults") if isinstance(result, dict) else []
    observed_files = set()
    for row in raw_results if isinstance(raw_results, list) else []:
        value = row.get("name") if isinstance(row, dict) else None
        if not isinstance(value, str):
            continue
        path = Path(value)
        if not path.is_absolute():
            path = ROOT / path
        try:
            observed_files.add(repository_path(path))
        except ValueError:
            pass
    tsc_command = ["npx", "tsc", "-b", "--pretty", "false"]
    tsc = run(tsc_command, timeout=1200)
    expected_files = set(NATIVE_TEST_FILES)
    checks = {
        "focused_native_contract_tests": {
            "passed": vitest.returncode == 0
            and result.get("success") is True
            and observed_files == expected_files
            and result.get("numFailedTests") == 0
            and result.get("numPendingTests") == 0
            and isinstance(result.get("numTotalTests"), int)
            and result["numTotalTests"] > 0
            and result.get("numPassedTests") == result["numTotalTests"],
            "expected_files": sorted(expected_files),
            "observed_files": sorted(observed_files),
            "total_tests": result.get("numTotalTests"),
            "passed_tests": result.get("numPassedTests"),
        },
        "typescript_project_check": {"passed": tsc.returncode == 0},
        "scope_limit": {
            "passed": True,
            "packaged_desktop_executed": False,
            "claim": "current native source contracts only; packaged acceptance remains a separate release gate",
        },
    }
    return write_identity(
        "native_stage.identity.json", ["frontend_report", "export_report"], checks,
        command_evidence=[execution(vitest, vitest_command), execution(tsc, tsc_command)],
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("stages", nargs="*", choices=["engine", "archive", "native"])
    args = parser.parse_args(argv)
    stages = args.stages or ["engine", "archive", "native"]
    generators = {"engine": generate_engine, "archive": generate_archive, "native": generate_native}
    outputs = [generators[stage]() for stage in stages]
    reports = [strict_json(path) for path in outputs]
    for path, report in zip(outputs, reports):
        print(f"wrote {path} | passed={report['passed']}")
    return 0 if all(report["passed"] is True for report in reports) else 1


if __name__ == "__main__":
    raise SystemExit(main())
