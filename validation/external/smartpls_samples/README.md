# SmartPLS / Book Sample Validation Workspace

This folder is for optional external validation inputs used during QuickPLS
development. Do not commit downloaded SmartPLS project files, textbook datasets,
or extracted source data here unless the redistribution license explicitly allows
it.

## Why This Exists

SmartPLS and the PLS-SEM textbook ecosystem publish useful sample projects and
case-study datasets. They are valuable for local comparison of QuickPLS result
tables, report flow, diagram readability, and known-difference documentation.

QuickPLS should use these files as external validation material, not as bundled
application content.

## Local Folder Layout

Place downloaded files under ignored folders:

```text
validation/external/smartpls_samples/
  data/
    corporate_reputation/
    employee_retention/
    regression/
    nca/
    process/
  projects/
    corporate_reputation/
    employee_retention/
  downloads/
```

The validation inventory script only records whether the expected local files
are present. It does not download files automatically.

## Suggested Sources

- SmartPLS sample projects:
  https://smartpls.com/documentation/sample-projects/
- Corporate Reputation sample project:
  https://www.smartpls.com/documentation/sample-projects/corporate-reputation/
- Employee Retention sample project:
  https://smartpls.com/documentation/sample-projects/employee-retention/
- PLS-SEM book downloads and case studies:
  https://www.pls-sem.net/downloads/3rd-edition-a-primer-on-pls-sem-1/
- Mendeley teaching dataset for SmartPLS 3:
  https://data.mendeley.com/datasets/4tkph3mxp9/2

## Rules

- Keep downloaded material local and ignored by Git.
- Cite the source in any validation report or publication.
- Do not claim SmartPLS equivalence. Record matching settings, reference output,
  tolerances, and known differences.
- Use these files for validation and UI review only; QuickPLS runtime must not
  depend on them.
