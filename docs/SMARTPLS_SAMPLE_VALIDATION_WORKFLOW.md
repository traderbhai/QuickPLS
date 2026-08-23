# SmartPLS Sample Validation Workflow

QuickPLS can use public SmartPLS/book sample datasets as external validation
inputs. Such files normally stay outside the bundled application. The
product-owner-approved Corporate Reputation comparison is the one explicit
tracked exception: QuickPLS bundles only the prepared 31-indicator CSV and its
provenance, model, and independent comparison reference—not a SmartPLS project,
logo, application asset, or result screenshot. Release owners remain responsible
for confirming redistribution terms.

## Purpose

Use external SmartPLS/book samples to review:

- result-table coverage and terminology;
- diagram and report readability;
- export flow and report wording;
- reproducibility metadata;
- known differences between documented QuickPLS methods and other engines.

This workflow does not reverse-engineer SmartPLS and does not claim identical
behavior. It uses public documentation, public datasets, and independently
implemented QuickPLS methods.

## External Sources

- SmartPLS sample projects:
  https://smartpls.com/documentation/sample-projects/
- Corporate Reputation sample project:
  https://www.smartpls.com/documentation/sample-projects/corporate-reputation/
- Employee Retention sample project:
  https://smartpls.com/documentation/sample-projects/employee-retention/
- PLS-SEM book case-study downloads:
  https://www.pls-sem.net/downloads/3rd-edition-a-primer-on-pls-sem-1/
- Mendeley SmartPLS teaching dataset:
  https://data.mendeley.com/datasets/4tkph3mxp9/2

## Local Setup

Downloaded files should be placed under:

```text
validation/external/smartpls_samples/
```

Use the layout documented in:

```text
validation/external/smartpls_samples/README.md
```

These local files are ignored by Git.

## Inventory Commands

Record what is present locally without failing on missing files:

```powershell
npm run qpls:smartpls-samples:inventory
```

Require all expected external files to be present:

```powershell
npm run qpls:smartpls-samples:require-files
```

The report is written to:

```text
validation/results/smartpls_sample_inventory.json
```

## Validation Use

After a sample is available locally:

1. Import the dataset into QuickPLS.
2. Rebuild the documented model using QuickPLS' visual designer.
3. Run the comparable documented method and settings.
4. Export QuickPLS Results and Report outputs.
5. Compare against published case-study tables, SmartPLS sample-project output,
   or independent reference engines where available.
6. Record any convention differences in the known-differences documentation.

## Redistribution Policy

Do not commit downloaded SmartPLS projects, textbook datasets, extracted sample
CSVs, or derived repackaged data unless redistribution rights are clear. It is
acceptable to commit:

- inventory reports;
- comparison summaries;
- screenshots created for internal UI review when legally appropriate;
- documentation describing where developers can download the original files.

The approved Corporate Reputation product fixture is maintained at
`validation/fixtures/corporate_reputation_smartpls_mean_replaced_v1.csv`, with
its exact source and cleaning receipt beside it. Any replacement or additional
third-party dataset still requires a separate redistribution decision.
