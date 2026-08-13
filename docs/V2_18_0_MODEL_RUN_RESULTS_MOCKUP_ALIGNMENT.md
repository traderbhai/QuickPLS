# QuickPLS v2.18.0 Model, Run, And Results Mockup Alignment

Status: `validated`

Milestone id: `v2_18_0_model_run_results_mockup_alignment`

## Scope

This frontend-only milestone aligns the Model shell, Run calculation workspace, and Results workspace with the QuickPLS 2.0 mockup direction while preserving the existing SEM canvas internals, analysis execution flow, result payloads, and interpretation logic.

## Implemented

- Added v2.18 screen markers and mockup-alignment classes to:
  - `src/components/ModelCanvas.tsx`
  - `src/components/RunWorkspace.tsx`
  - `src/components/RunHistory.tsx`
- Tightened the SEM designer surrounding chrome: compact canvas toolbar, flatter context toolbar, cleaner overlay and next-action notices.
- Reworked Run screen density toward a desktop calculation panel: compact launch package, readiness checks, output preview, execution plan, and handoff cards.
- Reworked Results screen density toward a desktop result workbook: compact navigation, grouped table/action controls, sticky selected-run context, denser lens panel, and tighter table/result card chrome.
- Fixed stale `R²` mojibake in the touched Model/Run UI strings.

## Verification

- `npm run build`
- `npm run qpls:v2180:model-run-results-smoke`
- `npm run qpls:v2180:model-run-results-audit`
- `cargo run -p qpls-cli -- gate v2_18_0_model_run_results_mockup_alignment`

## Boundary

No statistical engine, formula, result schema, project archive format, validation tolerance, or numerical fingerprint changes are part of this milestone. Versioned desktop artifacts were not created because this is an intermediate UI milestone and the user asked to reserve artifact builds for final/release-ready checkpoints or explicit requests.
