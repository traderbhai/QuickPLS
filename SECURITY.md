# Security Policy

## Supported Versions

| Version | Supported |
| --- | --- |
| 2.46.x | Yes |
| 2.45.x | Transition support while 2.46.x is the current stable line |
| Earlier releases and preview builds | No public security support |

## Reporting A Vulnerability

Please report suspected vulnerabilities privately by opening a GitHub security advisory if available, or by contacting the project owner directly.

Do not publish exploit details before the issue has been reviewed.

## Runtime Security Model

QuickPLS is designed as an offline Windows desktop application:

- No account.
- No activation server.
- No telemetry.
- No cloud sync.
- No remote computation.

The 2.46.0 installer is currently unsigned. Windows SmartScreen warnings are expected until a code-signing certificate is added and the exact signed artifacts pass the release-readiness gate. Unsigned builds must not be presented as the competitor-grade stable release.
