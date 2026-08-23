# QuickPLS Quick Start

This guide follows the Version 2.55 source workflow:
`Launcher → Data → Canvas → Calculate → Results`. Formal first diagnostic
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
separate evidence records, not byte-identical. A new provenance-bound unsigned
2.55 candidate build, isolated install, full installed-and-portable smoke,
evidence collection and bundling, final audit, and publication remain pending;
the latest downloadable public pre-release remains `v2.54.0` and code signing
is excluded.

The superseded final diagnostic `20260823T000930Z`, candidate/install/smoke
attempts `20260823T004848Z` and `20260823T005212Z`, portable probe
`20260822T233111Z`, and all prior 2.55 candidate, install, smoke, diagnostic, or
probe attempts are historical and ineligible.
The exact typed post-hoc authority correction passed its focused identity suite
17/17 but has not yet been validated in a packaged executable. The install
wrapper separately accepts only Tauri's exact three-byte `UNK` → `NSS` marker
transition and rejects every other byte difference. One new candidate,
isolated install, and full smoke are still required. Exactly one case—the actual Windows 200%
scaling case—may use the opt-in waiver; its real DPI screenshot and receipt
remain required, its status remains `waived`, and the other 54 named cases must
pass. If an existing registered installation must be removed for the isolated
install, only its exact registered uninstaller may be used and project files,
recovery data, and QuickPLS application user data must remain untouched.
Installed and portable journeys are both mandatory; neither substitutes for
the other.

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
