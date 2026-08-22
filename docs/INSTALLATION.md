# QuickPLS Installation

Current source version: **2.55.0**. Formal first diagnostic `20260822T142953Z`
at source `2e3a23f` executed all 14 steps and passed 13; the sole failure was
`frontend_typecheck`, where `src/data/v255NamedSemEvidenceFixtures.test.ts`
reported TypeScript error `TS2339`. The same-script, same-suite formal final
diagnostic `20260822T155928Z` at source `6aa92b7` passed 14/14,
including 453/453 Vitest suites, 1697/1697 tests, all 17 rebaseline assertions,
and zero captured console errors. Both formal records use runner SHA-256
`64969b4eb89c9789e586c372532d89b082766a44661cc4feea66b6cf3a0f9796`;
the reports are separate evidence records and are not byte-identical.
A new provenance-bound unsigned setup, portable, CLI, and checksum package,
its isolated install, and full installed-and-portable smoke remain pending,
followed by evidence collection, bundling, and publication. Code signing is
excluded. Exactly one case—the actual Windows 200% scaling case—may use the
opt-in waiver; its real observed DPI screenshot and receipt remain required,
its status remains `waived`, and the other 54 named cases must pass.

Latest published public pre-release: [`v2.54.0`](https://github.com/traderbhai/QuickPLS/releases/tag/v2.54.0). Its setup, portable, CLI, manifest, and checksum package passed the documented 2.54 workflow and remains unsigned. Candidate/install/smoke attempts `20260822T145055Z`, `20260822T151222Z`, and every earlier local 2.55 candidate, install, or smoke attempt are historical and ineligible. The install wrapper permits only Tauri's exact three-byte `UNK` → `NSS` package marker transition and rejects every other byte difference. One new candidate build, isolated install, and full installed-and-portable smoke remain pending, followed by evidence collection, bundling, and publication; no 2.55 download or installed-app success is claimed.

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

For controlled 2.55 release qualification, an existing registered QuickPLS installation may be removed only through its exact registered uninstaller before the isolated candidate install. Project files, recovery data, and QuickPLS application user data must remain untouched and must be verified unchanged across that operation. The portable executable is still required for its separate journey, but it does not substitute for installed-candidate evidence.
