# PLS model-fit independent full-refit oracle v1

Status: validation-only bounded oracle. It is not shipped with QuickPLS, is not
an accepted qualification receipt, and does not change capability coverage,
evidence, product surface, or promotion state.

Qualification status: [PLS model-fit v2 qualification status](PLS_MODEL_FIT_QUALIFICATION_STATUS_V2.md).

## Purpose

`validation/pls_model_fit_full_refit_oracle.py` independently checks that the
PLS model-fit workflow is more than a collection of matrix formulas. For each
supported raw-data fixture it:

1. sample-standardizes the indicators;
2. iterates a recursive path-weighted PLS-PM model to convergence;
3. re-estimates outer weights, construct scores, loadings, and structural paths;
4. constructs the observed, saturated-implied, and estimated-implied indicator
   correlation matrices;
5. computes SRMR, d_ULS, natural-log d_G, ML-function Chi-square, degrees of
   freedom, and NFI; and
6. independently performs separate saturated and estimated adapted
   Bollen--Stine null transformations followed by a complete refit for every
   fixed indexed case draw.

The implementation uses NumPy and SciPy dense linear algebra. It imports no
QuickPLS crate, binary, binding, or product function. A frozen product JSON file
is read only after the independent result exists and is treated as a behavioral
comparator, never as the numerical oracle.

## Frozen bounded contract

- Input is a finite raw complete-case matrix with at least three rows.
- The model is recursive and uses unique constructs, indicators, and paths.
- Reflective Mode A and formative Mode B blocks are supported.
- The inner weighting scheme is path weighting.
- Scores, proxies, and block-weighted composites use sample-standardized
  normalization and a deterministic orientation rule.
- Structural paths are fitted with centered least squares and rank checks.
- No ridge repair, nearest-correlation substitution, eigenvalue clamping,
  retry, or replacement of a failed draw is permitted.
- The exact-fit lane uses independent variant-domain seed identities. A NumPy
  `PCG64` stream is created from a SHA-256-derived `SeedSequence` for each tuple
  of master seed, model variant, and replicate index.
- Every requested draw retains its index and SHA-256 sample-index digest.
- HI95 and HI99 are Hyndman--Fan Type-7 quantiles. A criterion needs at least
  `max(2, ceil(0.90 * requested))` usable draws.

The small checked-in work report uses only 12 draws so that it remains fast and
deterministic. The oracle function itself accepts an explicit indexed plan,
which lets qualification tests map identical case identities across a row
permutation without changing the sample contents.

## Current evidence

The focused tests establish:

- point estimates and all three matrices agree with the frozen
  `simple_reflective` product artifact within `1e-10`;
- saturated and estimated null transformations recover their separate target
  correlations;
- saturated and estimated sample streams and discrepancy distributions are
  domain-separated;
- a repeated seed reproduces the complete fixed ledger;
- Type-7 HI95/HI99, tail proportions, usable counts, and decisions recompute
  from the ledger;
- one deliberately singular indexed draw remains one typed failure in each
  ten-cell ledger, leaving exactly nine usable cells;
- point results are invariant to row and consistent indicator permutations;
- exact summaries are invariant when the same sampled case identities are
  mapped across a row permutation; and
- non-positive-definite and singular inputs fail with typed diagnostics.

The generated work artifact is
`validation/results/method_factory/pls_model_fit_v2/work/independent_full_refit_oracle.json`.
It validates against
`validation/pls_model_fit_full_refit_oracle.schema.json` and hard-codes
`qualification_ready=false` and `promotion_requested=false`.

## Unclosed qualification gates

This oracle does not yet cover PLSc refits, higher-order constructs,
interactions, the complete mixed-model shape matrix, or a second independent
full-pipeline implementation. It also does not substitute for pre-registered
Type-I-error, power, coverage, and failure-rate calibration; real
999/5,000/10,000-draw archive and cross-format readback; installed and portable
Windows accessibility/scaling/cancellation runs; maximum-axis, compound-stress,
soak, leak, and performance evidence; or independent scientific review.

Those gates remain blockers. This artifact is early independent-oracle evidence
only and cannot be used to promote the capability.
