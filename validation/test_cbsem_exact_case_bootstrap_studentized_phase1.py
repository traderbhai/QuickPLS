from __future__ import annotations

import copy
import json
import subprocess
import sys
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator, ValidationError

from validation.cbsem_exact_case_bootstrap_studentized_phase1 import (
    DEFAULT_MANIFEST,
    MANIFEST_SCHEMA,
    REPORT_SCHEMA,
    payload_acceptance_reasons,
    validate_manifest,
)


ROOT = Path(__file__).resolve().parents[1]


def test_manifest_is_immutable_phase0_bound_and_predeclares_the_applied_cells() -> None:
    manifest, digest = validate_manifest(DEFAULT_MANIFEST)

    assert len(digest) == 64
    assert manifest["phase0_binding"]["report_sha256"] == (
        "37c489462887485b102252c7074b3be8a7fa9d302535bb011b558d6e0de5479e"
    )
    assert manifest["dimension_contract"] == {
        "n_complete_cases": 180,
        "v_observed_variables": 9,
        "p_free_parameter_rows": 18,
        "d_optimizer_dimensions": None,
        "d_status": "unavailable_not_exposed_by_public_exact_plan_api",
        "product_cap_effect": "blocked_missing_optimizer_dimension",
    }
    assert [
        (
            case["case_id"],
            case["replicates"],
            case["workers"],
            case["cancel_after"],
        )
        for case in manifest["cases"]
    ] == [
        ("applied_n180_v9_p18_b5000_w12", 5000, 12, None),
        ("applied_n180_v9_p18_b10000_w12", 10000, 12, None),
        ("cancellation_n180_v9_p18_b10000_w12", 10000, 12, 10),
    ]
    assert manifest["execution_policy"]["probe_runs"] == 1
    assert manifest["execution_policy"]["survivor_warmup_runs"] == 1
    assert manifest["execution_policy"]["survivor_measured_runs"] == 5


def test_runtime_ceilings_are_fixed_phase0_extrapolations_and_retrofit_is_rejected(
    tmp_path: Path,
) -> None:
    manifest = json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    derivation = manifest["acceptance_contract"]["runtime_derivation"]
    assert derivation["safety_factor"] == 2.0
    for case in manifest["cases"][:2]:
        expected = (
            derivation["baseline_p95_elapsed_seconds"]
            * case["replicates"]
            / derivation["baseline_replicates"]
            * derivation["safety_factor"]
        )
        assert case["maximum_p95_elapsed_seconds"] == expected

    tampered = copy.deepcopy(manifest)
    tampered["cases"][0]["maximum_p95_elapsed_seconds"] += 1.0
    tampered_path = tmp_path / "retrofitted-manifest.json"
    tampered_path.write_text(json.dumps(tampered), encoding="utf-8")
    with pytest.raises(ValueError, match="runtime ceiling was retrofitted"):
        validate_manifest(tampered_path)


