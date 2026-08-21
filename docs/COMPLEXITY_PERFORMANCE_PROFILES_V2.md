# Complexity and performance profiles V2

This contract turns the product-finalization performance requirements into a
machine-readable, fail-closed qualification boundary. It complements
`QualificationSpecV2`: method specifications still define method-specific
scientific workloads, while this contract supplies the shared hardware,
complexity, UI, runtime, memory, progress, cancellation, and regression rules.

No benchmark result is bundled with the contract. The checked-in manifest is
deliberately marked `targets_only_no_measurements`, and a structure-only check
does not qualify performance or product finalization.

## Contract files

- `validation/capabilities/complexity_performance_profiles_v2.manifest.json`
  defines targets and resolves a budget class for every active Registry V2
  option cell.
- `validation/capabilities/complexity_performance_profiles_v2.schema.json`
  defines the strict target-document shape.
- `validation/capabilities/complexity_performance_measurement_v2.schema.json`
  defines immutable capability and UI measurement receipts.
- `validation/complexity_performance_v2.py` validates the contract and supplied
  receipts.
- `validation/complexity_performance_measure.py` is a lightweight command
  harness. It never chooses or starts a maximum-axis or compound workload by
  itself.

The current Registry V2 binding contains 43 active catalogue rows and 57 active
option cells. Each cell resolves to 10 profile budgets: five profiles on each
of the Standard and Workstation hardware classes, for 570 resolved budgets.
Because the maximum-axis profile contains four separate cases, a complete
current evidence set contains 912 capability receipts plus eight UI receipts.
The four axes are measured separately; the contract does not imply one model
that combines every maximum at once.

## Required gates

The Standard profile is Windows 11 x64 with at least six physical cores and
16 GiB RAM; its qualified process-tree working set may not exceed 12 GiB. The
Workstation profile requires at least 12 physical cores and 32 GiB RAM, with a
24 GiB limit.

Every measured case uses one warm-up and five measured runs. The validator
recomputes Type-7 median and p95 values, checks the absolute per-capability
budget, and compares current results with an explicitly supplied accepted
baseline on the same hardware fingerprint and workload fingerprint. A contract
or workload change therefore requires a newly accepted baseline. Runtime or
memory growth greater than 20 percent fails. A failed run, missing result
artifact, incomplete process-tree measurement, or orphan process also fails.

Operations longer than two seconds must report real, monotonic progress.
Potentially long operations require a cancellation observation showing a
terminal `cancelled` state within one second, no partial visible or committed
result, and an unchanged archive. The applied profile additionally requires at
least 10 accepted repeated runs, at most five percent settled working-set
growth, and no orphan process.

The UI receipt matrix covers both hardware classes for the applied-diagram,
stress-diagram, typical-preflight, and stress-preflight scenarios. Its targets
are the product-finalization budgets: 2/10-second open, 100/250-ms edit p95,
45/30 FPS, and 0.5/3-second preflight respectively.

## Commands

Validate only the target document and Registry binding:

```powershell
npm run qpls:product:performance-contract
```

This command reports `measurements_verified: false` and
`product_finalization_performance_passed: false` by design. Validate a
complete set of accepted-baseline and current receipt files with:

```powershell
python validation/complexity_performance_v2.py <receipt> [...]
```

Calling the validator without receipts must fail and report the missing
capability and UI matrix. That is the qualification command exposed as:

```powershell
npm run qpls:product:performance-qualification
```

To create one capability receipt, select a Registry V2 option cell, hardware
profile, profile/case, and an explicit workload command:

```powershell
python validation/complexity_performance_measure.py `
  --capability-id <capability-id> `
  --cell-id <cell-id> `
  --capability-version <version> `
  --hardware-profile standard_windows_6c16g `
  --physical-cores 6 `
  --profile applied --case applied `
  --measurement-role accepted_baseline `
  --build-fingerprint <build-sha256> `
  --workload-fingerprint <workload-sha256> `
  --result-path <new-result-path> `
  --observations <observations.json> `
  --output <new-receipt-path> `
  -- <explicit-workload-command>
```

The workload receives `QPLS_PERFORMANCE_PHASE`,
`QPLS_PERFORMANCE_RUN_INDEX`, `QPLS_PERFORMANCE_PROGRESS_PATH`, and
`QPLS_PERFORMANCE_RESULT_PATH`. It must write a new result to the supplied
result path. For progress, append JSON lines such as `{"progress": 0.25}` to
the supplied progress path. The harness will not overwrite an existing receipt
or an existing first-run result.

For potentially long profiles, the observations JSON supplies
`cancellation_observation`. For the applied repeat-memory gate, it also supplies
`memory_growth_observation`; their exact typed fields are defined by the
measurement schema. Current receipts must name the accepted-baseline receipt
through `--baseline`. A receipt remains incomplete when any required external
observation is absent.

Maximum-axis and compound-stress runs belong to the scheduled performance
qualification lane. They are not part of the lightweight change gate, and this
contract must not be used to infer that those workloads have been measured.
