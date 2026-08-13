# QuickPLS Release Channels and Unsigned Preview Packaging

## Purpose

QuickPLS development can continue before an Authenticode identity is available,
but unsigned engineering artifacts are not beta, stable, or competitor-ready
releases. The machine-readable policy is
[`validation/quickpls_release_channels.json`](../validation/quickpls_release_channels.json).
The policy is enforced in code so changing a JSON flag cannot authorize an
unsigned commercial release.

## Channels

| Channel | Audience | Authenticode | Distribution rule | Factory |
| --- | --- | --- | --- | --- |
| `internal` | Maintainers | Not required | Maintainers only | Unsigned preview |
| `unsigned-preview` | Named technical-preview testers | Not required | Private named testers only | Unsigned preview |
| `beta` | Named external beta testers | Required | Signed prerelease only | Signed candidate |
| `stable` | Public users | Required | All commercial gates must pass | Signed candidate |

`unsigned-preview` is an engineering channel, not an additional commercial
channel. It cannot authorize competitor claims. Checksums prove that copied
bytes did not change; they do not establish publisher identity or replace an
Authenticode signature.

## Artifact naming and preservation

After a successful local Tauri release build, run:

```powershell
npm run qpls:release:artifacts
```

The command preserves the desktop executable, CLI, NSIS installer, and checksum
manifest under unique names such as:

```text
QuickPLS_2.46.0_unsigned-preview_<label>_<UTC>_x64_setup.exe
QuickPLS_2.46.0_unsigned-preview_<label>_<UTC>_x64_portable.exe
QuickPLS_2.46.0_unsigned-preview_<label>_<UTC>_x64_cli.exe
QuickPLS_2.46.0_unsigned-preview_<label>_<UTC>_x64_checksums.txt
```

The factory refuses `beta` and `stable`; those channels require a future signed
candidate pipeline and the existing commercial-readiness validator.

## Offline Windows installation

The NSIS bundle embeds Microsoft's WebView2 offline installer and runs it
silently. This increases installer size but makes clean installation independent
of network access. Installer downgrades are disabled. Portable execution still
depends on a compatible WebView2 runtime already being available on the machine;
that limitation must remain visible in portable-build instructions and tests.

Validate the configuration without building Tauri:

```powershell
npm run qpls:release:foundation
python -m unittest validation.test_package_release_artifacts validation.test_quickpls_release_foundation
```

## Certificate checkpoint

When an Authenticode identity is available, approve exactly one QuickPLS leaf
certificate in `validation/quickpls_signing_identity.json`. The record freezes
the exact subject, SHA-1 certificate thumbprint, approval, and hardware-backed
or managed key custody. It is currently `pending`; no caller-supplied publisher
pattern can substitute for it.

The signed-candidate factory intentionally cannot run without that approved
record, the protected main-branch release workflow, SignTool, the approved
certificate private key, and already signed/timestamped desktop, CLI, and
installer bytes:

`validation/quickpls_signing_identity.json` remains pending and the reviewed
`.github/workflows/release.yml` protected workflow has not yet been installed,
so the beta/stable path is deliberately blocked today.

```powershell
python validation/quickpls_signed_candidate.py `
  --channel beta `
  --label beta1 `
  --minimum-installed-version "<oldest tested upgrade source>" `
  --build-started-at "<ISO-8601 timestamp>" `
  --build-finished-at "<ISO-8601 timestamp>"
```

It independently re-verifies each source PE, copies it to durable
`release/candidates/` storage, rehashes and re-verifies the copy, and requires
the leaf subject, leaf thumbprint, product name, product/file version, and
original filename to match. It emits an actual CycloneDX 1.6 dependency graph,
an in-toto/SLSA provenance statement, and an approved-leaf-signed protected
build attestation.

The manual updater bundle contains the exact full installer plus a detached-CMS
signed beta/stable channel manifest. The manifest freezes the PE payload ID,
channel, version floor, no-downgrade rule, installer hash, and offline recovery
hash. Commercial readiness rehashes all files and live-verifies both PE and CMS
signatures. Candidate assembly does not approve beta or stable; every external
gate and the later release decision remain mandatory. Never retrofit unsigned-
preview results as signed beta or stable evidence.
