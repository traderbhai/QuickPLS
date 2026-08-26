#!/usr/bin/env python3
"""Receipt-bound transport for the global MultiMod metamorphic matrix.

The four Rust examples remain the sole scientific producers and the existing
metamorphic verifier remains the sole comparison authority.  This module only
defines the exact 25-cell execution graph and provides atomic, source-bound
checkpoint receipts so completed cells can be reused safely.
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
PLAN_ID = "qpls.multimod.metamorphic.cell-plan.v1"
RECEIPT_ID = "qpls.multimod.metamorphic.cell-receipt.v1"
STATUS_ID = "qpls.multimod.metamorphic.cell-status.v1"
EXECUTION_RECEIPT_ID = "qpls.multimod.metamorphic.execution-receipt.v1"
REPORT_ID = "qpls.multimod.global-metamorphic-qualification.v1"

FAMILIES: tuple[dict[str, str], ...] = (
    {
        "family": "mga",
        "example": "multimod_mga_qualification_v1",
        "identity_field": "suite_id",
        "identity": "qpls.multimod.mga.production-qualification.v1",
    },
    {
        "family": "heterogeneity",
        "example": "multimod_heterogeneity_qualification_v2",
        "identity_field": "suite_id",
        "identity": "qpls.multimod.heterogeneity.production-qualification.v2",
    },
    {
        "family": "conditional",
        "example": "multimod_conditional_qualification_v1",
        "identity_field": "producer_id",
        "identity": "qpls.multimod.conditional.raw-qualification.v1",
    },
    {
        "family": "causal",
        "example": "multimod_causal_qualification_v1",
        "identity_field": "producer_id",
        "identity": "qpls.multimod.interventional.raw-qualification.v1",
    },
)
SHARED_AXES = (
    "baseline",
    "seed_repeat",
    "row_reverse",
    "input_column_reverse",
    "declaration_reverse",
    "worker_parallel",
)


class ContractError(ValueError):
    """The persisted execution state is not admissible."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_json(value: Any) -> str:
    payload = json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def is_sha256(value: Any) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(
        character in "0123456789abcdef" for character in value
    )


def is_commit(value: Any) -> bool:
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


def expected_cells() -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    # All four scientific roots precede every dependent transformation.  This
    # keeps a bad producer root from spending time on five derivative axes.
    for family in FAMILIES:
        rows.append(cell_spec(family, "baseline"))
    for axis in SHARED_AXES[1:]:
        for family in FAMILIES:
            rows.append(cell_spec(family, axis))
    rows.append(cell_spec(FAMILIES[-1], "sign_reverse"))
    return rows


def expected_build_command() -> list[str]:
    command = [
        "cargo",
        "build",
        "--release",
        "--quiet",
        "--locked",
        "-p",
        "qpls-runner",
    ]
    for family in FAMILIES:
        command.extend(["--example", family["example"]])
    return command


def cell_spec(family: dict[str, str], axis: str) -> dict[str, Any]:
    family_id = family["family"]
    cell_id = f"{family_id}-{axis}"
    workers = 4 if axis == "worker_parallel" else 1
    # The previous orchestrator explicitly passed an empty sign-column value
    # for every non-sign axis. Retain that exact child-process setting rather
    # than inheriting an ambient value from the wrapper process.
    sign_columns = "c" if axis == "sign_reverse" else ""
    return {
        "cell_id": cell_id,
        "family": family_id,
        "example": family["example"],
        "axis": axis,
        "workers": workers,
        "sign_columns": sign_columns,
        "dependencies": [] if axis == "baseline" else [f"{family_id}-baseline"],
        "resource_class": "heavy" if axis == "worker_parallel" else "standard",
        "parallel_safe_after_build": True,
        "identity_field": family["identity_field"],
        "producer_identity": family["identity"],
        "argument_contract": [
            "--scale",
            "development",
            "--output",
            "{temporary_output}",
        ],
        "environment": {
            "QPLS_MULTIMOD_METAMORPHISM_V1": axis,
            "QPLS_MULTIMOD_WORKERS_V1": str(workers),
            "QPLS_MULTIMOD_METAMORPHIC_COMPACT_V1": "1",
            "QPLS_MULTIMOD_SIGN_COLUMNS_V1": sign_columns,
        },
    }


