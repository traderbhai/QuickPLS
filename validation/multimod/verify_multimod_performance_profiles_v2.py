#!/usr/bin/env python3
"""Fail-closed structural verifier for the real maximum-profile producer outputs."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def verify_mga(document: dict[str, Any]) -> dict[str, Any]:
    require(
        document.get("suite_id") == "qpls.multimod.mga.qualification-cell-result.v1",
        "MGA performance input is not an exact cell envelope",
    )
    require(document.get("cell_id") == "mga-general-20-groups", "MGA cell identity differs")
    require(document.get("payload_kind") == "group_matrix_cell", "MGA payload kind differs")
    require(document.get("scale") == "qualification" and document.get("seed") == 42, "MGA performance input is not the frozen qualification cell")
    require(document.get("permutation_samples") == 5000, "MGA permutation count differs")
    require(document.get("bootstrap_samples") == 5000, "MGA bootstrap count differs")
    require(document.get("metamorphism") == "baseline" and document.get("workers") == 1, "MGA baseline execution identity differs")
    require(len(document.get("cache_bindings", [])) == 1, "MGA cell omitted its completed production-cache binding")
    row = document.get("payload", {})
    require(row.get("expected_pair_count") == 190 and row.get("heavy_run_confirmed") is True, "20-group all-pairs contract is absent")
    analysis = row.get("analysis", {})
    pairwise = analysis.get("pairwise", [])
    pairs = {
        (entry.get("left_group_id"), entry.get("right_group_id"))
        for entry in pairwise
        if str(entry.get("procedure", "")).startswith("pairwise_permutation_")
    }
    require(len(pairs) == 190, f"MGA production result contains {len(pairs)} rather than 190 pairwise permutation pairs")
    require(bool(analysis.get("omnibus")), "20-group MGA production result omitted omnibus inference")
    plan = row.get("execution_plan", {})
    require(plan.get("retry_policy") == "none", "MGA execution plan retry policy changed")
    require(len(plan.get("shards", [])) > 190, "MGA execution plan does not contain the complete 20-group task graph")
    return {
        "workload_id": "qpls.v256.multimod.performance.mga-20-groups-190-pairs.v1",
        "group_count": 20,
        "pair_count": len(pairs),
        "execution_shard_count": len(plan["shards"]),
        "execution_plan_sha256": plan.get("plan_sha256"),
        "dataset_fingerprint": plan.get("dataset_fingerprint"),
        "analysis_identity_sha256": plan.get("analysis_identity_sha256"),
        "qualification_plan_sha256": document.get("qualification_plan_sha256"),
        "executable_sha256": document.get("executable_sha256"),
        "cache_binding_count": len(document["cache_bindings"]),
        "production_raw_runner_completed": True,
    }


def verify_heterogeneity(documents: dict[str, dict[str, Any]]) -> dict[str, Any]:
    expected_envelopes = {
        "sentinel": ("sentinel", []),
        "pos-destination-p23-discovery": ("pos_destination_p23_discovery", ["sentinel"]),
        "bootstrap-fimix-p23": ("bootstrap", ["pos-destination-p23-discovery", "sentinel"]),
        "bootstrap-pos-destination-p23": ("bootstrap", ["pos-destination-p23-discovery", "sentinel"]),
    }
    for shard_id, (payload_kind, dependencies) in expected_envelopes.items():
        document = documents[shard_id]
        require(
            document.get("suite_id") == "qpls.multimod.heterogeneity.qualification-shard.v1",
            f"{shard_id} is not an exact heterogeneity shard envelope",
        )
        require(document.get("shard_id") == shard_id, f"heterogeneity shard identity differs: {shard_id}")
        require(document.get("scale") == "qualification" and document.get("campaign_seed") == 42, f"{shard_id} is not the frozen qualification shard")
        require(document.get("metamorphism") == "baseline" and document.get("workers") == 1, f"{shard_id} baseline execution identity differs")
        require(document.get("dependency_shard_ids") == dependencies, f"{shard_id} dependency envelope differs")
        require(document.get("payload", {}).get("kind") == payload_kind, f"{shard_id} payload kind differs")

    require(
        documents["sentinel"]["payload"]["value"].get("claim")
        == "fail_fast_pipeline_sentinel_not_scientific_acceptance_evidence",
        "heterogeneity sentinel was promoted or relabelled as scientific evidence",
    )

    discovery = documents["pos-destination-p23-discovery"]["payload"]["value"]
    discovery_identity = discovery.get("analysis", {}).get("discovery_result_identity_sha256")
    require(isinstance(discovery_identity, str) and len(discovery_identity) == 64, "P23 discovery identity is absent")
    expected = {
        "bootstrap-fimix-p23": ("fimix-p23-fixed-k-bootstrap", "fimix_pls_v2"),
        "bootstrap-pos-destination-p23": (
            "pos-destination-p23-fixed-k-bootstrap",
            "pls_pos_destination_scored_interactions_v2",
        ),
    }
    receipts = []
    for shard_id, (cell_id, algorithm) in expected.items():
        cell = documents[shard_id]["payload"]["value"]
        require(cell.get("cell_id") == cell_id, f"{shard_id} cell identity differs")
        config = cell.get("config", {})
        require(config.get("profile") == "p23_all_current", f"{cell_id} is not the maximum admitted P23 profile")
        phase = config.get("phase", {})
        require(phase.get("kind") == "inference", f"{cell_id} is not locked inference")
        lock = phase.get("lock", {})
        require(lock.get("selected_algorithm") == algorithm and lock.get("selected_k") == 2, f"{cell_id} lock identity differs")
        require(lock.get("discovery_result_identity_sha256") == discovery_identity, f"{cell_id} does not bind the selected P23 discovery")
        bootstrap = config.get("bootstrap", {})
        require(bootstrap.get("resamples") == 500, f"{cell_id} did not execute the minimum qualified fixed-K bootstrap")
        evidence = cell.get("evidence", {}).get("bootstrap", [])
        require(len(evidence) == 1, f"{cell_id} did not retain one shared production bootstrap ledger")
        analysis = cell.get("analysis", {})
        require(analysis.get("bootstrap_ledger", {}).get("requested") == 500, f"{cell_id} result ledger differs from config")
        compiler = cell.get("compiler_receipt", {})
        for digest_field in ("dataset_fingerprint", "recipe_analytical_sha256", "model_scientific_sha256"):
            require(isinstance(compiler.get(digest_field), str) and len(compiler[digest_field]) == 64, f"{cell_id} lacks {digest_field}")
        receipts.append({
            "shard_id": shard_id,
            "cell_id": cell_id,
            "algorithm": algorithm,
            "profile": "p23_all_current",
            "selected_k": 2,
            "bootstrap_resamples": 500,
            "dataset_fingerprint": compiler.get("dataset_fingerprint"),
            "recipe_analytical_sha256": compiler.get("recipe_analytical_sha256"),
            "model_scientific_sha256": compiler.get("model_scientific_sha256"),
            "production_raw_runner_completed": True,
        })
    return {
        "workload_id": "qpls.v256.multimod.performance.locked-heterogeneity-p23.v1",
        "dependency_shard_ids": list(expected_envelopes),
        "discovery_result_identity_sha256": discovery_identity,
        "locked_maximum_profile_cells": receipts,
    }


def verify_execution_topology(document: dict[str, Any]) -> dict[str, Any]:
    require(
        document.get("schema_version") == 2
        and document.get("suite_id") == "qpls.v256.multimod.performance-execution-topology.v2",
        "performance execution topology identity differs",
    )
    require(document.get("passed") is True and document.get("seed") == 42, "performance topology is not the frozen passed run")
    require(document.get("one_release_cargo_build") is True, "performance topology did not retain the one-build contract")
    require(document.get("direct_binary_execution_after_build") is True, "performance topology did not invoke release binaries directly")
    require(document.get("internal_timeout_seconds") == 6480, "performance topology internal cap differs")
    release_build = document.get("release_build", {})
    require(
        release_build.get("attempt_count") == 1
        and isinstance(release_build.get("attempt_id"), str)
        and bool(release_build["attempt_id"])
        and isinstance(release_build.get("attempt_sha256"), str)
        and len(release_build["attempt_sha256"]) == 64,
        "release-build attempt identity or count differs",
    )
    stages = {row.get("stage_id"): row for row in document.get("stages", [])}
    expected_process_counts = {
        "release-build": 1,
        "mga-general-20-groups": 1,
        "heterogeneity-sentinel": 1,
        "heterogeneity-p23-discovery": 1,
        "heterogeneity-p23-bootstrap-pair": 2,
        "conditional-sidecar-resume": 1,
    }
    require(set(stages) == set(expected_process_counts), "performance stage inventory differs")
    for stage_id, process_count in expected_process_counts.items():
        measurement = stages[stage_id].get("measurement", {})
        require(measurement.get("process_count") == process_count, f"{stage_id} process topology differs")
        require(measurement.get("atomic_timeout_seconds", 1801) <= 1800, f"{stage_id} atomic timeout exceeds 1,800 seconds")
        require(measurement.get("working_set_sample_count", 0) > 0, f"{stage_id} has no process-tree samples")
        require(measurement.get("peak_working_set_bytes", 0) > 0, f"{stage_id} has no process-tree peak")
        process_peaks = measurement.get("per_process_peak_working_set_bytes", [])
        sampled_peak = measurement.get("sampled_concurrent_peak_working_set_bytes")
        require(
            isinstance(process_peaks, list)
            and len(process_peaks) == process_count
            and all(entry.get("peak_working_set_bytes", 0) > 0 for entry in process_peaks)
            and isinstance(sampled_peak, int)
            and sampled_peak >= 0
            and measurement.get("peak_working_set_bytes")
            == max(
                sampled_peak,
                sum(entry["peak_working_set_bytes"] for entry in process_peaks),
            ),
            f"{stage_id} conservative combined process-tree peak differs",
        )
        require(isinstance(stages[stage_id].get("receipt_sha256"), str) and len(stages[stage_id]["receipt_sha256"]) == 64, f"{stage_id} receipt identity is absent")
    bootstrap = stages["heterogeneity-p23-bootstrap-pair"]["measurement"]
    require(len(bootstrap.get("command_identities", [])) == 2, "P23 bootstrap branches were not measured as one concurrent process group")
    workloads = {row.get("workload_id"): row for row in document.get("workloads", [])}
    require(set(workloads) == {"mga_20_groups_190_pairs", "heterogeneity_locked_p23", "conditional_sidecar_resume"}, "performance workload inventory differs")
    require(workloads["heterogeneity_locked_p23"].get("atomic_process_count") == 4, "heterogeneity workload did not retain the exact four-process topology")
    for workload_id, workload in workloads.items():
        require(workload.get("output_size_bytes", 0) > 0, f"{workload_id} has no bounded scientific output size")
    return {
        "topology_plan_sha256": document.get("topology_plan_sha256"),
        "stage_count": len(stages),
        "release_build": release_build,
        "heterogeneity_atomic_process_count": workloads["heterogeneity_locked_p23"]["atomic_process_count"],
        "heterogeneity_combined_peak_working_set_bytes": workloads["heterogeneity_locked_p23"].get("peak_working_set_bytes"),
    }


def verify_maximum(document: dict[str, Any]) -> dict[str, Any]:
    require(document.get("suite_id") == "qpls.v256.multimod.maximum-profiles-production-performance.v1", "maximum-profile producer identity differs")
    conditional = document.get("conditional_maximum", {})
    require(conditional.get("path_count") == 8, "conditional maximum path count differs")
    require(conditional.get("moderator_count") == 4, "conditional maximum moderator count differs")
    require(conditional.get("cartesian_probe_tuple_count") == 64, "conditional maximum probe grid differs")
    require(conditional.get("conditional_cell_count") == 512, "conditional maximum cell count differs")
    require(conditional.get("inferential_target_count") == 1024, "conditional maximum target count differs")
    require(conditional.get("requested_resamples") == 500 and conditional.get("production_raw_runner_completed") is True, "conditional maximum did not execute its production resampling path")
    resume = document.get("mga_cancellation_resume", {})
    for field in ("cancelled_result_suppressed", "cache_strict_reopen_validated", "resumed_equals_uninterrupted"):
        require(resume.get(field) is True, f"MGA cancellation/resume lacks {field}")
    require(resume.get("cancelled_completed_shard_count", 0) > 0, "MGA cancellation retained no complete shard")
    sidecar = document.get("archive_sidecar_boundaries", {})
    require(sidecar.get("warning_bytes") == 128 * 1024 * 1024, "sidecar warning boundary differs")
    require(sidecar.get("maximum_bytes") == 512 * 1024 * 1024, "sidecar cap differs")
    require(sidecar.get("cost_states") == {
        "warning_exact": "within_limit",
        "warning_plus_one": "warning",
        "maximum_exact": "warning",
        "maximum_plus_one": "blocked",
    }, "sidecar cost-state boundary matrix differs")
    require(sidecar.get("aggregate_maximum_admitted_bytes") == 512 * 1024 * 1024, "archive aggregate maximum was not admitted")
    require(sidecar.get("aggregate_maximum_plus_one_rejected") is True and sidecar.get("production_archive_validator_executed") is True, "archive maximum-plus-one did not fail closed")
    micom = sidecar.get("micom_null_statistics", {})
    require(micom.get("representation") == "construct_ordinal_plus_statistic_kind_v1", "MICOM compact Arrow representation differs")
    require(micom.get("rows") == 25_000, "MICOM compact Arrow boundary row count differs")
    actual = micom.get("actual_uncompressed_bytes")
    predicted = micom.get("predicted_uncompressed_bytes")
    require(
        isinstance(actual, int)
        and isinstance(predicted, int)
        and 0 < actual <= predicted
        and micom.get("prediction_bounds_actual") is True,
        "MICOM compact Arrow preflight did not conservatively bound actual trusted bytes",
    )
    return {
        "conditional_maximum": conditional,
        "mga_cancellation_resume": resume,
        "archive_sidecar_boundaries": sidecar,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mga", type=Path, required=True)
    parser.add_argument("--heterogeneity-sentinel", type=Path, required=True)
    parser.add_argument("--heterogeneity-discovery", type=Path, required=True)
    parser.add_argument("--heterogeneity-fimix", type=Path, required=True)
    parser.add_argument("--heterogeneity-pos", type=Path, required=True)
    parser.add_argument("--maximum", type=Path, required=True)
    parser.add_argument("--execution-topology", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    heterogeneity_paths = {
        "sentinel": args.heterogeneity_sentinel,
        "pos-destination-p23-discovery": args.heterogeneity_discovery,
        "bootstrap-fimix-p23": args.heterogeneity_fimix,
        "bootstrap-pos-destination-p23": args.heterogeneity_pos,
    }
    sources = (
        args.mga,
        *heterogeneity_paths.values(),
        args.maximum,
        args.execution_topology,
    )
    for source in sources:
        require(source.is_file(), f"performance producer output is missing: {source}")
    report = {
        "schema_version": 2,
        "report_id": "qpls.v256.multimod.performance-output-verification.v2",
        "passed": True,
        "mga": verify_mga(read_json(args.mga)),
        "heterogeneity": verify_heterogeneity(
            {shard_id: read_json(path) for shard_id, path in heterogeneity_paths.items()}
        ),
        "maximum": verify_maximum(read_json(args.maximum)),
        "execution_topology": verify_execution_topology(read_json(args.execution_topology)),
        "producer_outputs": [
            {"path": str(source.resolve()), "sha256": sha256(source), "size": source.stat().st_size}
            for source in sources
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.output.with_suffix(args.output.suffix + ".tmp")
    temporary.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(args.output)
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

