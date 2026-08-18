# PLS-SEM Sample Size and Power v1

Status: bounded candidate source and qualification gates are implemented, but the method is unqualified and unavailable for product claims. The current transactional qualification failed its preregistered null-calibration gate, so the manifest remains `absent`. Source availability, signal-recovery results, or a smoke run never authorizes a sample-size claim.

`pls_sample_size_power_v1` is prospective Monte Carlo power analysis for exactly one bounded design: two ordinary reflective constructs joined by one predictor-to-outcome path. It is not the ten-times rule, inverse-square-root heuristic, gamma-exponential shortcut, retrospective observed power, or a generic guarantee that a sample will be adequate.

## Required design inputs

- Exactly one predictor construct and one outcome construct selected from an actual eligible one-path model. Each block is ordinary reflective and has 3-10 indicators.
- One population path in `[-0.80, 0.80]` and one ordered loading per displayed indicator in `[0.50, 0.95]`.
- Fixed standard-normal exogenous latent, structural disturbance, and independent indicator-error distributions. The outcome disturbance scale is `sqrt(1 - beta^2)`; generated data contain no missing values.
- A finite strictly increasing 2-16 point sample-size grid with every value from 30 to 5,000.
- Two-sided alpha, target power, Wilson confidence level, Monte Carlo replicate count, odd case-bootstrap replicate count, master seed, and worker count.
- Fixed path weighting, standardized preprocessing, convergence tolerance, and maximum PLS iterations.

The typed config fields are `scenario_identity`, predictor/outcome construct identities, predictor/outcome loading vectors, `population_path`, three fixed `standard_normal` distribution sentinels, fixed `missing_data = none`, fixed `inference = case_bootstrap_normal_reference_two_sided`, `sample_size_grid`, `alpha`, `target_power`, `confidence_level`, `monte_carlo_replicates`, `bootstrap_replicates`, `master_seed`, `workers`, and the fixed/bounded estimator settings. An omitted or unsupported assumption is an applicability error.

## Planned Monte Carlo procedure

For each sample size `n` and replicate `r`:

1. Derive an independent domain-separated stream from `(master_seed, "pls_sample_size_power_v1", scenario_identity, n, r, subdomain)`. Candidate sample sizes do not share common-random-number datasets.
2. Generate one complete synthetic dataset from the frozen data-generating design.
3. Fit production path-weighted standardized PLS-PM with no outer resampling, then execute the declared case-bootstrap normal-reference path through the production resampling engine to calculate the target path's bootstrap standard error and two-sided p value. This narrow inference path does not calculate unused BCa/delete-one jackknife or studentized intervals.
4. Record convergence, admissibility, the target estimate/test statistic, and whether `p_r <= alpha`.

Failed or inadmissible replicates remain in the denominator under the failure rule frozen before simulation. They may not be silently discarded or regenerated with a different stream.

For `R` planned replicates,

`power_hat(n) = sum_r I(p_r <= alpha) / R`.

The result reports a two-sided Wilson binomial interval for every grid point. The conservative minimum is the first evaluated grid value whose Wilson lower bound is at least the requested target power. Rows are never smoothed. Any decrease at adjacent grid points is reported prominently and does not change the rule. If no grid point qualifies, the result is "not reached on the evaluated grid," never an interpolated or extrapolated recommendation.

The pre-run workload is exact and conservative for this kernel: `datasets = grid_points * R`, `PLS fits = datasets * (1 + B)`, and `row-fits = sum(n_grid) * R * (1 + B)`. Setup rejects more than 250,000 PLS fits or 100,000,000 row-fits. The shipped defaults `n = 50,100,150`, `R = 250`, and `B = 199` plan 750 independent datasets, 150,000 PLS fits, and 15,000,000 row-fits.

## Scope and exclusions

V1 is limited to the exact two-construct reflective standard-Gaussian single-path/no-missing design above. Formative, higher-order, interaction, nonlinear, mixture, multigroup, endogeneity, weighted, clustered, ordinal, matrix-only, and every missing-data design are blocked. Broader recursive models, alternate distributions, correlated errors, controls, multiple predictors, alternate inferential tests, and sensitivity grids are not implied by this contract.

