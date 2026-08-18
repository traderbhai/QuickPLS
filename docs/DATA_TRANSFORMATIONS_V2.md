# Dataset Transformations V2 foundation

`DatasetTransformationV2` is the immutable data-preparation kernel for project
schema 6. It supports the active parity operations needed by the model editor:

- reverse-scale derivation;
- exact recoding;
- two-variable arithmetic, multiplication, and division;
- row-wise sums and means with explicit missing-data policy;
- dummy-variable generation; and
- exact-value or numeric-range group derivation.

Every operation creates a new dataset snapshot. The source rows are never
changed. A mutation records the complete normalized specification, source and
output fingerprints, exact input/output columns, row and missing counts, engine
version, and creation time. Preview and execution use the same evaluator.

The kernel fails before mutation when rows are not resident, a target would
overwrite a column, source data are incompatible, grouping rules overlap,
values are left unmatched under an error policy, division by zero occurs, or a
calculation is non-finite. Missing values remain missing unless the selected
operation explicitly specifies available-case aggregation, zero-valued dummy
handling, or an unmapped recode policy.

The installed Windows Data workspace now exposes the same contract through
**Derive variable**. The dialog supports every operation above, requires a
successful non-destructive preview before its explicit **Create Version**
action, displays typed row/setup issues, and records the returned lineage. A
successful commit selects the new immutable dataset version and its derived
variable; the version navigator keeps the source snapshot available.

The UI blocks read-only projects, matrix inputs, incomplete/nonresident data,
browser-only sessions, and active calculations. Full-data residency is
authoritative in the native project process: the React snapshot intentionally
contains only a bounded row preview for large datasets. The remaining product
work is broader transformation parity (for example additional documented
functions), packaged pointer/keyboard acceptance, and final capability
qualification; the live workflow does not by itself establish parity or
release qualification.
