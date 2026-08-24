# Conditional-process and causal raw qualification V1

This slice binds scientific acceptance to the public Recipe V4 compiler and
the production raw-data runners. The Rust producers do not mark themselves
qualified. A standard-library Python verifier independently audits their
machine-readable reports and is the only artifact consumed by downstream
dependency gates.

## One producer run per family

`run_conditional_causal_raw_qualification_v1.ps1` runs exactly one producer
and one verifier for the selected family. It retains both files:

- `<output-stem>.producer.json` contains the raw compiler, result, frame, and
  full-refit-ledger summaries;
- the requested output contains the independent verification receipt and a
  SHA-256 binding to the producer report.

The conditional producer covers the admitted percentile, BCa, studentized,
three-way, multiple-HOC, grouped, case-weighted, and frequency-weighted
profiles. It includes explicit first-stage, second-stage, both-stage, multiple
interaction, two-edge, six-edge, derivative, joint-probe, and contrast
fixtures. BCa retains the complete delete-one vector. Studentized evidence
retains the observed inner vector plus each usable outer estimate and its
nested standard error. Frequency weighting retains every compact multinomial
count draw and an independent physical-row-expansion point reference.
Every case also retains the validated original-sample edge functions by
stratum. The Python verifier independently multiplies those edge functions
and applies the frozen probes to reproduce every specific indirect, total
indirect, total effect, affine index, derivative, finite contrast, and grouped
contrast target; it does not accept the runner's target vector as its oracle.
Successful resample vectors are retained with their replicate identities so
the verifier also reproduces Type-7 percentile, full delete-one BCa, and
nested-studentized interval endpoints for every target, not only a convenient
first column.

The causal producer covers binary and explicit continuous treatment contrasts
and explicitly selected paths with two, three, and four edges. It emits the
complete observed columns and authored equations used by the runner. The
verifier independently repeats the canonical equation ordering, modified
Gram-Schmidt OLS fits, and parametric g-computation before comparing every
direct, joint interventional indirect, and total contrast. Separate,
correctly specified two-, three-, and four-edge fixtures predeclare analytic
DGP truths and numerical recovery tolerances. Equation-orthogonal residuals
make those truths independently identifiable in the emitted finite samples.
Every nonzero strong-signal target must meet the tolerance and its interval
must exclude zero. This is a deterministic recovery guard, not a Monte Carlo
power claim. The causal Type-7 bootstrap endpoints are likewise recomputed for
all direct, indirect, and total targets from the retained shared ledger.

## Fail-closed scope

The conditional receipt requires the frozen blocker codes for group plus
weight, HOC plus group, three-way plus HOC, and studentized inference outside
its profile. It never simplifies those requests.

The causal receipt requires adjustment and identification declarations,
observed-support positivity failure, and estimator/compiler blockers for
natural or cross-world effects, recanting witnesses, exposure-induced
mediator-outcome confounding, latent/composite/HOC roles, groups, and weights.
Natural or cross-world effects are intentionally not representable by Recipe
V4; that boundary is reported explicitly and the lower estimator guard is
exercised without claiming a public recipe request exists.

All causal output must retain the wording “assumption-dependent
interventional estimate” and must not state that causality was established.

## Qualification scale

The qualification scale requests 5,000 conditional percentile/BCa draws,
1,000 outer by 200 inner studentized draws, and 5,000 causal bootstrap draws.
The development scale is a non-release diagnostic with smaller draw counts.
Neither producer nor verifier replaces failed draws. A release claim still
requires the complete frozen campaign, exact-commit manifests, and installed
and portable offline promotion smokes.
