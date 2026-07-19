# Contributing To QuickPLS

QuickPLS is proprietary source-available software. Contributions are welcome, but submitting a contribution does not change the proprietary license.

## Useful Contributions

- Bug reports with reproducible `.qpls` projects or minimal CSV fixtures.
- Validation discrepancies with expected values and reference engine details.
- Documentation improvements.
- UI/UX feedback, especially SEM diagram workflows.
- Pull requests that fix clearly scoped issues.

## Contribution Rules

- Do not submit copied code from GPL or incompatible licenses into distributed QuickPLS code.
- Do not submit reverse-engineered SmartPLS behavior, decompiled code, or proprietary third-party material.
- Keep validation references in development tooling only unless their licenses permit distribution.
- Add tests or audit evidence for behavior changes.
- Keep public claims bounded to documented support.

## Local Verification

```powershell
npm test -- --run
npm run build
cargo test -p qpls-core
npm run qpls:v10:audit
cargo run -p qpls-cli -- gate v1_0_stable
```

