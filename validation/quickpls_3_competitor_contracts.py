#!/usr/bin/env python3
"""Portable, explicitly non-claiming QuickPLS 3 programme validation.

The release validator intentionally materializes every runtime artifact.  A
normal source checkout does not contain ignored screenshots, workbooks,
archives, or executables, so CI uses this command to validate the frozen
catalogue, parity contract, method contracts, and commercial gate structure.
It cannot authorize competitor readiness; strict release validation remains
the only claim-producing path.
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from validation import (  # noqa: E402
    method_promotion_contracts,
    parity_ledger,
    quickpls_3_release_readiness,
    quickpls_external_beta,
)
from validation.quickpls_3_competitor_program import (  # noqa: E402
    load_json,
    validate_program_contract_document,
)

VALIDATION_DIR = ROOT / "validation"
CATALOGUE = VALIDATION_DIR / "quickpls_3_competitor_catalogue.json"
LEDGER = VALIDATION_DIR / "quickpls_3_parity_ledger.json"
READINESS = VALIDATION_DIR / "quickpls_3_release_readiness.json"
BETA = VALIDATION_DIR / "quickpls_external_beta.json"


def _declared_factory_contract(report: dict[str, Any]) -> dict[str, Any]:
    """Return planning states that cannot be mistaken for verified evidence."""

    normalized = copy.deepcopy(report)
    normalized["claim_authorized"] = False
    normalized["evidence_verified"] = False
    for manifest in normalized.get("manifests", []):
        manifest["contract_derived_state"] = manifest.get("derived_state")
        manifest["derived_state"] = manifest.get("declared_state")
        manifest["passed"] = not manifest.get("errors")
    normalized["passed"] = bool(normalized.get("passed")) and all(
        manifest.get("passed") is True
        for manifest in normalized.get("manifests", [])
    )
    return normalized


def validate_contracts(
    *,
    repository_root: Path = ROOT,
    catalogue_path: Path = CATALOGUE,
    ledger_path: Path = LEDGER,
    readiness_path: Path = READINESS,
    beta_path: Path = BETA,
) -> dict[str, Any]:
    method_contract = method_promotion_contracts.validate_contracts(
        repository_root=repository_root
    )
    factory_contract = _declared_factory_contract(method_contract)
    parity_report = parity_ledger.validate_ledger(ledger_path, repository_root)
    commercial_report = quickpls_3_release_readiness.load_and_validate(
        readiness_path,
        repository_root=repository_root,
    )
    beta_report = quickpls_external_beta.validate_contract(
        quickpls_external_beta.strict_json(beta_path)
    )
    program_report = validate_program_contract_document(
        load_json(catalogue_path),
        parity_report,
        repository_root,
        commercial_readiness_report=commercial_report,
        external_beta_report=beta_report,
        manifest_factory_report=factory_contract,
    )

    errors: list[str] = []
    for label, report, pass_field in (
        ("method contracts", method_contract, "passed"),
        ("parity", parity_report, "passed"),
        ("commercial contract", commercial_report, "structurally_valid"),
        ("external beta contract", beta_report, "passed"),
        ("competitor catalogue", program_report, "passed"),
    ):
        if report.get(pass_field) is not True:
            report_errors = report.get("errors", [])
            if report_errors:
                errors.extend(f"{label}: {error}" for error in report_errors)
            else:
                errors.append(f"{label}: validation did not pass")

    if program_report.get("competitor_ready") is True:
        errors.append(
            "portable contract validation must never authorize competitor readiness"
        )

    return {
        "passed": not errors,
        "claim_authorized": False,
        "evidence_verified": False,
        "competitor_ready": False,
        "method_manifest_count": method_contract.get("manifest_count", 0),
        "method_count": program_report.get("method_count", 0),
        "competitor_scope_count": program_report.get("competitor_scope_count", 0),
        "status_counts": program_report.get("status_counts", {}),
        "errors": errors,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository-root", type=Path, default=ROOT)
    parser.add_argument("--catalogue", type=Path, default=CATALOGUE)
    parser.add_argument("--ledger", type=Path, default=LEDGER)
    parser.add_argument("--readiness", type=Path, default=READINESS)
    parser.add_argument("--beta", type=Path, default=BETA)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    report = validate_contracts(
        repository_root=args.repository_root,
        catalogue_path=args.catalogue,
        ledger_path=args.ledger,
        readiness_path=args.readiness,
        beta_path=args.beta,
    )
    if args.json:
        json.dump(report, sys.stdout, indent=2, sort_keys=True)
        sys.stdout.write("\n")
    else:
        status = "PASS" if report["passed"] else "FAIL"
        print(f"QuickPLS 3 portable competitor contracts: {status}")
        print(f"Methods: {report['method_count']}")
        print(f"Method manifests: {report['method_manifest_count']}")
        print("Runtime evidence verified: no")
        print("Competitor claim authorized: no")
        for error in report["errors"]:
            print(f"ERROR: {error}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
