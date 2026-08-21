# Security Policy

## Supported Versions

| Version | Supported |
| --- | --- |
| 2.50.x | Yes |
| 2.46.x | Transition support while 2.50.x is the current stable line |
| Earlier releases and preview builds | No public security support |

The lifecycle and transition rules are defined in
[`docs/VERSION_SUPPORT_POLICY.md`](docs/VERSION_SUPPORT_POLICY.md). A QuickPLS 3
preview does not become a supported stable release until its signed commercial
gate passes.

## Reporting A Vulnerability

Please report suspected vulnerabilities through a private
[GitHub security advisory](https://github.com/traderbhai/QuickPLS/security/advisories/new).
If private reporting is unavailable, do not open a public issue containing exploit
details or sensitive evidence; contact the project owner without attaching that
material until a private route is confirmed.

Do not publish exploit details before the issue has been reviewed.

Severity, acknowledgement targets, escalation, and coordinated disclosure are
defined in [`docs/SUPPORT_POLICY.md`](docs/SUPPORT_POLICY.md). The product threat
model and required release evidence are documented in
[`docs/SECURITY_MODEL.md`](docs/SECURITY_MODEL.md). These policies do not replace
the independent security and legal reviews required for QuickPLS 3.

## Runtime Security Model

QuickPLS is designed as an offline Windows desktop application:

- No account.
- No activation server.
- No telemetry.
- No cloud sync.
- No remote computation.

The 2.50.0 installer is currently unsigned. Windows SmartScreen warnings are expected until a code-signing certificate is added and the exact signed artifacts pass the release-readiness gate. Unsigned builds must not be presented as the competitor-grade stable release.
