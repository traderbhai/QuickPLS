# PLS Higher-Order Constructs v1

Status: validated for the documented QuickPLS v1.2.3 bounded repeated-indicator, two-stage, and hybrid higher-order construct scope.

This document freezes the QuickPLS recipe contract for higher-order constructs and the repeated-indicator, two-stage, and hybrid execution behavior promoted in v1.2.3. Unsupported HOC variants remain blocked or excluded.

## Scope

The `ModelSpec.higher_order_constructs` field records constructs that should later be estimated as higher-order constructs. Each entry contains:

- `id`: the existing construct id that represents the higher-order construct.
- `components`: two or more existing lower-order construct ids.
- `method`: one of `repeated_indicators`, `two_stage`, or `hybrid`.
- `stage_one_recipe`: optional saved recipe or run reference reserved for two-stage workflows.

The validator enforces recipe integrity:

- the higher-order construct id must exist;
- the id must be unique among higher-order declarations;
- at least two components are required;
- every component must exist;
- a construct cannot include itself as a component;
- components must be unique within a higher-order declaration.

Every higher-order declaration is validated only inside this bounded scope. Empty HOC placeholder constructs are allowed and emit `higher_order.indicators.generated`, because supported HOC methods can generate the HOC measurement block from lower-order components.

## Repeated-Indicator Estimation Behavior

For `method = "repeated_indicators"`, the PLS execution layer clones the recipe and replaces the HOC construct's indicator block with the ordered union of its lower-order components' indicators. Duplicate indicator names across components are kept once in first-seen component order. The rest of the PLS pipeline, including weighting, latent-score iteration, path estimation, effects, and assessment consumers, uses the expanded execution recipe.

The original project recipe remains typed as a HOC declaration. The run emits a scoped validation warning that HOC indicator blocks were expanded from lower-order component indicators.

Assessment uses the same expanded execution recipe so reliability, AVE, cross-loadings, HTMT, model fit, and downstream diagnostic tables consume the generated HOC indicator block rather than the empty project placeholder. Construct-specific loading and weight lookups are required because repeated indicators can legitimately appear in both lower-order and HOC blocks.

## Two-Stage Estimation Behavior

For `method = "two_stage"`, QuickPLS first estimates the lower-order construct model without the HOC construct. The stage-1 latent variable scores of the listed components are then appended to the stage-2 execution dataset as generated HOC indicators named `__qpls_hoc_<hoc>_<component>`. The HOC construct's indicator block is replaced by those generated component-score indicators before ordinary PLS execution.

The stage-2 execution keeps lower-order constructs in the model and permits them to be measurement-only components when they have no structural paths in the stage-2 model. The run emits a scoped validation warning that lower-order component scores were used as generated HOC indicators.

Assessment uses the same generated two-stage HOC indicator names and maps them to the component score vectors from the estimation result, so deterministic assessment tables can be produced without requiring generated score columns to exist in the original raw dataset.

## Hybrid Estimation Behavior

For `method = "hybrid"`, QuickPLS uses a bounded indicator-split contract consistent with Becker, Klein, and Wetzels' description that hybrid HOC estimation works similarly to repeated indicators while using each manifest indicator only once. Each component must have at least two indicators. QuickPLS keeps the first `ceil(k/2)` indicators on the lower-order component and assigns the remaining indicators to the higher-order construct, preserving component order and first-seen indicator order.

Example with two indicators per component:

- component `x`: `x1` remains on `x`; `x2` becomes a HOC indicator.
- component `z`: `z1` remains on `z`; `z2` becomes a HOC indicator.

The original project recipe remains typed as a hybrid HOC declaration. The execution recipe is split before ordinary PLS estimation, and assessment uses the same split so deterministic quality tables assess the same measurement blocks as estimation. Recipe validation emits `higher_order.hybrid_component_indicators` when any hybrid component has fewer than two indicators. Stable exports may report HOC output as validated only inside this documented v1.2.3 bounded scope.

## Acceptance Evidence

The schema is accepted when `cargo test -p qpls-core` proves valid repeated-indicator, two-stage, and hybrid declarations, generated HOC indicator placeholders, duplicate ids, unknown constructs, insufficient components, self-components, unknown components, duplicate components, hybrid component split feasibility, and scoped validation warnings.

The repeated-indicator estimator slice is accepted when `cargo test -p qpls-estimation` proves that an empty HOC placeholder is expanded from component indicators, produces HOC outer estimates and construct scores, and emits the scoped repeated-indicator warning.

The two-stage estimator slice is accepted when `cargo test -p qpls-estimation` proves that lower-order component scores are appended as generated HOC indicators, the HOC structural path is estimated, and the scoped two-stage HOC warning is emitted. `cargo test -p qpls-assessment` must also pass because assessment has to consume the generated HOC score indicators consistently.

The hybrid estimator slice is accepted when `cargo test -p qpls-estimation` proves that component indicator blocks are split between lower-order and higher-order constructs, the HOC structural path is estimated, and the scoped hybrid HOC warning is emitted. `cargo test -p qpls-assessment` must also pass because assessment has to consume the split HOC recipe consistently.

Independent reference evidence is accepted when `npm run qpls:hoc:reference` writes `validation/results/higher_order_reference_report.json` with `passed=true`. That script implements the repeated-indicator expansion and PLS path-weighting stages in Python outside the Rust engine and compares QuickPLS path coefficients, HOC loadings, and HOC weights within `1e-6`.

Metamorphic evidence is accepted when `npm run qpls:hoc:metamorphic` writes `validation/results/higher_order_metamorphic_report.json` with `passed=true`. The bounded screen checks independent-reference agreement, positive-affine invariance, row-order invariance, construct-order invariance, HOC component-order invariance, scoped-warning persistence, and degradation after permuting one lower-order component block.

Two-stage independent and metamorphic evidence is accepted when `npm run qpls:hoc:two-stage` writes `validation/results/higher_order_two_stage_reference_report.json` with `passed=true`. The script uses a bounded single-indicator component fixture with an explicit independent stage-one score calculation, estimates the stage-two HOC path model in Python outside the Rust engine, and checks QuickPLS path coefficients, generated HOC loadings, generated HOC weights, positive-affine invariance, row-order invariance, construct-order invariance, HOC component-order invariance, assessment-table availability, scoped-warning persistence, and degradation after permuting one lower-order component score source.

Hybrid independent and metamorphic evidence is accepted when `npm run qpls:hoc:hybrid-reference` writes `validation/results/higher_order_hybrid_reference_report.json` with `passed=true`. The script implements the indicator split in Python outside the Rust engine and checks QuickPLS path coefficients, HOC loadings, HOC weights, positive-affine invariance, row-order invariance, HOC component-order invariance, scoped-warning persistence, and degradation after permuting one HOC-side indicator partition. The current observed reference delta is `1.37e-14`.

Hybrid invalid-split guard evidence is accepted when `npm run qpls:hoc:hybrid-guard` writes `validation/results/higher_order_hybrid_guard_report.json` with `passed=true`. The guard proves that a hybrid HOC with a one-indicator component is rejected by validation and execution with `higher_order.hybrid_component_indicators`.

## Method Sources

- Becker, J.-M., Klein, K., and Wetzels, M. (2012). Hierarchical Latent Variable Models in PLS-SEM: Guidelines for Using Reflective-Formative Type Models. Long Range Planning, 45(5-6), 359-394.
- Sarstedt, Hair, Cheah, Becker, and Ringle (2019) describe HOC specification, estimation, and validation in PLS-SEM using repeated-indicator and two-stage approaches; broader HOC variants remain unsupported until separately audited.
