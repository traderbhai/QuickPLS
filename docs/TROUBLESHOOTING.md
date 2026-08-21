# QuickPLS Troubleshooting

## Windows SmartScreen Warns About The Installer

The public `v2.50.0` pre-release installer is unsigned. Download it only from the official GitHub Release and verify its SHA-256 value against the attached checksum file. The warning remains expected until a signed release is published.

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

## A Method Is Missing From Calculate

Choose `Calculate` from the active Data or Model context. QuickPLS lists only methods whose exact data/model predicate can be evaluated; blocked estimator cards show the corrective reason. Experimental cells also require the Labs preference.

## A Method Says Needs Setup Or Not Applicable

Open the method or estimator card from `Calculate`. QuickPLS shows the exact requirement, such as selecting a group column, choosing numeric X/Y variables for NCA, selecting a binary outcome for logistic regression, or using common-factor constructs and a supported Parameter Table for General SEM CB-SEM.

## General SEM Is Blocked

Confirm that the project is saved and activated, the Canvas and Parameter Table have no unresolved scientific decisions, and the selected PLS-SEM or CB-SEM estimator card is `Supported`. Higher-order constructs and moderated mediation must be authored through **Save As Revision** before calculation.

## R Or Python Not Found

R and Python are not runtime dependencies. They are used only for development validation scripts.
