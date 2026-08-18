#!/usr/bin/env python3
"""Focused tests for the customer-language cutover contract."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path


VALIDATION_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = VALIDATION_DIR.parent
sys.path.insert(0, str(VALIDATION_DIR))

from customer_language_contract import (  # noqa: E402
    LANGUAGE_RULES,
    iter_customer_source_files,
    scan_repository,
    scan_text,
)


class CustomerLanguageContractTests(unittest.TestCase):
    def test_detects_each_internal_governance_phrase_with_replacement(self) -> None:
        source = """
          <p>Validated scope</p>
          <p>Calculation scope</p>
          <p>Method scope</p>
          <p>Documented QuickPLS supported scope</p>
          <p>Trust evidence</p>
          <p>Validation artifacts</p>
          <p>Scope checked</p>
          <p>Validated native OLS scope</p>
          <p>Bounded reflective desktop scope</p>
          <p>PLS-POS-style bounded preview</p>
          <p>Candidate fixed-score output</p>
          <p>Current evidence</p>
          <p>Bounded native scope</p>
          <p>Native-qualified</p>
          <p>Release qualified</p>
          <p>Candidate, unqualified workflow</p>
          <p>Candidate scope</p>
          <p>Promotion evidence</p>
          <p>Packaged evidence</p>
          <p>Not promoted to current evidence</p>
          <p>Qualification evidence pending</p>
        """

        violations = scan_text(source, "src/components/Example.tsx")

        self.assertEqual({item.rule_id for item in violations}, {rule.rule_id for rule in LANGUAGE_RULES})
        self.assertTrue(all(item.replacement for item in violations))

    def test_does_not_flag_ordinary_scientific_or_programming_language(self) -> None:
        source = """
          const candidate = candidates.find((item) => item.supported);
          const evidence = modelEvidence.get(candidate.id);
          return <p>Cross-validation design uses ten folds.</p>;
          // A release-qualified claim in an internal implementation comment is not customer copy.
          /* Validated scope is mentioned in a migration note only. */
          const url = "https://example.test/docs//path";
        """

        self.assertEqual(scan_text(source, "src/domain/example.tsx"), [])

    def test_scan_excludes_tests_specs_stories_and_fixture_directories(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            files = {
                "src/customer.tsx": "export const copy = 'Validated scope';",
                "src/customer.test.tsx": "export const copy = 'Validated scope';",
                "src/customer.spec.ts": "export const copy = 'Validated scope';",
                "src/customer.stories.tsx": "export const copy = 'Validated scope';",
                "src/fixtures/customer.ts": "export const copy = 'Validated scope';",
            }
            for relative_path, contents in files.items():
                path = root / relative_path
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(contents, encoding="utf-8")

            scanned = [path.relative_to(root).as_posix() for path in iter_customer_source_files(root)]
            report = scan_repository(root)

        self.assertEqual(scanned, ["src/customer.tsx"])
        self.assertEqual(report["violation_count"], 1)
        self.assertFalse(report["strict_passed"])

    def test_explicit_internal_marker_skips_only_its_own_line(self) -> None:
        source = "\n".join(
            [
                "throw new Error('release-qualified is an internal enum'); // customer-copy-lint: allow-internal",
                "export const customerCopy = 'release-qualified';",
            ]
        )

        violations = scan_text(source, "src/mixed.ts")

        self.assertEqual(1, len(violations))
        self.assertEqual(2, violations[0].line)

    def test_clean_source_passes_strict_inventory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            source = root / "src" / "clean.tsx"
            source.parent.mkdir(parents=True)
            source.write_text("export const copy = 'Supported setup';", encoding="utf-8")

            report = scan_repository(root)

        self.assertTrue(report["strict_passed"])
        self.assertEqual(report["violation_count"], 0)
        self.assertEqual(report["scanned_files"], 1)

    def test_current_repository_can_be_inventoried_deterministically(self) -> None:
        first = scan_repository(REPOSITORY_ROOT)
        second = scan_repository(REPOSITORY_ROOT)

        self.assertEqual(first, second)
        self.assertGreater(first["scanned_files"], 0)
        self.assertEqual(
            sum(first["counts_by_rule"].values()),
            first["violation_count"],
        )
        for violation in first["violations"]:
            self.assertTrue(violation["path"].startswith("src/"))
            self.assertNotIn(".test.", violation["path"])


if __name__ == "__main__":
    unittest.main()
