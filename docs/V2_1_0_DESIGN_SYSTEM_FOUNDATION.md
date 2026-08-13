# QuickPLS v2.1.0 Design System Foundation

v2.1.0 starts the deeper QuickPLS 2.x rebuild by turning the approved desktop mockup into a reusable frontend design system. This milestone is frontend/product-only. It does not change estimators, formulas, result schemas, project archive format, validation tolerances, or numerical fingerprints.

## Scope

- Adds shared v2 workspace primitives for page shells, workspace headers, panels, metric cards, command groups, toolbar buttons, inline notices, and cards.
- Keeps the existing v2 visual contract as the source of truth for mockup alignment.
- Adds a visible design-system preview in Settings so the primitives can be inspected in the app.
- Adds smoke and audit scripts that verify shared primitive coverage, version metadata, registry state, documentation, and visual evidence.
- Preserves the existing SEM Designer behavior and numerical engine boundaries.

## Non-Goals

- No estimator changes.
- No formula, validation tolerance, project archive, result schema, or numerical fingerprint changes.
- No SmartPLS equivalence, SmartPLS project import, or undocumented-behavior claim.
- No full workspace rebuild in this milestone; later v2.1.x milestones will apply the primitives screen by screen.

## Verification

Run:

```powershell
npm run qpls:v2100:design-system-smoke
npm run qpls:v2100:design-system-audit
cargo run -p qpls-cli -- gate v2_1_0_design_system_foundation
```

The smoke test captures all primary workspaces and checks the Settings design-system preview. The audit verifies metadata, registry, roadmap, docs, script labels, primitive source coverage, screenshots, and claim boundaries.

## Release Artifacts

Versioned desktop artifacts must be generated with:

```powershell
npm run qpls:desktop:build-versioned
```

The output remains under:

```text
D:\QuickPLS\target\release\artifacts
```

with installer, portable executable, and checksum files using the versioned naming convention.
