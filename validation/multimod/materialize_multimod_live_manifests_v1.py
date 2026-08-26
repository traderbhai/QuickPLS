#!/usr/bin/env python3
"""Materialize and verify non-circular, source-bound MultiMod live manifests.

Tracked manifests remain Labs/absent templates. Release-qualified live cell
reports and manifests are written only beneath the external campaign root and
derive the Standard surface for the exact candidate. Source hashes bind clean
working-tree bytes at the exact candidate commit; evidence hashes bind prior
gate receipts. The manifest-set index hashes each live manifest but is never a
source artifact of the candidate, avoiding self-hashing and commit circularity.
"""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import re
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
PLAN = HERE / "v256_multimod_qualification_plan_v1.json"
CATALOG = HERE / "multimod_gate_bindings_v1.json"
INDEX = HERE / "multimod_capability_index_v1.json"
RUNNER_PROMOTION_SOURCE = ROOT / "crates/qpls-runner/src/multimod_execution_v1.rs"
NATIVE_EXPORT_SOURCE = ROOT / "src-tauri/src/canonical_result_export_publication_v2.rs"
NATIVE_AUTHORITY_SOURCE = ROOT / "src-tauri/src/multimod_candidate_authority_v1.rs"
NATIVE_JOB_SOURCE = ROOT / "src-tauri/src/multimod_jobs_v1.rs"
NATIVE_RAW_SIDECAR_SOURCE = ROOT / "src-tauri/src/multimod_raw_sidecar_export_v1.rs"
NATIVE_ARCHIVE_READ_SOURCE = ROOT / "src-tauri/src/project_archive_v6_read.rs"
NATIVE_ARCHIVE_SAVE_SOURCE = ROOT / "src-tauri/src/project_archive_v6_save_copy.rs"
NATIVE_PACKAGED_FIXTURE_SOURCE = (
    ROOT / "src-tauri/src/multimod_packaged_qualification_fixtures_v1.rs"
)
NATIVE_WORKSPACE_SOURCE = ROOT / "src/native/NativeMultiModLabsWorkspace.tsx"
DESKTOP_BUILD_SOURCE = ROOT / "src-tauri/build.rs"
SHA40 = re.compile(r"^[a-f0-9]{40}$")
SHA64 = re.compile(r"^[a-f0-9]{64}$")
TRACKED_SURFACE = "labs"
TRACKED_PROMOTION_ALLOWED = False
LIVE_SURFACE = "standard"
LIVE_PROMOTION_ALLOWED = True

PREPACKAGE_GLOBAL_GATES = (
    "contracts.schemas.source_identity",
    "core.project.compile",
    "typescript.contracts.typecheck",
    "estimation.point.kernels",
    "resampling.shared_ledgers",
    "runner.persistence.integration",
    "metamorphic.global",
    "archive.sidecar.integrity",
    "exports.semantic.readback",
    "legacy.continuous_and_serialization",
    "native.workflow.accessibility",
    "performance.maximum_profiles",
)
FINAL_RUNTIME_GATES = (
    "manifests.prepackage.authority",
    "package.candidate",
    "installed.offline.smoke",
    "portable.offline.smoke",
)
FAMILY_GATES = {
    "qpls.multimod.mga_multigroup_v1": (
        "mga.group_matrix",
        "mga.inference.matrix",
        "mga.label_reversal",
    ),
    "qpls.multimod.pls_heterogeneity_v2": (
        "fimix.recovery",
        "fimix.collapse.boundaries",
        "pos.recovery",
        "pos.common_metric",
        "heterogeneity.bootstrap",
    ),
    "qpls.multimod.general_sem_conditional_process_v2": (
        "conditional.profile_matrix",
        "conditional.bca_studentized",
        "conditional.hoc_group_weight",
        "conditional.probes",
    ),
    "qpls.multimod.interventional_causal_mediation_v1": (
        "causal.known_targets",
        "causal.assumption_failures",
    ),
}


class ManifestError(RuntimeError):
    pass


def valid_surface_promotion_pair(surface: Any, promotion_allowed: Any) -> bool:
    """Return whether one manifest uses a frozen tracked or promoted pair."""
    return (
        surface == TRACKED_SURFACE
        and promotion_allowed is TRACKED_PROMOTION_ALLOWED
    ) or (
        surface == LIVE_SURFACE
        and promotion_allowed is LIVE_PROMOTION_ALLOWED
    )


