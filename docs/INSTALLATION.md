# QuickPLS Installation

Current source version: **2.55.0**. The post-hoc-corrected formal source reports
`20260822T064330Z` at source `0ae22eb` and `20260822T064832Z` at source
`0238e20` each passed 14/14 using the same diagnostic script and suite contract.
A fresh provenance-bound unsigned setup, portable, CLI, and checksum package,
its installed and portable packaged journeys, the evidence bundle, and
publication remain pending. Code signing is excluded.

Latest published public pre-release: [`v2.54.0`](https://github.com/traderbhai/QuickPLS/releases/tag/v2.54.0). Its setup, portable, CLI, manifest, and checksum package passed the documented 2.54 workflow and remains unsigned. Every earlier local 2.55 candidate, including `20260822T052553Z`, is historical and ineligible because the current validation and post-hoc reopen-label product correction postdate it. Exact typed post-hoc PLS authority now takes precedence over the generic bootstrap fallback, and the focused identity suite passed 17/17; this is not packaged validation. The install wrapper separately permits only Tauri's exact three-byte `UNK` → `NSS` package marker transition and rejects all other byte differences. A fresh candidate, install, and smoke run remain pending and no 2.55 download or installed-app success is claimed.

## Choose An Install Type

Use one of the assets from the [QuickPLS 2.54.0 release page](https://github.com/traderbhai/QuickPLS/releases/tag/v2.54.0). Maintainers can create versioned local artifacts under `target/release/artifacts/` after a production build:

- `QuickPLS_<version>_<channel>_<label>_<UTC>_x64_setup.exe` for a normal Windows installation.
- `QuickPLS_<version>_<channel>_<label>_<UTC>_x64_portable.exe` for a portable launch without installing.
- `QuickPLS_<version>_<channel>_<label>_<UTC>_x64_cli.exe` for offline command-line and batch recipe execution.

For most users, choose **setup**. Choose **portable** when installation is not possible and a compatible Microsoft WebView2 runtime is already available. The `v2.54.0` files include `unsigned-preview` in their names because they are not Authenticode-signed.

The desktop, CLI, and analytical workflows require no internet connection,
account, or cloud service after download. The QuickPLS application and page make
no external requests. This is a functional-offline claim, not a literal
fully-offline, no-telemetry, or zero-egress process-tree claim: the
Microsoft-managed WebView2 runtime may make its own background service
connections unless an independently validated OS-enforced fixed-WebView2
network boundary is applied. See `docs/WEBVIEW2_OFFLINE_BOUNDARY.md`.

## Verify The Download

From PowerShell:

```powershell
Get-FileHash .\QuickPLS_<version>_<channel>_<label>_<UTC>_x64_setup.exe -Algorithm SHA256
Get-FileHash .\QuickPLS_<version>_<channel>_<label>_<UTC>_x64_portable.exe -Algorithm SHA256
Get-FileHash .\QuickPLS_<version>_<channel>_<label>_<UTC>_x64_cli.exe -Algorithm SHA256
```

Compare the hashes with the matching `QuickPLS_<version>_<channel>_<label>_<UTC>_x64_checksums.txt` file. Checksums detect changed or corrupted bytes; they do not establish publisher identity.

## Windows SmartScreen

The installer is unsigned. Windows may warn that the app is from an unknown publisher or show Microsoft SmartScreen. Confirm that the filename came from the official GitHub Release and that its SHA-256 value matches the attached checksum file before running it. A future signed build will replace this preview distribution path.

## First Launch

1. Open QuickPLS.
2. Choose `Open demo project`, create a project, or import your own dataset.
3. Save the project as a `.qpls` file to enable autosave and recovery.
4. Use Canvas to draw the diagram, then choose from the unchanged 18-method `Calculate` catalogue. Open the Advanced Parameter Table only when a CB-SEM setting or corrective action requires parameter-level editing.
5. After completion, use `Results` to inspect, export, save, close, and strictly reopen the verified result.

## Runtime Dependencies

QuickPLS does not require R, Rscript, Python, cloud services, accounts, or activation at runtime. QuickPLS product telemetry is disabled and its application/page makes no external requests. The installer embeds the WebView2 offline installer; portable execution requires a compatible WebView2 runtime already installed. The separate Microsoft-managed WebView2 process boundary described above still applies. R and Python are used only by development validation scripts.

## Uninstall

If you used the installer, uninstall QuickPLS from Windows Apps/Programs. If you used the portable executable, delete the executable when no longer needed.

For the controlled 2.55 release qualification, the existing registered QuickPLS 2.53 installation may be removed through its exact registered uninstaller before the isolated candidate install. Project files, recovery data, and QuickPLS application user data must remain untouched and should be verified unchanged across that operation. The portable executable may be used for its separate portable journey, but it does not substitute for installed-candidate evidence.
