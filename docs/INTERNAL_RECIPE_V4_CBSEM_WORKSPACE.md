# Internal CB-SEM Recipe-v4 workspace

> Historical V2/matrix-input Labs workspace. The current bounded Rank 3
> schema-6 General SEM ML and recursive-bootstrap workflow is scoped Standard
> and is documented in [CB-SEM General SEM V3](methods/CBSEM_GENERAL_SEM_V3.md).
> The matrix-input and broader Labs behavior below is not silently promoted.

Status date: 15 August 2026

QuickPLS exposes the cancellable Recipe-v4 CB-SEM job only in the model
workspace after Experimental Labs is enabled. It is not a Standard Calculate
method and does not change the legacy Recipe-v3 path.

## Workflow

1. Confirm every legacy construct as a Composite or Common factor in the
   Experimental Parameter Table. This CB-SEM workspace requires common factors.
2. Select an exact resident raw, covariance, or correlation dataset.
3. For matrix input, declare whether the source uses `n - 1` or `n`. Correlation
   input additionally requires one positive study standard deviation for every
   modeled observed variable.
4. Review the access, project, scientific-model, resident-dataset, input, and
   recipe preflight layers.
5. Start the native job. The UI polls the existing native lifecycle, displays
   real progress and typed failures, and permits cancellation. A canonical
   document is requested only after a native `completed` terminal state.
6. Inspect an existing standalone schema-6 project. Inspection supplies the
   exact source digest and project identity.
7. Explicitly append the native `CanonicalResultDocumentV2`, then reopen the
   updated archive under its new digest. TypeScript renders the returned typed
   cells directly and never rebuilds analytical tables.

The workspace captures project path, resident dataset ID and fingerprint, and
the scientific SemModelV4 projection when the job starts. A change to any of
those identities requests cancellation. Native code independently rechecks the
active project and dataset before publishing a result, so cancelled, failed,
or stale jobs expose no partial result.

## Current boundary

- Internal/Labs CB-SEM ML only.
- Project-resident data only; raw rows and matrix cells do not cross IPC.
- Raw continuous data, covariance matrices, and scaled correlation matrices.
- Single-group execution and the model shapes accepted by the native
  `cbsem_plan_v2` compiler.
- Reciprocal structural feedback and parameter-level group overrides are
  blocked in layered preflight with a corrective action.
- The native compilation receipt is displayed after successful completion.
- Schema-6 append is explicit, atomic, source-digest-bound, and project-ID-bound.

The SmartPLS-style CB-SEM special-assumption switches are not exposed or
inferred by this workspace. They need an explicit Recipe-v4/native compiler
contract before the UI can offer them; until then, no implied covariance or
fixed indicator variance is silently added.

This wiring is not scientific qualification. Installed-application pointer and
screen-reader runs, semantic CSV/XLSX/HTML/SVG/PNG export readback, complete
external-oracle and simulation evidence, performance/soak coverage, schema-6
Standard project cutover, and Capability Registry promotion remain separate
gates.
