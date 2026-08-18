# Internal project schema-6 upgrade assistant

The schema-6 upgrade assistant is an Experimental Labs service boundary. It is
not connected to the Standard project open/save flow and does not change the
live schema-5 archive format.

Its four typed operations are:

1. `inspect_internal_project_upgrade_v6` reads the selected source and returns
   its exact SHA-256 identity, schema access, counts, and future-version
   read-only summary.
2. `plan_internal_project_upgrade_v6` requires the inspection digest, a new
   unused destination, and an explicit Composite or Common factor choice for
   each ambiguous legacy model. Planning performs no write.
3. `execute_internal_project_upgrade_v6` consumes the exact in-memory plan ID
   and plan digest. The executor re-derives the authoritative source migration,
   creates a strict schema-6 ZIP at the unused destination, validates both the
   temporary and persisted archive through the strict reader, and rechecks the
   source identity before returning a receipt.
4. `cancel_internal_project_upgrade_v6` cancels before commit and removes only
   writer-owned files. If commit has already won the synchronized arbitration,
   it truthfully reports `cancelled: false` and `destinationWritten: true`.

The TypeScript UI-ready state model is in
`src/domain/internalProjectUpgradeV6.ts`; the native boundary is in
`src-tauri/src/project_upgrade_assistant.rs`; and the frontend service calls
are in `src/services/projectService.ts`.

Safety rules are fail-closed:

- source and destination paths must be absolute;
- the destination must not exist during planning or execution;
- the source and destination cannot be the same path;
- a changed source digest requires a new inspection and plan;
- a changed plan digest cannot execute or cancel another plan;
- future project schemas are summarized as read-only and cannot be planned;
- a new schema-6 project has an explicit `new_project` origin, while an upgrade
  has a source-bound `upgraded_copy` origin; legacy staged documents with the
  former top-level lineage field are read compatibly but normalize to the new
  origin wire;
- incomplete `sem_model_v4_draft` records are document-bound but cannot satisfy
  Recipe 4, compilation, or execution;
- schema-1-through-3 recipes remain immutable historical recipe envelopes;
  migration does not manufacture Recipe 4 records or reuse historical IDs;
- historical results are copied through immutable digest envelopes and are not
  recalculated, and their source-recipe relationship is explicitly bound to an
  exact historical recipe digest or marked `unbound_legacy`;
- execution re-derives every source-derived lane and accepts only exact legacy
  snapshots, permitted explicit estimand conversions, or an exact draft wrapper;
- Arrow dataset bytes are streamed unchanged and the exact ZIP manifest,
  checksums, document, resident data, and lineage are reopened before commit;
- failed publication removes only writer-owned temporary/destination files;
  a racing replacement is preserved and reported with a typed diagnostic;
- cancellation before execution performs no write.

The assistant also distinguishes a current schema-6 ZIP as
`current_v6_archive`: it is strictly inspected and exposed only as a read-only,
non-upgradeable archive. The separate Internal/Labs read service returns a
typed snapshot. The Settings Labs surface may retain that snapshot in an
isolated read-only session with explicit open and close, but it never enters the
Standard project store or reaches edit/run/save/autosave/recovery.

The Settings Labs surface contains an accessible read-only inspector for this
service. It can browse or inspect an exact path and show project identity and
raw contract counts, but it does not expose the upgrade executor, activate the
archive, edit models, or save. There is still no Standard navigation entry.

Schema-6 in-place save and generation save-as remain outside this boundary. A
provisional generation-save implementation and a later final-handle experiment
were both removed after path-identity race review, and no unsafe save API is
exposed. A retry requires pinned ancestor/parent directory handles and a native
relative child create so an ancestor junction cannot retarget the commit path.
