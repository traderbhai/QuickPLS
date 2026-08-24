# MultiMod scientific SUT slice V1

This is an executable development and qualification-support slice. It does not
qualify a capability, promote a manifest, or replace the complete V2.56
campaign. The Rust probe calls public production kernels; the Python process is
an independent standard-library reference and never imports QuickPLS code.

## Risk-based test architecture

| Layer | Purpose | Tests in this slice | Release role |
|---|---|---|---|
| Mathematical identity | Detect equation, sign, scaling, and criteria drift cheaply | independent OLS, polynomial algebra, log-sum-exp mixture likelihood, posterior normalization, information criteria, entropy, g-computation, expanded-row weighted fit | fast required gate |
| Kernel integration | Exercise the compiled QuickPLS implementations rather than reference-only code | multigroup eligibility/permutation/multiplicity, FIMIX EM, POS search, conditional-process math and frequency interaction estimator, causal observed-data estimator | required but not sufficient |
| Metamorphic | Detect identity and ledger errors without assuming one numerical implementation | MGA label reversal, POS same-seed start plan, likelihood/objective monotonicity, frequency expansion | required invariance gate |
| Campaign simulations | Establish recovery, null behavior, boundaries, and profile breadth | not completed here; command bindings list the missing cells | required before qualification |
| Installed application | Establish persistence, export, accessibility, cancellation, and offline behavior | outside this scientific slice | required before release |

## Coverage targets and acceptance

- Every emitted check has a stable `MMQ.*` failure code, fixed seed 42, a
  canonical input digest, and deterministic JSON output.
- Independent numeric identities use `1e-9` by default, tightened to `1e-12`
  for simple algebra and relaxed only where iterative likelihood arithmetic
  warrants `1e-8`.
- Development FIMIX uses 10 starts and at most 1,000 EM iterations.
  Qualification-support mode uses the frozen 30 starts and 5,000-iteration
  ceiling. One deterministic ARI pilot is explicitly not the required
  multi-seed median-ARI qualification.
- MGA always exercises the admitted 5,000-draw minimum. Failed draws are not
  replaced, and the fixture expects all 5,000 usable draws.
- POS checks the exact ten-start contract and recomputes every final segment's
  OLS coefficients and R-squared value independently. It does not represent
  the still-required production PLS measurement-score oracle.
- Conditional-process coverage includes first, second, both-stage, three-way,
  and six-edge effects, plus an independent count-space versus expanded-row
  joint interaction fit. Resampling profiles remain separate campaign gates.
- Causal coverage verifies the exact 1 direct, 8 indirect, and 9 total known
  contrast and stable assumption/positivity blockers.

## Commands

Development-scale examples:

```powershell
pwsh -NoProfile -File validation/multimod/run_scientific_sut_slice_v1.ps1 -Gate mga -Scale development
pwsh -NoProfile -File validation/multimod/run_scientific_sut_slice_v1.ps1 -Gate fimix -Scale development
pwsh -NoProfile -File validation/multimod/run_scientific_sut_slice_v1.ps1 -Gate pos -Scale development
pwsh -NoProfile -File validation/multimod/run_scientific_sut_slice_v1.ps1 -Gate conditional -Scale development
pwsh -NoProfile -File validation/multimod/run_scientific_sut_slice_v1.ps1 -Gate causal -Scale development
```

For a frozen candidate, replace `development` with `qualification`. Run one
gate at a time so the repository's one-Cargo-process rule remains enforceable.
The machine-readable candidate bindings and their explicit coverage gaps are
in `scientific_slice_command_bindings_v1.json`. Static review classifies the
five commands as partial diagnostic programs, not complete producer-gate
coverage. Their unexecuted commands are retained as `diagnostic_steps` on the
pending producer bindings. The MGA label-reversal and conditional-probe gates
may run the corresponding slice once because those narrower acceptance
contracts are fully named and independently checked; no broader profile,
recovery, resampling, or capability qualification follows from that use.

## Representative failure cases

- `MMQ.MGA.LABEL_REVERSAL.INVARIANCE`: signed differences did not negate, tails
  did not swap, or the deterministic partition identity changed.
- `MMQ.FIMIX.LIKELIHOOD.IDENTITY`: the optimized stored likelihood differs
  from a direct evaluation of the fitted mixture.
- `MMQ.POS.FULL_REFIT.IDENTITY`: the stored segment fit/objective differs from
  independent OLS over the final assignments.
- `MMQ.CONDITIONAL.FREQUENCY.INDEPENDENT_FIT`: count-space beta/gamma differs
  from physical expansion and independent joint OLS.
- `MMQ.CAUSAL.FAILURE.POSITIVITY`: an unsupported observed-support condition
  did not fail closed with the stable blocker.

The comparator always emits a result envelope, including on reference or input
failure. Exit code 0 means this bounded slice passed; 1 means one or more
scientific checks failed; 2 means the probe/comparator contract itself failed.
