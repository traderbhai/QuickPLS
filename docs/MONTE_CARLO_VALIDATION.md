# QuickPLS v0.4 Monte Carlo Validation Harness

Status: executable validation infrastructure. Quick, pilot, sensitivity, and studentized pilot runs are not qualification evidence. A larger preregistered run is qualification evidence only when a committed result explicitly says `qualification.evaluated: true` and `qualification.passed: true`.

## Purpose

`validation/monte_carlo` exercises the production `qpls-estimation` and `qpls-resampling` crates. It evaluates percentile, BCa, and optionally studentized interval coverage under a nonzero path and interval/test rejection behavior under a zero path. It does not reproduce bootstrap, BCa, or studentized formulas independently.

The data-generating process is deliberately identifiable without a measurement-error qualification problem:

- `x ~ N(0,1)` and independent `e ~ N(0,1)`.
- `y = beta*x + sqrt(1-beta^2)*e`.
- Single-item reflective constructs `x={x1}` and `y={y1}` with path `x -> y`.
- The population standardized PLS path is exactly `beta`.

Two scenarios are run: `beta=0.35` for interval coverage and bias, and `beta=0` for percentile/BCa exclusion of zero and the currently reported normal-reference p-value.

## Commands

From the repository root:

```powershell
cargo run -p qpls-cli -- qualify v04-inference --refresh-quick-monte-carlo
cargo run --release --manifest-path validation/monte_carlo/Cargo.toml -- --self-check
cargo run --release --manifest-path validation/monte_carlo/Cargo.toml -- --mode quick --output validation/results/monte_carlo_quick.json
cargo run --release --manifest-path validation/monte_carlo/Cargo.toml -- --mode pilot --output validation/results/monte_carlo_pilot.json
cargo run --release --manifest-path validation/monte_carlo/Cargo.toml -- --mode sensitivity --output validation/results/monte_carlo_sensitivity.json
cargo run --release --manifest-path validation/monte_carlo/Cargo.toml -- --mode studentized --output validation/results/monte_carlo_studentized.json
cargo run --release --manifest-path validation/monte_carlo/Cargo.toml -- --mode studentized-sensitivity --output validation/results/monte_carlo_studentized_sensitivity.json
cargo run --release --manifest-path validation/monte_carlo/Cargo.toml -- --mode studentized-qualification --output validation/results/monte_carlo_studentized_qualification.json
python validation/plan_studentized_qualification_shards.py
python validation/aggregate_studentized_qualification.py
```

The `qpls qualify v04-inference` command is the top-level accelerator gate. It runs the CLI worker matrix, runs bounded ordinary-bootstrap and nested studentized cancellation-latency smoke benchmarks, refreshes or validates the quick Monte Carlo report, validates the pilot, sensitivity, bounded studentized, bounded studentized-sensitivity, and full Monte Carlo reports when present, validates bounded studentized execution and worker-matrix artifacts, and writes `validation/results/v04_inference_qualification_quick.json`.

The quick mode uses 8 simulations per scenario, `n=60`, and 79 bootstrap replicates. It checks determinism, integration, schema production, and obvious failures only. Sampling uncertainty is far too large for accuracy claims.

The pilot mode uses 32 simulations per scenario, `n=100`, and 199 bootstrap replicates. It is a faster early-warning screen for coverage/type-I regressions before the expensive preregistered run. It records `qualification.evaluated: false` and is not release qualification evidence.

The sensitivity mode uses 96 simulations per scenario, `n=120`, and 399 bootstrap replicates. It now includes normal and standardized t(3) error scenarios for both `beta=0.35` and `beta=0`. It is a stronger drift screen than pilot mode but still records `qualification.evaluated: false`.

The bounded studentized mode uses 4 simulations per scenario, `n=100`, 999 outer bootstrap replicates, and 99 studentized inner replicates. It validates real minimum-plan bootstrap-t interval availability and early coverage/type-I plumbing, but the simulation count is far too small for accuracy claims.

The bounded studentized-sensitivity mode uses the same 999x99 studentized plan with 4 simulations per scenario and adds standardized t(3) heavy-tailed error scenarios for both `beta=0.35` and `beta=0`. It is scenario-plumbing evidence only, not promotion coverage evidence.

