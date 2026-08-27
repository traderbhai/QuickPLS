# Security Policy

## Supported Versions

| Version | Supported |
| --- | --- |
| 2.56.0 Beta (Unsigned) | Security reports accepted; fixes and issue triage are best effort |
| 2.54.x and earlier | Superseded builds; critical transition reports are reviewed, but users should reproduce against 2.56.0 when safe |

The lifecycle and transition rules are defined in
[`docs/VERSION_SUPPORT_POLICY.md`](docs/VERSION_SUPPORT_POLICY.md). Public
availability of an unsigned preview does not make it a supported stable release;
that requires the signed commercial gate.

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
- No QuickPLS product telemetry.
- No cloud sync.
- No remote computation.

The 2.56.0 Beta executables are unsigned. Windows may identify the publisher as
unknown or show Microsoft SmartScreen. Verify the exact SHA-256 values in
[`docs/INSTALLATION.md`](docs/INSTALLATION.md) before running either asset. This
Beta must not be presented as signed, Stable, or a full-parity release.

QuickPLS product telemetry is disabled and its documented analytical workflow
runs locally. The separately managed Microsoft WebView2 Runtime may still make
background service connections unless an OS-level network policy blocks them;
the Beta does not claim zero egress for the complete WebView2 process tree.
