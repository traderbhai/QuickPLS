#!/usr/bin/env python3
"""Run the immutable Phase-1 applied S2 cap-evidence matrix.

This runner never promotes a product cap. Optimizer dimension D is not exposed
by the authoritative exact-plan API, so even a fully accepted run remains
blocked for product-selector use.
"""

from __future__ import annotations

import argparse
import json
import math
import platform
import re
import tempfile
import uuid
from pathlib import Path
from typing import Any, Mapping

from jsonschema import Draft202012Validator

try:
    from validation.cbsem_exact_case_bootstrap_studentized_benchmark import (
        ROOT,
        command,
        default_binary,
        measured_run,
        sha256,
    )
    from validation.cbsem_exact_case_bootstrap_studentized_phase0 import (
        load_json,
        write_new_json,
    )
    from validation.complexity_performance_measure import detected_total_memory_bytes
    from validation.complexity_performance_v2 import aggregate_runs
except ModuleNotFoundError:
    from cbsem_exact_case_bootstrap_studentized_benchmark import (
        ROOT,
        command,
        default_binary,
        measured_run,
        sha256,
    )
    from cbsem_exact_case_bootstrap_studentized_phase0 import load_json, write_new_json
    from complexity_performance_measure import detected_total_memory_bytes
    from complexity_performance_v2 import aggregate_runs


DEFAULT_MANIFEST = ROOT / "validation/cbsem_exact_case_bootstrap_studentized_phase1_manifest_v1.json"
MANIFEST_SCHEMA = ROOT / "validation/cbsem_exact_case_bootstrap_studentized_phase1_manifest_v1.schema.json"
REPORT_SCHEMA = ROOT / "validation/cbsem_exact_case_bootstrap_studentized_phase1_report_v1.schema.json"
DEFAULT_OUTPUT_ROOT = ROOT / "validation/results/cbsem_exact_case_bootstrap_studentized_phase1"
REPORT_KIND = "cbsem_exact_case_bootstrap_studentized_phase1_report_v1"
CASE_KIND = "cbsem_exact_case_bootstrap_studentized_phase1_case_v1"
SHA256_PATTERN = re.compile(r"^[0-9a-f]{64}$")


def _bound_file(binding: Mapping[str, Any], path_key: str, digest_key: str) -> Path:
    path = (ROOT / str(binding[path_key])).resolve()
    if not path.is_file() or sha256(path) != binding[digest_key]:
        raise ValueError(f"immutable binding failed for {path}")
    return path


