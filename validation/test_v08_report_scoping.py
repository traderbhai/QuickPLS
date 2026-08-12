#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
import tempfile
import unittest
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

import v08_extended_methods_reference as reference


def passing_check(method: str) -> dict:
    versions = {
        "pca": "pca_v1",
        "ols": "regression_ols_v1",
        "logistic": "regression_logistic_v1",
        "process": "regression_process_v1",
        "nca": "nca_v2",
        "gsca": "gsca_v1",
    }
    return {"passed": True, "method_version": versions[method]}


class ScopedReferenceReportTests(unittest.TestCase):
    def test_single_section_writes_only_its_method_report(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            results = Path(temp)
            aggregate = results / reference.AGGREGATE_OUTPUT_NAME
            process = results / reference.METHOD_OUTPUT_NAMES["process"]
            aggregate.write_text("aggregate-sentinel", encoding="utf-8")
            process.write_text("process-sentinel", encoding="utf-8")

            written = reference.write_scoped_reports(
                "logistic",
                {"logistic": passing_check("logistic")},
                results,
            )

            self.assertEqual(set(written), {"logistic"})
            self.assertEqual(aggregate.read_text(encoding="utf-8"), "aggregate-sentinel")
            self.assertEqual(process.read_text(encoding="utf-8"), "process-sentinel")
            report = json.loads(written["logistic"].read_text(encoding="utf-8"))
            self.assertTrue(report["passed"])
            self.assertEqual(report["schema_version"], 2)
            self.assertEqual(report["report_scope"], "method_specific")
            self.assertEqual(report["selected_section"], "logistic")
            self.assertEqual(set(report["checks"]), {"logistic"})

    def test_peer_section_cannot_overwrite_existing_method_report(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            results = Path(temp)
            logistic = results / reference.METHOD_OUTPUT_NAMES["logistic"]
            reference.write_scoped_reports(
                "logistic",
                {"logistic": passing_check("logistic")},
                results,
            )
            logistic_before = logistic.read_bytes()

            reference.write_scoped_reports(
                "process",
                {"process": passing_check("process")},
                results,
            )

            self.assertEqual(logistic.read_bytes(), logistic_before)
            process = json.loads(
                (results / reference.METHOD_OUTPUT_NAMES["process"]).read_text(encoding="utf-8")
            )
            self.assertEqual(process["selected_section"], "process")
            self.assertEqual(set(process["checks"]), {"process"})

    def test_all_refreshes_aggregate_and_every_method_report(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            results = Path(temp)
            checks = {method: passing_check(method) for method in reference.CHECKS}

            written = reference.write_scoped_reports("all", checks, results)

            self.assertEqual(set(written), {"all", *reference.CHECKS})
            aggregate = json.loads(written["all"].read_text(encoding="utf-8"))
            self.assertEqual(aggregate["report_scope"], "aggregate")
            self.assertEqual(aggregate["selected_section"], "all")
            self.assertEqual(set(aggregate["checks"]), set(reference.CHECKS))
            for method in reference.CHECKS:
                method_report = json.loads(written[method].read_text(encoding="utf-8"))
                self.assertEqual(method_report["report_scope"], "method_specific")
                self.assertEqual(method_report["selected_section"], method)
                self.assertEqual(set(method_report["checks"]), {method})

    def test_mismatched_method_report_fails_before_write(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            results = Path(temp)
            with self.assertRaises(ValueError):
                reference.write_scoped_reports(
                    "logistic",
                    {"process": passing_check("process")},
                    results,
                )
            self.assertEqual(list(results.iterdir()), [])


class PromotionBindingTests(unittest.TestCase):
    def test_logistic_and_process_gates_bind_exact_method_reports(self) -> None:
        logistic = (VALIDATION / "logistic_method_promotion_audit.py").read_text(encoding="utf-8")
        process = (VALIDATION / "process_method_promotion_audit.py").read_text(encoding="utf-8")
        self.assertIn('"name": "v08_logistic_reference_report.json"', logistic)
        self.assertIn('"selected_section": "logistic"', logistic)
        self.assertIn('"report_scope": "method_specific"', logistic)
        self.assertNotIn("v08_extended_methods_reference_report.json", logistic)
        self.assertIn('"name": "v08_process_reference_report.json"', process)
        self.assertIn('"selected_section": "process"', process)
        self.assertIn('"report_scope": "method_specific"', process)
        self.assertNotIn("v08_extended_methods_reference_report.json", process)


if __name__ == "__main__":
    unittest.main()