def plan_document() -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "plan_id": PLAN_ID,
        "execution_contract": "one_bounded_cargo_build_then_25_direct_executable_cells",
        "scientific_scale": "development",
        "scientific_settings_changed": False,
        "root_policy": "all_family_baselines_before_dependent_axes",
        "maximum_parallel_cells": 4,
        "maximum_cell_seconds": 1800,
        "maximum_scientific_seconds": 6480,
        "maximum_wrapper_seconds": 6600,
        "aggregation_order": "plan_order",
        "cells": expected_cells(),
    }


def validated_plan(path: Path) -> tuple[dict[str, Any], dict[str, dict[str, Any]]]:
    plan = read_json(path)
    expected = plan_document()
    if plan != expected:
        raise ContractError("metamorphic cell plan differs from the exact frozen contract")
    cells = {row["cell_id"]: row for row in plan["cells"]}
    if len(cells) != 25:
        raise ContractError("metamorphic plan must contain exactly 25 unique cells")
    return plan, cells


def paths_for(cell_dir: Path, cell_id: str) -> dict[str, Path]:
    work_root = cell_dir.resolve().parent
    log_dir = work_root / "logs"
    return {
        "result": cell_dir.resolve() / f"{cell_id}.json",
        "temporary": cell_dir.resolve() / f"{cell_id}.result.tmp.json",
        "receipt": cell_dir.resolve() / f"{cell_id}.receipt.json",
        "stdout": log_dir / f"{cell_id}.stdout.log",
        "stderr": log_dir / f"{cell_id}.stderr.log",
    }


def parse_binaries(values: list[str]) -> dict[str, Path]:
    binaries: dict[str, Path] = {}
    expected_families = {row["family"] for row in FAMILIES}
    for value in values:
        family, separator, raw_path = value.partition("=")
        if not separator or family in binaries or family not in expected_families:
            raise ContractError(f"invalid or duplicate --binary binding: {value}")
        path = Path(raw_path).resolve()
        if not path.is_file():
            raise ContractError(f"bound producer executable is missing: {path}")
        binaries[family] = path
    if set(binaries) != expected_families:
        raise ContractError("all four exact family executables must be bound")
    return binaries


def validate_result(path: Path, spec: dict[str, Any]) -> dict[str, Any]:
    if not path.is_file() or path.stat().st_size <= 0:
        raise ContractError(f"cell output is missing or empty: {path}")
    result = read_json(path)
    if (
        result.get(spec["identity_field"]) != spec["producer_identity"]
        or result.get("scale") != "development"
        or result.get("metamorphism") != spec["axis"]
        or result.get("workers") != spec["workers"]
    ):
        raise ContractError(f"{spec['cell_id']} producer identity or execution mapping differs")
    return result


def expected_arguments(cell_dir: Path, cell_id: str) -> list[str]:
    return [
        "--scale",
        "development",
        "--output",
        str(paths_for(cell_dir, cell_id)["temporary"]),
    ]


