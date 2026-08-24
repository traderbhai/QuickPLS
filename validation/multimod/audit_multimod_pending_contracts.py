#!/usr/bin/env python3
"""Audit the unqualified tracked MultiMod templates without promoting anything.

This standard-library-only audit is intentionally limited to the development
state. It verifies that every indexed profile has one manifest declaration and
that every declaration remains Labs/absent with pending, unhashed evidence,
while the executable qualification plan is bound to the reviewed gate wrapper.
It does not run a statistical test and cannot emit release evidence.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


HERE = Path(__file__).resolve().parent
REPOSITORY = HERE.parents[1]


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def relative(path: Path) -> str:
    return path.relative_to(REPOSITORY).as_posix()


def audit() -> dict[str, Any]:
    errors: list[str] = []
    index_path = HERE / "multimod_capability_index_v1.json"
    index = load_json(index_path)
    declared_profiles: dict[str, str] = {}
    indexed_profiles: dict[str, str] = {}

    for family in index.get("families", []):
        family_id = family.get("family_id")
        manifest_path = REPOSITORY / family.get("manifest", "")
        if not manifest_path.is_file():
            errors.append(f"{family_id}: manifest is missing: {relative(manifest_path)}")
            continue
        manifest = load_json(manifest_path)
        if manifest.get("family_id") != family_id:
            errors.append(f"{family_id}: manifest family identity mismatch")
        if manifest.get("surface") != "labs":
            errors.append(f"{family_id}: pending manifest surface must be labs")
        if manifest.get("declared_evidence_state") != "absent":
            errors.append(f"{family_id}: pending evidence must be absent")
        if manifest.get("promotion_allowed") is not False:
            errors.append(f"{family_id}: promotion must be disabled")

        source_binding = manifest.get("source_binding", {})
        if source_binding.get("status") != "pending":
            errors.append(f"{family_id}: source binding must remain pending")
        if source_binding.get("candidate_commit_sha") is not None:
            errors.append(f"{family_id}: pending candidate commit must be null")
        for artifact in source_binding.get("source_artifacts", []):
            if artifact.get("sha256") is not None:
                errors.append(f"{family_id}: pending source hash is non-null: {artifact.get('path')}")

        manifest_profile_ids: list[str] = []
        for profile in manifest.get("profile_matrix", []):
            profile_id = profile.get("profile_id")
            if not isinstance(profile_id, str) or not profile_id:
                errors.append(f"{family_id}: empty profile identity")
                continue
            if profile_id in declared_profiles:
                errors.append(f"duplicate manifest profile: {profile_id}")
            declared_profiles[profile_id] = family_id
            manifest_profile_ids.append(profile_id)
            if profile.get("coverage_state") != "absent":
                errors.append(f"{profile_id}: pending coverage must be absent")
            if profile.get("evidence_state") != "absent":
                errors.append(f"{profile_id}: pending evidence must be absent")
            if profile.get("surface") != "labs":
                errors.append(f"{profile_id}: pending surface must be labs")
            seen_procedures: set[str] = set()
            for cell in profile.get("procedure_cells", []):
                procedure_id = cell.get("procedure_id")
                if procedure_id in seen_procedures:
                    errors.append(f"{profile_id}: duplicate procedure: {procedure_id}")
                seen_procedures.add(procedure_id)
                if cell.get("evidence_state") != "absent" or cell.get("gate_state") != "pending":
                    errors.append(f"{profile_id}::{procedure_id}: must remain absent/pending")
                if cell.get("report_path") is not None or cell.get("report_sha256") is not None:
                    errors.append(f"{profile_id}::{procedure_id}: pending report binding must be null")
            if not seen_procedures:
                errors.append(f"{profile_id}: no procedure cells")

        expected = family.get("profiles", [])
        if manifest_profile_ids != expected:
            errors.append(f"{family_id}: index and manifest profile order/content differ")
        for profile_id in expected:
            if profile_id in indexed_profiles:
                errors.append(f"duplicate indexed profile: {profile_id}")
            indexed_profiles[profile_id] = family_id

    if indexed_profiles != declared_profiles:
        missing = sorted(set(indexed_profiles) - set(declared_profiles))
        extra = sorted(set(declared_profiles) - set(indexed_profiles))
        if missing:
            errors.append(f"profiles missing from manifests: {missing}")
        if extra:
            errors.append(f"profiles absent from index: {extra}")

    unsupported = load_json(HERE / "unsupported_intersections_v1.json")
    codes = [entry.get("code") for entry in unsupported.get("entries", [])]
    if len(codes) != len(set(codes)):
        errors.append("unsupported-intersection reason codes are not unique")
    if any(not isinstance(code, str) or not code.startswith("multimod.") for code in codes):
        errors.append("unsupported-intersection reason code is invalid")

    plan = load_json(HERE / "v256_multimod_qualification_plan_v1.json")
    if set(plan.get("manifests", [])) != {
        family.get("manifest") for family in index.get("families", [])
    }:
        errors.append("qualification plan manifest set differs from the capability index")
    gate_ids = [gate.get("gate_id") for gate in plan.get("gates", [])]
    if len(gate_ids) != len(set(gate_ids)):
        errors.append("qualification gate identities are not unique")
    pending_gates: list[str] = []
    for gate in plan.get("gates", []):
        expected = {
            "executable": "pwsh",
            "arguments": [
                "-NoProfile",
                "-File",
                "validation/multimod/invoke_multimod_gate_v1.ps1",
                "-GateId",
                gate.get("gate_id"),
            ],
        }
        if gate.get("implementation_status") == "ready":
            if gate.get("command") != expected:
                errors.append(f"{gate.get('gate_id')}: reviewed gate wrapper binding is missing")
        elif gate.get("implementation_status") == "pending":
            pending_gates.append(str(gate.get("gate_id")))
            if gate.get("command") is not None:
                errors.append(f"{gate.get('gate_id')}: pending gate must not expose a command")
        else:
            errors.append(f"{gate.get('gate_id')}: gate implementation status is invalid")

    return {
        "schema_version": 1,
        "report_id": "qpls.multimod.pending_contract_audit.v1",
        "passed": not errors,
        "promotion_evidence": False,
        "family_count": len(index.get("families", [])),
        "profile_count": len(indexed_profiles),
        "reason_code_count": len(codes),
        "gate_count": len(gate_ids),
        "ready_gate_count": len(gate_ids) - len(pending_gates),
        "pending_gate_count": len(pending_gates),
        "pending_gates": pending_gates,
        "errors": errors,
        "source_artifacts": [
            relative(index_path),
            relative(HERE / "unsupported_intersections_v1.json"),
            relative(HERE / "v256_multimod_qualification_plan_v1.json"),
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = audit()
    payload = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(payload, encoding="utf-8")
    else:
        print(payload, end="")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
