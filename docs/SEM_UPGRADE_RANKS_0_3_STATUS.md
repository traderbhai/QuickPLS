# QuickPLS SEM Upgrade Program Status

This document tracks implementation and qualification of the existing QuickPLS
General SEM project mode. It does not describe a separate application.

## Program order

1. Rank 0: qualify and promote the existing General SEM mediation and
   simultaneous two-way moderation cells.
2. Rank 1: add and qualify the bounded higher-order construct matrix.
3. Rank 2: add and qualify bounded two-way moderated mediation.
4. Rank 3: bind the existing CB-SEM/CFA science to schema-6 authority and add a
   bounded recursive-SEM case bootstrap.

Later-rank groundwork may be prepared in isolated branches, but promotion and
integration remain ordered. Every new capability is additive; existing project,
recipe, method, capability, and result identities remain readable unchanged.

## Version 2.50 completion rule

The integrated Rank 0–3 release uses the streamlined coding-first profile
approved for Version 2.50:

- complete the full cross-rank source integration before running diagnostics;
- run one consolidated automated Rust, frontend, registry, typecheck, and build
  pass rather than many overlapping mini-gates;
- collect the complete failure list, correct it in one source pass, and run the
  final consolidated gate once;
- retain exact cell identities and fail-closed model, recipe, result, archive,
  cancellation, and unsupported-scope checks in production code;
- build one frozen Windows candidate and use that same candidate for the short
  installed/portable, calculate, cancel, save/reopen, Results, and export smoke;
  and
- promote each exact cell independently, so one failed cell cannot relabel or
  block a qualified sibling.

This profile does not repeat already accepted scientific fixtures or create a
new audit framework merely to restate production behavior. The release record
must still state exactly which automated checks and package smokes actually ran.

## Familiar graphical SEM workflow

QuickPLS keeps one desktop workflow for all four ranks: draw and edit the model
on Canvas, inspect the model or Parameter Table, review PLS-SEM and CB-SEM
eligibility cards, choose calculation settings, use the normal Calculate
command, monitor or cancel the run, and inspect verified output in the normal
Results workspace with the shared export action. Higher-order constructs and
moderated mediation use Save As Revision so the authored diagram remains the
scientific authority. This is intentionally familiar to SmartPLS users without
copying SmartPLS branding, layouts, or proprietary implementation details.

## Version 2.50 release checkpoint

Rank 0, Rank 1, Rank 2 moderated mediation, and both Rank 3 General SEM CB-SEM
cells are integrated on `main` as bounded Standard capabilities. Their
compilers, numerical runners, native job lifecycle, schema-6 persistence,
canonical Results workspace, shared export route, and Standard-access workflow
are connected without introducing a second application.

The consolidated automated diagnostic, one-pass correction, production build,
short Standard-access smoke, and compact streamlined evidence are complete. The
Version 2.50 setup, portable, CLI, checksum, artifact inventory, and evidence
files are published as the unsigned [`v2.50.0` GitHub pre-release](https://github.com/traderbhai/QuickPLS/releases/tag/v2.50.0).

The later automated whole-product screenshot and observation sweep remains a
separate UI/UX improvement pass. Code signing and signed beta/stable release
hardening also remain separate from analytical-cell Standard status.

## Storage policy

Cargo and packaging output stay on drive D. Broad build work soft-stops when
either C or D falls below 25 GB; the hard floor is 20 GB on both drives.
Only explicitly resolved reproducible caches or superseded untracked build
artifacts may be removed. Accepted installers, receipts, reports, archives, and
source hashes are retained.
