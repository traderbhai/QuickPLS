# SemModelV4 editor authoring contract

The current workbench now has versioned, persisted metadata for the scientific
choices that legacy nodes and edges cannot express on their own. This is an
additive editor contract only. It does not activate recipe schema 4, change old
results, or recalculate an existing project.

## Construct representation

`ConstructData.semModelV4` stores one of:

- `composite`;
- `common_factor`, with an optional explicit marker indicator; or
- `legacy_estimand_unspecified`.

An absent field is also read as legacy unspecified. Reflective measurement does
not automatically mean common factor, and formative measurement does not
silently decide the estimand. The Experimental scientific-authoring inspector
requires an explicit Composite or Common factor confirmation and persists the
factor marker indicator.

## Covariance use

`PathEdgeData.semModelV4` stores one covariance state:

- `scientific`: part of the SEM model, with either the two drawn construct
  variables or an explicit pair of residual/disturbance endpoints;
- `presentation_only`: a visual note that cannot affect calculations; or
- `legacy_unspecified`: an old or role-converted edge that still needs a choice.

The covariance drawing tool creates a new edge as scientific with origin
`new_authoring`. Changing an existing structural/control path's relationship
type to Covariance records `legacy_unspecified`; selecting a visual role alone
therefore cannot change the scientific model. The Experimental inspector
provides Model covariance, Residual/error covariance, Disturbance covariance,
and Presentation only actions. Exact residual and disturbance endpoints are
stored in the existing scientific state. Changing a covariance back to a
structural or control path removes covariance-only metadata.

Old edges with no versioned field stay unclassified. They are never upgraded to
scientific merely because their React Flow role is `covariance`.

## Current execution boundary

Recipe schema 3 still cannot carry scientific covariance relations. Before it
builds or filters any graph edge, `buildNativeAnalysisRecipe` calls
`semModelV4ExecutionBlockers`. A scientific covariance returns a model error
explaining that the compatible execution path is not active and asks the user
to change the edge to Presentation only or remove it. Malformed authoring
metadata also blocks. An explicit factor/composite choice and a covariance made
through relationship-role conversion also stop recipe v3, because that recipe
cannot preserve either choice. Legacy archive arcs with no metadata and
presentation-only arcs retain the historical non-executable behavior.

This guard must remain until the live runner consumes recipe schema 4 and the
compiled V4 plan.

## Persistence and conversion

The metadata lives inside the existing node/edge presentation payload, which is
already stored as opaque project JSON. Canonical project reconciliation now:

- restores valid construct authoring fields on canonical constructs;
- restores covariance edges separately from structural paths; and
- never uses a covariance presentation edge as the visual identity of a
  canonical regression path.

Project dirty-state signatures include only the versioned SEM authoring field,
not transient React Flow data. Store conversion actions use the existing
history mechanism, so create and both conversion directions are undoable.

`adaptAuthoredNativeWorkbenchToSemModelV4` reads these persisted fields into the
strict live-workbench adapter. Missing legacy decisions produce the normal
actionable confirmation diagnostic instead of a guessed model.

The customer-facing authoring workflow, endpoint validation, Experimental Labs
gate, and remaining live gaps are documented in
`SEM_MODEL_V4_SCIENTIFIC_AUTHORING.md`.

## Pure helpers

`semModelV4Authoring.ts` contains non-mutating helpers for:

- constructing a newly authored scientific covariance;
- assigning construct representation;
- converting covariance to scientific or presentation-only use;
- recording unresolved role conversion;
- duplicate unordered-pair detection;
- strict metadata inspection; and
- recipe-v3 execution blocking.
