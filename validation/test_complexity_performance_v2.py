from __future__ import annotations

import copy
import sys
import tempfile
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

from validation.capability_registry_v2 import load_json
from validation.complexity_performance_measure import measure_command_once, write_new_receipt
from validation.complexity_performance_v2 import (
    DEFAULT_MANIFEST_PATH,
    DEFAULT_MEASUREMENT_SCHEMA_PATH,
    DEFAULT_REGISTRY_PATH,
    DEFAULT_SCHEMA_PATH,
    _baseline_comparability_errors,
    _regression_errors,
    _validate_runs,
    aggregate_runs,
    type7_percentile,
    validate_contract_documents,
    validate_measurement_documents,
)


class ComplexityPerformanceContractV2Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = load_json(DEFAULT_MANIFEST_PATH)
        cls.registry = load_json(DEFAULT_REGISTRY_PATH)
        cls.schema = load_json(DEFAULT_SCHEMA_PATH)
        cls.measurement_schema = load_json(DEFAULT_MEASUREMENT_SCHEMA_PATH)

    def context(self, manifest=None, registry=None):
        return validate_contract_documents(
            manifest or self.manifest,
            registry or self.registry,
            self.schema,
            self.measurement_schema,
        )

    def test_contract_derives_every_active_cell_and_complete_budget_matrix(self) -> None:
        report = self.context()
        self.assertTrue(report["contract_valid"], report["errors"])
        self.assertEqual(report["active_row_count"], 43)
        self.assertEqual(report["active_option_cell_count"], 52)
        self.assertEqual(report["resolved_capability_count"], 52)
        self.assertEqual(report["resolved_budget_count"], 520)
        self.assertEqual(report["expected_capability_current_measurements"], 832)
        self.assertEqual(report["expected_ui_current_measurements"], 8)

    def test_machine_readable_schemas_are_valid_and_manifest_conforms(self) -> None:
        Draft202012Validator.check_schema(self.schema)
        Draft202012Validator.check_schema(self.measurement_schema)
        errors = sorted(
            Draft202012Validator(
                self.schema, format_checker=FormatChecker()
            ).iter_errors(self.manifest),
            key=lambda error: list(error.path),
        )
        self.assertEqual(errors, [])

    def test_registry_identity_and_reference_set_are_fail_closed(self) -> None:
        changed = copy.deepcopy(self.registry)
        changed["capabilities"][0]["option_cells"][0]["capability_version"] = "changed"
        report = self.context(registry=changed)
        self.assertFalse(report["contract_valid"])
        self.assertTrue(
            any("registry binding" in error.lower() for error in report["errors"]),
            report["errors"],
        )

    def test_missing_budget_and_relaxed_regression_rule_fail(self) -> None:
        missing = copy.deepcopy(self.manifest)
        missing["budget_classes"][0]["budgets"].pop()
        report = self.context(manifest=missing)
        self.assertFalse(report["contract_valid"])
        self.assertTrue(any("matrix differs" in error for error in report["errors"]))

        relaxed = copy.deepcopy(self.manifest)
        relaxed["measurement_policy"]["maximum_runtime_regression_percent"] = 20.01
        report = self.context(manifest=relaxed)
        self.assertFalse(report["contract_valid"])
        self.assertTrue(any("20 percent" in error for error in report["errors"]))

    def test_type7_and_five_run_aggregates_are_deterministic(self) -> None:
        self.assertAlmostEqual(type7_percentile([1, 2, 3, 4, 5], 0.95), 4.8)
        runs = [
            {
                "elapsed_seconds": value,
                "peak_working_set_bytes": value * 100,
                "result_bytes": value * 10,
            }
            for value in [1, 2, 3, 4, 5]
        ]
        self.assertEqual(
            aggregate_runs(runs),
            {
                "median_elapsed_seconds": 3,
                "p95_elapsed_seconds": 4.8,
                "median_working_set_bytes": 300,
                "p95_working_set_bytes": 480,
                "p95_result_bytes": 48,
            },
        )

    def test_empty_measurement_set_never_passes_product_finalization(self) -> None:
        context = self.context()
        report = validate_measurement_documents(
            self.manifest, self.registry, context, []
        )
        self.assertFalse(report["measurement_qualification_passed"])
        self.assertEqual(report["missing_capability_measurement_count"], 832)
        self.assertEqual(report["missing_ui_measurement_count"], 8)

    def test_run_gate_checks_absolute_budget_progress_cancel_and_memory(self) -> None:
        measured = []
        for index, elapsed in enumerate([2.1, 2.2, 2.3, 2.4, 2.5]):
            measured.append(
                {
                    "phase": "measured",
                    "index": index,
                    "exit_code": 0,
                    "elapsed_seconds": elapsed,
                    "peak_working_set_bytes": 1000 + index,
                    "result_bytes": 100 + index,
                    "progress_values": [0.25, 0.75],
                    "orphan_processes": 0,
                }
            )
        receipt = {
            "applicability": "measured",
            "not_applicable_reason": None,
            "predicate_references": [],
            "measurement_role": "accepted_baseline",
            "command": {
                "argv": ["quickpls.exe"],
                "working_directory": "D:\\QuickPLS",
                "build_fingerprint": "a" * 64,
                "workload_fingerprint": "b" * 64,
                "process_tree_measured": True,
            },
            "warmup_runs": [
                {
                    **measured[0],
                    "phase": "warmup",
                    "index": 0,
                }
            ],
            "measured_runs": measured,
            "aggregates": aggregate_runs(measured),
            "progress_observation": {
                "operation_exceeded_threshold": True,
                "real_progress_shown": True,
                "distinct_progress_values": 2,
                "monotonic": True,
            },
            "cancellation_observation": {
                "terminal_latency_seconds": 0.5,
                "terminal_state": "cancelled",
                "no_partial_visible_result": True,
                "no_partial_committed_result": True,
                "archive_unchanged": True,
            },
            "memory_growth_observation": {
                "accepted_runs": 10,
                "first_settled_working_set_bytes": 1000,
                "last_settled_working_set_bytes": 1040,
                "growth_percent": 4,
                "orphan_processes": 0,
            },
        }
        context = self.context()
        profile = context["profiles"]["applied"]
        budget = context["budget_index"]["balanced"][
            ("standard_windows_6c16g", "applied")
        ]
        hardware = context["hardware"]["standard_windows_6c16g"]
        self.assertEqual(
            _validate_runs(
                receipt, profile, budget, self.manifest, hardware=hardware
            ),
            [],
        )
        receipt["cancellation_observation"]["terminal_latency_seconds"] = 1.01
        receipt["memory_growth_observation"]["growth_percent"] = 5.01
        receipt["measured_runs"][0]["progress_values"] = []
        errors = _validate_runs(
            receipt, profile, budget, self.manifest, hardware=hardware
        )
        self.assertTrue(any("cancellation" in error for error in errors))
        self.assertTrue(any("memory" in error for error in errors))
        self.assertTrue(any("progress" in error for error in errors))

        receipt["measured_runs"][0]["peak_working_set_bytes"] = (
            hardware["maximum_qualified_working_set_bytes"] + 1
        )
        receipt["aggregates"] = aggregate_runs(receipt["measured_runs"])
        errors = _validate_runs(
            receipt, profile, budget, self.manifest, hardware=hardware
        )
        self.assertTrue(any("working-set ceiling" in error for error in errors))

    def test_more_than_twenty_percent_regression_fails(self) -> None:
        baseline = {
            "aggregates": {
                "median_elapsed_seconds": 10,
                "p95_elapsed_seconds": 12,
                "median_working_set_bytes": 1000,
                "p95_working_set_bytes": 1200,
            }
        }
        current = copy.deepcopy(baseline)
        current["aggregates"]["median_elapsed_seconds"] = 12.0001
        errors = _regression_errors(
            current, baseline, self.manifest["measurement_policy"]
        )
        self.assertTrue(any("more than 20 percent" in error for error in errors))

    def test_regression_requires_identical_hardware_and_workload(self) -> None:
        baseline = {
            "document_kind": "capability_performance_measurement",
            "hardware_fingerprint": {"cpu": "reference"},
            "command": {"workload_fingerprint": "a" * 64},
        }
        current = copy.deepcopy(baseline)
        self.assertEqual(_baseline_comparability_errors(current, baseline), [])
        current["hardware_fingerprint"]["cpu"] = "different"
        current["command"]["workload_fingerprint"] = "b" * 64
        errors = _baseline_comparability_errors(current, baseline)
        self.assertTrue(any("hardware" in error for error in errors))
        self.assertTrue(any("workload" in error for error in errors))

    def test_lightweight_harness_measures_process_tree_and_never_overwrites_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as directory_value:
            directory = Path(directory_value)
            result = directory / "result.json"
            script = (
                "import json,os,pathlib,time; "
                "time.sleep(0.08); "
                "pathlib.Path(os.environ['QPLS_PERFORMANCE_RESULT_PATH']).write_text(json.dumps({'ok':True}))"
            )
            run = measure_command_once(
                [sys.executable, "-c", script],
                cwd=directory,
                result_path=result,
                phase="warmup",
                index=0,
                remove_prior_result=False,
            )
            self.assertEqual(run["exit_code"], 0)
            self.assertGreater(run["elapsed_seconds"], 0)
            self.assertGreater(run["peak_working_set_bytes"], 0)
            self.assertGreater(run["result_bytes"], 0)

            receipt_path = directory / "receipt.json"
            write_new_receipt(receipt_path, {"schema_version": 2})
            with self.assertRaises(FileExistsError):
                write_new_receipt(receipt_path, {"schema_version": 2})


if __name__ == "__main__":
    unittest.main()
