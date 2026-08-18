# Canonical result export V2

`CanonicalResultDocumentV2` is the only analytical input to the new format-neutral export projection. The projection preserves:

- section, table, column, row, chart, series, notice, exclusion, and footnote order;
- stable IDs, typed cells, missing-value reasons, labels, precision policies, and chart data;
- exact model, data, recipe, capability-cell, method, engine, seed, worker, and run provenance;
- warnings, exclusions, footnotes, chart display settings, and source-run identity.

`canonicalResultSemanticExportV2.ts` builds deterministic JSON and rejects invalid source documents. Its parser reconstructs a canonical result, validates all cross-references and cell types, and rejects missing, unexpected, or reordered contract fields. Readback verification compares both the exact canonical document and its presentation-independent analytical projection.

`nativeCanonicalSemanticExportV2.ts` is the current activation boundary. It provides a readback-verified in-memory preview for engineering integration, but it does not replace the existing CSV, HTML, or XLSX writers and does not write files. Format-specific writers should consume this projection only after their own parse/readback qualification is complete.