def validate_manifest(path: Path) -> tuple[dict[str, Any], str]:
    manifest = load_json(path)
    schema = load_json(MANIFEST_SCHEMA)
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(manifest)

    source = manifest["source_binding"]
    _bound_file(source, "data_path", "data_sha256")
    _bound_file(source, "recipe_path", "recipe_sha256")
    phase0 = manifest["phase0_binding"]
    report_path = _bound_file(phase0, "report_path", "report_sha256")
    baseline_path = _bound_file(phase0, "baseline_case_path", "baseline_case_sha256")
    report = load_json(report_path)
    baseline = load_json(baseline_path)
    if report.get("status") != "phase0_measurement_complete_no_caps_or_qualification":
        raise ValueError("bound Phase-0 report is not the completed non-qualifying authority")
    if baseline.get("status") != phase0["required_status"]:
        raise ValueError("bound Phase-0 baseline did not survive its predeclared matrix")
    dimensions = baseline.get("dimensions", {})
    expected_dimensions = manifest["dimension_contract"]
    if (
        dimensions.get("n_complete_cases") != expected_dimensions["n_complete_cases"]
        or dimensions.get("v_observed_variables") != expected_dimensions["v_observed_variables"]
        or dimensions.get("p_free_parameter_rows") != expected_dimensions["p_free_parameter_rows"]
        or dimensions.get("d_optimizer_dimensions") is not None
    ):
        raise ValueError("Phase-0 baseline dimensions do not reproduce N=180/V=9/P=18/D=null")
    observed_p95 = baseline.get("aggregates", {}).get("p95_elapsed_seconds")
    if not isinstance(observed_p95, (int, float)) or not math.isclose(
        float(observed_p95),
        float(phase0["baseline_p95_elapsed_seconds"]),
        rel_tol=0.0,
        abs_tol=1.0e-12,
    ):
        raise ValueError("Phase-0 p95 runtime differs from the predeclared extrapolation input")

    policy = manifest["acceptance_contract"]["runtime_derivation"]
    if (
        policy["baseline_replicates"] != phase0["baseline_replicates"]
        or policy["baseline_workers"] != phase0["baseline_workers"]
        or not math.isclose(
            float(policy["baseline_p95_elapsed_seconds"]),
            float(phase0["baseline_p95_elapsed_seconds"]),
            rel_tol=0.0,
            abs_tol=1.0e-12,
        )
    ):
        raise ValueError("runtime derivation is not bound to the immutable Phase-0 baseline")
    cases = manifest["cases"]
    expected = [
        ("applied_n180_v9_p18_b5000_w12", 1, "full", 5000, 12, None),
        ("applied_n180_v9_p18_b10000_w12", 2, "full", 10000, 12, None),
        ("cancellation_n180_v9_p18_b10000_w12", 3, "cancellation", 10000, 12, 10),
    ]
    seen: set[str] = set()
    for case, expected_case in zip(cases, expected, strict=True):
        observed = (
            case["case_id"],
            case["order"],
            case["mode"],
            case["replicates"],
            case["workers"],
            case["cancel_after"],
        )
        if observed != expected_case or case["rows"] != 180 or case["factors"] != 3:
            raise ValueError(f"Phase-1 case drifted from its fixed cell: {case['case_id']}")
        if any(dependency not in seen for dependency in case["depends_on"]):
            raise ValueError(f"{case['case_id']} has a forward or unknown dependency")
        seen.add(case["case_id"])
        if case["mode"] == "full":
            derived = (
                float(policy["baseline_p95_elapsed_seconds"])
                * case["replicates"]
                / policy["baseline_replicates"]
                * float(policy["safety_factor"])
            )
            if not math.isclose(
                float(case["maximum_p95_elapsed_seconds"]),
                derived,
                rel_tol=0.0,
                abs_tol=1.0e-12,
            ):
                raise ValueError(f"{case['case_id']} runtime ceiling was retrofitted")
        elif case["maximum_p95_elapsed_seconds"] is not None:
            raise ValueError("cancellation case cannot invent a p95 completion ceiling")
    return manifest, sha256(path)


def full_command(binary: Path, result_path: Path, case: Mapping[str, Any]) -> list[str]:
    return command(
        binary,
        result_path,
        rows=int(case["rows"]),
        factors=int(case["factors"]),
        replicates=int(case["replicates"]),
        workers=int(case["workers"]),
        seed=91,
        cancel_after=case["cancel_after"],
    )


