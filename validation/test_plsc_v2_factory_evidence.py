from __future__ import annotations

import json
import sys
from pathlib import Path

import numpy as np
import pytest


VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from method_promotion_manifest import validate_manifest  # noqa: E402
from plsc_reference import rho_a  # noqa: E402
from plsc_v2_boundary_gate import _current_identity, _numeric_delta  # noqa: E402
from plsc_v2_factory_common import (  # noqa: E402
    MANIFEST_PATH,
    DuplicateKeyError,
    strict_load_json,
)
from plsc_v2_simulation import SCENARIOS  # noqa: E402


def test_strict_json_loader_rejects_duplicate_and_nonfinite_evidence(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"passed": true, "passed": false}\n', encoding="utf-8")
    with pytest.raises(DuplicateKeyError):
        strict_load_json(duplicate)

    nonfinite = tmp_path / "nonfinite.json"
    nonfinite.write_text('{"rho_a": NaN}\n', encoding="utf-8")
    with pytest.raises(ValueError, match="non-finite"):
        strict_load_json(nonfinite)


def test_equicorrelation_hand_fixture_reproduces_rho_a_three_quarters() -> None:
    correlation = np.full((3, 3), 0.5)
    np.fill_diagonal(correlation, 1.0)
    centered_basis = np.asarray(
        [
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, -1.0],
            [1.0, -1.0, -1.0],
            [-1.0, -1.0, 1.0],
        ]
    ) / 2.0
    standardized = centered_basis @ np.linalg.cholesky(correlation).T * np.sqrt(3.0)
    columns = [standardized[:, index] for index in range(3)]
    assert np.corrcoef(standardized, rowvar=False) == pytest.approx(correlation, abs=1e-12)
    assert rho_a(columns, [1.0, 1.0, 1.0]) == pytest.approx(0.75, abs=1e-12)


def test_simulation_registry_is_preregistered_and_spans_required_supported_dimensions() -> None:
    ids = [row["id"] for row in SCENARIOS]
    seeds = [row["seed"] for row in SCENARIOS]
    assert len(ids) == len(set(ids)) >= 5
    assert len(seeds) == len(set(seeds))
    assert {row["weighting"] for row in SCENARIOS} == {"path", "factor"}
    assert {row["distribution"] for row in SCENARIOS} == {"gaussian", "laplace"}
    assert 2 in {row["indicators"] for row in SCENARIOS}
    assert min(row["n"] for row in SCENARIOS) >= 200


def test_boundary_comparison_is_identity_aware_and_tolerant_only_for_numeric_drift() -> None:
    assert _numeric_delta({"x": 0.5}, {"x": 0.5 + 1e-12}) == pytest.approx(1e-12)
    assert _numeric_delta({"x": 0.5}, {"y": 0.5}) == float("inf")
    document = {
        "payload": {
            "kind": "pls_pm_v1",
            "estimation": {
                "method_version": "plsc_v2",
                "plsc": {
                    "method_version": "plsc_v2",
                    "reliability_method_version": "dijkstra_henseler_rho_a_v1",
                },
            },
        },
        "provenance": {"method": "plsc", "method_version": "pls_pm_v1+plsc_v2"},
    }
    assert _current_identity(document)
    document["payload"]["estimation"]["method_version"] = "plsc_v1"
    assert not _current_identity(document)


def test_release_contract_is_frozen_but_runtime_evidence_remains_fail_closed() -> None:
    report = validate_manifest(MANIFEST_PATH, MANIFEST_PATH.parents[2])
    assert not report["passed"], json.dumps(report, indent=2)
    assert report["declared_state"] == "native_qualified"
    assert report["derived_state"] == "native_qualified"
    assert report["target_state"] == "release_qualified"
    assert any("packaged_acceptance.identity.json" in error for error in report["errors"])
    contract = validate_manifest(MANIFEST_PATH, MANIFEST_PATH.parents[2], verify_evidence=False)
    assert contract["passed"], json.dumps(contract, indent=2)
    assert contract["derived_state"] == "release_qualified"
