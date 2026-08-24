# Interventional Causal Mediation V1

Status: separate observed-data method; not a PLS or SEM result relabelling.

Method version: `interventional_causal_mediation_v1`

Required result wording:

> Assumption-dependent interventional estimate; causality is not established
> by the calculation.

## Purpose and boundary

This module estimates a joint interventional indirect effect for an explicitly
ordered observed-variable mediator path. It is intentionally separate from
QuickPLS conditional-process analysis. A statistically significant PLS path or
interaction is never converted into a causal estimate.

V1 accepts directly observed numeric variables only:

- a binary treatment or a continuous treatment with explicit `x0 -> x1`;
- one to three ordered continuous mediators;
- a continuous outcome;
- an explicit nonempty set of observed adjustment covariates;
- observed baseline numeric or binary moderators;
- a recursive path of two to four edges.

V1 excludes latent/composite/HOC roles, groups, case/frequency/survey weights,
natural or cross-world effects, recanting-witness settings and exposure-induced
mediator-outcome confounding. These exclusions are hard blockers.

## Identification checklist

Calculation requires the user to review every item below. The checklist records
review, not proof:

- temporal ordering;
- consistency and well-defined interventions;
- treatment-outcome exchangeability given the adjustment set;
- treatment-mediator exchangeability given the adjustment set;
- mediator-outcome exchangeability given treatment and the adjustment set;
- absence of an exposure-induced mediator-outcome confounder;
- absence of a recanting witness for the selected path;
- adequacy of the declared linear equation specifications.

Any unreviewed item prevents execution. Output and exports retain the checklist
and never use wording such as “causality established,” “proven causal effect,”
or “natural indirect effect.”

## Equation contract

The user supplies equations in exactly this order:

1. first mediator;
2. each later mediator in path order;
3. final outcome.

Every equation includes an intercept automatically. A supplied term is the
row-wise product of one to three declared factors. Terms must be recursive:

- a mediator equation may use treatment, earlier mediators and baseline
  variables only;
- the outcome equation may use treatment, all selected mediators and baseline
  variables;
- each product term has all constituent main effects in the same equation;
- every declared adjustment covariate is a main effect in every equation;
- each selected path predecessor is a main effect in the next equation;
- a term contains at most one post-treatment mediator.

The final restriction permits treatment-mediator and baseline-moderator
interactions while keeping conditional expectations identifiable from linear
mean substitution. Products of multiple post-treatment mediators, nonlinear
outcomes and cyclic equations require a future separately qualified method.

Equations are fitted by a rank-checked QR least-squares solve. At least 20
complete observed cases are required, and every equation must have more rows
than parameters. Rank-deficient or nonfinite fits fail closed.

## Parametric g-computation estimand

For each observed baseline row, the method recursively computes mediator
conditional means under `x0` and `x1`. Declared baseline moderator intervention
values replace their observed values; unprobed moderators and adjustment
covariates are averaged over the empirical baseline distribution.

The method then evaluates:

```text
mu00 = E[Y(x0, G_x0)]
mu10 = E[Y(x1, G_x0)]
mu11 = E[Y(x1, G_x1)]

interventional direct contrast = mu10 - mu00
joint interventional indirect contrast = mu11 - mu10
total interventional contrast = mu11 - mu00
```

`G_x` denotes the fitted joint sequential mediator-distribution intervention
under treatment value `x`. The V1 linear expectation model integrates through
conditional means; it does not construct unit-level potential mediator values
or label the result as a natural effect. The additive decomposition residual is
persisted and must be numerically zero within qualification tolerance.

The target ID is deterministic and binds the method version, analysis identity,
role identities, treatment contrast, moderator intervention, equation
specifications and SHA-256 digest of the observed input columns.

## Positivity screen

Positivity checks are execution blockers, not cosmetic warnings.

For a binary treatment:

- every observed treatment value must equal `x0` or `x1`;
- each arm contains at least 10 observations by default;
- the global arm ratio is at most 10:1 by default;
- optional declared binary baseline strata must contain both arms, with a
  configurable minimum count in each arm.

For a continuous treatment:

- `x0` and `x1` are finite, distinct and within the observed range;
- each value has the configured minimum number of observations within a local
  neighborhood, defaulting to five observations within five percent of the
  observed treatment range.

The receipt calls this an observed-support screen. Passing it is necessary but
does not prove conditional positivity, exchangeability or correct model
specification.

## Required persisted result

The result retains:

- the mandatory cautious interpretation;
- method and deterministic target IDs;
- observed role identities and selected path order;
- `x0`, `x1` and any fixed baseline moderator values;
- fitted equation coefficients, ranks and residual scales;
- identification-checklist receipt;
- observed-support counts, ranges, checked strata and positivity wording;
- `mu00`, `mu10`, `mu11`, direct, joint indirect and total contrasts;
- additive decomposition residual;
- data, configuration, result and sidecar digests.

No UI, export or report may remove the interpretation or positivity wording.

## Qualification requirements

Before release qualification, evidence must include:

- hand-calculated one-, two- and three-mediator linear fixtures;
- independent parametric g-computation reproduction;
- binary and continuous treatment fixtures;
- treatment-mediator and baseline-moderator interaction fixtures;
- known-data-generating-process recovery with and without an indirect effect;
- incomplete-checklist, role-overlap, future-variable, hierarchy, rank and
  nonfinite blockers;
- binary global/stratified and continuous local-support failures;
- row-order and equation-term-order invariance after canonical compilation;
- deterministic target identity and input-data binding;
- save/reopen and tamper rejection;
- report tests proving the mandatory cautious language survives every export;
- installed, portable and fully offline Windows workflows.

The method remains unavailable outside its observed linear V1 envelope even if
an adjacent conditional-process or PLS profile is qualified.
