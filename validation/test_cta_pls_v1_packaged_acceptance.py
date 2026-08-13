from __future__ import annotations

import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from cta_pls_v1_packaged_acceptance import (  # noqa: E402
    EXPECTED_PAIRINGS,
    EXPECTED_SHEET_ORDER,
    FACTORY_XLSX,
    INTERNAL_APP_ORIGINS,
    RUNTIME_XLSX,
    build_source_paths,
    cli_source_paths,
    evaluate_method_functional_offline,
    read_network_observation,
    source_freshness,
    validate_packaged_report,
    verify_exact_values,
)


def valid_packaged_report() -> dict:
    passed = {"passed": True}
    return {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": "packaged_acceptance",
        "passed": True,
        "feature_id": "qpls3.assessment.cta_pls",
        "method_version": "cta_pls_tetrad_v1",
        "catalogue_snapshot_date": "2026-08-12",
        "generated_at_utc": "2026-08-13T12:00:00Z",
        "source_artifacts": [{"path": "validation/example.py", "size": 1, "sha256": "a" * 64}],
        "checks": {
            "passed": True,
            "prior_factory_stages": passed,
            "focused_report": passed,
            "tested_binary": passed,
            "source_freshness": passed,
            "invalid_setup": passed,
            "same_run": passed,
            "exact_archive": passed,
            "exact_xlsx": passed,
            "responsive_viewports": passed,
            "method_functional_offline": {
                "passed": True,
                "analysis_export_save_reopen_succeeded": True,
                "no_external_app_requests": True,
                "runtime_network_dependency": False,
                "allowed_origins": INTERNAL_APP_ORIGINS,
                "observed_origins": INTERNAL_APP_ORIGINS,
                "observed_request_count": 47,
                "external_request_count": 0,
                "external_requests": [],
                "browser": {"passed": True},
            },
            "platform_background_egress_observation": {
                "passed": True,
                "observation_kind": "sampled_exact_process_tree_tcp_v1",
                "sample_count": 3,
                "root_present_every_sample": True,
                "platform_background_egress_observed": True,
                "commercial_zero_egress_passed": False,
                "remote_connections": [{"owning_process": 100, "remote_address": "203.0.113.1", "remote_port": 443}],
            },
            "cleanup": passed,
            "fail_closed_mutations": passed,
        },
        "execution": {
            "command": ["powershell", "wrapper.ps1"],
            "returncode": 0,
            "duration_seconds": 1.0,
            "stdout_tail": "",
            "stderr_tail": "",
            "clean_after_wrapper": True,
            "cleanup_probe": {},
        },
    }


