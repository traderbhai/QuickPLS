"""Publication audit for v0.8 extended methods."""

import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "extended_methods_publication_audit.json"


REQUIRED_DOCS = ["PCA_V1.md", "REGRESSION_OLS_V1.md", "REGRESSION_LOGISTIC_V1.md", "PROCESS_V1.md", "NCA_V1.md", "GSCA_V1.md"]


def run(command, timeout=360):
    proc = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=timeout)
    return {"command": command, "returncode": proc.returncode, "passed": proc.returncode == 0, "stdout_tail": proc.stdout[-3000:], "stderr_tail": proc.stderr[-3000:]}


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    validate = run(["npm.cmd" if __import__("os").name == "nt" else "npm", "run", "qpls:v08:validate"])
    evidence_command = run(["cargo", "run", "-p", "qpls-cli", "--", "evidence", "v08-extended-methods"])
    cargo = run(["cargo", "test", "-p", "qpls-estimation"])
    reference_path = RESULTS / "v08_extended_methods_reference_report.json"
    reference = load(reference_path) if reference_path.exists() else {}
    checks = reference.get("checks", {})
    method_coverage = {name: isinstance(item, dict) and item.get("passed") is True for name, item in checks.items()}
    expected = ["pca", "ols", "logistic", "process", "nca", "gsca"]
    docs = []
    for name in REQUIRED_DOCS:
        path = ROOT / "docs" / "methods" / name
        docs.append({"path": str(path.relative_to(ROOT)), "present": path.exists(), "bytes": path.stat().st_size if path.exists() else None})
    passed = validate["passed"] and evidence_command["passed"] and cargo["passed"] and reference.get("passed") is True and all(method_coverage.get(name) for name in expected) and all(item["present"] for item in docs)
    report = {
        "schema_version": 1,
        "target": "v0.8 extended methods publication audit",
        "passed": passed,
        "method_coverage": method_coverage,
        "docs": docs,
        "commands": [validate, evidence_command, cargo],
        "note": "Supported publication scope is bounded to documented v0.8 cases; unsupported or preview-limited cases remain guarded.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
