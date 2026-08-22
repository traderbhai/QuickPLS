from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

import v255_evidence_bundle_builder as bundle_builder  # noqa: E402
import v255_named_evidence_collector as collector  # noqa: E402
import v255_named_evidence_verifier as verifier  # noqa: E402
import v255_product_completion_audit as product_audit  # noqa: E402


def write_json(path: Path, payload: dict[str, Any]) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cross_fixture(root: Path) -> tuple[Path, dict[str, Any], list[dict[str, Any]]]:
    candidate = root / "QuickPLS.exe"
    candidate.write_bytes(b"candidate")
    candidate_hash = hashlib.sha256(candidate.read_bytes()).hexdigest()
    source_commit = "cd" * 20
    phase_payloads: dict[str, dict[str, Any]] = {}
    bindings: list[dict[str, Any]] = []
    seed_pid = 47007

    for index, phase in enumerate(collector.CROSS_PHASES, start=1):
        if phase == "unsaved_close_guard":
            payload = {
                "schema_version": 1,
                "suite_id": collector.CROSS_NATIVE_GUARD_SUITE,
                "passed": True,
                "candidate": {
                    "pid": seed_pid,
                    "path": str(candidate),
                    "sha256": candidate_hash.upper(),
                },
                "cancel_kept_exact_pid_alive": True,
                "failures": [],
            }
        else:
            pid = seed_pid if phase == "unsaved_close_seed" else 47000 + index
            payload = {
                "schema_version": 1,
                "suite_id": collector.CROSS_RENDERER_SUITE,
                "target_release": "2.55.0",
                "passed": True,
                "phase": phase,
                "source_commit": source_commit,
                "candidate": {
                    "pid": pid,
                    "path": str(candidate),
                    "sha256": candidate_hash.upper(),
                },
                "offline": {"passed": True},
                "console_errors": [],
                "failures": [],
            }
        phase_path = root / "phases" / phase / "report.json"
        phase_hash = write_json(phase_path, payload)
        phase_payloads[phase] = payload
        bindings.append(
            {"phase": phase, "path": str(phase_path), "sha256": phase_hash.upper()}
        )

    wrapper = {
        "schema_version": 1,
        "suite_id": collector.CROSS_WRAPPER_SUITE,
        "target_release": "2.55.0",
        "passed": True,
        "source_commit": source_commit,
        "candidate": {
            "role": "portable",
            "path": str(candidate),
            "sha256": candidate_hash.upper(),
            "product_version": "2.55.0",
        },
        "process_safety": {
            "exact_pid_tree_cleanup_only": True,
            "no_existing_candidate_attached": True,
            "sentinel_pid": 47999,
            "sentinel_survived_candidate_cleanup": True,
            "terminations": [
                {
                    "root_pid": phase_payloads[phase]["candidate"]["pid"],
                    "exact_tree_terminated": True,
                    "endpoint_closed": True,
                }
                for phase in collector.CROSS_RENDERER_PHASES
            ],
        },
        "phase_reports": bindings,
        "console_errors": [],
        "failures": [],
    }
    wrapper_path = root / "wrapper.json"
    write_json(wrapper_path, wrapper)
    records = [
        {
            "phase": phase,
            "member": f"named-evidence/source-reports/{phase}-{index}.json",
            "sha256": bindings[index - 1]["sha256"].lower(),
            "suite_id": (
                collector.CROSS_NATIVE_GUARD_SUITE
                if phase == "unsaved_close_guard"
                else collector.CROSS_RENDERER_SUITE
            ),
            "schema_version": 1,
            "renderer_attached": phase != "unsaved_close_guard",
            "payload": phase_payloads[phase],
        }
        for index, phase in enumerate(collector.CROSS_PHASES, start=1)
    ]
    return wrapper_path, wrapper, records


def cross_expectations(wrapper: dict[str, Any]) -> dict[str, object]:
    candidate = wrapper["candidate"]
    return {
        "expected_candidate_sha256": candidate["sha256"],
        "expected_candidate_path": candidate["path"],
        "expected_product_version": candidate["product_version"],
        "expected_source_commit": wrapper["source_commit"],
    }