def assert_tracked_manifest_template(template: dict[str, Any]) -> None:
    family_id = template.get("family_id")
    if (
        not valid_surface_promotion_pair(
            template.get("surface"), template.get("promotion_allowed")
        )
        or template.get("surface") != TRACKED_SURFACE
        or template.get("declared_evidence_state") != "absent"
        or template.get("source_binding", {}).get("status") != "pending"
        or template.get("source_binding", {}).get("candidate_commit_sha") is not None
    ):
        raise ManifestError(
            f"tracked manifest template must remain Labs/absent: {family_id}"
        )
    source_artifacts = template["source_binding"].get("source_artifacts")
    if not isinstance(source_artifacts, list) or not source_artifacts:
        raise ManifestError(
            f"tracked manifest template has no source artifacts: {family_id}"
        )
    observed_sources: set[str] = set()
    for source in source_artifacts:
        path = source.get("path") if isinstance(source, dict) else None
        if (
            not isinstance(path, str)
            or not path
            or path in observed_sources
            or source.get("sha256") is not None
        ):
            raise ManifestError(
                f"tracked source artifact must remain unique and unbound: {family_id}"
            )
        observed_sources.add(path)
    profiles = template.get("profile_matrix")
    if not isinstance(profiles, list) or not profiles:
        raise ManifestError(f"tracked manifest template has no profiles: {family_id}")
    observed_profiles: set[str] = set()
    for profile in profiles:
        profile_id = profile.get("profile_id") if isinstance(profile, dict) else None
        if (
            not isinstance(profile, dict)
            or not isinstance(profile_id, str)
            or not profile_id
            or profile_id in observed_profiles
            or profile.get("surface") != TRACKED_SURFACE
            or profile.get("coverage_state") != "absent"
            or profile.get("evidence_state") != "absent"
        ):
            raise ManifestError(
                f"tracked manifest profile must remain Labs/absent: {family_id}"
            )
        observed_profiles.add(profile_id)
        procedures = profile.get("procedure_cells")
        if not isinstance(procedures, list) or not procedures:
            raise ManifestError(
                f"tracked manifest profile has no procedure cells: {profile_id}"
            )
        observed_cells: set[str] = set()
        for procedure in procedures:
            procedure_id = (
                procedure.get("procedure_id") if isinstance(procedure, dict) else None
            )
            identity = f"{profile_id}::{procedure_id}"
            if (
                not isinstance(procedure, dict)
                or not isinstance(procedure_id, str)
                or not procedure_id
                or identity in observed_cells
                or procedure.get("evidence_identity_template") != identity
                or procedure.get("evidence_state") != "absent"
                or procedure.get("gate_state") != "pending"
                or procedure.get("report_path") is not None
                or procedure.get("report_sha256") is not None
            ):
                raise ManifestError(
                    f"tracked procedure must remain an exact absent cell: {identity}"
                )
            observed_cells.add(identity)


def tracked_profile_cells(template: dict[str, Any]) -> dict[str, set[str]]:
    """Return the exact profile/cell inventory after template validation."""
    assert_tracked_manifest_template(template)
    return {
        profile["profile_id"]: {
            f"{profile['profile_id']}::{cell['procedure_id']}"
            for cell in profile["procedure_cells"]
        }
        for profile in template["profile_matrix"]
    }


def read_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")


