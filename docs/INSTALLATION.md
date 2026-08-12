# QuickPLS Installation

Current development release: `v2.45.0`.

## Choose An Install Type

Use one of the release assets from the latest GitHub Release, or use the versioned local artifacts from `target/release/artifacts/` after a production build:

- `QuickPLS_<version>_<milestone>_<timestamp>_x64_setup.exe` for a normal Windows installation.
- `QuickPLS_<version>_<milestone>_<timestamp>_x64_portable.exe` for a portable launch without installing.

Both run fully offline after download.

## Verify The Download

From PowerShell:

```powershell
Get-FileHash .\QuickPLS_<version>_<milestone>_<timestamp>_x64_setup.exe -Algorithm SHA256
Get-FileHash .\QuickPLS_<version>_<milestone>_<timestamp>_x64_portable.exe -Algorithm SHA256
```

Compare the hashes with the matching `QuickPLS_<version>_<milestone>_<timestamp>_x64_checksums.txt` file.

## Windows SmartScreen

The installer is unsigned. Windows may warn that the app is from an unknown publisher. That warning is expected until a signing certificate is added and audited.

## First Launch

1. Open QuickPLS.
2. Choose `Open demo project` to inspect the full workflow, or choose `Data` to import your own dataset.
3. Save your project as a `.qpls` file to enable autosave and recovery.

## Runtime Dependencies

QuickPLS does not require R, Rscript, Python, cloud services, telemetry, accounts, or activation at runtime. R and Python are used only by development validation scripts.

## Uninstall

If you used the installer, uninstall QuickPLS from Windows Apps/Programs. If you used the portable executable, delete the executable when no longer needed.
