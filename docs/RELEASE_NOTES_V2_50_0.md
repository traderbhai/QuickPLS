# QuickPLS 2.50.0 — SEM Upgrade

QuickPLS 2.50.0 integrates the Rank 0–3 SEM upgrade into the existing offline Windows application. It does not create a second project type or separate analysis program.

## Download

Download the public unsigned pre-release from the [QuickPLS 2.50.0 GitHub Release](https://github.com/traderbhai/QuickPLS/releases/tag/v2.50.0):

- **Setup** — recommended for a normal Windows installation.
- **Portable** — runs without installation when a compatible WebView2 runtime is present.
- **CLI** — command-line and batch recipe execution.
- **Checksums** — SHA-256 values for all three executables.

The release also includes the artifact inventory and compact streamlined integration evidence. The binaries are unsigned, so Windows SmartScreen may identify the publisher as unknown.

## General SEM workflow

Version 2.50 keeps one graphical workflow:

1. Import and inspect data.
2. Draw the model on Canvas and edit the active Parameter Table where applicable.
3. Select `Calculate` and review the exact PLS-SEM and CB-SEM eligibility cards.
4. Choose a supported estimator and settings.
5. Monitor or cancel the native calculation.
6. Review the verified canonical result in Results.
7. Export or save, close, and reopen the same schema-6 project.

Higher-order constructs and moderated mediation use versioned **Save As Revision** authoring so the original scientific authority is not silently rewritten.

## Rank 0 — General SEM mediation and moderation

- PLS mediation point estimation.
- Multiple mediation full-model case bootstrap.
- Multiple simultaneous two-way moderation point estimation.
- Multiple simultaneous two-way moderation full-model case bootstrap using scientific gamma.

## Rank 1 — Higher-order PLS constructs

- One non-nested second-order HOC per model.
- Exact supported matrix across repeated, extended-repeated, embedded two-stage, and disjoint two-stage approaches.
- Reflective relationships report loadings; formative relationships report weights and collinearity.
- Point estimation and full-model case bootstrap rerun the required stages from raw resampled cases.
- Hybrid, multiple/nested HOCs, HOC interactions, groups, weights, feedback, PLSc, and matrix input remain outside this cell.

## Rank 2 — Two-way moderated mediation

- One selected `X → M → Y` path.
- One first-stage `X × W → M` or second-stage `M × W → Y` interaction.
- Fixed standardized moderator probes at `−1`, `0`, and `+1`.
- One shared bootstrap ledger for scientific gamma, three conditional indirect effects, and the index of moderated mediation.
- Multiple interactions, both stages moderated, arbitrary probes, longer paths, HOCs, groups, weights, and causal claims remain excluded.

## Rank 3 — General SEM CB-SEM

- Bounded raw continuous, listwise, single-group recursive common-factor SEM.
- Ordinary normal-theory ML with typed parameter, fit, and identification output.
- Recursive-SEM full-refit percentile case bootstrap for eligible free parameters.
- Mean structures, feedback, robust/ordinal/FIML estimators, clustering, invariance, MGA, comparison, CB moderation, and derived indirect-effect inference remain excluded.

The established CFA/CB-SEM cells retain their original identities and archive behavior.

## Results, persistence, and export

- One `CanonicalResultDocumentV2` remains the source for tables, charts, exports, and strict reopen validation.
- Canonical General SEM results support CSV, XLSX, self-contained HTML, PDF, SVG, and PNG.
- Schema-6 append/readback binds the model, recipe, dataset, plan, method, capability cell, result, tables, receipts, and digests.
- Cancellation publishes no partial analytical result or partial export.

## Release status and limits

The promoted cells are scoped Standard under the user-approved streamlined Version 2.50 integration profile. Scoped Standard means the exact documented predicate is available without the Labs toggle; it does not mean unrestricted SmartPLS parity or numerical identity.

This GitHub publication is a **pre-release** because its Windows binaries are unsigned. Code signing, signed updater channels, the broader clean-install/upgrade matrix, and final SmartPLS-class product parity remain future release-hardening work.

See also:

- [Installation](INSTALLATION.md)
- [Quick Start](QUICK_START.md)
- [User Guide](USER_GUIDE.md)
- [Method Compatibility](METHOD_COMPATIBILITY.md)
- [Known Issues](KNOWN_ISSUES.md)
- [Rank 0–3 Program Status](SEM_UPGRADE_RANKS_0_3_STATUS.md)
