"""Fresh PCA v1 archive round-trip and fail-closed persistence evidence."""

from __future__ import annotations

from pca_v1_factory_common import run_command, write_identity_report


SOURCE = "validation/pca_persistence_gate.py"
PROJECT_SOURCE = "crates/qpls-project/src/lib.rs"
ARCHIVE_SOURCE = "crates/qpls-project/src/archive_integrity.rs"


def run_persistence_gate() -> dict:
    tests = [
        "tests::runner_generated_pca_v1_commits_saves_reopens_and_rejects_contract_tampering",
        "tests::changed_payload_is_rejected_by_its_manifest_checksum",
    ]
    executions = []
    passed = True
    for test_name in tests:
        completed, execution = run_command(
            ["cargo", "test", "-p", "qpls-project", test_name],
            timeout=900,
        )
        text = completed.stdout + completed.stderr
        test_passed = completed.returncode == 0 and "1 passed" in text and "0 failed" in text
        executions.append(
            {
                "test": test_name,
                "passed": test_passed,
                "execution": execution,
            }
        )
        passed = passed and test_passed
    return {
        "passed": passed,
        "checks": {
            "typed_pca_identity_round_trip": executions[0]["passed"],
            "exact_payload_values_survive_reopen": executions[0]["passed"],
            "method_version_tamper_rejected_atomically": executions[0]["passed"],
            "component_loading_score_tamper_rejected_atomically": executions[0]["passed"],
            "recipe_result_mismatch_rejected_atomically": executions[0]["passed"],
            "invalid_reopened_payload_cannot_be_resaved": executions[0]["passed"],
            "archive_checksum_tamper_rejected": executions[1]["passed"],
        },
        "executions": executions,
    }


def main() -> int:
    detail = run_persistence_gate()
    path = write_identity_report(
        "persistence_report",
        passed=detail["passed"],
        checks=detail,
        extras=[SOURCE, PROJECT_SOURCE, ARCHIVE_SOURCE],
    )
    print(f"wrote {path} | passed={detail['passed']}")
    return 0 if detail["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
