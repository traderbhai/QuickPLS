#!/usr/bin/env python3
"""Fail-closed tests for the regression bootstrap v1 promotion audit."""

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
sys.path.insert(0, str(VALIDATION))

import regression_bootstrap_method_promotion_audit as audit  # noqa: E402


NOW = datetime(2026, 8, 12, 12, 0, tzinfo=timezone.utc)


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


def write_artifact(path: Path, contents: bytes, root: Path) -> dict[str, object]:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(contents)
    return {
        "path": path.relative_to(root).as_posix(),
        "size": len(contents),
        "sha256": hashlib.sha256(contents).hexdigest(),
    }


def comparison(model: str) -> dict:
    thresholds = {
        "mean_pooled_se_units": 0.35,
        "standard_error_relative": 0.30,
        "percentile_endpoint_pooled_se_units": 0.75,
        "bca_endpoint_pooled_se_units": 0.90,
        "odds_ratio_log_endpoint_pooled_se_units": 0.75,
    }
    if model == "ols":
        thresholds.pop("odds_ratio_log_endpoint_pooled_se_units")
    return {
        "passed": True,
        "thresholds": thresholds,
        "observed_maxima": {name: value / 10 for name, value in thresholds.items()},
        "threshold_checks": {name: True for name in thresholds},
    }


