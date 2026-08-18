# PLS Algorithm v1 qualification status

Snapshot date: 2026-08-15

This note separates the release-qualified bounded QuickPLS contract from a broader future SmartPLS-parity program for `smartpls.pls_algorithm` / `qpls3.pls.algorithm`. It is an internal product-finalization record; the method-promotion manifest and Capability Registry remain the scoped-Standard authorities.

## Current official comparator

The official [SmartPLS PLS-SEM algorithm documentation](https://smartpls.com/documentation/algorithms-and-techniques/core-algorithm/pls/) describes the three-stage PLS procedure and the active path, factor, and PCA weighting schemes. It also documents standardized, mean-centered, and unstandardized results; standard and individual initial outer weights; a fixed 3,000-iteration limit; and a fixed `1e-7` stop criterion. The official [composite and common-factor documentation](https://smartpls.com/documentation/functionalities/composite-and-common-factor-and-mixed-models/) additionally documents Mode A, Mode B, sum scores, and predefined weights.

The official pages are behavioral and coverage comparators. SmartPLS is not the sole numerical oracle. Wold (1982) and Lohmöller (1989) remain the primary method references; R cSEM 0.6.1 and python-plspm 0.5.7 are independent development-validation implementations and are never packaged with QuickPLS.

## Coverage truth

The bounded `pls_pm_v1` engine contract covers recursive raw-data composite models, Mode A, Mode B, single-item blocks, path and factor weighting, bounded PCA block weighting, listwise deletion, and deterministic estimation. The Internal Recipe-v4 `pls_score_execution_v2` path now also executes Individual initialization plus Unit and Custom fixed unit-variance scoring. That is implemented product breadth, not qualification or full active parity.

Coverage remains partial relative to the full official comparator because these additional options are outside the promoted bounded contract:

- Individual initial outer weights are implemented internally but lack qualification-sized current-product evidence;
- sum-score and predefined-weight semantics are represented by internally executable Unit and Custom fixed unit-variance scoring, but remain unqualified; `none` and `sum_to_one` fixed normalizations remain deliberately non-executable;
- complete standardized, mean-centered, unstandardized, location-parameter, and result-table breadth;
- the full supported SEM model-shape surface through the implemented SemModelV4 compiler;
- every difficult-model combination required by the product-finalization matrix.

These items are a non-blocking future full-parity backlog. They do not prevent the documented bounded `pls_pm_v1` contract from being scoped Standard, and they must not be implied by that narrower claim.

## Evidence truth

The current-source QualificationSpec V2 lane may support a source-contract candidate descriptor and source-level oracle work. A separate ignored, environment-gated `quickpls-desktop` lib-test harness is available to create a four-variant, non-promotional work envelope through the real Recipe-v4 compiler, runner, native canonical builder, and schema-6 append/reopen path. No such work envelope is attached to the QualificationSpec, and it cannot support `engine_only` or any higher evidence state until the complete required role contract is satisfied.

The following remain mandatory only before any future full-parity claim:

- current-build kernel and product-to-oracle receipts;
- qualification-sized recovery, bias, failure-rate, and adversarial matrices;
- worker, desktop/native, save/reopen, and schema-6 archive equality;
- cross-format semantic export readback;
- installed and portable Windows, accessibility, scaling, cancellation, performance, and soak evidence;
- independent scientific review.

QualificationSpec V2 does not admit historical v1 reports or its compatibility fixture as evidence for broader parity. Independently, the current source-bound method-promotion lane qualifies the documented bounded contract, so the Capability Registry V2 and method manifest are `partial / release_qualified / Standard` and `release_qualified`, respectively. The empty V2 receipt set cannot downgrade or expand that scoped claim.

## Current-build handoff

Future full-parity product work must be generated only after scientific product sources freeze. The orchestrator builds the `quickpls-desktop` lib test, selects the exact test executable from Cargo JSON, binds its SHA-256 and size plus the source/scenario identities, and directly invokes only the ignored Internal harness. It must reject a stale or substituted executable. The customer `qpls` CLI is available for the separately governed bounded scoped-Standard contract; it is not a substitute for QualificationSpec V2 full-parity receipts. V2 output remains non-overwriting, work-only, and unable to mutate Registry, manifest, Standard, beta, or release state.
