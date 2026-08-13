# QuickPLS v2.40.0 Mockup Fidelity Native Shell Alignment

## Summary

This milestone tightens the QuickPLS 2 native shell against the supplied mockup set in:

`C:\Users\mohd.naved\.codex\generated_images\019f7393-a658-7743-8d7c-435913f960da`

The work remains frontend/product-only. It does not change statistical engines, formulas, estimator crates, project archive format, result schemas, validation tolerances, or numerical fingerprints.

## Implemented

- Added a stronger Windows-like shell title pattern: `QuickPLS 2.0 - <Workspace>`.
- Replaced the compact command strip with a mockup-style ribbon command bar.
- Added visible ribbon commands from the mockups:
  - Save;
  - Undo / Redo;
  - Select / Pan;
  - Add Latent;
  - Add Indicator;
  - Connect Path;
  - Covariance;
  - Delete;
  - Arrange;
  - Check Diagram;
  - Focus Diagram;
  - Zoom;
  - Run.
- Reworked the Model workbench wrapper to match the mockup anatomy:
  - hierarchical SEM Explorer tree;
  - compact canvas toolbar;
  - tabbed Object Inspector;
  - bottom output pane with Model Issues, Diagram Advisor, Calculation Log, and Output tabs.
- Tightened Data, Setup, and Results layouts toward workbench/property-sheet patterns.
- Added source markers for automated mockup-fidelity verification.
- Added `docs/V2_40_MOCKUP_EXTRA_FEATURE_BACKLOG.md` to list old UI surfaces and features that are not present in the mockups and should be reconsidered later.

## Evidence

- `validation/results/v2400_mockup_fidelity_smoke.json`
- `validation/results/v2400_mockup_fidelity_audit.json`
- screenshots under `validation/results/screens/v2400/mockup-fidelity/`

## Boundary

The implementation intentionally prioritizes the mockup visual contract over retaining every previous UI surface. Existing backend behavior and SEM designer logic remain unchanged.
