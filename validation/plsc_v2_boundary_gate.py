"""Executable PLSc v2 applicability, metamorphic, and failure boundaries."""

from __future__ import annotations

import copy
import math
import random
from typing import Any

from plsc_v2_factory_common import (
    REPORT_ROOT,
    ROOT,
    WORK_ROOT,
    construct,
    repository_path,
    run_plsc,
    strict_load_json,
    write_csv,
    write_identity_report,
)


SOURCE = "validation/plsc_v2_boundary_gate.py"
TOLERANCE = 1e-10


def _rows(seed: int = 20_261_001, n: int = 260) -> list[list[float]]:
    rng = random.Random(seed)
    rows: list[list[float]] = []
    for _ in range(n):
        x = rng.gauss(0.0, 1.0)
        y = 0.46 * x + math.sqrt(1.0 - 0.46**2) * rng.gauss(0.0, 1.0)
        rows.append(
            [
                x + rng.gauss(0.0, 0.12),
                0.94 * x + rng.gauss(0.0, 0.13),
                0.90 * x + rng.gauss(0.0, 0.15),
                y + rng.gauss(0.0, 0.12),
                0.95 * y + rng.gauss(0.0, 0.14),
                0.91 * y + rng.gauss(0.0, 0.15),
            ]
        )
    return rows


def canonical(run: dict[str, Any]) -> dict[str, Any]:
    plsc = run["plsc"]
    return {
        "rho_a": {row["construct"]: row["rho_a"] for row in plsc["reliabilities"]},
        "correlations": {
            tuple(sorted((row["left"], row["right"]))): {
                "original": row["original"],
                "corrected": row["corrected"],
            }
            for row in plsc["construct_correlations"]
        },
        "paths": {
            (row["source"], row["target"]): row["coefficient"]
            for row in plsc["corrected_paths"]
        },
        "r_squared": plsc["corrected_r_squared"],
        "loadings": {
            (row["construct"], row["indicator"]): row["loading"]
            for row in plsc["corrected_outer_loadings"]
        },
    }


def _numeric_delta(left: Any, right: Any) -> float:
    if isinstance(left, dict) and isinstance(right, dict):
        if set(left) != set(right):
            return math.inf
        return max((_numeric_delta(left[key], right[key]) for key in left), default=0.0)
    if isinstance(left, (int, float)) and isinstance(right, (int, float)):
        return abs(float(left) - float(right))
    return 0.0 if left == right else math.inf


def _current_identity(document: dict[str, Any]) -> bool:
    try:
        estimation = document["payload"]["estimation"]
        plsc = estimation["plsc"]
        provenance = document["provenance"]
        return (
            document["payload"]["kind"] == "pls_pm_v1"
            and estimation["method_version"] == "plsc_v2"
            and plsc["method_version"] == "plsc_v2"
            and plsc["reliability_method_version"] == "dijkstra_henseler_rho_a_v1"
            and provenance["method"] == "plsc"
            and str(provenance["method_version"]).startswith("pls_pm_v1+plsc_v2")
        )
    except (KeyError, TypeError):
        return False


def _invalid_case(
    case_id: str,
    expected_code: str,
    csv_path,
    constructs,
    paths,
    **kwargs,
) -> dict[str, Any]:
    run = run_plsc(
        name=f"factory_boundary_{case_id}",
        csv_path=csv_path,
        constructs=constructs,
        paths=paths,
        expect_success=False,
        **kwargs,
    )
    output = run["execution"]["stdout_tail"] + run["execution"]["stderr_tail"]
    return {
        "passed": run["passed"] and expected_code in output,
        "expected_code": expected_code,
        "diagnostic_observed": expected_code in output,
        "result_not_written": run["result_not_written"],
        "recipe": run["recipe"],
        "execution": run["execution"],
    }


