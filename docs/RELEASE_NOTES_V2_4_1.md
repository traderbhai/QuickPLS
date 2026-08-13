# QuickPLS v2.4.1 Release Notes

QuickPLS v2.4.1 is a release-readiness audit milestone for the QuickPLS 2.x frontend line.

## Changed

- Added a current v2.4.1 readiness gate that verifies version metadata, public documentation, screenshot coverage, registry state, artifact labeling, and frontend-only boundaries.
- Updated README, installation, and build-from-source guidance to point at the current v2.4.1 verification command.
- Added release-readiness smoke and static audit scripts.
- Updated the visible app milestone label to `v2.4.1 release readiness`.

## Unchanged

- No statistical engines changed.
- No formulas, estimator behavior, result payloads, project format, validation tolerances, or numerical fingerprints changed.
- QuickPLS remains offline, proprietary, and Windows desktop focused.

## Verify

```powershell
npm run qpls:v241:release-readiness
```
