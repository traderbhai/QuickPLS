# Wave 0 option-level parity acceptance matrix

This contract turns the frozen SmartPLS catalogue into a deterministic,
option-level acceptance backlog without claiming that uncaptured details are
complete. It is a validation and planning surface only. It does not enable a
method, promote evidence, or alter customer-facing product behavior.

## Authoritative files

- Contract: `validation/parity/wave0_option_level_acceptance_v1.json`
- JSON Schema: `validation/parity/wave0_option_level_acceptance_v1.schema.json`
- Validator and report generator: `validation/wave0_option_level_acceptance.py`
- Focused tests: `validation/test_wave0_option_level_acceptance.py`
- Deterministic report: `validation/results/wave0_option_level_acceptance_v1.report.json`

Capability Registry V2 remains the catalogue and option-cell source of truth.
The acceptance contract does not duplicate its 43 active rows, method names,
official URLs, or cell states. The validator expands those values from the
registry and checks the exact four-field cell identity:

```text
registry_schema_version + capability_id + cell_id + capability_version
```

`option_cells` are authoritative when present. Legacy row
`qualification_links` are accepted only as a migration fallback and, when both
exist, must be an exact identity match. Row-level coverage, evidence, and
surface values are informational projections only; they are never copied into
cells. This matters for rows such as PLS sample size and power, where the
post-hoc technical result and prospective Monte Carlo workflow have different
states.

Official-source inventories are stored as disjoint JSON fragments under
`validation/parity/fragments/` and listed by `override_fragments` in the main
matrix. The loader restricts paths to that directory, rejects unknown fragment
fields, binds every fragment by SHA-256 in the report, and merges only the
existing cell- and dimension-assessment wire shapes. This keeps method-family
research reviewable without making the main matrix another manual catalogue.

## Nine required acceptance dimensions

Every one of the 43 active catalogue rows is expanded across:

1. settings;
2. defaults;
3. model shapes;
4. data inputs;
5. preprocessing;
6. outputs;
7. charts;
8. failure conditions; and
9. workflows.

That produces 387 required row-dimension traces. An uncaptured dimension is not
omitted: it is explicitly `open`, resolves to every registry cell declared for
that row, and resolves to the row's recorded official SmartPLS URL. This is a
traceability result, not a completeness result.

A dimension may become `captured` only through an acceptance item containing:

- a stable item ID and description;
- one or more exact four-field cell identities belonging to the same official
  row;
- one or more official `https://smartpls.com/` references already recorded on
  that registry row; and
- one or more testable acceptance criteria.

Unknown cells, unrecorded sources, non-SmartPLS sources, empty criteria,
duplicate overrides, and missing dimensions fail closed.

## Independent cell states

Each exact active cell has four independent assessments:

- parity obligation: `active_parity` or `beyond_parity`;
- coverage: `full`, `partial`, or `absent`;
- evidence: `absent`, `engine_only`, `archive_qualified`,
  `native_qualified`, or `release_qualified`; and
- surface: `standard`, `labs`, or `internal`.

Coverage, evidence, and surface come only from authoritative registry
`option_cells`. The matrix records parity obligation separately because a
QuickPLS cell may be a beyond-parity experiment rather than an official parity
commitment. An evidence value never changes coverage. In particular,
`release_qualified` plus `partial` remains partial and cannot pass the
finalization gate.

The gate can become finalization-ready only when:

- all nine dimensions are captured for every active row;
- every active cell has an explicit parity role;
- every row maps to at least one `active_parity` cell;
- all cell assessments are captured; and
- each active-parity cell is `full`, `release_qualified`, and `standard`.

Beyond-parity cells remain visible in the contract but do not substitute for an
active parity obligation.

## Current baseline result

The checked-in baseline intentionally passes its structural contract and fails
its finalization gate:

| Measure | Result |
| --- | ---: |
| Catalogue rows | 45 |
| Active rows | 43 |
| Authoritative active option-cell identities | 45 |
| Active rows using legacy identity fallback | 0 |
| Required dimension traces | 387 |
| Complete trace links | 387 |
| Captured option-level dimensions | 387 |
| Explicitly open option-level dimensions | 0 |
| Active-parity cells identified | 44 |
| Beyond-parity cells identified | 1 |
| Cells with an open parity obligation | 0 |
| Finalization-ready | No |

This distinction is deliberate: all obligations are addressable, but their
option inventories have not been invented or silently inferred from generic
row references.

The official-source inventory now covers all 43 active catalogue rows across
all nine required dimensions. That closes the Wave-0 row/cell inventory; it
does not change any QuickPLS coverage, evidence, or surface state. QuickPLS's
prospective Monte Carlo power workflow is separately recorded as beyond
parity. Public SmartPLS pages that omit exact charts, desktop steps, failure
handling, or some defaults leave those details as explicit live-app evidence
obligations inside the captured acceptance items; they are not filled by
inference.

The CB-SEM/CFA foundation is also captured for ordinary CB-SEM, CFA,
CB-SEM bootstrapping, model comparison, multigroup analysis, measurement
invariance, moderation, and the PCA workflow used with CB-SEM. These acceptance
contracts preserve the current product truth: ordinary bounded ML cells are
partial, while the advanced CB-SEM cells remain absent until their scientific
and packaged qualification is complete.

The remaining analytical-family inventory covers GSCA, logistic regression,
path analysis and PROCESS, PROCESS bootstrapping, linear regression, and
regression bootstrapping. The assessment/prediction and advanced/group
fragments cover the active PLS assessment, prediction, comparison,
heterogeneity, interaction, mediation, nonlinear, higher-order, and
endogeneity rows. Their acceptance items distinguish facts stated on the
official pages from exact live-application details that still require captured
desktop fixtures.

Blindfolding and GoF remain the only explicit exclusions. They retain their
official references and exact cell identities in the report, but they are not
part of the 43-row active finalization matrix.

## Commands

Run the baseline contract and inspect its concise result:

```powershell
python validation\wave0_option_level_acceptance.py --summary
```

Verify the checked-in report is the exact deterministic projection:

```powershell
python validation\wave0_option_level_acceptance.py `
  --check-report validation\results\wave0_option_level_acceptance_v1.report.json `
  --summary
```

Regenerate the deterministic checked-in projection after a reviewed fragment
change:

```powershell
python validation\wave0_option_level_acceptance.py `
  --write-report validation\results\wave0_option_level_acceptance_v1.report.json `
  --summary
```

Run focused schema, mutation, trace, exclusion, and gate tests:

```powershell
python -m unittest validation.test_wave0_option_level_acceptance
```

The ordinary command exits successfully when the structural baseline is
valid. CI or a final product gate must request the stronger condition:

```powershell
python validation\wave0_option_level_acceptance.py --require-finalization-ready --summary
```

That command currently exits with code `2`, by design. Exit code `1` indicates
an invalid contract; exit code `2` indicates a valid contract that still has
open finalization obligations.
