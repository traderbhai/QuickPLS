# General SEM PLS simultaneous two-way moderation point v1

## Status and exact scope

This method is an Experimental Labs, engine-only capability. It is not qualification-ready and must not be exposed as Standard. It estimates one or more `interaction_v2` terms jointly after one shared PLS stage-one score model.

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

## Versioned provenance

- estimator: `qpls.general-sem-pls.multiple-two-way.point.v1`;
- product scale: `qpls.general-sem-pls.two-stage-product.sample-standardized.v1`;
- hierarchy: `qpls.general-sem-pls.interaction-hierarchy.strong.v1`;
- conditioning: `qpls.general-sem-pls.simple-slope.other-moderators-zero.v1`;
- compiler: `recipe_v4_to_compiled_pls_plan_v3_multiple_two_way_moderation_point_v1`;
- runner adapter: `compiled_general_sem_pls_recipe_v1_multiple_two_way_moderation_point_execution_v1`;
- capability cell: `smartpls.moderation / qpls3.pls.general_sem_multiple_two_way_moderation_point / general_sem_pls_multiple_two_way_moderation_point_v1`.

Cancellation before or during the interaction stage publishes no result. Result readback rejects changed identities, method versions, scale receipts, gamma rescaling, stage-one digest, or conditional/plot cross-references.

## Qualification boundary

`qualification_ready=false`. Engine implementation and focused deterministic tests do not establish release qualification. Promotion requires an independent simultaneous-interaction oracle, simulation recovery, boundary and collinearity evidence, complete-model bootstrap evidence, archive/native/export equality, packaged Windows acceptance, and an exact capability-cell audit.

SmartPLS public moderation documentation is a workflow and terminology benchmark. Scientific qualification remains based on independent formulas and references, including Henseler and Chin (2010), DOI `10.1080/10705510903439003`.
