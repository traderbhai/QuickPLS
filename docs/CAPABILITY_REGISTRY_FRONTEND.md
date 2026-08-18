# Capability Registry V2 frontend adapter

`src/domain/capabilityRegistryV2.ts` is the only frontend adapter for the frozen
`validation/capabilities/capability_registry_v2.json` source of truth. It parses
the registry at application-module load and fails closed before any capability
can be advertised.

The adapter enforces the 45-row/43-active catalogue contract and parses 48
authoritative option cells. Every cell independently owns coverage, evidence,
surface, predicates, schemas, differences, and one exact four-field
qualification link. Retained row state is checked as a conservative legacy
projection and is never used to resolve a requested calculation option.
Standard requires `surface=standard`, `evidence_state=release_qualified`, and
either `coverage_state=full` or `coverage_state=partial` with a nonempty
documented QuickPLS scope. The latter is Supported for that exact scope without
claiming complete SmartPLS breadth.

Use `capabilityRegistryV2.capability(id)` for official row metadata,
`capabilityRegistryV2.quickPlsCell(cellId)` for mappings, and
`requireOptionCell(capabilityId, cellId)` for an exact option. Cell lookups
return an array because one QuickPLS implementation can legitimately cover
multiple official catalogue positions (for example, PCA). Product and
preflight code must call the three-argument exact-cell `availability()` API;
`rowAvailability()` exists only for conservative catalogue/reporting views.

Customer UI code must consume `productProjection()` or
`visibleProductCapabilities()`. These projections deliberately omit evidence
states, qualification references, manifest paths, legacy governance fields,
and internal availability reasons. Internal setup and preflight code may use
`availability()`, which delegates to the shared `capabilityAvailabilityV2`
fail-closed Standard/Labs policy.

The native Calculate catalogue makes one deliberate product distinction:
Registry V2 remains authoritative for Standard qualification and disclosure,
while Experimental Labs retains the 14 established native workflows that were
already implemented and accepted before option-level promotion was introduced.
Those workflows remain selectable and show their bounded-scope guidance;
method-specific readiness and runtime validation still reject invalid setups.
An evidence-state downgrade must not silently delete an implemented workflow
from the Labs catalogue.

The optional `requireFrozenStateDistribution: false` parser setting exists only
for testing coverage-state revisions. Surface counts evolve as methods are
promoted but must always equal the registry's declared count block. The setting
does not relax schema, enum, identity, count-consistency, or Standard-gate
validation.

## Embedded desktop inspection

The read-only Tauri command `capability_registry_v2` returns the exact registry
JSON embedded in the desktop executable, its SHA-256 digest, identity, and
row/option-cell counts. `getNativeCapabilityRegistryV2()` verifies the digest,
parses the source through this adapter, and requires the executable rows and
option cells to match the frontend registry exactly. The native workbench runs
that check at startup and keeps Calculate fail-closed while verification is
pending or if the installed executable and frontend disagree. Browser preview
continues to use the strictly parsed bundled registry because no native command
exists there.
