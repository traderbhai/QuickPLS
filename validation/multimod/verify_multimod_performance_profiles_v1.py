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
    require(document.get("suite_id") == "qpls.multimod.mga.production-qualification.v1", "MGA producer identity differs")
    require(document.get("scale") == "qualification", "MGA performance input is not qualification scale")
    rows = [row for row in document.get("group_matrix", []) if row.get("group_count") == 20]
    require(len(rows) == 1, "MGA output must contain one 20-group cell")
    row = rows[0]
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
        "production_raw_runner_completed": True,
    }


def verify_heterogeneity(document: dict[str, Any]) -> dict[str, Any]:
    require(document.get("suite_id") == "qpls.multimod.heterogeneity.production-qualification.v2", "heterogeneity producer identity differs")
    require(document.get("scale") == "qualification", "heterogeneity performance input is not qualification scale")
    cells = {cell.get("cell_id"): cell for cell in document.get("fixed_k_bootstrap", [])}
    expected = {
        "fimix-p23-fixed-k-bootstrap": "fimix_pls_v2",
        "pos-destination-p23-fixed-k-bootstrap": "pls_pos_destination_scored_interactions_v2",
    }
    receipts = []
    for cell_id, algorithm in expected.items():
        require(cell_id in cells, f"locked maximum interaction cell is missing: {cell_id}")
        cell = cells[cell_id]
        config = cell.get("config", {})
        require(config.get("profile") == "p23_all_current", f"{cell_id} is not the maximum admitted P23 profile")
        phase = config.get("phase", {})
        require(phase.get("kind") == "inference", f"{cell_id} is not locked inference")
        lock = phase.get("lock", {})
        require(lock.get("selected_algorithm") == algorithm and lock.get("selected_k") == 2, f"{cell_id} lock identity differs")
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
        "locked_maximum_profile_cells": receipts,
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
    parser.add_argument("--heterogeneity", type=Path, required=True)
    parser.add_argument("--maximum", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    for source in (args.mga, args.heterogeneity, args.maximum):
        require(source.is_file(), f"performance producer output is missing: {source}")
    report = {
        "schema_version": 1,
        "report_id": "qpls.v256.multimod.performance-output-verification.v1",
        "passed": True,
        "mga": verify_mga(read_json(args.mga)),
        "heterogeneity": verify_heterogeneity(read_json(args.heterogeneity)),
        "maximum": verify_maximum(read_json(args.maximum)),
        "producer_outputs": [
            {"path": str(source.resolve()), "sha256": sha256(source), "size": source.stat().st_size}
            for source in (args.mga, args.heterogeneity, args.maximum)
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
