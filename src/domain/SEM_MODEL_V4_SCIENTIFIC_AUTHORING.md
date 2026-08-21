# SemModelV4 scientific authoring workflow

Status: Connected General SEM model-authority workflow. Exact estimator cells and
the Capability Registry still determine whether calculation is Standard, Labs,
or blocked.

## Purpose

The workflow removes two legacy ambiguities before the live graph can be compiled safely:

1. Every ordinary construct is confirmed as either a `composite` or a `common_factor`.
2. Every drawn covariance is classified as exactly one of:
   - model covariance between the two construct variables;
   - residual/error covariance between one observed indicator residual from each connected construct;
   - disturbance covariance between two endogenous construct disturbances;
   - presentation-only canvas annotation.

The pure contract is implemented in `semModelV4ScientificAuthoring.ts`. The
native controls are implemented in `NativeSemScientificAuthoring.tsx`.
Construct representation and marker controls are available in Expert mode when
the active General SEM workflow has a Registry-authorized CB-SEM cell; advanced
covariance semantics remain Labs-gated. The SemModelV4 Parameter Table shows the
same scientific parameters, presentation objects, and remaining decisions used
by native compilation.

## Safety behavior

- Missing legacy construct intent remains `legacy_estimand_unspecified`; measurement mode is never used to guess factor versus composite.
- Missing legacy covariance intent remains unclassified; it is never upgraded automatically.
- Residual endpoints must be indicators owned by the two constructs connected by the drawn covariance.
- Disturbance covariance requires both connected constructs to have an incoming regression path.
- Presentation-only covariance is emitted as an annotation, not a scientific relation.
- Confirmations preserve the existing node or edge ID and create one undoable store change.
- A confirmed construct decision can be cleared explicitly when a user must return to a legacy recipe; clearing records unresolved intent and never guesses a replacement meaning.
- The exact marker indicator and exact residual/disturbance endpoint IDs are serialized in the existing versioned `semModelV4` authoring metadata and survive project presentation reopen.
- Recipe-v4 execution fails closed before it can filter a scientific covariance or ignore an explicit factor/composite choice.

## Deliberate live gaps

- The strict resident SemModelV4 and Parameter Table are the executable
  scientific source for connected General SEM PLS and bounded CB-SEM cells.
- Standard Calculate opens the same General SEM estimator/settings workspace;
  unsupported scientific relations remain visible and block before estimation.
- Residual/error covariance is currently authored through construct-to-construct canvas arcs with exact indicator endpoint selectors. Direct indicator-to-indicator drawing remains future editor work.
- Free/fixed parameter rows, starts, compatible bounds, equality labels, and
  factor identification are represented in the Parameter Table. Means,
  intercepts, thresholds, group-specific parameters, feedback, robust/ordinal
  estimators, and unsupported constraint objects remain outside the bounded
  Version 2.50 CB-SEM cells.
