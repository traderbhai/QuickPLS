# QuickPLS Known Issues

Last reviewed: 2026-08-27 for QuickPLS 2.56.0 Beta (Unsigned).

## Active Release Blockers

| Area | Current condition | Impact | Required closure |
| --- | --- | --- | --- |
| Windows trust | The public 2.56.0 Beta setup and portable executables are unsigned. | Windows may identify the publisher as unknown or show SmartScreen; this is not a signed or Stable distribution. | Verify the exact release-page filename, byte size, and SHA-256. A future signed candidate requires separate signing and release gates. |
| Installer lifecycle | The complete oldest/current Windows clean-install, N-1 upgrade, interrupted recovery, unattended install, coexistence, and uninstall matrix is not yet accepted. | Offline and institutional deployment claims are not commercially qualified. | Run the matrix against the exact candidate and preserve reports. |
| Updater | Trusted beta/stable updater channels and recovery evidence are not yet release-qualified. | Preview users must use a full installer; no stable updater claim is allowed. | Qualify signed manifests, downgrade rejection, interruption recovery, and full-installer fallback. |
| Diagnostics | Diagnostic preview, redaction, and ZIP export are implemented, but the current packaged privacy and process-boundary evidence is incomplete. | Preview users should inspect the bundle before sharing it and avoid confidential research data. | Run the packaged privacy, redaction, and boundary checks against the signed candidate. |
| External review | Independent scientific review, security review, legal review, and the external beta have not produced final approval records. | Commercial and competitor claims remain blocked. | Complete each independent review and close material findings. |
| MultiMod build authority | Only an executable carrying the exact build-embedded release authority may expose Standard MultiMod. Ordinary source builds retain the Labs sentinel. | Identical-looking source or a valid project does not make a local build release-qualified; publication exports remain authority-gated. | Use the official hash-verified Beta binary for the qualified surface, or treat a source build as Experimental Labs. |
| Validation-driver scope | Final smoke evidence used sealed temporary validation-driver corrections; product and package bytes were unchanged. | The evidence qualifies the recorded packaged candidate and does not assert ordinary clean-source smoke equivalence. | Use the exact release manifest and qualified binary identity when reviewing the 2.56.0 evidence. |

## Documented Scope Limitations

Method and convention limitations are maintained in
[`KNOWN_DIFFERENCES.md`](KNOWN_DIFFERENCES.md),
[`METHOD_COMPATIBILITY.md`](METHOD_COMPATIBILITY.md), and the
[`QuickPLS 3 parity ledger`](QUICKPLS_3_PARITY_LEDGER.md). A documented limitation
is not automatically a defect, but the UI and exported evidence must represent it
truthfully.

For MultiMod, also review
[`MULTIMOD_BOUNDARIES_AND_QUALIFICATION_V1.md`](MULTIMOD_BOUNDARIES_AND_QUALIFICATION_V1.md)
and
[`MULTIMOD_UNSUPPORTED_INTERSECTIONS_V1.md`](MULTIMOD_UNSUPPORTED_INTERSECTIONS_V1.md).
Continuous moderation V1 remains a separate protected workflow. Quadratic or
self-moderation is not part of the 2.56.0 MultiMod scope. Unsupported profile
intersections fail closed rather than being simplified automatically.

## Reporting A New Issue

Use the public issue forms for product defects and numerical discrepancies. Do not
attach confidential research data. Report suspected vulnerabilities through a
private GitHub security advisory as described in [`SUPPORT_POLICY.md`](SUPPORT_POLICY.md).

This register must be reviewed for every beta, release candidate, stable release,
and rollback. Closing an entry requires a link to evidence for the exact version;
removing text alone does not close the underlying gate.
