# Shared build disk guard

QuickPLS uses one Cargo target directory at `D:\QuickPLS\target`. Parallel or
alternate target directories are prohibited because duplicate Rust artifacts
can exhaust both the workspace and system drives.

Before a substantial Rust build, run:

```powershell
npm run qpls:workspace:disk-guard
```

The guard is read-only. Run it only at a genuinely substantial build boundary,
not as a frequent background poll. It fails before a build when:

- the Windows system drive has less than 15 GB free;
- the workspace drive has less than 25 GB free; or
- `CARGO_TARGET_DIR` points anywhere other than the shared workspace target.

It also reports the shared target size, active Cargo/Rust compiler processes,
and whether an explicitly configured incremental-build setting is unsafe. A
passing guard is not permission to run Cargo in parallel: one root-controlled
Cargo process remains the workspace policy.

Do not delete the shared target while a build is active. When a development
cycle is complete, `cargo clean` may be used deliberately to recover the target
space, but only after confirming that no Cargo or Rust compiler process is
running and that a rebuild cost is acceptable. Normal cleanup is triggered only
after the relevant free-space threshold is crossed; passing drives are left
alone to avoid needless cache churn and repeated rebuilds.
