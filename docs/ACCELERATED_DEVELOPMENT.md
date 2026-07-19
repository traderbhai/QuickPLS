# Accelerated Development Procedure

QuickPLS now uses a method-slice factory instead of a single linear roadmap. Each slice must carry its own specification, engine implementation, validation evidence, product surface, automation, documentation, and release gate.

The machine-readable registry is:

```text
validation/development_slices.json
```

The active project goal recorded there is:

```text
Complete QuickPLS as a free, proprietary, fully offline Windows desktop application that is first ready in all sense before unrestricted research use.
```

## Daily Commands

Show the full accelerated roadmap:

```powershell
cargo run -p qpls-cli -- roadmap
```

Show only the current v0.4 work:

```powershell
cargo run -p qpls-cli -- roadmap --release v0.4
```

Show the blockers for one slice:

```powershell
cargo run -p qpls-cli -- gate v0_4_assessment_reliability
```

Run the v0.4 inference accelerator gate:

```powershell
npm run qpls:sensitivity:v04
npm run qpls:monte-carlo:v04
npm run qpls:studentized:reference
npm run qpls:studentized:monte-carlo
npm run qpls:studentized:sensitivity
npm run qpls:studentized:min
npm run qpls:studentized:workers
npm run qpls:studentized:performance
npm run qpls:bootstrap:external
npm run qpls:bootstrap:corporate-csem
npm run qpls:bootstrap:plspm
cargo run -p qpls-cli -- qualify v04-inference --refresh-quick-monte-carlo
```

This writes `validation/results/v04_inference_qualification_quick.json`. It runs the CLI worker matrix for workers `1`, `2`, and `4`, runs a bounded cancellation-latency smoke benchmark for a 10,000-replicate bootstrap, runs a bounded 999x99 cancellation smoke benchmark triggered from the nested studentized-inner phase, refreshes the quick Monte Carlo report when requested, validates the pilot, sensitivity, bounded studentized, bounded studentized-sensitivity, and full Monte Carlo reports when present, validates the supplied-value studentized Python/R Type-7 reference, validates the bounded 999x99 studentized execution artifact, validates the bounded 999x99 studentized worker matrix artifact, validates the bounded studentized performance benchmark, validates the simple-fixture cSEM and python-plspm matched-resample bootstrap reports, validates the broader corporate-reputation cSEM matched-resample bootstrap report, and checks whether the preregistered Monte Carlo qualification artifact exists. The quick/pilot/sensitivity reports, supplied-value studentized reference, bounded studentized Monte Carlo reports, matched-resample external-reference reports, bounded studentized execution, bounded studentized worker matrix, bounded performance benchmark, and cancellation smoke benchmarks are infrastructure evidence only; they do not promote inference methods to validated.

Run the full preregistered studentized qualification as resumable shards:

```powershell
npm run qpls:studentized:qualification:plan
npm run qpls:studentized:qualification:status
powershell -NoProfile -ExecutionPolicy Bypass -File validation/run_studentized_qualification_shards.ps1 --dry-run --max-shards 1
powershell -NoProfile -ExecutionPolicy Bypass -File validation/run_studentized_qualification_shards.ps1 --execute --max-shards 1
npm run qpls:studentized:qualification:aggregate -- --allow-incomplete
```

The default manifest creates 40 shards: four scenarios, 1,000 simulations per scenario, 100 simulations per shard, and 395,604,000 requested inner fits in total. The runner is safe by default: without `--execute` or `--dry-run`, it only writes status. When execution is enabled, it skips valid completed shards by default, reports invalid or failed shards, writes `validation/results/studentized_qualification_shards/status.json`, and can be limited by `--scenario`, `--name`, or `--max-shards`. Run `aggregate` without `--allow-incomplete` only after the status report says every shard is complete.

Generate the v0.4 assessment evidence traceability report:

```powershell
python validation\external_reference_probe.py
cargo run -p qpls-cli -- evidence v04-assessment
```

The probe writes `validation/results/external_reference_probe.json`; the evidence command writes `validation/results/v04_assessment_evidence.json`. Together they map every current assessment metric group to method specs, fixture files, test names, tolerances, available reference scripts, and missing external-reference blockers. This is traceability evidence only; metrics with `missing_evidence` remain experimental.

Create and validate the v0.4 demo evidence project:

```powershell
cargo run -p qpls-cli -- demo create
cargo run -p qpls-cli -- demo validate
```

The generated artifacts are:

```text
validation/demo/quickpls_v04_demo.qpls
validation/demo/quickpls_v04_demo.expected.json
validation/demo/quickpls_v04_demo.validation.json
```

The validator reruns the demo recipe and compares the canonical analytical result with exact structure matching and `1e-12` numeric tolerance. This is regression evidence for the current experimental engine, not independent publication validation.

In the native desktop app, the toolbar `Demo` action opens the same v0.4 demo evidence project with its dataset, model, fixed analysis settings, and saved run.

The npm aliases are:

```powershell
npm run qpls:roadmap
npm run qpls:gate:v04
npm run qpls:probe:external
npm run qpls:pls:csem
npm run qpls:pls:plspm
npm run qpls:pls:pca
npm run qpls:rho-a:csem
npm run qpls:evidence:v03
npm run qpls:evidence:v04
npm run qpls:pilot:v04
npm run qpls:sensitivity:v04
npm run qpls:monte-carlo:v04
npm run qpls:studentized:reference
npm run qpls:studentized:monte-carlo
npm run qpls:studentized:sensitivity
npm run qpls:studentized:qualification:plan
npm run qpls:studentized:qualification:status
npm run qpls:studentized:qualification:aggregate
npm run qpls:studentized:min
npm run qpls:studentized:workers
npm run qpls:studentized:performance
npm run qpls:bootstrap:external
npm run qpls:bootstrap:corporate-csem
npm run qpls:bootstrap:plspm
npm run qpls:qualify:v04
```

## Promotion Rules

Slice status moves only in this order:

```text
unsupported -> experimental -> validated
```

A slice cannot be `validated` while any gate is `open` or `blocked`. A slice cannot expose stable output unless it is `validated`. These rules are enforced by Rust tests through the bundled registry parser in `qpls-core`.

## Required Gates Per Method Slice

Every statistical method should be implemented as a vertical slice:

1. Freeze equations, terminology, preprocessing, defaults, convergence, warnings, citations, unsupported cases, and tolerance.
2. Implement the Rust engine function and typed result contract.
3. Add hand-calculated fixtures and boundary tests.
4. Add published-data or reference-engine fixtures.
5. Add metamorphic tests for ordering, scaling, sign, seed, and worker-count behavior where relevant.
6. Add CLI execution and serialized result equality checks.
7. Add GUI controls, result tables, warnings, and watermarks.
8. Add archive validation and tamper tests.
9. Add docs, known differences, and capability matrix entries.
10. Run full verification before promotion.

## Current Acceleration Focus

The current stage is `v0.4_partial`. The fastest useful path is:

- Keep `npm run qpls:htmt:csem` and the Ringle 2023 rounded HTMT+ fixture green, and use the recorded cSEM absolute-HTMT mismatch to guide the remaining equivalent HTMT+ reference search.
- Keep the published cSEM `threecommonfactors` PLS fixture green with `npm run qpls:pls:published`.
- Complete the `v0_4_inference_resampling` full simulation and studentized/bootstrap-t evidence.
- Create the `v0_4_demo_evidence_project` so every future release has a one-click local validation demonstration.
- Keep v0.5+ methods unsupported until the v0.4 validation factory is reliable.
