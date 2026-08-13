# CB-SEM Model Comparison v1

Status: frozen release contract. No implementation or promotion evidence is admitted by this document.

## Supported comparisons

`cbsem_model_comparison_v1` compares exactly two converged, admissible `cbsem_ml_v1` models fit to the same listwise-complete rows and identical observed-variable order.

For a preregistered nested pair, the restricted model must be obtainable solely by fixed/equality constraints on parameters in the unrestricted model. The primary likelihood-ratio statistic is `Delta_chi2 = chi2_restricted - chi2_unrestricted` with `Delta_df = df_restricted - df_unrestricted`; a negative value beyond tolerance, nonpositive `Delta_df`, different cases/variables/estimators, or unverified nesting blocks the test. The p-value uses the chi-square upper tail with `Delta_df` degrees of freedom.

For an explicitly non-nested pair, no chi-square-difference p-value is computed. The workflow reports `Delta_AIC = AIC_A - AIC_B` and `Delta_BIC = BIC_A - BIC_B` descriptively, where `AIC = -2 ell + 2k` and `BIC = -2 ell + k log(n)` use the full maximized raw-data log likelihood `ell`. Lower values are favored conditionally.

The contract follows the nested covariance-structure comparison framework in Satorra and Bentler (2001), *A Scaled Difference Chi-Square Test Statistic for Moment Structure Analysis*, DOI `10.1007/BF02296192`, while v1 is restricted to ordinary ML and therefore excludes scaled/robust correction. AIC follows Akaike (1974), DOI `10.1109/TAC.1974.1100705`; BIC follows Schwarz (1978), DOI `10.1214/aos/1176344136`.

## Output and interpretation

Persist both model identities/checksums, shared data identity and row set, nesting declaration and validation, estimator and objective identities, fit statistics, parameter counts, deltas, p-value only when valid, direction, diagnostics, and provenance. A favored model is not thereby proven true; theory, identification, residuals, and admissibility remain separate requirements.

## Exclusions

More than two models, different datasets or observed variables, unverified nesting, robust or ordinal estimator comparisons, boundary-parameter mixtures, cross-validation, automatic specification search, and a non-nested significance test are excluded from v1.
