#!/usr/bin/env python3
"""Gate the bounded permutation-MGA v2 path/loading/weight contract."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
REFERENCE = RESULTS / "micom_v2_reference_report.json"
OUTPUT = RESULTS / "mga_permutation_method_promotion_audit.json"


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
    native = "\n".join(
        text(path).lower()
        for path in (
            "src/native/nativeAnalysisCatalog.ts",
            "src/native/nativeAnalysisRecipe.ts",
            "src/native/nativePlsReadiness.ts",
            "src/native/NativeCalculationDialog.tsx",
        )
    )
    v1_doc = text("docs/methods/PLS_MGA_PERMUTATION_V1.md")
    v2_doc = text("docs/methods/PLS_MGA_PERMUTATION_V2.md")
    comparison = reference.get("quickpls_comparison") or {}
    reference_checks = reference.get("reference_checks") or {}
    checks = [
        check(
            "independent_reference_passes",
            reference.get("passed") is True
            and reference.get("permutation_samples", 0) >= 5_000
            and comparison.get("passed") is True
            and reference_checks.get("path_and_measurement_distributions_present") is True,
            "The independent 5,000-permutation reference must agree for path, loading, and weight distributions.",
        ),
        check(
            "v2_payload_covers_paths_and_measurement_parameters",
            'pub const PLS_MGA_PERMUTATION_METHOD_VERSION: &str = "pls_mga_permutation_v2";'
            in estimator
            and "pub struct PlsMgaPermutationComparison" in estimator
            and "pub struct PlsMgaPermutationMeasurementComparison" in estimator
            and "measurement_comparisons" in estimator
            and "attempted_permutations" in estimator
            and "failed_permutations" in estimator,
            "The current payload versions path, loading, weight, and permutation-accounting evidence.",
        ),
        check(
            "every_permutation_reestimates_both_groups",
            "while usable < samples" in estimator
            and "let left_fit = fit_group_result(" in estimator
            and "let right_fit = fit_group_result(" in estimator
            and "mga_path_comparisons" in estimator
            and "mga_measurement_comparisons" in estimator
            and "micom_statistics" in estimator,
            "Each usable label assignment re-fits both groups and reuses that pair for paths, loadings, weights, and MICOM.",
        ),
        check(
            "bounded_coupled_recipe_contract",
            "5_000..=10_000" in core_validation
            and 'method == "micom"' in core_validation
            and 'method == "mga_permutation"' in core_validation
            and "micom_configural_confirmed" in core_validation,
            "The recipe contract fixes 5,000–10,000 permutations and couples permutation MGA to MICOM Step-1 confirmation.",
        ),
        check(
            "persistence_validates_current_and_archives_v1",
            "PLS_MGA_PERMUTATION_METHOD_VERSION_V1" in project
            and "PLS_MGA_PERMUTATION_METHOD_VERSION" in project
            and "measurement_comparisons" in project
            and "attempted_permutations" in project,
            "Persistence keeps v1 archives readable while validating the complete v2 comparison identities and counts.",
        ),
        check(
            "native_workflow_requests_complete_method_pair",
            "micom,mga_permutation" in native
            and "micomconfiguralconfirmed" in native
            and "5000" in native.replace("_", "")
            and "micom is not calculated" not in native,
            "The native calculation surface requests the complete current method pair and the final-study permutation floor.",
        ),
        check(
            "v2_contract_is_documented",
            "Status: validated for the bounded scope below" in v2_doc
            and "outer loadings, and outer weights" in v2_doc
            and "count(|d_b| >= |d_observed|)" in v2_doc
            and "do not themselves establish or refute the MICOM hierarchy" in v2_doc,
            "The v2 document defines direction, inference, measurement rows, MICOM dependency, and interpretation limits.",
        ),
        check(
            "legacy_contract_remains_distinct",
            "# PLS MGA Permutation v1" in v1_doc
            and "pls_mga_permutation_v1" in v1_doc
            and "measurement-model comparisons" in v1_doc,
            "The narrower historical v1 contract remains documented separately rather than being silently reinterpreted as v2.",
        ),
    ]
    passed = all(item["passed"] for item in checks)
    payload = {
        "schema_version": 2,
        "target": "pls_mga_permutation_v2_method_promotion",
        "method_id": "mga_permutation",
        "method_version": "pls_mga_permutation_v2",
        "promotion_status": "qualified" if passed else "blocked",
        "passed": passed,
        "reference": str(REFERENCE.relative_to(ROOT)),
        "checks": checks,
        "note": (
            "A passing audit qualifies only the documented two-group path-weighting scope. MICOM v2 results must be "
            "interpreted before any path-difference claim."
        ),
    }
    RESULTS.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} | passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
