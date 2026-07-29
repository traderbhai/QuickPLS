# QuickPLS Troubleshooting

## Windows SmartScreen Warns About The Installer

The current installer is unsigned. This warning is expected until a code-signing certificate is added and audited.

## Browser Preview Cannot Run Analyses

The browser page is only a frontend preview. Native project storage, file dialogs, and engine jobs require the Tauri desktop app.

Use:

```powershell
npm run tauri dev
```

for development, or launch the release executable.

## Diagram Estimates Do Not Appear

Estimates are hidden until a compatible completed saved run is selected.

If the model changed after the run, QuickPLS suppresses stale overlays. Rerun the analysis.

## Import Fails

Check:

- Duplicate column names.
- Empty column names.
- Nonnumeric variables selected for numeric methods.
- Malformed CSV/TSV quoting.
- Unsupported covariance/correlation matrix shape.

## Installer Works But App Looks Old

Confirm you are running:

```text
target/release/quickpls-desktop.exe
```

or the installed release application. Development debug builds may require a running dev server.

## A Method Is Missing From The Top Bar

The top bar intentionally lists only recommended or currently available methods. Open `Setup` and use `Show all methods` to see methods that need setup, are not applicable, unsupported, or experimental.

## A Method Says Needs Setup Or Not Applicable

Open the method card in `Setup`. QuickPLS shows the exact requirement, such as selecting a group column, choosing numeric X/Y variables for NCA, selecting a binary outcome for logistic regression, or using reflective constructs for CB-SEM/CFA scope.

## R Or Python Not Found

R and Python are not runtime dependencies. They are used only for development validation scripts.
