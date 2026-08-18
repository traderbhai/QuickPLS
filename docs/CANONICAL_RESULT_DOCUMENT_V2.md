# Canonical Result Document V2

`CanonicalResultDocumentV2` is the method-neutral result representation for
QuickPLS product finalization. It replaces the current pattern in which each UI
or export surface independently converts analytical payloads into formatted
string tables.

The contract is implemented in
`src/domain/canonicalResultDocumentV2.ts` and contains:

- stable section, table, row, column, chart, notice, exclusion, and footnote IDs;
- typed number, text, boolean, and explicitly missing cells;
- chart data separated from chart display preferences;
- model, dataset, recipe, capability-cell, method, and engine provenance;
- a deterministic, distinct document-level option-cell set plus explicit
  table and section attribution, while retaining one primary provenance cell;
- accessible chart and column descriptions;
- explicit cross-references and deterministic ordering;
- a full canonical JSON representation and a separate analytical projection.

The analytical projection retains scientific values, order, model/data/recipe
identity, method/engine versions, notices, and exclusions. It excludes cached
display strings, presentation defaults, chart styling, worker count, and run
timing. This permits display customization and worker-invariant comparison
without weakening the scientific identity.

Historical string-only tables migrate losslessly as text. The migration does
not guess that a formatted value is numeric and does not reinterpret an old
result under a newer method contract.

New comparable documents declare `capability_cells` at the document, section,
and table levels. Each list is sorted and distinct by the exact four-field cell
identity. The document set includes the primary provenance cell, every table
uses a nonempty subset, and every section includes the cells used by its
tables. Historical documents may omit all three levels together so they remain
readable, but a partial declaration is invalid and complete attribution is
required for comparison. Add-on cells are never inferred from the primary.

## Integration sequence

1. Method adapters build typed tables and chart series directly from the
   immutable analytical payload.
2. GUI navigation, comparison, accessibility labels, CSV, XLSX, HTML, diagram
   overlays, and report reopening consume this document.
3. Archive validation binds both the full document and its analytical
   projection.
4. Export readback compares table/column/row IDs and typed cells rather than
   rendered strings alone.
5. The existing `ResultTable` remains a compatibility view until every current
   method has a typed adapter.

No current method is considered migrated merely because its old table can be
wrapped. Full migration requires typed construction from the analytical result
and cross-format readback qualification.

## Runtime compatibility adapter

`src/native/nativeCanonicalResultDocumentV2.ts` now builds a strict V2
document for a completed runtime `AnalysisRun`. It preserves native table and
navigation order, attaches explicit Capability Registry V2 cells to each table
and section, derives
technical model and recipe fingerprints, collects run and table warnings, and
fails closed when the analytical payload or native table set is incoherent.
An exact recorded SHA-256 dataset fingerprint is retained. If an older run
contains only a short fingerprint identifier, the bridge binds that identifier
with a compatibility digest and adds an explicit notice; it does not claim to
have re-hashed unavailable raw observations.

For current runs, the adapter conservatively types columns only when every
non-missing rendered value has the same unambiguous number or boolean form.
For runs that predate technical provenance, it uses the text-only historical
migration and never infers numeric meaning from formatted values. Diagram
positions, diagram styling, display precision, missing-value labels, and chart
defaults do not enter the analytical projection.

Blindfolding has a stricter legacy boundary. If an older PLS payload still
contains its historical Q² table, a newly generated current canonical document
omits that table and records one explicit legacy exclusion. The text table
remains readable only through the historical fallback, which is not eligible
for typed side-by-side comparison. Blindfolding is therefore never added to a
current document's option-cell set or treated as an active parity capability.

This is a runtime compatibility bridge, not the final method-adapter layer.
The GUI, CLI, archive format, CSV, XLSX, HTML, SVG, and PNG exporters do not yet
all consume this document. Direct typed construction from each immutable
analytical payload and cross-format readback remain required before claiming
canonical export parity.

Semantic compatibility and deterministic typed deltas for two completed
documents are specified separately in
`docs/CANONICAL_RESULT_COMPARISON_V2.md`.

## Dormant schema-6 archive attachment

`qpls-project` now contains a strict Rust wire mirror and immutable schema-6
attachment for this document. The attachment binds document, run, project,
schema, and canonical JSON digest, while legacy schema-6 foundation documents
may omit the collection safely. This is an internal persistence contract only;
the production schema-5 save path and qualified exporters remain unchanged.
See `docs/CANONICAL_RESULT_ARCHIVE_V2.md`.
