# QuickPLS FAQ

## Is QuickPLS free?

Yes. Official QuickPLS releases are free to use.

## Is QuickPLS open source?

No. QuickPLS is proprietary source-available software. The source can be inspected and contributions can be proposed, but redistribution and derived commercial use are restricted by `LICENSE.md` and `EULA.md`.

## What is the current version?

The current qualified candidate is QuickPLS 2.56.0 at commit `28939b73db8f2284f21ce184050eb3f04110bf94`. It passed 32/32 qualification gates, including installed and portable offline smoke. Its GitHub distribution is QuickPLS 2.56.0 Beta (Unsigned), not a signed or Stable release.

The setup artifact is 231,889,558 bytes with SHA-256 `46b121d252ba236f4e8d72e5e618bbf4542d0cdb0819b834fb9b3d3677599523`. The portable artifact is 79,480,832 bytes with SHA-256 `54abad31daca14dbad16588f85015fb39b030bac28e1cd475a10d4d4573cb2d1`.

## What is MultiMod in Version 2.56?

MultiMod is the new authority-gated moderation and heterogeneity workspace. It contains four separate result families:

1. Categorical moderation and 2-20-group multigroup analysis, including MICOM.
2. FIMIX-PLS and separately named PLS-POS latent-segmentation profiles.
3. Conditional-process analysis for explicitly selected moderated-mediation paths.
4. Observed-data interventional mediation using parametric g-computation.

Each family admits specific, bounded combinations of model features and inference methods. Unsupported intersections return an explicit blocker; QuickPLS does not silently simplify the requested analysis. See [QuickPLS 2.56.0 Features](FEATURES_V2_56_0.md) and the [2.56.0 release notes](RELEASE_NOTES_V2_56_0.md).

## How do I open MultiMod?

Prepare and activate an eligible strict General SEM model, then choose **Moderation & heterogeneity…** from the Model toolbar. Select the relevant family, complete its staged setup, review eligibility and assumptions, preview the targets and execution cost, and calculate. If the button or profile is unavailable, use the displayed blocker to correct the model, data, or requested settings.

## Is every MultiMod method always labelled Standard?

No. **Standard · Release-qualified** is shown only when the executable embeds the exact capability authority matching the qualified source and evidence identity. Generic source or development builds without that complete embedded authority remain **Experimental Labs · Unqualified** or unavailable. Editing a label or building the source alone does not promote a capability.

## Did MultiMod change existing continuous moderation?

No. Continuous moderation V1 is a protected regression and serialization boundary and remains unchanged in 2.56.0.

## Does MultiMod include quadratic or self-moderation?

No. Quadratic/self-moderation is outside the MultiMod development scope. QuickPLS's separate existing nonlinear diagnostic remains outside this suite.

## Is conditional-process analysis causal?

No. MultiMod conditional-process results are associational PLS-SEM estimates. A scalar index of moderated mediation is reported only when the selected indirect effect is affine in exactly one moderator; more complex models receive appropriate derivatives and probe contrasts instead.

## Does the interventional mediation module establish causality?

No. It reports an **assumption-dependent interventional estimate** for its qualified observed-data profile. Interpretation depends on the declared adjustment set, temporal ordering, positivity, model specification, and identification assumptions. QuickPLS never labels the result “causality established.”

## Why does MultiMod block some combinations?

Qualification is attached to exact method/profile cells rather than every Cartesian combination. Examples of deliberately unsupported intersections include HOC plus groups, HOC plus weights, groups plus weights, three-way plus HOC, and studentized inference outside its bounded profile. Blocking these requests prevents QuickPLS from changing the scientific question without the user's knowledge. See [Method Compatibility](METHOD_COMPATIBILITY.md) and [unsupported intersections](MULTIMOD_UNSUPPORTED_INTERSECTIONS_V1.md).

## Does QuickPLS import SmartPLS projects?

No.

## Does QuickPLS guarantee identical results to SmartPLS?

No. QuickPLS implements published methods independently and validates them against documented formulas, fixtures, simulations, and permitted reference engines. It does not reproduce proprietary implementation details, promise identical undocumented behavior, or claim full SmartPLS feature parity.

## Does QuickPLS require internet?

No account, cloud service, or remote computation is required. QuickPLS analytical workflows run locally after download. The Microsoft-managed WebView2 runtime may make its own background service connections unless Windows network controls block them.

## Does QuickPLS require R or Python?

No. R, Python, and their packages are validation-only development tools. They are not required by the shipped Windows application.

## Why does Windows warn about the installer?

The 2.56.0 Beta artifacts are unsigned, so Windows SmartScreen may identify the publisher as unknown. Download only from the official QuickPLS GitHub release page and verify the file before running it. The qualified setup SHA-256 is `46b121d252ba236f4e8d72e5e618bbf4542d0cdb0819b834fb9b3d3677599523`; the qualified portable SHA-256 is `54abad31daca14dbad16588f85015fb39b030bac28e1cd475a10d4d4573cb2d1`.

