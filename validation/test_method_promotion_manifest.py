#!/usr/bin/env python3
"""Mutation-focused tests for the QuickPLS method-promotion manifest factory."""

from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path
from typing import Any


VALIDATION_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = VALIDATION_DIR.parent
MANIFEST_DIR = VALIDATION_DIR / "methods"
sys.path.insert(0, str(VALIDATION_DIR))

from method_promotion_manifest import (  # noqa: E402
    FOCUSED_TEST_RELATIVE_PATH,
    SCHEMA_RELATIVE_PATH,
    VALIDATOR_RELATIVE_PATH,
    strict_load_json,
    validate_all,
    validate_manifest,
    validate_manifest_document,
)


STRUCTURAL_MANIFEST = MANIFEST_DIR / "structural_path_randomization_v1.manifest.json"
PLANNED_MANIFEST = MANIFEST_DIR / "history" / "pls_sample_size_power_v1.manifest.json"

LEGACY_RELEASE_MIGRATIONS = {
    "qpls3.inference.structural_path_randomization": {
        "manifest": STRUCTURAL_MANIFEST,
        "reports": (
            "validation/results/structural_path_randomization_method_promotion_audit.json",
            "validation/results/structural_path_randomization_v1_packaged_acceptance.json",
        ),
    },
    "qpls3.standalone.logistic": {
        "manifest": MANIFEST_DIR / "logistic_regression_v2.manifest.json",
        "reports": (
            "validation/results/logistic_method_promotion_audit.json",
            "validation/results/logistic_v2_packaged_acceptance.json",
        ),
    },
    "qpls3.standalone.regression_bootstrap": {
        "manifest": MANIFEST_DIR / "regression_bootstrap_v1.manifest.json",
        "reports": (
            "validation/results/regression_bootstrap_method_promotion_audit.json",
            "validation/results/regression_bootstrap_v1_packaged_acceptance.json",
        ),
    },
    "qpls3.standalone.process": {
        "manifest": MANIFEST_DIR / "process_v2.manifest.json",
        "reports": (
            "validation/results/process_v2_method_promotion_audit.json",
            "validation/results/process_v2_packaged_acceptance.json",
        ),
    },
}

IDENTITY_VERIFICATION = {
    "kind": "identity_report",
    "identity_pointers": {
        "passed": "/passed",
        "feature_id": "/feature_id",
        "method_version": "/method_version",
        "catalogue_snapshot_date": "/catalogue_snapshot_date",
    },
    "source_artifacts_pointer": "/source_artifacts",
    "generated_at_pointer": "/generated_at_utc",
}


def _write(root: Path, relative: str, content: bytes) -> Path:
    path = root / Path(relative)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content)
    return path


def _descriptor(root: Path, relative: str) -> dict[str, Any]:
    content = (root / Path(relative)).read_bytes()
    return {
        "path": relative,
        "size": len(content),
        "sha256": hashlib.sha256(content).hexdigest(),
    }


class MethodPromotionManifestTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.structural_document = strict_load_json(STRUCTURAL_MANIFEST)
        cls.planned_document = strict_load_json(PLANNED_MANIFEST)

    def _engine_fixture(
        self,
        temporary_directory: str,
        *,
        report_generated_at: str | None = None,
        readme_forgery: bool = False,
    ) -> tuple[Path, Path, dict[str, Any]]:
        root = Path(temporary_directory)
        document = deepcopy(self.planned_document)
        if report_generated_at is None:
            report_generated_at = document["governance"]["contract_frozen_at_utc"]
        document["qualification"]["declared_state"] = "engine_only"
        manifest_relative = document["governance"]["manifest_path"]
        report_relative = "validation/results/planned_engine_attestation.json"
        verification = {
            "kind": "identity_report",
            "identity_pointers": {
                "passed": "/passed",
                "feature_id": "/feature_id",
                "method_version": "/method_version",
                "catalogue_snapshot_date": "/catalogue_snapshot_date",
            },
            "source_artifacts_pointer": "/source_artifacts",
            "generated_at_pointer": "/generated_at_utc",
        }
        document["qualification"]["evidence"]["engine_only"] = [
            {
                "path": report_relative,
                "roles": [
                    "method_spec",
                    "independent_reference",
                    "simulation_report",
                    "boundary_report",
                ],
                "verification": verification,
            }
        ]
        if readme_forgery:
            document["qualification"]["source_requirements"]["method_spec"] = [
                "README.md"
            ]

        _write(root, SCHEMA_RELATIVE_PATH, (MANIFEST_DIR / "method_promotion_manifest.schema.json").read_bytes())
        _write(root, VALIDATOR_RELATIVE_PATH, (VALIDATION_DIR / "method_promotion_manifest.py").read_bytes())
        _write(root, FOCUSED_TEST_RELATIVE_PATH, Path(__file__).read_bytes())

        source_requirements = document["qualification"]["source_requirements"]
        role_paths = {
            path
            for role in (
                "method_spec",
                "independent_reference",
                "simulation_report",
                "boundary_report",
            )
            for path in source_requirements[role]
        }
        for relative in role_paths:
            _write(root, relative, f"fixture source: {relative}\n".encode("utf-8"))

        manifest_path = _write(
            root,
            manifest_relative,
            (json.dumps(document, indent=2) + "\n").encode("utf-8"),
        )
        required_paths = {
            manifest_relative,
            SCHEMA_RELATIVE_PATH,
            VALIDATOR_RELATIVE_PATH,
            FOCUSED_TEST_RELATIVE_PATH,
            *role_paths,
        }
        report = {
            "passed": True,
            "feature_id": document["feature"]["id"],
            "method_version": document["feature"]["method_version"],
            "catalogue_snapshot_date": document["feature"]["catalogue_snapshot_date"],
            "generated_at_utc": report_generated_at,
            "source_artifacts": [
                _descriptor(root, relative) for relative in sorted(required_paths)
            ],
        }
        report_path = _write(
            root,
            report_relative,
            (json.dumps(report, indent=2) + "\n").encode("utf-8"),
        )
        return manifest_path, report_path, document

    def test_all_factory_manifests_are_truthful_and_migrations_do_not_overclaim(self) -> None:
        report = validate_all(repository_root=REPOSITORY_ROOT)

        self.assertTrue(report["passed"], report)
        states = {
            row["feature_id"]: (row["declared_state"], row["derived_state"])
            for row in report["manifests"]
        }
        self.assertGreaterEqual(report["manifest_count"], len(LEGACY_RELEASE_MIGRATIONS) + 1)
        for feature_id, migration in LEGACY_RELEASE_MIGRATIONS.items():
            with self.subTest(feature_id=feature_id):
                manifest = strict_load_json(migration["manifest"])
                self.assertEqual(
                    states[feature_id],
                    (
                        manifest["qualification"]["declared_state"],
                        manifest["qualification"]["declared_state"],
                    ),
                )
        self.assertEqual(
            states["qpls3.pls.sample_size_power"],
            ("release_qualified", "release_qualified"),
        )

    def test_legacy_release_reports_cannot_bypass_factory_source_binding(self) -> None:
        """Passing pre-factory reports remain historical, not factory attestations."""

        for feature_id, migration in LEGACY_RELEASE_MIGRATIONS.items():
            for legacy_report in migration["reports"]:
                with self.subTest(feature_id=feature_id, legacy_report=legacy_report):
                    document = strict_load_json(migration["manifest"])
                    document["qualification"]["evidence"]["engine_only"] = [
                        {
                            "path": legacy_report,
                            "roles": [
                                "method_spec",
                                "independent_reference",
                                "simulation_report",
                                "boundary_report",
                            ],
                            "verification": deepcopy(IDENTITY_VERIFICATION),
                        }
                    ]

                    result = validate_manifest_document(document, REPOSITORY_ROOT)

                    self.assertFalse(result["passed"], result)
                    self.assertEqual(result["derived_state"], "absent")
                    self.assertTrue(
                        any("source_artifacts" in error for error in result["errors"]),
                        result,
                    )

    def test_legacy_release_migration_scaffolds_name_real_sources(self) -> None:
        for feature_id, migration in LEGACY_RELEASE_MIGRATIONS.items():
            document = strict_load_json(migration["manifest"])
            source_requirements = document["qualification"]["source_requirements"]
            for role, paths in source_requirements.items():
                for relative in paths:
                    with self.subTest(
                        feature_id=feature_id,
                        role=role,
                        source=relative,
                    ):
                        self.assertTrue(
                            (REPOSITORY_ROOT / relative).is_file(),
                            f"missing {role} scaffold source: {relative}",
                        )

    def test_stdlib_validator_accepts_exact_engine_attestation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            manifest_path, _, _ = self._engine_fixture(temporary_directory)
            report = validate_manifest(manifest_path, Path(temporary_directory))

        self.assertTrue(report["passed"], report)
        self.assertEqual(report["derived_state"], "engine_only")

    def test_unknown_manifest_property_is_rejected(self) -> None:
        document = deepcopy(self.planned_document)
        document["marketing_claim"] = "fully equivalent"

        report = validate_manifest_document(
            document, REPOSITORY_ROOT, verify_evidence=False
        )

        self.assertFalse(report["passed"])
        self.assertTrue(any("additional property" in error for error in report["errors"]))

    def test_duplicate_json_key_is_rejected_before_schema_validation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            path = Path(temporary_directory) / "duplicate.manifest.json"
            path.write_text(
                '{"schema_version":1,"schema_version":1}', encoding="utf-8"
            )
            report = validate_manifest(path, REPOSITORY_ROOT)

        self.assertFalse(report["passed"])
        self.assertTrue(any("duplicate JSON key" in error for error in report["errors"]))

    def test_non_finite_json_number_is_rejected_before_schema_validation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            path = Path(temporary_directory) / "non-finite.manifest.json"
            path.write_text(
                '{"schema_version":1,"numerical_tolerance":NaN}', encoding="utf-8"
            )
            report = validate_manifest(path, REPOSITORY_ROOT)

        self.assertFalse(report["passed"])
        self.assertTrue(any("non-finite JSON number" in error for error in report["errors"]))

    def test_exists_verification_is_not_a_valid_evidence_escape_hatch(self) -> None:
        document = deepcopy(self.planned_document)
        document["qualification"]["evidence"]["engine_only"] = [
            {
                "path": "docs/methods/PLS_SAMPLE_SIZE_POWER_V1.md",
                "roles": ["method_spec"],
                "verification": {"kind": "exists"},
            }
        ]

        report = validate_manifest_document(
            document, REPOSITORY_ROOT, verify_evidence=False
        )

        self.assertFalse(report["passed"])
        self.assertTrue(any("/verification" in error for error in report["errors"]))

    def test_readme_cannot_be_forged_as_method_spec_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            manifest_path, _, _ = self._engine_fixture(
                temporary_directory, readme_forgery=True
            )
            report = validate_manifest(manifest_path, Path(temporary_directory))

        self.assertFalse(report["passed"])
        self.assertIn(
            "method_spec sources must be Markdown files under docs/methods/",
            report["errors"],
        )

    def test_source_hash_mutation_invalidates_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            manifest_path, _, document = self._engine_fixture(temporary_directory)
            source = document["qualification"]["source_requirements"][
                "independent_reference"
            ][0]
            with (root / Path(source)).open("ab") as handle:
                handle.write(b"tamper")
            report = validate_manifest(manifest_path, root)

        self.assertFalse(report["passed"])
        self.assertTrue(any("SHA-256 mismatch" in error for error in report["errors"]))

    def test_source_size_mutation_invalidates_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            manifest_path, report_path, _ = self._engine_fixture(temporary_directory)
            evidence = strict_load_json(report_path)
            evidence["source_artifacts"][0]["size"] += 1
            report_path.write_text(json.dumps(evidence), encoding="utf-8")
            report = validate_manifest(manifest_path, root)

        self.assertFalse(report["passed"])
        self.assertTrue(any("size mismatch" in error for error in report["errors"]))

    def test_source_descriptor_rejects_extra_forgery_fields(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            manifest_path, report_path, _ = self._engine_fixture(temporary_directory)
            evidence = strict_load_json(report_path)
            evidence["source_artifacts"][0]["trusted"] = True
            report_path.write_text(json.dumps(evidence), encoding="utf-8")
            report = validate_manifest(manifest_path, root)

        self.assertFalse(report["passed"])
        self.assertTrue(
            any("must contain exactly path, size, and sha256" in error for error in report["errors"])
        )

    def test_stale_report_invalidates_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            manifest_path, _, _ = self._engine_fixture(
                temporary_directory,
                report_generated_at="2026-08-12T23:59:59Z",
            )
            report = validate_manifest(manifest_path, Path(temporary_directory))

        self.assertFalse(report["passed"])
        self.assertTrue(any("report is stale" in error for error in report["errors"]))

    def test_missing_factory_source_binding_invalidates_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            manifest_path, report_path, _ = self._engine_fixture(temporary_directory)
            evidence = strict_load_json(report_path)
            evidence["source_artifacts"] = [
                descriptor
                for descriptor in evidence["source_artifacts"]
                if descriptor["path"] != VALIDATOR_RELATIVE_PATH
            ]
            report_path.write_text(json.dumps(evidence), encoding="utf-8")
            report = validate_manifest(manifest_path, root)

        self.assertFalse(report["passed"])
        self.assertTrue(
            any(VALIDATOR_RELATIVE_PATH in error for error in report["errors"])
        )

    def test_method_identity_mutation_invalidates_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            manifest_path, report_path, _ = self._engine_fixture(temporary_directory)
            evidence = strict_load_json(report_path)
            evidence["method_version"] = "pls_sample_size_power_v2"
            report_path.write_text(json.dumps(evidence), encoding="utf-8")
            report = validate_manifest(manifest_path, root)

        self.assertFalse(report["passed"])
        self.assertTrue(any("method_version identity mismatch" in error for error in report["errors"]))

    def test_references_must_be_independent_and_equations_traceable(self) -> None:
        document = deepcopy(self.planned_document)
        for reference in document["scientific_contract"]["references"]:
            reference["independence_group"] = "same_oracle"
        document["scientific_contract"]["equations"][0]["source_reference_id"] = "missing"

        report = validate_manifest_document(
            document, REPOSITORY_ROOT, verify_evidence=False
        )

        self.assertFalse(report["passed"])
        self.assertIn(
            "references must contain at least two independence groups", report["errors"]
        )
        self.assertTrue(any("cites unknown reference" in error for error in report["errors"]))

    def test_boundary_and_persistence_tamper_coverage_is_fail_closed(self) -> None:
        document = deepcopy(self.planned_document)
        tamper_boundary = next(
            row
            for row in document["scientific_contract"]["boundaries"]
            if row["category"] == "tamper"
        )
        tamper_boundary["category"] = "determinism"
        legacy_tamper = next(
            row
            for row in document["product_contract"]["persistence"]["tamper_tests"]
            if row["category"] == "legacy_reinterpretation"
        )
        legacy_tamper["category"] = "checksum"

        report = validate_manifest_document(
            document, REPOSITORY_ROOT, verify_evidence=False
        )

        self.assertFalse(report["passed"])
        self.assertTrue(any("missing boundary categories: tamper" in error for error in report["errors"]))
        self.assertTrue(any("legacy_reinterpretation" in error for error in report["errors"]))

    def test_stochastic_method_requires_cancellation_and_retry(self) -> None:
        document = deepcopy(self.planned_document)
        packaged = document["product_contract"]["packaged"]
        packaged["cancellation_required"] = False
        packaged["workflow_steps"].remove("cancel_retry")

        report = validate_manifest_document(
            document, REPOSITORY_ROOT, verify_evidence=False
        )

        self.assertFalse(report["passed"])
        self.assertIn(
            "stochastic methods must require packaged cancellation", report["errors"]
        )
        self.assertIn(
            "stochastic methods must include the cancel_retry packaged step",
            report["errors"],
        )

    def test_absent_manifest_cannot_claim_release_without_evidence(self) -> None:
        document = deepcopy(self.planned_document)
        document["qualification"]["declared_state"] = "release_qualified"

        report = validate_manifest_document(
            document, REPOSITORY_ROOT, verify_evidence=False
        )

        self.assertFalse(report["passed"])
        self.assertEqual(report["derived_state"], "absent")
        self.assertTrue(
            any(
                "declared release_qualified but current evidence derives only absent"
                in error
                for error in report["errors"]
            )
        )

    def test_incomplete_stage_roles_are_rejected_without_double_counting(self) -> None:
        document = deepcopy(self.planned_document)
        document["qualification"]["evidence"]["engine_only"] = [
            {
                "path": "validation/results/one-report.json",
                "roles": ["method_spec", "method_spec"],
                "verification": {
                    "kind": "identity_report",
                    "identity_pointers": {
                        "passed": "/passed",
                        "feature_id": "/feature_id",
                        "method_version": "/method_version",
                        "catalogue_snapshot_date": "/catalogue_snapshot_date",
                    },
                    "source_artifacts_pointer": "/source_artifacts",
                    "generated_at_pointer": "/generated_at_utc",
                },
            }
        ]

        report = validate_manifest_document(
            document, REPOSITORY_ROOT, verify_evidence=False
        )

        self.assertFalse(report["passed"])
        self.assertTrue(any("array items must be unique" in error for error in report["errors"]))


if __name__ == "__main__":
    unittest.main()
