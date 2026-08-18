#!/usr/bin/env python3
"""Fresh source-tier qualification gates for established QuickPLS 3 methods.

The module refreshes engine, archive, and native identities. It never launches
or infers a packaged desktop workflow; release identities are verified and
written separately by the receipt-bound Phase-2 packaged closure.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable, Sequence


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results" / "method_factory"
COMMON_SOURCE = "validation/established_method_factory_common.py"
FACTORY_TEST = "validation/test_established_method_factory.py"
QUALIFICATION_STAGES = (
    "engine_only",
    "archive_qualified",
    "native_qualified",
    "release_qualified",
)

METHODS: dict[str, dict[str, Any]] = {
    "gsca": {
        "manifest": "validation/methods/gsca_als_v2.manifest.json",
        "feature_id": "qpls3.gsca.als",
        "method_version": "gsca_als_v2",
        "catalogue_date": "2026-08-12",
        "output": "gsca_als_v2",
        "reference_command": [sys.executable, "validation/gsca_release_reference.py"],
        "reference_report": "validation/results/method_factory/gsca_als_v2/independent_reference.json",
        "reference_pointer": ["method_version"],
        "simulation_filter": "gsca_als_v2",
        "boundary_filter": "gsca_als_v2",
        "persistence_filter": "runner_generated_gsca_als_v2_commits_saves_reopens_and_rejects_contract_tampering",
        "native_tests": [
            "src/native/nativeGscaCanonicalIntegration.test.ts",
            "src/native/nativeExportTables.test.ts",
        ],
    },
    "cca": {
        "manifest": "validation/methods/cca_residuals_v1.manifest.json",
        "feature_id": "qpls3.assessment.cca_residuals",
        "method_version": "cca_composite_residual_v1",
        "catalogue_date": "2026-08-12",
        "output": "cca_residuals_v1",
        "reference_command": [sys.executable, "validation/cca_release_reference.py"],
        "reference_report": "validation/results/method_factory/cca_residuals_v1/independent_reference.json",
        "reference_pointer": ["method_version"],
        "simulation_filter": "cca_non_saturated_residuals",
        "boundary_filter": "cca_bounded_scope",
        "persistence_filter": "runner_generated_cca_appends_round_trips_and_rejects_contract_tampering",
        "native_tests": [
            "src/native/nativeCcaCanonicalIntegration.test.ts",
            "src/native/nativeExportTables.test.ts",
        ],
    },
    "ipma": {
        "manifest": "validation/methods/ipma_v1.manifest.json",
        "feature_id": "qpls3.assessment.ipma",
        "method_version": "ipma_v1",
        "catalogue_date": "2026-08-12",
        "output": "ipma_v1",
        "reference_command": [sys.executable, "validation/ipma_release_reference.py"],
        "reference_report": "validation/results/method_factory/ipma_v1/independent_reference.json",
        "reference_pointer": ["method_version"],
        "simulation_filter": "bounded_ipma",
        "boundary_filter": "bounded_ipma",
        "persistence_filter": "runner_generated_ipma_commits_saves_reopens_and_rejects_contract_tampering",
        "native_tests": [
            "src/native/nativeIpma.test.ts",
            "src/native/nativeIpmaCanonicalIntegration.test.ts",
            "src/native/nativeExportTables.test.ts",
        ],
    },
    "nca": {
        "manifest": "validation/methods/nca_v2.manifest.json",
        "feature_id": "qpls3.standalone.nca",
        "method_version": "nca_v2",
        "catalogue_date": "2026-08-12",
        "output": "nca_v2",
        "reference_command": [sys.executable, "validation/nca_release_reference.py"],
        "reference_report": "validation/results/method_factory/nca_v2/independent_reference.json",
        "reference_pointer": ["checks", "nca", "method_version"],
        "simulation_filter": "nca_v2",
        "boundary_filter": "bounded_nca_v2",
        "persistence_filter": "runner_generated_nca_v2_commits_saves_reopens_and_rejects_contract_tampering",
        "native_tests": [
            "src/native/nativeNca.test.ts",
            "src/native/nativeResults.test.ts",
            "src/native/nativeExportTables.test.ts",
            "src/native/NativeExportDialog.test.ts",
        ],
    },
    "consistent_bootstrap": {
        "manifest": "validation/methods/consistent_bootstrap_v1.manifest.json",
        "feature_id": "qpls3.inference.consistent_bootstrap",
        "method_version": "plsc_bootstrap_v1",
        "catalogue_date": "2026-08-12",
        "output": "consistent_bootstrap_v1",
        "reference_command": [
            sys.executable,
            "validation/consistent_bootstrap_v1_release_reference.py",
        ],
        "reference_report": "validation/results/method_factory/consistent_bootstrap_v1/independent_reference.json",
        "reference_pointer": ["method_version"],
        "simulation_package": "qpls-resampling",
        "simulation_filter": "consistent_bootstrap::tests::consistent_bootstrap_fully_refits_plsc_and_is_worker_invariant",
        "boundary_package": "qpls-resampling",
        "boundary_filter": "consistent_bootstrap::tests::consistent_bootstrap_rejects_ordinary_pls_point_results_and_bad_counts",
        "persistence_filter": "plsc_consistent_bootstrap_round_trips_and_rejects_tampering_atomically",
        "cli_export_filter": "tests::plsc_consistent_bootstrap_export_is_typed_complete_and_fail_closed",
        "native_tests": [
            "src/native/nativeConsistentBootstrap.test.ts",
            "src/native/nativeAnalysisRecipe.test.ts",
            "src/native/NativeCalculationDialog.test.ts",
        ],
    },
}


def strict_json(path: Path) -> dict[str, Any]:
    def pairs(items: list[tuple[str, Any]]) -> dict[str, Any]:
        value: dict[str, Any] = {}
        for key, item in items:
            if key in value:
                raise ValueError(f"duplicate JSON key: {key}")
            value[key] = item
        return value

    with path.open("r", encoding="utf-8-sig") as handle:
        value = json.load(
            handle,
            object_pairs_hook=pairs,
            parse_constant=lambda token: (_ for _ in ()).throw(
                ValueError(f"non-finite JSON number: {token}")
            ),
        )
    if not isinstance(value, dict):
        raise TypeError(f"JSON root must be an object: {path}")
    return value


def descriptor(path: Path) -> dict[str, Any]:
    payload = path.read_bytes()
    return {
        "path": path.resolve().relative_to(ROOT.resolve()).as_posix(),
        "size": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }


def run(command: Sequence[str], timeout: int = 900) -> dict[str, Any]:
    invoked = list(command)
    if os.name == "nt" and invoked[0] in {"npm", "npx"}:
        invoked[0] += ".cmd"
    try:
        completed = subprocess.run(
            invoked,
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
            env={**os.environ, "CARGO_BUILD_JOBS": "1"},
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


def _pointer(value: Any, parts: Iterable[str]) -> Any:
    current = value
    for part in parts:
        if not isinstance(current, dict) or part not in current:
            raise KeyError("/".join(parts))
        current = current[part]
    return current


def _manifest(method: str) -> dict[str, Any]:
    config = METHODS[method]
    document = strict_json(ROOT / config["manifest"])
    feature = document["feature"]
    expected = {
        "id": config["feature_id"],
        "method_version": config["method_version"],
        "catalogue_snapshot_date": config["catalogue_date"],
    }
    for field, wanted in expected.items():
        if feature.get(field) != wanted:
            raise ValueError(
                f"{method} manifest {field} mismatch: expected {wanted!r}, found {feature.get(field)!r}"
            )
    return document


def method_contract_ready_for_factory(method: str) -> bool:
    """Validate the frozen target without making current maturity a precondition.

    A source-tier factory must be able to rebuild evidence after truthful
    reconciliation has lowered a stale qualification state.  Requiring the
    manifest to already be native- or release-qualified creates a circular
    promotion gate and prevents the fresh receipts that would justify that
    state.  Exact feature identity is already checked by ``_manifest``; this
    helper additionally freezes the target and evidence-stage structure.
    """

    qualification = _manifest(method)["qualification"]
    evidence = qualification.get("evidence")
    return (
        qualification.get("target_state") == "release_qualified"
        and isinstance(evidence, dict)
        and tuple(evidence) == QUALIFICATION_STAGES
        and all(isinstance(evidence[stage], list) for stage in QUALIFICATION_STAGES)
    )


def required_sources(method: str, roles: Iterable[str], extras: Iterable[str] = ()) -> list[str]:
    document = _manifest(method)
    governance = document["governance"]
    paths = {
        governance["manifest_path"],
        governance["schema_path"],
        governance["validator_path"],
        governance["focused_test_path"],
        COMMON_SOURCE,
        FACTORY_TEST,
        *extras,
    }
    requirements = document["qualification"]["source_requirements"]
    for role in roles:
        paths.update(requirements[role])
    missing = [relative for relative in sorted(paths) if not (ROOT / relative).is_file()]
    if missing:
        raise FileNotFoundError(f"{method} qualification sources are missing: {missing}")
    return sorted(paths)


def run_gate(method: str, gate: str) -> dict[str, Any]:
    if method not in METHODS:
        raise ValueError(f"unknown method: {method}")
    config = METHODS[method]
    commands: list[dict[str, Any]] = []
    checks: dict[str, Any] = {}

    if gate == "simulation":
        cli = ROOT / "target/debug/qpls.exe"
        cli_build = run(["cargo", "build", "-p", "qpls-cli"], timeout=1200)
        commands.append(cli_build)
        cli_check = run([str(cli), "--version"], timeout=60) if cli_build["passed"] and cli.is_file() else {
            "command": [str(cli), "--version"], "returncode": None, "passed": False,
            "stdout_tail": "", "stderr_tail": "coordinated debug CLI build failed or its executable is missing",
        }
        commands.append(cli_check)
        reference_run = run(config["reference_command"], timeout=1200) if cli_check["passed"] else {
            "command": config["reference_command"], "returncode": None, "passed": False,
            "stdout_tail": "", "stderr_tail": "skipped because the coordinated debug CLI is unavailable",
        }
        commands.append(reference_run)
        reference_path = ROOT / config["reference_report"]
        reference_ok = False
        reference_detail = "reference report was not produced"
        if reference_run["passed"] and reference_path.is_file():
            try:
                reference = strict_json(reference_path)
                reference_ok = (
                    reference.get("passed") is True
                    and _pointer(reference, config["reference_pointer"]) == config["method_version"]
                )
                reference_detail = descriptor(reference_path)
            except (OSError, TypeError, KeyError, ValueError) as error:
                reference_detail = f"{type(error).__name__}: {error}"
        estimator = run(
            [
                "cargo",
                "test",
                "-p",
                config.get("simulation_package", "qpls-estimation"),
                config["simulation_filter"],
                "--",
                "--nocapture",
            ],
            timeout=1200,
        )
        commands.append(estimator)
        checks = {
            "coordinated_current_cli_built": {"passed": cli_build["passed"]},
            "coordinated_current_cli_available": {
                "passed": cli_check["passed"],
                "artifact": descriptor(cli) if cli.is_file() else None,
            },
            "independent_reference_recomputed": {
                "passed": reference_ok,
                "detail": reference_detail,
            },
            "focused_estimator_contracts": {"passed": estimator["passed"]},
        }
    elif gate == "boundary":
        core = run(
            [
                "cargo",
                "test",
                "-p",
                config.get("boundary_package", "qpls-core"),
                config["boundary_filter"],
                "--",
                "--nocapture",
            ],
            timeout=1200,
        )
        commands.append(core)
        checks = {
            "applicability_and_unsupported_scope": {"passed": core["passed"]},
            "boundary_categories_declared": {
                "passed": {
                    row.get("category")
                    for row in _manifest(method)["scientific_contract"]["boundaries"]
                } == {"data_pathology", "unsupported_scope", "metamorphic", "determinism", "tamper"}
            },
        }
    elif gate == "persistence":
        project = run(
            ["cargo", "test", "-p", "qpls-project", config["persistence_filter"], "--", "--nocapture"],
            timeout=1200,
        )
        commands.append(project)
        checks = {
            "runner_result_commit_save_reopen": {"passed": project["passed"]},
            "contract_tampering_rejected": {"passed": project["passed"]},
        }
    else:
        raise ValueError(f"unknown gate: {gate}")

    return {
        "schema_version": 1,
        "gate": gate,
        "method": method,
        "passed": all(item.get("passed") is True for item in checks.values()),
        "checks": checks,
        "execution": commands,
    }


def write_gate_report(method: str, gate: str) -> Path:
    result = run_gate(method, gate)
    output = RESULTS / METHODS[method]["output"] / f"{gate}_gate.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2, sort_keys=True, allow_nan=False))
    if not result["passed"]:
        raise SystemExit(1)
    return output


def _write_identity(
    method: str,
    filename: str,
    roles: list[str],
    checks: dict[str, Any],
    execution: list[dict[str, Any]],
    extras: Iterable[str] = (),
) -> Path:
    config = METHODS[method]
    output = RESULTS / config["output"] / filename
    output.parent.mkdir(parents=True, exist_ok=True)
    report = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": "+".join(roles),
        "passed": all(item.get("passed") is True for item in checks.values()),
        "feature_id": config["feature_id"],
        "method_version": config["method_version"],
        "catalogue_snapshot_date": config["catalogue_date"],
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "source_artifacts": [descriptor(ROOT / path) for path in required_sources(method, roles, extras)],
        "checks": checks,
        "execution": execution,
    }
    output.write_text(json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n", encoding="utf-8")
    if not report["passed"]:
        raise RuntimeError(f"refusing to qualify {method}: {filename} contains a failed check")
    return output


def _write_release_gap(method: str) -> Path:
    config = METHODS[method]
    document = _manifest(method)
    packaged = document["product_contract"]["packaged"]
    declared_state = document["qualification"]["declared_state"]
    release_declared = declared_state == "release_qualified"
    blockers = [] if release_declared else [
        "No packaged Windows acceptance identity is bound to the current coordinated desktop build.",
        "No method-audit identity has verified that current packaged workflow evidence.",
    ]
    if not release_declared and packaged["cancellation_required"]:
        blockers.append("Packaged cancel/retry lifecycle evidence is required for this stochastic method.")
    output = RESULTS / config["output"] / "release_candidate_gap.json"
    report = {
        "schema_version": 1,
        "feature_id": config["feature_id"],
        "method_version": config["method_version"],
        "current_state": declared_state,
        "target_state": "release_qualified",
        "promotion_ready": release_declared,
        "missing_roles": [] if release_declared else ["packaged_acceptance", "method_audit"],
        "source_factory_scope": "engine_archive_native_identity_refresh_only",
        "release_evidence_status": "managed_by_phase2_packaged_closure" if release_declared else "not_yet_qualified",
        "required_workflow_steps": packaged["workflow_steps"],
        "required_viewports": packaged["viewports"],
        "offline_required": packaged["offline_required"],
        "process_cleanup_required": packaged["process_cleanup_required"],
        "blockers": blockers,
    }
    output.write_text(json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n", encoding="utf-8")
    return output


def qualify_method(method: str) -> list[Path]:
    config = METHODS[method]
    simulation = run_gate(method, "simulation")
    boundary = run_gate(method, "boundary")
    engine_checks = {
        "method_contract_is_current": {
            "passed": method_contract_ready_for_factory(method)
        },
        "fresh_independent_and_simulation_gate": {"passed": simulation["passed"]},
        "fresh_boundary_gate": {"passed": boundary["passed"]},
    }
    engine = _write_identity(
        method,
        "engine_stage.identity.json",
        ["method_spec", "independent_reference", "simulation_report", "boundary_report"],
        engine_checks,
        simulation["execution"] + boundary["execution"],
        [config["reference_report"]],
    )

    persistence = run_gate(method, "persistence")
    archive = _write_identity(
        method,
        "persistence_report.identity.json",
        ["persistence_report"],
        persistence["checks"],
        persistence["execution"],
    )

    native = run(["npx", "vitest", "run", *config["native_tests"]], timeout=1200)
    native_execution = [native]
    cli_export_filter = config.get("cli_export_filter")
    cli_export = (
        run(
            ["cargo", "test", "-p", "qpls-cli", cli_export_filter, "--", "--exact"],
            timeout=1200,
        )
        if cli_export_filter
        else None
    )
    if cli_export is not None:
        native_execution.append(cli_export)
    native_checks = {
        "native_setup_results_and_accessibility": {"passed": native["passed"]},
        "same_run_export_contract": {
            "passed": native["passed"]
            and (cli_export is None or cli_export["passed"])
        },
    }
    native_identity = _write_identity(
        method,
        "native_stage.identity.json",
        ["frontend_report", "export_report"],
        native_checks,
        native_execution,
    )
    release_gap = _write_release_gap(method)
    return [engine, archive, native_identity, release_gap]


def qualify_all(methods: Iterable[str] | None = None) -> list[Path]:
    outputs: list[Path] = []
    for method in methods or METHODS:
        outputs.extend(qualify_method(method))
    return outputs
