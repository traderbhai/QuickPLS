# QuickPLS v2.39.0 Native Frontend Screen Replacement Plan

## Summary

v2.39.0 adds the first production-candidate bridge for the QuickPLS 2 native desktop shell. The native shell remains opt-in and is not the default app yet.

The bridge is enabled with:

```text
?native_shell=1
```

This is separate from the isolated prototype flag:

```text
?native_prototype=1
```

## What Changed

- Added a `NativeShellCandidateApp` route in the main app entry point.
- Preserved the existing legacy application as the default route.
- Preserved the isolated native prototype route for visual and adapter testing.
- Added explicit mapping between native shell views and existing workspace ids:
  - Home -> `welcome`
  - Data -> `data`
  - Model -> `models`
  - Setup -> `analyses`
  - Run -> `run`
  - Results -> `runs`
  - Report -> `reports`
  - Trust Center -> `trust`
  - Settings -> `settings`
- Added a production-candidate mode marker to the native shell:
  - `data-v239-shell-mode="production-candidate"`
- Kept the v2.38 read-only workspace adapter intact.

## Boundary

This milestone is frontend/product only.

It does not change:

- statistical engines;
- formulas;
- estimator crates;
- result schemas;
- project archive format;
- validation tolerances;
- numerical fingerprints.

## Evidence

- `npm run build`
- `npm run qpls:v239:screen-replacement-smoke`
- `npm run qpls:v239:screen-replacement-audit`
- `cargo run -p qpls-cli -- gate v2_39_0_native_frontend_screen_replacement_plan`

Smoke screenshots are written under:

```text
validation/results/screens/v2390/screen-replacement/
```

## Next

The next milestone should start replacing individual production screens with native workbench components behind a controlled rollout path, beginning with low-risk support surfaces and read-only screens before touching creation/edit workflows.
