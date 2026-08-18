"""Preregistered six-shape simulation/reference matrix for CB-SEM ML v1."""

from __future__ import annotations

from cbsem_ml_v1_factory_common import ROOT, strict_load_json, write_identity_report
from cbsem_ml_v1_reference import EXPECTED_MODELS


SOURCE = "validation/cbsem_ml_v1_simulation.py"
REFERENCE_IDENTITY = (
    "validation/results/method_factory/cbsem_ml_v1/independent_reference.identity.json"
)


def run_simulation_matrix() -> dict:
    identity = strict_load_json(ROOT / REFERENCE_IDENTITY)
    reference = identity.get("checks", {})
    rows = {row["model"]: row for row in reference.get("models", [])}
    coverage = {
        "measurement_shapes": all(
            name in rows
            for name in ("one_factor_cfa", "two_factor_cfa", "three_factor_cfa")
        ),
        "recursive_path_shapes": all(
            name in rows
            for name in (
                "latent_regression_sem",
                "latent_mediation_sem",
                "correlated_exogenous_sem",
            )
        ),
        "all_six_models_converged_and_matched": (
            set(rows) == EXPECTED_MODELS
            and all(row.get("passed") is True for row in rows.values())
        ),
    }
    maxima = {
        metric: max(float(row[metric]) for row in rows.values())
        for metric in (
            "max_estimate_delta",
            "max_fit_delta",
            "max_standardized_delta",
            "max_standard_error_delta",
            "max_z_delta",
            "max_p_delta",
        )
    } if rows else {}
    return {
        "passed": reference.get("passed") is True and all(coverage.values()),
        "design": (
            "Six deterministic 240-case generated fixtures cover one-, two-, and "
            "three-factor CFA plus recursive regression, mediation, and correlated-"
            "exogenous SEM; identical data and syntax are compared with lavaan 0.7.2."
        ),
        "seed_policy": "cbsem_lavaan_reference.py fixes one seed per named model",
        "coverage": coverage,
        "maximum_observed_deltas": maxima,
        "models": sorted(rows),
    }


def main() -> int:
    checks = run_simulation_matrix()
    path = write_identity_report(
        "simulation_report",
        passed=checks["passed"],
        checks=checks,
        extras=[
            SOURCE,
            "validation/cbsem_ml_v1_reference.py",
            REFERENCE_IDENTITY,
        ],
    )
    print(f"wrote {path} | passed={checks['passed']}")
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