class RegressionBootstrapPromotionAuditTests(unittest.TestCase):
    def prepare_complete_evidence(self, root: Path) -> Path:
        results = root / "validation" / "results"
        results.mkdir(parents=True, exist_ok=True)
        (root / "dist" / "assets").mkdir(parents=True, exist_ok=True)
        (root / "dist" / "index.html").write_text("<main>QuickPLS</main>", encoding="utf-8")
        (root / "dist" / "assets" / "app.js").write_text("console.log('quickpls')", encoding="utf-8")
        (root / "dist" / "assets" / "app.css").write_text("body{margin:0}", encoding="utf-8")
        for relative in sorted(set([
            *audit.REFERENCE_SOURCE_PATHS,
            *audit.PACKAGED_SOURCE_PATHS,
            *audit.VISUAL_SOURCE_PATHS,
            audit.FAILURE_BOUNDARY_GENERATOR,
        ])):
            source = root / relative
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text("evidence generator", encoding="utf-8")
        (root / "crates" / "qpls-resampling" / "src" / "lib.rs").write_text(
            "\n".join([*sorted(audit.FAILURE_BOUNDARY_TEST_NAMES), "single_class_resample", "vec![7]"]),
            encoding="utf-8",
        )
        tested_cli = write_artifact(root / "target" / "release" / "qpls.exe", b"release-cli", root)
        tested_desktop = write_artifact(
            root / "target" / "release" / "quickpls-desktop.exe", b"release-desktop", root
        )
        tested_failure_binary = write_artifact(
            root / "target" / "release" / "deps" / "qpls_resampling-deadbeef.exe",
            b"release-rust-test-binary",
            root,
        )
        tested_project_binary = write_artifact(
            root / "target" / "release" / "deps" / "qpls_project-deadbeef.exe",
            b"release-project-test-binary",
            root,
        )
        tested_dist = audit._directory_manifest(root, "dist")

        write_json(
            results / audit.FAILURE_BOUNDARY_REPORT_NAME,
            {
                "schema_version": 1,
                "target": audit.FAILURE_BOUNDARY_TARGET,
                "feature_id": audit.FEATURE_ID,
                "method_version": audit.METHOD_VERSION,
                "generated_at_utc": "2026-08-12T11:57:00Z",
                "passed": True,
                "checks": {name: True for name in audit.FAILURE_BOUNDARY_TEST_NAMES},
                "archive_checks": {name: True for name in audit.ARCHIVE_BOUNDARY_TEST_NAMES},
                "build_commands": {
                    "qpls_resampling": [
                        "cargo", "test", "--release", "-p", "qpls-resampling", "--lib",
                        "--no-run", "--message-format=json",
                    ],
                    "qpls_project": [
                        "cargo", "test", "--release", "-p", "qpls-project", "--lib",
                        "--no-run", "--message-format=json",
                    ],
                },
                "test_executables": {
                    "qpls_resampling": tested_failure_binary,
                    "qpls_project": tested_project_binary,
                },
                "executions": {
                    name: {
                        "target": "qpls_resampling" if name in audit.FAILURE_BOUNDARY_TEST_NAMES else "qpls_project",
                        "full_name": f"tests::{name}",
                        "exit_code": 0,
                        "passed": True,
                        "stdout_tail": "1 passed; 0 failed",
                        "stderr_tail": "",
                    }
                    for name in audit.FAILURE_BOUNDARY_TEST_NAMES | audit.ARCHIVE_BOUNDARY_TEST_NAMES
                },
            },
        )

        method_doc = root / "docs" / "methods" / "REGRESSION_BOOTSTRAP_V1.md"
        method_doc.parent.mkdir(parents=True, exist_ok=True)
        method_doc.write_text(
            " ".join(
                [
                    audit.METHOD_VERSION,
                    "indexed_case_resampling_v1",
                    "percentile_primary_bca_conditional_v1",
                    "standard_normal_bootstrap_ratio_v1",
                    audit.WITNESS_VERSION,
                    "not rendered or exported",
                    "at most 50 predictors and controls plus the intercept",
                    "Studentized intervals, custom alpha/tails",
                    "PROCESS bootstrapping are excluded",
                ]
            ),
            encoding="utf-8",
        )
        compatibility_doc = root / "docs" / "METHOD_COMPATIBILITY.md"
        compatibility_doc.write_text(
            " | ".join(
                [
                    "Regression",
                    "OLS and binary-logistic bootstrapping",
                    f"Current {audit.METHOD_VERSION}",
                    "PROCESS, weights, multinomial, and ordinal inference remain excluded",
                    "Release-qualified for the bounded v1 scope",
                    "genuine packaged OLS and logistic 10,000-resample execution",
                ]
            ),
            encoding="utf-8",
        )

        for model, outcome, base in [
            ("ols", "y", "regression_ols_v1"),
            ("logistic", "bin_y", "regression_logistic_v2"),
        ]:
            model_config = (
                {"type": "ols", "robust_se": "hc3"}
                if model == "ols"
                else {"type": "logistic"}
            )
            write_json(
                results / f"regression_bootstrap_v1_{model}.recipe.json",
                {
                    "schema_version": 3,
                    "dataset_fingerprint": "v2:test",
                    "settings": {
                        "method": "regression",
                        "preprocessing": "unstandardized",
                        "missing_data": "listwise_deletion",
                        "bootstrap_samples": 1000,
                        "workers": 1,
                        "confidence_level": 0.95,
                    },
                    "method_config": {
                        "kind": "regression",
                        "outcome": outcome,
                        "predictors": ["x", "z"],
                        "controls": ["w"],
                        "model": model_config,
                        "bootstrap": {
                            "algorithm": "case_resampling",
                            "intervals": ["percentile", "bca"],
                        },
                    },
                    "metadata": {
                        "status": "validated_regression_bootstrap_v1_bounded_scope"
                    },
                    "model": {"constructs": [], "paths": []},
                },
            )
            write_json(
                results / f"regression_bootstrap_v1_{model}_quickpls.json",
                {
                    "status": "completed",
                    "provenance": {
                        "method": "regression",
                        "method_version": f"{base}+{audit.METHOD_VERSION}",
                        "settings": {"bootstrap_samples": 1000, "workers": 1},
                    },
                    "payload": {
                        "estimation": {
                            "method_version": base,
                            "regression": {
                                "method_version": base,
                                "regression_type": model,
                                "bootstrap": {
                                    "method_version": audit.METHOD_VERSION,
                                    "algorithm": "indexed_case_resampling_v1",
                                    "stream_token": "quickpls_indexed_resampling_v1",
                                    "coefficients": [{"term": "intercept"}],
                                    "validation_witness": {
                                        "method_version": audit.WITNESS_VERSION,
                                        "successful_bootstrap": [
                                            {"replicate_index": 0, "coefficients": [1.0]}
                                        ],
                                        "successful_jackknife": [
                                            {"omitted_case": 0, "coefficients": [1.0]}
                                        ],
                                    },
                                },
                            },
                        },
                        "assessment": {
                            "method_version": "assessment_not_applicable_v1"
                        },
                    },
                },
            )

        thresholds = {
            "mean_pooled_se_units": 0.35,
            "standard_error_relative": 0.30,
            "percentile_endpoint_pooled_se_units": 0.75,
            "bca_endpoint_pooled_se_units": 0.90,
            "odds_ratio_log_endpoint_pooled_se_units": 0.75,
        }
        write_json(
            results / audit.REFERENCE_REPORT_NAME,
            {
                "schema_version": 1,
                "target": audit.REFERENCE_TARGET,
                "feature_id": audit.FEATURE_ID,
                "method_version": audit.METHOD_VERSION,
                "catalogue_snapshot_date": audit.CATALOGUE_SNAPSHOT_DATE,
                "passed": True,
                "checks": {name: True for name in audit.REFERENCE_CHECK_NAMES},
                "scope": {
                    "base_methods": ["regression_ols_v1", "regression_logistic_v2"],
                    "algorithm": "case_resampling_with_replacement",
                    "intervals": ["percentile_type7_primary", "bca_midrank_conditional"],
                    "test_reference": "standard_normal_bootstrap_ratio_v1",
                    "alternative": "two_sided",
                    "confidence_level": 0.95,
                    "quickpls_replicates": 1000,
                    "python_reference_replicates": 2000,
                    "workers_compared": [1, 4],
                    "missing_data": "listwise_deletion",
                    "term_limit_including_intercept": 51,
                    "process_bootstrap": False,
                    "studentized_or_custom_alpha": False,
                },
                "exact_arithmetic": {
                    "tolerance": audit.EXACT_TOLERANCE,
                    "maximum_absolute_difference": 1e-12,
                    "frozen_supplied": {"maximum_absolute_difference": 1e-13},
                    "ols": {"maximum_absolute_difference": 1e-12},
                    "logistic": {"maximum_absolute_difference": 2e-12},
                },
                "independent_python": {
                    "point_tolerance": audit.POINT_TOLERANCE,
                    "point_maximum_absolute_difference": {
                        "ols": 1e-8,
                        "logistic": 2e-8,
                    },
                    "distribution_comparisons": {
                        "ols": comparison("ols"),
                        "logistic": comparison("logistic"),
                    },
                },
                "external_r": {
                    "available": True,
                    "passed": True,
                    "version": "R test",
                    "distribution_comparisons": {
                        "ols": comparison("ols"),
                        "logistic": comparison("logistic"),
                    },
                },
                "comparison_thresholds": thresholds,
                "artifacts": {
                    "fixture": "validation/results/v08_extended_methods_fixture.csv",
                    "r_script": "validation/regression_bootstrap_v1_reference.R",
                    "tested_cli": tested_cli,
                    "ols": {
                        "recipe": "validation/results/regression_bootstrap_v1_ols.recipe.json",
                        "quickpls_result": "validation/results/regression_bootstrap_v1_ols_quickpls.json",
                    },
                    "logistic": {
                        "recipe": "validation/results/regression_bootstrap_v1_logistic.recipe.json",
                        "quickpls_result": "validation/results/regression_bootstrap_v1_logistic_quickpls.json",
                    },
                },
            },
        )

        write_json(
            root / audit.PACKAGED_SOURCE_REPORT,
            {
                "schema_version": audit.PACKAGED_SCHEMA_VERSION,
                "passed": True,
                "generatedAt": "2026-08-12T11:58:00Z",
                "runtime": audit.PACKAGED_RUNTIME,
                "focusedRun": {"completedAt": "2026-08-12T11:59:00Z"},
                "feature_id": audit.FEATURE_ID,
                "method_version": audit.METHOD_VERSION,
                "catalogue_snapshot_date": audit.CATALOGUE_SNAPSHOT_DATE,
                "screenshots": [f"screen-{index}.png" for index in range(6)],
                "checks": {
                    "regressionBootstrapWorkflow": {
                        "passed": True,
                        "feature_id": audit.FEATURE_ID,
                        "method_version": audit.METHOD_VERSION,
                        "catalogue_snapshot_date": audit.CATALOGUE_SNAPSHOT_DATE,
                    },
                    "regressionBootstrapResults": {
                        "passed": True,
                        "olsInitialSelectedTable": audit.REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
                        "logisticInitialSelectedTable": audit.REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
                    },
                    "regressionBootstrapOlsExport": {"passed": True},
                    "regressionBootstrapLogisticExport": {"passed": True},
                    "regressionBootstrapSaveReopen": {
                        "passed": True,
                        "initialSelectedTables": {
                            "ols": audit.REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
                            "logistic": audit.REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
                        },
                    },
                    "regressionBootstrapCancellation": {"passed": True},
                    "regressionBootstrapWitnessBoundary": {"passed": True},
                },
            },
        )
        ols_xlsx = write_artifact(results / "regression-bootstrap-ols.xlsx", b"ols-xlsx", root)
        logistic_xlsx = write_artifact(
            results / "regression-bootstrap-logistic.xlsx", b"logistic-xlsx", root
        )
        archive = write_artifact(results / "regression-bootstrap.qpls", b"archive", root)
        screenshots = [
            write_artifact(
                results / "screens" / f"bootstrap-{index}.png",
                f"screen-{index}".encode(),
                root,
            )
            for index in range(6)
        ]
        checks = {
            "workflow": {
                "passed": True,
                "ols_completed": True,
                "logistic_completed": True,
                "active_lifecycle_captured": True,
                "model_free": True,
                "source_check": "regressionBootstrapWorkflow",
            },
            "results": {
                "passed": True,
                "ols_initial_selected_table": audit.REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
                "logistic_initial_selected_table": audit.REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
                "ols_coefficient_rows": 4,
                "logistic_coefficient_rows": 4,
                "percentile_primary_present": True,
                "bca_conditional_present": True,
                "failure_disclosure_truthful": True,
                "validation_witness_not_rendered": True,
                "no_na_fabrication": True,
                "source_check": "regressionBootstrapResults",
            },
            "ols_export": {
                "passed": True,
                "workbook_sheets": ["Regression bootstrap summary"],
                "validation_witness_excluded": True,
                "witness_scan": {
                    "passed": True,
                    "total_members": 12,
                    "scanned_xml_and_rels_members": ["xl/workbook.xml", "xl/worksheets/sheet1.xml"],
                    "worksheet_members": ["xl/worksheets/sheet1.xml"],
                    "worksheet_row_counts": {"xl/worksheets/sheet1.xml": 4},
                    "forbidden_matches": [],
                    "extraction_errors": [],
                },
                "artifact_sha256": ols_xlsx["sha256"],
                "source_check": "regressionBootstrapOlsExport",
            },
            "logistic_export": {
                "passed": True,
                "workbook_sheets": ["Regression bootstrap summary"],
                "validation_witness_excluded": True,
                "witness_scan": {
                    "passed": True,
                    "total_members": 12,
                    "scanned_xml_and_rels_members": ["xl/workbook.xml", "xl/worksheets/sheet1.xml"],
                    "worksheet_members": ["xl/worksheets/sheet1.xml"],
                    "worksheet_row_counts": {"xl/worksheets/sheet1.xml": 4},
                    "forbidden_matches": [],
                    "extraction_errors": [],
                },
                "artifact_sha256": logistic_xlsx["sha256"],
                "source_check": "regressionBootstrapLogisticExport",
            },
            "save_reopen": {
                "passed": True,
                "ols_same_run_restored": True,
                "logistic_same_run_restored": True,
                "ols_initial_selected_table": audit.REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
                "logistic_initial_selected_table": audit.REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
                "project_checksum_matches": True,
                "archive_witness_validated": True,
                "archive_sha256": archive["sha256"],
                "source_check": "regressionBootstrapSaveReopen",
            },
            "cancellation": {
                "passed": True,
                "active_lifecycle_captured": True,
                "no_partial_result": True,
                "source_check": "regressionBootstrapCancellation",
            },
            "witness_boundary": {
                "passed": True,
                "archive_only": True,
                "term_order_exact": True,
                "bootstrap_index_partition_exact": True,
                "jackknife_index_partition_exact": True,
                "excluded_from_results": True,
                "excluded_from_exports": True,
                "source_check": "regressionBootstrapWitnessBoundary",
            },
        }
        write_json(
            results / audit.PACKAGED_REPORT_NAME,
            {
                "schema_version": audit.PACKAGED_SCHEMA_VERSION,
                "kind": audit.PACKAGED_KIND,
                "passed": True,
                "generated_at_utc": "2026-08-12T11:58:00Z",
                "completed_at_utc": "2026-08-12T11:59:00Z",
                "feature_id": audit.FEATURE_ID,
                "method_version": audit.METHOD_VERSION,
                "catalogue_snapshot_date": audit.CATALOGUE_SNAPSHOT_DATE,
                "target": audit.PACKAGED_PLATFORM_TARGET,
                "runtime": audit.PACKAGED_RUNTIME,
                "endpoint": "http://127.0.0.1:9222",
                "generator": audit.PACKAGED_GENERATOR,
                "tested_product": {
                    "quickpls_desktop_exe": tested_desktop,
                    "dist_bundle": tested_dist,
                },
                "checks": checks,
                "artifacts": {
                    "ols_xlsx": ols_xlsx,
                    "logistic_xlsx": logistic_xlsx,
                    "project_archive": archive,
                    "screenshots": screenshots,
                },
                "console_errors": [],
                "failures": [],
                "source_report": audit.PACKAGED_SOURCE_REPORT,
            },
        )
        write_json(
            results / audit.PROCESS_CLEANUP_REPORT_NAME,
            {
                "launched_pid": 4242,
                "descendants_at_shutdown": [],
                "graceful_close_exit_code": 0,
                "graceful_exit_confirmed": True,
                "forced_parent_termination": False,
                "forced_descendant_pids": [],
                "parent_exit_confirmed": True,
                "lingering_descendant_pids": [],
                "passed": True,
            },
        )

        accessibility = {
            "labeledRegressionType": 1,
            "labeledOutcome": 1,
            "labeledBootstrapToggle": 1,
            "labeledSamples": 1,
            "labeledWorkers": 1,
            "labeledSeed": 1,
            "predictorGroup": 1,
            "controlGroup": 1,
            "distinctControlIds": 6,
        }
        screenshots = []
        visual_rows = []
        for viewport in sorted(audit.VISUAL_VIEWPORTS):
            for state in ("regression-bootstrap-ols-dialog", "regression-bootstrap-logistic-dialog"):
                screen = results / "screens" / f"{state}-{viewport}.png"
                screen.parent.mkdir(parents=True, exist_ok=True)
                screen.write_bytes(f"{state}-{viewport}".encode())
                screenshots.append({"state": state, "viewport": viewport, "path": str(screen)})
            visual_rows.append({
                "viewport": viewport,
                "fixture": {"variables": 5, "models": 0},
                "dataSurface": True,
                "visibleModelNodes": 0,
                "analyzeCommandCount": 1,
                "dialogOpened": True,
                "catalogCount": 14,
                "selectedMethod": "Regression",
                "linkage": {"linkage": True},
                "category": "Standalone analysis",
                "regressionTypeOptions": [
                    {"value": "ols", "label": "Ordinary least squares"},
                    {"value": "logistic", "label": "Binary logistic (outcome coded 0/1)"},
                ],
                "outcome": "outcome",
                "roles": {"selectedPredictors": ["predictor"], "selectedControls": ["control"]},
                "bootstrap": {
                    "value": "enabled",
                    "options": [
                        {"value": "off", "label": "Off"},
                        {"value": "enabled", "label": "Case-resampling bootstrap"},
                    ],
                    "samples": {"count": 1, "value": "10000", "min": "99", "max": "10000", "step": "1"},
                    "workers": {"value": "1"},
                    "seed": {"value": "20260718"},
                    "scope": audit.BOOTSTRAP_SCOPE_NOTE,
                    "toggleFocused": True,
                },
                "accessibility": accessibility,
                "ols": {
                    "type": "ols",
                    "validatedScope": audit.OLS_SCOPE_NOTE,
                    "blockers": {"runtime": ["desktop runtime"], "unexpected": [], "model": []},
                    "startCommandCount": 1,
                    "startCommandDisabled": True,
                    "truthAndOverflow": {"noFabricatedRunState": True, "noHorizontalOverflow": True},
                    "dialogBounds": {"withinHorizontalViewport": True, "pageHorizontalOverflow": False},
                    "noPhantomResult": True,
                },
                "logistic": {
                    "type": "logistic",
                    "typeFocused": True,
                    "validatedScope": audit.LOGISTIC_SCOPE_NOTE,
                    "bootstrapScope": audit.BOOTSTRAP_SCOPE_NOTE,
                    "bootstrapValue": "enabled",
                    "samples": "10000",
                    "workers": "1",
                    "seed": "20260718",
                    "profile": {"role": "status", "ariaLive": "polite", "ariaBusy": "false"},
                    "blockers": {
                        "runtime": ["desktop runtime"],
                        "allowedFixtureProfile": ["combined", "coding", "classes"],
                        "unexpected": [],
                        "model": [],
                    },
                    "startCommandCount": 1,
                    "startCommandDisabled": True,
                    "truthAndOverflow": {"noFabricatedRunState": True, "noHorizontalOverflow": True},
                    "dialogBounds": {"withinHorizontalViewport": True, "pageHorizontalOverflow": False},
                    "noPhantomResult": True,
                },
                "closeFocus": {"dialogClosed": True, "focusRestored": True},
                "completedResult": {"synthesizedByHarness": False},
            })
        write_json(
            results / audit.VISUAL_REPORT_NAME,
            {
                "passed": True,
                "viewports": [
                    {"id": viewport, "width": int(viewport.split("x")[0]), "height": int(viewport.split("x")[1])}
                    for viewport in sorted(audit.VISUAL_VIEWPORTS)
                ],
                "checks": {"regressionBootstrap": visual_rows},
                "screenshots": screenshots,
                "consoleErrors": [],
                "failures": [],
            },
        )
        return results

    def test_missing_evidence_fails_with_current_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            results = root / "validation" / "results"
            results.mkdir(parents=True)
            report = audit.build_audit(root=root, results=results, now=NOW)
            self.assertFalse(report["passed"])
            self.assertEqual(report["feature_id"], audit.FEATURE_ID)
            self.assertEqual(report["method_version"], audit.METHOD_VERSION)

    def test_complete_exact_evidence_can_pass(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            results = self.prepare_complete_evidence(root)
            report = audit.build_audit(root=root, results=results, now=NOW)
            self.assertTrue(report["passed"], report)

    def test_true_flags_cannot_hide_numeric_or_artifact_tampering(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            results = self.prepare_complete_evidence(root)
            reference_path = results / audit.REFERENCE_REPORT_NAME
            reference = json.loads(reference_path.read_text(encoding="utf-8"))
            reference["independent_python"]["distribution_comparisons"]["ols"][
                "observed_maxima"
            ]["mean_pooled_se_units"] = 99.0
            write_json(reference_path, reference)
            self.assertFalse(audit.build_audit(root=root, results=results, now=NOW)["passed"])

            self.prepare_complete_evidence(root)
            (results / "regression-bootstrap-ols.xlsx").write_bytes(b"tampered")
            self.assertFalse(audit.build_audit(root=root, results=results, now=NOW)["passed"])

    def test_generic_or_wrong_identity_packaged_report_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            results = self.prepare_complete_evidence(root)
            packaged_path = results / audit.PACKAGED_REPORT_NAME
            packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
            packaged["kind"] = "cumulative_native_acceptance"
            packaged["feature_id"] = "qpls3.standalone.logistic"
            write_json(packaged_path, packaged)
            report = audit.build_audit(root=root, results=results, now=NOW)
            self.assertFalse(report["passed"])
            self.assertFalse(report["packaged_acceptance"]["passed"])

    def test_bootstrap_default_table_must_be_summary_for_completion_and_reopen(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            results = self.prepare_complete_evidence(root)
            packaged_path = results / audit.PACKAGED_REPORT_NAME
            packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
            packaged["checks"]["results"]["ols_initial_selected_table"] = "ols_coefficients"
            write_json(packaged_path, packaged)
            report = audit.build_audit(root=root, results=results, now=NOW)
            self.assertFalse(report["passed"])
            self.assertFalse(report["packaged_attestation"]["passed"])

            self.prepare_complete_evidence(root)
            source_path = root / audit.PACKAGED_SOURCE_REPORT
            source = json.loads(source_path.read_text(encoding="utf-8"))
            source["checks"]["regressionBootstrapSaveReopen"]["initialSelectedTables"]["logistic"] = "logistic_coefficients"
            write_json(source_path, source)
            self.assertFalse(audit.build_audit(root=root, results=results, now=NOW)["passed"])

    def test_visual_gate_requires_exact_three_viewport_method_contract(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            results = self.prepare_complete_evidence(root)
            visual_path = results / audit.VISUAL_REPORT_NAME
            visual = json.loads(visual_path.read_text(encoding="utf-8"))
            visual["checks"]["regressionBootstrap"][0]["accessibility"][
                "labeledBootstrapToggle"
            ] = 0
            write_json(visual_path, visual)
            report = audit.build_audit(root=root, results=results, now=NOW)
            self.assertFalse(report["passed"])
            self.assertFalse(report["visual_attestation"]["passed"])
            visual = json.loads(visual_path.read_text(encoding="utf-8"))
            visual["checks"]["regressionBootstrap"].pop()
            write_json(visual_path, visual)
            self.assertFalse(audit.build_audit(root=root, results=results, now=NOW)["passed"])

    def test_product_binary_dist_and_xlsx_scan_are_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            results = self.prepare_complete_evidence(root)
            (root / "target" / "release" / "qpls.exe").write_bytes(b"changed-cli")
            self.assertFalse(audit.build_audit(root=root, results=results, now=NOW)["passed"])

            self.prepare_complete_evidence(root)
            (root / "target" / "release" / "quickpls-desktop.exe").write_bytes(b"changed-desktop")
            self.assertFalse(audit.build_audit(root=root, results=results, now=NOW)["passed"])

            self.prepare_complete_evidence(root)
            (root / "dist" / "assets" / "app.js").write_text("changed", encoding="utf-8")
            self.assertFalse(audit.build_audit(root=root, results=results, now=NOW)["passed"])

            self.prepare_complete_evidence(root)
            packaged_path = results / audit.PACKAGED_REPORT_NAME
            packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
            packaged["checks"]["ols_export"]["witness_scan"]["extraction_errors"] = [
                "xl/worksheets/sheet1.xml"
            ]
            write_json(packaged_path, packaged)
            self.assertFalse(audit.build_audit(root=root, results=results, now=NOW)["passed"])

            self.prepare_complete_evidence(root)
            cleanup_path = results / audit.PROCESS_CLEANUP_REPORT_NAME
            cleanup = json.loads(cleanup_path.read_text(encoding="utf-8"))
            cleanup["lingering_descendant_pids"] = [9001]
            cleanup["passed"] = False
            write_json(cleanup_path, cleanup)
            self.assertFalse(audit.build_audit(root=root, results=results, now=NOW)["passed"])

            self.prepare_complete_evidence(root)
            failure_path = results / audit.FAILURE_BOUNDARY_REPORT_NAME
            failure_report = json.loads(failure_path.read_text(encoding="utf-8"))
            failed_name = next(iter(audit.FAILURE_BOUNDARY_TEST_NAMES))
            failure_report["executions"][failed_name]["exit_code"] = 101
            failure_report["executions"][failed_name]["passed"] = False
            write_json(failure_path, failure_report)
            self.assertFalse(audit.build_audit(root=root, results=results, now=NOW)["passed"])

    def test_product_edits_after_evidence_invalidate_freshness(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            results = self.prepare_complete_evidence(root)
            source = root / "crates" / "qpls-resampling" / "src" / "lib.rs"
            source.write_text("post-evidence product edit", encoding="utf-8")
            newest_report = max(
                (results / audit.REFERENCE_REPORT_NAME).stat().st_mtime,
                (results / audit.PACKAGED_REPORT_NAME).stat().st_mtime,
                (results / audit.VISUAL_REPORT_NAME).stat().st_mtime,
            )
            os.utime(source, (newest_report + 5, newest_report + 5))
            report = audit.build_audit(root=root, results=results, now=NOW)
            self.assertFalse(report["passed"])
            self.assertFalse(report["reference_report"]["freshness"]["passed"])
            self.assertFalse(report["packaged_acceptance"]["freshness"]["passed"])


if __name__ == "__main__":
    unittest.main()
