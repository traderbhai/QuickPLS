#!/usr/bin/env python3
"""Gate promotion of MICOM v2 without reviving the withdrawn micom_v1 claim."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
REFERENCE = RESULTS / "micom_v2_reference_report.json"
OUTPUT = RESULTS / "micom_method_promotion_audit.json"


def text(path: str) -> str:
    candidate = ROOT / path
    return candidate.read_text(encoding="utf-8") if candidate.exists() else ""


def load(path: Path) -> dict:
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}


def check(name: str, passed: bool, detail: str) -> dict:
    return {"name": name, "passed": bool(passed), "detail": detail}


def main() -> int:
    reference = load(REFERENCE)
    estimator = text("crates/qpls-estimation/src/pls.rs")
    core_validation = text("crates/qpls-core/src/validation.rs")
    project = text("crates/qpls-project/src/lib.rs")
    runner = text("crates/qpls-runner/src/lib.rs")
    native = "\n".join(
        text(path).lower()
        for path in (
            "src/native/nativeAnalysisCatalog.ts",
            "src/native/nativeAnalysisRecipe.ts",
            "src/native/nativePlsReadiness.ts",
            "src/native/NativeCalculationDialog.tsx",
        )
    )
    v1_doc = text("docs/methods/MICOM_V1.md")
    v2_doc = text("docs/methods/MICOM_V2.md")
    reference_comparison = reference.get("quickpls_comparison") or {}
    checks = [
        check(
            "independent_reference_passes",
            reference.get("passed") is True
            and reference.get("reference_passed") is True
            and reference.get("promotion_sample_size") is True
            and reference.get("permutation_samples", 0) >= 5_000
            and reference_comparison.get("passed") is True,
            "The independent NumPy reference must agree with a current QuickPLS result at 5,000 or more usable permutations.",
        ),
        check(
            "current_method_version_and_three_step_payload",
            'pub const MICOM_METHOD_VERSION: &str = "micom_v2";' in estimator
            and "pub struct MicomAnalysis" in estimator
            and "compositional_correlation_lower" in estimator
            and "mean_difference_lower" in estimator
            and "variance_difference_upper" in estimator
            and "partial_invariance" in estimator
            and "full_invariance" in estimator,
            "The estimator must emit the versioned MICOM v2 hierarchy, statistics, bounds, and decisions.",
        ),
        check(
            "pooled_score_and_group_refit_contract",
            "fn pooled_composite_scores(" in estimator
            and "fn micom_location_dispersion(" in estimator
            and "fn align_group_result_to_pooled(" in estimator
            and "&pooled_fit" in estimator
            and 'group_method_requested(recipe, "micom")' in estimator,
            "Step 2 applies group weights to pooled indicators; Step 3 uses pooled scores; every group fit is orientation-aligned.",
        ),
        check(
            "recipe_scope_and_configural_confirmation",
            "micom_configural_confirmed" in core_validation
            and "5_000..=10_000" in core_validation
            and 'method == "micom"' in core_validation
            and 'method == "mga_permutation"' in core_validation,
            "Core validation requires explicit Step-1 confirmation, both coupled methods, and 5,000–10,000 permutations.",
        ),
        check(
            "new_results_persist_as_v2_only",
            "MICOM_METHOD_VERSION_V1" in project
            and "MICOM_METHOD_VERSION" in project
            and "validate_micom_v2" in project
            and "historical" in project.lower()
            and "MICOM_METHOD_VERSION" in runner,
            "Persistence accepts historical archives but validates current payloads and prevents new micom_v1 provenance.",
        ),
        check(
            "native_workflow_exposes_truthful_confirmation",
            "micom,mga_permutation" in native
            and "micomconfiguralconfirmed" in native
            and "5000" in native.replace("_", "")
            and "micom is not calculated" not in native,
            "The native MGA workflow requests the coupled methods, requires the configural review, and no longer carries the withdrawn disclosure.",
        ),
        check(
            "v2_contract_is_documented",
            "Status: validated for the bounded scope below" in v2_doc
            and "cor(X a_A, X a_B)" in v2_doc
            and "log(var(C_pooled,A) / var(C_pooled,B))" in v2_doc
            and "reference-only mode is intentionally non-promotable" in v2_doc,
            "MICOM v2 documentation states equations, hierarchy, bounded scope, and independent evidence requirements.",
        ),
        check(
            "micom_v1_withdrawal_is_preserved",
            "Status: withdrawn and execution-disabled" in v1_doc
            and "prior QuickPLS v1.2.2 validation claim is withdrawn" in v1_doc
            and "micom_v1" in v1_doc,
            "The historical invalid implementation remains explicitly withdrawn and archive-only.",
        ),
    ]
    passed = all(item["passed"] for item in checks)
    payload = {
        "schema_version": 3,
        "target": "micom_v2_method_promotion",
        "method_id": "micom",
        "method_version": "micom_v2",
        "promotion_status": "qualified" if passed else "blocked",
        "execution_enabled": 'pub const MICOM_METHOD_VERSION: &str = "micom_v2";' in estimator,
        "passed": passed,
        "reference": str(REFERENCE.relative_to(ROOT)),
        "checks": checks,
        "note": (
            "A passing audit qualifies only the bounded MICOM v2 contract. It never restores the withdrawn micom_v1 claim, "
            "and packaged-desktop release evidence remains a separate gate."
        ),
    }
    RESULTS.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
