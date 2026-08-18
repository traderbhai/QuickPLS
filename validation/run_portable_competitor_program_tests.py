#!/usr/bin/env python3
"""Run competitor mutations with a non-claiming clean-checkout factory fixture."""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from validation import method_promotion_contracts
from validation import method_promotion_manifest


def _fixture_report(
    paths=None,
    repository_root=method_promotion_manifest.REPOSITORY_ROOT,
    **_,
):
    if paths:
        return method_promotion_contracts.validate_contracts(
            paths=paths,
            repository_root=repository_root,
        )
    report = copy.deepcopy(
        method_promotion_contracts.validate_contracts(repository_root=repository_root)
    )
    report.pop("claim_authorized", None)
    report.pop("evidence_verified", None)
    for manifest in report.get("manifests", []):
        manifest["derived_state"] = manifest.get("declared_state")
        manifest["passed"] = not manifest.get("errors")
    report["passed"] = bool(report.get("passed")) and all(
        manifest.get("passed") is True for manifest in report.get("manifests", [])
    )
    return report


def main() -> int:
    suite = unittest.defaultTestLoader.loadTestsFromName(
        "validation.test_quickpls_3_competitor_program"
    )
    if suite.countTestCases() != 36:
        raise SystemExit(
            "portable competitor-test inventory changed; review the factory fixture"
        )
    with patch.object(method_promotion_manifest, "validate_all", _fixture_report):
        result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