def validate_receipt(
    plan_path: Path,
    cell_dir: Path,
    cell_id: str,
    source_commit: str,
    binaries: dict[str, Path],
    memo: dict[str, dict[str, Any]] | None = None,
    active: set[str] | None = None,
) -> dict[str, Any]:
    _, cells = validated_plan(plan_path)
    if not is_commit(source_commit):
        raise ContractError("source commit must be an exact lowercase Git SHA")
    if cell_id not in cells:
        raise ContractError(f"unknown metamorphic cell: {cell_id}")
    memo = {} if memo is None else memo
    active = set() if active is None else active
    if cell_id in memo:
        return memo[cell_id]
    if cell_id in active:
        raise ContractError("metamorphic dependency graph is cyclic")
    active.add(cell_id)
    spec = cells[cell_id]
    for dependency in spec["dependencies"]:
        validate_receipt(
            plan_path, cell_dir, dependency, source_commit, binaries, memo, active
        )
    expected_paths = paths_for(cell_dir, cell_id)
    receipt = read_json(expected_paths["receipt"])
    binary = binaries[spec["family"]]
    arguments = expected_arguments(cell_dir, cell_id)
    expected_environment = spec["environment"]
    plan_sha256 = sha256_file(plan_path)
    if (
        receipt.get("schema_version") != SCHEMA_VERSION
        or receipt.get("receipt_id") != RECEIPT_ID
        or receipt.get("cell_id") != cell_id
        or receipt.get("family") != spec["family"]
        or receipt.get("axis") != spec["axis"]
        or receipt.get("dependencies") != spec["dependencies"]
        or receipt.get("source_commit") != source_commit
        or receipt.get("plan_sha256") != plan_sha256
        or receipt.get("executable_path") != str(binary)
        or receipt.get("executable_sha256") != sha256_file(binary)
        or receipt.get("arguments") != arguments
        or receipt.get("arguments_sha256") != sha256_json(arguments)
        or receipt.get("environment") != expected_environment
        or receipt.get("environment_sha256") != sha256_json(expected_environment)
        or receipt.get("output_path") != str(expected_paths["result"])
        or receipt.get("stdout_path") != str(expected_paths["stdout"])
        or receipt.get("stderr_path") != str(expected_paths["stderr"])
        or receipt.get("exit_code") != 0
        or not isinstance(receipt.get("elapsed_milliseconds"), int)
        or not 0 <= receipt.get("elapsed_milliseconds", -1) <= 1_800_000
    ):
        raise ContractError(f"{cell_id} receipt identity differs")
    for field, path in (
        ("output", expected_paths["result"]),
        ("stdout", expected_paths["stdout"]),
        ("stderr", expected_paths["stderr"]),
    ):
        if not path.is_file():
            raise ContractError(f"{cell_id} {field} evidence is missing")
        if (
            receipt.get(f"{field}_sha256") != sha256_file(path)
            or receipt.get(f"{field}_size") != path.stat().st_size
        ):
            raise ContractError(f"{cell_id} {field} evidence digest differs")
    validate_result(expected_paths["result"], spec)
    memo[cell_id] = receipt
    active.remove(cell_id)
    return receipt


def seal_cell(args: argparse.Namespace) -> None:
    plan_path = args.plan.resolve()
    cell_dir = args.cell_dir.resolve()
    _, cells = validated_plan(plan_path)
    if args.cell_id not in cells:
        raise ContractError(f"unknown metamorphic cell: {args.cell_id}")
    if not is_commit(args.source_commit):
        raise ContractError("source commit must be an exact lowercase Git SHA")
    spec = cells[args.cell_id]
    if not 0 <= args.elapsed_milliseconds <= 1_800_000:
        raise ContractError("cell elapsed time exceeds the exact 1,800-second cap")
    binary = args.executable.resolve()
    if not binary.is_file() or sha256_file(binary) != args.executable_sha256:
        raise ContractError("producer executable bytes differ before sealing")
    paths = paths_for(cell_dir, args.cell_id)
    if args.temporary_result.resolve() != paths["temporary"]:
        raise ContractError("temporary result path differs from the deterministic contract")
    if args.stdout.resolve() != paths["stdout"] or args.stderr.resolve() != paths["stderr"]:
        raise ContractError("cell stream paths differ from the deterministic contract")
    if paths["result"].exists() or paths["receipt"].exists():
        raise ContractError("final cell result or receipt already exists")
    arguments = list(args.argument)
    if arguments != expected_arguments(cell_dir, args.cell_id):
        raise ContractError("executed arguments differ from the exact cell contract")
    family_binaries = {row["family"]: binary for row in FAMILIES}
    for dependency in spec["dependencies"]:
        # Dependency validation needs only the current family's executable.
        dependency_spec = cells[dependency]
        family_binaries[dependency_spec["family"]] = binary
        validate_receipt(
            plan_path,
            cell_dir,
            dependency,
            args.source_commit,
            family_binaries,
        )
    validate_result(paths["temporary"], spec)
    paths["result"].parent.mkdir(parents=True, exist_ok=True)
    os.replace(paths["temporary"], paths["result"])
    environment = spec["environment"]
    receipt = {
        "schema_version": SCHEMA_VERSION,
        "receipt_id": RECEIPT_ID,
        "cell_id": args.cell_id,
        "family": spec["family"],
        "axis": spec["axis"],
        "dependencies": spec["dependencies"],
        "source_commit": args.source_commit,
        "plan_sha256": sha256_file(plan_path),
        "executable_path": str(binary),
        "executable_sha256": args.executable_sha256,
        "arguments": arguments,
        "arguments_sha256": sha256_json(arguments),
        "environment": environment,
        "environment_sha256": sha256_json(environment),
        "exit_code": 0,
        "elapsed_milliseconds": args.elapsed_milliseconds,
        "output_path": str(paths["result"]),
        "output_sha256": sha256_file(paths["result"]),
        "output_size": paths["result"].stat().st_size,
        "stdout_path": str(paths["stdout"]),
        "stdout_sha256": sha256_file(paths["stdout"]),
        "stdout_size": paths["stdout"].stat().st_size,
        "stderr_path": str(paths["stderr"]),
        "stderr_sha256": sha256_file(paths["stderr"]),
        "stderr_size": paths["stderr"].stat().st_size,
    }
    atomic_json(paths["receipt"], receipt)


