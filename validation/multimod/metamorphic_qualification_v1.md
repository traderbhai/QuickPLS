# MultiMod global metamorphic qualification orchestration

This gate preserves the existing four Rust scientific producers, their
development-scale fixtures and the existing Python scientific verifier. The
change is orchestration only: it prevents the former sequential `cargo run`
matrix from becoming one unbounded all-or-nothing invocation.

## Frozen execution graph

One bounded Cargo command builds these four examples together:

- `multimod_mga_qualification_v1`
- `multimod_heterogeneity_qualification_v2`
- `multimod_conditional_qualification_v1`
- `multimod_causal_qualification_v1`

The wrapper then launches the built executables directly for exactly 25
deterministic cells. The four family baselines are dependency roots and must
have successful identity-bound receipts before any transformed axis starts.
The dependent inventory is five shared axes per family plus causal
`sign_reverse`. No verifier tolerance, fixture scale, seed behavior, worker
mapping or metamorphic scientific assertion is reduced.

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

After all 25 receipts pass, the unchanged
`verify_multimod_metamorphic_qualification_v1.py` produces the scientific
report. A final execution receipt binds that report, the one-build receipt and
all ordered cell receipts. Static transport tests exercise plan identity,
dependency invalidation, tamper rejection, aggregate binding, PowerShell
parsing, gate caps and stable work-root arguments without invoking Cargo.
