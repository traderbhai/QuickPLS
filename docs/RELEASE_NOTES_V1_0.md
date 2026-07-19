# QuickPLS 1.0.0 Release Notes

QuickPLS 1.0.0 is the first stable Windows release for the documented supported scope.

## Included

- Offline Windows desktop app with no account, activation, telemetry, cloud sync, or remote computation.
- Project archive, data import, autosave/recovery, saved runs, reports, and CLI workflows.
- Validated supported-scope PLS-SEM, assessment, inference, extended PLS, prediction/heterogeneity, bounded CB-SEM/CFA ML, bounded GSCA, PCA, regression/PROCESS, and NCA.
- Professional SEM designer with academic canvas style, draggable indicators, persistent layout metadata, result overlays, and publication SVG export.
- CLI/desktop export surfaces for CSV, HTML, XLSX, and SVG diagram export through the desktop report workflow.

## Not Included

- SmartPLS project import.
- Claims of identical SmartPLS results or undocumented SmartPLS behavior matching.
- Native CLI PDF/PNG export.
- Ordinal/polychoric/WLSMV/FIML CB-SEM.
- Signed installer, unless a signing certificate is separately provided and audited.

## Installer Status

The default v1.0.0 build is unsigned. Windows may show a SmartScreen warning. This is a packaging trust warning, not a runtime network or activation requirement.

## Validation

The v1.0.0 release is valid only when these gates are clear:

- `publication_ready_v0_1_to_v0_8`
- `v0_9_publication_release_candidate`
- `v0_9_3_professional_sem_designer`
- `v1_0_stable`
