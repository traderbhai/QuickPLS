# Canonical result archive attachment V2

Schema-6 project documents can now carry dormant
`CanonicalResultDocumentV2` attachments under the wire field
`canonical_result_documents`. This foundation does not change the production
schema-5 `.qpls` save path or any qualified export.

Each attachment has one exact envelope:

```text
document_id
run_id
document_schema_version = 2
canonical_document
canonical_document_sha256
immutable = true
```

The Rust payload in
`crates/qpls-project/src/canonical_result_document_v2.rs` mirrors the wire names
and tagged cell/chart types in
`src/domain/canonicalResultDocumentV2.ts`. Unknown fields, invalid tagged
variants, non-finite numbers, unstable or duplicate IDs, mismatched cell types,
dangling references, incomplete capability-cell attribution, invalid
provenance, and invalid presentation defaults fail closed.

The attachment digest covers recursively key-ordered JSON while preserving all
array order. The envelope identity, schema, run, project, immutable flag, and
digest are rechecked whenever a schema-6 document is serialized or reopened.
Duplicate document IDs and duplicate run attachments are rejected.

`attach_canonical_result_document_v2_v6` returns a validated clone, orders
attachments by document ID, and never edits an existing historical result or
upgrade lineage. Existing schema-6 foundation documents may omit
`canonical_result_documents`; omission reads as an empty attachment set and is
kept omitted when empty. Future archive schemas expose only a read-only count
and whole-document digest, without interpreting canonical payloads.

`append_canonical_result_document_v2_file_v6` is the durable standalone-file
append boundary. A caller must provide the SHA-256 of the exact schema-6 source
it inspected. The writer acquires a per-archive create-new lock, validates and
fsyncs a same-directory replacement, rechecks the source immediately before
commit, performs an atomic replacement while retaining the previous file as a
rollback copy, and removes that copy only after exact-byte, digest, strict
schema, and semantic readback all succeed. Cancellation is accepted only before
commit and leaves the archive byte-for-byte unchanged. Stale source digests,
concurrent writers, duplicate runs, symlinks, tampering, and failed readback all
fail closed.

The first internal connection now exists for Recipe-v4 PLS point estimation:
the cancellable native worker builds and validates the canonical document from
the same immutable analytical result, and the project service can append that
exact document through the source-digest-bound schema-6 writer. Cancelled,
failed, or stale-project jobs expose no document. A read-only internal command
can then reopen every attachment from an exact expected source digest, recheck
that the file did not change during validation, and return the strict canonical
documents to the comparison layer without reconstructing scientific tables.
The TypeScript IPC boundary treats that response as untrusted JSON: it accepts
only the exact outcome, snapshot, diagnostic, and attachment keys; revalidates
every `CanonicalResultDocumentV2`; binds project, document, and run identities;
verifies counts, ordering, uniqueness, immutable flags, and lowercase SHA-256
fields. Each attachment also carries the exact UTF-8 canonical JSON emitted by
`qpls_project::canonical_result_document_v2_json`. TypeScript hashes those raw
bytes rather than reserializing JavaScript numbers, strictly rejects unknown
JSON fields, validates the parsed document, and then proves that it is
semantically identical to the separately returned typed document. This keeps
integral floating-point values such as Rust's `1.0` digest-stable even though a
JavaScript object represents the same value as `1`. Missing Web Crypto support
fails closed. A schema-6 project with zero canonical attachments remains a
valid historical project and reopens as an empty document set.
Standard activation remains separate work: qualified method adapters, live schema-6 project cutover,
schema-6 ZIP entries/checksums, saved-report reopening, and installed-app
qualification must all close before this internal opt-in path replaces the
schema-5 workflow.
