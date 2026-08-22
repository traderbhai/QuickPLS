# QuickPLS Quick Start

This guide follows the Version 2.55 source workflow:
`Launcher → Data → Canvas → Calculate → Results`. After renderer/process
trust-chain remediation, the formal source reports `20260822T082854Z` at source
`b4a73f7`
and `20260822T083409Z` at source `1f0fa28` each passed 14/14 using the identical
diagnostic script and suite/invocation contract. They are separate evidence
records and are not claimed to be byte-identical reports. Renderer console/page
errors now fail closed, attach-only phases use fresh wrapper-owned processes,
and candidate/phase/trusted-driver process roles and hashes are bound. A fresh
provenance-bound unsigned 2.55 candidate build, isolated install, installed and
portable packaged journeys, evidence collection and bundling, and publication
remain pending; the latest downloadable public pre-release remains `v2.54.0` and
code signing is excluded.

Candidate `20260822T070205Z` and every earlier candidate or install attempt are
historical and ineligible because the trust-chain remediation and current formal
validation postdate them.
The exact typed post-hoc authority correction passed its focused identity suite
17/17 but has not yet been validated in a packaged executable. The install
wrapper separately accepts only Tauri's exact three-byte `UNK` → `NSS` marker
transition and rejects every other byte difference. Fresh candidate, install,
and smoke evidence is still required. Only the actual Windows 200% scaling case
may use the product-owner-approved opt-in waiver; its real DPI screenshot and
receipt remain required and the other 54 named cases must pass. If the exact
registered old installation must be removed for the isolated install, only its
registered uninstaller may be used and project files, recovery data, and
QuickPLS application user data must remain untouched. Portable evidence does
not replace the installed journey.

## 1. Start or open a project

Open QuickPLS and choose one:

- `Open demo project` to try a bundled workflow.
- `Import dataset` to start with CSV, TSV, XLSX, or SAV data.
- `Open project` to load an existing `.qpls` project.

Save early so autosave and recovery can preserve the project.

## 2. Import and inspect data

Open `Data`, import the dataset, and check row/variable counts, missing values, nonnumeric or constant columns, headers, and selected-variable metadata. Matrix input is available only for methods whose documented scope permits it.

## 3. Build the model

Open `Model` and use Canvas to add constructs, assign indicators, draw structural paths or supported covariances, arrange the diagram, and validate it. Canvas is the only permanent model-authoring document. For common-factor CB-SEM, set the applicable construct representation and open the **Advanced Parameter Table** from Canvas or CB-SEM setup when fixed/free parameters, bounds, or equality labels need attention.

Use the model navigator's **Indicators**, **Constructs**, and **Relationships**
sections to find and focus model elements. Arrange supports tidy selection,
alignment, distribution, and whole-model direction; Fit supports structure,
all, and selection. Indicator side, free placement, pins, routes, and label
offsets remain presentation metadata and do not change calculation authority.

For a higher-order construct (HOC), select at least two eligible constructs and choose **Model → Higher-Order Construct…** or the same command from the selection context menu. Choose the conceptual direction; QuickPLS derives the RR/RF/FR/FF type and recommends a valid construction approach. Select the HOC and press Enter, use its context menu, or use Properties to edit it. The permanent Canvas shows only the HOC marker; detailed settings stay in the dialog.

Mediation needs no special object: draw substantive paths such as `X → M → Y`.
QuickPLS ignores covariance, control, measurement, generated, and interaction-
hierarchy relationships when discovering indirect paths.

For moderation, drag a moderator construct onto an eligible structural path, or
select/right-click the path and choose **Add Moderating Effect…**. Pressing `M`
provides the same keyboard entry point. The small `×` anchor and dashed connector
show which path is moderated; they are visual only and never become scientific
paths. Select the anchor and press Enter to edit it or Delete to remove only that
effect. Adding a second moderator to an eligible parent interaction defines a
true three-way term when its bounded scope is available. Advanced changes use
**Save As Revision** so the source project remains unchanged.

## 4. Calculate

Choose `Calculate`. The searchable catalogue still contains 18 methods. QuickPLS evaluates the resident model, data, settings, and exact Registry cells, then shows only the settings relevant to the selected method.

- Choose **PLS Algorithm** for point estimation. Eligible mediation, two-way moderation, and higher-order output is detected from the model; an HOC routes to its existing point cell. The bounded three-way point cell is Standard for its exact one-term strong-hierarchy predicate.
- Choose **Bootstrapping** for inference. Eligible multiple mediation, simultaneous two-way moderation, higher-order, and bounded moderated-mediation calculations route automatically. The exactly-one-path mediation and bounded three-way bootstrap cells are Standard for their exact predicates.
- Choose **CB-SEM** for bounded common-factor ML, with case bootstrap selected inside its method settings when eligible.
- Use Data's `Analyze…` command for compatible model-free methods.

Detected feature summaries explain what will be calculated. An unavailable action shows the corrective reason and, where possible, a direct action such as opening the Advanced Parameter Table or creating a calculation-ready revision. A blocked setup never modifies the diagram. Standard methods do not require the Experimental Labs preference.

## 5. Monitor or cancel

Start the calculation and follow native progress. Cancellation stops publication: no partial analytical result or partial archive attachment is created.

## 6. Review results

After completion, QuickPLS opens the verified canonical result in `Results`. Its searchable sidebar contains only groups owned by that run, which may include Overview, Measurement Model, Structural Model, Direct/Indirect/Total Effects, Moderation, Higher-Order Constructs, Moderated Mediation, CB-SEM Parameters, Model Fit, Bootstrap Inference, and Run Details.

Normal Results use the construct, indicator, and relationship names saved with
the calculated model. Primary identity columns remain visible while wide tables
scroll, scientific values align numerically, and interval tables expose their
confidence level. Generated IDs and hashes remain in Run Details rather than
the researcher-facing table.

PLS fit is shown as **Model fit — descriptive**. Open its information button or Model Fit Details for interpretation. A compact exact-fit state says whether exact-fit inference was not run, is available, is partial or unavailable, or failed; Version 2.52 does not add adapted Bollen–Stine to Calculate.

## 7. Export and reopen

Use `Export` from Results. Canonical General SEM results support:

- CSV;
- XLSX;
- self-contained HTML;
- PDF;
- SVG; and
- PNG.

Other result families display only their compatible formats. Save the project, close it, and reopen it to restore the same verified model, recipe, result, tables, and provenance.

For the exact supported boundaries and current release status, see [Method Compatibility](METHOD_COMPATIBILITY.md), the [Version 2.54 Release Notes](RELEASE_NOTES_V2_54_0.md), and the [Version 2.55 Release Notes](RELEASE_NOTES_V2_55_0.md).
