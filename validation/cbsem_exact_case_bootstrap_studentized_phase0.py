#!/usr/bin/env python3
"""Run the predeclared, non-qualifying Phase-0 exact-CFA S2 benchmark matrix."""

from __future__ import annotations

import argparse
import json
import os
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
    from complexity_performance_measure import detected_total_memory_bytes
    from complexity_performance_v2 import aggregate_runs


DEFAULT_MANIFEST = (
    ROOT
    / "validation/cbsem_exact_case_bootstrap_studentized_phase0_manifest_v1.json"
)
MANIFEST_SCHEMA = (
    ROOT
    / "validation/cbsem_exact_case_bootstrap_studentized_phase0_manifest_v1.schema.json"
)
DEFAULT_OUTPUT_ROOT = (
    ROOT
    / "validation/results/cbsem_exact_case_bootstrap_studentized_phase0"
)
REPORT_KIND = "cbsem_exact_case_bootstrap_studentized_phase0_report_v1"
CASE_KIND = "cbsem_exact_case_bootstrap_studentized_phase0_case_v1"


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain an object")
    return value


def validate_manifest(path: Path) -> tuple[dict[str, Any], str]:
    manifest = load_json(path)
    schema = load_json(MANIFEST_SCHEMA)
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(manifest)
    binding = manifest["source_binding"]
    for path_key, digest_key in (
        ("data_path", "data_sha256"),
        ("recipe_path", "recipe_sha256"),
    ):
        source = ROOT / binding[path_key]
        if not source.is_file() or sha256(source) != binding[digest_key]:
            raise ValueError(f"manifest source binding failed for {source}")
    ids = [case["case_id"] for case in manifest["cases"]]
    orders = [case["order"] for case in manifest["cases"]]
    if len(ids) != len(set(ids)) or orders != list(range(1, len(ids) + 1)):
        raise ValueError("manifest case ids/orders are not unique and contiguous")
    seen: set[str] = set()
    for case in manifest["cases"]:
        if any(dependency not in seen for dependency in case["depends_on"]):
            raise ValueError(f"{case['case_id']} has a forward or unknown dependency")
        if (case["mode"] == "cancellation") != (case["cancel_after"] is not None):
            raise ValueError(f"{case['case_id']} cancellation fields disagree")
        seen.add(case["case_id"])
    if not set(manifest["worker_invariance_group"]) <= set(ids):
        raise ValueError("worker invariance group names an unknown case")
    return manifest, sha256(path)


def write_new_json(path: Path, value: Mapping[str, Any]) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n"
    with path.open("x", encoding="utf-8") as handle:
        handle.write(payload)
    return sha256(path)


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


def resource_stop_reasons(
    observation: Mapping[str, Any],
    payload: Mapping[str, Any],
    *,
    total_memory_bytes: int,
    criteria: Mapping[str, Any],
) -> list[str]:
    reasons: list[str] = []
    if int(observation["orphan_processes"]) > int(criteria["maximum_orphan_processes"]):
        reasons.append("orphan_process_detected")
    if total_memory_bytes <= 0:
        reasons.append("physical_memory_unavailable_for_50_percent_stop")
    elif int(observation["peak_working_set_bytes"]) >= int(
        total_memory_bytes
        * float(criteria["maximum_peak_working_set_fraction_of_physical_ram"])
    ):
        reasons.append("peak_working_set_exceeded_50_percent_physical_ram")
    metrics = payload.get("metrics")
    if isinstance(metrics, Mapping):
        combined = metrics.get("combined_s2_json_bytes")
        if isinstance(combined, int):
            if combined > int(criteria["maximum_text_export_proxy_bytes"]):
                reasons.append("combined_s2_bytes_exceeded_128_mib_text_proxy")
            if combined > int(criteria["maximum_project_document_proxy_bytes"]):
                reasons.append("combined_s2_bytes_exceeded_256_mib_project_proxy")
    return reasons


