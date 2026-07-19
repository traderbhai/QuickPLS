# GitHub Release Checklist For QuickPLS 1.0.0

## Before Publishing

Run:

```powershell
npm test -- --run
npm run build
npm run qpls:publication:all
npm run qpls:v093:sem-designer
npm run tauri -- build
npm run qpls:v10:audit
cargo run -p qpls-cli -- gate v1_0_stable
```

## Tag

```powershell
git tag v1.0.0
git push origin v1.0.0
```

## Release Title

```text
QuickPLS 1.0.0 Stable
```

## Attach Assets

- `target/release/bundle/nsis/QuickPLS_1.0.0_x64-setup.exe`
- `target/release/quickpls-desktop.exe`
- `docs/RELEASE_CHECKSUMS_V1_0.txt`

## Release Description

Use the content from `docs/RELEASE_NOTES_V1_0.md` and include:

- Supported scope link.
- Known differences link.
- Installer unsigned warning.
- SHA-256 checksums.
- No SmartPLS project import.
- No SmartPLS equivalence claim.

