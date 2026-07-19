# Security Policy

## Supported Versions

| Version | Supported |
| --- | --- |
| 1.0.x | Yes |
| Earlier preview builds | No public security support |

## Reporting A Vulnerability

Please report suspected vulnerabilities privately by opening a GitHub security advisory if available, or by contacting the project owner directly.

Do not publish exploit details before the issue has been reviewed.

## Runtime Security Model

QuickPLS 1.0.0 is designed as an offline Windows desktop application:

- No account.
- No activation server.
- No telemetry.
- No cloud sync.
- No remote computation.

The installer is currently unsigned. Windows SmartScreen warnings are expected until a code-signing certificate is added and audited.

