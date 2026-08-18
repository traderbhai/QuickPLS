# CB-SEM Bootstrap v2

Status: historical, readable, and non-promotional. Schema-3
`cbsem_bootstrap_v2` remains available only for archive compatibility and is
not emitted by current desktop bootstrap controls. Current exact CFA case
bootstrap uses the Recipe-v4 identities documented in
`CBSEM_EXACT_CASE_BOOTSTRAP_V1.md`; neither identity is relabeled as the other.

## Supported method

`cbsem_bootstrap_v2` is a seeded nonparametric case bootstrap for models already accepted by `cbsem_ml_v1`: raw-data, single-group, listwise-complete, continuous, reflective CFA or recursive ML SEM. Each replicate samples `n` rows with replacement, recomputes the sample moments, and refits the complete frozen model with the same identification and optimizer contract.

For parameter `theta`, the bootstrap standard error is the sample standard deviation of the `B_ok` usable preplanned primary-draw estimates and the frozen two-sided 95% percentile interval is `[Q_0.025(theta*), Q_0.975(theta*)]`, using the Type-7 linear quantile rule. Custom confidence levels are rejected under the v2 identity. When any fits fail, this is explicitly the empirical distribution conditional on an admissible ML refit, with the failure rate reported beside it; it is not labeled an unconditional `B`-draw distribution. Allowed requested replicate counts are 1,000 through 10,000. At least `max(1000, ceil(0.90 B))` replicates must converge to admissible results. A scientifically completed run below this threshold is retained with `inference.status = unavailable`, an empty interval array, the exact successful-fit witness, and the complete failed-replicate ledger; invalid setup, cancellation, or internal integrity loss remain execution errors.

This contract uses Efron (1979), *Bootstrap Methods: Another Look at the Jackknife*, DOI `10.1214/aos/1176344552`, for case resampling and Bollen and Stine (1992), *Bootstrapping Goodness-of-Fit Measures in Structural Equation Models*, DOI `10.1177/0049124192021002004`, for the distinct null-transformed goodness-of-fit bootstrap. Ordinary case-bootstrap parameter intervals must never be labeled a Bollen-Stine model-fit p-value.

## Reproducibility and failures

The persisted recipe includes seed, replicate count, fixed confidence level, quantile rule, worker-independent replicate stream, optimizer settings, and failure policy. Exactly `B` primary draws are preplanned; there is no retry or replacement draw after a failed ML fit. The analytical result and validation witness are exactly worker invariant; worker count is retained only in immutable outer run provenance/settings and is not an analytical field. The witness binds the dataset fingerprint, a canonical scientific recipe digest with the operational worker count normalized, an archive-stable base-result digest whose numeric canonicalization is stricter than the project validation tolerance, accepted parameter order, sampling-frame-position digest, and stored optimizer result. The enclosing v2 result stores every failed fit in a separate reason-coded ledger with that failed draw's sampling-position digest. Archive validation separately binds the complete unnormalized persisted recipe and outer provenance, so an uncoordinated saved-worker mutation remains detectable even though valid 1-worker and 4-worker runs have identical analytical payloads. Every runtime nonconverged, inadmissible, singular, or non-finite fit is counted and reason-coded. Cancellation produces no completed bootstrap result and restarting the run with the same seed must reproduce the uninterrupted analytical payload.

The current archive validator provides structural, accounting, identity, and deterministic-summary integrity. It reconstructs the preplanned sampling-position hashes and recomputes standard errors and Type-7 intervals from the stored successful-replicate estimates, but it does not rerun the CB-SEM ML optimizer for every archived sample. Consequently, archive checksums are not semantic replay authentication: a coordinated rewrite of a successful estimate together with its derived summaries, intervals, and outer checksum, or substitution of another structurally allowed failure reason/message, can remain structurally valid. This is an explicit boundary of unsigned local evidence, not a claim that structural validation is cryptographic authentication. The historical v2 identity remains non-promotional regardless; current exact-family qualification claims structural/accounting/checksum integrity and explicitly excludes malicious coordinated semantic rewrite and trusted-origin authentication.

The resampling stream is positional within the dataset fingerprint. Reordering
rows produces a different dataset fingerprint and therefore a distinct run; no
claim of byte-identical mapped resamples across separately imported row orders
is made. A row-order metamorphic qualification instead compares estimator and
interval behavior inside preregistered Monte Carlo tolerances.

## Current preview is inadmissible

The existing `cbsem_bootstrap_v1` engine preview constructs `estimate +/- 1.96 * analytical_standard_error` without resampling or refitting. It is a normal-theory interval preview, not a bootstrap, and is permanently barred from satisfying v2 evidence or being migrated/relabelled as v2.

## Exclusions

Ordinal/WLSMV or robust estimators, missing-data FIML, multilevel or clustered resampling, parametric bootstrap, BCa/studentized intervals, indirect-effect-specific products, and automatic model modification are excluded from this standalone v2 capability. A downstream MGA or moderator workflow may reuse the frozen case-resampling, seed, quantile, and failure protocol only when its group-stratified or LMS refit extension is independently qualified under that downstream method identity; such results are not standalone `cbsem_bootstrap_v2` evidence.
