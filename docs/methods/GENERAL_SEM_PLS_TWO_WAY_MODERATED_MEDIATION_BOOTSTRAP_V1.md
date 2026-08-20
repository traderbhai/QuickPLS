# General SEM PLS two-way moderated-mediation bootstrap v1

## Status and product route

This exact option cell is connected in opt-in Experimental Labs with `partial`
coverage and `engine_only` evidence. It is not Standard, qualification-ready,
or promotion-allowed. The connected route includes path selection, a new-file
model and Recipe revision, strict schema-6 activation, native execution,
canonical results and tables, shared XLSX table export, atomic result append,
and strict reopen. Connection does not imply independent numerical or packaged
release qualification.

The Registry identity is:

- capability: `smartpls.mediation`;
- cell: `qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap`; and
- capability version:
  `general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1`.

## Exact admitted predicate

The resident schema-6 `general_sem_v1` authority must use
`qpls.pls_sem.v3`, raw continuous composite data, listwise deletion, one group,
and no weights, clusters, strata, authored missing markers, or transformation
lineage. The structural model must be a recursive DAG with:

- one selected two-relation `X -> M -> Y` SpecificPath;
- exactly one two-way `interaction_v2` term using two-stage construction and
  strong hierarchy;
- first-stage (`X x W -> M`) or second-stage (`M x W -> Y`) moderation, never
  both;
- a moderator distinct from `X`, `M`, and `Y`; and
- no higher-order construct, additional interaction, arbitrary probe, longer
  selected path, or causal-identification claim.

Inference is mandatory full-model indexed case resampling with replacement,
2 through 10,000 requested replicates, a fixed seed, two-sided Type-7
percentile intervals, and at least `max(2, ceil(0.9 B))` usable replicates.

## Five-target shared-ledger contract

Every usable replicate reruns stage-one PLS scoring, score-sign orientation,
sample-standardized product construction, and the complete joint stage-two
solve. One ordered replicate ledger supplies exactly five targets:

1. scientific rescaled interaction gamma;
2. conditional indirect effect at standardized `W = -1`;
3. conditional indirect effect at standardized `W = 0`;
4. conditional indirect effect at standardized `W = +1`; and
5. the index of moderated mediation.

For first-stage moderation, the conditional indirect effect is
`(a + gamma W)b` and the index is `gamma b`. For second-stage moderation it is
`a(b + gamma W)` and the index is `a gamma`. QuickPLS publishes no partial
target set, no substituted gamma-only ledger, and no causal interpretation.
Failed replicates retain their index and typed reason; cancellation publishes
no partial result.

## File, session, and recovery boundary

Authoring never mutates the open source archive. The user selects one eligible
path and saves to a new `.qpls` destination. Revision v2 pins the source archive,
model, Recipe, scientific digests, stable path identity, new project/model/Recipe
identities, exact Registry cell, compiled plan, compiled target, and lineage.
Native persistence is no-replace and verifies the source before and after the
write. The desktop then strictly inspects the destination before replacing any
active authority.

If transport, inspection, or authority reconciliation fails before a verified
activation, the source stays active. If destination activation fails after the
source is released, QuickPLS strictly re-inspects the unchanged source path and
restores it only when its archive and authority identities still match. The UI
reports whether a destination was persisted, whether the source was restored,
and the exact reopen or restart action.

## Result, persistence, and export boundary

The canonical document retains the moderation point cell as primary and adds
the moderated-mediation cell as supplemental authority. It records joint-stage
coefficients, interaction effects, fixed conditional slopes and plots,
conditional indirect effects, the moderated-mediation index, all five target
identities, the shared-ledger identity and usable-index digest, failure ledger,
methods, cells, and source/compile digests. Generic canonical rendering and
all-table XLSX export consume those persisted tables without recomputation. Atomic schema-6 append
and strict reopen reject changed methods, target identities, counts, intervals,
receipts, tables, lineage, or digests.

## Versioned provenance

- compiled target:
  `qpls.compiled-pls-two-way-moderated-mediation-target.v1`;
- point method: `general_sem_pls_two_way_moderated_mediation_point_v1`;
- bootstrap method:
  `general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1`;
- bootstrap operation:
  `general_sem_pls_two_way_moderated_mediation_case_bootstrap_v1`;
- execution adapter:
  `compiled_general_sem_pls_recipe_v1_two_way_moderated_mediation_percentile_bootstrap_execution_v1`;
- product scale:
  `qpls.general-sem-pls.two-stage-product.sample-standardized.v1`;
- probe policy: `standardized_moderator_minus_one_zero_plus_one_v1`;
- conditional target: `conditional_indirect_effect_v1`;
- index target: `index_of_moderated_mediation_v1`; and
- resampling stream: `indexed_case_resampling_v1`.

## Promotion boundary

Standard promotion still requires an independent full-PLS oracle for both
moderated stages, simulation recovery and interval coverage, adversarial and
worker-invariance evidence, packaged Windows end-to-end acceptance, formal
accessibility, cross-format semantic export/readback, performance/memory/soak
evidence, and independent scientific review.

`qualification_ready=false`; `promotion_allowed=false`.
