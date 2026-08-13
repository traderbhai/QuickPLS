from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import legacy_release_factory_migration as migration


class LegacyReleaseFactoryMigrationTests(unittest.TestCase):
    def test_exact_reference_identity_is_fail_closed(self) -> None:
        config = migration.METHODS["logistic_regression_v2"]
        report = {
            "passed": True,
            "feature_id": config["feature_id"],
            "method_version": config["method_version"],
            "catalogue_snapshot_date": "2026-08-12",
        }
        self.assertTrue(migration.exact_reference_identity(report, config))
        for key in ("passed", "feature_id", "method_version", "catalogue_snapshot_date"):
            changed = copy.deepcopy(report)
            changed[key] = False if key == "passed" else "changed"
            self.assertFalse(migration.exact_reference_identity(changed, config))

    def test_passed_checks_rejects_false_and_duplicate_shapes(self) -> None:
        self.assertEqual(
            migration.passed_checks({"checks": {"one": True, "two": False}}),
            {"one"},
        )
        self.assertEqual(
            migration.passed_checks(
                {"checks": [{"name": "one", "passed": True}, {"name": "two", "passed": False}]}
            ),
            {"one"},
        )
        self.assertEqual(migration.passed_checks({"checks": "invalid"}), set())

    def test_all_four_migrations_have_distinct_exact_identities(self) -> None:
        identities = {
            (row["feature_id"], row["method_version"])
            for row in migration.METHODS.values()
        }
        self.assertEqual(len(identities), 4)
        self.assertTrue(
            all(row["required_checks"] for row in migration.METHODS.values())
        )

    def test_legacy_package_never_satisfies_current_candidate_without_exact_bytes(self) -> None:
        config = migration.METHODS["logistic_regression_v2"]
        current = {
            "path": "target/release/quickpls-desktop.exe",
            "size": 1,
            "sha256": "0" * 64,
        }
        gap = migration.legacy_package_gap(config, current)
        self.assertFalse(gap["passed"])
        self.assertFalse(gap["exact_current_desktop_binding"])
        self.assertEqual(gap["highest_current_factory_state"], "engine_only")
        self.assertIn("run_v247_logistic", gap["required_runtime_command"])


if __name__ == "__main__":
    unittest.main()
