#!/usr/bin/env python3
"""Generate focused frontend evidence for structural randomization v1.

The gate binds the dedicated fail-closed projector and the catalog, recipe,
request, dialog, canonical hydration, result, export, and interpretation test
families, including report, publication-diagram, Trust Center, and controller
truth boundaries. It then runs the repository TypeScript project check. It does not
build the app, invoke Cargo, or launch a browser/native shell.
"""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "validation" / "results" / "structural_path_randomization_frontend_gate_report.json"
FEATURE_ID = "qpls3.inference.structural_path_randomization"
METHOD_VERSION = "freedman_lane_permutation_v1"
TARGET = "structural_path_randomization_v1_focused_frontend_gate"

FRONTEND_TEST_FILES = (
    "src/native/nativeStructuralPathRandomization.test.ts",
    "src/native/nativeAnalysisCatalog.test.ts",
    "src/native/nativeAnalysisRecipe.test.ts",
    "src/native/nativeCalculationRequest.test.ts",
    "src/native/NativeCalculationDialog.test.ts",
    "src/native/nativeCanonicalProject.test.ts",
    "src/native/nativeResults.test.ts",
    "src/native/nativeExportTables.test.ts",
    "src/domain/resultInterpretation.test.ts",
    "src/store.test.ts",
    "src/components/ReportsWorkspace.test.ts",
    "src/native/NativeExportDialog.test.ts",
    "src/domain/publicationDiagram.test.ts",
    "src/components/TrustCenterWorkspace.test.tsx",
    "src/native/nativeControllerContracts.test.ts",
)
FRONTEND_TYPESCRIPT_SOURCES = tuple(
    sorted(
        path.relative_to(ROOT).as_posix()
        for suffix in ("*.ts", "*.tsx")
        for path in (ROOT / "src").rglob(suffix)
        if path.is_file()
    )
)
FRONTEND_STYLESHEET_SOURCES = tuple(
    sorted(
        path.relative_to(ROOT).as_posix()
        for path in (ROOT / "src").rglob("*.css")
        if path.is_file()
    )
)
FRONTEND_GATE_SOURCES = tuple(
    dict.fromkeys(
        (
            "package.json",
            "package-lock.json",
            "index.html",
            "tsconfig.json",
            "tsconfig.app.json",
            "tsconfig.node.json",
            "vite.config.ts",
            *FRONTEND_TYPESCRIPT_SOURCES,
            *FRONTEND_STYLESHEET_SOURCES,
            "validation/structural_path_randomization_frontend_gate.py",
        )
    )
)
VITEST_COMMAND = ["npx", "vitest", "run", *FRONTEND_TEST_FILES, "--reporter=json"]
TSC_COMMAND = ["npx", "tsc", "-b", "--pretty", "false"]


def executable(command: str) -> str:
    return f"{command}.cmd" if os.name == "nt" else command


def run(command: list[str], *, timeout: int = 900) -> subprocess.CompletedProcess[str]:
    invoked = [executable(command[0]), *command[1:]]
    return subprocess.run(
        invoked,
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def artifact(relative: str) -> dict[str, object]:
    path = ROOT / relative
    return {
        "path": relative,
        "size": path.stat().st_size,
        "sha256": sha256_file(path),
        "modified_at_ns": path.stat().st_mtime_ns,
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
        raise SystemExit(f"structural randomization frontend gate sources are missing: {missing}")
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    before_artifacts = [artifact(relative) for relative in FRONTEND_GATE_SOURCES]
    vitest = run(VITEST_COMMAND)
    try:
        payload = json.loads(vitest.stdout)
    except (UnicodeError, json.JSONDecodeError):
        payload = {}
    raw_rows = payload.get("testResults") if isinstance(payload, dict) else []
    raw_rows = raw_rows if isinstance(raw_rows, list) else []
    rows: list[dict[str, object]] = []
    for raw in raw_rows:
        if not isinstance(raw, dict):
            continue
        assertions = raw.get("assertionResults")
        assertions = assertions if isinstance(assertions, list) else []
        rows.append(
            {
                "path": normalized_result_path(raw.get("name")),
                "status": raw.get("status"),
                "assertions": len(assertions),
                "passed_assertions": sum(
                    isinstance(assertion, dict) and assertion.get("status") == "passed"
                    for assertion in assertions
                ),
                "failed_assertions": sum(
                    isinstance(assertion, dict) and assertion.get("status") == "failed"
                    for assertion in assertions
                ),
            }
        )
    observed_paths = [row["path"] for row in rows]
    vitest_passed = (
        vitest.returncode == 0
        and payload.get("success") is True
        and len(observed_paths) == len(set(observed_paths))
        and frozenset(observed_paths) == frozenset(FRONTEND_TEST_FILES)
        and len(rows) == len(FRONTEND_TEST_FILES)
        and all(
            row["status"] == "passed"
            and isinstance(row["assertions"], int)
            and row["assertions"] > 0
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
            "passed": tsc.returncode == 0,
            "exit_code": tsc.returncode,
            "stdout_tail": tsc.stdout[-2_000:],
            "stderr_tail": tsc.stderr[-2_000:],
        },
        "source_artifacts": source_artifacts,
        "source_stable_during_gate": source_stable,
        "passed": vitest_passed and tsc.returncode == 0 and source_stable,
    }
    REPORT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {REPORT} | passed={report['passed']}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
