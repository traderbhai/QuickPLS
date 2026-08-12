# MICOM v1

Status: withdrawn and execution-disabled pending a scientifically valid reimplementation and independent validation.

QuickPLS retains the `micom_v1` result schema only so older project archives can still be read. New production analyses must not emit this payload. Recipe validation and estimator execution both reject metadata whose comma-separated `group_methods` contains `micom`.

## Reason for withdrawal

The previous routine did not estimate group-specific composite weights for the original and permuted groups. Its reported compositional correlation therefore did not implement the MICOM compositional-invariance procedure and could produce invalid invariance decisions. The prior QuickPLS v1.2.2 validation claim is withdrawn.

## Runtime behavior

- `AnalysisMethod::Mga` remains available for the documented two-group MGA and group-label permutation-MGA scopes.
- Adding `micom` to `group_methods` is a blocking validation error and a defense-in-depth estimator error.
- Permutation MGA does not establish measurement invariance. Confirmatory interpretation requires invariance evidence from a qualified external method until QuickPLS has a correct, independently validated MICOM implementation.
- Existing `micom_v1` payload fields remain deserializable for archive compatibility, but they are historical output and must not be treated as validated evidence.

## Re-promotion requirements

A future MICOM implementation needs group-specific weight estimation under every original and permuted assignment, configural-invariance checks, reproducible permutation inference, invariant and non-invariant fixtures, independent numerical comparison, and explicit product/export qualification. `npm run qpls:promotion:micom` now audits the safety withdrawal instead of promoting the former routine.
