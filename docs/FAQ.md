# QuickPLS FAQ

## Is QuickPLS free?

Yes. Official QuickPLS releases are free to use.

## Is QuickPLS open source?

No. QuickPLS is proprietary source-available software. The source can be inspected and contributions can be proposed, but redistribution and derived commercial use are restricted by `LICENSE.md` and `EULA.md`.

## Does QuickPLS import SmartPLS projects?

No.

## Does QuickPLS guarantee identical results to SmartPLS?

No. QuickPLS implements published methods independently and validates against documented formulas, fixtures, simulations, and reference engines where permitted.

## Does QuickPLS require internet?

No account, cloud service, or remote computation is required. QuickPLS analytical workflows run locally after download. The Microsoft-managed WebView2 runtime may make its own background service connections unless Windows network controls block them.

## Does QuickPLS require R?

No. R/Rscript and R packages are validation-only development tools.

## Why does Windows warn about the installer?

The public `v2.53.0` GitHub prerelease is unsigned. Download it only from the [official release page](https://github.com/traderbhai/QuickPLS/releases/tag/v2.53.0), compare its SHA-256 value with the attached checksum file, and expect Windows SmartScreen to identify the publisher as unknown. Code signing remains a separate release-hardening step.

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

## How do I create or edit a higher-order construct?

Select at least two eligible constructs on Canvas and choose **Model → Higher-Order Construct…** or use the selection context menu. Choose the conceptual direction; QuickPLS derives RR/RF/FR/FF and recommends a valid approach from the model topology. Select the HOC and press Enter, use its context menu, or use Properties to edit it. Activated immutable projects use **Save As Revision** so the source archive remains unchanged.

## Why is PLS model fit labelled descriptive?

SRMR and NFI are approximate fit measures, while raw `d_ULS` and `d_G` do not by themselves establish exact fit. Version 2.52 therefore shows a neutral **Model fit — descriptive** entry and one compact exact-fit state. Open its information button or Model Fit Details for the full explanation. Adapted Bollen–Stine is not exposed as a Calculate option in Version 2.52.

## Is QuickPLS identical to SmartPLS?

No. QuickPLS offers a familiar graphical SEM workflow and independently implements documented methods. It does not import SmartPLS projects, copy proprietary implementation details, or promise identity for undocumented behavior.
