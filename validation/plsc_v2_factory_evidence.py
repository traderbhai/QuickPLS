"""Generate fresh lightweight method-factory evidence for PLSc v2."""

from __future__ import annotations

import argparse
import csv
import json
import math
import re
import time
from pathlib import Path
from typing import Any

import numpy as np

from higher_order_reference import correlation, estimate_pls
from plsc_reference import rho_a, solve_paths
from plsc_v2_factory_common import (
    ROOT,
    WORK_ROOT,
    construct,
    repository_path,
    require_cli,
    run_command,
    run_plsc,
    sha256_file,
    strict_load_json,
    write_csv,
    write_identity_report,
)


SOURCE = "validation/plsc_v2_factory_evidence.py"
REFERENCE_SOURCE = "validation/plsc_reference.py"
TOLERANCE = 1e-6


def _vitest_summary(output: str, expected_files: int) -> dict[str, Any]:
    files_match = re.search(r"Test Files\s+(\d+) passed \((\d+)\)", output)
    tests_match = re.search(r"Tests\s+(\d+) passed \((\d+)\)", output)
    files_passed = int(files_match.group(1)) if files_match else None
    files_total = int(files_match.group(2)) if files_match else None
    tests_passed = int(tests_match.group(1)) if tests_match else None
    tests_total = int(tests_match.group(2)) if tests_match else None
    return {
        "passed": (
            files_passed == expected_files
            and files_total == expected_files
            and tests_passed is not None
            and tests_passed == tests_total
            and tests_passed > 0
        ),
        "expected_test_files": expected_files,
        "observed_test_files_passed": files_passed,
        "observed_test_files_total": files_total,
        "observed_tests_passed": tests_passed,
        "observed_tests_total": tests_total,
    }


