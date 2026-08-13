# CB-SEM Multigroup Analysis v2

Status: frozen release contract. No implementation or promotion evidence is admitted by this document.

## Supported method

`cbsem_multigroup_v2` compares one preregistered structural path between exactly two observed, mutually exclusive groups under the accepted continuous reflective ML scope. Both groups use the same configural model, marker convention, observed-variable set, preprocessing, and estimator. At least metric invariance from `cbsem_invariance_v2` must be established before a structural coefficient comparison; scalar invariance is additionally required for latent-mean comparisons, which v2 does not perform.

The unstandardized group difference is `Delta_beta = beta_g1 - beta_g2`. The workflow fits an unconstrained simultaneous multigroup model and a model constraining the selected path equal. It reports `Delta_chi2 = chi2_equal - chi2_free` with one degree of freedom. It independently qualifies stratified reuse of the `cbsem_bootstrap_v2` case-resampling, seed, quantile, and failure protocol: rows are resampled within each group and every replicate refits both groups. The resulting percentile interval for `Delta_beta` belongs to `cbsem_multigroup_v2`, not the standalone bootstrap identity.

The equality-constraint framework follows Joreskog (1971), *Simultaneous Factor Analysis in Several Populations*, DOI `10.1007/BF02291466`, and the invariance prerequisite follows Meredith (1993), DOI `10.1007/BF02294825`.

## Preconditions and output

The group column is immutable and excluded from indicators. Missing group values, fewer or more than two retained levels, an empty/small group, non-overlapping measurement models, failed invariance, inadmissible group fits, or differing row policies block execution. Persist group labels/counts, constrained and unconstrained parameter tables and fits, the selected path, `Delta_beta`, likelihood-ratio result, bootstrap interval, all replicate failures, warnings, and provenance.

Standardized path differences may be displayed only as descriptive supplements because group-specific variances change their scale. The inferential estimand is the unstandardized coefficient difference.

## Current preview is inadmissible

The existing `cbsem_multigroup_v1` preview apportions pooled fit values by group size and synthesizes equality-step deltas without group-specific constrained ML refits. It is permanently barred from v2 evidence or migration/relabeling.

## Exclusions

Three or more groups, latent classes, unequal measurement forms, automated path scanning, simultaneous inference over multiple paths, latent means, ordinal/robust estimators, clustered observations, and partial pooling are excluded from v2.
