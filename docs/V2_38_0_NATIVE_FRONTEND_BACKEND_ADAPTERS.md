# QuickPLS v2.38.0 Native Frontend Backend Adapters

## Summary

`v2_38_0_native_frontend_backend_adapters` connects the isolated QuickPLS 2 native desktop prototype to existing frontend workspace state through read-only adapters.

The prototype remains behind `?native_prototype=1`. This milestone does not replace production screens and does not change any estimator, formula, result schema, project archive format, validation tolerance, or numerical fingerprint.

## Implemented

- Added `src/v2/nativePrototypeAdapters.ts`.
- Mapped the existing workspace store into native prototype DTOs:
  - project summary;
  - dataset headers, rows, and variable metadata;
  - constructs and structural paths;
  - current method label;
  - completed-run path rows where available;
  - trust/evidence rows.
- Kept fallback mock data only for missing workspace content.
- Updated `src/v2/NativePrototypeApp.tsx` so Home, Data, Model, Setup, Run, Results, Report, and Trust Center consume the adapter output.
- Added a `data-v238-adapter` marker for smoke validation.
- Kept the prototype feature-flag isolation in `src/App.tsx`.

## Evidence

- `npm run build`
- `npm run qpls:v238:native-adapters-smoke`
- `npm run qpls:v238:native-adapters-audit`
- `cargo run -p qpls-cli -- gate v2_38_0_native_frontend_backend_adapters`

Generated evidence:

- `validation/results/v2380_native_frontend_backend_adapters_smoke.json`
- `validation/results/v2380_native_frontend_backend_adapters_audit.json`
- `validation/results/screens/v2380/native-adapters/`

## Boundary

- Frontend/product adapter milestone only.
- The adapter reads existing store state and formats it for the isolated native prototype.
- No backend command, Rust crate, statistical engine, formula, validation tolerance, or result payload behavior is changed.

## Next

Proceed to production screen replacement planning, using the adapter as the boundary between the native workbench shell and the existing QuickPLS application state.
