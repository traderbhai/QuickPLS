#!/usr/bin/env python3
"""Focused fail-closed checks for the PLS model-fit v2 receipt factory."""

from __future__ import annotations

import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import jsonschema


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
sys.path.insert(0, str(VALIDATION))

import pls_model_fit_v2_qualification_factory as factory  # noqa: E402
from qualification_spec_v2 import (  # noqa: E402
    canonical_sha256,
    strict_load_json,
    validate_spec_document,
)


SCHEMA = VALIDATION / "pls_model_fit_v2_qualification_factory.schema.json"
SPEC = VALIDATION / "qualification_v2" / "pls_model_fit_exact_v1.qualification.json"
MANIFEST = VALIDATION / "methods" / "pls_model_fit_v2.manifest.json"
REGISTRY = VALIDATION / "capabilities" / "capability_registry_v2.json"


class PlsModelFitV2QualificationFactoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = strict_load_json(SCHEMA)
        cls.spec = strict_load_json(SPEC)
        cls.manifest = strict_load_json(MANIFEST)
        cls.registry = strict_load_json(REGISTRY)
        cls.audit = strict_load_json(factory.AUDIT_PATH)
        cls.contract = strict_load_json(factory.METHOD_CONTRACT_PATH)

    def test_checked_in_factory_artifacts_validate_against_schema(self) -> None:
        validator = jsonschema.Draft202012Validator(self.schema)
        validator.validate(self.audit)
        validator.validate(self.contract)

    def test_only_method_contract_candidate_is_emitted(self) -> None:
        descriptors = self.audit["candidate_receipt_descriptors"]
        self.assertEqual([row["role"] for row in descriptors], ["method_contract"])
        descriptor = descriptors[0]
        self.assertTrue(factory._descriptor_current(descriptor))
        self.assertEqual(descriptor["stage"], "contract")
        self.assertEqual(descriptor["evidence_class"], "qualification")
        self.assertTrue(self.contract["passed"])
        self.assertTrue(self.contract["receipt_eligible"])
        self.assertFalse(self.contract["qualification_ready"])
        self.assertFalse(self.contract["promotion_authority"])

    def test_candidate_descriptor_conforms_when_verified_but_does_not_qualify(self) -> None:
        candidate = copy.deepcopy(self.spec)
        candidate["evidence_contract"]["receipts"] = copy.deepcopy(
            self.audit["candidate_receipt_descriptors"]
        )
        result = validate_spec_document(
            candidate,
            repository_root=ROOT,
            verify_receipts=True,
            registry_document=self.registry,
            require_registry=True,
        )
        self.assertTrue(result["passed"], result)
        self.assertTrue(result["receipts_verified"], result)
        self.assertFalse(result["qualification_ready"], result)
        self.assertEqual(candidate["migration"]["status"], "compatibility_only")

    def test_role_matrix_is_exact_and_distinguishes_work_from_receipts(self) -> None:
        rows = {row["role"]: row for row in self.audit["role_matrix"]}
        self.assertEqual(tuple(rows), factory.EXPECTED_REQUIRED_ROLES)
        self.assertEqual(rows["method_contract"]["status"], "candidate_receipt_emitted")
        for role in (
            "oracle_independence",
            "generative_recovery",
            "adversarial_boundaries",
        ):
            self.assertEqual(rows[role]["status"], "work_evidence_only")
            self.assertTrue(rows[role]["work_evidence"]["passed_work_checks"])
            self.assertFalse(rows[role]["candidate_receipt_emitted"])
        for role in (
            "kernel_execution",
            "archive_persistence",
            "cross_format_export",
            "frontend_contract",
            "packaged_windows_e2e",
            "performance_scale",
        ):
            self.assertEqual(rows[role]["status"], "blocked")
            self.assertFalse(rows[role]["execution_envelope"]["exists"])
            self.assertFalse(rows[role]["candidate_receipt_emitted"])

    def test_source_presence_alone_never_becomes_execution_evidence(self) -> None:
        source_paths = {row["path"] for row in self.audit["source_artifacts"]}
        self.assertIn("crates/qpls-resampling/src/pls_model_fit_exact.rs", source_paths)
        self.assertIn("src/native/nativePlsModelFit.test.ts", source_paths)
        rows = {row["role"]: row for row in self.audit["role_matrix"]}
        self.assertFalse(rows["kernel_execution"]["candidate_receipt_emitted"])
        self.assertFalse(rows["frontend_contract"]["candidate_receipt_emitted"])
        self.assertIn(
            "immutable_execution_envelope_missing",
            rows["kernel_execution"]["execution_envelope"]["errors"],
        )

    def test_factory_is_source_and_scenario_hash_bound(self) -> None:
        descriptors = factory.source_descriptors()
        self.assertEqual(self.audit["source_artifacts"], descriptors)
        self.assertEqual(
            self.audit["source_set_sha256"],
            factory.source_set_sha256(descriptors),
        )
        self.assertEqual(
            self.audit["scenario_set_sha256"],
            canonical_sha256(self.spec["scenario_contract"]),
        )
        mutated = [dict(row) for row in descriptors]
        mutated[0]["sha256"] = "0" * 64
        self.assertNotEqual(
            factory.source_set_sha256(mutated),
            self.audit["source_set_sha256"],
        )
        self.assertTrue(factory.verify_checked_in_factory()["passed"])

    def test_registry_manifest_and_spec_remain_unpromoted(self) -> None:
        self.assertEqual(self.spec["migration"]["status"], "compatibility_only")
        self.assertEqual(self.spec["evidence_contract"]["receipts"], [])
        self.assertEqual(self.manifest["qualification"]["declared_state"], "absent")
        self.assertTrue(
            all(
                not rows
                for rows in self.manifest["qualification"]["evidence"].values()
            )
        )
        capability = next(
            row
            for row in self.registry["capabilities"]
            if row["capability_id"] == factory.CAPABILITY_ID
        )
        cell = next(
            row
            for row in capability["option_cells"]
            if row["cell_id"] == factory.CELL_ID
        )
        self.assertEqual(cell["coverage_state"], "partial")
        self.assertEqual(cell["evidence_state"], "absent")
        self.assertEqual(cell["surface"], "labs")
        self.assertFalse(self.audit["qualification_ready"])
        self.assertFalse(self.audit["promotion_allowed"])
        self.assertFalse(self.audit["scientific_review_satisfied"])
        self.assertFalse(self.audit["registry_mutated"])
        self.assertFalse(self.audit["manifest_mutated"])
        self.assertFalse(self.audit["qualification_spec_mutated"])

    def _execution_envelope(
        self,
        role: str,
        *,
        sources: list[dict[str, object]],
        source_sha: str,
        scenario_sha: str,
        build_fingerprint: str,
    ) -> dict[str, object]:
        artifact_path = VALIDATION / "fixtures" / "simple_reflective.csv"
        generated_at = "2026-08-14T18:40:00Z"
        return {
            "schema_version": 1,
            "report_kind": "pls_model_fit_v2_qualification_execution_v1",
            "role": role,
            "stage": factory.ROLE_STAGES[role],
            "passed": True,
            "qualification_evidence": True,
            "receipt_eligible": True,
            "qualification_id": factory.QUALIFICATION_ID,
            "capability_id": factory.CAPABILITY_ID,
            "cell_id": factory.CELL_ID,
            "method_version": factory.METHOD_VERSION,
            "generated_at_utc": generated_at,
            "source_set_sha256": source_sha,
            "scenario_set_sha256": scenario_sha,
            "build_fingerprint": build_fingerprint,
            "source_artifacts": sources,
            "hardware_fingerprint": factory.hardware_fingerprint(),
            "checks": [
                {"check_id": check_id, "passed": True}
                for check_id in factory.REQUIRED_CHECK_IDS[role]
            ],
            "commands": [
                {
                    "command": ["synthetic-test-only"],
                    "returncode": 0,
                    "started_at_utc": generated_at,
                    "finished_at_utc": generated_at,
                    "duration_seconds": 0.0,
                    "stdout_sha256": "0" * 64,
                    "stderr_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                }
            ],
            "artifacts": [
                {
                    "path": artifact_path.relative_to(ROOT).as_posix(),
                    "size_bytes": artifact_path.stat().st_size,
                    "sha256": factory.sha256_file(artifact_path),
                }
            ],
        }

    def test_execution_envelope_is_admitted_only_with_exact_current_proof(self) -> None:
        sources = factory.source_descriptors()
        source_sha = factory.source_set_sha256(sources)
        scenario_sha = canonical_sha256(self.spec["scenario_contract"])
        build_fingerprint = "product-build:test-fixture-only"
        identity = {
            "qualification_id": factory.QUALIFICATION_ID,
            "capability_id": factory.CAPABILITY_ID,
            "cell_id": factory.CELL_ID,
            "method_version": factory.METHOD_VERSION,
        }
        with tempfile.TemporaryDirectory(dir=VALIDATION) as temporary:
            input_root = Path(temporary)
            role = "kernel_execution"
            path = input_root / f"{role}.qualification.json"
            envelope = self._execution_envelope(
                role,
                sources=sources,
                source_sha=source_sha,
                scenario_sha=scenario_sha,
                build_fingerprint=build_fingerprint,
            )
            path.write_text(json.dumps(envelope), encoding="utf-8")
            with patch.object(factory, "INPUT_ROOT", input_root):
                accepted = factory.verify_execution_envelope(
                    role,
                    identity=identity,
                    generated_after=factory._parse_utc(
                        self.spec["identity"]["spec_frozen_at_utc"]
                    ),
                    sources=sources,
                    source_sha=source_sha,
                    scenario_sha=scenario_sha,
                    build_fingerprint=build_fingerprint,
                )
                self.assertTrue(accepted["eligible"], accepted)

                envelope["source_set_sha256"] = "0" * 64
                path.write_text(json.dumps(envelope), encoding="utf-8")
                rejected = factory.verify_execution_envelope(
                    role,
                    identity=identity,
                    generated_after=factory._parse_utc(
                        self.spec["identity"]["spec_frozen_at_utc"]
                    ),
                    sources=sources,
                    source_sha=source_sha,
                    scenario_sha=scenario_sha,
                    build_fingerprint=build_fingerprint,
                )
                self.assertFalse(rejected["eligible"])
                self.assertIn(
                    "execution_envelope_source_set_mismatch",
                    rejected["errors"],
                )

    def test_default_validation_fingerprint_cannot_admit_product_execution(self) -> None:
        sources = factory.source_descriptors()
        source_sha = factory.source_set_sha256(sources)
        scenario_sha = canonical_sha256(self.spec["scenario_contract"])
        build_fingerprint = f"validation-source-contract:{source_sha}"
        role = "kernel_execution"
        identity = {
            "qualification_id": factory.QUALIFICATION_ID,
            "capability_id": factory.CAPABILITY_ID,
            "cell_id": factory.CELL_ID,
            "method_version": factory.METHOD_VERSION,
        }
        with tempfile.TemporaryDirectory(dir=VALIDATION) as temporary:
            input_root = Path(temporary)
            path = input_root / f"{role}.qualification.json"
            path.write_text(
                json.dumps(
                    self._execution_envelope(
                        role,
                        sources=sources,
                        source_sha=source_sha,
                        scenario_sha=scenario_sha,
                        build_fingerprint=build_fingerprint,
                    )
                ),
                encoding="utf-8",
            )
            with patch.object(factory, "INPUT_ROOT", input_root):
                result = factory.verify_execution_envelope(
                    role,
                    identity=identity,
                    generated_after=factory._parse_utc(
                        self.spec["identity"]["spec_frozen_at_utc"]
                    ),
                    sources=sources,
                    source_sha=source_sha,
                    scenario_sha=scenario_sha,
                    build_fingerprint=build_fingerprint,
                )
        self.assertFalse(result["eligible"])
        self.assertIn(
            "execution_envelope_product_build_fingerprint_required",
            result["errors"],
        )


if __name__ == "__main__":
    unittest.main()
