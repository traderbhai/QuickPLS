# PLS post-hoc technical minimum sample size v2

Status: scoped Standard inference-aware implementation with engine, result,
export, and archive contracts. Standard applies only to the conservative
complete-linked-bootstrap v2 contract documented below; it is not a claim of
unqualified SmartPLS numerical identity.
It is distinct from prospective Monte Carlo power analysis
(`qpls3.pls.sample_size_power`).

## Scientific contract

For the selected driver coefficient `beta_min`, the result reports:

`ceil((2.486 / abs(beta_min))^2)`

The inverse-square-root calculation fixes `alpha = 0.05`, statistical
power `= 0.80`, and a directional test. The driver selection is the structural
path with the smallest absolute coefficient among paths whose complete linked
PLS bootstrap two-sided normal-reference p value is at most 0.05. Ties are
resolved by source stable ID and then target stable ID in ascending binary
string order. The payload records the selection rule, inference source,
threshold, eligible and significant path counts, driver identity, driver p
value, coefficient magnitude, analytical sample size, required sample size,
and adequacy comparison.

Point estimates do not establish significance. A PLS algorithm run without
bootstrap inference therefore returns `inference_unavailable` and no numeric
technical requirement. Missing, non-finite, duplicated, or mismatched path
probabilities return `inference_incomplete`. If complete inference contains no
path at or below 0.05, the result is
`no_statistically_significant_path`. No fallback silently substitutes the
smallest point estimate.

No-path, zero-path, and unrepresentable values are typed respectively as
`not_applicable_no_structural_path`, `undefined_zero_path`, and
`exceeds_supported_integer_range`. The implementation never substitutes an
epsilon, emits NaN or infinity, retries inference, or clamps the requirement.

## Workflow and interpretation

The result belongs only to PLS-SEM and is never attached automatically. A new
run must carry the exact typed Standard option
`pls_posthoc_technical_minimum_sample_size`, including its versioned capability
cell, method version, closed `base_analysis`, and matching inference contract.
`base_analysis = pls_bootstrap` requires a real nonzero case-bootstrap plan and
the two-sided normal-reference inference label; `base_analysis = pls_algorithm`
requires `point_estimate_only` and produces the explicit inference-unavailable
state. Permutation and studentized combinations are rejected before execution.
The runner attaches the numerical result only after the linked PLS bootstrap
result is complete; the same bootstrap parameter identities, original
coefficients, and probabilities are checked again by Results, exports, and
archive validation. The point-only Recipe-v4 PLS adapter accepts only the exact
point-analysis option and records it in immutable runner provenance.
The canonical result uses a dedicated typed table attributed only to
`qpls3.pls.posthoc_technical_minimum_sample_size`, rather than coercing numeric
sample-size and probability cells to display text. A numeric result is linked
to the same analytical observation count and is available in Results,
canonical reporting, CSV, HTML, and XLSX export paths.
The native table is marked validated, and tabular CLI exports carry an
`availability = Standard` row bound to the exact registry cell.

This is a retrospective technical-power diagnostic. It does not establish
population representativeness, causal validity, model correctness,
substantive relevance, or an a-priori recruitment target. Subgroup,
multigroup, missing-data, complex-sampling, attrition, and focal-effect
requirements remain separate research-design considerations.

## Documented workflow difference and Standard boundary

The official SmartPLS page says both that the result is available immediately
after the PLS-SEM algorithm and that the original inverse-square-root method
uses the smallest absolute statistically significant path. The page separately
recommends bootstrapping for significance inference, but does not specify which
bootstrap interval/test, one- versus two-sided selection probability, failed-fit
policy, or behavior when no path is significant drives this algorithm result.

QuickPLS v2 freezes the scientifically explicit, conservative boundary: it
requires complete two-sided normal-reference bootstrap probabilities and never
invents significance for an algorithm-only run. This closes the prior
point-estimate fallback. The cell remains `coverage_state=partial` because the
official page does not freeze that inference source or unavailable-state
policy, but its nonempty bounded QuickPLS scope is assigned Standard with
`evidence_state=release_qualified`. The difference is an explicit exclusion,
not an invitation to guess undocumented behavior.

The CLI and native catalogue resolve the exact Standard cell through
Capability Registry V2 without a Labs override. Unselected new runs omit the
payload, capability attribution, and result table. Historical archives
with the earlier unwrapped payload remain readable; typed option/payload/version
disagreement, missing opted payloads, and forged bootstrap authority fail
closed on execution, archive reopen, canonical reporting, and export.

The machine-readable QualificationSpec V2 contract is
`validation/qualification_v2/pls_posthoc_technical_minimum_sample_size_v2.qualification.json`.
It validates structurally and semantically in `compatibility_only` mode while
the coordinated release build still needs to mint immutable source/package
receipts. The scientific matrix itself is compact by design: this derived
kernel has no separate maximum-axis, compound-stress, or soak claim because
rows, indicators, resamples, cancellation, and long-run behavior belong to the
separately qualified linked PLS and bootstrap cells.

## Qualification covered by the source tests

- published examples from `0.10 -> 619` through `0.40 -> 39`;
- fixed assumptions and boundary at p = 0.05;
- exclusion of a smaller nonsignificant coefficient;
- sign and declaration-order invariance with stable ties;
- exact inference identity matching and incomplete-inference failures;
- no-path, no-significant-path, zero, near-zero, and range behavior;
- independent transparent Python oracle and independent base-R oracle over a
  frozen 24-scenario seeded matrix;
- exact one-worker/four-worker linked-bootstrap payload invariance;
- runner linkage to real bootstrap output;
- explicit point and bootstrap selection, unselected omission, and impossible-plan rejection;
- native result projection and corrective point-only presentation;
- CLI semantic export and tamper rejection; and
- project append, save, close/reopen equivalence, and tamper rejection.

The historical point-estimate-only `inverse_square_root_posthoc_v1` payload
remains readable and reproducible but is not used for new runner results.
