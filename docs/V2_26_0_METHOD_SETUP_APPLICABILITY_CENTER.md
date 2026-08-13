# QuickPLS v2.26.0 Method Setup And Applicability Center

Status: validated.

This frontend-only milestone converts Setup into a native desktop calculation setup center. The underlying method applicability rules already existed; v2.26 makes them visible as a professional workflow that separates primary estimators, add-ons, advanced diagnostics, standalone analyses, and methods that are not applicable to the current project.

## Completed Scope

- Added the `v2_26_0_method_setup_applicability_center` release checkpoint.
- Renamed the Setup page header to `Calculation Setup`.
- Added a desktop-style method category strip for recommended, available, needs-setup, diagnostics, standalone, and not-applicable method states.
- Split method choices into researcher-facing lanes:
  - Recommended for this project.
  - Available now.
  - Available with setup.
  - Advanced diagnostics.
  - Standalone analyses.
  - Not applicable or scoped out.
- Added a selected-method requirements panel with exact first blocker, action labels, and scope status.
- Added an inference add-ons panel so bootstrap is configured as an add-on instead of looking like a primary algorithm.
- Preserved the conservative top-bar method selector behavior: recommended/available methods are shown by default, with broader choices directed to Setup.
- Fixed stale `R²` mojibake in method output labels.

## Validation

- `validation/v2260_method_setup_smoke.mjs`
- `validation/v2260_method_setup_audit.py`
- `validation/results/v2260_method_setup_smoke.json`
- `validation/results/v2260_method_setup_audit.json`
- `cargo run -p qpls-cli -- gate v2_26_0_method_setup_applicability_center`

## Boundary

No statistical engines, formulas, estimator crates, result schemas, project archive format, validation tolerances, or numerical fingerprints changed.

The change is limited to frontend/product presentation and UI-only setup guidance.
