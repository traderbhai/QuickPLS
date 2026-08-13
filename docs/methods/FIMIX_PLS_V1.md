# FIMIX-PLS v1

Status: the bounded deterministic engine and its historical QuickPLS v1.2.2 audit exist. The QuickPLS 3 method-promotion factory derives `absent` because no evidence has been admitted under the new identity-bound contract. This document therefore specifies current behavior and its limitations; it does not claim QuickPLS 3 release qualification.

`fimix_pls_v1` is emitted from `AnalysisMethod::Predict` when the typed FIMIX configuration, `group_methods = "fimix"`, or compatible `fimix_classes` metadata is present.

## Exact current scope

- Exactly 2 or 3 requested classes.
- At least 40 complete model observations and enough rows to satisfy `max(8, ceil(n * minimum_share))` in every class.
- One through fifty deterministic starts and a minimum class share from 0.05 through 0.40.
- Standardized construct-score and structural path-product features.
- The same deterministic hard-partition core and segment-specific structural regressions as `pls_pos_v1`.

Case weights, generated interactions, higher-order constructs, covariance/correlation-only data, too-small samples, singular class fits, automatic class-count search, and more than three classes are unsupported.

## Pseudo-likelihood and membership calculations

Let `J` be the selected hard-partition structural residual sum of squares and `n` the complete-case count. QuickPLS computes:

- `sigma2 = max(J / n, 1e-12)`;
- `ell_tilde = -n/2 * [ln(2*pi*sigma2) + 1]`;
- `k = C * (number_of_paths + number_of_constructs) + C - 1`;
- `AIC = -2*ell_tilde + 2*k`;
- `BIC = -2*ell_tilde + k*ln(n)`; and
- `CAIC = -2*ell_tilde + k*(ln(n)+1)`.

For row `i` and class `c`, squared feature distance `d_ic` from the class centroid becomes

`q_ic = exp(-min(d_ic,700)) / sum_h exp(-min(d_ih,700))`.

The reported classification certainty is `1 - [-sum_i sum_c q_ic ln(q_ic)] / [n ln(C)]`.

These `q_ic` values are inverse-distance membership scores. They are not posterior probabilities from an estimated finite-mixture likelihood. `ell_tilde` is a Gaussian residual pseudo-likelihood for the selected hard partition, not the likelihood optimized by a full FIMIX expectation-maximization procedure. AIC/BIC/CAIC from this payload must not be compared with full-mixture criteria as though the likelihoods were equivalent.

## Interpretation and product wording

The payload includes class sizes, membership scores, dominant memberships, class paths and R-squared, the pseudo-likelihood criteria, classification certainty, starts, iterations, warnings, and provenance.

Current product wording must say "bounded deterministic FIMIX-style score-space diagnostic." It must preserve the random-start EM qualification exclusion and is not blanket full EM/FIMIX parity. Class labels are arbitrary; no class is substantively real without external stability and validation work.

If QuickPLS later implements a genuine finite-mixture likelihood with estimated class proportions/variances and expectation-maximization, it requires a new method version, specification, evidence manifest, result identity, and legacy policy. It must not silently change `fimix_pls_v1` semantics.

## QuickPLS 3 qualification contract

`validation/methods/fimix_pls_v1.manifest.json` freezes the evidence roles. Existing `npm run qpls:fimix:recovery`, `npm run qpls:v06:validate`, and `npm run qpls:promotion:fimix-pls` output is legacy engine evidence only.

Release qualification for the bounded diagnostic still requires an independent reproduction of every partition, pseudo-likelihood, criterion, score, and entropy identity; two/three-class and divergence-from-full-FIMIX simulations; strict archive validation; dedicated native and export coverage; identity-bound audit output; and installed Windows acceptance. A full commercial FIMIX-PLS claim additionally requires the separately versioned genuine mixture engine described above.

## Scientific context

Hahn, Johnson, Herrmann, and Huber (2002), *Capturing Customer Heterogeneity using a Finite Mixture PLS Approach*, https://doi.org/10.1007/BF03396655.
