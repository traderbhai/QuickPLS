import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULT = ROOT / "validation" / "results" / "inference_method_promotion_audit.json"


REQUIRED = {
    "inference_publication_audit": "validation/results/inference_publication_audit.json",
    "inference_matrix": "validation/results/inference_publication_matrix.json",
    "monte_carlo": "validation/results/monte_carlo_qualification.json",
    "studentized_monte_carlo": "validation/results/monte_carlo_studentized_qualification.json",
    "worker_matrix": "validation/results/studentized_worker_matrix.json",
    "release_stress": "validation/results/studentized_release_stress.json",
    "supplied_reference": "validation/results/studentized_supplied_reference.json",
    "csem_bootstrap_reference": "validation/results/pls_bootstrap_external_reference.json",
    "csem_corporate_reference": "validation/results/pls_bootstrap_corporate_csem_reference.json",
    "plspm_bootstrap_reference": "validation/results/pls_bootstrap_plspm_external_reference.json",
    "quick_qualification": "validation/results/v04_inference_qualification_quick.json",
    "resampling_spec": "docs/methods/RESAMPLING_ENGINE_V4.md",
    "jackknife_spec": "docs/methods/JACKKNIFE_ENGINE_V1.md",
    "permutation_spec": "docs/methods/PERMUTATION_ENGINE_V1.md",
    "studentized_spec": "docs/methods/STUDENTIZED_BOOTSTRAP_V1.md",
    "known_differences": "docs/KNOWN_DIFFERENCES.md",
    "method_compatibility": "docs/METHOD_COMPATIBILITY.md",
}


PROMOTED_PROCEDURES = {
    "bootstrap_percentile",
    "bca",
    "studentized_bootstrap_t",
    "jackknife",
    "freedman_lane_permutation",
}


def load_json(path: str) -> dict:
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def passed_json(path: str) -> bool:
    value = load_json(path)
    if value.get("passed") is True or value.get("status") == "passed":
        return True
    if path.endswith("monte_carlo_qualification.json") or path.endswith("monte_carlo_studentized_qualification.json"):
        qualification = value.get("qualification", {})
        return qualification.get("evaluated") is True and qualification.get("passed") is True
    if path.endswith("studentized_worker_matrix.json"):
        return value.get("passed") is True or value.get("all_equal") is True
    if path.endswith("studentized_release_stress.json"):
        return value.get("passed") is True or value.get("qualification_passed") is True
    if path.endswith("v04_inference_qualification_quick.json"):
        return value.get("qualification_passed") is True
    return False


def main():
    presence = {name: (ROOT / path).exists() for name, path in REQUIRED.items()}
    artifact_status = {
        name: passed_json(path)
        for name, path in REQUIRED.items()
        if path.startswith("validation/results/") and (ROOT / path).exists()
    }
    matrix = load_json(REQUIRED["inference_matrix"]) if presence["inference_matrix"] else {}
    rows = matrix.get("rows", [])
    complete_procedures = {row.get("procedure") for row in rows if row.get("complete") is True}
    missing_procedures = sorted(PROMOTED_PROCEDURES - complete_procedures)
    incomplete_rows = [row.get("procedure") for row in rows if row.get("complete") is not True]
    method_compat = (ROOT / REQUIRED["method_compatibility"]).read_text(encoding="utf-8")
    compatibility_updated = bool(
        re.search(
            r"Inference/resampling.*Validated for documented PLS resampling scope",
            method_compat,
            re.IGNORECASE,
        )
    )
    scope_decision = {
        "stable_output_scope": "documented PLS resampling and inference procedures",
        "promoted_procedures": sorted(PROMOTED_PROCEDURES),
        "included_claims": [
            "fixed-seed reproducibility",
            "worker-count invariant analytical payloads where audited",
            "percentile, BCa, and studentized/bootstrap-t intervals under documented settings",
            "jackknife support for BCa and diagnostics",
            "Freedman-Lane path permutation under documented fixed-score linear nuisance assumptions",
        ],
        "excluded_from_this_promotion": [
            "unqualified small-sample performance claims outside audited scenarios",
            "unqualified non-normal claims outside audited normal/heavy-tail qualification cells",
            "resampling support for unsupported model shapes or failed-fit cases beyond documented diagnostics",
            "publication claims for methods whose base estimator remains experimental",
        ],
    }
    checks = {
        "all_required_files_present": all(presence.values()),
        "all_result_artifacts_pass": all(artifact_status.values()) and len(artifact_status) >= 11,
        "inference_matrix_passed": matrix.get("passed") is True,
        "promoted_procedures_complete": not missing_procedures,
        "no_incomplete_rows": not incomplete_rows,
        "method_compatibility_updated": compatibility_updated,
    }
    report = {
        "schema_version": 1,
        "target": "inference_resampling_promotion",
        "passed": all(checks.values()),
        "status": "validated",
        "scope_decision": scope_decision,
        "required_files": presence,
        "artifact_status": artifact_status,
        "procedure_rows": len(rows),
        "missing_promoted_procedures": missing_procedures,
        "incomplete_rows": incomplete_rows,
        "checks": checks,
        "note": (
            "This promotes documented PLS inference/resampling procedures only. It does not make "
            "unsupported model shapes, unaudited small-sample/non-normal settings, or experimental "
            "base methods publication-ready."
        ),
    }
    RESULT.parent.mkdir(parents=True, exist_ok=True)
    RESULT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"wrote {RESULT} | passed={report['passed']}")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