def candidate_inventory_fixture() -> tuple[dict[str, Any], list[dict[str, str]]]:
    counter = 0

    def binding(label: str) -> tuple[str, str]:
        nonlocal counter
        counter += 1
        return f"evidence/{label}.json", f"{counter:064x}"

    installed_lifecycle = binding("installed-lifecycle")
    installed_method = binding("installed-method")
    installed_named = binding("installed-named")
    portable_lifecycle = binding("portable-lifecycle")
    portable_method = binding("portable-method")
    portable_frozen = binding("portable-frozen")
    portable_execute = binding("portable-posthoc-execute")
    portable_reopen = binding("portable-posthoc-reopen")
    portable_named = binding("portable-named")
    portable_cross = binding("portable-cross")
    candidate_report = {
        "outcomes": [
            {
                "name": "installed",
                "lifecycle": installed_lifecycle[0],
                "lifecycle_sha256": installed_lifecycle[1],
                "evidence": installed_method[0],
                "evidence_sha256": installed_method[1],
                "named_evidence_driver_reports": [
                    {"path": installed_named[0], "sha256": installed_named[1]}
                ],
            },
            {
                "name": "portable",
                "lifecycle": portable_lifecycle[0],
                "lifecycle_sha256": portable_lifecycle[1],
                "evidence": portable_method[0],
                "evidence_sha256": portable_method[1],
                "frozen_archive_collection": {
                    "aggregate_receipt": portable_frozen[0],
                    "aggregate_receipt_sha256": portable_frozen[1],
                },
                "posthoc_collection": {
                    "execute_receipt": portable_execute[0],
                    "execute_receipt_sha256": portable_execute[1],
                    "reopen_receipt": portable_reopen[0],
                    "reopen_receipt_sha256": portable_reopen[1],
                },
                "named_evidence_driver_reports": [
                    {"path": portable_named[0], "sha256": portable_named[1]},
                    {"path": portable_cross[0], "sha256": portable_cross[1]},
                ],
            },
        ]
    }
    inventory = verifier.candidate_driver_binding_inventory(candidate_report)
    assert inventory is not None
    rows = [
        {
            "candidate": candidate,
            "role": role,
            "sha256": digest,
            "suite_id": suite_id,
        }
        for candidate, role, digest, suite_id in inventory
    ]
    return candidate_report, rows


