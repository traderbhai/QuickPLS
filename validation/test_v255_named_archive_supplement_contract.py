from __future__ import annotations

import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
DRIVER = VALIDATION / "v255_named_case_driver.mjs"
MANIFEST = VALIDATION / "v255_named_case_manifest.json"
WRAPPER = VALIDATION / "run_v255_installed_portable_smoke.ps1"
AUDIT = VALIDATION / "v255_product_completion_audit.py"

EXPECTED_SUPPLEMENTS = {
    "specialized_result:parallel mediation": "pls_algorithm",
    "specialized_result:multiple-mediation bootstrap": "pls_bootstrap",
    "specialized_result:three-way moderation": "pls_bootstrap",
    "specialized_result:first-stage moderated mediation": "pls_bootstrap",
    "specialized_result:HOC bootstrap": "pls_bootstrap",
    "specialized_result:recursive SEM point": "cbsem",
    "specialized_result:recursive SEM case bootstrap": "cbsem",
}


class V255NamedArchiveSupplementContractTests(unittest.TestCase):
    def test_manifest_opt_in_is_exact_portable_fresh_route_coverage(self) -> None:
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
        observed: dict[str, str] = {}
        for case in manifest["cases"]:
            route = case.get("route") or {}
            public_kind = route.get("archive_supplement_public_kind")
            if public_kind is None:
                continue
            self.assertEqual("portable", case["candidate"])
            self.assertEqual("fresh_sem_result", route["kind"])
            self.assertEqual(public_kind, route["method"])
            self.assertIn("method_version", route["result"])
            self.assertTrue(route["result"]["capability_cell_ids"])
            observed[case["id"]] = public_kind
        self.assertEqual(EXPECTED_SUPPLEMENTS, observed)

    def test_driver_saves_hashes_reopens_and_reports_exact_bindings(self) -> None:
        source = DRIVER.read_text(encoding="utf-8")
        materialize = source[
            source.index('assert(route.kind === "fresh_sem_result"') :
            source.index("function validateManifest")
        ]
        self.assertIn("route.archive_supplement_public_kind", materialize)
        self.assertIn('action: "save_result_archive_supplement"', materialize)
        self.assertLess(
            materialize.index('action: "select_result_table"'),
            materialize.index('action: "save_result_archive_supplement"'),
        )
        save = source[
            source.index('if (step.action === "save_result_archive_supplement")') :
            source.index('if (step.action === "open_archive")')
        ]
        for required in (
            'context.candidateName === "portable"',
            "context.calculationRevision?.target",
            'page.keyboard.press("Control+s")',
            "afterSha256 !== beforeSha256",
            "inspectSavedSupplementArchive",
            'page.goto(`${PACKAGED_TAURI_ORIGIN}/?quickpls_smoke=1`',
            "waitForOpenedProjectPath(page, target, timeout)",
            "observeCompletedResultIdentity(page)",
            "Fresh reopen did not preserve",
            "candidateSha256",
            "manifestSha256",
        ):
            self.assertIn(required, save)
        self.assertIn("result_archive_supplements: []", source)
        self.assertIn("source_report_json_pointer", source)
        self.assertIn('type: "canonical_result_document_id"', source)
        self.assertIn('type: "schema5_result_run_id"', source)

    def test_wrapper_validates_report_then_restarts_for_frozen_crawl(self) -> None:
        source = WRAPPER.read_text(encoding="utf-8")
        named_call = source.index("& $node $namedCaseDriver")
        named_report_guard = source.index("Assert-NamedArchiveSupplements", named_call)
        named_stop = source.index("Stop-IsolatedCandidate $process $endpoint", named_report_guard)
        named_null = source.index("$process = $null", named_stop)
        frozen_start = source.index(
            "$process = Start-IsolatedCandidate $candidateFull $endpoint", named_null
        )
        frozen_call = source.index("& $node @frozenArguments", frozen_start)
        self.assertEqual(
            sorted(
                [
                    named_call,
                    named_report_guard,
                    named_stop,
                    named_null,
                    frozen_start,
                    frozen_call,
                ]
            ),
            [
                named_call,
                named_report_guard,
                named_stop,
                named_null,
                frozen_start,
                frozen_call,
            ],
        )
        self.assertIn(
            '"--named-supplement-report", $namedSupplementReportRelative', source
        )
        self.assertIn("candidate.pid", source)
        self.assertIn("candidate.sha256", source)
        self.assertIn("manifest.sha256", source)
        self.assertIn("source_report_json_pointer", source)

    def test_source_audit_accepts_only_portable_method_bound_supplements(self) -> None:
        source = AUDIT.read_text(encoding="utf-8")
        self.assertIn('"archive_supplement_public_kind",', source)
        self.assertIn('case.get("candidate") == "portable"', source)
        self.assertIn(
            'supplement_public_kind == route.get("method")',
            source,
        )

    def test_named_archive_expectations_bind_current_researcher_facing_tables(self) -> None:
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
        routes = {case["id"]: case.get("route") for case in manifest["cases"]}
        bootstrap = routes["specialized_result:regression case bootstrap"]
        self.assertEqual("regression_bootstrap_coefficients", bootstrap["table_id"])
        self.assertEqual(
            ["Original", "Bootstrap mean", "Bootstrap SE"],
            bootstrap["header_contains"],
        )
        self.assertEqual(["Intercept", "x", "z", "w"], bootstrap["row_contains"])

        logistic = routes["specialized_result:regression logistic"]
        self.assertEqual(["Intercept", "x", "z", "w"], logistic["row_contains"])

        process_reference = routes["specialized_result:PROCESS mediation"]
        self.assertEqual(
            ["Effect", "Reference condition"],
            process_reference["header_contains"],
        )
        self.assertEqual(
            ["Conditional indirect effect", "X → M1 → M2 → Y"],
            process_reference["row_contains"],
        )
        process_slopes = routes["specialized_result:PROCESS moderation"]
        self.assertEqual(
            ["Effect", "Moderator probe(s)", "Estimate"],
            process_slopes["header_contains"],
        )
        self.assertEqual(
            ["Conditional effect", "X × W → M3"],
            process_slopes["row_contains"],
        )

        identity_helper = (VALIDATION / "v255_named_archive_identity.py").read_text(
            encoding="utf-8"
        )
        self.assertIn('"regression_bootstrap_coefficients": (', identity_helper)
        self.assertNotIn('"regression_bootstrap_summary": (', identity_helper)
        readiness = (VALIDATION / "v255_named_route_readiness.py").read_text(
            encoding="utf-8"
        )
        self.assertIn(
            '"regression case bootstrap": ("regression", "regression_bootstrap_coefficients", 4)',
            readiness,
        )

    def test_owned_text_files_are_utf8_without_bom(self) -> None:
        for path in (DRIVER, MANIFEST, WRAPPER, AUDIT):
            with self.subTest(path=path.name):
                payload = path.read_bytes()
                self.assertFalse(payload.startswith(b"\xef\xbb\xbf"))
                payload.decode("utf-8")


if __name__ == "__main__":
    unittest.main()
