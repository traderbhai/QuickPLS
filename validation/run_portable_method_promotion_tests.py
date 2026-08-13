#!/usr/bin/env python3
"""Run every factory mutation test that does not require local runtime evidence."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))


EXCLUDED = {
    "validation.test_method_promotion_manifest.MethodPromotionManifestTests."
    "test_all_factory_manifests_are_truthful_and_migrations_do_not_overclaim"
}
EXPECTED_TOTAL = 20


def _flatten(suite: unittest.TestSuite):
    for item in suite:
        if isinstance(item, unittest.TestSuite):
            yield from _flatten(item)
        else:
            yield item


def main() -> int:
    loaded = unittest.defaultTestLoader.loadTestsFromName(
        "validation.test_method_promotion_manifest"
    )
    tests = list(_flatten(loaded))
    ids = {test.id() for test in tests}
    if len(tests) != EXPECTED_TOTAL or not EXCLUDED <= ids:
        print(
            "portable factory-test inventory changed; review the explicit runtime exclusion",
            file=sys.stderr,
        )
        return 2
    suite = unittest.TestSuite(test for test in tests if test.id() not in EXCLUDED)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