class V255ConsoleEvidenceTrustTests(unittest.TestCase):
    def test_bundled_trusted_report_inventory_is_closed_to_candidate_bindings(self) -> None:
        candidate_report, rows = candidate_inventory_fixture()
        validators = (
            verifier.exact_candidate_driver_inventory,
            bundle_builder.exact_candidate_driver_inventory,
        )
        for validate in validators:
            with self.subTest(validator=validate.__module__, mutation="exact"):
                self.assertTrue(validate(candidate_report, rows))
            with self.subTest(validator=validate.__module__, mutation="omitted"):
                self.assertFalse(validate(candidate_report, rows[:-1]))
            substituted = copy.deepcopy(rows)
            substituted[0]["sha256"] = "f" * 64
            with self.subTest(validator=validate.__module__, mutation="substituted"):
                self.assertFalse(validate(candidate_report, substituted))
            extra = copy.deepcopy(rows)
            extra.append(copy.deepcopy(rows[0]))
            with self.subTest(validator=validate.__module__, mutation="extra"):
                self.assertFalse(validate(candidate_report, extra))

            tandem_omission_candidate = copy.deepcopy(candidate_report)
            portable = tandem_omission_candidate["outcomes"][1]
            portable["frozen_archive_collection"] = None
            tandem_omission_rows = [
                row
                for row in rows
                if not (
                    row["candidate"] == "portable"
                    and row["role"] == "frozen_archive_reopen"
                )
            ]
            with self.subTest(
                validator=validate.__module__, mutation="tandem_omission"
            ):
                self.assertFalse(
                    validate(tandem_omission_candidate, tandem_omission_rows)
                )

            tandem_addition_candidate = copy.deepcopy(candidate_report)
            tandem_addition_candidate["outcomes"][0][
                "named_evidence_driver_reports"
            ].append({"path": "evidence/extra.json", "sha256": "e" * 64})
            tandem_addition_rows = copy.deepcopy(rows)
            tandem_addition_rows.append(
                {
                    "candidate": "installed",
                    "role": "named_evidence_driver_1",
                    "sha256": "e" * 64,
                    "suite_id": verifier.CROSS_WRAPPER_SUITE,
                }
            )
            with self.subTest(
                validator=validate.__module__, mutation="tandem_addition"
            ):
                self.assertFalse(
                    validate(tandem_addition_candidate, tandem_addition_rows)
                )

            suite_swap_candidate = copy.deepcopy(candidate_report)
            installed = suite_swap_candidate["outcomes"][0]
            installed["lifecycle"], installed["evidence"] = (
                installed["evidence"],
                installed["lifecycle"],
            )
            installed["lifecycle_sha256"], installed["evidence_sha256"] = (
                installed["evidence_sha256"],
                installed["lifecycle_sha256"],
            )
            suite_swap_rows = copy.deepcopy(rows)
            lifecycle_row = next(
                row
                for row in suite_swap_rows
                if row["candidate"] == "installed" and row["role"] == "lifecycle"
            )
            method_row = next(
                row
                for row in suite_swap_rows
                if row["candidate"] == "installed"
                and row["role"] == "method_evidence"
            )
            lifecycle_row["sha256"], method_row["sha256"] = (
                method_row["sha256"],
                lifecycle_row["sha256"],
            )
            lifecycle_row["suite_id"], method_row["suite_id"] = (
                method_row["suite_id"],
                lifecycle_row["suite_id"],
            )
            with self.subTest(validator=validate.__module__, mutation="suite_swap"):
                self.assertFalse(validate(suite_swap_candidate, suite_swap_rows))

    def test_collector_contract_requires_all_six_trusted_suites(self) -> None:
        index = json.loads(
            (VALIDATION / "v255_named_evidence_index.json").read_text(
                encoding="utf-8"
            )
        )
        contract = index["collector_contract"]
        self.assertEqual(
            collector.trusted_suite_versions(contract),
            collector.TRUSTED_DRIVER_SUITES,
        )
        incomplete = copy.deepcopy(contract)
        incomplete["trusted_driver_suites"] = incomplete[
            "trusted_driver_suites"
        ][:-1]
        with self.assertRaisesRegex(ValueError, "suite set is not exact"):
            collector.trusted_suite_versions(incomplete)

    def test_trusted_top_level_console_errors_is_fail_closed(self) -> None:
        validators = (
            collector.driver_report_passed,
            verifier.driver_report_passed,
            bundle_builder.driver_report_passed,
        )
        valid = {
            "target_release": "2.55.0",
            "passed": True,
            "console_errors": [],
        }
        invalid_values = (None, {}, ["renderer failure"])
        for validate in validators:
            with self.subTest(validator=validate.__module__, state="exact_empty"):
                self.assertTrue(validate(valid))
            missing = copy.deepcopy(valid)
            missing.pop("console_errors")
            with self.subTest(validator=validate.__module__, state="missing"):
                self.assertFalse(validate(missing))
            contradictory = {
                **valid,
                "passed": False,
                "status": "passed",
            }
            with self.subTest(validator=validate.__module__, state="contradictory"):
                self.assertFalse(validate(contradictory))
            for value in invalid_values:
                mutated = copy.deepcopy(valid)
                mutated["console_errors"] = value
                with self.subTest(validator=validate.__module__, state=repr(value)):
                    self.assertFalse(validate(mutated))

        self.assertTrue(product_audit.exact_empty_console_errors(valid))
        for value in (None, {}, ["renderer failure"]):
            mutated = copy.deepcopy(valid)
            mutated["console_errors"] = value
            self.assertFalse(product_audit.exact_empty_console_errors(mutated))
        missing = copy.deepcopy(valid)
        missing.pop("console_errors")
        self.assertFalse(product_audit.exact_empty_console_errors(missing))

    def test_schema_versions_reject_boolean_values(self) -> None:
        validators = (
            collector.exact_schema_version,
            verifier.exact_schema_version,
            bundle_builder.exact_schema_version,
            product_audit.exact_schema_version,
        )
        for validate in validators:
            with self.subTest(validator=validate.__module__, state="integer"):
                self.assertTrue(validate({"schema_version": 1}, 1))
            with self.subTest(validator=validate.__module__, state="boolean_payload"):
                self.assertFalse(validate({"schema_version": True}, 1))
            with self.subTest(validator=validate.__module__, state="boolean_expected"):
                self.assertFalse(validate({"schema_version": 1}, True))

    def test_exact_nine_cross_phases_accept_native_guard_without_console_field(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            wrapper_path, wrapper, records = cross_fixture(root)

            collected = collector.validate_cross_phase_reports(
                wrapper_path, wrapper, root, **cross_expectations(wrapper)
            )
            self.assertEqual(
                [record["phase"] for record in collected],
                list(collector.CROSS_PHASES),
            )
            self.assertNotIn(
                "console_errors",
                next(
                    record["payload"]
                    for record in collected
                    if record["phase"] == "unsaved_close_guard"
                ),
            )
            self.assertTrue(
                verifier.exact_cross_phase_payloads(
                    wrapper, records, **cross_expectations(wrapper)
                )
            )
            self.assertTrue(
                bundle_builder.exact_cross_phase_payloads(
                    wrapper, records, **cross_expectations(wrapper)
                )
            )
            self.assertTrue(
                product_audit.exact_cross_phase_report_bindings(
                    wrapper_path, wrapper
                )
            )

    def test_cross_phase_set_hash_and_renderer_console_are_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            wrapper_path, wrapper, records = cross_fixture(root)

            wrong_candidate = cross_expectations(wrapper)
            wrong_candidate["expected_candidate_sha256"] = "00" * 32
            with self.assertRaisesRegex(ValueError, "selected candidate"):
                collector.validate_cross_phase_reports(
                    wrapper_path, wrapper, root, **wrong_candidate
                )
            self.assertFalse(
                verifier.exact_cross_phase_payloads(
                    wrapper, records, **wrong_candidate
                )
            )
            self.assertFalse(
                bundle_builder.exact_cross_phase_payloads(
                    wrapper, records, **wrong_candidate
                )
            )

            bad_process_safety = copy.deepcopy(wrapper)
            bad_process_safety["process_safety"]["terminations"][0][
                "root_pid"
            ] += 1000
            with self.assertRaisesRegex(ValueError, "wrapper terminations"):
                collector.validate_cross_phase_reports(
                    wrapper_path,
                    bad_process_safety,
                    root,
                    **cross_expectations(wrapper),
                )
            self.assertFalse(
                verifier.exact_cross_phase_payloads(
                    bad_process_safety,
                    records,
                    **cross_expectations(wrapper),
                )
            )
            self.assertFalse(
                bundle_builder.exact_cross_phase_payloads(
                    bad_process_safety,
                    records,
                    **cross_expectations(wrapper),
                )
            )
            self.assertFalse(
                product_audit.exact_cross_phase_report_bindings(
                    wrapper_path, bad_process_safety
                )
            )
            extra_termination = copy.deepcopy(wrapper)
            extra_termination["process_safety"]["terminations"].append(
                "unverified cleanup"
            )
            with self.assertRaisesRegex(ValueError, "wrapper terminations"):
                collector.validate_cross_phase_reports(
                    wrapper_path,
                    extra_termination,
                    root,
                    **cross_expectations(wrapper),
                )
            self.assertFalse(
                verifier.exact_cross_phase_payloads(
                    extra_termination,
                    records,
                    **cross_expectations(wrapper),
                )
            )
            self.assertFalse(
                bundle_builder.exact_cross_phase_payloads(
                    extra_termination,
                    records,
                    **cross_expectations(wrapper),
                )
            )
            self.assertFalse(
                product_audit.exact_cross_phase_report_bindings(
                    wrapper_path, extra_termination
                )
            )

            missing_phase = copy.deepcopy(wrapper)
            missing_phase["phase_reports"] = missing_phase["phase_reports"][:-1]
            with self.assertRaisesRegex(ValueError, "exactly nine"):
                collector.validate_cross_phase_reports(
                    wrapper_path,
                    missing_phase,
                    root,
                    **cross_expectations(wrapper),
                )
            self.assertFalse(
                verifier.exact_cross_phase_payloads(
                    missing_phase,
                    records[:-1],
                    **cross_expectations(wrapper),
                )
            )
            self.assertFalse(
                bundle_builder.exact_cross_phase_payloads(
                    missing_phase,
                    records[:-1],
                    **cross_expectations(wrapper),
                )
            )
            self.assertFalse(
                product_audit.exact_cross_phase_report_bindings(
                    wrapper_path, missing_phase
                )
            )

            bad_hash = copy.deepcopy(wrapper)
            bad_hash["phase_reports"][0]["sha256"] = "00" * 32
            with self.assertRaisesRegex(ValueError, "SHA-256"):
                collector.validate_cross_phase_reports(
                    wrapper_path,
                    bad_hash,
                    root,
                    **cross_expectations(wrapper),
                )
            self.assertFalse(
                verifier.exact_cross_phase_payloads(
                    bad_hash, records, **cross_expectations(wrapper)
                )
            )
            self.assertFalse(
                bundle_builder.exact_cross_phase_payloads(
                    bad_hash, records, **cross_expectations(wrapper)
                )
            )
            self.assertFalse(
                product_audit.exact_cross_phase_report_bindings(
                    wrapper_path, bad_hash
                )
            )

            bad_pid_records = copy.deepcopy(records)
            next(
                record
                for record in bad_pid_records
                if record["phase"] == "imports"
            )["payload"]["candidate"]["pid"] = True
            self.assertFalse(
                verifier.exact_cross_phase_payloads(
                    wrapper, bad_pid_records, **cross_expectations(wrapper)
                )
            )
            self.assertFalse(
                bundle_builder.exact_cross_phase_payloads(
                    wrapper,
                    bad_pid_records,
                    **cross_expectations(wrapper),
                )
            )
            bad_guard_pid_records = copy.deepcopy(records)
            next(
                record
                for record in bad_guard_pid_records
                if record["phase"] == "unsaved_close_guard"
            )["payload"]["candidate"]["pid"] = True
            self.assertFalse(
                verifier.exact_cross_phase_payloads(
                    wrapper,
                    bad_guard_pid_records,
                    **cross_expectations(wrapper),
                )
            )
            self.assertFalse(
                bundle_builder.exact_cross_phase_payloads(
                    wrapper,
                    bad_guard_pid_records,
                    **cross_expectations(wrapper),
                )
            )

            imports_binding = next(
                binding
                for binding in wrapper["phase_reports"]
                if binding["phase"] == "imports"
            )
            imports_path = Path(imports_binding["path"])
            original_imports_payload = json.loads(
                imports_path.read_text(encoding="utf-8")
            )
            boolean_pid_payload = copy.deepcopy(original_imports_payload)
            boolean_pid_payload["candidate"]["pid"] = True
            imports_binding["sha256"] = write_json(
                imports_path, boolean_pid_payload
            ).upper()
            with self.assertRaisesRegex(ValueError, "wrapper candidate"):
                collector.validate_cross_phase_reports(
                    wrapper_path,
                    wrapper,
                    root,
                    **cross_expectations(wrapper),
                )
            self.assertFalse(
                product_audit.exact_cross_phase_report_bindings(
                    wrapper_path, wrapper
                )
            )
            imports_binding["sha256"] = write_json(
                imports_path, original_imports_payload
            ).upper()

            bad_records = copy.deepcopy(records)
            imports_record = next(
                record for record in bad_records if record["phase"] == "imports"
            )
            imports_record["payload"]["console_errors"] = None
            self.assertFalse(
                verifier.exact_cross_phase_payloads(
                    wrapper, bad_records, **cross_expectations(wrapper)
                )
            )
            self.assertFalse(
                bundle_builder.exact_cross_phase_payloads(
                    wrapper, bad_records, **cross_expectations(wrapper)
                )
            )

            imports_payload = json.loads(imports_path.read_text(encoding="utf-8"))
            imports_payload["console_errors"] = None
            imports_binding["sha256"] = write_json(
                imports_path, imports_payload
            ).upper()
            with self.assertRaisesRegex(ValueError, "zero console errors"):
                collector.validate_cross_phase_reports(
                    wrapper_path,
                    wrapper,
                    root,
                    **cross_expectations(wrapper),
                )
            self.assertFalse(
                product_audit.exact_cross_phase_report_bindings(
                    wrapper_path, wrapper
                )
            )


if __name__ == "__main__":
    unittest.main()
