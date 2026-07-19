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
    "mga_reference_report.json",
    "segmentation_recovery_simulation_report.json",
    "v06_group_methods_reference_report.json",
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
            "groups": ["micom_constructs", "max_mga_difference"],
            "pos": ["segments", "objective_improvement"],
            "fimix": ["classes", "bic", "entropy"],
        }
        return all(section in sections and all(key in sections[section] for key in keys) for section, keys in required.items())
    return False


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    validate = run(["npm.cmd" if __import__("os").name == "nt" else "npm", "run", "qpls:v06:validate"])
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
    coverage = {
        "micom": "groups" in sections and sections.get("groups", {}).get("micom_constructs", 0) >= 1,
        "permutation_mga": "groups" in sections and "max_mga_difference" in sections.get("groups", {}),
        "pls_pos": "pos" in sections and sections.get("pos", {}).get("segments", 0) >= 2,
        "fimix": "fimix" in sections and sections.get("fimix", {}).get("classes", 0) >= 2,
    }
    cargo = run(["cargo", "test", "-p", "qpls-estimation"])
    passed = validate["passed"] and cargo["passed"] and all(item["passed"] for item in evidence) and all(item["present"] for item in docs) and all(coverage.values())
    report = {
        "schema_version": 1,
        "target": "v0.6 prediction and heterogeneity publication audit",
        "passed": passed,
        "evidence": evidence,
        "coverage": coverage,
        "docs": docs,
        "commands": [validate, cargo],
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
