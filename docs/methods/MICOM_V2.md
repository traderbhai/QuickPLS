# MICOM v2

Status: validated for the bounded scope below. The independent numerical comparison, persistence checks, native workflow acceptance, real XLSX export, and packaged-desktop save/reopen gates pass. `micom_v1` remains withdrawn and must never be presented as current evidence.

MICOM v2 is QuickPLS's independently implemented, two-group measurement invariance of composites procedure. It follows the three-step hierarchy introduced by Henseler, Ringle, and Sarstedt (2016), *International Marketing Review*, 33(3), 405–431, [doi:10.1108/IMR-09-2014-0304](https://doi.org/10.1108/IMR-09-2014-0304). The open paper describes the hierarchy and equations used here: [Testing measurement invariance of composites using partial least squares](https://ris.utwente.nl/ws/files/287320134/2016_IMR_Henseler_Ringle_Sarstedt.pdf).

## Bounded execution contract

MICOM v2 runs only as part of the current two-group MGA workflow when all of the following are true:

- The weighting scheme is path weighting, preprocessing is standardized, and missing cases are removed listwise.
- Exactly two distinct observed group values are selected, with at least ten complete model cases per group.
- The recipe requests exactly `group_methods = "micom,mga_permutation"` and between 5,000 and 10,000 usable permutations.
- The researcher explicitly records `micom_configural_confirmed = "true"` after reviewing Step 1.
- Case weights, generated interactions, and higher-order constructs are absent.

The same model, indicators, data treatment, weighting scheme, convergence settings, and orientation rules are used for the pooled, original-group, and permuted-group fits. QuickPLS sign-aligns each group solution to the pooled solution before comparing it.

## Step 1: configural invariance

Step 1 is a prerequisite, not an automatically inferred statistical result. The application checks computational conditions that it can verify, including identical indicator assignments, model structure, preprocessing, and algorithm settings. The researcher must separately confirm that indicator wording, coding, treatment, and substantive meaning are equivalent across groups.

The confirmation flag records that review; it is not proof of semantic equivalence. Without it, the calculation is blocked.

## Step 2: compositional invariance

For each construct, QuickPLS estimates the original Group A and Group B outer-weight vectors and applies both to the same pooled raw indicator matrix. With effective raw-scale coefficients `a_gj = w_gj / s_gj`, the two proxies are

`C_A = X a_A` and `C_B = X a_B`,

and the observed statistic is the paper's Equation (4):

`c = cor(C_A, C_B) = cor(X a_A, X a_B)`.

Every label permutation preserves the original group sizes, re-estimates both group PLS models, sign-aligns the solutions, reapplies the permuted weights to the pooled indicators, and stores a new correlation. At confidence `1 - alpha`, compositional invariance is supported when the observed `c` is not below the empirical lower `alpha` quantile. The payload also reports an add-one lower-tail empirical p value.

## Step 3: equality of means and variances

QuickPLS fits the selected pooled observations once and retains those pooled construct scores. For the original and every permuted assignment, it computes:

- Mean difference: `d_mean = mean(C_pooled,A) - mean(C_pooled,B)`.
- Dispersion difference: `d_var = log(var(C_pooled,A) / var(C_pooled,B))`.

These correspond to the mean and log-variance-ratio hypotheses in Henseler et al. (2016). Equality is supported when the observed difference lies inside the central permutation interval. QuickPLS uses Type-7 quantiles at `alpha / 2` and `1 - alpha / 2` and reports add-one two-tailed empirical p values.

Step 3 is interpreted only after Steps 1 and 2. A confidence interval containing zero is useful descriptive evidence, but the persisted QuickPLS equality decision is based on whether the observed statistic falls inside its permutation interval, matching the procedure's comparison rule.

## Hierarchical decisions

- No invariance: Step 1 is not confirmed or Step 2 is not supported.
- Partial invariance: Step 1 is confirmed and Step 2 is supported.
- Full invariance: partial invariance is supported and both Step 3 equality decisions are supported.

Partial invariance permits comparison of standardized structural coefficients for the affected composites. Pooling groups requires full invariance and still requires the researcher to consider structural heterogeneity.

## Determinism and payload

`micom_v2` records the selected groups and case counts, requested/usable/attempted/failed permutations, confidence level, and one row per construct. Each construct row contains the observed compositional correlation, lower reference quantile, lower-tail p value, mean and log-variance-ratio differences, central interval bounds, two-tailed p values, equality flags, and partial/full decisions.

Permutation order is derived deterministically from the recipe seed. Singular or non-convergent assignments are counted and skipped; execution succeeds only after the requested number of usable fits is collected within the documented attempt bound.

## Interpretation limits

MICOM is evidence about composite measurement invariance within this bounded calculation contract. It does not establish construct validity, correct indicator semantics, causal identification, population representativeness, or the absence of unobserved heterogeneity. A failed Step 2 should stop interpretation of that composite's MGA result. A failed Step 3 prevents a full-invariance or pooling claim but does not erase partial invariance when Steps 1 and 2 hold.

## Independent validation

`validation/micom_v2_reference.py` is a validation-only NumPy implementation. It does not import QuickPLS production code. Its deterministic fixture includes a full-invariance control, a location shift, a dispersion shift, and a compositional shift. It independently re-estimates the pooled, original-group, and permuted-group models and compares the complete current payload, including paths, loadings, weights, MICOM intervals, p values, decisions, and permutation accounting.

Run the paired 5,000-permutation comparison with a separately built release CLI:

```powershell
$env:QUICKPLS_CLI_PATH = "D:\QuickPLS\target\release\qpls.exe"
python validation/micom_v2_reference.py --run-quickpls --permutations 5000
python validation/micom_method_promotion_audit.py
```

The reference-only mode is intentionally non-promotable. Promotion also requires the product, persistence, export, and packaged-desktop gates described at the top of this document.
