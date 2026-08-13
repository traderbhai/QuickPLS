# GSCA ALS v2

Status: validated for the bounded QuickPLS native `gsca_als_v2` scope.

## Executable contract

- Joint global least-squares alternating least squares (`alternating_least_squares_v1`).
- Deterministic +1 block-weight initialization.
- Raw numeric case-level data, listwise deletion, and standardized indicators.
- Two or more connected constructs with disjoint reflective or formative blocks and recursive single-group structural paths.
- Maximum 3,000 iterations and a `1e-7` stopping rule applied to both objective change and normalized component weights.
- Output includes component weights, measurement loadings, structural paths, endogenous R2, objective, FIT, adjusted FIT, measurement FIT, structural FIT, GFI, SRMR, convergence, iterations, complete cases, and omitted cases.

## Excluded scope

Controls, covariance paths, interactions, higher-order constructs, case weights, covariance/correlation-only input, multigroup analysis, feedback loops, regularization, bootstrap, permutation, and other inference are rejected. Construct-score export is not exposed because retained source-row identities are not yet carried with score rows.

## Evidence

`python validation/gsca_als_v2_reference.py` compares the QuickPLS result with an independently expressed SciPy SLSQP minimization of the global GSCA criterion. The method-specific report compares weights, loadings, paths, R2, objective, FIT, adjusted/local FIT, GFI, SRMR, and covariance discrepancy within `2e-6`.

The focused packaged acceptance (`validation/run_v247_gsca_native_acceptance.ps1`) additionally proves visible mixed reflective/formative authoring, a genuine native calculation lifecycle, strict results, native XLSX export, typed archive validation, explicit save, and same-run reopen.

## Legacy boundary

Historical `gsca_v1` payloads are retained only as legacy preview records. They are not reinterpreted as GSCA ALS v2 evidence and cannot justify current GSCA numerical or inference claims.
