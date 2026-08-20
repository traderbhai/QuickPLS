from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path
from unittest.mock import patch

from jsonschema import Draft202012Validator

from validation.general_sem_rank0_receipt_payload_v1 import (
    CONTRACT_ID,
    BOOTSTRAP_CELL_IDS,
    PAYLOAD_KIND,
    ROLE_STAGE,
    SCHEMA_PATH,
    SCIENTIFIC_SUITES,
    _validate_product_observation,
    canonical_sha256,
    method_manifest_contract_sha256,
    qualification_contract_sha256,
    strict_load_json,
    unified_rank0_source_receipt,
    validate_payload_document,
    validate_payload_path,
)
from validation.qualification_spec_v2 import validate_spec_document


ROOT = Path(__file__).resolve().parents[1]
SPEC_PATH = (
    ROOT
    / "validation"
    / "qualification_v2"
    / "general_sem_pls_multiple_moderation_bootstrap_v1.qualification.json"
)


class Rank0ReceiptPayloadTests(unittest.TestCase):
    def setUp(self) -> None:
        self.spec = strict_load_json(SPEC_PATH)

    @staticmethod
    def _write_json(root: Path, relative: str, value: object) -> dict[str, object]:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        data = (json.dumps(value, sort_keys=True) + "\n").encode("utf-8")
        path.write_bytes(data)
        return {
            "path": relative,
            "size": len(data),
            "sha256": hashlib.sha256(data).hexdigest(),
        }

    def _parts(
        self,
        root: Path,
        *,
        role: str,
        evidence: dict[str, object],
    ) -> tuple[dict[str, object], dict[str, object]]:
        source_path = root / "sources" / "frozen.txt"
        source_path.parent.mkdir(parents=True, exist_ok=True)
        source_path.write_text("frozen source\n", encoding="utf-8")
        source_data = source_path.read_bytes()
        sources = [
            {
                "path": "sources/frozen.txt",
                "size": len(source_data),
                "sha256": hashlib.sha256(source_data).hexdigest(),
            }
        ]
        identity = self.spec["identity"]
        analytical = identity["analytical_method_version"]
        if evidence.get("kind") == "general_sem_rank0_method_audit_v1":
            evidence = {
                **evidence,
                "manifest_contract_sha256": "d" * 64,
            }
        generated = "2026-08-19T00:05:00Z"
        hardware = {
            "os": "windows_11",
            "architecture": "x86_64",
            "cpu": "test-cpu",
            "logical_cores": 8,
            "memory_gib": 16.0,
        }
        payload = {
            "schema_version": 1,
            "kind": PAYLOAD_KIND,
            "contract_id": CONTRACT_ID,
            "passed": True,
            "role": role,
            "stage": ROLE_STAGE[role],
            "qualification_id": identity["qualification_id"],
            "capability_cell": deepcopy(identity["capability_cell"]),
            "method_version": identity["method_version"],
            "analytical_method_version": analytical,
            "generated_at_utc": generated,
            "source_descriptors": sources,
            "source_set_sha256": canonical_sha256(sources),
            "scenario_set_sha256": canonical_sha256(self.spec["scenario_contract"]),
            "qualification_contract_sha256": qualification_contract_sha256(self.spec),
            "build_fingerprint": "a" * 64,
            "hardware_fingerprint": hardware,
            "evidence": evidence,
        }
        receipt = {
            "role": role,
            "stage": ROLE_STAGE[role],
            "qualification_id": identity["qualification_id"],
            "capability_id": identity["capability_cell"]["capability_id"],
            "cell_id": identity["capability_cell"]["cell_id"],
            "method_version": identity["method_version"],
            "analytical_method_version": analytical,
            "generated_at_utc": generated,
            "source_set_sha256": payload["source_set_sha256"],
            "scenario_set_sha256": payload["scenario_set_sha256"],
            "qualification_contract_sha256": payload["qualification_contract_sha256"],
            "build_fingerprint": payload["build_fingerprint"],
            "hardware_fingerprint": hardware,
        }
        return payload, receipt

    def _errors(
        self,
        root: Path,
        payload: dict[str, object],
        receipt: dict[str, object],
    ) -> list[str]:
        source_path = root / "sources" / "frozen.txt"
        source_data = source_path.read_bytes()
        descriptors = [
            {
                "path": "sources/frozen.txt",
                "size": len(source_data),
                "sha256": hashlib.sha256(source_data).hexdigest(),
            }
        ]
        expected_source_receipt = {
            "scope": "quickpls_general_sem_rank0_unified_sources_v2",
            "file_count": 1,
            "files": descriptors,
            "source_set_sha256": canonical_sha256(descriptors),
        }
        with patch(
            "validation.general_sem_rank0_receipt_payload_v1.unified_rank0_source_receipt",
            return_value=expected_source_receipt,
        ):
            return validate_payload_document(
                payload,
                receipt=receipt,
                specification=self.spec,
                repository_root=root,
            )

    def test_payload_schema_is_valid_draft_2020_12(self) -> None:
        schema = strict_load_json(SCHEMA_PATH)
        Draft202012Validator.check_schema(schema)
        scientific = schema["$defs"]["scientific_product"]
        self.assertNotIn("shards", scientific["required"])
        self.assertNotIn("shards", scientific["properties"])
        self.assertNotIn("continuation_policy", scientific["required"])
        self.assertIn("continuation_policy", scientific["properties"])

    def test_unified_source_inventory_is_exact_sorted_and_excludes_mutable_results(
        self,
    ) -> None:
        receipt = unified_rank0_source_receipt(ROOT)
        paths = [str(row["path"]) for row in receipt["files"]]
        self.assertEqual(paths, sorted(paths, key=lambda value: value.encode("utf-8")))
        self.assertEqual(len(paths), len({value.casefold() for value in paths}))
        self.assertEqual(receipt["file_count"], len(paths))
        self.assertEqual(
            receipt["source_set_sha256"], canonical_sha256(receipt["files"])
        )
        self.assertIn("validation/general_sem_rank0_receipt_payload_v1.py", paths)
        self.assertIn(
            "validation/qualification_v2/general_sem_rank0_receipt_payload_v1.schema.json",
            paths,
        )
        self.assertIn("docs/methods/PLS_MEDIATION_V1.md", paths)
        self.assertIn("validation/mediation_reference.py", paths)
        self.assertFalse(any("/results/" in path for path in paths))
        self.assertFalse(any(path.endswith(".qualification.json") for path in paths))
        self.assertFalse(
            any(
                path.startswith("validation/methods/")
                and path.endswith(".manifest.json")
                for path in paths
            )
        )

    def test_qualification_contract_excludes_migration_and_receipts_only(self) -> None:
        original = qualification_contract_sha256(self.spec)
        mutable = deepcopy(self.spec)
        mutable["migration"]["status"] = "native"
        mutable["migration"]["unresolved_items"] = []
        mutable["evidence_contract"]["receipts"].append({"mutable": True})
        self.assertEqual(qualification_contract_sha256(mutable), original)

        changed = deepcopy(self.spec)
        changed["operational_contract"]["cancellation"]["maximum_latency_seconds"] += (
            0.1
        )
        self.assertNotEqual(qualification_contract_sha256(changed), original)

    def test_method_contract_excludes_only_evidence_populated_state(self) -> None:
        manifest_path = (
            ROOT
            / "validation"
            / "methods"
            / "general_sem_pls_multiple_moderation_bootstrap_v1.manifest.json"
        )
        manifest = strict_load_json(manifest_path)
        original = method_manifest_contract_sha256(manifest)
        mutable = deepcopy(manifest)
        mutable["qualification"]["declared_state"] = "release_qualified"
        mutable["qualification"]["evidence"]["release_qualified"] = [{"mutable": True}]
        self.assertEqual(method_manifest_contract_sha256(mutable), original)

        changed = deepcopy(manifest)
        changed["qualification"]["source_requirements"]["method_spec"].append(
            "docs/methods/CHANGED.md"
        )
        self.assertNotEqual(method_manifest_contract_sha256(changed), original)

    def test_strict_rank0_contract_requires_all_ten_roles(self) -> None:
        candidate = deepcopy(self.spec)
        candidate["evidence_contract"]["required_roles"].remove("cross_format_export")
        report = validate_spec_document(candidate)
        self.assertFalse(report["passed"], report)
        self.assertTrue(
            any("exact ten-role contract" in error for error in report["errors"]),
            report,
        )

    def test_scientific_roles_exactly_cover_each_point_and_bootstrap_plan(self) -> None:
        validation_path = str(ROOT / "validation")
        if validation_path not in sys.path:
            sys.path.insert(0, validation_path)
        import general_sem_rank0_qualification_runner as runner

        plan = runner.build_plan()
        suites_by_cell: dict[str, set[str]] = {}
        for scenario in plan["scenarios"]:
            suites_by_cell.setdefault(str(scenario["cell_id"]), set()).add(
                str(scenario["suite"])
            )
        delegated = {"maximum_axis", "compound_stress"}
        for cell_id, actual in suites_by_cell.items():
            kind = "bootstrap" if cell_id in BOOTSTRAP_CELL_IDS else "point"
            covered = set().union(
                *(role_suites[kind] for role_suites in SCIENTIFIC_SUITES.values())
            )
            self.assertEqual(covered, actual - delegated, cell_id)

    def test_product_producer_sha_is_distinct_from_final_app_build_sha(self) -> None:
        source_receipt = unified_rank0_source_receipt(ROOT)
        identity = self.spec["identity"]
        producer_sha256 = "b" * 64
        app_sha256 = "a" * 64
        executable = {
            "path": "target/release/examples/general_sem_rank0_product_comparison.exe",
            "size": 1,
            "sha256": producer_sha256,
        }
        payload = {
            "capability_cell": identity["capability_cell"],
            "analytical_method_version": identity["analytical_method_version"],
            "source_descriptors": source_receipt["files"],
            "source_set_sha256": source_receipt["source_set_sha256"],
            "build_fingerprint": app_sha256,
        }
        observation = {
            "cell_id": identity["capability_cell"]["cell_id"],
            "method_version": identity["capability_cell"]["capability_version"],
            "difference_count": 0,
            "difference_witnesses": [],
            "product_source_set_sha256": source_receipt["source_set_sha256"],
            "producer_executable_sha256": producer_sha256,
            "executable_descriptor": executable,
            "execution_receipt": {
                "source_receipt": source_receipt,
                "producer_executable_sha256": producer_sha256,
                "executable_descriptor": executable,
            },
            "worker_comparisons": [
                {
                    "production_receipts": {
                        "bootstrap": {
                            "supplemental_method_version": identity[
                                "analytical_method_version"
                            ]
                        }
                    }
                }
            ],
        }
        _validate_product_observation(
            observation, payload, source_receipt["source_set_sha256"], ROOT
        )

        equated = deepcopy(observation)
        equated["producer_executable_sha256"] = app_sha256
        equated["executable_descriptor"]["sha256"] = app_sha256
        equated["execution_receipt"]["producer_executable_sha256"] = app_sha256
        with self.assertRaisesRegex(ValueError, "producer and app identities distinct"):
            _validate_product_observation(
                equated, payload, source_receipt["source_set_sha256"], ROOT
            )

        swapped = deepcopy(observation)
        swapped["producer_executable_sha256"] = "c" * 64
        with self.assertRaisesRegex(ValueError, "producer and app identities distinct"):
            _validate_product_observation(
                swapped, payload, source_receipt["source_set_sha256"], ROOT
            )

    def test_arbitrary_json_is_not_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "arbitrary.json"
            path.write_text('{"passed": true}\n', encoding="utf-8")
            _, receipt = self._parts(
                root,
                role="method_contract",
                evidence={"kind": "general_sem_rank0_method_audit_v1", "manifest": {}},
            )
            errors = validate_payload_path(
                path,
                receipt=receipt,
                specification=self.spec,
                repository_root=root,
            )
        self.assertTrue(any("payload schema" in error for error in errors), errors)

    def test_qualification_spec_strict_verifier_dispatches_payload_validation(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "arbitrary.json"
            data = b'{"passed": true}\n'
            path.write_bytes(data)
            candidate = deepcopy(self.spec)
            identity = candidate["identity"]
            candidate["evidence_contract"]["receipts"] = [
                {
                    "role": "method_contract",
                    "stage": "contract",
                    "evidence_class": "compatibility_fixture",
                    "qualification_id": identity["qualification_id"],
                    "capability_id": identity["capability_cell"]["capability_id"],
                    "cell_id": identity["capability_cell"]["cell_id"],
                    "method_version": identity["method_version"],
                    "analytical_method_version": identity["analytical_method_version"],
                    "path": "arbitrary.json",
                    "size_bytes": len(data),
                    "sha256": hashlib.sha256(data).hexdigest(),
                    "generated_at_utc": "2026-08-19T00:05:00Z",
                    "source_set_sha256": "a" * 64,
                    "scenario_set_sha256": canonical_sha256(
                        candidate["scenario_contract"]
                    ),
                    "build_fingerprint": "b" * 64,
                    "hardware_fingerprint": {
                        "os": "windows_11",
                        "architecture": "x86_64",
                        "cpu": "test-cpu",
                        "logical_cores": 8,
                        "memory_gib": 16.0,
                    },
                }
            ]
            report = validate_spec_document(
                candidate,
                repository_root=root,
                verify_receipts=True,
            )
        self.assertFalse(report["passed"], report)
        self.assertFalse(report["receipts_verified"], report)
        self.assertTrue(
            any("payload schema" in error for error in report["errors"]), report
        )

    def test_passed_false_is_rejected_by_schema(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = self._write_json(root, "evidence/manifest.json", {})
            payload, receipt = self._parts(
                root,
                role="method_contract",
                evidence={
                    "kind": "general_sem_rank0_method_audit_v1",
                    "manifest": manifest,
                },
            )
            payload["passed"] = False
            errors = self._errors(root, payload, receipt)
        self.assertTrue(any("payload schema" in error for error in errors), errors)

    def test_swapped_role_stage_cell_build_and_analytical_method_fail(self) -> None:
        mutations = (
            ("role", "kernel_execution"),
            ("stage", "oracle"),
            ("build_fingerprint", "b" * 64),
            ("qualification_contract_sha256", "b" * 64),
            (
                "analytical_method_version",
                "general_sem_pls_full_model_case_bootstrap_v1",
            ),
        )
        for field, value in mutations:
            with self.subTest(field=field), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                manifest = self._write_json(root, "evidence/manifest.json", {})
                payload, receipt = self._parts(
                    root,
                    role="method_contract",
                    evidence={
                        "kind": "general_sem_rank0_method_audit_v1",
                        "manifest": manifest,
                    },
                )
                payload[field] = value
                errors = self._errors(root, payload, receipt)
                self.assertTrue(errors)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = self._write_json(root, "evidence/manifest.json", {})
            payload, receipt = self._parts(
                root,
                role="method_contract",
                evidence={
                    "kind": "general_sem_rank0_method_audit_v1",
                    "manifest": manifest,
                },
            )
            payload["capability_cell"]["cell_id"] = "swapped.cell"
            self.assertTrue(self._errors(root, payload, receipt))
            receipt["analytical_method_version"] = "swapped.method"
            self.assertTrue(self._errors(root, payload, receipt))

    def test_changed_source_bytes_and_source_set_digest_fail(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = self._write_json(root, "evidence/manifest.json", {})
            payload, receipt = self._parts(
                root,
                role="method_contract",
                evidence={
                    "kind": "general_sem_rank0_method_audit_v1",
                    "manifest": manifest,
                },
            )
            (root / "sources" / "frozen.txt").write_bytes(b"changed\n")
            errors = self._errors(root, payload, receipt)
            self.assertTrue(
                any("source_descriptors" in error for error in errors), errors
            )
            payload["source_descriptors"][0]["size"] = len(b"changed\n")
            payload["source_descriptors"][0]["sha256"] = hashlib.sha256(
                b"changed\n"
            ).hexdigest()
            payload["source_set_sha256"] = "c" * 64
            receipt["source_set_sha256"] = "c" * 64
            errors = self._errors(root, payload, receipt)
            self.assertTrue(
                any("source_set_sha256" in error for error in errors), errors
            )

    def test_caller_chosen_source_subset_or_extra_file_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = self._write_json(root, "evidence/manifest.json", {})
            payload, receipt = self._parts(
                root,
                role="method_contract",
                evidence={
                    "kind": "general_sem_rank0_method_audit_v1",
                    "manifest": manifest,
                },
            )
            extra_path = root / "sources" / "extra.txt"
            extra_path.write_text("not authoritative\n", encoding="utf-8")
            extra_data = extra_path.read_bytes()
            payload["source_descriptors"].append(
                {
                    "path": "sources/extra.txt",
                    "size": len(extra_data),
                    "sha256": hashlib.sha256(extra_data).hexdigest(),
                }
            )
            payload["source_descriptors"].sort(
                key=lambda row: str(row["path"]).encode("utf-8")
            )
            payload["source_set_sha256"] = canonical_sha256(
                payload["source_descriptors"]
            )
            receipt["source_set_sha256"] = payload["source_set_sha256"]
            errors = self._errors(root, payload, receipt)
        self.assertTrue(any("exact unified" in error for error in errors), errors)

    def test_fabricated_packaged_report_fails_deep_validation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            report = self._write_json(root, "evidence/package.json", {"passed": True})
            payload, receipt = self._parts(
                root,
                role="packaged_windows_e2e",
                evidence={
                    "kind": "general_sem_rank0_packaged_windows_evidence_v1",
                    "packaged_report": report,
                },
            )
            errors = self._errors(root, payload, receipt)
        self.assertTrue(errors)

    def test_fabricated_performance_index_fails_deep_validation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            index = self._write_json(
                root, "evidence/performance.json", {"passed": True}
            )
            payload, receipt = self._parts(
                root,
                role="performance_scale",
                evidence={
                    "kind": "general_sem_rank0_performance_evidence_v1",
                    "performance_index": index,
                },
            )
            errors = self._errors(root, payload, receipt)
        self.assertTrue(any("performance index" in error for error in errors), errors)


if __name__ == "__main__":
    unittest.main()
