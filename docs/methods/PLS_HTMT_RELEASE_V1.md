# PLS HTMT and HTMT+ Release Contract v1

Status: frozen release contract. No promotion evidence is admitted by this document. The mathematical definition remains authoritative in `PLS_HTMT_V1.md`.

## Stable capability

The stable capability identity is `qpls3.assessment.htmt` with primary method identity `ringle_et_al_htmt_plus_v1`; the same result payload also carries `henseler_et_al_htmt_v1` for original signed HTMT. Both are deterministic assessments of construct pairs from the same model-wide complete cases.

Original signed HTMT is `mean(r_cross) / sqrt(mean(r_within_i) mean(r_within_j))` following Henseler, Ringle, and Sarstedt (2015), DOI `10.1007/s11747-014-0403-8`. HTMT+ replaces each correlation by its absolute value following Ringle et al. (2023), DOI `10.1016/j.dib.2023.109074`. HTMT+ is not clamped at one. No cutoff is embedded in the point calculation. The separately versioned complete-bootstrap inference reports the documented one-tailed `.05` decision by testing whether its 90% bias-corrected upper bound is strictly below `.90`.

## Release boundary

Only reflective construct pairs with at least two indicators each are applicable. The output preserves construct order, signed/absolute definition, typed unavailable reasons, symmetric cells, and exact method versions. Formative blocks, single-indicator blocks, zero or invalid monotrait denominators, non-finite correlations, and malformed matrices are explicit unavailable/error states.

Legacy assessment v2-v5 absolute-correlation matrices remain readable under their historical payload identity. They cannot be relabeled as either current method. Existing independent fixtures and engine tests are useful implementation inputs, but this release manifest deliberately starts with empty evidence and requires fresh identity-bound reports before promotion.

## Interpretation limits

HTMT is evidence about discriminant validity under its measurement assumptions. It does not establish reliability, convergent validity, causal validity, or overall model fit. The `.90` inference decision follows the documented parity workflow; stricter context-specific criteria still require researcher justification.

## Inference boundary

`htmt_bias_corrected_bootstrap_inference_v1` is produced only by the complete
ordinary PLS bootstrap. It uses indexed case resampling, no replacement of
failed preplanned draws, a BC Type-7 interval (not BCa), and a minimum 90%
usable-replicate rule per construct pair. Exact contributing replicate indices
are digest-bound; pair-specific unavailable indices and reasons are retained.
The current recipe does not yet expose a generic one-tailed/two-tailed HTMT
choice, so this method identity must remain limited to the documented
one-tailed upper `.05` workflow.

## QualificationSpec V2 status

`validation/qualification_v2/htmt_plus_v1.qualification.json` is the frozen
source of truth for the full qualification workload. It binds the capability
cell `smartpls.htmt / qpls3.assessment.htmt`, both point definitions, the
complete-bootstrap inference identity, two independent validation
implementations, the seven mandatory scenario axes, a worst-case Monte Carlo
half-width of at most one percentage point, adversarial cases, archive and
cross-format contracts, installed/portable Windows matrices, cancellation,
and scale budgets.

The specification is intentionally `compatibility_only` and contains no
evidence receipts. The method factory writes lightweight checks only as
`*.source_audit.json`; those files cannot be admitted as qualification
evidence. A `*.identity.json` can be created only from an explicitly supplied,
passing, source-bound qualification execution with no blockers, and admission
to the promotion manifest remains a separate reviewed action.

The current source-completeness report is
`validation/results/method_factory/htmt_plus_v1/qualification_gap.json`. A
passing source-completeness audit means the validation machinery is ready; it
does not mean HTMT is product-qualified. The capability remains absent from
Standard and unpromoted until every immutable V2 receipt, broader product
surface, packaged Windows run, scale run, and independent scientific review is
complete.
