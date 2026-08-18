import copy
import hashlib
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from method_promotion_manifest import _verify_artifact, strict_load_json, validate_manifest
from cbsem_ml_v1_factory_common import (
    EXPECTED_PROVENANCE_VERSION,
    DuplicateKeyError,
    strict_load_json as strict_factory_json,
)
from cbsem_ml_v1_packaged_adapter import source_freshness, visual_contract
import phase2_release_packaged_common as phase2_common
from cbsem_ml_v1_reference import EXPECTED_MODELS, LIMITS, summarize_lavaan_report


ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "validation" / "methods" / "cbsem_ml_v1.manifest.json"
EXPECTED_NATIVE_ROLES = {
    "method_spec",
    "independent_reference",
    "simulation_report",
    "boundary_report",
    "persistence_report",
    "frontend_report",
    "export_report",
}


class CbsemMlV1FactoryTests(unittest.TestCase):
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

    def test_strict_factory_json_rejects_duplicate_keys(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "duplicate.json"
            path.write_text('{"passed":true,"passed":false}', encoding="utf-8")
            with self.assertRaises(DuplicateKeyError):
                strict_factory_json(path)

    def test_lavaan_summary_requires_exact_six_models_and_tolerances(self):
        rows = []
        for model in sorted(EXPECTED_MODELS):
            row = {
                "model": model,
                "passed": True,
                "quickpls_converged": True,
                "matched_parameters": 7,
            }
            row.update({metric: limit / 2 for metric, limit in LIMITS.items()})
            rows.append(row)
        report = {
            "kind": "cbsem_lavaan_reference_v1",
            "status": "passed",
            "lavaan": "0.7.2",
            "models": rows,
        }
        self.assertTrue(summarize_lavaan_report(report)["passed"])
        report["models"][0]["max_estimate_delta"] = 2e-6
        self.assertFalse(summarize_lavaan_report(report)["passed"])
        report["models"][0]["max_estimate_delta"] = 0.0
        report["models"].pop()
        self.assertFalse(summarize_lavaan_report(report)["passed"])

    def test_method_spec_factory_requires_reviewed_release_status(self):
        source = (ROOT / "validation/cbsem_ml_v1_factory_evidence.py").read_text(encoding="utf-8")
        specification = (ROOT / "docs/methods/CBSEM_ML_V1.md").read_text(encoding="utf-8")
        fragment = "`cbsem_ml_v1` is release-qualified"
        self.assertIn(fragment, source)
        self.assertIn(fragment, specification)

    def test_point_factory_tracks_current_assessment_provenance(self):
        assessment_source = (ROOT / "crates/qpls-assessment/src/lib.rs").read_text(
            encoding="utf-8"
        )
        self.assertTrue(EXPECTED_PROVENANCE_VERSION.endswith("+pls_assessment_v8"))
        self.assertIn(
            'pub const ASSESSMENT_METHOD_VERSION: &str = "pls_assessment_v8";',
            assessment_source,
        )

    def test_visual_contract_requires_all_three_responsive_fail_closed_views(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            screen_root = root / "validation/results/screens/v247-native-desktop-acceptance"
            screen_root.mkdir(parents=True)
            screenshots = []
            artifacts = []
            rows = []
            previous = {"left": 1, "top": 2, "width": 1100, "height": 800, "windowState": "normal"}
            for viewport in ("1024x700", "1280x720", "1440x900"):
                width, height = map(int, viewport.split("x"))
                path = screen_root / f"136v-tauri-native-cbsem-packaged-viewport-{viewport}.png"
                payload = viewport.encode("ascii")
                path.write_bytes(payload)
                screenshots.append(str(path.resolve()))
                artifacts.append({
                    "path": path.relative_to(root).as_posix(),
                    "size": len(payload),
                    "sha256": hashlib.sha256(payload).hexdigest(),
                })
                after = {"left": 1, "top": 2, "width": width + 16, "height": height + 39, "windowState": "normal"}
                rows.append({
                    "id": viewport,
                    "requestedClientViewport": {"width": width, "height": height},
                    "domInnerDimensions": {"width": width, "height": height},
                    "outerBoundsBefore": previous,
                    "outerBoundsAfter": after,
                    "origin": "http://tauri.localhost",
                    "tauriRuntime": True,
                    "surface": "results",
                    "noHorizontalOverflow": True,
                    "methodRunLinkage": True,
                    "methodVersionEvidenceBound": True,
                    "selectedRunId": "run-1",
                    "selectedRunLabel": "CB-SEM / CFA run",
                    "selectedTableId": "cbsem_standardized_parameters",
                    "resultRows": 23,
                    "screenshot": str(path.resolve()),
                    "passed": True,
                })
                previous = after
            report = {
                "checks": {"cbsemPackagedViewports": {
                    "passed": True,
                    "actualTauriWindow": True,
                    "resizeMechanism": "Browser.setWindowBounds",
                    "targetIdentity": {"targetId": "target-1", "windowId": 7, "lookupCommand": "Browser.getWindowForTarget", "origin": "http://tauri.localhost"},
                    "deviceMetricsOverride": {"clearCommand": "Emulation.clearDeviceMetricsOverride", "cleared": True, "playwrightViewportBefore": None, "pageSetViewportSizeUsed": False, "emulationOnly": False},
                    "method": {"slug": "cbsem_ml_v1", "version": "cbsem_ml_v1", "expectedRunId": "run-1", "expectedRunLabel": "CB-SEM / CFA", "expectedTableId": "cbsem_standardized_parameters"},
                    "exactViewports": rows,
                    "restoredFinalWindowState": {"passed": True, "requested": rows[0]["outerBoundsBefore"], "actual": rows[0]["outerBoundsBefore"]},
                }},
                "screenshots": screenshots,
                "screenshotArtifacts": artifacts,
            }
            with patch.object(phase2_common, "ROOT", root), patch.object(phase2_common, "SCREEN_ROOT", screen_root):
                self.assertTrue(visual_contract(report, run_id="run-1")["passed"])
                report["checks"]["cbsemPackagedViewports"]["deviceMetricsOverride"]["pageSetViewportSizeUsed"] = True
                with self.assertRaises(phase2_common.AdapterError):
                    visual_contract(report, run_id="run-1")

    def test_packaged_source_freshness_fails_closed_on_receipt_drift(self):
        from diagnostic_bundle_source_manifest import SourceManifestFailure
        import cbsem_ml_v1_packaged_adapter as packaged

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            desktop = root / "target" / "release" / "quickpls-desktop.exe"
            cli = root / "target" / "release" / "qpls.exe"
            receipt = root / "validation" / "results" / "receipt.json"
            for path, value in ((desktop, b"desktop"), (cli, b"cli"), (receipt, b"{}")):
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(value)
            with (
                patch.object(packaged, "ROOT", root),
                patch.object(packaged, "DESKTOP", desktop),
                patch.object(packaged, "RELEASE_CLI", cli),
                patch.object(packaged, "BUILD_RECEIPT", receipt),
                patch.object(packaged, "strict_load_json", return_value={}),
                patch.object(
                    packaged,
                    "validate_build_receipt",
                    side_effect=SourceManifestFailure("source drift"),
                ),
            ):
                result = source_freshness()
            self.assertFalse(result["passed"])
            self.assertIn("source drift", result["error"])

    def test_manifest_has_exact_release_qualification_roles(self):
        evidence = self.document["qualification"]["evidence"]
        roles = {
            role
            for stage in ("engine_only", "archive_qualified", "native_qualified")
            for artifact in evidence[stage]
            for role in artifact["roles"]
        }
        self.assertEqual(EXPECTED_NATIVE_ROLES, roles)
        self.assertEqual(
            {tuple(row["roles"]) for row in evidence["release_qualified"]},
            {("packaged_acceptance",), ("method_audit",)},
        )
        self.assertEqual("release_qualified", self.document["qualification"]["declared_state"])

    def test_current_native_identity_reports_bind_exact_source_bytes(self):
        evidence = self.document["qualification"]["evidence"]
        artifacts = [
            artifact
            for stage in ("engine_only", "archive_qualified", "native_qualified")
            for artifact in evidence[stage]
        ]
        for artifact in artifacts:
            passed, errors = _verify_artifact(
                artifact, self.document, ROOT, self.expected_identity
            )
            self.assertTrue(passed, errors)
        result = validate_manifest(MANIFEST_PATH, ROOT)
        self.assertTrue(result["passed"], result["errors"])
        self.assertEqual("release_qualified", result["derived_state"])

    def test_identity_mutations_fail_closed(self):
        artifact = self.document["qualification"]["evidence"]["engine_only"][0]
        report_path = ROOT / artifact["path"]
        original = strict_load_json(report_path)
        mutations = {
            "passed": lambda row: row.__setitem__("passed", False),
            "feature_id": lambda row: row.__setitem__("feature_id", "qpls3.cbsem.other"),
            "method_version": lambda row: row.__setitem__("method_version", "cbsem_ml_v2"),
            "catalogue_snapshot_date": lambda row: row.__setitem__(
                "catalogue_snapshot_date", "2026-08-11"
            ),
            "source_sha256": lambda row: row["source_artifacts"][0].__setitem__(
                "sha256", "0" * 64
            ),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label):
                changed = copy.deepcopy(original)
                mutate(changed)
                with patch("method_promotion_manifest.strict_load_json", return_value=changed):
                    passed, errors = _verify_artifact(
                        artifact, self.document, ROOT, self.expected_identity
                    )
                self.assertFalse(passed)
                self.assertTrue(errors)


if __name__ == "__main__":
    unittest.main()
