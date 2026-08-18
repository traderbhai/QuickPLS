#!/usr/bin/env python3
"""Fail-closed promotion audit for the candidate CB-SEM bootstrap v2.

Source presence is useful engineering evidence, but never promotes this method.
The audit can pass only after semantic replay authentication plus independent
simulation, archive, native, export, and packaged acceptance all qualify.
"""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "cbsem_bootstrap_v2_method_promotion_audit.json"


def load_report(name: str) -> dict | None:
    path = RESULTS / name
    if not path.is_file():
        return None
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return value if isinstance(value, dict) else None


def passed(report: dict | None, method_version: str = "cbsem_bootstrap_v2") -> bool:
    return bool(
        report
        and report.get("passed") is True
        and report.get("method_version", method_version) == method_version
    )


def main() -> int:
    manifest = json.loads(
        (ROOT / "validation" / "methods" / "cbsem_bootstrap_v2.manifest.json")
        .read_text(encoding="utf-8")
    )
    engine = (ROOT / "crates" / "qpls-resampling" / "src" / "cbsem_bootstrap.rs").read_text(encoding="utf-8")
    project = (ROOT / "crates" / "qpls-project" / "src" / "lib.rs").read_text(encoding="utf-8")
    frontend = (ROOT / "src" / "native" / "nativeCbsemBootstrap.test.ts").read_text(encoding="utf-8")

    reports = {
        "reference": load_report("cbsem_bootstrap_v2_reference.json"),
        "simulation": load_report("cbsem_bootstrap_v2_release_simulation.json"),
        "boundary": load_report("cbsem_bootstrap_v2_release_boundary.json"),
        "persistence": load_report("cbsem_bootstrap_v2_release_persistence.json"),
        "frontend": load_report("cbsem_bootstrap_v2_frontend.json"),
        "packaged": load_report("v252_cbsem_bootstrap_v2_native_acceptance.json"),
    }
    declared_state = manifest.get("qualification", {}).get("declared_state")
    source_contract_present = all(token in engine for token in (
        "bootstrap_cbsem_ml_validated",
        "estimate_cbsem_case_resample_validated_with_control",
        "CBSEM_BOOTSTRAP_METHOD_VERSION_V2",
        "MINIMUM_REPLICATES: u32 = 1_000",
    ))
    structural_archive_present = all(token in project for token in (
        "validate_cbsem_bootstrap_v2_payload_contract",
        "cbsem_bootstrap_v2_rejects_tampering",
        "cbsem_bootstrap_v1_read_only",
        "reject_duplicate_json_object_keys",
    ))
    native_source_present = all(token in frontend for token in (
        "candidate; unqualified",
        "fixed two-sided 95%",
        "cbsem_bootstrap_intervals",
        "CB-SEM bootstrap qualification",
    ))
    persistence_report = reports["persistence"]
    semantic_replay_qualified = bool(
        persistence_report
        and persistence_report.get("semantic_replay_authentication") == "qualified"
    )
    evidence_passed = all(passed(report) for report in reports.values())
    promotable_state = declared_state in {"release_qualified", "native_qualified"}
    audit_passed = bool(
        source_contract_present
        and structural_archive_present
        and native_source_present
        and semantic_replay_qualified
        and evidence_passed
        and promotable_state
    )
    blockers = []
    if not source_contract_present:
        blockers.append("executable_engine_contract_missing")
    if not structural_archive_present:
        blockers.append("structural_archive_contract_missing")
    if not native_source_present:
        blockers.append("native_candidate_contract_missing")
    if not semantic_replay_qualified:
        blockers.append("coordinated_witness_semantic_rewrite_not_replayed")
    blockers.extend(
        f"{name}_evidence_absent_or_failed"
        for name, report in reports.items()
        if not passed(report)
    )
    if not promotable_state:
        blockers.append(f"manifest_declared_state_{declared_state or 'missing'}")

    report = {
        "kind": "cbsem_bootstrap_v2_method_promotion_audit_v1",
        "passed": audit_passed,
        "feature_id": "qpls3.cbsem.bootstrap",
        "method_version": "cbsem_bootstrap_v2",
        "declared_state": declared_state,
        "checks": {
            "executable_engine_contract_present": source_contract_present,
            "structural_archive_contract_present": structural_archive_present,
            "native_candidate_contract_present": native_source_present,
            "semantic_replay_authentication_qualified": semantic_replay_qualified,
            "all_required_evidence_passed": evidence_passed,
            "manifest_state_is_promotable": promotable_state,
        },
        "evidence": {name: passed(value) for name, value in reports.items()},
        "blockers": blockers,
        "note": "Structural archive validation does not replay archived ML fits. Missing semantic replay authentication, or any missing, malformed, stale, or failed qualification evidence, keeps the audit false.",
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"passed": audit_passed, "blockers": blockers}, indent=2))
    return 0 if audit_passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