def status_document(args: argparse.Namespace) -> dict[str, Any]:
    plan_path = args.plan.resolve()
    cell_dir = args.cell_dir.resolve()
    plan, _ = validated_plan(plan_path)
    binaries = parse_binaries(args.binary)
    if not is_commit(args.source_commit):
        raise ContractError("source commit must be an exact lowercase Git SHA")
    valid: list[str] = []
    invalid: list[dict[str, str]] = []
    memo: dict[str, dict[str, Any]] = {}
    for spec in plan["cells"]:
        cell_id = spec["cell_id"]
        try:
            validate_receipt(
                plan_path,
                cell_dir,
                cell_id,
                args.source_commit,
                binaries,
                memo,
            )
            valid.append(cell_id)
        except (ContractError, FileNotFoundError, json.JSONDecodeError) as error:
            invalid.append({"cell_id": cell_id, "reason": str(error)})
    return {
        "schema_version": SCHEMA_VERSION,
        "status_id": STATUS_ID,
        "source_commit": args.source_commit,
        "plan_sha256": sha256_file(plan_path),
        "binary_sha256": {
            family: sha256_file(path) for family, path in sorted(binaries.items())
        },
        "valid_cell_ids": valid,
        "invalid_cells": invalid,
        "complete": len(valid) == 25,
    }