In PowerShell, run `Get-FileHash -Algorithm SHA256 -LiteralPath '<downloaded-file>'` and compare the full value. A checksum match verifies the artifact bytes; it is not a substitute for code signing.

## What export formats are supported?

Canonical General SEM results support CSV, XLSX, JSON, self-contained HTML, PDF, SVG, and PNG. Other completed methods expose only formats appropriate to their result type. MultiMod raw Arrow evidence export is available only after a strict Archive V6 reopen; table-only results do not invent a diagram export.

## How are large MultiMod results saved?

Archive V6 stores large evidence in compressed, SHA-256-bound Arrow sidecars inside the `.qpls` archive. Missing, altered, duplicate, schema-mismatched, or identity-mismatched sidecars cause reopening to fail rather than yielding an incomplete scientific result. QuickPLS warns when a predicted result sidecar exceeds 128 MiB and rejects a run above 512 MiB.

## Why are some methods unavailable?

QuickPLS evaluates each method against the current dataset, model, requested estimand, inference choice, and embedded qualification authority. Setup reports whether a method is eligible and provides the exact reason and next action when it is not.

## How do I draw mediation?

Draw the ordinary substantive paths, for example `X → M → Y`. QuickPLS derives eligible single, parallel, and serial indirect paths from the model. It does not create a mediation node, and it excludes covariance, control, measurement, generated, and interaction-hierarchy relationships from mediation discovery. In MultiMod conditional-process analysis, every analyzed indirect path must also be selected explicitly.

## How do I attach a moderator to a path?

Drag the moderator construct onto the focal structural path, right-click or select the path and choose **Add Moderating Effect…**, or press `M`. QuickPLS shows a compact `×` anchor and dashed connector from the moderator. The anchor is presentation-only: the scientific model remains the typed interaction and original focal relationship. Select the anchor and press Enter to edit, or Delete to remove only that moderation effect.

## Does QuickPLS support three-way moderation?

Yes, within bounded qualified profiles. A three-way cell requires exactly one three-way term with its complete main-effect and pairwise lower-order closure. The exact limits differ between MGA, segmentation, and conditional-process profiles. Fourth-order and general k-way interactions remain unsupported.

## How do I create or edit a higher-order construct?

Select at least two eligible constructs on Canvas and choose **Model → Higher-Order Construct…** or use the selection context menu. Choose the conceptual direction; QuickPLS derives RR/RF/FR/FF and recommends an eligible approach from the model topology. Select the HOC and press Enter, use its context menu, or use Properties to edit it. Activated immutable projects use **Save As Revision** so the source archive remains unchanged.

## Why is PLS model fit labelled descriptive?

SRMR and NFI are approximate fit measures, while raw `d_ULS` and `d_G` do not by themselves establish exact fit. QuickPLS therefore shows a neutral **Model fit: descriptive** entry and an explicit exact-fit state only where the selected method supports it.

## What was added in Version 2.50?

Version 2.50 integrated bounded General SEM mediation and simultaneous moderation, higher-order PLS point/bootstrap, two-way moderated mediation, recursive common-factor CB-SEM ML, and recursive-SEM case bootstrap into the same Canvas, Calculate, Results, export, and schema-6 reopen workflow. See the [2.50 release notes](RELEASE_NOTES_V2_50_0.md).

## What changed in Version 2.51?

Version 2.51 made Canvas the permanent model-authoring document and routed advanced model features through the existing Calculate catalogue. See the [2.51 release notes](RELEASE_NOTES_V2_51_0.md).

## What changed in Version 2.53?

Version 2.53 kept mediation diagram-derived and attached moderation visibly to its focal path through a compact native anchor, with shared drag, context-menu, and keyboard authoring. See the [2.53 release notes](RELEASE_NOTES_V2_53_0.md).

## What changed in Version 2.54?

Version 2.54 made Canvas edits authority-aware, added Relationships navigation and layout strategies, and strengthened authored-label and wide-table presentation without changing a numerical engine or stored result identity. See the [2.54 release notes](RELEASE_NOTES_V2_54_0.md).

## What changed in Version 2.55?

Version 2.55 improved model-aware diagram layout and routing, presentation-only connector editing, PROCESS preview rendering, compact Calculate dialogs, blocker prioritization, and bundled example models. Earlier 2.55 evidence notes are historical; 2.56.0 has its own qualified candidate and evidence. See the [2.55 release notes](RELEASE_NOTES_V2_55_0.md).

## Is QuickPLS identical to SmartPLS?

No. QuickPLS offers a graphical SEM workflow and independently implements documented methods. It does not import SmartPLS projects, copy proprietary implementation details, or promise identity for undocumented behavior.
