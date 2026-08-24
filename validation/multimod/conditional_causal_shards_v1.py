#!/usr/bin/env python3
"""Atomic checkpoint transport for conditional and causal raw qualification."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import tempfile
from pathlib import Path
from typing import Any


SCHEMA_VERSION = 1
PLAN_SUITE_ID = "qpls.multimod.raw-qualification.shard-plan.v1"
SHARD_SUITE_ID = "qpls.multimod.raw-qualification.shard.v1"
RECEIPT_SUITE_ID = "qpls.multimod.raw-qualification.shard-receipt.v1"
PRODUCER_IDS = {
    "conditional": "qpls.multimod.conditional.raw-qualification.v1",
    "causal": "qpls.multimod.interventional.raw-qualification.v1",
}


class ContractError(ValueError):
    """A shard plan, result, or receipt is not admissible."""


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


def spec(
    shard_id: str,
    payload_kind: str,
    dependencies: list[str],
    resource_class: str,
    scientific_identity: dict[str, Any],
) -> dict[str, Any]:
    return {
        "shard_id": shard_id,
        "payload_kind": payload_kind,
        "dependencies": dependencies,
        "resource_class": resource_class,
        "parallel_safe_after_build": True,
        "scientific_identity": scientific_identity,
    }


def case_identity(
    case_id: str,
    alternative: str | None = None,
    fixture_role: str | None = None,
) -> dict[str, Any]:
    value: dict[str, Any] = {"identity_kind": "case", "case_id": case_id}
    if alternative is not None:
        value["alternative"] = alternative
    if fixture_role is not None:
        value["fixture_role"] = fixture_role
    return value


def expected_specs(family: str) -> list[dict[str, Any]]:
    rows = [
        spec(
            "sentinel",
            "sentinel",
            [],
            "sentinel",
            {
                "identity_kind": "sentinel",
                "identity_id": "fast-root-development",
            },
        )
    ]
    if family == "conditional":
        rows.append(
            spec(
                "qualification-guards",
                "evidence",
                ["sentinel"],
                "light",
                {
                    "identity_kind": "evidence",
                    "identity_id": "conditional-qualification-guards-v1",
                },
            )
        )
        for prefix in (
            "multi-path",
            "bca-non-null",
            "bca-null",
            "studentized",
            "three-way",
        ):
            for alternative in ("two_sided", "less", "greater"):
                if prefix == "multi-path":
                    case_id = f"multi_path_percentile:{alternative}"
                    fixture_role = "non_null"
                elif prefix == "bca-non-null":
                    case_id = f"bca:non_null:{alternative}"
                    fixture_role = "non_null"
                elif prefix == "bca-null":
                    case_id = f"bca:null:{alternative}"
                    fixture_role = "null"
                elif prefix == "studentized":
                    case_id = f"studentized:non_null:{alternative}"
                    fixture_role = "non_null"
                else:
                    case_id = f"bounded_three_way:{alternative}"
                    fixture_role = "non_null"
                rows.append(
                    spec(
                        f"{prefix}-{alternative}",
                        "case",
                        ["sentinel"],
                        "heavy",
                        case_identity(case_id, alternative, fixture_role),
                    )
                )
        fixed_cases = {
            "hoc": "multiple_hoc:four_disjoint",
            "grouped": "grouped:stratified",
            "case-weighted": "case_weighted:positive",
            "frequency-weighted": "frequency_weighted:count_space_and_physical_expansion",
        }
        for shard_id, case_id in fixed_cases.items():
            rows.append(
                spec(
                    shard_id,
                    "case",
                    ["sentinel"],
                    "heavy",
                    case_identity(case_id, "two_sided", "non_null"),
                )
            )
    elif family == "causal":
        rows.append(
            spec(
                "assumption-scope-guards",
                "evidence",
                ["sentinel"],
                "light",
                {
                    "identity_kind": "evidence",
                    "identity_id": "causal-assumption-scope-guards-v1",
                },
            )
        )
        fixed_cases = {
            "binary-two-edge": "binary_two_edge_path",
            "binary-four-edge": "binary_four_edge_path",
            "continuous-three-edge": "continuous_three_edge_path",
        }
        for shard_id, case_id in fixed_cases.items():
            rows.append(
                spec(
                    shard_id,
                    "case",
                    ["sentinel"],
                    "heavy",
                    case_identity(case_id),
                )
            )
    else:
        raise ContractError(f"unknown family {family}")
    return rows


def validated_plan(
    path: Path, expected_family: str | None = None
) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    plan = read_json(path)
    family = plan.get("family")
    if (
        plan.get("schema_version") != SCHEMA_VERSION
        or plan.get("suite_id") != PLAN_SUITE_ID
        or family not in PRODUCER_IDS
        or (expected_family is not None and family != expected_family)
        or plan.get("producer_id") != PRODUCER_IDS.get(family)
        or plan.get("scale") not in {"development", "qualification"}
        or plan.get("seed") != 42
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
        or plan.get("execution_contract")
        != "one_cargo_build_then_exact_resumable_case_shards"
        or plan.get("sentinel_shard_id") != "sentinel"
        or plan.get("aggregation_order") != "plan_order"
        or plan.get("shards") != expected_specs(family)
    ):
        raise ContractError("conditional/causal shard plan identity is invalid")
    rows = plan["shards"]
    by_id: dict[str, dict[str, Any]] = {}
    completed: set[str] = set()
    for position, row in enumerate(rows):
        shard_id = row["shard_id"]
        dependencies = row["dependencies"]
        if shard_id in by_id or any(value not in completed for value in dependencies):
            raise ContractError(f"shard plan row {position} is duplicate or not topological")
        by_id[shard_id] = row
        completed.add(shard_id)
    return plan, by_id


def result_path(shard_dir: Path, shard_id: str) -> Path:
    return shard_dir / f"{shard_id}.json"


def receipt_path(shard_dir: Path, shard_id: str) -> Path:
    return shard_dir / f"{shard_id}.receipt.json"


def validate_result(
    value: dict[str, Any], plan: dict[str, Any], row: dict[str, Any]
) -> None:
    payload = value.get("payload")
    if (
        value.get("schema_version") != SCHEMA_VERSION
        or value.get("suite_id") != SHARD_SUITE_ID
        or value.get("family") != plan["family"]
        or value.get("producer_id") != plan["producer_id"]
        or value.get("shard_id") != row["shard_id"]
        or value.get("scale") != plan["scale"]
        or value.get("seed") != plan["seed"]
        or value.get("metamorphism") != plan["metamorphism"]
        or value.get("sign_columns") != plan.get("sign_columns")
        or value.get("workers") != plan["workers"]
        or value.get("dependency_shard_ids") != sorted(row["dependencies"])
        or value.get("scientific_identity") != row["scientific_identity"]
        or not isinstance(payload, dict)
        or payload.get("kind") != row["payload_kind"]
        or "value" not in payload
    ):
        raise ContractError(f"shard result identity is invalid for {row['shard_id']}")
    payload_value = payload["value"]
    if not isinstance(payload_value, dict):
        raise ContractError(f"shard payload is invalid for {row['shard_id']}")
    scientific_identity = row["scientific_identity"]
    identity_kind = scientific_identity["identity_kind"]
    if identity_kind == "sentinel":
        valid_payload_identity = (
            payload_value.get("sentinel_id") == scientific_identity["identity_id"]
        )
    elif identity_kind == "evidence":
        valid_payload_identity = (
            payload_value.get("evidence_id") == scientific_identity["identity_id"]
        )
    elif identity_kind == "case":
        valid_payload_identity = payload_value.get("case_id") == scientific_identity["case_id"]
        expected_role = scientific_identity.get("fixture_role")
        if expected_role is not None:
            valid_payload_identity = (
                valid_payload_identity
                and payload_value.get("fixture_role") == expected_role
            )
        expected_alternative = scientific_identity.get("alternative")
        if expected_alternative is not None:
            alternatives: set[str] = set()

            def collect_alternatives(candidate: Any) -> None:
                if isinstance(candidate, dict):
                    alternative = candidate.get("alternative")
                    if isinstance(alternative, str):
                        alternatives.add(alternative)
                    for child in candidate.values():
                        collect_alternatives(child)
                elif isinstance(candidate, list):
                    for child in candidate:
                        collect_alternatives(child)

            collect_alternatives(payload_value)
            valid_payload_identity = (
                valid_payload_identity and alternatives == {expected_alternative}
            )
    else:  # expected_specs is frozen, so an unknown kind is always invalid.
        valid_payload_identity = False
    if not valid_payload_identity:
        raise ContractError(
            f"shard scientific payload identity is invalid for {row['shard_id']}"
        )


def dependency_inventory(
    plan_path: Path,
    shard_dir: Path,
    row: dict[str, Any],
    executable_sha256: str,
    source_commit: str,
) -> list[dict[str, str]]:
    values = []
    for dependency_id in row["dependencies"]:
        verify_checkpoint(
            plan_path, shard_dir, dependency_id, executable_sha256, source_commit
        )
        values.append(
            {
                "shard_id": dependency_id,
                "receipt_sha256": sha256_file(receipt_path(shard_dir, dependency_id)),
                "result_sha256": sha256_file(result_path(shard_dir, dependency_id)),
            }
        )
    return values


def expected_receipt(
    plan_path: Path,
    shard_dir: Path,
    plan: dict[str, Any],
    row: dict[str, Any],
    executable_sha256: str,
    source_commit: str,
    result_sha256: str,
) -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "suite_id": RECEIPT_SUITE_ID,
        "status": "passed",
        "family": plan["family"],
        "shard_id": row["shard_id"],
        "payload_kind": row["payload_kind"],
        "scale": plan["scale"],
        "seed": plan["seed"],
        "plan_sha256": sha256_file(plan_path),
        "producer_executable_sha256": executable_sha256,
        "source_commit": source_commit,
        "result_sha256": result_sha256,
        "dependency_receipts": dependency_inventory(
            plan_path, shard_dir, row, executable_sha256, source_commit
        ),
    }


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
        raise ContractError("executable SHA-256 and exact source commit are required")
    result_file = result_path(shard_dir, shard_id)
    receipt_file = receipt_path(shard_dir, shard_id)
    if not result_file.is_file() or not receipt_file.is_file():
        raise ContractError(f"checkpoint files are incomplete for {shard_id}")
    result = read_json(result_file)
    validate_result(result, plan, by_id[shard_id])
    expected = expected_receipt(
        plan_path,
        shard_dir,
        plan,
        by_id[shard_id],
        executable_sha256,
        source_commit,
        sha256_file(result_file),
    )
    if read_json(receipt_file) != expected:
        raise ContractError(f"checkpoint receipt is stale or altered for {shard_id}")


def seal(arguments: argparse.Namespace) -> None:
    plan, by_id = validated_plan(arguments.plan, arguments.family)
    if arguments.shard_id not in by_id:
        raise ContractError(f"unknown shard id {arguments.shard_id}")
    if not is_sha256(arguments.executable_sha256) or not is_git_commit(arguments.source_commit):
        raise ContractError("executable SHA-256 and exact source commit are required")
    value = read_json(arguments.temporary_result)
    row = by_id[arguments.shard_id]
    validate_result(value, plan, row)
    arguments.shard_dir.mkdir(parents=True, exist_ok=True)
    destination = result_path(arguments.shard_dir, arguments.shard_id)
    os.replace(arguments.temporary_result, destination)
    receipt = expected_receipt(
        arguments.plan,
        arguments.shard_dir,
        plan,
        row,
        arguments.executable_sha256,
        arguments.source_commit,
        sha256_file(destination),
    )
    atomic_json(receipt_path(arguments.shard_dir, arguments.shard_id), receipt)


def record_failure(arguments: argparse.Namespace) -> None:
    plan, by_id = validated_plan(arguments.plan, arguments.family)
    if arguments.shard_id not in by_id:
        raise ContractError(f"unknown shard id {arguments.shard_id}")
    if not is_sha256(arguments.executable_sha256) or not is_git_commit(arguments.source_commit):
        raise ContractError("executable SHA-256 and exact source commit are required")
    value = {
        "schema_version": SCHEMA_VERSION,
        "suite_id": RECEIPT_SUITE_ID,
        "status": "failed",
        "family": arguments.family,
        "shard_id": arguments.shard_id,
        "scale": plan["scale"],
        "seed": plan["seed"],
        "plan_sha256": sha256_file(arguments.plan),
        "producer_executable_sha256": arguments.executable_sha256,
        "source_commit": arguments.source_commit,
        "exit_code": arguments.exit_code,
        "failure_reason": arguments.failure_reason,
        "stdout_sha256": sha256_file(arguments.stdout) if arguments.stdout.is_file() else None,
        "stderr_sha256": sha256_file(arguments.stderr) if arguments.stderr.is_file() else None,
    }
    atomic_json(arguments.failure_receipt, value)


def build_aggregate_report(
    family: str,
    plan_path: Path,
    shard_dir: Path,
    executable_sha256: str,
    source_commit: str,
) -> dict[str, Any]:
    plan, _ = validated_plan(plan_path, family)
    for row in plan["shards"]:
        verify_checkpoint(
            plan_path,
            shard_dir,
            row["shard_id"],
            executable_sha256,
            source_commit,
        )
    sentinel = read_json(result_path(shard_dir, "sentinel"))["payload"]["value"]
    header = sentinel.get("header")
    if not isinstance(header, dict):
        raise ContractError("sentinel header is absent")
    expected_family = (
        "conditional_process_v2"
        if family == "conditional"
        else "interventional_causal_mediation_v1"
    )
    if (
        header.get("producer_id") != plan["producer_id"]
        or header.get("family") != expected_family
        or header.get("scale") != plan["scale"]
        or header.get("seed") != plan["seed"]
        or header.get("metamorphism") != plan["metamorphism"]
        or header.get("sign_columns") != plan.get("sign_columns")
        or header.get("workers") != plan["workers"]
    ):
        raise ContractError("sentinel header is not bound to the exact shard plan")
    report = dict(header)
    cases = []
    for row in plan["shards"]:
        if row["payload_kind"] != "case":
            continue
        case = dict(read_json(result_path(shard_dir, row["shard_id"]))["payload"]["value"])
        if "qualification_shard_id" in case or "qualification_scientific_identity" in case:
            raise ContractError(
                f"case payload reserves qualification provenance keys for {row['shard_id']}"
            )
        case["qualification_shard_id"] = row["shard_id"]
        case["qualification_scientific_identity"] = row["scientific_identity"]
        cases.append(case)
    report["cases"] = cases
    if family == "conditional":
        evidence = read_json(result_path(shard_dir, "qualification-guards"))["payload"][
            "value"
        ]
        report["qualification_boundary_guards"] = evidence[
            "qualification_boundary_guards"
        ]
        report["unsupported_intersections"] = evidence["unsupported_intersections"]
    else:
        evidence = read_json(result_path(shard_dir, "assumption-scope-guards"))[
            "payload"
        ]["value"]
        report["assumption_and_scope_blockers"] = evidence["assumption_and_scope_blockers"]
    report["shard_execution_receipt"] = {
        "schema_version": SCHEMA_VERSION,
        "family": family,
        "plan_sha256": sha256_file(plan_path),
        "producer_executable_sha256": executable_sha256,
        "source_commit": source_commit,
        "shards": [
            {
                "shard_id": row["shard_id"],
                "receipt_sha256": sha256_file(receipt_path(shard_dir, row["shard_id"])),
                "result_sha256": sha256_file(result_path(shard_dir, row["shard_id"])),
            }
            for row in plan["shards"]
        ],
    }
    return report


def aggregate(arguments: argparse.Namespace) -> None:
    report = build_aggregate_report(
        arguments.family,
        arguments.plan,
        arguments.shard_dir,
        arguments.executable_sha256,
        arguments.source_commit,
    )
    atomic_json(arguments.output, report)


def identity_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--family", choices=("conditional", "causal"), required=True)
    parser.add_argument("--plan", type=Path, required=True)
    parser.add_argument("--shard-dir", type=Path, required=True)
    parser.add_argument("--executable-sha256", required=True)
    parser.add_argument("--source-commit", required=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="command", required=True)
    verify = commands.add_parser("verify")
    identity_arguments(verify)
    verify.add_argument("--shard-id", required=True)
    seal_parser = commands.add_parser("seal")
    identity_arguments(seal_parser)
    seal_parser.add_argument("--shard-id", required=True)
    seal_parser.add_argument("--temporary-result", type=Path, required=True)
    failure = commands.add_parser("record-failure")
    identity_arguments(failure)
    failure.add_argument("--shard-id", required=True)
    failure.add_argument("--failure-receipt", type=Path, required=True)
    failure.add_argument("--exit-code", type=int, required=True)
    failure.add_argument("--failure-reason", required=True)
    failure.add_argument("--stdout", type=Path, required=True)
    failure.add_argument("--stderr", type=Path, required=True)
    aggregate_parser = commands.add_parser("aggregate")
    identity_arguments(aggregate_parser)
    aggregate_parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    arguments = parse_args()
    try:
        if arguments.command == "verify":
            validated_plan(arguments.plan, arguments.family)
            verify_checkpoint(
                arguments.plan,
                arguments.shard_dir,
                arguments.shard_id,
                arguments.executable_sha256,
                arguments.source_commit,
            )
        elif arguments.command == "seal":
            seal(arguments)
        elif arguments.command == "record-failure":
            record_failure(arguments)
        elif arguments.command == "aggregate":
            aggregate(arguments)
        else:  # pragma: no cover
            raise ContractError(f"unsupported command {arguments.command}")
    except (ContractError, OSError, json.JSONDecodeError, KeyError, TypeError) as error:
        print(f"conditional/causal shard contract failed: {error}", file=os.sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
