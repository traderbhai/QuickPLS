"""Fresh archive round-trip and fail-closed evidence for CB-SEM ML v1."""

from __future__ import annotations

from cbsem_ml_v1_factory_common import engine_source_paths, run_command, write_identity_report


SOURCE = "validation/cbsem_ml_v1_persistence_gate.py"


def run_persistence_gate() -> dict:
    tests = [
        "runner_generated_cbsem_and_cfa_commit_save_reopen_and_reject_contract_tampering",
        "historical_cbsem_and_ols_ignore_status_annotations_but_reject_scientific_tampering",
        "changed_payload_is_rejected_by_its_manifest_checksum",
    ]
    rows = []
    for test_name in tests:
        completed, execution = run_command(
            ["cargo", "test", "-p", "qpls-project", test_name, "--"],
            timeout=1800,
        )
        output = completed.stdout + completed.stderr
        rows.append(
            {
                "test": test_name,
                "passed": completed.returncode == 0
                and "1 passed" in output
                and "0 failed" in output,
                "execution": execution,
            }
        )
    passed = all(row["passed"] for row in rows)
    return {
        "passed": passed,
        "checks": {
            "typed_cbsem_and_cfa_round_trip": rows[0]["passed"],
            "exact_parameter_fit_matrix_values_survive_reopen": rows[0]["passed"],
            "feature_and_method_version_tamper_rejected_atomically": rows[0]["passed"],
            "malformed_parameter_matrix_fit_tamper_rejected_atomically": rows[0]["passed"],
            "legacy_preview_not_reinterpreted": rows[1]["passed"],
            "archive_checksum_tamper_rejected": rows[2]["passed"],
        },
        "executions": rows,
    }


def main() -> int:
    checks = run_persistence_gate()
    path = write_identity_report(
        "persistence_report",
        passed=checks["passed"],
        checks=checks,
        extras=[SOURCE, *engine_source_paths()],
    )
    print(f"wrote {path} | passed={checks['passed']}")
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())

