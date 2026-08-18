# Established method integration contract v1

Status: phase 2, consumer-adopted. The generated TypeScript table is imported by the method-capability and canonical-table bridges. The generated Rust table is exported by `qpls-core` and used by the four established CLI arms. This governance update verifies that adoption without changing any consumer, contract, schema, or generated TS/Rust bytes.

## Boundary and authority

The contract owns only integration wiring for CCA, GSCA, IPMA, and NCA:

- runtime analysis/config/workbench/result discriminators;
- the required capability option-cell identities and base/primary roles;
- canonical result-table prefix ownership;
- established-method factory selectors and reference paths.

Existing authorities remain authoritative:

| Authority | Continues to own | Deliberately absent from generated files |
| --- | --- | --- |
| Capability Registry v2 | Coverage, evidence, customer surface, and option-cell identity links | Coverage, evidence, and surface fields |
| Method manifests | Scientific contract and qualification/promotion state | Qualification state, evidence receipts, and timestamps |
| Adopted TS/Rust consumers | Runtime fallback placement and existing error/output behavior | New scientific or qualification policy |

The generator reads only the identity subset needed to resolve each declared registry cell: registry schema version, capability ID, cell ID, capability version, and the identity manifest's feature ID/method version. A primary cell must point to the method's declared manifest. Base cells resolve their own registry-linked manifest.

## Flow

```text
established_methods_v1.json
        |
        +-- strict shape/path checks against the closed v1 schema
        +-- exact Registry v2 cell/link and manifest identity resolution
        +-- Python factory identity-field parity
        +-- strict TS method/canonical consumer-adoption checks
        +-- strict qpls-core export and four-arm Rust CLI adoption checks
        |
        +-- established_method_ownership_v1.json  (adoption_phase = 2)
        +-- establishedMethodContractsV1.ts       (adopted, byte-stable)
        +-- established_method_contracts_v1.rs   (adopted, byte-stable)
```

The TS and Rust generated tables expose the same two seams without depending on consumer types:

- find a method contract by `(analysis_method, method_config_kind)`;
- find canonical table owner options by table-ID prefix.

They contain plain self-contained structs/interfaces, constants, and lookup functions. The ownership JSON records `consumer_adopted: true`, `shadow_only: false`, all five consumer/factory sources, and all input/target paths.

Consumer invariants are deliberately narrow:

- The TS method bridge imports one generated lookup. CCA and IPMA retain contract order `base -> primary`; GSCA and NCA remain primary-only. The generated lookup is attempted once before legacy simple/switch fallbacks.
- The TS canonical bridge imports generated table-prefix ownership and returns only requirements whose option is a generated primary owner option. Unknown tables continue through the existing fallback chain.
- `qpls-core` exports the generated Rust lookup. Exactly four CLI method/config arms route through one helper. That helper explicitly evaluates `primary -> base`, preserving the CLI's established first-failure and error-byte ordering even though the contract stores base first for TS.
- Target branches cannot retain the old primary literals. Helper imports, bodies, filters, role order, arm count, and bounded unknown/mismatch fallbacks are source-shape locked.

## Determinism and write safety

- JSON rejects duplicate keys and non-finite values before semantic validation.
- A strict stdlib validator enforces the supported JSON Schema subset. The schema's canonical semantic SHA-256 is pinned so an unimplemented schema change cannot pass silently; contract objects are closed and unknown or missing keys fail.
- Repository paths must match the schema's portable character set, be normalized relative POSIX paths, remain inside the selected root, and name existing inputs.
- The output allowlist is hard-coded to exactly three targets; the contract cannot redirect writes.
- Output paths reject symlink and Windows reparse-point components before any target is written.
- The source digest is SHA-256 over canonical semantic JSON (`sort_keys`, UTF-8, no insignificant whitespace), so formatting-only edits do not change it.
- All outputs are rendered and validated before writing. Each changed target is written to a same-directory temporary file, flushed, and atomically replaced. This is per-file atomicity, not a three-file transaction.
- `--check` is the default and never writes. `--write` skips byte-identical targets and then verifies all three outputs.

## Acceptance coverage

`validation/test_established_method_contract_codegen_v1.py` provides fifteen focused acceptance tests:

1. Strict schema/contract parsing rejects duplicate keys, unknown keys, invalid IDs, non-finite values, duplicate paths, and path escapes.
2. The contract contains exactly four methods in stable sorted order.
3. Every requirement resolves exactly one Registry v2 option cell with an exact four-field identity link.
4. Every primary registry identity exactly matches its declared method manifest.
5. Generated outputs copy no registry coverage/evidence/surface fields or manifest qualification/promotion state.
6. Repeated rendering and semantic hashing are byte-deterministic.
7. Check mode is read-only and detects a deliberately mutated output.
8. Write mode uses same-directory atomic replacement, is idempotent, rejects redirected output paths, and adds only the three declared targets.
9. Contract-owned factory fields, generated base/primary ordering, and primary-only canonical ownership remain exact.
10. The TS method import, helper, contract order, fallback placement, and stale-literal removal are mutation-tested.
11. The TS canonical import, owner-option filter, fallback placement, and stale-prefix removal are mutation-tested.
12. The `qpls-core` generated-module export is exact and mutation-tested.
13. The Rust CLI generated lookup, helper body, explicit `primary -> base` role order, and filter are mutation-tested.
14. Exactly four Rust CLI arms route only through the helper; missing/extra arms, stale literals, and legacy fallback drift are mutation-tested.
15. The phase-2 ownership receipt names every consumer/input/target path, and canonical prefixes do not overlap.

Cargo-free gates:

```powershell
python -B -m unittest validation/test_established_method_contract_codegen_v1.py -v
python -B validation/established_method_contract_codegen_v1.py --check
rustfmt --edition 2021 --check crates/qpls-core/src/generated/established_method_contracts_v1.rs
```

## Phase-2 bounded file lock

The original phase-1 slice created eight files. This phase-2 governance update changes exactly four of them:

1. `validation/established_method_contract_codegen_v1.py`
2. `validation/test_established_method_contract_codegen_v1.py`
3. `validation/method_contracts/generated/established_method_ownership_v1.json`
4. `docs/ESTABLISHED_METHOD_CONTRACT_V1.md`

The contract, schema, generated TS, generated Rust, existing consumers, registry, manifests, status/results, package scripts, and Cargo files are read-only in this update. The generator asserts the adopted consumer sources but never writes them.

## Migration path and trade-offs

1. Phase 1 established the byte-deterministic shadow contract and literal parity.
2. Phase 2 adopted the generated TS method/canonical seams and Rust CLI lookup, then replaced literal parity with strict consumer-adoption verification. This is the current phase.
3. Generate the Python factory table last, after qualification harness consumers can accept a pure-data mapping. Remove its old literals only after equivalent adoption gates pass.
4. If more methods or rule kinds are added, create a versioned contract revision rather than weakening the exact four-method v1 checks.

The adoption readers are intentionally source-shape-sensitive. That brittleness turns any import, helper, role-order, branch, or fallback rewrite into an explicit contract review instead of silently accepting behavior drift. This gate does not prove runtime execution; targeted TS/Rust consumer tests remain the runtime authority. Cross-file transactional generation and automatic consumer rewrites remain deferred.
