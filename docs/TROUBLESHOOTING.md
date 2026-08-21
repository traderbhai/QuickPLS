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

Open the method from `Calculate`. QuickPLS shows the exact requirement, such as selecting a group column, choosing numeric X/Y variables for NCA, selecting a binary outcome for logistic regression, or using common-factor constructs and a supported Advanced Parameter Table for CB-SEM.

## An Advanced SEM Calculation Is Blocked

Confirm that the project is saved, Canvas has no unresolved scientific decisions, and the selected PLS Algorithm, Bootstrapping, or CB-SEM method is `Supported`. Follow the displayed correction action to open the Advanced Parameter Table, select an eligible path, or create a calculation-ready revision. The original project is not changed by a blocked preflight.

## I Cannot Find The General SEM Or Exact CB-SEM Tab

Version 2.51 intentionally removes those permanent tabs. Build the model on Canvas, then choose PLS Algorithm, Bootstrapping, or CB-SEM from Calculate. QuickPLS detects the advanced feature and routes it to the same bounded engine internally. Historical projects remain readable through compatibility adapters.

## R Or Python Not Found

R and Python are not runtime dependencies. They are used only for development validation scripts.
