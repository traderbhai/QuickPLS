# QuickPLS 2.56.0 Features

QuickPLS 2.56.0 adds a qualified, fully offline moderation and heterogeneity workspace while preserving the existing PLS-SEM workflow and continuous moderation behavior. The exact qualified Windows build is tied to commit `28939b73db8f2284f21ce184050eb3f04110bf94`.

## First-run workflow

1. Create a project or open a compatible `.qpls` archive.
2. Import a local dataset and assign indicators to constructs.
3. Draw and configure the structural model on Canvas.
4. Activate the model, open **Calculate**, and choose an eligible method.
5. For moderation or heterogeneity work, choose **Moderation & heterogeneity...** from an activated strict General SEM model.
6. Review the method-specific eligibility and assumptions, run the analysis, inspect the categorized Results workspace, and save or export the completed result.

QuickPLS runs analytical workflows locally. It does not require an account, cloud computation, R, or Python. The Microsoft-managed WebView2 runtime can still make platform-service connections unless Windows network controls prevent them.

## Core PLS-SEM workspace

- Visual construct, indicator, structural-path, mediation, moderation, and higher-order-construct authoring.
- General SEM PLS estimation with the model structures admitted by the selected profile, including reflective, composite/formative, single-item, control, interaction, HOC, weighted, and reflective PLSc cells where qualified.
- Diagram-derived mediation and explicit moderation authoring without exposing generated interaction constructs as ordinary Canvas nodes.
- Categorized eligibility, diagnostics, estimates, inference, exclusions, failures, provenance, and evidence views.
- Local project save/reopen and semantic result exports.

Existing continuous moderation V1 is a protected compatibility boundary and is unchanged in 2.56.0. Quadratic or self-moderation is not part of MultiMod; the separate existing nonlinear diagnostic remains outside this suite.

## MultiMod family 1: categorical moderation and MGA

The multigroup workflow is **Groups → Comparisons → MICOM review → Inference → Cost**.

- Select 2-20 mutually exclusive typed groups from a grouping column that is not a model indicator.
- Require at least 10 complete model cases per group, warn below 30, warn above 2:1 imbalance, and reject imbalance above 10:1.
- Fit the same admitted PLS model in every group and retain explicit row exclusions and reasons.
- Record MICOM Step 1 as a structured researcher checklist; calculate Steps 2-3 pairwise without inventing an omnibus MICOM claim.
- For three or more groups, calculate a max-spread omnibus permutation statistic before selected pairwise follow-up.
- Support pairwise permutation, directional Henseler PLS-MGA probability, BC bootstrap difference intervals, pooled-variance and Welch sensitivity cells, and a K-group inverse-variance Wald cell where eligible.
- Apply Holm FWER by default, with Bonferroni and Sidak alternatives, exploratory BH, or an explicitly selected unadjusted result.
- Use deterministic, group-size-preserving resampling and fail if the required usable-draw threshold is not met.

Qualified cells are deliberately non-Cartesian. They include ordinary recursive General SEM, multiple two-way moderation, one bounded three-way term with its lower-order closure, bounded two-way moderated mediation, up to four disjoint nonnested second-order HOCs using one admitted approach, positive case weights, positive-integer frequency weights, and reflective PLSc. The eligibility view reports the exact admitted cell or a stable blocker.

## MultiMod family 2: latent segmentation

The segmentation workflow is **Candidate K → Inspect diagnostics → Lock method and K → Bootstrap**. QuickPLS never selects the number of segments automatically.

- **FIMIX-PLS V2** uses a genuine log-sum-exp EM procedure with pooled PLS scores, responsibility-weighted fitting, seeded multi-start optimization, posterior membership probabilities, class-specific paths and residual variances, convergence histories, collapse checks, and likelihood-based information criteria.
- **PLS-POS published V2** is the publication-faithful structural-only profile. A separately named destination-scored-interactions profile admits bounded two-way and three-way interaction configurations.
- Candidate `K` is 2-5; `K=1` is a read-only pooled baseline.
- Fixed-method, fixed-`K` bootstrap reruns the full segmentation pipeline, aligns labels, rejects ambiguous matches, and uses one shared validity ledger.
- POS between-segment interaction comparisons require the `pos_common_metric_comparability_v1` gate. If it fails, segment-local descriptive results remain available but gamma, delta, slope, and effect comparisons are suppressed.
- Tandem FIMIX/POS results remain separate; cross-tabulation and adjusted Rand index are descriptive comparisons only.

