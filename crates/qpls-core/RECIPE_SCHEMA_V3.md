# Analysis recipe schema v3

Schema v3 separates the executable scientific contract from descriptive
annotations:

- `method_config` is a Serde-tagged contract. Its `kind` uses the stable native
  workflow name and must be compatible with `settings.method`.
- `method_config` and its nested configuration objects reject unknown fields.
  Misspelled executable settings therefore fail during deserialization instead
  of being ignored and replaced by defaults.
- `metadata` remains available for non-executable annotations such as `status`,
  `demo`, fixture provenance, and user-facing labels. A v3 recipe containing a
  recognized legacy executable metadata key is invalid, even if that value
  happens to match `method_config`.
- Recipe schema versions are independent from the project archive version.
  `ANALYSIS_RECIPE_SCHEMA_VERSION` is `3`; the project archive remains on its
  own version contract.

Historical v1 and v2 recipes deserialize with `method_config: None` and retain
their original `schema_version`. They are never upgraded during deserialization.
For a v2 recipe, callers may first use `normalize_legacy_v2` to inspect the
deterministic typed interpretation, then explicitly call `migrated_v3` to create
a separate v3 value. The migration removes recognized executable keys while
preserving non-executable metadata. Alias disagreements, ambiguous simultaneous
PLS bootstrap and permutation settings, invalid values, and executable keys for
another method fail closed.

The historical `settings.method = bootstrap` alias is normalized to the
canonical schema-v3 pairing `settings.method = pls_pm` plus
`method_config.kind = pls_bootstrap`. The source v2 recipe is unchanged.
An alias with zero bootstrap samples, or any historical PLS recipe combining
bootstrap and permutation without an explicit v3 primary workflow, fails
closed.
`migrated_v3` is deterministic and retains the historical recipe ID for
inspection or archive transformation. A caller preparing a distinct new run or
append should use `migrated_v3_with_fresh_id`; duplicate-ID enforcement remains
the responsibility of the project boundary.

For a newly authored v3 PLS recipe, `pls_bootstrap` may also request the
existing optional permutation result. Bootstrap remains the primary typed
workflow, and both inference payloads retain their independent method-version
labels. This deliberately preserves QuickPLS's combined-output workflow. A v2
recipe enabling both remains migration-ambiguous and fails closed because its
author did not make that v3 choice explicitly. The asymmetry is intentional:
`pls_permutation` is permutation-only and rejects bootstrap or studentized
bootstrap settings.

`AnalysisRecipe::effective_metadata` is the authoritative compatibility API for
scientific validators and engines while remaining workspace consumers move to
typed fields. Its precedence is explicit:

1. v1/v2 return the original metadata only and reject an unexpected typed
   config.
2. v3 retains non-executable metadata and adds a projection generated from
   `method_config`.
3. v3 rejects any caller-supplied recognized executable metadata key instead of
   overwriting either value.
4. unknown/future recipe schemas fail closed.

`AnalysisRecipe::with_effective_metadata` supplies the same policy as a cloned
recipe for legacy engine entry points. Neither API mutates the persisted recipe.
The internal deterministic projection used by that policy is never persisted as
authoritative v3 configuration.

Production engine entry points do not accept that projected clone directly.
`ValidatedExecutionRecipe::for_dataset` is an opaque execution capability: it
requires the current schema, binds the recipe to the concrete dataset
fingerprint, runs the complete scientific validator, and owns the generated
compatibility projection. Callers can inspect only read-only source/effective
views and cannot construct or mutate the capability. Estimation, assessment,
resampling, and the runner therefore cannot be invoked through a public raw
"already projected" recipe bypass. Its `without_outer_resampling` derivation
revalidates a base-estimation recipe once before worker loops while preserving
dedicated method-internal plans such as MGA/MICOM and NCA permutations.

Standalone regression has one shared executable envelope for OLS, logistic,
and PROCESS recipes: path weighting as a non-operative sentinel,
unstandardized values, listwise deletion, no case weights, fixed two-sided 95%
confidence intervals, and an empty SEM model. Outcome, predictor, and control
names must all be non-empty and distinct. HC3 is an OLS-only requirement. Every
PROCESS relationship variable must be a declared predictor, must differ from
the outcome, and must be distinct from the other relationship variables.

