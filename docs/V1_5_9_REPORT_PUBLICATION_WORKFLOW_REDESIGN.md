# QuickPLS v1.5.9 Report Publication Workflow Redesign

Status: validated.

This frontend-only milestone completes the v1.5.7 Report remediation set. It improves the publication workflow, export controls, publication preview, and comparison handoff without changing statistical engines, result schemas, formulas, project format, validation tolerances, or numerical fingerprints.

## Completed Changes

- Replaced the passive preset strip with selectable report preset cards for thesis appendix, journal figure, journal tables, presentation, and full reproducibility report.
- Grouped publication setup into clear sections: Figure, Statistics, Tables, and Notes.
- Added explicit export actions for CSV, HTML, XLSX, Print/PDF, and Model diagram SVG, each with nearby disabled reasons.
- Kept native PDF as a documented browser print path, not a native-audited export claim.
- Improved the publication preview frame with layout-risk guidance and a visible current-canvas versus tidy-publication status.
- Moved Run comparison out of the Report page and into the Results comparison workspace through a direct handoff button.
- Added SVG label protections for SmartPLS-like diagrams so construct labels and structural path labels are less likely to collide.

## Evidence

- `validation/v159_report_publication_smoke.mjs`
- `validation/v159_report_publication_audit.py`
- `validation/results/v159_report_publication_smoke.json`
- `validation/results/v159_report_publication_audit.json`

## Boundary

The milestone changes Report workspace presentation and export workflow only. Existing saved runs and all numerical payloads remain unchanged.
