# QuickPLS Known Issues

Last reviewed: 2026-08-13

## Active Release Blockers

| Area | Current condition | Impact | Required closure |
| --- | --- | --- | --- |
| Windows trust | Current 2.50.0 artifacts are unsigned. | Windows may identify the publisher as unknown; the build is not competitor-grade distribution evidence. | Sign and timestamp the exact QuickPLS candidate, verify with SignTool, then rerun candidate-bound acceptance. |
| Installer lifecycle | The complete oldest/current Windows clean-install, N-1 upgrade, interrupted recovery, unattended install, coexistence, and uninstall matrix is not yet accepted. | Offline and institutional deployment claims are not commercially qualified. | Run the matrix against the exact candidate and preserve reports. |
| Updater | Trusted beta/stable updater channels and recovery evidence are not yet release-qualified. | Preview users must use a full installer; no stable updater claim is allowed. | Qualify signed manifests, downgrade rejection, interruption recovery, and full-installer fallback. |
| Diagnostics | The privacy-preserving rotating-log and diagnostic-bundle product workflow is designed but not implemented and packaged-tested. | Users must provide manual, privacy-safe reproduction details. | Implement preview/redaction/export, then run packaged boundary and privacy tests. |
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
