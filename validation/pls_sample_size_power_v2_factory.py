#!/usr/bin/env python3
"""Focused source-tier qualification factory for PLS power v2.

The independent calibration is intentionally not repeated on every invocation.
It is reused only when its report is passing and no older than the exact
reference source. Changed product contracts are covered by narrow Rust and
Vitest commands; the final packaged identity is produced separately after the
coordinated frozen desktop build.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable


ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "validation/methods/pls_sample_size_power_v2.manifest.json"
SCHEMA_PATH = ROOT / "validation/methods/method_promotion_manifest.schema.json"
REFERENCE_SOURCE = ROOT / "validation/pls_sample_size_power_v2_reference.py"
REFERENCE_REPORT = ROOT / "validation/results/pls_sample_size_power_v2_reference_report.json"
REPORT_ROOT = ROOT / "validation/results/method_factory/pls_sample_size_power_v2"
FEATURE_ID = "qpls3.pls.sample_size_power"
METHOD_VERSION = "pls_sample_size_power_v2"
CATALOGUE_SNAPSHOT_DATE = "2026-08-12"


STAGES = {
    "engine": {
        "file": "engine_stage.identity.json",
        "roles": ["method_spec", "independent_reference", "simulation_report", "boundary_report"],
        "commands": [
            [
                "cargo", "test", "-p", "qpls-resampling",
                "v2_uses_exact_null_centered_plus_one_accounting_without_relabeling_v1",
                "--", "--nocapture",
            ],
        ],
    },
    "archive": {
        "file": "persistence_report.identity.json",
        "roles": ["persistence_report"],
        "commands": [
            [
                "cargo", "test", "-p", "qpls-project",
                "runner_generated_pls_sample_size_power_v2_round_trips_and_rejects_contract_tampering",
                "--", "--nocapture",
            ],
        ],
    },
    "native": {
        "file": "native_stage.identity.json",
        "roles": ["frontend_report", "export_report"],
        "commands": [
            [
                "cargo", "test", "-p", "qpls-cli",
                "typed_power_v2_export_exposes_exact_tail_accounting_and_rejects_tampering",
                "--", "--nocapture",
            ],
            [
                "npm.cmd", "test", "--", "--run",
                "src/domain/methodStatus.test.ts",
                "src/domain/methodApplicability.test.ts",
                "src/native/nativeAnalysisRecipe.test.ts",
                "src/native/nativePlsSampleSizePower.test.ts",
            ],
        ],
    },
}


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def descriptor(relative: str) -> dict[str, Any]:
    path = ROOT / relative
    if not path.is_file():
        raise FileNotFoundError(f"required source is missing: {relative}")
    return {"path": relative, "size": path.stat().st_size, "sha256": sha256(path)}


def source_paths(roles: Iterable[str]) -> list[str]:
    manifest = load_json(MANIFEST_PATH)
    governance = manifest["governance"]
    paths = {
        governance["manifest_path"],
        governance["schema_path"],
        governance["validator_path"],
        governance["focused_test_path"],
        "validation/pls_sample_size_power_v2_factory.py",
    }
    requirements = manifest["qualification"]["source_requirements"]
    for role in roles:
        paths.update(requirements[role])
    return sorted(paths)


def verify_reference_report() -> dict[str, Any]:
    if not REFERENCE_REPORT.is_file():
        raise FileNotFoundError("run the compact v2 independent calibration first")
    if REFERENCE_REPORT.stat().st_mtime_ns < REFERENCE_SOURCE.stat().st_mtime_ns:
        raise RuntimeError("the v2 independent calibration report predates its source")
    report = load_json(REFERENCE_REPORT)
    checks = report.get("checks", {})
    expected = {
        "report_kind": "quickpls_pls_sample_size_power_v2_independent_calibration",
        "passed": True,
        "method_version": METHOD_VERSION,
        "inference_method": "pls_pm_case_bootstrap_null_centered_two_sided_plus_one_v2",
        "profile": "compact",
    }
    for field, value in expected.items():
        if report.get(field) != value:
            raise RuntimeError(f"calibration report {field} mismatch")
    required_boolean_checks = [
        "all_runs_succeeded",
        "bootstrap_failure_rate_at_most_0_001",
        "each_null_rate_at_most_0_08",
        "pooled_null_rate_at_most_0_065",
        "pooled_null_wilson_contains_0_05",
        "signal_rejection_rate_at_least_0_60",
    ]
    if any(checks.get(name) is not True for name in required_boolean_checks):
        raise RuntimeError("one or more frozen v2 calibration checks did not pass")
    if len(report.get("scenarios", [])) != 3:
        raise RuntimeError("the compact v2 calibration must contain exactly three scenarios")
    return {
        "sha256": sha256(REFERENCE_REPORT),
        "pooled_null_rate": checks["pooled_null_rate"],
        "pooled_null_wilson_95": checks["pooled_null_wilson_95"],
        "bootstrap_failure_rate": checks["bootstrap_failure_rate"],
        "signal_rejection_rate": report["scenarios"][2]["rejection_rate"],
    }


def run_command(command: list[str]) -> dict[str, Any]:
    started = time.monotonic()
    completed = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    return {
        "command": command,
        "returncode": completed.returncode,
        "duration_seconds": round(time.monotonic() - started, 3),
        "stdout_tail": completed.stdout[-4_000:],
        "stderr_tail": completed.stderr[-4_000:],
    }


def write_stage(stage: str) -> tuple[Path, bool]:
    definition = STAGES[stage]
    calibration = verify_reference_report()
    executions = [run_command(command) for command in definition["commands"]]
    passed = all(item["returncode"] == 0 for item in executions)
    report = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": stage,
        "passed": passed,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "source_artifacts": [descriptor(path) for path in source_paths(definition["roles"])],
        "checks": {"independent_calibration": calibration, "commands": executions},
    }
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    path = REPORT_ROOT / definition["file"]
    path.write_text(json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n", encoding="utf-8")
    return path, passed


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("stages", nargs="*", choices=tuple(STAGES))
    parser.add_argument(
        "--rerun-calibration",
        action="store_true",
        help="Repeat the compact independent calibration before focused source checks.",
    )
    args = parser.parse_args()
    if args.rerun_calibration:
        completed = subprocess.run(
            [sys.executable, str(REFERENCE_SOURCE), "--profile", "compact", "--output", str(REFERENCE_REPORT)],
            cwd=ROOT,
        )
        if completed.returncode != 0:
            return completed.returncode
    stages = args.stages or list(STAGES)
    passed = True
    for stage in stages:
        path, stage_passed = write_stage(stage)
        print(f"{stage}: {'PASS' if stage_passed else 'FAIL'} -> {path.relative_to(ROOT)}")
        passed = passed and stage_passed
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
