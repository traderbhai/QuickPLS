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

## Run Source Capability Checks

```powershell
python validation/capability_registry_v2.py --check-legacy
```

The repository also retains historical release-campaign scripts and evidence
contracts for older exact candidates. They are not generic packaging commands
for 2.56.0 and can intentionally reject the current version.

## Development Desktop App

```powershell
npm run tauri dev
```

The browser URL used during development is only a preview. Native file dialogs, durable project storage, and engine jobs require the Tauri desktop app.

## Production Build

```powershell
npm run tauri build
```

This creates a local, unsigned production build. It is suitable for inspection
and approved contribution testing; it is not the release-qualified QuickPLS
2.56.0 package. MultiMod displays **Standard · Release-qualified** only when the
build embeds the exact external authority for the qualified source commit and
every declared capability cell. A normal local build fails closed to
Labs/unavailable.

The historical `qpls:desktop:build-versioned` maintenance command remains bound
to an earlier packaging campaign and must not be used to claim or reproduce the
2.56.0 release. Official 2.56.0 binaries and checksums are published on the
[GitHub Beta download page](https://github.com/traderbhai/QuickPLS/releases/tag/v2.56.0-beta.1).

Maintainer-approved packaging campaigns preserve versioned artifacts under:

```text
target/release/artifacts/
```

The qualified 2.56.0 release manifest uses these exact artifact names:

```text
QuickPLS_2.56.0_unsigned-preview_multimod_28939b73_x64_setup.exe
QuickPLS_2.56.0_unsigned-preview_multimod_28939b73_x64_portable.exe
QuickPLS_2.56.0_SHA256SUMS.txt
QuickPLS_2.56.0_RELEASE_MANIFEST.json
```

Do not overwrite older release artifacts.
