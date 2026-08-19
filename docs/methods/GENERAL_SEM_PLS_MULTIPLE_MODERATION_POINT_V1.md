# General SEM PLS simultaneous two-way moderation point v1

## Status and exact scope

This method is an Experimental Labs capability whose evidence tier remains
`engine_only`. It is not qualification-ready and must not be exposed as
Standard. The existing QuickPLS desktop application now executes the exact
cell end to end, but connected native plumbing does not substitute for the
independent numerical, simulation, export, packaged-Windows, and review
evidence required for promotion. The method estimates one or more
`interaction_v2` terms jointly after one shared PLS stage-one score model.

The executable predicate is exact:

- schema generation `general_sem_v1`;
- continuous, single-group raw data with listwise deletion and no case weights;
- an acyclic, direct-only structural graph;
- one or more two-operand `interaction_v2` terms;
- two-stage construction and strong hierarchy;
- every focal and moderator main-effect path present;
- point estimation only;
- no requested mediation effects or authored conditional-probe policy.

Bootstrap inference, three-way or higher interactions, moderated mediation, any directed structural chain, HOC interaction systems, arbitrary probes, groups, and feedback remain blocked. A blocked request is preserved in the scientific model and is never simplified silently.

## Frozen estimation contract

Let `x` and `w` be the stage-one construct scores for the focal predictor and moderator. Each score is sample-standardized using the same complete-case rows. Define

```text
p_i = z(x_i) z(w_i)
z(p_i) = (p_i - mean(p)) / sd_sample(p)
```

For every endogenous construct, QuickPLS fits one joint stage-two least-squares equation containing all ordinary predecessors and all interaction-product columns targeting that outcome. Simultaneous interactions are never fit in isolated regressions.

The canonical result retains both coefficient scales:

```text
beta_product_standardized = coefficient fitted to z(p)
gamma_scientific = beta_product_standardized / sd_sample(p)
```

`gamma_scientific` is the change in the standardized focal slope per one standardized moderator unit. The result also retains `mean(p)`, `sd_sample(p)`, row count, generated-column identity, focal relation, interaction relation and parameter identities, stage-one scientific digest, and all method-policy versions.

## Conditional effects and plots

The frozen conditioning policy is `qpls.general-sem-pls.simple-slope.other-moderators-zero.v1`:

- probe the selected moderator at standardized `-1`, `0`, and `+1`;
- hold other moderators on the same focal relation at standardized zero;
- hold all omitted ordinary predictors at standardized zero;
- retain the product-centering term when evaluating the full standardized outcome linear predictor.

For focal relation coefficient `beta_x`, the simple slope for interaction `j` is

```text
slope_x(w_j) = beta_x + gamma_j w_j
```

when all other same-focal moderators are zero. Every conditional row and plot references the authoritative canonical interaction-effect ID.

The canonical result also publishes the complete joint stage-two structural
coefficient ledger. Ordinary structural/control rows, interaction rows,
conditional rows, and plot points are reconciled to the same compiled plan and
stable identities before a schema-6 result can be appended or reopened.

## Existing QuickPLS workflow boundary

The exact cell is connected through the existing General SEM workspace in
QuickPLS:

1. a newly created schema-6 `general_sem_v1` project supplies the resident
   dataset, `SemModelV4`, Recipe V4, and exact compiled-plan authority;
2. native preflight selects this point-only cell for supported same-focal or
   different-focal simultaneous two-way interactions;
3. the shared job lifecycle starts, reports progress, supports cancellation,
   and publishes only a completed canonical result;
4. QuickPLS renders persisted conditional-effect line charts with an
   accessible table/summary, non-color-only series styling, and stable source
   identities; and
5. result append, strict reopen, close/reopen restoration, and XLSX table
   projection all read the same canonical values without adding inference.

Cancellation is checked before execution, during interaction processing,
before canonical publication, and after archive assembly but before result
publication. Cancellation leaves the source archive unchanged and exposes no
partial result.

An activated `general_sem_v1` model and Recipe V4 are immutable scientific
authorities. The **Create Moderating Effect** action therefore applies the
versioned `add_general_sem_interaction_v2` intent by creating a new schema-6
revision file. It pins the source archive and resident model/recipe digests,
adds exactly one two-operand `two_stage`/`strong` interaction plus required
lower-order paths, recompiles the resident Recipe V4, publishes with
no-replace semantics, strictly reopens the destination, and then activates the
revision. The source file and its historical results remain unchanged. A
cancelled or pre-commit failure neither mutates the source nor leaves a
published destination. After a native persisted receipt exists, a later
frontend resolution or activation failure retains that strictly reopened
destination, keeps or restores the unchanged source authority, and reports the
exact revision path with explicit reopen/restart recovery; it never reports a
false rollback.

That immutable state does not hide the qualified edit. QuickPLS exposes
**Moderating Effect (Save As Revision)…** through its toolbar, eligible-path
context menu, and keyboard command while retaining every ordinary direct-edit
lock. Pending authority operations, calculations, temporary results, dirty
session state, or another revision disable all entry points with a corrective
reason rather than falling back to an in-place edit.

## Versioned provenance

- estimator: `qpls.general-sem-pls.multiple-two-way.point.v1`;
- product scale: `qpls.general-sem-pls.two-stage-product.sample-standardized.v1`;
- hierarchy: `qpls.general-sem-pls.interaction-hierarchy.strong.v1`;
- conditioning: `qpls.general-sem-pls.simple-slope.other-moderators-zero.v1`;
- compiler: `recipe_v4_to_compiled_pls_plan_v3_multiple_two_way_moderation_point_v1`;
- runner adapter: `compiled_general_sem_pls_recipe_v1_multiple_two_way_moderation_point_execution_v1`;
- capability cell: `smartpls.moderation / qpls3.pls.general_sem_multiple_two_way_moderation_point / general_sem_pls_multiple_two_way_moderation_point_v1`.

Result readback rejects changed identities, method versions, scale receipts,
gamma rescaling, stage-one digest, the joint stage-two coefficient ledger, or
conditional/plot cross-references.

## Qualification boundary

`qualification_ready=false`. Engine implementation, a connected native
workflow, and focused deterministic tests do not establish release
qualification. Promotion requires an independent full simultaneous-interaction
PLS oracle, qualification-scale simulation recovery, boundary and collinearity
evidence, complete-model bootstrap evidence, cross-surface and semantic-export
equality, packaged Windows acceptance, accessibility/performance acceptance,
independent review, and an exact capability-cell promotion audit.

SmartPLS public moderation documentation is a workflow and terminology benchmark. Scientific qualification remains based on independent formulas and references, including Henseler and Chin (2010), DOI `10.1080/10705510903439003`.
