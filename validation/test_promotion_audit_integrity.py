#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import os
import sys
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
sys.path.insert(0, str(VALIDATION))

from performance_release_publication_audit import verify_release_artifacts
from promotion_audit_integrity import (
    evaluate_document,
    evaluate_report,
    explicit_pass_state,
    report_passed,
    write_method_audit,
)


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


class ExplicitPassStateTests(unittest.TestCase):
    def test_checks_without_overall_state_fail_closed(self) -> None:
        self.assertFalse(explicit_pass_state({"checks": {}})["passed"])
        self.assertFalse(explicit_pass_state({"checks": {"x": {"passed": True}}})["passed"])

    def test_explicit_positive_state_passes(self) -> None:
        self.assertTrue(explicit_pass_state({"passed": True})["passed"])
        self.assertTrue(explicit_pass_state({"status": "passed"})["passed"])
        self.assertTrue(explicit_pass_state({"qualification": {"passed": True}})["passed"])

    def test_contradictory_explicit_states_fail(self) -> None:
        state = explicit_pass_state({"passed": True, "qualification_passed": False})
        self.assertTrue(state["present"])
        self.assertFalse(state["passed"])

    def test_promotion_ready_requires_artifact_states(self) -> None:
        self.assertFalse(explicit_pass_state({"promotion_ready": True})["passed"])
        self.assertTrue(explicit_pass_state({
            "promotion_ready": True,
            "all_listed_artifacts_present": True,
            "all_listed_artifacts_passed": True,
        })["passed"])


class EvidenceBindingTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.results = self.root / "validation" / "results"
        self.results.mkdir(parents=True)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_missing_and_malformed_reports_fail_closed(self) -> None:
        self.assertFalse(evaluate_report(self.root, self.results, "missing.json")["passed"])
        malformed = self.results / "malformed.json"
        malformed.write_text("{", encoding="utf-8")
        self.assertFalse(evaluate_report(self.root, self.results, "malformed.json")["passed"])
        self.assertFalse(report_passed(malformed))

    def test_method_version_companion_and_hash_are_bound(self) -> None:
        write_json(self.results / "method.json", {
            "passed": True,
            "kind": "reference_v1",
            "checks": {"method": {"passed": True, "method_version": "method_v1"}},
        })
        write_json(self.results / "result.json", {
            "status": "completed",
            "payload": {"method_version": "method_v1"},
        })
        evidence = evaluate_report(self.root, self.results, {
            "name": "method.json",
            "required_values": {
                "kind": "reference_v1",
                "checks.method.method_version": "method_v1",
            },
            "required_true": ["checks.method.passed"],
            "companions": [{
                "path": "validation/results/result.json",
                "required_values": {
                    "status": "completed",
                    "payload.method_version": "method_v1",
                },
            }],
        })
        self.assertTrue(evidence["passed"])
        self.assertRegex(evidence["sha256"], r"^[0-9a-f]{64}$")
        self.assertRegex(evidence["companions"][0]["sha256"], r"^[0-9a-f]{64}$")

        companion = json.loads((self.results / "result.json").read_text(encoding="utf-8"))
        companion["payload"]["method_version"] = "wrong_v0"
        write_json(self.results / "result.json", companion)
        self.assertFalse(evaluate_report(self.root, self.results, {
            "name": "method.json",
            "companions": [{
                "path": "validation/results/result.json",
                "required_values": {"payload.method_version": "method_v1"},
            }],
        })["passed"])

    def test_source_newer_than_report_is_stale(self) -> None:
        report = self.results / "reference.json"
        source = self.root / "validation" / "reference.py"
        write_json(report, {"passed": True})
        source.write_text("# source\n", encoding="utf-8")
        os.utime(report, (1_700_000_000, 1_700_000_000))
        os.utime(source, (1_700_000_100, 1_700_000_100))
        evidence = evaluate_report(
            self.root,
            self.results,
            {"name": "reference.json", "source_paths": ["validation/reference.py"]},
            now=datetime(2026, 8, 12, tzinfo=timezone.utc),
        )
        self.assertFalse(evidence["freshness"]["passed"])
        self.assertFalse(evidence["passed"])

    def test_required_list_item_must_exist_once_and_pass(self) -> None:
        write_json(self.results / "manifest.json", {
            "passed": True,
            "artifacts": [{"file": "a.json", "present": True, "passed": True}],
        })
        spec = {
            "name": "manifest.json",
            "required_list_items": [{
                "path": "artifacts",
                "where": {"file": "a.json"},
                "required_true": ["present", "passed"],
            }],
        }
        self.assertTrue(evaluate_report(self.root, self.results, spec)["passed"])
        write_json(self.results / "manifest.json", {"passed": True, "artifacts": []})
        self.assertFalse(evaluate_report(self.root, self.results, spec)["passed"])

    def test_document_requires_semantic_phrases(self) -> None:
        doc = self.root / "docs" / "methods" / "METHOD.md"
        doc.parent.mkdir(parents=True)
        doc.write_text("Method v1 is diagnostic, not causal proof.", encoding="utf-8")
        self.assertTrue(evaluate_document(self.root, {
            "name": "METHOD.md",
            "required_phrases": ["method v1", "not causal proof"],
        })["passed"])
        self.assertFalse(evaluate_document(self.root, {
            "name": "METHOD.md",
            "required_phrases": ["bootstrap qualified"],
        })["passed"])

    def test_method_audit_writes_failed_state_for_unqualified_report(self) -> None:
        write_json(self.results / "checks_only.json", {"checks": {"x": {"passed": True}}})
        doc = self.root / "docs" / "methods" / "METHOD.md"
        doc.parent.mkdir(parents=True)
        doc.write_text("bounded scope", encoding="utf-8")
        code = write_method_audit(
            target="test",
            method_id="method",
            promoted_scope="bounded",
            required_reports=["checks_only.json"],
            required_docs=[{"name": "METHOD.md", "required_phrases": ["bounded scope"]}],
            root=self.root,
            results=self.results,
        )
        self.assertEqual(code, 1)
        output = json.loads((self.results / "method_method_promotion_audit.json").read_text(encoding="utf-8"))
        self.assertFalse(output["passed"])
        self.assertEqual(output["integrity_contract"], "explicit_pass_state_and_bound_evidence_v1")


