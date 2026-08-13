# QuickPLS v2.1.1 Home/Data Mockup Alignment

## Summary

v2.1.1 applies the QuickPLS 2.1 design-system primitives to the Home and Data workspaces. The milestone keeps the existing researcher workflow, native import commands, metadata editing, prefix construct creation, and method guidance, but moves the visible surfaces closer to the approved QuickPLS 2.0 desktop mockup.

This is a frontend/product-only release. It does not change statistical engines, formulas, result schemas, project archive format, validation tolerances, or numerical fingerprints.

## What Changed

- Home now composes `WorkspacePage`, `PageHeader`, `Panel`, `Card`, `MetricCard`, and `InlineNotice`.
- Data now composes `WorkspacePage`, `PageHeader`, `Panel`, `MetricCard`, and `InlineNotice`.
- Home shows a tighter project command center with primary actions, workspace metrics, workflow status, sample project gallery, and start-from-dataset guidance.
- Data keeps import, quality, prefix creation, preview, and metadata editing in the same workflow, but aligns its first viewport to the v2 panel and metric system.
- Native import, sample data loading, column metadata persistence, and prefix construct creation remain wired to the existing APIs.

## Non-Goals

- No estimator changes.
- No formula or result-value changes.
- No project archive schema changes.
- No new method claims or SmartPLS-equivalence claims.
- No mobile-specific redesign.

## Verification

Run:

```powershell
npm test -- --run
npm run build
npm run qpls:v2111:home-data
cargo test -p qpls-core -p qpls-project -p qpls-runner -p quickpls-desktop
cargo run -p qpls-cli -- gate v2_1_1_home_data_mockup_alignment
npm run qpls:desktop:build-versioned
```

## Release Artifacts

Every v2.1.1 build must create a fresh installer, portable executable, and checksum file under:

```text
D:\QuickPLS\target\release\artifacts
```

The artifact filenames must include `2.1.1`, `v2_1_1_home_data_mockup_alignment`, timestamp, architecture, and artifact type.
