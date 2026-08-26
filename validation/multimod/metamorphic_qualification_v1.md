# MultiMod global metamorphic qualification orchestration

This gate uses the four Rust scientific producers and their development-scale
fixtures, but its evidence contract distinguishes completed results from
declared preparation-only payloads. The orchestration prevents the former
sequential `cargo run` matrix from becoming one unbounded all-or-nothing
invocation.

## Frozen execution graph

One bounded optimized release Cargo command builds these four examples together:

- `multimod_mga_qualification_v1`
- `multimod_heterogeneity_qualification_v2`
- `multimod_conditional_qualification_v1`
- `multimod_causal_qualification_v1`

The wrapper then launches the built executables directly for exactly 25
deterministic cells. The four family baselines are dependency roots and must
have successful identity-bound receipts before any transformed axis starts.
The dependent inventory is five shared axes per family plus causal
`sign_reverse`. No verifier tolerance, fixture scale, seed behavior or worker
mapping is reduced.

For POS destination-scored P2 and P23 common-metric profiles, each cell runs the
public compiler plus the locked point/common-metric preparation seam. The
payload is explicitly typed as a preparation, must retain a passed
common-metric gate, contains no bootstrap result, and is never counted as a
completed analysis. The global matrix compares that complete preparation under
every mapped axis. It does not repeat the 500-draw bootstrap in every axis.

Before the matrix starts, an exact-candidate dependency binds the dedicated
heterogeneity production report. That report must contain the full 500-draw P2
and P23 fixed-K bootstrap cells, independently reconstructed MICOM Step 2 and
passed common-metric gates, and label-aligned shared ledgers. Failure or stale
identity blocks this gate. Candidate promotion still requires the separate
`pos.common_metric` and `heterogeneity.bootstrap` receipts; a passing global
preparation comparison cannot promote either profile by itself.

At most four cell processes run concurrently. Each cell is limited to 1,800
seconds, all post-build scientific work is limited to 6,480 seconds, and the
wrapper is limited to 6,600 seconds with 120 seconds reserved for process-tree
termination and evidence preservation. A timeout fails the invocation; it does
not reduce draws, fixtures, targets or assertions.

## Checkpoint and resume contract

`multimod_metamorphic_cells_v1.py` freezes the ordered graph and atomically
seals each completed cell. Every reusable receipt binds:

- the exact clean Git commit and frozen plan digest;
- the producer executable path and SHA-256;
- the exact direct-executable argument vector and environment mapping;
- the output, stdout and stderr paths, sizes and SHA-256 values; and
- the successful family/axis producer identity and dependency receipts.

On retry, the stable campaign-level `-WorkRoot` is checked before execution.
Only fully valid receipts are reused. Missing, malformed, tampered or
identity-mismatched cells and their dependent axes are moved into attempt
history and recomputed. The wrapper always performs one current-commit build;
there is never a second build hidden in a cell command.

After all 25 receipts pass,
`verify_multimod_metamorphic_qualification_v1.py` produces the scoped scientific
report. It reports completed-result and preparation counts separately and
fails if a preparation is result-shaped, lacks a passed common-metric gate, or
claims a bootstrap was completed in this matrix. A final execution receipt
binds that report, the one-build receipt and all ordered cell receipts. Static
transport tests exercise plan identity, dependency invalidation, tamper
rejection, aggregate binding, PowerShell parsing, gate caps and stable
work-root arguments without invoking Cargo.
