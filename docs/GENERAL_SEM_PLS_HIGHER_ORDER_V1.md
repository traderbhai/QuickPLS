# General SEM PLS higher-order constructs v1

QuickPLS executes one non-nested second-order higher-order construct (HOC) from the existing General SEM Canvas and schema-6 project workflow. The point cell is `qpls3.pls.general_sem_higher_order_point`; full-model case-bootstrap inference uses the supplemental `qpls3.pls.general_sem_higher_order_full_model_case_bootstrap` cell.

## Supported matrix

| Approach | RR | RF | FR | FF |
|---|---:|---:|---:|---:|
| Repeated indicators | Yes | Exogenous HOC only | Yes | Exogenous HOC only |
| Extended repeated indicators | No | Endogenous HOC | No | Endogenous HOC |
| Embedded two-stage | Yes | Yes | Yes | Yes |
| Disjoint two-stage | Yes | Yes | Yes | Yes |

The first HCM term determines lower-order component scoring (reflective = Mode A, formative = Mode B). The second term determines whether HOC-to-component results are loadings or weights with collinearity. Hybrid remains readable for compatibility but is not executable by these cells.

## Exact scope

- One HOC and at least two ordinary composite lower-order components.
- Raw continuous listwise data, standardized preprocessing, path weighting, no case weights, one group, and a recursive structural graph.
- Multiple authored structural paths may enter or leave the HOC. HOC/component relationships and extended-repeated technical paths are compiler-generated and remain distinct from authored hypotheses.
- Repeated indicators use deterministic virtual aliases. Embedded and disjoint two-stage approaches rerun their complete first and final stages from the resident SemModelV4 authority.
- Extended repeated indicators report the authored direct effect, technical indirect-via-component effect, and total effect separately.
- On an activated immutable General SEM project, Save As Revision adds the HOC and one selected incoming or outgoing structural path in the same atomic ModelV4 + RecipeV4 transaction; the source archive remains unchanged.

## Point and bootstrap results

Point results contain ordered stage receipts, generated-variable mappings, component loadings or weights, formative VIF, authored HOC structural paths, technical paths, and extended effects where applicable. Tables are stored in the canonical result document and use the shared CSV/XLSX/HTML/PDF/SVG/PNG export dispatcher.

For every bootstrap replicate QuickPLS resamples complete raw cases, reruns every required PLS stage, aligns score direction, regenerates component values, refits the final model, and validates the full point contract. Inference covers HOC component loadings/weights, authored structural paths touching the HOC, and extended-repeated total effects. All targets share one ordered failure ledger and the result is unavailable below `max(2, ceil(0.9B))` usable replicates. Percentile Type-7, two-sided inference is the only v1 interval.

## Exclusions

Nested or multiple HOCs, HOC interactions, feedback, groups, weights, matrix input, PLSc, permutation, hybrid execution, BCa/studentized inference, and inference for unrelated ordinary paths, controls, technical paths, or VIF are excluded. Existing legacy HOC projects and historical result identities are not reinterpreted.
