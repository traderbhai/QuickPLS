# Canonical Result Comparison V2

`src/domain/canonicalResultComparisonV2.ts` defines the method-neutral
compatibility and side-by-side comparison contract for
`CanonicalResultDocumentV2`.

Two results can be compared only when they contain valid typed documents and
share the same:

- primary Capability Registry cell and capability version;
- complete document-level option-cell set and each table's option-cell set;
- dataset fingerprint;
- scientific model digest;
- analysis-recipe digest;
- table IDs, column IDs and column data types; and
- row IDs within each comparable table.

Every incompatibility is returned with a stable code, customer-facing title,
corrective explanation, related IDs, and separate technical details for
diagnostics. Historical text-only results remain readable, but the comparison
contract asks the user to recalculate them instead of treating formatted text
as scientific numbers.
Documents saved without explicit option-cell attribution also remain readable,
but comparison asks for recalculation instead of assuming that every table was
produced only by the primary cell. This is necessary for combined runs such as
PLS plus bootstrapping or post-hoc sample size, and PLSpredict plus CVPAT.

## Deterministic comparison model

Compatible tables, rows, and columns are aligned and sorted by stable ID rather
than array position. The comparison document records:

- left and right source document IDs;
- the shared analytical identity;
- the shared primary cell, complete option-cell set, and per-table attribution;
- typed number, text, boolean, and missing-value deltas;
- missing-value transitions, including reason changes and values becoming
  available or unavailable; and
- table, row, cell, and changed-cell counts suitable for GUI summaries and
  export readback.

Numeric deltas contain the left value, right value, signed change, absolute
change, and a changed flag. Text and boolean deltas retain both values. Missing
deltas retain display-independent cell snapshots and an explicit transition.
All generated IDs, counts, types, and finite values are validated before the
comparison is returned.

Display precision, formatted number or missing-value caches, chart display
settings, general presentation preferences, worker count, and run timing do
not affect compatibility or comparison output. Scientific chart rendering is
not recalculated by this module; table values remain the comparison source.

## Current integration boundary

This module does not replace `runComparison.ts`, the live Results workspace, or
any CSV, XLSX, HTML, SVG, or PNG exporter yet. Those consumers can migrate only
after current method runs produce qualified `CanonicalResultDocumentV2`
documents and comparison export/readback tests are in place.
