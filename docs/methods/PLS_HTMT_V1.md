# HTMT and HTMT+ Specification v1

Status: frozen for experimental `pls_assessment_v6` and retained by current `pls_assessment_v7`. Neither variant is validated or publication-ready.

## Definitions

For reflective constructs `i` and `j` with `K_i,K_j >= 2`, use Pearson indicator correlations from the same model-wide complete cases as estimation.

Original signed HTMT follows Henseler, Ringle, and Sarstedt (2015), DOI `10.1007/s11747-014-0403-8`:

`HTMT_ij = mean(r_cross) / sqrt(mean(r_within_i) * mean(r_within_j))`.

Both within-block arithmetic means must be strictly positive beyond numerical tolerance. A nonpositive mean makes the original signed statistic unavailable because its measurement assumptions and real denominator are not satisfied. The cross-block mean may be negative, so a finite signed HTMT may be negative.

HTMT+ follows Appendix A.1 of Ringle et al. (2023), DOI `10.1016/j.dib.2023.109074`:

`HTMT+_ij = mean(abs(r_cross)) / sqrt(mean(abs(r_within_i)) * mean(abs(r_within_j)))`.

HTMT+ is invariant to individual indicator sign reversal. It is not mathematically bounded by one and must never be clamped; a value above one is valid output indicating severe discriminant-validity failure. This is HTMT+, not the distinct geometric-mean HTMT2+ statistic.

## Applicability and Numerics

- Only pairs of reflective constructs with at least two indicators each are applicable.
- Applicable diagonal cells are exactly one as a presentation identity.
- Formative participation uses `htmt.formative_not_applicable`.
- A reflective block with fewer than two indicators uses `htmt.single_indicator_not_applicable`.
- HTMT+ with a zero/effectively-zero within-block absolute mean uses `htmt.zero_monotrait_denominator`.
- Original HTMT with a nonpositive/effectively-zero signed within-block mean uses `htmt.original_nonpositive_monotrait_mean`.
- A zero cross-block numerator with a valid denominator is available and equals zero.
- Correlations use deterministic recipe indicator order. Nonfinite correlations are an assessment error, not a null cell.
- No `.85` or `.90` cutoff is enforced; thresholds are interpretation choices.

Use `tol = 64*f64::EPSILON`. A denominator component `<= tol` is unavailable. Correlation roundoff within `tol` of `[-1,1]` is canonicalized; a material value outside that range is a numerical error.

## Persistence

Current `pls_assessment_v7` writes no ambiguous legacy `htmt` field. It writes:

- `htmt_plus_method_version = "ringle_et_al_htmt_plus_v1"` and typed `htmt_plus`;
- `htmt_original_method_version = "henseler_et_al_htmt_v1"` and typed `htmt_original`.

Each artifact records construct order, `correlation_type = "pearson"`, whether absolute correlations are used, and a symmetric matrix of typed cells containing `value`, `status`, and `reason`. Project validation binds construct order and applicability to the immutable recipe and validates symmetry, diagonal policy, reason/status shapes, and method metadata.

Assessment v2-v5 `htmt` matrices remain readable exactly as stored and are interpreted as legacy absolute-correlation HTMT+ output. They cannot carry v6 artifacts. V6 cannot carry the legacy field.

## Fixtures

- `validation/htmt_reference.py` independently computes Pearson sample correlations and the frozen original HTMT/HTMT+ formulas without calling the Rust engine.
- `validation/results/htmt_reference.json` stores the corporate-reputation baseline matrices plus positive-affine, individual sign-reversal, and construct-order fixture variants.
- `validation/results/htmt_csem_comparison.json` stores executable cSEM 0.6.1 agreement for original signed HTMT via `calculateHTMT(..., .absolute = FALSE)`. It also records that cSEM `.absolute = TRUE` is not equivalent to Ringle et al. HTMT+ for mixed-sign cross-block correlations, so it is a definition-difference probe rather than HTMT+ validation.
- `validation/results/htmt_seminr_comparison.json` stores executable seminr 2.5.0 agreement for HTMT+ on the corporate-reputation fixture, including mixed-sign cross-block correlations.
- `validation/results/htmt_published_ringle_2023.json` stores the rounded Appendix A.1 HTMT+ worked examples from Ringle et al. (2023), DOI `10.1016/j.dib.2023.109074`; `htmt_plus_matches_ringle_2023_rounded_formula_examples` checks them within `1.5e-3` to account for the article's rounded intermediate means.
- The Rust assessment test `htmt_matches_independent_corporate_reputation_reference` compares current `pls_assessment_v7` output against the committed independent baseline within `1e-12` and verifies HTMT+ values above one are preserved.
- Reproduce the three rounded Appendix matrices from Ringle et al. within `5e-4`.
- Verify all-positive equality between original HTMT and HTMT+.
- Verify individual sign-reversal invariance for HTMT+, and the expected original-HTMT change/unavailability.
- Verify a valid HTMT+ value above one is not clamped, zero numerator returns zero, and zero/nonpositive within means are typed unavailable cells.
- Cover two-item minima, formative/single-item applicability, permutation/affine invariance, symmetry, diagonals, legacy v1-v5 reads, and v6 metadata/status/reason tampering.
