"""Publication audit for v0.6 prediction and heterogeneity methods."""

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "prediction_heterogeneity_publication_audit.json"


REQUIRED_REPORTS = [
    "plspredict_holdout_reference_report.json",
    "ipma_reference_report.json",
    "ipma_method_promotion_audit.json",
    "mga_reference_report.json",
    "segmentation_recovery_simulation_report.json",
    "v06_group_methods_reference_report.json",
    "micom_method_promotion_audit.json",
]

REQUIRED_DOCS = [
    "PLSPREDICT_HOLDOUT_V1.md",
    "IPMA_V1.md",
    "PLS_MGA_TWO_GROUP_V1.md",
    "PLS_MGA_PERMUTATION_V1.md",
    "MICOM_V1.md",
    "FIMIX_PLS_V1.md",
    "PLS_POS_V1.md",
    "PLS_POS_BOUNDED_V1.md",
]


def run(command, timeout=360):
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    return {"command": command, "returncode": proc.returncode, "passed": proc.returncode == 0, "stdout_tail": proc.stdout[-3000:], "stderr_tail": proc.stderr[-3000:]}


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def report_passed(path):
    value = load(path)
    if value.get("passed") is True or value.get("status") == "passed":
        return True
    checks = value.get("checks")
    if isinstance(checks, dict):
        return all(item.get("passed") is True for item in checks.values() if isinstance(item, dict))
    sections = value.get("sections")
    if isinstance(sections, dict):
        required = {
            "groups": [
                "group_a",
                "group_b",
                "group_a_observations",
                "group_b_observations",
                "group_methods",
                "mga_method_version",
                "permutation_method_version",
                "usable_permutations",
                "max_abs_path_difference",
                "micom_execution_enabled",
            ],
            "pos": ["segments", "objective_improvement"],
            "fimix": ["classes", "bic", "entropy"],
        }
        if not all(section in sections and all(key in sections[section] for key in keys) for section, keys in required.items()):
            return False
        groups = sections["groups"]
        return (
            groups["group_a"] != groups["group_b"]
            and groups["group_a_observations"] >= 10
            and groups["group_b_observations"] >= 10
            and groups["group_methods"] == "mga_permutation"
            and groups["mga_method_version"] == "pls_mga_two_group_v1"
            and groups["permutation_method_version"] == "pls_mga_permutation_v1"
            and groups["usable_permutations"] >= 99
            and groups["max_abs_path_difference"] > 0
            and groups["micom_execution_enabled"] is False
        )
    return False


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    validate = run(["npm.cmd" if __import__("os").name == "nt" else "npm", "run", "qpls:v06:validate"])
    micom_withdrawal = run(["npm.cmd" if __import__("os").name == "nt" else "npm", "run", "qpls:promotion:micom"])
    ipma_promotion = run(["npm.cmd" if __import__("os").name == "nt" else "npm", "run", "qpls:promotion:ipma"])
    evidence = []
    for name in REQUIRED_REPORTS:
        path = RESULTS / name
        evidence.append({"path": str(path.relative_to(ROOT)), "present": path.exists(), "passed": path.exists() and report_passed(path)})
    docs = []
    for name in REQUIRED_DOCS:
        path = ROOT / "docs" / "methods" / name
        docs.append({"path": str(path.relative_to(ROOT)), "present": path.exists(), "bytes": path.stat().st_size if path.exists() else None})
    v06 = load(RESULTS / "v06_group_methods_reference_report.json") if (RESULTS / "v06_group_methods_reference_report.json").exists() else {}
    sections = v06.get("sections", {})
    groups = sections.get("groups", {})
    micom = load(RESULTS / "micom_method_promotion_audit.json") if (RESULTS / "micom_method_promotion_audit.json").exists() else {}
    ipma = load(RESULTS / "ipma_method_promotion_audit.json") if (RESULTS / "ipma_method_promotion_audit.json").exists() else {}
    coverage = {
        "ipma_native_workflow": ipma.get("passed") is True
        and any(
            item.get("name") == "packaged_native_workflow" and item.get("passed") is True
            for item in ipma.get("checks", [])
        ),
        "micom_safety_withdrawal": micom.get("passed") is True
        and micom.get("promotion_status") == "withdrawn"
        and micom.get("execution_enabled") is False,
        "permutation_mga": groups.get("group_methods") == "mga_permutation"
        and groups.get("group_a") != groups.get("group_b")
        and groups.get("permutation_method_version") == "pls_mga_permutation_v1"
        and groups.get("micom_execution_enabled") is False,
        "pls_pos": "pos" in sections and sections.get("pos", {}).get("segments", 0) >= 2,
        "fimix": "fimix" in sections and sections.get("fimix", {}).get("classes", 0) >= 2,
    }
    cargo = run(["cargo", "test", "-p", "qpls-estimation"])
    passed = validate["passed"] and micom_withdrawal["passed"] and ipma_promotion["passed"] and cargo["passed"] and all(item["passed"] for item in evidence) and all(item["present"] for item in docs) and all(coverage.values())
    report = {
        "schema_version": 1,
        "target": "v0.6 prediction and heterogeneity publication audit",
        "passed": passed,
        "evidence": evidence,
        "coverage": coverage,
        "docs": docs,
        "commands": [validate, micom_withdrawal, ipma_promotion, cargo],
        "note": "IPMA publication readiness requires the canonical predecessor-only reference plus genuine packaged-native run/export/save/reopen evidence. Permutation MGA remains limited to explicit Group A/B comparisons, and MICOM coverage means verified execution withdrawal rather than scientific promotion.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
