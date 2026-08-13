# CB-SEM Measurement Invariance v2

Status: frozen release contract. No implementation or promotion evidence is admitted by this document.

## Supported hierarchy

`cbsem_invariance_v2` performs simultaneous two-group ML confirmatory factor analysis for the continuous reflective measurement model accepted by `cbsem_ml_v1`. Models are fitted in a strict hierarchy:

1. Configural: identical indicator-factor pattern and identification, with parameters free by group.
2. Metric: corresponding free loadings constrained equal across groups.
3. Scalar: metric constraints plus corresponding observed intercepts constrained equal; this step requires a fitted mean structure and identified latent means.

For adjacent steps, `Delta_chi2 = chi2_restricted - chi2_less_restricted` and `Delta_df = df_restricted - df_less_restricted`. The workflow reports the ordinary-ML chi-square difference test plus `Delta_CFI` and `Delta_RMSEA` as diagnostics, without converting heuristic cutoffs into universal pass/fail truth.

The hierarchy and equality definitions follow Meredith (1993), *Measurement Invariance, Factor Analysis and Factorial Invariance*, DOI `10.1007/BF02294825`, and Joreskog (1971), DOI `10.1007/BF02291466`.

## Preconditions and failure behavior

Exactly two observed groups must share the same indicators, factor pattern, row policy, estimator, and marker convention. Each step requires convergence, admissibility, positive degrees of freedom, and a valid nested relationship. A failed earlier step blocks all later steps. Group sizes, constraints, released constraints, fits, deltas, residuals, diagnostics, and provenance are persisted.

Partial invariance is allowed only as an explicitly user-selected release of named constraints after the fully constrained step fails; every release is recorded and no automatic modification-index search is permitted. Partial scalar results cannot support unconstrained latent-mean claims.

## Current preview is inadmissible

The existing `cbsem_invariance_v1`/multigroup preview changes pooled fit numbers arithmetically rather than fitting constrained group models and does not implement a valid scalar mean structure. It is permanently barred from v2 evidence or migration/relabeling.

## Exclusions

More than two groups, longitudinal/dependent samples, ordinal thresholds, robust or WLSMV estimators, strict residual invariance, alignment optimization, approximate/Bayesian invariance, automatic constraint release, and publication claims based solely on heuristic fit-index deltas are excluded from v2.
