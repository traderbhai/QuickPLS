# QuickPLS 3 Competitor Program

## Purpose

This program defines the evidence and delivery work required before QuickPLS can be positioned as a professional SmartPLS competitor for documented workflows. It is not a full-parity or numerical-identity claim. QuickPLS remains an independent, fully offline Windows product, and every public method claim remains bounded by its specification, accepted evidence, and known differences.

The machine-readable source of truth is [`validation/quickpls_3_competitor_catalogue.json`](../validation/quickpls_3_competitor_catalogue.json). The fail-closed validator is [`validation/quickpls_3_competitor_program.py`](../validation/quickpls_3_competitor_program.py).

## Frozen catalogue baseline

The crosswalk freezes the 45 named entries in the [official SmartPLS algorithms and techniques catalogue](https://smartpls.com/documentation/algorithms-and-techniques/) using the canonical 2026-08-12 snapshot already bound into the parity ledger and release evidence; the same list was reverified on 2026-08-13 without changing evidence identity. It preserves the catalogue order and family names. PCA appears at both position 5 under Estimation & Core Algorithm and position 45 under CB-SEM and CFA; both rows intentionally map to the same bounded QuickPLS PCA capability.

The snapshot is a comparison baseline, not a statement that QuickPLS implements every option behind each SmartPLS page. A source-page change requires a reviewed snapshot update rather than a silent edit.

## Current evidence baseline

The validated manifest currently reports:

| Status | Catalogue entries | Meaning |
| --- | ---: | --- |
| `release-qualified` | 4 | A current QuickPLS parity-ledger capability has complete scoped method and packaged acceptance evidence. |
| `native-qualified` | 18 | An accepted native QuickPLS capability exists, but a current method-scoped release evidence pair is still required. |
| `engine-preview` | 14 | Engine-only, diagnostic, or partially native implementation evidence exists, but there is no accepted matching capability in the current parity ledger. |
| `absent` | 8 | No accepted implementation is claimed. |
| `deferred` | 1 | Intentionally outside the QuickPLS 3 competitor claim gate, with a disclosed rationale. |

These counts cover catalogue entries, not unique QuickPLS capabilities. Shared capabilities explain why 45 catalogue rows map to a 17-feature QuickPLS parity ledger. The validator derives accepted status only from the parity evaluator's current evidence-backed `derived_state`; raw ledger labels, method labels, source files, or old audit artifacts cannot promote a row. A frozen per-method mapping table makes every intentional one-to-many and many-to-one mapping reviewable and rejects capability borrowing.

`competitor_ready` is currently `false` by design.

Final readiness also requires [`validation/results/quickpls_3_competitor_approval.json`](../validation/results/quickpls_3_competitor_approval.json). That file is intentionally absent until final approval. Absence is a pending gate, not a malformed roadmap; a present but stale or incomplete envelope fails validation.

## Gap inventory

### Engine or partial workflow requiring promotion

- Blindfolding/Q-squared assessment coverage
- CTA-PLS
- HTMT as an independently tracked catalogue capability
- Bounded model-fit metrics
- PLS-POS
- FIMIX-PLS
- Moderation
- Mediation
- Nonlinear relationships
- Higher-order models
- Gaussian-copula endogeneity diagnostics
- CB-SEM bootstrapping
- CB-SEM multigroup analysis
- CB-SEM measurement invariance

The `engine-preview` label is deliberately conservative. It does not authorize a public native or release-qualified claim.

### Absent workflows to implement

- PLS-SEM sample-size and power analysis
- Consistent bootstrapping for PLSc
- Consistent permutation for PLSc
- PLS model comparison
- Prediction-oriented model selection
- Consistent MGA
- CB-SEM model comparison
- CB-SEM moderator analysis

### Intentionally deferred

- PLS Goodness of Fit (GoF) remains outside the 3.0 claim gate because the official catalogue calls it legacy and advises against relying on it. QuickPLS should disclose this decision rather than implementing a low-value method merely for list parity.

## Release train

| Release | Program outcome |
| --- | --- |
| `2.47.0` | Establish signed distribution, clean install/upgrade/uninstall certification, and release operations. |
| `2.48.0` | Promote accepted native methods and bounded assessment capabilities with method-scoped release evidence. |
| `2.49.0` | Complete diagnostic and extended-relationship native workflows. |
| `2.50.0` | Complete PLS-POS and FIMIX-PLS native segmentation workflows. |
| `2.51.0` | Add power analysis, consistent bootstrap, consistent permutation, and consistent MGA. |
| `2.52.0` | Add model comparison/selection and advanced CB-SEM workflows. |
| `3.0.0-beta.1` | Run a signed external competitor beta after operational policies and independent scientific review are in place. |
| `3.0.0` | Ship only after the method and non-method competitor claim gates pass. |
| `post-3.0` | Reassess explicitly deferred legacy or lower-value scope. |

Dependencies in the manifest are executable planning constraints. A method cannot target an earlier release than one of its dependencies, and cycles or unknown dependencies fail validation.

## Method promotion definition of done

Each competitive-scope catalogue row must become `release-qualified` through a bounded vertical slice:

1. Freeze equations, defaults, preprocessing, output definitions, supported shapes, rejected shapes, and known differences.
2. Validate deterministic calculations against independent equations and a second source where feasible.
3. Add formula, property, metamorphic, boundary, failure, and simulation tests appropriate to the method.
4. Prove seed and worker invariance for stochastic procedures.
5. Bind desktop and CLI execution to the same typed recipe, data fingerprint, settings, seed, and method version.
6. Complete native setup, applicability guidance, progress, cancellation, recovery, results, warnings, and accessible interpretation.
7. Verify displayed values and CSV/XLSX/HTML/SVG outputs against the same completed run.
8. Persist strict provenance and prove explicit save, close, reopen, and tamper rejection.
9. Execute the exact packaged Windows application and retain method-scoped acceptance evidence.
10. Promote the parity ledger only from a current scoped method audit and packaged acceptance report carrying the exact capability ID, method version, and catalogue snapshot identity.

Unsupported variants must remain blocked, hidden, or clearly disclosed. A descriptive comparison screen, source-code path, or historical passing report is not enough to promote a method.

## Competitor claim gate

The competitor claim is allowed only when:

- all 44 non-deferred catalogue entries are `release-qualified` for their documented QuickPLS scope;
- the fail-closed commercial-readiness contract independently derives `release_ready: true`, including Authenticode signing and timestamping;
- clean install, upgrade, recovery, repair, and uninstall are certified on supported Windows configurations;
- public support, security, vulnerability-response, update, known-issues, and rollback policies operate;
- high-risk scientific method families receive independent review; and
- the external beta exit criteria pass without unresolved P0/P1 defects or reproducible data loss;
- the method-manifest factory validates successfully, every mapped capability has a manifest, and each is evidence-derived as release-qualified; and
- the catalogue, parity ledger, and every method manifest carry the exact same canonical catalogue snapshot date; and
- a final aggregate approval envelope cryptographically binds the exact catalogue, parity ledger, evidence-derived parity report, commercial-readiness contract and report, complete method-manifest file set, and method-manifest report.

### Aggregate approval envelope

The final approval envelope is deliberately separate from all inputs it hashes, preventing circular digests. Its seven closed bindings are:

1. `competitor_catalogue`: exact repository-relative catalogue path and SHA-256.
2. `parity_ledger`: exact repository-relative ledger path and SHA-256.
3. `parity_report`: canonical evidence-derived report SHA-256 plus parity-validator path and SHA-256.
4. `commercial_readiness_contract`: exact contract path and SHA-256.
5. `commercial_readiness_report`: canonical derived report SHA-256 plus commercial-validator path and SHA-256.
6. `method_manifest_set`: every sorted `validation/methods/*.manifest.json` path and SHA-256 plus a file-set digest.
7. `method_manifest_report`: canonical factory-report SHA-256 plus factory-validator and schema paths and SHA-256 values.

Repository-local absolute paths in derived reports are normalized to repository-relative POSIX paths before canonical JSON hashing, so the approval is portable across clean workspaces while still binding the same repository inputs. The assembly timestamp must not predate catalogue reverification, any method-manifest freeze timestamp, or final commercial release approval. The separate final approval timestamp must strictly postdate digest assembly. Future timestamps are rejected.

Any later catalogue, parity evidence, readiness contract, validator, schema, method-manifest file, or evidence-derived report change invalidates the envelope and requires fresh assembly and approval. Merely editing a completion flag cannot satisfy this gate.

This gate supports the bounded statement that QuickPLS is a professional independent competitor for its documented workflows. It does not support "identical to SmartPLS," "complete replacement," or unrestricted full-parity language.

## Validation and change control

Run the focused checks from the repository root:

```powershell
python validation/quickpls_3_competitor_program.py
python -m unittest validation.test_quickpls_3_competitor_program -v
```

The validator exits zero when the roadmap is internally valid even though planned gaps keep `competitor_ready` false. Production validation invokes the evidence-backed parity evaluator, commercial-readiness validator, and method-manifest factory on their actual repository paths. It fails closed on:

- catalogue omissions, duplicates, reordering, or renamed frozen entries;
- unknown statuses, priorities, or release targets;
- missing, cyclic, or release-order-invalid dependencies;
- missing repository evidence for an `engine-preview` row;
- capability mappings that differ from the frozen per-method mapping or contradict the parity evaluator's evidence-derived state;
- accepted parity capabilities omitted from the crosswalk; or
- invalid commercial-readiness or method-manifest reports, missing manifest coverage, snapshot drift, or invalid competitor claim-gate references.
- a present aggregate approval with missing bindings, digest drift, unsafe timing, an unapproved commercial decision, or any bound input/report mismatch.

To update the frozen vendor baseline, review the official catalogue, change the snapshot and closed expected catalogue together, explain additions/removals in this document, and add mutation tests. To promote a QuickPLS row, first promote its capability through the parity evidence process; changing only this roadmap must fail validation.
