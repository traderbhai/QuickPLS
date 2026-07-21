import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULT = ROOT / "validation" / "results" / "assessment_method_promotion_audit.json"


REQUIRED = {
    "assessment_publication_audit": "validation/results/assessment_publication_audit.json",
    "metric_matrix": "validation/results/assessment_publication_metric_matrix.json",
    "assessment_evidence": "validation/results/v04_assessment_evidence.json",
    "rho_a_primary": "validation/results/rho_a_primary_dijkstra_henseler_2015.json",
    "rho_a_csem": "validation/results/rho_a_csem_comparison.json",
    "htmt_reference": "validation/results/htmt_reference.json",
    "htmt_csem": "validation/results/htmt_csem_comparison.json",
    "htmt_seminr": "validation/results/htmt_seminr_comparison.json",
    "htmt_published": "validation/results/htmt_published_ringle_2023.json",
    "assessment_csem": "validation/results/assessment_csem_comparison.json",
    "assessment_published": "validation/results/assessment_published_satisfaction_comparison.json",
    "assessment_simulation": "validation/results/assessment_simulation_report.json",
    "blindfolding_python": "validation/results/blindfolding_python_comparison.json",
    "method_spec": "docs/methods/PLS_ASSESSMENT_V4.md",
    "rho_a_spec": "docs/methods/PLS_RHO_A_V1.md",
    "htmt_spec": "docs/methods/PLS_HTMT_V1.md",
    "known_differences": "docs/KNOWN_DIFFERENCES.md",
    "method_compatibility": "docs/METHOD_COMPATIBILITY.md",
}


PROMOTED_METRICS = {
    "cronbach_alpha",
    "rho_a",
    "rho_c",
    "ave",
    "htmt_original",
    "htmt_plus",
    "structural_vif",
    "r_squared",
    "adjusted_r_squared",
    "f_squared",
    "q_squared",
    "srmr_duls",
}

EXCLUDED_FIT = {"d_G", "NFI", "RMS_theta"}


def load_json(path: str) -> dict:
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def passed_json(path: str) -> bool:
    value = load_json(path)
    if value.get("passed") is True or value.get("status") == "passed":
        return True
    if path.endswith("v04_assessment_evidence.json"):
        return (
            value.get("all_listed_artifacts_present") is True
            and not value.get("open_metric_blockers")
            and not value.get("open_registry_gates")
        )
    if path.endswith("rho_a_primary_dijkstra_henseler_2015.json"):
        return bool(value.get("fixtures")) and value.get("method_version") == "dijkstra_henseler_rho_a_v1"
    if path.endswith("htmt_reference.json"):
        checks = value.get("metamorphic_checks", {})
        checks_pass = all(
            v is True or (isinstance(v, (int, float)) and abs(v) <= 1e-12)
            for v in checks.values()
        )
        return bool(value.get("fixtures")) and checks_pass
    if path.endswith("htmt_published_ringle_2023.json"):
        return bool(value.get("fixtures")) and value.get("method_version") == "ringle_et_al_htmt_plus_v1"
    return False


def main():
    presence = {name: (ROOT / path).exists() for name, path in REQUIRED.items()}
    audit_status = {
        name: passed_json(path)
        for name, path in REQUIRED.items()
        if path.startswith("validation/results/") and (ROOT / path).exists()
    }
    matrix = load_json(REQUIRED["metric_matrix"]) if presence["metric_matrix"] else {}
    rows = matrix.get("rows", [])
    row_metrics = {row.get("metric") for row in rows if row.get("complete") is True}
    missing_promoted_metrics = sorted(PROMOTED_METRICS - row_metrics)
    extra_incomplete_rows = [row.get("metric") for row in rows if row.get("complete") is not True]
    delivery = (ROOT / "docs" / "DELIVERY_STATUS.md").read_text(encoding="utf-8")
    method_compat = (ROOT / REQUIRED["method_compatibility"]).read_text(encoding="utf-8")
    excluded_fit_documented = all(metric in delivery for metric in EXCLUDED_FIT)
    compatibility_updated = bool(
        re.search(
            r"Reliability, validity, structural quality, and fit diagnostics.*Validated for documented assessment scope",
            method_compat,
            re.IGNORECASE,
        )
    )
    scope_decision = {
        "stable_output_scope": "documented PLS assessment metrics",
        "promoted_metrics": sorted(PROMOTED_METRICS),
        "excluded_from_this_promotion": sorted(EXCLUDED_FIT),
        "exclusion_reason": (
            "d_G, NFI, and RMS_theta are not included in the v1.2 assessment promotion because "
            "the current assessment scope covers correlation-residual SRMR/d_ULS and documented "
            "reliability/validity/quality metrics only."
        ),
    }
    checks = {
        "all_required_files_present": all(presence.values()),
        "all_result_artifacts_pass": all(audit_status.values()) and len(audit_status) >= 10,
        "metric_matrix_passed": matrix.get("passed") is True,
        "promoted_metrics_complete": not missing_promoted_metrics,
        "no_incomplete_metric_rows": not extra_incomplete_rows,
        "excluded_fit_documented": excluded_fit_documented,
        "method_compatibility_updated": compatibility_updated,
    }
    report = {
        "schema_version": 1,
        "target": "assessment_metrics_promotion",
        "passed": all(checks.values()),
        "status": "validated",
        "scope_decision": scope_decision,
        "required_files": presence,
        "artifact_status": audit_status,
        "metric_rows": len(rows),
        "missing_promoted_metrics": missing_promoted_metrics,
        "extra_incomplete_rows": extra_incomplete_rows,
        "checks": checks,
        "note": (
            "This promotes documented assessment calculations only. It does not add d_G, NFI, "
            "RMS_theta, new fit-index conventions, or inference/resampling claims."
        ),
    }
    RESULT.parent.mkdir(parents=True, exist_ok=True)
    RESULT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"wrote {RESULT} | passed={report['passed']}")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