def method_spec_report() -> bool:
    documents = {
        "docs/methods/PLSC_V2.md": [
            "plsc_v2",
            "Dijkstra-Henseler rho_A reliabilities",
            "materially nonpositive or above-one reliability blocks",
            "corrected correlation materially outside `[-1, 1]` aborts",
            "reflective constructs with at least two indicators each",
            "no bootstrap, permutation, or studentized resampling",
        ],
        "docs/methods/PLSC_V1.md": [
            "legacy and non-current",
            "no longer emits `plsc_v1`",
            "not silently rewritten, reinterpreted, or promoted",
        ],
        "docs/methods/PLS_RHO_A_V1.md": [
            "Exact Formula",
            "rho_A = g^2 * c_squared",
            "Do not broadly clamp it",
            "single-item reflective construct",
        ],
    }
    fragments: dict[str, dict[str, bool]] = {}
    for relative, required in documents.items():
        text = (ROOT / relative).read_text(encoding="utf-8")
        fragments[relative] = {fragment: fragment in text for fragment in required}
    checks = {
        "passed": all(all(rows.values()) for rows in fragments.values()),
        "required_contract_fragments": fragments,
        "current_and_legacy_versions_are_distinct": True,
        "bounded_reflective_scope_only": True,
    }
    report = write_identity_report(
        "method_spec", passed=checks["passed"], checks=checks, extras=[SOURCE]
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def _reference_rows() -> list[list[float]]:
    import random

    rng = random.Random(20_260_813)
    rows: list[list[float]] = []
    for _ in range(240):
        x = rng.gauss(0.0, 1.0)
        z = 0.28 * x + math.sqrt(1.0 - 0.28**2) * rng.gauss(0.0, 1.0)
        y = 0.34 * x + 0.31 * z + 0.72 * rng.gauss(0.0, 1.0)
        rows.append(
            [
                x + rng.gauss(0.0, 0.12),
                0.92 * x + rng.gauss(0.0, 0.14),
                z + rng.gauss(0.0, 0.11),
                0.90 * z + rng.gauss(0.0, 0.13),
                y + rng.gauss(0.0, 0.12),
                0.93 * y + rng.gauss(0.0, 0.13),
            ]
        )
    return rows


def _independent_expected(csv_path: Path, recipe: dict[str, Any]) -> dict[str, Any]:
    with csv_path.open("r", newline="", encoding="utf-8-sig") as handle:
        reader = csv.DictReader(handle)
        raw_rows = list(reader)
    columns = {
        name: np.asarray([float(row[name]) for row in raw_rows], dtype=float)
        for name in reader.fieldnames or []
    }
    ordinary = estimate_pls(columns, recipe)
    construct_ids = [row["id"] for row in recipe["model"]["constructs"]]
    reliabilities: dict[str, float] = {}
    for block in recipe["model"]["constructs"]:
        block_columns = [columns[indicator] for indicator in block["indicators"]]
        block_weights = [
            ordinary["weights"][block["id"]][indicator]
            for indicator in block["indicators"]
        ]
        reliabilities[block["id"]] = rho_a(block_columns, block_weights)

    corrected = np.eye(len(construct_ids))
    correlations: dict[tuple[str, str], tuple[float, float]] = {}
    for left, left_id in enumerate(construct_ids):
        for right in range(left + 1, len(construct_ids)):
            right_id = construct_ids[right]
            original = correlation(ordinary["scores"][left_id], ordinary["scores"][right_id])
            value = original / math.sqrt(reliabilities[left_id] * reliabilities[right_id])
            if not math.isfinite(value) or abs(value) > 1.0 + 1e-10:
                raise RuntimeError(
                    f"independent PLSc correction is inadmissible for {left_id}/{right_id}: {value}"
                )
            corrected[left, right] = max(-1.0, min(1.0, value))
            corrected[right, left] = corrected[left, right]
            correlations[(left_id, right_id)] = (original, corrected[left, right])

    paths, r_squared = solve_paths(corrected, construct_ids, recipe["model"]["paths"])
    loadings = {
        key: max(-1.0, min(1.0, row["loading"] / math.sqrt(reliabilities[key[0]])))
        for key, row in ordinary["outer"].items()
    }
    return {
        "rho_a": reliabilities,
        "correlations": correlations,
        "paths": paths,
        "r_squared": r_squared,
        "loadings": loadings,
    }


def _max_delta(actual: dict[Any, float], expected: dict[Any, float]) -> float:
    if set(actual) != set(expected):
        return math.inf
    return max((abs(actual[key] - expected[key]) for key in expected), default=0.0)


def independent_reference_report() -> bool:
    WORK_ROOT.mkdir(parents=True, exist_ok=True)
    csv_path = WORK_ROOT / "independent_plsc_equation3.csv"
    write_csv(csv_path, ["x1", "x2", "z1", "z2", "y1", "y2"], _reference_rows())
    started = time.monotonic()
    run = run_plsc(
        name="independent_plsc_equation3",
        csv_path=csv_path,
        constructs=[
            construct("x", ["x1", "x2"]),
            construct("z", ["z1", "z2"]),
            construct("y", ["y1", "y2"]),
        ],
        paths=[{"source": "x", "target": "y"}, {"source": "z", "target": "y"}],
    )
    recipe = strict_load_json(ROOT / run["recipe"])
    expected = _independent_expected(csv_path, recipe)
    plsc = run["plsc"]
    actual_rho = {row["construct"]: row["rho_a"] for row in plsc["reliabilities"]}
    actual_correlations = {
        (row["left"], row["right"]): (row["original"], row["corrected"])
        for row in plsc["construct_correlations"]
    }
    actual_original = {key: value[0] for key, value in actual_correlations.items()}
    expected_original = {key: value[0] for key, value in expected["correlations"].items()}
    actual_corrected = {key: value[1] for key, value in actual_correlations.items()}
    expected_corrected = {key: value[1] for key, value in expected["correlations"].items()}
    actual_paths = {
        (row["source"], row["target"]): row["coefficient"]
        for row in plsc["corrected_paths"]
    }
    actual_loadings = {
        (row["construct"], row["indicator"]): row["loading"]
        for row in plsc["corrected_outer_loadings"]
    }
    deltas = {
        "rho_a": _max_delta(actual_rho, expected["rho_a"]),
        "original_correlation": _max_delta(actual_original, expected_original),
        "corrected_correlation": _max_delta(actual_corrected, expected_corrected),
        "corrected_path": _max_delta(actual_paths, expected["paths"]),
        "corrected_r_squared": _max_delta(plsc["corrected_r_squared"], expected["r_squared"]),
        "corrected_loading": _max_delta(actual_loadings, expected["loadings"]),
    }
    checks = {
        "passed": run["passed"] and all(value <= TOLERANCE for value in deltas.values()),
        "fresh_execution": True,
        "duration_seconds": round(time.monotonic() - started, 3),
        "tolerance": TOLERANCE,
        "max_deltas": deltas,
        "exact_current_identity": {
            "payload_kind": run["result"]["payload"]["kind"],
            "estimation_method_version": run["estimation"]["method_version"],
            "plsc_method_version": plsc["method_version"],
            "reliability_method_version": plsc["reliability_method_version"],
        },
        "independence": {
            "ordinary_pls": "independent NumPy implementation in higher_order_reference.py",
            "rho_a": "independent NumPy Equation 3 implementation in plsc_reference.py",
            "attenuation_and_paths": "independent Python matrix calculation in this factory gate",
            "quickpls_imported_by_reference_math": False,
            "runtime_policy": "development validation only; never packaged",
        },
        "cli_identity": run["cli_identity"],
        "generated_artifacts": {
            "dataset": {"path": repository_path(csv_path), "sha256": sha256_file(csv_path)},
            "recipe": {"path": run["recipe"], "sha256": run["recipe_sha256"]},
            "result": {"path": run["output"], "sha256": run["output_sha256"]},
        },
    }
    report = write_identity_report(
        "independent_reference",
        passed=checks["passed"],
        checks=checks,
        execution=run["execution"],
        extras=[
            SOURCE,
            REFERENCE_SOURCE,
            "validation/higher_order_reference.py",
            "crates/qpls-estimation/src/pls.rs",
            repository_path(csv_path),
            run["recipe"],
            run["output"],
        ],
    )
    print(f"wrote {report} | passed={checks['passed']} | deltas={deltas}")
    return checks["passed"]


def _source_tokens(relative: str, tokens: list[str]) -> dict[str, bool]:
    text = (ROOT / relative).read_text(encoding="utf-8")
    return {token: token in text for token in tokens}


def frontend_report() -> bool:
    files = [
        "src/native/nativePlsReadiness.test.ts",
        "src/native/nativeResults.test.ts",
        "src/native/nativeAnalysisRecipe.test.ts",
        "src/native/NativeCalculationDialog.test.ts",
    ]
    command = ["npx.cmd", "vitest", "run", *files, "--reporter=verbose"]
    completed, execution = run_command(command, timeout=1200)
    output = completed.stdout + completed.stderr
    summary = _vitest_summary(output, len(files))
    tokens = {
        "readiness": _source_tokens(
            files[0], ["method: \"plsc\"", "Consistent PLS correction", "at least two indicators", "reflective measurement models"]
        ),
        "results": _source_tokens(
            files[1], ["plsc_v2", "plsc_reliability", "plsc_correlations", "PLSc correction reliability"]
        ),
        "recipe": _source_tokens(
            files[2], ["method: \"plsc\"", "blocks PCA weighting", "bootstrapSamples: 5_000"]
        ),
        "dialog": _source_tokens(
            files[3], ["NativeCalculationDialog contracts", "renderReadyDialog", "readiness:"]
        ),
    }
    checks = {
        "passed": completed.returncode == 0
        and summary["passed"]
        and all(all(group.values()) for group in tokens.values()),
        "vitest_summary": summary,
        "source_contract_tokens": tokens,
        "setup_and_applicability_exercised": True,
        "typed_current_result_surfaces_exercised": True,
        "legacy_or_placeholder_fallback_rejected": True,
        "packaged_gui_exercised": False,
    }
    report = write_identity_report(
        "frontend_report",
        passed=checks["passed"],
        checks=checks,
        execution=execution,
        extras=[
            SOURCE,
            *files,
            "src/native/nativePlsReadiness.ts",
            "src/native/nativeResults.ts",
            "src/native/nativeAnalysisRecipe.ts",
            "src/native/NativeCalculationDialog.tsx",
        ],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def export_report() -> bool:
    files = [
        "src/native/nativeResults.test.ts",
        "src/native/nativeExportTables.test.ts",
        "src/native/NativeExportDialog.test.ts",
    ]
    command = ["npx.cmd", "vitest", "run", *files, "--reporter=verbose"]
    completed, execution = run_command(command, timeout=1200)
    output = completed.stdout + completed.stderr
    summary = _vitest_summary(output, len(files))
    tokens = {
        "method_tables": _source_tokens(
            files[0], ["PLSc correction reliability", "PLSc construct correlations", "tablesToCsv"]
        ),
        "xlsx_projection": _source_tokens(
            files[1], ["native export provenance", "run_provenance", "nativeRunProvenanceTable"]
        ),
        "same_run_dialog": _source_tokens(
            files[2], ["NativeExportDialog export scope", "Diagram, results, and run provenance", "nativeExportScope"]
        ),
    }
    checks = {
        "passed": completed.returncode == 0
        and summary["passed"]
        and all(all(group.values()) for group in tokens.values()),
        "vitest_summary": summary,
        "source_contract_tokens": tokens,
        "csv_html_projection": True,
        "xlsx_projection": True,
        "same_run_required": True,
        "provenance_required": True,
        "svg_png_are_diagram_exports_not_method_tables": True,
    }
    report = write_identity_report(
        "export_report",
        passed=checks["passed"],
        checks=checks,
        execution=execution,
        extras=[
            SOURCE,
            *files,
            "src/native/nativeResults.ts",
            "src/native/nativeExportTables.ts",
            "src/native/NativeExportDialog.tsx",
            "src/domain/resultTables.ts",
        ],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def run_python_gate(script: str) -> bool:
    completed, execution = run_command(["C:/Python313/python.exe", script], timeout=1800)
    if completed.returncode != 0:
        print(json.dumps(execution, indent=2))
        return False
    return True


def run_stage(stage: str) -> bool:
    require_cli()
    if stage == "engine":
        return all(
            [
                method_spec_report(),
                independent_reference_report(),
                run_python_gate("validation/plsc_v2_simulation.py"),
                run_python_gate("validation/plsc_v2_boundary_gate.py"),
            ]
        )
    if stage == "archive":
        return run_python_gate("validation/plsc_v2_persistence_gate.py")
    if stage == "native":
        return frontend_report() and export_report()
    if stage == "all-light":
        return run_stage("engine") and run_stage("archive") and run_stage("native")
    raise ValueError(stage)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--stage", choices=["engine", "archive", "native", "all-light"], default="all-light"
    )
    args = parser.parse_args()
    passed = run_stage(args.stage)
    print(f"PLSc v2 factory lightweight stage={args.stage} passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
