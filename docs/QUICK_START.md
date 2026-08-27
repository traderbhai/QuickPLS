# QuickPLS 2.56.0 Quick Start

QuickPLS runs locally on Windows. The GitHub distribution is **QuickPLS 2.56.0
Beta (Unsigned)**, intended for technical evaluation. Download it from the
[`v2.56.0-beta.1` release](https://github.com/traderbhai/QuickPLS/releases/tag/v2.56.0-beta.1).
It is not Authenticode-signed, so Windows may display an unknown-publisher or
Microsoft Defender SmartScreen warning. Verify the published SHA-256 checksum
before running it.

The basic workflow is:

`Launcher → Data → Model → Calculate → Results → Save/Export`

## 1. Create or open a project

In **Launcher**, create a project, open a recent project, choose **Open** for an
existing `.qpls` file, or open a bundled sample.

If Calculate offers **Save and activate project…**, use it to create the strict
calculation-ready General SEM authority and continue from the activated project.

## 2. Import and check data

Open **Data** and import the dataset. Check row and variable counts, headers,
numeric types, missing values, and constants. Keep an MGA grouping column
outside the model indicators. Retain the intended weight/frequency column for
a weighted profile.

## 3. Build the model

Open **Model**. Create constructs, assign indicators, and draw structural paths.
Author the required moderating effects on Canvas before using Conditional
Process or an interaction segmentation profile.

Drag a moderator onto an eligible path, or select/right-click the path and
choose **Add Moderating Effect…**. The `M` key is the shortcut. Use
**Higher-Order Construct…** only when the selected profile admits HOCs.

## 4. Open MultiMod

With a strict calculation-ready project active, select **Moderation &
heterogeneity…** in the Model Canvas toolbar. Confirm the workspace heading
**Moderation and heterogeneity suite** and its badge:

- **Standard · Release-qualified** carries exact embedded authority for the
  admitted profile/procedure cell.
- **Experimental Labs · Unqualified** is not Standard and can withhold
  publication exports. Enabling Labs cannot create Standard authority.

## 5. Configure one family

### Categorical moderation

1. Choose a non-indicator **Grouping column** and select 2-20 groups.
2. Select the exact **Profile envelope** and any required weight column.
3. Complete **MICOM Step 1: configural invariance review**.
4. Select procedures, 5,000-10,000 draws, alternative, multiplicity, and exact
   parameters to compare.
5. Confirm the heavy run if all 190 pairs at 20 groups are requested.
6. Review **Predicted result sidecar** and choose **Calculate**.

Each group requires at least 10 complete cases. QuickPLS warns below 30 or
above 2:1 imbalance and rejects imbalance above 10:1. With 3-20 groups, a
max-spread omnibus permutation precedes pairwise follow-up; MICOM remains
pairwise.

### Latent segmentation

1. Select **Discover candidates (never auto-select K)**.
2. Choose P0 structural-only, P2 multiple two-way, or P23 bounded three-way
   closure and compatible FIMIX-PLS/PLS-POS algorithms.
3. Select candidate K values from 2 through 5 and calculate. K=1 is a read-only
   pooled baseline.
4. Inspect candidate diagnostics and criteria.
5. Return and select **Lock reviewed algorithm and K for inference**.
6. Choose the converged method/K, confirm the lock, select 500-10,000 bootstrap
   replicates, and calculate again.

Bootstrap keeps algorithm and K fixed. A failed PLS-POS common-metric gate keeps
descriptive output but suppresses invalid between-segment inference.

### Conditional process

1. Select the exact percentile, BCa, studentized, three-way, HOC, grouped,
   case-weighted, or frequency-weighted **Profile envelope**.
2. Select every intended item under **1. Explicit indirect paths**. QuickPLS
   never guesses a path.
3. Select the Canvas-authored terms under **2. Authored interaction terms**.
4. Review **3. Frozen original-sample probes**. Defaults are standardized
   `−1, 0, +1`. Add joint tuples or finite contrasts when required.
5. Raw units require an observed single-indicator moderator and **Prepare exact
   metric**.
6. Select estimands, outer resamples, and any required studentized inner
   resamples. Review targets, shared ledger, and cost, then calculate.

Conditional-process output is associational, not causal. A scalar index appears
only when the selected indirect effect is affine in one moderator.

### Interventional mediation

1. Select directly observed treatment and outcome variables.
2. Choose **Binary control → treated** or **Continuous x0 → x1**.
3. Select ordered continuous mediators, baseline moderators, and a nonempty
   adjustment set.
4. Enter each **Explicit causal path**, for example:

   ```text
   path:id = treatment_id > mediator_id > outcome_id
   ```

5. Review the explicit observed-data equations and allowed pairwise terms.
6. Complete every **Identification and positivity declaration**.
7. Select 500-10,000 bootstrap replicates, review cost, and calculate.

Results are labelled **assumption-dependent interventional estimate**. Passing
the checklist and support screen does not establish causality.

## 6. Monitor and review Results

The **MultiMod native job** panel shows progress. **Cancel safely** publishes no
partial scientific result. A validated cache can offer **Resume MGA shards** or
**Resume publication**. Other families restart the exact execution if cancelled
before the archive-ready boundary.

Use these result tabs:

- **Eligibility**
- **Diagnostics**
- **Estimates**
- **Inference**
- **Exclusions**
- **Failures**
- **Provenance**
- **Sidecars**

If result-level gates do not pass, Estimates and Inference are withheld.

## 7. Save, reopen, and export

Save the `.qpls` project. Archive V6 stores MultiMod results with compressed,
SHA-256-checked Arrow sidecars and reads compatible V5 projects. A required
sidecar that is missing, changed, duplicated, schema-mismatched, or
identity-mismatched makes reopen fail. Cost review warns above 128 MiB and
blocks a run above 512 MiB.

A complete Standard result can offer CSV, XLSX, JSON, HTML, PDF, SVG, and PNG
through **Export Results**. **Sidecars** can separately export eligible
validated Arrow evidence without overwriting an existing file.

## Essential boundaries

- Continuous moderation V1 is unchanged and separate from MultiMod.
- Quadratic/self-moderation is outside MultiMod.
- Unsupported intersections fail closed; QuickPLS does not silently remove a
  group, HOC, weight, interaction, path, or inference method.
- HOC+groups, HOC+weights, groups+weights, three-way+HOC,
  nested/overlapping HOCs, mixed HOC approaches, and more than one three-way
  term are unsupported.
- Survey weights, clusters, strata, and PPS are unsupported.
- Interventional mediation admits directly observed variables only within its
  bounded V1 profile.
- Standard means an exact qualified cell, not full SmartPLS parity.

## Fast troubleshooting

- **No `Moderation & heterogeneity…` button:** use **Calculate → Save and
  activate project…** and continue from the activated project.
- **Calculate disabled:** read the nearby stable code. Complete the checklist,
  correct groups/weights, select an explicit path or stable method/K, prepare a
  raw probe metric, reduce cost, or remove the unsupported intersection.
- **Scientific output withheld:** inspect Eligibility, Exclusions, Failures,
  and Sidecars for invariance, positivity, common-metric, ledger, archive, or
  authority blockers.
- **Archive integrity error:** stop and reopen the unchanged original or a
  verified backup; do not edit archive contents manually.
- **Windows warning:** this Beta is unsigned. Verify SHA-256 against the
  official GitHub release before running it.

See the [User Guide](USER_GUIDE.md), [2.56.0 Release Notes](RELEASE_NOTES_V2_56_0.md),
[Method Compatibility](METHOD_COMPATIBILITY.md), [MultiMod boundaries](MULTIMOD_BOUNDARIES_AND_QUALIFICATION_V1.md),
and [unsupported intersections](MULTIMOD_UNSUPPORTED_INTERSECTIONS_V1.md).
