#!/usr/bin/env python3
"""Executable boundary and metamorphic gate for PLS power v1."""

from __future__ import annotations

import json
from copy import deepcopy
from typing import Any

from pls_sample_size_power_simulation import (
    EXECUTION_SOURCES,
    FEATURE_ID,
    METHOD_VERSION,
    canonical_recipe,
    ensure_fixture_dataset,
    require_current_cli,
    require_stable_execution_sources,
    run_product_power,
    validate_product_analysis,
    write_identity_report,
)


SOURCE = "validation/pls_sample_size_power_boundary_gate.py"


def _outcomes_by_key(analysis: dict[str, Any]) -> dict[tuple[int, int], dict[str, Any]]:
    return {
        (int(row["sample_size"]), int(row["replicate_index"])): row
        for row in analysis["outcomes"]
    }


def _invalid_case(
    *,
    name: str,
    recipe: dict[str, Any],
    dataset,
    expected_fragment: str,
) -> tuple[dict[str, Any], dict[str, Any]]:
    run = run_product_power(
        name=name,
        recipe=recipe,
        dataset=dataset,
        expect_success=False,
        timeout=120,
    )
    combined = (
        run["execution"].get("stdout_tail", "")
        + "\n"
        + run["execution"].get("stderr_tail", "")
    ).lower()
    result = {
        "name": name,
        "passed": run["passed"] and expected_fragment.lower() in combined,
        "nonzero_exit": run["returncode"] != 0,
        "no_result_created": run["result"] is None,
        "expected_fragment": expected_fragment,
        "message_observed": expected_fragment.lower() in combined,
    }
    return result, run["execution"]