def payload_acceptance_reasons(
    observation: Mapping[str, Any],
    payload: Mapping[str, Any],
    case: Mapping[str, Any],
    acceptance: Mapping[str, Any],
    total_memory_bytes: int,
) -> list[str]:
    reasons: list[str] = []
    if int(observation.get("orphan_processes", 0)) > int(acceptance["maximum_orphan_processes"]):
        reasons.append("orphan_process_detected")
    if total_memory_bytes <= 0:
        reasons.append("physical_memory_unavailable")
    elif int(observation.get("peak_working_set_bytes", 0)) > int(
        total_memory_bytes * float(acceptance["maximum_peak_working_set_fraction_of_physical_ram"])
    ):
        reasons.append("peak_working_set_exceeded_50_percent_physical_ram")
    dimensions = payload.get("case", {}).get("dimensions", {})
    if (
        dimensions.get("n_complete_cases") != 180
        or dimensions.get("v_observed_variables") != 9
        or dimensions.get("p_free_parameter_rows") != 18
        or dimensions.get("d_optimizer_dimensions") is not None
    ):
        reasons.append("execution_dimensions_differ_from_n180_v9_p18_d_null")
    digest = payload.get("scientific_result_sha256")
    if case["mode"] == "full" and (
        not isinstance(digest, str) or SHA256_PATTERN.fullmatch(digest) is None
    ):
        reasons.append("scientific_digest_missing_or_invalid")
    if case["mode"] == "full":
        maximum_runtime = float(case["maximum_p95_elapsed_seconds"])
        if float(observation.get("elapsed_seconds", math.inf)) > maximum_runtime:
            reasons.append("single_execution_exceeded_predeclared_runtime_ceiling")
        metrics = payload.get("metrics", {})
        if metrics.get("failed_point_refits") != acceptance["required_failed_point_refits"]:
            reasons.append("known_fixture_point_refit_failure")
        if (
            metrics.get("analytic_se_unavailable_refits")
            != acceptance["required_analytic_se_unavailable_refits"]
        ):
            reasons.append("known_fixture_analytic_se_unavailable")
        if metrics.get("successful_point_refits") != case["replicates"]:
            reasons.append("point_success_count_differs_from_requested_replicates")
        if metrics.get("studentized_usable_refits") != case["replicates"]:
            reasons.append("studentized_usable_count_differs_from_requested_replicates")
        combined = metrics.get("combined_s2_json_bytes")
        if not isinstance(combined, int) or combined <= 0:
            reasons.append("combined_s2_size_missing")
        else:
            if combined > acceptance["maximum_text_export_proxy_bytes"]:
                reasons.append("combined_s2_exceeded_128_mib_text_proxy")
            if combined > acceptance["maximum_project_document_proxy_bytes"]:
                reasons.append("combined_s2_exceeded_256_mib_project_proxy")
    return sorted(set(reasons))


def _execute(
    argv: list[str], result_path: Path, phase: str, index: int
) -> tuple[dict[str, Any], dict[str, Any]]:
    result_path.unlink(missing_ok=True)
    return measured_run(argv, result_path, phase, index)


def run_full_case(
    binary: Path,
    case: Mapping[str, Any],
    result_path: Path,
    manifest_sha256: str,
    phase0_report_sha256: str,
    acceptance: Mapping[str, Any],
    total_memory_bytes: int,
) -> dict[str, Any]:
    argv = full_command(binary, result_path, case)
    observations: list[dict[str, Any]] = []
    payloads: list[dict[str, Any]] = []
    probe, probe_payload = _execute(argv, result_path, "probe", 0)
    observations.append(probe)
    payloads.append(probe_payload)
    reasons = payload_acceptance_reasons(
        probe, probe_payload, case, acceptance, total_memory_bytes
    )
    warmups: list[dict[str, Any]] = []
    measured: list[dict[str, Any]] = []
    representative = probe_payload
    if not reasons:
        warmup, warmup_payload = _execute(argv, result_path, "warmup", 0)
        warmups.append(warmup)
        observations.append(warmup)
        payloads.append(warmup_payload)
        representative = warmup_payload
        reasons.extend(
            payload_acceptance_reasons(
                warmup, warmup_payload, case, acceptance, total_memory_bytes
            )
        )
    if not reasons:
        for index in range(5):
            observation, payload = _execute(argv, result_path, "measured", index)
            measured.append(observation)
            observations.append(observation)
            payloads.append(payload)
            representative = payload
            run_reasons = payload_acceptance_reasons(
                observation, payload, case, acceptance, total_memory_bytes
            )
            reasons.extend(run_reasons)
            if run_reasons:
                break
    aggregates = aggregate_runs(measured) if len(measured) == 5 else None
    if aggregates is not None and float(aggregates["p95_elapsed_seconds"]) > float(
        case["maximum_p95_elapsed_seconds"]
    ):
        reasons.append("measured_p95_exceeded_predeclared_runtime_ceiling")
    digests = {payload.get("scientific_result_sha256") for payload in payloads}
    if len(payloads) != 7 or None in digests or len(digests) != 1:
        reasons.append("probe_warmup_measured_scientific_digest_drift_or_incomplete_run_set")
    reasons = sorted(set(reasons))
    return {
        "schema_version": 1,
        "kind": CASE_KIND,
        "case": dict(case),
        "manifest_sha256": manifest_sha256,
        "phase0_report_sha256": phase0_report_sha256,
        "status": "accepted" if not reasons else "rejected",
        "acceptance_reasons": reasons,
        "probe": probe,
        "warmup_runs": warmups,
        "measured_runs": measured,
        "aggregates": aggregates,
        "dimensions": representative.get("case", {}).get("dimensions"),
        "scientific_result_sha256": next(iter(digests)) if len(digests) == 1 else None,
        "representative_metrics": representative.get("metrics"),
        "runtime_acceptance": {
            "derivation": acceptance["runtime_derivation"],
            "maximum_p95_elapsed_seconds": case["maximum_p95_elapsed_seconds"],
            "observed_p95_elapsed_seconds": None if aggregates is None else aggregates["p95_elapsed_seconds"],
        },
        "cap_decision": "blocked_missing_optimizer_dimension",
    }


