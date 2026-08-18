# PLS model comparison v1 — Internal/Labs contract

## Status

`pls_model_comparison_v1` is an executable scientific foundation for genuine
two-model PLS comparison. It is **Internal/Labs**, is not exposed in Standard
Calculate, and is not release-qualified. Its presence does not change any
Capability Registry V2 coverage or evidence state.

This method is distinct from `pls_saved_run_comparison_v1`. The saved-run
comparison is descriptive report inspection; this method re-estimates both
models on exact shared training partitions and produces new analytical results.

## Frozen parity source snapshot (15 August 2026)

The current official SmartPLS [Model Comparison](https://www.smartpls.com/documentation/algorithms-and-techniques/model-comparison/)
page defines a two-model project workflow and lists four result families:

1. PLSpredict;
2. CVPAT;
3. prediction-oriented BIC; and
4. Akaike weights.

The official [Prediction-oriented Model Selection](https://www.smartpls.com/documentation/algorithms-and-techniques/prediction-oriented-model-selection/)
page says that SmartPLS provides BIC, explicitly excludes GM, and says the
lowest BIC is preferred. That page also contains an older statement that
CVPAT-based *model comparison* has not been implemented. This conflicts with
the newer active Model Comparison page, which explicitly lists model-pair
CVPAT. Contract v1 resolves the delta narrowly:

- the active Model Comparison page governs the two-model workflow;
- the standalone selection page governs the BIC-only criterion and GM
  exclusion; and
- neither page authorizes substituting a generic likelihood BIC, AIC, GM, or a
  descriptive comparison of saved reports.

The calculation details are frozen from the cited primary methods:

- Liengaard et al. (2021), *Prediction: Coveted, Yet Forsaken? Introducing a
  Cross-Validated Predictive Ability Test in Partial Least Squares Path
  Modeling*, especially equations 1–3 for paired case loss and its t statistic.
- Sharma et al. (2019), *PLS-Based Model Selection: The Role of Alternative
  Explanations in Information Systems Research*, for equation-level
  prediction-oriented BIC.
- Danks, Sharma, and Sarstedt (2020), *Model Selection Uncertainty and
  Multimodel Inference in PLS-SEM*, for BIC-derived Akaike weights.
- Shmueli et al. (2016, 2019) for PLSpredict's training/holdout principle.

## Exact v1 input contract

The engine accepts:

- one raw `Dataset`;
- one established-model `AnalysisRecipe`;
- one alternative-model `AnalysisRecipe`; and
- explicit folds, repetitions, seed, and confidence level.

Both recipes must:

- bind to the exact dataset fingerprint;
- be schema-v3 `MethodConfig::PlsAlgorithm` point-estimate recipes;
- use identical weighting, preprocessing, tolerance, and iteration settings;
- use listwise deletion without case weights;
- be scientifically distinct after names, UUIDs, and declaration order are
  removed from identity; and
- expose the exact same nonempty reflective endogenous construct IDs and
  indicator sets.

Contract v1 rejects observed controls, higher-order constructs, interactions,
formative endogenous targets, matrix inputs, weights, and unequal target sets.
These are typed bounded-scope failures, not silent omissions.

## Shared-fold PLSpredict calculation

The official defaults are 10 folds and 10 repetitions. A smaller explicit plan
is permitted for deterministic microcases and Internal development tests.

1. Find complete cases across the **union** of indicators used by both models.
2. For each repetition, rank source-row IDs with SHA-256 over the contract
   version, seed, repetition, and source row.
3. Allocate the ranked rows round-robin to folds.
4. Persist the complete assignment ledger and its digest.
5. For every fold, fit both PLS models on the exact same training rows.
6. Apply training transforms, outer weights, structural coefficients, and
   indicator regressions to the exact same holdout rows.
7. Aggregate indicator RMSE, MAE, SSE, and the shared indicator-average
   benchmark. Calculate Q-squared-predict only when the benchmark SSE is
   strictly positive.

No fitted value, fold, target, or missing-data decision is borrowed from a saved
run. A cancellation callback is passed through to every in-flight PLS fit.

## Paired model CVPAT

For each complete case, loss is the mean squared prediction error across all
common endogenous indicators, averaged over repetitions. Let
`L_i,1` be the established-model case loss and `L_i,2` the alternative-model
case loss. Contract v1 follows Liengaard et al. exactly:

```text
D_i      = L_i,2 - L_i,1
D_bar    = sum(D_i) / N
S^2      = sum((D_i - D_bar)^2) / (N - 1)
T        = D_bar / sqrt(S^2 / N)
df       = N - 1
```

The directional test is lower-tailed: the predeclared alternative is that the
alternative model has lower loss. A two-sided p value and two-sided confidence
interval are also reported. Zero variance produces an explicit unavailable
status with no NaN or infinity.

## Prediction-oriented BIC and Akaike weights

For every common endogenous structural equation, and only for that equation:

```text
BIC = N * ln(SSE / N) + p * ln(N)
```

`SSE` is the full complete-case structural-score residual sum of squares and
`p` is the number of incoming predictor constructs plus one, as specified for
the regression equation. Perfect/nonpositive SSE, nonfinite values, and invalid
sample or parameter counts fail with a typed error; the engine never applies an
arbitrary epsilon floor.

For the two candidates:

```text
delta_i  = BIC_i - min(BIC_1, BIC_2)
weight_i = exp(-0.5 * delta_i) / sum_j exp(-0.5 * delta_j)
```

The engine does **not** calculate or infer whole-model likelihood BIC, AIC,
AICc, HQ, GM, or a combined BIC across unrelated target equations.

## Result contract and failures

The result records method versions, Internal/Labs surface, `qualified=false`,
dataset fingerprint, both scientific model digests, full shared-fold ledger,
PLSpredict indicator results, paired CVPAT, equation-level BIC, two-candidate
Akaike weights, and non-promotional warnings.

Typed failures cover dataset/recipe mismatch, same-model comparison,
incompatible settings or targets, unsupported model features, invalid folds,
insufficient complete cases, Arrow subsetting, PLS fit failure, prediction
contract drift, invalid BIC inputs, invalid weights, invalid CVPAT inputs, and
cancellation.

## Internal/Labs execution and persistence seam

`InternalLabsPlsModelComparisonRequestV1` is the only desktop execution
request for this method. It is strict and source-bound: it carries both full
schema-v3 recipes, independently rechecked recipe SHA-256 values, the exact
resident dataset ID and fingerprint, the exact capability cell
`smartpls.pls_model_comparison / qpls3.comparison.pls_models`, and one shared
fold-assignment version, seed, fold count, repeat count, and confidence level.
The request must remain `surface=internal_labs`, Experimental Labs enabled,
and `qualification_ready=false`. Both recipes must record `workers=1`; the v1
engine is serial and never suggests worker invariance that it has not proven.

The desktop runs this request through an isolated job service with typed
status, progress, cancellation, failure, and result commands. Its result map is
not the Standard result map and cannot accept a saved-run comparison payload.
Admission is nevertheless reserved in the existing shared Standard/Recipe-v4
pool, so the global four-active-job and CPU-worker limits include PLS model
comparison. An RAII reservation is released after success, cancellation,
caught worker panic, or thread-spawn failure. A completed result is published
only after the active project and resident dataset identity are rechecked and
after the canonical document passes validation; cancellation never publishes
or retains a partial result.

`CanonicalResultDocumentV2` is built from the new analytical result and has
exact option-cell attribution at document, section, and table levels. It
contains:

- immutable request, recipe, model, dataset, method, and shared-fold digests;
- indicator-level PLSpredict accuracy for both candidates and the
  indicator-average benchmark;
- exact held-out source-row identities and paired fold losses;
- paired case losses and the complete CVPAT row;
- the documented equation-level prediction-oriented BIC variants, deltas, and
  two-candidate weights; and
- an explicit Internal/Labs, not-qualified notice and generic-information-
  criterion exclusion.

The existing schema-6 atomic canonical append and strict reopen APIs consume
this document without changing the live schema-5 writer. A focused integration
test writes a schema-6 project copy, appends the comparison document using the
exact inspected source hash, reopens it, and compares the complete canonical
payload and capability attribution. The analytical run result and archive
attachment are immutable values; there is no implicit recalculation on reopen.

## Verification

- Rust microcases independently check BIC, weights, paired-loss direction,
  zero-variance behavior, fold balance, exact repeatability, typed
  cancellation, and same-model rejection.
- A product test fits two real PLS models on every shared fold and verifies that
  the richer alternative predicts the common outcome with lower loss.
- Runner and desktop-job tests verify strict access/source hashes, exact
  canonical tables, authoritative cancellation, no partial result, shared job
  admission, and reservation release after success, pre-execution
  cancellation, and injected thread-spawn failure.
- A schema-6 integration test verifies atomic append and exact canonical
  reopen while the schema-5 source project remains untouched.
- `validation/pls_model_comparison_v1_oracle.py` is a transparent Python
  micro-oracle for the formulas and assignment digest. It neither imports nor
  calls QuickPLS.

Remaining before any qualification claim: additional independent maintained
computational references, published-data fixtures, generative recovery,
metamorphic/adversarial campaigns, the full historical/future/tamper archive
corpus, all-format export readback, a public CLI command, GUI setup/results and
saved-run selection, packaged Windows flows, accessibility,
performance/cancellation-latency/soak evidence, and independent scientific
review. None of the targeted execution or schema-6 tests is an accepted
qualification receipt or a Registry/evidence promotion.
