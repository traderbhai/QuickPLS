import csv
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULT = ROOT / "validation" / "results" / "pls_core_method_promotion_audit.json"


REQUIRED = {
    "method_spec": "docs/methods/PLS_PM_V1.md",
    "csem_reference": "validation/results/pls_csem_comparison.json",
    "plspm_reference": "validation/results/pls_plspm_comparison.json",
    "numpy_pca_reference": "validation/results/pls_pca_numpy_comparison.json",
    "published_csem_fixture": "validation/results/pls_csem_threecommonfactors_comparison.json",
    "publication_audit": "validation/results/pls_publication_audit.json",
    "stable_export_csv": "validation/results/pls_publication_stable_export.csv",
    "stable_export_audit": "validation/results/stable_export_publication_audit.json",
    "known_differences": "docs/KNOWN_DIFFERENCES.md",
    "method_compatibility": "docs/METHOD_COMPATIBILITY.md",
}


EXCLUDED_STABLE_SECTIONS = {
    "assessment",
    "bootstrap",
    "resampling",
    "permutation",
    "rho_a",
    "htmt",
    "ave",
    "composite_reliability",
    "cronbach_alpha",
}


def load_json(path: str) -> dict:
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def passed_json(path: str) -> bool:
    value = load_json(path)
    return value.get("passed") is True or value.get("status") == "passed"


def stable_export_sections(path: Path) -> set[str]:
    with path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        return {row.get("section", "").strip().lower() for row in reader}


def main():
    presence = {name: (ROOT / path).exists() for name, path in REQUIRED.items()}
    reference_status = {
        name: passed_json(path)
        for name, path in REQUIRED.items()
        if name
        in {
            "csem_reference",
            "plspm_reference",
            "numpy_pca_reference",
            "published_csem_fixture",
            "publication_audit",
            "stable_export_audit",
        }
        and (ROOT / path).exists()
    }
    export_path = ROOT / REQUIRED["stable_export_csv"]
    sections = stable_export_sections(export_path) if export_path.exists() else set()
    excluded_sections_present = sorted(sections.intersection(EXCLUDED_STABLE_SECTIONS))
    export_text = export_path.read_text(encoding="utf-8") if export_path.exists() else ""
    method_compat = (ROOT / REQUIRED["method_compatibility"]).read_text(encoding="utf-8")
    compatibility_updated = bool(
        re.search(
            r"PLS path modeling core.*Estimator-only output validated",
            method_compat,
            re.IGNORECASE,
        )
    )
    scope_decision = {
        "stable_output_scope": "PLS core estimator-only output",
        "excluded_from_this_promotion": [
            "assessment metrics",
            "bootstrap and resampling inference",
            "extended PLS diagnostics",
            "prediction, groups, CB-SEM, and extended methods",
        ],
        "full_run_envelope_dependency": (
            "Full PLS run-envelope researcher-ready status depends on assessment and inference "
            "promotion in the same v1.2 first batch."
        ),
    }
    checks = {
        "all_required_files_present": all(presence.values()),
        "all_reference_reports_pass": all(reference_status.values()) and len(reference_status) == 6,
        "stable_export_has_estimator_scope_note": "v0.3 validated estimator only" in export_text,
        "stable_export_excludes_experimental_sections": not excluded_sections_present,
        "method_compatibility_updated": compatibility_updated,
    }
    report = {
        "schema_version": 1,
        "target": "pls_core_estimator_output_promotion",
        "passed": all(checks.values()),
        "status": "validated",
        "scope_decision": scope_decision,
        "required_files": presence,
        "reference_status": reference_status,
        "stable_export_sections": sorted(sections),
        "excluded_sections_present": excluded_sections_present,
        "checks": checks,
        "note": (
            "This promotes only the deterministic PLS core estimator output. It does not promote "
            "assessment, inference/resampling, or any extended method payload."
        ),
    }
    RESULT.parent.mkdir(parents=True, exist_ok=True)
    RESULT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"wrote {RESULT} | passed={report['passed']}")
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