def worker_digest_stop_reasons(expected: str | None, actual: Any) -> list[str]:
    if expected is not None and actual != expected:
        return ["worker_scientific_digest_drift"]
    return []


def run_case(
    binary: Path,
    case: Mapping[str, Any],
    run_directory: Path,
    *,
    manifest_sha256: str,
    total_memory_bytes: int,
    criteria: Mapping[str, Any],
    expected_worker_scientific_sha256: str | None,
) -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix=f"qpls-phase0-{case['case_id']}-") as raw:
        result_path = Path(raw) / "result.json"
        argv = full_command(binary, result_path, case)
        if case["mode"] == "cancellation":
            observation, payload = measured_run(argv, result_path, "measured", 0)
            reasons = resource_stop_reasons(
                observation,
                payload,
                total_memory_bytes=total_memory_bytes,
                criteria=criteria,
            )
            latency = payload.get("cancellation_latency_seconds")
            if payload.get("status") != "cancelled_as_requested":
                reasons.append("typed_cancellation_outcome_missing")
            if not isinstance(latency, (int, float)) or latency > float(
                criteria["maximum_cancellation_terminal_latency_seconds"]
            ):
                reasons.append("cancellation_terminal_latency_exceeded_one_second")
            receipt = {
                "schema_version": 1,
                "kind": CASE_KIND,
                "case": dict(case),
                "manifest_sha256": manifest_sha256,
                "status": "cancellation_observed" if not reasons else "stop_triggered",
                "stop_reasons": reasons,
                "probe": observation,
                "dimensions": payload.get("case", {}).get("dimensions"),
                "cancellation": {
                    "typed_status": payload.get("status"),
                    "terminal_latency_seconds": latency,
                },
                "warmup_runs": [],
                "measured_runs": [],
                "aggregates": None,
                "scientific_result_sha256": None,
                "representative_metrics": None,
                "cap_decision": "not_permitted_from_phase0",
            }
            write_new_json(run_directory / "cases" / f"{case['case_id']}.json", receipt)
            return receipt

        probe, probe_payload = measured_run(argv, result_path, "measured", 0)
        reasons = resource_stop_reasons(
            probe,
            probe_payload,
            total_memory_bytes=total_memory_bytes,
            criteria=criteria,
        )
        reasons.extend(
            worker_digest_stop_reasons(
                expected_worker_scientific_sha256,
                probe_payload.get("scientific_result_sha256"),
            )
        )
        if reasons:
            receipt = {
                "schema_version": 1,
                "kind": CASE_KIND,
                "case": dict(case),
                "manifest_sha256": manifest_sha256,
                "status": "stopped_after_probe",
                "stop_reasons": reasons,
                "probe": probe,
                "dimensions": probe_payload.get("case", {}).get("dimensions"),
                "warmup_runs": [],
                "measured_runs": [],
                "aggregates": None,
                "scientific_result_sha256": probe_payload.get("scientific_result_sha256"),
                "representative_metrics": probe_payload.get("metrics"),
                "cap_decision": "not_permitted_from_phase0",
            }
            write_new_json(run_directory / "cases" / f"{case['case_id']}.json", receipt)
            return receipt

        result_path.unlink(missing_ok=True)
        warmup, warmup_payload = measured_run(argv, result_path, "warmup", 0)
        reasons.extend(
            resource_stop_reasons(
                warmup,
                warmup_payload,
                total_memory_bytes=total_memory_bytes,
                criteria=criteria,
            )
        )
        measured: list[dict[str, Any]] = []
        representative = warmup_payload
        if not reasons:
            for index in range(5):
                observation, payload = measured_run(argv, result_path, "measured", index)
                measured.append(observation)
                representative = payload
                run_reasons = resource_stop_reasons(
                    observation,
                    payload,
                    total_memory_bytes=total_memory_bytes,
                    criteria=criteria,
                )
                reasons.extend(run_reasons)
                if run_reasons:
                    break
        digests = {row.get("scientific_result_sha256") for row in measured}
        if len(measured) == 5 and (None in digests or len(digests) != 1):
            reasons.append("repeated_run_scientific_digest_drift")
        reasons = sorted(set(reasons))
        receipt = {
            "schema_version": 1,
            "kind": CASE_KIND,
            "case": dict(case),
            "manifest_sha256": manifest_sha256,
            "status": "survivor_measurement_recorded" if not reasons and len(measured) == 5 else "stop_triggered_after_probe_survival",
            "stop_reasons": reasons,
            "probe": probe,
            "dimensions": representative.get("case", {}).get("dimensions"),
            "warmup_runs": [warmup],
            "measured_runs": measured,
            "aggregates": aggregate_runs(measured) if len(measured) == 5 else None,
            "scientific_result_sha256": representative.get("scientific_result_sha256"),
            "representative_metrics": representative.get("metrics"),
            "cap_decision": "not_permitted_from_phase0",
        }
        write_new_json(run_directory / "cases" / f"{case['case_id']}.json", receipt)
        return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--binary", type=Path, default=default_binary())
    parser.add_argument("--output-root", type=Path, default=DEFAULT_OUTPUT_ROOT)
    parser.add_argument("--run-id")
    parser.add_argument("--execute", action="store_true")
    args = parser.parse_args()

    if args.manifest.resolve() != DEFAULT_MANIFEST.resolve():
        parser.error("Phase-0 accepts only the checked-in predeclared manifest")
    manifest, manifest_sha256 = validate_manifest(args.manifest.resolve())
    binary = args.binary.resolve()
    plan = [case["case_id"] for case in manifest["cases"]]
    if not args.execute:
        print(json.dumps({
            "status": "dry_run_no_workloads_executed",
            "manifest_sha256": manifest_sha256,
            "cases": plan,
            "build_command": "cargo build --release -p qpls-resampling --example cbsem_exact_case_bootstrap_studentized_benchmark",
        }, indent=2))
        return 0
    if not binary.is_file():
        parser.error(f"release example is missing: {binary}")

    run_id = args.run_id or f"phase0-{uuid.uuid4()}"
    if re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", run_id) is None:
        parser.error("--run-id must be one path-safe identifier")
    output_root = args.output_root.resolve()
    allowed_output_root = (ROOT / "validation/results").resolve()
    if not output_root.is_relative_to(allowed_output_root):
        parser.error("--output-root must remain under validation/results")
    if not binary.is_relative_to(ROOT.resolve()):
        parser.error("--binary must remain inside the repository")
    run_directory = output_root / run_id
    if run_directory.exists():
        parser.error(f"append-only run directory already exists: {run_directory}")
    run_directory.mkdir(parents=True)
    total_memory_bytes = detected_total_memory_bytes()
    receipts: dict[str, dict[str, Any]] = {}
    references: list[dict[str, Any]] = []
    for case in manifest["cases"]:
        unmet = [
            dependency
            for dependency in case["depends_on"]
            if receipts.get(dependency, {}).get("status")
            != "survivor_measurement_recorded"
        ]
        if unmet:
            receipt = {
                "schema_version": 1,
                "kind": CASE_KIND,
                "case": dict(case),
                "manifest_sha256": manifest_sha256,
                "status": "skipped_due_dependency_stop",
                "stop_reasons": [f"dependency_not_survivor:{value}" for value in unmet],
                "probe": None,
                "dimensions": None,
                "warmup_runs": [],
                "measured_runs": [],
                "aggregates": None,
                "scientific_result_sha256": None,
                "representative_metrics": None,
                "cap_decision": "not_permitted_from_phase0",
            }
            path = run_directory / "cases" / f"{case['case_id']}.json"
            digest = write_new_json(path, receipt)
        else:
            path = run_directory / "cases" / f"{case['case_id']}.json"
            worker_group = manifest["worker_invariance_group"]
            expected_worker_digest = None
            if case["case_id"] in worker_group[1:]:
                expected_worker_digest = receipts.get(worker_group[0], {}).get(
                    "scientific_result_sha256"
                )
            try:
                receipt = run_case(
                    binary,
                    case,
                    run_directory,
                    manifest_sha256=manifest_sha256,
                    total_memory_bytes=total_memory_bytes,
                    criteria=manifest["stop_criteria"],
                    expected_worker_scientific_sha256=expected_worker_digest,
                )
                digest = sha256(path)
            except Exception as error:
                receipt = {
                    "schema_version": 1,
                    "kind": CASE_KIND,
                    "case": dict(case),
                    "manifest_sha256": manifest_sha256,
                    "status": "stop_triggered_by_child_error",
                    "stop_reasons": [f"child_execution_error:{type(error).__name__}:{error}"],
                    "probe": None,
                    "dimensions": None,
                    "warmup_runs": [],
                    "measured_runs": [],
                    "aggregates": None,
                    "scientific_result_sha256": None,
                    "representative_metrics": None,
                    "cap_decision": "not_permitted_from_phase0",
                }
                digest = write_new_json(path, receipt)
        receipts[case["case_id"]] = receipt
        references.append({
            "case_id": case["case_id"],
            "status": receipt["status"],
            "path": str(path.relative_to(ROOT)).replace("\\", "/"),
            "sha256": digest,
        })

    worker_rows = [receipts[value] for value in manifest["worker_invariance_group"]]
    worker_digests = {
        row["scientific_result_sha256"]
        for row in worker_rows
        if row["status"] == "survivor_measurement_recorded"
    }
    worker_complete = all(
        row["status"] == "survivor_measurement_recorded" for row in worker_rows
    )
    worker_invariance = {
        "status": (
            "exact"
            if worker_complete and len(worker_digests) == 1
            else "drift"
            if worker_complete
            else "not_evaluated_due_stopped_or_skipped_case"
        ),
        "case_ids": manifest["worker_invariance_group"],
        "scientific_result_sha256": next(iter(worker_digests))
        if worker_complete and len(worker_digests) == 1
        else None,
    }
    stop_reasons = sorted(
        {
            reason
            for receipt in receipts.values()
            for reason in receipt.get("stop_reasons", [])
        }
    )
    if worker_invariance["status"] == "drift":
        stop_reasons.append("worker_scientific_digest_drift")
    report = {
        "schema_version": 1,
        "kind": REPORT_KIND,
        "status": "phase0_measurement_complete_no_caps_or_qualification",
        "run_id": run_id,
        "manifest_path": str(args.manifest.resolve().relative_to(ROOT)).replace("\\", "/"),
        "manifest_sha256": manifest_sha256,
        "report_schema_path": manifest["source_binding"]["phase0_report_schema"],
        "binary_path": str(binary.relative_to(ROOT)).replace("\\", "/"),
        "binary_sha256": sha256(binary),
        "rust_example_sha256": sha256(ROOT / manifest["source_binding"]["rust_example"]),
        "hardware": {
            "os": platform.system(),
            "os_release": platform.release(),
            "architecture": platform.machine(),
            "processor": platform.processor() or "unknown",
            "logical_cores": os.cpu_count(),
            "physical_memory_bytes": total_memory_bytes or None,
        },
        "physical_memory_bytes": total_memory_bytes or None,
        "stop_criteria": manifest["stop_criteria"],
        "case_receipts": references,
        "worker_invariance": worker_invariance,
        "stop_evaluation": {
            "status": "stop_triggered" if stop_reasons else "no_stop_triggered",
            "reasons": stop_reasons,
        },
        "cap_decision": {
            "status": "not_evaluated",
            "reason": "Phase-0 records bounded source-bound measurements and stop events only; it does not choose caps or qualify the product selector."
        },
    }
    report_path = run_directory / "phase0-report.json"
    report_schema = load_json(ROOT / manifest["source_binding"]["phase0_report_schema"])
    Draft202012Validator.check_schema(report_schema)
    Draft202012Validator(report_schema).validate(report)
    write_new_json(report_path, report)
    print(f"wrote append-only Phase-0 evidence {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
