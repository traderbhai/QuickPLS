# QuickPLS v2.42.1 Native Shell QA Test Pack

## Summary

`v2_42_1_native_shell_qa_test_pack` performs the first full QA pass after the native mockup-parity shell became the default QuickPLS UI. The milestone is intentionally frontend/product focused: it validates screen coverage, menu/dialog interaction wiring, old web-app trace removal, and release metadata for a user-testable desktop build.

## Scope

- Home, Data, Model, Setup, Run, Results, Report, Trust Center, and Settings are captured in the default native shell.
- Import Data and Calculation Setup dialogs are captured and checked.
- Menus, rail navigation, dialog close paths, Escape handling, and backend-adapter surfaces are smoke tested.
- The older shell remains available only through `?legacy_shell=1`.
- Release-facing version metadata is set to `2.42.1`.
- The versioned artifact label is `v2_42_1_native_shell_qa_test_pack`.

## Evidence

- `validation/results/v2421_native_screen_qa_smoke.json`
- `validation/results/v2421_native_interaction_wiring_smoke.json`
- `validation/results/v2421_native_web_trace_audit.json`
- `validation/results/v2421_native_qa_test_pack_audit.json`
- Screenshots under `validation/results/screens/v2421/native-qa/`

## Test Artifacts

- `target/release/artifacts/QuickPLS_2.42.1_v2_42_1_native_shell_qa_test_pack_20260730-183548_x64_setup.exe`
- `target/release/artifacts/QuickPLS_2.42.1_v2_42_1_native_shell_qa_test_pack_20260730-183548_x64_portable.exe`
- `target/release/artifacts/QuickPLS_2.42.1_v2_42_1_native_shell_qa_test_pack_20260730-183548_x64_checksums.txt`

## Commands

```powershell
npm run build
npm run qpls:v2421:screen-qa
npm run qpls:v2421:interaction-wiring
npm run qpls:v2421:web-trace-audit
npm run qpls:v2421:qa-test-pack
cargo run -p qpls-cli -- gate v2_42_1_native_shell_qa_test_pack
npm run qpls:desktop:build-versioned
```

## Boundary

No estimator, formula, method validation, result schema, project archive format, validation tolerance, or numerical fingerprint behavior changes are part of this milestone.
