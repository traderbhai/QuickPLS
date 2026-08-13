#!/usr/bin/env python3
"""Run and bind the four regression-bootstrap failure-boundary Rust tests.

This is a release-gate generator. It performs one serial release-profile Cargo
test build, discovers the exact produced lib-test executable from Cargo JSON,
then invokes each frozen test by its full exact name. Promotion consumes the
resulting report and re-hashes the same executable; source names alone never
satisfy the gate.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
REPORT = RESULTS / "regression_bootstrap_failure_boundary_test_report.json"
FEATURE_ID = "qpls3.standalone.regression_bootstrap"
METHOD_VERSION = "regression_bootstrap_v1"
TARGET = "regression_bootstrap_v1_failure_boundary_rust_tests"
EXPECTED_TESTS = frozenset(
    {
        "regression_bootstrap_failure_boundary_listwise_complete_cases_are_the_only_sampling_frame",
        "regression_bootstrap_failure_boundary_captures_zero_based_single_class_replicates",
        "regression_bootstrap_failure_boundary_rejects_below_ninety_percent_usable",
        "regression_bootstrap_failure_boundary_real_delete_one_failure_disables_all_bca",
    }
)
EXPECTED_ARCHIVE_TESTS = frozenset(
    {
        "regression_bootstrap_json_roundtrip_tolerance_is_narrow",
        "regression_bootstrap_append_save_reopen_and_tamper_contract_are_atomic",
    }
)


def run(command: list[str], *, timeout: int = 1200) -> subprocess.CompletedProcess[str]:
    environment = dict(os.environ)
    environment["CARGO_BUILD_JOBS"] = "1"
    return subprocess.run(
        command,
        cwd=ROOT,
        env=environment,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )


def artifact(path: Path) -> dict[str, object]:
    contents = path.read_bytes()
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "size": len(contents),
        "sha256": hashlib.sha256(contents).hexdigest(),
    }


def discover_test_executable(output: str, target_name: str) -> Path:
    candidates: list[Path] = []
    for line in output.splitlines():
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        executable = message.get("executable")
        target = message.get("target")
        profile = message.get("profile")
        if (
            message.get("reason") == "compiler-artifact"
            and isinstance(target, dict)
            and target.get("name") == target_name
            and isinstance(profile, dict)
            and profile.get("test") is True
            and isinstance(executable, str)
        ):
            candidates.append(Path(executable).resolve())
    unique = list(dict.fromkeys(candidates))
    if len(unique) != 1 or not unique[0].is_file():
        raise RuntimeError(f"Expected one {target_name} lib-test executable; observed {unique}")
    executable = unique[0]
    expected_parent = (ROOT / "target" / "release" / "deps").resolve()
    if executable.parent != expected_parent or not executable.name.startswith(f"{target_name}-"):
        raise RuntimeError(f"Cargo reported an unexpected test executable: {executable}")
    return executable


def main() -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    suites = {
        "qpls_resampling": {"package": "qpls-resampling", "tests": EXPECTED_TESTS},
        "qpls_project": {"package": "qpls-project", "tests": EXPECTED_ARCHIVE_TESTS},
    }
    build_commands: dict[str, list[str]] = {}
    executables: dict[str, Path] = {}
    listed_names: dict[str, set[str]] = {}
    for target_name, suite in suites.items():
        command = [
            "cargo", "test", "--release", "-p", str(suite["package"]), "--lib",
            "--no-run", "--message-format=json",
        ]
        build_commands[target_name] = command
        build = run(command)
        if build.returncode != 0:
            raise SystemExit(f"Cargo test build failed for {target_name} ({build.returncode}):\n{build.stderr[-4000:]}")
        executable = discover_test_executable(build.stdout, target_name)
        executables[target_name] = executable
        listed = run([str(executable), "--list"], timeout=120)
        if listed.returncode != 0:
            raise SystemExit(f"Rust test listing failed for {target_name} ({listed.returncode}):\n{listed.stderr[-4000:]}")
        listed_names[target_name] = {
            line.rsplit(": test", 1)[0].strip()
            for line in listed.stdout.splitlines()
            if line.rstrip().endswith(": test")
        }
    selected: dict[str, str] = {}
    test_targets: dict[str, str] = {}
    for target_name, suite in suites.items():
        for expected in suite["tests"]:
            matches = [name for name in listed_names[target_name] if name == expected or name.endswith(f"::{expected}")]
            if len(matches) != 1:
                raise SystemExit(f"Expected exactly one {target_name} test ending in {expected!r}; observed {matches}")
            selected[expected] = matches[0]
            test_targets[expected] = target_name
    checks: dict[str, bool] = {}
    archive_checks: dict[str, bool] = {}
    executions: dict[str, dict[str, object]] = {}
    for short_name in sorted(EXPECTED_TESTS | EXPECTED_ARCHIVE_TESTS):
        full_name = selected[short_name]
        target_name = test_targets[short_name]
        executable = executables[target_name]
        completed = run([str(executable), full_name, "--exact", "--test-threads=1"], timeout=300)
        passed = completed.returncode == 0 and "1 passed; 0 failed" in completed.stdout
        (checks if short_name in EXPECTED_TESTS else archive_checks)[short_name] = passed
        executions[short_name] = {
            "target": target_name,
            "full_name": full_name,
            "exit_code": completed.returncode,
            "passed": passed,
            "stdout_tail": completed.stdout[-2000:],
            "stderr_tail": completed.stderr[-2000:],
        }
    report = {
        "schema_version": 1,
        "target": TARGET,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "passed": (
            frozenset(checks) == EXPECTED_TESTS
            and frozenset(archive_checks) == EXPECTED_ARCHIVE_TESTS
            and all(checks.values())
            and all(archive_checks.values())
        ),
        "checks": checks,
        "archive_checks": archive_checks,
        "build_commands": build_commands,
        "test_executables": {
            target_name: artifact(executable)
            for target_name, executable in executables.items()
        },
        "executions": executions,
    }
    REPORT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {REPORT} | passed={report['passed']}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
