"""Publication audit for v0.4 assessment metrics."""

import csv
import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "assessment_publication_audit.json"
MATRIX = RESULTS / "assessment_publication_metric_matrix.json"


def run(command, timeout=240):
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    return {
        "command": command,
        "returncode": proc.returncode,
        "passed": proc.returncode == 0,
        "stdout_tail": proc.stdout[-3000:],
        "stderr_tail": proc.stderr[-3000:],
    }


def load(path):
    return json.loads((ROOT / path).read_text(encoding="utf-8"))


def report_passed(path):
    value = load(path)
    if value.get("passed") is True or value.get("status") == "passed":
        return True
    if path.endswith("v04_assessment_evidence.json"):
        return value.get("open_registry_gates") == [] and value.get("all_listed_artifacts_present") is True
    if path.endswith("rho_a_primary_dijkstra_henseler_2015.json"):
        fixtures = value.get("fixtures", [])
        return value.get("method_version") == "dijkstra_henseler_rho_a_v1" and len(fixtures) >= 2
    if path.endswith("htmt_reference.json"):
        fixtures = value.get("fixtures", [])
        metamorphic = value.get("metamorphic_checks", [])
        return (
            value.get("method_versions", {}).get("htmt_plus") == "ringle_et_al_htmt_plus_v1"
            and value.get("method_versions", {}).get("htmt_original") == "henseler_et_al_htmt_v1"
            and len(fixtures) >= 1
            and len(metamorphic) >= 1
        )
    return False


METRICS = [
    ("cronbach_alpha", "construct_quality[].cronbach_alpha", "hand/cSEM", "1e-6", "validation/results/assessment_csem_comparison.json", "standardized alpha convention documented"),
    ("rho_a", "construct_quality[].rho_a", "Dijkstra-Henseler/cSEM", "1e-6", "validation/results/rho_a_csem_comparison.json", "two-indicator and improper values warn explicitly"),
    ("rho_c", "construct_quality[].rho_c", "cSEM", "1e-6", "validation/results/assessment_csem_comparison.json", "formative not applicable"),
    ("ave", "construct_quality[].ave", "cSEM", "1e-6", "validation/results/assessment_csem_comparison.json", "formative not applicable"),
    ("htmt_original", "htmt_original", "cSEM/hand", "1e-6", "validation/results/htmt_csem_comparison.json", "signed original HTMT semantics"),
    ("htmt_plus", "htmt_plus", "seminr/Ringle examples", "4 decimals for published examples", "validation/results/htmt_seminr_comparison.json", "cSEM absolute HTMT is documented non-equivalent for mixed signs"),
    ("structural_vif", "structural_vif", "cSEM/auxiliary regression", "1e-6", "validation/results/assessment_csem_comparison.json", "perfect explanation warns"),
    ("r_squared", "structural_quality[].r_squared", "cSEM", "1e-6", "validation/results/assessment_csem_comparison.json", "fixed-score contract"),
    ("adjusted_r_squared", "structural_quality[].adjusted_r_squared", "cSEM", "1e-6", "validation/results/assessment_csem_comparison.json", "fixed-score contract"),
    ("f_squared", "f_squared", "fixed-score cSEM-compatible regression", "1e-6", "validation/results/assessment_csem_comparison.json", "single predictor intercept-only exclusion documented"),
    ("q_squared", "blindfolding", "independent Python", "1e-6", "validation/results/blindfolding_python_comparison.json", "omission-distance limitations documented"),
    ("srmr_duls", "model_fit", "cSEM/hand residuals", "1e-6", "validation/results/assessment_csem_comparison.json", "correlation-residual fit convention"),
]


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    required = [
        "validation/results/v04_assessment_evidence.json",
        "validation/results/rho_a_csem_comparison.json",
        "validation/results/rho_a_primary_dijkstra_henseler_2015.json",
        "validation/results/htmt_reference.json",
        "validation/results/htmt_csem_comparison.json",
        "validation/results/htmt_seminr_comparison.json",
        "validation/results/assessment_csem_comparison.json",
        "validation/results/assessment_published_satisfaction_comparison.json",
        "validation/results/assessment_simulation_report.json",
        "validation/results/blindfolding_python_comparison.json",
    ]
    evidence = []
    for path in required:
        full = ROOT / path
        evidence.append({"path": path, "present": full.exists(), "passed": full.exists() and report_passed(path)})
    matrix_rows = [
        {
            "metric": metric,
            "formula_spec": "docs/methods/PLS_ASSESSMENT_V1.md or metric-specific v0.4 method docs",
            "quickpls_field": field,
            "reference_source": ref,
            "tolerance": tolerance,
            "evidence_file": evidence_file,
            "known_difference_or_unsupported_case": known,
            "complete": bool(metric and field and ref and tolerance and evidence_file and known) and (ROOT / evidence_file).exists(),
        }
        for metric, field, ref, tolerance, evidence_file, known in METRICS
    ]
    MATRIX.write_text(json.dumps({"schema_version": 1, "rows": matrix_rows, "passed": all(row["complete"] for row in matrix_rows)}, indent=2), encoding="utf-8")
    export_csv = RESULTS / "assessment_publication_export_check.csv"
    export_command = run(["cargo", "run", "-p", "qpls-cli", "--", "export", "validation/results/pls_quickpls_path_mode_a.json", "--format", "csv", "--output", str(export_csv.relative_to(ROOT)), "--include-experimental"])
    export_text = export_csv.read_text(encoding="utf-8") if export_csv.exists() else ""
    export_precision = {
        "path": str(export_csv.relative_to(ROOT)),
        "exists": export_csv.exists(),
        "has_alpha_or_quality_rows": "construct_quality" in export_text or "rho_a" in export_text,
        "has_supported_scope_warning": "QuickPLS v0.9.0-rc.1 supported scope" in export_text,
    }
    cargo = run(["cargo", "test", "-p", "qpls-assessment"])
    passed = all(item["passed"] for item in evidence) and all(row["complete"] for row in matrix_rows) and export_command["passed"] and export_precision["exists"] and export_precision["has_supported_scope_warning"] and cargo["passed"]
    report = {
        "schema_version": 1,
        "target": "v0.4 assessment publication audit",
        "passed": passed,
        "evidence": evidence,
        "metric_matrix": {"path": str(MATRIX.relative_to(ROOT)), "rows": len(matrix_rows), "passed": all(row["complete"] for row in matrix_rows)},
        "export_precision_check": export_precision,
        "commands": [export_command, cargo],
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
