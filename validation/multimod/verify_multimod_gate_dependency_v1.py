#!/usr/bin/env python3
"""Verify an earlier gate's immutable executable evidence without rerunning it.

The campaign uses this verifier to split one expensive producer workload into
non-overlapping review gates.  A dependency passes only when the producer
receipt is source-bound to this candidate, is itself hash-bound by campaign
state, and its declared log or JSON artifact contains every exact requested
test/check identity.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
from pathlib import Path
from typing import Any


SHA40 = re.compile(r"^[a-f0-9]{40}$")
SHA64 = re.compile(r"^[a-f0-9]{64}$")


class DependencyError(RuntimeError):
    pass


def read_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inside(root: Path, path: Path) -> Path:
    root = root.resolve()
    path = path.resolve()
    try:
        path.relative_to(root)
    except ValueError as error:
        raise DependencyError(f"dependency path escapes campaign root: {path}") from error
    return path


def bound_environment() -> dict[str, str]:
    values = {
        "candidate_commit_sha": os.environ.get("QPLS_MULTIMOD_CANDIDATE_COMMIT", ""),
        "candidate_version": os.environ.get("QPLS_MULTIMOD_CANDIDATE_VERSION", ""),
        "plan_sha256": os.environ.get("QPLS_MULTIMOD_PLAN_SHA256", ""),
        "binding_sha256": os.environ.get("QPLS_MULTIMOD_BINDING_SHA256", ""),
        "seed": os.environ.get("QPLS_MULTIMOD_SEED", ""),
    }
    if not SHA40.fullmatch(values["candidate_commit_sha"]):
        raise DependencyError("candidate commit environment binding is invalid")
    if not SHA64.fullmatch(values["plan_sha256"]) or not SHA64.fullmatch(values["binding_sha256"]):
        raise DependencyError("plan or binding digest environment binding is invalid")
    if not values["candidate_version"] or not values["seed"].isdigit():
        raise DependencyError("candidate version or seed environment binding is invalid")
    return values


def producer(
    campaign_root: Path,
    gate_id: str,
    step_id: str,
    environment: dict[str, str],
) -> tuple[dict[str, Any], dict[str, Any], Path]:
    receipt_path = inside(campaign_root, campaign_root / gate_id / "gate_receipt.json")
    state_path = inside(campaign_root, campaign_root / "campaign_state.json")
    if not receipt_path.is_file() or not state_path.is_file():
        raise DependencyError(f"producer receipt or campaign state is missing: {gate_id}")
    receipt = read_json(receipt_path)
    if (
        receipt.get("receipt_kind") != "qpls_multimod_gate_receipt_v1"
        or receipt.get("coverage_binding_state") != "executed_real_commands"
        or receipt.get("gate_id") != gate_id
        or receipt.get("status") != "passed"
        or receipt.get("candidate_commit_sha") != environment["candidate_commit_sha"]
        or receipt.get("candidate_version") != environment["candidate_version"]
        or receipt.get("plan_sha256") != environment["plan_sha256"]
        or receipt.get("binding_sha256") != environment["binding_sha256"]
        or int(receipt.get("seed", -1)) != int(environment["seed"])
    ):
        raise DependencyError(f"producer receipt is invalid or stale: {gate_id}")
    state = read_json(state_path)
    if (
        state.get("candidate_commit_sha") != environment["candidate_commit_sha"]
        or state.get("candidate_version") != environment["candidate_version"]
        or state.get("plan_sha256") != environment["plan_sha256"]
        or state.get("binding_sha256") != environment["binding_sha256"]
    ):
        raise DependencyError("campaign state is invalid or stale")
    rows = [row for row in state.get("gates", []) if row.get("gate_id") == gate_id]
    if (
        len(rows) != 1
        or rows[0].get("status") != "passed"
        or rows[0].get("evidence_valid") is not True
        or rows[0].get("receipt_sha256") != sha256(receipt_path)
    ):
        raise DependencyError(f"campaign state does not hash-bind producer receipt: {gate_id}")
    steps = [row for row in receipt.get("steps", []) if row.get("step_id") == step_id]
    if len(steps) != 1 or steps[0].get("status") != "passed":
        raise DependencyError(f"required producer step did not pass exactly once: {gate_id}/{step_id}")
    return receipt, steps[0], receipt_path


def verified_log(campaign_root: Path, step: dict[str, Any]) -> str:
    chunks: list[str] = []
    for stream in ("stdout", "stderr"):
        path = inside(campaign_root, Path(str(step.get(f"{stream}_path", ""))))
        expected = step.get(f"{stream}_sha256")
        if not path.is_file() or not SHA64.fullmatch(str(expected or "")) or sha256(path) != expected:
            raise DependencyError(f"producer {stream} log is missing or stale")
        chunks.append(path.read_text(encoding="utf-8", errors="replace"))
    return "\n".join(chunks)


def verified_artifact(
    campaign_root: Path,
    step: dict[str, Any],
    basename: str,
) -> tuple[dict[str, Any], Path]:
    rows = [
        row
        for row in step.get("expected_outputs", [])
        if Path(str(row.get("path", ""))).name == basename
    ]
    if len(rows) != 1:
        raise DependencyError(f"producer does not declare one artifact named {basename}")
    row = rows[0]
    path = inside(campaign_root, Path(str(row.get("path", ""))))
    if (
        not path.is_file()
        or not SHA64.fullmatch(str(row.get("sha256", "")))
        or sha256(path) != row.get("sha256")
        or path.stat().st_size != int(row.get("size", -1))
    ):
        raise DependencyError(f"producer artifact is missing or stale: {basename}")
    return read_json(path), path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--campaign-root", type=Path, required=True)
    parser.add_argument("--producer-gate", required=True)
    parser.add_argument("--producer-step", required=True)
    parser.add_argument("--required-log-contains", action="append", default=[])
    parser.add_argument("--artifact-basename")
    parser.add_argument("--required-scientific-check", action="append", default=[])
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    report: dict[str, Any]
    try:
        environment = bound_environment()
        campaign_root = arguments.campaign_root.resolve()
        receipt, step, receipt_path = producer(
            campaign_root,
            arguments.producer_gate,
            arguments.producer_step,
            environment,
        )
        log = verified_log(campaign_root, step)
        missing_log = [text for text in arguments.required_log_contains if text not in log]
        if missing_log:
            raise DependencyError(f"producer log omitted required executable identities: {missing_log}")
        artifact_path: Path | None = None
        artifact_sha256: str | None = None
        observed_checks: dict[str, str] = {}
        if arguments.artifact_basename:
            document, artifact_path = verified_artifact(campaign_root, step, arguments.artifact_basename)
            artifact_sha256 = sha256(artifact_path)
            for result in document.get("results", {}).values():
                for check in result.get("checks", []):
                    observed_checks[str(check.get("check_id"))] = str(check.get("status"))
            missing_checks = [
                check_id
                for check_id in arguments.required_scientific_check
                if observed_checks.get(check_id) != "passed"
            ]
            if document.get("status") != "passed" or missing_checks:
                raise DependencyError(f"scientific producer omitted passing checks: {missing_checks}")
        elif arguments.required_scientific_check:
            raise DependencyError("scientific check identities require --artifact-basename")
        if not arguments.required_log_contains and not arguments.required_scientific_check:
            raise DependencyError("dependency must name executable log or scientific check coverage")
        report = {
            "schema_version": 1,
            "report_id": "qpls.multimod.verified-gate-dependency.v1",
            "passed": True,
            **environment,
            "producer_gate": arguments.producer_gate,
            "producer_step": arguments.producer_step,
            "producer_receipt_path": str(receipt_path),
            "producer_receipt_sha256": sha256(receipt_path),
            "producer_input_digest": receipt.get("input_digest"),
            "required_log_contains": arguments.required_log_contains,
            "required_scientific_checks": arguments.required_scientific_check,
            "artifact_path": str(artifact_path) if artifact_path else None,
            "artifact_sha256": artifact_sha256,
        }
    except Exception as error:
        report = {
            "schema_version": 1,
            "report_id": "qpls.multimod.verified-gate-dependency.v1",
            "passed": False,
            "producer_gate": arguments.producer_gate,
            "producer_step": arguments.producer_step,
            "error": f"{type(error).__name__}:{error}",
        }
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = arguments.output.with_name(arguments.output.name + ".tmp")
    temporary.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(arguments.output)
    print(json.dumps(report, sort_keys=True))
    return 0 if report.get("passed") is True else 1


if __name__ == "__main__":
    raise SystemExit(main())
