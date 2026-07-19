# Bounded PLS-POS Segmentation v1

`pls_pos_bounded_v1` is the first QuickPLS v0.6 segmentation-discovery slice. It is available only inside `AnalysisMethod::Predict` recipes that set metadata `pls_pos_segments` to `"2"`.

## Scope

- Supports exactly two discovered segments.
- Supports one or more structural paths in a recursive model.
- Uses complete-case construct scores from the current PLS run.
- Does not use group labels, oracle segment columns, or external clustering input.
- Produces an experimental `segmentation` payload with algorithm name, objective values, objective improvement, minimum segment share, segment-size imbalance, maximum path separation, segment path coefficients, segment R2, assignment rule, deterministic observation memberships, and warnings.

## Algorithm

For the structural paths in the model:

1. Compute the mean path-alignment score `source_score * target_score` across structural paths for every complete observation.
2. Sort observations by alignment score with original row order as the deterministic tie-breaker.
3. Scan every two-way split that leaves at least `max(floor(n / 5), 12)` observations in each segment.
4. For each candidate segment, fit centered regressions for every endogenous construct using its direct predecessor scores.
5. Select the split with the lowest summed within-segment structural squared residual objective.
6. Report the objective improvement against the pooled centered structural-regression objective.

## Diagnostics

The preview reports three bounded class diagnostics:

- `min_segment_share`: the smaller segment size divided by total complete observations.
- `segment_size_imbalance`: the absolute segment-size difference divided by total complete observations.
- `max_path_separation`: the largest absolute difference between matching segment-specific path coefficients.

These diagnostics are descriptive screening values only. They are not information criteria, likelihood tests, class probabilities, or validated publication inference.

## Validation Fixture

`npm run qpls:segmentation:oracle-recovery` generates known two-segment single-path and multi-path datasets, verifies oracle split recoverability, then runs QuickPLS on the pooled data with `pls_pos_segments = "2"` and no oracle labels. It also generates a homogeneous multi-path null fixture to verify that a forced two-segment split has weak objective gain and weak path separation when no segment structure was generated.

The same report includes a bounded empirical fit screen with 12 homogeneous null replicates. The current screen records upper empirical p-values for objective improvement, path separation, and their joint criterion.

Current fixture evidence is written to `validation/results/segmentation_recovery_simulation_report.json`.

## Limitations

This is not full FIMIX-PLS or full PLS-POS. Unsupported in this slice: more than two segments, multiple starts, latent-class likelihood, information criteria, class probabilities, MGA/MICOM workspaces, and publication-ready inference.
