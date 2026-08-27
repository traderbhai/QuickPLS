# QuickPLS 2.56.0 User Guide

QuickPLS is an offline Windows desktop application for structural equation
modeling. It does not require an account, cloud service, R, or Python at
runtime.

The GitHub distribution is **QuickPLS 2.56.0 Beta (Unsigned)**, intended for
technical evaluation. Download it from the
[`v2.56.0-beta.1` release](https://github.com/traderbhai/QuickPLS/releases/tag/v2.56.0-beta.1).
The executables are not Authenticode-signed, so Windows may display an
unknown-publisher or Microsoft Defender SmartScreen warning. Verify the
published SHA-256 checksum before running a downloaded file.

This guide does not claim a signed or stable-channel release, unrestricted
method coverage, SmartPLS project compatibility, or numerical identity with
another product.

## Common workflow

The normal journey is:

`Launcher → Data → Model → Calculate → Results → Save/Export`

1. In **Launcher**, create a project, open a `.qpls` project, or open a bundled
   sample.
2. In **Data**, import and inspect the dataset. Resolve incorrect headers,
   nonnumeric analysis columns, constants, missing-value problems, and reported
   quality issues.
3. In **Model**, create constructs, assign indicators, draw structural paths,
   and author only the advanced terms required by the hypothesis.
4. Choose **Calculate**. QuickPLS checks the exact dataset, model, recipe,
   method profile, and release authority before enabling a run.
5. Monitor the calculation. A cancelled or failed run is never presented as a
   completed scientific result.
6. Review the completed result in **Results**, save the project, and use
   **Export Results** for the formats admitted by that result family.

Save early. A scientific edit to an activated calculation-ready project uses a
source-preserving revision instead of silently changing the authority behind
an existing result.

## Launcher, Data, and Model

Use Launcher, **File → Open Project…**, or **Open** to open a `.qpls` file.
Recent projects appear in Launcher. Bundled samples are useful for learning the
interface because they include data, a model, and a completed result.

In **Data**, verify row and variable counts, column names and numeric types,
missing values, constants, and scale metadata. For categorical MGA or grouped
conditional-process analysis, keep the grouping column outside the model
indicators. For weighted profiles, retain the intended case-weight or frequency
column.

In **Model**, use Canvas and the **Indicators**, **Constructs**, and
**Relationships** navigator sections. Canvas layout is presentation metadata
and does not change scientific identities.

To author moderation, drag a moderator onto an eligible structural path, or
select/right-click the path and choose **Add Moderating Effect…**. The `M` key
opens the same action. A small `×` anchor and dashed connector identify the
effect. Select the anchor and press Enter to edit it or Delete to remove it. A
bounded three-way term is created by adding a second moderator to an eligible
parent two-way interaction; the required main and pairwise lower-order terms
must be present.

For a higher-order construct, select eligible component constructs and choose
**Model → Higher-Order Construct…**. QuickPLS derives the RR/RF/FR/FF type and
offers only admitted construction approaches.

## Standard and Experimental Labs

MultiMod displays one of two exact authority labels:

- **Standard · Release-qualified** means the executable contains the embedded
  authority for the exact source commit, method profile, procedure cell,
  schemas, and qualification evidence used by the request. It is not a broad
  promise that adjacent or combined methods are supported.
- **Experimental Labs · Unqualified** means the surface does not carry release
  authority. A Labs result must not be reported as Standard, and publication
  exports can remain disabled.

The qualified 2.56.0 Beta package is expected to show **Standard ·
Release-qualified** for admitted MultiMod cells. A source build, altered build,
or package without complete embedded authority is expected to remain Labs-only
or unavailable. Enabling Experimental Labs cannot manufacture Standard
authority.

## Open the MultiMod suite

MultiMod is additive to the existing General SEM workflow. It does not replace
continuous moderation V1.

1. Open or create a calculation-ready General SEM project with data and a valid
   model.
2. If offered, use **Calculate → Save and activate project…**, then continue
   from the newly activated project.
3. Return to **Model** and select **Moderation & heterogeneity…** in the Canvas
   toolbar.
4. Confirm that the workspace says **Moderation and heterogeneity suite** and
   inspect its authority badge.
5. Choose **Categorical moderation**, **Latent segmentation**, **Conditional
   process**, or **Interventional mediation**.

Every tab validates the exact request continuously. **Stage Recipe V4** stores
the configuration without running it. **Calculate** is enabled only when native
preflight says that exact profile is executable. **Predicted result sidecar**
warns above 128 MiB and blocks a run above 512 MiB.

## Categorical moderation: MICOM and 2-20-group MGA

Use **Categorical moderation** when the moderator is a typed grouping variable
and the research question concerns group differences.

1. Under **1. Groups**, choose a **Grouping column** that is not a model
   indicator and select 2-20 mutually exclusive values. Unselected levels and
   unusable rows are excluded with stable row tokens and reasons.
2. Review complete-case counts. Each group needs at least 10 complete model
   cases. QuickPLS warns below 30 cases, warns above 2:1 imbalance, and rejects
   imbalance above 10:1.
3. Under **2. Comparison and profile**, choose the exact **Profile envelope**:
   ordinary recursive General SEM PLS, multiple two-way moderation, bounded
   three-way moderation, bounded two-way moderated mediation, multiple
   nonnested HOCs, positive case-weighted PLS, positive-integer
   frequency-weighted PLS, or reflective PLSc.
4. Enter the weight column when required. With more than two groups, a
   max-spread omnibus permutation precedes pairwise follow-up. All 190 pairs at
   20 groups require explicit heavy-run confirmation.
5. Complete **MICOM Step 1: configural invariance review**. QuickPLS calculates
   Steps 2-3 pairwise and does not invent an omnibus MICOM conclusion. Partial
   measurement invariance is required before interpreting affected structural
   differences.
6. Under **4. Inference and multiplicity**, select the needed procedures:
   pairwise MICOM, size-preserving permutation, Henseler PLS-MGA directional
   probability, bootstrap BC difference interval, pooled-variance parametric,
   Welch-Satterthwaite, or K-group inverse-variance Wald.
7. Choose 5,000-10,000 draws, the alternative, multiplicity method, and exact
   **Parameters to compare**. Holm FWER is the confirmatory default. One-sided
   A-minus-B alternatives must be declared before calculation.
8. Review cost and choose **Calculate**.

In Results, inspect group eligibility, pairwise MICOM, group-specific
parameters, omnibus comparisons, signed left-minus-right pairwise comparisons,
excluded rows, and ledgers. Henseler's output remains a directional probability,
not an ordinary two-sided p value; the bootstrap interval is BC, not BCa.

## Latent segmentation: FIMIX-PLS and PLS-POS V2

The **Latent segmentation** workflow separates discovery from inference and
never chooses K for the analyst.

### Discover candidates

1. Select **Discover candidates (never auto-select K)**.
2. Choose **P0: structural only**, **P2: multiple two-way interactions**, or
   **P23: bounded three-way closure**.
3. Select the discovery algorithms. FIMIX-PLS V2 admits P0/P2/P23. **PLS-POS
   published V2** is P0-only; interaction profiles use the separately named
   **PLS-POS destination-scored interactions V2**.
4. Select candidate K values from 2 through 5. K=1 remains a read-only pooled
   reference.
5. Choose **Calculate**, then inspect candidate diagnostics, criteria, shares,
   convergence, multi-start stability, blockers, and the pooled baseline.

Criteria support judgment; they do not select a solution automatically.

### Lock the solution and run inference

1. Return with the completed discovery result available and select **Lock
   reviewed algorithm and K for inference**.
2. Choose a converged **Locked algorithm** and **Locked K**.
3. Confirm that the completed diagnostics were reviewed and this exact method/K
   is locked.
4. Choose 500-10,000 **Bootstrap replicates**.
5. For PLS-POS interaction contrasts, optionally request the pooled common
   metric and complete its configural-invariance checklist.
6. Choose **Calculate** again.

Bootstrap keeps algorithm and K fixed. Ambiguous or non-majority label matches
are rejected rather than replaced. If PLS-POS common-metric comparability fails,
QuickPLS retains segment-local descriptive output but suppresses invalid
between-segment gamma, delta, slope, and effect tests. FIMIX and POS remain
separate analyses.

## Conditional process: broader moderated mediation

Use **Conditional process** for explicitly selected indirect paths whose edge
coefficients vary with authored moderators. These are associational PLS
estimates and receive no causal label.

1. Choose the exact **Profile envelope**:
   - Multiple two-way · percentile: 1-8 interactions, 1-8 paths of 2-6 edges,
     and up to 4 moderators.
   - Multiple two-way · full delete-one BCa: the same structural envelope with
     a complete delete-one jackknife.
   - Studentized bounded profile: up to 4 interactions, 1-2 paths of 2-4 edges,
     up to 3 moderators, 27 probes, and 256 targets.
   - Bounded three-way · percentile: exactly one three-way term with complete
     lower-order closure and 1-4 paths of 2-5 edges.
   - Multiple HOC · percentile: up to 4 pairwise-disjoint nonnested HOCs and 2
     two-way interactions.
   - Grouped · stratified percentile: 2-20 groups, up to 4 interactions, and 4
     paths of 2-4 edges.
   - Positive case weighted or positive-integer frequency weighted: the
     corresponding bounded two-way profile.
2. Under **1. Explicit indirect paths**, select every intended ordered path.
   QuickPLS never guesses or shortens a path.
3. Under **2. Authored interaction terms**, select terms already created on
   Canvas.
4. Review **3. Frozen original-sample probes**. Defaults are standardized
   `−1, 0, +1`, with up to five values per moderator. Use a Cartesian grid or
   **Explicit joint tuples**; joint tuples replace the grid. **Finite probe
   contrasts** retain their declared left-minus-right direction.
5. Raw units are allowed only for an observed continuous or binary
   single-indicator moderator. Select **Raw observed units** and use **Prepare
   exact metric**. Composite and HOC probes remain standardized.
6. Select the profile-specific HOCs, groups, or weight column and choose the
   requested estimands.
7. Enter **Outer resamples**. Studentized inference also requires **Inner
   resamples** and is capped at one million inner refits. One-sided alternatives
   are available only in profiles that admit them and must be predeclared.
8. Review conditional cells, inferential targets, interval type, one shared
   ledger, and cost; then choose **Calculate**.

All targets use the same bootstrap ledger. A scalar index is reported only when
the selected indirect effect is affine in one moderator. Both-stage,
multiple-moderator, and three-way effects instead receive correctly named local
derivatives and finite contrasts.

## Interventional mediation: observed-data g-computation

This is a separate observed-data workflow. Every result uses the wording
**assumption-dependent interventional estimate**; QuickPLS never states that
causality is established.

1. Under **Observed treatment and outcome**, select a directly observed
   treatment and continuous outcome.
2. Choose **Binary control → treated** or **Continuous x0 → x1** and enter the
   treatment contrast.
3. Select one to three ordered **Observed continuous mediators**.
4. Select observed baseline numeric/binary moderators and a **Required nonempty
   adjustment set**.
5. Enter **Explicit causal paths**, one per line, using a stable ID and `>`
   separated variables, for example:

   ```text
   path:treatment-mediator-outcome = treatment_id > mediator_id > outcome_id
   ```

   A path contains 2-4 directed edges.
6. Review **Explicit observed-data equations**. Required predecessor and
   adjustment main effects cannot be removed. Add only allowed recursive
   pairwise terms after selecting their main effects.
7. Complete every **Identification and positivity declaration**. It records
   analyst review; it does not prove the assumptions.
8. Choose 500-10,000 **Bootstrap replicates**, review cost, and select
   **Calculate**.

The first profile excludes latent/composite/HOC roles, groups, weights, natural
or cross-world effects, recanting-witness settings, and exposure-induced
mediator-outcome confounding. Positivity and rank failures block execution.

## Monitor, cancel, retry, and resume

The **MultiMod native job** panel shows state, phase, progress, warnings, and
validated cache receipts.

- **Cancel safely** publishes no partial scientific result.
- A cancelled or failed MGA run with a validated execution cache can offer
  **Resume MGA shards**.
- An archive-ready cache can offer **Resume publication**.
- Estimation resume is not available for other MultiMod families before the
  archive-ready boundary; cancellation there restarts the exact execution.
- A failed request without a reusable cache can offer **Retry exact request**.
- **Dismiss job state** clears a reviewed terminal state.

Resume and retry revalidate identities. A stale or mismatched cache is rejected.

## Results

A strictly reopened completed result says **Complete result payload** and shows
its family, profile, authority, and result ID. The result tabs are:

- **Eligibility**: admission and interpretation gates;
- **Diagnostics**: invariance, convergence, comparability, and positivity;
- **Estimates**: completed point estimates;
- **Inference**: intervals, probabilities, omnibus tests, and contrasts;
- **Exclusions**: rows, blockers, warnings, and scope boundaries;
- **Failures**: requested, usable, and failed resampling evidence;
- **Provenance**: method, recipe, model, dataset, engine, seed, capability, and
  digest identities; and
- **Sidecars**: the required Arrow evidence inventory.

If result-level gates did not pass, scientific Estimates and Inference are
withheld. Large tables are paged and virtualized and remain keyboard accessible.

## Save, reopen, and Archive V6

QuickPLS 2.56 uses Archive V6 `.qpls` files and reads compatible V5 projects
without reinterpreting historical method identities. A completed MultiMod
result is atomically attached, saved, and strictly reopened before it becomes
the resident result.

Large evidence is stored as compressed, SHA-256-checked Arrow sidecars. These
can hold posteriors, memberships, assignments, start traces, bootstrap ledgers,
target vectors, validity bitmaps, and jackknife summaries.

- A predicted sidecar above 128 MiB requires review.
- An aggregate above 512 MiB is rejected before execution.
- A missing, altered, duplicate, schema-mismatched, or identity-mismatched
  required sidecar makes strict reopen fail.
- Do not edit a `.qpls` ZIP or sidecar manually. Return to the unchanged
  original or a verified backup after an integrity failure.

## Export

A complete Standard canonical result can offer CSV, XLSX, JSON, self-contained
HTML, PDF, SVG, and PNG through **Export Results**. Exact choices depend on the
result family. Exports retain the source result ID and provenance.

The **Sidecars** tab can offer **Optional validated Arrow evidence** for eligible
posterior, membership, assignment, replicate-ledger, or target-vector payloads.
Each raw export strictly reopens and validates the archive and never overwrites
an existing file.

Publication can be withheld for Labs results, incomplete results, or a missing
strict post-reopen canonical projection. An export cannot override an on-screen
interpretation blocker.

## Strict supported boundaries

QuickPLS fails closed when a request has no exact qualified cell. It does not
silently drop terms, change inference, shorten paths, or remove weights.

- Continuous moderation V1 remains separate and unchanged.
- Quadratic/self-moderation is outside MultiMod.
- One Recipe V4 analysis contains only one additive MultiMod configuration.
- HOC+groups, HOC+weights, groups+weights, three-way+HOC,
  nested/overlapping HOCs, mixed HOC approaches, and more than one three-way
  term are unsupported.
- Studentized inference is limited to its bounded profile. BCa requires the
  complete delete-one jackknife and has no percentile fallback.
- Raw-unit probes are unsupported for composites and HOCs.
- Case weights are analytic weights, not survey weights. Sampling weights,
  clusters, strata, PPS, and survey-design claims are unsupported.
- PLS-POS published V2 is P0 structural-only; P2/P23 use the separately named
  destination-scored interaction method.
- Conditional-process results are associational and not causal.
- Interventional mediation admits only its observed linear V1 envelope and
  remains assumption-dependent.

## Troubleshooting

### `Moderation & heterogeneity…` is not visible

The project lacks strict calculation-ready General SEM authority, or the
executable lacks MultiMod authority. Use **Calculate → Save and activate
project…** when offered and reopen the saved project. Enabling Labs does not
upgrade an unqualified build to Standard.

### Calculate is disabled

Read the nearby stable error code and corrective text. Common causes include an
incomplete checklist, invalid group/weight column, insufficient cases, an
unsupported intersection, no explicit path, a missing raw probe metric, no
stable segmentation lock candidate, or cost above 512 MiB.

### A group is missing or rejected

Only selected typed levels are analyzed. Confirm the column is not an indicator,
each group has at least 10 complete model cases, and imbalance is not above
10:1. Review **Exclusions** for row-level reasons.

### Estimates or inference are withheld

Inspect **Eligibility**, **Exclusions**, **Failures**, and **Sidecars**.
Invariance, common-metric comparability, positivity, usable-ledger, archive, or
authority gates can suppress scientific output.

### Reopen reports a sidecar integrity failure

Stop and reopen the original unchanged file or a verified backup. QuickPLS
intentionally rejects missing, altered, duplicated, schema-mismatched, and
identity-mismatched sidecars.

### Windows warns about the download

QuickPLS 2.56.0 Beta (Unsigned) is not Authenticode-signed. Download only from
the official GitHub release and compare its SHA-256 with the published value. A
checksum verifies bytes; it does not establish a signed publisher identity.

## Further reading

- [Quick Start](QUICK_START.md)
- [QuickPLS 2.56.0 Release Notes](RELEASE_NOTES_V2_56_0.md)
- [QuickPLS 2.56.0 Features](FEATURES_V2_56_0.md)
- [Method Compatibility](METHOD_COMPATIBILITY.md)
- [MultiMod boundaries](MULTIMOD_BOUNDARIES_AND_QUALIFICATION_V1.md)
- [Unsupported intersections](MULTIMOD_UNSUPPORTED_INTERSECTIONS_V1.md)
- [FAQ](FAQ.md)