The result is conditional on the declared design. It does not guarantee convergence on future real data, measurement validity, external validity, adequate subgroup sizes, or sufficient power for an unplanned analysis. Retrospective "observed power" is excluded.

## Determinism, persistence, and product behavior

Scenario and replicate identities, not scheduling order, determine random streams. `stream_identity` identifies the indexed random-number stream for one `(n, r)` pair; it is not a generated-dataset byte digest. The normalized scientific `recipe_digest` binds the path, loadings, grid, estimator, distributions, replicate counts, and seed while treating worker count as execution-only. The full sealed archive and outer project provenance still bind the requested worker count. The same scientific recipe and seed produce exactly equal ordered outcomes and analytical results under every supported worker count.

The typed result stores schema/capability/method identities; normalized recipe, ordered-outcome, PLS-method, and resampling-method identities; stream, inference, interval, and failure-policy identities; exact dataset/fit/row-fit workload; one row per grid point with requested/attempted/successful/failed/rejection counts, power, Wilson bounds, and qualification; the ordered replicate ledger with stream identity, estimate, p value, rejection, and named failure; first-qualified/not-reached decision; nonmonotonic count; deterministic warnings; and deterministic exclusions. It stores no generated datasets.

Archive validation proves structural, accounting, identity, and deterministic-summary integrity from the stored replicate ledger and rejects isolated mutations. It checks the typed recipe/result contract, indexed stream identities, ledger accounting, the digest of the stored ordered ledger, and deterministic recomputation of power rows, Wilson intervals, warnings, exclusions, and the grid decision from that ledger. The persistence gate also exercises isolated identity, recipe/provenance, decision, malformed-payload, duplicate-key, and checksum mutations.

It does not replay generated data, PLS estimates, or bootstrap fits and does not authenticate a coordinated rewrite of target estimates, p values, rejection flags, outcome digest, derived power rows, grid decision, and outer archive checksum. Such a coordinated rewrite can remain internally self-consistent. Scientific validity comes from the separate identity-bound engine, independent-reference, and simulation evidence; archive qualification claims structural persistence integrity only. Native qualification may compose those separate evidence roles, but neither archive nor native qualification may be described as semantic replay, cryptographic authenticity, or tamper-proof provenance.

The native workflow must expose every assumption, provide an accessible table equivalent to the power curve, estimate workload before execution, support cancellation/retry, and make "not reached" and failure rates prominent. GUI and CLI must produce identical analytical payloads. Exports must include the complete design and provenance, not only the recommended number.

## Qualification result and redesign blocker

The source, independent reference, native workflow, and focused gates exist. The current exact-source qualification completed 2,500 null datasets with zero fit failures, but rejected 190 nulls: `7.60%`, with a pooled 95% Wilson interval of `[6.625%, 8.705%]`. Both preregistered sample-size points were anti-conservative (`7.44%` at `n=160`; `7.76%` at `n=320`), so the interval excludes the nominal `5%` level and the simulation identity correctly failed. The separate signal run completed and recovered a monotone curve, but signal behavior cannot override failed null calibration.

This is not repaired by changing seeds, loosening the acceptance bound, increasing only the 99 bootstrap draws, substituting a `t(98)` cutoff, or lowering the nominal alpha after observing the result. At the exact null, path-weighted PLS adaptively selects block directions from sample noise; a conditional case-bootstrap normal-reference statistic is therefore non-regular and anti-conservative for this design. A future method version requires an independently preregistered, DGP-matched null calibration (or another separately justified inference design), full PLS refits, held-out alternative simulations, and new evidence. Until that redesign qualifies, the manifest stays `absent` and packaged acceptance is inapplicable.

## Scientific source

Muthen and Muthen (2002), *How to Use a Monte Carlo Study to Decide on Sample Size and Determine Power*, https://doi.org/10.1207/S15328007SEM0904_8.
