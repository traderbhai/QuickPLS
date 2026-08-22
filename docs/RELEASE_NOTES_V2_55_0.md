# QuickPLS 2.55.0 Release Notes

Status: source gate complete; unsigned candidate and packaged release evidence pending.

QuickPLS 2.55 focuses on professional Calculate setup, model-aware diagram layout, and a fail-closed current-method evidence architecture and collection workflow. It preserves the existing 18 public Calculate methods and does not relabel or replace historical scientific identities.

## What changed

### Model-aware diagram presentation

- Automatic layout reserves construct and indicator envelopes instead of treating every construct as the same box.
- Structural, measurement, and moderation presentation routes avoid eligible obstacles and preserve explicit pinned overrides.
- Selected polyline paths expose bend handles; Reset route and Reset label return a path to automatic presentation.
- Moderation anchors are draggable within the stored safe focal-path range and remain presentation-only.
- PROCESS setup uses the shared diagram renderer in an isolated read-only viewport with Fit and zoom.

These changes do not alter model topology, estimands, analytical methods, or scientific hashes.

### Professional Calculate setup

- Shared dialogs retain a stable **Start calculation** action and remain usable at the supported compact viewport.
- Settings wrap, scroll, and omit empty groups instead of truncating primary controls.
- Scientific eligibility is prioritized before runtime availability and optional advice.
- Logistic regression identifies invalid binary coding directly; MGA shows group value, effective N, and excluded rows.
- CB-SEM, PLSpredict/CVPAT, IPMA, NCA, GSCA, and other method surfaces disclose only implemented controls and fixed execution policies.
- NCA provenance is normalized to its actual fixed single-worker execution.

### Current evidence architecture

- The public method catalogue remains exactly 18 entries.
- The setup matrix declares 64 method-option cases plus one direct Calculate capture for each method.
- Cross-method and specialized contracts cover imports, exports, persistence, mediation, moderation, higher-order constructs, CB-SEM, regression, PROCESS, and archive lifecycle behavior.
- Candidate build, install, and packaged automation are provenance-bound and restricted to exact harness-owned process trees.

## Source verification

The formal first consolidated pass recorded the complete finite correction batch. The same gate script was then rerun without modification. The final report passed all 14 steps:

1. `diff_check` — clean patch contract;
2. `v255_evidence_contract` — source evidence declarations;
3. `v255_rebased_contract` — rebased interaction contract;
4. `frontend_full_vitest` — full Vitest;
5. `rust_authority` — Registry authority;
6. `rust_archive_schema6_authoring` — schema-6 archive authoring lifecycle;
7. `rust_archive_three_way` — exact three-way archive lifecycle;
8. `rust_desktop_three_way` — desktop three-way lifecycle;
9. `frontend_typecheck` — full TypeScript check;
10. `frontend_build` — production frontend build;
11. `python_export_semantic_readback` — six-format semantic export readback;
12. `rebaselined_interactions` — all 17 rebaselined interaction assertions;
13. `method_setup_crawler` — the exact 18-method setup crawler; and
14. `v255_final_evidence_contract` — final evidence contract.

The qualifying final report is stored under `validation/results/v255_consolidated_diagnostics_20260822T020700Z/` and is paired with the committed formal first-pass report under `validation/results/v255_consolidated_diagnostics_20260822T015826Z/`.

## What is not yet claimed

- No 2.55 unsigned installer, portable executable, CLI package, or checksum set is claimed yet.
- Installed and portable packaged journeys have not yet completed.
- Actual Windows 200% scaling evidence has not yet completed.
- The GitHub 2.55 release has not been published.
- Code signing is excluded.

Until those gates pass, use the verified unsigned assets from the [QuickPLS 2.54.0 release page](https://github.com/traderbhai/QuickPLS/releases/tag/v2.54.0).
