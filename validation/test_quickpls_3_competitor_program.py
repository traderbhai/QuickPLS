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

from validation import (  # noqa: E402
    method_promotion_manifest,
    parity_ledger,
    quickpls_3_release_readiness,
    quickpls_external_beta,
)
from validation.quickpls_3_competitor_program import (  # noqa: E402
    APPROVAL_BINDING_IDS,
    EXPECTED_FACTORY_AUXILIARY_CAPABILITY_IDS,
    build_aggregate_approval_bindings,
    load_json,
    validate_aggregate_approval,
    validate_program,
    validate_program_document,
)


MANIFEST = VALIDATION_DIR / "quickpls_3_competitor_catalogue.json"
LEDGER = VALIDATION_DIR / "quickpls_3_parity_ledger.json"
READINESS = VALIDATION_DIR / "quickpls_3_release_readiness.json"
BETA = VALIDATION_DIR / "quickpls_external_beta.json"


class QuickPls3CompetitorProgramTests(unittest.TestCase):
    def setUp(self) -> None:
        self.manifest = load_json(MANIFEST)
        self.raw_ledger = load_json(LEDGER)
        self.parity_report = parity_ledger.validate_ledger(LEDGER, REPOSITORY_ROOT)
        self.commercial_report = quickpls_3_release_readiness.load_and_validate(
            READINESS,
            repository_root=REPOSITORY_ROOT,
        )
        self.beta_report = quickpls_external_beta.validate_contract(
            quickpls_external_beta.strict_json(BETA)
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
        beta_report=None,
        factory_report=None,
    ):
        return validate_program_document(
            self.manifest if manifest is None else manifest,
            self.parity_report if parity_report is None else parity_report,
            REPOSITORY_ROOT,
            commercial_readiness_report=(
                self.commercial_report if commercial_report is None else commercial_report
            ),
            external_beta_report=(
                self.beta_report if beta_report is None else beta_report
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
        self.assertTrue(report["capability_registry_passed"])
        self.assertTrue(report["parity_evidence_passed"])
        self.assertFalse(report["commercial_release_ready"])
        self.assertFalse(report["external_beta_ready"])
        self.assertTrue(report["method_manifest_factory_passed"])
        self.assertFalse(report["aggregate_approval_present"])
        self.assertFalse(report["aggregate_approval_passed"])
        self.assertEqual(report["catalogue_snapshot_date"], "2026-08-12")
        self.assertEqual(report["method_count"], 45)
        self.assertEqual(report["competitor_scope_count"], 43)
        self.assertEqual(
            report["status_counts"],
            {
                "absent": 14,
                "deferred": 2,
                "engine-preview": 4,
                "release-qualified": 25,
            },
        )
        self.assertEqual(len(report["pending_non_method_gates"]), 18)
        self.assertEqual(report["method_manifest_count"], 47)
        self.assertEqual(report["missing_method_manifests"], [])
        self.assertTrue(
            all(
                method["quickpls_capability_ids"]
                for method in self.manifest["methods"]
                if method["competitor_scope"]
            )
        )
        self.assertEqual(
            {
                method["id"]
                for method in self.manifest["methods"]
                if not method["competitor_scope"]
            },
            {"smartpls.blindfolding", "smartpls.gof"},
        )

    def test_frozen_catalogue_name_or_order_change_is_rejected(self) -> None:
        manifest = deepcopy(self.manifest)
        manifest["methods"][0]["official_method"] = "Renamed algorithm"

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(any("frozen catalogue" in error for error in report["errors"]))

    def test_legacy_deferrals_cannot_silently_reenter_competitor_scope(self) -> None:
        manifest = deepcopy(self.manifest)
        blindfolding = next(
            item for item in manifest["methods"] if item["id"] == "smartpls.blindfolding"
        )
        blindfolding["status"] = "absent"
        blindfolding["target_release"] = "2.48.0"
        blindfolding["competitor_scope"] = True

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(any("frozen legacy decision" in error for error in report["errors"]))
        self.assertTrue(any("requires a frozen capability ID" in error for error in report["errors"]))

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
        process = next(
            item for item in promoted["features"] if item["id"] == "qpls3.standalone.process"
        )
        process["state"] = "release_qualified"

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
        self.assertEqual(
            demoted_report["status_counts"],
            {
                "absent": 14,
                "deferred": 2,
                "engine-preview": 4,
                "release-qualified": 25,
            },
        )
        self.assertFalse(promoted_report["passed"])
        self.assertFalse(promoted_report["competitor_ready"])
        self.assertTrue(
            any("evidence-backed parity validation did not pass" in error for error in promoted_report["errors"])
        )

    def test_release_claim_requires_evidence_derived_release_state(self) -> None:
        manifest = deepcopy(self.manifest)
        method = next(
            item for item in manifest["methods"]
            if item["id"] == "smartpls.moderation"
        )
        method["status"] = "release-qualified"
        method["target_release"] = "current"

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(
            any(
                "contradicts authoritative Capability Registry V2 status engine-preview"
                in error
                for error in report["errors"]
            )
        )

    def test_absent_capability_raw_status_cannot_promote_manifest_state(self) -> None:
        manifest = deepcopy(self.manifest)
        method = next(
            item for item in manifest["methods"]
            if item["id"] == "smartpls.pls_model_comparison"
        )
        method["status"] = "release-qualified"
        method["target_release"] = "current"

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertFalse(report["competitor_ready"])
        self.assertTrue(
            any(
                "contradicts authoritative Capability Registry V2 status absent"
                in error
                for error in report["errors"]
            )
        )

    def test_extended_relationship_rows_match_factory_derived_states(self) -> None:
        expected = {
            "smartpls.moderation": "engine-preview",
            "smartpls.mediation": "engine-preview",
            "smartpls.nonlinear_relationships": "engine-preview",
            "smartpls.higher_order_models": "engine-preview",
            "smartpls.endogeneity_gaussian_copulas": "absent",
        }

        actual = {
            method["id"]: method["status"]
            for method in self.manifest["methods"]
            if method["id"] in expected
        }

        self.assertEqual(actual, expected)
        report = self.validate()
        self.assertTrue(report["passed"], report["errors"])

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
        self.assertTrue(any("capability mapping differs from Capability Registry V2" in error for error in report["errors"]))

    def test_shared_pca_mapping_cannot_be_removed_from_either_context(self) -> None:
        manifest = deepcopy(self.manifest)
        method = next(item for item in manifest["methods"] if item["id"] == "smartpls.pca_cbsem")
        method["quickpls_capability_ids"] = []

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(any("capability mapping differs from Capability Registry V2" in error for error in report["errors"]))

    def test_duplicate_capability_mapping_is_rejected(self) -> None:
        manifest = deepcopy(self.manifest)
        method = next(
            item for item in manifest["methods"]
            if item["id"] == "smartpls.pls_power_analysis"
        )
        method["quickpls_capability_ids"] = [
            "qpls3.pls.sample_size_power",
            "qpls3.pls.sample_size_power",
        ]

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(any("contains duplicates" in error for error in report["errors"]))

    def test_mapped_status_rejects_editable_implementation_paths(self) -> None:
        manifest = deepcopy(self.manifest)
        method = next(item for item in manifest["methods"] if item["id"] == "smartpls.cta_pls")
        method["implementation_evidence"] = ["crates/qpls-estimation/src/pls.rs"]

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(any("must not declare active implementation evidence" in error for error in report["errors"]))

    def test_absent_method_cannot_claim_preview_evidence(self) -> None:
        manifest = deepcopy(self.manifest)
        method = next(
            item for item in manifest["methods"]
            if item["id"] == "smartpls.pls_model_comparison"
        )
        method["implementation_evidence"] = ["crates/qpls-estimation/src/pls.rs"]

        report = self.validate(manifest=manifest)

        self.assertFalse(report["passed"])
        self.assertTrue(any("must not declare active implementation evidence" in error for error in report["errors"]))

    def test_missing_exact_method_manifest_is_rejected(self) -> None:
        factory_report = deepcopy(self.factory_report)
        factory_report["manifests"] = [
            result for result in factory_report["manifests"]
            if result.get("feature_id") != "qpls3.pls.sample_size_power"
        ]
        factory_report["manifest_count"] = len(factory_report["manifests"])
        factory_report["passed"] = True

        report = self.validate(factory_report=factory_report)

        self.assertFalse(report["passed"])
        self.assertFalse(report["competitor_ready"])
        self.assertTrue(any("missing frozen capabilities" in error for error in report["errors"]))
        self.assertIn("qpls3.pls.sample_size_power", report["missing_method_manifests"])

    def test_unmapped_or_duplicate_factory_capability_is_rejected(self) -> None:
        factory_report = deepcopy(self.factory_report)
        duplicate = deepcopy(factory_report["manifests"][0])
        duplicate["path"] = "validation/methods/duplicate.manifest.json"
        factory_report["manifests"].append(duplicate)
        factory_report["manifest_count"] = len(factory_report["manifests"])
        factory_report["passed"] = True

        report = self.validate(factory_report=factory_report)

        self.assertFalse(report["passed"])
        self.assertTrue(any("duplicate feature IDs" in error for error in report["errors"]))

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
        method = next(item for item in manifest["methods"] if item["id"] == "smartpls.mediation")
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

    def test_missing_external_beta_contract_cannot_produce_ready(self) -> None:
        missing = REPOSITORY_ROOT / "validation" / "missing_external_beta.json"

        report = validate_program(
            MANIFEST,
            LEDGER,
            REPOSITORY_ROOT,
            beta_contract_path=missing,
        )

        self.assertFalse(report["passed"])
        self.assertFalse(report["competitor_ready"])
        self.assertFalse(report["external_beta_ready"])
        self.assertTrue(any("external-beta validation did not pass" in error for error in report["errors"]))

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
            external_beta_report=self.beta_report,
            manifest_factory_report=self.factory_report,
        )
        missing_factory = validate_program_document(
            self.manifest,
            self.parity_report,
            REPOSITORY_ROOT,
            commercial_readiness_report=self.commercial_report,
            external_beta_report=self.beta_report,
            manifest_factory_report=None,
        )
        missing_beta = validate_program_document(
            self.manifest,
            self.parity_report,
            REPOSITORY_ROOT,
            commercial_readiness_report=self.commercial_report,
            external_beta_report=None,
            manifest_factory_report=self.factory_report,
        )

        self.assertFalse(missing_commercial["competitor_ready"])
        self.assertTrue(any("commercial-readiness report is required" in error for error in missing_commercial["errors"]))
        self.assertFalse(missing_factory["competitor_ready"])
        self.assertTrue(any("method-manifest factory report is required" in error for error in missing_factory["errors"]))
        self.assertFalse(missing_beta["competitor_ready"])
        self.assertTrue(any("external-beta report is required" in error for error in missing_beta["errors"]))

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
            "validation/quickpls_external_beta.py",
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
        beta = load_json(BETA)
        beta["status"] = "completed"
        beta["decision"] = {
            "status": "approved",
            "approved_by": "external-beta-board",
            "approved_at": "2026-08-13T23:30:00+00:00",
            "record_id": "approval:external-beta:final",
        }
        beta_path = root / "validation" / "quickpls_external_beta.json"
        beta_path.write_text(json.dumps(beta), encoding="utf-8")
        manifest_paths = []
        final_factory_ids = {
            capability
            for method in self.manifest["methods"]
            for capability in method["quickpls_capability_ids"]
        } | set(EXPECTED_FACTORY_AUXILIARY_CAPABILITY_IDS)
        for index, feature_id in enumerate(sorted(final_factory_ids)):
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
        self.final_beta_report = {
            "passed": True,
            "program_id": "quickpls_3_external_beta_v1",
            "target_release": "3.0.0-beta",
            "program_status": "completed",
            "beta_ready": True,
            "counts": {"participants": 15, "institutions": 5, "workflows": 30},
            "unassisted_journey_completion_rate": 1.0,
            "thresholds": {"all": True},
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
                beta_contract_path=root / "validation" / "quickpls_external_beta.json",
                parity_report=parity_report,
                commercial_readiness_report=commercial_report,
                external_beta_report=self.final_beta_report,
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
            beta_contract_path=root / "validation" / "quickpls_external_beta.json",
            parity_report=parity_report,
            commercial_readiness_report=commercial_report,
            external_beta_report=self.final_beta_report,
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
                beta_contract_path=root / "validation" / "quickpls_external_beta.json",
                parity_report=parity_report,
                commercial_readiness_report=commercial_report,
                external_beta_report=self.final_beta_report,
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

    def test_missing_external_beta_binding_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root, parity_report, commercial_report, factory_report = self._write_finalizable_inputs(
                Path(temporary_directory)
            )
            bindings = build_aggregate_approval_bindings(
                repository_root=root,
                catalogue_path=root / "validation" / "quickpls_3_competitor_catalogue.json",
                ledger_path=root / "validation" / "quickpls_3_parity_ledger.json",
                readiness_contract_path=root / "validation" / "quickpls_3_release_readiness.json",
                beta_contract_path=root / "validation" / "quickpls_external_beta.json",
                parity_report=parity_report,
                commercial_readiness_report=commercial_report,
                external_beta_report=self.final_beta_report,
                manifest_factory_report=factory_report,
            )
            del bindings["external_beta_report"]
            envelope = self._write_aggregate_envelope(
                root, parity_report, commercial_report, factory_report, bindings=bindings
            )

            report = self._validate_test_envelope(
                root, envelope, parity_report, commercial_report, factory_report
            )

        self.assertFalse(report["passed"])
        self.assertTrue(any("external_beta_report" in error for error in report["errors"]))

    def test_external_beta_contract_or_report_drift_invalidates_approval(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root, parity_report, commercial_report, factory_report = self._write_finalizable_inputs(
                Path(temporary_directory)
            )
            envelope = self._write_aggregate_envelope(
                root, parity_report, commercial_report, factory_report
            )
            beta_contract = root / "validation" / "quickpls_external_beta.json"
            original_contract = beta_contract.read_bytes()
            beta_contract.write_text(
                beta_contract.read_text(encoding="utf-8") + "\n", encoding="utf-8"
            )
            contract_drift = self._validate_test_envelope(
                root, envelope, parity_report, commercial_report, factory_report
            )

            beta_contract.write_bytes(original_contract)
            mutated_report = deepcopy(self.final_beta_report)
            mutated_report["counts"]["workflows"] = 31
            original_report = self.final_beta_report
            self.final_beta_report = mutated_report
            try:
                report_drift = self._validate_test_envelope(
                    root, envelope, parity_report, commercial_report, factory_report
                )
            finally:
                self.final_beta_report = original_report

        self.assertFalse(contract_drift["passed"])
        self.assertTrue(any("external_beta_contract" in error for error in contract_drift["errors"]))
        self.assertFalse(report_drift["passed"])
        self.assertTrue(any("external_beta_report" in error for error in report_drift["errors"]))

    def test_aggregate_rejects_beta_report_that_is_not_ready(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root, parity_report, commercial_report, factory_report = self._write_finalizable_inputs(
                Path(temporary_directory)
            )
            envelope = self._write_aggregate_envelope(
                root, parity_report, commercial_report, factory_report
            )
            original_report = self.final_beta_report
            self.final_beta_report = deepcopy(original_report)
            self.final_beta_report["beta_ready"] = False
            try:
                report = self._validate_test_envelope(
                    root, envelope, parity_report, commercial_report, factory_report
                )
            finally:
                self.final_beta_report = original_report

        self.assertFalse(report["passed"])
        self.assertTrue(any("before beta_ready" in error for error in report["errors"]))

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
