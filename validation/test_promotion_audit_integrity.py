#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import os
import re
import sys
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
sys.path.insert(0, str(VALIDATION))

from performance_release_publication_audit import verify_release_artifacts
from promotion_audit_integrity import (
    evaluate_document,
    evaluate_report,
    explicit_pass_state,
    report_passed,
    write_method_audit,
)


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


class ExplicitPassStateTests(unittest.TestCase):
    def test_checks_without_overall_state_fail_closed(self) -> None:
        self.assertFalse(explicit_pass_state({"checks": {}})["passed"])
        self.assertFalse(explicit_pass_state({"checks": {"x": {"passed": True}}})["passed"])

    def test_explicit_positive_state_passes(self) -> None:
        self.assertTrue(explicit_pass_state({"passed": True})["passed"])
        self.assertTrue(explicit_pass_state({"status": "passed"})["passed"])
        self.assertTrue(explicit_pass_state({"qualification": {"passed": True}})["passed"])

    def test_contradictory_explicit_states_fail(self) -> None:
        state = explicit_pass_state({"passed": True, "qualification_passed": False})
        self.assertTrue(state["present"])
        self.assertFalse(state["passed"])

    def test_promotion_ready_requires_artifact_states(self) -> None:
        self.assertFalse(explicit_pass_state({"promotion_ready": True})["passed"])
        self.assertTrue(explicit_pass_state({
            "promotion_ready": True,
            "all_listed_artifacts_present": True,
            "all_listed_artifacts_passed": True,
        })["passed"])


class EvidenceBindingTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.results = self.root / "validation" / "results"
        self.results.mkdir(parents=True)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_missing_and_malformed_reports_fail_closed(self) -> None:
        self.assertFalse(evaluate_report(self.root, self.results, "missing.json")["passed"])
        malformed = self.results / "malformed.json"
        malformed.write_text("{", encoding="utf-8")
        self.assertFalse(evaluate_report(self.root, self.results, "malformed.json")["passed"])
        self.assertFalse(report_passed(malformed))

    def test_method_version_companion_and_hash_are_bound(self) -> None:
        write_json(self.results / "method.json", {
            "passed": True,
            "kind": "reference_v1",
            "checks": {"method": {"passed": True, "method_version": "method_v1"}},
        })
        write_json(self.results / "result.json", {
            "status": "completed",
            "payload": {"method_version": "method_v1"},
        })
        evidence = evaluate_report(self.root, self.results, {
            "name": "method.json",
            "required_values": {
                "kind": "reference_v1",
                "checks.method.method_version": "method_v1",
            },
            "required_true": ["checks.method.passed"],
            "companions": [{
                "path": "validation/results/result.json",
                "required_values": {
                    "status": "completed",
                    "payload.method_version": "method_v1",
                },
            }],
        })
        self.assertTrue(evidence["passed"])
        self.assertRegex(evidence["sha256"], r"^[0-9a-f]{64}$")
        self.assertRegex(evidence["companions"][0]["sha256"], r"^[0-9a-f]{64}$")

        companion = json.loads((self.results / "result.json").read_text(encoding="utf-8"))
        companion["payload"]["method_version"] = "wrong_v0"
        write_json(self.results / "result.json", companion)
        self.assertFalse(evaluate_report(self.root, self.results, {
            "name": "method.json",
            "companions": [{
                "path": "validation/results/result.json",
                "required_values": {"payload.method_version": "method_v1"},
            }],
        })["passed"])

    def test_source_newer_than_report_is_stale(self) -> None:
        report = self.results / "reference.json"
        source = self.root / "validation" / "reference.py"
        write_json(report, {"passed": True})
        source.write_text("# source\n", encoding="utf-8")
        os.utime(report, (1_700_000_000, 1_700_000_000))
        os.utime(source, (1_700_000_100, 1_700_000_100))
        evidence = evaluate_report(
            self.root,
            self.results,
            {"name": "reference.json", "source_paths": ["validation/reference.py"]},
            now=datetime(2026, 8, 12, tzinfo=timezone.utc),
        )
        self.assertFalse(evidence["freshness"]["passed"])
        self.assertFalse(evidence["passed"])

    def test_required_list_item_must_exist_once_and_pass(self) -> None:
        write_json(self.results / "manifest.json", {
            "passed": True,
            "artifacts": [{"file": "a.json", "present": True, "passed": True}],
        })
        spec = {
            "name": "manifest.json",
            "required_list_items": [{
                "path": "artifacts",
                "where": {"file": "a.json"},
                "required_true": ["present", "passed"],
            }],
        }
        self.assertTrue(evaluate_report(self.root, self.results, spec)["passed"])
        write_json(self.results / "manifest.json", {"passed": True, "artifacts": []})
        self.assertFalse(evaluate_report(self.root, self.results, spec)["passed"])

    def test_document_requires_semantic_phrases(self) -> None:
        doc = self.root / "docs" / "methods" / "METHOD.md"
        doc.parent.mkdir(parents=True)
        doc.write_text("Method v1 is diagnostic, not causal proof.", encoding="utf-8")
        self.assertTrue(evaluate_document(self.root, {
            "name": "METHOD.md",
            "required_phrases": ["method v1", "not causal proof"],
        })["passed"])
        self.assertFalse(evaluate_document(self.root, {
            "name": "METHOD.md",
            "required_phrases": ["bootstrap qualified"],
        })["passed"])

    def test_method_audit_writes_failed_state_for_unqualified_report(self) -> None:
        write_json(self.results / "checks_only.json", {"checks": {"x": {"passed": True}}})
        doc = self.root / "docs" / "methods" / "METHOD.md"
        doc.parent.mkdir(parents=True)
        doc.write_text("bounded scope", encoding="utf-8")
        code = write_method_audit(
            target="test",
            method_id="method",
            promoted_scope="bounded",
            required_reports=["checks_only.json"],
            required_docs=[{"name": "METHOD.md", "required_phrases": ["bounded scope"]}],
            root=self.root,
            results=self.results,
        )
        self.assertEqual(code, 1)
        output = json.loads((self.results / "method_method_promotion_audit.json").read_text(encoding="utf-8"))
        self.assertFalse(output["passed"])
        self.assertEqual(output["integrity_contract"], "explicit_pass_state_and_bound_evidence_v1")


