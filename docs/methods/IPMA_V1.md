# IPMA / cIPMA V1

`ipma_v1` is validated for the documented QuickPLS v0.9.0-rc.1 supported scope. Broader cIPMA extensions outside this contract remain unsupported.

## Scope

- Available through `AnalysisMethod::Ipma`.
- Uses the ordinary standardized PLS-PM estimator.
- Target constructs are read from recipe metadata `ipma_targets` or `ipma.targets` as a comma-separated construct-id list.
- If no target metadata is supplied, every endogenous construct is used as a target.
- Construct importance is the total effect from each construct to the target. The target construct receives self-importance `1.0`.
- Construct performance is the mean 0-100 min-max scaling of the construct's standardized score.
- Indicator performance is the mean 0-100 min-max scaling of the standardized indicator column, reported with its loading and parent construct importance.

## Current Limitations

- This is a bounded IPMA/cIPMA preview.
- It does not use scale metadata or explicit theoretical minimum/maximum values.
- Case-weighted IPMA, generated interaction constructs, and higher-order constructs are blocked in this preview.
- All output remains watermarked as experimental.

## Validation Evidence

`npm run qpls:ipma:reference` writes `validation/results/ipma_reference_report.json`.

The fixture uses a transparent single-indicator mediated path model. An independent Python reference standardizes the indicators, estimates structural OLS equations, decomposes total effects, computes 0-100 min-max performance, and compares the construct and indicator IPMA rows against QuickPLS within the `1e-6` deterministic gate.
