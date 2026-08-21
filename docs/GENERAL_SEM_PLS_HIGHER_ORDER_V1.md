# General SEM PLS higher-order constructs v1

QuickPLS executes one non-nested second-order higher-order construct (HOC) from the normal Canvas → Calculate → Results workflow. The point cell is `qpls3.pls.general_sem_higher_order_point`; full-model case-bootstrap inference uses the supplemental `qpls3.pls.general_sem_higher_order_full_model_case_bootstrap` cell. Version 2.52 changes authoring and presentation only; these scientific cells and their stored result identities are unchanged.

## Canvas workflow

Select at least two eligible ordinary constructs, then open **Higher-Order Construct…** from the Model menu or the selection context menu. The same compact dialog creates and edits an HOC:

- enter the HOC name and select its lower-order dimensions;
- choose **HOC explains its dimensions** or **Dimensions form the HOC**;
- review the derived type: the first letter is the dimensions' reflective/formative measurement and the second letter is the HOC's reflective/formative measurement (RR, RF, FR, or FF); and
- accept the topology-aware recommended approach, or open **Advanced** to choose another valid approach and optionally retain a legacy short code.

Measurement-only dimensions prefer disjoint two-stage. Dimensions already participating in structural relationships prefer embedded two-stage when eligible. Repeated and extended-repeated appear only for supported topology/type combinations; hybrid remains read-only compatibility. The Canvas node keeps only its name and a small HOC marker. Selecting it highlights its dimensions with presentation-only membership connectors, which are never stored as structural relationships.

Edit the selected HOC from its context menu, the Model menu, Properties, or Enter. On an activated immutable project, create and edit use one atomic **Save As Revision** transaction. Editing preserves the resident term/output identities and authored structural paths; the source archive remains unchanged.

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
- On an activated immutable project, Save As Revision adds the HOC and one selected incoming or outgoing structural path in the same atomic ModelV4 + RecipeV4 transaction; the source archive remains unchanged.

## Point and bootstrap results

Choose **PLS Algorithm** for an eligible point calculation. Choose **PLS Bootstrapping** for the existing point plus full-model case-bootstrap route. Calculate shows the HOC as one compact name/type/approach row and keeps fixed estimator descriptions in Method Details.

Point results contain ordered stage receipts, generated-variable mappings, component loadings or weights, formative VIF, authored HOC structural paths, technical paths, and extended effects where applicable. The normal Results tree groups researcher-facing component relationships, HOC structural paths, extended effects where applicable, and bootstrap inference. Stage receipts, raw IDs, and hashes remain under Run Details. Tables are stored in the canonical result document and use the shared CSV/XLSX/HTML/PDF/SVG/PNG export dispatcher.

For every bootstrap replicate QuickPLS resamples complete raw cases, reruns every required PLS stage, aligns score direction, regenerates component values, refits the final model, and validates the full point contract. Inference covers HOC component loadings/weights, authored structural paths touching the HOC, and extended-repeated total effects. All targets share one ordered failure ledger and the result is unavailable below `max(2, ceil(0.9B))` usable replicates. Percentile Type-7, two-sided inference is the only v1 interval.

## Exclusions

Nested or multiple HOCs, HOC interactions, feedback, groups, weights, matrix input, PLSc, permutation, hybrid execution, BCa/studentized inference, and inference for unrelated ordinary paths, controls, technical paths, or VIF are excluded. Existing legacy HOC projects and historical result identities are not reinterpreted. PLS model-fit tables remain omitted for this bounded HOC workflow; Run Details states that model fit is not reported.
