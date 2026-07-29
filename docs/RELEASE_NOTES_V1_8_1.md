# QuickPLS v1.8.1 Release Notes

Release tag: `v1.8.1`

Milestone: `v1_8_1_method_applicability_guided_setup`

## Highlights

- Added method applicability guidance so users can see which analyses fit the current dataset, model, and settings.
- Reworked Setup around recommended methods, available-after-setup methods, advanced diagnostics, standalone analyses, and clear unsupported/experimental states.
- Added exact reasons and next actions for unavailable methods.
- Simplified the global method/run experience so the top bar no longer encourages choosing every method blindly.
- Added Data and Model guidance panels for deciding what can be done with the current data/model.
- Preserved existing statistical engines, result schemas, project format, and numerical fingerprints.

## Release Artifacts

- `QuickPLS_1.8.1_v1_8_1_method_applicability_guided_setup_20260729-140437_x64_setup.exe`
- `QuickPLS_1.8.1_v1_8_1_method_applicability_guided_setup_20260729-140437_x64_portable.exe`
- `QuickPLS_1.8.1_v1_8_1_method_applicability_guided_setup_20260729-140437_x64_checksums.txt`

## Verification

```powershell
npm test -- --run
npm run build
npm run qpls:v18:results-report-refinement
npm run qpls:v181:method-applicability
cargo test -p qpls-core -p qpls-project -p qpls-runner -p quickpls-desktop
cargo run -p qpls-cli -- gate v1_8_1_method_applicability_guided_setup
```

## Notes

QuickPLS remains free, proprietary, offline, and Windows-focused. The installer is unsigned. R/Rscript and Python remain validation-only development dependencies.