class ReleaseArtifactIntegrityTests(unittest.TestCase):
    def test_release_artifacts_require_real_hash_matched_setup_and_portable(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            results = root / "validation" / "results"
            release = root / "release"
            results.mkdir(parents=True)
            release.mkdir()
            portable = release / "QuickPLS_3.0.0_x64_portable.exe"
            setup = release / "QuickPLS_3.0.0_x64_setup.exe"
            portable.write_bytes(b"portable-binary")
            setup.write_bytes(b"setup-binary")
            portable_hash = hashlib.sha256(portable.read_bytes()).hexdigest()
            setup_hash = hashlib.sha256(setup.read_bytes()).hexdigest()
            checksums = release / "QuickPLS_3.0.0_x64_checksums.txt"
            checksums.write_text(
                f"{portable_hash}  {portable.name}\n{setup_hash}  {setup.name}\n",
                encoding="utf-8",
            )
            checksum_hash = hashlib.sha256(checksums.read_bytes()).hexdigest()
            manifest = {
                "schema_version": 1,
                "target": "test release",
                "passed": True,
                "version": "3.0.0",
                "timestamp_utc": "2026-08-12T00:00:00Z",
                "artifacts": [
                    {
                        "path": f"target/release/artifacts/{portable.name}",
                        "bytes": portable.stat().st_size,
                        "sha256": portable_hash,
                    },
                    {
                        "path": f"target/release/artifacts/{setup.name}",
                        "bytes": setup.stat().st_size,
                        "sha256": setup_hash,
                    },
                    {
                        "path": f"target/release/artifacts/{checksums.name}",
                        "bytes": checksums.stat().st_size,
                        "sha256": checksum_hash,
                    },
                ],
            }
            manifest_path = results / "release_artifacts.json"
            write_json(manifest_path, manifest)
            evidence = verify_release_artifacts(
                manifest_path,
                release,
                "3.0.0",
                root=root,
                results=results,
            )
            self.assertTrue(evidence["passed"])
            setup.write_bytes(b"tampered")
            self.assertFalse(verify_release_artifacts(
                manifest_path,
                release,
                "3.0.0",
                root=root,
                results=results,
            )["passed"])


class StaticPromotionScriptTests(unittest.TestCase):
    def test_native_randomization_catalog_uses_exact_semantic_scope_contract(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        for token in (
            r"/single-model Freedman(?:\u2013|-|\s)Lane randomization/i.test(pathRandomizationDescription)",
            r"/structural paths/i.test(pathRandomizationDescription)",
            r"/fixed original PLS construct scores/i.test(pathRandomizationDescription)",
            r"/unadjusted pathwise p values/i.test(pathRandomizationDescription)",
            r"/\bMGA\b|\bMICOM\b/i.test(pathRandomizationDescription)",
            "did not preserve its required single-model Freedman-Lane structural-path, fixed-score, unadjusted pathwise scope, or mentioned MGA/MICOM",
        ):
            self.assertIn(token, source)
        self.assertNotIn(
            r"randomization inference/i.test(pathRandomizationDescription)",
            source,
        )

        valid = (
            "Structural Path Randomization Run candidate single-model Freedman-Lane "
            "randomization for structural paths using fixed original PLS construct scores "
            "and unadjusted pathwise p values."
        )

        def preserves_scope(description: str) -> bool:
            return (
                re.search(r"single-model Freedman(?:\u2013|-|\s)Lane randomization", description, re.IGNORECASE)
                is not None
                and re.search(r"structural paths", description, re.IGNORECASE) is not None
                and re.search(r"fixed original PLS construct scores", description, re.IGNORECASE) is not None
                and re.search(r"unadjusted pathwise p values", description, re.IGNORECASE) is not None
                and re.search(r"\bMGA\b|\bMICOM\b", description, re.IGNORECASE) is None
            )

        self.assertTrue(preserves_scope(valid))
        for mutation in (
            valid.replace("single-model Freedman-Lane randomization", "path randomization"),
            valid.replace("structural paths", "parameters"),
            valid.replace("fixed original PLS construct scores", "construct scores"),
            valid.replace("unadjusted pathwise p values", "p values"),
            f"{valid} This is MGA.",
            f"{valid} This is MICOM.",
        ):
            self.assertFalse(preserves_scope(mutation), mutation)

    def test_native_recent_project_selection_binds_exact_name_and_current_path(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        for token in (
            "function exactRecentProjectRow(projectName, projectPath)",
            "Recent-project selection for ${projectName} requires an exact project path.",
            'page.locator("strong").filter({ hasText: exactVisibleText(projectName) })',
            'page.locator("small").filter({ hasText: exactVisibleText(projectPath) })',
            "async function openRecentProject(projectName, projectPath)",
            "exactRecentProjectRow(mgaProjectName, mgaProjectPath)",
        ):
            self.assertIn(token, source)

        def unscoped_open_calls(candidate: str) -> list[str]:
            return re.findall(
                r"await openRecentProject\(\s*[^,\n()]+\s*\);",
                candidate,
            )

        self.assertEqual(unscoped_open_calls(source), [])
        mutated = source.replace(
            "openRecentProject(mgaProjectName, mgaProjectPath)",
            "openRecentProject(mgaProjectName)",
            1,
        )
        self.assertEqual(
            unscoped_open_calls(mutated),
            ["await openRecentProject(mgaProjectName);"],
        )

    def test_native_mga_catalog_reuses_the_canonical_method_manifest(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        contract = "JSON.stringify(mgaMethodNames) !== JSON.stringify(expectedOptionLabels)"
        self.assertIn(contract, source)
        self.assertIn(
            "did not preserve the canonical ${expectedOptionLabels.length}-method catalog with the joint MICOM/MGA entry",
            source,
        )
        self.assertNotIn("const expectedMgaMethodNames = [", source)
        self.assertNotIn("did not expose exactly ten truthful methods", source)

        mutated = source.replace(contract, "mgaMethodNames.length !== 10", 1)
        self.assertNotIn(contract, mutated)
        self.assertIn("mgaMethodNames.length !== 10", mutated)

    def test_focused_dialogs_reuse_the_canonical_method_count(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        contracts = (
            "evidence.checks.cbsemDialog.catalogCount !== expectedOptionLabels.length",
            "evidence.checks.gscaDialog.catalogCount !== expectedOptionLabels.length",
            "evidence.checks.olsDialog.catalogCount !== expectedOptionLabels.length",
            "evidence.checks.logisticDialog.catalogCount !== expectedOptionLabels.length",
            "evidence.checks.ctaPlsDialog.catalogCount !== expectedOptionLabels.length",
            "evidence.checks.pcaDialog.catalogCount !== expectedOptionLabels.length",
            "contract.catalogCount !== expectedOptionLabels.length",
            "contract.catalogCount === expectedOptionLabels.length",
        )
        for contract in contracts:
            self.assertIn(contract, source)
            mutated = source.replace(contract, "contract.catalogCount !== 14", 1)
            self.assertNotIn(contract, mutated)
            self.assertIn("contract.catalogCount !== 14", mutated)

        self.assertNotRegex(source, r"catalogCount\s*(?:!==|===)\s*\d+\b")

    def test_native_mga_archive_inspection_requires_typed_schema_v3_config(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        for token in (
            "const methodConfig = recipe?.method_config;",
            '"configural_invariance_confirmed",',
            '"group_column",',
            '"methods",',
            '"permutation_samples",',
            'JSON.stringify(methodConfig.methods) === JSON.stringify(["micom", "mga_permutation"])',
            "contract.recipe?.schemaVersion !== 3",
            'contract.recipe?.methodConfigKind !== "mga"',
            "contract.recipe?.groupPermutationSamples !== mgaRuntimePermutationSamples",
            "contract.recipe?.configuralConfirmed !== true",
        ):
            self.assertIn(token, source)
        for legacy_key in (
            "recipe.metadata?.group_methods",
            "recipe.metadata?.group_permutation_samples",
            "recipe.metadata?.micom_configural_confirmed",
            "recipe.metadata?.mga_group_column",
            "recipe.metadata?.mga_group_a",
            "recipe.metadata?.mga_group_b",
        ):
            self.assertNotIn(legacy_key, source)

        typed_declaration = "const methodConfig = recipe?.method_config;"
        typed_declaration_count = source.count(typed_declaration)
        self.assertGreaterEqual(typed_declaration_count, 1)
        mutated = source.replace(typed_declaration, "const methodConfig = recipe?.metadata;", 1)
        self.assertEqual(mutated.count(typed_declaration), typed_declaration_count - 1)
        self.assertEqual(mutated.count("const methodConfig = recipe?.metadata;"), 1)

    def test_native_current_archive_inspectors_do_not_execute_free_form_metadata(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        legacy_tokens = (
            "recipe.metadata?.ipma_targets",
            "recipe.metadata?.nca_x",
            "recipe.metadata?.nca_y",
            "recipe.metadata?.nca_ceiling",
            "recipe.metadata?.nca_permutation_samples",
            "recipe.metadata?.pca_variables",
            "recipe.metadata?.pca_component_rule",
            "recipe.metadata?.pca_variance_threshold",
            "recipe.metadata?.regression_type",
            "recipe.metadata?.regression_outcome",
            "recipe.metadata?.regression_predictors",
            "recipe.metadata?.regression_controls",
            "recipe.metadata?.robust_se",
            "recipe.metadata?.cbsem_model_type",
            "recipe.metadata?.cbsem_estimator",
            "recipe.metadata?.cbsem_input",
            "recipe.metadata?.cbsem_mean_structure",
        )
        for token in legacy_tokens:
            self.assertNotIn(token, source)
        for token in (
            "ipmaTargets: methodConfig?.targets ?? null",
            "ncaX: methodConfig?.condition ?? null",
            "ncaPermutationSamples: methodConfig?.permutation_samples ?? null",
            "variables: methodConfig?.variables ?? null",
            "componentRule: retention?.rule ?? null",
            "regressionType: regressionModel?.type ?? null",
            "robustSe: regressionModel?.robust_se ?? null",
            "modelType: methodConfig?.model_type ?? null",
            "methodBootstrapSamples: methodConfig?.bootstrap_samples ?? null",
        ):
            self.assertIn(token, source)

        mutation = source.replace(
            "ipmaTargets: methodConfig?.targets ?? null",
            "ipmaTargets: recipe.metadata?.ipma_targets ?? null",
            1,
        )
        self.assertTrue(any(token in mutation for token in legacy_tokens))

    def test_gsca_archive_accepts_only_absent_or_empty_legacy_analysis_shells(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        for shell in ("mediation", "moderation"):
            absent = f"estimation?.{shell} == null"
            empty = (
                f"Array.isArray(estimation.{shell}.estimates) "
                f"&& estimation.{shell}.estimates.length === 0"
            )
            self.assertIn(absent, source)
            self.assertIn(empty, source)

            mutated = source.replace(absent, "true", 1)
            self.assertNotIn(absent, mutated)
            self.assertIn(empty, mutated)

        self.assertNotIn("estimation?.mediation?.estimates?.length ?? -1", source)
        self.assertNotIn("estimation?.moderation?.estimates?.length ?? -1", source)

    def test_standard_logistic_archive_does_not_reference_bootstrap_witness(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        logistic_inspector = source[
            source.index("async function inspectSavedLogisticArchive") : source.index(
                "function exactZeroBasedPartition"
            )
        ]
        self.assertNotIn("witness", logistic_inspector)
        self.assertIn("resultCount: project.results?.length ?? null", logistic_inspector)
        self.assertIn("recipeCount: project.recipes?.length ?? null", logistic_inspector)

        mutated = logistic_inspector.replace(
            "recipeCount: project.recipes?.length ?? null,",
            "recipeCount: project.recipes?.length ?? null,\n      witnessCount: witness ? 1 : 0,",
            1,
        )
        self.assertIn("witnessCount: witness ? 1 : 0", mutated)

    def test_native_nca_caption_matches_the_ascii_product_contract(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        contract = 'plotContract.captionPair !== "x -> y"'
        self.assertIn(contract, source)
        self.assertNotIn(r'plotContract.captionPair !== "x \u2192 y"', source)

        mutated = source.replace(contract, r'plotContract.captionPair !== "x \u2192 y"', 1)
        self.assertNotIn(contract, mutated)
        self.assertIn(r'plotContract.captionPair !== "x \u2192 y"', mutated)

    def test_native_reopen_catalog_uses_frozen_catalog_identity(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        self.assertIn(
            "reopenedOptionLabels.length !== expectedOptionLabels.length",
            source,
        )
        self.assertIn(
            "JSON.stringify(reopenedOptionLabels) !== JSON.stringify(expectedOptionLabels)",
            source,
        )
        self.assertIn(
            "expected ${expectedOptionLabels.length}-method catalog: ${reopenedOptionLabels.join",
            source,
        )
        self.assertNotIn(
            'plscListbox.getByRole("option").count() !== 10',
            source,
        )

    def test_fast_gsca_logistic_plsc_wpls_and_prediction_lifecycle_transitions_require_completed_run_proof(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        self.assertIn(
            "{ allowTerminalTransitionAfterCapture = false } = {}",
            source,
        )
        self.assertIn(
            "if (!stillActive && !allowTerminalTransitionAfterCapture)",
            source,
        )
        self.assertEqual(
            source.count("{ allowTerminalTransitionAfterCapture: true }"),
            7,
        )
        for method_label in (
            "Structural Path Randomization 10,000-permutation run",
            "GSCA",
            "binary logistic regression",
            "Consistent PLS",
            "Weighted PLS",
            "PLSpredict / CVPAT retry",
        ):
            self.assertIn(
                f'"{method_label}",\n    {{ allowTerminalTransitionAfterCapture: true }},',
                source,
            )
        self.assertIn(
            "evidence.checks.logisticProgress.completedRunProof = {",
            source,
        )
        self.assertIn(
            'matched: logisticRunLabel === "Binary Logistic Regression run"',
            source,
        )
        self.assertIn(
            "if (!evidence.checks.logisticProgress.completedRunProof.matched)",
            source,
        )
        self.assertIn(
            "evidence.checks.gscaProgress.completedRunProof = {",
            source,
        )
        self.assertIn(
            "if (!evidence.checks.gscaProgress.completedRunProof.matched)",
            source,
        )
        self.assertIn(
            "evidence.checks.plscProgress.completedRunProof = {",
            source,
        )
        self.assertIn(
            "if (!evidence.checks.plscProgress.completedRunProof.matched)",
            source,
        )
        self.assertIn(
            "evidence.checks.wplsProgress.completedRunProof = {",
            source,
        )
        self.assertIn(
            "if (!evidence.checks.wplsProgress.completedRunProof.matched)",
            source,
        )
        self.assertIn(
            "evidence.checks.predictionProgress.completedRunProof = {",
            source,
        )
        self.assertIn(
            "if (!evidence.checks.predictionProgress.completedRunProof.matched)",
            source,
        )
        self.assertIn(
            "evidence.checks.predictionV2Progress.completedRunProof = {",
            source,
        )
        self.assertIn(
            "if (!evidence.checks.predictionV2Progress.completedRunProof.matched)",
            source,
        )
        self.assertIn(
            'predictionCaptureName(93, "running"),\n    "PLSpredict / CVPAT retry",\n    { allowTerminalTransitionAfterCapture: true },',
            source,
        )

    def test_prediction_result_identity_uses_exact_cells_not_concatenated_text(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        self.assertIn(
            'const displayedIndicators = indicatorCells.map((cells) => cells[1] ?? "");',
            source,
        )
        self.assertIn(
            'JSON.stringify(displayedIndicators) !== JSON.stringify(["y1", "y2"])',
            source,
        )
        self.assertIn(
            ".every((cell) => !/construct-/i.test(cell));",
            source,
        )
        self.assertNotIn("!/\\by1\\b/.test(indicatorText)", source)
        self.assertNotIn("!/\\by2\\b/.test(indicatorText)", source)

    def test_deleted_source_model_routes_historical_result_to_edit_data(self) -> None:
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        self.assertIn(
            'const resultDataCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Data$/ });',
            source,
        )
        self.assertIn(
            "const editModelCount = await resultModelCommand.count();",
            source,
        )
        self.assertIn(
            "const editDataCount = await resultDataCommand.count();",
            source,
        )
        self.assertIn("editDataVisible: editDataCount === 1", source)
        self.assertIn("editDataEnabled: editDataCount === 1", source)
        self.assertIn(
            "workspaceExplorerHistoricalResult.editModelCount !== 0",
            source,
        )
        self.assertIn(
            "workspaceExplorerHistoricalResult.editDataCount !== 1",
            source,
        )
        self.assertNotIn("editDeletedModelDisabled", source)

    def test_targeted_promotion_scripts_have_no_literal_passing_checks(self) -> None:
        targets = [
            "logistic_method_promotion_audit.py",
            "process_method_promotion_audit.py",
            "cta_pls_method_promotion_audit.py",
            "endogeneity_method_promotion_audit.py",
            "nonlinear_effects_method_promotion_audit.py",
            "moderated_mediation_method_promotion_audit.py",
            "fimix_pls_method_promotion_audit.py",
            "pls_pos_method_promotion_audit.py",
            "performance_release_publication_audit.py",
        ]
        for name in targets:
            with self.subTest(name=name):
                source = (VALIDATION / name).read_text(encoding="utf-8")
                self.assertNotIn('"passed": True', source)


if __name__ == "__main__":
    unittest.main()
