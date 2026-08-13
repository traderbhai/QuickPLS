# QuickPLS v2.7.0 Visual Issue Register

## Scope

This milestone adds a repeatable rendered-screen issue register for the QuickPLS 2.x frontend shell.

## What It Checks

- Home, Data, Model, Setup, Run, Results, Report, Trust, and Settings render at desktop viewports.
- The shell has no document-level horizontal overflow in the checked viewports.
- Disabled controls expose a visible or accessible reason.
- User-facing text avoids mojibake, placeholder wording, `Validation fixture`, and SmartPLS-equivalence claims.
- Screenshots and a machine-readable issue register are written for future UI work.

## Evidence

- `validation/results/v270_visual_issue_register_smoke.json`
- `validation/results/v270_visual_issue_register_audit.json`
- `validation/results/v270_visual_issue_register.json`
- screenshots under `validation/results/screens/v270/visual-issue-register/`

## Boundary

This is a frontend governance and visual QA milestone. It does not change statistical engines, method validation, result schemas, project archive behavior, or numerical fingerprints.
