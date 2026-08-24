#!/usr/bin/env python3
"""Verify and aggregate resumable production MGA qualification cells.

The Rust producer remains the sole scientific authority and persists its
``MgaExecutionCacheV1`` after each immutable production shard. This helper
owns only transport integrity: exact inventory validation, atomic completed
cell publication, SHA-256 binding, and lossless reconstruction of the legacy
raw receipt consumed by the independent comparator.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import tempfile
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
PRODUCER_SUITE_ID = "qpls.multimod.mga.production-qualification.v1"
PLAN_SUITE_ID = "qpls.multimod.mga.qualification-cell-plan.v1"
CELL_RESULT_SUITE_ID = "qpls.multimod.mga.qualification-cell-result.v1"
CELL_RECEIPT_SUITE_ID = "qpls.multimod.mga.qualification-cell-receipt.v1"
CACHE_CHECKPOINT_SUITE_ID = (
    "qpls.multimod.mga.production-shard-cache-checkpoint.v1"
)
AGGREGATION_RECEIPT_ID = "qpls.multimod.mga.qualification-cell-aggregation.v1"
PERMUTATIONS = 5_000
BOOTSTRAPS = 5_000
BASELINE_ENVIRONMENT_CONTRACT = (
    "qpls.multimod.mga.baseline-environment.v1\n"
    "QPLS_MULTIMOD_METAMORPHISM_V1=baseline\n"
    "QPLS_MULTIMOD_WORKERS_V1=1\n"
    "QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1=unset\n"
    "QPLS_MULTIMOD_SIGN_COLUMNS_V1=unset\n"
)
REQUIRED_PROFILE_IDS = [
    "mga.general_sem_pls.v1",
    "mga.multiple_two_way_moderation.v1",
    "mga.bounded_three_way_moderation.v1",
    "mga.bounded_two_way_moderated_mediation.v1",
    "mga.multiple_nonnested_hoc.v1",
    "mga.case_weighted_pls.v1",
    "mga.frequency_weighted_pls.v1",
    "mga.reflective_plsc.v1",
]


class ContractError(ValueError):
    """A plan, completed cell, or aggregation violates the frozen contract."""


class MissingCellError(ContractError):
    """A completed cell is absent and may be executed or resumed."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_json(value: Any) -> str:
    return sha256_bytes(
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode(
            "utf-8"
        )
    )


