# QuickPLS v2.1.5 Rendered Shell Consistency Audit

## Summary

`v2_1_5_rendered_shell_consistency_audit` adds one consolidated rendered QA gate over the current QuickPLS 2 shell. It verifies that every primary workspace opens, renders meaningful content, keeps the v2 shell surfaces visible, and avoids stale version text, mojibake, framework overlays, and unsupported SmartPLS-equivalence claims.

This is a frontend/product-only milestone. It does not redesign screens or change statistical behavior.

## User-Facing Scope

- Home, Data, Model, Setup, Run, Results, Report, Trust Center, and Settings are captured at `1440x900` and `1280x800`.
- The primary workflow rail and desktop top action strip remain visible.
- Workspace switching opens the selected page at the top of the main content area.
- Model keeps the existing SEM designer and React Flow canvas behavior.
- Trust Center and Settings remain aligned with shared v2 shell primitives.
- Global version and artifact labeling use `2.1.5` and `v2_1_5_rendered_shell_consistency_audit`.

## Evidence

- `validation/v2115_rendered_shell_consistency_smoke.mjs`
- `validation/v2115_rendered_shell_consistency_audit.py`
- `validation/results/v2115_rendered_shell_consistency_smoke.json`
- `validation/results/v2115_rendered_shell_consistency_audit.json`
- Screenshots under `validation/results/screens/v2115/rendered-shell/`

## Verification

```powershell
npm run build
npm run qpls:v2115:shell-smoke
npm run qpls:v2115:shell-audit
cargo run -p qpls-cli -- gate v2_1_5_rendered_shell_consistency_audit
```

For completed-version release artifacts:

```powershell
npm run qpls:desktop:build-versioned
```

Artifacts must be written under `D:\QuickPLS\target\release\artifacts` with version `2.1.5`, milestone label `v2_1_5_rendered_shell_consistency_audit`, timestamp, architecture, and artifact type.

## Non-Goals

- No estimator, formula, method-validation, result-schema, project-archive, or numerical-fingerprint changes.
- No method promotion or scope expansion.
- No project archive migration.
- No SEM designer behavior rewrite.
- No SmartPLS equivalence or project-import claim.
