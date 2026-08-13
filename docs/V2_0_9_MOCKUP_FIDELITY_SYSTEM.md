# QuickPLS v2.0.9 Mockup Fidelity System

## Summary

v2.0.9 turns the approved QuickPLS 2.0 mockup direction into an enforceable implementation system. The milestone is frontend/product-only: it does not change statistical engines, formulas, result schemas, project archive format, validation tolerances, or numerical fingerprints.

The purpose is to keep all later QuickPLS 2.0 screens aligned with the selected professional desktop mockup instead of drifting into one-off page designs.

## Completed Scope

- Expanded `docs/V2_UI_VISUAL_CONTRACT.md` with:
  - source-of-truth rules;
  - viewport targets;
  - mockup-matching rules;
  - screen completion checklist;
  - versioned artifact requirement.
- Added static smoke coverage for:
  - v2 shared primitive presence;
  - required shell/workspace source files;
  - mockup contract rules;
  - forbidden mojibake and stale v1-only wording;
  - v2.0.9 version and artifact labels.
- Added audit coverage for:
  - package, Tauri, Cargo, package-lock, and Cargo.lock version consistency;
  - registry current stage;
  - roadmap current stage expectation;
  - delivery and ledger documentation;
  - presence of all earlier v2 milestone docs;
  - release artifact naming convention.
- Added registry gate:
  - `v2_0_9_mockup_fidelity_system`.

## Design Contract Requirements

Every future v2 screen should be judged against:

- `1440x900` and `1280x800` desktop viewports;
- shared `qpls2` primitives;
- consistent title, action, panel, chip, and table treatment;
- local disabled-action explanations;
- no avoidable page-level horizontal scroll;
- no broad SmartPLS equivalence claims;
- no unsupported method promotion wording;
- no R-squared mojibake.

## Non-Goals

- No estimator changes.
- No backend numerical changes.
- No result payload or project archive changes.
- No SEM Designer behavior rewrite.
- No native PDF/PNG export promotion.

## Verification

```powershell
npm test -- --run
npm run build
npm run qpls:v209:mockup-fidelity-smoke
npm run qpls:v209:mockup-fidelity-audit
cargo test -p qpls-core -p qpls-project -p qpls-runner -p quickpls-desktop
cargo run -p qpls-cli -- gate v2_0_9_mockup_fidelity_system
npm run qpls:desktop:build-versioned
```

## Release Artifacts

Versioned artifacts must be written under:

```text
D:\QuickPLS\target\release\artifacts
```

Expected naming pattern:

```text
QuickPLS_2.0.9_v2_0_9_mockup_fidelity_system_<timestamp>_x64_setup.exe
QuickPLS_2.0.9_v2_0_9_mockup_fidelity_system_<timestamp>_x64_portable.exe
QuickPLS_2.0.9_v2_0_9_mockup_fidelity_system_<timestamp>_x64_checksums.txt
```
