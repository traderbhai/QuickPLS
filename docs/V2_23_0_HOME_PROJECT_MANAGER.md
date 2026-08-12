# QuickPLS v2.23.0 Home And Project Manager

v2.23.0 turns Home into a compact desktop start center instead of a web-style landing page.

## Scope

- Project launcher with new/build, data import, demo, and open-project entry points.
- Current workspace summary with rows, constructs, and runs.
- Recent project list backed by existing UI onboarding state.
- Recovery/autosave status panel.
- Quick links to Trust Center, method setup, and keyboard shortcuts.

## Boundaries

- Frontend/product only.
- No estimator, formula, result schema, project archive, or numerical fingerprint changes.
- No SmartPLS equivalence or import claim.

## Evidence

- `validation/results/v2230_home_project_manager_smoke.json`
- `validation/results/v2230_home_project_manager_audit.json`
- `cargo run -p qpls-cli -- gate v2_23_0_home_project_manager`
