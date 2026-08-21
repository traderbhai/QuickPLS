# QuickPLS 2.53.0 — Native Mediation and Moderation Workflow

Status: **workflow implementation, the three bounded exact-cell qualifications,
post-promotion archive verification, the 2.53.0 version-authority update, the
unsigned Windows candidate, and the packaged save/fresh-reopen smoke are
complete; public release publication remains pending**.

## Diagram-native authoring

Mediation continues to use ordinary substantive structural paths. Draw a chain
such as `X → M → Y`; QuickPLS discovers eligible single, parallel, and serial
indirect paths without adding a mediation node or workspace. Covariance,
control, measurement, generated, and interaction-hierarchy relationships are
excluded from discovery.

Moderation is attached visibly to the focal path:

- drag a moderator construct onto an eligible path;
- use **Add Moderating Effect…** from the selected path's context menu; or
- select the path and press `M`.

A compact `×` anchor sits on the focal path and a thin dashed arrow connects the
moderator to it. Enter edits the selected effect and Delete removes only that
effect. Dragging is never the only entry point. The anchor and connector are
presentation-only: `SemModelV4 interaction_v2` remains the scientific authority,
and no visual edge enters persistence, scientific hashes, mediation discovery,
or estimation matrices.

Generated interaction constructs are hidden from the ordinary Canvas and
Constructs list. Expert/Diagnostics can still reveal the compatibility objects
and identities. Historical archives with visible interaction nodes remain
readable without rewrite.

## Calculation and results

The public Calculate catalogue remains exactly 18 methods. PLS Algorithm and
Bootstrapping select the exact internal capability from the resident model:

- single, parallel, and serial mediation;
- one or several simultaneous two-way moderation effects;
- one bounded true three-way moderation term;
- researcher-selected first- or second-stage moderated mediation; and
- the existing HOC and ordinary PLS routes.

Results expose only applicable categories, including Direct Effects, Specific
Indirect Effects, Total Indirect Effects, Two-Way Moderation, Three-Way
Moderation, Conditional Effects, Simple Slopes, Moderated Mediation, Bootstrap
Inference, and Run Details. Selecting an entry highlights the complete chain or
moderated focal relationship in the read-only diagram.

## New exact cells — scoped Standard

Three additive cells are independently scoped Standard under `quickpls_v253_streamlined_integration_v1`:

| Owner | Cell | Method version | Current evidence |
| --- | --- | --- | --- |
| `smartpls.mediation` | `qpls3.pls.general_sem_single_mediation_bootstrap` | `general_sem_pls_single_mediation_full_model_case_bootstrap_v1` | `release_qualified` |
| `smartpls.moderation` | `qpls3.pls.general_sem_three_way_moderation_point` | `general_sem_pls_three_way_moderation_point_v1` | `release_qualified` |
| `smartpls.moderation` | `qpls3.pls.general_sem_three_way_moderation_bootstrap` | `general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1` | `release_qualified` |

Their method manifests and capability contracts now bind the accepted compact
Python/R/product reference and the single consolidated integration pass. The
contracts set `availability = standard`, `qualification_ready = true`, and
`promotion_allowed = true`. Existing multiple-mediation, simultaneous two-way
moderation, and bounded moderated-mediation identities retain their established
states.

## Bounded three-way scope

The new three-way source path admits one `X × W × Z` term per model, PLS
two-stage score construction, and strong hierarchy. It jointly estimates all
main effects, the three pairwise terms, and the three-way term. Output includes
the three-way coefficient, conditional `X × W` effects at fixed `Z` probes, and
simple slopes of `X` over a `W × Z` grid.

Continuous moderators use standardized `−1`, `0`, and `+1` probes. Binary
moderators must be coded exactly `0/1` and use their actual categories. The
bootstrap source path refits the complete model in one indexed traversal and
uses one shared failure ledger with two-sided percentile Type-7 intervals.

This release does not broaden to a second three-way term, fourth-order effects,
HOC interactions, three-way moderated mediation, Johnson–Neyman output, groups,
permutation, unsupported weights, or unsupported missing-data handling.

## Release verification

The compact reference, bounded failed-step remediation, independent exact-cell
activation, two Registry-dependent post-promotion archive checks, unsigned
Windows build, and packaged moderation create → calculate → Results →
save/fresh-reopen journey are complete. The packaged run retained the exact
canonical identity after relaunch, captured six screenshots, reported no console
errors, and left both C: and D: above the 20 GB floor. Public release publication
is the only remaining distribution step.

No earlier large mediation or two-way moderation qualification matrix needs to
be repeated because those engines and estimands are unchanged.

## Method references

- [Single-mediation bootstrap v1](methods/GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_V1.md)
- [Three-way moderation point v1](methods/GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_V1.md)
- [Three-way moderation bootstrap v1](methods/GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_V1.md)
- [SmartPLS mediation documentation](https://www.smartpls.com/documentation/algorithms-and-techniques/extended-relationships/mediation/)
- [SmartPLS moderation documentation](https://www.smartpls.com/documentation/algorithms-and-techniques/extended-relationships/moderation/)
