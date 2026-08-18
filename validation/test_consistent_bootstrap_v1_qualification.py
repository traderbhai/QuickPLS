from __future__ import annotations

import copy
import json
import subprocess
import sys
from pathlib import Path

import pytest


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
sys.path.insert(0, str(VALIDATION))

import consistent_bootstrap_v1_promotion_audit as promotion_audit  # noqa: E402
import consistent_bootstrap_v1_qualification as qualification  # noqa: E402
import consistent_bootstrap_v1_reference as reference  # noqa: E402
from method_promotion_manifest import validate_manifest  # noqa: E402


def load(path: Path) -> dict:
    document = reference.strict_load_json(path)
    assert isinstance(document, dict)
    return document


def registry_cell() -> tuple[dict, dict]:
    registry = load(qualification.REGISTRY)
    rows = [
        row
        for row in registry["capabilities"]
        if row["capability_id"] == qualification.CAPABILITY_ID
    ]
    assert len(rows) == 1
    cells = [
        cell
        for cell in rows[0]["option_cells"]
        if cell["cell_id"] == qualification.CELL_ID
    ]
    assert len(cells) == 1
    return rows[0], cells[0]


def test_frozen_qualification_spec_is_deterministic_valid_and_non_promotional() -> None:
    frozen = load(qualification.OUTPUT)
    assert frozen == qualification.build_spec()
    report = qualification.verify(frozen)
    assert report["passed"] is True
    assert report["schema_valid"] is True
    assert report["semantic_valid"] is True
    assert report["registry_verified"] is True
    assert report["receipts_verified"] is False
    assert report["qualification_ready"] is False
    assert frozen["migration"]["status"] == "compatibility_only"
    assert frozen["migration"]["unresolved_items"]
    assert frozen["evidence_contract"]["receipts"] == []


def test_qualification_spec_freezes_all_operational_and_evidence_roles() -> None:
    frozen = load(qualification.OUTPUT)
    oracles = {
        oracle["id"]: oracle for oracle in frozen["scientific_contract"]["oracles"]
    }
    assert not (
        ROOT / oracles["required_independent_python_full_plsc_reference"]["locator"]
    ).exists()
    assert not (
        ROOT / oracles["required_independent_r_full_plsc_reference"]["locator"]
    ).exists()
    assert "not been supplied or executed" in oracles[
        "required_independent_python_full_plsc_reference"
    ]["citation"]
    assert "plsc_parameter_resampling_distribution" not in oracles[
        "transparent_python_arithmetic_microreference"
    ]["covered_estimand_ids"]
    assert "plsc_parameter_resampling_distribution" not in oracles[
        "transparent_r_arithmetic_microreference"
    ]["covered_estimand_ids"]
    operational = frozen["operational_contract"]
    assert operational["archive"] == {
        "current_schema_version": 5,
        "readable_schema_versions": [1, 2, 3, 4, 5],
        "writable_schema_versions": [5],
        "future_schema_policy": "verified_read_only",
        "corruption_cases": [
            "feature_identity",
            "method_version",
            "dataset_fingerprint",
            "checksum",
            "duplicate_entry",
            "malformed_payload",
            "legacy_reinterpretation",
            "interrupted_save",
        ],
    }
    assert operational["export"]["formats"] == [
        "csv",
        "xlsx",
        "html",
        "svg",
        "pdf",
        "png",
    ]
    assert operational["export"]["semantic_readback_formats"] == [
        "csv",
        "xlsx",
        "html",
        "svg",
        "pdf",
    ]
    assert operational["export"]["same_run_required"] is True
    assert operational["export"]["provenance_required"] is True
    assert operational["windows"]["package_kinds"] == ["installed", "portable"]
    assert operational["windows"]["display_scale_percent"] == [100, 125, 150, 200]
    assert operational["cancellation"]["maximum_latency_seconds"] == 1.0
    assert operational["cancellation"]["no_partial_visible_result"] is True
    assert operational["cancellation"]["no_partial_committed_result"] is True
    assert operational["cancellation"]["archive_unchanged"] is True
    assert operational["cancellation"]["same_settings_retry"] is True
    assert [profile["id"] for profile in frozen["scenario_contract"]["complexity_profiles"]] == [
        "micro_exact",
        "applied",
        "large",
        "maximum_axis",
        "compound_stress",
    ]
    assert frozen["evidence_contract"]["required_roles"] == [
        "method_contract",
        "kernel_execution",
        "oracle_independence",
        "generative_recovery",
        "adversarial_boundaries",
        "archive_persistence",
        "cross_format_export",
        "frontend_contract",
        "packaged_windows_e2e",
        "performance_scale",
    ]


