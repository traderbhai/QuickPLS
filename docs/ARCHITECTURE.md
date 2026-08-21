# QuickPLS Foundation Architecture

QuickPLS is local-first. The React desktop surface owns interaction and visualization; the Tauri boundary exposes a small typed command set; Rust owns project validation, numerical work, reproducibility, and exports.

## Boundaries

- `src/`: desktop workspace, visual model editor, data preview, capability catalog, runs, and reports.
- `crates/qpls-core`: versioned contracts, method gates, deterministic statistical primitives, and model validation.
- `crates/qpls-data`: typed research-data import, metadata, Arrow IPC, previews, and SHA-256 fingerprints.
- `crates/qpls-project`: `.qpls` manifests, checksums, atomic writes, recovery, and version handling.
- `crates/qpls-cli`: headless access to the same contracts and validation behavior.
- `src-tauri`: desktop packaging and IPC only. Statistical logic does not live here.

## Project Container

The `.qpls` container is a ZIP archive with a versioned manifest, Arrow data, JSON model and recipe contracts, and diagram layouts. Every payload has a SHA-256 checksum. Saves use a temporary archive and atomic replacement while retaining the preceding valid archive as `.qpls.bak`. Immutable result records and attachments are reserved until their schemas exist.

## Scientific Gate

Evidence maturity progresses only after the method contract, deterministic
kernel checks, independent references, simulations where applicable,
metamorphic and adversarial checks, persistence/export checks, and packaged
desktop checks pass. Standard means the exact documented QuickPLS option is
release-qualified: full-coverage options and explicitly bounded partial-coverage
options can both be Supported, while the coverage axis remains truthful about
unimplemented breadth. The exact rules are encoded by Capability Registry V2
and QualificationSpec V2 rather than inferred from a family-level status label.

## Product-finalization architecture (V2 foundations)

The product-finalization program separates three questions that the original
catalogue/status model conflated:

1. **Coverage:** does QuickPLS implement the complete documented research
   workflow represented by the SmartPLS catalogue row?
2. **Evidence:** how far has that exact option cell progressed through the
   QuickPLS qualification ladder?
3. **Customer surface:** is the cell Standard, Experimental Labs, Legacy, or
   Internal?

The source contract is
`validation/capabilities/capability_registry_v2.json`. A cell is eligible for
Standard only when evidence is `release_qualified`, its surface is `standard`,
and it has either `full` coverage or `partial` coverage with a nonempty scope
statement. Release evidence for a narrower implementation never upgrades its
partial coverage. The schema-v1 competitor catalogue is now a checked,
deterministic compatibility projection rather than an independent authority.
The Rust core embeds and independently validates the same option-cell registry;
`qpls methods` exposes its source digest and exact CLI availability projection
instead of the legacy one-dimensional execution-status list.

`SemModelV4` in `crates/qpls-core/src/sem_model_v4.rs` is the new scientific
model IR. Scientific variables, relations, parameters, constraints, derived
terms, groups, and data bindings are separated from annotations and canvas
presentation. Its deterministic scientific identity excludes presentation-only
objects. The immutable PLS compiler still fails closed on unsupported input.
The CB-SEM V2 compiler represents the complete valid scientific parameter table,
while a separate typed capability validator gates the existing bounded ML
estimator without deleting unsupported model intent. Runtime
project/runner/estimator cutover is still staged. Project schema 6 records a
truthful `new_project` or source-bound `upgraded_copy` origin, stores incomplete
authoring-integrity-checked models as non-executable drafts, and preserves
schema-1-through-3 recipes as immutable historical envelopes instead of
fabricating Recipe 4. Ready Recipe-4 models remain a separate current-authoring
lane. The staged schema-6 path has strict archive I/O, compiler receipts, an
internal PLS execution bridge, and source-digest-bound atomic canonical-result
append with rollback and cancellation-before-commit. The
internal execution bridge now has a typed cancellable job lifecycle, shares the
desktop worker/job limit, rechecks the exact project and resident dataset before
publishing completion, retains no partial result, and produces a native-built
canonical document that is pre-validated against the schema-6 archive wire
contract. The internal project service can append that exact document through
the atomic digest-bound schema-6 writer. The legacy schema-5 live route remains
active until schema-6 save/reopen is wired into the product shell and the full
method, migration, export, and installed-app qualification matrix is complete.

