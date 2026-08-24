# MultiMod unsupported intersections and reason codes

Status: frozen fail-closed product contract for the candidate implementation.

The codes below are stable external identities. UI, runner, persisted failure
records and exports may add detail, but must not replace a code or silently
simplify the requested analysis. `block` means no scientific result is
published. `suppress_inference` preserves explicitly identified descriptive
output while withholding an invalid comparison.

| Stable reason code | Trigger | Action |
|---|---|---|
| `multimod.protected.continuous_moderation_v1` | A MultiMod adapter attempts to alter or reinterpret continuous-moderation V1. | block |
| `multimod.recipe.multiple_v2_configs` | More than one additive MultiMod configuration is present in one recipe. | block |
| `multimod.profile.hoc_plus_groups` | HOC and grouped conditional-process profiles are combined. | block |
| `multimod.profile.hoc_plus_weights` | HOC and case/frequency weights are combined. | block |
| `multimod.profile.groups_plus_weights` | A grouped analysis also requests weights. | block |
| `multimod.profile.three_way_plus_hoc` | A bounded three-way term is combined with an HOC. | block |
| `multimod.profile.three_way_count` | More than one three-way term, missing lower-order closure, fourth-order or arbitrary k-way products are requested. | block |
| `multimod.profile.hoc_nested_overlapping` | HOCs are nested, hybrid, overlapping, or not pairwise-disjoint. | block |
| `multimod.profile.hoc_mixed_approach` | More than one HOC approach is used in the same run. | block |
| `multimod.inference.studentized_outside_profile` | Studentized inference exceeds its interaction/path/moderator/probe/target bounds or is selected in another profile. | block |
| `multimod.inference.bca_incomplete_jackknife` | Any required delete-one fit or BCa acceleration input is missing. | block |
| `multimod.inference.one_sided_not_predeclared` | `less` or `greater` was not frozen before calculation. | block |
| `multimod.inference.usable_draws_below_threshold` | A shared ledger has fewer than its method-specific minimum usable draws. | block |
| `multimod.target.path_not_explicit` | No exact indirect path was selected or a selected path is discontinuous. | block |
| `multimod.target.constant_index_not_defined` | A scalar index is requested when the indirect-effect polynomial is not affine in one moderator. | block |
| `multimod.probe.raw_composite_or_hoc` | A raw-unit probe is requested for a composite or HOC moderator. | block |
| `multimod.probe.receipt_missing` | An observed raw-unit moderator lacks its transformation receipt. | block |
| `multimod.weight.case_invalid` | Case weights are nonfinite, nonpositive or exceed the normalized `1e6` max/min ratio. | block |
| `multimod.weight.frequency_invalid` | Frequencies are not positive integers or their total exceeds `2^53-1`. | block |
| `multimod.weight.survey_design` | Sampling weights, clusters, strata, PPS or a survey-design claim is requested. | block |
| `multimod.mga.group_column_is_indicator` | The grouping column is used as a model indicator. | block |
| `multimod.mga.group_count` | Fewer than two or more than twenty typed selected groups remain. | block |
| `multimod.mga.group_complete_cases` | A selected group has fewer than ten complete model cases. | block |
| `multimod.mga.group_imbalance` | Largest/smallest complete group size is greater than 10:1. | block |
| `multimod.mga.micom_step1_unconfirmed` | The structured configural-invariance checklist is incomplete. | block |
| `multimod.mga.partial_invariance_missing` | A requested structural comparison lacks partial measurement invariance for an affected construct. | suppress_inference |
| `multimod.mga.all_pairs_heavy_confirmation` | All 190 pairs at twenty groups are requested without heavy-run confirmation. | block |
| `multimod.mga.omnibus_micom_claim` | A K-group omnibus MICOM conclusion is requested. | block |
| `multimod.mga.henseler_probability_mislabel` | A directional Henseler probability is requested as an ordinary two-sided p-value. | block |
| `multimod.mga.bootstrap_bca_claim` | Zero-acceleration bias-corrected intervals are requested or labelled as BCa. | block |
| `multimod.segmentation.auto_select_k` | The product is asked to select K automatically. | block |
| `multimod.segmentation.bootstrap_reselect_k` | Bootstrap attempts to repeat discovery or model selection. | block |
| `multimod.fimix.start_not_reproduced` | Fewer than two aligned starts reproduce the optimum within frozen tolerances. | block |
| `multimod.fimix.likelihood_decrease` | A start has a material likelihood decrease. | block |
| `multimod.fimix.class_or_variance_collapse` | A class collapses or residual variance falls below `1e-8`. | block |
| `multimod.fimix.singular_or_nonfinite` | A weighted design is singular or a required value is nonfinite. | block |
| `multimod.pos.published_interaction` | P2 or P23 is requested under `qpls.pls-pos.published.v2`. | block |
| `multimod.pos.start_not_reproduced` | Fewer than two starts reproduce the canonical partition/objective. | block |
| `multimod.pos.common_metric_failed` | Configural identity or required pairwise compositional invariance fails. | suppress_inference |
| `multimod.segmentation.label_match_ambiguous` | A fixed-K bootstrap label match is ambiguous or lacks majority overlap. | block |
| `multimod.causal.latent_or_composite_role` | A latent, composite or HOC variable is assigned a causal-module role. | block |
| `multimod.causal.groups_or_weights` | Groups or any weight design is requested in the causal module. | block |
| `multimod.causal.cross_world_effect` | A natural, cross-world or recanting-witness estimand is requested. | block |
| `multimod.causal.exposure_induced_confounding` | Exposure-induced mediator-outcome confounding is declared or detected. | block |
| `multimod.causal.adjustment_set_missing` | The explicit adjustment set is absent. | block |
| `multimod.causal.identification_unreviewed` | Any required temporal-order or identification checklist item is unreviewed. | block |
| `multimod.causal.positivity_failure` | Treatment/moderator support is inadequate for the requested intervention. | block |
| `multimod.causal.causality_wording` | Output or export claims causality is established. | block |
| `multimod.archive.sidecar_limit` | Predicted sidecar size exceeds 512 MiB for one run. | block |
| `multimod.archive.sidecar_integrity` | A sidecar is missing, altered, duplicated, schema-mismatched or identity-mismatched. | block |
| `multimod.profile.unqualified_intersection` | A requested combination has no exact release-qualified profile cell. | block |

The machine-readable source of this table is
`validation/multimod/unsupported_intersections_v1.json`.