def write_json_atomic(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_bytes(canonical_bytes(value))
    temporary.replace(path)


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def git(*arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(ROOT), *arguments],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def assert_candidate(candidate_commit: str, plan_sha256: str) -> None:
    if not SHA40.fullmatch(candidate_commit):
        raise ManifestError("candidate commit is not an exact lowercase SHA")
    if not SHA64.fullmatch(plan_sha256) or sha256(PLAN) != plan_sha256:
        raise ManifestError("qualification plan digest is stale")
    if git("rev-parse", "HEAD") != candidate_commit:
        raise ManifestError("candidate commit differs from current HEAD")
    if git("status", "--porcelain=v1", "--untracked-files=all"):
        raise ManifestError("candidate worktree changed during qualification")


def assert_runtime_promotion_mechanism() -> None:
    """Reject external qualification claims the candidate cannot consume.

    This is deliberately a source-level fail-closed boundary in addition to
    installed/portable functional evidence. The final candidate must already
    contain a typed exact-commit authority path; live JSON cannot promote a
    permanently Labs-only binary after the fact.
    """
    runner = RUNNER_PROMOTION_SOURCE.read_text(encoding="utf-8", errors="replace")
    native_export = NATIVE_EXPORT_SOURCE.read_text(encoding="utf-8", errors="replace")
    native_authority = NATIVE_AUTHORITY_SOURCE.read_text(encoding="utf-8", errors="replace")
    native_jobs = NATIVE_JOB_SOURCE.read_text(encoding="utf-8", errors="replace")
    native_raw_sidecar = NATIVE_RAW_SIDECAR_SOURCE.read_text(
        encoding="utf-8", errors="replace"
    )
    native_archive_read = NATIVE_ARCHIVE_READ_SOURCE.read_text(
        encoding="utf-8", errors="replace"
    )
    native_archive_save = NATIVE_ARCHIVE_SAVE_SOURCE.read_text(
        encoding="utf-8", errors="replace"
    )
    native_packaged_fixture = NATIVE_PACKAGED_FIXTURE_SOURCE.read_text(
        encoding="utf-8", errors="replace"
    )
    native_workspace = NATIVE_WORKSPACE_SOURCE.read_text(
        encoding="utf-8", errors="replace"
    )
    build_source = DESKTOP_BUILD_SOURCE.read_text(encoding="utf-8", errors="replace")
    if not re.search(
        r"qualification\s*:\s*MultimodQualificationStateV1::UnqualifiedLabs\s*,",
        runner,
    ):
        raise ManifestError(
            "runtime promotion unsafe: the runner must always emit receipt-free Labs provenance"
        )
    required_build_identities = (
        "QPLS_MULTIMOD_BUILD_CANDIDATE_AUTHORITY_V1",
        "QPLS_MULTIMOD_BUILD_PREPACKAGE_MANIFEST_SET_V1",
        "git_worktree_is_clean",
        "validate_prepackage_manifests",
        "tracked_exact_cells",
        "LABS_SENTINEL",
    )
    if any(identity not in build_source for identity in required_build_identities):
        raise ManifestError(
            "runtime promotion unavailable: build.rs lacks the immutable prepackage authority boundary"
        )
    if (
        "promote_completed_multimod_result_v1" not in native_authority
        or "required_multimod_candidate_profile_cells_v1" not in native_authority
        or "multimod_standard_surface_authorized_v1" not in native_authority
        or "promote_completed_multimod_result_v1" not in native_jobs
        or "MULTIMOD_STANDARD_SURFACE_V1" not in native_jobs
        or '"standard_multimod_v1"' not in native_jobs
        or '"standard_multimod_v1"' not in native_raw_sidecar
        or not re.search(
            r"persist_multimod_execution_surface_v1\(\s*&mut recipe\.metadata,\s*request\.surface\.clone\(\)\s*\)",
            native_jobs,
        )
        or not re.search(
            r'metadata\.insert\(\s*"execution_surface"\.into\(\),\s*request_surface\s*\)',
            native_jobs,
        )
        or any(
            "multimod_standard_surface_authorized_v1" not in source
            or '"standard_multimod_v1"' not in source
            for source in (
                native_archive_read,
                native_archive_save,
                native_packaged_fixture,
            )
        )
        or '"Standard · Release-qualified"' not in native_workspace
        or '"standard_multimod_v1"' not in native_workspace
        or "verify_multimod_candidate_receipt_against_embedded_v1" not in native_export
    ):
        raise ManifestError(
            "runtime promotion unavailable: native Standard execution/export does not consume the embedded exact-cell authority"
        )


def ensure_inside(parent: Path, child: Path) -> Path:
    resolved_parent = parent.resolve()
    resolved_child = child.resolve()
    try:
        resolved_child.relative_to(resolved_parent)
    except ValueError as error:
        raise ManifestError(f"path escapes campaign root: {child}") from error
    return resolved_child


def receipt_for(campaign_root: Path, gate_id: str, candidate_commit: str, plan_sha256: str) -> tuple[dict[str, Any], Path]:
    path = ensure_inside(campaign_root, campaign_root / gate_id / "gate_receipt.json")
    if not path.is_file():
        raise ManifestError(f"required gate receipt is missing: {gate_id}")
    receipt = read_json(path)
    if (
        receipt.get("receipt_kind") != "qpls_multimod_gate_receipt_v1"
        or receipt.get("coverage_binding_state") != "executed_real_commands"
        or receipt.get("gate_id") != gate_id
        or receipt.get("status") != "passed"
        or receipt.get("candidate_commit_sha") != candidate_commit
        or receipt.get("candidate_version") != read_json(PLAN)["candidate"]["final_version"]
        or receipt.get("plan_sha256") != plan_sha256
        or receipt.get("binding_sha256") != sha256(CATALOG)
        or not SHA64.fullmatch(str(receipt.get("input_digest", "")))
        or receipt.get("seed") != 42
    ):
        raise ManifestError(f"required gate receipt is invalid or stale: {gate_id}")
    return receipt, path


def profile_specific_gates(family_id: str, profile_id: str) -> tuple[str, ...]:
    if family_id == "qpls.multimod.pls_heterogeneity_v2":
        if profile_id.startswith("fimix."):
            return ("fimix.recovery", "fimix.collapse.boundaries", "heterogeneity.bootstrap")
        if profile_id.startswith("pos.common_metric."):
            return ("pos.recovery", "pos.common_metric", "heterogeneity.bootstrap")
        return ("pos.recovery", "pos.common_metric", "heterogeneity.bootstrap")
    if family_id == "qpls.multimod.general_sem_conditional_process_v2":
        gates = ["conditional.profile_matrix", "conditional.probes"]
        if profile_id in {"conditional.multi_two_way_bca.v2", "conditional.studentized.v2"}:
            gates.append("conditional.bca_studentized")
        if profile_id in {
            "conditional.multiple_hoc_percentile.v2",
            "conditional.grouped_percentile.v2",
            "conditional.case_weighted_percentile.v2",
            "conditional.frequency_weighted_percentile.v2",
        }:
            gates.append("conditional.hoc_group_weight")
        return tuple(gates)
    return FAMILY_GATES[family_id]


def coverage_matches(pattern: str, identity: str) -> bool:
    return fnmatch.fnmatchcase(identity, pattern)


def source_rows(template: dict[str, Any]) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    for declared in template["source_binding"]["source_artifacts"]:
        relative = declared["path"]
        path = (ROOT / relative).resolve()
        try:
            path.relative_to(ROOT)
        except ValueError as error:
            raise ManifestError(f"source path escapes repository: {relative}") from error
        if not path.is_file():
            raise ManifestError(f"source artifact is missing: {relative}")
        rows.append({"path": relative, "sha256": sha256(path)})
    return rows


def gate_input_rows(gate_id: str) -> list[dict[str, str]]:
    matches = [
        gate
        for gate in read_json(CATALOG).get("gates", [])
        if gate.get("gate_id") == gate_id
    ]
    if len(matches) != 1:
        raise ManifestError(f"gate binding is missing or duplicated: {gate_id}")
    rows: list[dict[str, str]] = []
    for relative in matches[0].get("input_artifacts", []):
        path = (ROOT / relative).resolve()
        try:
            path.relative_to(ROOT)
        except ValueError as error:
            raise ManifestError(f"gate input escapes repository: {relative}") from error
        if not path.is_file():
            raise ManifestError(f"gate input is missing: {relative}")
        rows.append({"path": relative, "sha256": sha256(path)})
    if not rows:
        raise ManifestError(f"gate binding declares no input artifacts: {gate_id}")
    return rows


def assert_no_open_issues(
    campaign_root: Path,
    candidate_commit: str,
    plan_sha256: str,
    global_gates: tuple[str, ...],
) -> None:
    inventory_path = campaign_root / "issue_inventory.json"
    state_path = campaign_root / "campaign_state.json"
    if not inventory_path.is_file() or not state_path.is_file():
        raise ManifestError("campaign state or issue inventory is missing")
    inventory = read_json(inventory_path)
    state = read_json(state_path)
    for document, label in ((inventory, "issue inventory"), (state, "campaign state")):
        if (
            document.get("candidate_commit_sha") != candidate_commit
            or document.get("candidate_version") != read_json(PLAN)["candidate"]["final_version"]
            or document.get("plan_sha256") != plan_sha256
            or document.get("binding_sha256") != sha256(CATALOG)
        ):
            raise ManifestError(f"{label} is stale")
    open_issues = [item for item in inventory.get("issues", []) if item.get("disposition") == "open"]
    if open_issues:
        raise ManifestError("live manifests cannot be materialized while the campaign has open issues")
    states = {
        row.get("gate_id"): (row.get("status"), row.get("evidence_valid"))
        for row in state.get("gates", [])
    }
    for gate_id in (*global_gates, *{item for gates in FAMILY_GATES.values() for item in gates}):
        if states.get(gate_id) != ("passed", True):
            raise ManifestError(f"required gate has not passed with valid evidence: {gate_id}")


def materialize(
    campaign_root: Path,
    candidate_commit: str,
    plan_sha256: str,
    output: Path,
    stage: str,
    authority_output: Path | None = None,
) -> dict[str, Any]:
    assert_candidate(candidate_commit, plan_sha256)
    assert_runtime_promotion_mechanism()
    campaign_root = campaign_root.resolve()
    output = ensure_inside(campaign_root, output)
    if stage not in {"prepackage_authority", "final_live"}:
        raise ManifestError(f"unsupported manifest stage: {stage}")
    global_gates = PREPACKAGE_GLOBAL_GATES + (FINAL_RUNTIME_GATES if stage == "final_live" else ())
    assert_no_open_issues(campaign_root, candidate_commit, plan_sha256, global_gates)
    plan = read_json(PLAN)
    manifest_rows: list[dict[str, Any]] = []
    generated_at = datetime.now(timezone.utc).isoformat()

    for declared_template in plan["manifests"]:
        template_path = ROOT / declared_template
        template = read_json(template_path)
        assert_tracked_manifest_template(template)
        family_id = template["family_id"]
        sources = source_rows(template)
        live = json.loads(json.dumps(template))
        live["surface"] = LIVE_SURFACE
        live["declared_evidence_state"] = "release_qualified"
        live["promotion_allowed"] = LIVE_PROMOTION_ALLOWED
        live["source_binding"] = {
            "status": "bound",
            "candidate_commit_sha": candidate_commit,
            "source_artifacts": sources,
        }

        for profile in live["profile_matrix"]:
            profile_id = profile["profile_id"]
            required_gates = tuple(dict.fromkeys((*global_gates, *profile_specific_gates(family_id, profile_id))))
            receipt_rows: list[dict[str, Any]] = []
            receipts: list[dict[str, Any]] = []
            for gate_id in required_gates:
                receipt, receipt_path = receipt_for(campaign_root, gate_id, candidate_commit, plan_sha256)
                receipts.append(receipt)
                receipt_rows.append(
                    {
                        "gate_id": gate_id,
                        "path": receipt_path.relative_to(campaign_root).as_posix(),
                        "sha256": sha256(receipt_path),
                        "input_digest": receipt["input_digest"],
                        "seed": receipt["seed"],
                        "input_artifacts": gate_input_rows(gate_id),
                    }
                )

            profile["coverage_state"] = "release_qualified"
            profile["evidence_state"] = "release_qualified"
            profile["surface"] = LIVE_SURFACE
            for cell in profile["procedure_cells"]:
                identity = f"{profile_id}::{cell['procedure_id']}"
                scientific_receipts = [
                    receipt
                    for receipt in receipts
                    if receipt["gate_id"] not in global_gates
                ]
                if not any(
                    coverage_matches(pattern, identity)
                    for receipt in scientific_receipts
                    for pattern in receipt.get("covered_evidence_cells", [])
                ):
                    raise ManifestError(f"no profile-specific executed coverage declaration for {identity}")
                report = {
                    "schema_version": 1,
                    "report_kind": "qpls_multimod_profile_procedure_evidence_v1",
                    "report_id": f"qpls.multimod.evidence.{profile_id}.{cell['procedure_id']}",
                    "family_id": family_id,
                    "profile_id": profile_id,
                    "procedure_id": cell["procedure_id"],
                    "method_version": profile["method_version"],
                    "evidence_state": "release_qualified",
                    "candidate_commit_sha": candidate_commit,
                    "plan_sha256": plan_sha256,
                    "template_path": declared_template,
                    "template_sha256": sha256(template_path),
                    "source_artifacts": sources,
                    "required_gate_receipts": receipt_rows,
                    "generated_at_utc": generated_at,
                    "qualification_claim": "This exact profile and procedure cell passed every bound gate in the cited candidate campaign.",
                }
                report_path = campaign_root / "cell-reports" / stage / family_id / profile_id / f"{cell['procedure_id']}.json"
                write_json_atomic(report_path, report)
                cell["evidence_state"] = "release_qualified"
                cell["gate_state"] = "passed"
                cell["report_path"] = report_path.relative_to(campaign_root).as_posix()
                cell["report_sha256"] = sha256(report_path)

        live_path = output.parent / template_path.name
        write_json_atomic(live_path, live)
        exact_profile_cells = sorted(
            f"{profile['profile_id']}::{cell['procedure_id']}"
            for profile in live["profile_matrix"]
            for cell in profile["procedure_cells"]
        )
        manifest_rows.append(
            {
                "family_id": family_id,
                "path": live_path.relative_to(campaign_root).as_posix(),
                "sha256": sha256(live_path),
                "template_path": declared_template,
                "template_sha256": sha256(template_path),
                "exact_profile_cells": exact_profile_cells,
            }
        )

    exact_profile_cells = sorted(
        cell
        for row in manifest_rows
        for cell in row["exact_profile_cells"]
    )
    if len(exact_profile_cells) != len(set(exact_profile_cells)) or not exact_profile_cells:
        raise ManifestError("materialized exact profile cells are empty or duplicated")
    prepackage = stage == "prepackage_authority"
    manifest_set = {
        "schema_version": 1,
        "manifest_set_id": (
            "qpls.v256.multimod.prepackage-authority-set.v1"
            if prepackage
            else "qpls.v256.multimod.final-live-manifest-set.v1"
        ),
        "stage": stage,
        "state": "release_qualified",
        "surface": LIVE_SURFACE,
        "promotion_allowed": LIVE_PROMOTION_ALLOWED,
        "candidate_commit_sha": candidate_commit,
        "candidate_version": plan["candidate"]["final_version"],
        "plan_sha256": plan_sha256,
        "gate_binding_sha256": sha256(CATALOG),
        "capability_index_sha256": sha256(INDEX),
        "generated_at_utc": generated_at,
        "non_circular_binding": {
            "candidate_source_contains_evidence": False,
            "authority_generated_after_candidate_commit": True,
            "build_consumes_external_prepackage_set": prepackage,
            "artifact_paths_relative_to_campaign_root": True,
            "campaign_root_ancestor_depth": 2,
        },
        "manifests": manifest_rows,
        "exact_profile_cells": exact_profile_cells,
    }
    write_json_atomic(output, manifest_set)
    if prepackage:
        if authority_output is None:
            raise ManifestError("prepackage materialization requires --authority-output")
        authority_output = ensure_inside(campaign_root, authority_output)
        binding = {
            "candidate_commit_sha": candidate_commit,
            "candidate_version": plan["candidate"]["final_version"],
            "qualification_plan_sha256": plan_sha256,
            "gate_binding_sha256": sha256(CATALOG),
            "capability_index_sha256": sha256(INDEX),
            "prepackage_manifest_set_sha256": sha256(output),
            "exact_profile_cells": exact_profile_cells,
        }
        compact_binding = json.dumps(binding, ensure_ascii=True, separators=(",", ":")).encode("utf-8")
        authority = {
            "schema_version": 1,
            "authority_kind": "qpls_multimod_embedded_candidate_authority_v1",
            "state": "release_qualified_candidate",
            "binding": binding,
            "authority_binding_sha256": sha256_bytes(compact_binding),
        }
        write_json_atomic(authority_output, authority)
    return verify(campaign_root, candidate_commit, plan_sha256, output, stage, authority_output)


def verify(
    campaign_root: Path,
    candidate_commit: str,
    plan_sha256: str,
    output: Path,
    stage: str,
    authority_output: Path | None = None,
) -> dict[str, Any]:
    assert_candidate(candidate_commit, plan_sha256)
    assert_runtime_promotion_mechanism()
    campaign_root = campaign_root.resolve()
    output = ensure_inside(campaign_root, output)
    if stage not in {"prepackage_authority", "final_live"}:
        raise ManifestError(f"unsupported manifest stage: {stage}")
    if not output.is_file():
        raise ManifestError("live manifest-set index is missing")
    manifest_set = read_json(output)
    expected_set_id = (
        "qpls.v256.multimod.prepackage-authority-set.v1"
        if stage == "prepackage_authority"
        else "qpls.v256.multimod.final-live-manifest-set.v1"
    )
    if (
        manifest_set.get("manifest_set_id") != expected_set_id
        or manifest_set.get("stage") != stage
        or manifest_set.get("state") != "release_qualified"
        or manifest_set.get("surface") != LIVE_SURFACE
        or manifest_set.get("promotion_allowed") is not LIVE_PROMOTION_ALLOWED
        or manifest_set.get("candidate_commit_sha") != candidate_commit
        or manifest_set.get("candidate_version") != read_json(PLAN)["candidate"]["final_version"]
        or manifest_set.get("plan_sha256") != plan_sha256
        or manifest_set.get("gate_binding_sha256") != sha256(CATALOG)
        or manifest_set.get("capability_index_sha256") != sha256(INDEX)
    ):
        raise ManifestError("live manifest-set identity or candidate binding is invalid")
    expected_non_circular = {
        "candidate_source_contains_evidence": False,
        "authority_generated_after_candidate_commit": True,
        "build_consumes_external_prepackage_set": stage == "prepackage_authority",
        "artifact_paths_relative_to_campaign_root": True,
        "campaign_root_ancestor_depth": 2,
    }
    if manifest_set.get("non_circular_binding") != expected_non_circular:
        raise ManifestError("manifest-set non-circular binding is incomplete or malformed")
    expected_families = {family["family_id"] for family in read_json(INDEX)["families"]}
    observed_families: set[str] = set()
    observed_cells: list[str] = []
    manifest_rows = manifest_set.get("manifests", [])
    if not isinstance(manifest_rows, list) or not manifest_rows:
        raise ManifestError("manifest-set contains no typed manifest rows")
    observed_row_paths: set[str] = set()
    for row in manifest_rows:
        if not isinstance(row, dict) or set(row) != {
            "family_id",
            "path",
            "sha256",
            "template_path",
            "template_sha256",
            "exact_profile_cells",
        }:
            raise ManifestError("manifest-set contains a malformed manifest row")
        if row["path"] in observed_row_paths:
            raise ManifestError(f"manifest path is duplicated: {row['path']}")
        observed_row_paths.add(row["path"])
        path = ensure_inside(campaign_root, campaign_root / row["path"])
        if not path.is_file() or sha256(path) != row.get("sha256"):
            raise ManifestError(f"live manifest is missing or stale: {row.get('family_id')}")
        template_path = ROOT / row["template_path"]
        if not template_path.is_file() or sha256(template_path) != row.get("template_sha256"):
            raise ManifestError(f"tracked template changed after derivation: {row.get('family_id')}")
        template = read_json(template_path)
        expected_profile_cells = tracked_profile_cells(template)
        manifest = read_json(path)
        family_id = manifest.get("family_id")
        if family_id != row.get("family_id"):
            raise ManifestError("manifest row family identity differs from its live manifest")
        observed_families.add(family_id)
        if (
            manifest.get("declared_evidence_state") != "release_qualified"
            or manifest.get("surface") != LIVE_SURFACE
            or manifest.get("promotion_allowed") is not LIVE_PROMOTION_ALLOWED
            or manifest.get("source_binding", {}).get("status") != "bound"
            or manifest.get("source_binding", {}).get("candidate_commit_sha") != candidate_commit
        ):
            raise ManifestError(f"live manifest state is invalid: {family_id}")
        for source in manifest["source_binding"]["source_artifacts"]:
            source_path = ROOT / source["path"]
            if not source_path.is_file() or sha256(source_path) != source.get("sha256"):
                raise ManifestError(f"source hash is stale: {source.get('path')}")
        live_profile_cells: dict[str, set[str]] = {}
        for profile in manifest.get("profile_matrix", []):
            profile_id = profile.get("profile_id")
            if (
                not isinstance(profile_id, str)
                or profile_id in live_profile_cells
                or profile.get("coverage_state") != "release_qualified"
                or profile.get("evidence_state") != "release_qualified"
                or profile.get("surface") != LIVE_SURFACE
            ):
                raise ManifestError(f"profile is not release-qualified: {profile.get('profile_id')}")
            live_profile_cells[profile_id] = set()
            for cell in profile.get("procedure_cells", []):
                identity = f"{profile_id}::{cell.get('procedure_id')}"
                if identity in live_profile_cells[profile_id]:
                    raise ManifestError(f"procedure evidence is duplicated: {identity}")
                live_profile_cells[profile_id].add(identity)
                observed_cells.append(identity)
                report_path = ensure_inside(campaign_root, campaign_root / cell.get("report_path", ""))
                if (
                    cell.get("evidence_state") != "release_qualified"
                    or cell.get("gate_state") != "passed"
                    or cell.get("evidence_identity_template") != identity
                    or not report_path.is_file()
                    or sha256(report_path) != cell.get("report_sha256")
                ):
                    raise ManifestError(
                        f"procedure evidence is missing or stale: {identity}"
                    )
                report = read_json(report_path)
                if (
                    report.get("schema_version") != 1
                    or report.get("report_kind")
                    != "qpls_multimod_profile_procedure_evidence_v1"
                    or report.get("family_id") != family_id
                    or report.get("candidate_commit_sha") != candidate_commit
                    or report.get("plan_sha256") != plan_sha256
                    or report.get("profile_id") != profile.get("profile_id")
                    or report.get("procedure_id") != cell.get("procedure_id")
                    or report.get("method_version") != profile.get("method_version")
                    or report.get("evidence_state") != "release_qualified"
                    or report.get("template_path") != row.get("template_path")
                    or report.get("template_sha256") != row.get("template_sha256")
                    or report.get("source_artifacts")
                    != manifest.get("source_binding", {}).get("source_artifacts")
                ):
                    raise ManifestError(f"procedure report identity mismatch: {report_path}")
                required_receipts = report.get("required_gate_receipts", [])
                expected_gates = tuple(
                    dict.fromkeys(
                        (
                            *(
                                PREPACKAGE_GLOBAL_GATES
                                + (FINAL_RUNTIME_GATES if stage == "final_live" else ())
                            ),
                            *profile_specific_gates(family_id, profile.get("profile_id")),
                        )
                    )
                )
                if (
                    not isinstance(required_receipts, list)
                    or [row.get("gate_id") for row in required_receipts]
                    != list(expected_gates)
                ):
                    raise ManifestError(f"procedure report gate inventory mismatch: {report_path}")
                for receipt_row in required_receipts:
                    if not isinstance(receipt_row, dict) or set(receipt_row) != {
                        "gate_id",
                        "path",
                        "sha256",
                        "input_digest",
                        "seed",
                        "input_artifacts",
                    }:
                        raise ManifestError("procedure report contains a malformed gate receipt row")
                    receipt_path = ensure_inside(campaign_root, campaign_root / receipt_row["path"])
                    if not receipt_path.is_file() or sha256(receipt_path) != receipt_row.get("sha256"):
                        raise ManifestError(f"gate receipt changed after derivation: {receipt_row.get('gate_id')}")
                    receipt = read_json(receipt_path)
                    expected_inputs = gate_input_rows(str(receipt_row.get("gate_id", "")))
                    if (
                        receipt.get("receipt_kind") != "qpls_multimod_gate_receipt_v1"
                        or receipt.get("coverage_binding_state") != "executed_real_commands"
                        or receipt.get("gate_id") != receipt_row.get("gate_id")
                        or receipt.get("status") != "passed"
                        or receipt.get("candidate_commit_sha") != candidate_commit
                        or receipt.get("candidate_version")
                        != read_json(PLAN)["candidate"]["final_version"]
                        or receipt.get("plan_sha256") != plan_sha256
                        or receipt.get("binding_sha256") != sha256(CATALOG)
                        or receipt.get("input_digest") != receipt_row.get("input_digest")
                        or receipt.get("seed") != receipt_row.get("seed")
                        or receipt_row.get("seed") != 42
                        or receipt_row.get("input_artifacts") != expected_inputs
                        or not SHA64.fullmatch(str(receipt_row.get("input_digest", "")))
                    ):
                        raise ManifestError(
                            f"gate receipt identity mismatch: {receipt_row.get('gate_id')}"
                        )
        if live_profile_cells != expected_profile_cells:
            raise ManifestError(
                f"live Standard manifest differs from the full tracked profile/cell inventory: {family_id}"
            )
        manifest_cells = sorted(
            f"{profile['profile_id']}::{cell['procedure_id']}"
            for profile in manifest.get("profile_matrix", [])
            for cell in profile.get("procedure_cells", [])
        )
        if row.get("exact_profile_cells") != manifest_cells:
            raise ManifestError(f"manifest row exact cells are stale: {family_id}")
    if observed_families != expected_families:
        raise ManifestError("live manifest families differ from the capability index")
    if sorted(observed_cells) != manifest_set.get("exact_profile_cells") or len(observed_cells) != len(set(observed_cells)):
        raise ManifestError("manifest-set exact cell inventory differs from its live manifests")
    if stage == "prepackage_authority":
        if authority_output is None:
            raise ManifestError("prepackage verification requires --authority-output")
        authority_output = ensure_inside(campaign_root, authority_output)
        if not authority_output.is_file():
            raise ManifestError("candidate authority output is missing")
        authority = read_json(authority_output)
        binding = authority.get("binding", {})
        if (
            authority.get("authority_kind") != "qpls_multimod_embedded_candidate_authority_v1"
            or authority.get("state") != "release_qualified_candidate"
            or binding.get("candidate_commit_sha") != candidate_commit
            or binding.get("candidate_version") != read_json(PLAN)["candidate"]["final_version"]
            or binding.get("qualification_plan_sha256") != plan_sha256
            or binding.get("gate_binding_sha256") != sha256(CATALOG)
            or binding.get("capability_index_sha256") != sha256(INDEX)
            or binding.get("prepackage_manifest_set_sha256") != sha256(output)
            or binding.get("exact_profile_cells") != manifest_set.get("exact_profile_cells")
            or authority.get("authority_binding_sha256")
            != sha256_bytes(json.dumps(binding, ensure_ascii=True, separators=(",", ":")).encode("utf-8"))
        ):
            raise ManifestError("candidate authority output differs from its prepackage set")
    return manifest_set


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "operation",
        choices=(
            "materialize-prepackage",
            "verify-prepackage",
            "materialize-final",
            "verify-final",
        ),
    )
    parser.add_argument("--campaign-root", type=Path, required=True)
    parser.add_argument("--candidate-commit", required=True)
    parser.add_argument("--plan-sha256", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--authority-output", type=Path)
    arguments = parser.parse_args()
    try:
        stage = "prepackage_authority" if arguments.operation.endswith("prepackage") else "final_live"
        if arguments.operation.startswith("materialize"):
            document = materialize(
                arguments.campaign_root,
                arguments.candidate_commit,
                arguments.plan_sha256,
                arguments.output,
                stage,
                arguments.authority_output,
            )
        else:
            document = verify(
                arguments.campaign_root,
                arguments.candidate_commit,
                arguments.plan_sha256,
                arguments.output,
                stage,
                arguments.authority_output,
            )
        print(json.dumps({"passed": True, "manifest_set": document}, sort_keys=True))
        return 0
    except Exception as error:
        print(
            json.dumps(
                {
                    "passed": False,
                    "state": "absent",
                    "stale_hash_behavior": "derive_absent",
                    "error": f"{type(error).__name__}:{error}",
                },
                sort_keys=True,
            )
        )
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
