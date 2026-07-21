# NCA_V1

Status: validated for the documented QuickPLS v1.2.1 numeric X/Y CE-FDH and CR-FDH scope.

`nca_v1` provides numeric necessary condition analysis for selected X/Y pairs.

## Contract

- Required metadata:
  - `nca_x`
  - `nca_y`
- Optional metadata:
  - `nca_ceiling = ce_fdh|cr_fdh|both`
  - `nca_permutation_samples`
- Output includes CE-FDH and/or CR-FDH ceiling effect sizes, deterministic permutation p values, bottleneck rows, observations, usable permutations, warnings, and `method_version = nca_v1`.
- Constant or nonnumeric variables are rejected.

## Unsupported In v0.8

- Multiple-predictor NCA workflows.
- Categorical or ordinal NCA.
- Publication-stable ceiling smoothing claims.
- Full NCA package parity.

## Validation

`npm run qpls:nca:reference` compares CE-FDH/CR-FDH effect size and bottleneck monotonicity against an independent Python fixture. Broader NCA variants, nonnumeric variables, categorical handling, and unsupported ceiling methods remain outside the promoted scope.
