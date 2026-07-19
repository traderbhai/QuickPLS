"""Publication audit for methodology docs and known differences."""

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "validation" / "results"
OUTPUT = RESULTS / "documentation_publication_audit.json"


REQUIRED_DOCS = [
    "docs/PUBLICATION_READY_AUDIT.md",
    "docs/METHOD_COMPATIBILITY.md",
    "docs/DELIVERY_STATUS.md",
    "docs/DEVELOPMENT_LEDGER.md",
    "docs/KNOWN_DIFFERENCES.md",
    "docs/MONTE_CARLO_VALIDATION.md",
]


REQUIRED_METHOD_DOCS = [
    "PLS_PM_V1.md",
    "PLS_ASSESSMENT_V1.md",
    "RESAMPLING_ENGINE_V4.md",
    "PLS_MEDIATION_V1.md",
    "PLSPREDICT_HOLDOUT_V1.md",
    "CBSEM_ML_V1.md",
    "PCA_V1.md",
    "REGRESSION_OLS_V1.md",
    "NCA_V1.md",
    "GSCA_V1.md",
]


def main():
    RESULTS.mkdir(parents=True, exist_ok=True)
    docs = []
    for relative in REQUIRED_DOCS:
        path = ROOT / relative
        text = path.read_text(encoding="utf-8") if path.exists() else ""
        docs.append({
            "path": relative,
            "present": path.exists(),
            "bytes": path.stat().st_size if path.exists() else None,
            "mentions_validation": "validation" in text.lower(),
            "mentions_known_differences": "known" in text.lower() or "differences" in text.lower(),
        })
    method_docs = []
    for name in REQUIRED_METHOD_DOCS:
        path = ROOT / "docs" / "methods" / name
        text = path.read_text(encoding="utf-8") if path.exists() else ""
        method_docs.append({
            "path": str(path.relative_to(ROOT)),
            "present": path.exists(),
            "bytes": path.stat().st_size if path.exists() else None,
            "mentions_unsupported": "unsupported" in text.lower() or "not supported" in text.lower(),
            "mentions_validation": "validation" in text.lower() or "reference" in text.lower(),
        })
    passed = (
        all(item["present"] for item in docs)
        and any(item["mentions_known_differences"] for item in docs)
        and all(item["present"] and item["mentions_validation"] for item in method_docs)
    )
    report = {
        "schema_version": 1,
        "target": "documentation publication audit",
        "passed": passed,
        "docs": docs,
        "method_docs": method_docs,
    }
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