def aggregate(args: argparse.Namespace) -> None:
    plan_path = args.plan.resolve()
    cell_dir = args.cell_dir.resolve()
    plan, _ = validated_plan(plan_path)
    binaries = parse_binaries(args.binary)
    if not is_commit(args.source_commit):
        raise ContractError("source commit must be an exact lowercase Git SHA")
    report_path = args.report.resolve()
    report = read_json(report_path)
    if report.get("report_id") != REPORT_ID or report.get("status") != "passed":
        raise ContractError("existing scientific verifier did not publish a passing report")
    receipts = []
    memo: dict[str, dict[str, Any]] = {}
    for spec in plan["cells"]:
        cell_id = spec["cell_id"]
        receipt = validate_receipt(
            plan_path,
            cell_dir,
            cell_id,
            args.source_commit,
            binaries,
            memo,
        )
        receipt_path = paths_for(cell_dir, cell_id)["receipt"]
        receipts.append(
            {
                "cell_id": cell_id,
                "family": spec["family"],
                "axis": spec["axis"],
                "receipt_path": str(receipt_path),
                "receipt_sha256": sha256_file(receipt_path),
                "output_sha256": receipt["output_sha256"],
            }
        )
    build_receipt_path = args.build_receipt.resolve()
    build_receipt = read_json(build_receipt_path)
    if (
        build_receipt.get("receipt_id")
        != "qpls.multimod.metamorphic.single-build-receipt.v1"
        or build_receipt.get("status") != "passed"
        or build_receipt.get("source_commit") != args.source_commit
        or build_receipt.get("cargo_invocation_count") != 1
        or build_receipt.get("command") != expected_build_command()
        or build_receipt.get("command_sha256")
        != sha256_json(expected_build_command())
        or build_receipt.get("exit_code") != 0
        or not isinstance(build_receipt.get("elapsed_milliseconds"), int)
        or not 0 <= build_receipt.get("elapsed_milliseconds", -1) <= 1_800_000
    ):
        raise ContractError("single-build receipt identity differs")
    expected_binary_sha = {
        family: sha256_file(path) for family, path in sorted(binaries.items())
    }
    if build_receipt.get("binary_sha256") != expected_binary_sha:
        raise ContractError("single-build receipt executable hashes differ")
    build_root = build_receipt_path.parent.resolve()
    for stream in ("stdout", "stderr"):
        raw_path = build_receipt.get(f"{stream}_path")
        if not isinstance(raw_path, str) or not raw_path:
            raise ContractError(f"single-build {stream} path is absent")
        stream_path = Path(raw_path).resolve()
        try:
            stream_path.relative_to(build_root)
        except ValueError as error:
            raise ContractError(
                f"single-build {stream} evidence escapes the stable work root"
            ) from error
        if (
            not stream_path.is_file()
            or build_receipt.get(f"{stream}_sha256") != sha256_file(stream_path)
            or build_receipt.get(f"{stream}_size") != stream_path.stat().st_size
        ):
            raise ContractError(f"single-build {stream} evidence digest differs")
    execution = {
        "schema_version": SCHEMA_VERSION,
        "receipt_id": EXECUTION_RECEIPT_ID,
        "status": "passed",
        "source_commit": args.source_commit,
        "plan_sha256": sha256_file(plan_path),
        "one_cargo_build": True,
        "cargo_build_receipt_path": str(build_receipt_path),
        "cargo_build_receipt_sha256": sha256_file(build_receipt_path),
        "maximum_parallel_cells": 4,
        "maximum_cell_seconds": 1800,
        "maximum_scientific_seconds": 6480,
        "maximum_wrapper_seconds": 6600,
        "binary_sha256": expected_binary_sha,
        "cell_count": len(receipts),
        "baseline_root_cell_ids": [f"{row['family']}-baseline" for row in FAMILIES],
        "cell_receipts": receipts,
        "scientific_report_path": str(report_path),
        "scientific_report_sha256": sha256_file(report_path),
        "scientific_report_size": report_path.stat().st_size,
        "verifier_contract_source_bound": True,
        "scientific_settings_changed": False,
    }
    atomic_json(args.output.resolve(), execution)


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser()
    commands = root.add_subparsers(dest="command", required=True)
    plan = commands.add_parser("plan")
    plan.add_argument("--output", type=Path, required=True)

    status = commands.add_parser("status")
    status.add_argument("--plan", type=Path, required=True)
    status.add_argument("--cell-dir", type=Path, required=True)
    status.add_argument("--source-commit", required=True)
    status.add_argument("--binary", action="append", default=[], required=True)
    status.add_argument("--output", type=Path, required=True)

    seal = commands.add_parser("seal")
    seal.add_argument("--plan", type=Path, required=True)
    seal.add_argument("--cell-dir", type=Path, required=True)
    seal.add_argument("--cell-id", required=True)
    seal.add_argument("--source-commit", required=True)
    seal.add_argument("--executable", type=Path, required=True)
    seal.add_argument("--executable-sha256", required=True)
    seal.add_argument("--temporary-result", type=Path, required=True)
    seal.add_argument("--stdout", type=Path, required=True)
    seal.add_argument("--stderr", type=Path, required=True)
    seal.add_argument("--elapsed-milliseconds", type=int, required=True)
    seal.add_argument("--argument", action="append", default=[], required=True)

    combine = commands.add_parser("aggregate")
    combine.add_argument("--plan", type=Path, required=True)
    combine.add_argument("--cell-dir", type=Path, required=True)
    combine.add_argument("--source-commit", required=True)
    combine.add_argument("--binary", action="append", default=[], required=True)
    combine.add_argument("--build-receipt", type=Path, required=True)
    combine.add_argument("--report", type=Path, required=True)
    combine.add_argument("--output", type=Path, required=True)
    return root


def main() -> int:
    args = parser().parse_args()
    try:
        if args.command == "plan":
            atomic_json(args.output.resolve(), plan_document())
        elif args.command == "status":
            atomic_json(args.output.resolve(), status_document(args))
        elif args.command == "seal":
            seal_cell(args)
        elif args.command == "aggregate":
            aggregate(args)
        else:  # pragma: no cover - argparse owns command admission.
            raise ContractError(f"unsupported command: {args.command}")
    except (ContractError, FileNotFoundError, json.JSONDecodeError) as error:
        print(f"MMQ.METAMORPHIC.CHECKPOINT: {error}", file=os.sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
