#!/usr/bin/env python3
"""Migrate four legacy-qualified methods into the strict promotion factory.

The legacy packaged reports remain useful historical evidence, but they do not
bind the current release executable.  This script therefore regenerates the
independent method references against the current coordinated release CLI,
runs focused fail-closed Python contract tests, emits source-bound engine-stage
identity reports, and writes an explicit (failing) current-package gap report.

It never invokes Cargo, builds a binary, starts QuickPLS, or manufactures a
release qualification from a historical packaged report.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import tempfile
import zipfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
CLI = ROOT / "target" / "release" / "qpls.exe"
RECEIPT = VALIDATION / "results" / "diagnostic_bundle_build_receipt.json"
SOURCE = "validation/legacy_release_factory_migration.py"
FOCUSED_TEST = "validation/test_legacy_release_factory_migration.py"
ENGINE_ROLES = (
    "method_spec",
    "independent_reference",
    "simulation_report",
    "boundary_report",
)


METHODS: dict[str, dict[str, Any]] = {
    "structural_path_randomization_v1": {
        "feature_id": "qpls3.inference.structural_path_randomization",
        "method_version": "freedman_lane_permutation_v1",
        "reference_script": "validation/structural_path_randomization_reference.py",
        "reference_report": "validation/results/structural_path_randomization_reference_report.json",
        "test_module": "validation.test_structural_path_randomization_reference",
        "legacy_packaged": "validation/results/structural_path_randomization_v1_packaged_acceptance.json",
        "required_checks": {
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
        },
        "cli_pointer": ("tested_product", "release_cli"),
        "runtime_command": "powershell -NoProfile -ExecutionPolicy Bypass -File validation/run_v247_structural_path_randomization_native_acceptance.ps1",
        "archive": "validation/results/v247-native-structural-path-randomization-1786603559512-56988.qpls",
        "archive_test_module": "validation.test_structural_path_randomization_v1_packaged_acceptance",
        "archive_identity_token": "freedman_lane_permutation_v1",
        "native_tests": ["src/native/nativeStructuralPathRandomization.test.ts", "src/native/nativeResults.test.ts", "src/native/nativeExportTables.test.ts"],
    },
    "logistic_regression_v2": {
        "feature_id": "qpls3.standalone.logistic",
        "method_version": "regression_logistic_v2",
        "reference_script": "validation/logistic_v2_reference.py",
        "reference_report": "validation/results/logistic_v2_reference_report.json",
        "test_module": "validation.test_logistic_method_promotion_audit",
        "legacy_packaged": "validation/results/logistic_v2_packaged_acceptance.json",
        "required_checks": {
            "quickpls_v2_exact_contract",
            "independent_python_full_arithmetic",
            "external_r_glm_full_arithmetic",
            "seed_invariant_deterministic_result",
            "non_binary_outcome_rejected",
            "single_class_outcome_rejected",
            "rank_deficiency_rejected",
            "complete_separation_rejected",
            "nondefault_worker_count_rejected",
        },
        "cli_pointer": (),
        "runtime_command": "powershell -NoProfile -ExecutionPolicy Bypass -File validation/run_v247_logistic_native_acceptance.ps1",
        "archive": "validation/results/v247-native-logistic-1786604241673-47796.qpls",
        "archive_test_module": "validation.test_logistic_method_promotion_audit",
        "archive_identity_token": "regression_logistic_v2",
        "native_tests": ["src/native/nativeLogistic.test.ts", "src/native/nativeResults.test.ts", "src/native/nativeExportTables.test.ts"],
    },
    "regression_bootstrap_v1": {
        "feature_id": "qpls3.standalone.regression_bootstrap",
        "method_version": "regression_bootstrap_v1",
        "reference_script": "validation/regression_bootstrap_v1_reference.py",
        "reference_report": "validation/results/regression_bootstrap_v1_reference_report.json",
        "test_module": "validation.test_regression_bootstrap_v1_reference",
        "legacy_packaged": "validation/results/regression_bootstrap_v1_packaged_acceptance.json",
        "required_checks": {
            "frozen_supplied_type7_bca_normal_ratio_or",
            "quickpls_ols_exact_contract",
            "quickpls_logistic_exact_contract",
            "exact_witness_arithmetic_ols",
            "exact_witness_arithmetic_logistic",
            "witness_index_partitions_ols",
            "witness_index_partitions_logistic",
            "independent_python_ols_resampling",
            "independent_python_logistic_resampling",
            "external_r_ols_resampling",
            "external_r_logistic_resampling",
            "deterministic_worker_invariant_ols",
            "deterministic_worker_invariant_logistic",
        },
        "cli_pointer": ("artifacts", "tested_cli"),
        "runtime_command": "powershell -NoProfile -ExecutionPolicy Bypass -File validation/run_v247_regression_bootstrap_native_acceptance.ps1",
        "archive": "validation/results/v247-native-regression-bootstrap-1786604468092-21920.qpls",
        "archive_test_module": "validation.test_regression_bootstrap_method_promotion_audit",
        "archive_identity_token": "regression_bootstrap_v1",
        "native_tests": ["src/native/nativeRegressionBootstrapWitness.test.ts", "src/native/nativeResults.test.ts", "src/native/nativeExportTables.test.ts"],
    },
    "process_v2": {
        "feature_id": "qpls3.standalone.process",
        "method_version": "regression_process_v2",
        "reference_script": "validation/process_v2_reference.py",
        "reference_report": "validation/results/process_v2_reference_report.json",
        "test_module": "validation.test_process_v2_reference",
        "legacy_packaged": "validation/results/process_v2_packaged_acceptance.json",
        "required_checks": {
            "typed_recipe_and_method_identity",
            "global_listwise_sample_and_profiles",
            "equation_order_terms_hc3_fit",
            "hc3_high_leverage_fail_closed",
            "hc3_covariance_diagonal_fail_closed",
            "simple_slope_variance_fail_closed",
            "scale_aware_solver_affine_unit_invariance_and_relative_collinearity",
            "johnson_neyman_root_solver_affine_stability",
            "johnson_neyman_invalid_covariance_fail_closed",
            "binary_endogenous_outcome_rejected_in_original_sample",
            "semantic_probe_grid_rejects_collapsed_f64_levels",
            "reference_effect_condition_policy",
            "point_metamorphic_invariance",
            "parallel_serial_effect_arithmetic",
            "continuous_binary_three_way_slopes",
            "first_second_stage_moderated_mediation",
            "johnson_neyman_roots_regions_curves",
            "conditional_plot_arithmetic",
            "bootstrap_witness_type7_bca_ratio",
            "bootstrap_index_partitions",
            "independent_python_resampling_distribution",
            "independent_r_equations_effects_resampling",
            "seed_worker_invariance",
            "dedicated_graph_result_not_generic_regression",
            "legacy_v1_not_current_evidence",
        },
        "cli_pointer": ("artifacts", "tested_cli"),
        "runtime_command": "powershell -NoProfile -ExecutionPolicy Bypass -File validation/run_v247_process_v2_native_acceptance.ps1",
        "archive": "validation/results/v247-native-process-v2-1786603623786-24804.qpls",
        "archive_test_module": "validation.test_process_v2_method_promotion_audit",
        "archive_identity_token": "regression_process_v2",
        "native_tests": ["src/native/nativeProcess.test.ts", "src/native/NativeProcessSurface.test.tsx", "src/native/nativeResults.test.ts", "src/native/nativeExportTables.test.ts"],
    },
}


def strict_load(path: Path) -> dict[str, Any]:
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


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def descriptor(path: Path) -> dict[str, Any]:
    return {
        "path": path.resolve().relative_to(ROOT.resolve()).as_posix(),
        "size": path.stat().st_size,
        "sha256": sha256_file(path),
    }


def lookup(document: dict[str, Any], pointer: Iterable[str]) -> Any:
    value: Any = document
    for key in pointer:
        if not isinstance(value, dict):
            return None
        value = value.get(key)
    return value


def passed_checks(report: dict[str, Any]) -> set[str]:
    checks = report.get("checks")
    if isinstance(checks, dict):
        return {key for key, value in checks.items() if value is True}
    if isinstance(checks, list):
        return {
            row["name"]
            for row in checks
            if isinstance(row, dict)
            and isinstance(row.get("name"), str)
            and row.get("passed") is True
        }
    return set()


def run(command: list[str], *, env: dict[str, str] | None = None) -> dict[str, Any]:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=1_200,
        check=False,
        env={**os.environ, **(env or {})},
    )
    return {
        "command": command,
        "returncode": completed.returncode,
        "stdout_tail": completed.stdout[-2000:],
        "stderr_tail": completed.stderr[-2000:],
    }


def required_sources(
    manifest: dict[str, Any], roles: Iterable[str] = ENGINE_ROLES
) -> list[dict[str, Any]]:
    governance = manifest["governance"]
    paths = {
        governance["manifest_path"],
        governance["schema_path"],
        governance["validator_path"],
        governance["focused_test_path"],
        SOURCE,
        FOCUSED_TEST,
    }
    requirements = manifest["qualification"]["source_requirements"]
    for role in roles:
        paths.update(requirements[role])
    return [descriptor(ROOT / path) for path in sorted(paths)]


def exact_reference_identity(report: dict[str, Any], config: dict[str, Any]) -> bool:
    return (
        report.get("passed") is True
        and report.get("feature_id") == config["feature_id"]
        and report.get("method_version") == config["method_version"]
        and report.get("catalogue_snapshot_date") == "2026-08-12"
    )


def current_cli_binding(report: dict[str, Any], config: dict[str, Any]) -> dict[str, Any]:
    actual = descriptor(CLI)
    reported = lookup(report, config["cli_pointer"]) if config["cli_pointer"] else None
    explicit_match = (
        all(reported.get(key) == actual[key] for key in ("path", "size", "sha256"))
        if isinstance(reported, dict)
        else None
    )
    return {
        "passed": explicit_match is not False,
        "current_cli": actual,
        "reference_report_cli": reported,
        "explicit_report_binding": explicit_match,
        "execution_binding": "reference script executed in this evidence run with QUICKPLS_CLI_PATH set to current_cli",
    }


def legacy_package_gap(config: dict[str, Any], current_desktop: dict[str, Any]) -> dict[str, Any]:
    path = ROOT / config["legacy_packaged"]
    legacy = strict_load(path)
    tested = legacy.get("tested_product")
    old_desktop = (
        tested.get("quickpls_desktop_exe")
        if isinstance(tested, dict)
        and isinstance(tested.get("quickpls_desktop_exe"), dict)
        else None
    )
    exact_current_binding = (
        isinstance(old_desktop, dict)
        and old_desktop.get("path") == current_desktop.get("path")
        and old_desktop.get("size") == current_desktop.get("size")
        and old_desktop.get("sha256") == current_desktop.get("sha256")
    )
    return {
        "schema_version": 1,
        "kind": "quickpls_method_factory_release_candidate_gap",
        "passed": False,
        "feature_id": config["feature_id"],
        "method_version": config["method_version"],
        "catalogue_snapshot_date": "2026-08-12",
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "highest_current_factory_state": "engine_only",
        "legacy_packaged_report": descriptor(path),
        "legacy_report_passed": legacy.get("passed") is True,
        "legacy_tested_desktop": old_desktop,
        "current_candidate_desktop": current_desktop,
        "exact_current_desktop_binding": exact_current_binding,
        "release_qualification_blocker": "Dedicated packaged acceptance has not been rerun against the exact current release executable.",
        "required_runtime_command": config["runtime_command"],
    }


def write_stage_identity(
    *,
    output_root: Path,
    manifest: dict[str, Any],
    config: dict[str, Any],
    filename: str,
    role: str,
    roles: Iterable[str],
    passed: bool,
    checks: dict[str, Any],
) -> None:
    payload = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": role,
        "passed": passed,
        "feature_id": config["feature_id"],
        "method_version": config["method_version"],
        "catalogue_snapshot_date": "2026-08-12",
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "source_artifacts": required_sources(manifest, roles),
        "checks": checks,
    }
    (output_root / filename).write_text(
        json.dumps(payload, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )


def archive_and_native_reports(
    manifest: dict[str, Any], config: dict[str, Any], output_root: Path
) -> tuple[bool, bool]:
    archive = ROOT / config["archive"]
    validation = run([str(CLI), "validate", config["archive"], "--json"])
    inspection = run([str(CLI), "inspect", config["archive"], "--json"])
    tamper_execution: dict[str, Any]
    with tempfile.TemporaryDirectory(prefix="quickpls-factory-tamper-") as temporary:
        tampered = Path(temporary) / "tampered.qpls"
        with zipfile.ZipFile(archive) as source, zipfile.ZipFile(
            tampered, "w", compression=zipfile.ZIP_DEFLATED
        ) as target:
            for member in source.infolist():
                payload = source.read(member.filename)
                if member.filename == "project.json":
                    project = json.loads(payload)
                    project["results"][0]["id"] = "00000000-0000-0000-0000-000000000000"
                    payload = json.dumps(project, separators=(",", ":")).encode("utf-8")
                target.writestr(member, payload)
        tamper_execution = run([str(CLI), "validate", str(tampered), "--json"])
    try:
        validation_json = json.loads(validation["stdout_tail"])
        inspection_json = json.loads(inspection["stdout_tail"])
        with zipfile.ZipFile(archive) as package:
            project = json.loads(package.read("project.json"))
        identity_present = config["archive_identity_token"] in json.dumps(
            project, sort_keys=True, separators=(",", ":")
        )
    except (OSError, ValueError, KeyError, zipfile.BadZipFile, json.JSONDecodeError):
        validation_json = {}
        inspection_json = {}
        identity_present = False
    archive_checks = {
        "passed": (
            validation["returncode"] == 0
            and validation_json.get("valid") is True
            and inspection["returncode"] == 0
            and inspection_json.get("readOnly") is False
            and inspection_json.get("results", 0) >= 1
            and identity_present
            and tamper_execution["returncode"] != 0
        ),
        "current_cli_validate": validation,
        "current_cli_inspect": inspection,
        "checksum_tamper_rejected_by_current_cli": tamper_execution,
        "archive": descriptor(archive),
        "archive_identity_token_present": identity_present,
        "archive_origin": "historical packaged workflow; revalidated and reopened by the exact current release CLI",
        "current_cli": descriptor(CLI),
        "current_desktop_write_not_claimed": True,
    }
    write_stage_identity(
        output_root=output_root,
        manifest=manifest,
        config=config,
        filename="persistence_report.identity.json",
        role="persistence_report",
        roles=("persistence_report",),
        passed=archive_checks["passed"],
        checks=archive_checks,
    )

    native_execution = run(
        ["npx.cmd", "vitest", "run", *config["native_tests"], "--reporter=verbose"]
    )
    native_checks = {
        "passed": native_execution["returncode"] == 0,
        "focused_current_source_tests": native_execution,
        "test_files": config["native_tests"],
        "method_specific_frontend_present": config["native_tests"][0] in native_execution["command"],
        "typed_results_projection_present": "src/native/nativeResults.test.ts" in native_execution["command"],
        "export_projection_present": "src/native/nativeExportTables.test.ts" in native_execution["command"],
        "packaged_desktop_execution_claimed": False,
    }
    native_checks["passed"] = native_checks["passed"] and all(
        native_checks[key]
        for key in (
            "method_specific_frontend_present",
            "typed_results_projection_present",
            "export_projection_present",
        )
    )
    write_stage_identity(
        output_root=output_root,
        manifest=manifest,
        config=config,
        filename="native_stage.identity.json",
        role="native_stage",
        roles=("frontend_report", "export_report"),
        passed=native_checks["passed"],
        checks=native_checks,
    )
    return bool(archive_checks["passed"]), bool(native_checks["passed"])


def generate_method(method: str, *, rerun_reference: bool) -> bool:
    config = METHODS[method]
    manifest_path = VALIDATION / "methods" / f"{method}.manifest.json"
    manifest = strict_load(manifest_path)
    report_path = ROOT / config["reference_report"]
    reference_execution: dict[str, Any]
    if rerun_reference:
        reference_execution = run(
            ["python", config["reference_script"]],
            env={"QUICKPLS_CLI_PATH": str(CLI.resolve())},
        )
    else:
        reference_execution = {
            "command": ["not_rerun"],
            "returncode": None,
            "stdout_tail": "",
            "stderr_tail": "",
        }
    test_execution = run(
        ["python", "-m", "unittest", config["test_module"]]
    )
    reference = strict_load(report_path)
    observed = passed_checks(reference)
    required = set(config["required_checks"])
    cli_binding = current_cli_binding(reference, config)
    checks = {
        "passed": (
            (reference_execution["returncode"] == 0 if rerun_reference else True)
            and test_execution["returncode"] == 0
            and exact_reference_identity(reference, config)
            and observed == required
            and cli_binding["passed"]
        ),
        "current_reference_execution": reference_execution,
        "focused_fail_closed_tests": test_execution,
        "reference_identity_exact": exact_reference_identity(reference, config),
        "required_check_set_exact": observed == required,
        "expected_checks": sorted(required),
        "observed_passed_checks": sorted(observed),
        "current_cli_binding": cli_binding,
        "reference_report": descriptor(report_path),
        "bounded_state": "engine_only",
        "archive_native_release_evidence_borrowed": False,
    }
    output_root = VALIDATION / "results" / "method_factory" / method
    output_root.mkdir(parents=True, exist_ok=True)
    identity = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": "engine_stage",
        "passed": checks["passed"],
        "feature_id": config["feature_id"],
        "method_version": config["method_version"],
        "catalogue_snapshot_date": "2026-08-12",
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "source_artifacts": required_sources(manifest),
        "checks": checks,
    }
    identity_path = output_root / "engine_stage.identity.json"
    identity_path.write_text(
        json.dumps(identity, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    archive_passed, native_passed = archive_and_native_reports(
        manifest, config, output_root
    )
    receipt = strict_load(RECEIPT)
    current_desktop = receipt.get("tested_desktop")
    if not isinstance(current_desktop, dict):
        raise ValueError("build receipt does not contain tested_desktop")
    gap = legacy_package_gap(config, current_desktop)
    (output_root / "release_candidate_gap.json").write_text(
        json.dumps(gap, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    print(
        f"{method}: engine={identity['passed']} archive={archive_passed} "
        f"native={native_passed} release={gap['passed']} "
        f"current_desktop_bound={gap['exact_current_desktop_binding']}"
    )
    return bool(identity["passed"] and archive_passed and native_passed and gap["passed"] is False)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("methods", nargs="*", choices=sorted(METHODS))
    parser.add_argument(
        "--reuse-current-reference",
        action="store_true",
        help="Reuse a report generated separately; focused tests and all bindings still run.",
    )
    args = parser.parse_args()
    selected = args.methods or list(METHODS)
    if not CLI.is_file():
        raise SystemExit("Current coordinated release CLI is missing; this script never builds it.")
    passed = all(
        generate_method(method, rerun_reference=not args.reuse_current_reference)
        for method in selected
    )
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
