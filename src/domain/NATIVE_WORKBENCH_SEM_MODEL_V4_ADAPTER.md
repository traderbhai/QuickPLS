# Native workbench to SemModelV4 adapter

`nativeWorkbenchSemModelV4Adapter.ts` is a dormant Wave 1 bridge from the
current editable React Flow graph to `SemModelV4`. It deliberately does not
replace `buildNativeRecipeModel`, change archive versions, or activate the V4
compilers in the live calculation runner.

## Why this adapter reads the live graph

The schema-v3 recipe model was designed for the current estimators and omits
`role=covariance` edges. Converting that recipe would therefore lose a
scientific relation before V4 compilation. This adapter consumes the original
workbench nodes and edges, so every structural, control, covariance, and
generated measurement edge is accounted for.

## Fail-closed inputs

The caller must supply two decisions that the legacy graph cannot prove:

- `construct_estimands` has one `composite` or `common_factor` entry for every
  construct. Missing entries and `legacy_estimand_unspecified` return
  `native_workbench.estimand_confirmation_required`; the adapter never infers
  factor semantics from a reflective arrow.
- `covariance_semantics` has one entry for every current covariance edge. A
  `scientific` entry creates a relation plus parameter. A
  `presentation_only` entry creates only a `display_only_covariance`
  annotation. An unclassified covariance returns
  `native_workbench.covariance_classification_required`.

The convenience entry point `adaptAuthoredNativeWorkbenchToSemModelV4` reads
the same decisions from the versioned `semModelV4` metadata persisted by the
editor. Missing legacy metadata remains pending; it is not treated as a
scientific covariance or factor/composite decision.

The authored entry point also applies exact parameter metadata from the owning
construct or relationship. It supports explicit free/fixed specifications,
starts, bounds, equality labels, all three common-factor identification
policies, observed intercepts, latent means, and ordinal thresholds. Invalid,
duplicate, stale, or cross-owned entries return typed diagnostics instead of
being ignored. It never implies undrawn exogenous or causal-indicator
covariances and never fixes causal-indicator variances without an explicit
future setting.

For a scientific latent covariance, omit both endpoint overrides and the two
drawn construct variables are used. Residual or disturbance covariance uses
explicit `SemEndpointV4` values. Both exact endpoints must belong to the two
constructs connected on the canvas, which prevents a diagram from displaying
one relation while the engine receives another.

## Determinism and identity

Construct variables use `construct:<source id>` and observed variables use
`observed:<source column>`. Relations and parameters use an injective UTF-8
hex encoding of the stable source edge or measurement identity. The completed
model is canonicalized, so node, edge, indicator, and declaration order do not
change its scientific identity. Common-factor marker selection is explicit
when provided and otherwise uses the lexicographically first assigned
indicator, avoiding declaration-order dependence.

The success receipt contains a trace from every live edge to its scientific
relation or presentation annotation. `data_binding` and `group` are copied
without reinterpretation. Additional weight, grouping, cluster, or strata
columns can be declared through `observed_semantics`; the normal SemModelV4
checks then verify their roles and scale types.

## Diagnostics and non-activation boundary

`adaptNativeWorkbenchToSemModelV4` returns a typed success/failure union.
Failures contain a stable code, stage, subject, message, and corrective action.
`requireNativeWorkbenchSemModelV4` is the throwing convenience wrapper. Invalid
or stale endpoints, duplicate paths, ambiguous indicators, unsupported derived
legacy nodes, malformed bindings, and incomplete covariance endpoint pairs all
fail before a V4 model can be used.

This bridge must remain opt-in until schema 6 / recipe 4 migration UX is wired.
Existing projects and historical results retain their current interpretation;
the adapter neither upgrades nor recalculates them automatically.

## Focused checks

`nativeWorkbenchSemModelV4Adapter.test.ts` covers:

- latent and residual scientific covariance survival;
- presentation-only covariance isolation from the scientific hash;
- factor/composite differentiation and legacy ambiguity blocking;
- JSON save/reopen plus strict SemModelV4 parsing;
- stable IDs and output under declaration reorder;
- exact data-binding preservation; and
- actionable failures for stale structural, measurement, and covariance
  endpoints.
