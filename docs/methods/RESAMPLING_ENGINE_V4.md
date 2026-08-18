# Indexed Resampling Engine Specification v4

Status: release-qualified for the bounded PLS-PM scope registered as `qpls3.inference.bootstrap`. Qualification remains source-bound and must be refreshed after relevant engine, archive, native, export, or packaged-source changes.

V4 inherits indexed bootstrap, Hyndman-Fan Type 7 quantiles for percentile and BCa bounds, normal-reference tests, fixed complete-case sampling, canonical identities, sign alignment, deterministic aggregation, and legacy compatibility from v3. It adds the optional compact `nested_studentized_v1` artifact specified in `STUDENTIZED_BOOTSTRAP_V1.md`.

Every requested primary draw is attempted exactly once. The persisted accounting therefore satisfies `requested = attempted = usable + failed`; there are no retries or replacement draws. New v4 results retain every failed replicate with its zero-based index, a stable typed reason code, and the estimator message. Historical payloads that predate typed reasons remain readable under the explicit `legacy_unclassified_failure` category. Current reason categories are `cancelled`, `insufficient_observations`, `constant_indicator`, `rank_deficient_inner_model`, `isolated_construct`, `non_convergence`, `invalid_indicator`, `score_execution_contract`, `numerical_failure`, and the bounded fallback `estimation_failure`.

The selected immutable run exposes requested, attempted, usable, and failed counts in the native result tree and run provenance. Failed-refit rows expose the typed category and message. BCa parameters that cannot be computed remain visible as unavailable rows with their stored reason; they are not dropped from results or same-run CSV/XLSX exports.

Studentization is disabled when `studentized_inner_samples = 0`. Enabling it requires at least 999 primary replicates and an odd inner count from 99 through 999. Each inner stream is derived independently from the master seed, primary replicate index, and inner replicate index. Inner solutions align to their immediate primary parent; primary solutions remain aligned to the original result.

The persisted studentized table contains no raw primary or inner estimates. A successful artifact records the method version, confidence, inner count, 90% usability policy, stream domain, canonical parameter identity, original estimate, primary-bootstrap standard error, outer-estimate scale for the numerical-zero rule, usable pivot count, pivot quantiles, reversed-tail bounds, and a stable unavailable reason. A nested infrastructure or schema failure instead stores no parameter rows and a compact deterministic failure summary while leaving primary, percentile, and BCa results intact. Current project validation recomputes every available bound, verifies unavailable reasons against their count and scale condition, and rejects malformed failure summaries. V1-v3 artifacts remain readable and cannot carry studentized results.

Nested execution emits `studentized_inner` progress against the requested `B * M` fits and forwards cancellation into every estimator. Enabling studentization does not change the indexed primary samples, percentile summaries, BCa summaries, or permutation streams.

## Row-order metamorphic contract

The empirical nonparametric bootstrap distribution depends on the multiset of complete cases, not their storage order. Exhaustively enumerating all ordered size-`N` draws from a small `N`-case sample must therefore yield the same distribution of resampled case-count vectors after a row permutation and the corresponding inverse relabeling.

A finite indexed run is a Monte Carlo approximation to that distribution. QuickPLS maps a seed-derived integer position to the row occupying that position, while the dataset fingerprint deliberately binds the stored row order. Reordering rows therefore creates a distinct dataset identity and a different, but equally valid, finite coupling of observations to the indexed draws. Exact equality between reordered finite-run interval endpoints is not required and would incorrectly treat the seed as a stable case identifier. Canonically sorting cases would also change every existing v4 seeded result and would require a new resampling method version and provenance contract.

The frozen v4 metamorphic gate instead requires all of the following:

- original estimates and canonical parameter identities are equal after consistent row reordering;
- exhaustive small-sample resampling distributions are exactly equal after case-label relabeling;
- at `B = 9,999`, every mapped percentile and BCa summary difference between original and reordered data is no larger than `1.10` times the maximum corresponding different-seed drift observed on either order, plus a `1e-12` numerical allowance; availability and requested/usable/failure accounting must also agree;
- repeated identical-data, identical-seed runs and supported worker-count changes preserve the analytical bootstrap payload exactly.

The 10 percent comparison margin is a preregistered allowance for estimating a Monte Carlo drift envelope from two frozen independent seeds. It is not a scientific-effect tolerance and cannot excuse a changed estimand, missing parameter, altered failure denominator, or deterministic-repeat mismatch.
