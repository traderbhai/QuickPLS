# QuickPLS Installation

Current development release: `v2.50.0`.

## Choose An Install Type

Use one of the release assets from the latest GitHub Release, or use the versioned local artifacts from `target/release/artifacts/` after a production build:

- `QuickPLS_<version>_<milestone>_<timestamp>_x64_setup.exe` for a normal Windows installation.
- `QuickPLS_<version>_<milestone>_<timestamp>_x64_portable.exe` for a portable launch without installing.
- `QuickPLS_<version>_<milestone>_<timestamp>_x64_cli.exe` for offline command-line and batch recipe execution.

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
Get-FileHash .\QuickPLS_<version>_<milestone>_<timestamp>_x64_setup.exe -Algorithm SHA256
Get-FileHash .\QuickPLS_<version>_<milestone>_<timestamp>_x64_portable.exe -Algorithm SHA256
Get-FileHash .\QuickPLS_<version>_<milestone>_<timestamp>_x64_cli.exe -Algorithm SHA256
```

Compare the hashes with the matching `QuickPLS_<version>_<milestone>_<timestamp>_x64_checksums.txt` file.

## Windows SmartScreen

The installer is unsigned. Windows may warn that the app is from an unknown publisher. That warning is expected until a signing certificate is added and audited.

## First Launch

1. Open QuickPLS.
2. Choose `Open demo project` to inspect the full workflow, or choose `Data` to import your own dataset.
3. Save your project as a `.qpls` file to enable autosave and recovery.

## Runtime Dependencies

QuickPLS does not require R, Rscript, Python, cloud services, accounts, or activation at runtime. QuickPLS product telemetry is disabled and its application/page makes no external requests. The separate Microsoft-managed WebView2 process boundary described above still applies. R and Python are used only by development validation scripts.

## Uninstall

If you used the installer, uninstall QuickPLS from Windows Apps/Programs. If you used the portable executable, delete the executable when no longer needed.
