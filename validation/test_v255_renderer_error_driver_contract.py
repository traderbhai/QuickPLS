from __future__ import annotations

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


if __name__ == "__main__":
    unittest.main()