`qpls-project` also has a strict read-only schema-6 ZIP codec. It validates the
exact manifest/checksum/entry set, manifest-to-document identity, canonical
schema-6 JSON, Arrow payloads, dataset fingerprints, and resident lineage before
returning a typed document and datasets. The legacy loader rejects schema 6
explicitly instead of projecting it as a partial future schema-5 project.
An Internal/Labs upgrade-copy writer can now create a strictly validated
schema-6 ZIP at a new path while preserving and rechecking the source. It
re-derives the source-bound plan, copies resident Arrow bytes unchanged, uses
no-clobber publication, and performs identity-aware cleanup on cancellation or
failure. A separate Internal/Labs native bridge can return a typed read-only
snapshot through the strict reader, and the Settings Labs surface can hold it
in an isolated read-only session that never enters the Standard/schema-5 store.
Schema-6 document APIs also
support clone-only draft insertion, compare-and-swap draft replacement, and
exact promotion while blocking referenced models and preserving all non-model
lanes. Their native/TypeScript bridge is explicitly in-memory and
`not_persisted`; the detached Labs editor consumes that bridge with strict
SemModelV4 JSON and exact draft CAS identities, and closing the session discards
the changes. `PROJECT_ARCHIVE_VERSION` remains 5: these
paths do not activate a schema-6 project and do not enable live save, autosave,
backup, recovery, new-project creation, or Standard open cutover.

Schema-6 in-place save and generation save-as are not implemented. A
provisional generation writer was removed after path-identity race review, so
no unsafe schema-6 save API is exposed; only the source-preserving upgrade-copy
writer and existing digest-bound canonical-result append remain available to
Internal/Labs code.
The minimum retry boundary is a Windows-native relative child create beneath
pinned ancestor/parent directory handles; path rechecks alone cannot close an
ancestor-junction retarget window.

The model workspace's Advanced Parameter Table is projected by
`src/domain/semParameterTableV4.ts` through the authored workbench-to-`SemModelV4`
adapter. It opens on demand from Canvas, CB-SEM setup, or a corrective preflight
action instead of occupying a permanent document tab. It lists scientific
variables, relations, parameters, constraints, derived terms, and groups
separately from presentation-only objects and keeps stable links back to canvas
sources. If a legacy construct estimand or covariance use has not been chosen,
projection fails closed and shows typed corrective diagnostics instead of
inventing parameter rows. Accepted scientific edits create a versioned revision;
opening the table alone does not activate an estimator path.

The native calculation coordinator projects resident model features into the
unchanged 18-method catalogue. PLS Algorithm, Bootstrapping, and CB-SEM resolve
their exact Registry cells from topology plus requested inference. Former General
SEM and Exact CB-SEM workspaces remain compatibility adapters for historical
payloads and are not permanent navigation surfaces.

`CanonicalResultDocumentV2` in
`src/domain/canonicalResultDocumentV2.ts` is the typed result target for the GUI,
comparison, archive, accessibility, and all export formats. The current
string-table system remains a compatibility view until each method constructs
typed cells and chart data directly from its immutable analytical payload.

Customer visibility policy lives in
`src/domain/capabilitySurfaceV2.ts`; Experimental Labs is disabled by default.
The live Method Details dialog resolves the selected recipe to exact option
cells and renders the common nine-section explanation documented in
`docs/METHOD_DETAILS_V2.md`. Combined methods explain their base estimator and
each requested add-on independently.
The migration inventory and final copy gate live in
`validation/customer_language_contract.py`. Internal evidence and promotion
terms remain available in validation artifacts and technical diagnostics, but
are being removed from normal setup, results, and reports.

Packaged Windows checks are defined by
`validation/capabilities/packaged_windows_acceptance_v2.manifest.json`.
Supervisors, adapters, receipts, and method identities consume its exact check
IDs and hash; no fixed aggregate count is an authority.
