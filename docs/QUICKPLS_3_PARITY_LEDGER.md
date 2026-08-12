# QuickPLS 3 parity ledger

`validation/quickpls_3_parity_ledger.json` is the machine-readable source of truth for the QuickPLS 3 feature-equivalence program. It freezes the comparison reference to the official SmartPLS algorithms and techniques catalogue dated 12 August 2026 and records a stable QuickPLS feature ID, current method version, bounded scope, known differences, evidence rules, and promotion state for each of the 14 accepted native calculation catalogue entries.

The state ladder is:

`absent -> engine_only -> archive_qualified -> native_qualified -> release_qualified`

The checked-in Wave 0 baseline derives 13 entries as `native_qualified` from current method-audit plus packaged result, export, and save/reopen evidence. Structural Path Randomization remains `engine_only`: its engine audit and responsive native setup exist, but the current packaged report does not contain a completed method-specific result, export, and same-run reopen. No entry is `release_qualified` until a fresh Wave 0 or later wave gate produces current scoped reports.

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
