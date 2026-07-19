"""Publication audit for v0.5 extended PLS methods."""

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "extended_pls_publication_audit.json"


REQUIRED_REPORTS = [
    "mediation_reference_report.json",
    "mediation_r_reference_report.json",
    "mediation_published_example_report.json",
    "mediation_metamorphic_report.json",
    "mediation_randomization_report.json",
    "moderation_reference_report.json",
    "moderation_r_reference_report.json",
    "moderation_published_formula_report.json",
    "moderation_published_empirical_report.json",
    "moderation_simulation_report.json",
    "moderation_inference_report.json",
    "moderation_inference_qualification_report.json",
    "moderation_coverage_qualification_report.json",
    "higher_order_reference_report.json",
    "higher_order_metamorphic_report.json",
    "higher_order_two_stage_reference_report.json",
    "higher_order_hybrid_reference_report.json",
    "higher_order_hybrid_guard_report.json",
    "plsc_reference_report.json",
    "plsc_unsupported_guard_report.json",
    "wpls_reference_report.json",
    "cca_reference_report.json",
    "cta_pls_reference_report.json",
    "endogeneity_reference_report.json",
    "nonlinear_effects_reference_report.json",
    "moderated_mediation_reference_report.json",
    "extended_pls_unsupported_guard_report.json",
    "v05_extended_pls_evidence.json",
]


REQUIRED_DOCS = [
    "PLS_MEDIATION_V1.md",
    "PLS_TWO_STAGE_MODERATION_V1.md",
    "PLS_MODERATED_MEDIATION_V1.md",
    "PLS_HIGHER_ORDER_V1.md",
    "PLSC_V1.md",
    "PLS_WPLS_V1.md",
    "PLS_CCA_V1.md",
    "PLS_CTA_PLS_V1.md",
    "PLS_GAUSSIAN_COPULA_ENDOGENEITY_V1.md",
    "PLS_NONLINEAR_EFFECTS_V1.md",
    "PLS_CONTROLS_V1.md",
]


def run(command, timeout=300):
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    return {
        "command": command,
        "returncode": proc.returncode,
        "passed": proc.returncode == 0,
        "stdout_tail": proc.stdout[-3000:],
        "stderr_tail": proc.stderr[-3000:],
    }


def load_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def report_passed(path):
    value = load_json(path)
    if value.get("passed") is True or value.get("status") == "passed":
        return True
    if value.get("all_listed_artifacts_passed") is True and value.get("all_listed_artifacts_present") is True:
        return True
    if value.get("qualification_passed") is True:
        return True
    checks = value.get("checks")
    if isinstance(checks, dict):
        return all(item.get("passed") is True for item in checks.values() if isinstance(item, dict))
    if isinstance(checks, list):
        return all(item.get("passed") is True or item.get("status") == "passed" for item in checks if isinstance(item, dict))
    return False


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    evidence_command = run(["cargo", "run", "-p", "qpls-cli", "--", "evidence", "v05-extended-pls"])
    guard_command = run(["npm.cmd" if __import__("os").name == "nt" else "npm", "run", "qpls:extended-pls:unsupported-guard"])
    cargo = run(["cargo", "test", "-p", "qpls-estimation"])
    reports = []
    for name in REQUIRED_REPORTS:
        path = RESULTS / name
        reports.append({"path": str(path.relative_to(ROOT)), "present": path.exists(), "passed": path.exists() and report_passed(path)})
    docs = []
    for name in REQUIRED_DOCS:
        path = ROOT / "docs" / "methods" / name
        docs.append({"path": str(path.relative_to(ROOT)), "present": path.exists(), "bytes": path.stat().st_size if path.exists() else None})
    method_families = {
        "mediation": ["mediation_reference_report.json", "mediation_r_reference_report.json", "mediation_published_example_report.json", "mediation_metamorphic_report.json", "mediation_randomization_report.json"],
        "moderation": ["moderation_reference_report.json", "moderation_r_reference_report.json", "moderation_published_formula_report.json", "moderation_published_empirical_report.json", "moderation_simulation_report.json", "moderation_inference_report.json", "moderation_inference_qualification_report.json", "moderation_coverage_qualification_report.json"],
        "higher_order": ["higher_order_reference_report.json", "higher_order_metamorphic_report.json", "higher_order_two_stage_reference_report.json", "higher_order_hybrid_reference_report.json", "higher_order_hybrid_guard_report.json"],
        "plsc_wpls_cca_cta": ["plsc_reference_report.json", "plsc_unsupported_guard_report.json", "wpls_reference_report.json", "cca_reference_report.json", "cta_pls_reference_report.json"],
        "endogeneity_nonlinear_controls": ["endogeneity_reference_report.json", "nonlinear_effects_reference_report.json", "extended_pls_unsupported_guard_report.json"],
        "moderated_mediation": ["moderated_mediation_reference_report.json"],
    }
    family_matrix = {
        family: all((RESULTS / report).exists() and report_passed(RESULTS / report) for report in reports_for_family)
        for family, reports_for_family in method_families.items()
    }
    passed = (
        evidence_command["passed"]
        and guard_command["passed"]
        and cargo["passed"]
        and all(item["passed"] for item in reports)
        and all(item["present"] for item in docs)
        and all(family_matrix.values())
    )
    report = {
        "schema_version": 1,
        "target": "v0.5 extended PLS publication audit",
        "passed": passed,
        "reports": reports,
        "method_family_matrix": family_matrix,
        "docs": docs,
        "commands": [evidence_command, guard_command, cargo],
        "note": "This audit promotes the currently implemented v0.5 scope only; unsupported cases remain documented and guarded.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
