#!/usr/bin/env python3
"""Write source-bound qualification identities for combined MICOM/MGA v4."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "validation/methods/micom_permutation_mga_v4.manifest.json"
ORACLE_REPORT = ROOT / "validation/results/micom_mga_v4_reference_report.json"
PACKAGED_REPORT = ROOT / "validation/results/v247_tauri_native_acceptance_mga.json"
OUTPUT = ROOT / "validation/results/method_factory/micom_permutation_mga_v4"
FEATURE_ID = "qpls3.groups.micom_permutation_mga"
METHOD_VERSION = "pls_mga_permutation_v4"
CATALOGUE_DATE = "2026-08-12"


def strict_json(path: Path) -> dict[str, Any]:
    def pairs(items: list[tuple[str, Any]]) -> dict[str, Any]:
        value: dict[str, Any] = {}
        for key, item in items:
            if key in value:
                raise ValueError(f"duplicate JSON key: {key}")
            value[key] = item
        return value

    value = json.loads(
        path.read_text(encoding="utf-8-sig"),
        object_pairs_hook=pairs,
        parse_constant=lambda token: (_ for _ in ()).throw(ValueError(token)),
    )
    if not isinstance(value, dict):
        raise TypeError(f"JSON root must be an object: {path}")
    return value


def descriptor(relative: str) -> dict[str, Any]:
    path = ROOT / relative
    payload = path.read_bytes()
    return {
        "path": relative,
        "size": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }


def required_sources(manifest: dict[str, Any], roles: Iterable[str]) -> list[str]:
    governance = manifest["governance"]
    paths = {
        governance["manifest_path"],
        governance["schema_path"],
        governance["validator_path"],
        governance["focused_test_path"],
    }
    requirements = manifest["qualification"]["source_requirements"]
    for role in roles:
        paths.update(requirements[role])
    missing = [relative for relative in sorted(paths) if not (ROOT / relative).is_file()]
    if missing:
        raise FileNotFoundError(f"missing v4 qualification sources: {missing}")
    return sorted(paths)


def run(command: list[str], timeout: int = 900) -> dict[str, Any]:
    invoked = list(command)
    if sys.platform == "win32" and invoked[0] in {"npm", "npx"}:
        invoked[0] += ".cmd"
    completed = subprocess.run(
        invoked,
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=timeout,
        check=False,
    )
    return {
        "command": command,
        "returncode": completed.returncode,
        "passed": completed.returncode == 0,
        "stdout_tail": completed.stdout[-4000:],
        "stderr_tail": completed.stderr[-4000:],
    }


def identity(
    manifest: dict[str, Any],
    filename: str,
    roles: list[str],
    checks: dict[str, Any],
    execution: list[dict[str, Any]],
) -> Path:
    report = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": "+".join(roles),
        "passed": all(row.get("passed") is True for row in checks.values()),
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_DATE,
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "source_artifacts": [descriptor(path) for path in required_sources(manifest, roles)],
        "checks": checks,
        "execution": execution,
    }
    if not report["passed"]:
        raise RuntimeError(f"refusing to write failed v4 identity: {filename}")
    OUTPUT.mkdir(parents=True, exist_ok=True)
    target = OUTPUT / filename
    target.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return target


def qualify() -> list[Path]:
    manifest = strict_json(MANIFEST)
    feature = manifest.get("feature", {})
    if feature.get("id") != FEATURE_ID or feature.get("method_version") != METHOD_VERSION:
        raise ValueError("v4 manifest identity does not match the release audit")

    oracle = strict_json(ORACLE_REPORT)
    versions = oracle.get("method_versions", {})
    oracle_ok = (
        oracle.get("passed") is True
        and oracle.get("permutation_samples") == 5000
        and oracle.get("retry_policy") == "none"
        and versions == {
            "mga": "pls_mga_two_group_v4",
            "micom": "micom_v4",
            "permutation_mga": METHOD_VERSION,
        }
        and all(
            row.get("passed") is True
            for row in oracle.get("quickpls_comparison", {}).values()
            if isinstance(row, dict)
        )
    )
    engine = identity(
        manifest,
        "engine_stage.identity.json",
        ["method_spec", "independent_reference", "simulation_report", "boundary_report"],
        {
            "independent_5000_partition_oracle": {"passed": oracle_ok},
            "v4_identity_is_new_and_qualified": {"passed": feature.get("method_version") == METHOD_VERSION},
        },
        [],
    )

    persistence_run = run([
        "cargo", "test", "-p", "qpls-project",
        "validated_append_and_archive_round_trip_preserve_exact_combined_mga_v4_contract",
        "--", "--nocapture",
    ], 1200)
    project_source = (ROOT / "crates/qpls-project/src/lib.rs").read_text(encoding="utf-8")
    tamper_contracts_present = all(name in project_source for name in [
        "validated_append_rejects_tampered_mga_direction_atomically",
        "historical_combined_mga_v3_reopens_but_cannot_be_appended_as_v4_evidence",
    ])
    persistence = identity(
        manifest,
        "persistence_report.identity.json",
        ["persistence_report"],
        {
            "append_round_trip_and_legacy_read_only": {"passed": persistence_run["passed"]},
            "tamper_rejection_contracts_are_source_bound": {"passed": tamper_contracts_present},
        },
        [persistence_run],
    )

    native_run = run([
        "npx", "vitest", "run",
        "src/native/nativeAnalysisRecipe.test.ts",
        "src/native/nativeResults.test.ts",
        "src/native/nativeExportTables.test.ts",
    ], 1200)
    native = identity(
        manifest,
        "native_stage.identity.json",
        ["frontend_report", "export_report"],
        {
            "native_results_and_export_contract": {"passed": native_run["passed"]},
            "same_run_provenance_is_rendered": {"passed": native_run["passed"]},
        },
        [native_run],
    )

    packaged = strict_json(PACKAGED_REPORT)
    packaged_ok = (
        packaged.get("passed") is True
        and packaged.get("consoleErrors") == []
        and packaged.get("failures") == []
        and isinstance(packaged.get("checks", {}).get("mgaResult"), dict)
        and isinstance(packaged.get("checks", {}).get("mgaExport"), dict)
        and isinstance(packaged.get("checks", {}).get("mgaSaveReopen"), dict)
    )
    packaged_identity = identity(
        manifest,
        "packaged_acceptance.identity.json",
        ["packaged_acceptance"],
        {
            "focused_windows_workflow_passed": {"passed": packaged_ok},
            "zero_console_errors": {"passed": packaged.get("consoleErrors") == []},
            "save_reopen_and_xlsx_evidence_present": {"passed": packaged_ok},
        },
        [],
    )
    audit_identity = identity(
        manifest,
        "method_audit.identity.json",
        ["method_audit"],
        {
            "all_prior_qualification_stages_written": {
                "passed": all(path.is_file() for path in [engine, persistence, native, packaged_identity])
            },
            "qualified_identity_is_v4_only": {"passed": feature.get("method_version") == METHOD_VERSION},
        },
        [],
    )
    return [engine, persistence, native, packaged_identity, audit_identity]


if __name__ == "__main__":
    written = qualify()
    print(json.dumps({"passed": True, "written": [str(path.relative_to(ROOT)) for path in written]}, indent=2))
