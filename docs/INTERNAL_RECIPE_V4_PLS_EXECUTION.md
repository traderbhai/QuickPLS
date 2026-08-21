# Internal Recipe-v4 PLS execution boundary

> Historical generic-command boundary. Version 2.50 General SEM Standard uses
> the newer exact-cell native job route documented in
> [Rank 0–3 SEM Upgrade Status](SEM_UPGRADE_RANKS_0_3_STATUS.md). The internal
> command names and restrictions below remain relevant only to their preserved
> compatibility surface.

QuickPLS now has an internal typed Tauri command for the first Recipe-v4 PLS
point-estimation slice:

`run_internal_labs_recipe_v4_pls_execution`

It also has the preferred cancellable lifecycle:

- `start_internal_labs_recipe_v4_pls_job`
- `internal_labs_recipe_v4_pls_job_status`
- `cancel_internal_labs_recipe_v4_pls_job`
- `internal_labs_recipe_v4_pls_job_result`
- `dismiss_internal_labs_recipe_v4_pls_job`

The command is deliberately absent from Standard Calculate. Its request must
declare `surface: internal_labs`, confirm that Experimental Labs is enabled,
select project-resident data, and carry all of the following exact identities:

- resident dataset ID and fingerprint;
- `AnalysisRecipeV4`;
- explicitly resolved `SemModelV4`;
- `pls_plan_v2` compiler target; and
- the `smartpls.pls_algorithm` / `qpls3.pls.algorithm` capability cell.

Raw rows remain in the native project state; they are not copied through IPC.
The command resolves the resident dataset, compiles a new immutable artifact,
revalidates it at the runner boundary, and dispatches the existing production
PLS point estimator.

Successful synchronous responses remain ephemeral and contain the deterministic
compilation and execution provenance receipt. A successful cancellable job
instead returns one native-built `InternalRecipeV4CompletedResultV1` containing
both the analytical result and a validated `CanonicalResultDocumentV2`.
The canonical document is validated against both the live qpls-core contract
and the independent schema-6 archive wire type before the job becomes
`completed`. Failures are typed with `stage`, `subject`,
`code`, `message`, and `correctiveAction`, covering access, capability, resident
data, compilation, projection, estimation, and integrity boundaries.

The job lifecycle stores a scientific result only after successful completion.
Cancellation is checked inside the production estimator and again while the
exact active project and resident dataset are locked for publication. A
cancelled, failed, stale-project, or stale-dataset job retains no partial
result. The snapshot records queued, started, and terminal UTC timestamps at
the lifecycle boundary so later canonical provenance need not infer them.
Fetching the completed result consumes the terminal job entry. The TypeScript
`persistInternalLabsRecipeV4PlsJobResultToSchema6` service then passes that exact
native-built document to the digest-bound, atomic schema-6 append command; it
does not reconstruct tables in TypeScript. The matching read service reopens
the exact schema-6 attachments under the caller's expected source digest for
saved-report comparison. The older synchronous command
remains an internal compatibility seam for focused tests; new integration work
should use the job lifecycle.

This slice does not:

- replace the existing Recipe-v3/schema-v5 Calculate and job routes;
- append Recipe-v4 recipes to the active schema-5 project;
- activate archive schema 6 in Standard workflows;
- expose a Standard capability;
- add bootstrap, permutation, assessment, report, or CB-SEM execution; or
- add a user-facing invocation from the current application shell.

The matching TypeScript API is in `projectService.ts`. The start, status,
cancel, result, persistence, and dismiss functions share the same request and
typed snapshot contracts. The request uses literal `internal_labs`, `true`, and
`project_resident` gates so normal frontend code cannot accidentally present it
as a Standard analysis.
