"""Fail-closed persistence boundary for the CB-SEM bootstrap v2 archive.

Structural checks and deterministic summary recomputation are useful, but they
do not authenticate stored replicate semantics without replaying each ML fit.
"""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "validation" / "results" / "cbsem_bootstrap_v2_release_persistence.json"


def main() -> int:
    project = (ROOT / "crates/qpls-project/src/lib.rs").read_text(encoding="utf-8")
    contract = (ROOT / "crates/qpls-estimation/src/cbsem_bootstrap_contract.rs").read_text(encoding="utf-8")
    docs = (ROOT / "docs/methods/CBSEM_BOOTSTRAP_V2.md").read_text(encoding="utf-8")
    manifest = (ROOT / "validation/methods/cbsem_bootstrap_v2.manifest.json").read_text(encoding="utf-8")
    validator = project.split("fn validate_cbsem_bootstrap_v2_payload_contract", 1)[-1].split("\nfn ", 1)[0]
    checks = {
        "archive_checksums_existing": "verify_archive_checksums" in project,
        "outer_recipe_settings_provenance_bound": "recipe.settings != result.provenance.settings" in project,
        "outer_dataset_fingerprint_bound": "recipe.dataset_fingerprint != result.provenance.dataset_fingerprint" in project,
        "v2_scientific_witness_fields_present": all(
            token in contract
            for token in (
                "dataset_fingerprint",
                "recipe_sha256",
                "base_result_sha256",
                "sample_indices_sha256",
            )
        ),
        "structural_v2_summary_validator_present": "validate_cbsem_bootstrap_v2_payload_contract" in project,
        "legacy_v1_read_only_test_present": "cbsem_bootstrap_v1_read_only" in project,
        "v2_tamper_mutation_tests_present": "cbsem_bootstrap_v2_rejects_tampering" in project,
        "coordinated_rewrite_gap_documented": "coordinated-rewrite gap is an explicit promotion blocker" in docs
        and "coordinated_witness_rewrite_not_replayed" in manifest,
    }
    full_ml_replay_present = "estimate_cbsem_case_resample_validated_with_control" in validator
    blockers = [key for key, value in checks.items() if not value]
    if not full_ml_replay_present:
        blockers.append("coordinated_witness_semantic_rewrite_not_replayed")
    qualified = all(checks.values()) and full_ml_replay_present
    report = {
        "kind": "cbsem_bootstrap_v2_persistence_v2",
        "passed": qualified,
        "feature_id": "qpls3.cbsem.bootstrap",
        "method_version": "cbsem_bootstrap_v2",
        "structural_checks_passed": all(checks.values()),
        "semantic_replay_authentication": "qualified" if full_ml_replay_present else "not_implemented",
        "checks": checks,
        "blockers": blockers,
        "note": (
            "The validator binds identities, provenance, accounting, sample-position hashes, "
            "and summaries derived from stored estimates. It does not currently rerun every "
            "sampled full-ML fit, so coordinated semantic witness rewrites are not authenticated."
        ),
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"passed": report["passed"], "blockers": blockers}, indent=2))
    return 0 if qualified else 1


if __name__ == "__main__":
    raise SystemExit(main())