def test_acceptance_gate_requires_exact_success_resource_size_and_digest_evidence() -> None:
    manifest, _ = validate_manifest(DEFAULT_MANIFEST)
    case = manifest["cases"][0]
    acceptance = manifest["acceptance_contract"]
    total_memory_bytes = 16 * 1024 * 1024 * 1024
    observation = {
        "orphan_processes": 0,
        "peak_working_set_bytes": total_memory_bytes // 2,
        "elapsed_seconds": case["maximum_p95_elapsed_seconds"],
    }
    payload = {
        "case": {
            "dimensions": {
                "n_complete_cases": 180,
                "v_observed_variables": 9,
                "p_free_parameter_rows": 18,
                "d_optimizer_dimensions": None,
            }
        },
        "scientific_result_sha256": "a" * 64,
        "metrics": {
            "failed_point_refits": 0,
            "analytic_se_unavailable_refits": 0,
            "successful_point_refits": 5000,
            "studentized_usable_refits": 5000,
            "combined_s2_json_bytes": 128 * 1024 * 1024,
        },
    }
    assert (
        payload_acceptance_reasons(
            observation, payload, case, acceptance, total_memory_bytes
        )
        == []
    )

    rejected_observation = dict(observation)
    rejected_observation["orphan_processes"] = 1
    rejected_observation["peak_working_set_bytes"] += 1
    rejected_payload = copy.deepcopy(payload)
    rejected_payload["scientific_result_sha256"] = "not-a-sha"
    rejected_payload["metrics"].update(
        {
            "failed_point_refits": 1,
            "analytic_se_unavailable_refits": 1,
            "successful_point_refits": 4999,
            "studentized_usable_refits": 4998,
            "combined_s2_json_bytes": 256 * 1024 * 1024 + 1,
        }
    )
    reasons = payload_acceptance_reasons(
        rejected_observation,
        rejected_payload,
        case,
        acceptance,
        total_memory_bytes,
    )
    assert reasons == sorted(
        [
            "known_fixture_analytic_se_unavailable",
            "combined_s2_exceeded_128_mib_text_proxy",
            "combined_s2_exceeded_256_mib_project_proxy",
            "known_fixture_point_refit_failure",
            "orphan_process_detected",
            "peak_working_set_exceeded_50_percent_physical_ram",
            "point_success_count_differs_from_requested_replicates",
            "scientific_digest_missing_or_invalid",
            "studentized_usable_count_differs_from_requested_replicates",
        ]
    )


def test_dry_run_executes_no_workloads_and_keeps_product_cap_blocked() -> None:
    completed = subprocess.run(
        [
            sys.executable,
            str(
                ROOT
                / "validation/cbsem_exact_case_bootstrap_studentized_phase1.py"
            ),
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert completed.returncode == 0, completed.stderr
    output = json.loads(completed.stdout)
    assert output["status"] == "dry_run_no_workloads_executed"
    assert output["product_cap_status"] == "blocked_missing_optimizer_dimension"
    assert output["cases"] == [
        "applied_n180_v9_p18_b5000_w12",
        "applied_n180_v9_p18_b10000_w12",
        "cancellation_n180_v9_p18_b10000_w12",
    ]


def test_schemas_reject_any_report_that_claims_product_cap_promotion() -> None:
    manifest_schema = json.loads(MANIFEST_SCHEMA.read_text(encoding="utf-8"))
    report_schema = json.loads(REPORT_SCHEMA.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(manifest_schema)
    Draft202012Validator.check_schema(report_schema)
    Draft202012Validator(manifest_schema).validate(
        json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    )

    report = {
        "schema_version": 1,
        "kind": "cbsem_exact_case_bootstrap_studentized_phase1_report_v1",
        "run_id": "test-run",
        "status": "phase1_applied_evidence_passed_product_cap_blocked_missing_optimizer_dimension",
        "manifest_path": DEFAULT_MANIFEST.relative_to(ROOT).as_posix(),
        "manifest_sha256": "a" * 64,
        "phase0_report_path": "validation/results/phase0-report.json",
        "phase0_report_sha256": "b" * 64,
        "binary_path": "target/release/examples/benchmark.exe",
        "binary_sha256": "c" * 64,
        "physical_memory_bytes": 1,
        "hardware": {},
        "case_receipts": [
            {
                "case_id": f"case_{index}",
                "path": f"validation/results/case_{index}.json",
                "sha256": str(index) * 64,
                "status": "accepted",
                "acceptance_reasons": [],
            }
            for index in range(3)
        ],
        "acceptance_summary": {
            "status": "all_predeclared_cases_accepted",
            "reasons": [],
            "runtime_safety_factor": 2.0,
            "optimizer_dimension_status": "unavailable_product_cap_blocked",
        },
        "cap_decision": {
            "status": "blocked_missing_optimizer_dimension",
            "reason": "D is unavailable.",
        },
    }
    Draft202012Validator(report_schema).validate(report)
    report["cap_decision"]["status"] = "promoted"
    with pytest.raises(ValidationError):
        Draft202012Validator(report_schema).validate(report)
