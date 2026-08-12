# Project archive v5 implementation contract

This is the Wave 0 implementation contract for QuickPLS project archive v5. It is based on the current v4 code, not a greenfield format. Recipe schemas and archive schemas remain independent.

## Already retained from v4

- SHA-256 checksums for `project.json` and every Arrow dataset entry.
- Same-directory temporary saves, file synchronization, and a previous-generation `.qpls.bak`.
- Primary, autosave, and backup recovery with a reported recovery source.
- Result validation at append, save, and load, including atomic recipe/result rollback on rejected append.
- Read-only guards for most project mutations.

These foundations remain useful but do not by themselves satisfy the v5 migration, future-schema, transaction, or memory requirements.

## V5 wire and migration

- Set the archive version to 5 and dispatch versions exactly: v1-v2 legacy migration, v3 result migration, v4 deterministic migration, v5 current parsing, and a distinct future-schema read-only path.
- Keep the current top-level `project.json` collection shape unless a required new field is identified.
- Permit mixed historical recipe schemas in v5 archives. Loading must not upgrade recipe v1/v2 or reinterpret its results.
- New jobs and new result appends require recipe schema v3. A historical recipe becomes v3 only through an explicit copy/rerun operation.
- V4-to-v5 migration preserves dataset, model, recipe, result, layout order and all scientific values, identifiers, timestamps, fingerprints, versions, diagnostics, and payloads.
- Record the source archive version as non-persisted load information for migration reporting. The original v4 file becomes the automatic backup on the first explicit v5 save.

## Complete integrity validation

The v5 manifest declares the checksum algorithm (`sha256`; v4 defaults to it). Loading must reject malformed hashes, duplicate ZIP entry names, missing or unexpected entries, checksum-map mismatches, duplicate dataset IDs, or missing Arrow entries. Every non-manifest entry is verified before interpretation, including entries in future schemas.

Apply bounded entry-count and uncompressed-size limits before allocating buffers. Checksums detect corruption and tampering; they do not authenticate the archive.

Saving must avoid holding every Arrow dataset version in memory at once. Serialize and hash entries incrementally while writing the ZIP.

## Atomic save transaction

1. Create a unique same-directory temporary archive using create-new semantics.
2. Write, finish, and synchronize it.
3. Reopen and fully validate the temporary archive.
4. Rotate the valid current primary without first destroying the only known-good backup.
5. Promote the validated temporary archive.
6. Restore the original primary if promotion fails, and report a distinct rollback failure if restoration also fails.
7. Remove temporary and rotation files through a cleanup guard.

`save_project` returns the manifest that was actually persisted. Desktop state adopts that manifest after a successful save. Autosave cleanup failure is reported as a warning after the successful primary save; it must not leave the UI claiming that an already-persisted save failed.

## Identity-safe recovery

An autosave may replace a primary or backup only when its `project_id` matches the project being recovered. A foreign, corrupt, stale, or future-incompatible autosave is ignored or quarantined. When all candidates fail, preserve primary and recovery error details instead of returning only the first error.

## Future-schema read-only projects

Future archives are verified before best-effort decoding and represented separately from current writable projects. The application may expose compatible rows, group profiles, results, and exports, plus counts/notices for unsupported items. Raw scientific values remain available only for viewing/export and are never resaved.

Allowed operations are read-only row paging, group profiling, compatible export, and ephemeral dataset selection. Import, validation-fixture import, metadata edits, recoding, model/explorer edits, save, autosave, job start, and result commit reject without changing project state. A central writable-project guard replaces scattered checks and is rechecked after slow external file operations before commit.

## Historical-result immutability

Archive loading must not append warnings or otherwise mutate a stored `AnalysisResult` after checksum verification. Legacy notices are derived into snapshot compatibility metadata. One archival-version registry blocks all legacy-only method versions from new execution or append while keeping their original stored results readable.

## Recipe v3 execution bridge

Schema-v3 recipes persist typed `method_config` and no executable metadata keys. Until every estimator consumes the typed configuration directly, engine, Tauri preflight, runner, and project validation use `AnalysisRecipe::with_effective_metadata()` as the single compatibility projection. The projection is never persisted and rejects missing, mismatched, future, or conflicting configurations.

## Required verification

- New v5 round-trip with multiple datasets and mixed historical/current recipes.
- Frozen v4 migration twice with identical canonical scientific JSON and an untouched source archive.
- Pre/post migration equality of every historical result field.
- Missing, extra, duplicate, malformed, oversized, and tampered archive cases.
- Compatible and unknown future-schema read-only cases with all mutation commands rejected atomically.
- Same-project autosave recovery and foreign/stale/corrupt autosave rejection.
- Fault injection across write, finish, sync, temp validation, rotation, promotion, rollback, and autosave cleanup.
- Every typed method configuration reaches estimation and persistence without executable metadata being stored.
- All existing method-specific append/save/reopen/tamper suites remain green.
