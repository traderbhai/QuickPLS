"""Fresh archive round-trip and fail-closed endogeneity persistence checks."""

from __future__ import annotations

import argparse
from typing import Any

from endogeneity_factory_common import (
    optionally_write_identity_report,
    run_command,
)


SOURCE = "validation/endogeneity_persistence_gate.py"


def run_persistence_gate() -> dict[str, Any]:
    tests = [
        "tests::runner_generated_endogeneity_appends_round_trips_and_rejects_contract_tampering",
        "tests::changed_payload_is_rejected_by_its_manifest_checksum",
    ]
    executions: list[dict[str, Any]] = []
    passed = True
    for test_name in tests:
        completed, execution = run_command(
            [
                "cargo",
                "test",
                "-p",
                "qpls-project",
                test_name,
                "--",
                "--exact",
            ],
            timeout=900,
        )
        output = completed.stdout + completed.stderr
        test_passed = (
            completed.returncode == 0
            and "1 passed" in output
            and "0 failed" in output
        )
        executions.append(
            {"test": test_name, "passed": test_passed, "execution": execution}
        )
        passed = passed and test_passed
    typed = executions[0]["passed"]
    checksum = executions[1]["passed"]
    return {
        "passed": passed,
        "checks": {
            "typed_result_round_trip": typed,
            "exact_nested_method_version_required": typed,
            "exact_transform_identity_required": typed,
            "coefficient_and_t_consistency_required": typed,
            "applicability_matches_saved_skewness": typed,
            "duplicate_path_diagnostics_rejected": typed,
            "malformed_uncertainty_rejected": typed,
            "dataset_fingerprint_mismatch_rejected": typed,
            "resampling_reinterpretation_rejected": typed,
            "legacy_provenance_reinterpretation_rejected": typed,
            "archive_checksum_tamper_rejected": checksum,
        },
        "executions": executions,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-identity", action="store_true")
    args = parser.parse_args()
    detail = run_persistence_gate()
    report = optionally_write_identity_report(
        "persistence_report",
        write_identity=args.write_identity,
        passed=detail["passed"],
        checks=detail,
        extras=[
            SOURCE,
            "crates/qpls-project/src/lib.rs",
            "crates/qpls-project/src/archive_integrity.rs",
            "crates/qpls-runner/src/lib.rs",
            "crates/qpls-core/src/contract.rs",
        ],
    )
    print(
        f"endogeneity persistence passed={detail['passed']} "
        f"identity={report or 'not-written'}"
    )
    return 0 if detail["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