The logistic member of that envelope is the exact binary numeric contract:
after listwise deletion the outcome must contain both `0` and `1` and no other
finite value. Dataset-dependent readiness is reported by the estimation input
profiler rather than guessed by recipe validation. Categorical auto-encoding,
weights, Firth correction, multinomial and ordinal models remain outside the
typed schema-v3 logistic scope.
Point-only standalone regression freezes `workers = 1`. OLS, binary logistic,
and graph-defined PROCESS v2 recipes may instead include the strict typed
`bootstrap` object with
`algorithm = case_resampling` and ordered intervals `[percentile, bca]`, paired
with 99 to 10,000 `settings.bootstrap_samples` and at most 50 selected
predictors/controls plus the intercept for OLS or binary logistic regression;
graph-defined PROCESS retains the stricter eight-predictor and one-control
limits below. That dedicated outer engine
accepts 1 to 64 workers but derives each with-replacement sample from the fixed
seed and replicate index, then reduces in index order. It cannot enter the
generic PLS bootstrap payload. Graph-defined PROCESS uses its dedicated
`regression_process_bootstrap_v1` engine, result, and witness tokens; it never
inherits the release claim of `regression_bootstrap_v1` for OLS/logistic.
`without_outer_resampling` removes the typed
bootstrap object, clears the sample count, restores one worker, and revalidates
the point-only capability used inside each fit. Historical PROCESS v1 is
archive-readable only: current execution capabilities reject its typed
mediation/moderation relationships, and v2-to-v3 migration returns an explicit
archive-only error instead of manufacturing an executable v1 recipe.
Studentized and permutation regression inference remain excluded from this
contract. Exact
result arithmetic and version tokens are frozen in
`docs/methods/REGRESSION_BOOTSTRAP_V1.md`.

Graph-defined PROCESS v2 is a typed DAG rather than a numbered macro. The wire
declares a focal predictor, 1 to 16 unique paths, at most four mediators, at
most two reusable continuous or exact `0/1` moderators, at most four moderated
paths, no more than eight predictors in total, one optional control, no more
than 50 generated design terms in any equation, and the immutable centering token
`equation_complete_case_mean_v1`. Predictors are ordered as focal X, mediators
in deterministic topological/lexical order, then moderators in declaration
order; one optional control enters every equation. The outcome is terminal and
must be reachable from X. Continuous variables used in products are centered
inside each original/resample/delete-one estimation sample; binary moderators
remain uncentered. Moderator hierarchy terms are generated by the estimator,
not encoded as graph paths. Two-moderator three-way effects are limited to the
direct X-to-outcome path. A mediated path may contain at most one moderated
first or final stage. Cyclic/reciprocal graphs, weights/clusters, logistic
equations, categorical coding beyond exact binary moderators, and copied
numbered PROCESS templates are not part of this contract.

CB-SEM `bootstrap_samples` is required on the v3 wire even when its value is
zero. This keeps an omitted inference choice distinguishable from an explicit
request for no bootstrap; optional grouping and invariance fields retain their
documented empty defaults.

New runner executions and newly appended project results require schema v3.
Historical v1/v2 recipes and their existing results remain archive-readable;
execution or append requires an explicit migrated v3 copy. Runner and project
validation use an effective compatibility clone only within the current call,
while result provenance and project storage retain the original v3 recipe.

`AnalysisRecipe::try_new` is the safe convenience constructor. It infers only
complete deterministic defaults and returns
`ExplicitMethodConfigRequired` for MGA, PCA, regression, and NCA, whose
scientific parameters cannot be invented. `AnalysisRecipe::new` remains a
source-compatible convenience wrapper for established non-parameterized
callers and fails loudly on those methods; `new_with_method_config` is the
explicit construction API. Standard prediction without segmentation and IPMA
with its documented all-endogenous-target default remain deterministic, while
CB-SEM infers CFA versus SEM from whether the model contains structural paths.
