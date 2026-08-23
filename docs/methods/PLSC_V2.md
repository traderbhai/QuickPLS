# PLSc v2

Status: evidence-derived native-qualified for the documented reflective path/factor-weighting PLSc scope. Method-scoped packaged Windows acceptance and the final release audit remain required before release qualification.

`AnalysisMethod::Plsc` first estimates the ordinary Mode A PLS model, then applies the consistent correction for reflective constructs. New results emit `method_version = "plsc_v2"` and a typed PLSc payload containing Dijkstra-Henseler rho_A reliabilities, original and corrected construct correlations, corrected structural paths, corrected outer loadings, corrected R2, and warnings.

## Reliability correction

For each construct, QuickPLS builds the empirical indicator correlation matrix `R` and expresses its Mode A weight vector in standardized-indicator coordinates. It normalizes the vector so `w'Rw = 1`, then evaluates Equation 3 of Dijkstra and Henseler (2015):

`rho_A = (w'w)^2 * [w'(R - diag(R))w] / [w'(ww' - diag(ww'))w]`.

The equation kernel is shared with `qpls-assessment`, whose `dijkstra_henseler_rho_a_v1` contract is covered by hand-calculated, independent Decimal, primary-paper, and cSEM fixtures. The primary source is Dijkstra and Henseler, *Consistent Partial Least Squares Path Modeling*, MIS Quarterly 39(2), 2015, Equation 3 and Appendix B (DOI `10.25300/MISQ/2015/39.2.02`).

Finite-sample improper rho_A values are not broadly clamped. Only floating-point boundary excursions are canonicalized. A materially nonpositive or above-one reliability blocks PLSc attenuation correction.

## Corrected model and inadmissibility policy

Off-diagonal construct correlations are divided by the square root of the two construct reliabilities. A corrected correlation materially outside `[-1, 1]` aborts the calculation as inadmissible. QuickPLS does not truncate the value to manufacture a valid correlation matrix. Values inside floating-point tolerance are canonicalized only after the guard.

The corrected correlation matrix determines structural paths and R2. Corrected outer loadings use the same construct reliability. Reported R2 and loadings retain their existing bounded reporting policy; that policy does not alter or conceal an inadmissible construct-correlation matrix.

## Supported scope

- reflective constructs with at least two indicators each;
- path or factor inner weighting;
- no PCA weighting;
- no generated interaction or higher-order constructs;
- no bootstrap, permutation, or studentized resampling within the PLSc run;
- complete-case preprocessing according to the ordinary PLS recipe.

The two-indicator case is allowed but has limited reliability information. Broader PLSc estimator shapes remain unsupported.

## Reproducible evidence

- `qpls-core` hand fixtures exercise Equation 3 directly.
- `qpls-assessment` retains the primary-paper Equation 3, independent Decimal, metamorphic, improper-value, and cSEM comparison suites.
- `qpls-estimation` compares the PLSc payload against independently evaluated canonical rho_A values on the committed 120-case PLSc fixture.
- `npm run qpls:plsc:reference` regenerates the deterministic dataset, recipe, QuickPLS payload, and independent NumPy Equation 3 report.
- `qpls-project` proves current `plsc_v2` save/reopen behavior, exact legacy `plsc_v1` compatibility, and rejection of mismatched or unknown payload/provenance versions.

The native method selector, applicability checks, correction/result tables, and export projections are source-bound and tested. Release qualification remains pending genuine packaged Windows execution, export, save/reopen, and the final method audit.

The legacy 12-case `validation/fixtures/corporate_reputation.csv` smoke fixture is intentionally not positive PLSc evidence: its attenuation-corrected correlation matrix is inadmissible under the canonical equation. It is separate from the full built-in Corporate Reputation PLS-SEM sample. That failure must remain visible rather than being clamped or hidden.
