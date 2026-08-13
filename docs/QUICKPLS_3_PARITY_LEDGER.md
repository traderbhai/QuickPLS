# QuickPLS 3 parity ledger

`validation/quickpls_3_parity_ledger.json` is the machine-readable source of truth for the established QuickPLS 3 feature-equivalence capabilities. It freezes the comparison reference to the official SmartPLS algorithms and techniques catalogue dated 12 August 2026 and records a stable QuickPLS feature ID, current method version, bounded scope, known differences, evidence rules, and promotion state. The live native calculation catalogue currently contains 15 entries; newer capabilities such as CTA-PLS can be governed by the strict method factory before they are incorporated into this established 17-capability ledger.

A catalogue entry and a scientific capability are deliberately different things. One launcher may contain multiple independently promoted workflows. For example, the single Regression entry maps to separate OLS, binary logistic, regression-bootstrap, and PROCESS capability IDs. The parity validator enforces the frozen established-capability mapping and globally unique IDs, while the competitor catalogue and method factory govern the current 15-entry launcher and future capabilities. Sharing a launcher can never let evidence for one workflow promote another.

The state ladder is:

`absent -> engine_only -> archive_qualified -> native_qualified -> release_qualified`

The current ledger derives 14 capabilities as `native_qualified` and three capabilities as `release_qualified`: Structural Path Randomization v1, Binary Logistic Regression v2, and Regression Bootstrapping v1. Graph-defined PROCESS v2 remains native-qualified: its scientific, archive, frontend, export, packaged workflow, and cleanup evidence pass, but the current repeated-completion resource report did not prove terminal process-role stability, so release qualification is withheld. Structural Path Randomization qualification is bounded to fixed original PLS construct scores, exchangeable reduced-model residuals, unadjusted pathwise plus-one probabilities, and the documented paired homoscedastic Gaussian constant-variance calibration scenarios. It does not qualify measurement-model re-estimation, heteroskedastic or broader non-Gaussian validity, MGA, MICOM, multiplicity-adjusted inference, or causal proof. Newly introduced capabilities begin at `absent` or the highest state their current evidence actually derives.

## Enforcement

Run the lightweight validator from the repository root:

```powershell
python validation\parity_ledger.py
python validation\test_parity_ledger.py
```

Use `python validation\parity_ledger.py --json` when a promotion or release script needs the full evaluated evidence report.

The validator does not trust a declared state. It reads every referenced source or JSON report and derives the highest supported state. Missing files, failed reports, identity mismatches, duplicate feature IDs, an incomplete 14-workflow inventory, or a declared state above the derived state fail validation.

`release_qualified` has an additional non-negotiable contract. Each feature must reference both:

- a current scoped method audit; and
- a packaged acceptance report.

Both reports must contain `passed: true` and exact values for the feature ID, current method version, and frozen catalogue snapshot date. A generic release pass, prose claim, stale report, or method token found elsewhere in a large file cannot promote a feature.

## Updating a feature

1. Keep the stable feature ID unchanged.
2. Update scope, method version, and known differences when the executable contract changes.
3. Point evidence rules at current generated reports; never insert a manual pass flag as evidence.
4. Advance only one or more ladder steps actually derived by `parity_ledger.py`.
5. Add release evidence only after the method-specific and packaged gates emit the required identities.

This ledger tracks qualification evidence; it is not itself a scientific validation result and it does not turn the existing bounded workflows into broader SmartPLS parity claims.
