#!/usr/bin/env python3
"""Identity-bound checkpoints for the bounded MultiMod performance topology.

This module owns transport only.  The Rust examples and their existing MGA and
heterogeneity checkpoint helpers remain the scientific authorities.  A stage is
publishable only when its measurement and every declared artifact are sealed in
one exact, portable, SHA-256-bound transaction.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import tempfile
from pathlib import Path
from typing import Any

import multimod_heterogeneity_shards_v2 as heterogeneity
import multimod_mga_shards_v1 as mga


SCHEMA_VERSION = 2
PLAN_SUITE_ID = "qpls.v256.multimod.performance-topology-plan.v2"
MEASUREMENT_SUITE_ID = "qpls.v256.multimod.performance-stage-measurement.v2"
RECEIPT_SUITE_ID = "qpls.v256.multimod.performance-stage-receipt.v2"
AGGREGATE_SUITE_ID = "qpls.v256.multimod.performance-execution-topology.v2"
BUILD_ATTEMPT_SUITE_ID = "qpls.v256.multimod.performance-release-build-attempt.v2"
SOURCE_COMMIT_LENGTH = 40
ATOMIC_TIMEOUT_SECONDS = 1800
INTERNAL_TIMEOUT_SECONDS = 6480
PEAK_WORKING_SET_LIMIT_BYTES = 12 * 1024**3

MGA_CELL_ID = "mga-general-20-groups"
HETEROGENEITY_SHARD_IDS = (
    "sentinel",
    "pos-destination-p23-discovery",
    "bootstrap-fimix-p23",
    "bootstrap-pos-destination-p23",
)


class ContractError(ValueError):
    """The topology plan, measurement, artifact, or receipt is inadmissible."""


class MissingCheckpointError(ContractError):
    """No complete transaction exists for the requested stage."""


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


def is_source_commit(value: Any) -> bool:
    return isinstance(value, str) and len(value) == SOURCE_COMMIT_LENGTH and all(
        character in "0123456789abcdef" for character in value
    )


def read_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ContractError(f"could not read JSON object {path}: {error}") from error
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


def artifact(relative_path: str, role: str) -> dict[str, str]:
    return {"role": role, "relative_path": relative_path}


def stage_specs(seed: int) -> list[dict[str, Any]]:
    return [
        {
            "stage_id": "release-build",
            "executable_id": "cargo",
            "dependencies": [],
            "process_count": 1,
            "parallel_processes": 1,
            "command_identities": [
                [
                    "cargo",
                    "build",
                    "--release",
                    "--locked",
                    "-p",
                    "qpls-runner",
                    "--example",
                    "multimod_mga_qualification_v1",
                    "--example",
                    "multimod_heterogeneity_qualification_v2",
                    "--example",
                    "multimod_maximum_profiles_performance_v1",
                ]
            ],
            "artifacts": [
                artifact("build/build-attempt.json", "release_build_attempt"),
                artifact("logs/release-build.stdout.log", "stdout"),
                artifact("logs/release-build.stderr.log", "stderr"),
            ],
            "atomic_timeout_seconds": ATOMIC_TIMEOUT_SECONDS,
            "maximum_peak_working_set_bytes": PEAK_WORKING_SET_LIMIT_BYTES,
        },
        {
            "stage_id": "mga-general-20-groups",
            "executable_id": "mga",
            "dependencies": ["release-build"],
            "process_count": 1,
            "parallel_processes": 1,
            "command_identities": [
                [
                    "multimod_mga_qualification_v1",
                    "--scale",
                    "qualification",
                    "--seed",
                    str(seed),
                    "--cell",
                    MGA_CELL_ID,
                    "--cache-root",
                    "mga/production-cache",
                    "--plan-path",
                    "mga/cell-plan.json",
                ]
            ],
            "artifacts": [
                artifact(
                    f"mga/cells/{MGA_CELL_ID}/result.json", "scientific_result"
                ),
                artifact(
                    f"mga/cells/{MGA_CELL_ID}/receipt.json",
                    "scientific_receipt",
                ),
                artifact("mga/cell-plan.json", "producer_plan"),
                artifact("logs/mga-general-20-groups.stdout.log", "stdout"),
                artifact("logs/mga-general-20-groups.stderr.log", "stderr"),
            ],
            "atomic_timeout_seconds": ATOMIC_TIMEOUT_SECONDS,
            "maximum_peak_working_set_bytes": PEAK_WORKING_SET_LIMIT_BYTES,
        },
        {
            "stage_id": "heterogeneity-sentinel",
            "executable_id": "heterogeneity",
            "dependencies": ["release-build"],
            "process_count": 1,
            "parallel_processes": 1,
            "command_identities": [
                [
                    "multimod_heterogeneity_qualification_v2",
                    "--scale",
                    "qualification",
                    "--seed",
                    str(seed),
                    "--shard",
                    "sentinel",
                ]
            ],
            "artifacts": [
                artifact("heterogeneity/shards/sentinel.json", "scientific_result"),
                artifact(
                    "heterogeneity/shards/sentinel.receipt.json",
                    "scientific_receipt",
                ),
                artifact("heterogeneity/shard-plan.json", "producer_plan"),
                artifact("logs/heterogeneity-sentinel.stdout.log", "stdout"),
                artifact("logs/heterogeneity-sentinel.stderr.log", "stderr"),
            ],
            "atomic_timeout_seconds": min(120, ATOMIC_TIMEOUT_SECONDS),
            "maximum_peak_working_set_bytes": PEAK_WORKING_SET_LIMIT_BYTES,
        },
        {
            "stage_id": "heterogeneity-p23-discovery",
            "executable_id": "heterogeneity",
            "dependencies": ["release-build", "heterogeneity-sentinel"],
            "process_count": 1,
            "parallel_processes": 1,
            "command_identities": [
                [
                    "multimod_heterogeneity_qualification_v2",
                    "--scale",
                    "qualification",
                    "--seed",
                    str(seed),
                    "--shard",
                    "pos-destination-p23-discovery",
                    "--dependency",
                    "heterogeneity/shards/sentinel.json",
                ]
            ],
            "artifacts": [
                artifact(
                    "heterogeneity/shards/pos-destination-p23-discovery.json",
                    "scientific_result",
                ),
                artifact(
                    "heterogeneity/shards/pos-destination-p23-discovery.receipt.json",
                    "scientific_receipt",
                ),
                artifact("heterogeneity/shard-plan.json", "producer_plan"),
                artifact("logs/heterogeneity-p23-discovery.stdout.log", "stdout"),
                artifact("logs/heterogeneity-p23-discovery.stderr.log", "stderr"),
            ],
            "atomic_timeout_seconds": ATOMIC_TIMEOUT_SECONDS,
            "maximum_peak_working_set_bytes": PEAK_WORKING_SET_LIMIT_BYTES,
        },
        {
            "stage_id": "heterogeneity-p23-bootstrap-pair",
            "executable_id": "heterogeneity",
            "dependencies": [
                "release-build",
                "heterogeneity-sentinel",
                "heterogeneity-p23-discovery",
            ],
            "process_count": 2,
            "parallel_processes": 2,
            "command_identities": [
                [
                    "multimod_heterogeneity_qualification_v2",
                    "--scale",
                    "qualification",
                    "--seed",
                    str(seed),
                    "--shard",
                    "bootstrap-fimix-p23",
                    "--dependency",
                    "heterogeneity/shards/sentinel.json",
                    "--dependency",
                    "heterogeneity/shards/pos-destination-p23-discovery.json",
                ],
                [
                    "multimod_heterogeneity_qualification_v2",
                    "--scale",
                    "qualification",
                    "--seed",
                    str(seed),
                    "--shard",
                    "bootstrap-pos-destination-p23",
                    "--dependency",
                    "heterogeneity/shards/sentinel.json",
                    "--dependency",
                    "heterogeneity/shards/pos-destination-p23-discovery.json",
                ],
            ],
            "artifacts": [
                artifact(
                    "heterogeneity/shards/bootstrap-fimix-p23.json",
                    "fimix_scientific_result",
                ),
                artifact(
                    "heterogeneity/shards/bootstrap-fimix-p23.receipt.json",
                    "fimix_scientific_receipt",
                ),
                artifact(
                    "heterogeneity/shards/bootstrap-pos-destination-p23.json",
                    "pos_scientific_result",
                ),
                artifact(
                    "heterogeneity/shards/bootstrap-pos-destination-p23.receipt.json",
                    "pos_scientific_receipt",
                ),
                artifact("heterogeneity/shard-plan.json", "producer_plan"),
                artifact("logs/heterogeneity-bootstrap-fimix-p23.stdout.log", "fimix_stdout"),
                artifact("logs/heterogeneity-bootstrap-fimix-p23.stderr.log", "fimix_stderr"),
                artifact("logs/heterogeneity-bootstrap-pos-p23.stdout.log", "pos_stdout"),
                artifact("logs/heterogeneity-bootstrap-pos-p23.stderr.log", "pos_stderr"),
            ],
            "atomic_timeout_seconds": ATOMIC_TIMEOUT_SECONDS,
            "maximum_peak_working_set_bytes": PEAK_WORKING_SET_LIMIT_BYTES,
        },
        {
            "stage_id": "conditional-sidecar-resume",
            "executable_id": "maximum",
            "dependencies": ["release-build"],
            "process_count": 1,
            "parallel_processes": 1,
            "command_identities": [
                [
                    "multimod_maximum_profiles_performance_v1",
                    "--seed",
                    str(seed),
                ]
            ],
            "artifacts": [
                artifact("maximum/result.json", "scientific_result"),
                artifact("logs/conditional-sidecar-resume.stdout.log", "stdout"),
                artifact("logs/conditional-sidecar-resume.stderr.log", "stderr"),
            ],
            "atomic_timeout_seconds": ATOMIC_TIMEOUT_SECONDS,
            "maximum_peak_working_set_bytes": PEAK_WORKING_SET_LIMIT_BYTES,
        },
    ]


def validate_build_attempt(
    value: dict[str, Any],
    *,
    source_commit: str,
    cargo_sha256: str,
    cargo_lock_sha256: str,
    output_executables: dict[str, dict[str, str]],
) -> None:
    required = {
        "schema_version",
        "suite_id",
        "status",
        "attempt_id",
        "attempt_count",
        "source_commit",
        "cargo_executable_sha256",
        "cargo_lock_sha256",
        "command_identity",
        "output_executables",
        "measurement",
    }
    measurement_required = {
        "wall_time_milliseconds",
        "peak_working_set_bytes",
        "sampled_concurrent_peak_working_set_bytes",
        "per_process_peak_working_set_bytes",
        "working_set_sample_count",
        "process_count",
        "process_exit_codes",
    }
    measurement = value.get("measurement")
    if (
        set(value) != required
        or value.get("schema_version") != SCHEMA_VERSION
        or value.get("suite_id") != BUILD_ATTEMPT_SUITE_ID
        or value.get("status") != "completed"
        or not isinstance(value.get("attempt_id"), str)
        or not value["attempt_id"]
        or value.get("attempt_count") != 1
        or value.get("source_commit") != source_commit
        or value.get("cargo_executable_sha256") != cargo_sha256
        or value.get("cargo_lock_sha256") != cargo_lock_sha256
        or value.get("command_identity") != stage_specs(42)[0]["command_identities"][0]
        or value.get("output_executables") != output_executables
        or not isinstance(measurement, dict)
        or set(measurement) != measurement_required
        or measurement.get("process_count") != 1
        or measurement.get("process_exit_codes") != [0]
        or not isinstance(measurement.get("wall_time_milliseconds"), int)
        or not 0 <= measurement["wall_time_milliseconds"] <= ATOMIC_TIMEOUT_SECONDS * 1000
        or not isinstance(measurement.get("working_set_sample_count"), int)
        or measurement["working_set_sample_count"] < 1
        or not isinstance(measurement.get("sampled_concurrent_peak_working_set_bytes"), int)
        or measurement["sampled_concurrent_peak_working_set_bytes"] < 0
        or not isinstance(measurement.get("per_process_peak_working_set_bytes"), list)
        or len(measurement["per_process_peak_working_set_bytes"]) != 1
        or not isinstance(measurement.get("peak_working_set_bytes"), int)
        or measurement["peak_working_set_bytes"] <= 0
    ):
        raise ContractError("release-build attempt identity or measurement is invalid")
    process_peak = measurement["per_process_peak_working_set_bytes"][0]
    if (
        not isinstance(process_peak, dict)
        or set(process_peak) != {"process_identity", "peak_working_set_bytes"}
        or process_peak.get("process_identity") != "release-build"
        or not isinstance(process_peak.get("peak_working_set_bytes"), int)
        or process_peak["peak_working_set_bytes"] <= 0
        or measurement["peak_working_set_bytes"]
        != max(
            measurement["sampled_concurrent_peak_working_set_bytes"],
            process_peak["peak_working_set_bytes"],
        )
    ):
        raise ContractError("release-build conservative peak identity is invalid")


def build_plan(arguments: argparse.Namespace) -> dict[str, Any]:
    if arguments.seed != 42:
        raise ContractError("qualification performance topology requires seed 42")
    if not is_source_commit(arguments.source_commit):
        raise ContractError("an exact lowercase 40-character source commit is required")
    mga_plan, mga_cells = mga.validated_plan(arguments.mga_plan)
    heterogeneity_plan, heterogeneity_shards = heterogeneity.validated_plan(
        arguments.heterogeneity_plan
    )
    if (
        mga_plan["scale"] != "qualification"
        or mga_plan["seed"] != arguments.seed
        or mga_plan["permutation_samples"] != 5000
        or mga_plan["bootstrap_samples"] != 5000
        or MGA_CELL_ID not in mga_cells
    ):
        raise ContractError("MGA plan is not the exact 5,000-draw 20-group envelope")
    expected_dependencies = {
        "sentinel": [],
        "pos-destination-p23-discovery": ["sentinel"],
        "bootstrap-fimix-p23": ["sentinel", "pos-destination-p23-discovery"],
        "bootstrap-pos-destination-p23": [
            "sentinel",
            "pos-destination-p23-discovery",
        ],
    }
    if (
        heterogeneity_plan["scale"] != "qualification"
        or heterogeneity_plan["campaign_seed"] != arguments.seed
        or heterogeneity_plan["metamorphism"] != "baseline"
        or heterogeneity_plan["workers"] != 1
    ):
        raise ContractError("heterogeneity plan is not the exact baseline envelope")
    for shard_id, dependencies in expected_dependencies.items():
        if (
            shard_id not in heterogeneity_shards
            or heterogeneity_shards[shard_id]["dependencies"] != dependencies
        ):
            raise ContractError(f"heterogeneity dependency envelope differs for {shard_id}")
    executables = {
        "cargo": arguments.cargo_executable,
        "mga": arguments.mga_executable,
        "heterogeneity": arguments.heterogeneity_executable,
        "maximum": arguments.maximum_executable,
    }
    for executable_id, path in executables.items():
        if not path.is_file():
            raise ContractError(f"{executable_id} release executable is absent: {path}")
    executable_rows = {
        executable_id: {
            "file_name": path.name,
            "sha256": sha256_file(path),
        }
        for executable_id, path in executables.items()
    }
    cargo_lock_sha256 = sha256_file(arguments.cargo_lock)
    build_attempt = read_object(arguments.build_attempt)
    validate_build_attempt(
        build_attempt,
        source_commit=arguments.source_commit,
        cargo_sha256=executable_rows["cargo"]["sha256"],
        cargo_lock_sha256=cargo_lock_sha256,
        output_executables={
            executable_id: executable_rows[executable_id]
            for executable_id in ("mga", "heterogeneity", "maximum")
        },
    )
    return {
        "schema_version": SCHEMA_VERSION,
        "suite_id": PLAN_SUITE_ID,
        "source_commit": arguments.source_commit,
        "seed": arguments.seed,
        "scale": "qualification",
        "execution_contract": (
            "one_release_build_then_direct_dependency_bounded_processes_v1"
        ),
        "internal_timeout_seconds": INTERNAL_TIMEOUT_SECONDS,
        "per_atomic_process_timeout_seconds": ATOMIC_TIMEOUT_SECONDS,
        "mga_permutation_samples": 5000,
        "mga_bootstrap_samples": 5000,
        "heterogeneity_bootstrap_samples": 500,
        "cargo_lock_sha256": cargo_lock_sha256,
        "build_attempt_sha256": sha256_file(arguments.build_attempt),
        "mga_plan_sha256": sha256_file(arguments.mga_plan),
        "mga_environment_sha256": mga_plan["baseline_environment_sha256"],
        "heterogeneity_plan_sha256": sha256_file(arguments.heterogeneity_plan),
        "executables": executable_rows,
        "stages": stage_specs(arguments.seed),
    }


def validated_plan(path: Path) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    plan = read_object(path)
    required = {
        "schema_version",
        "suite_id",
        "source_commit",
        "seed",
        "scale",
        "execution_contract",
        "internal_timeout_seconds",
        "per_atomic_process_timeout_seconds",
        "mga_permutation_samples",
        "mga_bootstrap_samples",
        "heterogeneity_bootstrap_samples",
        "cargo_lock_sha256",
        "build_attempt_sha256",
        "mga_plan_sha256",
        "mga_environment_sha256",
        "heterogeneity_plan_sha256",
        "executables",
        "stages",
    }
    if set(plan) != required:
        raise ContractError("performance topology plan schema is not exact")
    if (
        plan["schema_version"] != SCHEMA_VERSION
        or plan["suite_id"] != PLAN_SUITE_ID
        or not is_source_commit(plan["source_commit"])
        or plan["seed"] != 42
        or plan["scale"] != "qualification"
        or plan["execution_contract"]
        != "one_release_build_then_direct_dependency_bounded_processes_v1"
        or plan["internal_timeout_seconds"] != INTERNAL_TIMEOUT_SECONDS
        or plan["per_atomic_process_timeout_seconds"] != ATOMIC_TIMEOUT_SECONDS
        or plan["mga_permutation_samples"] != 5000
        or plan["mga_bootstrap_samples"] != 5000
        or plan["heterogeneity_bootstrap_samples"] != 500
        or not is_sha256(plan["cargo_lock_sha256"])
        or not is_sha256(plan["build_attempt_sha256"])
        or not is_sha256(plan["mga_plan_sha256"])
        or not is_sha256(plan["mga_environment_sha256"])
        or not is_sha256(plan["heterogeneity_plan_sha256"])
        or plan["stages"] != stage_specs(42)
    ):
        raise ContractError("performance topology plan identity is invalid")
    executables = plan["executables"]
    if not isinstance(executables, dict) or set(executables) != {
        "cargo",
        "mga",
        "heterogeneity",
        "maximum",
    }:
        raise ContractError("performance executable inventory is not exact")
    for executable_id, value in executables.items():
        if (
            not isinstance(value, dict)
            or set(value) != {"file_name", "sha256"}
            or not isinstance(value["file_name"], str)
            or not value["file_name"]
            or not is_sha256(value["sha256"])
        ):
            raise ContractError(f"performance executable identity is invalid: {executable_id}")
    by_id = {row["stage_id"]: row for row in plan["stages"]}
    if len(by_id) != len(plan["stages"]):
        raise ContractError("performance stage identities are duplicated")
    return plan, by_id


def checkpoint_directory(root: Path, stage_id: str) -> Path:
    return root / "performance-receipts" / stage_id


def measured_artifacts(
    work_root: Path, spec: dict[str, Any], plan: dict[str, Any]
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for item in spec["artifacts"]:
        path = work_root / Path(item["relative_path"])
        if not path.is_file():
            raise ContractError(
                f"{spec['stage_id']} artifact is absent: {item['relative_path']}"
            )
        digest = sha256_file(path)
        if item["relative_path"] == "build/build-attempt.json":
            if digest != plan["build_attempt_sha256"]:
                raise ContractError("release-build attempt differs from the topology identity")
            validate_build_attempt(
                read_object(path),
                source_commit=plan["source_commit"],
                cargo_sha256=plan["executables"]["cargo"]["sha256"],
                cargo_lock_sha256=plan["cargo_lock_sha256"],
                output_executables={
                    executable_id: plan["executables"][executable_id]
                    for executable_id in ("mga", "heterogeneity", "maximum")
                },
            )
        if item["relative_path"] == "mga/cell-plan.json" and digest != plan["mga_plan_sha256"]:
            raise ContractError("MGA producer plan differs from the topology identity")
        if (
            item["relative_path"] == "heterogeneity/shard-plan.json"
            and digest != plan["heterogeneity_plan_sha256"]
        ):
            raise ContractError("heterogeneity producer plan differs from the topology identity")
        rows.append(
            {
                "role": item["role"],
                "relative_path": item["relative_path"],
                "size_bytes": path.stat().st_size,
                "sha256": digest,
            }
        )
    return rows


def validate_measurement(
    value: dict[str, Any], plan: dict[str, Any], spec: dict[str, Any]
) -> None:
    required = {
        "schema_version",
        "suite_id",
        "stage_id",
        "topology_plan_sha256",
        "source_commit",
        "executable_sha256",
        "command_identities",
        "wall_time_milliseconds",
        "peak_working_set_bytes",
        "sampled_concurrent_peak_working_set_bytes",
        "per_process_peak_working_set_bytes",
        "working_set_sample_count",
        "process_count",
        "process_exit_codes",
        "atomic_timeout_seconds",
    }
    executable_sha256 = plan["executables"][spec["executable_id"]]["sha256"]
    if set(value) != required:
        raise ContractError(f"{spec['stage_id']} measurement schema is not exact")
    per_process_peaks = value["per_process_peak_working_set_bytes"]
    per_process_peak_sum = 0
    per_process_peak_identities: set[str] = set()
    if isinstance(per_process_peaks, list):
        for row in per_process_peaks:
            if (
                not isinstance(row, dict)
                or set(row) != {"process_identity", "peak_working_set_bytes"}
                or not isinstance(row.get("process_identity"), str)
                or not row["process_identity"]
                or row["process_identity"] in per_process_peak_identities
                or not isinstance(row.get("peak_working_set_bytes"), int)
                or row["peak_working_set_bytes"] <= 0
            ):
                raise ContractError(
                    f"{spec['stage_id']} per-process peak inventory is invalid"
                )
            per_process_peak_identities.add(row["process_identity"])
            per_process_peak_sum += row["peak_working_set_bytes"]
    sampled_concurrent_peak = value["sampled_concurrent_peak_working_set_bytes"]
    if (
        value["schema_version"] != SCHEMA_VERSION
        or value["suite_id"] != MEASUREMENT_SUITE_ID
        or value["stage_id"] != spec["stage_id"]
        or value["source_commit"] != plan["source_commit"]
        or value["executable_sha256"] != executable_sha256
        or value["command_identities"] != spec["command_identities"]
        or value["process_count"] != spec["process_count"]
        or value["process_exit_codes"] != [0] * spec["process_count"]
        or value["atomic_timeout_seconds"] != spec["atomic_timeout_seconds"]
        or not isinstance(value["wall_time_milliseconds"], int)
        or value["wall_time_milliseconds"] < 0
        or value["wall_time_milliseconds"]
        > spec["atomic_timeout_seconds"] * 1000
        or not isinstance(value["peak_working_set_bytes"], int)
        or not 0 < value["peak_working_set_bytes"]
        <= spec["maximum_peak_working_set_bytes"]
        or not isinstance(sampled_concurrent_peak, int)
        or sampled_concurrent_peak < 0
        or not isinstance(per_process_peaks, list)
        or len(per_process_peaks) != spec["process_count"]
        or value["peak_working_set_bytes"]
        != max(sampled_concurrent_peak, per_process_peak_sum)
        or not isinstance(value["working_set_sample_count"], int)
        or value["working_set_sample_count"] < 1
    ):
        raise ContractError(f"{spec['stage_id']} measurement identity or bounds differ")


def dependency_inventory(
    plan_path: Path, work_root: Path, receipt_root: Path, spec: dict[str, Any]
) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    for dependency_id in spec["dependencies"]:
        verify_checkpoint(plan_path, work_root, receipt_root, dependency_id)
        directory = checkpoint_directory(receipt_root, dependency_id)
        rows.append(
            {
                "stage_id": dependency_id,
                "measurement_sha256": sha256_file(directory / "measurement.json"),
                "receipt_sha256": sha256_file(directory / "receipt.json"),
            }
        )
    return rows


def expected_receipt(
    plan_path: Path,
    work_root: Path,
    receipt_root: Path,
    spec: dict[str, Any],
    measurement_path: Path,
) -> dict[str, Any]:
    plan, _ = validated_plan(plan_path)
    return {
        "schema_version": SCHEMA_VERSION,
        "suite_id": RECEIPT_SUITE_ID,
        "stage_id": spec["stage_id"],
        "source_commit": plan["source_commit"],
        "topology_plan_sha256": sha256_file(plan_path),
        "executable_sha256": plan["executables"][spec["executable_id"]]["sha256"],
        "measurement_sha256": sha256_file(measurement_path),
        "artifacts": measured_artifacts(work_root, spec, plan),
        "dependency_receipts": dependency_inventory(
            plan_path, work_root, receipt_root, spec
        ),
    }


def verify_checkpoint(
    plan_path: Path,
    work_root: Path,
    receipt_root: Path,
    stage_id: str,
) -> dict[str, Any]:
    plan, by_id = validated_plan(plan_path)
    if stage_id not in by_id:
        raise ContractError(f"unknown performance stage {stage_id}")
    directory = checkpoint_directory(receipt_root, stage_id)
    if not directory.exists():
        raise MissingCheckpointError(f"performance checkpoint is absent: {stage_id}")
    if not directory.is_dir() or sorted(path.name for path in directory.iterdir()) != [
        "measurement.json",
        "receipt.json",
    ]:
        raise ContractError(f"performance checkpoint transaction is not exact: {stage_id}")
    measurement_path = directory / "measurement.json"
    receipt_path = directory / "receipt.json"
    measurement = read_object(measurement_path)
    if measurement.get("topology_plan_sha256") != sha256_file(plan_path):
        raise ContractError(f"{stage_id} measurement belongs to another topology plan")
    validate_measurement(measurement, plan, by_id[stage_id])
    expected = expected_receipt(
        plan_path, work_root, receipt_root, by_id[stage_id], measurement_path
    )
    if read_object(receipt_path) != expected:
        raise ContractError(f"performance checkpoint is stale or altered: {stage_id}")
    return measurement


def seal_checkpoint(arguments: argparse.Namespace) -> None:
    plan, by_id = validated_plan(arguments.plan)
    if arguments.stage_id not in by_id:
        raise ContractError(f"unknown performance stage {arguments.stage_id}")
    spec = by_id[arguments.stage_id]
    measurement = read_object(arguments.temporary_measurement)
    if measurement.get("topology_plan_sha256") != sha256_file(arguments.plan):
        raise ContractError("temporary measurement belongs to another topology plan")
    validate_measurement(measurement, plan, spec)
    destination = checkpoint_directory(arguments.receipt_root, arguments.stage_id)
    if destination.exists():
        raise ContractError(f"duplicate performance checkpoint rejected: {arguments.stage_id}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary_directory = Path(
        tempfile.mkdtemp(
            dir=destination.parent,
            prefix=f".{arguments.stage_id}.",
            suffix=".tmp",
        )
    )
    try:
        measurement_path = temporary_directory / "measurement.json"
        shutil.move(str(arguments.temporary_measurement), measurement_path)
        receipt = expected_receipt(
            arguments.plan,
            arguments.work_root,
            arguments.receipt_root,
            spec,
            measurement_path,
        )
        atomic_json(temporary_directory / "receipt.json", receipt)
        os.rename(temporary_directory, destination)
    except Exception:
        # A dot-prefixed directory is deliberately unpublishable and retained.
        raise


def build_aggregate(
    plan_path: Path, work_root: Path, receipt_root: Path
) -> dict[str, Any]:
    plan, _ = validated_plan(plan_path)
    measurements = {
        stage["stage_id"]: verify_checkpoint(
            plan_path, work_root, receipt_root, stage["stage_id"]
        )
        for stage in plan["stages"]
    }
    build_attempt = read_object(work_root / "build" / "build-attempt.json")
    validate_build_attempt(
        build_attempt,
        source_commit=plan["source_commit"],
        cargo_sha256=plan["executables"]["cargo"]["sha256"],
        cargo_lock_sha256=plan["cargo_lock_sha256"],
        output_executables={
            executable_id: plan["executables"][executable_id]
            for executable_id in ("mga", "heterogeneity", "maximum")
        },
    )
    receipt_rows = []
    scientific_output_bytes: dict[str, int] = {}
    for stage in plan["stages"]:
        stage_id = stage["stage_id"]
        directory = checkpoint_directory(receipt_root, stage_id)
        receipt = read_object(directory / "receipt.json")
        scientific_output_bytes[stage_id] = sum(
            row["size_bytes"]
            for row in receipt["artifacts"]
            if row["role"].endswith("scientific_result")
        )
        receipt_rows.append(
            {
                "stage_id": stage_id,
                "measurement": measurements[stage_id],
                "measurement_sha256": sha256_file(directory / "measurement.json"),
                "receipt_sha256": sha256_file(directory / "receipt.json"),
            }
        )

    def workload(workload_id: str, stage_ids: list[str]) -> dict[str, Any]:
        selected = [measurements[stage_id] for stage_id in stage_ids]
        return {
            "workload_id": workload_id,
            "stage_ids": stage_ids,
            "wall_time_milliseconds": sum(
                row["wall_time_milliseconds"] for row in selected
            ),
            "peak_working_set_bytes": max(
                row["peak_working_set_bytes"] for row in selected
            ),
            "working_set_sample_count": sum(
                row["working_set_sample_count"] for row in selected
            ),
            "atomic_process_count": sum(row["process_count"] for row in selected),
            "output_size_bytes": sum(
                scientific_output_bytes[stage_id] for stage_id in stage_ids
            ),
        }

    return {
        "schema_version": SCHEMA_VERSION,
        "suite_id": AGGREGATE_SUITE_ID,
        "passed": True,
        "source_commit": plan["source_commit"],
        "seed": plan["seed"],
        "topology_plan_sha256": sha256_file(plan_path),
        "one_release_cargo_build": build_attempt["attempt_count"] == 1,
        "release_build": {
            "attempt_id": build_attempt["attempt_id"],
            "attempt_count": build_attempt["attempt_count"],
            "attempt_sha256": sha256_file(
                work_root / "build" / "build-attempt.json"
            ),
            "cargo_executable_sha256": build_attempt["cargo_executable_sha256"],
            "cargo_lock_sha256": build_attempt["cargo_lock_sha256"],
            "output_executables": build_attempt["output_executables"],
        },
        "direct_binary_execution_after_build": True,
        "internal_timeout_seconds": INTERNAL_TIMEOUT_SECONDS,
        "stages": receipt_rows,
        "workloads": [
            workload("mga_20_groups_190_pairs", ["mga-general-20-groups"]),
            workload(
                "heterogeneity_locked_p23",
                [
                    "heterogeneity-sentinel",
                    "heterogeneity-p23-discovery",
                    "heterogeneity-p23-bootstrap-pair",
                ],
            ),
            workload(
                "conditional_sidecar_resume", ["conditional-sidecar-resume"]
            ),
        ],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    write_plan = commands.add_parser("write-plan")
    write_plan.add_argument("--output", required=True, type=Path)
    write_plan.add_argument("--seed", required=True, type=int)
    write_plan.add_argument("--source-commit", required=True)
    write_plan.add_argument("--mga-plan", required=True, type=Path)
    write_plan.add_argument("--heterogeneity-plan", required=True, type=Path)
    write_plan.add_argument("--cargo-lock", required=True, type=Path)
    write_plan.add_argument("--build-attempt", required=True, type=Path)
    write_plan.add_argument("--cargo-executable", required=True, type=Path)
    write_plan.add_argument("--mga-executable", required=True, type=Path)
    write_plan.add_argument("--heterogeneity-executable", required=True, type=Path)
    write_plan.add_argument("--maximum-executable", required=True, type=Path)

    validate = commands.add_parser("validate-plan")
    validate.add_argument("--plan", required=True, type=Path)

    for name in ("verify", "seal"):
        command = commands.add_parser(name)
        command.add_argument("--plan", required=True, type=Path)
        command.add_argument("--work-root", required=True, type=Path)
        command.add_argument("--receipt-root", required=True, type=Path)
        command.add_argument("--stage-id", required=True)
        if name == "seal":
            command.add_argument("--temporary-measurement", required=True, type=Path)

    aggregate = commands.add_parser("aggregate")
    aggregate.add_argument("--plan", required=True, type=Path)
    aggregate.add_argument("--work-root", required=True, type=Path)
    aggregate.add_argument("--receipt-root", required=True, type=Path)
    aggregate.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    arguments = parse_args()
    try:
        if arguments.command == "write-plan":
            atomic_json(arguments.output, build_plan(arguments))
        elif arguments.command == "validate-plan":
            validated_plan(arguments.plan)
        elif arguments.command == "verify":
            verify_checkpoint(
                arguments.plan,
                arguments.work_root,
                arguments.receipt_root,
                arguments.stage_id,
            )
        elif arguments.command == "seal":
            seal_checkpoint(arguments)
        elif arguments.command == "aggregate":
            atomic_json(
                arguments.output,
                build_aggregate(
                    arguments.plan, arguments.work_root, arguments.receipt_root
                ),
            )
        else:  # pragma: no cover - argparse freezes the command inventory.
            raise ContractError(f"unknown command {arguments.command}")
    except MissingCheckpointError as error:
        print(error)
        return 3
    except (ContractError, OSError, json.JSONDecodeError, KeyError, TypeError) as error:
        print(f"performance topology contract failed: {error}")
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

