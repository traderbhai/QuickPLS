# QuickPLS FAQ

## Is QuickPLS free?

Yes. Official QuickPLS releases are free to use.

## Is QuickPLS open source?

No. QuickPLS is proprietary source-available software. The source can be inspected and contributions can be proposed, but redistribution and derived commercial use are restricted by `LICENSE.md` and `EULA.md`.

## What is the current version?

The coordinated source version is 2.55.4. The 2.55.0 evidence described below
is historical and does not qualify a 2.55.4 package. Formal first diagnostic
`20260822T142953Z` at source `2e3a23f` executed all 14 steps and passed 13; the
sole failure was `frontend_typecheck`, where
`src/data/v255NamedSemEvidenceFixtures.test.ts` reported TypeScript error
`TS2339`. The final consolidated diagnostic `20260823T030939Z` at source
`e5723df08b7205ce75f1887c5f4709f235ad893c` passed 14/14, including 453/453
Vitest suites, 1724/1724 tests, 17/17 rebaseline assertions, and zero captured
console errors. Its report is
`validation/results/v255_consolidated_diagnostics_20260823T030939Z/v255_consolidated_diagnostics.json`,
has SHA-256 `03da7a8e0db2924d0157eb0cb0ca92e841fffd61f470d5cd16ccd58f87fe9b2a`,
and is retained in evidence commit `8a727262c07dd38bae38d8154e1662c78fbb8ee7`.
Both formal records use runner SHA-256
`64969b4eb89c9789e586c372532d89b082766a44661cc4feea66b6cf3a0f9796` and are
separate evidence records, not byte-identical.
Renderer console/page errors fail closed, attach-only phases use fresh
wrapper-owned processes, and candidate/phase/trusted-driver identities and
hashes are bound. The superseded final diagnostic `20260823T000930Z`,
candidate/install/smoke attempts `20260823T004848Z` and `20260823T005212Z`,
portable probe `20260822T233111Z`, and all prior 2.55 candidate, install, smoke,
diagnostic, or probe attempts are historical and ineligible. One new unsigned
2.55 candidate, isolated install, full installed-and-portable smoke, evidence
collection and bundling, final audit, and publication remain pending. Exactly
one case—the actual Windows
200% scaling case—may be `waived`; its real DPI screenshot and receipt remain
required and the other 54 named cases must pass. If an existing registered
installation must be removed, only its exact registered uninstaller may be
used, with project files, recovery data, and QuickPLS application user data left
untouched. Portable evidence does not replace installed evidence. The latest
published public pre-release remains
[`v2.54.0`](https://github.com/traderbhai/QuickPLS/releases/tag/v2.54.0); 2.55
remains unsigned and code signing is excluded.

## Does QuickPLS import SmartPLS projects?

No.

## Does QuickPLS guarantee identical results to SmartPLS?

No. QuickPLS implements published methods independently and validates against documented formulas, fixtures, simulations, and reference engines where permitted.

## Does QuickPLS require internet?

No account, cloud service, or remote computation is required. QuickPLS analytical workflows run locally after download. The Microsoft-managed WebView2 runtime may make its own background service connections unless Windows network controls block them.

## Does QuickPLS require R?

No. R/Rscript and R packages are validation-only development tools.

## Why does Windows warn about the installer?

The `v2.54.0` Windows artifacts are unsigned. Download them only from the [official release page](https://github.com/traderbhai/QuickPLS/releases/tag/v2.54.0), compare each SHA-256 value with the attached checksum file, and expect Windows SmartScreen to identify the publisher as unknown. Code signing is excluded from this release.

## What export formats are supported?

Canonical General SEM results support CSV, XLSX, self-contained HTML, PDF, SVG, and PNG. Other completed methods expose only the formats appropriate to their result type, including SVG or Windows Print/PDF where documented.

## Why are some methods unavailable?

QuickPLS evaluates each method against the current dataset, model, and settings. Setup shows whether a method is recommended, available after setup, not applicable, unsupported, or experimental, with the exact reason and next action.

## What was added in Version 2.50?

Version 2.50 integrates bounded General SEM mediation and simultaneous moderation, higher-order PLS point/bootstrap, two-way moderated mediation, recursive common-factor CB-SEM ML, and recursive-SEM case bootstrap into the same Canvas, Calculate, Results, export, and schema-6 reopen workflow. See the [release notes](RELEASE_NOTES_V2_50_0.md).

## What changes in Version 2.51?

Version 2.51 makes Canvas the only permanent model-authoring document. The existing 18-method Calculate catalogue detects advanced model features and routes them internally; completed output opens in the normal categorized Results workspace. The scientific engines and archived method identities are not renamed. The verified unsigned preview is available from GitHub. See the [release notes](RELEASE_NOTES_V2_51_0.md).

## Where did Parameter Table, General SEM, and Exact CB-SEM go?

They are no longer permanent tabs. Open **Advanced Parameter Table** from Canvas, CB-SEM setup, or a corrective setup action. Mediation, moderation, higher-order constructs, moderated mediation, and bounded CB-SEM are configured through Canvas and the relevant Calculate method. Older specialized projects reopen through compatibility adapters without exposing separate tabs.

## How do I draw mediation?

Draw the ordinary substantive paths, for example `X → M → Y`. QuickPLS derives
single, parallel, and serial indirect paths from the model. It does not create a
mediation node, and it excludes covariance, control, measurement, generated,
and interaction-hierarchy relationships from mediation discovery.

## How do I attach a moderator to a path?

Drag the moderator construct onto the focal structural path, right-click or
select the path and choose **Add Moderating Effect…**, or press `M`. QuickPLS
shows a compact `×` anchor and dashed connector from the moderator. The anchor
is presentation-only: the scientific model remains the typed interaction and
the original focal relationship. Select the anchor and press Enter to edit or
Delete to remove only that moderation effect.

## Does QuickPLS support three-way moderation?

Version 2.53 adds bounded source cells for one true three-way term with two-stage
construction, strong hierarchy, all main and pairwise lower-order effects, and
fixed continuous or correctly coded `0/1` binary probes. These cells are independently scoped Standard under
`quickpls_v253_streamlined_integration_v1` after their compact independent reference and consolidated
workflow pass succeeded. Fourth-order effects, HOC
interactions, and three-way moderated mediation remain outside this scope.

## What changes in Version 2.53?

Mediation remains diagram-derived, while moderation becomes visibly attached to
the focal path through a native compact anchor. Drag, context-menu, and keyboard
entry points share the same authoring dialog; generated interaction terms are
hidden from the ordinary Canvas. The public Calculate catalogue remains exactly
18 methods. See the [2.53 release notes](RELEASE_NOTES_V2_53_0.md) for the
implementation and verification status.

## What changes in Version 2.54?

Version 2.54 makes Canvas edits authority-aware across direct gestures, menus,
and Properties; adds Relationships navigation and compact arrangement/Fit
strategies; and makes normal Results use authored labels with stronger wide-table
presentation. It does not add a public method or change a numerical engine,
estimand, Registry cell, or stored result identity. The source diagnostic
recorded 8/9 passing steps, followed by a 69/69 targeted remediation pass; the
final unsigned candidate then passed the isolated 10/10 create → calculate →
Results → save → fresh-reopen packaged journey with zero application-page
external requests and zero console errors. Release packaging and checksums also
passed; code signing is excluded. See the [2.54 release notes](RELEASE_NOTES_V2_54_0.md).

## What changes in Version 2.55?

Version 2.55 makes automatic diagram layout and routing model-aware, adds
presentation-only path bend and moderation-anchor editing, and reuses the full
read-only diagram renderer for PROCESS preview. Calculate dialogs share compact,
responsive behavior, prioritize scientific blockers, and disclose only controls
and fixed policies that the engine actually implements. The exact 18-method
catalogue is unchanged. Formal first diagnostic `20260822T142953Z` at source
`2e3a23f` passed 13 of 14 executed steps; the sole failure was
`frontend_typecheck`, where `src/data/v255NamedSemEvidenceFixtures.test.ts`
reported TypeScript error `TS2339`. The final consolidated diagnostic
`20260823T030939Z` at source `e5723df08b7205ce75f1887c5f4709f235ad893c`
passed 14/14, including 453/453 Vitest suites, 1724/1724 tests, 17/17 rebaseline
assertions, and zero captured console errors. Its report is
`validation/results/v255_consolidated_diagnostics_20260823T030939Z/v255_consolidated_diagnostics.json`,
has SHA-256 `03da7a8e0db2924d0157eb0cb0ca92e841fffd61f470d5cd16ccd58f87fe9b2a`,
and is retained in evidence commit `8a727262c07dd38bae38d8154e1662c78fbb8ee7`.
Both formal records used runner SHA-256
`64969b4eb89c9789e586c372532d89b082766a44661cc4feea66b6cf3a0f9796` and are
separate evidence records, not byte-identical.
Renderer console/page errors fail closed and every candidate/phase/trusted-driver
process role and hash is bound. Exactly one opt-in waiver applies only to the
actual Windows 200% scaling case; its observed-DPI screenshot and receipt remain
required, its status remains `waived`, and the other 54 named cases must pass.
A new candidate, isolated install, full installed-and-portable smoke, evidence
collection and bundling, final audit, and publication remain pending. See the
[2.55 release notes](RELEASE_NOTES_V2_55_0.md).

The superseded final diagnostic `20260823T000930Z`, candidate/install/smoke
attempts `20260823T004848Z` and `20260823T005212Z`, portable probe
`20260822T233111Z`, and all prior 2.55 candidate, install, smoke, diagnostic, or
probe attempts are historical and ineligible.
The focused 17/17 source test covers the exact typed identity correction, not a
packaged executable. The install wrapper separately accepts only the exact
Tauri `UNK` → `NSS` three-byte marker change and rejects any other byte
difference; one new candidate, isolated install, and full smoke run are still
required.

## How do I create or edit a higher-order construct?

Select at least two eligible constructs on Canvas and choose **Model → Higher-Order Construct…** or use the selection context menu. Choose the conceptual direction; QuickPLS derives RR/RF/FR/FF and recommends a valid approach from the model topology. Select the HOC and press Enter, use its context menu, or use Properties to edit it. Activated immutable projects use **Save As Revision** so the source archive remains unchanged.

## Why is PLS model fit labelled descriptive?

SRMR and NFI are approximate fit measures, while raw `d_ULS` and `d_G` do not by themselves establish exact fit. Version 2.52 therefore shows a neutral **Model fit — descriptive** entry and one compact exact-fit state. Open its information button or Model Fit Details for the full explanation. Adapted Bollen–Stine is not exposed as a Calculate option in Version 2.52.

## Is QuickPLS identical to SmartPLS?

No. QuickPLS offers a familiar graphical SEM workflow and independently implements documented methods. It does not import SmartPLS projects, copy proprietary implementation details, or promise identity for undocumented behavior.
