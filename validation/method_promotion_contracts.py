#!/usr/bin/env python3
"""Portable, non-claiming validation for method-promotion contracts.

This command validates every manifest's strict JSON, schema, and semantic
contract without reading local runtime evidence. It exists for clean-checkout
CI, where signed binaries, screenshots, archives, and workbooks are restored
only by a separate trusted release-evidence job. It must never authorize a
scientific, release, parity, or competitor claim; those commands continue to
use ``method_promotion_manifest.py`` with strict evidence verification.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Iterable

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from validation import method_promotion_manifest as factory


def validate_contracts(
    paths: Iterable[Path] | None = None,
    repository_root: Path = factory.REPOSITORY_ROOT,
) -> dict[str, Any]:
    manifests = factory.discover_manifests(paths)
    if not manifests:
        return {
            "passed": False,
            "claim_authorized": False,
            "manifest_count": 0,
            "manifests": [],
            "errors": ["no method-promotion manifests found"],
        }

    results = [
        factory.validate_manifest(
            path,
            repository_root,
            verify_evidence=False,
        )
        for path in manifests
    ]
    feature_ids = [row.get("feature_id") for row in results if row.get("feature_id")]
    duplicate_ids = sorted(
        feature_id
        for feature_id in set(feature_ids)
        if feature_ids.count(feature_id) > 1
    )
    errors = (
        ["duplicate feature IDs across manifests: " + ", ".join(duplicate_ids)]
        if duplicate_ids
        else []
    )
    return {
        "passed": all(row["passed"] for row in results) and not errors,
        "claim_authorized": False,
        "evidence_verified": False,
        "manifest_count": len(results),
        "manifests": results,
        "errors": errors,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", type=Path)
    parser.add_argument("--repository-root", type=Path, default=factory.REPOSITORY_ROOT)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    report = validate_contracts(args.paths or None, args.repository_root)
    if args.json:
        json.dump(report, sys.stdout, indent=2, sort_keys=True)
        sys.stdout.write("\n")
    else:
        status = "PASS" if report["passed"] else "FAIL"
        print(f"QuickPLS method-promotion contracts: {status}")
        print(f"Manifests: {report['manifest_count']}")
        print("Runtime evidence verified: no")
        print("Claim authorized: no")
        for error in report["errors"]:
            print(f"ERROR: {error}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