def run_cancellation_case(
    binary: Path,
    case: Mapping[str, Any],
    result_path: Path,
    manifest_sha256: str,
    phase0_report_sha256: str,
    acceptance: Mapping[str, Any],
    total_memory_bytes: int,
) -> dict[str, Any]:
    observation, payload = _execute(full_command(binary, result_path, case), result_path, "probe", 0)
    reasons = payload_acceptance_reasons(
        observation, payload, case, acceptance, total_memory_bytes
    )
    latency = payload.get("cancellation_latency_seconds")
    if payload.get("status") != "cancelled_as_requested":
        reasons.append("typed_cancellation_outcome_missing")
    if not isinstance(latency, (int, float)) or float(latency) > float(
        acceptance["maximum_cancellation_terminal_latency_seconds"]
    ):
        reasons.append("cancellation_terminal_latency_exceeded_one_second")
    reasons = sorted(set(reasons))
    return {
        "schema_version": 1,
        "kind": CASE_KIND,
        "case": dict(case),
        "manifest_sha256": manifest_sha256,
        "phase0_report_sha256": phase0_report_sha256,
        "status": "accepted" if not reasons else "rejected",
        "acceptance_reasons": reasons,
        "probe": observation,
        "warmup_runs": [],
        "measured_runs": [],
        "aggregates": None,
        "dimensions": payload.get("case", {}).get("dimensions"),
        "scientific_result_sha256": None,
        "representative_metrics": None,
        "cancellation": {
            "typed_status": payload.get("status"),
            "terminal_latency_seconds": latency,
        },
        "cap_decision": "blocked_missing_optimizer_dimension",
    }


