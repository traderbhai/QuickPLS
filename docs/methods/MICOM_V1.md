# MICOM v1

Status: validated for the documented QuickPLS v1.2.2 two-group MICOM scope.

`micom_v1` is a measurement-invariance payload emitted from `AnalysisMethod::Mga` when recipe metadata contains `group_methods = "micom"` and `mga_group_column` names a two-group observed column.

## Scope

- Exactly two observed, non-missing groups.
- Same model, preprocessing, missing-data policy, and PLS settings for both groups.
- Configural invariance is reported as a checklist status for the shared recipe contract.
- Compositional invariance is screened with deterministic group-label permutations over construct scores.
- Equality of composite means and variances is screened with the same deterministic permutation plan.
- Default `group_permutation_samples` is `999`; validation fixtures may use fewer samples for fast CI smoke checks.

## Output

The `micom` result includes group sizes, construct-level compositional correlation, compositional p value, mean difference/p value, variance difference/p value, partial-invariance flag, full-invariance flag, and bounded-scope warnings.

## Unsupported

Case weights, generated interactions, higher-order constructs, covariance/correlation-only data, more than two groups, too-small groups, and broader invariance claims outside this contract are unsupported.

## Validation

`npm run qpls:micom:reference`, `npm run qpls:v06:validate`, and `npm run qpls:promotion:micom` write the reference and promotion artifacts. Promotion is limited to this documented scope.
