# Indexed Jackknife Engine Specification v1

Status: deterministic execution foundation used by experimental `indexed_resampling_v3` BCa inference.

## Operation

For an analysis sample with `N` cases, the delete-one jackknife executes exactly `N` estimates. Outcome position `i` corresponds to the estimate obtained after omitting case `i`. The operation contains no random stream or seed.

## Reproducibility and Execution

- Omitted-case identity and result ordering are independent of scheduling and requested worker count.
- Worker count is execution provenance and is restricted to `1..64`.
- Progress is serialized, monotonic, and reports exactly `N` completed units.
- Estimator failures remain typed at their omitted-case position for the method-specific adapter to evaluate.
- Cancellation prevents a completed `JackknifeRun` from being returned; partial outcomes are not publication artifacts.
- At least three input cases are required by the generic foundation. Method-specific estimators may require more.

## PLS Adapter

- PLS jackknife uses the same fixed model-wide complete-case sample as the original estimate and requires at least four complete cases so every reduced sample still satisfies the PLS minimum.
- Each omitted-case dataset contains only model indicators and excludes exactly one position from the original analysis sample.
- Construct orientations are aligned to the corresponding original-score positions before parameters are extracted.
- Each successful transient outcome stores only its omitted-case index and canonical v2 parameter map. It does not persist full scores or estimator objects.

## Scope Boundary

This contract supplies deterministic leave-one-out orchestration and transient aligned PLS parameter estimates. `RESAMPLING_ENGINE_V3.md` consumes them to compute acceleration and BCa intervals under a frozen all-cases-success policy. Pseudo-values, standalone jackknife bias/variance reporting, and publication-ready simulation validation remain outside this contract.
