# QuickPLS v2.1.4 Model/Trust/Settings Shell Alignment

## Summary

`v2_1_4_model_trust_settings_shell_alignment` completes the QuickPLS 2.1 shell-alignment pass for the remaining Model shell, Trust Center, Settings, and global shell surfaces.

This is a frontend/product-only milestone. It does not change statistical engines, formulas, method validation, result schemas, project archive format, validation tolerances, or numerical fingerprints.

## User-Facing Scope

- Model keeps the existing SEM designer behavior, but the central canvas shell now exposes v2.1.4 workspace, toolbar, and overlay-status hooks for consistent visual QA.
- Trust Center uses the shared v2 workspace page, page header, panel, metric, table, card, and status primitives.
- Settings keeps the shared v2 workspace shell and explicitly identifies the desktop UI settings surface as part of the v2.1.4 alignment pass.
- Global version and artifact labeling use `2.1.4` and `v2_1_4_model_trust_settings_shell_alignment`.
- The milestone keeps scoped validation wording and must not make SmartPLS-equivalence or SmartPLS project-import claims.

## Evidence

- `validation/v2114_model_trust_settings_shell_smoke.mjs`
- `validation/v2114_model_trust_settings_shell_audit.py`
- `validation/results/v2114_model_trust_settings_shell_smoke.json`
- `validation/results/v2114_model_trust_settings_shell_audit.json`
- Screenshots under `validation/results/screens/v2114/model-trust-settings/`

## Verification

```powershell
npm run build
npm run qpls:v2114:shell-smoke
npm run qpls:v2114:shell-audit
cargo run -p qpls-cli -- gate v2_1_4_model_trust_settings_shell_alignment
```

For release artifacts:

```powershell
npm run qpls:desktop:build-versioned
```

Artifacts must be written under `D:\QuickPLS\target\release\artifacts` with version `2.1.4`, milestone label `v2_1_4_model_trust_settings_shell_alignment`, timestamp, architecture, and artifact type.

## Non-Goals

- No estimator or numerical result changes.
- No method promotion or scope expansion.
- No project archive migration.
- No SEM designer behavior rewrite.
- No SmartPLS equivalence or project-import claim.
