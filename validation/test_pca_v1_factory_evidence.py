import copy
import unittest
from pathlib import Path
from unittest.mock import patch

from method_promotion_manifest import (
    _verify_artifact,
    strict_load_json,
    validate_manifest,
)


ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "validation" / "methods" / "pca_v1.manifest.json"
EXPECTED_ROLES = {
    "method_spec",
    "independent_reference",
    "simulation_report",
    "boundary_report",
    "persistence_report",
    "frontend_report",
    "export_report",
    "method_audit",
    "packaged_acceptance",
}


class PcaV1FactoryEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.document = strict_load_json(MANIFEST_PATH)
        feature = cls.document["feature"]
        cls.expected_identity = {
            "passed": True,
            "feature_id": feature["id"],
            "method_version": feature["method_version"],
            "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
        }
        cls.artifacts = [
            artifact
            for stage in cls.document["qualification"]["evidence"].values()
            for artifact in stage
        ]

    def test_manifest_has_exactly_one_fresh_identity_report_per_role(self):
        roles = [role for artifact in self.artifacts for role in artifact["roles"]]
        self.assertEqual(EXPECTED_ROLES, set(roles))
        self.assertEqual(len(EXPECTED_ROLES), len(roles))
        for artifact in self.artifacts:
            passed, errors = _verify_artifact(
                artifact,
                self.document,
                ROOT,
                self.expected_identity,
            )
            self.assertTrue(passed, errors)

    def test_manifest_derives_release_qualified_only_from_current_bytes(self):
        result = validate_manifest(MANIFEST_PATH, ROOT)
        self.assertTrue(result["passed"], result["errors"])
        self.assertEqual("release_qualified", result["derived_state"])
        self.assertEqual("release_qualified", result["declared_state"])

    def test_identity_and_source_mutations_fail_closed(self):
        artifact = self.artifacts[0]
        report_path = ROOT / artifact["path"]
        original = strict_load_json(report_path)
        mutations = {
            "passed": lambda row: row.__setitem__("passed", False),
            "feature_id": lambda row: row.__setitem__("feature_id", "qpls3.standalone.other"),
            "method_version": lambda row: row.__setitem__("method_version", "pca_v2"),
            "catalogue_snapshot_date": lambda row: row.__setitem__("catalogue_snapshot_date", "2026-08-11"),
            "generated_at_utc": lambda row: row.__setitem__("generated_at_utc", "2026-08-13T00:00:00Z"),
            "source_sha256": lambda row: row["source_artifacts"][0].__setitem__("sha256", "0" * 64),
            "source_size": lambda row: row["source_artifacts"][0].__setitem__("size", 0),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label):
                changed = copy.deepcopy(original)
                mutate(changed)
                with patch("method_promotion_manifest.strict_load_json", return_value=changed):
                    passed, errors = _verify_artifact(
                        artifact,
                        self.document,
                        ROOT,
                        self.expected_identity,
                    )
                self.assertFalse(passed)
                self.assertTrue(errors)


if __name__ == "__main__":
    unittest.main()
