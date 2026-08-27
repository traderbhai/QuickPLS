# QuickPLS 2.56.0 Release Notes

QuickPLS 2.56.0 introduces the qualified MultiMod moderation and heterogeneity suite as an additive extension to the existing offline PLS-SEM desktop application.

## Release identity

| Item | Qualified value |
|---|---|
| Version | `2.56.0` |
| Source commit | `28939b73db8f2284f21ce184050eb3f04110bf94` |
| Distribution | QuickPLS 2.56.0 Beta (Unsigned) on GitHub |
| Setup artifact | 231,889,558 bytes; SHA-256 `46b121d252ba236f4e8d72e5e618bbf4542d0cdb0819b834fb9b3d3677599523` |
| Portable artifact | 79,480,832 bytes; SHA-256 `54abad31daca14dbad16588f85015fb39b030bac28e1cd475a10d4d4573cb2d1` |
| Qualification | 32/32 gates passed |
| Windows smoke | Installed and portable offline smoke passed |

This is not a signed or stable-channel release. Windows may show an unknown-publisher warning. Verify the downloaded artifact against the exact SHA-256 value above before running it.

## What is new

### Categorical moderation and multigroup analysis

- New 2-20-group workflow with typed group selection, stable exclusions, minimum-size and imbalance controls, structured MICOM Step 1 review, pairwise MICOM Steps 2-3, and a K-group max-spread omnibus permutation statistic.
- Deterministic pairwise permutation, directional Henseler PLS-MGA probability, BC bootstrap differences, parametric sensitivity cells, and explicit multiplicity handling with Holm as the confirmatory default.
- Independently bounded cells for ordinary General SEM, multiple two-way and one three-way moderation, bounded moderated mediation, multiple disjoint HOCs, case and frequency weights, and reflective PLSc.

### FIMIX-PLS and PLS-POS latent segmentation

- New FIMIX-PLS V2 EM implementation with pooled scores, posterior probabilities, class paths and variances, multi-start convergence records, collapse checks, and likelihood-based model criteria.
- Separately named publication-faithful structural-only and destination-scored-interaction PLS-POS V2 profiles.
- Explicit candidate `K` review, fixed-method/fixed-`K` bootstrap, exhaustive label alignment for `K≤5`, and common-metric comparability gating for POS interaction contrasts.

### Conditional-process analysis

- New explicit-path workflows for bounded both-stage, multiple two-way, three-way, longer-path, multiple-HOC, grouped, case-weighted, and frequency-weighted moderated mediation.
- Bounded joint probes and contrasts, shared bootstrap ledgers, percentile, full delete-one BCa, nested studentized, and one-sided inference only where the selected profile qualifies them.
- Correct estimand naming: a scalar index is reported only for an indirect effect affine in one moderator; more complex effects receive derivatives and probe contrasts.

### Observed-data interventional mediation

- Separate parametric g-computation workflow for observed treatments, mediators, outcomes, baseline moderators, and an explicit adjustment set.
- Required temporal-order, identification, and positivity review.
- Mandatory result wording: **assumption-dependent interventional estimate**. The module does not state that causality has been established.

## Evidence, projects, and results

- Added versioned MultiMod recipe/result identities and strict profile eligibility with stable fail-closed reason codes.
- Added Archive V6 with backward reading of compatible V5 projects and compressed, SHA-256-checked Arrow sidecars for large evidence.
- Added strict reopen rejection for missing, altered, duplicate, schema-mismatched, or identity-mismatched sidecars.
- Added deterministic resampling ledgers, provenance, exclusion/failure reporting, qualified semantic exports, accessible virtualized tables, cost warnings, and no publication of partial scientific results after cancellation.

## Compatibility preserved

- Continuous moderation V1 behavior and serialization are unchanged.
- Legacy MGA, segmentation, moderated-mediation, recipe, result, and archive contracts are not reinterpreted by the new V2/V4 families.
- Quadratic/self-moderation is outside MultiMod. The separate existing nonlinear diagnostic is not promoted into this suite.

## Qualification scope

The exact candidate identified above passed every one of the 32 campaign gates, including installed and portable offline Windows smoke. Qualification covers the explicitly admitted method/profile cells, not every possible combination of groups, HOCs, weights, interactions, estimands, and inference procedures.

The final smoke evidence used sealed corrections to the temporary validation drivers. Those corrections did not change the product or package bytes. The qualification record does not claim that an ordinary clean-source smoke run is equivalent to that sealed validation setup; it claims the exact packaged candidate and evidence identity recorded in the release manifest.

The **Standard · Release-qualified** presentation depends on exact embedded authority matching the candidate source and evidence. A source or development build without that complete authority is expected to remain **Experimental Labs · Unqualified** or unavailable. That behavior is a release safeguard, not a product defect.

## Known boundaries

- GitHub distribution is QuickPLS 2.56.0 Beta (Unsigned); it is not code-signed or represented as Stable.
- QuickPLS does not import SmartPLS project files or claim full parity or numerical identity with SmartPLS.
- Unsupported MultiMod intersections fail closed rather than silently simplifying the requested model.
- Conditional-process PLS results are not causal. The separate interventional module remains assumption-dependent and supports only its qualified observed-data profile.
- Python and R references are qualification tools only; the shipped Windows application remains self-contained and offline for analysis.

See [QuickPLS 2.56.0 Features](FEATURES_V2_56_0.md), [Method Compatibility](METHOD_COMPATIBILITY.md), [MultiMod boundaries and qualification](MULTIMOD_BOUNDARIES_AND_QUALIFICATION_V1.md), and [unsupported intersections](MULTIMOD_UNSUPPORTED_INTERSECTIONS_V1.md).
