# QuickPLS v2.8.0 Release Handoff Consistency

## Scope

This milestone aligns the public handoff path for QuickPLS 2.x: README, installation guide, source-build guide, active milestone tracker, release notes, version metadata, and artifact naming.

## What It Checks

- Public docs identify the current development release as `v2.8.0`.
- User-facing download/install instructions explain setup, portable, and checksum files.
- Build instructions point to the current v2.8 gate and the versioned desktop artifact command.
- README screenshots exist under `docs/screenshots/v2/`.
- Release artifact preservation remains wired to the current milestone label.
- Docs avoid stale current-version markers, mojibake, and SmartPLS-equivalence claims.

## Evidence

- `validation/results/v280_release_handoff_audit.json`
- `docs/RELEASE_NOTES_V2_8_0.md`
- `docs/V2_ACTIVE_MILESTONE.md`

## Boundary

This is a documentation, release-handoff, and frontend governance milestone. It does not change statistical engines, method validation, result schemas, project archive behavior, or numerical fingerprints.
