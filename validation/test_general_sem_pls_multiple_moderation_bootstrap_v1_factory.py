from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FACTORY_PATH = (
    ROOT / "validation/general_sem_pls_multiple_moderation_bootstrap_v1_factory.py"
)
SPEC = importlib.util.spec_from_file_location("moderation_bootstrap_factory", FACTORY_PATH)
assert SPEC is not None and SPEC.loader is not None
FACTORY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(FACTORY)


class GeneralSemPlsMultipleModerationBootstrapFactoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = FACTORY.build_report()

    def test_factory_report_has_exact_identity_and_engine_only_boundary(self):
        self.assertTrue(self.report["passed"])
        self.assertEqual(
            self.report["feature_id"],
            "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
        )
        self.assertEqual(
            self.report["method_version"],
            "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
        )
        self.assertEqual(self.report["catalogue_snapshot_date"], "2026-08-19")
        self.assertFalse(self.report["qualification_ready"])
        self.assertFalse(self.report["promotion_allowed"])
        self.assertFalse(self.report["checks"]["release_qualification_complete"])

    def test_factory_binds_governance_role_and_current_engine_test_sources(self):
        descriptors = {row["path"]: row for row in self.report["source_artifacts"]}
        required = {
            "docs/methods/GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_V1.md",
            "validation/general_sem_pls_multiple_moderation_bootstrap_v1_reference.py",
            "validation/methods/general_sem_pls_multiple_moderation_bootstrap_v1.manifest.json",
            "validation/methods/method_promotion_manifest.schema.json",
            "validation/method_promotion_manifest.py",
            "validation/test_method_promotion_manifest.py",
            "validation/capabilities/general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1.cell.manifest.json",
            "crates/qpls-resampling/src/general_sem_pls_bootstrap_v1.rs",
            "crates/qpls-runner/src/recipe_v4_general_sem_pls_execution.rs",
            "src/native/NativeRecipeV4GeneralSemWorkspace.test.tsx",
        }
        self.assertTrue(required <= set(descriptors))
        self.assertTrue(
            all(row["size"] > 0 and len(row["sha256"]) == 64 for row in descriptors.values())
        )

    def test_factory_embeds_passing_gamma_only_independent_reference(self):
        reference = self.report["reference_execution"]
        self.assertTrue(reference["passed"])
        self.assertEqual(reference["inferential_target_policy"], "scientific_rescaled_gamma_only")
        self.assertEqual(len(reference["gamma_inference"]), 4)
        self.assertTrue(self.report["checks"]["gamma_only_inference_inventory"])

    def test_output_and_check_report_round_trip(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "engine.identity.json"
            written = subprocess.run(
                [sys.executable, str(FACTORY_PATH), "--output", str(output)],
                cwd=ROOT,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(written.returncode, 0, written.stderr)
            output_report = json.loads(output.read_text(encoding="utf-8"))
            self.assertTrue(output_report["passed"])
            self.assertEqual(output_report["feature_id"], self.report["feature_id"])
            self.assertEqual(output_report["method_version"], self.report["method_version"])
            self.assertEqual(output_report["source_artifacts"], self.report["source_artifacts"])
            checked = subprocess.run(
                [sys.executable, str(FACTORY_PATH), "--check-report", str(output)],
                cwd=ROOT,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(checked.returncode, 0, checked.stderr)


if __name__ == "__main__":
    unittest.main()
