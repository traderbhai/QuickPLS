# QuickPLS v2.12.0 Real Dataset Review Protocol

Status: validated after the v2.12.0 gate passes.

This frontend/product milestone adds a repeatable protocol for reviewing real researcher datasets without storing private data in the repository. It is intentionally not a statistical validation milestone and does not change engines, formulas, result schemas, project archive format, or numerical fingerprints.

## Purpose

Real user datasets are necessary for launch-quality UI/UX judgment, but they must not be treated like committed test fixtures. This protocol separates three concerns:

- Product/UI issues: layout, wording, navigation, method guidance, table readability, export friction.
- Data-handling notes: import format, metadata clarity, missing-value handling, variable naming, group/weight/outcome setup.
- Statistical evidence gaps: anything that would require formal reference-output validation before a method claim changes.

Only product/UI and anonymized workflow notes belong in routine v2 review artifacts. Statistical evidence gaps are logged separately and must not be used to promote method scope by implication.

## No-Private-Data Rule

Do not commit any of the following:

- raw private datasets;
- private `.qpls` projects;
- value-revealing screenshots that disclose private row values, variable names, organization names, participant identifiers, or research-sensitive constructs;
- exported reports/tables from private datasets;
- absolute paths that identify a user's private folder structure beyond a local, non-actionable alias.

Permitted artifacts:

- anonymized issue-register entries using dataset aliases such as `private-dataset-a`;
- UI screenshots after values and labels are redacted;
- synthetic reproductions made from generated data;
- bundled/demo fixture evidence;
- checklist outcomes that describe the workflow without private values.

## Manual Review Checklist

Use the template at `validation/templates/real_dataset_issue_register_template.json` for each private dataset review.

### 1. Intake

- Assign a dataset alias.
- Record source type only: CSV, XLSX, SAV, covariance, correlation, or other.
- Record whether raw data may be temporarily opened locally.
- Confirm that no private file will be copied into the repository.

### 2. Data Workspace

- Can the user import the file through the native desktop flow?
- Are row/variable/missing/type summaries clear?
- Are scale type, label, missing marker, and metadata controls understandable?
- Does prefix detection help, mislead, or need override controls?
- Are any warnings too loud, too vague, or missing?

### 3. Setup Workspace

- Does the method guidance show only sensible recommendations?
- Are unavailable methods explained with exact next actions?
- Are group column, weight column, binary outcome, NCA X/Y, and PCA variable requirements clear when relevant?
- Are experimental or unsupported scopes clearly separated from validated scope?

### 4. Results Workspace

- Are top findings useful and not repetitive?
- Are measurement, structural, validity, mediation, inference, prediction, and group tables readable?
- Are table overflows handled with sticky headers/first columns and clear controls?
- Does interpretation use actual values without overclaiming?
- Does selecting rows help the user understand where to look in the model?

### 5. Report Workspace

- Is the four-step export flow clear?
- Does the SVG preview match expectations?
- Are CSV, HTML, XLSX, SVG, and print/PDF path controls clearly enabled or disabled?
- Is the reviewer/reproducibility pack understandable?
- Are export destinations and success feedback clear?

### 6. Follow-Up Classification

Classify every note as one of:

- `ui_issue`: visual, copy, spacing, navigation, table, or interaction problem.
- `workflow_gap`: user cannot discover what to do next.
- `method_guidance_gap`: applicability/status/reason is unclear.
- `export_gap`: report or file output flow is unclear.
- `statistical_evidence_gap`: requires method validation or reference-output work.
- `cannot_reproduce_without_private_data`: requires a synthetic reproduction before implementation.

## Automated Gate Inputs

Automated v2.12 checks use only:

- bundled/demo fixtures already in the repository;
- generated synthetic metadata;
- static docs/templates;
- smoke/audit JSON files under `validation/results/`.

Private dataset review notes remain manual inputs and must be anonymized before they influence a development milestone.

## Evidence

The v2.12 gate writes:

- `validation/results/v2120_real_dataset_protocol_smoke.json`
- `validation/results/v2120_real_dataset_protocol_audit.json`

These files prove that the protocol, template, registry, scripts, active tracker, and no-private-data rules are in place. They do not prove statistical accuracy for any private dataset.
