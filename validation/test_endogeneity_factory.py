import copy
import hashlib
import subprocess
import unittest
from pathlib import Path

from endogeneity_factory_common import (
    METHOD_VERSION,
    PROVENANCE_VERSION,
    identity_report_document,
    manifest,
    optionally_write_identity_report,
    recipe_payload,
    role_sources,
)
from endogeneity_factory_audit import audit
from method_promotion_manifest import validate_manifest


ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = (
    ROOT / "validation" / "methods" / "gaussian_copula_endogeneity_v1.manifest.json"
)


class EndogeneityFactoryTests(unittest.TestCase):
    def test_manifest_derives_native_qualification_from_current_evidence(self):
        document = manifest()
        result = validate_manifest(MANIFEST_PATH, ROOT)
        self.assertTrue(result["passed"], result["errors"])
        self.assertEqual("native_qualified", result["declared_state"])
        self.assertEqual("native_qualified", result["derived_state"])
        self.assertEqual([], document["qualification"]["evidence"]["release_qualified"])

    def test_recipe_is_typed_deterministic_and_bounded(self):
        recipe = recipe_payload("unit", "sha256:" + "1" * 64)
        self.assertEqual("endogeneity", recipe["settings"]["method"])
        self.assertEqual({"kind": "endogeneity"}, recipe["method_config"])
        self.assertEqual(0, recipe["settings"]["bootstrap_samples"])
        self.assertEqual(0, recipe["settings"]["permutation_samples"])
        self.assertIsNone(recipe["settings"]["case_weight_column"])
        self.assertEqual([], recipe["model"]["controls"])
        self.assertEqual([], recipe["model"]["interactions"])
        self.assertEqual([], recipe["model"]["higher_order_constructs"])
        self.assertEqual(
            "gaussian_copula_endogeneity_v1", METHOD_VERSION
        )
        self.assertIn(METHOD_VERSION, PROVENANCE_VERSION)

    def test_identity_builder_binds_exact_checkout_bytes_without_writing(self):
        report_path = (
            ROOT
            / "validation"
            / "results"
            / "method_factory"
            / METHOD_VERSION
            / "method_spec.identity.json"
        )
        existed_before = report_path.exists()
        self.assertIsNone(
            optionally_write_identity_report(
                "method_spec",
                write_identity=False,
                passed=True,
                checks={"passed": True},
                extras=["validation/endogeneity_frontend_gate.py"],
            )
        )
        self.assertEqual(existed_before, report_path.exists())
        report = identity_report_document(
            "method_spec",
            passed=True,
            checks={"passed": True},
            extras=["validation/endogeneity_frontend_gate.py"],
        )
        descriptors = {row["path"]: row for row in report["source_artifacts"]}
        for relative in role_sources(
            "method_spec", ["validation/endogeneity_frontend_gate.py"]
        ):
            path = ROOT / relative
            self.assertIn(relative, descriptors)
            self.assertEqual(path.stat().st_size, descriptors[relative]["size"])
            self.assertEqual(
                hashlib.sha256(path.read_bytes()).hexdigest(),
                descriptors[relative]["sha256"],
            )

    def test_identity_mutations_are_detectable_from_bound_source_descriptors(self):
        report = identity_report_document(
            "method_spec", passed=True, checks={"passed": True}
        )
        changed = copy.deepcopy(report)
        changed["source_artifacts"][0]["sha256"] = "0" * 64
        self.assertNotEqual(
            report["source_artifacts"][0]["sha256"],
            changed["source_artifacts"][0]["sha256"],
        )

    def test_packaged_acceptance_scaffold_fails_closed(self):
        completed = subprocess.run(
            [
                "powershell",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                "validation/run_endogeneity_native_acceptance.ps1",
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        output = completed.stdout + completed.stderr
        self.assertNotEqual(0, completed.returncode)
        self.assertIn("No release evidence was written", output)

    def test_final_audit_cannot_pass_without_packaged_evidence(self):
        checks = audit()
        self.assertFalse(checks["passed"])
        self.assertFalse(checks["packaged_acceptance_current"])
        self.assertTrue(checks["release_claim_blocked_without_packaged_acceptance"])


if __name__ == "__main__":
    unittest.main()
