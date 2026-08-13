# QuickPLS v2.5.1 Workflow Navigation Parity

## Summary

v2.5.1 makes the top workflow strip and left navigation rail tell the same information-architecture story.

The left rail remains the full workspace navigation surface. It contains the primary research workflow plus the separate Support utilities. The top strip is now explicitly the primary calculation workflow only: Data, Model, Setup, Run, Results, and Report.

## Scope

Frontend/product only.

## Changes

- Added explicit workflow scope metadata to `WorkflowStrip`.
- Added a visible `Workflow` label to the top strip.
- Kept Trust and Settings as Support destinations in the left rail, not as workflow steps.
- Added rendered smoke evidence for the workflow strip, rail hierarchy, and Settings support route.
- Added static audit coverage for version metadata, registry state, roadmap expectations, source contracts, mojibake prevention, claim boundaries, and frontend-only scope.

## Non-Goals

- No statistical engine changes.
- No estimator, formula, result schema, project archive, validation tolerance, or numerical fingerprint changes.
- No reclassification of method validation status.
- No installer signing change.

## Verification

```powershell
npm run qpls:v251:workflow-navigation
```

This runs the production frontend build, rendered workflow smoke, static workflow audit, and final gate:

```powershell
cargo run -p qpls-cli -- gate v2_5_1_workflow_navigation_parity
```

## Artifact Convention

Completed milestone artifacts must be created with:

```powershell
npm run qpls:desktop:build-versioned
```

The generated setup executable, portable executable, and checksum file must be copied under:

```text
D:\QuickPLS\target\release\artifacts
```

with the version, milestone label, timestamp, architecture, and artifact type in the filename.
