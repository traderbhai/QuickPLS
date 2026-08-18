# Analysis recipe schema v4 foundation

`analysis_recipe_v4.rs` defines the staged schema-v4 recipe contract for the
SemModelV4 migration. It is deliberately additive: `AnalysisRecipe` schema v3
remains the only recipe accepted by current estimators until each estimator has
an audited SemModelV4 compiler adapter.

## Model binding and legacy estimands

A schema-v4 recipe has exactly one explicit model binding:

- an embedded `SemModelV4` plus its scientific SHA-256 digest;
- a project `SemModelV4` reference plus its scientific SHA-256 digest; or
- `legacy_estimand_unspecified`, which preserves the legacy `ModelSpec` identity
  and digest but blocks execution.

Legacy migration never guesses factor-versus-composite semantics. The
`confirm_legacy_recipe_estimand_v4` operation converts a pending recipe only
after an explicit `pls_composite` or `cbsem_common_factor` choice. The resulting
recipe records whether it was confirmed as a composite or common factor.

`execution_readiness` is fail-closed. It separately reports pending estimand
confirmation, unresolved project models, missing method configuration, and the
absence of an estimator adapter. This prevents the archive contract from being
mistaken for an executable integration.

## Covariance migration

Legacy covariance drawings are passed to the converter as
`LegacyDisplayCovarianceV4` and become `display_only_covariance` annotations.
They do not enter the scientific relation or parameter tables and do not alter
the model's scientific digest.

Only `convert_display_covariance_to_model_v4` can promote one of these drawings
to a scientific covariance. The caller must provide new relation and parameter
IDs and parameter settings. The operation returns a validated copy, removes the
annotation, creates the covariance relation and free parameter, and changes the
scientific digest.

## Compatibility boundary

- Schema-v1 through schema-v3 recipe bytes remain historical inputs.
- Migration preserves recipe ID, creation time, dataset fingerprint, settings,
  typed method configuration, metadata, and a digest of the source recipe.
- Schema-v4 unknown fields and binding/hash mismatches fail closed.
- No current runner, PLS estimator, or CB-SEM estimator consumes schema v4 yet.

The opt-in, non-executing compiler bridge and archive-validation receipt are
documented in `RECIPE_V4_COMPILER_ADAPTERS.md`.
