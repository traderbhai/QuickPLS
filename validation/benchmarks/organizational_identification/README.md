# Organizational Identification screenshot parity benchmark

This benchmark supports the bundled `organizational_identification` project, titled **Organizational Identification Model**. It preserves the supplied workbook as a deterministic values-only CSV fixture and compares QuickPLS point estimates with the 27 values displayed in the supplied model screenshot.

The checked-in evidence is:

- `validation/fixtures/organizational_identification_v1.csv`
- `validation/fixtures/organizational_identification_v1.provenance.json`
- `validation/benchmarks/organizational_identification/screenshot_reference_v1.json`
- `validation/benchmarks/organizational_identification/mediation_reference_v1.json`
- `validation/benchmarks/organizational_identification/moderation_reference_v2.json`
- `validation/benchmarks/organizational_identification/moderated_mediation_reference_v1.json`
- `validation/benchmarks/organizational_identification/higher_order_reference_v1.json`

All five OI-based bundled projects reference the same values-only CSV asset. The fixture is not copied for each alternative model. The two-outcome moderation model is bundled as a strict General SEM schema-6 sample because its simultaneous interactions are outside the ordinary project's single-interaction persistence contract.

## Data preparation

The source workbook contains 305 observations and 22 integer-valued columns. Twenty-one 1–5 indicators form the pictured model. `gender` is retained as an unmodeled coded numeric variable with 157 code-1 cases and 148 code-2 cases; no labels are inferred for those codes, and the default CSV import records numeric/continuous metadata.

The workbook contains no missing, formula, error, sentinel, or out-of-range cells, so the fixture requires no imputation, reverse coding, row filtering, or other statistical cleaning. Two complete response rows occur twice: Excel rows 39/40 and 229/274. A third indicator-only duplicate pattern occurs at rows 12/214 because their `gender` codes differ. These records are preserved because the workbook contains no respondent identifier that would justify deletion.

The CSV preserves the original row and column order and uses UTF-8 without a byte-order mark, comma delimiters, minimal RFC 4180 quoting, LF line endings, base-10 integer text, and a terminal LF. The source workbook contains creator and data-connection privacy metadata; the checked-in CSV contains values only and does not reproduce that metadata.

## Model and run settings

All four constructs use reflective Mode A measurement:

- `org_prestige`: `org_pre1` through `org_pre8`
- `org_identification`: `org_ident1` through `org_ident6`
- `affective_commitment_joy`: `ac_joy1` through `ac_joy4`
- `affective_commitment_love`: `ac_love1` through `ac_love3`

The structural model contains these paths:

- `org_prestige -> org_identification`
- `org_identification -> affective_commitment_joy`
- `org_identification -> affective_commitment_love`

The verified QuickPLS run uses `pls_pm_v1`, path weighting, standardized indicators, listwise missing-data handling, initial unit outer weights, a `1e-7` stop criterion, and at most 3,000 iterations. It uses all 305 observations, omits none, converges in seven iterations with a final maximum outer-weight change of `8.33536528688228e-9`, and emits no warnings.

## Verified result

After rounding QuickPLS estimates to three decimals, all 21 outer loadings, three structural paths, and three endogenous R-squared values match the screenshot: 27 of 27 displayed values. The `org_identification -> affective_commitment_love` coefficient is genuinely negative (`-0.409`) and must retain its sign.

The canonical CSV imports with dataset fingerprint:

```text
v2:fa5968177bc154d04ae8bfbba9853c56126a07ff84fc8d7d40cf3a46c6d8290a
```

## Reproducibility checksums

- Source workbook SHA-256: `5d803952b8009d406ab2f6317527d3df646de083ea86aa0b534a65b339713ae7`
- Canonical CSV SHA-256: `5066d3b4bd24d14ad5d3efc91c1c40c57c41de63d987456d2cf8aad40c20ceed`
- Supplied screenshot SHA-256: `6809e7f76ed209f8f83dfc2c16fa057c1f5d6e5f09633a9507eb3dd278e83544`
- Screenshot reference JSON SHA-256: `9c1c6848f055ec2f3c00b1d3fe81ddb9fc973a4cc279b0399ffa54ca339af4fa`

## Interpretation boundary

The screenshot exposes values to three decimals. Passing this benchmark proves equality of the displayed values at that precision; it does not establish bitwise equivalence with unexposed calculations from another engine. A full-precision external result export should be recorded as a separate reference if one becomes available.

The mediation variant matches its supplied screenshot at the displayed three-decimal precision. The two-outcome moderation sample estimates both outcome equations and both strong-hierarchy interactions jointly through the qualified General SEM point cell. Its 21 loadings, two focal paths, and two scientific gamma values are stored in the canonical result; the two screenshot R-squared values are independently reproduced from the same joint equations but are not yet a typed canonical result field. The sample therefore discloses 25 directly persisted and two reconstructed screenshot-comparable values, and makes no causal-moderation claim. The moderated-mediation sample remains explicitly a point topology: it freezes the current qualified single-interaction PLS point result and separately records the nearby supplied screenshot values. The dedicated moderated-mediation method is not bundled because its result is not yet covered by the ordinary-project persistence contract; conditional-effect inference requires a separately qualified workflow. The higher-order reference freezes QuickPLS's supported disjoint two-stage result and separately records the nearby screenshot values, so it does not claim screenshot parity.
