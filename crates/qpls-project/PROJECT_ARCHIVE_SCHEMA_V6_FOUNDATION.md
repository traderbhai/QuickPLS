# Project archive schema v6 foundation

`project_schema_v6.rs` defines the staged archive-v6 document and migration
plan. The production `.qpls` reader/writer remains schema v5 while GUI, CLI,
archive I/O, and estimator consumers are migrated together.

## Upgrade-copy contract

`plan_project_upgrade_to_v6` consumes a project already decoded by the existing
historical loader and produces an in-memory upgrade plan. Every plan requires:

- a source archive digest;
- distinct source and destination paths;
- `source_preservation = required`;
- `write_policy = new_archive_only`; and
- immutable historical-result envelopes.

`execute_project_upgrade_copy_v6` is the callable, non-UI executor for that
plan. It binds the supplied source and destination paths exactly to the lineage,
hashes the source before and during the operation, requires a nonexistent
destination, writes a sibling temporary file with `create_new`, flushes and
fsyncs it, validates its exact bytes, and atomically publishes it without
clobbering a racing writer. It then reopens and strictly validates the persisted
bytes. A guard removes the temporary file on every pre-publication failure and
removes the new destination after any failed post-write check. The source is
never opened for writing.

`write_project_document_v6_new` exposes the same new-destination-only atomic
writer for an already validated `ProjectArchiveDocumentV6`. It is deliberately
a standalone JSON foundation API. It does not alter or replace the production
schema-v5 `.qpls` ZIP reader/writer.

## Deterministic document I/O

`serialize_project_document_v6` recursively orders JSON object keys while
preserving meaningful vector order. Equal documents therefore produce the same
compact UTF-8 bytes. `deserialize_project_document_v6` rejects duplicate keys,
unknown fields, malformed references, and every model, recipe, lineage, or
historical-result digest mismatch before returning a document.

`inspect_project_document_bytes_v6` derives the schema version from the bytes
instead of trusting a caller-supplied version. Schemas above 6 yield only a
digest/count summary marked read-only; their scientific payload is not decoded.

## Model migration

The migration collects legacy models from both the project model collection and
historical recipes. It automatically converts only an unambiguous family:

- PLS-family recipes become composite SemModelV4 models;
- CB-SEM recipes become common-factor SemModelV4 models; and
- method-neutral, unused, or mixed-family models remain
  `legacy_estimand_unspecified`.

If a legacy shape cannot be represented by the basic converter, it remains
pending with an explicit conversion blocker. `confirm_project_legacy_estimand_v6`
applies the user's factor-versus-composite confirmation to a copied document and
updates every pending recipe reference for that model.

Legacy covariance drawings are supplied by the UI/archive adapter because the
old `ModelSpec` never treated them as scientific relations. They are validated
and stored as display-only annotations after conversion. Migration never turns
them into model covariances.

## Historical results

Each historical result is serialized without recalculation and wrapped with its
result ID, source result schema, and canonical JSON SHA-256 digest. Fields are
private to normal Rust callers. Document validation checks identity, schema,
digest, and uniqueness. Estimand confirmation clones these envelopes unchanged.

New typed results may be added separately through the dormant
`canonical_result_documents` attachment collection. Its strict V2 Rust wire
mirror, immutable envelope, canonical JSON digest, and project/run identity
checks are documented in `docs/CANONICAL_RESULT_ARCHIVE_V2.md`. Attaching one
returns a clone and preserves every historical-result envelope and migration
lineage exactly. Empty collections remain omitted so earlier schema-6
foundation documents reopen as an empty attachment set.

## Read behavior

- Schemas 1-5: the existing archive loader reads them; v6 reports that an
  upgraded copy is required.
- Schema 6: strict decoding and complete cross-reference/hash validation.
- Schemas above 6: checksum-compatible bytes are inspected only as a read-only
  count/digest summary, including a canonical-result attachment count. Unknown
  scientific payloads are not interpreted.
- Schema 0, duplicate JSON keys, unknown schema-6 fields, invalid references,
  and digest mismatches fail closed.

## Deliberate remaining work

- Integrate the schema-v6 document with ZIP manifest/checksum save/load.
- Connect the existing internal service boundary to an accessible Labs dialog
  with explicit destination selection; see
  `docs/INTERNAL_PROJECT_SCHEMA6_UPGRADE_ASSISTANT.md`.
- Add recipe-v4 compiler adapters method by method.
- Keep historical result rendering connected to its original payload and wire
  qualified result adapters to the new attachment only during the coordinated
  schema-6 production cutover.
