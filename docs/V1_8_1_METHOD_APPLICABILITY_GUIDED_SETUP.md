# QuickPLS v1.8.1 Method Applicability And Guided Setup

QuickPLS v1.8.1 adds a frontend-only method guidance layer so researchers see which analyses fit the current dataset, SEM model, and selected settings before they run anything.

## Scope

- No statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changed.
- Method claims remain bounded to documented QuickPLS scopes.
- Unsupported and experimental methods remain visible under Show all methods with reasons instead of being silently hidden.
- Inference procedures such as bootstrap are presented as add-ons/settings for compatible methods, not as ordinary primary analyses.

## User-Facing Changes

- Setup groups methods into Recommended for this project, Available after setup, Advanced diagnostics, Standalone analyses, and Show all methods.
- Each method card explains its current state, reason, expected outputs, and next action.
- The top-bar method selector now lists only recommended/available primary methods by default and points users to Setup for the full method catalog.
- Data includes a What can I do with this data? panel.
- Model includes a What can I do with this model? panel.
- Run readiness now uses method-specific applicability blockers such as missing group column, nonbinary logistic outcome, invalid WPLS weights, and unsupported CB-SEM/formative shapes.

## Applicability States

- Recommended: ready and sensible for the current project.
- Available: runnable, but not necessarily the primary recommendation.
- Needs setup: possible after required fields/settings are completed.
- Not applicable: incompatible with current data/model shape.
- Unsupported: outside QuickPLS documented scope.
- Experimental: available only with experimental/watermarked handling.

## Validation Evidence

- `validation/results/v181_method_applicability_smoke.json`
- `validation/results/v181_method_applicability_audit.json`
- `src/domain/methodApplicability.test.ts`

## Release Artifacts

Versioned builds for this milestone must be generated with:

```powershell
npm run qpls:desktop:build-versioned
```

Artifacts are written to `D:\QuickPLS\target\release\artifacts` and must include setup exe, portable exe, and checksums with `1.8.1` and `v1_8_1_method_applicability_guided_setup` in the filename.
