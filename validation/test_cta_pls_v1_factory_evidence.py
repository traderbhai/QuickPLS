from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest


VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from cta_pls_boundary_gate import canonical_tetrads  # noqa: E402
from cta_pls_simulation import independent_tetrads  # noqa: E402
from cta_pls_v1_factory_common import (  # noqa: E402
    MANIFEST_PATH,
    DuplicateKeyError,
    strict_load_json,
)
from method_promotion_manifest import validate_manifest  # noqa: E402


def test_strict_json_loader_rejects_ambiguous_or_nonfinite_evidence(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"passed": true, "passed": false}\n', encoding="utf-8")
    with pytest.raises(DuplicateKeyError):
        strict_load_json(duplicate)

    nonfinite = tmp_path / "nonfinite.json"
    nonfinite.write_text('{"value": NaN}\n', encoding="utf-8")
    with pytest.raises(ValueError, match="non-finite"):
        strict_load_json(nonfinite)


def test_independent_hand_tetrad_uses_sample_covariance_and_all_three_pairings() -> None:
    rows = [
        [-1.0, 0.0, -1.0, -1.0],
        [0.0, -1.0, -1.0, 1.0],
        [1.0, 0.0, 1.0, 1.0],
        [0.0, 1.0, 1.0, -1.0],
    ]
    reference = independent_tetrads(
        rows,
        ["a", "b", "c", "d"],
        {"x": ["a", "b", "c", "d"]},
        "mean_centered",
    )
    values = reference["estimates"]
    assert values[("x", "a", "b", "c", "d", "ab_cd_minus_ac_bd")] == pytest.approx(4 / 9)
    assert values[("x", "a", "b", "c", "d", "ac_bd_minus_ad_bc")] == pytest.approx(-8 / 9)
    assert values[("x", "a", "b", "c", "d", "ad_bc_minus_ab_cd")] == pytest.approx(4 / 9)
    assert sum(values.values()) == pytest.approx(0.0)
    assert reference["maxima"] == {"x": pytest.approx(8 / 9)}


def _run(order: tuple[str, str, str, str], tetrads: tuple[float, float, float]) -> dict:
    pairings = (
        "ab_cd_minus_ac_bd",
        "ac_bd_minus_ad_bc",
        "ad_bc_minus_ab_cd",
    )
    return {
        "cta": {
            "estimates": [
                {
                    "construct": "x",
                    "indicator_a": order[0],
                    "indicator_b": order[1],
                    "indicator_c": order[2],
                    "indicator_d": order[3],
                    "pairing": pairing,
                    "tetrad": value,
                }
                for pairing, value in zip(pairings, tetrads)
            ]
        }
    }


def test_canonical_tetrads_preserve_signed_algebra_under_indicator_reorder() -> None:
    baseline = _run(("a", "b", "c", "d"), (1.0, 2.0, -3.0))
    reordered = _run(("b", "a", "c", "d"), (3.0, -2.0, -1.0))
    assert canonical_tetrads(baseline) == canonical_tetrads(reordered)


def test_current_manifest_is_staged_for_exact_release_qualification() -> None:
    report = validate_manifest(MANIFEST_PATH, MANIFEST_PATH.parents[2])
    packaged = MANIFEST_PATH.parents[1] / "results" / "method_factory" / "cta_pls_v1" / "packaged_acceptance.identity.json"
    audit = MANIFEST_PATH.parents[1] / "results" / "method_factory" / "cta_pls_v1" / "method_audit.identity.json"
    if packaged.is_file() and audit.is_file():
        assert report["passed"], json.dumps(report, indent=2)
        assert report["derived_state"] == "release_qualified"
    else:
        assert not report["passed"]
        assert report["derived_state"] in {"absent", "native_qualified"}
        assert any("evidence file is missing" in error or "mismatch" in error for error in report["errors"])
    assert report["declared_state"] == "release_qualified"
    assert report["target_state"] == "release_qualified"
