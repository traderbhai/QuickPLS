#!/usr/bin/env python3
"""Fail-closed HTMT archive/persistence qualification gate."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any

from htmt_plus_v1_factory_common import (
    ROOT,
    require_exact_case_ids,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/htmt_release_persistence_gate.py"
PROJECT_SOURCE = ROOT / "crates" / "qpls-project" / "src" / "lib.rs"
ARCHIVE_SOURCE = ROOT / "crates" / "qpls-project" / "src" / "archive_integrity.rs"
REQUIRED_CASE_IDS = (
    "feature_identity",
    "method_version",
    "dataset_fingerprint",
    "checksum",
    "duplicate_entry",
    "malformed_payload",
    "legacy_reinterpretation",
    "interrupted_save",
    "decision_tamper",
    "usable_index_digest_tamper",
    "pair_unavailable_index_tamper",
    "future_version_read_only",
    "cancelled_run_archive_unchanged",
)


def source_persistence_checks() -> dict[str, Any]:
    project = PROJECT_SOURCE.read_text(encoding="utf-8")
    archive = ARCHIVE_SOURCE.read_text(encoding="utf-8")
    version_match = re.search(
        r"pub const PROJECT_ARCHIVE_VERSION:\s*u32\s*=\s*(\d+);", project
    )
    cases = {
        "current_archive_version_declared": version_match is not None,
        "method_scoped_round_trip_test_present": (
            "fn bootstrap_pls_payload_round_trips_with_recipe_provenance" in project
        ),
        "decision_tamper_test_present": (
            'cell["upper_bound_below_critical_value"]' in project
        ),
        "usable_index_digest_tamper_test_present": (
            '["usable_replicate_indices_sha256"]' in project
            and 'json!("00".repeat(32))' in project
        ),
        "pair_unavailable_ledger_validated": (
            "pair_unavailable_replicates.iter().any" in project
        ),
        "future_version_guard_present": (
            "source_archive_version > PROJECT_ARCHIVE_VERSION" in project
            or "schema_version > PROJECT_ARCHIVE_VERSION" in project
        ),
        "checksum_validation_present": "validate_manifest_checksums" in project,
        "duplicate_archive_entry_guard_present": (
            "validate_expected_project_entries" in project
            and "expected_project_entries" in project
        ),
        "archive_integrity_module_present": (
            "verify_archive_checksums" in archive and "duplicate" in archive.casefold()
        ),
    }
    return {
        "passed": all(cases.values()),
        "cases": cases,
        "observed_archive_version": (
            int(version_match.group(1)) if version_match else None
        ),
        "qualification_case_ids": list(REQUIRED_CASE_IDS),
    }


def validate_qualification_report(path: Path) -> dict[str, Any]:
    document = strict_load_json(path)
    cases = require_exact_case_ids(document, REQUIRED_CASE_IDS)
    identity = (
        document.get("qualification_id") == "qpls3.assessment.htmt.qualification_v2"
        and document.get("method_version") == "ringle_et_al_htmt_plus_v1"
        and document.get("atomic_rejection") is True
        and document.get("partial_commits") == 0
        and document.get("archive_changed_after_cancel") is False
    )
    return {
        "passed": identity and cases["passed"],
        "identity": identity,
        "cases": cases,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--qualification-report", type=Path)
    parser.add_argument("--admit", action="store_true")
    args = parser.parse_args()
    source_checks = source_persistence_checks()
    checks: dict[str, Any] = {"source_contract": source_checks}
    blockers = [
        "fresh_archive_save_close_reopen_execution_not_supplied",
        "interrupted_save_and_cancellation_archive_checks_not_executed",
        "historical_and_future_archive_corpus_not_executed",
    ]
    qualification_evidence = False
    if args.qualification_report:
        qualification = validate_qualification_report(args.qualification_report)
        checks["qualification_execution"] = qualification
        if qualification["passed"]:
            blockers = []
            qualification_evidence = args.admit
        else:
            blockers.append("qualification_report_failed_contract")
    passed = source_checks["passed"] and (
        not args.qualification_report or checks["qualification_execution"]["passed"]
    )
    report = write_identity_report(
        "persistence_report",
        stage="persistence_export",
        passed=passed,
        checks=checks,
        blockers=blockers,
        extras=[
            SOURCE,
            "crates/qpls-project/src/lib.rs",
            "crates/qpls-project/src/archive_integrity.rs",
        ],
        qualification_evidence=qualification_evidence,
    )
    print(
        json.dumps(
            {
                "passed": passed,
                "qualification_evidence": qualification_evidence,
                "blockers": blockers,
                "report": report.relative_to(ROOT).as_posix(),
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0 if passed and (not args.admit or qualification_evidence) else 1


if __name__ == "__main__":
    raise SystemExit(main())
