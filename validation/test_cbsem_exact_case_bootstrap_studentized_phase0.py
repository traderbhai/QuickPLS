from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

from jsonschema import Draft202012Validator

from validation.cbsem_exact_case_bootstrap_studentized_phase0 import (
    DEFAULT_MANIFEST,
    MANIFEST_SCHEMA,
    resource_stop_reasons,
    validate_manifest,
    worker_digest_stop_reasons,
)


ROOT = Path(__file__).resolve().parents[1]


def test_manifest_is_predeclared_source_bound_and_covers_required_axes() -> None:
    manifest, digest = validate_manifest(DEFAULT_MANIFEST)
    assert len(digest) == 64
    assert [case["order"] for case in manifest["cases"]] == list(range(1, 10))
    by_id = {case["case_id"]: case for case in manifest["cases"]}
    assert by_id["compact_b500_w1"]["replicates"] == 500
    assert [by_id[case]["workers"] for case in manifest["worker_invariance_group"]] == [1, 4, 12]
    assert by_id["result_size_n180_f3_b2000_w4"]["replicates"] == 2000
    assert by_id["availability_floor_n180_f3_b1000_w4"]["depends_on"] == [
        "availability_floor_n180_f3_b1000_w1"
    ]
    assert by_id["availability_floor_n180_f3_b1000_w12"]["depends_on"] == [
        "availability_floor_n180_f3_b1000_w4"
    ]
    assert by_id["result_size_n180_f3_b2000_w4"]["depends_on"] == [
        "availability_floor_n180_f3_b1000_w4"
    ]
    assert manifest["dimension_contract"]["D"].startswith("null_until")


def test_manifest_schema_and_existing_standalone_evidence_remain_readable() -> None:
    manifest_schema = json.loads(MANIFEST_SCHEMA.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(manifest_schema)
    Draft202012Validator(manifest_schema).validate(
        json.loads(DEFAULT_MANIFEST.read_text(encoding="utf-8"))
    )
    standalone_schema = json.loads(
        (ROOT / "validation/cbsem_exact_case_bootstrap_studentized_benchmark_v1.schema.json")
        .read_text(encoding="utf-8")
    )
    existing = ROOT / "validation/results/cbsem_exact_case_bootstrap_studentized_benchmark_run_001.json"
    if existing.is_file():
        Draft202012Validator(standalone_schema).validate(
            json.loads(existing.read_text(encoding="utf-8"))
        )


def test_resource_and_worker_stop_rules_are_fail_closed() -> None:
    criteria = {
        "maximum_orphan_processes": 0,
        "maximum_peak_working_set_fraction_of_physical_ram": 0.5,
        "maximum_text_export_proxy_bytes": 128 * 1024 * 1024,
        "maximum_project_document_proxy_bytes": 256 * 1024 * 1024,
    }
    observation = {"orphan_processes": 1, "peak_working_set_bytes": 500}
    payload = {"metrics": {"combined_s2_json_bytes": 256 * 1024 * 1024 + 1}}
    reasons = resource_stop_reasons(
        observation, payload, total_memory_bytes=1000, criteria=criteria
    )
    assert "orphan_process_detected" in reasons
    assert "peak_working_set_exceeded_50_percent_physical_ram" in reasons
    assert "combined_s2_bytes_exceeded_128_mib_text_proxy" in reasons
    assert "combined_s2_bytes_exceeded_256_mib_project_proxy" in reasons
    assert worker_digest_stop_reasons("a" * 64, "b" * 64) == [
        "worker_scientific_digest_drift"
    ]
    assert worker_digest_stop_reasons("a" * 64, "a" * 64) == []


def test_dry_run_executes_no_workloads_and_scheduler_workers_do_not_change_recipe() -> None:
    completed = subprocess.run(
        [sys.executable, str(ROOT / "validation/cbsem_exact_case_bootstrap_studentized_phase0.py")],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert completed.returncode == 0, completed.stderr
    assert '"status": "dry_run_no_workloads_executed"' in completed.stdout
    source = (
        ROOT
        / "crates/qpls-resampling/examples/cbsem_exact_case_bootstrap_studentized_benchmark.rs"
    ).read_text(encoding="utf-8")
    assert "settings.workers = 1;" in source
    assert "settings.workers = args.workers" not in source
    assert '"v_observed_variables"' in source
    assert '"d_optimizer_dimensions": Value::Null' in source
