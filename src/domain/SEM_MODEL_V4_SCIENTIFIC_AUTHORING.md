# SemModelV4 scientific authoring workflow

Status: Internal/Experimental editor contract. It is not a Standard calculation surface and does not activate a SemModelV4 estimator.

## Purpose

The workflow removes two legacy ambiguities before the live graph can be compiled safely:

1. Every ordinary construct is confirmed as either a `composite` or a `common_factor`.
2. Every drawn covariance is classified as exactly one of:
   - model covariance between the two construct variables;
   - residual/error covariance between one observed indicator residual from each connected construct;
   - disturbance covariance between two endogenous construct disturbances;
   - presentation-only canvas annotation.

The pure contract is implemented in `semModelV4ScientificAuthoring.ts`. The native controls are implemented in `NativeSemScientificAuthoring.tsx` and are mounted only when Experimental Labs is enabled. The SemModelV4 Parameter Table uses the existing live-workbench adapter to show the resulting scientific parameters, presentation objects, and remaining decisions.

## Safety behavior

- Missing legacy construct intent remains `legacy_estimand_unspecified`; measurement mode is never used to guess factor versus composite.
- Missing legacy covariance intent remains unclassified; it is never upgraded automatically.
- Residual endpoints must be indicators owned by the two constructs connected by the drawn covariance.
- Disturbance covariance requires both connected constructs to have an incoming regression path.
- Presentation-only covariance is emitted as an annotation, not a scientific relation.
- Confirmations preserve the existing node or edge ID and create one undoable store change.
- A confirmed construct decision can be cleared explicitly when a user must return to a legacy recipe; clearing records unresolved intent and never guesses a replacement meaning.
- The exact marker indicator and exact residual/disturbance endpoint IDs are serialized in the existing versioned `semModelV4` authoring metadata and survive project presentation reopen.
- Current recipe-v3 execution continues to fail closed before it can filter a scientific covariance or ignore an explicit factor/composite choice.

## Deliberate live gaps

- No recipe-v4 runner consumes these choices yet.
- The canonical v3 model remains the executable scientific source; SemModelV4 authoring metadata is dormant presentation state during current project reconciliation.
- Standard Calculate does not expose this authoring workflow or claim support for its scientific relations.
- Residual/error covariance is currently authored through construct-to-construct canvas arcs with exact indicator endpoint selectors. Direct indicator-to-indicator drawing remains future editor work.
- Free/fixed parameter specifications, starts, bounds, equality labels, factor identification, observed intercepts, latent means, and ordinal thresholds are now authored through the Experimental Parameter Table; see `SEM_MODEL_V4_PARAMETER_AUTHORING.md`.
- Observed structural-variable drawing, group-specific parameters, Special Assumptions toggles, and all recipe-v4 execution remain future work.
