#!/usr/bin/env python3
"""Fail-closed tests for graph-defined PROCESS v2 promotion evidence."""

from __future__ import annotations

import hashlib
import json
import os
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from xml.sax.saxutils import escape, quoteattr

from jsonschema import Draft202012Validator


VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

import process_v2_method_promotion_audit as audit  # noqa: E402
import process_v2_resource_policy_v3 as resource_policy  # noqa: E402
import process_v2_boundary_gate as boundary_gate  # noqa: E402
import process_v2_reference as process_reference  # noqa: E402


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


def artifact(path: Path, root: Path, contents: bytes | None = None) -> dict[str, object]:
    path.parent.mkdir(parents=True, exist_ok=True)
    if contents is not None:
        path.write_bytes(contents)
    data = path.read_bytes()
    return {
        "path": path.relative_to(root).as_posix(),
        "size": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
    }


def make_xlsx(path: Path, *, run_provenance_warning: str | None = None) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    strings: list[str] = []
    indexes: dict[str, int] = {}

    def shared(value: str) -> int:
        if value not in indexes:
            indexes[value] = len(strings)
            strings.append(value)
        return indexes[value]

    def cell(reference: str, value: str) -> str:
        return f'<c r={quoteattr(reference)} t="s"><v>{shared(value)}</v></c>'

    relationships = []
    worksheets = []
    sheet_rows = []
    for index, name in enumerate(audit.EXPECTED_WORKBOOK_SHEETS, 1):
        relation_id = f"rId{index}"
        relationships.append(
            f'<Relationship Id={quoteattr(relation_id)} '
            f'Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" '
            f'Target={quoteattr(f"worksheets/sheet{index}.xml")}/>'
        )
        worksheets.append(
            f'<sheet name={quoteattr(name)} sheetId={quoteattr(str(index))} r:id={quoteattr(relation_id)}/>'
        )
        warning = run_provenance_warning if name == "Run provenance" else None
        if name == "Johnson-Neyman curve data":
            warning = audit.PROCESS_CURVE_WARNING_DISCLOSURE
        warning_row = f'<row r="3">{cell("A3", "Warning")}'
        if warning is not None:
            warning_row += cell("B3", warning)
        warning_row += "</row>"
        rows = [
            f'<row r="1">{cell("A1", name)}</row>',
            f'<row r="2">{cell("A2", "Status")}{cell("B2", "validated")}</row>',
            warning_row,
        ]
        if name == "Reference effects":
            headers = ["Effect ID", "Kind", "Path", "Estimate", "Reference condition"]
            rows.append('<row r="5">' + "".join(
                cell(f"{column}5", header)
                for column, header in zip(("A", "B", "C", "D", "E"), headers)
            ) + "</row>")
            for row_number in range(6, 12):
                rows.append(
                    f'<row r="{row_number}">'
                    f'{cell(f"A{row_number}", f"effect-{row_number - 5}")}'
                    f'{cell(f"B{row_number}", "Direct")}'
                    f'{cell(f"C{row_number}", "X to Y")}'
                    f'{cell(f"D{row_number}", "0.100000")}'
                    f'{cell(f"E{row_number}", audit.REFERENCE_CONDITION)}'
                    "</row>"
                )
        else:
            rows.append(f'<row r="5">{cell("A5", "Field")}{cell("B5", "Value")}</row>')
            rows.append(f'<row r="6">{cell("A6", "Fixture")}{cell("B6", "Value")}</row>')
        sheet_rows.append(
            '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
            f'<sheetData>{"".join(rows)}</sheetData></worksheet>'
        )
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as workbook:
        workbook.writestr("[Content_Types].xml", "<Types/>")
        workbook.writestr(
            "xl/workbook.xml",
            '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" '
            'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">'
            f'<sheets>{"".join(worksheets)}</sheets></workbook>',
        )
        workbook.writestr(
            "xl/_rels/workbook.xml.rels",
            '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">'
            f'{"".join(relationships)}</Relationships>',
        )
        workbook.writestr(
            "xl/sharedStrings.xml",
            '<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">'
            + "".join(f"<si><t>{escape(value)}</t></si>" for value in strings)
            + "</sst>",
        )
        for index, worksheet in enumerate(sheet_rows, 1):
            workbook.writestr(f"xl/worksheets/sheet{index}.xml", worksheet)


