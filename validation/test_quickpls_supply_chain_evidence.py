from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from validation import quickpls_supply_chain_evidence as evidence


def descriptor(path: Path, root: Path) -> dict[str, object]:
    return {
        "path": path.relative_to(root).as_posix(),
        "bytes": path.stat().st_size,
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest().upper(),
        "copy_verified": True,
    }


def fixture(root: Path) -> tuple[Path, Path]:
    artifacts = root / "target" / "release" / "artifacts"
    artifacts.mkdir(parents=True, exist_ok=True)
    rows = []
    for role, payload in (
        ("portable", b"desktop"),
        ("cli", b"cli"),
        ("setup", b"setup"),
        ("checksums", b"hashes"),
    ):
        path = artifacts / f"candidate_{role}.bin"
        path.write_bytes(payload)
        rows.append({"role": role, "source": None, "source_bytes": None, "source_sha256": None, **descriptor(path, root)})
    report = {
        "schema_version": 2,
        "target": "QuickPLS unsigned preview artifact preservation",
        "passed": True,
        "version": "3.0.0",
        "version_contract": {},
        "release_channel": "unsigned-preview",
        "channel_policy": {},
        "trust": {
            "authenticode_required": False,
            "authenticode_verification_performed": False,
            "status": "not_verified",
            "stable_eligible": False,
            "competitor_claims_authorized": False,
        },
        "label": "fixture",
        "timestamp_utc": "20260813-120000",
        "artifact_directory": "target/release/artifacts",
        "artifacts": rows,
        "note": "unsigned",
    }
    report_path = root / "validation" / "results" / "release_artifacts.json"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report), encoding="utf-8")
    (root / "package-lock.json").write_text(
        json.dumps(
            {
                "name": "quickpls",
                "version": "3.0.0",
                "lockfileVersion": 3,
                "packages": {
                    "": {"name": "quickpls", "version": "3.0.0", "dependencies": {"runtime": "1.0.0"}},
                    "node_modules/runtime": {"name": "runtime", "version": "1.0.0", "license": "MIT"},
                    "node_modules/dev-only": {"name": "dev-only", "version": "2.0.0", "license": "MIT", "dev": True},
                },
            }
        ),
        encoding="utf-8",
    )
    cargo = {
        "packages": [
            {"id": "path+file:///quickpls#3.0.0", "name": "quickpls-desktop", "version": "3.0.0", "license": "LicenseRef-Proprietary"},
            {"id": "registry+crate#1.0.0", "name": "crate", "version": "1.0.0", "license": "Apache-2.0"},
        ],
        "resolve": {
            "nodes": [
                {"id": "path+file:///quickpls#3.0.0", "dependencies": ["registry+crate#1.0.0"]},
                {"id": "registry+crate#1.0.0", "dependencies": []},
            ]
        },
    }
    cargo_path = root / "validation" / "cargo-metadata.json"
    cargo_path.write_text(json.dumps(cargo), encoding="utf-8")
    return report_path, cargo_path


class SupplyChainEvidenceTests(unittest.TestCase):
    def generate(self, root: Path) -> dict[str, object]:
        report, cargo = fixture(root)
        generator = root / "validation" / "quickpls_supply_chain_evidence.py"
        generator.write_text("# fixture generator\n", encoding="utf-8")
        source = {"commit": "a" * 40, "branch": "test", "clean": False, "changed_repository_paths": ["source.ts"]}
        with mock.patch.object(evidence, "git_identity", return_value=source), mock.patch.object(
            evidence, "tool_version", return_value="fixture-tool 1"
        ):
            return evidence.generate_supply_chain_evidence(
                root=root,
                artifact_report_path=report,
                report_path=root / "validation" / "results" / "supply.json",
                output_directory=root / "target" / "release" / "artifacts",
                cargo_metadata_path=cargo,
                generator_path=generator,
                generated_at="2026-08-13T12:00:00Z",
            )

    def test_generates_bound_cyclonedx_license_and_provenance_for_unsigned_preview(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            report = self.generate(root)

            self.assertTrue(report["passed"])
            self.assertEqual(report["component_counts"], {"total": 4, "npm": 2, "cargo": 2})
            self.assertFalse(report["trust"]["commercial_gate_eligible"])
            self.assertFalse(report["trust"]["clean_source_checkout"])
            outputs = report["outputs"]
            self.assertEqual(set(outputs), {"sbom", "licenses", "provenance"})
            for item in outputs.values():
                path = root / item["path"]
                self.assertEqual(path.stat().st_size, item["size"])
                self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), item["sha256"])
            sbom = json.loads((root / outputs["sbom"]["path"]).read_text())
            self.assertEqual(sbom["bomFormat"], "CycloneDX")
            self.assertEqual(sbom["specVersion"], "1.6")
            dev = next(row for row in sbom["components"] if row["name"] == "dev-only")
            self.assertIn(
                {"name": "quickpls:distribution_scope", "value": "build"},
                dev["properties"],
            )

    def test_rejects_artifact_byte_drift_and_signed_channel_bypass(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            report_path, _ = fixture(root)
            document = json.loads(report_path.read_text())
            (root / document["artifacts"][0]["path"]).write_bytes(b"changed")
            with self.assertRaises(evidence.EvidenceError):
                evidence.validate_artifact_report(report_path, root)

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            report_path, _ = fixture(root)
            document = json.loads(report_path.read_text())
            document["release_channel"] = "stable"
            report_path.write_text(json.dumps(document), encoding="utf-8")
            with self.assertRaisesRegex(evidence.EvidenceError, "signed-candidate factory"):
                evidence.validate_artifact_report(report_path, root)

    def test_rejects_missing_license_duplicate_json_and_output_collision(self) -> None:
        with self.assertRaisesRegex(evidence.EvidenceError, "no license"):
            evidence.npm_components(
                {"packages": {"": {}, "node_modules/no-license": {"name": "no-license", "version": "1"}}}
            )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / "duplicate.json"
            path.write_text('{"passed":false,"passed":true}', encoding="utf-8")
            with self.assertRaises(evidence.EvidenceError):
                evidence.load_json(path)

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.generate(root)
            with self.assertRaisesRegex(evidence.EvidenceError, "refusing to overwrite"):
                self.generate(root)


if __name__ == "__main__":
    unittest.main()
