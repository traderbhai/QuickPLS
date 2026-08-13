# QuickPLS v2.42.0 Make Native Mockup Shell Default

QuickPLS v2.42.0 promotes the v2 native mockup-parity shell from opt-in route to the default app UI.

## Scope

- Frontend/product routing only.
- No estimator, validation, project archive, result schema, tolerance, or numerical fingerprint changes.
- Default app launch now renders the production-candidate native shell.
- `?native_shell=1` remains accepted for compatibility.
- `?native_prototype=1` remains the static prototype route.
- `?legacy_shell=1` is the explicit fallback for the older v1-style shell during transition testing.

## Evidence

- `src/App.tsx` makes `NativeShellCandidateApp` the default route.
- `validation/v2420_native_default_shell_smoke.mjs` captures the default native shell screens and the explicit legacy fallback screenshot.
- `validation/v2420_native_default_shell_audit.py` checks routing, registry, scripts, screenshots, and frontend-only boundaries.
- Screenshots are written under `validation/results/screens/v2420/native-default/`.

## Acceptance

The gate `v2_42_0_make_native_mockup_shell_default` passes only when the default URL loads the native production-candidate workbench, all primary/support screens render inside the native shell, legacy chrome is absent by default, and the legacy shell is available only through the explicit fallback route.