class ReleaseArtifactIntegrityTests(unittest.TestCase):
    def test_release_artifacts_require_real_hash_matched_setup_and_portable(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            results = root / "validation" / "results"
            release = root / "release"
            results.mkdir(parents=True)
            release.mkdir()
            portable = release / "QuickPLS_3.0.0_x64_portable.exe"
            setup = release / "QuickPLS_3.0.0_x64_setup.exe"
            portable.write_bytes(b"portable-binary")
            setup.write_bytes(b"setup-binary")
            portable_hash = hashlib.sha256(portable.read_bytes()).hexdigest()
            setup_hash = hashlib.sha256(setup.read_bytes()).hexdigest()
            checksums = release / "QuickPLS_3.0.0_x64_checksums.txt"
            checksums.write_text(
                f"{portable_hash}  {portable.name}\n{setup_hash}  {setup.name}\n",
                encoding="utf-8",
            )
            checksum_hash = hashlib.sha256(checksums.read_bytes()).hexdigest()
            manifest = {
                "schema_version": 1,
                "target": "test release",
                "passed": True,
                "version": "3.0.0",
                "timestamp_utc": "2026-08-12T00:00:00Z",
                "artifacts": [
                    {
                        "path": f"target/release/artifacts/{portable.name}",
                        "bytes": portable.stat().st_size,
                        "sha256": portable_hash,
                    },
                    {
                        "path": f"target/release/artifacts/{setup.name}",
                        "bytes": setup.stat().st_size,
                        "sha256": setup_hash,
                    },
                    {
                        "path": f"target/release/artifacts/{checksums.name}",
                        "bytes": checksums.stat().st_size,
                        "sha256": checksum_hash,
                    },
                ],
            }
            manifest_path = results / "release_artifacts.json"
            write_json(manifest_path, manifest)
            evidence = verify_release_artifacts(
                manifest_path,
                release,
                "3.0.0",
                root=root,
                results=results,
            )
            self.assertTrue(evidence["passed"])
            setup.write_bytes(b"tampered")
            self.assertFalse(verify_release_artifacts(
                manifest_path,
                release,
                "3.0.0",
                root=root,
                results=results,
            )["passed"])


class StaticPromotionScriptTests(unittest.TestCase):
    def test_targeted_promotion_scripts_have_no_literal_passing_checks(self) -> None:
        targets = [
            "logistic_method_promotion_audit.py",
            "process_method_promotion_audit.py",
            "cta_pls_method_promotion_audit.py",
            "endogeneity_method_promotion_audit.py",
            "nonlinear_effects_method_promotion_audit.py",
            "moderated_mediation_method_promotion_audit.py",
            "fimix_pls_method_promotion_audit.py",
            "pls_pos_method_promotion_audit.py",
            "performance_release_publication_audit.py",
        ]
        for name in targets:
            with self.subTest(name=name):
                source = (VALIDATION / name).read_text(encoding="utf-8")
                self.assertNotIn('"passed": True', source)


if __name__ == "__main__":
    unittest.main()
