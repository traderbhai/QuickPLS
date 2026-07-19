# CTA-PLS v1

Status: validated for the documented QuickPLS v0.9.0-rc.1 supported CTA-PLS diagnostic scope. Broader decision rules outside this contract remain unsupported.

`AnalysisMethod::CtaPls` runs the ordinary PLS estimator first, then computes tetrad diagnostics for indicator blocks with four or more indicators. The current result reports `method_version = "cta_pls_tetrad_v1"` and stores a typed `cta_pls` payload.

Implemented contract:

- indicator columns use the same preprocessing and complete-case row set as the PLS execution recipe;
- tetrads are computed from sample covariances of the preprocessed indicator columns;
- each indicator quadruple emits three tetrad pairings: `ab_cd_minus_ac_bd`, `ac_bd_minus_ad_bc`, and `ad_bc_minus_ab_cd`;
- the payload reports construct id, ordered indicators, pairing id, signed tetrad, absolute tetrad, and max absolute tetrad by construct;
- recipes must contain at least one construct with four or more indicators.

Unsupported in this preview:

- PCA weighting;
- bootstrap, permutation, or asymptotic tetrad inference;
- vanishing/non-vanishing tetrad classification decisions;
- publication-ready CTA interpretation;
- dedicated result tables and diagram annotations.

Validation evidence:

- `npm run qpls:cta:reference` writes `validation/results/cta_pls_reference_report.json`.
- The reference script independently standardizes the data, computes sample covariances, recomputes all tetrad pairings, and checks the invalid less-than-four-indicator guard.
- Current observed max delta is `4.94e-14`.

Publication status: experimental. Treat the output as a descriptive tetrad screen until inference, simulations, published examples, UI/report review, and release-family promotion are complete.
