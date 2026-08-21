# QuickPLS Version Support Policy

## Release Lines

QuickPLS uses semantic versions. A release line is the `major.minor` portion; a
patch release is intended to preserve documented project compatibility within
that line.

| Channel | Support policy |
| --- | --- |
| Stable | The current stable line receives correctness, security, installer, and critical documentation fixes. |
| Previous stable | Receives transition support for 90 days after the next stable line is published, limited to P0/P1 defects and migration assistance. |
| Beta | Supported only for the active named beta cohort and only until the next beta or stable replacement is published. |
| Unsigned preview | Public technical evaluation with best-effort issue triage; no signed-release SLA or publication-ready claim. |
| Internal | Maintainer-only; no user support commitment. |

Only artifacts published through an official QuickPLS release channel are covered.
Locally modified builds and unofficial redistribution are unsupported.

## Supported Platform Matrix

QuickPLS 3 targets Windows x64. Each stable release note must identify the exact
oldest supported Windows build and the current patched Windows build used for
clean-install acceptance. An operating-system claim is not active until those
exact builds pass installed and portable lifecycle tests.

Support requires a maintained Windows installation, sufficient local storage,
and the WebView2 condition documented for that release. R and Python are not
runtime dependencies. Offline use remains supported after installation.

## Compatibility And Project Safety

- A patch release must open projects produced by earlier patches in the same line.
- An N-1 upgrade must preserve supported projects, results, settings, and recovery state.
- A newer project must not be silently rewritten by an older release.
- When downgrade compatibility is uncertain, users must work on a copy and retain
  the original project and export files.
- Experimental or preview method payloads remain governed by their recorded method
  version and are not silently relabeled as stable evidence.

## End Of Support

The project announces end of support in release notes and the known-issues
register. After the 90-day transition window, an older line may receive a fix only
when the release manager determines that a severe security or data-integrity risk
justifies an exceptional patch.

Users on an unsupported line are directed to the latest supported full installer.
If a safe upgrade path is unavailable, the release manager must publish a bounded
recovery or export procedure before ending support.

## Current Transition

QuickPLS 2.51.x is the current published unified-workflow preview line while QuickPLS 3 remains under development.
The published `v2.53.0` artifacts are unsigned technical previews, not Beta or Stable, even though they are publicly downloadable from GitHub.
Unsigned QuickPLS 3 builds likewise remain preview artifacts and do not replace a signed stable line.
The first competitor-grade QuickPLS 3 stable line begins only after the signed
commercial-readiness gate passes.
