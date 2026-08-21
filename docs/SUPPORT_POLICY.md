# QuickPLS Support Policy

## Scope

This policy defines support intake, classification, response targets, escalation,
and privacy rules for official QuickPLS releases. It is an operating policy, not
evidence that a channel has already been staffed or that a particular release is
commercially ready.

Support covers reproducible problems in documented QuickPLS workflows,
installation and upgrade failures, data-loss risks, numerical discrepancies,
accessibility barriers, and documentation defects. Method extensions, research
consulting, statistical interpretation for a specific study, journal acceptance,
and unsupported project formats are outside the support commitment.

The public `v2.50.0` unsigned pre-release receives best-effort issue triage and
security intake, not the response-time commitment of a future signed Stable
channel. Public availability must not be interpreted as commercial support.

## Support Channels

- Public product support and reproducible bug reports use the
  [QuickPLS issue tracker](https://github.com/traderbhai/QuickPLS/issues/new/choose).
- Numerical discrepancies use the `Validation discrepancy` issue form and must
  identify the QuickPLS version, method settings, reference, and tolerance.
- Suspected vulnerabilities must use a private
  [GitHub security advisory](https://github.com/traderbhai/QuickPLS/security/advisories/new),
  not a public issue.

Do not submit confidential datasets, proprietary project files, credentials,
personal data, or unpublished research through a public issue. Prefer a small,
synthetic, or anonymized reproducer. A diagnostic bundle must be reviewed by the
user before it is attached.

Before external beta begins, the release manager must record a primary and backup
owner for each channel and prove that the channels are being monitored. Publishing
this policy alone does not satisfy that release gate.

## Severity And Response Targets

Targets are measured in business days from receipt of a report containing enough
information to start triage. They are acknowledgement and triage targets, not
guaranteed resolution times.

| Severity | Definition | Initial acknowledgement | Triage or next action |
| --- | --- | ---: | ---: |
| P0 Critical | Active release compromise, arbitrary code execution, signing/update compromise, or widespread unrecoverable data loss | 1 business day | Same business day after acknowledgement |
| P1 High | Reproducible project corruption, materially incorrect supported-method output, installation failure affecting the supported matrix, or no viable recovery path | 2 business days | 3 business days |
| P2 Normal | Important defect with a safe workaround, accessibility blocker outside the core journey, or documentation that can cause material misuse | 5 business days | 10 business days |
| P3 Low | Cosmetic issue, minor documentation improvement, or feature request without correctness impact | 10 business days | Backlog decision when capacity permits |

A suspected numerical discrepancy is P1 until reproducibility and scope are
understood. It may then be reclassified with the reason recorded. A security
report is handled privately regardless of its initial severity.

## Triage And Escalation

Every accepted report must record:

1. QuickPLS version, release channel, Windows version, and artifact identity when available.
2. A privacy-safe reproducer or exact steps.
3. Severity, affected supported scope, owner, and next review date.
4. Whether data loss, scientific correctness, security, or update trust is involved.
5. Workaround, fix target, claim restriction, or reason for closure.

P0 and P1 reports are escalated to the release manager. Security reports also go
to the security owner; scientifically material discrepancies also go to the
scientific review lead. A release remains blocked while a P0 or P1 issue is open.

## Resolution And Disclosure

The project may resolve a report by fixing it, providing a verified recovery,
narrowing supported scope, documenting a bounded difference, or rejecting it with
reproducible evidence. Security disclosure timing is coordinated privately until
users have a reasonable path to a fixed build. Public release notes must identify
material fixes, known limitations, and any required user action without exposing
private reporter data.

## Operational Evidence Required

The QuickPLS 3 commercial gate still requires evidence of channel monitoring,
named escalation ownership, response performance, current known issues, and a
private security route for the target release. Those records must be reviewed and
attached to `support.operations`; this policy cannot substitute for them.