def test_registry_and_legacy_manifest_remain_fail_closed() -> None:
    row, cell = registry_cell()
    assert cell["capability_version"] == "plsc_bootstrap_v1"
    assert cell["coverage_state"] == "partial"
    assert cell["evidence_state"] == "absent"
    assert cell["surface"] == "labs"
    assert row["legacy_row"]["status"] == "absent"
    manifest = validate_manifest(
        promotion_audit.MANIFEST_PATH,
        ROOT,
        verify_evidence=True,
    )
    assert manifest["passed"] is True
    assert manifest["declared_state"] == "absent"
    assert manifest["derived_state"] == "absent"


def test_transparent_microreference_matches_frozen_hand_fixture() -> None:
    fixture = load(reference.DEFAULT_FIXTURE)
    report = reference.validate_fixture(fixture)
    assert report["passed"] is True
    assert report["fixture_only"] is True
    assert report["qualification_evidence"] is False
    assert report["microcase_count"] == 2
    assert report["boundary_case_count"] == len(reference.REQUIRED_BOUNDARY_IDS)
    assert reference.sample_indices_sha256([0, 1, 2, 3, 4]) == (
        "e3a26ddb9466564d1f33e37a211135d57ddbef1f9f6f11ec5b4d2db674e8cf18"
    )
    assert reference.parameter_values_sha256(
        {
            '["plsc_r_squared",["B"]]': 0.25,
            '["plsc_path",["A","B"]]': 5.0,
        }
    ) == "9f79028a935932bb47675f62d776b11b99b64d6a642ff7eb0c28182ea110c86d"
    assert reference.rust_scientific_12(5.0) == "5.000000000000e0"
    assert reference.rust_scientific_12(0.25) == "2.500000000000e-1"


@pytest.mark.parametrize(
    "mutator, expected_fragment",
    [
        (
            lambda document: document["microcases"][0]["ledger"][0].update(
                expected_sample_indices_sha256="0" * 64
            ),
            "sample digest differs",
        ),
        (
            lambda document: document["microcases"][0]["ledger"][1].update(
                replicate_index=0
            ),
            "not in exact replicate-index order",
        ),
        (
            lambda document: document["microcases"][0]["ledger"][9].update(
                reason_code="unknown_reason"
            ),
            "unknown failure reason",
        ),
        (
            lambda document: document["microcases"][0]["ledger"][0].update(
                expected_parameter_values_sha256="f" * 64
            ),
            "parameter digest differs",
        ),
        (
            lambda document: document["boundary_cases"].pop(),
            "required boundary cases are missing",
        ),
        (
            lambda document: document["scope"].update(qualification_evidence=True),
            "qualification_evidence=false",
        ),
    ],
)
def test_microreference_rejects_digest_ledger_failure_boundary_and_claim_mutations(
    mutator,
    expected_fragment: str,
) -> None:
    fixture = copy.deepcopy(load(reference.DEFAULT_FIXTURE))
    mutator(fixture)
    report = reference.validate_fixture(fixture)
    assert report["passed"] is False
    assert any(expected_fragment in error for error in report["errors"])


