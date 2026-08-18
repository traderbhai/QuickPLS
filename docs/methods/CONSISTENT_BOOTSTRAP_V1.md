# Consistent Bootstrap v1

Status: bounded Standard product contract. The Rust runner, project validator, native recipe, fail-closed result projection, export provenance, and method-scoped Windows acceptance contract recognize `plsc_bootstrap_v1`. A final release build must still refresh the source-bound and packaged receipts after the current source cohort freezes; this status is not a SmartPLS-parity claim.

`plsc_bootstrap_v1` is a nonparametric case bootstrap for the bounded `plsc_v2` estimator. It depends on the separately qualified PLSc point estimator and the deterministic scheduling, canonical identity, cancellation, and aggregation rules of `indexed_resampling_v4`. Ordinary PLS bootstrap evidence cannot qualify this method: every bootstrap and jackknife sample re-estimates the complete PLSc pipeline, including weights, rho_A, attenuation correction, corrected construct correlations, paths, loadings, effects, and R-squared.

## Bounded v1 scope

- The model must satisfy the exact reflective, two-or-more-indicator, path-or-factor weighting, complete-case scope of `plsc_v2`.
- Generated interactions, higher-order constructs, PCA weighting, case weights, multigroup inference, permutation inference, and covariance/correlation-only input are excluded.
- A release run requests 1,000 through 10,000 indexed case-resampling replicates and a fixed seed. Each replicate contains the same number of cases as the frozen complete-case sample and samples with replacement.
- V1 reports original PLSc estimates, bootstrap means and bias, standard errors, two-sided normal-reference diagnostics, percentile intervals, and BCa intervals at the frozen confidence level. Studentized intervals are excluded until separately qualified for PLSc.
- Primary replicate `(b)` uses a domain-separated stream derived from `(master_seed, "plsc_consistent_bootstrap_v1", b)`. Replicate ordering and analytical output are independent of scheduling and worker count.

For canonical parameter `j`, successful replicate estimates are `theta*_bj = PLSc_v2(D*_b)_j`. The bootstrap standard error is the sample standard deviation of successful `theta*_bj` values. Percentile intervals use the frozen Type 7 quantile rule. BCa uses the same mid-rank bias correction and delete-one acceleration rules documented in `RESAMPLING_ENGINE_V3.md`, but every delete-one fit is a full PLSc v2 fit.

## Orientation and failure policy

Each successful resample is aligned to the original solution using the frozen indicator/construct orientation rule before parameter aggregation. Parameter identities must exactly match the original reflective model.

A materially improper rho_A, inadmissible attenuation-corrected correlation, nonconvergence, singular structural equation, identity mismatch, or nonfinite estimate fails that replicate. Failed replicates are never silently replaced or assigned a different stream. Requested, usable, and failed counts, exact replicate indices, sample-index digests, successful-parameter digests, and stable failure reasons are persisted. The bounded v1 contract freezes the minimum usable fraction at 90%. A failed delete-one PLSc fit makes every BCa interval unavailable rather than changing its acceleration sample.

## Persistence and product contract

The current result envelope is `pls_pm_v2` with a typed `plsc_bootstrap_v1` artifact. The archive stores no raw resampled datasets or fitted models. It stores the plan, operation, counts, canonical parameter identities, the original-parameter digest, the complete indexed ledger, replayable successful primary and delete-one parameter witnesses, compact summaries, intervals, unavailable reasons, and exact provenance. Parameter digests use an archive-stable 13-significant-digit scientific encoding, which remains stricter than the PLSc validation tolerance while surviving the typed-to-JSON archive boundary. The validator recomputes each successful digest, percentile and normal-reference summary, and conditional BCa result from those witnesses before append or reopen. Legacy ordinary-PLS bootstrap output must never be reinterpreted as consistent bootstrap evidence.

The native recipe and results code distinguishes this method from ordinary PLS bootstrapping, exposes the bounded settings contract, fails closed on malformed attribution, accounting, or witnesses, shows attempted/usable/failed and delete-one accounting, and exports method-specific provenance through the canonical CSV, XLSX, and HTML result paths. Source checks cover full-refit worker invariance, witness-based interval recomputation, semantic archive tamper rejection, and fail-closed native result/export projection.

## Release evidence refresh

The current source-bound chain uses the independently qualified `plsc_v2` point estimator, an independent bootstrap arithmetic/digest reference, focused full-refit/determinism checks, strict archive/native/export checks, and a separate Windows supervisor. The packaged run creates a real model, blocks an invalid setup, cancels without appending partial state, retries the same bounded replicate plan, verifies requested/attempted/usable/failed and replayable-witness nodes, exports XLSX from the selected run, saves/reopens the same immutable run, resizes the actual Tauri window to 1024×700, 1280×720, and 1440×900, samples the exact process tree for remote egress, and requires graceful PID/CDP shutdown.

The method-level offline gate is functional: the complete analysis/export/save/reopen workflow must succeed without an internet-dependent QuickPLS service, and the application/page evidence must report zero external requests. Exact process-tree TCP samples are a separate disclosure. Microsoft-managed WebView2 background connections are recorded as `platform_background_egress_observed`; when present, `commercial_zero_egress_passed` must remain `false`, but the bounded method qualification does not fail. A literal fully-offline, no-telemetry, or zero-process-egress claim remains subject to the separate OS-enforced commercial containment gate documented in `docs/WEBVIEW2_OFFLINE_BOUNDARY.md`.

After the shared release build is frozen, refresh only this method with:

```powershell
python validation/established_method_factory.py consistent_bootstrap
# Run this only after the shared release desktop and CLI build is complete.
python validation/consistent_bootstrap_v1_packaged_acceptance.py
```

The packaged adapter invokes the method-scoped supervisor; it does not rebuild the desktop or CLI. Its `--skip-run --not-before-utc ... --receipt ...` route may instead consume one explicitly named, append-only supervisor receipt. Large simulation studies also need not be rerun when their scientific inputs and source identity remain unchanged.

The Standard claim remains intentionally scoped. V1 freezes two-sided normal-reference diagnostics plus percentile and conditional BCa intervals; it does not offer selectable test tails, studentized intervals, generated interactions, multigroup/permutation inference, or consistent-bootstrap inference for every measurement-assessment table such as Cronbach alpha and HTMT/HTMT+.

## Scientific sources

- Dijkstra and Henseler (2015), *Consistent and Asymptotically Normal PLS Estimators for Linear Structural Equations*, https://doi.org/10.1016/j.csda.2014.07.008.
- Dijkstra and Henseler (2015), *Consistent Partial Least Squares Path Modeling*, https://doi.org/10.25300/MISQ/2015/39.2.02.
- Efron and Tibshirani (1993), *An Introduction to the Bootstrap*.