def make_project(path: Path) -> None:
    families = {
        "reference_effects": audit.EXPECTED_ESTIMAND_IDS[:6],
        "conditional_indirect_effects": audit.EXPECTED_ESTIMAND_IDS[6:11],
        "moderated_mediation_indices": audit.EXPECTED_ESTIMAND_IDS[11:13],
        "simple_slopes": audit.EXPECTED_ESTIMAND_IDS[13:],
    }
    estimand_ids = [effect for rows in families.values() for effect in rows]
    graph = {
        **{name: [{"effect_id": effect} for effect in values] for name, values in families.items()},
        "policies": {
            "centering": "equation_complete_case_mean_v1", "covariance": "hc3_v1",
            "inference_reference": "student_t_residual_df_v1", "confidence_level": 0.95,
        },
        "complete_cases": 175, "omitted_cases": 5,
        "paths": [{} for _ in range(8)], "moderations": [{} for _ in range(3)],
        "equations": [{} for _ in range(5)],
        "plots": [
            {"series": [{"points": [{} for _ in range(25)]} for _ in range(series)]}
            for series in (3, 6, 2)
        ],
        "johnson_neyman": [
            *({"status": "available", "curve_points": [{} for _ in range(101)]} for _ in range(3)),
            {"status": "unavailable"},
        ],
        "bootstrap": {
            "method_version": audit.BOOTSTRAP_METHOD_VERSION,
            "requested_replicates": 10_000,
            "usable_replicates": 9_000,
            "algorithm": "indexed_case_resampling_v1",
            "interval_policy": "percentile_primary_bca_conditional_v1",
            "test_reference": "standard_normal_bootstrap_ratio_v1",
            "stream_token": "process_indexed_case_stream_v1",
            "seed": 20_260_812, "workers": 2,
            "jackknife_cases": 175,
            "usable_jackknife_cases": 175,
            "failed_replicates": [
                {"replicate_index": index, "reason_code": "rank_deficient_equation", "message": "fixture"}
                for index in range(9_000, 10_000)
            ],
            "estimands": [{"effect_id": effect} for effect in estimand_ids],
            "validation_witness": {
                "method_version": audit.WITNESS_VERSION,
                "estimand_ids": estimand_ids,
                "successful_bootstrap": [
                    {"replicate_index": index, "estimates": [0.0] * 24}
                    for index in range(9_000)
                ],
                "successful_jackknife": [
                    {"omitted_case": index, "estimates": [0.0] * 24}
                    for index in range(175)
                ],
                "failed_jackknife": [],
            },
        },
    }
    project = {
        "results": [{
            "id": "process-run-1",
            "status": "completed",
            "provenance": {
                "method": "regression",
                "method_version": f"{audit.METHOD_VERSION}+{audit.BOOTSTRAP_METHOD_VERSION}",
                "recipe_id": "recipe-process-v2",
            },
            "payload": {"estimation": {"method_version": audit.METHOD_VERSION, "regression": {
                "method_version": audit.METHOD_VERSION, "regression_type": "process",
                "outcome": "Y", "predictors": ["X", "M1", "M2", "M3", "M4", "W", "B"],
                "controls": ["C"], "observations": 175,
                "coefficients": [], "fit": None, "predictions": [],
                "process": {
                    "method_version": audit.METHOD_VERSION, "model": "graph",
                    "effects": [], "simple_slopes": [], "graph_v2": graph,
                },
            }}},
        }],
        "recipes": [{
            "id": "recipe-process-v2", "schema_version": 3,
            "metadata": {"status": "validated_regression_process_v2_plus_bootstrap_v1_bounded_scope"},
        }], "models": [],
        "layouts": {"workspace": {
            "runs": [{"id": "process-run-1", "status": "completed"}],
            "diagramOverlaySettings": {"selectedRunId": "process-run-1"},
        }},
    }
    project_bytes = json.dumps(project).encode()
    manifest = {
        "schema_version": 5, "checksum_algorithm": "sha256",
        "checksums": {"project.json": hashlib.sha256(project_bytes).hexdigest()},
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("project.json", project_bytes)
        archive.writestr("manifest.json", json.dumps(manifest))


def make_resource_project(
    source: Path,
    target: Path,
    *,
    result_count: int,
    selected_run_id: str | None,
) -> None:
    with zipfile.ZipFile(source) as archive:
        project = json.loads(archive.read("project.json").decode("utf-8"))
    template = project["results"][0]
    results = []
    for index in range(result_count):
        row = json.loads(json.dumps(template))
        row["id"] = f"process-run-{index + 1}"
        results.append(row)
    project["results"] = results
    project["recipes"] = project["recipes"] if result_count else []
    project["layouts"] = {"workspace": {
        "runs": [{"id": row["id"], "status": "completed"} for row in results],
        "diagramOverlaySettings": {"selectedRunId": selected_run_id},
    }}
    project_bytes = json.dumps(project).encode()
    manifest = {
        "schema_version": 5, "checksum_algorithm": "sha256",
        "checksums": {"project.json": hashlib.sha256(project_bytes).hexdigest()},
    }
    target.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(target, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("project.json", project_bytes)
        archive.writestr("manifest.json", json.dumps(manifest))


class ProcessV2PromotionAuditTests(unittest.TestCase):
    def test_reference_generator_and_promotion_share_exact_check_manifest(self) -> None:
        self.assertEqual(process_reference.REFERENCE_CHECK_NAMES, audit.REFERENCE_CHECKS)

    def test_reference_requires_exclusive_process_shell_without_legacy_analyses(self) -> None:
        source = (VALIDATION / "process_v2_reference.py").read_text(encoding="utf-8")
        check_start = source.index('"dedicated_graph_result_not_generic_regression": (')
        check_end = source.index('"legacy_v1_not_current_evidence":', check_start)
        dedicated = source[check_start:check_end]
        self.assertIn('estimation.get("mediation") is None', dedicated)
        self.assertIn('estimation.get("moderation") is None', dedicated)
        self.assertIn('"bootstrap" not in regression', dedicated)
        harness = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        archive_start = harness.index("async function inspectSavedProcessV2Archive")
        archive_end = harness.index("async function", archive_start + 20)
        archive_inspector = harness[archive_start:archive_end]
        self.assertIn('!("mediation" in estimation)', archive_inspector)
        self.assertIn('!("moderation" in estimation)', archive_inspector)

    def test_packaged_harness_separates_jn_analyses_regions_and_safe_warning_prose(self) -> None:
        harness = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        self.assertIn("johnsonNeymanRegionRows", harness)
        self.assertIn("processV2PrivateWitnessWireToken.test(renderedPrivateDataText)", harness)
        self.assertNotIn("processV2PrivateWitnessWireToken.test(renderedText)", harness)
        self.assertIn("curveWarningDisclosureExact", harness)
        self.assertIn("genericRegressionShellNotApplicable: null", harness)
        process_acceptance = harness[harness.index("async function runFocusedProcessV2Acceptance"):]
        shared_strings_start = process_acceptance.index("const expectedSharedStrings = [")
        shared_strings_end = process_acceptance.index("];", shared_strings_start)
        expected_shared_strings = process_acceptance[shared_strings_start:shared_strings_end]
        self.assertIn('"Graph-Defined Path Analysis with Bootstrap"', expected_shared_strings)
        self.assertIn(
            '"Original-sample raw moderator probes; each resample and delete-one equation '
            're-centered internally"',
            expected_shared_strings,
        )
        self.assertNotIn('"Graph-defined path analysis"', expected_shared_strings)
        self.assertNotIn('"Original-sample raw moderator probes"', expected_shared_strings)

    def test_packaged_archive_policy_contract_is_order_insensitive_but_exact(self) -> None:
        harness = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        archive_start = harness.index("async function inspectSavedProcessV2Archive")
        archive_end = harness.index("async function", archive_start + 20)
        archive_inspector = harness[archive_start:archive_end]
        self.assertIn("processV2PoliciesExact(contract.policies)", archive_inspector)
        self.assertNotIn("JSON.stringify(contract.policies)", archive_inspector)

        keys_start = harness.index("const processV2PolicyKeys =")
        keys_end = harness.index("];", keys_start) + 2
        helper_start = harness.index("function processV2PoliciesExact")
        helper_end = harness.index("\n}\n", helper_start) + 3
        helper = f"{harness[keys_start:keys_end]}\n{harness[helper_start:helper_end]}"
        mutations = {
            "canonical": {
                "centering": "equation_complete_case_mean_v1", "confidence_level": 0.95,
                "covariance": "hc3_v1", "inference_reference": "student_t_residual_df_v1",
            },
            "reordered": {
                "inference_reference": "student_t_residual_df_v1", "covariance": "hc3_v1",
                "confidence_level": 0.95, "centering": "equation_complete_case_mean_v1",
            },
            "missing": {
                "centering": "equation_complete_case_mean_v1", "confidence_level": 0.95,
                "covariance": "hc3_v1",
            },
            "extra": {
                "centering": "equation_complete_case_mean_v1", "confidence_level": 0.95,
                "covariance": "hc3_v1", "inference_reference": "student_t_residual_df_v1",
                "unexpected": True,
            },
            "wrong": {
                "centering": "equation_complete_case_mean_v1", "confidence_level": 0.90,
                "covariance": "hc3_v1", "inference_reference": "student_t_residual_df_v1",
            },
        }
        script = (
            f"{helper}\nconst rows = {json.dumps(mutations)};\n"
            "const observed = Object.fromEntries(Object.entries(rows).map(([name, value]) => "
            "[name, processV2PoliciesExact(value)]));\n"
            "const expected = {canonical:true,reordered:true,missing:false,extra:false,wrong:false};\n"
            "if (JSON.stringify(observed) !== JSON.stringify(expected)) { "
            "throw new Error(JSON.stringify({observed, expected})); }\n"
        )
        completed = subprocess.run(
            ["node", "--input-type=module", "-e", script],
            cwd=VALIDATION.parent, capture_output=True, text=True, check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)

    def test_process_archive_identity_is_defined_and_validated_before_reset_clone_use(self) -> None:
        harness = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        inspector_start = harness.index("async function inspectSavedProcessV2Archive")
        inspector_end = harness.index("async function inspectProcessV2LogicalArchiveState", inspector_start)
        inspector = harness[inspector_start:inspector_end]
        for field in (
            "resultId: result.id ?? null",
            "recipeId: recipe?.id ?? null",
            "runId: run?.id ?? null",
            "resultCount: project.results?.length ?? null",
            "recipeCount: project.recipes?.length ?? null",
            "witnessCount: witness ? 1 : 0",
            "contract.identity.resultId === runId",
            "contract.identity.recipeId === result.provenance?.recipe_id",
            "contract.identity.runId === runId",
        ):
            self.assertIn(field, inspector)
        reset_start = harness.index("evidence.checks.processV2ResourceResetClone =")
        reset_end = harness.index("if (!evidence.checks.processV2ResourceResetClone.passed)", reset_start)
        reset = harness[reset_start:reset_end]
        self.assertIn("resetArchive.identity.resultId === archive.identity.resultId", reset)
        self.assertIn("identity: resetArchive.identity", reset)

    def test_packaged_repeated_completion_uses_exact_run_id_set_difference(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        repeated_start = source.index("const completedRunIdsBefore =")
        repeated_end = source.index("const historyDefaultTable =", repeated_start)
        repeated = source[repeated_start:repeated_end]
        self.assertIn("completedRunIdsAfter.length === completedRunIdsBefore.length + 1", repeated)
        self.assertIn("new Set(completedRunIdsAfter).size", repeated)
        self.assertIn("const addedRunIds = completedRunIdsAfter.filter", repeated)
        self.assertIn("const autoSelectedRunId = await runSelect.inputValue();", repeated)
        self.assertIn("autoSelectedRunId === repeatedRunId", repeated)
        self.assertIn("explicitlySelectedRunId === repeatedRunId", repeated)
        self.assertNotIn("repeatedRunOptions.last()", repeated)
        self.assertLess(
            repeated.index("const autoSelectedRunId = await runSelect.inputValue();"),
            repeated.index("await runSelect.selectOption(repeatedRunId)"),
        )

    def test_boundary_runner_and_promotion_share_exact_frozen_names(self) -> None:
        self.assertEqual(
            {target: frozenset(suite["tests"]) for target, suite in boundary_gate.SUITES.items()},
            audit.BOUNDARY_SUITES,
        )
        self.assertIn(
            "process_graph_v2_append_save_reopen_and_tamper_are_atomic",
            audit.BOUNDARY_SUITES["qpls_project"],
        )
        self.assertNotIn(
            "process_graph_v2_archive_append_reopen_and_tamper_are_atomic",
            audit.BOUNDARY_SUITES["qpls_project"],
        )
        self.assertIn(
            "process_graph_v2_rejects_high_leverage_hc3_instability_without_clamping",
            audit.BOUNDARY_SUITES["qpls_estimation"],
        )
        self.assertIn(
            "process_graph_v2_bootstrap_maps_high_leverage_hc3_failure",
            audit.BOUNDARY_SUITES["qpls_resampling"],
        )
        self.assertIn(
            "process_graph_v2_rejects_nonpositive_hc3_variance_without_absolute_value",
            audit.BOUNDARY_SUITES["qpls_estimation"],
        )
        self.assertIn(
            "process_graph_v2_rejects_degenerate_simple_slope_variance",
            audit.BOUNDARY_SUITES["qpls_estimation"],
        )
        self.assertIn(
            "process_graph_v2_point_progress_completes_and_cancellation_returns_no_result",
            audit.BOUNDARY_SUITES["qpls_estimation"],
        )
        self.assertIn(
            "process_graph_v2_bootstrap_maps_invalid_hc3_covariance_failure",
            audit.BOUNDARY_SUITES["qpls_resampling"],
        )
        self.assertIn(
            "process_graph_v2_bootstrap_maps_degenerate_simple_slope_failure",
            audit.BOUNDARY_SUITES["qpls_resampling"],
        )
        self.assertEqual(
            audit.BOUNDARY_SUITES["qpls_runner"],
            frozenset({
                "process_v2_point_progress_completes_and_base_fit_cancellation_returns_no_result",
                "process_v2_runner_rejects_exact_binary_endogenous_original_profiles",
            }),
        )
        self.assertIn(
            "process_graph_v2_point_is_row_irrelevant_column_and_recipe_order_invariant",
            audit.BOUNDARY_SUITES["qpls_estimation"],
        )
        for name in (
            "process_graph_v2_scale_aware_svd_is_affine_unit_invariant_and_rejects_relative_collinearity",
            "process_graph_v2_jn_root_solver_is_affine_stable_and_deduplicates_near_double_roots",
            "process_graph_v2_jn_nonpositive_contrast_variance_is_tagged_unavailable",
            "process_graph_v2_rejects_exact_binary_endogenous_outcomes_in_original_sample",
            "process_graph_v2_semantic_probe_grid_rejects_collapsed_f64_levels",
        ):
            self.assertIn(name, audit.BOUNDARY_SUITES["qpls_estimation"])
        self.assertIn(
            "process_v2_runner_rejects_exact_binary_endogenous_original_profiles",
            audit.BOUNDARY_SUITES["qpls_runner"],
        )

    def test_product_freshness_manifests_cover_transitive_build_inputs(self) -> None:
        self.assertTrue({
            "Cargo.toml", "Cargo.lock", "crates/qpls-data/Cargo.toml",
            "crates/qpls-data/src/lib.rs", "crates/qpls-cli/Cargo.toml",
            "crates/qpls-cli/src/main.rs",
        }.issubset(audit.REFERENCE_PRODUCT_SOURCES))
        self.assertTrue({
            "package.json", "package-lock.json", "src/native/NativeDesktopApp.tsx",
            "src/native/NativeExportDialog.tsx", "src-tauri/Cargo.toml",
            "src-tauri/tauri.conf.json", "src-tauri/capabilities/default.json",
        }.issubset(audit.PACKAGED_PRODUCT_SOURCES))
        self.assertTrue({
            "Cargo.toml", "Cargo.lock", "crates/qpls-core/Cargo.toml",
            "crates/qpls-core/src/contract.rs", "crates/qpls-data/Cargo.toml",
            "crates/qpls-data/src/lib.rs", "crates/qpls-estimation/Cargo.toml",
            "crates/qpls-estimation/src/pls.rs",
        }.issubset(audit.BOUNDARY_PRODUCT_SOURCES["qpls_estimation"]))
        self.assertTrue({
            "crates/qpls-core/src/contract.rs", "crates/qpls-data/src/lib.rs",
            "crates/qpls-estimation/src/pls.rs", "crates/qpls-resampling/src/lib.rs",
            "crates/qpls-project/src/lib.rs",
        }.issubset(audit.BOUNDARY_PRODUCT_SOURCES["qpls_project"]))
        self.assertTrue({
            "Cargo.toml", "Cargo.lock", "crates/qpls-core/src/contract.rs",
            "crates/qpls-data/src/lib.rs", "crates/qpls-estimation/src/pls.rs",
            "crates/qpls-resampling/src/lib.rs", "crates/qpls-runner/Cargo.toml",
            "crates/qpls-runner/src/lib.rs",
        }.issubset(audit.BOUNDARY_PRODUCT_SOURCES["qpls_runner"]))
        self.assertEqual(len(audit.FRONTEND_TEST_FILES), 19)
        self.assertIn("src/domain/resultInterpretation.test.ts", audit.FRONTEND_TEST_FILES)
        self.assertIn("src/components/ReportsWorkspace.test.ts", audit.FRONTEND_TEST_FILES)
        self.assertIn("src/components/accessibilityContracts.test.ts", audit.FRONTEND_TEST_FILES)
        self.assertTrue(set(audit.FRONTEND_TEST_FILES).issubset(audit.FRONTEND_GATE_SOURCES))
        self.assertTrue(set(audit.FRONTEND_TEST_FILES).issubset(audit.FRONTEND_TYPESCRIPT_SOURCES))
        self.assertIn("src/native/nativeProcessTestFixture.ts", audit.FRONTEND_GATE_SOURCES)
        self.assertIn("validation/r_runtime.py", audit.REFERENCE_SOURCES)
        self.assertIn("validation/lib/v2_ui_smoke_harness.mjs", audit.VISUAL_SOURCES)
        self.assertTrue({
            "validation/windows_native_save_export.py",
            "validation/close_tauri_test_window.mjs",
            "validation/process_v2_resource_policy_v3.py",
        }.issubset(audit.PACKAGED_SOURCES))

    def prepare_complete_evidence(self, root: Path) -> Path:
        results = root / "validation/results"
        results.mkdir(parents=True, exist_ok=True)

        all_sources = sorted(set([
            *audit.REFERENCE_SOURCES, *audit.VISUAL_SOURCES, *audit.PACKAGED_SOURCES,
            *audit.FRONTEND_GATE_SOURCES,
            "validation/process_v2_boundary_gate.py",
        ]))
        for relative in all_sources:
            source = root / relative
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text(f"source:{relative}", encoding="utf-8")
        (root / "validation/process_v2_packaged_acceptance.schema.json").write_text(
            (VALIDATION / "process_v2_packaged_acceptance.schema.json").read_text(encoding="utf-8"),
            encoding="utf-8",
        )
        (root / "crates/qpls-resampling/src/lib.rs").write_text(
            " ".join([
                "summarize_process_bootstrap_estimands", "zero_bootstrap_standard_error",
                "incomplete_jackknife", "zero_jackknife_variance",
                "nonfinite_adjusted_probability", audit.WITNESS_VERSION,
                "high_leverage_hc3_instability", "invalid_hc3_covariance",
                "degenerate_simple_slope_variance", "process_indexed_case_stream_v1",
            ]),
            encoding="utf-8",
        )
        (root / "crates/qpls-estimation/src/pls.rs").write_text(
            " ".join([
                "process_bootstrap_estimands_at_reference",
                "equation_complete_case_mean_v1", "hc3_v1", "student_t_residual_df_v1",
                "high_leverage_hc3_instability",
                "invalid_hc3_covariance", "degenerate_simple_slope_variance",
                "process_scale_aware_ols", "PROCESS_RELATIVE_RANK_TOLERANCE_MULTIPLIER",
                "process_johnson_neyman_coded_roots",
                "PROCESS_JN_COEFFICIENT_TOLERANCE_MULTIPLIER",
                "PROCESS_JN_ROOT_DEDUP_TOLERANCE_MULTIPLIER",
                "binary_process_equation_outcome", "collapsed_process_probe_grid",
                audit.JN_INVALID_COVARIANCE_MESSAGE,
            ]),
            encoding="utf-8",
        )

        method_doc = root / "docs/methods/PROCESS_V2.md"
        method_doc.parent.mkdir(parents=True, exist_ok=True)
        method_doc.write_text(
            " ".join([
                audit.METHOD_VERSION, audit.BOOTSTRAP_METHOD_VERSION, "graph-defined",
                "numbered PROCESS templates", "HC3", "Johnson-Neyman", "percentile", "BCa",
                "listwise", "Binary/logistic outcomes", "weights", "clusters", "studentized",
                "regression_process_v1", "high_leverage_hc3_instability",
                "invalid_hc3_covariance", "degenerate_simple_slope_variance",
                "thin SVD", "64 * machine_epsilon", "128 * machine_epsilon",
                "three distinct finite", "original complete-sample raw mean",
                "Supported in Standard", "terminally stable process roles",
                *audit.DOCUMENTATION_REFERENCE_IDENTIFIERS,
            ]),
            encoding="utf-8",
        )

        dist = root / "dist"
        (dist / "assets").mkdir(parents=True, exist_ok=True)
        (dist / "index.html").write_text("<main>QuickPLS</main>", encoding="utf-8")
        (dist / "assets/app.js").write_text("console.log('process-v2')", encoding="utf-8")
        (dist / "assets/app.css").write_text("body{margin:0}", encoding="utf-8")

        cli = artifact(root / "target/release/qpls.exe", root, b"release-cli")
        desktop = artifact(root / "target/release/quickpls-desktop.exe", root, b"release-desktop")
        boundary_executables = {
            target: artifact(root / f"target/release/deps/{target}-deadbeef.exe", root, f"test:{target}".encode())
            for target in audit.BOUNDARY_SUITES
        }

        fixture = artifact(results / "process_v2_reference_fixture.csv", root, b"X,Y\n1,2\n")
        r_script = artifact(root / "validation/process_v2_reference.R", root)
        reference = {
            "schema_version": 1,
            "target": "process_v2_independent_reference",
            "feature_id": audit.FEATURE_ID,
            "method_version": audit.METHOD_VERSION,
            "bootstrap_method_version": audit.BOOTSTRAP_METHOD_VERSION,
            "catalogue_snapshot_date": audit.CATALOGUE_SNAPSHOT_DATE,
            "passed": True,
            "checks": {name: True for name in audit.REFERENCE_CHECKS},
            "scope": {
                "equation_solver": {
                    "algorithm": "scale_aware_thin_svd_v1",
                    "normalization": "non_intercept_welford_mean_population_rms_v1",
                    "rank_tolerance_multiplier": 100.0,
                },
                "johnson_neyman_solver": {
                    "domain_normalization": "coded_range_to_minus_one_plus_one_v1",
                    "coefficient_tolerance_multiplier": 64.0,
                    "root_deduplication_tolerance_multiplier": 128.0,
                    "invalid_covariance_reason": "invalid_hc3_covariance",
                    "invalid_covariance_message": audit.JN_INVALID_COVARIANCE_MESSAGE,
                },
                "reference_condition": audit.REFERENCE_CONDITION,
                "binary_endogenous_outcome_policy": "reject_exact_0_1_in_original_complete_sample_only",
                "semantic_probe_grid": {
                    "assignment": "canonical_grid_index_primary_outer_conditioning_inner",
                    "collapsed_reason": "collapsed_process_probe_grid",
                },
                "capacity": {
                    "top_level_predictors_maximum": 8,
                    "controls_maximum": 1,
                    "equation_non_intercept_terms_maximum": 50,
                },
                "recipe_status": "validated_regression_process_v2_plus_bootstrap_v1_bounded_scope",
            },
            "point_reference": {
                "maximum_absolute_difference": 1e-12,
                "reference": {
                    "complete_cases": 175, "omitted_cases": 5,
                    "equations": [{} for _ in range(5)], "paths": [{} for _ in range(8)],
                    "moderations": [{} for _ in range(3)],
                    "reference_effects": [{} for _ in range(6)],
                    "conditional_indirect_effects": [{} for _ in range(5)],
                    "moderated_mediation_indices": [{} for _ in range(2)],
                    "simple_slopes": [{} for _ in range(11)],
                    "plots": [
                        {"series": [{"points": [{} for _ in range(25)]} for _ in range(count)]}
                        for count in (3, 6, 2)
                    ],
                    "johnson_neyman": [
                        {
                            "status": "available", "moderation_id": "moderation:X->M3@W",
                            "solved_moderator": "W", "conditioning_values": [],
                            "regions": [{}, {}], "curve_points": [{} for _ in range(101)],
                        },
                        {
                            "status": "available", "moderation_id": "moderation:X->Y@W|B",
                            "solved_moderator": "W",
                            "conditioning_values": [{"variable": "B", "raw_value": 0.0, "coded_value": 0.0}],
                            "regions": [{}], "curve_points": [{} for _ in range(101)],
                        },
                        {
                            "status": "available", "moderation_id": "moderation:X->Y@W|B",
                            "solved_moderator": "W",
                            "conditioning_values": [{"variable": "B", "raw_value": 1.0, "coded_value": 1.0}],
                            "regions": [{}, {}, {}], "curve_points": [{} for _ in range(101)],
                        },
                        {
                            "status": "unavailable", "moderation_id": "moderation:M4->Y@B",
                            "solved_moderator": "B", "conditioning_values": [],
                        },
                    ],
                },
            },
            "point_metamorphic": {
                "passed": True,
                "row_order_maximum_absolute_difference": 1e-13,
                "irrelevant_column_maximum_absolute_difference": 0.0,
                "path_order_canonicalized": True,
            },
            "numerical_boundaries": {
                "scale_aware_solver": {
                    "passed": True,
                    "normalization": "non_intercept_welford_mean_population_rms_v1",
                    "rank_rule": "s_min_gt_s_max_times_max_n_p_times_epsilon_times_100",
                    "fitted_maximum_absolute_difference": 1e-12,
                    "slope_back_transform_absolute_difference": 1e-12,
                    "intercept_back_transform_absolute_difference": 1e-12,
                    "covariance_back_transform_absolute_difference": 1e-12,
                    "statistic_absolute_difference": 1e-12,
                    "relative_collinearity_rejected": True,
                },
                "johnson_neyman_root_solver": {
                    "passed": True,
                    "domain_normalization": "coded_range_to_minus_one_plus_one_v1",
                    "coefficient_tolerance_multiplier": 64.0,
                    "root_deduplication_tolerance_multiplier": 128.0,
                    "stable_quadratic_formula": "q_formula_v1",
                    "exact_double_root_count": 1,
                    "resolvable_near_double_root_count": 2,
                },
                "johnson_neyman_invalid_covariance": {
                    "passed": True,
                    "reason_code": "invalid_hc3_covariance",
                    "message": audit.JN_INVALID_COVARIANCE_MESSAGE,
                    "variance_rule": "finite_and_strictly_positive_across_tested_range",
                },
                "binary_endogenous_outcome": {
                    "passed": True,
                    "reason_code": "binary_process_equation_outcome",
                    "rejected_outcomes": ["M1", "Y"],
                    "original_sample_only": True,
                    "continuous_fixture_accepted": True,
                },
                "collapsed_probe_grid": {
                    "passed": True,
                    "reason_code": "collapsed_process_probe_grid",
                    "message": (
                        "PROCESS continuous moderator W does not have three distinct finite "
                        "mean-minus-SD, mean, and mean-plus-SD probes in f64"
                    ),
                    "semantic_assignment": "canonical_grid_index_primary_outer_conditioning_inner",
                },
            },
            "reference_condition": {
                "passed": True,
                "column": "Reference condition",
                "value": audit.REFERENCE_CONDITION,
                "continuous_coded_value": 0.0,
                "binary_raw_value": 0.0,
                "maximum_absolute_difference": 0.0,
            },
            "bootstrap_exact_arithmetic": {"maximum_absolute_difference": 1e-13},
            "independent_python": {"comparison": {"passed": True}},
            "external_r": {
                "passed": True,
                "point_comparison": {"passed": True},
                "comparison": {"passed": True},
                "numerical_boundaries": {
                    "passed": True,
                    "scale_aware_solver": {"passed": True},
                    "johnson_neyman_root_solver": {"passed": True},
                    "johnson_neyman_invalid_covariance": {
                        "passed": True,
                        "reason_code": "invalid_hc3_covariance",
                        "message": audit.JN_INVALID_COVARIANCE_MESSAGE,
                        "variance_rule": "finite_and_strictly_positive_across_tested_range",
                    },
                    "binary_endogenous_outcome": {"passed": True},
                    "collapsed_probe_grid": {
                        "passed": True,
                        "reason_code": "collapsed_process_probe_grid",
                        "semantic_assignment": "canonical_grid_index_primary_outer_conditioning_inner",
                    },
                },
                "reference_condition": {
                    "passed": True,
                    "column": "Reference condition",
                    "value": audit.REFERENCE_CONDITION,
                    "continuous_coded_value": 0,
                    "binary_raw_value": 0,
                },
            },
            "artifacts": {"tested_cli": cli, "fixture": fixture, "r_script": r_script},
        }
        write_json(results / audit.REFERENCE_REPORT, reference)

        boundary = {
            "schema_version": 1,
            "target": "process_v2_focused_rust_boundary_tests",
            "feature_id": audit.FEATURE_ID,
            "method_version": audit.METHOD_VERSION,
            "passed": True,
            "checks": {target: {name: True for name in names} for target, names in audit.BOUNDARY_SUITES.items()},
            "build_commands": {
                target: [
                    "cargo", "test", "--release", "-p", target.replace("_", "-"),
                    "--lib", "--no-run", "--message-format=json",
                ]
                for target in audit.BOUNDARY_SUITES
            },
            "environment": {"CARGO_BUILD_JOBS": "1"},
            "test_executables": boundary_executables,
            "executions": {
                name: {
                    "target": target, "full_name": f"tests::{name}", "exit_code": 0,
                    "passed": True, "stdout_tail": "test result: ok. 1 passed; 0 failed",
                    "stderr_tail": "",
                }
                for target, names in audit.BOUNDARY_SUITES.items() for name in names
            },
        }
        write_json(results / audit.BOUNDARY_REPORT, boundary)

        frontend = {
            "schema_version": 1,
            "target": "process_v2_focused_frontend_gate",
            "feature_id": audit.FEATURE_ID,
            "method_version": audit.METHOD_VERSION,
            "generated_at_utc": "2026-08-12T12:00:00Z",
            "commands": {
                "vitest": ["npx", "vitest", "run", *audit.FRONTEND_TEST_FILES, "--reporter=json"],
                "tsc": ["npx", "tsc", "-b", "--pretty", "false"],
            },
            "test_files": list(audit.FRONTEND_TEST_FILES),
            "vitest": {
                "passed": True, "exit_code": 0,
                "test_files": [
                    {
                        "path": path, "status": "passed", "assertions": 1,
                        "passed_assertions": 1, "failed_assertions": 0,
                    }
                    for path in audit.FRONTEND_TEST_FILES
                ],
                "total_tests": len(audit.FRONTEND_TEST_FILES),
                "passed_tests": len(audit.FRONTEND_TEST_FILES),
                "failed_tests": 0, "pending_tests": 0,
                "stdout_tail": "", "stderr_tail": "",
            },
            "tsc": {"passed": True, "exit_code": 0, "stdout_tail": "", "stderr_tail": ""},
            "source_artifacts": [artifact(root / path, root) for path in audit.FRONTEND_GATE_SOURCES],
            "source_stable_during_gate": True,
            "passed": True,
        }
        write_json(results / audit.FRONTEND_REPORT, frontend)

        expected_options = [
            {"value": "ols", "label": "Ordinary least squares"},
            {"value": "logistic", "label": "Binary logistic (outcome coded 0/1)"},
            {"value": "process", "label": "Graph-defined Path Analysis / PROCESS"},
        ]
        visual_rows = []
        for viewport in sorted(audit.VISUAL_VIEWPORTS):
            stable_groups = {}
            for name, row_order in {
                "paths": [[f"nd-process-path-from-{index}", f"nd-process-path-to-{index}"] for index in range(8)],
                "moderators": [[f"nd-process-moderator-variable-{index}", f"nd-process-moderator-scale-{index}"] for index in range(2)],
                "moderations": [[
                    f"nd-process-moderation-edge-{index}", f"nd-process-moderation-primary-{index}",
                    f"nd-process-moderation-conditioning-{index}",
                ] for index in range(3)],
            }.items():
                mutations = [{
                    "id": control_id, "disabled": False, "changed": True,
                    "focusedBefore": True, "focusedAfterChange": True,
                    "rowOrderUnchanged": True,
                } for row in row_order for control_id in row]
                stable_groups[name] = {
                    "initialRowOrder": row_order, "finalRowOrder": row_order,
                    "rowCount": len(row_order), "selectCount": len(mutations),
                    "enabledSelectCount": len(mutations), "disabledSelectCount": 0,
                    "changedSelectCount": len(mutations), "mutations": mutations, "passed": True,
                }
            visual_rows.append({
                "viewport": viewport, "fixture": {"variables": 9, "models": 0},
                "dataSurface": True, "dialogOpened": True,
                "regressionTypeOptions": expected_options, "regressionType": "process",
                "setup": {
                    "outcome": "Y", "focal": "X", "pathRows": 8,
                    "pathsExact": True,
                    "moderatorRows": 2, "moderationRows": 3, "selectedControls": ["C"],
                    "capacity": {
                        "topLevelPredictors": 7, "topLevelPredictorsMaximum": 8,
                        "controls": 1, "controlsMaximum": 1,
                        "equationNonInterceptTermsMaximum": 50,
                    },
                    "moderatorsExact": True, "moderationsExact": True,
                    "bootstrap": "enabled", "samples": "10000",
                    "samplesBounds": {"min": "99", "max": "10000", "step": "1"},
                    "workers": "4", "workersBounds": {"min": "1", "max": "64", "step": None},
                    "seed": "20260812", "seedBounds": {"min": "0", "max": "4294967295", "step": None},
                    "startLabel": "Start graph-defined path analysis with bootstrap",
                    "startDisabledInBrowserPreview": True,
                    "runtimeBlockers": ["Calculations require the offline QuickPLS desktop runtime."],
                    "unexpectedBlockers": [], "profileReady": True,
                    "scopeExact": True, "bootstrapScopeExact": True,
                    "previewExact": True, "previewAccessible": True,
                    "stableRowIdentity": {**stable_groups, "passed": True},
                },
                "accessibility": {
                    "controlsLabeled": True, "groupsNamed": True,
                    "keyboardReachable": True, "focusRestored": True,
                },
                "truthAndOverflow": {"noFabricatedRunState": True, "noHorizontalOverflow": True},
                "dialogBounds": {"withinHorizontalViewport": True, "pageHorizontalOverflow": False},
                "completedResult": {"synthesizedByHarness": False, "available": False},
            })
        visual = {
            "passed": True, "generatedAt": "2026-08-12T12:00:00Z",
            "failures": [], "consoleErrors": [], "checks": {"processV2": visual_rows},
        }
        write_json(results / audit.VISUAL_REPORT, visual)

        xlsx_path = results / "process-v2.xlsx"
        make_xlsx(xlsx_path)
        xlsx_descriptor = artifact(xlsx_path, root)
        xlsx_scan = audit.xlsx_witness_attestation(xlsx_path)
        archive_path = results / "v247-native-process-v2-test.qpls"
        make_project(archive_path)
        archive_descriptor = artifact(archive_path, root)
        archive_backup_path = Path(f"{archive_path}.bak")
        archive_backup_path.write_bytes(archive_path.read_bytes())
        archive_identity_path = Path(f"{archive_path}.identity.json")
        archive_identity_path.write_text('{"schemaVersion":1}', encoding="utf-8")
        model_free_path = results / "v247-native-process-v2-model-free-test.qpls"
        history_path = results / "v247-native-process-v2-test.qpls.autosave"
        reset_path = results / "v247-native-process-v2-reset-test.qpls"
        make_resource_project(archive_path, model_free_path, result_count=0, selected_run_id=None)
        make_resource_project(archive_path, history_path, result_count=2, selected_run_id="process-run-2")
        history_identity_path = Path(f"{history_path}.identity.json")
        history_identity_path.write_text('{"schemaVersion":1}', encoding="utf-8")
        reset_path.write_bytes(archive_path.read_bytes())
        reset_autosave_path = Path(f"{reset_path}.autosave")
        reset_autosave_path.write_bytes(reset_path.read_bytes())
        reset_autosave_identity_path = Path(f"{reset_autosave_path}.identity.json")
        reset_autosave_identity_path.write_text('{"schemaVersion":1}', encoding="utf-8")
        model_free_descriptor = artifact(model_free_path, root)
        history_descriptor = artifact(history_path, root)
        archive_backup_descriptor = artifact(archive_backup_path, root)
        archive_identity_descriptor = artifact(archive_identity_path, root)
        history_identity_descriptor = artifact(history_identity_path, root)
        reset_descriptor = artifact(reset_path, root)
        reset_autosave_descriptor = artifact(reset_autosave_path, root)
        reset_autosave_identity_descriptor = artifact(reset_autosave_identity_path, root)
        role_counts = {
            "desktop_root": 1, "webview_browser": 0, "webview_renderer": 0,
            "webview_gpu": 0, "webview_utility": 0, "webview_other": 0,
            "other_descendant": 0,
        }

        def sample(recorded: str, working_set: int, private: int, handles: int, threads: int) -> dict:
            return {
                "recorded_at_utc": recorded, "root_present": True, "root_pid": 1234,
                "total_working_set_bytes": working_set, "total_private_memory_bytes": private,
                "total_handle_count": handles, "total_thread_count": threads,
                "process_role_counts": role_counts,
                "processes": [{
                    "pid": 1234, "parent_pid": 1000, "name": "quickpls-desktop.exe",
                    "role": "desktop_root", "creation_date": "20260812115959.000000+000",
                    "working_set_bytes": working_set, "private_memory_bytes": private,
                    "handle_count": handles, "thread_count": threads,
                }],
            }

        phases = [
            ("initial_idle", "model_free_fixture", "data", 0, 0, None, 0, 200_000_000, 180_000_000, 100, 20, model_free_path),
            ("post_cancellation_idle", "cancelled_setup_no_result", "data", 0, 0, None, 12, 240_000_000, 210_000_000, 105, 22, model_free_path),
            ("post_completed_cycle_1_idle", "one_result_reopened_original", "results", 1, 1, "process-run-1", 24, 260_000_000, 230_000_000, 110, 24, archive_path),
            ("post_completed_history_2_idle", "two_results_retained_history", "results", 2, 2, "process-run-2", 36, 300_000_000, 260_000_000, 115, 26, history_path),
            ("post_completed_cycle_2_idle", "one_result_reopened_reset_clone", "results", 1, 1, "process-run-1", 48, 250_000_000, 225_000_000, 112, 25, reset_path),
        ]

        def iso(milliseconds: int) -> str:
            minute_ms = 60_000
            seconds, millis = divmod(milliseconds % minute_ms, 1_000)
            return f"2026-08-12T12:00:{seconds:02d}.{millis:03d}Z"

        raw_samples = [sample("2026-08-12T11:59:59.000Z", 190_000_000, 170_000_000, 98, 19)]
        checkpoint_rows = []
        checkpoint_samples_by_name = {}
        checkpoint_diagnostics = []
        phase_snapshot_descriptors = []
        phase_entries = {}
        for name, kind, surface, results_count, witness_count, selected, second, working, private, handles, threads, source_path in phases:
            window = [sample(iso(second * 1_000 + offset), working, private, handles, threads)
                      for offset in (500, 750, 1_000, 1_250, 1_500, 1_750)]
            raw_samples.extend(window)
            checkpoint_samples_by_name[name] = window
            role_window = audit.bounded_process_role_window(window)
            logical = {
                "surface": surface, "completed_result_count": results_count,
                "witness_count": witness_count, "selected_run_id": selected, "state_kind": kind,
            }
            completed_ids = [f"process-run-{index + 1}" for index in range(results_count)]
            archive_logical = {
                "manifestValid": True, "completedResultCount": results_count,
                "witnessCount": witness_count, "completedRunIds": completed_ids,
                "witnessRunIds": completed_ids, "recipeIds": ["recipe-process-v2"] * results_count,
                "recipeCount": 0 if results_count == 0 else 1, "workspaceRunIds": completed_ids,
                "selectedRunId": selected,
            }
            snapshot_path = results / f"process-v2-resource-snapshot-20260812-1234-{name}.qpls"
            snapshot_path.write_bytes(source_path.read_bytes())
            snapshot_artifact = artifact(snapshot_path, root)
            reported_source_path = (
                archive_descriptor["path"] if name in {"initial_idle", "post_cancellation_idle"}
                else history_descriptor["path"] if name in {
                    "post_completed_cycle_1_idle", "post_completed_history_2_idle",
                }
                else f"{reset_descriptor['path']}.autosave"
            )
            source_mtime_ns = str(source_path.stat().st_mtime_ns)
            effective_archive = {
                "path": snapshot_artifact["path"], "bytes": snapshot_artifact["size"],
                "sha256": snapshot_artifact["sha256"], "source_path": reported_source_path,
                "source_before": {
                    "bytes": snapshot_artifact["size"], "sha256": snapshot_artifact["sha256"],
                    "mtime_ns": source_mtime_ns,
                },
                "source_after": {
                    "bytes": snapshot_artifact["size"], "sha256": snapshot_artifact["sha256"],
                    "mtime_ns": source_mtime_ns,
                },
                "source_stable_during_copy": True, "exclusive_atomic_copy": True,
                "application_opened": False, "logical_state": archive_logical,
            }
            phase_snapshot_descriptors.append(snapshot_artifact)
            checkpoint_rows.append({
                "name": name, "phase_recorded_at_utc": iso(second * 1_000),
                "window_start_utc": iso(second * 1_000 + 500),
                "window_end_utc": iso(second * 1_000 + 10_500),
                "sample_recorded_at_utc": [row["recorded_at_utc"] for row in window],
                "sample_count": 6, "median_working_set_bytes": working, "p95_working_set_bytes": working,
                "median_private_memory_bytes": private, "p95_private_memory_bytes": private,
                "median_handle_count": handles, "p95_handle_count": handles,
                "median_thread_count": threads, "p95_thread_count": threads,
                "median_process_count": 1, "p95_process_count": 1,
                "process_role_counts": role_counts,
                "process_roles_bounded_and_terminally_stable": True,
                "process_role_window": role_window,
                "idle_settle_milliseconds": 5_000, "capture_delay_milliseconds": 500,
                "sample_window_milliseconds": 10_000, "logical_state": logical,
                "effective_archive": effective_archive,
            })
            checkpoint_diagnostics.append({
                "name": name, "passed": True, "phase_present": True,
                "phase_recorded_at_utc": iso(second * 1_000),
                "window_start_utc": iso(second * 1_000 + 500),
                "window_end_utc": iso(second * 1_000 + 10_500),
                "eligible_sample_recorded_at_utc": [row["recorded_at_utc"] for row in window],
                "eligible_sample_count": 6,
                "expected_idle_settle_milliseconds": 5_000,
                "actual_idle_settle_milliseconds": 5_000,
                "expected_capture_delay_milliseconds": 500,
                "actual_capture_delay_milliseconds": 500,
                "expected_sample_window_milliseconds": 10_000,
                "actual_sample_window_milliseconds": 10_000,
                "minimum_samples": 6, "failure_reasons": [],
            })
            phase_entries[name] = {
                "recorded_at_utc": iso(second * 1_000),
                "idle_settle_milliseconds": 5_000,
                "capture_delay_milliseconds": 500,
                "sample_window_milliseconds": 10_000,
                "logical_state": logical,
                "effective_archive": effective_archive,
                "primary_archive": {"path": archive_descriptor["path"], "bytes": archive_descriptor["size"]},
                "export": {"path": xlsx_descriptor["path"], "bytes": xlsx_descriptor["size"]},
            }
        cycle1_autosave_descriptor = {
            "path": history_descriptor["path"],
            "size": archive_descriptor["size"],
            "sha256": archive_descriptor["sha256"],
        }
        cycle1_live_descriptors = [
            cycle1_autosave_descriptor,
            history_identity_descriptor,
            archive_backup_descriptor,
            archive_identity_descriptor,
        ]
        cycle1_captures = []
        capture_contents = [
            archive_path.read_bytes(), history_identity_path.read_bytes(),
            archive_backup_path.read_bytes(), archive_identity_path.read_bytes(),
        ]
        for index, (source, contents) in enumerate(zip(cycle1_live_descriptors, capture_contents)):
            snapshot = results / f"process-v2-resource-snapshot-20260812-1234-sidecar-cycle1-{index}.bin"
            snapshot.write_bytes(contents)
            cycle1_captures.append({
                "source_path": source["path"], "source_size": source["size"],
                "source_sha256": source["sha256"], "snapshot": artifact(snapshot, root),
            })
        cycle1_autosave_state = {
            "prefix": archive_descriptor["path"], "coversEverySiblingPrefix": True,
            "present": [row["path"] for row in cycle1_live_descriptors],
            "artifacts": cycle1_live_descriptors,
            "required": [cycle1_autosave_descriptor["path"], history_identity_descriptor["path"]],
            "allowed": sorted([
                cycle1_autosave_descriptor["path"], f"{cycle1_autosave_descriptor['path']}.bak",
                history_identity_descriptor["path"], archive_backup_descriptor["path"],
                archive_identity_descriptor["path"],
            ]),
            "missing": [], "forbidden": [], "exactAllowedIdentity": True,
            "autosavePath": cycle1_autosave_descriptor["path"],
            "logicalState": checkpoint_rows[2]["effective_archive"]["logical_state"],
            "capturedArtifacts": cycle1_captures,
        }
        raw_samples.append(sample("2026-08-12T12:00:59.000Z", 450_000_000, 400_000_000, 130, 30))
        raw_samples_path = results / "process_v2_resource_samples.jsonl"
        raw_samples_path.write_text("\n".join(json.dumps(row) for row in raw_samples) + "\n", encoding="utf-8")
        raw_samples_descriptor = artifact(raw_samples_path, root)
        phase_document = {
            "schema_version": 2, "feature_id": audit.FEATURE_ID,
            "method_version": audit.METHOD_VERSION, "phases": phase_entries,
        }
        resource_phases_path = results / "process_v2_resource_phases.json"
        write_json(resource_phases_path, phase_document)
        resource_phases_descriptor = artifact(resource_phases_path, root)
        initial_archive_bytes = model_free_descriptor["size"]
        first_resource_sample = raw_samples[0]
        terminal_selection = audit.terminal_resource_selection(
            checkpoint_samples_by_name["post_cancellation_idle"],
            checkpoint_rows[1]["process_role_window"],
        )
        full_window_disclosure = audit.resource_full_window_disclosure(
            checkpoint_samples_by_name["initial_idle"],
            checkpoint_samples_by_name["post_cancellation_idle"],
        )
        self.assertTrue(terminal_selection["passed"])
        self.assertIsNotNone(full_window_disclosure)
        resource_document = {
            "schema_version": 1, "target": "process_v2_packaged_resource_report",
            "feature_id": audit.FEATURE_ID, "method_version": audit.METHOD_VERSION,
            "generated_at_utc": "2026-08-12T12:02:01Z", "launched_pid": 1234,
            "sample_interval_milliseconds": 250, "sample_count": len(raw_samples), "raw_sample_count": len(raw_samples),
            "first_sample": first_resource_sample, "monitor_terminal_reason": "stop_signal",
            "capture_delay_milliseconds": 500, "sample_window_milliseconds": 10_000,
            "raw_samples": raw_samples_descriptor,
            "phase_document": resource_phases_descriptor,
            "phase_snapshots": phase_snapshot_descriptors,
            "idle_checkpoints": checkpoint_rows,
            "checkpoint_diagnostics": checkpoint_diagnostics,
            "memory": {
                "policy": audit.RESOURCE_POLICY,
                "peak_working_set_bytes": 450_000_000,
                "peak_private_memory_bytes": 400_000_000, "peak_working_set_under_2_gib": True,
                "cancellation_working_set_tolerance_bytes": 134_217_728,
                "cancellation_private_memory_tolerance_bytes": 134_217_728,
                "cancellation_terminal_sample_count": terminal_selection["sample_count"],
                "cancellation_terminal_minimum_samples": audit.RESOURCE_TERMINAL_SAMPLE_COUNT,
                "cancellation_terminal_samples_role_stable": terminal_selection["samples_role_stable"],
                "cancellation_terminal_sample_recorded_at_utc": terminal_selection["sample_recorded_at_utc"],
                "cancellation_terminal_max_working_set_bytes": terminal_selection["max_working_set_bytes"],
                "cancellation_terminal_max_private_memory_bytes": terminal_selection["max_private_memory_bytes"],
                "cancellation_within_baseline_tolerance": True,
                "full_window_disclosure": full_window_disclosure,
                "equal_state_working_set_tolerance_bytes": 67_108_864,
                "equal_state_private_memory_tolerance_bytes": 67_108_864,
                "equal_state_working_set_within_tolerance": True,
                "equal_state_private_memory_within_tolerance": True,
                "equal_state_handle_tolerance": 64, "equal_state_thread_tolerance": 16,
                "equal_state_handle_count_within_tolerance": True,
                "equal_state_thread_count_within_tolerance": True,
                "equal_state_process_roles_exact": True,
                "process_roles_bounded_and_terminally_stable": True,
                "retained_history_disclosure": {
                    "checkpoint": "post_completed_history_2_idle",
                    "median_working_set_bytes": 300_000_000,
                    "median_private_memory_bytes": 260_000_000,
                    "completed_result_count": 2, "witness_count": 2,
                    "qualification_role": "disclosure_only_not_a_threshold",
                },
                "phase_snapshots_attested": True,
                "phase_document_attested": True,
                "conclusion": audit.RESOURCE_CONCLUSION,
                "cancellation_cycle_count": 1, "completed_cycle_count": 2,
                "idle_checkpoint_count": 5,
                "idle_settle_milliseconds": 5000,
                "idle_checkpoints_ordered_and_distinct": True,
                "capture_delay_milliseconds": 500, "sample_window_milliseconds": 10_000,
                "minimum_samples_per_checkpoint": 6,
                "checkpoint_diagnostic_count": 5,
                "checkpoint_diagnostics_all_passed": True,
            },
            "disk": {
                "project_archive": {
                    "path": archive_descriptor["path"], "initial_bytes": initial_archive_bytes,
                    "final_bytes": archive_descriptor["size"],
                    "delta_bytes": archive_descriptor["size"] - initial_archive_bytes,
                },
                "xlsx_export": {
                    "path": xlsx_descriptor["path"], "initial_bytes": 0,
                    "final_bytes": xlsx_descriptor["size"], "delta_bytes": xlsx_descriptor["size"],
                },
            },
            "process_cleanup": {
                "graceful_close_exit_code": 0, "graceful_exit_confirmed": True,
                "forced_parent_termination": False, "forced_descendant_pids": [],
                "forced_resource_monitor_termination": False,
                "parent_exit_confirmed": True, "lingering_descendant_pids": [],
                "resource_monitor_exit_confirmed": True, "resource_monitor_exit_code": 0,
                "resource_monitor_stderr": "", "resource_monitor_terminal_reason": "stop_signal",
            },
            "passed": True,
        }
        resource_path = results / "process_v2_resource_report.json"
        write_json(resource_path, resource_document)
        resource_descriptor = artifact(resource_path, root)
        screenshots = [
            artifact(
                results / f"{180 + index}-tauri-native-process-v2-state-{index}.png",
                root,
                f"screenshot:{index}".encode(),
            )
            for index in range(5)
        ]
        process_table_contract = {
            "passed": True,
            "reference_sheet": "Reference effects",
            "reference_columns": ["Effect ID", "Kind", "Path", "Estimate", "Reference condition"],
            "reference_effect_rows": 6,
            "reference_condition": audit.REFERENCE_CONDITION,
            "result_status": "validated",
            "promotion_pending_warning_absent": True,
            "curve_warning_disclosure": audit.PROCESS_CURVE_WARNING_DISCLOSURE,
            "curve_warning_disclosure_exact": True,
            "required_shared_strings_verified": True,
        }
        raw_page = {
            "index": 0, "url": "http://tauri.localhost/",
            "origin": audit.PACKAGED_TAURI_ORIGIN, "title": "QuickPLS",
            "shellVisible": True, "tauriRuntime": True,
        }
        packaged_page = {
            "index": 0, "url": "http://tauri.localhost/",
            "origin": audit.PACKAGED_TAURI_ORIGIN, "title": "QuickPLS",
            "shell_visible": True, "tauri_runtime": True,
        }
        retry_setup_snapshot = {
            "catalogCount": 18, "selectedMethod": "Regression", "regressionType": "process",
            "outcome": "Y", "focalPredictor": "X",
            "paths": [
                {"from": "X", "to": "Y"}, {"from": "X", "to": "M1"},
                {"from": "M1", "to": "M2"}, {"from": "M2", "to": "Y"},
                {"from": "X", "to": "M3"}, {"from": "M3", "to": "Y"},
                {"from": "X", "to": "M4"}, {"from": "M4", "to": "Y"},
            ],
            "moderators": [
                {"variable": "W", "scale": "continuous"},
                {"variable": "B", "scale": "binary_0_1"},
            ],
            "moderations": [
                {"edge": "X -> Y", "primary": "W", "conditioning": "B"},
                {"edge": "X -> M3", "primary": "W", "conditioning": ""},
                {"edge": "M4 -> Y", "primary": "B", "conditioning": ""},
            ],
            "selectedControls": ["C"], "samples": "10000", "workers": "4",
            "seed": "20260812",
            "profile": "175 global listwise-complete cases; 5 rows omitted; 5 OLS equations verified",
            "profileAriaBusy": "false", "scope": "Frozen PROCESS v2 scope",
            "bootstrapScope": "Frozen bootstrap scope", "blockers": [], "startEnabled": True,
            "capacity": {
                "topLevelPredictors": 7, "topLevelPredictorsMaximum": 8,
                "controls": 1, "controlsMaximum": 1, "equationNonInterceptTermsMaximum": 50,
            },
            "graphDefinedWithoutNumberedTemplates": True,
        }
        source_report = {
            "passed": True, "feature_id": audit.FEATURE_ID,
            "method_version": audit.METHOD_VERSION,
            "bootstrap_method_version": audit.BOOTSTRAP_METHOD_VERSION,
            "catalogue_snapshot_date": audit.CATALOGUE_SNAPSHOT_DATE,
            "generatedAt": "2026-08-12T12:01:00Z",
            "endpoint": "http://127.0.0.1:9222", "runtime": "tauri-webview2-cdp",
            "focusedRun": {
                "scope": "process_v2", "priorGeneratedAt": None,
                "completedAt": "2026-08-12T12:02:00Z",
            },
            "screenshots": [str((root / row["path"]).resolve()) for row in screenshots],
            "consoleErrors": [], "failures": [],
            "checks": {
                "runtimePreflight": {
                    "passed": True, "expectedOrigin": audit.PACKAGED_TAURI_ORIGIN,
                    "enumeratedPages": [raw_page], "qualifyingPageCount": 1,
                    "preReload": raw_page, "reloadCount": 1,
                    "postReload": raw_page, "sameOrigin": True,
                },
                "processV2ReferenceFixture": {"rows": 180},
                "processV2FixtureProvisioning": {"project": "process-v2.qpls"},
                "runtime": {"tauriRuntime": True},
                "processV2Workflow": {"passed": True},
                "processV2Fixture": {"passed": True},
                "processV2Cancellation": {"passed": True},
                "processV2CancelledRetrySetup": {
                    "passed": True, "readOnly": True, "exactFrozenSetupMatch": True,
                    "snapshot": retry_setup_snapshot, "frozenSetup": retry_setup_snapshot,
                },
                "processV2Setup": {"passed": True},
                "processV2Export": {"passed": True},
                "processV2Results": {"passed": True},
                "processV2WitnessBoundary": {"passed": True},
                "processV2RepeatedCompletion": {"passed": True},
                "processV2SaveReopen": {"passed": True},
                "processV2ResourceResetClone": {"passed": True},
                "recentProjectsRestored": True,
            },
        }
        source_report["checks"]["processV2RepeatedCompletion"] = {
            "passed": True, "activeLifecycleCaptured": True,
            "priorRunId": "process-run-1", "repeatedRunId": "process-run-2",
            "completedRunIdsBefore": ["process-run-1"],
            "completedRunIdsAfter": ["process-run-2", "process-run-1"],
            "addedRunIds": ["process-run-2"],
            "completedRunCountBefore": 1, "completedRunCount": 2,
            "uniqueCompletedRunCount": 2,
            "autoSelectedRunId": "process-run-2", "explicitlySelectedRunId": "process-run-2",
            "initialSelectedTable": "process_model_summary",
        }
        source_report["checks"]["processV2SaveReopen"].update({
            "settledAutosave": cycle1_autosave_state,
            "autosaveAfterCheckpoint": json.loads(json.dumps(cycle1_autosave_state)),
        })
        source_report["checks"]["processV2ResourceResetClone"] = {
            "passed": True,
            "identity": {"resultId": "process-run-1", "recipeId": "recipe-process-v2", "runId": "process-run-1"},
            "logicalState": {"completedResultCount": 1, "witnessCount": 1},
            "resetTableIds": audit.EXPECTED_TABLE_IDS,
            "selectedRunId": "process-run-1", "selectedTableId": "process_model_summary",
            "sidecarsBeforeCopy": {"present": []}, "sidecarsAfterCopy": {"present": []},
            "sidecarsBeforeOpen": {"present": []},
            "settledAutosave": {
                "prefix": reset_descriptor["path"], "coversEverySiblingPrefix": True,
                "present": [reset_autosave_descriptor["path"], reset_autosave_identity_descriptor["path"]],
                "artifacts": [reset_autosave_descriptor, reset_autosave_identity_descriptor],
                "required": [reset_autosave_descriptor["path"], reset_autosave_identity_descriptor["path"]],
                "allowed": [
                    reset_autosave_descriptor["path"],
                    f"{reset_autosave_descriptor['path']}.bak",
                    reset_autosave_identity_descriptor["path"],
                ],
                "missing": [], "forbidden": [], "exactAllowedIdentity": True,
                "autosavePath": reset_autosave_descriptor["path"],
                "logicalState": checkpoint_rows[-1]["effective_archive"]["logical_state"],
            },
            "autosaveAfterCheckpoint": {},
            "recoveryDisclosureAbsent": True,
        }
        source_report["checks"]["processV2ResourceResetClone"]["autosaveAfterCheckpoint"] = json.loads(
            json.dumps(source_report["checks"]["processV2ResourceResetClone"]["settledAutosave"])
        )
        source_report["checks"]["processV2Results"].update({
            "referenceEffectColumnsExact": True,
            "referenceConditionRowsExact": True,
            "promotionPendingWarningAbsent": True,
            "curveWarningDisclosureExact": True,
            "johnsonNeymanRows": 7,
            "johnsonNeymanAnalysisCount": 4,
            "johnsonNeymanAnalysisKeys": [
                ["moderation:X->M3@W", "W", ""],
                ["moderation:X->Y@W|B", "W", "B = 0.0000 (coded 0.0000)"],
                ["moderation:X->Y@W|B", "W", "B = 1.0000 (coded 1.0000)"],
                ["moderation:M4->Y@B", "B", ""],
            ],
        })
        source_report["checks"]["processV2Export"]["nativeXlsx"] = {
            "processTableContract": process_table_contract,
        }
        write_json(root / audit.PACKAGED_SOURCE_REPORT, source_report)
        workbook_sheets = [
            "Model summary", "Directed paths", "Equation coefficients", "Equation fit",
            "Reference effects", "Conditional indirect effects", "Moderated-mediation indices",
            "Simple slopes and conditional p", "Conditional outcome plot data",
            "Johnson-Neyman regions", "Johnson-Neyman curve data", "Bootstrap summary",
            "Bootstrap failures", "Bootstrap inference", "Bootstrap BCa intervals", "Scope and provenance",
            "Run provenance",
        ]
        checks = {
            "runtime_preflight": {
                "passed": True, "expected_origin": audit.PACKAGED_TAURI_ORIGIN,
                "enumerated_pages": [packaged_page], "qualifying_page_count": 1,
                "pre_reload": packaged_page, "reload_count": 1,
                "post_reload": packaged_page, "same_origin": True,
                "source_check": "runtimePreflight",
            },
            "workflow": {
                "passed": True, "completed": True, "active_lifecycle_captured": True,
                "model_free": True, "graph_defined_without_numbered_templates": True,
                "source_check": "processV2Workflow",
            },
            "setup": {
                "passed": True, "outcome": "Y", "focal_predictor": "X", "paths": 8,
                "top_level_predictors": 7, "top_level_predictors_maximum": 8,
                "moderators": 2, "moderations": 3, "controls": 1,
                "controls_maximum": 1, "equation_non_intercept_terms_maximum": 50,
                "bootstrap_replicates": 10_000, "workers": 4, "seed": 20_260_812,
                "source_check": "processV2Setup",
            },
            "results": {
                "passed": True, "initial_selected_table": "process_model_summary",
                "table_ids": audit.EXPECTED_TABLE_IDS, "exact_table_ids": True,
                "equation_count": 5, "reference_effect_rows": 6,
                "conditional_indirect_rows": 5, "moderated_mediation_index_rows": 2,
                "simple_slope_rows": 11, "conditional_plot_point_rows": 275,
                "johnson_neyman_rows": 7, "johnson_neyman_analysis_count": 4,
                "johnson_neyman_analysis_keys": [
                    ["moderation:X->M3@W", "W", ""],
                    ["moderation:X->Y@W|B", "W", "B = 0.0000 (coded 0.0000)"],
                    ["moderation:X->Y@W|B", "W", "B = 1.0000 (coded 1.0000)"],
                    ["moderation:M4->Y@B", "B", ""],
                ],
                "johnson_neyman_curve_point_rows": 303,
                "bootstrap_estimand_rows": 24, "failure_disclosure_truthful": True,
                "accessible_non_color_plot_semantics": True,
                "reference_effect_columns_exact": True,
                "reference_condition_rows_exact": True,
                "promotion_pending_warning_absent": True,
                "curve_warning_disclosure_exact": True,
                "validation_witness_not_rendered": True, "no_na_fabrication": True,
                "generic_regression_shell_not_applicable": True,
                "expected_counts_source": "validation/process_v2_reference.py:reference_graph",
                "expected_graph_counts": {
                    "completeCases": 175, "omittedCases": 5, "equations": 5,
                    "paths": 8, "moderations": 3, "referenceEffects": 6,
                    "conditionalIndirectEffects": 5, "moderatedMediationIndices": 2,
                    "simpleSlopes": 11, "plots": 3, "conditionalPlotPoints": 275,
                    "johnsonNeyman": 4, "johnsonNeymanRegionRows": 7,
                    "availableJohnsonNeyman": 3,
                    "johnsonNeymanCurvePoints": 303, "estimands": 24,
                },
                "source_check": "processV2Results",
            },
            "export": {
                "passed": True, "workbook_sheets": workbook_sheets,
                "validation_witness_excluded": True,
                "witness_scan": {
                    "passed": True, "total_members": len(xlsx_scan["members"]),
                    "scanned_xml_and_rels_members": xlsx_scan["scanned_xml_and_rels_members"],
                    "worksheet_members": xlsx_scan["worksheet_members"],
                    "worksheet_row_counts": xlsx_scan["worksheet_row_counts"],
                    "forbidden_matches": [], "extraction_errors": [],
                },
                "process_table_contract": process_table_contract,
                "artifact_sha256": xlsx_descriptor["sha256"],
                "source_check": "processV2Export",
            },
            "save_reopen": {
                "passed": True, "same_run_restored": True,
                "initial_selected_table": "process_model_summary",
                "project_checksum_matches": True, "archive_witness_validated": True,
                "archive_sha256": archive_descriptor["sha256"],
                "cycle_1_settled_autosave": cycle1_autosave_state,
                "cycle_1_autosave_after_checkpoint": json.loads(json.dumps(cycle1_autosave_state)),
                "source_check": "processV2SaveReopen",
            },
            "cancellation": {
                "passed": True, "active_lifecycle_captured": True,
                "no_partial_result": True, "source_check": "processV2Cancellation",
            },
            "cancelled_retry_setup": {
                "passed": True, "read_only": True, "exact_frozen_setup_match": True,
                "snapshot": retry_setup_snapshot, "frozen_setup": retry_setup_snapshot,
                "source_check": "processV2CancelledRetrySetup",
            },
            "witness_boundary": {
                "passed": True, "archive_only": True,
                "witness_method_version": audit.WITNESS_VERSION,
                "estimand_order_exact": True, "bootstrap_index_partition_exact": True,
                "jackknife_index_partition_exact": True, "excluded_from_results": True,
                "excluded_from_exports": True, "source_check": "processV2WitnessBoundary",
            },
            "resource_reset": {
                "passed": True, "original_path": archive_descriptor["path"],
                "reset_path": reset_descriptor["path"], "distinct_path": True,
                "original_archive": archive_descriptor, "reset_archive": reset_descriptor,
                "result_id": "process-run-1", "recipe_id": "recipe-process-v2", "run_id": "process-run-1",
                "completed_result_count": 1, "witness_count": 1,
                "no_sidecars_before_copy": True, "no_sidecars_after_copy": True,
                "no_sidecars_before_open": True,
                "settled_autosave_sidecars_exact": True,
                "autosave_sidecars_stable_after_checkpoint": True,
                "recovery_disclosure_absent": True,
                "table_ids": audit.EXPECTED_TABLE_IDS, "selected_run_id": "process-run-1",
                "selected_table_id": "process_model_summary", "source_check": "processV2ResourceResetClone",
            },
            "resources": {
                "passed": True, "sample_count": len(raw_samples), "raw_sample_count": len(raw_samples),
                "first_sample": first_resource_sample, "monitor_terminal_reason": "stop_signal",
                "peak_working_set_bytes": 450_000_000, "peak_private_memory_bytes": 400_000_000,
                "peak_working_set_under_2_gib": True,
                "policy": audit.RESOURCE_POLICY,
                "cancellation_terminal_sample_count": terminal_selection["sample_count"],
                "cancellation_terminal_minimum_samples": audit.RESOURCE_TERMINAL_SAMPLE_COUNT,
                "cancellation_terminal_samples_role_stable": terminal_selection["samples_role_stable"],
                "cancellation_terminal_sample_recorded_at_utc": terminal_selection["sample_recorded_at_utc"],
                "cancellation_terminal_max_working_set_bytes": terminal_selection["max_working_set_bytes"],
                "cancellation_terminal_max_private_memory_bytes": terminal_selection["max_private_memory_bytes"],
                "cancellation_within_baseline_tolerance": True,
                "full_window_disclosure": full_window_disclosure,
                "equal_state_working_set_within_tolerance": True,
                "equal_state_private_memory_within_tolerance": True,
                "equal_state_handle_count_within_tolerance": True,
                "equal_state_thread_count_within_tolerance": True,
                "equal_state_process_roles_exact": True,
                "process_roles_bounded_and_terminally_stable": True,
                "retained_history_disclosure": {
                    "checkpoint": "post_completed_history_2_idle",
                    "median_working_set_bytes": 300_000_000,
                    "median_private_memory_bytes": 260_000_000,
                    "completed_result_count": 2, "witness_count": 2,
                    "qualification_role": "disclosure_only_not_a_threshold",
                },
                "phase_snapshots_attested": True,
                "phase_document_attested": True,
                "conclusion": audit.RESOURCE_CONCLUSION,
                "cancellation_cycle_count": 1, "completed_cycle_count": 2,
                "idle_checkpoint_count": 5,
                "idle_settle_milliseconds": 5000,
                "idle_checkpoints_ordered_and_distinct": True,
                "capture_delay_milliseconds": 500, "sample_window_milliseconds": 10_000,
                "minimum_samples_per_checkpoint": 6,
                "checkpoint_diagnostic_count": 5,
                "checkpoint_diagnostics_all_passed": True,
                "artifact_disk_deltas_recorded": True,
                "zero_lingering_descendants": True,
                "graceful_exit_confirmed": True, "parent_absent": True,
                "forced_parent_termination": False, "forced_descendant_pids": [],
                "forced_resource_monitor_termination": False,
                "source_check": "processV2Resources",
            },
        }
        packaged = {
            "schema_version": "quickpls.packaged_acceptance.v1",
            "kind": "quickpls3_scoped_tauri_process_v2_acceptance",
            "passed": True, "generated_at_utc": "2026-08-12T12:01:00Z",
            "completed_at_utc": "2026-08-12T12:02:00Z",
            "feature_id": audit.FEATURE_ID, "method_version": audit.METHOD_VERSION,
            "bootstrap_method_version": audit.BOOTSTRAP_METHOD_VERSION,
            "catalogue_snapshot_date": audit.CATALOGUE_SNAPSHOT_DATE,
            "target": "windows_10_11_x64_packaged_tauri",
            "runtime": "tauri-webview2-cdp", "endpoint": "http://127.0.0.1:9222",
            "generator": "validation/v247_tauri_native_acceptance.mjs",
            "tested_product": {
                "qpls_cli_exe": cli,
                "quickpls_desktop_exe": desktop,
                "dist_bundle": audit.directory_manifest(root, "dist"),
            },
            "checks": checks,
            "artifacts": {
                "xlsx": xlsx_descriptor, "project_archive": archive_descriptor,
                "resource_report": resource_descriptor, "resource_samples": raw_samples_descriptor,
                "resource_phases": resource_phases_descriptor,
                "resource_phase_snapshots": phase_snapshot_descriptors,
                "screenshots": screenshots,
            },
            "console_errors": [], "failures": [], "source_report": audit.PACKAGED_SOURCE_REPORT,
        }
        packaged_path = results / audit.PACKAGED_REPORT
        write_json(packaged_path, packaged)
        write_json(results / audit.PROCESS_CLEANUP_REPORT, {
            "generated_at_utc": "2026-08-12T12:02:02Z",
            "launched_pid": 1234, "parent_exit_confirmed": True,
            "graceful_close_exit_code": 0, "graceful_exit_confirmed": True,
            "forced_parent_termination": False, "forced_descendant_pids": [],
            "forced_resource_monitor_termination": False,
            "lingering_descendant_pids": [], "resource_monitor_pid": 5678,
            "resource_monitor_exit_confirmed": True, "resource_monitor_exit_code": 0,
            "resource_monitor_stderr": "", "resource_monitor_terminal_reason": "stop_signal",
            "resource_monitor_first_sample": first_resource_sample, "passed": True,
        })
        return results

    def test_complete_current_evidence_can_pass(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-audit-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            report = audit.build_audit(root, results)
            self.assertTrue(report["passed"], json.dumps(report["checks"], indent=2))

    def test_complete_packaged_fixture_matches_dedicated_schema(self) -> None:
        schema = json.loads((VALIDATION / "process_v2_packaged_acceptance.schema.json").read_text(encoding="utf-8"))
        Draft202012Validator.check_schema(schema)
        with tempfile.TemporaryDirectory(prefix="quickpls-process-schema-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            document = json.loads((results / audit.PACKAGED_REPORT).read_text(encoding="utf-8"))
            errors = sorted(Draft202012Validator(schema).iter_errors(document), key=lambda row: list(row.path))
            self.assertEqual(errors, [], "\n".join(error.message for error in errors))

    def test_promotion_executes_draft202012_schema_and_rejects_unknown_property(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-schema-audit-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            path = results / audit.PACKAGED_REPORT
            document = json.loads(path.read_text(encoding="utf-8"))
            document["unexpected_evidence"] = True
            write_json(path, document)
            report = audit.build_audit(root, results)
            self.assertFalse(report["packaged_attestation"]["checks"]["draft202012_schema_valid"])
            self.assertTrue(report["packaged_attestation"]["schema_validation"]["errors"])
            self.assertFalse(report["passed"])

    def test_johnson_neyman_region_expansion_and_analysis_identity_fail_closed(self) -> None:
        mutations = {
            "region_rows_confused_with_analysis_count": lambda document: document["checks"]["results"].update({
                "johnson_neyman_rows": 4,
            }),
            "analysis_identity_drift": lambda document: document["checks"]["results"][
                "johnson_neyman_analysis_keys"
            ].reverse(),
            "archive_region_count_drift": lambda document: document["checks"]["results"][
                "expected_graph_counts"
            ].update({"johnsonNeymanRegionRows": 6}),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory(
                prefix="quickpls-process-jn-counts-",
            ) as directory:
                root = Path(directory)
                results = self.prepare_complete_evidence(root)
                path = results / audit.PACKAGED_REPORT
                document = json.loads(path.read_text(encoding="utf-8"))
                mutate(document)
                write_json(path, document)
                report = audit.build_audit(root, results)
                self.assertFalse(report["packaged_attestation"]["checks"]["exact_result_contract"])
                self.assertFalse(report["passed"])

    def test_packaged_runtime_preflight_fails_closed_for_origin_or_reload_drift(self) -> None:
        mutations = {
            "wrong_post_reload_origin": lambda document: document["checks"]["runtime_preflight"]["post_reload"].update({
                "url": "http://localhost:1420/", "origin": "http://localhost:1420",
            }),
            "missing_reload": lambda document: document["checks"]["runtime_preflight"].update({"reload_count": 0}),
            "multiple_qualifying_pages": lambda document: document["checks"]["runtime_preflight"].update({
                "enumerated_pages": [
                    document["checks"]["runtime_preflight"]["pre_reload"],
                    {**document["checks"]["runtime_preflight"]["pre_reload"], "index": 1},
                ],
                "qualifying_page_count": 2,
            }),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory(
                prefix="quickpls-process-preflight-",
            ) as directory:
                root = Path(directory)
                results = self.prepare_complete_evidence(root)
                path = results / audit.PACKAGED_REPORT
                document = json.loads(path.read_text(encoding="utf-8"))
                mutate(document)
                write_json(path, document)
                report = audit.build_audit(root, results)
                self.assertFalse(report["packaged_attestation"]["checks"]["runtime_preflight_contract"])
                self.assertFalse(report["passed"])

    def test_process_scoped_source_report_rejects_inherited_checks_and_screenshots(self) -> None:
        mutations = {
            "unrelated_check": lambda document: document["checks"].update({"logisticWorkflow": {"passed": True}}),
            "unrelated_screenshot": lambda document: document["screenshots"].append(
                str(VALIDATION / "results/screens/v247-native-desktop-acceptance/170-tauri-native-regression-bootstrap.png")
            ),
            "prior_generic_report": lambda document: document["focusedRun"].update({
                "priorGeneratedAt": "2026-08-12T11:00:00Z",
            }),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory(
                prefix="quickpls-process-source-isolation-",
            ) as directory:
                root = Path(directory)
                results = self.prepare_complete_evidence(root)
                path = root / audit.PACKAGED_SOURCE_REPORT
                document = json.loads(path.read_text(encoding="utf-8"))
                mutate(document)
                write_json(path, document)
                report = audit.build_audit(root, results)
                self.assertFalse(report["packaged_attestation"]["checks"]["source_report_bound"])
                self.assertFalse(report["passed"])

    def test_repeated_completion_source_evidence_rejects_identity_drift(self) -> None:
        mutations = {
            "duplicate_after_id": lambda row: row.update({
                "completedRunIdsAfter": ["process-run-1", "process-run-1"],
                "uniqueCompletedRunCount": 1,
            }),
            "missing_set_difference": lambda row: row.update({"addedRunIds": []}),
            "stale_auto_selection": lambda row: row.update({"autoSelectedRunId": "process-run-1"}),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory(
                prefix="quickpls-process-repeat-identity-",
            ) as directory:
                root = Path(directory)
                results = self.prepare_complete_evidence(root)
                path = root / audit.PACKAGED_SOURCE_REPORT
                document = json.loads(path.read_text(encoding="utf-8"))
                mutate(document["checks"]["processV2RepeatedCompletion"])
                write_json(path, document)
                report = audit.build_audit(root, results)
                self.assertFalse(report["packaged_attestation"]["checks"]["source_report_bound"])
                self.assertFalse(report["passed"])

    def test_gate_fails_closed_for_missing_reports(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-audit-empty-") as directory:
            root = Path(directory)
            report = audit.build_audit(root, root / "validation/results")
            self.assertFalse(report["passed"])
            self.assertTrue(all(row["passed"] is False for row in report["checks"]))

    def test_visual_gate_requires_exact_three_viewports(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-audit-visual-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            path = results / audit.VISUAL_REPORT
            document = json.loads(path.read_text(encoding="utf-8"))
            document["checks"]["processV2"].pop()
            write_json(path, document)
            report = audit.build_audit(root, results)
            self.assertFalse(report["visual_attestation"]["checks"]["exact_three_viewports"])
            self.assertFalse(report["passed"])

    def test_failed_named_rust_boundary_cannot_be_promoted(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-audit-boundary-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            path = results / audit.BOUNDARY_REPORT
            document = json.loads(path.read_text(encoding="utf-8"))
            test_name = next(iter(audit.BOUNDARY_SUITES["qpls_estimation"]))
            document["checks"]["qpls_estimation"][test_name] = False
            document["executions"][test_name]["passed"] = False
            write_json(path, document)
            report = audit.build_audit(root, results)
            self.assertFalse(report["boundary_attestation"]["passed"])
            self.assertFalse(report["passed"])

    def test_frontend_gate_requires_every_frozen_file_and_tsc(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-audit-frontend-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            path = results / audit.FRONTEND_REPORT
            document = json.loads(path.read_text(encoding="utf-8"))
            document["vitest"]["test_files"].pop()
            document["tsc"]["exit_code"] = 1
            document["tsc"]["passed"] = False
            write_json(path, document)
            report = audit.build_audit(root, results)
            self.assertFalse(report["frontend_attestation"]["checks"]["exact_test_manifest"])
            self.assertFalse(report["frontend_attestation"]["checks"]["typescript_project_check_passed"])
            self.assertFalse(report["passed"])

    def test_xlsx_witness_leak_and_archive_partition_tamper_fail(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-audit-artifact-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            xlsx = results / "process-v2.xlsx"
            with zipfile.ZipFile(xlsx, "a", compression=zipfile.ZIP_DEFLATED) as workbook:
                workbook.writestr("xl/worksheets/witness.xml", "<row>validation_witness</row>")
            report = audit.build_audit(root, results)
            self.assertFalse(report["packaged_attestation"]["checks"]["witness_scan_fail_closed"])
            self.assertFalse(report["passed"])

    def test_safe_curve_warning_is_accepted_but_private_witness_token_is_not(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-safe-warning-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            xlsx = results / "process-v2.xlsx"
            clean = audit.xlsx_witness_attestation(xlsx)
            self.assertTrue(clean["passed"], clean)
            self.assertTrue(clean["process_table_contract"]["curve_warning_disclosure_exact"])
            with zipfile.ZipFile(xlsx, "a", compression=zipfile.ZIP_DEFLATED) as workbook:
                workbook.writestr("xl/worksheets/private-wire.xml", "<row>failed_jackknife</row>")
            dirty = audit.xlsx_witness_attestation(xlsx)
            self.assertFalse(dirty["passed"])
            self.assertTrue(any(
                row.get("token") == "failed_jackknife" for row in dirty["forbidden_matches"]
            ))

    def test_process_sheets_use_validated_status_without_promotion_warning_and_run_provenance_stays_empty(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-provenance-warning-") as directory:
            root = Path(directory)
            clean_path = root / "clean.xlsx"
            make_xlsx(clean_path)
            clean = audit.xlsx_process_table_attestation(clean_path)
            self.assertTrue(clean["passed"], clean)
            self.assertTrue(clean["run_provenance_warning_absent"])
            self.assertTrue(clean["promotion_pending_warning_absent"])
            self.assertIsNone(clean["warning_by_sheet"]["Run provenance"])
            self.assertTrue(all(
                clean["status_by_sheet"].get(name) == "validated"
                for name in audit.EXPECTED_WORKBOOK_SHEETS
            ))
            self.assertEqual(
                clean["warning_by_sheet"]["Johnson-Neyman curve data"],
                audit.PROCESS_CURVE_WARNING_DISCLOSURE,
            )

            empty_path = root / "empty.xlsx"
            make_xlsx(empty_path, run_provenance_warning="")
            empty = audit.xlsx_process_table_attestation(empty_path)
            self.assertTrue(empty["passed"], empty)
            self.assertTrue(empty["run_provenance_warning_absent"])
            self.assertIn(empty["warning_by_sheet"]["Run provenance"], (None, ""))

            for label, warning in (
                ("whitespace", " "),
                ("text", "Unexpected run provenance warning"),
            ):
                with self.subTest(label=label):
                    warned_path = root / f"warned-{label}.xlsx"
                    make_xlsx(warned_path, run_provenance_warning=warning)
                    warned = audit.xlsx_process_table_attestation(warned_path)
                    self.assertFalse(warned["passed"])
                    self.assertFalse(warned["run_provenance_warning_absent"])

    def test_xlsx_reference_condition_and_validated_status_are_independently_verified(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-audit-xlsx-contract-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            xlsx = results / "process-v2.xlsx"
            with zipfile.ZipFile(xlsx, "r") as workbook:
                members = {info.filename: workbook.read(info) for info in workbook.infolist()}
            members["xl/sharedStrings.xml"] = members["xl/sharedStrings.xml"].replace(
                audit.REFERENCE_CONDITION.encode("utf-8"),
                b"Incorrect reference condition",
            )
            with zipfile.ZipFile(xlsx, "w", compression=zipfile.ZIP_DEFLATED) as workbook:
                for name, data in members.items():
                    workbook.writestr(name, data)
            direct = audit.xlsx_process_table_attestation(xlsx)
            self.assertFalse(direct["passed"])
            report = audit.build_audit(root, results)
            self.assertFalse(
                report["packaged_attestation"]["checks"]
                ["xlsx_process_table_contract_independently_verified"]
            )
            self.assertFalse(report["passed"])

    def test_mojibake_in_process_surface_cannot_be_promoted(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-audit-utf8-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            surface = root / "src/native/nativeProcessResults.ts"
            surface.write_text(
                surface.read_text(encoding="utf-8") + "\n" + chr(0x00C3),
                encoding="utf-8",
            )
            report = audit.build_audit(root, results)
            self.assertFalse(report["text_integrity"]["passed"])
            self.assertFalse(report["passed"])

    def test_archive_rejects_unknown_bootstrap_failure_reason(self) -> None:
        for reason in ("invented_failure_reason", "cancelled"):
            with self.subTest(reason=reason), tempfile.TemporaryDirectory(
                prefix="quickpls-process-audit-reason-"
            ) as directory:
                root = Path(directory)
                archive_path = root / "unknown-reason.qpls"
                make_project(archive_path)
                with zipfile.ZipFile(archive_path, "r") as archive:
                    project = json.loads(archive.read("project.json"))
                bootstrap = project["results"][0]["payload"]["estimation"]["regression"]["process"]["graph_v2"]["bootstrap"]
                bootstrap["failed_replicates"][0]["reason_code"] = reason
                project_bytes = json.dumps(project).encode()
                manifest = {
                    "schema_version": 5, "checksum_algorithm": "sha256",
                    "checksums": {"project.json": hashlib.sha256(project_bytes).hexdigest()},
                }
                with zipfile.ZipFile(archive_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
                    archive.writestr("project.json", project_bytes)
                    archive.writestr("manifest.json", json.dumps(manifest))
                attestation = audit.archive_witness_attestation(archive_path)
                self.assertFalse(attestation["passed"])

    def test_archive_rejects_legacy_mediation_or_moderation_shells_on_process_v2(self) -> None:
        for field, value in (
            ("mediation", {"method_version": "pls_mediation_v1", "estimates": [], "tolerance": 1e-12, "warnings": []}),
            ("moderation", {"method_version": "pls_two_stage_moderation_v1", "estimates": [], "moderator_score_levels": [-1, 0, 1], "warnings": []}),
        ):
            with self.subTest(field=field), tempfile.TemporaryDirectory(
                prefix=f"quickpls-process-audit-legacy-{field}-"
            ) as directory:
                root = Path(directory)
                archive_path = root / f"legacy-{field}.qpls"
                make_project(archive_path)
                with zipfile.ZipFile(archive_path, "r") as archive:
                    project = json.loads(archive.read("project.json"))
                project["results"][0]["payload"]["estimation"][field] = value
                project_bytes = json.dumps(project).encode()
                manifest = {
                    "schema_version": 5, "checksum_algorithm": "sha256",
                    "checksums": {"project.json": hashlib.sha256(project_bytes).hexdigest()},
                }
                with zipfile.ZipFile(archive_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
                    archive.writestr("project.json", project_bytes)
                    archive.writestr("manifest.json", json.dumps(manifest))
                self.assertFalse(audit.archive_witness_attestation(archive_path)["passed"])

    def test_stale_report_and_legacy_method_identity_fail(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-audit-stale-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            reference_path = results / audit.REFERENCE_REPORT
            document = json.loads(reference_path.read_text(encoding="utf-8"))
            document["method_version"] = "regression_process_v1"
            write_json(reference_path, document)
            future = reference_path.stat().st_mtime + 10
            source = root / "crates/qpls-estimation/src/pls.rs"
            os.utime(source, (future, future))
            report = audit.build_audit(root, results)
            self.assertFalse(report["reference_attestation"]["checks"]["identity"])
            self.assertFalse(report["reference_attestation"]["checks"]["fresh_report"])
            self.assertFalse(report["passed"])

    def test_nonvalidated_recipe_status_cannot_be_promoted(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-audit-status-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            reference_path = results / audit.REFERENCE_REPORT
            reference = json.loads(reference_path.read_text(encoding="utf-8"))
            reference["scope"]["recipe_status"] = "candidate_regression_process_v2_plus_bootstrap_v1_bounded_scope"
            write_json(reference_path, reference)
            report = audit.build_audit(root, results)
            self.assertFalse(report["reference_attestation"]["checks"]["validated_recipe_status"])
            self.assertFalse(report["passed"])

    def test_resource_gate_rejects_mutated_window_metrics_and_old_unequal_state_fields(self) -> None:
        mutations = {
            "cancel_bound": lambda resource: resource["memory"].update({"cancellation_within_baseline_tolerance": False}),
            "equal_private_bound": lambda resource: resource["memory"].update({"equal_state_private_memory_within_tolerance": False}),
            "handle_bound": lambda resource: resource["memory"].update({"equal_state_handle_count_within_tolerance": False}),
            "thread_bound": lambda resource: resource["memory"].update({"equal_state_thread_count_within_tolerance": False}),
            "role_drift": lambda resource: resource["memory"].update({"equal_state_process_roles_exact": False}),
            "stored_role_window_drift": lambda resource: resource["idle_checkpoints"][0][
                "process_role_window"
            ].update({"modal_sample_count": 5}),
            "history_disclosure_drift": lambda resource: resource["memory"]["retained_history_disclosure"].update({
                "median_private_memory_bytes": 999_999_999,
            }),
            "peak_bound": lambda resource: resource["memory"].update({"peak_working_set_under_2_gib": False}),
            "old_unequal_state_field": lambda resource: resource["memory"].update({"all_post_cycle_idles_within_baseline_tolerance": True}),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory(
                prefix=f"quickpls-process-audit-resource-{label}-"
            ) as directory:
                root = Path(directory)
                results = self.prepare_complete_evidence(root)
                packaged_path = results / audit.PACKAGED_REPORT
                packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
                resource_path = results / "process_v2_resource_report.json"
                resource = json.loads(resource_path.read_text(encoding="utf-8"))
                mutate(resource)
                write_json(resource_path, resource)
                packaged["artifacts"]["resource_report"] = artifact(resource_path, root)
                write_json(packaged_path, packaged)
                report = audit.build_audit(root, results)
                self.assertFalse(report["packaged_attestation"]["checks"]["resource_contract"])
                self.assertFalse(report["passed"])

    def test_terminal_resource_selection_fails_closed_on_short_tail_role_drift_and_metric_tamper(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-terminal-policy-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            resource = json.loads((results / "process_v2_resource_report.json").read_text(encoding="utf-8"))
            samples = {
                row["recorded_at_utc"]: row
                for row in (
                    json.loads(line)
                    for line in (results / "process_v2_resource_samples.jsonl").read_text(
                        encoding="utf-8"
                    ).splitlines()
                    if line.strip()
                )
            }
            cancellation = next(
                row for row in resource["idle_checkpoints"]
                if row["name"] == "post_cancellation_idle"
            )
            rows = [samples[value] for value in cancellation["sample_recorded_at_utc"]]
            stable = audit.terminal_resource_selection(rows, cancellation["process_role_window"])
            self.assertTrue(stable["passed"], stable)
            self.assertEqual(stable["sample_count"], 6)

            short = audit.terminal_resource_selection(rows[-5:], cancellation["process_role_window"])
            self.assertFalse(short["passed"])
            self.assertEqual(short["sample_count"], 0)

            role_drift = json.loads(json.dumps(rows))
            role_drift[-1]["process_role_counts"]["webview_utility"] = 1
            self.assertFalse(
                audit.terminal_resource_selection(role_drift, cancellation["process_role_window"])["passed"]
            )

            metric_tamper = json.loads(json.dumps(rows))
            metric_tamper[-1]["total_private_memory_bytes"] += 1
            self.assertFalse(
                audit.terminal_resource_selection(metric_tamper, cancellation["process_role_window"])["passed"]
            )

    def test_v3_terminal_and_full_window_disclosures_reject_coordinated_report_tamper(self) -> None:
        mutations = {
            "terminal_max": lambda resource: resource["memory"].update({
                "cancellation_terminal_max_private_memory_bytes": 1,
            }),
            "terminal_timestamps": lambda resource: resource["memory"].update({
                "cancellation_terminal_sample_recorded_at_utc": ["2026-08-12T00:00:00Z"] * 6,
            }),
            "per_role_delta": lambda resource: resource["memory"]["full_window_disclosure"][
                "per_role_deltas"
            ][0].update({"private_memory_delta_bytes": 999}),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory(
                prefix=f"quickpls-process-v3-tamper-{label}-"
            ) as directory:
                root = Path(directory)
                results = self.prepare_complete_evidence(root)
                packaged_path = results / audit.PACKAGED_REPORT
                resource_path = results / "process_v2_resource_report.json"
                packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
                resource = json.loads(resource_path.read_text(encoding="utf-8"))
                mutate(resource)
                packaged["checks"]["resources"] = {
                    **packaged["checks"]["resources"],
                    **{
                        key: resource["memory"][key]
                        for key in (
                            "cancellation_terminal_sample_recorded_at_utc",
                            "cancellation_terminal_max_private_memory_bytes",
                            "full_window_disclosure",
                        )
                    },
                }
                write_json(resource_path, resource)
                packaged["artifacts"]["resource_report"] = artifact(resource_path, root)
                write_json(packaged_path, packaged)
                report = audit.build_audit(root, results)
                self.assertFalse(report["packaged_attestation"]["checks"]["resource_contract"])
                self.assertFalse(report["passed"])

    def test_offline_v3_remint_preserves_failed_v2_bytes_and_mints_schema_valid_current_receipt(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-v3-remint-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            resource_path = results / "process_v2_resource_report.json"
            packaged_path = results / audit.PACKAGED_REPORT
            resource = json.loads(resource_path.read_text(encoding="utf-8"))
            packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
            for key in (
                "cancellation_terminal_sample_count",
                "cancellation_terminal_minimum_samples",
                "cancellation_terminal_samples_role_stable",
                "cancellation_terminal_sample_recorded_at_utc",
                "cancellation_terminal_max_working_set_bytes",
                "cancellation_terminal_max_private_memory_bytes",
                "full_window_disclosure",
            ):
                resource["memory"].pop(key)
                packaged["checks"]["resources"].pop(key)
            resource["memory"].update({
                "policy": resource_policy.POLICY_V2,
                "cancellation_within_baseline_tolerance": False,
                "conclusion": "bounded_post_replacement_recovery_v2",
            })
            resource["passed"] = False
            packaged["checks"]["resources"].update({
                "passed": False,
                "policy": resource_policy.POLICY_V2,
                "cancellation_within_baseline_tolerance": False,
                "conclusion": "bounded_post_replacement_recovery_v2",
            })
            packaged["passed"] = False
            write_json(resource_path, resource)
            packaged["artifacts"]["resource_report"] = artifact(resource_path, root)
            write_json(packaged_path, packaged)
            v2_resource_bytes = resource_path.read_bytes()
            v2_packaged_bytes = packaged_path.read_bytes()

            result = resource_policy.remint(root)
            current_resource = json.loads(resource_path.read_text(encoding="utf-8"))
            current_packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
            self.assertEqual(current_resource["memory"]["policy"], resource_policy.POLICY_V3)
            self.assertTrue(current_resource["passed"])
            self.assertTrue(current_packaged["passed"])
            self.assertEqual(Path(result["preserved_v2_resource_report"]).read_bytes(), v2_resource_bytes)
            self.assertEqual(Path(result["preserved_v2_packaged_report"]).read_bytes(), v2_packaged_bytes)
            schema = json.loads(
                (root / "validation/process_v2_packaged_acceptance.schema.json").read_text(
                    encoding="utf-8"
                )
            )
            self.assertEqual(list(Draft202012Validator(schema).iter_errors(current_packaged)), [])

    def test_resource_gate_rejects_early_or_cross_phase_checkpoint_samples(self) -> None:
        mutations = {
            "before_capture_delay": "2026-08-12T12:00:00.499Z",
            "at_next_phase": "2026-08-12T12:00:10Z",
        }
        for label, sample_timestamp in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory(
                prefix=f"quickpls-process-audit-checkpoint-{label}-"
            ) as directory:
                root = Path(directory)
                results = self.prepare_complete_evidence(root)
                packaged_path = results / audit.PACKAGED_REPORT
                packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
                resource_path = results / "process_v2_resource_report.json"
                resource = json.loads(resource_path.read_text(encoding="utf-8"))
                resource["idle_checkpoints"][0]["sample_recorded_at_utc"][0] = sample_timestamp
                write_json(resource_path, resource)
                packaged["artifacts"]["resource_report"] = artifact(resource_path, root)
                write_json(packaged_path, packaged)
                report = audit.build_audit(root, results)
                self.assertFalse(
                    report["packaged_attestation"]["checks"]["resource_report_independently_verified"]
                )
                self.assertFalse(report["passed"])

    def test_cleanup_gate_rejects_any_forced_termination(self) -> None:
        for field, value in (
            ("forced_parent_termination", True),
            ("forced_descendant_pids", [4321]),
            ("forced_resource_monitor_termination", True),
        ):
            with self.subTest(field=field), tempfile.TemporaryDirectory(
                prefix="quickpls-process-audit-cleanup-"
            ) as directory:
                root = Path(directory)
                results = self.prepare_complete_evidence(root)
                cleanup_path = results / audit.PROCESS_CLEANUP_REPORT
                cleanup = json.loads(cleanup_path.read_text(encoding="utf-8"))
                cleanup[field] = value
                write_json(cleanup_path, cleanup)
                report = audit.build_audit(root, results)
                self.assertFalse(report["packaged_attestation"]["checks"]["exact_pid_cleanup_confirmed"])
                self.assertFalse(report["passed"])

    def test_cancelled_retry_setup_snapshot_drift_cannot_pass(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-audit-retry-snapshot-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            packaged_path = results / audit.PACKAGED_REPORT
            packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
            packaged["checks"]["cancelled_retry_setup"]["snapshot"]["paths"][0]["to"] = "M1"
            write_json(packaged_path, packaged)
            report = audit.build_audit(root, results)
            self.assertFalse(report["packaged_attestation"]["checks"]["cancelled_retry_setup_contract"])
            self.assertFalse(report["packaged_attestation"]["checks"]["draft202012_schema_valid"])
            self.assertFalse(report["passed"])

    def test_resource_monitor_handshake_and_terminal_evidence_fail_closed(self) -> None:
        mutations = {
            "missing_root": lambda packaged, resource, cleanup: packaged["checks"]["resources"]["first_sample"].update({
                "root_present": False,
            }),
            "wrong_root_pid": lambda packaged, resource, cleanup: packaged["checks"]["resources"]["first_sample"].update({
                "root_pid": 4321,
            }),
            "monitor_error": lambda packaged, resource, cleanup: cleanup.update({
                "resource_monitor_exit_code": 1,
                "resource_monitor_stderr": "monitor failed",
                "resource_monitor_terminal_reason": "monitor_error",
            }),
            "cleanup_first_sample_drift": lambda packaged, resource, cleanup: cleanup["resource_monitor_first_sample"].update({
                "total_working_set_bytes": 199_999_999,
            }),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory(
                prefix=f"quickpls-process-audit-monitor-{label}-"
            ) as directory:
                root = Path(directory)
                results = self.prepare_complete_evidence(root)
                packaged_path = results / audit.PACKAGED_REPORT
                resource_path = results / "process_v2_resource_report.json"
                cleanup_path = results / audit.PROCESS_CLEANUP_REPORT
                packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
                resource = json.loads(resource_path.read_text(encoding="utf-8"))
                cleanup = json.loads(cleanup_path.read_text(encoding="utf-8"))
                mutate(packaged, resource, cleanup)
                write_json(packaged_path, packaged)
                write_json(resource_path, resource)
                write_json(cleanup_path, cleanup)
                report = audit.build_audit(root, results)
                self.assertFalse(report["passed"])
                self.assertTrue(
                    not report["packaged_attestation"]["checks"]["resource_contract"]
                    or not report["packaged_attestation"]["checks"]["exact_pid_cleanup_confirmed"]
                )

    def test_canonical_first_sample_role_counts_are_recomputed_from_raw_processes(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-audit-first-role-counts-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            packaged_path = results / audit.PACKAGED_REPORT
            resource_path = results / "process_v2_resource_report.json"
            cleanup_path = results / audit.PROCESS_CLEANUP_REPORT
            packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
            resource = json.loads(resource_path.read_text(encoding="utf-8"))
            cleanup = json.loads(cleanup_path.read_text(encoding="utf-8"))

            for first_sample in (
                packaged["checks"]["resources"]["first_sample"],
                resource["first_sample"],
                cleanup["resource_monitor_first_sample"],
            ):
                first_sample["process_role_counts"]["webview_gpu"] = 1

            write_json(resource_path, resource)
            packaged["artifacts"]["resource_report"] = artifact(resource_path, root)
            write_json(packaged_path, packaged)
            write_json(cleanup_path, cleanup)

            report = audit.build_audit(root, results)
            self.assertFalse(report["packaged_attestation"]["checks"]["resource_contract"])
            self.assertFalse(report["passed"])

    def test_documentation_requires_every_frozen_primary_reference_identifier(self) -> None:
        with tempfile.TemporaryDirectory(prefix="quickpls-process-audit-citations-") as directory:
            root = Path(directory)
            results = self.prepare_complete_evidence(root)
            document_path = root / "docs/methods/PROCESS_V2.md"
            text = document_path.read_text(encoding="utf-8")
            missing = audit.DOCUMENTATION_REFERENCE_IDENTIFIERS[0]
            document_path.write_text(text.replace(missing, "citation-removed"), encoding="utf-8")
            report = audit.build_audit(root, results)
            self.assertFalse(report["docs"]["reference_identifiers"][missing])
            self.assertFalse(report["passed"])

    def test_packaged_wrapper_and_resource_monitor_use_powershell_51_path_apis(self) -> None:
        for relative in (
            "validation/run_v247_process_v2_native_acceptance.ps1",
            "validation/monitor_quickpls_process_tree.ps1",
        ):
            with self.subTest(relative=relative):
                source = (VALIDATION.parent / relative).read_text(encoding="utf-8")
                self.assertNotIn("IsPathFullyQualified", source)
                self.assertIn("function Test-FullyQualifiedWindowsPath", source)
                self.assertIn("[System.IO.Path]::IsPathRooted", source)
                self.assertIn("[System.IO.Path]::GetFullPath", source)

    def test_resource_monitor_uses_powershell_51_safe_working_set_sum(self) -> None:
        source = (VALIDATION / "monitor_quickpls_process_tree.ps1").read_text(encoding="utf-8")
        self.assertNotIn("Measure-Object -Property working_set_bytes", source)
        self.assertIn('[long]$process["working_set_bytes"]', source)
        self.assertIn('[long]$process["private_memory_bytes"]', source)
        self.assertIn('[int]$process["handle_count"]', source)
        self.assertIn('[int]$process["thread_count"]', source)
        self.assertIn("process_role_counts = $roleCounts", source)
        self.assertIn('return "desktop_root"', source)
        self.assertIn('return "webview_renderer"', source)
        self.assertIn("did not exist after the bounded monitor startup retry", source)
        self.assertIn("disappeared or changed identity before the monitor stop signal", source)

    def test_wrapper_canonicalizes_reported_first_sample_roles_without_rewriting_raw_samples(self) -> None:
        source = (VALIDATION / "run_v247_process_v2_native_acceptance.ps1").read_text(
            encoding="utf-8"
        )
        canonicalize = "$resourceMonitorFirstSample.process_role_counts = [pscustomobject]("
        cleanup_projection = "$cleanup.resource_monitor_first_sample = $resourceMonitorFirstSample"
        self.assertIn(canonicalize, source)
        self.assertIn(cleanup_projection, source)
        self.assertLess(source.index(canonicalize), source.index(cleanup_projection))
        self.assertLess(source.index(cleanup_projection), source.index("$resourceReport = [ordered]@{"))
        self.assertNotIn("Set-Content -LiteralPath $resourceMonitorOutput", source)

    def test_resource_gate_uses_five_typed_windows_and_a_sidecar_free_equal_state_clone(self) -> None:
        harness = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        wrapper = (VALIDATION / "run_v247_process_v2_native_acceptance.ps1").read_text(encoding="utf-8")
        self.assertIn("processV2ResourceSampleWindowMilliseconds = 10_000", harness)
        self.assertIn("processV2ResourceSampleCaptureMilliseconds\n  + processV2ResourceSampleWindowMilliseconds", harness)
        for phase, state_kind in (
            ("initial_idle", "model_free_fixture"),
            ("post_cancellation_idle", "cancelled_setup_no_result"),
            ("post_completed_cycle_1_idle", "one_result_reopened_original"),
            ("post_completed_history_2_idle", "two_results_retained_history"),
            ("post_completed_cycle_2_idle", "one_result_reopened_reset_clone"),
        ):
            self.assertIn(f'markProcessV2ResourcePhase("{phase}"', harness)
            self.assertIn(f'state_kind: "{state_kind}"', harness)
        self.assertIn("await clearProcessV2ResetArtifacts(processV2ResetProjectPath)", harness)
        self.assertIn("fs.copyFile(processV2ProjectPath, processV2ResetProjectPath, fsConstants.COPYFILE_EXCL)", harness)
        self.assertIn("await openProjectAtExactPath(processV2ProjectName, processV2ResetProjectPath)", harness)
        self.assertIn("processV2SettledAutosaveState", harness)
        self.assertIn("autosaveAfterCheckpoint", harness)
        self.assertIn('}, `${processV2ProjectPath}.autosave`);', harness)
        self.assertIn('}, `${processV2ResetProjectPath}.autosave`);', harness)
        self.assertIn("function Select-ResourceSamplesAtPhase", wrapper)
        self.assertIn("$resourceCaptureDelayMilliseconds = 500", wrapper)
        self.assertIn("$resourceSampleWindowMilliseconds = 10000", wrapper)
        self.assertIn("$resourceMinimumSamplesPerCheckpoint = 6", wrapper)
        self.assertIn("$windowStart = $phaseTime.AddMilliseconds($resourceCaptureDelayMilliseconds)", wrapper)
        self.assertIn("$windowEnd = $windowStart.AddMilliseconds($resourceSampleWindowMilliseconds)", wrapper)
        self.assertIn("$windowSamples.Count -lt $resourceMinimumSamplesPerCheckpoint", wrapper)
        self.assertIn('$failureReasons.Add("insufficient_window_samples")', wrapper)
        self.assertIn("checkpoint_diagnostics = $checkpointDiagnostics", wrapper)
        self.assertIn("eligible_sample_recorded_at_utc", wrapper)
        self.assertIn("function Get-BoundedResourceRoleWindow", wrapper)
        self.assertIn('policy = "modal_pid_role_identity_with_bounded_webview_churn_v1"', wrapper)
        self.assertIn("modalSampleCount * 100 -ge $Samples.Count * 80", wrapper)
        self.assertIn("$firstThreeExactModal", wrapper)
        self.assertIn("$lastThreeExactModal", wrapper)
        self.assertNotIn("$firstThreeExactModal -and $lastThreeExactModal", wrapper)
        self.assertIn("$longestDeviationStreak -le 2", wrapper)
        self.assertIn("baseline_identities_never_removed_or_replaced", wrapper)
        self.assertIn("transient_processes = $transientProcesses", wrapper)
        self.assertIn("process_roles_bounded_and_terminally_stable", wrapper)
        self.assertNotIn("process_roles_stable_within_window", wrapper)
        self.assertIn("Get-MedianLong", wrapper)
        self.assertIn("Get-P95Long", wrapper)
        self.assertIn('policy = "bounded_equal_logical_state_terminal_stable_v3"', wrapper)
        self.assertIn('conclusion = "bounded_post_replacement_recovery_terminal_stable_v3"', wrapper)
        self.assertIn("$cancellationTerminalMinimumSamples = 6", wrapper)
        self.assertIn("$cancellationTerminalSamplesRoleStable", wrapper)
        self.assertIn("cancellation_terminal_max_working_set_bytes", wrapper)
        self.assertIn("full_window_disclosure = $fullWindowDisclosure", wrapper)
        self.assertIn("Get-ResourceRoleMedianDisclosure", wrapper)
        self.assertIn("resourceSamplesEvidencePath", wrapper)
        self.assertIn("snapshotProcessV2ResourceArchive", harness)
        self.assertIn("fsConstants.COPYFILE_EXCL", harness)
        self.assertIn("sourceStatBefore.mtimeNs === sourceStatAfter.mtimeNs", harness)
        self.assertIn("source_stable_during_copy: true", harness)
        self.assertIn("exclusive_atomic_copy: true", harness)
        self.assertIn("application_opened: false", harness)
        self.assertIn("const archiveState = effectiveArchive.logical_state", harness)
        self.assertIn("phase_snapshots = $phaseSnapshotArtifacts", wrapper)
        self.assertIn("phase_document = $resourcePhasesDescriptor", wrapper)
        self.assertIn("resourcePhasesEvidencePath", wrapper)
        self.assertIn("$phaseSnapshotArtifacts = @()", wrapper)
        self.assertLess(wrapper.index("$phaseSnapshotArtifacts = @()"), wrapper.index("$checkpointEvidence = @()"))
        self.assertNotIn("$phaseSnapshotArtifacts = @($checkpointEvidence", wrapper)
        self.assertIn("resource_phase_snapshots", wrapper)
        self.assertIn("resource_phases", wrapper)
        for obsolete in (
            "all_post_cycle_idles_within_baseline_tolerance",
            "final_not_above_maximum_prior_idle",
            "maximum_prior_idle_bytes",
        ):
            self.assertNotIn(obsolete, wrapper)

    def test_resource_phase_snapshot_identity_and_content_mutations_fail_closed(self) -> None:
        mutations = {
            "source_not_stable": lambda resource, packaged, results: resource["idle_checkpoints"][0][
                "effective_archive"
            ].update({"source_stable_during_copy": False}),
            "application_opened": lambda resource, packaged, results: resource["idle_checkpoints"][2][
                "effective_archive"
            ].update({"application_opened": True}),
            "history_count_drift": lambda resource, packaged, results: resource["idle_checkpoints"][3][
                "effective_archive"
            ]["logical_state"].update({"completedResultCount": 1}),
            "missing_packaged_snapshot": lambda resource, packaged, results: packaged["artifacts"][
                "resource_phase_snapshots"
            ].pop(),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory(
                prefix=f"quickpls-process-audit-phase-snapshot-{label}-"
            ) as directory:
                root = Path(directory)
                results = self.prepare_complete_evidence(root)
                packaged_path = results / audit.PACKAGED_REPORT
                resource_path = results / "process_v2_resource_report.json"
                packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
                resource = json.loads(resource_path.read_text(encoding="utf-8"))
                mutate(resource, packaged, results)
                write_json(resource_path, resource)
                packaged["artifacts"]["resource_report"] = artifact(resource_path, root)
                write_json(packaged_path, packaged)
                report = audit.build_audit(root, results)
                self.assertFalse(
                    report["packaged_attestation"]["checks"]["resource_phase_snapshots_bound"]
                    and report["packaged_attestation"]["checks"]["resource_contract"]
                )
                self.assertFalse(report["passed"])

    def test_bounded_webview_churn_requires_modal_terminal_identity_and_additive_children(self) -> None:
        base_processes = [
            {"pid": 100, "parent_pid": 1, "name": "quickpls-desktop.exe", "role": "desktop_root", "creation_date": "root", "working_set_bytes": 10, "private_memory_bytes": 10, "handle_count": 10, "thread_count": 2},
            {"pid": 200, "parent_pid": 100, "name": "msedgewebview2.exe", "role": "webview_browser", "creation_date": "browser", "working_set_bytes": 20, "private_memory_bytes": 20, "handle_count": 20, "thread_count": 3},
            {"pid": 300, "parent_pid": 200, "name": "msedgewebview2.exe", "role": "webview_renderer", "creation_date": "renderer", "working_set_bytes": 30, "private_memory_bytes": 30, "handle_count": 30, "thread_count": 4},
            {"pid": 400, "parent_pid": 200, "name": "msedgewebview2.exe", "role": "webview_gpu", "creation_date": "gpu", "working_set_bytes": 40, "private_memory_bytes": 40, "handle_count": 40, "thread_count": 5},
        ]

        def samples(count: int = 10) -> list[dict]:
            rows = []
            for index in range(count):
                processes = json.loads(json.dumps(base_processes))
                role_counts = {role: 0 for role in audit.RESOURCE_ROLE_NAMES}
                for process in processes:
                    role_counts[process["role"]] += 1
                rows.append({
                    "recorded_at_utc": f"2026-08-12T12:00:{index:02d}.000Z",
                    "root_present": True, "root_pid": 100,
                    "process_role_counts": role_counts, "processes": processes,
                })
            return rows

        def add_process(rows: list[dict], index: int, process: dict, update_counts: bool = True) -> None:
            rows[index]["processes"].append(process)
            if update_counts:
                rows[index]["process_role_counts"][process["role"]] += 1

        transient = {
            "pid": 700, "parent_pid": 200, "name": "msedgewebview2.exe", "role": "webview_gpu",
            "creation_date": "transient", "working_set_bytes": 7, "private_memory_bytes": 6,
            "handle_count": 5, "thread_count": 1,
        }
        accepted = samples()
        add_process(accepted, 6, transient)
        accepted_contract = audit.bounded_process_role_window(accepted)
        self.assertTrue(accepted_contract["passed"])
        self.assertEqual(accepted_contract["modal_sample_count"], 9)
        self.assertEqual(accepted_contract["deviating_sample_indices"], [6])
        self.assertEqual(accepted_contract["transient_identity_count"], 1)
        self.assertEqual(accepted_contract["transient_processes"][0]["pid"], 700)
        self.assertTrue(accepted_contract["transient_processes"][0]["descendant_of_persistent_browser"])

        mutations = {}
        terminal = samples()
        add_process(terminal, 9, transient)
        mutations["terminal_churn"] = terminal
        boundary = samples()
        add_process(boundary, 1, transient)
        boundary_contract = audit.bounded_process_role_window(boundary)
        self.assertTrue(boundary_contract["passed"])
        self.assertFalse(boundary_contract["first_three_exact_modal"])
        self.assertTrue(boundary_contract["last_three_exact_modal"])
        self.assertTrue(boundary_contract["transients_absent_terminal_three"])
        removed = samples()
        removed[6]["processes"] = [row for row in removed[6]["processes"] if row["pid"] != 300]
        removed[6]["process_role_counts"]["webview_renderer"] -= 1
        mutations["baseline_removal"] = removed
        root_replaced = samples()
        root_replaced[6]["processes"][0]["pid"] = 101
        root_replaced[6]["root_pid"] = 101
        mutations["root_replacement"] = root_replaced
        foreign = samples()
        foreign_process = {**transient, "pid": 701, "parent_pid": 100}
        add_process(foreign, 6, foreign_process)
        mutations["not_browser_descendant"] = foreign
        other = samples()
        other_process = {**transient, "pid": 702, "parent_pid": 100, "name": "helper.exe", "role": "other_descendant"}
        add_process(other, 6, other_process)
        mutations["other_descendant"] = other
        count_drift = samples()
        add_process(count_drift, 6, transient, update_counts=False)
        mutations["reported_role_count_drift"] = count_drift
        streak = samples(20)
        for index in (8, 9, 10):
            add_process(streak, index, {**transient, "pid": 710 + index, "creation_date": f"transient-{index}"})
        mutations["three_sample_streak"] = streak
        five_samples = samples(5)
        mutations["fewer_than_six"] = five_samples
        for label, rows in mutations.items():
            with self.subTest(label=label):
                self.assertFalse(audit.bounded_process_role_window(rows)["passed"])

    def test_cycle1_autosave_symmetry_and_captured_sidecars_fail_closed(self) -> None:
        for label in ("before_after_drift", "captured_digest_drift"):
            with self.subTest(label=label), tempfile.TemporaryDirectory(
                prefix=f"quickpls-process-audit-cycle1-autosave-{label}-"
            ) as directory:
                root = Path(directory)
                results = self.prepare_complete_evidence(root)
                packaged_path = results / audit.PACKAGED_REPORT
                packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
                if label == "before_after_drift":
                    packaged["checks"]["save_reopen"]["cycle_1_autosave_after_checkpoint"][
                        "forbidden"
                    ] = ["validation/results/forged.transaction.json"]
                else:
                    capture = packaged["checks"]["save_reopen"]["cycle_1_settled_autosave"][
                        "capturedArtifacts"
                    ][0]["snapshot"]
                    (root / capture["path"]).write_bytes(b"tampered-cycle1-sidecar")
                write_json(packaged_path, packaged)
                report = audit.build_audit(root, results)
                self.assertFalse(report["packaged_attestation"]["checks"]["save_reopen_contract"])
                self.assertFalse(report["passed"])

    def test_resource_phase_document_and_checkpoint_diagnostics_fail_closed(self) -> None:
        for label in ("phase_document_window_drift", "diagnostic_sample_count_drift"):
            with self.subTest(label=label), tempfile.TemporaryDirectory(
                prefix=f"quickpls-process-audit-phase-diagnostic-{label}-"
            ) as directory:
                root = Path(directory)
                results = self.prepare_complete_evidence(root)
                packaged_path = results / audit.PACKAGED_REPORT
                resource_path = results / "process_v2_resource_report.json"
                phase_path = results / "process_v2_resource_phases.json"
                packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
                resource = json.loads(resource_path.read_text(encoding="utf-8"))
                if label == "phase_document_window_drift":
                    phases = json.loads(phase_path.read_text(encoding="utf-8"))
                    phases["phases"]["initial_idle"]["sample_window_milliseconds"] = 2_000
                    write_json(phase_path, phases)
                    phase_descriptor = artifact(phase_path, root)
                    resource["phase_document"] = phase_descriptor
                    packaged["artifacts"]["resource_phases"] = phase_descriptor
                else:
                    diagnostic = resource["checkpoint_diagnostics"][0]
                    diagnostic["eligible_sample_count"] = 5
                    diagnostic["passed"] = False
                    diagnostic["failure_reasons"] = ["insufficient_window_samples"]
                write_json(resource_path, resource)
                packaged["artifacts"]["resource_report"] = artifact(resource_path, root)
                write_json(packaged_path, packaged)
                report = audit.build_audit(root, results)
                self.assertFalse(report["packaged_attestation"]["checks"]["resource_contract"])
                self.assertFalse(report["passed"])

    def test_resource_raw_sample_and_reset_identity_mutations_fail_closed(self) -> None:
        for label in ("raw_sample", "reset_digest"):
            with self.subTest(label=label), tempfile.TemporaryDirectory(
                prefix=f"quickpls-process-audit-resource-{label}-"
            ) as directory:
                root = Path(directory)
                results = self.prepare_complete_evidence(root)
                packaged_path = results / audit.PACKAGED_REPORT
                packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
                if label == "raw_sample":
                    samples_path = results / "process_v2_resource_samples.jsonl"
                    rows = [json.loads(line) for line in samples_path.read_text(encoding="utf-8").splitlines()]
                    rows[1]["total_private_memory_bytes"] += 500_000_000
                    rows[1]["processes"][0]["private_memory_bytes"] += 500_000_000
                    samples_path.write_text("\n".join(json.dumps(row) for row in rows) + "\n", encoding="utf-8")
                    packaged["artifacts"]["resource_samples"] = artifact(samples_path, root)
                else:
                    packaged["checks"]["resource_reset"]["reset_archive"]["sha256"] = "0" * 64
                write_json(packaged_path, packaged)
                report = audit.build_audit(root, results)
                self.assertFalse(report["packaged_attestation"]["checks"]["resource_contract"] if label == "raw_sample" else report["packaged_attestation"]["checks"]["resource_reset_contract"])
                self.assertFalse(report["passed"])

    def test_process_cancellation_reuses_frozen_setup_through_explicit_retry(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        cancellation_start = source.index("const cancelledSetup = await configure();")
        full_setup = source.index("const fullSetup = {", cancellation_start)
        cancellation_slice = source[cancellation_start:full_setup]
        self.assertIn('name: "Retry graph-defined path analysis with bootstrap"', cancellation_slice)
        self.assertNotIn("const fullSetup = await configure();", source)
        self.assertNotIn(
            'cancelledSetup.calculation.getByRole("button", { name: "Close", exact: true }).click()',
            cancellation_slice,
        )
        self.assertIn("persisted path rows drifted before retry", source)
        self.assertIn("persisted moderator rows drifted before retry", source)
        self.assertIn("persisted moderation rows drifted before retry", source)
        self.assertIn("const preRetrySnapshot = await readProcessV2SetupSnapshot", source)
        self.assertIn("JSON.stringify(preRetrySnapshot) === JSON.stringify(cancelledSetup.contract)", source)
        self.assertIn('readOnly: true', source)

    def test_active_lifecycle_capture_ignores_stale_cancelled_state(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        helper_start = source.index("async function captureActiveCalculation")
        helper_end = source.index("async function openResultTable", helper_start)
        helper = source[helper_start:helper_end]
        self.assertIn('.nd-run-progress[aria-busy="true"]:is(.queued,.validating,.running,.cancelling)', helper)
        self.assertIn('ariaBusy: element.getAttribute("aria-busy")', helper)
        self.assertIn('if (state.ariaBusy !== "true" || !state.status)', helper)
        self.assertIn("if (!stillActive && !allowTerminalTransitionAfterCapture)", helper)
        self.assertNotIn('dialog.locator(".nd-run-progress")', helper)

    def test_packaged_harness_preflights_single_production_tauri_page_before_process_mutation(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        self.assertIn('const packagedTauriOrigin = "http://tauri.localhost";', source)
        self.assertIn("browserInstance.contexts().flatMap((context) => context.pages())", source)
        self.assertIn("if (qualifying.length !== 1)", source)
        self.assertIn("runtimePreflight.reloadCount = 1", source)
        self.assertIn("postReload.origin === packagedTauriOrigin", source)
        self.assertIn(
            "const isolatedFocusedOnly = mgaOnly || hocOnly || predictionOnly || cbsemOnly || pcaOnly || olsOnly",
            source,
        )
        self.assertIn(
            "|| logisticOnly || regressionBootstrapOnly || ncaOnly || ctaPlsOnly || processV2Only",
            source,
        )
        self.assertIn("|| structuralPathRandomizationOnly || gscaOnly;", source)
        self.assertIn("const inheritPriorEvidence = focusedOnly && !isolatedFocusedOnly;", source)
        self.assertIn("if (isolatedFocusedOnly && scopedReportPath !== reportPath)", source)
        self.assertLess(
            source.index("evidence.checks.runtimePreflight.passed ="),
            source.index("evidence.checks.processV2ReferenceFixture ="),
        )

    def test_packaged_wrapper_restores_prior_environment_even_when_launch_fails(self) -> None:
        source = (VALIDATION / "run_v247_process_v2_native_acceptance.ps1").read_text(encoding="utf-8")
        self.assertNotIn("-DateKind", source)
        self.assertIn("$application = $null", source)
        self.assertIn("$priorAcceptanceEnvironment = @{}", source)
        self.assertIn("[Environment]::SetEnvironmentVariable", source)
        self.assertIn("$resourceMonitorReady = $false", source)
        self.assertIn("QuickPLS resource monitor did not produce its first sample", source)
        self.assertIn("-RedirectStandardError $resourceMonitorStderrPath", source)
        self.assertIn("$candidateSample.root_present -eq $true", source)
        self.assertIn("[int]$candidateSample.root_pid -eq $application.Id", source)
        self.assertIn("[long]$candidateSample.total_working_set_bytes -gt 0", source)
        self.assertIn('resource_monitor_terminal_reason = $null', source)
        self.assertEqual(source.count("[string]$monitorStderrText = Get-Content"), 2)
        self.assertEqual(source.count("$monitorStderrText.Trim()"), 2)
        self.assertIn(
            "$cleanup.resource_monitor_stderr = if ($null -eq $monitorStderrText)",
            source,
        )
        self.assertNotIn("[string](Get-Content", source)
        self.assertIn("$monitorProcess = $resourceMonitor", source)
        monitor_start = source.index("$resourceMonitor = Start-Process")
        handle_capture = source.index("$resourceMonitorHandle = $resourceMonitor.Handle", monitor_start)
        monitor_ready = source.index("$resourceMonitorReady = $false", handle_capture)
        self.assertLess(monitor_start, handle_capture)
        self.assertLess(handle_capture, monitor_ready)
        self.assertIn("$monitorExitConfirmed = $monitorProcess.WaitForExit(5000)", source)
        self.assertIn("$cleanup.resource_monitor_exit_confirmed = [bool]$monitorExitConfirmed", source)
        self.assertIn("$monitorProcess.WaitForExit()", source)
        self.assertIn("$monitorProcess.Refresh()", source)
        self.assertIn("$capturedMonitorExitCode = $monitorProcess.ExitCode", source)
        self.assertIn("$cleanup.resource_monitor_exit_code = [int]$capturedMonitorExitCode", source)
        self.assertIn("$monitorPidAbsent = -not [bool](Get-Process", source)
        self.assertIn("$monitorPidAbsent -and", source)
        self.assertIn('"exit_code_unavailable"', source)
        self.assertNotIn("$cleanup.resource_monitor_exit_confirmed = -not [bool](Get-Process", source)
        monitor_cleanup = source.index("try {", source.index("$cleanup = [ordered]@{"))
        application_cleanup = source.index("if ($application -and -not $application.HasExited)", monitor_cleanup)
        self.assertIn("$cleanup.resource_monitor_terminal_reason = if ($monitorExitCodeUnavailable)", source[monitor_cleanup:application_cleanup])
        self.assertIn('"exit_code_unavailable"', source[monitor_cleanup:application_cleanup])
        self.assertIn('"monitor_error"', source[monitor_cleanup:application_cleanup])
        self.assertIn("} catch {", source[monitor_cleanup:application_cleanup])
        env_assignment = source.index('$env:QUICKPLS_ACCEPTANCE_SCOPE = "process_v2"')
        launch = source.index("$application = Start-Process")
        outer_try = source.rfind("try {", 0, env_assignment)
        self.assertGreaterEqual(outer_try, 0)
        self.assertLess(outer_try, env_assignment)
        self.assertLess(env_assignment, launch)
        self.assertNotIn("Remove-Item `\n        Env:QUICKPLS_ACCEPTANCE_SCOPE", source)


if __name__ == "__main__":
    unittest.main()
