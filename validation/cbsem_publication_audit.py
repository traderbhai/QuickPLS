"""Publication audit for the bounded v1.2.4 CB-SEM/CFA scope."""

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
    legacy_evidence_path = RESULTS / "v07_cbsem_evidence.json"
    legacy_evidence = {
        "path": str(legacy_evidence_path.relative_to(ROOT)),
        "present": legacy_evidence_path.exists(),
        "bytes": legacy_evidence_path.stat().st_size if legacy_evidence_path.exists() else None,
        "authoritative_for_current_promotion": False,
    }
    reference_path = RESULTS / "cbsem_v07_reference_report.json"
    lavaan_path = RESULTS / "cbsem_lavaan_reference_report.json"
    reference = load(reference_path) if reference_path.exists() else {}
    lavaan = load(lavaan_path) if lavaan_path.exists() else {}
    sections = reference.get("sections", {})
    required_sections = ["cfa", "sem", "fit", "mi", "bootstrap", "multigroup", "export", "guard"]
    section_coverage = {section: section in sections for section in required_sections}
    lavaan_models = lavaan.get("models", [])
    native_path = RESULTS / "v247_tauri_native_acceptance.json"
    native = load(native_path) if native_path.exists() else {}
    native_checks = native.get("checks", {})
    native_result = native_checks.get("cbsemResult", {})
    native_archive = native_checks.get("cbsemSaveReopen", {}).get("archive", {})
    native_export = native_checks.get("cbsemExport", {}).get("nativeXlsx", {})
    packaged_native_coverage = {
        "report_passed": native.get("passed") is True,
        "genuine_result": native_result.get("initialSelectedTable") == "cbsem_fit" and native_result.get("noPlaceholder") is True,
        "xlsx_verified": native_export.get("attempted") is True and native_export.get("file", {}).get("isFile") is True,
        "archive_verified": native_archive.get("cbsem", {}).get("fitContract") is True and native_archive.get("cbsem", {}).get("modificationContract") is True,
        "same_run_reopened": native_checks.get("cbsemSaveReopen", {}).get("sameRunRestored") is True,
    }
    unsupported_guard_coverage = {
        "bootstrap_blocked": sections.get("bootstrap", {}).get("execution_enabled") is False
        and sections.get("bootstrap", {}).get("guard_codes") == ["cbsem.bootstrap_unsupported"],
        "multigroup_blocked": sections.get("multigroup", {}).get("execution_enabled") is False
        and sections.get("multigroup", {}).get("guard_codes") == ["cbsem.mean_structure_unsupported", "cbsem.multigroup_unsupported"],
    }
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
        and legacy_evidence["present"]
        and cargo["passed"]
        and reference.get("status") == "passed"
        and all(section_coverage.values())
        and lavaan_coverage["status_passed"]
        and lavaan_coverage["model_count"] >= 6
        and lavaan_coverage["all_models_passed"]
        and all(packaged_native_coverage.values())
        and all(unsupported_guard_coverage.values())
        and all(item["present"] for item in docs)
    )
    report = {
        "schema_version": 1,
        "target": "v1.2.4 bounded CB-SEM/CFA publication audit",
        "passed": passed,
        "section_coverage": section_coverage,
        "lavaan_coverage": lavaan_coverage,
        "packaged_native_coverage": packaged_native_coverage,
        "unsupported_guard_coverage": unsupported_guard_coverage,
        "legacy_evidence": legacy_evidence,
        "docs": docs,
        "commands": [validate, cargo],
        "note": "Publication support is bounded to the documented raw-data single-group reflective ML CFA/recursive SEM cases. The retained v0.7 evidence JSON is archival and is not regenerated or treated as current promotion authority. Bootstrap, multigroup/invariance, mean structures, robust/ordinal/FIML estimators, and broader constraints are blocked rather than counted as executable evidence.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
