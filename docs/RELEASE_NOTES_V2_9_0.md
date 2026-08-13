# QuickPLS 2.9.0

QuickPLS 2.9.0 is a frontend/product governance release for the QuickPLS 2.x redesign program.

## What Changed

- Added a rendered acceptance-backlog smoke across Home, Data, Model, Setup, Run, Results, Report, Trust Center, and Settings.
- Generated a grouped next-pass backlog at `validation/results/v290_acceptance_backlog.json`.
- Classified upcoming UI work as `do_next`, `defer`, and `do_not_do` so future work stays in larger release-sized milestones.
- Kept Results/Report refinement, method applicability follow-up, and real-dataset review protocol as the next highest-value workstreams.
- Preserved the SEM Designer core as deferred unless explicitly requested.

## Verification

```powershell
npm run qpls:v290:acceptance-backlog
cargo run -p qpls-cli -- gate v2_9_0_acceptance_backlog_and_next_pass
```

## Boundary

This release does not change statistical engines, formulas, method validation, result schemas, project archives, or numerical fingerprints.
