#!/usr/bin/env python3
"""Shared helpers for v1.2.1 second-batch method-promotion audits."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def report_passed(path: Path) -> bool:
    value = load_json(path)
    if value.get("passed") is True or value.get("status") == "passed":
        return True
    if value.get("qualification_passed") is True:
        return True
    if value.get("all_listed_artifacts_present") is True and value.get("all_listed_artifacts_passed") is True:
        return True
    checks = value.get("checks")
    if isinstance(checks, dict):
        return all(item.get("passed") is True for item in checks.values() if isinstance(item, dict))
    if isinstance(checks, list):
        return all(item.get("passed") is True or item.get("status") == "passed" for item in checks if isinstance(item, dict))
    return False


def audit_method(method_id: str, promoted_scope: str, required_reports: list[str], required_docs: list[str], extra_checks: list[dict] | None = None) -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    reports = []
    for name in required_reports:
        path = RESULTS / name
        reports.append({
            "path": str(path.relative_to(ROOT)),
            "present": path.exists(),
            "passed": path.exists() and report_passed(path),
        })
    docs = []
    for name in required_docs:
        path = ROOT / "docs" / "methods" / name
        docs.append({
            "path": str(path.relative_to(ROOT)),
            "present": path.exists(),
            "bytes": path.stat().st_size if path.exists() else None,
        })
    checks = extra_checks or []
    passed = all(item["passed"] for item in reports) and all(item["present"] for item in docs) and all(item.get("passed") for item in checks)
    output = RESULTS / f"{method_id}_method_promotion_audit.json"
    output.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "target": "v1_2_1_second_batch_method_promotion",
                "method_id": method_id,
                "promoted_scope": promoted_scope,
                "passed": passed,
                "reports": reports,
                "docs": docs,
                "checks": checks,
                "note": "Promotion is limited to this documented scope; broader variants remain experimental or unsupported.",
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    print(f"wrote {output} | passed={passed}")
    return 0 if passed else 1