The explicit studentized qualification mode is:

```powershell
cargo run --release --manifest-path validation/monte_carlo/Cargo.toml -- --mode studentized-qualification --output validation/results/monte_carlo_studentized_qualification.json
```

It uses 1,000 simulations per scenario, `n=100`, 999 outer bootstrap replicates, and 99 studentized inner replicates. It includes normal and standardized t(3) error scenarios for both `beta=0.35` and `beta=0`. This is intentionally very expensive because each scenario requests 99,900,000 nested inner fits before failed primary replicates are considered.

For documented hardware runs, the same deterministic seed stream can be sharded. Each shard must specify the scenario name, simulation count, simulation offset, and its own output path:

```powershell
python validation/plan_studentized_qualification_shards.py --total 1000 --chunk 100 --output validation/results/studentized_qualification_shards/manifest.json
cargo run --release --manifest-path validation/monte_carlo/Cargo.toml -- --mode studentized-qualification --scenario coverage_beta_0_35 --simulations 100 --simulation-offset 0 --output validation/results/studentized_qualification_shards/coverage_beta_0_35_0000_0099.json
cargo run --release --manifest-path validation/monte_carlo/Cargo.toml -- --mode studentized-qualification --scenario coverage_beta_0_35 --simulations 100 --simulation-offset 100 --output validation/results/studentized_qualification_shards/coverage_beta_0_35_0100_0199.json
python validation/aggregate_studentized_qualification.py --input validation/results/studentized_qualification_shards --output validation/results/monte_carlo_studentized_qualification.json
```

The planner writes a machine-readable manifest with every shard command and the final aggregate command. The aggregator recomputes counts, rates, bias, and qualification checks from shard summaries, skips non-shard JSON files, and rejects overlapping scenario/offset ranges so smoke shards cannot be mixed with real qualification shards. By default it exits nonzero until every preregistered scenario reaches at least 1,000 completed simulations with zero failures. Use `--allow-incomplete` only for smoke-testing the aggregation path; incomplete aggregates are not accepted by `qpls qualify v04-inference`.

The explicit qualification mode is:

```powershell
cargo run --release --manifest-path validation/monte_carlo/Cargo.toml -- --mode qualification --output validation/results/monte_carlo_qualification.json
```

It uses 1,000 simulations per scenario, `n=100`, and 999 bootstrap replicates. This is intentionally expensive because every bootstrap run also performs delete-one jackknife estimation for BCa acceleration.

## Preregistered Qualification Checks

Qualification is evaluated only when both scenarios complete at least 1,000 simulations. The machine-readable report then requires:

- Percentile coverage and BCa coverage each in `[0.925, 0.975]` for nominal 95% intervals.
- Percentile, BCa, and normal-reference type-I rates each in `[0.025, 0.075]` under `beta=0`.
- Absolute mean path bias no greater than `0.03` under `beta=0.35`.
- BCa availability equals `1.0` in both the alternative and null scenarios.
- Normal-reference probability availability equals `1.0` in the null scenario.
- Mean usable-bootstrap rate is at least `0.99` in both scenarios.

These thresholds qualify only this normal, single-path, single-item DGP. Promotion still requires non-normal data, multi-indicator measurement error, smaller samples, multiple predictors, tail probabilities, worker-count invariance, and performance/cancellation evidence.

For `studentized-qualification`, the same ordinary percentile, BCa, normal-reference, bias, and availability checks are evaluated, and the report also requires studentized coverage/type-I/availability checks for both normal and standardized t(3) scenarios. The top-level qualifier accepts this artifact only when all required studentized checks pass and every normal/heavy-tail scenario completes at least 1,000 simulations with zero failures.

## Reproducibility and Output

The harness uses fixed indexed seeds derived from master seed `20260718041` and ChaCha20. The JSON report records the DGP, engine versions, configuration, completed/failed simulation counts, interval availability, coverage, exclusion rates, normal-reference type-I rate, bias, usable-replicate rate, thresholds, individual qualification checks, and elapsed time.

Quick, pilot, sensitivity, bounded studentized, and bounded studentized-sensitivity reports deliberately contain `qualification.evaluated: false` and `qualification.passed: null`. They must not be cited as evidence that the v0.4 inference gate passed.