def blocked_receipt(
    case: Mapping[str, Any], manifest_sha256: str, phase0_report_sha256: str
) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "kind": CASE_KIND,
        "case": dict(case),
        "manifest_sha256": manifest_sha256,
        "phase0_report_sha256": phase0_report_sha256,
        "status": "blocked_by_dependency",
        "acceptance_reasons": ["predeclared_dependency_not_accepted"],
        "probe": None,
        "warmup_runs": [],
        "measured_runs": [],
        "aggregates": None,
        "dimensions": None,
        "scientific_result_sha256": None,
        "representative_metrics": None,
        "cap_decision": "blocked_missing_optimizer_dimension",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--binary", type=Path, default=default_binary())
    parser.add_argument("--output-root", type=Path, default=DEFAULT_OUTPUT_ROOT)
    parser.add_argument("--run-id")
    parser.add_argument("--execute", action="store_true")
    args = parser.parse_args()
    if args.manifest.resolve() != DEFAULT_MANIFEST.resolve():
        parser.error("Phase-1 accepts only the checked-in predeclared manifest")
    manifest, manifest_sha256 = validate_manifest(args.manifest.resolve())
    phase0 = manifest["phase0_binding"]
    plan = [case["case_id"] for case in manifest["cases"]]
    if not args.execute:
        print(
            json.dumps(
                {
                    "status": "dry_run_no_workloads_executed",
                    "manifest_sha256": manifest_sha256,
                    "phase0_report_sha256": phase0["report_sha256"],
                    "cases": plan,
                    "product_cap_status": "blocked_missing_optimizer_dimension",
                    "build_command": "cargo build --release -p qpls-resampling --example cbsem_exact_case_bootstrap_studentized_benchmark",
                },
                indent=2,
            )
        )
        return 0
    binary = args.binary.resolve()
    if not binary.is_file() or not binary.is_relative_to(ROOT.resolve()):
        parser.error("--binary must be an existing repository-local release example")
    run_id = args.run_id or f"phase1-{uuid.uuid4()}"
    if re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", run_id) is None:
        parser.error("--run-id must be one path-safe identifier")
    output_root = args.output_root.resolve()
    if not output_root.is_relative_to((ROOT / "validation/results").resolve()):
        parser.error("--output-root must remain under validation/results")
    run_directory = output_root / run_id
    if run_directory.exists():
        parser.error(f"append-only run directory already exists: {run_directory}")
    run_directory.mkdir(parents=True)
    total_memory_bytes = detected_total_memory_bytes()
    acceptance = manifest["acceptance_contract"]
    receipts: dict[str, dict[str, Any]] = {}
    receipt_rows: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="qpls-phase1-") as raw:
        result_path = Path(raw) / "result.json"
        for case in manifest["cases"]:
            if any(receipts[dependency]["status"] != "accepted" for dependency in case["depends_on"]):
                receipt = blocked_receipt(case, manifest_sha256, phase0["report_sha256"])
            elif case["mode"] == "cancellation":
                receipt = run_cancellation_case(
                    binary,
                    case,
                    result_path,
                    manifest_sha256,
                    phase0["report_sha256"],
                    acceptance,
                    total_memory_bytes,
                )
            else:
                receipt = run_full_case(
                    binary,
                    case,
                    result_path,
                    manifest_sha256,
                    phase0["report_sha256"],
                    acceptance,
                    total_memory_bytes,
                )
            receipts[case["case_id"]] = receipt
            receipt_path = run_directory / "cases" / f"{case['case_id']}.json"
            digest = write_new_json(receipt_path, receipt)
            receipt_rows.append(
                {
                    "case_id": case["case_id"],
                    "path": receipt_path.relative_to(ROOT).as_posix(),
                    "sha256": digest,
                    "status": receipt["status"],
                    "acceptance_reasons": receipt["acceptance_reasons"],
                }
            )
    all_accepted = all(receipt["status"] == "accepted" for receipt in receipts.values())
    summary_reasons = sorted(
        {
            reason
            for receipt in receipts.values()
            for reason in receipt["acceptance_reasons"]
        }
    )
    report = {
        "schema_version": 1,
        "kind": REPORT_KIND,
        "run_id": run_id,
        "status": (
            "phase1_applied_evidence_passed_product_cap_blocked_missing_optimizer_dimension"
            if all_accepted
            else "phase1_applied_evidence_failed_product_cap_blocked"
        ),
        "manifest_path": DEFAULT_MANIFEST.relative_to(ROOT).as_posix(),
        "manifest_sha256": manifest_sha256,
        "phase0_report_path": phase0["report_path"],
        "phase0_report_sha256": phase0["report_sha256"],
        "binary_path": binary.relative_to(ROOT).as_posix(),
        "binary_sha256": sha256(binary),
        "physical_memory_bytes": total_memory_bytes,
        "hardware": {
            "os": platform.system(),
            "os_release": platform.release(),
            "architecture": platform.machine(),
            "processor": platform.processor(),
            "logical_cores": __import__("os").cpu_count(),
        },
        "case_receipts": receipt_rows,
        "acceptance_summary": {
            "status": "all_predeclared_cases_accepted" if all_accepted else "one_or_more_predeclared_cases_not_accepted",
            "reasons": summary_reasons,
            "runtime_safety_factor": acceptance["runtime_derivation"]["safety_factor"],
            "optimizer_dimension_status": "unavailable_product_cap_blocked",
        },
        "cap_decision": {
            "status": "blocked_missing_optimizer_dimension",
            "reason": "Phase-1 cannot establish an applied product cap until optimizer dimension D is exposed by an authoritative exact-plan API.",
        },
    }
    report_schema = load_json(REPORT_SCHEMA)
    Draft202012Validator.check_schema(report_schema)
    Draft202012Validator(report_schema).validate(report)
    write_new_json(run_directory / "phase1-report.json", report)
    print(json.dumps({"status": report["status"], "run_directory": str(run_directory)}, indent=2))
    return 0 if all_accepted else 2


if __name__ == "__main__":
    raise SystemExit(main())
