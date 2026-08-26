from __future__ import annotations

import copy
import importlib.util
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SCHEMA = HERE / "multimod_source_bound_manifest_v1.schema.json"
PLAN = HERE / "v256_multimod_qualification_plan_v1.json"
MATERIALIZER = HERE / "materialize_multimod_live_manifests_v1.py"
BUILD = ROOT / "src-tauri" / "build.rs"
RELEASE = HERE / "verify_multimod_release_acceptance_v1.py"
PACKAGE = HERE / "package_multimod_candidate_v1.ps1"
PACKAGED_DRIVER = HERE / "multimod_packaged_smoke_driver_v1.mjs"
PACKAGED_WRAPPER = HERE / "run_multimod_packaged_offline_smoke_v1.ps1"
RUNTIME_SMOKE_SCHEMA = HERE / "multimod_runtime_promotion_smoke_v1.schema.json"


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def load_materializer():
    spec = importlib.util.spec_from_file_location(
        "materialize_multimod_live_manifests_v1", MATERIALIZER
    )
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class MultiModStandardPromotionContractV1Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.validator = Draft202012Validator(read_json(SCHEMA))
        cls.materializer = load_materializer()
        cls.templates = [
            read_json(ROOT / relative) for relative in read_json(PLAN)["manifests"]
        ]

    def promoted_copy(self, template: dict) -> dict:
        live = copy.deepcopy(template)
        live["surface"] = "standard"
        live["declared_evidence_state"] = "release_qualified"
        live["promotion_allowed"] = True
        live["source_binding"]["status"] = "bound"
        live["source_binding"]["candidate_commit_sha"] = "a" * 40
        for source in live["source_binding"]["source_artifacts"]:
            source["sha256"] = "b" * 64
        for profile in live["profile_matrix"]:
            profile["surface"] = "standard"
            profile["coverage_state"] = "release_qualified"
            profile["evidence_state"] = "release_qualified"
            for procedure in profile["procedure_cells"]:
                procedure["evidence_state"] = "release_qualified"
                procedure["gate_state"] = "passed"
                procedure["report_path"] = "cell-reports/evidence.json"
                procedure["report_sha256"] = "c" * 64
        return live

    def test_schema_accepts_tracked_labs_and_external_standard_pairs(self) -> None:
        for template in self.templates:
            self.validator.validate(template)
            self.validator.validate(self.promoted_copy(template))

    def test_schema_rejects_crossed_surface_promotion_pairs(self) -> None:
        standard = self.promoted_copy(self.templates[0])
        standard["promotion_allowed"] = False
        self.assertTrue(list(self.validator.iter_errors(standard)))

        standard = self.promoted_copy(self.templates[0])
        standard["profile_matrix"][0]["surface"] = "labs"
        self.assertTrue(list(self.validator.iter_errors(standard)))

        labs = copy.deepcopy(self.templates[0])
        labs["promotion_allowed"] = True
        self.assertTrue(list(self.validator.iter_errors(labs)))

        standard = self.promoted_copy(self.templates[0])
        standard["source_binding"]["status"] = "pending"
        self.assertTrue(list(self.validator.iter_errors(standard)))

        standard = self.promoted_copy(self.templates[0])
        standard["profile_matrix"][0]["procedure_cells"][0]["gate_state"] = (
            "pending"
        )
        self.assertTrue(list(self.validator.iter_errors(standard)))

    def test_materializer_keeps_tracked_templates_fail_closed(self) -> None:
        for template in self.templates:
            self.materializer.assert_tracked_manifest_template(template)
            self.assertEqual(
                set(self.materializer.tracked_profile_cells(template)),
                {row["profile_id"] for row in template["profile_matrix"]},
            )
        changed = copy.deepcopy(self.templates[0])
        changed["surface"] = "standard"
        changed["promotion_allowed"] = True
        with self.assertRaises(self.materializer.ManifestError):
            self.materializer.assert_tracked_manifest_template(changed)
        self.assertTrue(
            self.materializer.valid_surface_promotion_pair("standard", True)
        )
        self.assertFalse(
            self.materializer.valid_surface_promotion_pair("standard", False)
        )

    def test_build_boundary_requires_full_standard_authority(self) -> None:
        source = BUILD.read_text(encoding="utf-8")
        for required in (
            'manifest_set.surface != "standard"',
            "!manifest_set.promotion_allowed",
            'text(manifest.get("surface"), "live manifest.surface") != "standard"',
            "live_profiles != tracked_profile_ids",
            "manifest_cells != tracked_cells",
            "authority_cells != tracked_exact_cells",
        ):
            self.assertIn(required, source)

    def test_release_acceptance_requires_visible_standard_evidence(self) -> None:
        source = RELEASE.read_text(encoding="utf-8")
        for required in (
            '"standard_surface_verified": True',
            '"labs_opt_in_not_required": True',
            '"lab_badge_absent": True',
            '"standard_manifest_authority_verified"',
        ):
            self.assertIn(required, source)

    def test_packaged_smoke_produces_standard_without_labs_opt_in(self) -> None:
        driver = PACKAGED_DRIVER.read_text(encoding="utf-8")
        self.assertIn(
            'const STANDARD_MULTIMOD_SURFACE = "standard_multimod_v1";',
            driver,
        )
        self.assertIn("authority.standardSurfaceAuthorized === true", driver)
        self.assertNotIn("experimentalLabsEnabled: true", driver)
        self.assertNotIn('surface: "internal_labs', driver)
        for required in (
            'new CustomEvent("quickpls:open-project-path"',
            'page.locator(\'[data-testid="native-multimod-workspace-open"]\')',
            'page.locator(".nd-multimod-labs-badge.standard"',
            'hasText: "Standard · Release-qualified"',
            "multiModSurfacePresentation.standard_badge_count >= 1",
            "standard_surface_verified: true",
            "labs_opt_in_not_required: true",
            "lab_badge_absent: multiModSurfacePresentation.lab_badge_count === 0",
        ):
            self.assertIn(required, driver)

        smoke_schema = read_json(RUNTIME_SMOKE_SCHEMA)
        for field in (
            "standard_surface_verified",
            "labs_opt_in_not_required",
            "lab_badge_absent",
        ):
            self.assertIn(field, smoke_schema["required"])
            self.assertEqual(smoke_schema["properties"][field], {"const": True})
            self.assertIn(field, PACKAGED_WRAPPER.read_text(encoding="utf-8"))

        package = PACKAGE.read_text(encoding="utf-8")
        self.assertIn('$manifestSet.surface -cne "standard"', package)
        self.assertIn("$manifestSet.promotion_allowed -ne $true", package)


if __name__ == "__main__":
    unittest.main()
