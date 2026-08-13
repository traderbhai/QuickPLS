#!/usr/bin/env python3
"""Focused fail-closed tests for structural randomization promotion evidence."""

from __future__ import annotations

import copy
import hashlib
import inspect
import json
import shutil
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

import inference_method_promotion_audit as broad_promotion
import inference_publication_audit as broad_publication
import method_promotion_product_enforcement_audit as product_enforcement
import structural_path_randomization_frontend_gate as frontend_gate
import structural_path_randomization_method_promotion_audit as audit


class StructuralPathRandomizationPromotionAuditTests(unittest.TestCase):
    def test_frozen_identity_and_exact_dedicated_gate_manifests(self) -> None:
        self.assertEqual(audit.FEATURE_ID, "qpls3.inference.structural_path_randomization")
        self.assertEqual(audit.METHOD_VERSION, "freedman_lane_permutation_v1")
        self.assertEqual(audit.CATALOGUE_SNAPSHOT_DATE, "2026-08-12")
        self.assertEqual(audit.ANALYSIS_RESULT_SCHEMA_VERSION, 1)
        self.assertEqual(audit.ANALYSIS_RECIPE_SCHEMA_VERSION, 3)
        self.assertEqual(audit.PLS_INFERENCE_PAYLOAD_KIND, "pls_pm_v3")
        self.assertEqual(audit.PROJECT_ARCHIVE_SCHEMA_VERSION, 5)
        self.assertEqual(
            audit.PACKAGED_SOURCE_CHECKS,
            frozenset({
                "runtimePreflight", "structuralPathRandomizationFixtureProvisioning",
                "structuralPathRandomizationSetup", "structuralPathRandomizationCancellation",
                "structuralPathRandomizationResults", "structuralPathRandomizationExport",
                "structuralPathRandomizationArchive", "structuralPathRandomizationSaveReopen",
                "resources", "cleanup",
            }),
        )
        self.assertEqual(
            audit.PACKAGED_SOURCE_METADATA_CHECKS,
            frozenset({"runtime", "recentProjectsRestored"}),
        )
        self.assertEqual(
            audit.REFERENCE_EVIDENCE_SOURCE_PATHS[0],
            "validation/structural_path_randomization_reference.py",
        )
        self.assertIn(
            "validation/structural_path_randomization_method_promotion_audit.py",
            audit.REFERENCE_EVIDENCE_SOURCE_PATHS,
        )
        self.assertEqual(sum(len(names) for names in audit.BOUNDARY_SUITES.values()), 6)
        self.assertEqual(len(audit.FRONTEND_TEST_FILES), 15)
        self.assertIn("src/store.test.ts", audit.FRONTEND_TEST_FILES)
        self.assertTrue(
            {
                "src/components/ReportsWorkspace.test.ts",
                "src/native/NativeExportDialog.test.ts",
                "src/domain/publicationDiagram.test.ts",
                "src/components/TrustCenterWorkspace.test.tsx",
                "src/native/nativeControllerContracts.test.ts",
            }.issubset(audit.FRONTEND_TEST_FILES)
        )
        self.assertEqual(frontend_gate.FRONTEND_TEST_FILES, audit.FRONTEND_TEST_FILES)
        self.assertIn("index.html", frontend_gate.FRONTEND_GATE_SOURCES)
        self.assertTrue(
            {
                path.relative_to(audit.ROOT).as_posix()
                for path in (audit.ROOT / "src").rglob("*.css")
            }.issubset(frontend_gate.FRONTEND_GATE_SOURCES)
        )
        self.assertNotIn("bootstrap", " ".join(audit.BOUNDARY_SUITES))
        reference_paths = {
            path.relative_to(audit.ROOT).as_posix()
            for path in audit.reference_product_sources(audit.ROOT)
        }
        packaged_paths = {
            path.relative_to(audit.ROOT).as_posix()
            for path in audit.packaged_product_sources(audit.ROOT)
        }
        self.assertIn("crates/qpls-project/Cargo.toml", reference_paths)
        self.assertTrue(
            any(
                path.startswith("crates/qpls-project/") and path.endswith(".rs")
                for path in reference_paths
            )
        )
        self.assertTrue(reference_paths.issubset(packaged_paths))
        cli_manifest = (audit.ROOT / "crates" / "qpls-cli" / "Cargo.toml").read_text(
            encoding="utf-8"
        )
        tauri_manifest = (audit.ROOT / "src-tauri" / "Cargo.toml").read_text(
            encoding="utf-8"
        )
        self.assertIn('qpls-project = { path = "../qpls-project" }', cli_manifest)
        self.assertIn('qpls-project = { path = "../crates/qpls-project" }', tauri_manifest)
        self.assertEqual(
            audit.PACKAGED_VALIDATION_SOURCE_PATHS,
            (
                "validation/v247_tauri_native_acceptance.mjs",
                "validation/run_v247_structural_path_randomization_native_acceptance.ps1",
                "validation/monitor_quickpls_process_tree.ps1",
                "validation/windows_native_save_export.py",
                "validation/close_tauri_test_window.mjs",
            ),
        )

    @staticmethod
    def packaged_results() -> dict[str, object]:
        rows = []
        for path, exceedances, original in (("X -> Y", 1500, 0.25), ("Z -> Y", 500, -0.125)):
            rows.append(
                [
                    path,
                    str(original),
                    str(exceedances),
                    "10000",
                    str((exceedances + 1) / 10001),
                ]
            )
        return {
            "passed": True,
            "runId": "run-1",
            "selectedRunId": "run-1",
            "initialSelectedTable": "model_estimates",
            "group": "Inference",
            "tableId": "permutation",
            "title": "Structural path randomization",
            "warning": audit.PACKAGED_WARNING,
            "columns": list(audit.PACKAGED_RESULT_COLUMNS),
            "rows": rows,
            "pathOrder": list(audit.PACKAGED_PATH_LABELS),
            "noBootstrapTables": True,
            "noPlaceholderValues": True,
            "runDetails": {
                "properties": {
                    "Method": "Structural Path Randomization",
                    "Recorded seed": "20260718",
                    "Method version": audit.EXPECTED_PROVENANCE_METHOD_VERSION,
                    "Recipe": "recipe-1",
                    "Dataset fingerprint": "sha256:" + "a" * 64,
                },
                "logEntries": 1,
            },
        }

    @staticmethod
    def write_packaged_archive(
        root: Path,
        filename: str,
        exceedances: tuple[int, int] = (1500, 500),
        result_schema_version: int = audit.ANALYSIS_RESULT_SCHEMA_VERSION,
    ) -> tuple[Path, dict[str, object]]:
        dataset_id = "10000000-0000-0000-0000-000000000001"
        model_id = "20000000-0000-0000-0000-000000000001"
        recipe_id = "30000000-0000-0000-0000-000000000001"
        result_id = "40000000-0000-0000-0000-000000000001"
        fingerprint = "sha256:" + "a" * 64
        paths = [
            {"source": "construct-x", "target": "construct-y"},
            {"source": "construct-z", "target": "construct-y"},
        ]
        constructs = [
            {"id": "construct-x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1"]},
            {"id": "construct-z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1"]},
            {"id": "construct-y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1"]},
        ]
        model = {
            "id": model_id,
            "name": "Structural Path Randomization Model",
            "constructs": constructs,
            "paths": paths,
            "controls": [],
            "higher_order_constructs": [],
            "interactions": [],
        }
        settings = {
            "method": "pls_pm",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3000,
            "bootstrap_samples": 0,
            "studentized_inner_samples": 0,
            "permutation_samples": 10000,
            "seed": 20260718,
            "workers": 4,
            "confidence_level": 0.95,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
            "case_weight_column": None,
        }
        originals = (0.25, -0.125)
        parameters = [
            {
                "parameter": json.dumps(
                    ["path", [path["source"], path["target"]]],
                    separators=(",", ":"),
                ),
                "original": original,
                "permutations": 10000,
                "exceedances": exceedance,
                "p_value_two_sided": (exceedance + 1) / 10001,
            }
            for path, original, exceedance in zip(paths, originals, exceedances)
        ]
        project = {
            "datasets": [{"id": dataset_id, "name": "fixture", "schema": {}, "fingerprint": fingerprint}],
            "models": [model],
            "recipes": [{
                "schema_version": 3,
                "id": recipe_id,
                "created_at": "2026-08-13T00:00:00Z",
                "dataset_fingerprint": fingerprint,
                "model": copy.deepcopy(model),
                "settings": settings,
                "method_config": {"kind": "pls_permutation"},
                "metadata": {"status": "candidate_freedman_lane_path_randomization_scope"},
            }],
            "layouts": {"workspace": {"runs": [{
                "id": result_id,
                "method": "Structural Path Randomization",
                "status": "completed",
                "modelId": model_id,
                "logs": [{"message": "completed"}],
                "modelSnapshot": {
                    "nodes": [{"id": construct["id"]} for construct in constructs],
                    "edges": copy.deepcopy(paths),
                },
            }]}},
            "results": [{
                "schema_version": result_schema_version,
                "id": result_id,
                "status": "completed",
                "provenance": {
                    "recipe_id": recipe_id,
                    "dataset_fingerprint": fingerprint,
                    "method": "pls_pm",
                    "method_version": audit.EXPECTED_PROVENANCE_METHOD_VERSION,
                    "engine_version": "2.45.0",
                    "seed": 20260718,
                    "settings": copy.deepcopy(settings),
                    "started_at": "2026-08-13T00:00:00Z",
                    "completed_at": "2026-08-13T00:01:00Z",
                },
                "diagnostics": [],
                "payload": {
                    "kind": "pls_pm_v3",
                    "estimation": {
                        "method_version": "pls_pm_v1",
                        "paths": [
                            {**path, "coefficient": original}
                            for path, original in zip(paths, originals)
                        ],
                    },
                    "assessment": {},
                    "bootstrap": None,
                    "permutation": {
                        "method_version": audit.METHOD_VERSION,
                        "parameters": parameters,
                        "plan": {
                            "master_seed": 20260718,
                            "operation": audit.PLAN_OPERATION,
                            "permutations": 10000,
                        },
                    },
                },
            }],
        }
        project_bytes = json.dumps(project, indent=2).encode("utf-8")
        data_bytes = b"arrow fixture bytes"
        manifest = {
            "schema_version": 5,
            "project_id": "50000000-0000-0000-0000-000000000001",
            "name": "Native Structural Path Randomization Acceptance",
            "created_at": "2026-08-13T00:00:00Z",
            "modified_at": "2026-08-13T00:01:00Z",
            "engine_version": "2.45.0",
            "checksum_algorithm": "sha256",
            "checksums": {
                f"data/{dataset_id}.arrow": hashlib.sha256(data_bytes).hexdigest(),
                "project.json": hashlib.sha256(project_bytes).hexdigest(),
            },
        }
        archive = root / "validation" / "results" / filename
        archive.parent.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as output:
            output.writestr("project.json", project_bytes)
            output.writestr(f"data/{dataset_id}.arrow", data_bytes)
            output.writestr("manifest.json", json.dumps(manifest, indent=2).encode("utf-8"))
        return archive, audit.file_descriptor(root, archive)

    @staticmethod
    def write_cancellation_snapshot(
        root: Path, phase: str, source: Path | None = None
    ) -> tuple[Path, dict[str, object]]:
        archive = root / "validation" / "results" / (
            f"v247-native-structural-path-randomization-1-2-cancellation-{phase}.qpls"
        )
        archive.parent.mkdir(parents=True, exist_ok=True)
        if source is not None:
            shutil.copyfile(source, archive)
            return archive, audit.file_descriptor(root, archive)
        dataset_id = "10000000-0000-0000-0000-000000000001"
        model_id = "20000000-0000-0000-0000-000000000001"
        constructs = [
            {"id": "construct-x", "name": "X", "short_name": "X", "mode": "reflective", "indicators": ["x1"]},
            {"id": "construct-z", "name": "Z", "short_name": "Z", "mode": "reflective", "indicators": ["z1"]},
            {"id": "construct-y", "name": "Y", "short_name": "Y", "mode": "reflective", "indicators": ["y1"]},
        ]
        paths = [
            {"source": "construct-x", "target": "construct-y"},
            {"source": "construct-z", "target": "construct-y"},
        ]
        model = {
            "id": model_id,
            "name": "Structural Path Randomization Model",
            "constructs": constructs,
            "paths": paths,
            "controls": [],
            "higher_order_constructs": [],
            "interactions": [],
        }
        project = {
            "datasets": [{"id": dataset_id, "name": "fixture", "schema": {}, "fingerprint": "sha256:" + "a" * 64}],
            "models": [model],
            "recipes": [],
            "layouts": {"workspace": {"activeModelId": model_id, "runs": []}},
            "results": [],
        }
        project_bytes = json.dumps(project, indent=2).encode("utf-8")
        data_bytes = b"arrow fixture bytes"
        manifest = {
            "schema_version": 5,
            "project_id": "50000000-0000-0000-0000-000000000001",
            "name": "Native Structural Path Randomization Acceptance",
            "created_at": "2026-08-13T00:00:00Z",
            "modified_at": "2026-08-13T00:00:00Z",
            "engine_version": "2.45.0",
            "checksum_algorithm": "sha256",
            "checksums": {
                f"data/{dataset_id}.arrow": hashlib.sha256(data_bytes).hexdigest(),
                "project.json": hashlib.sha256(project_bytes).hexdigest(),
            },
        }
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as output:
            output.writestr("project.json", project_bytes)
            output.writestr(f"data/{dataset_id}.arrow", data_bytes)
            output.writestr("manifest.json", json.dumps(manifest, indent=2).encode("utf-8"))
        return archive, audit.file_descriptor(root, archive)

    def test_exact_check_rows_rejects_missing_extra_duplicate_and_failure(self) -> None:
        expected = frozenset({"one", "two"})
        valid = [{"name": "one", "passed": True}, {"name": "two", "passed": True}]
        self.assertTrue(audit.exact_check_rows(valid, expected)["passed"])
        for mutation in (
            valid[:1],
            [*valid, {"name": "extra", "passed": True}],
            [valid[0], copy.deepcopy(valid[0])],
            [{"name": "one", "passed": False}, valid[1]],
            [valid[0], "two"],
        ):
            self.assertFalse(audit.exact_check_rows(mutation, expected)["passed"])

    def test_packaged_source_checks_bind_pass_rows_and_exact_runtime_metadata(self) -> None:
        valid = {name: {"passed": True} for name in audit.PACKAGED_SOURCE_CHECKS}
        valid.update({
            "runtime": {
                "title": "QuickPLS", "tauriRuntime": True,
                "viewport": {"width": 1536, "height": 794, "dpr": 1.25},
                "surface": "launcher",
            },
            "recentProjectsRestored": True,
        })
        self.assertTrue(audit.packaged_source_check_rows(valid)["passed"])
        mutations = (
            lambda value: value["runtime"].update(surface="results"),
            lambda value: value["runtime"]["viewport"].update(height=699),
            lambda value: value.update(recentProjectsRestored=False),
            lambda value: value["runtime"].update(extra=True),
            lambda value: value.update(extra={"passed": True}),
        )
        for mutate in mutations:
            value = copy.deepcopy(valid)
            mutate(value)
            self.assertFalse(audit.packaged_source_check_rows(value)["passed"])

    def test_reference_worker_rows_require_exact_result_envelope_schema_v1(self) -> None:
        fingerprint = "v2:" + "a" * 64
        rows = [
            {
                "workers": workers,
                "result_schema_version": audit.ANALYSIS_RESULT_SCHEMA_VERSION,
                "recipe_id": f"00000000-0000-0000-0000-{32_000 + workers:012d}",
                "dataset_fingerprint": fingerprint,
                "method_version": audit.EXPECTED_PROVENANCE_METHOD_VERSION,
                "construct_score_rows": 12,
                "parameters": [{}, {}, {}],
            }
            for workers in (1, 4)
        ]
        self.assertTrue(audit.reference_worker_rows_passed(rows, fingerprint))
        for wrong_version in (0, 2, 3, 4, audit.PROJECT_ARCHIVE_SCHEMA_VERSION):
            mutation = copy.deepcopy(rows)
            mutation[0]["result_schema_version"] = wrong_version
            with self.subTest(wrong_version=wrong_version):
                self.assertFalse(
                    audit.reference_worker_rows_passed(mutation, fingerprint)
                )

    def test_frontend_freshness_closes_over_entrypoint_and_all_stylesheets(self) -> None:
        observed = {
            path.relative_to(audit.ROOT).as_posix()
            for path in audit.frontend_source_paths(audit.ROOT)
        }
        expected_css = {
            path.relative_to(audit.ROOT).as_posix()
            for path in (audit.ROOT / "src").rglob("*.css")
        }
        self.assertIn("index.html", observed)
        self.assertTrue(expected_css)
        self.assertTrue(expected_css.issubset(observed))
        self.assertTrue(
            {
                "src/styles.css",
                "src/native/nativeDesktop.css",
                "src/native/nativeCanvas.css",
                "src/v2/nativePrototype.css",
            }.issubset(observed)
        )
        mutated = set(observed)
        mutated.remove("index.html")
        self.assertFalse({"index.html", *expected_css}.issubset(mutated))

    def test_frontend_rows_are_order_independent_but_exact_and_typed(self) -> None:
        rows = [
            {
                "path": path,
                "status": "passed",
                "assertions": 1,
                "passed_assertions": 1,
                "failed_assertions": 0,
            }
            for path in reversed(audit.FRONTEND_TEST_FILES)
        ]
        self.assertTrue(audit.frontend_test_rows_passed(rows))
        duplicate = copy.deepcopy(rows)
        duplicate[-1]["path"] = duplicate[0]["path"]
        self.assertFalse(audit.frontend_test_rows_passed(duplicate))
        bool_count = copy.deepcopy(rows)
        bool_count[0]["assertions"] = True
        self.assertFalse(audit.frontend_test_rows_passed(bool_count))

    def test_visual_contract_is_exact_and_fails_closed(self) -> None:
        viewport = "1024x700"
        row = {
            "viewport": viewport,
            "dialogOpened": True,
            "pointerSelected": True,
            "linkage": {
                "expectedKind": "pls_permutation",
                "expectedLabel": "Structural Path Randomization",
                "selectedCount": 1,
                "panelCount": 1,
                "headingCount": 1,
                "linkage": True,
            },
            "permutationsInputCount": 1,
            "permutationsInputType": "number",
            "permutationsInputValue": "999",
            "expectedDefaultPermutations": "999",
            "bootstrapSamplesInputCount": 0,
            "mutuallyExclusive": True,
            "methodDescription": (
                "Structural Path Randomization Run candidate single-model Freedman-Lane "
                "randomization for structural paths using fixed original PLS construct scores "
                "and unadjusted pathwise p values."
            ),
            "distinctFromMgaAndMicom": True,
            "truthAndOverflow": {
                "noFabricatedRunState": True,
                "noHorizontalOverflow": True,
                "horizontalOverflow": {
                    "dialogOutsideViewport": False,
                    "dialogContentOverflow": False,
                    "pageOverflow": False,
                },
            },
            "closeFocus": {"dialogClosed": True, "focusRestored": True},
        }
        self.assertTrue(audit.visual_row_passed(row, viewport))
        mutation = copy.deepcopy(row)
        mutation["bootstrapSamplesInputCount"] = 1
        self.assertFalse(audit.visual_row_passed(mutation, viewport))
        description_mutations = (
            lambda value: value.replace("single-model ", ""),
            lambda value: value.replace("Freedman-Lane randomization", "path randomization"),
            lambda value: value.replace("structural paths", "model paths"),
            lambda value: value.replace(
                "fixed original PLS construct scores", "construct scores"
            ),
            lambda value: value.replace("unadjusted pathwise p values", "p values"),
        )
        for mutate in description_mutations:
            with self.subTest(mutate=mutate):
                mutation = copy.deepcopy(row)
                mutation["methodDescription"] = mutate(row["methodDescription"])
                self.assertFalse(audit.visual_row_passed(mutation, viewport))
        mutation = copy.deepcopy(row)
        mutation["distinctFromMgaAndMicom"] = False
        self.assertFalse(audit.visual_row_passed(mutation, viewport))

    def test_visual_screenshots_are_exactly_ordered_and_rehashed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            descriptors = []
            for index, expected in enumerate(audit.VISUAL_RANDOMIZATION_SCREENSHOTS):
                screenshot = root / expected["path"]
                screenshot.parent.mkdir(parents=True, exist_ok=True)
                screenshot.write_bytes(f"screenshot-{index}".encode("ascii"))
                descriptors.append(
                    {
                        **audit.file_descriptor(root, screenshot),
                        "viewport": expected["viewport"],
                        "state": expected["state"],
                    }
                )
            self.assertTrue(
                audit.visual_screenshot_attestation(root, descriptors)["passed"]
            )
            mutations = (
                lambda rows: rows[0].update(path="validation/results/screens/wrong.png"),
                lambda rows: rows[0].update(size=1),
                lambda rows: rows[0].update(sha256="0" * 64),
                lambda rows: rows[0].update(viewport="1440x900"),
                lambda rows: rows[0].update(state="other-state"),
                lambda rows: rows.append(copy.deepcopy(rows[0])),
            )
            for mutate in mutations:
                changed = copy.deepcopy(descriptors)
                mutate(changed)
                self.assertFalse(
                    audit.visual_screenshot_attestation(root, changed)["passed"]
                )

    def test_packaged_results_recompute_visible_plus_one_rows_and_run_identity(self) -> None:
        valid = self.packaged_results()
        result = audit.packaged_results_contract(valid)
        self.assertTrue(result["passed"])
        for mutate in (
            lambda value: value["rows"].reverse(),
            lambda value: value["rows"][0].__setitem__(4, "0.15"),
            lambda value: value["rows"][0].__setitem__(2, "1500.0"),
            lambda value: value["runDetails"]["properties"].__setitem__("Recipe", ""),
            lambda value: value["runDetails"]["properties"].__setitem__("Recorded seed", "1"),
        ):
            mutation = copy.deepcopy(valid)
            mutate(mutation)
            self.assertFalse(audit.packaged_results_contract(mutation)["passed"])

    def test_packaged_cancellation_binds_terminal_cancel_archive_and_retry_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            before_path, before_descriptor = self.write_cancellation_snapshot(root, "before")
            _, after_descriptor = self.write_cancellation_snapshot(root, "after", before_path)

            def boundary(phase: str, descriptor: dict[str, object]) -> dict[str, object]:
                return {
                    "phase": phase,
                    "artifact": copy.deepcopy(descriptor),
                    "sourcePath": "validation/results/v247-native-structural-path-randomization-1-2.qpls",
                    "sourceSize": descriptor["size"],
                    "sourceSha256": descriptor["sha256"],
                    "sourceMtimeNsBefore": "100",
                    "sourceMtimeNsAfter": "100",
                    "sourceStableDuringCopy": True,
                    "snapshotMatchesSource": True,
                    "datasetCount": 1,
                    "modelCount": 1,
                    "modelName": "Structural Path Randomization Model",
                    "constructLabels": ["X", "Z", "Y"],
                    "pathLabels": ["X -> Y", "Z -> Y"],
                    "recipeCount": 0,
                    "resultCount": 0,
                    "runCount": 0,
                    "recipeIds": [],
                    "resultIds": [],
                    "runIds": [],
                }

            def setup(workers: int, start_label: str) -> dict[str, object]:
                return {
                    "catalogCount": 15,
                    "selectedMethod": "Structural Path Randomization",
                    "permutations": {
                        "count": 1, "type": "number", "minimum": "99",
                        "maximum": "10000", "step": "1", "value": "10000",
                    },
                    "workers": {
                        "count": 1, "type": "number", "minimum": "1",
                        "maximum": "64", "value": str(workers),
                    },
                    "seed": {
                        "count": 1, "type": "number", "minimum": "0",
                        "maximum": "4294967295", "value": "20260718",
                    },
                    "bootstrapControls": 0,
                    "groupControls": 0,
                    "scopeLabel": "Candidate scope",
                    "scope": audit.PACKAGED_WARNING,
                    "blockers": [],
                    "startLabel": start_label,
                    "startEnabled": True,
                }

            valid = {
                "passed": True,
                "activeLifecycleCaptured": True,
                "activeState": {
                    "ariaBusy": "true", "status": "queued", "phase": "Queued",
                    "message": "Native engine accepted the calculation job.",
                    "progressValue": "0", "progressMax": "100", "logEntries": 3,
                },
                "cancelButtonCount": 1,
                "cancelButtonEnabled": True,
                "cancelClickDispatched": True,
                "terminalOutcome": "cancelled",
                "completionWonRace": False,
                "cancelledState": {
                    "ariaBusy": "false", "status": "cancelled", "phase": "Cancelled",
                    "message": "Calculation cancelled.",
                    "progressValue": "0", "progressMax": "100", "logEntries": 5,
                    "logMessages": ["Calculation cancelled.", "Cancellation requested.", "Running", "Queued"],
                },
                "cancelledMessage": "Calculation cancelled.",
                "cancellationLogNewest": ["Calculation cancelled.", "Cancellation requested."],
                "cancellationLogExact": True,
                "archiveBeforeSnapshot": boundary("before", before_descriptor),
                "archiveAfterSnapshot": boundary("after", after_descriptor),
                "archiveSnapshotsByteIdentical": True,
                "zeroResultRecipeRunDelta": True,
                "noPartialResult": True,
                "cancellationSetup": setup(1, "Start path randomization"),
                "exactFrozenSetupOnRetry": True,
                "retrySetup": setup(1, "Retry path randomization"),
                "completionSetup": setup(4, "Retry path randomization"),
                "completionSetupExact": True,
                "retryCompleted": True,
                "retryRunId": "run-1",
                "retryNewIdentity": True,
            }
            artifacts = {
                "cancellation_archive_before": copy.deepcopy(before_descriptor),
                "cancellation_archive_after": copy.deepcopy(after_descriptor),
            }
            self.assertTrue(
                audit.packaged_cancellation_contract(
                    valid, copy.deepcopy(valid), artifacts, root
                )["passed"]
            )
            mutations = (
                lambda packaged, source, top: packaged.update(cancelButtonEnabled=False),
                lambda packaged, source, top: packaged.update(terminalOutcome="completed"),
                lambda packaged, source, top: packaged.update(completionWonRace=True),
                lambda packaged, source, top: packaged["activeState"].update(status="completed"),
                lambda packaged, source, top: packaged["cancelledState"].update(status="completed"),
                lambda packaged, source, top: packaged["archiveAfterSnapshot"]["artifact"].update(sha256="b" * 64),
                lambda packaged, source, top: top["cancellation_archive_after"].update(path=top["cancellation_archive_before"]["path"]),
                lambda packaged, source, top: packaged.update(archiveSnapshotsByteIdentical=False),
                lambda packaged, source, top: packaged["cancellationSetup"]["workers"].update(value="4"),
                lambda packaged, source, top: packaged["retrySetup"]["workers"].update(value="4"),
                lambda packaged, source, top: packaged["completionSetup"]["workers"].update(value="1"),
                lambda packaged, source, top: packaged.update(completionSetupExact=False),
                lambda packaged, source, top: packaged.update(retryNewIdentity=False),
                lambda packaged, source, top: source["cancelledState"].update(message="different"),
                lambda packaged, source, top: packaged.update(extra=True),
            )
            for mutate in mutations:
                with self.subTest(mutate=mutate):
                    packaged = copy.deepcopy(valid)
                    source = copy.deepcopy(valid)
                    top = copy.deepcopy(artifacts)
                    mutate(packaged, source, top)
                    self.assertFalse(
                        audit.packaged_cancellation_contract(packaged, source, top, root)["passed"]
                    )

            before_path.write_bytes(b"tampered snapshot")
            self.assertFalse(
                audit.packaged_cancellation_contract(
                    valid, copy.deepcopy(valid), artifacts, root
                )["passed"]
            )

    def test_packaged_export_binds_exact_disclosures_to_verified_workbook(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            xlsx = root / "validation" / "results" / "result.xlsx"
            xlsx.parent.mkdir(parents=True)
            xlsx.write_bytes(b"genuine workbook bytes")
            descriptor = audit.file_descriptor(root, xlsx)
            absolute = str(xlsx.resolve())
            valid = {
                "passed": True,
                "expectedSheets": ["Structural path randomization", "Run provenance"],
                "expectedSharedStrings": list(audit.PACKAGED_SHARED_STRINGS),
                "nativeXlsx": {
                    "targetPath": absolute,
                    "file": {"size": descriptor["size"], "isFile": True},
                    "workbookSheets": list(audit.PACKAGED_WORKBOOK_SHEETS),
                    "exactRequiredSheets": True,
                    "noBootstrapSheets": True,
                    "helper": {
                        "ready": {"targetPath": absolute},
                        "completion": {
                            "passed": True,
                            "targetPath": absolute,
                            "workbook": {
                                "path": absolute,
                                "size": descriptor["size"],
                                "sha256": descriptor["sha256"],
                                "sheetNames": list(audit.PACKAGED_WORKBOOK_SHEETS),
                                "requiredSharedStrings": list(audit.PACKAGED_SHARED_STRINGS),
                                "requiredSheets": ["Structural path randomization", "Run provenance"],
                            },
                        },
                    },
                },
            }
            self.assertTrue(
                audit.packaged_export_contract(valid, descriptor, root)["passed"]
            )
            mutations = (
                lambda value, artifact: value["expectedSharedStrings"].remove(
                    audit.PACKAGED_PROBABILITY_DISCLOSURE
                ),
                lambda value, artifact: value["nativeXlsx"]["helper"]["completion"][
                    "workbook"
                ]["requiredSharedStrings"].remove(audit.PACKAGED_QUALIFICATION_DISCLOSURE),
                lambda value, artifact: value["nativeXlsx"].update(
                    targetPath=str(root / "validation" / "results" / "other.xlsx")
                ),
                lambda value, artifact: value["nativeXlsx"]["file"].update(size=1),
                lambda value, artifact: value["nativeXlsx"]["workbookSheets"].reverse(),
                lambda value, artifact: value["nativeXlsx"]["helper"]["completion"][
                    "workbook"
                ]["sheetNames"].reverse(),
                lambda value, artifact: artifact.update(sha256="0" * 64),
            )
            for mutate in mutations:
                value = copy.deepcopy(valid)
                artifact = copy.deepcopy(descriptor)
                mutate(value, artifact)
                self.assertFalse(
                    audit.packaged_export_contract(value, artifact, root)["passed"]
                )

    def test_packaged_archive_recomputes_parameter_order_arithmetic_and_ui_binding(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive, descriptor = self.write_packaged_archive(
                root, "v247-native-structural-path-randomization-1-2.qpls"
            )
            results = audit.packaged_results_contract(self.packaged_results())
            inspection = audit.inspect_packaged_archive_artifact(root, descriptor)
            self.assertTrue(inspection["passed"], inspection["errors"])
            self.assertEqual(
                inspection["contract"]["permutation"]["exceedances"], [1500, 500]
            )
            valid = {"passed": True, **copy.deepcopy(inspection["contract"])}
            self.assertTrue(
                audit.packaged_archive_contract(
                    valid, results["rows"], descriptor, root
                )["passed"]
            )
            mutations = (
                lambda value, artifact: value["permutation"]["parameterIds"].reverse(),
                lambda value, artifact: value["permutation"]["pValues"].__setitem__(0, 0.5),
                lambda value, artifact: value["resultPathIds"].reverse(),
                lambda value, artifact: value["constructs"].reverse(),
                lambda value, artifact: artifact.update(path="validation/results/other.qpls"),
                lambda value, artifact: artifact.update(size=1),
                lambda value, artifact: artifact.update(sha256="0" * 64),
            )
            for mutate in mutations:
                value = copy.deepcopy(valid)
                artifact = copy.deepcopy(descriptor)
                mutate(value, artifact)
                self.assertFalse(
                    audit.packaged_archive_contract(
                        value, results["rows"], artifact, root
                    )["passed"]
                )

            swapped_archive, swapped_descriptor = self.write_packaged_archive(
                root,
                "v247-native-structural-path-randomization-3-4.qpls",
                exceedances=(1600, 600),
            )
            self.assertTrue(swapped_archive.is_file())
            self.assertFalse(
                audit.packaged_archive_contract(
                    valid, results["rows"], swapped_descriptor, root
                )["passed"]
            )

            for wrong_version in (0, 2, 3, 4, audit.PROJECT_ARCHIVE_SCHEMA_VERSION):
                _, wrong_descriptor = self.write_packaged_archive(
                    root,
                    f"wrong-result-schema-{wrong_version}.qpls",
                    result_schema_version=wrong_version,
                )
                wrong_inspection = audit.inspect_packaged_archive_artifact(
                    root, wrong_descriptor
                )
                with self.subTest(wrong_version=wrong_version):
                    self.assertFalse(wrong_inspection["passed"])

            with zipfile.ZipFile(archive, "r") as source:
                entries = {name: source.read(name) for name in source.namelist()}
            project = json.loads(entries["project.json"])
            project["results"][0]["payload"]["permutation"]["parameters"][0][
                "exceedances"
            ] = 9999
            entries["project.json"] = json.dumps(project, indent=2).encode("utf-8")
            with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as output:
                for name, contents in entries.items():
                    output.writestr(name, contents)
            tampered_descriptor = audit.file_descriptor(root, archive)
            tampered = audit.inspect_packaged_archive_artifact(root, tampered_descriptor)
            self.assertFalse(tampered["passed"])
            self.assertTrue(
                any("checksum mismatch for project.json" in error for error in tampered["errors"])
            )

    def test_artifact_attestation_rehashes_and_rejects_path_or_digest_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            artifact = root / "evidence.bin"
            artifact.write_bytes(b"dedicated randomization evidence")
            descriptor = {
                "path": "evidence.bin",
                "size": artifact.stat().st_size,
                "sha256": hashlib.sha256(artifact.read_bytes()).hexdigest(),
            }
            self.assertTrue(audit.artifact_attestation(root, descriptor)["passed"])
            wrong_digest = dict(descriptor, sha256="0" * 64)
            self.assertFalse(audit.artifact_attestation(root, wrong_digest)["passed"])
            escaped = dict(descriptor, path="../evidence.bin")
            self.assertFalse(audit.artifact_attestation(root, escaped)["passed"])

    def test_broad_inference_gates_bind_only_dedicated_randomization_evidence(self) -> None:
        row = next(
            procedure
            for procedure in broad_publication.PROCEDURES
            if procedure[0] == "freedman_lane_permutation"
        )
        evidence_paths = row[2:6]
        self.assertTrue(
            all("structural_path_randomization" in path for path in evidence_paths)
        )
        self.assertTrue(all("bootstrap" not in path for path in evidence_paths))
        self.assertEqual(
            broad_promotion.REQUIRED["structural_randomization_promotion"],
            audit.OUTPUT.relative_to(audit.ROOT).as_posix(),
        )

    def test_broad_compatibility_contract_rejects_scope_or_status_drift(self) -> None:
        compatibility = (audit.ROOT / "docs" / "METHOD_COMPATIBILITY.md").read_text(
            encoding="utf-8"
        )
        self.assertTrue(
            broad_promotion.method_compatibility_matches_current_scope(compatibility)
        )
        self.assertFalse(
            broad_promotion.method_compatibility_matches_current_scope(
                compatibility.replace(
                    "exchangeable reduced-model residuals",
                    "arbitrary residuals",
                    1,
                )
            )
        )
        self.assertFalse(
            broad_promotion.method_compatibility_matches_current_scope(
                compatibility.replace(
                    "release-qualified bounded v1 evidence",
                    "candidate bounded v1 evidence",
                    1,
                )
            )
        )

    def test_product_enforcement_rejects_validated_label_and_identity_drift(self) -> None:
        sources = product_enforcement.load_product_contract_sources(audit.ROOT)
        checks = {
            row["name"]: row
            for row in product_enforcement.product_contract_checks(sources)
        }
        self.assertTrue(all(row["passed"] for row in checks.values()))

        catalog_mutation = dict(sources)
        catalog_mutation["src/data/sample.ts"] = catalog_mutation[
            "src/data/sample.ts"
        ].replace(
            'name: "Freedman-Lane permutation", status: "experimental"',
            'name: "Freedman-Lane permutation", status: "validated"',
            1,
        )
        mutated_checks = {
            row["name"]: row
            for row in product_enforcement.product_contract_checks(catalog_mutation)
        }
        self.assertFalse(
            mutated_checks[
                "catalog_separates_bootstrap_from_structural_randomization"
            ]["passed"]
        )

        result_status_mutation = dict(sources)
        result_status_mutation["src/domain/resultTables.ts"] = (
            result_status_mutation["src/domain/resultTables.ts"].replace(
                'const runStatus = structuralPathRandomization ? "experimental" : resultScopeStatus(run.result);',
                'const runStatus = structuralPathRandomization ? "validated" : resultScopeStatus(run.result);',
                1,
            )
        )
        mutated_checks = {
            row["name"]: row
            for row in product_enforcement.product_contract_checks(
                result_status_mutation
            )
        }
        self.assertFalse(
            mutated_checks[
                "result_tables_fail_closed_by_exact_method_identity"
            ]["passed"]
        )

        compatibility_mutation = dict(sources)
        compatibility_mutation["docs/METHOD_COMPATIBILITY.md"] = (
            compatibility_mutation["docs/METHOD_COMPATIBILITY.md"].replace(
                "release-qualified bounded v1 evidence with an explicit conditional/approximate interpretation warning",
                "candidate bounded v1 evidence",
                1,
            )
        )
        mutated_checks = {
            row["name"]: row
            for row in product_enforcement.product_contract_checks(
                compatibility_mutation
            )
        }
        self.assertFalse(
            mutated_checks["compatibility_docs_match_promoted_scope"]["passed"]
        )

    def test_broad_sources_defer_to_dedicated_randomization_qualification(self) -> None:
        product_enforcement = (audit.ROOT / "validation" / "method_promotion_product_enforcement_audit.py").read_text(encoding="utf-8")
        registry = json.loads((audit.ROOT / "validation" / "development_slices.json").read_text(encoding="utf-8"))
        promotion_source = inspect.getsource(broad_publication)
        matrix_source = (audit.ROOT / "validation" / "promotion_matrix.py").read_text(encoding="utf-8")
        mediation_source = (audit.ROOT / "validation" / "mediation_method_promotion_audit.py").read_text(encoding="utf-8")
        moderation_source = (audit.ROOT / "validation" / "moderation_method_promotion_audit.py").read_text(encoding="utf-8")
        inference_slice = next(row for row in registry["slices"] if row["id"] == "v0_4_inference_resampling")

        self.assertIn('name: "Freedman-Lane permutation", status: "experimental"', product_enforcement)
        self.assertNotIn('name: "Freedman-Lane permutation", status: "validated"', product_enforcement)
        self.assertNotIn("permutation", inference_slice["name"].casefold())
        self.assertIn(
            "release-qualified only through validation/results/structural_path_randomization_method_promotion_audit.json",
            inference_slice["summary"],
        )
        self.assertIn("generic bootstrap evidence cannot substitute", inference_slice["summary"])
        self.assertIn(
            "Structural Path Randomization is release-qualified only through validation/results/structural_path_randomization_method_promotion_audit.json",
            matrix_source,
        )
        self.assertIn("generic bootstrap evidence is not reusable", matrix_source)
        self.assertIn("does not qualify Structural Path Randomization", mediation_source)
        self.assertIn("does not qualify Structural Path Randomization", moderation_source)
        self.assertIn("structural_path_randomization_method_promotion_audit.json", promotion_source)

    def test_promoted_scope_discloses_exchangeability_and_calibration_boundary(self) -> None:
        source = inspect.getsource(audit.audit)
        self.assertIn("exchangeable reduced-model residuals", source)
        self.assertIn("homoscedastic Gaussian-error scenarios", source)
        self.assertIn("does not qualify heteroskedastic or broader non-Gaussian validity", source)


if __name__ == "__main__":
    unittest.main()
