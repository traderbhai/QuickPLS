"""Publication audit for v0.7 CB-SEM/CFA."""

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "cbsem_publication_audit.json"


REQUIRED_DOCS = [
    "CBSEM_ML_V1.md",
    "CFA_ML_V1.md",
    "CBSEM_FIT_V1.md",
    "CBSEM_MODIFICATION_INDICES_V1.md",
    "CBSEM_MULTIGROUP_INVARIANCE_V1.md",
]


def run(command, timeout=420):
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    return {"command": command, "returncode": proc.returncode, "passed": proc.returncode == 0, "stdout_tail": proc.stdout[-3000:], "stderr_tail": proc.stderr[-3000:]}


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    validate = run(["npm.cmd" if __import__("os").name == "nt" else "npm", "run", "qpls:v07:validate"])
    evidence_command = run(["cargo", "run", "-p", "qpls-cli", "--", "evidence", "v07-cbsem"])
    reference_path = RESULTS / "cbsem_v07_reference_report.json"
    lavaan_path = RESULTS / "cbsem_lavaan_reference_report.json"
    reference = load(reference_path) if reference_path.exists() else {}
    lavaan = load(lavaan_path) if lavaan_path.exists() else {}
    sections = reference.get("sections", {})
    required_sections = ["cfa", "sem", "fit", "mi", "bootstrap", "multigroup", "export", "guard"]
    section_coverage = {section: section in sections for section in required_sections}
    lavaan_models = lavaan.get("models", [])
    lavaan_coverage = {
        "status_passed": lavaan.get("status") == "passed" or lavaan.get("passed") is True,
        "model_count": len(lavaan_models),
        "all_models_passed": all(model.get("passed") is True or model.get("status") == "passed" for model in lavaan_models),
    }
    docs = []
    for name in REQUIRED_DOCS:
        path = ROOT / "docs" / "methods" / name
        docs.append({"path": str(path.relative_to(ROOT)), "present": path.exists(), "bytes": path.stat().st_size if path.exists() else None})
    cargo = run(["cargo", "test", "-p", "qpls-estimation"])
    passed = (
        validate["passed"]
        and evidence_command["passed"]
        and cargo["passed"]
        and reference.get("status") == "passed"
        and all(section_coverage.values())
        and lavaan_coverage["status_passed"]
        and lavaan_coverage["model_count"] >= 6
        and lavaan_coverage["all_models_passed"]
        and all(item["present"] for item in docs)
    )
    report = {
        "schema_version": 1,
        "target": "v0.7 CB-SEM/CFA publication audit",
        "passed": passed,
        "section_coverage": section_coverage,
        "lavaan_coverage": lavaan_coverage,
        "docs": docs,
        "commands": [validate, evidence_command, cargo],
        "note": "Supported publication scope is bounded to the documented raw-data reflective ML CFA/SEM cases; unsupported estimators and constraints remain blocked or experimental.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
