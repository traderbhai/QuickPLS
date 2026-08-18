# Recipe schema v4 compiler adapters

`recipe_v4_compiler.rs` is the opt-in bridge from an `AnalysisRecipeV4` and an
explicitly resolved `SemModelV4` to the immutable basic estimator plans:

- `CompiledPlsPlanV2` for the exact `smartpls.pls_algorithm` /
  `qpls3.pls.algorithm` capability cell; and
- `CompiledCbsemPlanV2` for the exact `smartpls.cbsem` /
  `qpls3.cbsem.ml` capability cell.

The bridge does not make recipe schema v4 the default and does not activate
archive schema v6. The production constants remain executable recipe schema v3
and archive schema v5.

## Resolution and compilation

`compile_analysis_recipe_v4` requires the caller to supply a resolved model for
both embedded and project-reference bindings. It rejects:

- an unresolved project model;
- `legacy_estimand_unspecified`;
- embedded payload/resolved-model differences;
- model ID or scientific-digest mismatches;
- composite/common-factor and compiler mismatches;
- method/config/compiler mismatches;
- capability-cell substitutions;
- executable legacy metadata;
- invalid or non-finite common settings; and
- every model or data feature rejected by the PLS plan compiler or by the
  separate bounded-ML capability check applied after CB-SEM representation.

The PLS adapter accepts only a non-resampled `pls_pm` recipe with
`pls_algorithm` configuration and composite-only model. The CB-SEM adapter
accepts only bounded ML without bootstrap, groups, invariance, or mean structure,
with common-factor-only model and matching model/input configuration. The
underlying `CompiledCbsemPlanV2` can represent broader models; the adapter calls
`ensure_cbsem_ml_v1_estimator_capability_v2` before attaching the bounded ML
capability cell.

## Compilation receipt

Every successful compilation produces `RecipeV4CompilationReceipt`, containing:

- exact recipe-document SHA-256;
- analytical recipe SHA-256;
- exact resolved-model document SHA-256;
- model scientific SHA-256;
- exact capability-cell reference;
- compiler target and version;
- serialized plan SHA-256; and
- a deterministic analytical-identity SHA-256.

Presentation and annotations can change the exact recipe/model document hashes,
but do not change the scientific model hash, compiled plan, or receipt analytical
identity. A scientific covariance changes all analytical identities and, for
CB-SEM, appears in the compiled covariance table.

`validate_compiled_analysis_recipe_v4` is the archive-validation boundary. It
recompiles from the exact recipe and explicitly resolved model and compares the
entire typed plan and receipt. Stored receipt or plan tampering therefore fails
closed. Receipt, wrapper, and nested compiled-plan objects reject unknown JSON
fields.

## Deliberate boundary

These adapters compile scientific plans; they do not execute them. Runner,
project save/load, CLI, and GUI dispatch remain on their existing contracts
until each method is separately qualified and activated.
