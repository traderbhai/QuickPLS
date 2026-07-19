# Indexed Resampling Engine Specification v4

Status: implemented experimental inference; publication-ready promotion still requires the frozen simulation, independent-reference, and performance gates.

V4 inherits indexed bootstrap, percentile, normal-reference, BCa, fixed complete-case sampling, canonical identities, sign alignment, deterministic aggregation, and legacy compatibility from v3. It adds the optional compact `nested_studentized_v1` artifact specified in `STUDENTIZED_BOOTSTRAP_V1.md`.

Studentization is disabled when `studentized_inner_samples = 0`. Enabling it requires at least 999 primary replicates and an odd inner count from 99 through 999. Each inner stream is derived independently from the master seed, primary replicate index, and inner replicate index. Inner solutions align to their immediate primary parent; primary solutions remain aligned to the original result.

The persisted studentized table contains no raw primary or inner estimates. A successful artifact records the method version, confidence, inner count, 90% usability policy, stream domain, canonical parameter identity, original estimate, primary-bootstrap standard error, outer-estimate scale for the numerical-zero rule, usable pivot count, pivot quantiles, reversed-tail bounds, and a stable unavailable reason. A nested infrastructure or schema failure instead stores no parameter rows and a compact deterministic failure summary while leaving primary, percentile, and BCa results intact. Current project validation recomputes every available bound, verifies unavailable reasons against their count and scale condition, and rejects malformed failure summaries. V1-v3 artifacts remain readable and cannot carry studentized results.

Nested execution emits `studentized_inner` progress against the requested `B * M` fits and forwards cancellation into every estimator. Enabling studentization does not change the indexed primary samples, percentile summaries, BCa summaries, or permutation streams.
