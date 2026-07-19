# PLS Two-Stage Moderation v1

Status: experimental implementation. Publication validation is not complete.

Planned method version: `pls_two_stage_moderation_v1`

## Scope

The current slice freezes the model contract and implements deterministic two-stage product-score estimation for complete-data PLS-PM recipes.

The `ModelSpec.interactions` array stores:

- `id`
- `predictor`
- `moderator`
- `product_construct`
- `outcome`
- `method = two_stage_product_score`

The desktop editor can create an interaction placeholder construct and a product-to-outcome path. The placeholder persists in the workspace and is serialized into native recipes.

## Estimation Contract

For each interaction term:

1. Stage 1 removes generated product constructs and their paths, clears interaction metadata, and estimates the base PLS model.
2. QuickPLS computes a generated product-score indicator as `predictor_score * moderator_score` from the standardized stage-1 construct scores.
3. Stage 2 assigns that generated indicator to the product construct, clears interaction metadata to prevent recursion, and runs the ordinary PLS estimator with the product-to-outcome path.
4. The resulting product-construct structural path is the interaction coefficient.
5. QuickPLS reports simple slopes for the predictor effect at standardized moderator scores `-1`, `0`, and `+1` as `predictor_main_effect + interaction_effect * moderator_score`.

For missing data, stage 1 uses the ordinary model-wide complete-case rows. Stage 2 is then built from the same row set, so generated product scores remain aligned to the original complete-case observations and final used/omitted observation counts refer to the source dataset.

## Current Evidence

`npm run qpls:moderation:reference` writes `validation/results/moderation_reference_report.json`. The script computes the single-item two-stage contract independently with standardized multiple regression, then checks QuickPLS path and simple-slope output against that reference. It also verifies positive-affine invariance, row-order invariance, construct-order invariance, complete-case missing-data row mapping, experimental warning persistence, and interaction-signal degradation after moderator permutation.

`npm run qpls:moderation:r-reference` writes `validation/results/moderation_r_reference_report.json`. The script runs development-only R base `lm` on the same single-item fixture, using standardized `x`, `m`, `y`, and the standardized product of `x*m`, then compares paths and simple slopes with QuickPLS.

`npm run qpls:moderation:published-formula` writes `validation/results/moderation_published_formula_report.json`. The script freezes the standard moderated-regression equation and simple-slope interpretation on a fixed auditable table, then checks QuickPLS paths and simple slopes against independent standardized OLS equations. This is published-formula evidence, not a published empirical-data replication.

`npm run qpls:moderation:published-empirical` writes `validation/results/moderation_published_empirical_report.json`. The script uses the 32-row `mtcars` empirical dataset from the R datasets package, originally extracted from 1974 Motor Trend US magazine, and verifies the single-item moderation model `mpg ~ wt + hp + wt*hp` against independent standardized OLS paths and simple slopes.

`npm run qpls:moderation:simulation` writes `validation/results/moderation_simulation_report.json`. The script runs bounded signal and null interaction datasets, compares every QuickPLS interaction coefficient against independent standardized OLS, verifies positive interaction recovery under generated signal, and checks that null interaction estimates remain small. This is deterministic recovery evidence, not a full bootstrap/permutation coverage qualification.

`npm run qpls:moderation:inference` writes `validation/results/moderation_inference_report.json`. The script verifies that the generated product path is included in percentile bootstrap, BCa, and Freedman-Lane permutation output, that the bootstrap original equals the typed moderation interaction effect, and that the bounded analytical payload is invariant across one and two workers.

`npm run qpls:moderation:inference-qualification` writes `validation/results/moderation_inference_qualification_report.json`. The script runs the actual Freedman-Lane product-path permutation pipeline over six generated signal and six generated null moderation datasets, checks QuickPLS interaction coefficients against independent standardized OLS, and verifies bounded signal detection and false-positive containment. This is an always-on qualification screen, not the final long-running Monte Carlo coverage study.

`npm run qpls:moderation:coverage-qualification` writes `validation/results/moderation_coverage_qualification_report.json`. The script runs a heavier deterministic release-oriented coverage screen with 24 generated signal datasets, 24 generated null datasets, `n = 220`, 199 Freedman-Lane permutations per dataset, and four workers. The current report detected 24/24 signal datasets, flagged 1/24 null datasets at `p <= 0.05`, matched independent standardized OLS interaction coefficients within `1e-10`, and completed in 22.43 seconds on the local workstation. This is bounded development evidence, not final publication-scale Monte Carlo validation.

## Gate Behavior

`qpls-core` validates interaction metadata and returns warnings with codes `method.moderation.experimental` and, when applicable, `interaction.product_indicator.generated`. Generated product constructs may start without dataset indicators because the estimator supplies the product-score indicator between stage 1 and stage 2.

The validation gate remains open while the broader v0.5 extended-method family is incomplete. Two-stage moderation itself remains experimental until the release-family audit decides whether the bounded qualification evidence is sufficient for the preview label.

## Compatibility

Older recipes without `interactions` deserialize with an empty interaction list. This preserves v0.1-v0.4 project compatibility.