def test_strict_fixture_loader_rejects_duplicate_and_nonfinite_json(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"a": 1, "a": 2}\n', encoding="utf-8")
    with pytest.raises(reference.DuplicateKeyError):
        reference.strict_load_json(duplicate)
    nonfinite = tmp_path / "nonfinite.json"
    nonfinite.write_text('{"a": NaN}\n', encoding="utf-8")
    with pytest.raises(ValueError, match="non-finite"):
        reference.strict_load_json(nonfinite)


def test_bca_and_normal_reference_degenerate_cases_fail_closed() -> None:
    normal = reference.normal_reference_test(2.0, 0.0)
    assert normal == {
        "available": False,
        "t_statistic": None,
        "p_value_two_sided": None,
    }
    bca = reference.bca_interval([2.0] * 4, 2.0, [2.0] * 3, 0.95)
    assert bca == {
        "available": False,
        "reason": "degenerate_jackknife_acceleration",
    }
    assert reference.bca_interval([1.0], 1.0, [0.9, 1.0, 1.1], 0.95) == {
        "available": False,
        "reason": "invalid_bca_inputs",
    }


def test_promotion_audit_is_deterministic_valid_and_blocks_promotion() -> None:
    report = promotion_audit.build_report()
    frozen = load(promotion_audit.REPORT_PATH)
    assert report == frozen
    assert report["scaffold_valid"] is True
    assert report["passed"] is False
    assert report["qualification_ready"] is False
    assert report["promotion_allowed"] is False
    assert report["decision"] == "BLOCK_PROMOTION_KEEP_EVIDENCE_ABSENT"
    assert report["current_state"]["receipt_count"] == 0
    assert len(report["evidence_requirements"]) == 10
    assert all(
        requirement["status"] != "admitted"
        for requirement in report["evidence_requirements"]
    )
    assert "persistence.successful_replicate_vectors_not_replayable_from_archive" in report[
        "promotion_blockers"
    ]
    assert "oracle.base_r_microreference_not_executed_in_this_scaffold_gate" in report[
        "promotion_blockers"
    ]


def test_promotion_cli_has_separate_scaffold_and_fail_closed_exit_codes() -> None:
    scaffold = subprocess.run(
        [
            sys.executable,
            str(VALIDATION / "consistent_bootstrap_v1_promotion_audit.py"),
            "--scaffold-only",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert scaffold.returncode == 0, scaffold.stderr or scaffold.stdout
    scaffold_report = json.loads(scaffold.stdout)
    assert scaffold_report["scaffold_valid"] is True
    assert scaffold_report["promotion_allowed"] is False

    promotion = subprocess.run(
        [sys.executable, str(VALIDATION / "consistent_bootstrap_v1_promotion_audit.py")],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert promotion.returncode == 1
    promotion_report = json.loads(promotion.stdout)
    assert promotion_report["passed"] is False
    assert promotion_report["decision"] == "BLOCK_PROMOTION_KEEP_EVIDENCE_ABSENT"


def test_r_reference_is_explicitly_arithmetic_only_and_non_promotional() -> None:
    source = promotion_audit.R_REFERENCE_PATH.read_text(encoding="utf-8")
    assert "does not implement the PLSc estimator" in source
    assert "quantile(" in source
    assert "type = 7" in source
    assert "pnorm(" in source
    assert "qnorm(" in source
    assert "qualification_evidence=false" in source


def test_qualification_document_discloses_current_blockers_and_evidence_boundaries() -> None:
    document = promotion_audit.DOC_PATH.read_text(encoding="utf-8")
    for required in (
        "evidence: `absent`",
        "admitted immutable receipts: zero",
        "does not qualify or promote",
        "The 10-replicate fixture is intentionally below the executable product minimum",
        "successful parameter digests but not the corresponding successful parameter vectors",
        "CSV, XLSX, HTML, SVG, PDF, and PNG",
        "The second is the promotion decision and must exit nonzero",
    ):
        assert required in document
