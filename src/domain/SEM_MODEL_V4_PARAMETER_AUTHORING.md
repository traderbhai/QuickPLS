# SemModelV4 parameter authoring

Status: Internal/Experimental editor contract. This surface stores a future SemModelV4 scientific model; it does not activate recipe-v4 or change any historical result.

## Supported authoring

The Experimental Parameter Table can edit generated scientific parameters as either:

- free, with an optional finite start value, lower bound, upper bound, and equality label; or
- fixed, with one finite value.

Common factors can use marker-loading, fixed-variance, or effects-coding identification. Marker and fixed-variance parameters are governed by the selected identification method. Effects coding keeps all participating loadings free and materializes an explicit stable linear sum constraint.

The variable rows can also add:

- one observed intercept for a supported continuous or binary indicator;
- one estimated latent mean for a common factor; and
- `categories - 1` thresholds for an ordinal indicator with at least two declared categories.

All authored entries have stable IDs and exact typed targets. Construct-owned loading, variance, residual-variance, mean, intercept, and threshold entries are stored on the owning construct's versioned `semModelV4` metadata. Regression and covariance entries are stored on their owning relationship. One form submission creates one immutable store update, so undo and redo restore the complete edit together. The same metadata survives JSON/project-presentation reopen.

## Fail-closed behavior

- A parameter edit must remain on the construct or relationship that owns its exact target.
- Duplicate IDs, stale targets, malformed metadata, non-finite values, invalid bounds, and invalid equality labels produce typed diagnostics with a corrective action.
- The editor preflights a candidate model before committing it. An incompatible identification or cross-object edit does not leave invalid metadata in the store.
- Legacy factor-versus-composite ambiguity and unclassified covariance arcs still block the table; neither is guessed.
- Group-specific overrides and feedback/reciprocal systems are blocked in this authoring surface with a typed diagnostic. The general SemModelV4 storage model still preserves feedback relations.
- Results and technical provenance remain read-only. Parameter authoring changes only the experimental model presentation.

## Explicit non-inference and remaining execution gaps

Only a drawn scientific covariance becomes a covariance relation. This slice never implies all exogenous latent correlations, never implies causal-indicator correlations, and never fixes causal-indicator variances automatically. SmartPLS-style Special Assumptions toggles for those three behaviors remain unimplemented settings/compiler work; until they have explicit persisted settings and plan/result/provenance materialization, they must be treated as unsupported rather than inferred.

The current recipe-v3 runner does not consume any of this parameter metadata. The existing CB-SEM compiler also rejects mean, intercept, and threshold parameters. Group overrides, observed-variable structural authoring, cross-group constraints, and qualified nonrecursive estimation remain future execution-path work. Standard Calculate does not expose this surface.

## Focused checks

- `semModelV4ParameterAuthoring.test.ts`: specification validation, ownership, all three factor identification policies, location parameters, feedback/group blocking, stable IDs, reopen, and reorder invariance.
- `store.semModelV4ParameterAuthoring.test.ts`: atomic undo/redo and JSON reopen for construct and relationship metadata.
- `NativeSemParameterEditor.test.tsx` and `nativeSemParameterTableContracts.test.ts`: labelled form controls, Experimental gating, focus management, Escape cancellation, and non-`contenteditable` table behavior.

