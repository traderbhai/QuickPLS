# QuickPLS 1.0.0 Methodology Manual

QuickPLS 1.0.0 implements published statistical methods independently and validates the supported scope through method specifications, unit tests, independent reference fixtures, external reference engines where appropriate, and release audits.

## Validation Standard

- Deterministic reported coefficients target agreement within `1e-6` when conventions match.
- Displayed values normally match at four decimals where settings and conventions are equivalent.
- Stochastic procedures use deterministic seeds and indexed random streams.
- Known differences are documented in `docs/V1_KNOWN_DIFFERENCES.md`.

## Runtime Independence

QuickPLS does not require R, Python, cloud services, telemetry, or activation at runtime. R and Python scripts are validation-only references.

## SmartPLS Relationship

QuickPLS is a free proprietary alternative for documented supported workflows. It does not import SmartPLS project files, decompile SmartPLS, or claim identical behavior to SmartPLS.

## Method Boundaries

Each method is stable only for the shapes listed in `docs/V1_SUPPORTED_SCOPE.md`, `docs/V1_COMPATIBILITY_MATRIX.md`, and the relevant method specification under `docs/methods/`.