## MultiMod family 3: conditional-process analysis

The workflow is **Select paths → Attach moderators → Define probes → Preview targets → Calculate**.

- Explicitly select every indirect path; QuickPLS does not guess the path to analyze.
- Analyze bounded first-stage, second-stage, both-stage, multiple two-way, one three-way, longer-path, multiple-HOC, grouped, positive case-weighted, and positive-integer frequency-weighted profiles where their exact contracts admit them.
- Use standardized default probes of −1, 0, and +1, with bounded Cartesian grids or explicit joint tuples and contrasts.
- Keep original-sample probe anchors fixed across resamples and use one bootstrap ledger for every requested target.
- Provide percentile, full delete-one BCa, nested studentized, and one-sided alternatives only in the profiles that explicitly qualify them.
- Report a conventional scalar index of moderated mediation only when the selected indirect effect is affine in exactly one moderator. Otherwise, report the appropriate local derivatives and finite probe contrasts without mislabelling them as a constant Hayes index.

Conditional-process results are associational PLS-SEM estimates. They are not causal or counterfactual claims.

## MultiMod family 4: interventional mediation

This is a separate observed-data workflow for parametric g-computation.

- Use directly observed treatment, continuous mediators and outcome, baseline numeric/binary moderators, an explicit nonempty adjustment set, and explicitly selected recursive paths of 2-4 edges.
- Support binary treatment or continuous treatment with a declared `x0 → x1` contrast.
- Require temporal-order, identification, and positivity review before calculation.
- Label every result as an **assumption-dependent interventional estimate**; QuickPLS never states that causality has been established.

The first qualified profile excludes latent/composite/HOC analysis roles, natural or cross-world effects, recanting-witness settings, exposure-induced mediator-outcome confounding, groups, and weights.

## Persistence, evidence, and exports

- Project Archive V6 reads compatible V5 projects and stores large MultiMod evidence as compressed, SHA-256-bound Arrow sidecars.
- Missing, altered, duplicate, schema-mismatched, or identity-mismatched sidecars fail reopening instead of silently degrading the result.
- A predicted result sidecar above 128 MiB triggers a warning; a run above 512 MiB is rejected.
- Cancellation does not publish a partial scientific result.
- Canonical General SEM results support CSV, XLSX, JSON, self-contained HTML, PDF, SVG, and PNG. Each MultiMod family exposes only the formats defined for its result type; raw Arrow evidence export requires a strict Archive V6 reopen.
- Large result tables are virtualized and keyboard accessible.

## Qualification and product boundaries

The Windows candidate passed 32/32 release gates, including installed and portable offline smoke. A **Standard · Release-qualified** label applies only when the executable embeds the exact qualified capability authority for its source and evidence identity. Generic source or development builds without that authority remain **Experimental Labs · Unqualified** or unavailable; changing a UI label does not qualify a method.

QuickPLS independently implements published methods. It does not import SmartPLS projects, reproduce proprietary implementation details, promise identical undocumented behavior, or claim full SmartPLS feature parity.

For detailed profile intersections, see [Method Compatibility](METHOD_COMPATIBILITY.md), [MultiMod boundaries and qualification](MULTIMOD_BOUNDARIES_AND_QUALIFICATION_V1.md), and [unsupported intersections](MULTIMOD_UNSUPPORTED_INTERSECTIONS_V1.md). See also the [2.56.0 release notes](RELEASE_NOTES_V2_56_0.md), [Quick Start](QUICK_START.md), and [User Guide](USER_GUIDE.md).
