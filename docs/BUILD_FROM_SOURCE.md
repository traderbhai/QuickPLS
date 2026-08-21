# Build QuickPLS From Source

QuickPLS is proprietary source-available software. Building from source is intended for inspection, local testing, and approved contribution work under [LICENSE.md](../LICENSE.md).

## Requirements

- Windows 10/11.
- Node.js and npm.
- Rust stable toolchain.
- Tauri 2 prerequisites for Windows.
- WebView2 runtime.
- NSIS for installer bundling.

R/Rscript and Python are validation-only tools. They are not required to run QuickPLS.

## Install Dependencies

```powershell
npm install
```

## Run Frontend Checks

```powershell
npm test -- --run
npm run build
```

## Run Core Desktop Checks

```powershell
cargo test -p qpls-core -p qpls-project -p qpls-runner -p quickpls-desktop
```

## Run Current Release Foundation Checks

```powershell
npm run qpls:release:foundation
python validation/capability_registry_v2.py --check-legacy
```

## Development Desktop App

```powershell
npm run tauri dev
```

The browser URL used during development is only a preview. Native file dialogs, durable project storage, and engine jobs require the Tauri desktop app.

## Production Build

```powershell
npm run qpls:desktop:build-versioned
```

This command runs the Tauri production build and then packages versioned artifacts into:

```text
target/release/artifacts/
```

Current artifact naming pattern:

```text
QuickPLS_<version>_<channel>_<label>_<UTC>_x64_setup.exe
QuickPLS_<version>_<channel>_<label>_<UTC>_x64_portable.exe
QuickPLS_<version>_<channel>_<label>_<UTC>_x64_cli.exe
QuickPLS_<version>_<channel>_<label>_<UTC>_x64_checksums.txt
```

Do not overwrite older release artifacts.
