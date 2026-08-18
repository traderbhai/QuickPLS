# QuickPLS 3 parity ledger

`validation/quickpls_3_parity_ledger.json` is a frozen historical record for 17 established QuickPLS 3 capabilities. It is no longer the current evidence source of truth. Capability Registry V2 and the live method-manifest derivation govern current coverage, evidence, and customer surface. This ledger still validates its original comparison identity and evidence envelope so historical archives and reports remain interpretable, but none of its labels can promote a Registry V2 option cell.

A catalogue entry and a scientific capability are deliberately different things. One launcher may contain multiple independently promoted workflows. For example, the single Regression entry maps to separate OLS, binary logistic, regression-bootstrap, and PROCESS capability IDs. The parity validator enforces the frozen established-capability mapping and globally unique IDs, while the competitor catalogue and method factory govern the current 16-entry launcher and future capabilities. Sharing a launcher can never let evidence for one workflow promote another.

The state ladder is:

`absent -> engine_only -> archive_qualified -> native_qualified -> release_qualified`

The historical file still records one `absent`, one `native_qualified`, and fifteen `release_qualified` labels. Those are preserved archival metadata, not current claims. At the August 2026 pre-promotion reconciliation checkpoint, the then-current method-manifest set derived 35 absent, three archive-qualified, and two engine-only capabilities across 40 manifests. Those checkpoint counts are intentionally not a current product-status statement; current truth is re-derived from Registry V2 and the active manifests. Historical audit files remain on disk so old archives stay interpretable.

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
