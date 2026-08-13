# PLS HTMT and HTMT+ Release Contract v1

Status: frozen release contract. No promotion evidence is admitted by this document. The mathematical definition remains authoritative in `PLS_HTMT_V1.md`.

## Stable capability

The stable capability identity is `qpls3.assessment.htmt` with primary method identity `ringle_et_al_htmt_plus_v1`; the same result payload also carries `henseler_et_al_htmt_v1` for original signed HTMT. Both are deterministic assessments of construct pairs from the same model-wide complete cases.

Original signed HTMT is `mean(r_cross) / sqrt(mean(r_within_i) mean(r_within_j))` following Henseler, Ringle, and Sarstedt (2015), DOI `10.1007/s11747-014-0403-8`. HTMT+ replaces each correlation by its absolute value following Ringle et al. (2023), DOI `10.1016/j.dib.2023.109074`. HTMT+ is not clamped at one. A cutoff is an interpretation setting and is never embedded in the calculation.

## Release boundary

Only reflective construct pairs with at least two indicators each are applicable. The output preserves construct order, signed/absolute definition, typed unavailable reasons, symmetric cells, and exact method versions. Formative blocks, single-indicator blocks, zero or invalid monotrait denominators, non-finite correlations, and malformed matrices are explicit unavailable/error states.

Legacy assessment v2-v5 absolute-correlation matrices remain readable under their historical payload identity. They cannot be relabeled as either current method. Existing independent fixtures and engine tests are useful implementation inputs, but this release manifest deliberately starts with empty evidence and requires fresh identity-bound reports before promotion.

## Interpretation limits

HTMT is evidence about discriminant validity under its measurement assumptions. It does not establish reliability, convergent validity, causal validity, overall model fit, or a universal threshold decision.
