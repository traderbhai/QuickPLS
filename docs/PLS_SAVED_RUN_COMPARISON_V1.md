# PLS Saved-Run Comparison V1

Status: Internal/Labs descriptive slice. It is not the analytical
`pls_model_comparison_v1` method and is not promotion evidence for
`qpls3.comparison.pls_models`.

## Official outcome boundary

The SmartPLS model-comparison documentation lists PLSpredict, CVPAT, BIC, and
Akaike weights as model-comparison results:

- <https://www.smartpls.com/documentation/algorithms-and-techniques/model-comparison/>
- <https://smartpls.com/documentation/algorithms-and-techniques/validity-and-model-fit/model-comparison/>

The frozen QuickPLS registry and Wave 0 matrix still classify
`qpls3.comparison.pls_models` as `coverage_state=absent`,
`evidence_state=absent`, `surface=labs`. This slice does not change those
states. The analytical factory contract in
`validation/methods/pls_model_comparison_v1.manifest.json` also says that a
saved-run side-by-side view is descriptive and cannot satisfy the method
identity.

## Implemented contract

`src/domain/plsSavedRunComparisonV1.ts` accepts two valid
`CanonicalResultDocumentV2` documents and blocks unless they bind:

- two distinct scientific model digests;
- the same immutable dataset fingerprint;
- the same analysis method version;
- the same canonical analytical-settings digest; and
- when prediction is present, the same outcome IDs, evaluation cases,
  indicator-average benchmark, folds, repetitions, assignment digest, and
  seed.

The current canonical bridge derives `recipe_digest` from method, method
version, and analytical settings with worker count excluded. Model identity is
bound separately by `model_digest`, so distinct models can legitimately have
the same analytical-settings digest.

Columns are resolved by stable canonical column ID and typed cell, never by
display label. Labels may be edited or localized without changing scientific
alignment. Missing cells stay missing with their recorded reason.

The ready projection can contain:

- deterministic indicator-level PLSpredict rows with second-minus-first
  descriptive changes;
- each model's already stored IA/LM CVPAT benchmark assessment, with an
  explicit notice that this is not a paired CVPAT test between models; and
- exact stored prediction-oriented BIC values, with lower-BIC and exact-tie
  results.

Generic regression or CB-SEM BIC values are never substituted. BIC is never
derived from formatted rows or incomplete residual information.

## Akaike-weight boundary

BIC-only data are not transformed or relabeled as Akaike weights. A weight is
shown only if an exact-attributed canonical
`pls_prediction_information_criteria` row stores all of the following:

- `akaike_weight` between zero and one;
- `akaike_weight_definition=akaike_weight_v1`;
- a SHA-256 `candidate_set_digest`; and
- `candidate_count=2`.

Both model rows must bind the same candidate-set digest and the stored pair
must sum to one within the frozen numerical tolerance. Otherwise weights remain
unavailable with a typed corrective notice.

The exact future table IDs are:

```text
table: pls_prediction_information_criteria
columns:
  outcome: text
  bic: number
  bic_definition: text (prediction_oriented_bic_v1)
  observations: number
  parameter_count: number
  akaike_weight: number or explicit missing
  akaike_weight_definition: text or explicit missing
  candidate_set_digest: text or explicit missing
  candidate_count: number or explicit missing
```

The table must be attributed to either the exact
`qpls3.comparison.pls_models` or
`qpls3.selection.prediction_oriented` Capability Registry V2 identity.

## Product surface and accessibility

`src/native/nativePlsSavedRunComparisonV1.ts` converts completed native runs
through the canonical result bridge. It returns `hidden` before any conversion
when Experimental Labs is disabled.

`src/components/PlsSavedRunComparisonPanelV1.tsx` provides semantic headings,
status and alert regions, captions, row and column headers, text-based status,
and one Experimental chip. It does not rely on colour to communicate a result.
The component is an Internal/Labs seam and is not registered in Standard
Calculate or used for parity claims.

## Persistence boundary

This projection is intentionally in memory. Project schema 6 currently binds
and verifies `CanonicalResultDocumentV2` attachments, but it has no frozen
comparison-definition or comparison-result attachment contract. Persisting the
new object would require a separate schema, digest, migration, tamper, reopen,
and export/readback program. Until that program exists, saved source runs remain
the durable objects and the comparison is rebuilt from them.

## Explicit remaining work

- Execute both candidate models on one immutable common-fold plan and calculate
  a genuine between-model paired CVPAT statistic.
- Implement and independently qualify prediction-oriented BIC production.
- Freeze and qualify the canonical Akaike-weight producer contract.
- Add comparison persistence, semantic exports, CLI parity, installed Windows
  flows, performance, cancellation, and independent scientific review.
- Only after those gates may the registry coverage or evidence state change.