def main() -> int:
    cli_identity = require_current_cli()
    before = cli_identity["execution_sources"]
    dataset, fingerprint, fixture_execution = ensure_fixture_dataset()
    executions: list[dict[str, Any]] = [fixture_execution]

    baseline_recipe = canonical_recipe(
        fingerprint,
        name="boundary_baseline",
        population_path=0.40,
        sample_size_grid=(60, 120),
        monte_carlo_replicates=100,
        bootstrap_replicates=99,
        target_power=0.70,
        seed=20_260_818,
        workers=1,
    )
    baseline = run_product_power(
        name="boundary_baseline", recipe=baseline_recipe, dataset=dataset
    )
    executions.append(baseline["execution"])
    baseline_contract = validate_product_analysis(baseline_recipe, baseline["analysis"])

    extended_recipe = deepcopy(baseline_recipe)
    extended_recipe["id"] = "00000000-0000-0000-0000-000000250402"
    extended_recipe["method_config"]["sample_size_grid"] = [60, 120, 180]
    extended = run_product_power(
        name="boundary_grid_extended", recipe=extended_recipe, dataset=dataset
    )
    executions.append(extended["execution"])
    base_outcomes = _outcomes_by_key(baseline["analysis"])
    extended_outcomes = _outcomes_by_key(extended["analysis"])
    extension_keys_preserved = all(
        extended_outcomes.get(key) == value for key, value in base_outcomes.items()
    )
    appended_keys = sorted(set(extended_outcomes) - set(base_outcomes))
    extension_check = {
        "passed": extension_keys_preserved
        and len(appended_keys) == 100
        and all(sample_size == 180 for sample_size, _ in appended_keys),
        "existing_outcomes_exact": extension_keys_preserved,
        "appended_outcome_count": len(appended_keys),
        "appended_sample_sizes": sorted({sample_size for sample_size, _ in appended_keys}),
        "base_recipe_digest": baseline["analysis"]["recipe_digest"],
        "extended_recipe_digest": extended["analysis"]["recipe_digest"],
        "recipe_digest_changes_with_grid": baseline["analysis"]["recipe_digest"]
        != extended["analysis"]["recipe_digest"],
    }
    extension_check["passed"] = extension_check["passed"] and extension_check[
        "recipe_digest_changes_with_grid"
    ]

    worker_recipe = deepcopy(baseline_recipe)
    worker_recipe["id"] = "00000000-0000-0000-0000-000000250403"
    worker_recipe["settings"]["workers"] = 4
    worker = run_product_power(
        name="boundary_workers_4", recipe=worker_recipe, dataset=dataset
    )
    executions.append(worker["execution"])
    worker_check = {
        "passed": baseline["analysis"] == worker["analysis"],
        "analytical_payload_exact": baseline["analysis"] == worker["analysis"],
        "outer_worker_counts": [
            baseline["document"]["provenance"]["settings"]["workers"],
            worker["document"]["provenance"]["settings"]["workers"],
        ],
        "outer_provenance_preserves_operational_workers": baseline["document"]["provenance"]
        ["settings"]["workers"]
        == 1
        and worker["document"]["provenance"]["settings"]["workers"] == 4,
    }
    worker_check["passed"] = worker_check["passed"] and worker_check[
        "outer_provenance_preserves_operational_workers"
    ]

    invalid_cases: list[dict[str, Any]] = []

    non_increasing = deepcopy(baseline_recipe)
    non_increasing["method_config"]["sample_size_grid"] = [120, 60]
    row, execution = _invalid_case(
        name="boundary_non_increasing_grid",
        recipe=non_increasing,
        dataset=dataset,
        expected_fragment="strictly increasing",
    )
    invalid_cases.append(row)
    executions.append(execution)

    too_few = deepcopy(baseline_recipe)
    too_few["method_config"]["predictor_indicator_loadings"] = [0.8, 0.8]
    row, execution = _invalid_case(
        name="boundary_too_few_loadings",
        recipe=too_few,
        dataset=dataset,
        expected_fragment="declared loadings must map one-to-one to model indicators",
    )
    invalid_cases.append(row)
    executions.append(execution)

    unsupported = deepcopy(baseline_recipe)
    unsupported["model"]["paths"].append({"source": "y", "target": "x"})
    row, execution = _invalid_case(
        name="boundary_unsupported_model_family",
        recipe=unsupported,
        dataset=dataset,
        expected_fragment="exactly two constructs and the single declared",
    )
    invalid_cases.append(row)
    executions.append(execution)

    excessive = deepcopy(baseline_recipe)
    excessive["method_config"]["sample_size_grid"] = [1_000, 2_000]
    excessive["method_config"]["monte_carlo_replicates"] = 1_251
    row, execution = _invalid_case(
        name="boundary_desktop_workload_cap",
        recipe=excessive,
        dataset=dataset,
        expected_fragment="250000 PLS fits",
    )
    invalid_cases.append(row)
    executions.append(execution)

    confidence_mismatch = deepcopy(baseline_recipe)
    confidence_mismatch["settings"]["confidence_level"] = 0.90
    row, execution = _invalid_case(
        name="boundary_confidence_identity",
        recipe=confidence_mismatch,
        dataset=dataset,
        expected_fragment="must be exactly equal",
    )
    invalid_cases.append(row)
    executions.append(execution)

    source_stability = require_stable_execution_sources(before)
    checks = {
        "baseline_typed_result": baseline["passed"],
        "baseline_contract_recomputed": baseline_contract["passed"],
        "grid_extension_invariance": extension_check["passed"],
        "seed_worker_invariance": worker_check["passed"],
        "invalid_setup_blocked_before_result": all(row["passed"] for row in invalid_cases),
        "feature_identity": baseline["analysis"].get("capability_id") == FEATURE_ID,
        "method_identity": baseline["analysis"].get("method_version") == METHOD_VERSION,
        "source_stable_during_gate": source_stability["passed"],
    }
    passed = all(checks.values())
    report = {
        "passed": passed,
        "checks": checks,
        "grid_extension": extension_check,
        "worker_invariance": worker_check,
        "invalid_cases": invalid_cases,
        "source_stability": source_stability,
        "runtime_claim_scope": (
            "Product CLI setup rejection, indexed grid-extension invariance, and worker invariance. "
            "Packaged cancellation and responsive GUI behavior are not claimed by this gate."
        ),
    }
    path = write_identity_report(
        "boundary_report",
        passed=passed,
        checks=report,
        execution=executions,
        extras=[SOURCE, *EXECUTION_SOURCES],
    )
    print(json.dumps({"passed": passed, "output": str(path)}, indent=2))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
