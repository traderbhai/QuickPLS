# CTA-PLS v1

`cta_pls_tetrad_v1` is the scoped Standard QuickPLS workflow for the frozen v1 bounded descriptive contract. Its numerical, archive, native, and packaged identities passed under source-bound qualification; current-build receipts are refreshed at the coordinated release gate. This status proves a functional offline CTA workflow, not the separate commercial zero-process-egress release gate. Broader inferential CTA-PLS decision rules remain unsupported.

`AnalysisMethod::CtaPls` runs the ordinary PLS estimator first, then computes tetrad diagnostics for indicator blocks with four or more indicators. The current result reports `method_version = "cta_pls_tetrad_v1"` and stores a typed `cta_pls` payload.

Implemented contract:

- indicator columns use the same preprocessing and complete-case row set as the PLS execution recipe;
- tetrads are computed from sample covariances of the preprocessed indicator columns;
- each indicator quadruple emits three tetrad pairings: `ab_cd_minus_ac_bd`, `ac_bd_minus_ad_bc`, and `ad_bc_minus_ab_cd`;
- the payload reports construct id, ordered indicators, pairing id, signed tetrad, absolute tetrad, and max absolute tetrad by construct;
- recipes must contain at least one ordinary construct with four or more indicators;
- reflective and formative blocks are both reportable because the diagnostic does not classify either measurement shape;
- the native v1 workflow excludes controls, covariance edges, interactions, higher-order constructs, case weights, PCA weighting, and every resampling setting;
- the saved payload is accepted only when method identity, complete pairing coverage, signed/absolute values, the three-pair zero-sum identity, construct maxima, immutable recipe settings, and dataset fingerprint remain coherent;
- the native Results tree exposes an accessible block summary, complete tetrad table, scope/exclusions table, and same-run CSV/HTML/XLSX provenance export.

Unsupported outside the validated v1.2.3 descriptive scope:

- PCA weighting;
- bootstrap, permutation, or asymptotic tetrad inference;
- vanishing/non-vanishing tetrad classification decisions;
- bootstrap/permutation CTA decision rules;
- diagram annotations or reflective/formative classification.

Validation evidence:

- `npm run qpls:cta:reference` writes `validation/results/cta_pls_reference_report.json`.
- The reference script independently standardizes the data, computes sample covariances, recomputes all tetrad pairings, and checks the invalid less-than-four-indicator guard.
- Current observed max delta is `4.94e-14`.
- `validation/cta_pls_simulation.py`, `validation/cta_pls_boundary_gate.py`, and `validation/cta_pls_persistence_gate.py` are the factory evidence entry points for numerical recovery, fail-closed boundaries, and strict archive round trips.
- The frozen simulation gate covers six 40-to-180-row scenarios, four-to-six-indicator blocks, standardized and mean-centered preprocessing, listwise missingness, bounded nonnormality, correlated-residual non-vanishing tetrads, and two eligible blocks. Every expected pairing is checked against an independent Python sample-covariance implementation within `1e-6`.
- The native gate exercises CTA-specific setup, accessible results, identity tamper rejection, typed recipe scope, accessible calculation-dialog content, same-run export selection, CSV/HTML rendering, XLSX table projection, and run provenance.
- `validation/methods/cta_pls_v1.manifest.json` reserves the release roles for the dedicated packaged Windows report and final method audit. The validator derives `release_qualified` only when both artifacts are current and prove invalid setup blocking, execution, exact same-run UI/archive/XLSX values, save/reopen, all three frozen viewports, functional offline operation, fail-closed mutations, and cleanup.
- Functional offline qualification requires a successful analysis/export/save-reopen chain and application/browser requests confined exactly to `http://tauri.localhost` and `http://ipc.localhost`, with no external application request or runtime network dependency.
- The same packaged report separately records sampled process-tree TCP rows as `platform_background_egress_observed` and derives `commercial_zero_egress_passed`. Observed WebView2 background egress is preserved with `commercial_zero_egress_passed = false`; it does not fail this bounded method qualification and cannot be represented as commercial zero-egress evidence. The global diagnostics/commercial release gate remains strict and authoritative for that claim.

Publication status: Standard within the bounded descriptive contract above. It is not inferential CTA-PLS evidence and does not by itself support a commercial zero-egress claim. Bootstrap/permutation tetrad decision rules and broader CTA interpretation remain unsupported.
