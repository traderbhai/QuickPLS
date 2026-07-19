"""Indirect-effect randomization screen for QuickPLS mediation.

This bounded validation check is intentionally separate from the direct-path
Freedman-Lane permutation engine. It verifies that the experimental
`pls_mediation_v1` indirect effect behaves as expected when the mediator link is
randomized:

- QuickPLS agrees with independent standardized OLS equations for the observed
  generated mediation signal.
- A deterministic distribution of mediator permutations gives a small plus-one
  randomization p value for the observed indirect effect.
- QuickPLS agrees with the independent equations on a permuted-mediator dataset
  and the indirect effect materially drops.
"""

import json
import random

from mediation_metamorphic import (
    RESULTS,
    generated_rows,
    independent_effects,
    run_quickpls,
)


OUTPUT = RESULTS / "mediation_randomization_report.json"
PERMUTATIONS = 199
SEED = 20260719
TOLERANCE = 1e-10


def permute_mediator(rows, seed):
    rng = random.Random(seed)
    mediator = [row["m"] for row in rows]
    rng.shuffle(mediator)
    return [
        {"x": row["x"], "m": mediator[index], "y": row["y"]}
        for index, row in enumerate(rows)
    ]


def indirect_xy(rows):
    return independent_effects(rows)[("x", "y")]["indirect"]


def plus_one_p_value(observed, randomized):
    exceedances = sum(1 for value in randomized if abs(value) >= abs(observed))
    return (exceedances + 1.0) / (len(randomized) + 1.0), exceedances


def main():
    rows = generated_rows(seed=SEED, n=120)
    quickpls = run_quickpls("mediation_randomization_observed", rows)
    independent = independent_effects(rows)
    observed_indirect = quickpls[("x", "y")]["indirect"]
    independent_observed = independent[("x", "y")]["indirect"]
    observed_delta = abs(observed_indirect - independent_observed)

    randomized_indirects = [
        indirect_xy(permute_mediator(rows, SEED + index + 1))
        for index in range(PERMUTATIONS)
    ]
    p_value, exceedances = plus_one_p_value(observed_indirect, randomized_indirects)

    permuted_rows = permute_mediator(rows, SEED + 10_000)
    quickpls_permuted = run_quickpls("mediation_randomization_permuted", permuted_rows)
    independent_permuted = independent_effects(permuted_rows)[("x", "y")]["indirect"]
    quickpls_permuted_indirect = quickpls_permuted[("x", "y")]["indirect"]
    permuted_delta = abs(quickpls_permuted_indirect - independent_permuted)
    drop_ratio = abs(quickpls_permuted_indirect) / abs(observed_indirect)

    randomized_abs = [abs(value) for value in randomized_indirects]
    checks = {
        "observed_indirect": observed_indirect,
        "independent_observed_indirect": independent_observed,
        "observed_independent_delta": observed_delta,
        "randomization_permutations": PERMUTATIONS,
        "randomization_exceedances": exceedances,
        "randomization_p_value_two_sided": p_value,
        "randomized_abs_median": sorted(randomized_abs)[len(randomized_abs) // 2],
        "randomized_abs_max": max(randomized_abs),
        "quickpls_permuted_indirect": quickpls_permuted_indirect,
        "independent_permuted_indirect": independent_permuted,
        "permuted_independent_delta": permuted_delta,
        "permuted_drop_ratio": drop_ratio,
    }
    passed = (
        checks["observed_independent_delta"] <= TOLERANCE
        and checks["permuted_independent_delta"] <= TOLERANCE
        and abs(checks["observed_indirect"]) >= 0.35
        and checks["randomization_p_value_two_sided"] <= 0.05
        and checks["permuted_drop_ratio"] <= 0.5
    )
    report = {
        "schema_version": 1,
        "kind": "pls_mediation_indirect_randomization_v1",
        "passed": passed,
        "seed": SEED,
        "tolerance": TOLERANCE,
        "checks": checks,
        "note": "Bounded independent randomization screen for the indirect effect; not a full formal indirect-effect permutation estimator.",
    }
    OUTPUT.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(
        f"wrote {OUTPUT} | passed={passed} | "
        f"p={p_value:.3g} | drop_ratio={drop_ratio:.3g}"
    )
    if not passed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
