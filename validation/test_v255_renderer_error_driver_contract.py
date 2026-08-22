from __future__ import annotations

import csv
import json
import math
import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
BROWSER_DRIVERS = (
    "v255_named_case_driver.mjs",
    "v255_cross_method_candidate_driver.mjs",
    "v255_frozen_archive_reopen_crawler.mjs",
    "v255_posthoc_minimum_sample_packaged_smoke.mjs",
    "v255_method_evidence_crawler.mjs",
)


def source(name: str) -> str:
    return (VALIDATION / name).read_text(encoding="utf-8")


class V255RendererErrorDriverContractTests(unittest.TestCase):
    def test_every_browser_driver_captures_both_error_channels_without_allowlist(self) -> None:
        for name in BROWSER_DRIVERS:
            with self.subTest(driver=name):
                text = source(name)
                self.assertIn('page.on("pageerror", onPageError);', text)
                self.assertIn('page.on("console", onConsole);', text)
                self.assertIn('message.type() === "error"', text)
                self.assertIn('type: "pageerror"', text)
                self.assertIn('type: "console"', text)
                self.assertIn("RENDERER_ERROR_SETTLE_MS = 250", text)
                self.assertNotIn("allowedConsole", text)
                self.assertNotIn("ignoredConsole", text)

    def test_listener_is_armed_before_each_first_page_action(self) -> None:
        named = source("v255_named_case_driver.mjs")
        self.assertLess(
            named.index("rendererErrors = observeRendererErrors(page);"),
            named.index("for (let ordinal = 1; ordinal <= selectedCases.length"),
        )

        cross = source("v255_cross_method_candidate_driver.mjs")
        self.assertLess(
            cross.index("const rendererErrors = observeRendererErrors(page);"),
            cross.index('records = args.phase === "imports"'),
        )

        frozen = source("v255_frozen_archive_reopen_crawler.mjs")
        self.assertLess(
            frozen.index("const rendererErrors = observeRendererErrors(connection.page);"),
            frozen.index("for (let index = 0; index < rows.length; index += 1)"),
        )

        posthoc = source("v255_posthoc_minimum_sample_packaged_smoke.mjs")
        observations = [
            match.start()
            for match in re.finditer(
                re.escape("const rendererErrors = observeRendererErrors(page);"),
                posthoc,
            )
        ]
        archive_opens = [
            match.start()
            for match in re.finditer(re.escape("await openArchive(page,"), posthoc)
        ]
        self.assertEqual(2, len(observations))
        self.assertEqual(2, len(archive_opens))
        self.assertTrue(
            all(observation < archive_open for observation, archive_open in zip(observations, archive_opens, strict=True))
        )

        method = source("v255_method_evidence_crawler.mjs")
        crawler = method[method.index("async function runBrowserCrawler"):]
        observations = [
            match.start()
            for match in re.finditer(
                re.escape("rendererErrors = observeRendererErrors(page);"), crawler
            )
        ]
        first_loads = [
            match.start()
            for match in re.finditer(re.escape("await page.goto("), crawler)
        ][:2]
        self.assertEqual(2, len(observations))
        self.assertEqual(2, len(first_loads))
        self.assertTrue(
            all(observation < first_load for observation, first_load in zip(observations, first_loads, strict=True))
        )

    def test_every_top_level_report_fails_closed_on_renderer_errors(self) -> None:
        named = source("v255_named_case_driver.mjs")
        self.assertIn("console_errors: []", named)
        self.assertIn("if (report.console_errors.length > 0)", named)
        self.assertIn('report.status = "failed";', named)
        self.assertIn("report.passed = false;", named)

        cross = source("v255_cross_method_candidate_driver.mjs")
        self.assertIn("console_errors: consoleErrors", cross)
        self.assertIn("passed: failures.length === 0 && consoleErrors.length === 0", cross)

        frozen = source("v255_frozen_archive_reopen_crawler.mjs")
        self.assertIn("console_errors: consoleErrors", frozen)
        self.assertIn('failure_code: "renderer_console_error"', frozen)
        self.assertIn('status: failures.length ? "failed" : "passed"', frozen)

        posthoc = source("v255_posthoc_minimum_sample_packaged_smoke.mjs")
        self.assertEqual(
            2,
            posthoc.count('status: consoleErrors.length === 0 ? "passed" : "failed"'),
        )
        self.assertEqual(2, posthoc.count("console_errors: consoleErrors"))

        method = source("v255_method_evidence_crawler.mjs")
        self.assertIn("console_errors: []", method)
        self.assertIn("assert(report.console_errors.length === 0", method)

        lifecycle = source("v255_live_calculation_lifecycle_smoke.mjs")
        self.assertIn("Array.isArray(underlyingPhase?.consoleErrors)", lifecycle)
        self.assertIn("phase.console_errors = underlyingPhase.consoleErrors.map", lifecycle)
        self.assertIn(
            "report.passed = report.complete && report.console_errors.length === 0;",
            lifecycle,
        )

    def test_named_archive_routes_reset_and_wait_for_exact_project_path(self) -> None:
        named = source("v255_named_case_driver.mjs")
        archive_route = named[
            named.index('if (route.kind === "archive_result")'):
            named.index('if (route.kind === "fresh_cfa_bootstrap_result")')
        ]
        self.assertLess(
            archive_route.index('{ action: "goto_packaged" }'),
            archive_route.index('{ action: "open_archive"'),
        )
        self.assertIn("async function waitForOpenedProjectPath", named)
        self.assertIn('document.querySelector(".nd-document-context span")', named)

        save_reopen = named[
            named.index('if (step.action === "save_and_reopen_case_revision")'):
            named.index('if (step.action === "run_calculation")')
        ]
        self.assertLess(
            save_reopen.index("await page.goto(`${PACKAGED_TAURI_ORIGIN}/?quickpls_smoke=1`"),
            save_reopen.index('quickpls:open-project-path'),
        )
        self.assertLess(
            save_reopen.index('quickpls:open-project-path'),
            save_reopen.index("await waitForOpenedProjectPath(page, target, timeout);"),
        )

        open_archive = named[
            named.index('if (step.action === "open_archive")'):
            named.index('if (step.action === "select_result")')
        ]
        self.assertIn(
            "await waitForOpenedProjectPath(page, archive, timeout);", open_archive
        )

    def test_named_driver_waits_for_controller_surfaces_and_keyboard_readiness(self) -> None:
        named = source("v255_named_case_driver.mjs")
        goto_packaged = named[
            named.index('if (step.action === "goto_packaged")'):
            named.index('if (step.action === "create_project")')
        ]
        self.assertLess(
            goto_packaged.index("await page.goto("),
            goto_packaged.index("loadNamedSemEvidenceFixture"),
        )
        self.assertIn('.nd-app[data-native-desktop-shell="true"]', goto_packaged)
        self.assertIn('typeof window.__QUICKPLS_SMOKE__?.setView === "function"', goto_packaged)

        create_project = named[
            named.index('if (step.action === "create_project")'):
            named.index('if (step.action === "set_viewport")')
        ]
        self.assertIn('getByRole("button", { name: "New Project…", exact: true })', create_project)
        self.assertNotIn('page.keyboard.press("Control+n")', create_project)

        set_view = named[
            named.index('if (step.action === "set_view")'):
            named.index('if (step.action === "load_fixture")')
        ]
        self.assertIn("surfaceForView(step.view)", set_view)
        self.assertIn('data-surface="${surface}"', set_view)

        generic_actions = named[
            named.index('if (["click", "double_click", "fill", "select_option", "press"]'):
            named.index('if (step.action === "native_file_dialog")')
        ]
        self.assertIn('step.action === "press" && step.selector === "body"', generic_actions)
        self.assertIn("await page.keyboard.press(step.key);", generic_actions)
        self.assertIn("await locator.isEnabled().catch", generic_actions)
        self.assertIn("await locator.focus({ timeout });", generic_actions)
        self.assertIn("document.activeElement === element", generic_actions)
        self.assertIn("assert(focusVerified", generic_actions)
        self.assertIn("await locator.press(step.key, { timeout });", generic_actions)
        self.assertNotIn("focusDeadline", generic_actions)

        calculation_revision = named[
            named.index('if (step.action === "prepare_calculation_revision")'):
            named.index('if (step.action === "exercise_advanced_parameter_revision")')
        ]
        generic_plan = named[
            named.index('assert(route.kind === "fresh_sem_result"'):
            named.index('const query = {', named.index('assert(route.kind === "fresh_sem_result"'))
        ]
        self.assertIn('const prepareCalculationRevision = {', generic_plan)
        self.assertIn('method: route.method,', generic_plan)
        self.assertIn('inference: route.inference,', generic_plan)
        self.assertIn('bootstrap_samples: route.bootstrap_samples,', generic_plan)
        self.assertIn('prepareCalculationRevision,', generic_plan)
        exact_cfa_plan = named[
            named.index('if (route.kind === "fresh_cfa_bootstrap_result")'):
            named.index('assert(route.kind === "fresh_sem_result"')
        ]
        exact_cfa_prepare = '{ action: "prepare_calculation_revision", method: "cbsem", inference: "point" }'
        exact_cfa_run = '{ action: "run_calculation", method: route.method, inference: route.inference'
        self.assertEqual(1, exact_cfa_plan.count(exact_cfa_prepare))
        self.assertEqual(1, exact_cfa_plan.count(exact_cfa_run))
        self.assertLess(exact_cfa_plan.index(exact_cfa_prepare), exact_cfa_plan.index(exact_cfa_run))
        self.assertIn('requires_requested_revision: false', exact_cfa_plan)
        self.assertIn('name: "Advanced Parameter Table…", exact: true', calculation_revision)
        self.assertIn('name: "Continue to Calculate", exact: true', calculation_revision)
        self.assertLess(
            calculation_revision.index('name: "Advanced Parameter Table…", exact: true'),
            calculation_revision.index('name: "Continue to Calculate", exact: true'),
        )
        self.assertIn('new Set(["pls_algorithm", "pls_bootstrap", "cbsem"]).has(step.method)', calculation_revision)
        self.assertIn('const estimatorId = step.method === "cbsem" ? "qpls.cbsem.v3" : "qpls.pls_sem.v3"', calculation_revision)
        self.assertIn('step.method === "pls_bootstrap" || (step.method === "cbsem" && step.inference === "case_bootstrap")', calculation_revision)
        self.assertIn('estimator.selectOption(estimatorId)', calculation_revision)
        self.assertIn('bootstrap.setChecked(caseBootstrap)', calculation_revision)
        self.assertIn('samples.fill(String(step.bootstrap_samples ?? 500))', calculation_revision)
        self.assertLess(
            calculation_revision.index('estimator.selectOption(estimatorId)'),
            calculation_revision.index('await activate.click({ timeout });'),
        )
        self.assertLess(
            calculation_revision.index('bootstrap.setChecked(caseBootstrap)'),
            calculation_revision.index('await activate.click({ timeout });'),
        )
        self.assertLess(
            calculation_revision.index('await activatedAuthority.waitFor({ state: "visible", timeout });'),
            calculation_revision.index('"The activated calculation authority changed the requested estimator."'),
        )
        self.assertIn('"The activated calculation authority changed the requested inference."', calculation_revision)
        self.assertIn('"The activated calculation authority changed the requested bootstrap sample count."', calculation_revision)
        self.assertIn('selected_method: step.method,', calculation_revision)
        self.assertIn('selected_inference: step.inference,', calculation_revision)
        self.assertNotIn('return { action: step.action, required: false }', calculation_revision)

        advanced_parameter_revision = named[
            named.index('if (step.action === "exercise_advanced_parameter_revision")'):
            named.index('if (step.action === "save_and_reopen_case_revision")')
        ]
        self.assertIn('name: "Continue to Calculate", exact: true', advanced_parameter_revision)
        self.assertIn('estimator.selectOption("qpls.cbsem.v3")', advanced_parameter_revision)
        self.assertIn('bootstrap.setChecked(false)', advanced_parameter_revision)
        self.assertIn('-parameter-revision.qpls', advanced_parameter_revision)
        self.assertIn('target !== sourceTarget', advanced_parameter_revision)
        self.assertIn('await fileSha256(sourceTarget) === sourceSha256', advanced_parameter_revision)
        self.assertIn('context.calculationRevision = { target, initialSha256: targetSha256, sourceTarget, sourceSha256 }', advanced_parameter_revision)
        self.assertIn('const freeLoadingRows = dialog.locator', advanced_parameter_revision)
        self.assertIn('compatibleRowIndexes.length === 2', advanced_parameter_revision)
        self.assertIn('editor.locator(".nd-sem-editor-heading code")', advanced_parameter_revision)
        self.assertIn('parameterIds.every((parameterId)', advanced_parameter_revision)
        self.assertIn('activated?.model?.model_id === before.model.model_id', advanced_parameter_revision)
        self.assertNotIn('activated?.model?.model_id !== before.model.model_id', advanced_parameter_revision)
        self.assertNotIn('page.keyboard.press("Control+s")', advanced_parameter_revision)

        reopen_parameter_revision = named[
            named.index('if (step.action === "save_and_reopen_case_revision")'):
            named.index('if (step.action === "run_calculation")')
        ]
        self.assertNotIn('page.keyboard.press("Control+s")', reopen_parameter_revision)
        self.assertIn('waitForOpenedProjectPath(page, target, timeout)', reopen_parameter_revision)
        self.assertIn('stable Advanced Parameter identities did not survive reopen', reopen_parameter_revision)
        self.assertIn('model digest differs from the activated revision', reopen_parameter_revision)

        general_workspace = (ROOT / "src" / "native" / "NativeRecipeV4GeneralSemWorkspace.tsx").read_text(encoding="utf-8")
        self.assertIn("adoptActiveProject: openNativeProjectAt", general_workspace)

        cbsem_workspace = (ROOT / "src" / "native" / "NativeRecipeV4CbsemWorkspace.tsx").read_text(encoding="utf-8")
        self.assertIn("strictlyPublishCbsemCalculationResultV1", cbsem_workspace)
        self.assertIn('if (publication.status === "blocked")', cbsem_workspace)
        self.assertIn("setArchiveFailure(publication.diagnostic)", cbsem_workspace)
        self.assertIn("document: publication.entry.canonicalDocument", cbsem_workspace)
        self.assertNotIn("Compatibility projects that cannot accept schema-6 append", cbsem_workspace)

        cfa_observer = named[
            named.index('if (kind === "cfa_compatibility_result")'):
            named.index('if (kind === "result_surface")')
        ]
        self.assertIn("const canonical = smokeSnapshot?.canonical_result ?? null", cfa_observer)
        self.assertIn("canonical?.document_id", cfa_observer)
        self.assertNotIn("#nd-cbsem-compatibility-calculation", cfa_observer)

        dialog_helper = named[
            named.index("function createDialogHelper"):
            named.index("async function observe")
        ]
        self.assertIn('"--window-title", "QuickPLS"', dialog_helper)
        self.assertNotIn("windowTitle: await page.title()", named)

        run_calculation = named[
            named.index('if (step.action === "run_calculation")'):
            named.index('if (step.action === "save_result_archive_supplement")')
        ]
        self.assertIn('requires_requested_revision: false', named)
        self.assertNotIn('requires_requested_revision: route.method === "pls_bootstrap"', named)
        self.assertNotIn('requires_requested_revision: route.method !== "pls_algorithm"', named)
        self.assertIn('typeof step.requires_requested_revision === "boolean"', run_calculation)
        active_revision_guard = run_calculation.index("if (!step.requires_requested_revision)")
        configure_start = run_calculation.index("await configureAndStart();")
        self.assertLess(active_revision_guard, configure_start)
        active_revision_contract = run_calculation[active_revision_guard:configure_start]
        self.assertIn("context.calculationRevision", active_revision_contract)
        self.assertIn("waitForOpenedProjectPath(page, activeRevision.target, timeout)", active_revision_contract)
        self.assertIn("fileSha256(activeRevision.target) === activeRevision.initialSha256", active_revision_contract)
        self.assertIn('requestedRevisionHelper = createDialogHelper', run_calculation)
        self.assertLess(
            run_calculation.index('requestedRevisionHelper = createDialogHelper'),
            run_calculation.index('await configureAndStart();'),
        )
        self.assertLess(
            run_calculation.index('const ready = await requestedRevisionHelper.ready'),
            run_calculation.index('await configureAndStart();'),
        )
        self.assertLess(
            run_calculation.index('const completed = await requestedRevisionHelper.completed'),
            run_calculation.index('const calculationProgress = page.locator'),
        )
        self.assertIn('.nd-cbsem-v4-monitor, .nd-results-workspace', run_calculation)
        self.assertIn('requestedRevisionReady = await advanced.getByText("Activated calculation authority", { exact: true }).isVisible()', run_calculation)
        self.assertIn('|| await calculationProgress.isVisible()', run_calculation)
        self.assertIn('!calculationProgressVisible', run_calculation)
        self.assertEqual(run_calculation.count('await configureAndStart();'), 1)
        self.assertIn(
            '${context.lastCalculation?.routeText ?? ""}', run_calculation
        )
        self.assertNotIn(
            'bounded timeout: ${routeText}', run_calculation
        )
        self.assertNotIn('name: "Save and activate project…"', run_calculation)
        self.assertNotIn('advancedState.includes("New calculation-ready draft")', run_calculation)

        supplement = named[
            named.index('if (step.action === "save_result_archive_supplement")'):
            named.index('if (step.action === "open_archive")')
        ]
        self.assertIn("beforeSha256 === context.calculationRevision.initialSha256", supplement)
        self.assertIn('persistenceMode = "already_persisted"', supplement)
        self.assertIn('persistenceMode = "explicit_control_s"', supplement)
        self.assertIn('page.keyboard.press("Control+s")', supplement)
        self.assertIn("inspectSavedSupplementArchive(context, target, step.table_id, liveIdentity, timeout)", supplement)
        self.assertLess(
            supplement.index("beforeSha256 === context.calculationRevision.initialSha256"),
            supplement.index('page.keyboard.press("Control+s")'),
        )
        self.assertLess(
            supplement.index('page.keyboard.press("Control+s")'),
            supplement.index("inspectSavedSupplementArchive(context, target, step.table_id, liveIdentity, timeout)"),
        )

        run_loop = named[
            named.index("for (let ordinal = 1; ordinal <= selectedCases.length"):
            named.index("report.offline = offline.summary()")
        ]
        self.assertIn("if (page.isClosed())", run_loop)
        self.assertIn("selectedCases.slice(ordinal - 1)", run_loop)
        self.assertIn("candidate renderer page closed", run_loop)

    def test_named_result_tables_bind_the_single_visible_surface(self) -> None:
        named = source("v255_named_case_driver.mjs")
        helper = named[
            named.index("async function waitForSingleVisibleLocator"):
            named.index("function parseArgs")
        ]
        self.assertIn("totalCount = await matches.count()", helper)
        self.assertIn("const visibleMatches = matches.filter({ visible: true })", helper)
        self.assertIn("visibleCount = await visibleMatches.count()", helper)
        self.assertIn("if (visibleCount === 1) return visibleMatches", helper)
        self.assertIn("assert(visibleCount < 2", helper)
        self.assertIn("${totalCount} total, ${visibleCount} visible", helper)
        self.assertIn("if (allowAbsent) return null", helper)
        self.assertNotIn("matches.first()", helper)
        self.assertNotIn("matches.nth(", helper)

        observe = named[
            named.index("async function observe"):
            named.index("function observeCompletedResultIdentity")
        ]
        self.assertEqual(
            3,
            observe.count(
                "await waitForSingleVisibleLocator(page, resultTableSelector(query.table_id), context.timeout)"
            ),
        )

        supplement = named[
            named.index('if (step.action === "save_result_archive_supplement")'):
            named.index('if (step.action === "open_archive")')
        ]
        self.assertIn(
            "waitForSingleVisibleLocator(page, resultTableSelector(step.table_id), 0, { allowAbsent: true })",
            supplement,
        )
        self.assertIn(
            "waitForSingleVisibleLocator(page, resultTreeItemSelector(step.table_id), timeout)",
            supplement,
        )
        self.assertIn(
            "waitForSingleVisibleLocator(page, resultTableSelector(step.table_id), timeout)",
            supplement,
        )

        select_table = named[
            named.index('if (step.action === "select_result_table")'):
            named.index('if (step.action === "wait_for")')
        ]
        self.assertIn(
            "waitForSingleVisibleLocator(page, resultTableSelector(step.table_id), 0, { allowAbsent: true })",
            select_table,
        )
        self.assertIn(
            "waitForSingleVisibleLocator(page, resultTreeItemSelector(step.table_id), timeout)",
            select_table,
        )
        self.assertIn(
            "waitForSingleVisibleLocator(page, resultTableSelector(step.table_id), timeout)",
            select_table,
        )

        run_case = named[named.index("async function runCase"):named.index("async function main")]
        self.assertIn(
            "waitForSingleVisibleLocator(page, entry.screenshot.selector, context.timeout)",
            run_case,
        )
        self.assertNotRegex(
            named,
            r"page\.locator\(resultTableSelector\([^\n]+\)\)\.first\(\)",
        )

        generic_plan = named[
            named.index('assert(route.kind === "fresh_sem_result"'):
            named.index('const query = {', named.index('assert(route.kind === "fresh_sem_result"'))
        ]
        self.assertLess(
            generic_plan.index("prepareCalculationRevision,"),
            generic_plan.index('action: "run_calculation"'),
        )
        self.assertIn("requires_requested_revision: false", generic_plan)
        self.assertNotIn('route.method === "pls_bootstrap"', generic_plan)

        run_calculation = named[
            named.index('if (step.action === "run_calculation")'):
            named.index('if (step.action === "save_result_archive_supplement")')
        ]
        self.assertIn("if (step.requires_requested_revision)", run_calculation)
        self.assertIn("requestedRevisionHelper = createDialogHelper", run_calculation)

    def test_named_sem_routes_import_one_curated_native_resident_csv(self) -> None:
        dataset_path = VALIDATION / "fixtures" / "v255" / "named-sem-evidence.csv"
        with dataset_path.open("r", encoding="utf-8", newline="") as stream:
            rows = list(csv.reader(stream))
        self.assertEqual(
            rows[0],
            [
                "x1", "x2", "x3", "m11", "m12", "m13", "m21", "m22",
                "w1", "w2", "z1", "z2", "b", "c11", "c12", "c21",
                "c22", "y1", "y2", "y3",
            ],
        )
        self.assertEqual(len(rows), 361)
        self.assertTrue(all(len(row) == 20 for row in rows[1:]))
        self.assertTrue(all(row[12] in {"0", "1"} for row in rows[1:]))
        for row in rows[1:]:
            for value in row:
                self.assertTrue(float(value) == float(value))

        cbsem_column_names = ["x1", "x2", "x3", "m11", "m12", "m13", "y1", "y2", "y3"]
        cbsem_positions = [rows[0].index(name) for name in cbsem_column_names]
        cbsem_rows = [[float(row[position]) for position in cbsem_positions] for row in rows[1:]]
        means = [sum(row[column] for row in cbsem_rows) / len(cbsem_rows) for column in range(9)]
        covariance = [
            [
                sum(
                    (row[left] - means[left]) * (row[right] - means[right])
                    for row in cbsem_rows
                ) / len(cbsem_rows)
                for right in range(9)
            ]
            for left in range(9)
        ]
        lower = [[0.0] * 9 for _ in range(9)]
        minimum_pivot = math.inf
        for row in range(9):
            for column in range(row + 1):
                adjusted = covariance[row][column] - sum(
                    lower[row][index] * lower[column][index]
                    for index in range(column)
                )
                if row == column:
                    minimum_pivot = min(minimum_pivot, adjusted)
                    lower[row][column] = math.sqrt(max(0.0, adjusted))
                else:
                    lower[row][column] = adjusted / lower[column][column]
        maximum_within_factor_correlation = max(
            abs(covariance[left][right])
            / math.sqrt(covariance[left][left] * covariance[right][right])
            for group in ((0, 1, 2), (3, 4, 5), (6, 7, 8))
            for left_offset, left in enumerate(group)
            for right in group[:left_offset]
        )
        self.assertGreater(minimum_pivot, 0.02)
        self.assertLess(maximum_within_factor_correlation, 0.98)

        service = (ROOT / "src" / "services" / "projectService.ts").read_text(encoding="utf-8")
        user_import = service[
            service.index("export async function importNativeDataset("):
            service.index("export async function importNativeDatasetAtPathForValidation(")
        ]
        validation_import_start = service.index("export async function importNativeDatasetAtPathForValidation(")
        validation_import = service[
            validation_import_start:
            service.index("export async function importNativeValidationFixture(", validation_import_start)
        ]
        self.assertIn("{ path, dataKind, sampleSize, missingMarkers }", user_import)
        self.assertNotIn("missingMarkers: []", user_import)
        self.assertIn("missingMarkers: []", validation_import)

        named = source("v255_named_case_driver.mjs")
        fixture_step = named[
            named.index('if (step.action === "load_named_sem_fixture")'):
            named.index('if (step.action === "inspect_archive_identity")')
        ]
        self.assertIn("loadNamedSemEvidenceFixture({ fixture, datasetPath })", fixture_step)
        self.assertIn("observed.datasetResident === true", fixture_step)
        self.assertIn("observed.datasetRows === 360", fixture_step)
        self.assertIn("observed.datasetColumns === 20", fixture_step)

        app = (ROOT / "src" / "native" / "NativeDesktopApp.tsx").read_text(encoding="utf-8")
        fixture_hook_start = app.index("loadNamedSemEvidenceFixture: async")
        fixture_hook = app[
            fixture_hook_start:
            app.index("namedSemEvidenceSnapshot:", fixture_hook_start)
        ]
        self.assertIn("importNativeDatasetAtPathForValidation(datasetPath)", fixture_hook)
        self.assertIn("const afterImport = useWorkspace.getState();", fixture_hook)
        self.assertIn("afterImport.projectId !== retainedProjectId", fixture_hook)
        self.assertIn("afterImport.projectPath !== null", fixture_hook)
        self.assertIn("afterImport.generalSemProjectDraftMode?.sourceProjectId !== retainedDraft.sourceProjectId", fixture_hook)
        self.assertIn("afterImport.generalSemPublicationPending", fixture_hook)
        self.assertIn('updateNativeColumnMetadata(importedDataset.id, "b", {', fixture_hook)
        self.assertIn('scale_type: "binary"', fixture_hook)
        self.assertIn("theoretical_min: 0", fixture_hook)
        self.assertIn("theoretical_max: 1", fixture_hook)
        self.assertIn("const binaryPreviewRowsAreExactZeroOne = residentDataset.rows.length > 0", fixture_hook)
        self.assertIn('row.b === "0" || row.b === "1"', fixture_hook)
        self.assertIn("!binaryPreviewRowsAreExactZeroOne", fixture_hook)
        self.assertIn('value_labels: { "0": "Group 0", "1": "Group 1" }', fixture_hook)
        self.assertIn("residentDataset.id === importedDataset.id", fixture_hook)
        self.assertIn("residentDataset.fingerprint === importedDataset.fingerprint", fixture_hook)
        self.assertIn("dataset: residentDataset", fixture_hook)
        self.assertIn("retainedDraft ? { preserveGeneralSemProjectDraftMode: retainedDraft } : {}", fixture_hook)
        self.assertLess(
            fixture_hook.index("importNativeDatasetAtPathForValidation(datasetPath)"),
            fixture_hook.index("const afterImport = useWorkspace.getState();"),
        )
        self.assertLess(
            fixture_hook.index("const afterImport = useWorkspace.getState();"),
            fixture_hook.index('updateNativeColumnMetadata(importedDataset.id, "b", {'),
        )
        self.assertLess(
            fixture_hook.index('updateNativeColumnMetadata(importedDataset.id, "b", {'),
            fixture_hook.index("loadProject({"),
        )

        snapshot_start = app.index("namedSemEvidenceSnapshot:", fixture_hook_start)
        snapshot = app[
            snapshot_start:
            app.index("modelCounts:", snapshot_start)
        ]
        self.assertIn("three_way_moderation_bootstrap_receipt?.capability_cell", snapshot)
        self.assertIn("three_way_simple_slopes", snapshot)
        self.assertIn("three_way_probe_contracts: threeWayProbeContracts", snapshot)
        self.assertIn("advanced_equality_labels: advancedEqualitySnapshot.labels", snapshot)
        self.assertIn("advanced_equalities: advancedEqualitySnapshot.equalities", snapshot)
        self.assertIn("namedSemAdvancedEqualitySnapshotV1(authority?.model.parameters ?? [])", snapshot)
        self.assertLess(
            snapshot.index("three_way_moderation_bootstrap_receipt?.capability_cell"),
            snapshot.index("inference_receipt?.capability_cell"),
        )
        self.assertNotIn("exerciseNamedAdvancedParameterRevision", app)

        readiness = (VALIDATION / "v255_named_route_readiness.py").read_text(encoding="utf-8")
        audit = (VALIDATION / "v255_product_completion_audit.py").read_text(encoding="utf-8")
        self.assertIn('repo / "validation" / "fixtures" / "v255" / "named-sem-evidence.csv"', readiness)
        self.assertIn('"validation/fixtures/v255/named-sem-evidence.csv"', audit)

    def test_named_sem_manifest_pins_qualified_probe_and_bootstrap_identities(self) -> None:
        manifest = json.loads(source("v255_named_case_manifest.json"))
        routes = {case["id"]: case["route"] for case in manifest["cases"] if "route" in case}
        explicit_w = {
            "moderator_id": "construct:w",
            "kind": "explicit",
            "values": [-1, 0, 1],
        }
        simultaneous = routes["specialized_result:simultaneous two-way moderation"]
        self.assertEqual(
            [explicit_w, {**explicit_w, "moderator_id": "construct:z"}],
            simultaneous["result_counts"]["conditional_probe_contracts"],
        )
        for case_id in (
            "specialized_result:first-stage moderated mediation",
            "specialized_result:second-stage moderated mediation",
        ):
            self.assertEqual(
                [explicit_w],
                routes[case_id]["result_counts"]["conditional_probe_contracts"],
            )
        self.assertEqual(
            [
                {"moderator_id": "construct:w", "kind": "continuous_standardized", "values": [-1, 0, 1]},
                {"moderator_id": "construct:z", "kind": "continuous_standardized", "values": [-1, 0, 1]},
            ],
            routes["specialized_result:three-way moderation"]["result_counts"]["three_way_probe_contracts"],
        )

        binary = routes["specialized_result:binary moderator probes"]
        self.assertEqual("general_sem_three_way_simple_slopes", binary["table_id"])
        self.assertEqual(
            ["Moderator 1", "Moderator 1 value", "Moderator 2", "Moderator 2 value", "Estimate"],
            binary["header_contains"],
        )
        self.assertEqual(["Moderator W", "Binary moderator", "0", "1"], binary["row_contains"])
        self.assertEqual(
            {
                "ordinary_construct_count": 4,
                "common_factor_count": 0,
                "structural_relation_count": 7,
                "interaction_orders": [2, 2, 2, 3],
                "higher_order_measurement_types": [],
            },
            binary["model"],
        )
        self.assertEqual(
            {
                "method_version": "qpls.general-sem-pls.three-way.point.v1",
                "primary_cell_id": "qpls3.pls.general_sem_three_way_moderation_point",
                "execution_cell_id": "qpls3.pls.general_sem_three_way_moderation_point",
                "capability_cell_ids": [
                    "qpls3.pls.algorithm",
                    "qpls3.pls.general_sem_three_way_moderation_point",
                ],
            },
            binary["result"],
        )
        self.assertEqual(
            [
                {"moderator_id": "construct:b", "kind": "binary_zero_one", "values": [0, 1]},
                {"moderator_id": "construct:w", "kind": "continuous_standardized", "values": [-1, 0, 1]},
            ],
            binary["result_counts"]["three_way_probe_contracts"],
        )

        recursive = routes["specialized_result:recursive SEM case bootstrap"]["result"]
        self.assertEqual("cbsem_exact_recursive_sem_case_bootstrap_v1", recursive["method_version"])
        self.assertEqual("qpls3.cbsem.bootstrap.recursive_sem", recursive["primary_cell_id"])
        self.assertEqual("qpls3.cbsem.bootstrap.recursive_sem", recursive["execution_cell_id"])
        self.assertEqual(
            ["Point estimate", "Lower", "Upper"],
            routes["specialized_result:recursive SEM case bootstrap"]["header_contains"],
        )
        self.assertEqual(
            ["Factor variance", "Measurement parameter", "Residual variance", "Structural parameter"],
            routes["specialized_result:recursive SEM case bootstrap"]["row_contains"],
        )

        visible_mediation_rows = ["specific_indirect", "Predictor", "Outcome"]
        for case_id in (
            "specialized_result:parallel mediation",
            "specialized_result:single-mediation bootstrap",
            "specialized_result:multiple-mediation bootstrap",
        ):
            self.assertEqual(visible_mediation_rows, routes[case_id]["row_contains"])
        self.assertEqual(
            ["Predictor", "Mediator 1", "Mediator 2", "Outcome"],
            routes["specialized_result:serial mediation"]["row_contains"],
        )

        fresh_cbsem = [
            route
            for route in routes.values()
            if route.get("kind") == "fresh_sem_result" and route.get("method") == "cbsem"
        ]
        self.assertEqual(4, len(fresh_cbsem))
        self.assertEqual(3, sum(route["inference"] == "point" for route in fresh_cbsem))
        self.assertEqual(1, sum(route["inference"] == "case_bootstrap" for route in fresh_cbsem))
        exact_cfa = routes["specialized_result:CFA case bootstrap"]
        self.assertEqual("fresh_cfa_bootstrap_result", exact_cfa["kind"])
        self.assertEqual("cbsem", exact_cfa["method"])
        self.assertEqual("case_bootstrap", exact_cfa["inference"])
        self.assertEqual(1000, exact_cfa["bootstrap_samples"])
        self.assertEqual(
            ["Parameter", "Percentile lower", "Percentile upper"],
            exact_cfa["header_contains"],
        )
        self.assertEqual(
            ["Factor variance", "Measurement parameter", "Residual variance"],
            exact_cfa["row_contains"],
        )

    def test_frozen_archive_loop_resets_controller_before_every_exact_path_wait(self) -> None:
        frozen = source("v255_frozen_archive_reopen_crawler.mjs")
        open_archive = frozen[
            frozen.index("async function openArchive"):
            frozen.index("function expectedResultOption")
        ]
        reset = 'page.goto(`${PACKAGED_TAURI_ORIGIN}/?quickpls_smoke=1`'
        ready_shell = "page.locator('.nd-app[data-native-desktop-shell=\"true\"]')"
        ready_controller = 'typeof window.__QUICKPLS_SMOKE__?.setView === "function"'
        dispatch = 'window.dispatchEvent(new CustomEvent("quickpls:open-project-path"'
        exact_path = "waitForOpenedProjectPath(page, archive, timeout)"
        self.assertRegex(
            open_archive,
            r"async function openArchive\(page, archive, timeout, methodKind\)",
        )
        self.assertIn('"open.packaged_root_reset"', open_archive)
        self.assertIn('"open.native_shell_ready"', open_archive)
        self.assertIn('"open.controller_ready"', open_archive)
        self.assertIn('"open.path_dispatch"', open_archive)
        self.assertIn('"open.exact_path_wait"', open_archive)
        self.assertLess(open_archive.index(reset), open_archive.index(ready_shell))
        self.assertLess(open_archive.index(ready_shell), open_archive.index(ready_controller))
        self.assertLess(open_archive.index(ready_controller), open_archive.index(dispatch))
        self.assertLess(open_archive.index(dispatch), open_archive.index(exact_path))

        exact_wait = frozen[
            frozen.index("async function waitForOpenedProjectPath"):
            frozen.index("async function openArchive")
        ]
        self.assertIn('document.querySelector(".nd-document-context span")', exact_wait)
        self.assertIn("normalize(displayed) === normalize(target)", exact_wait)
        self.assertIn("}, archive, { timeout });", exact_wait)

        crawl_method = frozen[
            frozen.index("async function crawlMethod"):
            frozen.index("async function run()")
        ]
        self.assertLess(
            crawl_method.index("await openArchive(page, stagedArchiveAbsolute, timeout, row.public_kind);"),
            crawl_method.index("await navigateToDeclaredResult(page, row, timeout);"),
        )
        self.assertNotIn("openArchive(page, archiveAbsolute", crawl_method)
        self.assertIn("assert(row.archive_sha256 === sha256", crawl_method)
        self.assertIn('"archive.integrity_postflight"', crawl_method)
        self.assertIn("source_sha256_unchanged", frozen)
        self.assertIn("source_autosave_siblings_after", frozen)
        self.assertIn('"results.selector_wait"', frozen)
        self.assertIn('"results.workspace_wait"', frozen)
        crawl_loop = frozen[
            frozen.index("for (let index = 0; index < rows.length; index += 1)"):
            frozen.index("try {\n    await rendererErrors.settle();")
        ]
        self.assertTrue(crawl_loop.startswith("for (let index = 0;"))
        self.assertIn("for (const route of methodRoutes)", crawl_loop)
        self.assertEqual(1, crawl_loop.count("const routeEvidence = await crawlMethod({"))
        self.assertIn("startEvidenceIndex: evidence.length", crawl_loop)

    def test_frozen_archive_crawler_emits_strict_multi_capture_receipts(self) -> None:
        frozen = source("v255_frozen_archive_reopen_crawler.mjs")
        self.assertIn(
            'const INVENTORY_SCHEMA = "quickpls.v255.reusable_archive_inventory.v2";',
            frozen,
        )
        self.assertIn('"named-supplement-report"', frozen)
        self.assertIn("function materializeCoverageRoute", frozen)
        self.assertIn('route.source.kind === "posthoc_supplement"', frozen)
        self.assertIn('route.source.kind === "named_supplement"', frozen)
        self.assertIn("json_pointer: supplement.source_report_json_pointer", frozen)
        self.assertIn(
            'const resolvedPointer = declaredPointer.startsWith("#/") ? declaredPointer.slice(1) : declaredPointer;',
            frozen,
        )
        self.assertIn("deepEqual(observed?.candidate, row.named_supplement_candidate)", frozen)
        self.assertIn("deepEqual(observed?.manifest, row.named_supplement_manifest)", frozen)
        self.assertIn("async function activateCaptureTarget", frozen)
        self.assertLess(
            frozen.index("await activateCaptureTarget(page, row, capture, timeout);"),
            frozen.index("const labels = await runStage(row.public_kind, `capture.${capture.capture_id}.observed_results_labels`"),
        )
        for observation_source in (
            "navigation",
            "visible_headings",
            "visible_result_table_ids",
            "table_titles",
            "table_headers",
            "table_rows",
            "chart_titles",
        ):
            self.assertIn(f'"{observation_source}"', frozen)
        self.assertIn(
            "const pointer = `/evidence/${evidenceIndex}/observed_results_labels/${declaration.source}/${matchedIndex}`;",
            frozen,
        )
        self.assertIn("schema_version: 2", frozen)
        self.assertIn("evidence,", frozen)
        self.assertIn(
            "assert(methodReceipts.length === 18",
            frozen,
        )
        aggregate = frozen[frozen.index("const manifest = {") : frozen.index("const manifestPath =")]
        self.assertIn("schema_version: 1", aggregate)
        self.assertIn("suite_id: SUITE_ID", aggregate)

    def test_frozen_inventory_v2_exactly_binds_static_and_dynamic_routes(self) -> None:
        inventory_path = VALIDATION / "v255_reusable_archive_inventory.json"
        encoded = inventory_path.read_bytes()
        self.assertFalse(encoded.startswith(b"\xef\xbb\xbf"))
        inventory = json.loads(encoded.decode("utf-8"))
        self.assertEqual(
            "quickpls.v255.reusable_archive_inventory.v2", inventory["schema"]
        )
        methods = inventory["public_methods"]
        routes = inventory["coverage_routes"]
        method_kinds = {method["public_kind"] for method in methods}
        self.assertEqual(18, len(methods))
        self.assertEqual(18, len(method_kinds))
        self.assertEqual(method_kinds, {route["public_kind"] for route in routes})
        self.assertEqual(len(routes), len({route["route_id"] for route in routes}))

        capture_ids: list[str] = []
        named_case_ids: list[str] = []
        posthoc_routes = 0
        for route in routes:
            source_contract = route["source"]
            if source_contract["kind"] == "inventory":
                section = source_contract["inventory_section"]
                key = "public_kind" if section == "public_methods" else "feature"
                source_rows = [
                    row
                    for row in inventory[section]
                    if row[key] == source_contract["inventory_key"]
                ]
                self.assertEqual(1, len(source_rows), route["route_id"])
                for field in (
                    "archive_path",
                    "archive_sha256",
                    "result_identity",
                    "scientific_identity",
                    "prior_receipt",
                ):
                    self.assertEqual(source_rows[0][field], route[field])
            else:
                for field in (
                    "archive_path",
                    "result_identity",
                    "scientific_identity",
                    "prior_receipt",
                ):
                    self.assertNotIn(field, route)
                if source_contract["kind"] == "named_supplement":
                    named_case_ids.append(source_contract["case_id"])
                else:
                    self.assertEqual("posthoc_supplement", source_contract["kind"])
                    posthoc_routes += 1

            for capture_contract in route["captures"]:
                capture_ids.append(capture_contract["capture_id"])
                self.assertEqual(
                    capture_contract["covers"],
                    [item["family"] for item in capture_contract["observations"]],
                )

        self.assertEqual(len(capture_ids), len(set(capture_ids)))
        self.assertEqual(7, len(named_case_ids))
        self.assertEqual(7, len(set(named_case_ids)))
        self.assertEqual(1, posthoc_routes)

        frozen = source("v255_frozen_archive_reopen_crawler.mjs")
        self.assertIn('encoding: "utf8", flag: "wx"', frozen)
        self.assertIn("assert(methodReceipts.length === 18", frozen)


if __name__ == "__main__":
    unittest.main()
