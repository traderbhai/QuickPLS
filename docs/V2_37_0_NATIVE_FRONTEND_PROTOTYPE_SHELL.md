# QuickPLS v2.37.0 Native Frontend Prototype Shell

## Summary

This milestone adds an isolated QuickPLS 2 native desktop frontend prototype inside the existing application. It renders the complete shell, workflow screens, support screens, and core task dialogs from realistic dummy data before backend wiring.

The prototype is available through:

```text
?native_prototype=1
```

## Scope

- Build a full native desktop shell prototype with dummy data.
- Include Home, Data, Model, Setup, Run, Results, Report, Trust Center, and Settings.
- Include New Project, Sample Gallery, Import Data, Calculation Setup, Method Scope, Export Options, Help/Shortcuts, and Settings dialogs.
- Keep the current production UI available by default.
- Preserve backend wiring for a later adapter milestone.

## Boundary

- Frontend prototype only.
- No statistical engine, formula, estimator, result schema, project archive, validation tolerance, or numerical fingerprint changes.
- SEM designer core behavior is not replaced in this milestone; the model screen is a prototype shell for layout and visual parity.

## Evidence

- `src/v2/NativePrototypeApp.tsx`
- `src/v2/nativePrototypeData.ts`
- `src/v2/nativePrototype.css`
- `validation/v237_native_frontend_prototype_smoke.mjs`
- `validation/v237_native_frontend_prototype_audit.py`
- `validation/results/v2370_native_frontend_prototype_smoke.json`
- `validation/results/v2370_native_frontend_prototype_audit.json`
