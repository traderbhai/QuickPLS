# QuickPLS v2.14.0 Real Dataset Feedback Triage

## Purpose

This milestone converts the v2.12 real-dataset review protocol and v2.13 in-app entrypoints into an actionable triage loop. The goal is to keep future QuickPLS 2.x frontend work driven by anonymized researcher evidence rather than one-off visual reactions.

## Scope

This is a product and process milestone only.

- No estimator code changes.
- No method validation changes.
- No result schema changes.
- No project archive changes.
- No numerical fingerprint changes.
- No raw private datasets, private `.qpls` projects, value-revealing screenshots, or unredacted exports may be committed.

## Intake Workflow

1. Review the private dataset manually using `docs/V2_12_0_REAL_DATASET_REVIEW_PROTOCOL.md`.
2. Record anonymized findings with `validation/templates/real_dataset_feedback_triage_template.json`.
3. Separate findings into:
   - `launch_blocker`
   - `workflow_friction`
   - `visual_polish`
   - `method_guidance`
   - `reporting_export`
   - `statistical_evidence_gap`
4. Convert repeatable issues into synthetic fixtures or bundled sample cases before implementation.
5. Group accepted work into a single milestone in `docs/V2_ACTIVE_MILESTONE.md`.

## Prioritization

Priority is assigned by user impact and reproducibility:

| Priority | Meaning | Action |
|---|---|---|
| P0 | Prevents using QuickPLS on a valid research workflow | Fix in next grouped milestone |
| P1 | Creates serious confusion, wrong action, or report/export friction | Schedule in next or following grouped milestone |
| P2 | Reduces polish or slows experienced users | Batch with related UI work |
| P3 | Cosmetic or low-frequency concern | Defer unless repeated |

## Acceptance

The milestone is complete when:

- a triage template exists and blocks private raw-data persistence by design;
- generated backlog evidence exists under `validation/results/`;
- the active milestone tracker names v2.14 as the current completed checkpoint;
- registry gate `v2_14_0_real_dataset_feedback_triage` is clear;
- future next work is expressed as grouped milestones, not micro-fixes.
