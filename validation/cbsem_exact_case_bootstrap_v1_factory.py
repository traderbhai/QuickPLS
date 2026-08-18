#!/usr/bin/env python3
"""Source-bound method factory for the current exact-CFA bootstrap family.

The factory never builds or packages QuickPLS. Source roles are minted only
after their focused Rust, TypeScript, and Python gates pass. Release roles are
adapted only from a fresh method-scoped packaged report whose source and
release-binary descriptors still match repository bytes.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import time
from collections.abc import Iterable, Sequence
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

try:
    from validation.method_promotion_manifest import _verify_artifact, validate_manifest
except ModuleNotFoundError:
    from method_promotion_manifest import _verify_artifact, validate_manifest


ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "validation/methods/cbsem_exact_case_bootstrap_v1.manifest.json"
REPORT_ROOT = ROOT / "validation/results/method_factory/cbsem_exact_case_bootstrap_v1"
RAW_PACKAGED_REPORT = ROOT / "validation/results/cbsem_exact_case_bootstrap_v1_packaged_acceptance.json"
FACTORY_SOURCE = "validation/cbsem_exact_case_bootstrap_v1_factory.py"
FACTORY_TEST = "validation/test_cbsem_exact_case_bootstrap_v1_factory.py"
FEATURE_ID = "qpls3.cbsem.bootstrap"
METHOD_VERSION = "cbsem_exact_case_bootstrap_v1"
CATALOGUE_SNAPSHOT_DATE = "2026-08-12"

SOURCE_ROLES = (
    "method_spec",
    "independent_reference",
    "simulation_report",
    "boundary_report",
    "persistence_report",
    "frontend_report",
    "export_report",
)
PRIOR_AUDIT_ROLES = (*SOURCE_ROLES, "packaged_acceptance")
EXPECTED_PACKAGED_CHECKS = frozenset(
    {
        "setup",
        "invalid_setup_blocked",
        "execute_percentile",
        "execute_studentized",
        "execute_bca",
        "cancellation_retry",
        "result_identity",
        "xlsx_same_run",
        "save_reopen_same_run",
        "offline",
        "viewports",
        "process_cleanup",
    }
)


class FactoryError(RuntimeError):
    """A gate or evidence input failed closed."""


class DuplicateKeyError(ValueError):
    """Strict JSON rejected an ambiguous object."""


def _strict_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise DuplicateKeyError(f"duplicate JSON key: {key}")
        value[key] = item
    return value


def strict_load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8-sig") as handle:
        return json.load(
            handle,
            object_pairs_hook=_strict_pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                ValueError(f"non-finite JSON number: {value}")
            ),
        )


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _safe_path(root: Path, relative: str) -> Path:
    candidate = Path(relative)
    if candidate.is_absolute() or "\\" in relative or ".." in candidate.parts:
        raise FactoryError(f"unsafe repository path: {relative!r}")
    path = (root / candidate).resolve()
    try:
        path.relative_to(root.resolve())
    except ValueError as error:
        raise FactoryError(f"path escapes repository: {relative!r}") from error
    return path


def source_descriptors(root: Path, paths: Iterable[str]) -> list[dict[str, Any]]:
    descriptors: list[dict[str, Any]] = []
    for relative in sorted(set(paths)):
        path = _safe_path(root, relative)
        if not path.is_file():
            raise FactoryError(f"required source is missing: {relative}")
        descriptors.append(
            {"path": relative, "size": path.stat().st_size, "sha256": sha256_file(path)}
        )
    return descriptors


def verify_descriptors(
    root: Path,
    descriptors: Any,
    required: Iterable[str],
) -> tuple[bool, list[str]]:
    errors: list[str] = []
    if not isinstance(descriptors, list):
        return False, ["source_artifacts must be an array"]
    by_path: dict[str, dict[str, Any]] = {}
    for row in descriptors:
        if not isinstance(row, dict) or not isinstance(row.get("path"), str):
            errors.append("every source descriptor must be an object with a path")
            continue
        relative = row["path"]
        if relative in by_path:
            errors.append(f"duplicate source descriptor: {relative}")
            continue
        by_path[relative] = row
        try:
            path = _safe_path(root, relative)
        except FactoryError as error:
            errors.append(str(error))
            continue
        if not path.is_file():
            errors.append(f"described source is missing: {relative}")
            continue
        if row.get("size") != path.stat().st_size:
            errors.append(f"source size mismatch: {relative}")
        if row.get("sha256") != sha256_file(path):
            errors.append(f"source sha256 mismatch: {relative}")
    for relative in sorted(set(required) - set(by_path)):
        errors.append(f"required source descriptor is absent: {relative}")
    return not errors, errors


def load_manifest(root: Path = ROOT) -> dict[str, Any]:
    document = strict_load_json(root / MANIFEST_PATH.relative_to(ROOT))
    feature = document.get("feature", {}) if isinstance(document, dict) else {}
    expected = {
        "id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
    }
    if any(feature.get(key) != value for key, value in expected.items()):
        raise FactoryError("exact-CFA bootstrap manifest identity mismatch")
    return document


def role_sources(document: dict[str, Any], role: str) -> list[str]:
    governance = document["governance"]
    return sorted(
        {
            governance["manifest_path"],
            governance["schema_path"],
            governance["validator_path"],
            governance["focused_test_path"],
            FACTORY_SOURCE,
            FACTORY_TEST,
            *document["qualification"]["source_requirements"][role],
        }
    )


def _utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def run_command(command: Sequence[str], timeout: int = 1200) -> dict[str, Any]:
    started = time.monotonic()
    completed = subprocess.run(
        list(command),
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        env={**os.environ, "CARGO_BUILD_JOBS": "1"},
    )
    return {
        "command": list(command),
        "returncode": completed.returncode,
        "duration_seconds": round(time.monotonic() - started, 3),
        "stdout_tail": completed.stdout[-5000:],
        "stderr_tail": completed.stderr[-5000:],
        "passed": completed.returncode == 0,
    }


def _identity_verification() -> dict[str, Any]:
    return {
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


def write_identity_report(
    role: str,
    checks: dict[str, Any],
    *,
    extras: Iterable[str] = (),
    root: Path = ROOT,
) -> Path:
    if role not in {*SOURCE_ROLES, "packaged_acceptance", "method_audit"}:
        raise FactoryError(f"unknown evidence role: {role}")
    if checks.get("passed") is not True:
        raise FactoryError(f"refusing to mint failed {role} evidence")
    document = load_manifest(root)
    output_root = root / REPORT_ROOT.relative_to(ROOT)
    output_root.mkdir(parents=True, exist_ok=True)
    report = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": role,
        "passed": True,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "generated_at_utc": _utc_now(),
        "source_artifacts": source_descriptors(
            root, [*role_sources(document, role), *extras]
        ),
        "checks": checks,
    }
    path = output_root / f"{role}.identity.json"
    path.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return path


def _require_commands(executions: dict[str, dict[str, Any]]) -> None:
    failures = [name for name, row in executions.items() if row.get("passed") is not True]
    if failures:
        raise FactoryError("focused source gates failed: " + ", ".join(failures))


def mint_source_roles() -> list[Path]:
    """Run only the focused current-source gates and mint seven staged roles."""

    python = sys.executable
    native_files = [
        "src/domain/internalRecipeV4CbsemExecution.test.ts",
        "src/domain/internalRecipeV4CbsemWorkspace.test.ts",
        "src/domain/internalProjectSchema6ResultRead.test.ts",
        "src/native/nativeRecipeV4CbsemWorkspaceContracts.test.ts",
        "src/native/NativeRecipeV4CbsemWorkspace.test.tsx",
        "src/native/NativeDesktopApp.test.ts",
        "src/native/NativeCalculationDialog.test.ts",
        "src/native/nativePlsReadiness.test.ts",
    ]
    commands: dict[str, Sequence[str]] = {
        "manifest_contract": [python, "validation/method_promotion_contracts.py", str(MANIFEST_PATH.relative_to(ROOT))],
        "python_contract": [python, "-m", "pytest", "-q", "-p", "no:cacheprovider", "validation/test_cbsem_exact_case_bootstrap_v1_source_closure.py"],
        "compiler": ["cargo", "test", "-p", "qpls-core", "exact_cbsem_cfa_case_bootstrap_v2_is_explicit_bounded_and_identity_bound", "--", "--nocapture"],
        "execution": ["cargo", "test", "-p", "quickpls-desktop", "access_cancellation_and_unsupported_moments_remain_typed", "--", "--nocapture"],
        "append": ["cargo", "test", "-p", "quickpls-desktop", "command_access_accepts_only_historical_internal_or_exact_cbsem_standard_and_requires_an_absolute_path", "--", "--nocapture"],
        "read": ["cargo", "test", "-p", "quickpls-desktop", "access_digest_and_tampering_fail_closed", "--", "--nocapture"],
        "native": ["npx.cmd", "vitest", "run", *native_files],
        "export": ["npx.cmd", "vitest", "run", "src/native/NativeRecipeV4CbsemWorkspace.test.tsx", "src/domain/internalProjectSchema6ResultRead.test.ts"],
    }
    executions = {name: run_command(command) for name, command in commands.items()}
    _require_commands(executions)
    role_checks = {
        "method_spec": {"passed": True, "gates": {name: executions[name] for name in ("manifest_contract", "python_contract")}},
        "independent_reference": {"passed": True, "gates": {"python_contract": executions["python_contract"]}, "fixture": "compact deterministic Type-7, studentized, and BCa oracle"},
        "simulation_report": {"passed": True, "gates": {"python_contract": executions["python_contract"]}, "claim": "deterministic contract matrix only; no coverage or soak claim"},
        "boundary_report": {"passed": True, "gates": {name: executions[name] for name in ("python_contract", "compiler", "execution")}},
        "persistence_report": {"passed": True, "gates": {name: executions[name] for name in ("append", "read")}},
        "frontend_report": {"passed": True, "gates": {"native": executions["native"]}},
        "export_report": {"passed": True, "gates": {"export": executions["export"]}},
    }
    return [write_identity_report(role, role_checks[role]) for role in SOURCE_ROLES]


def validate_packaged_contract(document: Any) -> tuple[bool, list[str]]:
    errors: list[str] = []
    if not isinstance(document, dict):
        return False, ["packaged report root must be an object"]
    expected = {
        "passed": True,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "scope": METHOD_VERSION,
    }
    for key, value in expected.items():
        if document.get(key) != value:
            errors.append(f"packaged report {key} mismatch")
    checks = document.get("checks")
    if not isinstance(checks, dict) or set(checks) != EXPECTED_PACKAGED_CHECKS:
        errors.append("packaged report must contain exactly the 12 method-scoped checks")
    elif any(not isinstance(row, dict) or row.get("passed") is not True for row in checks.values()):
        errors.append("every packaged check must pass explicitly")
    else:
        offline = checks["offline"].get("evidence")
        if not (
            isinstance(offline, dict)
            and offline.get("passed") is True
            and offline.get("externalRequestCount") == 0
            and offline.get("externalRequests") == []
            and offline.get("analyticalWorkflowRequiresInternet") is False
            and offline.get("strictZeroProcessEgressClaimed") is False
            and offline.get("platformBackgroundEgressOutsidePageRequestScope") is True
        ):
            errors.append("packaged report must preserve bounded page/application functional-offline evidence")
        cleanup = checks["process_cleanup"]
        platform = cleanup.get("platform_background_egress_observation")
        observed = platform.get("platform_background_egress_observed") if isinstance(platform, dict) else None
        commercial = platform.get("commercial_zero_egress_passed") if isinstance(platform, dict) else None
        remote = platform.get("remote_connections") if isinstance(platform, dict) else None
        platform_truthful = (
            isinstance(platform, dict)
            and platform.get("passed") is True
            and platform.get("observation_kind") == "sampled_exact_process_tree_tcp_v1"
            and isinstance(platform.get("sample_count"), int)
            and platform.get("sample_count", 0) > 0
            and platform.get("root_present_every_sample") is True
            and isinstance(observed, bool)
            and isinstance(commercial, bool)
            and isinstance(remote, list)
            and bool(remote) is observed
            and commercial is (not observed)
            and cleanup.get("sampled_process_tree_zero_egress") is commercial
            and cleanup.get("network_sample_count") == platform.get("sample_count")
        )
        if not platform_truthful:
            errors.append("packaged report mislabels or omits sampled platform background egress")
    generated = document.get("generated_at_utc")
    if not isinstance(generated, str) or not generated.endswith("Z"):
        errors.append("packaged report must contain a UTC generation timestamp")
    binaries = document.get("binary_artifacts")
    if not isinstance(binaries, dict) or set(binaries) != {"desktop", "cli"}:
        errors.append("packaged report must bind exactly desktop and CLI binaries")
    return not errors, errors


def adapt_packaged_report(path: Path = RAW_PACKAGED_REPORT) -> Path:
    document = load_manifest()
    if not path.is_file():
        raise FactoryError(f"method-scoped packaged report is missing: {path}")
    raw = strict_load_json(path)
    contract_ok, errors = validate_packaged_contract(raw)
    required_sources = role_sources(document, "packaged_acceptance")
    source_ok, source_errors = verify_descriptors(ROOT, raw.get("source_artifacts"), required_sources)
    errors.extend(source_errors)
    binaries = raw.get("binary_artifacts", {}) if isinstance(raw, dict) else {}
    binary_errors: list[str] = []
    for name in ("desktop", "cli"):
        row = binaries.get(name) if isinstance(binaries, dict) else None
        if not isinstance(row, dict) or not isinstance(row.get("path"), str):
            binary_errors.append(f"{name} binary descriptor is missing")
            continue
        try:
            binary = _safe_path(ROOT, row["path"])
        except FactoryError as error:
            binary_errors.append(str(error))
            continue
        if not binary.is_file() or row.get("sha256") != sha256_file(binary):
            binary_errors.append(f"{name} binary bytes do not match packaged report")
    errors.extend(binary_errors)
    checks = {
        "passed": contract_ok and source_ok and not binary_errors,
        "raw_report": path.relative_to(ROOT).as_posix(),
        "contract_errors": errors,
        "source_binding_passed": source_ok,
        "binary_binding_passed": not binary_errors,
        "method_functional_offline": raw.get("checks", {}).get("offline", {}).get("evidence") if isinstance(raw, dict) else None,
        "platform_background_egress_observation": raw.get("checks", {}).get("process_cleanup", {}).get("platform_background_egress_observation") if isinstance(raw, dict) else None,
        "packaged_checks": raw.get("checks") if isinstance(raw, dict) else None,
    }
    return write_identity_report(
        "packaged_acceptance",
        checks,
        extras=[path.relative_to(ROOT).as_posix()],
    )


def audit_prior_roles() -> dict[str, Any]:
    document = load_manifest()
    expected_identity = {
        "passed": True,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
    }
    observed: list[dict[str, Any]] = []
    for stage, artifacts in document["qualification"]["evidence"].items():
        for artifact in artifacts:
            roles = artifact.get("roles", [])
            if roles == ["method_audit"]:
                continue
            passed, errors = _verify_artifact(artifact, document, ROOT, expected_identity)
            observed.append(
                {"stage": stage, "roles": roles, "path": artifact.get("path"), "passed": passed, "errors": errors}
            )
    role_counts = {
        role: sum(role in row["roles"] for row in observed) for role in PRIOR_AUDIT_ROLES
    }
    packaged_rows = [row for row in observed if row["roles"] == ["packaged_acceptance"]]
    packaged_identity: dict[str, Any] = {}
    if len(packaged_rows) == 1 and isinstance(packaged_rows[0].get("path"), str):
        packaged_path = ROOT / packaged_rows[0]["path"]
        if packaged_path.is_file():
            loaded = strict_load_json(packaged_path)
            if isinstance(loaded, dict):
                packaged_identity = loaded
    packaged_checks = packaged_identity.get("checks", {}) if isinstance(packaged_identity, dict) else {}
    method_offline = packaged_checks.get("method_functional_offline", {}) if isinstance(packaged_checks, dict) else {}
    platform = packaged_checks.get("platform_background_egress_observation", {}) if isinstance(packaged_checks, dict) else {}
    observed_egress = platform.get("platform_background_egress_observed") if isinstance(platform, dict) else None
    commercial_zero_egress_passed = platform.get("commercial_zero_egress_passed") if isinstance(platform, dict) else None
    remote_connections = platform.get("remote_connections") if isinstance(platform, dict) else None
    method_functional_offline = (
        isinstance(method_offline, dict)
        and method_offline.get("passed") is True
        and method_offline.get("externalRequestCount") == 0
        and method_offline.get("externalRequests") == []
        and method_offline.get("analyticalWorkflowRequiresInternet") is False
        and method_offline.get("strictZeroProcessEgressClaimed") is False
    )
    platform_egress_recorded_truthfully = (
        isinstance(platform, dict)
        and platform.get("passed") is True
        and platform.get("observation_kind") == "sampled_exact_process_tree_tcp_v1"
        and isinstance(platform.get("sample_count"), int)
        and platform.get("sample_count", 0) > 0
        and platform.get("root_present_every_sample") is True
        and isinstance(observed_egress, bool)
        and isinstance(commercial_zero_egress_passed, bool)
        and isinstance(remote_connections, list)
        and bool(remote_connections) is observed_egress
        and commercial_zero_egress_passed is (not observed_egress)
    )
    return {
        "passed": all(row["passed"] for row in observed)
        and all(count == 1 for count in role_counts.values())
        and method_functional_offline
        and platform_egress_recorded_truthfully,
        "verified_artifacts": observed,
        "role_counts": role_counts,
        "method_functional_offline": method_functional_offline,
        "platform_background_egress_recorded_truthfully": platform_egress_recorded_truthfully,
        "commercial_zero_egress_passed": commercial_zero_egress_passed,
        "release_claim_requires_separate_packaged_identity": True,
    }


def mint_method_audit() -> Path:
    checks = audit_prior_roles()
    extras = [row["path"] for row in checks["verified_artifacts"] if row.get("path")]
    return write_identity_report("method_audit", checks, extras=extras)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--source", action="store_true", help="run focused source gates and mint engine/archive/native roles")
    action.add_argument("--adapt-packaged", action="store_true", help="adapt a fresh packaged report and mint the final audit")
    action.add_argument("--audit", action="store_true", help="mint only the final audit from already-bound prior roles")
    parser.add_argument("--packaged-report", type=Path, default=RAW_PACKAGED_REPORT)
    args = parser.parse_args(argv)
    try:
        if args.source:
            paths = mint_source_roles()
        elif args.adapt_packaged:
            raw = args.packaged_report if args.packaged_report.is_absolute() else ROOT / args.packaged_report
            paths = [adapt_packaged_report(raw), mint_method_audit()]
            result = validate_manifest(MANIFEST_PATH)
            if not result["passed"] or result["derived_state"] != "release_qualified":
                raise FactoryError("final manifest did not derive release_qualified")
        else:
            paths = [mint_method_audit()]
    except (FactoryError, OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"CB-SEM exact-bootstrap factory failed closed: {error}", file=sys.stderr)
        return 1
    for path in paths:
        print(path.relative_to(ROOT).as_posix())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
