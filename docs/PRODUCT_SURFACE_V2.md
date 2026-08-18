# Standard, Experimental Labs, Legacy, and Internal surfaces

QuickPLS product visibility is evaluated per capability option cell. A method
family cannot inherit a single status when its estimator, bootstrap, group, or
comparison options have different coverage or evidence.

The executable policy is in `src/domain/capabilitySurfaceV2.ts`.

## Fail-closed visibility

| Surface | Visibility rule |
|---|---|
| Standard | `evidence_state=release_qualified` and either full coverage or partial coverage with a nonempty documented QuickPLS scope |
| Experimental Labs | Labs preference enabled, coverage not absent/excluded, and at least engine evidence |
| Legacy | Never shown in normal Calculate; historical archives remain readable |
| Internal | Never shown in customer workflows |

A `release_qualified` evidence label does not imply full SmartPLS coverage. A
partial cell can be Standard only for its exact nonempty documented QuickPLS
scope; its exclusions and partial coverage remain visible in Method Details.

Experimental Labs uses one exact warning per capability cell and application
session. The acknowledgement is intentionally session-only; the Labs preference
itself may be persisted. Each capability also requires the complete nine-part
Method Details contract before it can be exposed.

The native Calculate dialog resolves each selected method and add-on through the
exact option-cell registry bridge. PCA, GSCA, NCA, and Regression currently
enter Standard through release-qualified scoped cells; other executable partial
cells remain available only after Experimental Labs is enabled. Catalogue,
result, CLI, and documentation surfaces consume the same option-cell authority.
