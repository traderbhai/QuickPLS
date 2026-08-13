# PLS Bounded Model Fit v1

Status: frozen release contract. No implementation or promotion evidence is admitted by this document.

## Supported metric set

`pls_model_fit_v1` reports deterministic SRMR and squared Euclidean discrepancy `d_ULS` for both saturated and estimated PLS indicator-correlation models. With observed correlation matrix `S` and implied correlation matrix `Sigma_hat`, `d_ULS = sum_{i>=j}(S_ij - Sigma_hat_ij)^2` and `SRMR = sqrt(d_ULS / (p(p+1)/2))` under the frozen lower-triangle convention.

The composite-model fit context follows Henseler et al. (2014), *Common Beliefs and Reality About PLS*, DOI `10.1177/1094428114526928`. The SRMR interpretation boundary follows Hu and Bentler (1999), DOI `10.1080/10705519909540118`: empirical cutoffs are diagnostics conditional on model, estimator, and data and are not universal proof of fit.

## Preconditions and output

All indicators use the same finite complete cases and deterministic recipe order. Observed and implied matrices must be finite, symmetric, dimensionally identical correlation matrices. Persist model form, indicator order, both matrices or their checksummed reproducible inputs, lower-triangle convention, saturated and estimated SRMR/d_ULS, warnings, and provenance.

SRMR and d_ULS are descriptive discrepancies. No automatic acceptable/unacceptable label is emitted. The current assessment output lacks a dedicated fit method identity; it therefore cannot satisfy this release contract until the exact identity and archive validation are added.

## Exclusions

`d_G`, NFI, RMS_theta, bootstrap exact-fit tests, approximate-fit p-values, universal thresholds, formative-only fit claims, CB-SEM fit semantics, and claims that low discrepancy proves the specified causal model are excluded from v1.
