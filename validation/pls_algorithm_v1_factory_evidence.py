"""Orchestrate fresh identity reports for PLS Algorithm v1 factory stages."""

from __future__ import annotations

import argparse
import csv
import json
import math
import shutil
import time
from pathlib import Path
from typing import Any

from pls_algorithm_v1_factory_common import (
    ROOT,
    WORK_ROOT,
    repository_path,
    run_command,
    run_pls,
    sha256_file,
    source_descriptors,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/pls_algorithm_v1_factory_evidence.py"
REFERENCE_WORK = WORK_ROOT / "independent_reference"
REFERENCE_TOLERANCE = 1e-6


def method_spec_report() -> bool:
    path = ROOT / "docs" / "methods" / "PLS_PM_V1.md"
    text = path.read_text(encoding="utf-8")
    required_fragments = [
        "recursive PLS path models",
        "reflective Mode A blocks",
        "formative Mode B blocks",
        "single-item blocks",
        "path and factor inner weighting",
        "PCA block weighting",
        "Rows missing any model indicator are removed listwise",
        "column-pivoted QR",
        "maximum absolute signed weight change",
        "indirect effects equal total minus direct",
        "No random initialization is used",
        "1e-6",
    ]
    fragments = {fragment: fragment in text for fragment in required_fragments}
    bounded_scope = (
        "Cyclic models and covariance-only inputs are rejected" in text
        and "assessment metrics" not in text.lower()
    )
    checks = {
        "passed": all(fragments.values()) and bounded_scope,
        "document": "docs/methods/PLS_PM_V1.md",
        "required_contract_fragments": fragments,
        "bounded_estimator_scope": bounded_scope,
    }
    report = write_identity_report(
        "method_spec",
        passed=checks["passed"],
        checks=checks,
        extras=[SOURCE],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def _construct(
    construct_id: str,
    indicators: list[str],
    *,
    mode: str = "reflective",
) -> dict[str, Any]:
    return {
        "id": construct_id,
        "name": construct_id,
        "short_name": construct_id,
        "mode": mode,
        "indicators": indicators,
    }


def _qpls_values(run: dict[str, Any]) -> dict[tuple[str, str, str, str], float]:
    estimation = run["estimation"]
    values: dict[tuple[str, str, str, str], float] = {}
    for row in estimation["paths"]:
        values[("path", row["source"], row["target"], "")] = float(
            row["coefficient"]
        )
    for row in estimation["outer_estimates"]:
        key = (row["construct"], "", row["indicator"])
        values[("loading", *key)] = float(row["loading"])
        values[("weight", *key)] = float(row["weight"])
    return values


def _read_csem_csv(path: Path) -> dict[str, dict[tuple[str, str, str, str], float]]:
    variants: dict[str, dict[tuple[str, str, str, str], float]] = {}
    with path.open(newline="", encoding="utf-8-sig") as handle:
        for row in csv.DictReader(handle):
            variant = row.get("variant") or "PUBLISHED_PATH_MODE_A"
            key = (row["kind"], row["source"], row["target"], row["indicator"])
            if key in variants.setdefault(variant, {}):
                raise ValueError(f"duplicate cSEM reference key for {variant}: {key}")
            variants[variant][key] = float(row["value"])
    if not variants or any(not rows for rows in variants.values()):
        raise ValueError(f"empty cSEM reference: {path}")
    return variants


def _read_plspm_json(path: Path) -> dict[str, dict[tuple[str, str, str, str], float]]:
    document = strict_load_json(path)
    variants: dict[str, dict[tuple[str, str, str, str], float]] = {}
    for variant in document["variants"]:
        label = variant["variant"]
        if label in variants:
            raise ValueError(f"duplicate python-plspm variant: {label}")
        values: dict[tuple[str, str, str, str], float] = {}
        for row in variant["paths"]:
            values[("path", row["source"], row["target"], "")] = float(row["value"])
        for row in variant["outer"]:
            values[("loading", row["construct"], "", row["indicator"])] = float(
                row["loading"]
            )
        variants[label] = values
    if not variants or any(not rows for rows in variants.values()):
        raise ValueError(f"empty python-plspm reference: {path}")
    return variants


def _read_numpy_json(path: Path) -> dict[tuple[str, str, str, str], float]:
    document = strict_load_json(path)
    values: dict[tuple[str, str, str, str], float] = {}
    for row in document["paths"]:
        values[("path", row["source"], row["target"], "")] = float(row["value"])
    for row in document["outer"]:
        key = (row["construct"], "", row["indicator"])
        values[("loading", *key)] = float(row["loading"])
        values[("weight", *key)] = float(row["weight"])
    if not values:
        raise ValueError(f"empty NumPy reference: {path}")
    return values


def _compare_values(
    *,
    engine: str,
    variant: str,
    actual: dict[tuple[str, str, str, str], float],
    expected: dict[tuple[str, str, str, str], float],
    tolerance: float = REFERENCE_TOLERANCE,
) -> dict[str, Any]:
    actual_keys = set(actual)
    expected_keys = set(expected)
    rows: list[dict[str, Any]] = []
    for key in sorted(actual_keys & expected_keys):
        observed = actual[key]
        reference = expected[key]
        difference = observed - reference
        rows.append(
            {
                "kind": key[0],
                "source": key[1],
                "target": key[2],
                "indicator": key[3],
                "quickpls": observed,
                "reference": reference,
                "difference": difference,
                "abs_diff": abs(difference),
                "passed": math.isfinite(difference) and abs(difference) <= tolerance,
            }
        )
    exact_membership = actual_keys == expected_keys
    passed = exact_membership and bool(rows) and all(row["passed"] for row in rows)
    return {
        "engine": engine,
        "variant": variant,
        "passed": passed,
        "tolerance": tolerance,
        "exact_key_membership": exact_membership,
        "missing_from_quickpls": [list(key) for key in sorted(expected_keys - actual_keys)],
        "unexpected_from_quickpls": [list(key) for key in sorted(actual_keys - expected_keys)],
        "compared_quantity_count": len(rows),
        "max_abs_diff": max((row["abs_diff"] for row in rows), default=None),
        "comparisons": rows,
    }


def _fresh_artifact(path: Path, started_ns: int) -> dict[str, Any]:
    if not path.is_file() or path.stat().st_size <= 0:
        return {"path": repository_path(path), "passed": False, "reason": "missing_or_empty"}
    stat = path.stat()
    # Windows filesystem timestamps can be slightly coarser than perf-counter boundaries.
    fresh = stat.st_mtime_ns >= started_ns - 2_000_000_000
    return {
        "path": repository_path(path),
        "passed": fresh,
        "size": stat.st_size,
        "sha256": sha256_file(path),
        "mtime_ns": stat.st_mtime_ns,
    }


def _require_command(
    command: list[str],
    executions: list[dict[str, Any]],
    *,
    timeout: int = 1800,
    env: dict[str, str] | None = None,
) -> None:
    completed, execution = run_command(command, timeout=timeout, env=env)
    executions.append(execution)
    if completed.returncode != 0:
        raise RuntimeError(
            f"independent reference command failed ({completed.returncode}): {command}"
        )


def _generated_run_artifacts(run: dict[str, Any]) -> list[str]:
    return [
        run["recipe"],
        run["output"],
        run["fingerprint_execution"]["project"],
    ]


def _csem_versions(paths: list[Path]) -> list[str]:
    versions: set[str] = set()
    for path in paths:
        with path.open(newline="", encoding="utf-8-sig") as handle:
            for row in csv.DictReader(handle):
                version = row.get("csem_version")
                if not version:
                    raise ValueError(f"cSEM reference omitted its engine version: {path}")
                versions.add(version)
    return sorted(versions)


def independent_reference_report() -> bool:
    static_sources = [
        SOURCE,
        "validation/Resolve-Rscript.ps1",
        "validation/r_csem_reference.R",
        "validation/r_csem_threecommonfactors_reference.R",
        "validation/python_plspm_reference.py",
        "validation/pls_pca_reference.py",
        "validation/fixtures/simple_reflective.csv",
        "crates/qpls-core/src/contract.rs",
        "crates/qpls-estimation/src/pls.rs",
        "crates/qpls-runner/src/lib.rs",
        "crates/qpls-cli/src/main.rs",
    ]
    REFERENCE_WORK.mkdir(parents=True, exist_ok=True)
    reference_root = REFERENCE_WORK.resolve()
    for child in REFERENCE_WORK.iterdir():
        resolved_child = child.resolve()
        if reference_root not in resolved_child.parents:
            raise ValueError(f"independent-reference cleanup escaped its work root: {child}")
        if child.is_file():
            child.unlink()
        elif child.is_dir():
            shutil.rmtree(child)

    started_ns = time.time_ns()
    executions: list[dict[str, Any]] = []
    generated: list[Path] = []
    comparisons: list[dict[str, Any]] = []
    blocker: str | None = None
    runtime_identity: dict[str, Any] = {}
    try:
        csem_simple = REFERENCE_WORK / "csem_simple.csv"
        csem_published = REFERENCE_WORK / "csem_threecommonfactors.csv"
        published_data = REFERENCE_WORK / "csem_threecommonfactors.data.csv"
        plspm_reference = REFERENCE_WORK / "python_plspm.json"
        numpy_reference = REFERENCE_WORK / "numpy_pca.json"

        resolved, resolve_execution = run_command(
            [
                "powershell",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                "validation/Resolve-Rscript.ps1",
            ],
            timeout=120,
        )
        executions.append(resolve_execution)
        if resolved.returncode != 0:
            raise RuntimeError("Rscript resolution failed")
        rscript_lines = [line.strip() for line in resolved.stdout.splitlines() if line.strip()]
        if len(rscript_lines) != 1 or not Path(rscript_lines[0]).is_file():
            raise RuntimeError(f"Rscript resolution was ambiguous: {rscript_lines}")
        rscript = rscript_lines[0]
        r_env = {"R_LIBS_USER": str(ROOT / "validation" / "r-library")}
        _require_command(
            [
                rscript,
                "--vanilla",
                str(ROOT / "validation" / "r_csem_reference.R"),
                str(ROOT / "validation" / "fixtures" / "simple_reflective.csv"),
                str(csem_simple),
            ],
            executions,
            env=r_env,
        )
        _require_command(
            [
                rscript,
                "--vanilla",
                str(ROOT / "validation" / "r_csem_threecommonfactors_reference.R"),
                str(published_data),
                str(csem_published),
            ],
            executions,
            env=r_env,
        )

        plspm_python = ROOT / "validation" / ".venv" / "Scripts" / "python.exe"
        if not plspm_python.is_file():
            raise FileNotFoundError(
                "validation/.venv/Scripts/python.exe is required for the frozen python-plspm reference"
            )
        _require_command(
            [
                str(plspm_python),
                "validation/python_plspm_reference.py",
                str(plspm_reference),
            ],
            executions,
        )
        _require_command(
            [
                "python",
                "-c",
                (
                    "from pathlib import Path; import sys; "
                    "import validation.pls_pca_reference as reference; "
                    "reference.OUTPUT = Path(sys.argv[1]); reference.main()"
                ),
                str(numpy_reference),
            ],
            executions,
        )
        generated.extend(
            [csem_simple, csem_published, published_data, plspm_reference, numpy_reference]
        )

        simple_constructs = [
            _construct("x", ["x1", "x2"]),
            _construct("y", ["y1", "y2"]),
        ]
        simple_formative = [
            _construct("x", ["x1", "x2"], mode="formative"),
            _construct("y", ["y1", "y2"], mode="formative"),
        ]
        simple_data = ROOT / "validation" / "fixtures" / "simple_reflective.csv"
        simple_paths = [{"source": "x", "target": "y"}]
        qpls_runs = {
            "PATH_MODE_A": run_pls(
                name="independent_path_mode_a",
                csv_path=simple_data,
                constructs=simple_constructs,
                paths=simple_paths,
                weighting_scheme="path",
                tolerance=1e-12,
            ),
            "MODE_B": run_pls(
                name="independent_mode_b",
                csv_path=simple_data,
                constructs=simple_formative,
                paths=simple_paths,
                weighting_scheme="path",
                tolerance=1e-12,
            ),
            "FACTOR": run_pls(
                name="independent_factor",
                csv_path=simple_data,
                constructs=simple_constructs,
                paths=simple_paths,
                weighting_scheme="factor",
                tolerance=1e-12,
            ),
            "PCA": run_pls(
                name="independent_pca",
                csv_path=simple_data,
                constructs=simple_constructs,
                paths=simple_paths,
                weighting_scheme="pca",
                tolerance=1e-12,
            ),
        }
        published_run = run_pls(
            name="independent_csem_threecommonfactors",
            csv_path=published_data,
            constructs=[
                _construct("eta1", ["y11", "y12", "y13"]),
                _construct("eta2", ["y21", "y22", "y23"]),
                _construct("eta3", ["y31", "y32", "y33"]),
            ],
            paths=[
                {"source": "eta1", "target": "eta2"},
                {"source": "eta1", "target": "eta3"},
                {"source": "eta2", "target": "eta3"},
            ],
            weighting_scheme="path",
            tolerance=1e-12,
        )

        csem_variants = _read_csem_csv(csem_simple)
        plspm_variants = _read_plspm_json(plspm_reference)
        numpy_values = _read_numpy_json(numpy_reference)
        for variant, qpls_run in qpls_runs.items():
            actual = _qpls_values(qpls_run)
            comparisons.append(
                _compare_values(
                    engine="R cSEM",
                    variant=variant,
                    actual=actual,
                    expected=csem_variants[variant],
                )
            )
            if variant in plspm_variants:
                plspm_keys = set(plspm_variants[variant])
                comparisons.append(
                    _compare_values(
                        engine="Python plspm",
                        variant=variant,
                        actual={key: value for key, value in actual.items() if key in plspm_keys},
                        expected=plspm_variants[variant],
                    )
                )
        comparisons.append(
            _compare_values(
                engine="NumPy eigensystem",
                variant="PCA",
                actual=_qpls_values(qpls_runs["PCA"]),
                expected=numpy_values,
            )
        )
        published_reference = next(iter(_read_csem_csv(csem_published).values()))
        comparisons.append(
            _compare_values(
                engine="R cSEM published threecommonfactors",
                variant="PUBLISHED_PATH_MODE_A",
                actual=_qpls_values(published_run),
                expected=published_reference,
            )
        )

        generated.extend(
            ROOT / path
            for run in [*qpls_runs.values(), published_run]
            for path in _generated_run_artifacts(run)
        )
        runtime_identity = {
            "rscript": rscript,
            "csem_versions": _csem_versions([csem_simple, csem_published]),
            "python_plspm_engine": strict_load_json(plspm_reference)["engine"],
            "numpy_engine": strict_load_json(numpy_reference)["engine"],
        }
    except Exception as error:  # noqa: BLE001 - evidence must record every failure
        blocker = f"{type(error).__name__}: {error}"

    generated = sorted({path.resolve() for path in generated if path.is_file()})
    freshness = [_fresh_artifact(path, started_ns) for path in generated]
    passed = (
        blocker is None
        and len(comparisons) == 9
        and all(row["passed"] for row in comparisons)
        and bool(freshness)
        and all(row["passed"] for row in freshness)
    )
    generated_relative = [repository_path(path) for path in generated]
    checks = {
        "passed": passed,
        "fresh_generation_only": True,
        "legacy_reports_accepted": False,
        "independence": {
            "independent_engines": ["R cSEM", "Python plspm", "NumPy eigensystem"],
            "runtime_policy": "development validation only; never packaged with QuickPLS",
            "quickpls_imported_by_reference_scripts": False,
        },
        "runtime_identity": runtime_identity,
        "comparison_count": len(comparisons),
        "comparisons": comparisons,
        "generated_artifacts": freshness,
        "generated_source_descriptors": source_descriptors(generated_relative),
        "executions": executions,
        "blocker": blocker,
    }
    report = write_identity_report(
        "independent_reference",
        passed=passed,
        checks=checks,
        extras=[*static_sources, *generated_relative],
    )
    print(f"wrote {report} | passed={passed}")
    if blocker:
        print(blocker)
    return passed


def run_python_gate(script: str) -> bool:
    completed, execution = run_command(["python", script], timeout=1800)
    if completed.returncode != 0:
        print(json.dumps(execution, indent=2))
        return False
    return True


def frontend_report() -> bool:
    command = [
        "npx.cmd",
        "vitest",
        "run",
        "src/native/plsAlgorithmFactory.test.ts",
        "src/native/nativeAnalysisRecipe.test.ts",
        "src/native/NativeResultsSurface.test.tsx",
        "--reporter=verbose",
    ]
    completed, execution = run_command(command, timeout=900)
    output = completed.stdout + completed.stderr
    checks = {
        "passed": completed.returncode == 0,
        "method_scoped_contract_test": "plsAlgorithmFactory.test.ts" in output,
        "typed_recipe_identity_test": "nativeAnalysisRecipe.test.ts" in output,
        "accessible_result_surface_test": "NativeResultsSurface.test.tsx" in output,
        "test_files_passed": "Test Files" in output and "passed" in output,
    }
    checks["passed"] = checks["passed"] and all(
        checks[key]
        for key in (
            "method_scoped_contract_test",
            "typed_recipe_identity_test",
            "accessible_result_surface_test",
            "test_files_passed",
        )
    )
    report = write_identity_report(
        "frontend_report",
        passed=checks["passed"],
        checks=checks,
        execution=execution,
        extras=[
            SOURCE,
            "src/native/plsAlgorithmFactory.test.ts",
            "src/native/nativeAnalysisRecipe.ts",
            "src/native/nativeResults.ts",
            "src/native/NativeResultsSurface.tsx",
            "src/data/smokeRun.ts",
        ],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def export_report() -> bool:
    command = [
        "npx.cmd",
        "vitest",
        "run",
        "src/native/plsAlgorithmFactory.test.ts",
        "src/native/nativeExportTables.test.ts",
        "src/native/NativeExportDialog.test.ts",
        "--reporter=verbose",
    ]
    completed, execution = run_command(command, timeout=900)
    output = completed.stdout + completed.stderr
    checks = {
        "passed": completed.returncode == 0,
        "method_scoped_export_projection": "plsAlgorithmFactory.test.ts" in output,
        "table_projection_exercised": "nativeExportTables.test.ts" in output,
        "same_run_export_scope_exercised": "NativeExportDialog.test.ts" in output,
        "test_files_passed": "Test Files" in output and "passed" in output,
    }
    checks["passed"] = checks["passed"] and all(
        checks[key]
        for key in (
            "method_scoped_export_projection",
            "table_projection_exercised",
            "same_run_export_scope_exercised",
            "test_files_passed",
        )
    )
    report = write_identity_report(
        "export_report",
        passed=checks["passed"],
        checks=checks,
        execution=execution,
        extras=[
            SOURCE,
            "src/native/plsAlgorithmFactory.test.ts",
            "src/native/nativeExportTables.ts",
            "src/native/NativeExportDialog.tsx",
            "src/native/NativeExportDialog.test.ts",
        ],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def run_stage(stage: str) -> bool:
    if stage == "engine":
        return all(
            [
                method_spec_report(),
                independent_reference_report(),
                run_python_gate("validation/pls_core_simulation.py"),
                run_python_gate("validation/pls_core_boundary_gate.py"),
            ]
        )
    if stage == "archive":
        return run_python_gate("validation/pls_core_persistence_gate.py")
    if stage == "native":
        return frontend_report() and export_report()
    if stage == "all-light":
        return run_stage("engine") and run_stage("archive") and run_stage("native")
    raise ValueError(stage)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--stage",
        choices=["engine", "archive", "native", "all-light"],
        default="all-light",
    )
    args = parser.parse_args()
    passed = run_stage(args.stage)
    print(f"PLS Algorithm v1 factory lightweight stage={args.stage} passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
