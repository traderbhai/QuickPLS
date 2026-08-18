#!/usr/bin/env python3
"""Orchestrate light evidence and audit release readiness for PLS power v1."""

from __future__ import annotations

import argparse
import ast
import json
import re
import shutil
import subprocess
import sys
import time
from copy import deepcopy
from pathlib import Path
from typing import Any

from method_promotion_manifest import (
    _verify_artifact,
    validate_manifest,
    validate_manifest_document,
)
from pls_sample_size_power_persistence_gate import ARCHIVE_CLAIM_BOUNDARY
from pls_sample_size_power_reference import fixture_recipe, run_reference, self_test
from pls_sample_size_power_simulation import (
    EXECUTION_SOURCES,
    FEATURE_ID,
    IDENTITY_VERIFICATION,
    METHOD_VERSION,
    REPORT_ROOT,
    ROOT,
    manifest,
    role_sources,
    run_command,
    source_descriptors,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/pls_sample_size_power_method_promotion_audit.py"
REFERENCE_PY = "validation/pls_sample_size_power_reference.py"
REFERENCE_R = "validation/pls_sample_size_power_reference.R"


def method_spec_report() -> bool:
    path = ROOT / "docs" / "methods" / "PLS_SAMPLE_SIZE_POWER_V1.md"
    text = path.read_text(encoding="utf-8")
    required = [
        "exactly one bounded design",
        "two ordinary reflective constructs",
        "3-10 indicators",
        "standard-normal exogenous latent",
        "case-bootstrap normal-reference",
        "Failed or inadmissible replicates remain in the denominator",
        "two-sided Wilson binomial interval",
        "first evaluated grid value",
        "never an interpolated or extrapolated recommendation",
        "250,000 PLS fits",
        "100,000,000 row-fits",
        "worker count as execution-only",
        "Retrospective \"observed power\" is excluded",
        ARCHIVE_CLAIM_BOUNDARY["verified"],
        ARCHIVE_CLAIM_BOUNDARY["not_verified"],
    ]
    fragments = {fragment: fragment in text for fragment in required}
    exclusions = all(
        fragment in text
        for fragment in (
            "Formative, higher-order, interaction, nonlinear, mixture, multigroup",
            "every missing-data design are blocked",
            "It does not guarantee convergence on future real data",
        )
    )
    checks = {
        "passed": all(fragments.values()) and exclusions,
        "document": "docs/methods/PLS_SAMPLE_SIZE_POWER_V1.md",
        "frozen_contract_fragments": fragments,
        "bounded_exclusions_explicit": exclusions,
        "status_text_is_not_used_as_qualification_evidence": True,
    }
    report = write_identity_report(
        "method_spec", passed=checks["passed"], checks=checks, extras=[SOURCE]
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def archive_claim_boundary_contract() -> dict[str, Any]:
    """Enforce the deliberately structural scope of archive qualification."""

    document = manifest()
    specification = (
        ROOT / "docs/methods/PLS_SAMPLE_SIZE_POWER_V1.md"
    ).read_text(encoding="utf-8")
    persistence_source = (
        ROOT / "validation/pls_sample_size_power_persistence_gate.py"
    ).read_text(encoding="utf-8")
    warnings = document["claim"]["warnings"]
    verified = ARCHIVE_CLAIM_BOUNDARY["verified"]
    not_verified = ARCHIVE_CLAIM_BOUNDARY["not_verified"]
    checks = {
        "method_spec_states_verified_scope": verified in specification,
        "method_spec_states_coordinated_rewrite_limit": not_verified in specification,
        "manifest_states_verified_scope": verified in warnings,
        "manifest_states_coordinated_rewrite_limit": not_verified in warnings,
        "persistence_report_marks_semantic_replay_false": (
            '"semantic_replay_performed": False' in persistence_source
        ),
        "persistence_report_marks_coordinated_authentication_false": (
            '"coordinated_rewrite_authenticated": False' in persistence_source
        ),
        "prior_outer_checksum_overclaim_removed": (
            "even if an attacker recomputes an outer checksum" not in specification
        ),
    }
    return {
        "passed": all(checks.values()),
        "checks": checks,
        "claim_boundary": ARCHIVE_CLAIM_BOUNDARY,
    }


def _reference_import_contract() -> dict[str, Any]:
    tree = ast.parse((ROOT / REFERENCE_PY).read_text(encoding="utf-8"))
    imported = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            imported.extend(alias.name for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module:
            imported.append(node.module)
    forbidden = [name for name in imported if name.startswith(("qpls", "validation"))]
    return {
        "passed": not forbidden and "subprocess" not in imported,
        "imported_modules": sorted(set(imported)),
        "forbidden_product_imports": forbidden,
        "subprocess_imported": "subprocess" in imported,
    }


def independent_reference_report() -> bool:
    started = time.monotonic()
    quick = self_test()
    recipe = fixture_recipe(effect=0.40, replicates=100, bootstrap=99)
    recipe["scenario_identity"] = "factory_independent_reference_production_counts"
    recipe["master_seed"] = 20_260_821
    production = run_reference(recipe, enforce_production_counts=True)
    import_contract = _reference_import_contract()
    rscript = shutil.which("Rscript")
    if rscript:
        r_completed, r_execution = run_command(
            [rscript, REFERENCE_R, "--self-test"], timeout=600
        )
        r_check = {
            "available": True,
            "passed": r_completed.returncode == 0,
            "execution": r_execution,
        }
    else:
        r_check = {
            "available": False,
            "passed": True,
            "execution": None,
            "note": "Base-R is an additional independent source; NumPy is the required executed oracle.",
        }
    row_counts = all(
        row["requested_replicates"] == 100
        and row["attempted_replicates"] == 100
        for row in production["rows"]
    )
    checks = {
        "passed": quick["passed"]
        and production["passed"]
        and row_counts
        and import_contract["passed"]
        and r_check["passed"],
        "python_self_test": quick,
        "production_count_reference": {
            "passed": production["passed"],
            "rows": production["rows"],
            "decision": production["decision"],
            "outcome_digest": production["outcome_digest"],
            "outcome_count": len(production["outcomes"]),
            "row_counts_exact": row_counts,
        },
        "independence": import_contract,
        "base_r_self_test": r_check,
        "duration_seconds": round(time.monotonic() - started, 3),
    }
    report = write_identity_report(
        "independent_reference",
        passed=checks["passed"],
        checks=checks,
        execution=[
            {
                "command": [sys.executable, REFERENCE_PY, "production-count direct import"],
                "returncode": 0 if production["passed"] else 1,
                "duration_seconds": checks["duration_seconds"],
            },
            r_check["execution"],
        ],
        extras=[SOURCE, REFERENCE_PY, REFERENCE_R],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def _vitest_summary(output: str, minimum_files: int, minimum_tests: int) -> dict[str, Any]:
    plain = re.sub(r"\x1b\[[0-9;]*m", "", output)
    files = re.search(r"Test Files\s+(\d+) passed", plain)
    tests = re.search(r"Tests\s+(\d+) passed", plain)
    file_count = int(files.group(1)) if files else None
    test_count = int(tests.group(1)) if tests else None
    return {
        "passed": file_count is not None
        and file_count >= minimum_files
        and test_count is not None
        and test_count >= minimum_tests,
        "observed_files": file_count,
        "observed_tests": test_count,
        "minimum_files": minimum_files,
        "minimum_tests": minimum_tests,
    }


def frontend_report() -> bool:
    files = [
        "src/native/nativePlsSampleSizePower.test.ts",
        "src/native/nativeAnalysisCatalog.test.ts",
        "src/native/NativeCalculationDialog.test.ts",
        "src/native/nativeResults.test.ts",
    ]
    completed, execution = run_command(
        ["npx.cmd", "vitest", "run", *files, "--reporter=verbose"], timeout=900
    )
    output = completed.stdout + "\n" + completed.stderr
    summary = _vitest_summary(output, minimum_files=4, minimum_tests=25)
    typecheck, typecheck_execution = run_command(["npx.cmd", "tsc", "-b"], timeout=900)
    dialog = (ROOT / "src/native/NativeCalculationDialog.tsx").read_text(encoding="utf-8")
    results = (ROOT / "src/native/NativeResultsSurface.tsx").read_text(encoding="utf-8")
    catalog = (ROOT / "src/native/nativeAnalysisCatalog.ts").read_text(encoding="utf-8")
    controls = [
        "nd-calculation-pls-power-scenario",
        "nd-calculation-pls-power-predictor",
        "nd-calculation-pls-power-outcome",
        "nd-calculation-pls-power-predictor-loadings",
        "nd-calculation-pls-power-outcome-loadings",
        "nd-calculation-pls-power-path",
        "nd-calculation-pls-power-grid",
        "nd-calculation-pls-power-alpha",
        "nd-calculation-pls-power-target",
        "nd-calculation-pls-power-confidence",
        "nd-calculation-pls-power-mc",
        "nd-calculation-pls-power-bootstrap",
        "nd-calculation-pls-power-workers",
        "nd-calculation-pls-power-workload",
    ]
    source_contract = {
        "catalog_identity": 'kind: "pls_sample_size_power"' in catalog
        and FEATURE_ID in catalog,
        "all_assumption_controls_labeled": all(
            f'htmlFor="{control}"' in dialog or f'id="{control}"' in dialog
            for control in controls
        ),
        "workload_live_region": 'id="nd-calculation-pls-power-workload"' in dialog
        and 'aria-live="polite"' in dialog,
        "results_decision_exposed_as_text": "powerResult.presentation.decisionLabel" in results,
        "results_workload_exposed_as_text": "powerResult.result.workload.estimated_pls_fits" in results,
    }
    passed = (
        completed.returncode == 0
        and summary["passed"]
        and typecheck.returncode == 0
        and all(source_contract.values())
    )
    checks = {
        "passed": passed,
        "vitest": summary,
        "typecheck_passed": typecheck.returncode == 0,
        "source_contract": source_contract,
        "gui_runtime_claimed": False,
        "component_and_type_contract_only": True,
    }
    report = write_identity_report(
        "frontend_report",
        passed=passed,
        checks=checks,
        execution=[execution, typecheck_execution],
        extras=[
            SOURCE,
            "src/native/nativePlsSampleSizePower.ts",
            "src/native/nativeAnalysisCatalog.ts",
            "src/native/NativeCalculationDialog.tsx",
            "src/native/NativeResultsSurface.tsx",
            "src/native/nativeResults.ts",
            *files,
        ],
    )
    print(f"wrote {report} | passed={passed}")
    return passed


def run_python_gate(script: str, *arguments: str, timeout: int = 3_600) -> bool:
    completed = subprocess.run(
        [sys.executable, script, *arguments],
        cwd=ROOT,
        timeout=timeout,
    )
    return completed.returncode == 0


def run_stage(stage: str) -> bool:
    if stage == "engine":
        return all(
            (
                method_spec_report(),
                independent_reference_report(),
                run_python_gate("validation/pls_sample_size_power_simulation.py"),
                run_python_gate("validation/pls_sample_size_power_boundary_gate.py"),
            )
        )
    if stage == "archive":
        claim_boundary = archive_claim_boundary_contract()
        if not claim_boundary["passed"]:
            print(json.dumps(claim_boundary, indent=2, sort_keys=True))
            return False
        return run_python_gate("validation/pls_sample_size_power_persistence_gate.py")
    if stage == "native":
        return frontend_report() and run_python_gate(
            "validation/pls_sample_size_power_export_gate.py"
        )
    if stage == "all-light":
        return run_stage("engine") and run_stage("archive") and run_stage("native")
    raise ValueError(stage)


def _artifact(role: str) -> dict[str, Any]:
    return {
        "path": (
            "validation/results/method_factory/pls_sample_size_power_v1/"
            f"{role}.identity.json"
        ),
        "roles": [role],
        "verification": deepcopy(IDENTITY_VERIFICATION),
    }


def promote_native_transactionally() -> bool:
    """Wire evidence first, regenerate against it, and retain only on success.

    Identity reports bind the manifest bytes themselves.  Therefore promotion
    must write the final evidence topology before generating reports.  This
    transaction restores the original manifest if any executable gate or the
    strict factory validator fails, so an interrupted/failed run cannot leave a
    higher declared state behind.
    """

    manifest_path = ROOT / "validation/methods/history/pls_sample_size_power_v1.manifest.json"
    original = manifest_path.read_bytes()
    document = manifest()
    declared = document["qualification"]["declared_state"]
    if declared not in {"absent", "engine_only", "archive_qualified", "native_qualified"}:
        raise RuntimeError(f"refusing to rewrite manifest from unexpected state {declared!r}")
    if document["qualification"]["evidence"]["release_qualified"]:
        raise RuntimeError("release evidence already exists; native promotion cannot rewrite it")
    candidate = deepcopy(document)
    candidate["qualification"]["declared_state"] = "native_qualified"
    candidate["qualification"]["evidence"] = {
        "engine_only": [
            _artifact("method_spec"),
            _artifact("independent_reference"),
            _artifact("simulation_report"),
            _artifact("boundary_report"),
        ],
        "archive_qualified": [_artifact("persistence_report")],
        "native_qualified": [
            _artifact("frontend_report"),
            _artifact("export_report"),
        ],
        "release_qualified": [],
    }
    portable = validate_manifest_document(candidate, ROOT, verify_evidence=False)
    if not portable["passed"]:
        raise RuntimeError(f"candidate native evidence topology is invalid: {portable['errors']}")

    kept = False
    try:
        manifest_path.write_text(
            json.dumps(candidate, indent=2, ensure_ascii=False, allow_nan=False) + "\n",
            encoding="utf-8",
        )
        stages_passed = run_stage("all-light")
        verified = validate_manifest(manifest_path, ROOT)
        kept = (
            stages_passed
            and verified["passed"]
            and verified["declared_state"] == "native_qualified"
            and verified["derived_state"] == "native_qualified"
        )
        if not kept:
            print(json.dumps(verified, indent=2, sort_keys=True))
        return kept
    finally:
        if not kept:
            manifest_path.write_bytes(original)


def evaluate_release_audit() -> dict[str, Any]:
    document = manifest()
    feature = document["feature"]
    identity = {
        "passed": True,
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
    }
    verified: list[dict[str, Any]] = []
    for stage in ("engine_only", "archive_qualified", "native_qualified"):
        for artifact in document["qualification"]["evidence"][stage]:
            passed, errors = _verify_artifact(artifact, document, ROOT, identity)
            verified.append(
                {
                    "stage": stage,
                    "path": artifact["path"],
                    "roles": artifact["roles"],
                    "passed": passed,
                    "errors": errors,
                }
            )
    packaged_candidates = [
        artifact
        for artifact in document["qualification"]["evidence"]["release_qualified"]
        if artifact.get("roles") == ["packaged_acceptance"]
    ]
    packaged: dict[str, Any] = {}
    packaged_passed = False
    packaged_errors = ["missing packaged_acceptance evidence"]
    packaged_path = None
    if len(packaged_candidates) == 1:
        candidate = packaged_candidates[0]
        packaged_path = candidate["path"]
        packaged_passed, packaged_errors = _verify_artifact(
            candidate, document, ROOT, identity
        )
        try:
            packaged = strict_load_json(ROOT / packaged_path)
        except (OSError, UnicodeError, ValueError) as error:
            packaged_passed = False
            packaged_errors = [*packaged_errors, str(error)]
    packaged_checks = packaged.get("checks", {}) if isinstance(packaged, dict) else {}
    required_packaged = {
        "installed_offline_desktop": packaged_checks.get("installed_offline_desktop") is True,
        "invalid_setup_blocked": packaged_checks.get("invalid_setup_blocked") is True,
        "execute_cancel_retry": packaged_checks.get("execute_cancel_retry") is True,
        "responsive_viewports": packaged_checks.get("responsive_viewports")
        == ["1024x700", "1280x720", "1440x900"],
        "accessible_results_inspected": packaged_checks.get("accessible_results_inspected")
        is True,
        "same_run_csv_xlsx": packaged_checks.get("same_run_csv_xlsx") is True,
        "archive_reopened_same_run": packaged_checks.get("archive_reopened_same_run") is True,
        "offline_and_cleanup": packaged_checks.get("offline_and_cleanup") is True,
        "source_receipt_current": packaged_checks.get("source_receipt_current") is True,
    }
    contract_report = validate_manifest(
        ROOT / document["governance"]["manifest_path"], ROOT
    )
    prior_roles = {
        role
        for row in verified
        if row["passed"]
        for role in row["roles"]
    }
    required_prior = {
        "method_spec",
        "independent_reference",
        "simulation_report",
        "boundary_report",
        "persistence_report",
        "frontend_report",
        "export_report",
    }
    checks = {
        "prior_stage_artifacts_verify": required_prior.issubset(prior_roles)
        and all(row["passed"] for row in verified),
        "manifest_currently_valid": contract_report["passed"],
        "exactly_one_packaged_candidate": len(packaged_candidates) == 1,
        "packaged_identity_verifies": packaged_passed,
        "packaged_semantics": all(required_packaged.values()),
    }
    return {
        "passed": all(checks.values()),
        "checks": checks,
        "verified_prior_artifacts": verified,
        "packaged": {
            "path": packaged_path,
            "identity_passed": packaged_passed,
            "errors": packaged_errors,
            "semantic_checks": required_packaged,
        },
        "claim_boundary": document["claim"],
        "release_claim_written": False,
    }


def release_audit() -> bool:
    checks = evaluate_release_audit()
    extras = [
        SOURCE,
        *[
            row["path"]
            for row in checks["verified_prior_artifacts"]
            if row.get("path") and (ROOT / row["path"]).is_file()
        ],
    ]
    packaged_path = checks["packaged"].get("path")
    if packaged_path and (ROOT / packaged_path).is_file():
        extras.append(packaged_path)
    report = write_identity_report(
        "method_audit", passed=checks["passed"], checks=checks, extras=extras
    )
    gap = REPORT_ROOT / "release_candidate_gap.json"
    gap.write_text(
        json.dumps(
            {
                "passed": False,
                "feature_id": FEATURE_ID,
                "method_version": METHOD_VERSION,
                "blockers": [name for name, passed in checks["checks"].items() if not passed],
                "note": (
                    "Native source qualification is not installed Windows packaged acceptance. "
                    "No release claim is authorized until the focused adapter, offline monitor, "
                    "cancel/retry, three viewports, exact archive reopen, same-run exports, current "
                    "source receipt, and cleanup all pass."
                ),
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"wrote {report} | passed={checks['passed']}")
    if not checks["passed"]:
        print(json.dumps(checks, indent=2, sort_keys=True))
    return checks["passed"]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--stage",
        choices=(
            "engine",
            "archive",
            "native",
            "all-light",
            "promote-native",
            "release-audit",
        ),
        default="release-audit",
    )
    args = parser.parse_args()
    if args.stage == "release-audit":
        passed = release_audit()
    elif args.stage == "promote-native":
        passed = promote_native_transactionally()
    else:
        passed = run_stage(args.stage)
    print(f"PLS power v1 factory stage={args.stage} passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