def main() -> int:
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    csv_path = WORK_ROOT / "boundary_baseline.csv"
    write_csv(csv_path, ["x1", "x2", "x3", "y1", "y2", "y3"], _rows())
    baseline_constructs = [
        construct("x", ["x1", "x2", "x3"]),
        construct("y", ["y1", "y2", "y3"]),
    ]
    paths = [{"source": "x", "target": "y"}]
    baseline = run_plsc(
        name="factory_boundary_baseline",
        csv_path=csv_path,
        constructs=baseline_constructs,
        paths=paths,
    )
    repeat = run_plsc(
        name="factory_boundary_repeat",
        csv_path=csv_path,
        constructs=baseline_constructs,
        paths=paths,
    )
    reordered = run_plsc(
        name="factory_boundary_constructs_indicators_reordered",
        csv_path=csv_path,
        constructs=[
            construct("y", ["y3", "y2", "y1"]),
            construct("x", ["x3", "x2", "x1"]),
        ],
        paths=paths,
    )
    baseline_values = canonical(baseline)
    repeat_values = canonical(repeat)
    reordered_values = canonical(reordered)
    determinism = {
        "passed": baseline_values == repeat_values,
        "bit_identical_analytical_payload": baseline_values == repeat_values,
    }
    metamorphic_delta = _numeric_delta(baseline_values, reordered_values)
    metamorphic = {
        "passed": metamorphic_delta <= TOLERANCE,
        "max_mapped_delta": metamorphic_delta,
        "tolerance": TOLERANCE,
        "construct_and_indicator_order_changed_together": True,
    }

    invalid: dict[str, Any] = {}
    invalid["formative"] = _invalid_case(
        "formative",
        "plsc.reflective_only",
        csv_path,
        [construct("x", ["x1", "x2", "x3"], mode="formative"), baseline_constructs[1]],
        paths,
    )
    invalid["single_indicator"] = _invalid_case(
        "single_indicator",
        "plsc.minimum_indicators",
        csv_path,
        [construct("x", ["x1"]), baseline_constructs[1]],
        paths,
    )
    invalid["pca_weighting"] = _invalid_case(
        "pca_weighting",
        "plsc.pca_unsupported",
        csv_path,
        baseline_constructs,
        paths,
        weighting_scheme="pca",
    )
    invalid["bootstrap_samples_below_minimum"] = _invalid_case(
        "bootstrap_samples_below_minimum",
        "plsc.consistent_bootstrap_samples",
        csv_path,
        baseline_constructs,
        paths,
        settings_overrides={"bootstrap_samples": 100},
    )

    corporate_recipe = strict_load_json(
        ROOT / "validation" / "fixtures" / "corporate_reputation.recipe.json"
    )
    inadmissible = run_plsc(
        name="factory_boundary_inadmissible_correlation",
        csv_path=ROOT / "validation" / "fixtures" / "corporate_reputation.csv",
        constructs=corporate_recipe["model"]["constructs"],
        paths=corporate_recipe["model"]["paths"],
        expect_success=False,
    )
    inadmissible_text = (
        inadmissible["execution"]["stdout_tail"] + inadmissible["execution"]["stderr_tail"]
    )
    inadmissibility_diagnostics = [
        "corrected construct correlation is outside [-1, 1]",
        "invalid PLSc rho_A",
    ]
    observed_inadmissibility = [
        diagnostic for diagnostic in inadmissibility_diagnostics if diagnostic in inadmissible_text
    ]
    data_pathology = {
        "passed": inadmissible["passed"] and bool(observed_inadmissibility),
        "result_not_written": inadmissible["result_not_written"],
        "accepted_inadmissibility_diagnostics": inadmissibility_diagnostics,
        "inadmissibility_diagnostics_observed": observed_inadmissibility,
        "recipe": inadmissible["recipe"],
        "execution": inadmissible["execution"],
    }

    tampered = copy.deepcopy(baseline["result"])
    tampered["payload"]["estimation"]["method_version"] = "plsc_v999"
    tamper = {
        "passed": _current_identity(baseline["result"]) and not _current_identity(tampered),
        "baseline_current_identity": _current_identity(baseline["result"]),
        "method_version_tamper_rejected_by_strict_identity": not _current_identity(tampered),
        "archive_level_tamper_is_required_in_persistence_stage": True,
    }
    categories = {
        "data_pathology": data_pathology,
        "unsupported_scope": {
            "passed": all(row["passed"] for row in invalid.values()),
            "cases": invalid,
        },
        "metamorphic": metamorphic,
        "determinism": determinism,
        "tamper": tamper,
    }
    checks = {
        "passed": baseline["passed"]
        and repeat["passed"]
        and reordered["passed"]
        and all(row["passed"] for row in categories.values()),
        "required_categories": categories,
        "baseline_identity": baseline["identity_passed"],
    }
    generated = [
        repository_path(csv_path),
        baseline["recipe"],
        baseline["output"],
        repeat["recipe"],
        repeat["output"],
        reordered["recipe"],
        reordered["output"],
        *(row["recipe"] for row in invalid.values()),
        inadmissible["recipe"],
    ]
    report = write_identity_report(
        "boundary_report",
        passed=checks["passed"],
        checks=checks,
        extras=[
            SOURCE,
            "validation/plsc_unsupported_guard.py",
            "validation/fixtures/corporate_reputation.csv",
            "validation/fixtures/corporate_reputation.recipe.json",
            "crates/qpls-core/src/validation.rs",
            "crates/qpls-estimation/src/pls.rs",
            *generated,
        ],
    )
    print(f"wrote {report} | passed={checks['passed']} | metamorphic_delta={metamorphic_delta}")
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
