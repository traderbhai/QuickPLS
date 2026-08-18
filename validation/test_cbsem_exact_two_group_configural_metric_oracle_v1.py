from __future__ import annotations

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

from validation import cbsem_exact_two_group_configural_metric_oracle_v1 as oracle


REPORT = (
    oracle.ROOT
    / "validation/results/cbsem_exact_two_group_configural_metric/reference-report-v2.json"
)
REJECTED_REPORT = (
    oracle.ROOT
    / "validation/results/cbsem_exact_two_group_configural_metric/reference-report-v1.json"
)


def test_immutable_manifest_and_all_bound_sources_validate() -> None:
    manifest, digest = oracle.validate_manifest(oracle.DEFAULT_MANIFEST)
    assert len(manifest["source_bindings"]) == 9
    assert len({row["role"] for row in manifest["source_bindings"]}) == 9
    assert len({row["path"] for row in manifest["source_bindings"]}) == 9
    assert oracle.SHA256_PATTERN.fullmatch(digest)


def test_primary_fixture_and_exact_label_swap_are_frozen() -> None:
    manifest, _ = oracle.validate_manifest(oracle.DEFAULT_MANIFEST)
    primary = oracle.read_fixture(
        oracle.ROOT / manifest["fixture_contract"]["primary_path"]
    )
    swapped = oracle.read_fixture(
        oracle.ROOT / manifest["fixture_contract"]["label_swap_path"],
        {"A": 220, "B": 180},
    )
    oracle.validate_label_swap(primary, swapped)
    assert primary["A"].shape == (180, 6)
    assert primary["B"].shape == (220, 6)


def test_independent_python_joint_ml_reference_matches_frozen_solution() -> None:
    manifest, _ = oracle.validate_manifest(oracle.DEFAULT_MANIFEST)
    primary = oracle.read_fixture(
        oracle.ROOT / manifest["fixture_contract"]["primary_path"]
    )
    result = oracle.fit_python_reference(primary)
    assert result["configural"]["free_dimensions"] == 26
    assert result["configural"]["degrees_of_freedom"] == 16
    assert result["metric"]["free_dimensions"] == 22
    assert result["metric"]["degrees_of_freedom"] == 20
    assert result["likelihood_ratio_test"]["degrees_of_freedom"] == 4
    assert result["configural"]["chi_square"] == pytest.approx(
        16.84210524994615, abs=1e-8
    )
    assert result["metric"]["chi_square"] == pytest.approx(
        38.46858758317431, abs=1e-8
    )
    assert result["likelihood_ratio_test"]["statistic"] == pytest.approx(
        21.62648233322816, abs=1e-8
    )
    metric_groups = result["metric"]["groups"]
    for loading in ("loading_x2", "loading_x3", "loading_y2", "loading_y3"):
        assert metric_groups[0]["parameters"][loading] == metric_groups[1]["parameters"][
            loading
        ]


def test_checked_in_report_is_schema_valid_passed_and_quarantined() -> None:
    report = oracle.load_json(REPORT)
    schema = oracle.load_json(oracle.REPORT_SCHEMA)
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(report)
    assert report["status"] == (
        "engine_reference_evidence_v2_passed_reproducible_product_qualification_blocked"
    )
    assert report["engine_python_parity"]["status"] == "accepted"
    assert report["engine_python_parity"]["failures"] == []
    assert report["lavaan_reference"] == {
        "reason": "Rscript_not_found",
        "required_contract": report["lavaan_reference"]["required_contract"],
        "script_sha256": report["lavaan_reference"]["script_sha256"],
        "status": "unavailable_current_host",
    }
    assert report["qualification_boundary"]["status"] == (
        "blocked_not_product_qualification"
    )
    assert report["supersedes"] == {
        "disposition": "rejected_nondeterministic_import_dataset_identity",
        "path": "validation/results/cbsem_exact_two_group_configural_metric/reference-report-v1.json",
        "reason": "import_generated_random_dataset_uuids_changed_full_engine_json_between_executions",
        "sha256": "bc430dbd7907d39ed95f278ce8cfa2e8cf28478f1a3486226bcad2192350b919",
    }


def test_frozen_distinct_dataset_uuids_and_repeat_run_reproducibility() -> None:
    manifest, _ = oracle.validate_manifest(oracle.DEFAULT_MANIFEST)
    uuids = manifest["fixture_contract"]["dataset_uuids"]
    assert uuids == {
        "primary": "cb5e2a01-0000-0000-0000-000000000001",
        "label_swap": "cb5e2a01-0000-0000-0000-000000000002",
    }
    assert uuids["primary"] != uuids["label_swap"]
    binary = next(
        oracle.ROOT / row["path"]
        for row in manifest["source_bindings"]
        if row["role"] == "release_validation_binary"
    )
    payload, _, reproducibility = oracle.run_engine(binary, manifest)
    assert payload["source_binding"]["primary_dataset_uuid"] == uuids["primary"]
    assert payload["source_binding"]["label_swap_dataset_uuid"] == uuids["label_swap"]
    assert payload["primary"]["data"]["dataset_id"] == uuids["primary"]
    assert payload["label_swap"]["data"]["dataset_id"] == uuids["label_swap"]
    assert reproducibility["status"] == "accepted"
    assert reproducibility["raw_stdout_byte_identical"] is True
    assert reproducibility["canonical_json_identical"] is True
    assert len(set(reproducibility["raw_stdout_sha256"])) == 1
    assert len(set(reproducibility["canonical_full_engine_json_sha256"])) == 1


def test_rejected_v1_report_is_preserved_byte_exactly() -> None:
    assert oracle.sha256(REJECTED_REPORT) == (
        "bc430dbd7907d39ed95f278ce8cfa2e8cf28478f1a3486226bcad2192350b919"
    )


def test_manifest_validation_fails_closed_on_bound_source_digest_drift(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    real_sha256 = oracle.sha256

    def drifted(path: Path) -> str:
        if path.name == "cbsem_exact_two_group_invariance.rs":
            return "0" * 64
        return real_sha256(path)

    monkeypatch.setattr(oracle, "sha256", drifted)
    with pytest.raises(ValueError, match="immutable source binding drifted"):
        oracle.validate_manifest(oracle.DEFAULT_MANIFEST)


def test_malformed_fixture_is_rejected_before_fitting(tmp_path: Path) -> None:
    malformed = tmp_path / "malformed.csv"
    malformed.write_text(
        "group,x1,x2,x3,y1,y2,y3\nA,1,2,3,4,5,not-finite\n",
        encoding="utf-8",
    )
    with pytest.raises(ValueError):
        oracle.read_fixture(malformed)


def test_lavaan_script_freezes_required_options_without_claiming_execution() -> None:
    script = (
        oracle.ROOT
        / "validation/cbsem_exact_two_group_configural_metric_oracle_v1.R"
    ).read_text(encoding="utf-8")
    for required in (
        'estimator = "ML"',
        'missing = "listwise"',
        "meanstructure = FALSE",
        "fixed.x = FALSE",
        'metric_args$group.equal <- "loadings"',
        'lower.tail = FALSE',
    ):
        assert required in script
    report = json.loads(REPORT.read_text(encoding="utf-8"))
    assert report["lavaan_reference"]["status"] == "unavailable_current_host"
