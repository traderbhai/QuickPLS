# Necessary Condition Analysis v2

Status: validated for the bounded standalone raw-data scope below, including independent numerical evidence, strict archive persistence, and packaged-native setup/results/XLSX/save/reopen acceptance. This specification does not claim full SmartPLS or NCA-package parity.

`nca_v2` performs a two-variable Necessary Condition Analysis for one observed numeric condition X and one observed numeric outcome Y. It provides CE-FDH, CR-FDH, seeded permutation p values, and observed-range bottleneck rows.

## Input contract

- `settings.method = "nca"`.
- `metadata.nca_x` and `metadata.nca_y` identify two different numeric columns.
- `metadata.nca_ceiling` is `ce_fdh`, `cr_fdh`, or `both`; the default is `both`.
- `metadata.nca_permutation_samples` is an integer from 1 through 10,000; the default is 999.
- X and Y are used on their original numeric scales: `preprocessing = "unstandardized"`.
- Missing X/Y pairs and non-finite values are removed together by listwise deletion. At least three complete rows and nonconstant X and Y are required.
- Case weights, external bootstrap, studentized bootstrap, and the generic PLS permutation plan are rejected. `weighting_scheme = "path"` is a schema sentinel only; no PLS weighting is performed.
- The embedded recipe model may be empty and is not an editable SEM model.

## CE-FDH

Rows are sorted by ascending X. For duplicate X values, the largest Y is retained. Scanning those unique X values from left to right, a point becomes a CE-FDH peer only when its Y is strictly greater than every earlier retained Y.

The peers therefore have strictly increasing X and Y. They define a nondecreasing step frontier: each peer's Y extends horizontally to the next peer's X. The CE-FDH effect size is the empty area above that frontier divided by the observed scope area

`(max(X) - min(X)) * (max(Y) - min(Y))`.

## CR-FDH

CR-FDH is the ordinary least-squares line `Y = intercept + slope * X` fitted through the CE-FDH peers. Its effect size is the empty area above the line within the observed X/Y scope, clipping the fitted line at the observed Y bounds before integrating.

CE-FDH rows report null `slope` and `intercept`. CR-FDH rows report the fitted finite values.

## Permutation test

For each requested ceiling, every replicate independently permutes Y while holding X fixed, rebuilds the CE peers, and recomputes that ceiling's effect. Streams are derived from the recorded master seed, ceiling identifier, and replicate index using the `quickpls:nca-permutation:v2` domain. The reported one-sided empirical value is

`(1 + count(permuted effect >= observed effect)) / (B + 1)`.

Accordingly, the p value is finite, lies in `[1/(B+1), 1]`, and lies exactly on the `1/(B+1)` lattice. Repeating the same data, settings, and seed gives the same NCA payload.

## Bottlenecks

Each selected ceiling produces rows for outcome levels 10% through 90% of the observed Y range. `required_x_percent` expresses the required X on the observed X range. It is null when no X is required or the requested outcome is not attainable under that ceiling. `status` disambiguates the null:

- `required`: `required_x_percent` is finite from 0 through 100;
- `not_necessary`: the frontier already reaches the outcome at the minimum observed X; or
- `not_attainable`: the frontier does not reach the outcome inside the observed X range.

## Result and provenance contract

The runner records `provenance.method = "nca"` and exact `provenance.method_version = "nca_v2"`. The existing typed envelope is `payload.kind = "pls_pm_v1"`; its estimation object has `method_version = "nca_v2"` and a single `nca` object containing:

- X/Y names, observation counts, selected ceiling, requested/usable permutation counts, and warnings;
- `scope` with `minimum_x`, `maximum_x`, `minimum_y`, and `maximum_y`;
- ordered `ce_fdh_peers` points;
- `ceilings` with ceiling name, effect size, permutation p value, slope, and intercept; and
- per-ceiling `bottlenecks` with outcome percent, nullable required X percent, and status.

PLS assessment is not applicable. The assessment sentinel remains versioned as `assessment_not_applicable_v1`, with the versionless user-facing warning that PLS assessment is not applicable to standalone raw-data analyses. There is no external bootstrap or permutation payload.

Project append, save, and reopen require the recipe, result, nested method versions, settings, ceiling geometry, effect sizes, bottlenecks, warnings, and assessment sentinel to satisfy this contract. A mismatched or internally tampered payload is rejected. Standalone recipe placeholders are excluded from active-model resolution, so reopening an NCA-only project does not create a phantom model.

## Explicit exclusions

- Multiple simultaneous conditions or multiple outcomes.
- Latent-variable-score NCA.
- cIPMA or IPMA/NCA integration.
- Categorical or ordinal encoding.
- Ceiling techniques other than CE-FDH and CR-FDH.
- Theoretical ranges, alternative bottleneck grids, pairwise deletion, case weights, or inferential procedures other than the documented permutation test.
- Full feature, numerical, visual, or workflow parity with SmartPLS or the R NCA package.

## Qualification evidence

- `validation/v08_extended_methods_reference.py --section nca` independently constructs CE peers, fits the CR line with NumPy least squares, integrates both empty-ceiling areas, checks every bottleneck row, verifies the p-value lattice, and repeats the seeded run.
- `qpls-estimation` hand fixtures verify known peers, effects, CR coefficients, bottlenecks, independent permutation streams, and geometry-tamper rejection.
- `qpls-core` verifies the no-model recipe and rejects ambiguous variables or incompatible settings.
- `qpls-project` exercises genuine runner append, save, reopen, strict contract tampering, and explicit `nca_v1` legacy compatibility.
- the desktop project snapshot test proves that a saved/reopened NCA-only project has no canonical or active phantom model.
- `validation/results/v247_tauri_native_acceptance.json` proves the genuine model-free native setup, active lifecycle, result tables and accessible ceiling plot, XLSX export, explicit save, and same-run reopen contract.

Method definitions were checked against the official SmartPLS NCA overview and the CRAN NCA package's CE-FDH, CR-FDH, peer, and bottleneck sources. Those references define the techniques; QuickPLS remains an independent implementation.
