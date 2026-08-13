# Consistent Multigroup Analysis v1

Status: contract-only and `absent` in the QuickPLS 3 method-promotion factory. The current ordinary-PLS MGA and permutation-MGA results do not qualify PLSc multigroup inference.

`plsc_mga_v1` is the planned two-group bootstrap multigroup analysis for attenuation-corrected PLSc parameters. It depends on release-qualified `plsc_v2` and `plsc_bootstrap_v1`. Interpretation also depends on the paired `plsc_permutation_v1` measurement-invariance decision; a bootstrap difference must not be presented as substantively comparable when partial invariance is absent.

## Bounded v1 scope

- Exactly two selected observed groups, each satisfying the complete reflective `plsc_v2` scope and the minimum sample rules frozen by the independent simulation gate.
- Identical model, indicator ordering, preprocessing, missing-data policy, estimator settings, confidence level, and canonical orientation across groups.
- Each group receives its own indexed nonparametric case bootstrap with the same requested replicate count but domain-separated streams.
- V1 reports original `A - B` differences, the empirical PLS-MGA tail probability, group-specific percentile/BCa intervals, and interval-overlap diagnostics for corrected paths and outer loadings.
- Parametric pooled-variance tests, Welch-Satterthwaite tests, more than two groups, one-tailed claims, interactions, higher-order constructs, weights, and automatic multiplicity claims are excluded.

Let `theta*_(A,b)` and `theta*_(B,b)` be independently resampled, canonically aligned PLSc v2 estimates. With `d*_bj = theta*_(A,b),j - theta*_(B,b),j`, QuickPLS reports the frozen directional empirical tail probability and an explicitly two-sided decision rule. The exact tie handling, finite-sample correction, and decision transformation must be fixed in the independent reference before implementation; they cannot be inferred from or copied out of another product. Confidence-interval overlap is descriptive and is never substituted for the primary frozen MGA decision.

## Measurement invariance and failures

The result joins each parameter to the applicable consistent-permutation invariance status. Structural paths involving a construct without partial invariance remain visible for diagnosis but are marked not interpretable for group-comparison claims. The application must not convert missing invariance evidence into a pass.

Each group bootstrap follows the `plsc_bootstrap_v1` failure policy. Failed PLSc fits remain in requested/attempted/failed accounting and are not silently replaced. If either group misses the preregistered usable threshold, no MGA probability or interval-overlap decision is published for that run.

## Persistence and product contract

The target `pls_pm_v3` envelope contains a typed `plsc_mga_v1` result bound to the two group-specific `plsc_bootstrap_v1` summaries and exact `plsc_permutation_v1` invariance identity. It stores no raw bootstrap datasets. Strict validation rejects mismatched group labels, samples, method versions, parameter identities, replicate plans, invariance results, or arithmetic.

The native workflow must lead with group and invariance status, then show parameter differences, group estimates, tail probabilities, confidence intervals, warnings, and provenance. GUI and CLI analytical payloads and exports must agree for identical inputs. Packaged acceptance must exercise invalid setup, long-run cancellation and retry, export, save/reopen, offline behavior, and cleanup.

## Qualification work still required

The manifest intentionally contains no promotion evidence. Release qualification requires an independently reviewed tail-probability convention, independent code and fixtures, null/type-I-error and power simulations, imbalance and failure boundaries, strict persistence, native and export tests, an identity-bound audit, and installed Windows acceptance.

## Scientific sources

- Dijkstra and Henseler (2015), *Consistent Partial Least Squares Path Modeling*, https://doi.org/10.25300/MISQ/2015/39.2.02.
- Sarstedt, Henseler, and Ringle (2011), *Multigroup Analysis in Partial Least Squares Path Modeling: Alternative Methods and Empirical Results*.
- Henseler, Ringle, and Sarstedt (2016), *Testing Measurement Invariance of Composites Using Partial Least Squares*, https://doi.org/10.1108/IMR-09-2014-0304.
