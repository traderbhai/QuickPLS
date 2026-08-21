# QuickPLS 2.53 expert-user product audit

- Audit date: 2026-08-21
- QuickPLS build: 2.53.0
- Git commit: `f959847708b653af86656e6c336c38ed34a8b135`
- Packaged executable SHA-256: `8d5c2fa199a429ffe2d14044399234e0aa564ce89ba1ca77f60e96dfd65de1be`

## Purpose and comparison boundary

This audit evaluates QuickPLS as an experienced SmartPLS user would encounter it: import data, construct a model, configure a method from Calculate, run it, inspect Results, export, save, close, and reopen. It also covers mediation, moderation, higher-order constructs (HOCs), CB-SEM, group analysis, prediction, standalone analyses, keyboard use, compact layouts, and recovery evidence.

The current official SmartPLS baseline checked for this audit was SmartPLS 4.1.1.8. The comparison uses documented user workflows from the [SmartPLS first-model tutorial](https://www.smartpls.com/documentation/tutorials/first-pls-path-model/), [algorithm catalogue](https://www.smartpls.com/documentation/algorithms-and-techniques/), [mediation guidance](https://www.smartpls.com/documentation/algorithms-and-techniques/extended-relationships/mediation/), [moderation guidance](https://www.smartpls.com/documentation/algorithms-and-techniques/extended-relationships/moderation/), and [higher-order-model guidance](https://www.smartpls.com/documentation/algorithms-and-techniques/extended-relationships/higher-order/).

The objective is familiarity and professional competitiveness, not visual cloning. Exact SmartPLS colors, spacing, icons, file formats, undocumented shortcuts, internal routing, or numerical output are not acceptance requirements. QuickPLS must retain its independent scientific authority.

Evidence labels used below:

- **D**: directly documented SmartPLS workflow or behavior.
- **Q**: professional QuickPLS quality requirement.
- **G**: competitive feature-gap candidate, not automatically a defect.

Priorities:

- **P0**: blocks a promised analysis, risks wrong routing/results or data loss, or prevents save/reopen.
- **P1**: major researcher usability, workflow, presentation, or confidence issue.
- **P2**: useful polish or optional competitive improvement.

## Executive verdict

QuickPLS already has a credible, professional Windows workbench and a recognizably SmartPLS-familiar primary flow. Project, Data, Canvas, Calculate, and Results are coherent. The Calculate catalogue contains exactly 18 searchable methods, the setup dialog remains usable at 1024×700, mediation is represented by ordinary structural paths, the new moderation anchor is a strong independent design, and the compact HOC dialog is substantially better than a separate technical workspace.

No confirmed P0 installed-product defect was established by this audit. The most important confirmed weakness is advanced Results presentation: current moderation and HOC results expose raw generated IDs, hashes, index-based axes, and internal table identities in the main researcher view. Those calculations complete and reopen correctly, but the output is not yet presentation-quality for normal researchers.

The next most important issues are model readability at Fit/large-model scale, clipped method controls and compact inference tables, unclear exact-fit wording, and several advanced workflows that have strong source/unit coverage but no current packaged visual evidence.

## Automated evidence summary

| Evidence | Outcome | Interpretation |
|---|---:|---|
| Current production-bundle visual automation | 96 current screenshots; 0 console errors; 41 explicit preview limitations | Strong current UI evidence across 1024×700, 1280×720, 1440×900, DPR 2, and a 20-construct/80-indicator model. |
| Legacy v2.47 visual assertions | 118 failures | These are not 118 product defects: 117 individual assertions cover removed toolbar/tutorial/scope-copy contracts; the 118th is an aggregate matrix assertion for six deliberately removed captures. The harness requires rebaselining. |
| Full frontend Vitest pass | 1,567 passed; 17 failed; 1,584 total | Most failures are stale source-string, pre-unification surface, or pre-promotion Registry expectations. They reduce regression confidence but do not establish 17 user-facing defects. |
| Focused failed-contract JSON | 92 passed; 17 failed; 109 tests in the 11 failing files | Machine-readable details: [`vitest_failed_contracts.json`](../validation/results/smartpls_expert_product_audit_v253_20260821/vitest_failed_contracts.json). |
| TypeScript project typecheck | Passed | `npm run typecheck:full` completed successfully. |
| Same-build packaged moderation journey | Passed; 6 screenshots; no external network; fresh-process reopen retained the same canonical identity | Current 2.53 evidence from the packaged build generated immediately after the audited executable was built. |
| HOC packaged journey | Passed; 6 screenshots; save/reopen retained identity | v2.52 packaged evidence for the integrated HOC workflow; current 2.53 source and typecheck were reviewed, but a new packaged HOC run was blocked by another open QuickPLS instance. |
| Fresh packaged rerun during this audit | Not run to completion | Another QuickPLS instance was already open. The audit did not close it because that could destroy unsaved user work. |

The headless browser correctly disables execution because native IPC is absent. Every red “offline QuickPLS desktop runtime” message in those screenshots is a preview limitation, not an installed-app failure.

All screenshot links in this report resolve against the local audit workspace. The generated images and machine-readable result files are intentionally kept out of normal source history until the user decides which observations and evidence should be published.

## Confirmed findings for user review

### Results and scientific interpretation

| ID | Priority | Basis | Observation and user impact | Recommendation | Evidence |
|---|---|---|---|---|---|
| UX-001 | P1 | Q | Moderation Results expose generated construct IDs and interaction-term IDs in the Results tree, chart axes, chart title, series, table cells, and source-table column. The chart is clipped by the oversized raw axis label. A researcher cannot readily tell which variables, probes, or slopes are being shown. | Resolve every internal identity to the authored construct/path labels in the main view. Show `W = −1 SD`, `W = mean`, `W = +1 SD`, meaningful axis titles, and a compact legend. Keep raw IDs only under Run Details. | [Packaged moderation Results](../validation/results/v253_mediation_moderation_packaged_smoke_20260821_final_r6/screens/05-results.png) |
| UX-002 | P1 | Q | HOC Results use `Target index` values 1–3, raw effect IDs, and internal source-table names. The chart bars have no researcher-facing component/path labels, and the explanatory text exposes “resident canonical result” architecture. | Label bars and rows with component relationships and authored HOC structural paths. Move canonical IDs/source tables to Diagnostics. | [Packaged HOC Results](../validation/results/v252_hoc_packaged_smoke_20260821_120805_r2/screens/05-results.png) |
| UX-003 | P1 | Q | The 1024×700 aggregate mediation-bootstrap table loses the path identity after horizontal keyboard movement; the first visible cell can become only the tail of a path such as `nce → loyalty`. | Make the path column sticky, retain a visible horizontal scrollbar, and collapse lower-priority columns responsively. | [Compact mediation bootstrap](../validation/results/screens/smartpls-expert-audit-v253/19-mediation-bootstrap-inference-1024x700.png) |
| UX-004 | P1 | Q | Specific indirect-effect rows use lowercase/internal short labels instead of the model’s visible names, and selecting the table does not show a linked chain overlay. | Render authored construct labels. Add `Show on model` or retain a compact synchronized diagram that highlights the complete indirect chain. | [Specific indirect effects](../validation/results/screens/smartpls-expert-audit-v253/18-mediation-results-1280x720.png) |
| UX-005 | P1 | Q | `Exact-fit bootstrap: Historical descriptive measures` is not a clear run state and conflicts with the intended five-state model-fit presentation. | Show `Not run`, `Available`, `Partial`, `Unavailable`, or `Failed`; retain the historical reason under Run Details. | [Completed Results](../validation/results/screens/smartpls-expert-audit-v253/17-completed-results-1280x720.png) |
| UX-006 | P2 | Q | Numerical table columns are generally left-aligned and compact inference headings do not expose the confidence level. | Right-align numbers/decimals and include the interval level in the heading or table properties. | [Specific indirect effects](../validation/results/screens/smartpls-expert-audit-v253/18-mediation-results-1280x720.png), [bootstrap inference](../validation/results/screens/smartpls-expert-audit-v253/19-mediation-bootstrap-inference-1024x700.png) |
| UX-007 | P2 | Q | Empty Results gives a correct explanation but no direct action. | Add one compact `Calculate…` button while retaining the existing menu/toolbar command. | [Empty Results](../validation/results/screens/smartpls-expert-audit-v253/18-empty-results-1280x720.png) |
| UX-008 | P2 | Q | Export evidence stops at `Preparing export options…` with no visible progress treatment. | If preparation can exceed an instant, show an indeterminate progress indicator and retain Escape/Cancel. | [Export preparation](../validation/results/screens/smartpls-expert-audit-v253/18-export-dialog-1280x720.png) |

### Canvas and advanced model authoring

| ID | Priority | Basis | Observation and user impact | Recommendation | Evidence |
|---|---|---|---|---|---|
| UX-009 | P1 | Q | Fit-to-screen renders even a four-construct model smaller than necessary, while the 20-construct model becomes effectively unreadable. Names, path coefficients, and indicator labels are too small despite substantial whitespace. | Fit to the usable canvas bounds with a minimum readable zoom, offer `Fit selection`, and make large-model layout/zoom defaults prioritize labels rather than total containment. | [Normal Canvas](../validation/results/screens/smartpls-expert-audit-v253/05-model-1280x720.png), [large model](../validation/results/screens/smartpls-expert-audit-v253/11-large-model-20c-80i-1440x900-large-model.png) |
| UX-010 | P1 | Q | In packaged moderation/HOC layouts, indicator boxes and measurement arrows overlap latent constructs and structural paths. The moderation anchor itself is clear, but the surrounding model is unnecessarily congested. | Make auto-layout indicator-aware and reserve non-overlapping lanes around latent nodes and focal paths. | [Moderation Canvas](../validation/results/v253_mediation_moderation_packaged_smoke_20260821_final_r6/screens/02-canvas.png), [HOC Canvas](../validation/results/v252_hoc_packaged_smoke_20260821_120805_r2/screens/02-canvas.png) |
| UX-011 | P1 | Q | `Prepare Advanced Methods`, `Conditional Process`, and `Advanced Parameters` sit beside Canvas as permanent tab-like controls. This revives the workspace ambiguity that the unified workflow was intended to remove and consumes compact width. | Move them to Model/context menus or style them unmistakably as contextual commands rather than document tabs. | [Canvas 1280](../validation/results/screens/smartpls-expert-audit-v253/05-model-1280x720.png), [DPR-2 Canvas](../validation/results/screens/smartpls-expert-audit-v253/10-model-200pct-scale-1024x700@200pct-device-scale.png) |
| UX-012 | P1 | Q | The complex PROCESS preview is too small for four mediators, two moderators, and eight paths; lines cross and moderation annotations are tiny. | Reuse the full diagram renderer with Fit/zoom and semantic moderation anchors, or provide an enlarged preview action. | [PROCESS setup](../validation/results/screens/smartpls-expert-audit-v253/21-process-v2-dialog-1280x720.png) |
| UX-013 | P2 | Q | Project overview is clean but the detail pane is mostly unused whitespace. | Add a compact model preview, last calculation/report, modified time, and dataset status—without turning it into a card dashboard. | [Project overview](../validation/results/screens/smartpls-expert-audit-v253/01-launcher-1280x720.png) |

### Calculate, eligibility, and method setup

| ID | Priority | Basis | Observation and user impact | Recommendation | Evidence |
|---|---|---|---|---|---|
| UX-014 | P1 | Q | Several values and primary-action labels truncate at normal desktop widths: WPLS case-weight prompt, NCA ceiling line, PCA retention method, regression type, PLSpredict action, and PROCESS action. | Use shorter stable primary labels such as `Start calculation`, allow settings to wrap, and expose full values via layout or tooltip. | [WPLS](../validation/results/screens/smartpls-expert-audit-v253/08-wpls-dialog-1280x720.png), [PCA](../validation/results/screens/smartpls-expert-audit-v253/16-pca-standalone-dialog-1280x720.png), [Regression](../validation/results/screens/smartpls-expert-audit-v253/17-ols-standalone-dialog-1280x720.png), [PROCESS](../validation/results/screens/smartpls-expert-audit-v253/21-process-v2-dialog-1280x720.png) |
| UX-015 | P1 | Q | The logistic fixture reports `36 complete cases: 0 class 0 and 0 class 1`. The visible blocker only mentions preview runtime, not the invalid outcome coding. | Prioritize the scientific blocker: `Outcome must contain both 0 and 1`; list detected distinct values and offer a direct recode/change-outcome action. Confirm in packaged Tauri before deciding whether this is P0. | [Logistic setup](../validation/results/screens/smartpls-expert-audit-v253/18-logistic-standalone-dialog-1280x720.png) |
| UX-016 | P1 | Q | Local eligibility problems for WPLS, CTA-PLS, MGA, and logistic are masked by the preview-runtime blocker. That prevents useful model inspection even though the preview advertises inspection capability. | Show model/data eligibility first and runtime availability second, or show both concise causes in priority order. | [WPLS](../validation/results/screens/smartpls-expert-audit-v253/08-wpls-dialog-1280x720.png), [CTA-PLS](../validation/results/screens/smartpls-expert-audit-v253/09-cta-pls-dialog-1280x720.png), [MGA](../validation/results/screens/smartpls-expert-audit-v253/13-mga-dialog-1280x720.png) |
| UX-017 | P1 | Q | MGA group values such as `3 — 1 complete of 1` are cryptic and the fixture has only one complete case per group. | Show group label/value plus `N`, state excluded rows separately, and present minimum-sample eligibility before the Step-1 confirmation. | [MGA setup](../validation/results/screens/smartpls-expert-audit-v253/13-mga-dialog-1280x720.png) |
| UX-018 | P1 | Q | The generic CB-SEM screenshot shows ML convergence options but no visible point/case-bootstrap selection, topology summary, bootstrap controls, or Advanced Parameter Table entry. It is unclear whether this is conditional on a strict eligible model. | In an eligible CB-SEM model, expose one concise topology/inference row and an `Advanced parameters…` action in Calculate. Add current packaged evidence for both CFA and recursive topology. | [CB-SEM setup](../validation/results/screens/smartpls-expert-audit-v253/10-cbsem-dialog-1280x720.png) |
| UX-019 | P2 | Q | GSCA displays an empty bordered `Method settings` group, which looks unfinished. | Remove the empty group or show a one-line fixed execution summary. | [GSCA setup](../validation/results/screens/smartpls-expert-audit-v253/09-gsca-dialog-1280x720.png) |
| UX-020 | P2 | Q | At 1024×700 the method list may scroll selected methods to the top with category headings partly cut off and no prominent scrollbar. | Preserve the selected category heading or add a subtle always-visible scrollbar/fade cue. | [MGA 1024](../validation/results/screens/smartpls-expert-audit-v253/13-mga-dialog-1024x700.png), [PCA 1024](../validation/results/screens/smartpls-expert-audit-v253/16-pca-standalone-dialog-1024x700.png) |
| UX-021 | P2 | Q | The compact logistic-bootstrap dialog clips the last part of the red blocker near the fixed footer. | Add footer-aware bottom padding and ensure the blocker scrolls fully into view. | [Logistic bootstrap 1024](../validation/results/screens/smartpls-expert-audit-v253/20-regression-bootstrap-logistic-dialog-1024x700.png) |

### Engineering and automation confidence

| ID | Priority | Basis | Observation and user impact | Recommendation | Evidence |
|---|---|---|---|---|---|
| ENG-001 | P1 | Q | The current frontend suite has 17 failures. Most expect removed General SEM/Exact CB-SEM tabs, Labs-era access, old copy, or literal source shapes; one accessibility contract still prohibits local Enter handling that the new Canvas intentionally uses. | Rebaseline only after reviewing each assertion against the 2.53 workflow. Replace source-string tests with behavior tests where possible. | [`vitest_failed_contracts.json`](../validation/results/smartpls_expert_product_audit_v253_20260821/vitest_failed_contracts.json) |
| ENG-002 | P1 | Q | The broad visual harness reports 118 failures because it still expects removed HOC/moderation toolbar buttons, long in-dialog methodology, and obsolete scope copy. It captured 96 valid current screenshots with no console errors, but cannot be used as a pass/fail release gate. | Create a 2.53 baseline around Model-menu/context/keyboard entry, compact dialogs, and Method Details. Preserve the 96 current screenshots as diagnostic evidence, not a green receipt. | [`smartpls_expert_audit_v253_visual.json`](../validation/results/smartpls_expert_audit_v253_visual.json) |
| ENG-003 | P1 | Q | Only 14 of the 18 methods have a dedicated selected setup screenshot in the current visual matrix. Missing selected captures are PLS-SEM Bootstrapping, PLSc Bootstrapping, Post-hoc Technical Minimum Sample Size, and Sample Size/Power. | Add current setup and one representative completed-result capture for the four missing entries. | [Calculate catalogue](../validation/results/screens/smartpls-expert-audit-v253/06-calculation-dialog-1280x720.png) |
| ENG-004 | P1 | Q | The current visual matrix has only PLS/mediation completed Results. Other methods rely on older packaged evidence or unit tests. | Build a serial, low-cost current packaged screenshot crawl that opens frozen successful archives instead of recomputing expensive engines. | [Current completed Results](../validation/results/screens/smartpls-expert-audit-v253/17-completed-results-1280x720.png) |
| ENG-005 | P2 | Q | DPR-2 browser rendering is crisp and unclipped, but it is not proof of Windows 200% scaling or screen-reader speech. | Add one packaged Windows 200% layout journey and one structured screen-reader/manual keyboard record. | [DPR-2 screenshot](../validation/results/screens/smartpls-expert-audit-v253/10-model-200pct-scale-1024x700@200pct-device-scale.png) |

## Competitive feature-gap candidates

These are choices for product planning, not defects in existing promised functionality.

| ID | Priority | Basis | Gap | Recommendation | Evidence |
|---|---|---|---|---|---|
| GAP-001 | P1 | G | PLSpredict/CVPAT does not visibly expose folds or repetitions. SmartPLS documents both as core settings. | Either expose them or show the fixed `10 folds × 10 repetitions` plan explicitly. | [PLSpredict setup](../validation/results/screens/smartpls-expert-audit-v253/12-prediction-dialog-1280x720.png), [SmartPLS prediction documentation](https://www.smartpls.com/documentation/algorithms-and-techniques/prediction-and-segmentation/predict/) |
| GAP-002 | P1 | G | IPMA does not visibly expose predecessor scope or theoretical indicator ranges used for 0–100 performance scaling. | Add an Advanced section with defaults, validation, and a concise range summary. | [IPMA setup](../validation/results/screens/smartpls-expert-audit-v253/10-ipma-dialog-1280x720.png), [SmartPLS IPMA documentation](https://www.smartpls.com/documentation/algorithms-and-techniques/prediction-and-segmentation/ipma/) |
| GAP-003 | P1 | G | NCA does not visibly expose bottleneck-table steps, significance level, or parallel workers. | Add only the controls QuickPLS actually supports; otherwise document fixed values in Method Details and Run Details. | [NCA setup](../validation/results/screens/smartpls-expert-audit-v253/14-nca-standalone-dialog-1280x720.png), [SmartPLS NCA documentation](https://www.smartpls.com/documentation/algorithms-and-techniques/nca/) |
| GAP-004 | P1 | G | No side-by-side saved-report comparison with synchronized result navigation was found. | Consider a read-only comparison workspace after core result-labeling issues are fixed. | [Project explorer](../validation/results/screens/smartpls-expert-audit-v253/1a-workspace-explorer-1280x720.png), [SmartPLS first-project documentation](https://www.smartpls.com/documentation/tutorials/first-project/) |
| GAP-005 | P1 | G | No routed searchable offline Help browser was found; current Help exposes shortcuts and About, while methodology is spread across external docs/Method Details. | Add a compact offline Help/Method Details search dialog, not a permanent Help workspace. | [Main shell](../validation/results/screens/smartpls-expert-audit-v253/05-model-1280x720.png) |
| GAP-006 | P2 | G | Current screenshot evidence does not show general missing-value treatment choices comparable to none/mean replacement/casewise/pairwise workflows. | Decide and document the scientifically supported QuickPLS policy; expose alternatives only when the engines implement them consistently. | [Import setup](../validation/results/screens/smartpls-expert-audit-v253/04-import-data-dialog-1024x700.png), [SmartPLS missing-values documentation](https://www.smartpls.com/documentation/functionalities/missing-values/) |
| GAP-007 | P2 | G | No current evidence shows copying Results tables or model graphics directly into Excel/Word-compatible targets. | Add clipboard commands only after canonical labels and diagram export quality are corrected. | [Completed Results](../validation/results/screens/smartpls-expert-audit-v253/17-completed-results-1280x720.png) |
| GAP-008 | P2 | G | No current theme/color-blind workflow was found. | Consider a high-contrast/color-blind theme after verifying all selection and warning states remain non-color-dependent. | [Model Canvas](../validation/results/screens/smartpls-expert-audit-v253/05-model-1280x720.png) |

## Important evidence gaps—not yet product defects

The following capabilities exist in code/tests or earlier receipts but still need one current packaged screenshot journey before a complete visual acceptance claim:

- single, parallel, and serial mediation as separate authoring/routing examples;
- single-mediation versus multiple-mediation bootstrap routing;
- multiple simultaneous two-way moderation;
- second-moderator authoring and true three-way moderation Results;
- first- and second-stage latent moderated mediation;
- continuous and binary moderator probes, including invalid non-0/1 coding;
- RF, FR, and FF HOCs, advanced valid approaches, HOC editing, and HOC bootstrapping;
- Advanced Parameter Table edit → safe revision → calculation;
- CB-SEM point/case-bootstrap for CFA and recursive SEM, plus all exact-fit states;
- matrix, XLSX, SPSS, and ODS packaged import;
- every method family’s completed Results and chart/table exports;
- all six canonical exports written and reopened in one current packaged journey;
- autosave recovery, legacy migration, future-read-only archives, and unsaved-close prompts;
- actual Windows 200% scaling, contrast measurement, and screen-reader speech.

These gaps should be automated by opening existing verified archives wherever possible. Re-running expensive scientific qualification is unnecessary for this product-UX audit.

## Positive observations

- Project, Data, Model, Calculate, and Results form a coherent Windows-style workbench.
- The primary `Canvas → Calculate → Results` flow is familiar to SmartPLS users without copying SmartPLS visuals.
- Exactly 18 methods are searchable and grouped by purpose.
- Import setup is compact, supports raw/covariance/correlation data, lists the promised file formats, and gives a concise corrective error.
- The moderation dialog is exceptionally clear: focal relationship, moderator, derived order, compact summary, and collapsed Advanced details.
- The moderation anchor and dashed connector clearly target the focal path while generated interaction constructs remain hidden from the normal construct list.
- The HOC dialog is compact, derives RR/RF/FR/FF from conceptual direction and component modes, recommends an approach, and keeps legacy/advanced controls collapsed.
- Results navigation exposes familiar direct, indirect, total, moderation, HOC, bootstrap, and diagnostics categories only when applicable.
- Save/reopen evidence preserves canonical result identity for moderation and HOC workflows.
- The audited packaged moderation workflow made no external network request.
- Structural accessibility automation found no unnamed interactives, broken `aria-labelledby` references, duplicate IDs, positive `tabindex`, page overflow, or focus-trap failures in the inspected states.

## Recommended order if the user accepts the findings

1. Replace raw internal IDs in moderation and HOC Results with authored labels.
2. Fix Canvas/Results fit and auto-layout readability.
3. Fix compact table/path identity and Calculate control truncation.
4. Correct exact-fit state wording and scientific blocker priority.
5. Rebaseline stale tests and the visual harness to the current unified workflow.
6. Add low-cost current packaged evidence for the advanced routes listed above.
7. Decide the competitive gap candidates individually; do not expand scope merely for numerical feature-count parity.