def test_packaged_schema_is_valid_and_rejects_red_or_incomplete_reports() -> None:
    schema = json.loads((VALIDATION / "cta_pls_v1_packaged_acceptance.schema.json").read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    report = valid_packaged_report()
    assert not list(validator.iter_errors(report))
    for mutation in ("passed", "same_run", "source_freshness", "method_functional_offline", "cleanup"):
        changed = copy.deepcopy(report)
        if mutation == "passed":
            changed["passed"] = False
        else:
            changed["checks"][mutation]["passed"] = False
        assert list(validator.iter_errors(changed)), mutation


def test_method_functional_offline_rejects_any_external_app_request() -> None:
    clean_browser = {
        "passed": True,
        "observedRequestCount": 47,
        "externalRequestCount": 0,
        "origins": INTERNAL_APP_ORIGINS,
        "externalRequests": [],
    }
    clean = evaluate_method_functional_offline(
        clean_browser,
        analysis_export_save_reopen_succeeded=True,
    )
    assert clean["passed"]
    assert clean["runtime_network_dependency"] is False

    external_browser = copy.deepcopy(clean_browser)
    external_browser.update({
        "passed": False,
        "observedRequestCount": 48,
        "externalRequestCount": 1,
        "origins": [*INTERNAL_APP_ORIGINS, "https://example.invalid"],
        "externalRequests": ["https://example.invalid/telemetry"],
    })
    external = evaluate_method_functional_offline(
        external_browser,
        analysis_export_save_reopen_succeeded=True,
    )
    assert external["passed"] is False
    assert external["no_external_app_requests"] is False
    assert external["runtime_network_dependency"] is True


def test_platform_egress_is_recorded_without_mislabeling_commercial_zero_egress() -> None:
    sample = {
        "root_present": True,
        "observation": "sampled_exact_process_tree_tcp_v1",
        "remote_connections": [
            {"owning_process": 100, "remote_address": "203.0.113.1", "remote_port": 443}
        ],
    }
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "network.jsonl"
        path.write_text(json.dumps(sample) + "\n", encoding="utf-8")
        observed = read_network_observation(path)
    assert observed["passed"] is True
    assert observed["platform_background_egress_observed"] is True
    assert observed["commercial_zero_egress_passed"] is False
    assert observed["remote_connections"] == sample["remote_connections"]

    mislabeled = valid_packaged_report()
    mislabeled["checks"]["platform_background_egress_observation"]["commercial_zero_egress_passed"] = True
    errors = validate_packaged_report(mislabeled)
    assert any("False was expected" in error for error in errors)


def test_validator_helper_rejects_identity_and_cleanup_mutations() -> None:
    report = valid_packaged_report()
    assert validate_packaged_report(report) == []
    report["feature_id"] = "qpls3.assessment.cca_residuals"
    report["execution"]["clean_after_wrapper"] = False
    errors = validate_packaged_report(report)
    assert any("qpls3.assessment.cta_pls" in error for error in errors)
    assert any("True was expected" in error for error in errors)


def test_exact_archive_ui_xlsx_cross_check_is_same_run_and_all_three_pairings() -> None:
    archived_rows = [
        {
            "construct": "x",
            "indicator_a": "x1",
            "indicator_b": "x2",
            "indicator_c": "x3",
            "indicator_d": "x4",
            "pairing": pairing,
            "tetrad": value,
            "absolute_tetrad": abs(value),
        }
        for pairing, value in zip(EXPECTED_PAIRINGS, (0.125, -0.25, 0.125))
    ]
    pair_labels = ["Ab cd minus ac bd", "Ac bd minus ad bc", "Ad bc minus ab cd"]
    visible_rows = [
        ["Predictor", "x1", "x2", "x3", "x4", label, f"{row['tetrad']:.4f}", f"{row['absolute_tetrad']:.4f}"]
        for label, row in zip(pair_labels, archived_rows)
    ]
    project = {
        "results": [{
            "id": "cta-run",
            "provenance": {
                "method": "cta_pls",
                "method_version": "pls_pm_v1+cta_pls_tetrad_v1+pls_mediation_v1+pls_assessment_v7",
                "recipe_id": "recipe-1",
                "dataset_fingerprint": "v2:fixture",
            },
            "payload": {"estimation": {"method_version": "cta_pls_tetrad_v1", "cta_pls": {"method_version": "cta_pls_tetrad_v1", "estimates": archived_rows}}},
        }]
    }
    raw = {"checks": {"ctaPlsResult": {"runId": "cta-run", "tetrads": {"values": visible_rows}}, "ctaPlsSaveReopen": {"sameRunRestored": True, "expectedRunId": "cta-run", "selectedRunId": "cta-run"}}}
    tables = {sheet: [] for sheet in EXPECTED_SHEET_ORDER}
    tables["CTA-PLS tetrad summary"] = [["Construct"]]
    tables["CTA-PLS tetrads"] = [["Construct", "Indicator A", "Indicator B", "Indicator C", "Indicator D", "Pairing", "Tetrad", "Absolute tetrad"], *visible_rows]
    tables["CTA-PLS scope and exclusions"] = [["Field", "Value"]]
    tables["Run provenance"] = [["Field", "Value"], ["Recipe", "recipe-1"], ["Dataset fingerprint", "v2:fixture"], ["Method version", "pls_pm_v1+cta_pls_tetrad_v1+pls_mediation_v1+pls_assessment_v7"]]
    exact = verify_exact_values(raw, project, tables)
    assert exact["passed"], exact
    assert exact["exact_sheet_order"]
    reordered = {name: tables[name] for name in reversed(EXPECTED_SHEET_ORDER)}
    assert not verify_exact_values(raw, project, reordered)["passed"]
    raw["checks"]["ctaPlsSaveReopen"]["selectedRunId"] = "other-run"
    assert not verify_exact_values(raw, project, tables)["passed"]


def test_build_source_discovery_binds_all_relevant_product_sources() -> None:
    paths = build_source_paths()
    assert paths == sorted(set(paths))
    required = {
        "src/native/nativeCtaPls.ts",
        "src/native/nativeResults.ts",
        "src-tauri/src/lib.rs",
        "crates/qpls-estimation/src/pls.rs",
        "crates/qpls-project/src/lib.rs",
        "Cargo.lock",
        "package-lock.json",
    }
    assert required <= set(paths)
    assert all((ROOT / path).is_file() for path in paths)

    cli_paths = cli_source_paths()
    assert cli_paths == sorted(set(cli_paths))
    assert "crates/qpls-cli/src/main.rs" in cli_paths
    assert "crates/qpls-project/src/lib.rs" in cli_paths
    assert not any(path.startswith("src/") or path.startswith("src-tauri/") for path in cli_paths)
    freshness = source_freshness()
    assert "desktop_newer_sources" in freshness
    assert "cli_newer_sources" in freshness
    assert "gate_sources" in freshness
    assert RUNTIME_XLSX.parent == ROOT / "validation" / "results"
    assert FACTORY_XLSX.parent == ROOT / "validation" / "results" / "method_factory" / "cta_pls_v1"
    assert RUNTIME_XLSX != FACTORY_XLSX


def test_data_only_archive_treats_absent_optional_workspace_arrays_as_empty() -> None:
    harness = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
    assert "runCount: Array.isArray(workspace?.runs) ? workspace.runs.length : 0" in harness
    assert "nodes: Array.isArray(workspace?.nodes) ? workspace.nodes.length : 0" in harness
    assert "edges: Array.isArray(workspace?.edges) ? workspace.edges.length : 0" in harness
    assert "const ctaSpecificInvalidBlockers = invalidBlockers.filter" in harness
    assert "ctaSpecificInvalidBlockers.length !== 1" in harness
    assert "invalidBlockers.length !== 1" not in harness
    assert "weighting.locator('option[value=\"pca\"]').evaluate((option) => option.disabled)" in harness
    assert "weighting.locator('option[value=\"pca\"]').isDisabled()" not in harness
    assert 'const packagedTauriIpcOrigin = "http://ipc.localhost"' in harness
    assert "new Set([packagedTauriOrigin, packagedTauriIpcOrigin])" in harness
    assert "!packagedInternalOrigins.has(request.origin)" in harness


def test_shared_harness_and_wrapper_keep_cta_scope_isolated_and_fail_closed() -> None:
    harness = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
    wrapper = (VALIDATION / "run_cta_pls_native_acceptance.ps1").read_text(encoding="utf-8")
    for token in (
        'const ctaPlsOnly = acceptanceScope === "cta_pls"',
        "const isolatedFocusedOnly = ctaPlsOnly ||",
        "runFocusedCtaPlsAcceptance()",
        "ctaPlsInvalidSetup",
        "ctaPlsResponsiveViewports",
        "ctaPlsBrowserNetwork",
        "QUICKPLS_CTA_PLS_NATIVE_EXPORT_PATH",
    ):
        assert token in harness
    for token in (
        'QUICKPLS_ACCEPTANCE_SCOPE = "cta_pls"',
        "monitor_quickpls_network.ps1",
        "never overwrites evidence",
        "Get-NetTCPConnection",
        "clean",
    ):
        assert token.lower() in wrapper.lower()


class CtaPlsV1PackagedAcceptanceTests(unittest.TestCase):
    """Make the same contracts runnable by the repository's direct unittest lane."""

    def test_schema_contract(self) -> None:
        test_packaged_schema_is_valid_and_rejects_red_or_incomplete_reports()

    def test_validator_mutations(self) -> None:
        test_validator_helper_rejects_identity_and_cleanup_mutations()

    def test_functional_offline_boundary(self) -> None:
        test_method_functional_offline_rejects_any_external_app_request()

    def test_platform_egress_labeling(self) -> None:
        test_platform_egress_is_recorded_without_mislabeling_commercial_zero_egress()

    def test_exact_cross_surface_values(self) -> None:
        test_exact_archive_ui_xlsx_cross_check_is_same_run_and_all_three_pairings()

    def test_source_discovery(self) -> None:
        test_build_source_discovery_binds_all_relevant_product_sources()

    def test_isolated_harness_contract(self) -> None:
        test_shared_harness_and_wrapper_keep_cta_scope_isolated_and_fail_closed()


if __name__ == "__main__":
    unittest.main()