def is_sha256(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(
        character in "0123456789abcdef" for character in value
    )


def is_git_commit(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 40 and all(
        character in "0123456789abcdef" for character in value
    )


def expected_cell_specs(scale: str) -> list[dict[str, Any]]:
    if scale not in {"development", "qualification"}:
        raise ContractError(f"unknown MGA qualification scale {scale!r}")

    def row(cell_id: str, payload_kind: str, cache: bool = True) -> dict[str, Any]:
        return {
            "cell_id": cell_id,
            "payload_kind": payload_kind,
            "production_cache_required": cache,
        }

    rows = [
        row("mga-general-2-groups", "group_matrix_cell"),
        row("mga-general-3-groups", "group_matrix_cell"),
    ]
    if scale == "qualification":
        rows.extend(
            [
                row("mga-general-5-groups", "group_matrix_cell"),
                row("mga-general-20-groups", "group_matrix_cell"),
            ]
        )
    rows.append(row("mga-profile-multiple_two_way", "profile_matrix_cell"))
    if scale == "qualification":
        rows.extend(
            [
                row("mga-profile-bounded_three_way", "profile_matrix_cell"),
                row(
                    "mga-profile-bounded_two_way_moderated_mediation",
                    "profile_matrix_cell",
                ),
                row("mga-profile-multiple_nonnested_hoc", "profile_matrix_cell"),
                row("mga-profile-case_weighted_pls", "profile_matrix_cell"),
            ]
        )
    rows.extend(
        [
            row("mga-profile-reflective_plsc", "profile_matrix_cell"),
            row("mga-general-parametric-3-groups", "parametric_sensitivity"),
            row("mga-frequency-compact", "frequency_compact"),
            row("mga-frequency-physically-expanded", "frequency_expanded"),
            row("mga-label-forward", "label_forward"),
            row("mga-label-reverse", "label_reverse"),
            row("mga-boundaries", "boundaries", False),
        ]
    )
    return rows


def expected_plan(scale: str, seed: int) -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "suite_id": PLAN_SUITE_ID,
        "producer_suite_id": PRODUCER_SUITE_ID,
        "scale": scale,
        "seed": seed,
        "metamorphism": "baseline",
        "workers": 1,
        "permutation_samples": PERMUTATIONS,
        "bootstrap_samples": BOOTSTRAPS,
        "baseline_environment_sha256": sha256_bytes(
            BASELINE_ENVIRONMENT_CONTRACT.encode("utf-8")
        ),
        "execution_contract": (
            "one_build_then_parallel_resumable_cells_over_production_mga_shards"
        ),
        "root_sentinel_cell_id": "mga-general-2-groups",
        "aggregation_order": "exact_plan_order",
        "cells": expected_cell_specs(scale),
    }


def read_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ContractError(f"could not read JSON object {path}: {error}") from error
    if not isinstance(value, dict):
        raise ContractError(f"{path} must contain one JSON object")
    return value


def validated_plan(path: Path) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    plan = read_object(path)
    scale = plan.get("scale")
    seed = plan.get("seed")
    if scale not in {"development", "qualification"} or not isinstance(seed, int):
        raise ContractError("MGA qualification plan scale and seed are invalid")
    if scale == "qualification" and seed != 42:
        raise ContractError("qualification-scale MGA plan must use seed 42")
    if plan != expected_plan(scale, seed):
        raise ContractError("MGA qualification plan differs from the frozen exact inventory")
    by_id = {row["cell_id"]: row for row in plan["cells"]}
    if len(by_id) != len(plan["cells"]):
        raise ContractError("MGA qualification plan contains duplicate cell identities")
    return plan, by_id


def result_path(cell_dir: Path, cell_id: str) -> Path:
    return cell_dir / cell_id / "result.json"


def receipt_path(cell_dir: Path, cell_id: str) -> Path:
    return cell_dir / cell_id / "receipt.json"


def _validate_cache_binding(
    binding: Any,
    *,
    cell_id: str,
    result: dict[str, Any],
    plan_sha256: str,
) -> None:
    if not isinstance(binding, dict):
        raise ContractError(f"{cell_id} cache binding must be an object")
    required_keys = {
        "suite_id",
        "orchestration_cell_id",
        "subfit_id",
        "source_commit",
        "executable_sha256",
        "qualification_plan_sha256",
        "environment_sha256",
        "scale",
        "seed",
        "production_plan_sha256",
        "cache_sha256",
        "finalized_cache_sha256",
        "loaded_completed_shards",
        "completed_shards",
        "planned_shards",
    }
    if set(binding) != required_keys:
        raise ContractError(f"{cell_id} cache binding schema is not exact")
    if (
        binding.get("suite_id") != CACHE_CHECKPOINT_SUITE_ID
        or binding.get("orchestration_cell_id") != cell_id
        or binding.get("subfit_id") != cell_id
        or binding.get("source_commit") != result["source_commit"]
        or binding.get("executable_sha256") != result["executable_sha256"]
        or binding.get("qualification_plan_sha256") != plan_sha256
        or binding.get("environment_sha256") != result["environment_sha256"]
        or binding.get("scale") != result["scale"]
        or binding.get("seed") != result["seed"]
        or not is_sha256(binding.get("production_plan_sha256"))
        or not is_sha256(binding.get("cache_sha256"))
        or not is_sha256(binding.get("finalized_cache_sha256"))
        or not isinstance(binding.get("loaded_completed_shards"), int)
        or not isinstance(binding.get("completed_shards"), int)
        or not isinstance(binding.get("planned_shards"), int)
        or binding["loaded_completed_shards"] < 0
        or binding["loaded_completed_shards"] > binding["completed_shards"]
        or binding["completed_shards"] != binding["planned_shards"]
        or binding["planned_shards"] <= 0
    ):
        raise ContractError(f"{cell_id} cache binding identity or completion is invalid")


def validate_cell_result(
    path: Path,
    plan_path: Path,
    cell_id: str,
    executable_sha256: str,
    source_commit: str,
    environment_sha256: str,
) -> dict[str, Any]:
    plan, by_id = validated_plan(plan_path)
    if cell_id not in by_id:
        raise ContractError(f"cell {cell_id} is outside the exact MGA inventory")
    if not is_sha256(executable_sha256) or not is_git_commit(source_commit):
        raise ContractError("executable and source identities are malformed")
    if environment_sha256 != plan["baseline_environment_sha256"]:
        raise ContractError("baseline environment identity is invalid")
    if not path.is_file():
        raise MissingCellError(f"completed MGA cell is absent: {cell_id}")
    result = read_object(path)
    required_keys = {
        "schema_version",
        "suite_id",
        "producer_suite_id",
        "cell_id",
        "payload_kind",
        "scale",
        "seed",
        "metamorphism",
        "workers",
        "source_commit",
        "executable_sha256",
        "qualification_plan_sha256",
        "environment_sha256",
        "permutation_samples",
        "bootstrap_samples",
        "cache_bindings",
        "payload",
    }
    spec = by_id[cell_id]
    plan_sha256 = sha256_file(plan_path)
    if set(result) != required_keys:
        raise ContractError(f"{cell_id} result schema is not exact")
    if (
        result.get("schema_version") != SCHEMA_VERSION
        or result.get("suite_id") != CELL_RESULT_SUITE_ID
        or result.get("producer_suite_id") != PRODUCER_SUITE_ID
        or result.get("cell_id") != cell_id
        or result.get("payload_kind") != spec["payload_kind"]
        or result.get("scale") != plan["scale"]
        or result.get("seed") != plan["seed"]
        or result.get("metamorphism") != "baseline"
        or result.get("workers") != 1
        or result.get("source_commit") != source_commit
        or result.get("executable_sha256") != executable_sha256
        or result.get("qualification_plan_sha256") != plan_sha256
        or result.get("environment_sha256") != environment_sha256
        or result.get("permutation_samples") != PERMUTATIONS
        or result.get("bootstrap_samples") != BOOTSTRAPS
        or not isinstance(result.get("payload"), dict)
        or not isinstance(result.get("cache_bindings"), list)
    ):
        raise ContractError(f"{cell_id} result identity is invalid")
    bindings = result["cache_bindings"]
    expected_binding_count = 1 if spec["production_cache_required"] else 0
    if len(bindings) != expected_binding_count:
        raise ContractError(f"{cell_id} completed cache inventory is invalid")
    for binding in bindings:
        _validate_cache_binding(
            binding, cell_id=cell_id, result=result, plan_sha256=plan_sha256
        )
    if spec["production_cache_required"]:
        payload = result["payload"]
        if payload.get("cell_id") != cell_id or payload.get("external_cache_binding") != bindings[0]:
            raise ContractError(f"{cell_id} payload/cache identity is inconsistent")
    return result


def validate_completed_cell(
    plan_path: Path,
    cell_dir: Path,
    cell_id: str,
    executable_sha256: str,
    source_commit: str,
    environment_sha256: str,
) -> dict[str, Any]:
    directory = cell_dir / cell_id
    result_file = result_path(cell_dir, cell_id)
    receipt_file = receipt_path(cell_dir, cell_id)
    if not directory.exists():
        raise MissingCellError(f"completed MGA cell is absent: {cell_id}")
    if not directory.is_dir() or sorted(path.name for path in directory.iterdir()) != [
        "receipt.json",
        "result.json",
    ]:
        raise ContractError(f"completed MGA cell transaction is not exact: {cell_id}")
    result = validate_cell_result(
        result_file,
        plan_path,
        cell_id,
        executable_sha256,
        source_commit,
        environment_sha256,
    )
    receipt = read_object(receipt_file)
    expected = {
        "schema_version": SCHEMA_VERSION,
        "suite_id": CELL_RECEIPT_SUITE_ID,
        "cell_id": cell_id,
        "source_commit": source_commit,
        "executable_sha256": executable_sha256,
        "qualification_plan_sha256": sha256_file(plan_path),
        "environment_sha256": environment_sha256,
        "scale": result["scale"],
        "seed": result["seed"],
        "result_sha256": sha256_file(result_file),
        "cache_bindings_sha256": sha256_json(result["cache_bindings"]),
    }
    if receipt != expected:
        raise ContractError(f"completed MGA cell receipt is stale or tampered: {cell_id}")
    return result


def seal_cell(
    plan_path: Path,
    cell_dir: Path,
    cell_id: str,
    temporary_result: Path,
    executable_sha256: str,
    source_commit: str,
    environment_sha256: str,
) -> None:
    destination = cell_dir / cell_id
    if destination.exists():
        raise ContractError(f"duplicate completed MGA cell rejected: {cell_id}")
    result = validate_cell_result(
        temporary_result,
        plan_path,
        cell_id,
        executable_sha256,
        source_commit,
        environment_sha256,
    )
    cell_dir.mkdir(parents=True, exist_ok=True)
    temporary_directory = Path(
        tempfile.mkdtemp(dir=cell_dir, prefix=f".{cell_id}.", suffix=".tmp")
    )
    try:
        sealed_result = temporary_directory / "result.json"
        os.replace(temporary_result, sealed_result)
        receipt = {
            "schema_version": SCHEMA_VERSION,
            "suite_id": CELL_RECEIPT_SUITE_ID,
            "cell_id": cell_id,
            "source_commit": source_commit,
            "executable_sha256": executable_sha256,
            "qualification_plan_sha256": sha256_file(plan_path),
            "environment_sha256": environment_sha256,
            "scale": result["scale"],
            "seed": result["seed"],
            "result_sha256": sha256_file(sealed_result),
            "cache_bindings_sha256": sha256_json(result["cache_bindings"]),
        }
        atomic_json(temporary_directory / "receipt.json", receipt)
        os.rename(temporary_directory, destination)
    except Exception:
        # A dot-prefixed transaction directory is intentionally unpublishable.
        # It is retained for diagnosis; a later exact run creates a fresh one.
        raise


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


def aggregate(
    plan_path: Path,
    cell_dir: Path,
    executable_sha256: str,
    source_commit: str,
    environment_sha256: str,
    output: Path,
) -> None:
    plan, _ = validated_plan(plan_path)
    specs = plan["cells"]
    expected_ids = [row["cell_id"] for row in specs]
    actual_ids = sorted(
        path.name
        for path in cell_dir.iterdir()
        if not path.name.startswith(".")
    )
    if actual_ids != sorted(expected_ids):
        raise ContractError(
            "completed MGA cell directory differs from exact inventory: "
            f"expected={sorted(expected_ids)!r}, actual={actual_ids!r}"
        )
    cells: dict[str, dict[str, Any]] = {}
    receipts: list[dict[str, Any]] = []
    for spec in specs:
        cell_id = spec["cell_id"]
        path = result_path(cell_dir, cell_id)
        result = validate_completed_cell(
            plan_path,
            cell_dir,
            cell_id,
            executable_sha256,
            source_commit,
            environment_sha256,
        )
        cells[cell_id] = result
        receipts.append(
            {
                "cell_id": cell_id,
                "payload_kind": spec["payload_kind"],
                "result_sha256": sha256_file(path),
                "cache_bindings_sha256": sha256_json(result["cache_bindings"]),
            }
        )

    def payload(cell_id: str) -> dict[str, Any]:
        return cells[cell_id]["payload"]

    group_matrix = [
        payload(row["cell_id"])
        for row in specs
        if row["payload_kind"] == "group_matrix_cell"
    ]
    profile_matrix = [
        payload(row["cell_id"])
        for row in specs
        if row["payload_kind"] == "profile_matrix_cell"
    ]
    compact = payload("mga-frequency-compact")
    expanded = payload("mga-frequency-physically-expanded")
    represented_rows = expanded.get("dataset_rows")
    if not isinstance(represented_rows, int) or represented_rows <= 0:
        raise ContractError("expanded frequency cell omitted represented row count")
    report = {
        "schema_version": SCHEMA_VERSION,
        "suite_id": PRODUCER_SUITE_ID,
        "scale": plan["scale"],
        "seed": plan["seed"],
        "metamorphism": "baseline",
        "workers": 1,
        "execution_contract": "public_recipe_v4_compiler_plus_raw_resumable_mga_runner",
        "qualification_claim": "raw_sut_facts_for_independent_comparison_only",
        "required_profile_ids": REQUIRED_PROFILE_IDS,
        "group_matrix": group_matrix,
        "parametric_sensitivity": payload("mga-general-parametric-3-groups"),
        "profile_matrix": profile_matrix,
        "frequency_expansion": {
            "compact_frequency_run": compact,
            "physically_expanded_unweighted_run": expanded,
            "compact_source_rows": 30,
            "represented_rows": represented_rows,
            "equivalence_oracle": "production_count_space_vs_physical_expansion_v1",
        },
        "label_reversal": {
            "forward": payload("mga-label-forward"),
            "reverse": payload("mga-label-reverse"),
        },
        "boundaries": payload("mga-boundaries"),
        "qualification_cell_aggregation": {
            "receipt_id": AGGREGATION_RECEIPT_ID,
            "source_commit": source_commit,
            "executable_sha256": executable_sha256,
            "qualification_plan_sha256": sha256_file(plan_path),
            "environment_sha256": environment_sha256,
            "cell_count": len(receipts),
            "aggregation_order": expected_ids,
            "cell_receipts": receipts,
        },
    }
    atomic_json(output, report)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    for name in ("validate-plan", "verify", "seal", "aggregate"):
        command = subparsers.add_parser(name)
        command.add_argument("--plan", required=True, type=Path)
        if name != "validate-plan":
            command.add_argument("--cell-dir", required=True, type=Path)
            command.add_argument("--executable-sha256", required=True)
            command.add_argument("--source-commit", required=True)
            command.add_argument("--environment-sha256", required=True)
        if name in {"verify", "seal"}:
            command.add_argument("--cell-id", required=True)
        if name == "seal":
            command.add_argument("--temporary-result", required=True, type=Path)
        if name == "aggregate":
            command.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if args.command == "validate-plan":
            validated_plan(args.plan)
        elif args.command == "verify":
            validate_completed_cell(
                args.plan,
                args.cell_dir,
                args.cell_id,
                args.executable_sha256,
                args.source_commit,
                args.environment_sha256,
            )
        elif args.command == "seal":
            seal_cell(
                args.plan,
                args.cell_dir,
                args.cell_id,
                args.temporary_result,
                args.executable_sha256,
                args.source_commit,
                args.environment_sha256,
            )
        elif args.command == "aggregate":
            aggregate(
                args.plan,
                args.cell_dir,
                args.executable_sha256,
                args.source_commit,
                args.environment_sha256,
                args.output,
            )
        else:  # pragma: no cover - argparse freezes this inventory.
            raise ContractError(f"unknown command {args.command}")
    except MissingCellError as error:
        print(error)
        return 3
    except ContractError as error:
        print(error)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
