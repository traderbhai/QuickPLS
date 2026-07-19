# Benchmarks

## v0.3 PLS Core Alpha

Date: 2026-07-18

Hardware and operating system:

- AMD Ryzen 5 7530U, 6 cores / 12 logical processors
- 15.3 GB RAM
- Windows 11 Pro 10.0.26200
- Rust release profile, QuickPLS 0.9.0-rc.1

Target-shape synthetic model:

- 100,000 rows
- 300 numeric indicators
- 100 reflective constructs with three indicators each
- 99 recursive structural paths
- Standardized preprocessing and path weighting
- Two iterations to convergence
- Estimator time: 2.761 seconds on the first qualified run; 2.840 seconds under memory monitoring
- Peak benchmark-process working set: 659.1 MB

Run with:

```powershell
cargo test -p qpls-estimation --release benchmark_target_shape_100k_300_100 -- --ignored --nocapture
```

The benchmark uses deterministic synthetic values and is an estimator throughput qualification, not a comparison with SmartPLS or another engine. The 10,000-resample benchmark is deferred to v0.4 because v0.3 contains no resampling engine.
