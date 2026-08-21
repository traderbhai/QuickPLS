# QuickPLS Known Issues

Last reviewed: 2026-08-21

## Active Release Blockers

| Area | Current condition | Impact | Required closure |
| --- | --- | --- | --- |
| Windows trust | The public 2.50.0 GitHub pre-release artifacts are unsigned. | Windows may identify the publisher as unknown or show SmartScreen; this is not a signed stable distribution. | Sign and timestamp a future candidate, verify it with SignTool, and publish it through the signed beta/stable channel. |
| Installer lifecycle | The complete oldest/current Windows clean-install, N-1 upgrade, interrupted recovery, unattended install, coexistence, and uninstall matrix is not yet accepted. | Offline and institutional deployment claims are not commercially qualified. | Run the matrix against the exact candidate and preserve reports. |
| Updater | Trusted beta/stable updater channels and recovery evidence are not yet release-qualified. | Preview users must use a full installer; no stable updater claim is allowed. | Qualify signed manifests, downgrade rejection, interruption recovery, and full-installer fallback. |
| Diagnostics | Diagnostic preview, redaction, and ZIP export are implemented, but the current packaged privacy and process-boundary evidence is incomplete. | Preview users should inspect the bundle before sharing it and avoid confidential research data. | Run the packaged privacy, redaction, and boundary checks against the signed candidate. |
| External review | Independent scientific review, security review, legal review, and the external beta have not produced final approval records. | Commercial and competitor claims remain blocked. | Complete each independent review and close material findings. |

## Documented Scope Limitations

Method and convention limitations are maintained in
[`KNOWN_DIFFERENCES.md`](KNOWN_DIFFERENCES.md),
[`METHOD_COMPATIBILITY.md`](METHOD_COMPATIBILITY.md), and the
[`QuickPLS 3 parity ledger`](QUICKPLS_3_PARITY_LEDGER.md). A documented limitation
is not automatically a defect, but the UI and exported evidence must represent it
truthfully.

## Reporting A New Issue

Use the public issue forms for product defects and numerical discrepancies. Do not
attach confidential research data. Report suspected vulnerabilities through a
private GitHub security advisory as described in [`SUPPORT_POLICY.md`](SUPPORT_POLICY.md).

This register must be reviewed for every beta, release candidate, stable release,
and rollback. Closing an entry requires a link to evidence for the exact version;
removing text alone does not close the underlying gate.
