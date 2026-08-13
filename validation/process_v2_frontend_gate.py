#!/usr/bin/env python3
"""Generate focused machine-readable PROCESS v2 frontend evidence.

This runner executes the frozen focused Vitest manifest and the repository
TypeScript project check. It does not build the app, launch a browser, or run
Cargo. Promotion independently re-hashes every declared source input.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path

from process_v2_method_promotion_audit import (
    FEATURE_ID,
    FRONTEND_GATE_SOURCES,
    FRONTEND_TEST_FILES,
    METHOD_VERSION,
)


ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "validation" / "results" / "process_v2_frontend_gate_report.json"
TARGET = "process_v2_focused_frontend_gate"
VITEST_COMMAND = ["npx", "vitest", "run", *FRONTEND_TEST_FILES, "--reporter=json"]
TSC_COMMAND = ["npx", "tsc", "-b", "--pretty", "false"]


def executable(command: str) -> str:
    return f"{command}.cmd" if os.name == "nt" else command


def run(command: list[str], *, timeout: int = 600) -> subprocess.CompletedProcess[str]:
    invoked = [executable(command[0]), *command[1:]]
    return subprocess.run(
        invoked, cwd=ROOT, capture_output=True, text=True, timeout=timeout, check=False
    )


def artifact(relative: str) -> dict[str, object]:
    path = ROOT / relative
    contents = path.read_bytes()
    return {
        "path": relative,
        "size": len(contents),
        "sha256": hashlib.sha256(contents).hexdigest(),
    }


def normalized_result_path(value: object) -> str | None:
    if not isinstance(value, str):
        return None
    path = Path(value)
    if not path.is_absolute():
        path = ROOT / path
    try:
        return path.resolve().relative_to(ROOT.resolve()).as_posix()
    except ValueError:
        return None


def main() -> int:
    missing = [relative for relative in FRONTEND_GATE_SOURCES if not (ROOT / relative).is_file()]
    if missing:
        raise SystemExit(f"PROCESS v2 frontend gate sources are missing: {missing}")
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    before_artifacts = [artifact(relative) for relative in FRONTEND_GATE_SOURCES]
    vitest = run(VITEST_COMMAND)
    try:
        payload = json.loads(vitest.stdout)
    except (UnicodeError, json.JSONDecodeError):
        payload = {}
    raw_rows = payload.get("testResults") if isinstance(payload, dict) else []
    raw_rows = raw_rows if isinstance(raw_rows, list) else []
    rows = []
    for row in raw_rows:
        if not isinstance(row, dict):
            continue
        assertions = row.get("assertionResults")
        assertions = assertions if isinstance(assertions, list) else []
        rows.append({
            "path": normalized_result_path(row.get("name")),
            "status": row.get("status"),
            "assertions": len(assertions),
            "passed_assertions": sum(
                isinstance(assertion, dict) and assertion.get("status") == "passed"
                for assertion in assertions
            ),
            "failed_assertions": sum(
                isinstance(assertion, dict) and assertion.get("status") == "failed"
                for assertion in assertions
            ),
        })
    observed_paths = [row["path"] for row in rows]
    vitest_passed = (
        vitest.returncode == 0
        and payload.get("success") is True
        and len(observed_paths) == len(set(observed_paths))
        and frozenset(observed_paths) == frozenset(FRONTEND_TEST_FILES)
        and len(rows) == len(FRONTEND_TEST_FILES)
        and all(
            row["status"] == "passed" and row["assertions"] > 0
            and row["passed_assertions"] == row["assertions"]
            and row["failed_assertions"] == 0
            for row in rows
        )
        and payload.get("numFailedTests") == 0
        and payload.get("numPendingTests") == 0
        and isinstance(payload.get("numTotalTests"), int)
        and payload["numTotalTests"] > 0
        and payload.get("numPassedTests") == payload["numTotalTests"]
    )
    tsc = run(TSC_COMMAND)
    tsc_passed = tsc.returncode == 0
    source_artifacts = [artifact(relative) for relative in FRONTEND_GATE_SOURCES]
    source_stable = source_artifacts == before_artifacts
    report = {
        "schema_version": 1,
        "target": TARGET,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "commands": {"vitest": VITEST_COMMAND, "tsc": TSC_COMMAND},
        "test_files": list(FRONTEND_TEST_FILES),
        "vitest": {
            "passed": vitest_passed,
            "exit_code": vitest.returncode,
            "test_files": rows,
            "total_tests": payload.get("numTotalTests"),
            "passed_tests": payload.get("numPassedTests"),
            "failed_tests": payload.get("numFailedTests"),
            "pending_tests": payload.get("numPendingTests"),
            "stdout_tail": vitest.stdout[-2_000:],
            "stderr_tail": vitest.stderr[-2_000:],
        },
        "tsc": {
            "passed": tsc_passed,
            "exit_code": tsc.returncode,
            "stdout_tail": tsc.stdout[-2_000:],
            "stderr_tail": tsc.stderr[-2_000:],
        },
        "source_artifacts": source_artifacts,
        "source_stable_during_gate": source_stable,
        "passed": vitest_passed and tsc_passed and source_stable,
    }
    REPORT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {REPORT} | passed={report['passed']}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
