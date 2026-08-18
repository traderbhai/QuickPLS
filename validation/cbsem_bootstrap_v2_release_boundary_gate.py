"""Fail-closed static boundary gate for CB-SEM bootstrap v2.

This gate proves the declared implementation boundary only. It cannot promote
the method without runtime reference/simulation/persistence/native evidence.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "validation" / "results" / "cbsem_bootstrap_v2_release_boundary.json"
SOURCES = (
    "crates/qpls-estimation/src/cbsem_bootstrap_contract.rs",
    "crates/qpls-resampling/src/cbsem_bootstrap.rs",
    "docs/methods/CBSEM_BOOTSTRAP_V2.md",
    "validation/methods/cbsem_bootstrap_v2.manifest.json",
    "validation/cbsem_bootstrap_v2_reference.py",
    "validation/cbsem_bootstrap_v2_release_simulation.py",
    "validation/cbsem_bootstrap_v2_release_boundary_gate.py",
    "validation/cbsem_bootstrap_v2_release_persistence_gate.py",
    "validation/cbsem_bootstrap_v2_method_promotion_audit.py",
    "validation/run_v252_cbsem_bootstrap_v2_native_acceptance.ps1",
    "src/native/nativeCbsemBootstrap.test.ts",
)


def descriptor(relative: str) -> dict[str, object]:
    path = ROOT / relative
    data = path.read_bytes()
    return {"path": relative, "size": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def main() -> int:
    engine = (ROOT / "crates/qpls-resampling/src/cbsem_bootstrap.rs").read_text(encoding="utf-8")
    production_engine = engine.split("#[cfg(test)]", 1)[0]
    contract = (ROOT / "crates/qpls-estimation/src/cbsem_bootstrap_contract.rs").read_text(encoding="utf-8")
    legacy = (ROOT / "crates/qpls-estimation/src/pls.rs").read_text(encoding="utf-8")
    checks = {
        "v2_identity_separate": "CBSEM_BOOTSTRAP_METHOD_VERSION_V2" in contract
        and '"cbsem_bootstrap_v2"' in contract,
        "legacy_v1_still_separate": 'CBSEM_BOOTSTRAP_METHOD_VERSION: &str = "cbsem_bootstrap_v1"' in legacy
        and "pub bootstrap: Option<CbsemBootstrapAnalysis>" in legacy
        and "pub bootstrap_v2: Option<CbsemBootstrapAnalysisV2>" in legacy,
        "runtime_minimum_1000": "const MINIMUM_REPLICATES: u32 = 1_000" in engine,
        "runtime_maximum_10000": "const MAXIMUM_REPLICATES: u32 = 10_000" in engine,
        "full_ml_refit_api_called": "estimate_cbsem_case_resample_validated_with_control" in engine,
        "normal_theory_not_used": "1.96" not in production_engine
        and "standard_error.unwrap_or" not in production_engine,
        "type7_percentile_explicit": '"percentile_type7_v1"' in contract
        and "quantile_type7" in engine,
        "confidence_frozen_95": "confidence_level.to_bits() != 0.95_f64.to_bits()" in engine,
        "retry_is_fixed_and_bounded": "CBSEM_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V2: u8 = 1" in contract
        and "no_retry_fixed_preplanned_primary_draws_v1" in contract,
        "base_identity_bound": all(
            token in contract
            for token in ("dataset_fingerprint", "recipe_sha256", "base_result_sha256")
        ),
        "below_threshold_payload_is_retained": "CbsemBootstrapInferenceV2" in contract
        and "insufficient_usable_replicates" in engine
        and "InsufficientUsableReplicates" not in engine,
        "workers_not_analytical_field": "pub workers:" not in contract,
        "semantic_replay_claim_boundary_explicit": "coordinated-rewrite gap is an explicit promotion blocker" in (
            ROOT / "docs/methods/CBSEM_BOOTSTRAP_V2.md"
        ).read_text(encoding="utf-8")
        and "coordinated_witness_rewrite_not_replayed" in (
            ROOT / "validation/methods/cbsem_bootstrap_v2.manifest.json"
        ).read_text(encoding="utf-8"),
        "multigroup_not_implemented": "CbsemMultigroup" not in contract and "group-specific" not in engine,
    }
    report = {
        "kind": "cbsem_bootstrap_v2_boundary_v1",
        "passed": all(checks.values()),
        "feature_id": "qpls3.cbsem.bootstrap",
        "method_version": "cbsem_bootstrap_v2",
        "claim_scope": (
            "Static engine and structural archive boundary only; this passing report does not "
            "authenticate the semantic truth of stored replicate fits or failure meanings."
        ),
        "checks": checks,
        "sources": [descriptor(relative) for relative in SOURCES],
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"passed": report["passed"], "output": str(OUTPUT)}, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
