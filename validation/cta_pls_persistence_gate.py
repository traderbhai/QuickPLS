"""Fresh CTA-PLS v1 archive round-trip and fail-closed persistence evidence."""

from __future__ import annotations

from cta_pls_v1_factory_common import run_command, write_identity_report


SOURCE = "validation/cta_pls_persistence_gate.py"
PROJECT_SOURCE = "crates/qpls-project/src/lib.rs"
ARCHIVE_SOURCE = "crates/qpls-project/src/archive_integrity.rs"


def run_persistence_gate() -> dict:
    tests = [
        "tests::runner_generated_cta_pls_appends_round_trips_and_rejects_contract_tampering",
        "tests::plsc_and_wpls_payloads_round_trip_and_reject_contract_tampering",
        "tests::changed_payload_is_rejected_by_its_manifest_checksum",
    ]
    executions = []
    passed = True
    for test_name in tests:
        completed, execution = run_command(
            ["cargo", "test", "-p", "qpls-project", test_name],
            timeout=900,
        )
        output = completed.stdout + completed.stderr
        test_passed = completed.returncode == 0 and "1 passed" in output and "0 failed" in output
        executions.append({"test": test_name, "passed": test_passed, "execution": execution})
        passed = passed and test_passed
    method_test = executions[0]["passed"]
    shared_identity_test = executions[1]["passed"]
    checksum_test = executions[2]["passed"]
    categories = {
        "feature_identity": method_test,
        "method_version": method_test,
        "dataset_fingerprint": method_test and shared_identity_test,
        "checksum": checksum_test,
        "malformed_payload": method_test,
        "legacy_reinterpretation": method_test,
    }
    return {
        "passed": passed and all(categories.values()),
        "tamper_categories": categories,
        "checks": {
            "typed_cta_identity_round_trip": method_test,
            "exact_tetrads_and_maxima_survive_reopen": method_test,
            "method_version_tamper_rejected_atomically": method_test,
            "unknown_and_duplicate_pairings_rejected_atomically": method_test,
            "signed_absolute_and_maximum_tamper_rejected_atomically": method_test,
            "resampling_identity_tamper_rejected_atomically": method_test,
            "shared_dataset_fingerprint_mutation_guard_exercised": shared_identity_test,
            "legacy_method_version_not_reinterpreted_as_current": method_test,
            "archive_checksum_tamper_rejected": checksum_test,
        },
        "executions": executions,
    }


def main() -> int:
    detail = run_persistence_gate()
    report = write_identity_report(
        "persistence_report",
        passed=detail["passed"],
        checks=detail,
        extras=[SOURCE, PROJECT_SOURCE, ARCHIVE_SOURCE],
    )
    print(f"wrote {report} | passed={detail['passed']}")
    return 0 if detail["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
