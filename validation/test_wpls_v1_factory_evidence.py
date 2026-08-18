from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest


VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from method_promotion_manifest import validate_manifest  # noqa: E402
from wpls_reference import estimate_reference, weighted_cov, weighted_dof, weighted_mean  # noqa: E402
from wpls_v1_factory_common import (  # noqa: E402
    MANIFEST_PATH,
    DuplicateKeyError,
    strict_load_json,
)
from wpls_v1_simulation import scenarios  # noqa: E402


def test_strict_json_loader_rejects_duplicate_and_nonfinite_evidence(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"passed": true, "passed": false}\n', encoding="utf-8")
    with pytest.raises(DuplicateKeyError):
        strict_load_json(duplicate)

    nonfinite = tmp_path / "nonfinite.json"
    nonfinite.write_text('{"value": NaN}\n', encoding="utf-8")
    with pytest.raises(ValueError, match="non-finite"):
        strict_load_json(nonfinite)


def test_published_weighted_moment_fixture_uses_unbiased_denominator() -> None:
    values = [1.0, 2.0, 5.0]
    weights = [1.0, 2.0, 3.0]
    assert weighted_mean(values, weights) == pytest.approx(10.0 / 3.0)
    assert weighted_dof(weights) == pytest.approx(11.0 / 3.0)
    assert weighted_cov(values, values, weights) == pytest.approx(52.0 / 11.0)


def test_independent_reference_is_exactly_invariant_to_positive_weight_scaling() -> None:
    rows = [
        {"x1": 1.0, "x2": 1.2, "y1": 2.0, "y2": 2.1, "case_wt": 0.5},
        {"x1": 2.0, "x2": 1.8, "y1": 2.8, "y2": 3.0, "case_wt": 1.0},
        {"x1": 3.0, "x2": 3.2, "y1": 4.2, "y2": 4.0, "case_wt": 1.5},
        {"x1": 4.0, "x2": 3.9, "y1": 5.0, "y2": 5.2, "case_wt": 2.0},
        {"x1": 5.0, "x2": 5.1, "y1": 6.2, "y2": 6.0, "case_wt": 2.5},
        {"x1": 6.0, "x2": 5.8, "y1": 6.9, "y2": 7.1, "case_wt": 3.0},
    ]
    scaled = [{**row, "case_wt": row["case_wt"] * 17.0} for row in rows]
    first = estimate_reference(rows)
    second = estimate_reference(scaled)
    assert first["paths"] == pytest.approx(second["paths"])
    assert first["r_squared"] == pytest.approx(second["r_squared"])
    assert first["loadings"] == pytest.approx(second["loadings"])
    assert first["weights"] == pytest.approx(second["weights"])
    assert first["effective_sample_size"] == pytest.approx(second["effective_sample_size"])
    assert second["weight_sum"] == pytest.approx(first["weight_sum"] * 17.0)


def test_simulation_matrix_freezes_required_weight_and_data_coverage() -> None:
    matrix = scenarios()
    assert len(matrix) == 5
    assert {row.weight_pattern for row in matrix} == {"uniform", "moderate", "strong"}
    assert {row.weighting_scheme for row in matrix} == {"path", "factor"}
    assert any(row.missing for row in matrix)
    assert any(row.non_gaussian for row in matrix)


def test_factory_does_not_build_or_claim_gui_runtime() -> None:
    common = (VALIDATION / "wpls_v1_factory_common.py").read_text(encoding="utf-8")
    evidence = (VALIDATION / "wpls_v1_factory_evidence.py").read_text(encoding="utf-8")
    audit = (VALIDATION / "wpls_v1_factory_audit.py").read_text(encoding="utf-8")
    assert '"built_by_factory": False' in common
    assert "cargo\", \"build" not in common
    assert '"--allow-internal-qualification"' in common
    assert '"gui_runtime_claimed": False' in evidence
    assert 'native.get("checks", {}).get("invalid_weight_blocked") is True' in audit
    assert '"exact_run_export_and_reopen_pass"' in audit


def test_release_contract_is_frozen_but_runtime_evidence_remains_fail_closed() -> None:
    report = validate_manifest(MANIFEST_PATH, MANIFEST_PATH.parents[2])
    assert report["declared_state"] == "release_qualified"
    assert report["target_state"] == "release_qualified"
    if report["passed"]:
        assert report["derived_state"] == "release_qualified"
    else:
        assert report["derived_state"] != "release_qualified"
        assert report["errors"], json.dumps(report, indent=2)
    contract = validate_manifest(MANIFEST_PATH, MANIFEST_PATH.parents[2], verify_evidence=False)
    assert contract["passed"], json.dumps(contract, indent=2)
    assert contract["derived_state"] == "release_qualified"
