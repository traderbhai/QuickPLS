#!/usr/bin/env python3
"""Focused mutation tests for the QuickPLS 3 competitor-program validator."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from copy import deepcopy
from datetime import datetime, timezone
from pathlib import Path


VALIDATION_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = VALIDATION_DIR.parent
sys.path.insert(0, str(REPOSITORY_ROOT))

from validation import method_promotion_manifest, parity_ledger, quickpls_3_release_readiness  # noqa: E402
from validation.quickpls_3_competitor_program import (  # noqa: E402
    APPROVAL_BINDING_IDS,
    build_aggregate_approval_bindings,
    load_json,
    validate_aggregate_approval,
    validate_program,
    validate_program_document,
)


MANIFEST = VALIDATION_DIR / "quickpls_3_competitor_catalogue.json"
LEDGER = VALIDATION_DIR / "quickpls_3_parity_ledger.json"
READINESS = VALIDATION_DIR / "quickpls_3_release_readiness.json"


class QuickPls3CompetitorProgramTests(unittest.TestCase):
    def setUp(self) -> None:
        self.manifest = load_json(MANIFEST)
        self.raw_ledger = load_json(LEDGER)
        self.parity_report = parity_ledger.validate_ledger(LEDGER, REPOSITORY_ROOT)
        self.commercial_report = quickpls_3_release_readiness.load_and_validate(
            READINESS,
            repository_root=REPOSITORY_ROOT,
        )
        self.factory_report = method_promotion_manifest.validate_all(
            repository_root=REPOSITORY_ROOT
        )

    def validate(
        self,
        *,
        manifest=None,
        parity_report=None,
        commercial_report=None,
        factory_report=None,
    ):
        return validate_program_document(
            self.manifest if manifest is None else manifest,
            self.parity_report if parity_report is None else parity_report,
            REPOSITORY_ROOT,
            commercial_readiness_report=(
                self.commercial_report if commercial_report is None else commercial_report
            ),
            manifest_factory_report=(
                self.factory_report if factory_report is None else factory_report
            ),
            aggregate_approval_report={
                "present": False,
                "passed": False,
                "pending": True,
                "errors": [],
            },
        )

    def test_current_catalogue_is_valid_but_not_competitor_ready(self) -> None:
        report = validate_program(MANIFEST, LEDGER, REPOSITORY_ROOT)

        self.assertTrue(report["passed"], report["errors"])
        self.assertFalse(report["competitor_ready"])
        self.assertTrue(report["parity_evidence_passed"])
        self.assertFalse(report["commercial_release_ready"])
        self.assertTrue(report["method_manifest_factory_passed"])
        self.assertFalse(report["aggregate_approval_present"])
        self.assertFalse(report["aggregate_approval_passed"])
        self.assertEqual(report["catalogue_snapshot_date"], "2026-08-12")
        self.assertEqual(report["method_count"], 45)
        self.assertEqual(report["competitor_scope_count"], 44)
        self.assertEqual(
            report["status_counts"],
            {
                "absent": 8,
                "deferred": 1,
                "engine-preview": 14,
                "native-qualified": 18,
                "release-qualified": 4,
            },
        )
        self.assertEqual(len(report["pending_non_method_gates"]), 18)
        self.assertGreater(len(report["missing_method_manifests"]), 0)

    def test_frozen_catalogue_name_or_order_change_is_rejected(self) -> None:
        manifest = deepcopy(self.manifest)
        manifest["methods"][0]["official_method"] = "Renamed algorithm"

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(any("frozen catalogue" in error for error in report["errors"]))

    def test_duplicate_method_id_and_unknown_status_are_rejected(self) -> None:
        manifest = deepcopy(self.manifest)
        manifest["methods"][1]["id"] = manifest["methods"][0]["id"]
        manifest["methods"][2]["status"] = "planned"

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(any("method IDs must be unique" in error for error in report["errors"]))
        self.assertTrue(any("unknown status" in error for error in report["errors"]))

    def test_raw_ledger_state_cannot_promote_or_demote_evidence_derived_status(self) -> None:
        demoted = deepcopy(self.raw_ledger)
        algorithm = next(
            item for item in demoted["features"] if item["id"] == "qpls3.pls.algorithm"
        )
        algorithm["state"] = "engine_only"

        promoted = deepcopy(self.raw_ledger)
        algorithm = next(
            item for item in promoted["features"] if item["id"] == "qpls3.pls.algorithm"
        )
        algorithm["state"] = "release_qualified"

        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            demoted_path = directory / "demoted.json"
            promoted_path = directory / "promoted.json"
            demoted_path.write_text(json.dumps(demoted), encoding="utf-8")
            promoted_path.write_text(json.dumps(promoted), encoding="utf-8")

            demoted_report = validate_program(MANIFEST, demoted_path, REPOSITORY_ROOT)
            promoted_report = validate_program(MANIFEST, promoted_path, REPOSITORY_ROOT)

        self.assertTrue(demoted_report["passed"], demoted_report["errors"])
        self.assertFalse(demoted_report["competitor_ready"])
        self.assertEqual(demoted_report["status_counts"]["native-qualified"], 18)
        self.assertFalse(promoted_report["passed"])
        self.assertFalse(promoted_report["competitor_ready"])
        self.assertTrue(
            any("evidence-backed parity validation did not pass" in error for error in promoted_report["errors"])
        )

    def test_release_claim_requires_evidence_derived_release_state(self) -> None:
        manifest = deepcopy(self.manifest)
        method = manifest["methods"][0]
        method["status"] = "release-qualified"
        method["target_release"] = "current"

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(
            any("contradicts evidence-derived native-qualified" in error for error in report["errors"])
        )

    def test_borrowed_capability_is_rejected_by_exact_crosswalk(self) -> None:
        manifest = deepcopy(self.manifest)
        method = next(
            item for item in manifest["methods"]
            if item["id"] == "smartpls.pls_power_analysis"
        )
        method["quickpls_capability_ids"] = ["qpls3.pls.algorithm"]
        method["status"] = "native-qualified"

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertFalse(report["competitor_ready"])
        self.assertTrue(any("capability mapping differs from frozen crosswalk" in error for error in report["errors"]))

    def test_shared_pca_mapping_cannot_be_removed_from_either_context(self) -> None:
        manifest = deepcopy(self.manifest)
        method = next(item for item in manifest["methods"] if item["id"] == "smartpls.pca_cbsem")
        method["quickpls_capability_ids"] = []

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(any("capability mapping differs from frozen crosswalk" in error for error in report["errors"]))

    def test_engine_preview_requires_existing_repository_evidence(self) -> None:
        manifest = deepcopy(self.manifest)
        method = next(item for item in manifest["methods"] if item["id"] == "smartpls.cta_pls")
        method["implementation_evidence"] = ["validation/results/does-not-exist.json"]

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(any("implementation evidence is missing" in error for error in report["errors"]))

    def test_absent_method_cannot_claim_preview_evidence(self) -> None:
        manifest = deepcopy(self.manifest)
        method = next(
            item for item in manifest["methods"]
            if item["id"] == "smartpls.pls_power_analysis"
        )
        method["implementation_evidence"] = ["crates/qpls-estimation/src/pls.rs"]

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(any("absent must not declare" in error for error in report["errors"]))

    def test_unknown_dependency_and_dependency_cycle_are_rejected(self) -> None:
        manifest = deepcopy(self.manifest)
        algorithm = next(item for item in manifest["methods"] if item["id"] == "smartpls.pls_algorithm")
        power = next(item for item in manifest["methods"] if item["id"] == "smartpls.pls_power_analysis")
        algorithm["dependencies"] = [power["id"]]
        power["dependencies"] = [algorithm["id"], "smartpls.unknown"]

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(any("unknown dependency" in error for error in report["errors"]))
        self.assertTrue(any("dependency cycle detected" in error for error in report["errors"]))

    def test_dependency_cannot_target_a_later_release(self) -> None:
        manifest = deepcopy(self.manifest)
        method = next(item for item in manifest["methods"] if item["id"] == "smartpls.cta_pls")
        method["dependencies"].append("smartpls.cbsem_model_comparison")

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(any("targets later release" in error for error in report["errors"]))

    def test_completed_planning_flags_cannot_replace_commercial_evidence(self) -> None:
        manifest = deepcopy(self.manifest)
        manifest["competitor_claim_gate"]["non_method_gates"] = [
            {"id": "everything", "status": "complete", "target_release": "3.0.0"}
        ]

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertFalse(report["competitor_ready"])
        self.assertFalse(report["commercial_release_ready"])
        self.assertTrue(any("keys differ" in error for error in report["errors"]))

    def test_missing_commercial_contract_cannot_produce_ready(self) -> None:
        missing = REPOSITORY_ROOT / "validation" / "missing_release_readiness.json"

        report = validate_program(
            MANIFEST,
            LEDGER,
            REPOSITORY_ROOT,
            readiness_contract_path=missing,
        )

        self.assertFalse(report["passed"])
        self.assertFalse(report["competitor_ready"])
        self.assertTrue(any("commercial-readiness validation did not pass" in error for error in report["errors"]))

    def test_missing_method_manifest_cannot_produce_ready(self) -> None:
        missing = REPOSITORY_ROOT / "validation" / "methods" / "missing.manifest.json"

        report = validate_program(
            MANIFEST,
            LEDGER,
            REPOSITORY_ROOT,
            method_manifest_paths=[missing],
        )

        self.assertFalse(report["passed"])
        self.assertFalse(report["competitor_ready"])
        self.assertTrue(any("method-manifest factory validation did not pass" in error for error in report["errors"]))

    def test_precomputed_external_reports_are_required(self) -> None:
        missing_commercial = validate_program_document(
            self.manifest,
            self.parity_report,
            REPOSITORY_ROOT,
            commercial_readiness_report=None,
            manifest_factory_report=self.factory_report,
        )
        missing_factory = validate_program_document(
            self.manifest,
            self.parity_report,
            REPOSITORY_ROOT,
            commercial_readiness_report=self.commercial_report,
            manifest_factory_report=None,
        )

        self.assertFalse(missing_commercial["competitor_ready"])
        self.assertTrue(any("commercial-readiness report is required" in error for error in missing_commercial["errors"]))
        self.assertFalse(missing_factory["competitor_ready"])
        self.assertTrue(any("method-manifest factory report is required" in error for error in missing_factory["errors"]))

    def _write_finalizable_inputs(self, directory: Path):
        root = directory / "repo"
        (root / "validation" / "methods").mkdir(parents=True)
        (root / "validation" / "results").mkdir(parents=True)
        (root / "validation" / "quickpls_3_competitor_catalogue.json").write_text(
            MANIFEST.read_text(encoding="utf-8"), encoding="utf-8"
        )
        (root / "validation" / "quickpls_3_parity_ledger.json").write_text(
            LEDGER.read_text(encoding="utf-8"), encoding="utf-8"
        )
        for relative in (
            "validation/parity_ledger.py",
            "validation/quickpls_3_release_readiness.py",
            "validation/method_promotion_manifest.py",
            "validation/methods/method_promotion_manifest.schema.json",
        ):
            destination = root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes((REPOSITORY_ROOT / relative).read_bytes())

        readiness = load_json(READINESS)
        readiness["overall_status"] = "passed"
        readiness["release_decision"] = {
            "status": "approved",
            "approved_by": "release-manager",
            "approved_at": "2026-08-14T00:00:00Z",
            "record": "validation/results/release_decision.json",
        }
        readiness_path = root / "validation" / "quickpls_3_release_readiness.json"
        readiness_path.write_text(json.dumps(readiness), encoding="utf-8")
        manifest_paths = []
        for index, feature_id in enumerate(sorted({
            capability
            for method in self.manifest["methods"]
            for capability in method["quickpls_capability_ids"]
        })):
            path = root / "validation" / "methods" / f"method_{index:02}.manifest.json"
            path.write_text(
                json.dumps(
                    {
                        "governance": {"contract_frozen_at_utc": "2026-08-13T00:00:00Z"},
                        "feature": {"id": feature_id},
                    }
                ),
                encoding="utf-8",
            )
            manifest_paths.append(path)

        parity_report = deepcopy(self.parity_report)
        commercial_report = {
            "structurally_valid": True,
            "target_release": "3.0.0",
            "release_ready": True,
            "pending": [],
            "failed": [],
            "release_decision": "approved",
        }
        factory_report = {
            "passed": True,
            "manifest_count": len(manifest_paths),
            "manifests": [
                {
                    "passed": True,
                    "path": str(path),
                    "feature_id": load_json(path)["feature"]["id"],
                    "catalogue_snapshot_date": "2026-08-12",
                    "derived_state": "release_qualified",
                    "errors": [],
                }
                for path in manifest_paths
            ],
            "errors": [],
        }
        return root, parity_report, commercial_report, factory_report

    def _write_aggregate_envelope(
        self,
        root: Path,
        parity_report,
        commercial_report,
        factory_report,
        *,
        bindings=None,
        assembled_at="2026-08-14T00:01:00Z",
        approved_at="2026-08-14T00:02:00Z",
    ) -> Path:
        if bindings is None:
            bindings = build_aggregate_approval_bindings(
                repository_root=root,
                catalogue_path=root / "validation" / "quickpls_3_competitor_catalogue.json",
                ledger_path=root / "validation" / "quickpls_3_parity_ledger.json",
                readiness_contract_path=root / "validation" / "quickpls_3_release_readiness.json",
                parity_report=parity_report,
                commercial_readiness_report=commercial_report,
                manifest_factory_report=factory_report,
            )
        envelope = {
            "schema_version": 1,
            "approval_id": "quickpls_3_competitor_3_0_0_final",
            "target_release": "3.0.0",
            "catalogue_snapshot_date": "2026-08-12",
            "hash_algorithm": "sha256",
            "approved": True,
            "approved_by": "competitor-release-board",
            "assembled_at_utc": assembled_at,
            "approved_at_utc": approved_at,
            "bindings": bindings,
        }
        path = root / "validation" / "results" / "quickpls_3_competitor_approval.json"
        path.write_text(json.dumps(envelope), encoding="utf-8")
        return path

    def _validate_test_envelope(
        self,
        root,
        envelope,
        parity_report,
        commercial_report,
        factory_report,
    ):
        return validate_aggregate_approval(
            envelope,
            repository_root=root,
            catalogue_path=root / "validation" / "quickpls_3_competitor_catalogue.json",
            ledger_path=root / "validation" / "quickpls_3_parity_ledger.json",
            readiness_contract_path=root / "validation" / "quickpls_3_release_readiness.json",
            parity_report=parity_report,
            commercial_readiness_report=commercial_report,
            manifest_factory_report=factory_report,
            now=datetime(2026, 8, 15, tzinfo=timezone.utc),
        )

    def test_complete_aggregate_approval_binds_all_inputs_and_postdates_them(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root, parity_report, commercial_report, factory_report = self._write_finalizable_inputs(
                Path(temporary_directory)
            )
            envelope = self._write_aggregate_envelope(
                root, parity_report, commercial_report, factory_report
            )

            report = self._validate_test_envelope(
                root, envelope, parity_report, commercial_report, factory_report
            )

        self.assertTrue(report["passed"], report["errors"])
        self.assertEqual(set(report["binding_ids"]), set(APPROVAL_BINDING_IDS))

    def test_catalogue_change_after_approval_invalidates_binding(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root, parity_report, commercial_report, factory_report = self._write_finalizable_inputs(
                Path(temporary_directory)
            )
            envelope = self._write_aggregate_envelope(
                root, parity_report, commercial_report, factory_report
            )
            catalogue = root / "validation" / "quickpls_3_competitor_catalogue.json"
            catalogue.write_text(catalogue.read_text(encoding="utf-8") + "\n", encoding="utf-8")

            report = self._validate_test_envelope(
                root, envelope, parity_report, commercial_report, factory_report
            )

        self.assertFalse(report["passed"])
        self.assertTrue(any("competitor_catalogue" in error for error in report["errors"]))

    def test_parity_change_after_approval_invalidates_file_or_report_binding(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root, parity_report, commercial_report, factory_report = self._write_finalizable_inputs(
                Path(temporary_directory)
            )
            envelope = self._write_aggregate_envelope(
                root, parity_report, commercial_report, factory_report
            )
            mutated_report = deepcopy(parity_report)
            mutated_report["derived_states"] = {"release_qualified": 17}

            report = self._validate_test_envelope(
                root, envelope, mutated_report, commercial_report, factory_report
            )

        self.assertFalse(report["passed"])
        self.assertTrue(any("parity_report" in error for error in report["errors"]))

    def test_method_manifest_change_after_approval_invalidates_set_binding(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root, parity_report, commercial_report, factory_report = self._write_finalizable_inputs(
                Path(temporary_directory)
            )
            envelope = self._write_aggregate_envelope(
                root, parity_report, commercial_report, factory_report
            )
            manifest = sorted((root / "validation" / "methods").glob("*.manifest.json"))[0]
            manifest.write_text(manifest.read_text(encoding="utf-8") + "\n", encoding="utf-8")

            report = self._validate_test_envelope(
                root, envelope, parity_report, commercial_report, factory_report
            )

        self.assertFalse(report["passed"])
        self.assertTrue(any("method_manifest_set" in error for error in report["errors"]))

    def test_missing_aggregate_binding_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root, parity_report, commercial_report, factory_report = self._write_finalizable_inputs(
                Path(temporary_directory)
            )
            bindings = build_aggregate_approval_bindings(
                repository_root=root,
                catalogue_path=root / "validation" / "quickpls_3_competitor_catalogue.json",
                ledger_path=root / "validation" / "quickpls_3_parity_ledger.json",
                readiness_contract_path=root / "validation" / "quickpls_3_release_readiness.json",
                parity_report=parity_report,
                commercial_readiness_report=commercial_report,
                manifest_factory_report=factory_report,
            )
            del bindings["method_manifest_report"]
            envelope = self._write_aggregate_envelope(
                root,
                parity_report,
                commercial_report,
                factory_report,
                bindings=bindings,
            )

            report = self._validate_test_envelope(
                root, envelope, parity_report, commercial_report, factory_report
            )

        self.assertFalse(report["passed"])
        self.assertTrue(any("binding IDs differ" in error for error in report["errors"]))

    def test_added_method_manifest_after_approval_invalidates_exact_file_set(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root, parity_report, commercial_report, factory_report = self._write_finalizable_inputs(
                Path(temporary_directory)
            )
            envelope = self._write_aggregate_envelope(
                root, parity_report, commercial_report, factory_report
            )
            added = root / "validation" / "methods" / "added.manifest.json"
            added.write_text(
                json.dumps(
                    {
                        "governance": {"contract_frozen_at_utc": "2026-08-14T00:01:30Z"},
                        "feature": {"id": "qpls3.unapproved.addition"},
                    }
                ),
                encoding="utf-8",
            )

            report = self._validate_test_envelope(
                root, envelope, parity_report, commercial_report, factory_report
            )

        self.assertFalse(report["passed"])
        self.assertTrue(any("method_manifest_set" in error for error in report["errors"]))

    def test_approval_must_postdate_assembly_and_commercial_decision(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root, parity_report, commercial_report, factory_report = self._write_finalizable_inputs(
                Path(temporary_directory)
            )
            envelope = self._write_aggregate_envelope(
                root,
                parity_report,
                commercial_report,
                factory_report,
                assembled_at="2026-08-13T23:59:00Z",
                approved_at="2026-08-13T23:58:00Z",
            )

            report = self._validate_test_envelope(
                root, envelope, parity_report, commercial_report, factory_report
            )

        self.assertFalse(report["passed"])
        self.assertTrue(any("postdate digest assembly" in error for error in report["errors"]))
        self.assertTrue(any("must not predate commercial release approval" in error for error in report["errors"]))

    def test_snapshot_drift_in_parity_or_method_manifests_is_rejected(self) -> None:
        parity_report = deepcopy(self.parity_report)
        parity_report["catalogue_snapshot"]["date"] = "2026-08-13"
        parity_drift = self.validate(parity_report=parity_report)

        factory_report = deepcopy(self.factory_report)
        factory_report["manifests"][0]["catalogue_snapshot_date"] = "2026-08-13"
        manifest_drift = self.validate(factory_report=factory_report)

        self.assertFalse(parity_drift["passed"])
        self.assertFalse(parity_drift["competitor_ready"])
        self.assertTrue(any("snapshot date differs" in error for error in parity_drift["errors"]))
        self.assertFalse(manifest_drift["passed"])
        self.assertFalse(manifest_drift["competitor_ready"])
        self.assertTrue(any("snapshot date differs" in error for error in manifest_drift["errors"]))

    def test_malformed_dependency_fails_closed(self) -> None:
        manifest = deepcopy(self.manifest)
        manifest["methods"][0]["dependencies"] = [{"not": "an id"}]

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(any("dependencies must contain only strings" in error for error in report["errors"]))

    def test_duplicate_json_keys_are_rejected_for_catalogue_and_ledger(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            manifest = directory / "manifest.json"
            ledger = directory / "ledger.json"
            manifest.write_text('{"schema_version":1,"schema_version":1}', encoding="utf-8")
            ledger.write_text('{"schema_version":1,"schema_version":1}', encoding="utf-8")

            manifest_report = validate_program(manifest, LEDGER, REPOSITORY_ROOT)
            ledger_report = validate_program(MANIFEST, ledger, REPOSITORY_ROOT)

        self.assertFalse(manifest_report["passed"])
        self.assertTrue(any("duplicate JSON key" in error for error in manifest_report["errors"]))
        self.assertFalse(ledger_report["passed"])
        self.assertTrue(any("duplicate JSON key" in error for error in ledger_report["errors"]))

    def test_nonfinite_ledger_numbers_are_rejected_before_parity_evaluation(self) -> None:
        for constant in ("NaN", "Infinity", "-Infinity"):
            with self.subTest(constant=constant), tempfile.TemporaryDirectory() as temporary_directory:
                ledger = Path(temporary_directory) / "ledger.json"
                ledger.write_text(f'{{"schema_version":{constant}}}', encoding="utf-8")

                report = validate_program(MANIFEST, ledger, REPOSITORY_ROOT)

                self.assertFalse(report["passed"])
                self.assertFalse(report["competitor_ready"])
                self.assertTrue(any("non-finite JSON constant" in error for error in report["errors"]))


if __name__ == "__main__":
    unittest.main()
