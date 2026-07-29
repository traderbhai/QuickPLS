"""Inventory optional SmartPLS/book sample datasets for local validation.

This script intentionally does not download or redistribute third-party sample
data. It records where a developer should place external sample files and whether
the current workstation is ready to run follow-up comparisons.
"""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "validation" / "external" / "smartpls_samples"
OUTPUT = ROOT / "validation" / "results" / "smartpls_sample_inventory.json"


SAMPLES = [
    {
        "id": "corporate_reputation_pls_sem",
        "title": "Corporate Reputation Model",
        "source_url": "https://www.smartpls.com/documentation/sample-projects/corporate-reputation/",
        "book_url": "https://www.pls-sem.net/downloads/3rd-edition-a-primer-on-pls-sem-1/",
        "expected_local_files": [
            "data/corporate_reputation/corporate_reputation.csv",
            "projects/corporate_reputation/Corporate Reputation.zip",
        ],
        "validation_use": [
            "PLS-SEM result layout review",
            "measurement and structural table comparison",
            "report/export workflow review",
            "known-difference documentation",
        ],
        "redistribution_status": "external-local-only unless source license permits redistribution",
    },
    {
        "id": "employee_retention_cbsem_pls_gsca",
        "title": "Employee Retention Model",
        "source_url": "https://smartpls.com/documentation/sample-projects/employee-retention/",
        "expected_local_files": [
            "data/employee_retention/employee_retention.csv",
            "projects/employee_retention/Employee Retention.zip",
        ],
        "validation_use": [
            "CB-SEM/CFA UI and result table review",
            "PLS-SEM/PLSc/GSCA workflow comparison",
            "multi-method report wording review",
        ],
        "redistribution_status": "external-local-only unless source license permits redistribution",
    },
    {
        "id": "smartpls_regression_examples",
        "title": "SmartPLS Regression Examples",
        "source_url": "https://smartpls.com/documentation/sample-projects/regression/",
        "expected_local_files": [
            "data/regression/regression_example.csv",
            "projects/regression/Regression.zip",
        ],
        "validation_use": [
            "OLS/logistic regression setup and result display review",
            "report table wording review",
        ],
        "redistribution_status": "external-local-only unless source license permits redistribution",
    },
    {
        "id": "smartpls_nca_examples",
        "title": "SmartPLS NCA Examples",
        "source_url": "https://smartpls.com/documentation/sample-projects/",
        "expected_local_files": [
            "data/nca/corporate_reputation_nca.csv",
            "data/nca/extended_tam_nca.csv",
        ],
        "validation_use": [
            "NCA ceiling/effect/bottleneck table review",
            "NCA report wording review",
        ],
        "redistribution_status": "external-local-only unless source license permits redistribution",
    },
    {
        "id": "smartpls_process_path_examples",
        "title": "SmartPLS Path Analysis and PROCESS Examples",
        "source_url": "https://smartpls.com/documentation/sample-projects/",
        "expected_local_files": [
            "data/process/process_path_example.csv",
            "projects/process/Path Analysis and PROCESS.zip",
        ],
        "validation_use": [
            "mediation/moderation result layout review",
            "indirect-effect wording and report flow review",
        ],
        "redistribution_status": "external-local-only unless source license permits redistribution",
    },
    {
        "id": "mendeley_smartpls_teaching_dataset",
        "title": "Dataset to run examples in SmartPLS 3",
        "source_url": "https://data.mendeley.com/datasets/4tkph3mxp9/2",
        "expected_local_files": [
            "data/mendeley_teaching/dataset_to_Mendeley.zip",
        ],
        "validation_use": [
            "teaching/example workflow review",
            "import robustness review after local extraction",
        ],
        "redistribution_status": "check dataset license before redistributing derived fixtures",
    },
]


def file_record(relative: str) -> dict:
    path = BASE / relative
    return {
        "relative_path": str(Path("validation") / "external" / "smartpls_samples" / relative),
        "present": path.exists(),
        "bytes": path.stat().st_size if path.exists() else None,
    }


def sample_record(sample: dict) -> dict:
    files = [file_record(relative) for relative in sample["expected_local_files"]]
    return {
        **sample,
        "files": files,
        "ready_for_local_validation": all(item["present"] for item in files),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--require-files",
        action="store_true",
        help="fail if any expected external sample file is missing",
    )
    args = parser.parse_args()

    BASE.mkdir(parents=True, exist_ok=True)
    records = [sample_record(sample) for sample in SAMPLES]
    ready = [sample["id"] for sample in records if sample["ready_for_local_validation"]]
    missing = [
        {
            "sample_id": sample["id"],
            "missing_files": [item["relative_path"] for item in sample["files"] if not item["present"]],
        }
        for sample in records
        if not sample["ready_for_local_validation"]
    ]
    report = {
        "schema_version": 1,
        "target": "optional SmartPLS/book sample validation inventory",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "passed": not args.require_files or not missing,
        "mode": "require-files" if args.require_files else "inventory-only",
        "base_directory": str(BASE.relative_to(ROOT)),
        "ready_sample_count": len(ready),
        "total_sample_count": len(records),
        "ready_samples": ready,
        "missing": missing,
        "samples": records,
        "policy": {
            "download_automation": "disabled",
            "bundling": "do not bundle third-party sample data unless redistribution is explicitly permitted",
            "runtime_dependency": "none; these are development-only validation inputs",
            "claims": "do not claim SmartPLS equivalence; document comparable settings and known differences",
        },
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if not report["passed"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
