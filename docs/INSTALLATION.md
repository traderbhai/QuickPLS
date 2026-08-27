# QuickPLS 2.56.0 installation

QuickPLS 2.56.0 Beta (Unsigned) is a Windows x64 evaluation release. It is not a
signed or Stable release and does not claim unrestricted SmartPLS parity.

- Release page: [v2.56.0-beta.1](https://github.com/traderbhai/QuickPLS/releases/tag/v2.56.0-beta.1)
- Candidate source: `28939b73db8f2284f21ce184050eb3f04110bf94`
- Supported Beta platform: maintained Windows 10/11 x64 installations

## Choose a download

| Use | Asset | Size | SHA-256 |
| --- | --- | ---: | --- |
| Recommended installation | `QuickPLS_2.56.0_unsigned-preview_multimod_28939b73_x64_setup.exe` | 231,889,558 bytes (221.15 MiB) | `46b121d252ba236f4e8d72e5e618bbf4542d0cdb0819b834fb9b3d3677599523` |
| Portable launch | `QuickPLS_2.56.0_unsigned-preview_multimod_28939b73_x64_portable.exe` | 79,480,832 bytes (75.80 MiB) | `54abad31daca14dbad16588f85015fb39b030bac28e1cd475a10d4d4573cb2d1` |

Choose **setup** for normal use. It includes Microsoft's WebView2 offline
installer and can prepare a machine without downloading WebView2 during the
QuickPLS installation. Choose **portable** only when a compatible WebView2
Runtime is already installed and normal application installation is unavailable
or undesirable.

QuickPLS does not require R, Rscript, Python, a QuickPLS account, an activation
server, cloud storage, or remote computation at runtime.

## Verify the download

Open PowerShell in the download folder and run the command for your asset:

```powershell
Get-FileHash .\QuickPLS_2.56.0_unsigned-preview_multimod_28939b73_x64_setup.exe -Algorithm SHA256
Get-FileHash .\QuickPLS_2.56.0_unsigned-preview_multimod_28939b73_x64_portable.exe -Algorithm SHA256
```

Expected values:

```text
setup     46b121d252ba236f4e8d72e5e618bbf4542d0cdb0819b834fb9b3d3677599523
portable  54abad31daca14dbad16588f85015fb39b030bac28e1cd475a10d4d4573cb2d1
```

Compare all 64 hexadecimal characters. Do not run a file with a different
size, hash, name, or download source. A checksum can detect changed or corrupted
bytes, but it does not establish publisher identity.

## Windows SmartScreen

The Beta executables are not Authenticode-signed. Windows may report an unknown
publisher or display Microsoft SmartScreen. This is expected for this unsigned
Beta; it is not evidence of a signed or verified publisher.

Proceed only if:

1. the file came from the official release page above;
2. its exact filename and byte size match this guide; and
3. its complete SHA-256 value matches this guide.

If any item differs, cancel the launch and report it through the
[QuickPLS issue tracker](https://github.com/traderbhai/QuickPLS/issues/new/choose).

## Install with setup

1. Back up important `.qpls` projects and original datasets before changing an
   application installation.
2. Run `QuickPLS_2.56.0_unsigned-preview_multimod_28939b73_x64_setup.exe`.
3. Review the Windows warning and continue only after completing the verification
   above.
4. Complete the installer. The bundled WebView2 offline installer runs silently
   when the required runtime is unavailable.
5. Start QuickPLS from the Start menu.
6. Open **Help → About** and confirm Version `2.56.0`.

If Windows reports an installation conflict with an older registered QuickPLS
installation, remove that installation through **Settings → Apps → Installed
apps** using its registered uninstaller, then run the 2.56.0 setup again. Do not
manually delete project, recovery, or QuickPLS application-data folders as an
installation workaround.

This Beta does not claim the complete oldest-version upgrade, unattended
deployment, coexistence, interruption-recovery, or institutional rollout matrix
required for a future Stable release.

## Use the portable executable

1. Verify the portable file exactly as described above.
2. Move it to a user-writable folder if desired.
3. Run `QuickPLS_2.56.0_unsigned-preview_multimod_28939b73_x64_portable.exe`.
4. If QuickPLS cannot start because WebView2 is unavailable, use the setup
   installer or install a compatible Microsoft WebView2 Runtime through your
   organization's approved process.

The portable executable is a separate application launch, not proof that the
setup installer or an installed upgrade path works. Delete the portable
executable when it is no longer required.

## First launch

1. Create a project, open a `.qpls` project, or select a bundled sample.
2. Import and inspect data in **Data**.
3. Create or activate a model on **Canvas**.
4. Use **Calculate** for the normal method catalogue.
5. For the new suite, activate a calculation-ready model and choose
   **Moderation & heterogeneity…** from the Model toolbar. Select Categorical
   moderation, Latent segmentation, Conditional process, or Interventional
   mediation.
6. Review every eligibility warning and exact method boundary before calculating
   or reporting a result.

The MultiMod result should visibly identify its qualification state. An official
candidate-authorized result says **Standard · Release-qualified**. A source-built
or altered executable without the exact embedded candidate authority remains
**Experimental Labs · Unqualified**. A Labs-only build cannot be promoted by a
project file, environment variable, or UI request.

See [MultiMod boundaries and qualification](MULTIMOD_BOUNDARIES_AND_QUALIFICATION_V1.md)
and [unsupported intersections](MULTIMOD_UNSUPPORTED_INTERSECTIONS_V1.md) before
using the new workflows.

## Offline behavior

QuickPLS calculations, projects, and exports remain local. QuickPLS product
telemetry is disabled, and no account or cloud service is required. The QuickPLS
application/page is configured to make no external requests during the
documented offline workflow.

Microsoft manages the separate WebView2 Runtime. Its process tree may make
background service connections unless Windows or an independently verified
OS-level network policy blocks them. QuickPLS 2.56.0 therefore does not claim
zero network egress for the complete WebView2 process tree.

## Uninstall

- Setup installation: use **Settings → Apps → Installed apps → QuickPLS →
  Uninstall**.
- Portable: close QuickPLS and delete the portable executable.

Uninstalling the application is not a substitute for managing research data.
Keep your own backups and confirm the location of projects and exports before
removing software or application data.

For help, see [Known Issues](KNOWN_ISSUES.md), [Support Policy](SUPPORT_POLICY.md),
and the repository [Security Policy](../SECURITY.md).
