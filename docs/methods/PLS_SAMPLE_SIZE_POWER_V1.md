# PLS-SEM Sample Size and Power v1

Status: contract-only and `absent` in the QuickPLS 3 method-promotion factory. QuickPLS does not currently implement or expose this calculation. This specification does not authorize a sample-size recommendation until every evidence role in `validation/methods/pls_sample_size_power_v1.manifest.json` passes.

`pls_sample_size_power_v1` is a planned prospective Monte Carlo power analysis for a frozen, bounded family of recursive PLS structural models. It is not the ten-times rule, inverse-square-root heuristic, gamma-exponential shortcut, or a generic guarantee that a sample will be adequate.

## Required design inputs

- A supported typed PLS model and one explicitly selected target path/test.
- Population path values, measurement loadings/weights within the supported family, indicator distributions, construct correlations, reliability assumptions, and the complete missing-data design.
- A finite strictly increasing sample-size grid.
- Significance level, target power, inferential procedure, replicate count, fixed seed, and worker count.
- Any nuisance parameters required to generate every variable in the model.

An omitted assumption is an applicability error. QuickPLS must not silently insert a favorable effect size, reliability, distribution, missingness mechanism, or target test.

## Planned Monte Carlo procedure

For each sample size `n` and replicate `r`:

1. Derive a domain-separated stream from `(master_seed, "pls_sample_size_power_v1", scenario_identity, n, r)`.
2. Generate one complete synthetic dataset from the frozen data-generating design.
3. Estimate the exact supported PLS and inference procedure named by the design.
4. Record convergence, admissibility, the target estimate/test statistic, and whether `p_r <= alpha`.

Failed or inadmissible replicates remain in the denominator under the failure rule frozen before simulation. They may not be silently discarded or regenerated with a different stream.

For `R` planned replicates,

`power_hat(n) = sum_r I(p_r <= alpha) / R`.

The result reports the binomial uncertainty interval for every grid point. The conservative minimum is the first grid value whose preregistered lower confidence bound is at least the requested target power. If no grid point qualifies, the result is "not reached on the evaluated grid," not an extrapolated recommendation.

## Scope and exclusions

The initial implementation is limited to the model, estimator, measurement, distribution, and inference cells that pass the independent simulation gate. Formative, higher-order, interaction, nonlinear, mixture, multigroup, endogeneity, weighted, clustered, ordinal, matrix-only, and unspecified missing-data designs are blocked unless separately added to that evidence envelope.

The result is conditional on the declared design. It does not guarantee convergence on future real data, measurement validity, external validity, adequate subgroup sizes, or sufficient power for an unplanned analysis. Retrospective "observed power" is excluded.

## Determinism, persistence, and product behavior

Scenario and replicate identities, not scheduling order, determine random streams. The same design and seed must produce the same generated-data plan, failures, estimates, power curve, confidence intervals, and selected grid point under every supported worker count.

The typed result stores the full design, grid, target test, stream domain, requested/attempted/successful/failed counts, power rows, uncertainty intervals, minimum-grid decision, warnings, and exact provenance. It stores no generated datasets unless a separately approved validation mode is used. Strict save/reopen validation rejects changed designs, seeds, denominators, power rows, recommendation logic, or identities.

The native workflow must expose every assumption, provide an accessible table equivalent to the power curve, estimate workload before execution, support cancellation/retry, and make "not reached" and failure rates prominent. GUI and CLI must produce identical analytical payloads. Exports must include the complete design and provenance, not only the recommended number.

## Qualification work still required

No contract evidence exists yet. Required work includes independent Python and R generators/calculators, null-calibration and power-recovery simulations, unsupported/failure boundaries, deterministic parallel execution, strict persistence, native setup/results/export, an identity-bound method audit, and installed offline Windows acceptance.

## Scientific source

Muthen and Muthen (2002), *How to Use a Monte Carlo Study to Decide on Sample Size and Determine Power*, https://doi.org/10.1207/S15328007SEM0904_8.
