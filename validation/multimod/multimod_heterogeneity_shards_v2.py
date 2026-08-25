#!/usr/bin/env python3
"""Seal, verify, and deterministically aggregate heterogeneity V2 shards.

The Rust producer remains the scientific authority. This helper owns only the
checkpoint transport: exact plan validation, atomic SHA-256 receipts, resume
validation, and lossless reconstruction of the historical monolithic raw
receipt consumed by the independent comparator.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import secrets
import tempfile
from pathlib import Path
from typing import Any


PLAN_SUITE_ID = "qpls.multimod.heterogeneity.qualification-shard-plan.v1"
SHARD_SUITE_ID = "qpls.multimod.heterogeneity.qualification-shard.v1"
RECEIPT_SUITE_ID = "qpls.multimod.heterogeneity.qualification-shard-receipt.v1"
BOOTSTRAP_PREPARED_SUITE_ID = (
    "qpls.multimod.heterogeneity.bootstrap-prepared-execution.v1"
)
BOOTSTRAP_PREPARED_RECEIPT_SUITE_ID = (
    "qpls.multimod.heterogeneity.bootstrap-prepared-execution-receipt.v1"
)
BOOTSTRAP_CACHE_SUITE_ID = "qpls.multimod.heterogeneity.bootstrap-shard-cache.v1"
BOOTSTRAP_CACHE_RECEIPT_SUITE_ID = (
    "qpls.multimod.heterogeneity.bootstrap-shard-cache-receipt.v1"
)
BOOTSTRAP_CURRENT_POINTER_SUITE_ID = (
    "qpls.multimod.heterogeneity.bootstrap-current-generation.v1"
)
BOOTSTRAP_CACHE_INVENTORY_SUITE_ID = (
    "qpls.multimod.heterogeneity.bootstrap-cache-inventory.v1"
)
PRODUCER_SUITE_ID = "qpls.multimod.heterogeneity.production-qualification.v2"
SCHEMA_VERSION = 1
BOOTSTRAP_CHUNK_COUNT = 100
BOOTSTRAP_REQUESTED_REPLICATES = 500
MULTICLASS_POINT_FIXTURE_PLAN = {
    "schema_version": 1,
    "plan_id": "qpls.multimod.heterogeneity.pos-published-p0-k3-k5-point-discovery.v1",
    "purpose": "published_p0_pos_candidate_point_discovery_only",
    "selected_k": [3, 4, 5],
    "observations_per_fixture": 120,
    "allocation": "row_mod_k_exactly_balanced",
    "bootstrap_evidence": "not_requested",
}


class ContractError(ValueError):
    """A checkpoint is not admissible under the frozen shard contract."""


def expected_shard_specs(scale: str) -> list[dict[str, Any]]:
    def row(
        shard_id: str,
        payload_kind: str,
        index: int | None,
        dependencies: list[str],
        resource_class: str,
    ) -> dict[str, Any]:
        return {
            "shard_id": shard_id,
            "payload_kind": payload_kind,
            "index": index,
            "dependencies": dependencies,
            "resource_class": resource_class,
            "parallel_safe_after_build": True,
        }

    rows = [row("sentinel", "sentinel", None, [], "sentinel")]
    recovery_count = 5 if scale == "qualification" else 2
    scenario_count = 5 if scale == "qualification" else 2
    power_count = 10 if scale == "qualification" else 2
    for prefix, payload_kind, count in (
        ("fimix-recovery", "fimix_recovery", recovery_count),
        ("fimix-power", "fimix_power", power_count),
        ("fimix-overlap", "fimix_overlap", scenario_count),
        ("fimix-imbalance", "fimix_imbalance", scenario_count),
        ("fimix-nonnormal", "fimix_nonnormal", scenario_count),
    ):
        rows.extend(
            row(f"{prefix}-{index:02}", payload_kind, index, ["sentinel"], "point")
            for index in range(count)
        )
    for shard_id, payload_kind in (
        ("fimix-candidate-k", "fimix_candidate_k"),
        ("fimix-homogeneous-null", "fimix_homogeneous_null"),
        ("pos-published-p0-discovery", "pos_published_p0_discovery"),
        ("pos-destination-p2-discovery", "pos_destination_p2_discovery"),
        ("pos-destination-p23-discovery", "pos_destination_p23_discovery"),
        (
            "pos-common-metric-failure-discovery",
            "pos_common_metric_failure_discovery",
        ),
        ("pos-homogeneous-null-discovery", "pos_homogeneous_null_discovery"),
        ("pos-overlap-discovery", "pos_overlap_discovery"),
        ("boundary-rank", "boundary_rank"),
        ("boundary-variance", "boundary_variance"),
        ("boundary-rare", "boundary_rare"),
    ):
        rows.append(row(shard_id, payload_kind, None, ["sentinel"], "point"))
    if scale == "qualification":
        for selected_k in range(3, 6):
            discovery_id = f"pos-published-k{selected_k}-discovery"
            rows.append(
                row(
                    discovery_id,
                    "pos_published_k_discovery",
                    selected_k,
                    ["sentinel"],
                    "point",
                )
            )
        for shard_id, dependency in (
            ("bootstrap-fimix-p0", "fimix-recovery-00"),
            ("bootstrap-pos-published-p0", "pos-published-p0-discovery"),
            ("bootstrap-fimix-p2", "pos-destination-p2-discovery"),
            ("bootstrap-pos-destination-p2", "pos-destination-p2-discovery"),
            ("bootstrap-fimix-p23", "pos-destination-p23-discovery"),
            ("bootstrap-pos-destination-p23", "pos-destination-p23-discovery"),
            (
                "bootstrap-pos-common-metric-failure",
                "pos-common-metric-failure-discovery",
            ),
        ):
            rows.append(
                row(
                    shard_id,
                    "bootstrap",
                    None,
                    ["sentinel", dependency],
                    "bootstrap",
                )
            )
    return rows


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def is_sha256(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(
        character in "0123456789abcdef" for character in value
    )


def is_git_commit(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 40 and all(
        character in "0123456789abcdef" for character in value
    )


def read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ContractError(f"{path} must contain one JSON object")
    return value


def atomic_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        dir=path.parent, prefix=f".{path.name}.", suffix=".tmp"
    )
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as stream:
            json.dump(value, stream, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary_name, path)
    except Exception:
        try:
            os.unlink(temporary_name)
        except FileNotFoundError:
            pass
        raise


def validated_plan(path: Path) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    plan = read_json(path)
    if (
        plan.get("schema_version") != SCHEMA_VERSION
        or plan.get("suite_id") != PLAN_SUITE_ID
        or plan.get("producer_suite_id") != PRODUCER_SUITE_ID
        or plan.get("scale") not in {"development", "qualification"}
        or not isinstance(plan.get("campaign_seed"), int)
        or plan.get("metamorphism")
        not in {
            "baseline",
            "declaration_reverse",
            "worker_parallel",
            "seed_repeat",
            "row_reverse",
            "input_column_reverse",
            "sign_reverse",
        }
        or (
            plan.get("metamorphism") == "sign_reverse"
            and (not isinstance(plan.get("sign_columns"), str) or not plan["sign_columns"].strip())
        )
        or (
            plan.get("metamorphism") != "sign_reverse"
            and plan.get("sign_columns") is not None
        )
        or not isinstance(plan.get("workers"), int)
        or not 1 <= plan["workers"] <= 64
        or not isinstance(plan.get("fixture_observations"), int)
        or plan["fixture_observations"] <= 0
        or plan.get("multiclass_point_fixture_plan")
        != MULTICLASS_POINT_FIXTURE_PLAN
        or plan.get("execution_contract")
        != "one_cargo_build_then_dependency_aware_non_cargo_shards"
        or plan.get("sentinel_shard_id") != "sentinel"
        or plan.get("aggregation_order") != "plan_order"
    ):
        raise ContractError("heterogeneity shard plan identity is invalid")
    shards = plan.get("shards")
    if not isinstance(shards, list) or not shards:
        raise ContractError("heterogeneity shard plan must be nonempty")
    by_id: dict[str, dict[str, Any]] = {}
    completed: set[str] = set()
    for position, row in enumerate(shards):
        if not isinstance(row, dict):
            raise ContractError(f"shard plan row {position} is not an object")
        shard_id = row.get("shard_id")
        dependencies = row.get("dependencies")
        if (
            not isinstance(shard_id, str)
            or not shard_id
            or shard_id in by_id
            or not isinstance(row.get("payload_kind"), str)
            or not isinstance(dependencies, list)
            or any(not isinstance(value, str) for value in dependencies)
            or len(set(dependencies)) != len(dependencies)
            or any(value not in completed for value in dependencies)
            or row.get("resource_class") not in {"sentinel", "point", "bootstrap"}
            or row.get("parallel_safe_after_build") is not True
        ):
            raise ContractError(f"shard plan row {position} is invalid or not topological")
        if position > 0 and (
            not dependencies
            or dependencies[0] != "sentinel"
            or (
                row["resource_class"] == "point"
                and dependencies != ["sentinel"]
            )
            or (
                row["resource_class"] == "bootstrap"
                and len(dependencies) != 2
            )
        ):
            raise ContractError(
                f"shard plan row {position} does not have the frozen sentinel/dependency shape"
            )
        by_id[shard_id] = row
        completed.add(shard_id)
    if shards[0].get("shard_id") != "sentinel" or shards[0].get("dependencies") != []:
        raise ContractError("the fast root sentinel must be the first dependency-free shard")
    if shards != expected_shard_specs(plan["scale"]):
        raise ContractError("heterogeneity shard plan differs from the frozen exact inventory")
    return plan, by_id


def result_path(shard_dir: Path, shard_id: str) -> Path:
    return shard_dir / f"{shard_id}.json"


def receipt_path(shard_dir: Path, shard_id: str) -> Path:
    return shard_dir / f"{shard_id}.receipt.json"


def bootstrap_cell_dir(shard_dir: Path, shard_id: str) -> Path:
    return shard_dir.parent / "bootstrap" / shard_id


def bootstrap_prepared_pointer_path(shard_dir: Path, shard_id: str) -> Path:
    return bootstrap_cell_dir(shard_dir, shard_id) / "prepared.current.json"


def bootstrap_cache_pointer_path(
    shard_dir: Path, shard_id: str, chunk_index: int
) -> Path:
    return bootstrap_cell_dir(shard_dir, shard_id) / (
        f"cache-{chunk_index:03d}-of-{BOOTSTRAP_CHUNK_COUNT:03d}.current.json"
    )


def bootstrap_cache_inventory_path(shard_dir: Path, shard_id: str) -> Path:
    return bootstrap_cell_dir(shard_dir, shard_id) / "cache-inventory.json"


def _resolve_current_generation(
    pointer_file: Path,
    shard_id: str,
    kind: str,
    chunk_index: int | None,
) -> tuple[Path, Path, dict[str, Any]]:
    if not pointer_file.is_file():
        raise ContractError(f"current {kind} generation pointer is absent for {shard_id}")
    pointer = read_json(pointer_file)
    generation = pointer.get("generation_id")
    payload_name = pointer.get("payload_file")
    receipt_name = pointer.get("receipt_file")
    if (
        pointer.get("schema_version") != SCHEMA_VERSION
        or pointer.get("suite_id") != BOOTSTRAP_CURRENT_POINTER_SUITE_ID
        or pointer.get("scientific_shard_id") != shard_id
        or pointer.get("kind") != kind
        or pointer.get("chunk_index") != chunk_index
        or not isinstance(generation, str)
        or len(generation) != 32
        or any(character not in "0123456789abcdef" for character in generation)
        or not isinstance(payload_name, str)
        or not payload_name
        or Path(payload_name).name != payload_name
        or not isinstance(receipt_name, str)
        or not receipt_name
        or Path(receipt_name).name != receipt_name
        or not is_sha256(pointer.get("payload_sha256"))
        or not is_sha256(pointer.get("receipt_sha256"))
    ):
        raise ContractError(f"current {kind} generation pointer is invalid for {shard_id}")
    payload = pointer_file.parent / payload_name
    receipt = pointer_file.parent / receipt_name
    if (
        not payload.is_file()
        or not receipt.is_file()
        or sha256_file(payload) != pointer["payload_sha256"]
        or sha256_file(receipt) != pointer["receipt_sha256"]
    ):
        raise ContractError(
            f"current {kind} generation payload/receipt is incomplete or altered for {shard_id}"
        )
    return payload, receipt, pointer


def bootstrap_prepared_path(shard_dir: Path, shard_id: str) -> Path:
    return _resolve_current_generation(
        bootstrap_prepared_pointer_path(shard_dir, shard_id),
        shard_id,
        "prepared",
        None,
    )[0]


def bootstrap_prepared_receipt_path(shard_dir: Path, shard_id: str) -> Path:
    return _resolve_current_generation(
        bootstrap_prepared_pointer_path(shard_dir, shard_id),
        shard_id,
        "prepared",
        None,
    )[1]


def bootstrap_cache_path(shard_dir: Path, shard_id: str, chunk_index: int) -> Path:
    return _resolve_current_generation(
        bootstrap_cache_pointer_path(shard_dir, shard_id, chunk_index),
        shard_id,
        "cache",
        chunk_index,
    )[0]


def bootstrap_cache_receipt_path(
    shard_dir: Path, shard_id: str, chunk_index: int
) -> Path:
    return _resolve_current_generation(
        bootstrap_cache_pointer_path(shard_dir, shard_id, chunk_index),
        shard_id,
        "cache",
        chunk_index,
    )[1]


def validate_shard_result(
    result: dict[str, Any], plan: dict[str, Any], spec: dict[str, Any]
) -> None:
    payload = result.get("payload")
    if (
        result.get("schema_version") != SCHEMA_VERSION
        or result.get("suite_id") != SHARD_SUITE_ID
        or result.get("producer_suite_id") != PRODUCER_SUITE_ID
        or result.get("shard_id") != spec["shard_id"]
        or result.get("scale") != plan["scale"]
        or result.get("campaign_seed") != plan["campaign_seed"]
        or result.get("metamorphism") != plan["metamorphism"]
        or result.get("sign_columns") != plan.get("sign_columns")
        or result.get("workers") != plan["workers"]
        or result.get("fixture_observations") != plan["fixture_observations"]
        or result.get("multiclass_point_fixture_plan")
        != plan["multiclass_point_fixture_plan"]
        or result.get("dependency_shard_ids") != sorted(spec["dependencies"])
        or not isinstance(payload, dict)
        or payload.get("kind") != spec["payload_kind"]
    ):
        raise ContractError(f"shard result identity is invalid for {spec['shard_id']}")
    expected_index = spec.get("index")
    if expected_index is not None and payload.get("index") != expected_index:
        raise ContractError(f"shard payload index is invalid for {spec['shard_id']}")
    if "value" not in payload:
        raise ContractError(f"shard payload value is absent for {spec['shard_id']}")


def dependency_inventory(
    plan_path: Path,
    shard_dir: Path,
    spec: dict[str, Any],
    executable_sha256: str,
    source_commit: str,
) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    for dependency_id in spec["dependencies"]:
        verify_checkpoint(
            plan_path,
            shard_dir,
            dependency_id,
            executable_sha256,
            source_commit,
        )
        dependency_receipt = receipt_path(shard_dir, dependency_id)
        dependency_result = result_path(shard_dir, dependency_id)
        rows.append(
            {
                "shard_id": dependency_id,
                "receipt_sha256": sha256_file(dependency_receipt),
                "result_sha256": sha256_file(dependency_result),
            }
        )
    return rows


def require_bootstrap_spec(
    plan: dict[str, Any], by_id: dict[str, dict[str, Any]], shard_id: str
) -> dict[str, Any]:
    spec = by_id.get(shard_id)
    if spec is None or spec.get("resource_class") != "bootstrap":
        raise ContractError(f"{shard_id} is not a retained bootstrap scientific shard")
    if plan.get("scale") != "qualification":
        raise ContractError("resumable bootstrap cells exist only in qualification scale")
    return spec


def validate_bootstrap_prepared(
    value: dict[str, Any],
    plan: dict[str, Any],
    spec: dict[str, Any],
) -> None:
    execution = value.get("execution")
    reference = execution.get("reference") if isinstance(execution, dict) else None
    orchestrator = (
        reference.get("orchestrator_plan") if isinstance(reference, dict) else None
    )
    heterogeneity = (
        reference.get("heterogeneity_plan") if isinstance(reference, dict) else None
    )
    if (
        value.get("schema_version") != SCHEMA_VERSION
        or value.get("suite_id") != BOOTSTRAP_PREPARED_SUITE_ID
        or value.get("producer_suite_id") != PRODUCER_SUITE_ID
        or value.get("scientific_shard_id") != spec["shard_id"]
        or value.get("scale") != plan["scale"]
        or value.get("campaign_seed") != plan["campaign_seed"]
        or value.get("metamorphism") != plan["metamorphism"]
        or value.get("sign_columns") != plan.get("sign_columns")
        or value.get("workers") != plan["workers"]
        or value.get("fixture_observations") != plan["fixture_observations"]
        or value.get("multiclass_point_fixture_plan")
        != plan["multiclass_point_fixture_plan"]
        or value.get("dependency_shard_ids") != sorted(spec["dependencies"])
        or not isinstance(value.get("cell_id"), str)
        or not value["cell_id"]
        or value.get("chunk_count") != BOOTSTRAP_CHUNK_COUNT
        or value.get("requested_replicates") != BOOTSTRAP_REQUESTED_REPLICATES
        or not isinstance(execution, dict)
        or execution.get("method_version")
        != "qpls.heterogeneity.raw-bootstrap-execution.v2"
        or not is_sha256(execution.get("execution_identity_sha256"))
        or not isinstance(reference, dict)
        or not is_sha256(reference.get("reference_identity_sha256"))
        or not isinstance(orchestrator, dict)
        or orchestrator.get("requested_replicates")
        != BOOTSTRAP_REQUESTED_REPLICATES
        or orchestrator.get("minimum_usable_fraction") != 0.9
        or not isinstance(heterogeneity, dict)
        or heterogeneity.get("requested_replicates")
        != BOOTSTRAP_REQUESTED_REPLICATES
        or heterogeneity.get("minimum_usable_share") != 0.9
    ):
        raise ContractError(
            f"prepared bootstrap execution is invalid for {spec['shard_id']}"
        )


def expected_bootstrap_prepared_receipt(
    plan_path: Path,
    shard_dir: Path,
    spec: dict[str, Any],
    executable_sha256: str,
    source_commit: str,
    prepared_sha256: str,
    execution_identity_sha256: str,
    generation_id: str,
) -> dict[str, Any]:
    plan = read_json(plan_path)
    return {
        "schema_version": SCHEMA_VERSION,
        "suite_id": BOOTSTRAP_PREPARED_RECEIPT_SUITE_ID,
        "status": "passed",
        "generation_id": generation_id,
        "scientific_shard_id": spec["shard_id"],
        "scale": plan["scale"],
        "campaign_seed": plan["campaign_seed"],
        "plan_sha256": sha256_file(plan_path),
        "producer_executable_sha256": executable_sha256,
        "source_commit": source_commit,
        "prepared_execution_sha256": prepared_sha256,
        "execution_identity_sha256": execution_identity_sha256,
        "dependency_receipts": dependency_inventory(
            plan_path, shard_dir, spec, executable_sha256, source_commit
        ),
    }


def verify_bootstrap_prepared(
    plan_path: Path,
    shard_dir: Path,
    shard_id: str,
    executable_sha256: str,
    source_commit: str,
) -> dict[str, Any]:
    plan, by_id = validated_plan(plan_path)
    spec = require_bootstrap_spec(plan, by_id, shard_id)
    if not is_sha256(executable_sha256) or not is_git_commit(source_commit):
        raise ContractError("executable SHA-256 and 40-character source commit are required")
    prepared_file, receipt_file, pointer = _resolve_current_generation(
        bootstrap_prepared_pointer_path(shard_dir, shard_id),
        shard_id,
        "prepared",
        None,
    )
    prepared = read_json(prepared_file)
    validate_bootstrap_prepared(prepared, plan, spec)
    execution = prepared["execution"]
    expected = expected_bootstrap_prepared_receipt(
        plan_path,
        shard_dir,
        spec,
        executable_sha256,
        source_commit,
        sha256_file(prepared_file),
        execution["execution_identity_sha256"],
        pointer["generation_id"],
    )
    if read_json(receipt_file) != expected:
        raise ContractError(f"prepared bootstrap receipt is stale or altered for {shard_id}")
    return prepared


def seal_bootstrap_prepared(arguments: argparse.Namespace) -> None:
    plan, by_id = validated_plan(arguments.plan)
    spec = require_bootstrap_spec(plan, by_id, arguments.shard_id)
    if not is_sha256(arguments.executable_sha256) or not is_git_commit(
        arguments.source_commit
    ):
        raise ContractError("executable SHA-256 and 40-character source commit are required")
    prepared = read_json(arguments.temporary_prepared)
    validate_bootstrap_prepared(prepared, plan, spec)
    cell_dir = bootstrap_cell_dir(arguments.shard_dir, arguments.shard_id)
    cell_dir.mkdir(parents=True, exist_ok=True)
    pointer_file = bootstrap_prepared_pointer_path(
        arguments.shard_dir, arguments.shard_id
    )
    if pointer_file.exists():
        raise ContractError(
            f"prepared bootstrap checkpoint already exists for {arguments.shard_id}"
        )
    generation_id = secrets.token_hex(16)
    destination = cell_dir / f"prepared.g-{generation_id}.json"
    receipt_file = cell_dir / f"prepared.g-{generation_id}.receipt.json"
    if destination.exists() or receipt_file.exists():  # pragma: no cover - random collision
        raise ContractError("prepared generation identity collision")
    os.replace(arguments.temporary_prepared, destination)
    receipt = expected_bootstrap_prepared_receipt(
        arguments.plan,
        arguments.shard_dir,
        spec,
        arguments.executable_sha256,
        arguments.source_commit,
        sha256_file(destination),
        prepared["execution"]["execution_identity_sha256"],
        generation_id,
    )
    atomic_json(receipt_file, receipt)
    atomic_json(
        pointer_file,
        {
            "schema_version": SCHEMA_VERSION,
            "suite_id": BOOTSTRAP_CURRENT_POINTER_SUITE_ID,
            "scientific_shard_id": arguments.shard_id,
            "kind": "prepared",
            "chunk_index": None,
            "generation_id": generation_id,
            "payload_file": destination.name,
            "payload_sha256": sha256_file(destination),
            "receipt_file": receipt_file.name,
            "receipt_sha256": sha256_file(receipt_file),
        },
    )


def validate_bootstrap_cache(
    value: dict[str, Any],
    plan: dict[str, Any],
    spec: dict[str, Any],
    prepared: dict[str, Any],
    chunk_index: int,
) -> None:
    cache = value.get("cache")
    shard = cache.get("shard") if isinstance(cache, dict) else None
    records = cache.get("records") if isinstance(cache, dict) else None
    expected_indices = list(
        range(chunk_index, BOOTSTRAP_REQUESTED_REPLICATES, BOOTSTRAP_CHUNK_COUNT)
    )
    record_indices = (
        [record.get("index") for record in records]
        if isinstance(records, list) and all(isinstance(record, dict) for record in records)
        else None
    )
    record_count = value.get("record_count")
    expected_count = len(expected_indices)
    completed = value.get("completed")
    cancelled = cache.get("cancelled") if isinstance(cache, dict) else None
    if (
        value.get("schema_version") != SCHEMA_VERSION
        or value.get("suite_id") != BOOTSTRAP_CACHE_SUITE_ID
        or value.get("producer_suite_id") != PRODUCER_SUITE_ID
        or value.get("scientific_shard_id") != spec["shard_id"]
        or value.get("scale") != plan["scale"]
        or value.get("campaign_seed") != plan["campaign_seed"]
        or value.get("metamorphism") != plan["metamorphism"]
        or value.get("sign_columns") != plan.get("sign_columns")
        or value.get("workers") != plan["workers"]
        or value.get("fixture_observations") != plan["fixture_observations"]
        or value.get("multiclass_point_fixture_plan")
        != plan["multiclass_point_fixture_plan"]
        or value.get("dependency_shard_ids") != sorted(spec["dependencies"])
        or value.get("cell_id") != prepared.get("cell_id")
        or value.get("prepared_execution_identity_sha256")
        != prepared["execution"]["execution_identity_sha256"]
        or value.get("chunk_index") != chunk_index
        or value.get("chunk_count") != BOOTSTRAP_CHUNK_COUNT
        or value.get("requested_replicates") != BOOTSTRAP_REQUESTED_REPLICATES
        or type(value.get("prior_record_count")) is not int
        or value["prior_record_count"] < 0
        or type(record_count) is not int
        or not 0 <= record_count <= expected_count
        or value.get("expected_record_count") != expected_count
        or not isinstance(cache, dict)
        or not isinstance(shard, dict)
        or shard.get("shard_index") != chunk_index
        or shard.get("shard_count") != BOOTSTRAP_CHUNK_COUNT
        or not is_sha256(shard.get("execution_identity_sha256"))
        or not is_sha256(shard.get("shard_identity_sha256"))
        or not isinstance(records, list)
        or len(records) != record_count
        or record_indices != expected_indices[:record_count]
        or not isinstance(completed, bool)
        or not isinstance(cancelled, bool)
        or completed != (record_count == expected_count and cancelled is False)
        or (not completed and cancelled is not True)
        or (cancelled and record_count == expected_count)
    ):
        raise ContractError(
            f"bootstrap cache is invalid for {spec['shard_id']} chunk {chunk_index}"
        )


def expected_bootstrap_cache_receipt(
    plan_path: Path,
    shard_dir: Path,
    spec: dict[str, Any],
    chunk_index: int,
    executable_sha256: str,
    source_commit: str,
    cache_sha256: str,
    cache_value: dict[str, Any],
    generation_id: str,
) -> dict[str, Any]:
    plan = read_json(plan_path)
    prepared_file = bootstrap_prepared_path(shard_dir, spec["shard_id"])
    prepared_receipt = bootstrap_prepared_receipt_path(shard_dir, spec["shard_id"])
    return {
        "schema_version": SCHEMA_VERSION,
        "suite_id": BOOTSTRAP_CACHE_RECEIPT_SUITE_ID,
        "status": "complete" if cache_value["completed"] else "resumable",
        "generation_id": generation_id,
        "scientific_shard_id": spec["shard_id"],
        "chunk_index": chunk_index,
        "chunk_count": BOOTSTRAP_CHUNK_COUNT,
        "scale": plan["scale"],
        "campaign_seed": plan["campaign_seed"],
        "plan_sha256": sha256_file(plan_path),
        "producer_executable_sha256": executable_sha256,
        "source_commit": source_commit,
        "prepared_execution_sha256": sha256_file(prepared_file),
        "prepared_receipt_sha256": sha256_file(prepared_receipt),
        "execution_identity_sha256": cache_value[
            "prepared_execution_identity_sha256"
        ],
        "cache_sha256": cache_sha256,
        "cache_shard_identity_sha256": cache_value["cache"]["shard"][
            "shard_identity_sha256"
        ],
        "record_count": cache_value["record_count"],
        "expected_record_count": cache_value["expected_record_count"],
        "dependency_receipts": dependency_inventory(
            plan_path, shard_dir, spec, executable_sha256, source_commit
        ),
    }


def verify_bootstrap_cache(
    plan_path: Path,
    shard_dir: Path,
    shard_id: str,
    chunk_index: int,
    executable_sha256: str,
    source_commit: str,
    require_complete: bool = False,
) -> dict[str, Any]:
    plan, by_id = validated_plan(plan_path)
    spec = require_bootstrap_spec(plan, by_id, shard_id)
    if not 0 <= chunk_index < BOOTSTRAP_CHUNK_COUNT:
        raise ContractError("bootstrap chunk index is outside the frozen 100-chunk plan")
    prepared = verify_bootstrap_prepared(
        plan_path, shard_dir, shard_id, executable_sha256, source_commit
    )
    cache_file, receipt_file, pointer = _resolve_current_generation(
        bootstrap_cache_pointer_path(shard_dir, shard_id, chunk_index),
        shard_id,
        "cache",
        chunk_index,
    )
    cache_value = read_json(cache_file)
    validate_bootstrap_cache(cache_value, plan, spec, prepared, chunk_index)
    expected = expected_bootstrap_cache_receipt(
        plan_path,
        shard_dir,
        spec,
        chunk_index,
        executable_sha256,
        source_commit,
        sha256_file(cache_file),
        cache_value,
        pointer["generation_id"],
    )
    if read_json(receipt_file) != expected:
        raise ContractError(
            f"bootstrap cache receipt is stale or altered for {shard_id} chunk {chunk_index}"
        )
    if require_complete and not cache_value["completed"]:
        raise ContractError(
            f"bootstrap cache remains resumable for {shard_id} chunk {chunk_index}"
        )
    return cache_value


def seal_bootstrap_cache(arguments: argparse.Namespace) -> None:
    plan, by_id = validated_plan(arguments.plan)
    spec = require_bootstrap_spec(plan, by_id, arguments.shard_id)
    if not 0 <= arguments.chunk_index < BOOTSTRAP_CHUNK_COUNT:
        raise ContractError("bootstrap chunk index is outside the frozen 100-chunk plan")
    prepared = verify_bootstrap_prepared(
        arguments.plan,
        arguments.shard_dir,
        arguments.shard_id,
        arguments.executable_sha256,
        arguments.source_commit,
    )
    incoming = read_json(arguments.temporary_cache)
    validate_bootstrap_cache(incoming, plan, spec, prepared, arguments.chunk_index)
    pointer_file = bootstrap_cache_pointer_path(
        arguments.shard_dir, arguments.shard_id, arguments.chunk_index
    )
    previous_count = 0
    if pointer_file.exists():
        previous = verify_bootstrap_cache(
            arguments.plan,
            arguments.shard_dir,
            arguments.shard_id,
            arguments.chunk_index,
            arguments.executable_sha256,
            arguments.source_commit,
        )
        if previous["completed"]:
            raise ContractError("a completed bootstrap cache cannot be replaced")
        previous_count = previous["record_count"]
    if incoming["prior_record_count"] != previous_count:
        raise ContractError("bootstrap cache resume did not start from the verified prior cache")
    if incoming["record_count"] <= previous_count:
        raise ContractError("bootstrap cache attempt made zero verified record progress")
    cell_dir = bootstrap_cell_dir(arguments.shard_dir, arguments.shard_id)
    cell_dir.mkdir(parents=True, exist_ok=True)
    generation_id = secrets.token_hex(16)
    prefix = f"cache-{arguments.chunk_index:03d}-of-{BOOTSTRAP_CHUNK_COUNT:03d}"
    destination = cell_dir / f"{prefix}.g-{generation_id}.json"
    receipt_file = cell_dir / f"{prefix}.g-{generation_id}.receipt.json"
    if destination.exists() or receipt_file.exists():  # pragma: no cover - random collision
        raise ContractError("bootstrap cache generation identity collision")
    os.replace(arguments.temporary_cache, destination)
    receipt = expected_bootstrap_cache_receipt(
        arguments.plan,
        arguments.shard_dir,
        spec,
        arguments.chunk_index,
        arguments.executable_sha256,
        arguments.source_commit,
        sha256_file(destination),
        incoming,
        generation_id,
    )
    atomic_json(receipt_file, receipt)
    atomic_json(
        pointer_file,
        {
            "schema_version": SCHEMA_VERSION,
            "suite_id": BOOTSTRAP_CURRENT_POINTER_SUITE_ID,
            "scientific_shard_id": arguments.shard_id,
            "kind": "cache",
            "chunk_index": arguments.chunk_index,
            "generation_id": generation_id,
            "payload_file": destination.name,
            "payload_sha256": sha256_file(destination),
            "receipt_file": receipt_file.name,
            "receipt_sha256": sha256_file(receipt_file),
        },
    )


def complete_bootstrap_inventory(
    plan_path: Path,
    shard_dir: Path,
    spec: dict[str, Any],
    executable_sha256: str,
    source_commit: str,
) -> dict[str, Any]:
    shard_id = spec["shard_id"]
    prepared = verify_bootstrap_prepared(
        plan_path, shard_dir, shard_id, executable_sha256, source_commit
    )
    caches = []
    for chunk_index in range(BOOTSTRAP_CHUNK_COUNT):
        value = verify_bootstrap_cache(
            plan_path,
            shard_dir,
            shard_id,
            chunk_index,
            executable_sha256,
            source_commit,
            require_complete=True,
        )
        cache_file = bootstrap_cache_path(shard_dir, shard_id, chunk_index)
        cache_receipt = bootstrap_cache_receipt_path(
            shard_dir, shard_id, chunk_index
        )
        cache_pointer = bootstrap_cache_pointer_path(
            shard_dir, shard_id, chunk_index
        )
        caches.append(
            {
                "chunk_index": chunk_index,
                "record_count": value["record_count"],
                "cache_file": cache_file.name,
                "cache_sha256": sha256_file(cache_file),
                "receipt_file": cache_receipt.name,
                "receipt_sha256": sha256_file(cache_receipt),
                "pointer_file": cache_pointer.name,
                "pointer_sha256": sha256_file(cache_pointer),
                "cache_shard_identity_sha256": value["cache"]["shard"][
                    "shard_identity_sha256"
                ],
            }
        )
    prepared_file = bootstrap_prepared_path(shard_dir, shard_id)
    prepared_receipt = bootstrap_prepared_receipt_path(shard_dir, shard_id)
    prepared_pointer = bootstrap_prepared_pointer_path(shard_dir, shard_id)
    plan = read_json(plan_path)
    return {
        "schema_version": SCHEMA_VERSION,
        "suite_id": BOOTSTRAP_CACHE_INVENTORY_SUITE_ID,
        "producer_suite_id": PRODUCER_SUITE_ID,
        "scientific_shard_id": shard_id,
        "scale": plan["scale"],
        "campaign_seed": plan["campaign_seed"],
        "metamorphism": plan["metamorphism"],
        "sign_columns": plan.get("sign_columns"),
        "workers": plan["workers"],
        "fixture_observations": plan["fixture_observations"],
        "multiclass_point_fixture_plan": plan["multiclass_point_fixture_plan"],
        "dependency_shard_ids": sorted(spec["dependencies"]),
        "dependency_receipts": dependency_inventory(
            plan_path, shard_dir, spec, executable_sha256, source_commit
        ),
        "cell_id": prepared["cell_id"],
        "plan_sha256": sha256_file(plan_path),
        "producer_executable_sha256": executable_sha256,
        "source_commit": source_commit,
        "chunk_count": BOOTSTRAP_CHUNK_COUNT,
        "requested_replicates": BOOTSTRAP_REQUESTED_REPLICATES,
        "prepared_execution_file": prepared_file.name,
        "prepared_execution_sha256": sha256_file(prepared_file),
        "prepared_receipt_file": prepared_receipt.name,
        "prepared_receipt_sha256": sha256_file(prepared_receipt),
        "prepared_pointer_file": prepared_pointer.name,
        "prepared_pointer_sha256": sha256_file(prepared_pointer),
        "execution_identity_sha256": prepared["execution"][
            "execution_identity_sha256"
        ],
        "caches": caches,
    }


def write_bootstrap_inventory(arguments: argparse.Namespace) -> None:
    plan, by_id = validated_plan(arguments.plan)
    spec = require_bootstrap_spec(plan, by_id, arguments.shard_id)
    cell_dir = bootstrap_cell_dir(arguments.shard_dir, arguments.shard_id)
    if arguments.output.parent.resolve() != cell_dir.resolve():
        raise ContractError("bootstrap cache inventory must be written inside its cell directory")
    inventory = complete_bootstrap_inventory(
        arguments.plan,
        arguments.shard_dir,
        spec,
        arguments.executable_sha256,
        arguments.source_commit,
    )
    atomic_json(arguments.output, inventory)


def expected_receipt(
    plan_path: Path,
    shard_dir: Path,
    spec: dict[str, Any],
    executable_sha256: str,
    source_commit: str,
    result_sha256: str,
) -> dict[str, Any]:
    receipt = {
        "schema_version": SCHEMA_VERSION,
        "suite_id": RECEIPT_SUITE_ID,
        "status": "passed",
        "shard_id": spec["shard_id"],
        "payload_kind": spec["payload_kind"],
        "scale": read_json(plan_path)["scale"],
        "campaign_seed": read_json(plan_path)["campaign_seed"],
        "plan_sha256": sha256_file(plan_path),
        "producer_executable_sha256": executable_sha256,
        "source_commit": source_commit,
        "result_sha256": result_sha256,
        "dependency_receipts": dependency_inventory(
            plan_path, shard_dir, spec, executable_sha256, source_commit
        ),
    }
    if spec["resource_class"] == "bootstrap":
        receipt["resumable_bootstrap"] = complete_bootstrap_inventory(
            plan_path,
            shard_dir,
            spec,
            executable_sha256,
            source_commit,
        )
    return receipt


def verify_checkpoint(
    plan_path: Path,
    shard_dir: Path,
    shard_id: str,
    executable_sha256: str,
    source_commit: str,
) -> None:
    plan, by_id = validated_plan(plan_path)
    if shard_id not in by_id:
        raise ContractError(f"unknown shard id {shard_id}")
    if not is_sha256(executable_sha256) or not is_git_commit(source_commit):
        raise ContractError("executable SHA-256 and 40-character source commit are required")
    result_file = result_path(shard_dir, shard_id)
    receipt_file = receipt_path(shard_dir, shard_id)
    if not result_file.is_file() or not receipt_file.is_file():
        raise ContractError(f"checkpoint files are incomplete for {shard_id}")
    result = read_json(result_file)
    validate_shard_result(result, plan, by_id[shard_id])
    actual_receipt = read_json(receipt_file)
    expected = expected_receipt(
        plan_path,
        shard_dir,
        by_id[shard_id],
        executable_sha256,
        source_commit,
        sha256_file(result_file),
    )
    if actual_receipt != expected:
        raise ContractError(f"checkpoint receipt is stale or altered for {shard_id}")


def seal_checkpoint(arguments: argparse.Namespace) -> None:
    plan, by_id = validated_plan(arguments.plan)
    if arguments.shard_id not in by_id:
        raise ContractError(f"unknown shard id {arguments.shard_id}")
    if not is_sha256(arguments.executable_sha256) or not is_git_commit(arguments.source_commit):
        raise ContractError("executable SHA-256 and 40-character source commit are required")
    spec = by_id[arguments.shard_id]
    result = read_json(arguments.temporary_result)
    validate_shard_result(result, plan, spec)
    arguments.shard_dir.mkdir(parents=True, exist_ok=True)
    destination = result_path(arguments.shard_dir, arguments.shard_id)
    os.replace(arguments.temporary_result, destination)
    receipt = expected_receipt(
        arguments.plan,
        arguments.shard_dir,
        spec,
        arguments.executable_sha256,
        arguments.source_commit,
        sha256_file(destination),
    )
    atomic_json(receipt_path(arguments.shard_dir, arguments.shard_id), receipt)


def record_failure(arguments: argparse.Namespace) -> None:
    plan, by_id = validated_plan(arguments.plan)
    if arguments.shard_id not in by_id:
        raise ContractError(f"unknown shard id {arguments.shard_id}")
    if not is_sha256(arguments.executable_sha256) or not is_git_commit(arguments.source_commit):
        raise ContractError("executable SHA-256 and 40-character source commit are required")
    value: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "suite_id": RECEIPT_SUITE_ID,
        "status": "failed",
        "shard_id": arguments.shard_id,
        "scale": plan["scale"],
        "campaign_seed": plan["campaign_seed"],
        "plan_sha256": sha256_file(arguments.plan),
        "producer_executable_sha256": arguments.executable_sha256,
        "source_commit": arguments.source_commit,
        "exit_code": arguments.exit_code,
        "failure_reason": arguments.failure_reason,
        "stdout_sha256": sha256_file(arguments.stdout) if arguments.stdout.is_file() else None,
        "stderr_sha256": sha256_file(arguments.stderr) if arguments.stderr.is_file() else None,
    }
    atomic_json(arguments.failure_receipt, value)


def payload_rows(
    plan: dict[str, Any], shard_dir: Path, kind: str
) -> list[dict[str, Any]]:
    rows: list[tuple[int, dict[str, Any]]] = []
    for position, spec in enumerate(plan["shards"]):
        if spec["payload_kind"] != kind:
            continue
        payload = read_json(result_path(shard_dir, spec["shard_id"]))["payload"]
        rows.append((position, payload))
    return [payload for _, payload in sorted(rows, key=lambda row: row[0])]


def singleton_payload(plan: dict[str, Any], shard_dir: Path, kind: str) -> dict[str, Any]:
    rows = payload_rows(plan, shard_dir, kind)
    if len(rows) != 1:
        raise ContractError(f"expected exactly one {kind} shard, observed {len(rows)}")
    return rows[0]


def aggregate(arguments: argparse.Namespace) -> None:
    plan, _ = validated_plan(arguments.plan)
    for spec in plan["shards"]:
        verify_checkpoint(
            arguments.plan,
            arguments.shard_dir,
            spec["shard_id"],
            arguments.executable_sha256,
            arguments.source_commit,
        )
    sentinel = singleton_payload(plan, arguments.shard_dir, "sentinel")["value"]
    header = sentinel.get("header")
    if not isinstance(header, dict):
        raise ContractError("sentinel header is absent")
    if (
        header.get("scale") != plan["scale"]
        or header.get("campaign_seed") != plan["campaign_seed"]
        or header.get("seed") != plan["campaign_seed"]
        or header.get("metamorphism") != plan["metamorphism"]
        or header.get("sign_columns") != plan.get("sign_columns")
        or header.get("workers") != plan["workers"]
        or header.get("fixture_observations") != plan["fixture_observations"]
        or header.get("multiclass_point_fixture_plan")
        != plan["multiclass_point_fixture_plan"]
    ):
        raise ContractError("sentinel report header is not bound to the shard plan")
    report = dict(header)
    report["label_alignment_decision_matrix"] = sentinel[
        "label_alignment_decision_matrix"
    ]

    def values(kind: str) -> list[Any]:
        return [row["value"] for row in payload_rows(plan, arguments.shard_dir, kind)]

    report["fimix"] = {
        "strong_separation_recovery": values("fimix_recovery"),
        "power_recovery": values("fimix_power"),
        "overlap_recovery": values("fimix_overlap"),
        "imbalance_recovery": values("fimix_imbalance"),
        "nonnormal_recovery": values("fimix_nonnormal"),
        "simulation_acceptance": {
            "strong_median_ari_minimum": 0.80,
            "power_success_ari_minimum": 0.60,
            "power_success_rate_minimum": 0.80,
            "overlap_median_ari_minimum": 0.35,
            "imbalance_median_ari_minimum": 0.70,
            "nonnormal_median_ari_minimum": 0.70,
        },
        "candidate_k_table": singleton_payload(
            plan, arguments.shard_dir, "fimix_candidate_k"
        )["value"],
        "homogeneous_null": singleton_payload(
            plan, arguments.shard_dir, "fimix_homogeneous_null"
        )["value"],
        "boundaries": {
            "rank_deficient": singleton_payload(
                plan, arguments.shard_dir, "boundary_rank"
            )["value"],
            "variance_collapse": singleton_payload(
                plan, arguments.shard_dir, "boundary_variance"
            )["value"],
            "rare_class": singleton_payload(
                plan, arguments.shard_dir, "boundary_rare"
            )["value"],
            "separately_bound_kernel_test_filter": "fimix_failure_boundary_",
        },
    }
    report["pos"] = {
        "published_p0_discovery": singleton_payload(
            plan, arguments.shard_dir, "pos_published_p0_discovery"
        )["value"],
        "destination_p2_discovery": singleton_payload(
            plan, arguments.shard_dir, "pos_destination_p2_discovery"
        )["value"],
        "destination_p23_discovery": singleton_payload(
            plan, arguments.shard_dir, "pos_destination_p23_discovery"
        )["value"],
        "common_metric_failure_discovery": singleton_payload(
            plan, arguments.shard_dir, "pos_common_metric_failure_discovery"
        )["value"],
        "homogeneous_null_discovery": singleton_payload(
            plan, arguments.shard_dir, "pos_homogeneous_null_discovery"
        )["value"],
        "overlap_discovery": singleton_payload(
            plan, arguments.shard_dir, "pos_overlap_discovery"
        )["value"],
        "published_p0_k3_through_k5_discovery": values("pos_published_k_discovery"),
        "recovery_acceptance": {
            "published_p0_ari_minimum": 0.80,
            "destination_p2_ari_minimum": 0.75,
            "destination_p23_ari_minimum": 0.70,
            "overlap_ari_minimum": 0.35,
            "homogeneous_null_ari_maximum": 0.25,
        },
    }
    bootstrap_order = {
        cell_id: index
        for index, cell_id in enumerate(
            [
                "fimix-p0-fixed-k-bootstrap",
                "pos-published-p0-fixed-k-bootstrap",
                "fimix-p2-fixed-k-bootstrap",
                "pos-destination-p2-fixed-k-bootstrap",
                "fimix-p23-fixed-k-bootstrap",
                "pos-destination-p23-fixed-k-bootstrap",
                "pos-p2-common-metric-failure-fixed-k-bootstrap",
            ]
        )
    }
    bootstrap = values("bootstrap")
    bootstrap.sort(key=lambda row: bootstrap_order.get(row.get("cell_id"), 10_000))
    report["fixed_k_bootstrap"] = bootstrap
    report["shard_execution_receipt"] = {
        "schema_version": SCHEMA_VERSION,
        "plan_sha256": sha256_file(arguments.plan),
        "producer_executable_sha256": arguments.executable_sha256,
        "source_commit": arguments.source_commit,
        "shards": [
            {
                "shard_id": spec["shard_id"],
                "receipt_sha256": sha256_file(
                    receipt_path(arguments.shard_dir, spec["shard_id"])
                ),
            }
            for spec in plan["shards"]
        ],
    }
    atomic_json(arguments.output, report)


def add_identity_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--plan", required=True, type=Path)
    parser.add_argument("--shard-dir", required=True, type=Path)
    parser.add_argument("--executable-sha256", required=True)
    parser.add_argument("--source-commit", required=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    verify = subparsers.add_parser("verify")
    add_identity_arguments(verify)
    verify.add_argument("--shard-id", required=True)

    seal = subparsers.add_parser("seal")
    add_identity_arguments(seal)
    seal.add_argument("--shard-id", required=True)
    seal.add_argument("--temporary-result", required=True, type=Path)

    failure = subparsers.add_parser("record-failure")
    add_identity_arguments(failure)
    failure.add_argument("--shard-id", required=True)
    failure.add_argument("--failure-receipt", required=True, type=Path)
    failure.add_argument("--exit-code", required=True, type=int)
    failure.add_argument("--failure-reason", required=True)
    failure.add_argument("--stdout", required=True, type=Path)
    failure.add_argument("--stderr", required=True, type=Path)

    verify_prepared = subparsers.add_parser("verify-bootstrap-prepared")
    add_identity_arguments(verify_prepared)
    verify_prepared.add_argument("--shard-id", required=True)

    seal_prepared = subparsers.add_parser("seal-bootstrap-prepared")
    add_identity_arguments(seal_prepared)
    seal_prepared.add_argument("--shard-id", required=True)
    seal_prepared.add_argument("--temporary-prepared", required=True, type=Path)

    verify_cache = subparsers.add_parser("verify-bootstrap-cache")
    add_identity_arguments(verify_cache)
    verify_cache.add_argument("--shard-id", required=True)
    verify_cache.add_argument("--chunk-index", required=True, type=int)
    verify_cache.add_argument("--require-complete", action="store_true")

    seal_cache = subparsers.add_parser("seal-bootstrap-cache")
    add_identity_arguments(seal_cache)
    seal_cache.add_argument("--shard-id", required=True)
    seal_cache.add_argument("--chunk-index", required=True, type=int)
    seal_cache.add_argument("--temporary-cache", required=True, type=Path)

    inventory = subparsers.add_parser("write-bootstrap-inventory")
    add_identity_arguments(inventory)
    inventory.add_argument("--shard-id", required=True)
    inventory.add_argument("--output", required=True, type=Path)

    aggregate_parser = subparsers.add_parser("aggregate")
    add_identity_arguments(aggregate_parser)
    aggregate_parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    arguments = parse_args()
    try:
        if arguments.command == "verify":
            verify_checkpoint(
                arguments.plan,
                arguments.shard_dir,
                arguments.shard_id,
                arguments.executable_sha256,
                arguments.source_commit,
            )
        elif arguments.command == "seal":
            seal_checkpoint(arguments)
        elif arguments.command == "record-failure":
            record_failure(arguments)
        elif arguments.command == "verify-bootstrap-prepared":
            verify_bootstrap_prepared(
                arguments.plan,
                arguments.shard_dir,
                arguments.shard_id,
                arguments.executable_sha256,
                arguments.source_commit,
            )
        elif arguments.command == "seal-bootstrap-prepared":
            seal_bootstrap_prepared(arguments)
        elif arguments.command == "verify-bootstrap-cache":
            verify_bootstrap_cache(
                arguments.plan,
                arguments.shard_dir,
                arguments.shard_id,
                arguments.chunk_index,
                arguments.executable_sha256,
                arguments.source_commit,
                arguments.require_complete,
            )
        elif arguments.command == "seal-bootstrap-cache":
            seal_bootstrap_cache(arguments)
        elif arguments.command == "write-bootstrap-inventory":
            write_bootstrap_inventory(arguments)
        elif arguments.command == "aggregate":
            aggregate(arguments)
        else:  # pragma: no cover - argparse owns this boundary
            raise ContractError(f"unsupported command {arguments.command}")
    except (ContractError, OSError, json.JSONDecodeError, KeyError, TypeError) as error:
        print(f"heterogeneity shard contract failed: {error}", file=os.sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
