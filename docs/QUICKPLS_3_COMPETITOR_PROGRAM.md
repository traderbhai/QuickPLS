# QuickPLS 3 Competitor Program

## Purpose

This program defines the evidence and delivery work required before QuickPLS can be positioned as a professional SmartPLS competitor for documented workflows. It is not a full-parity or numerical-identity claim. QuickPLS remains an independent Windows product whose analytical workflows require no internet connection, account, or cloud service; its application/page makes no external requests. Every public method claim remains bounded by its specification, accepted evidence, and known differences. A literal fully-offline, no-telemetry, or zero-process-egress claim is separate and remains blocked by the Microsoft-managed WebView2 runtime unless an OS-enforced fixed-WebView2 containment gate passes.

The machine-readable source of truth is [`validation/quickpls_3_competitor_catalogue.json`](../validation/quickpls_3_competitor_catalogue.json). The fail-closed validator is [`validation/quickpls_3_competitor_program.py`](../validation/quickpls_3_competitor_program.py).

That JSON file is now a generated compatibility projection. Current coverage,
evidence, and surface truth comes only from Capability Registry V2. The older
parity ledger remains a validated historical input and cannot promote a current
catalogue row. Commercial-release and signing material later in this legacy
document is outside the Product Finalization Program completion contract.

## Frozen catalogue baseline

The crosswalk freezes the 45 named entries in the [official SmartPLS algorithms and techniques catalogue](https://smartpls.com/documentation/algorithms-and-techniques/) using the canonical 2026-08-12 snapshot already bound into the parity ledger and release evidence; the same list was reverified on 2026-08-13 without changing evidence identity. It preserves the catalogue order and family names. PCA appears at both position 5 under Estimation & Core Algorithm and position 45 under CB-SEM and CFA; both rows intentionally map to the same bounded QuickPLS PCA capability.

The snapshot is a comparison baseline, not a statement that QuickPLS implements every option behind each SmartPLS page. A source-page change requires a reviewed snapshot update rather than a silent edit.

## Historical pre-refresh evidence baseline

The counts in this section record the fail-closed reconciliation checkpoint before the scoped-method promotion program. They remain useful for audit chronology but are not current product availability. Current status is derived from Capability Registry V2 plus the active method manifests.

At that recorded checkpoint, the validated catalogue derived:

| Status | Catalogue entries | Meaning |
| --- | ---: | --- |
| `release-qualified` | 0 | No current catalogue row has complete source-bound release evidence. |
| `native-qualified` | 0 | No current catalogue row has complete source-bound native evidence. |
| `engine-preview` | 4 | Three rows derive archive-qualified and one derives engine-only; the schema-v1 projection intentionally groups both as an engine preview. |
| `absent` | 39 | The current exact option cell does not derive accepted executable evidence. Existing source code or historical audits cannot override this state. |
| `deferred` | 2 | Intentionally outside the QuickPLS 3 competitor claim gate, with a disclosed legacy rationale. |

These counts cover catalogue entries, not unique QuickPLS capabilities. Every one of the 43 competitor-scope rows maps to a stable option-cell identity; reviewed PCA, MICOM/MGA, PLSpredict/CVPAT, PROCESS, and CB-SEM/CFA contexts share capabilities, while the sample-size/power and Permutation rows each map two independently governed cells. Together the rows map 39 option-cell identities: 38 method-manifest capabilities plus the separate post-hoc technical sample-size cell contract. The complete 40-manifest set additionally retains the deferred Blindfolding contract and QuickPLS moderated-mediation extension.

The validator uses one current authority chain and no editable completion flags: Registry V2 option cells are checked against the live state re-derived by the exact method-manifest validator. An invalid linked manifest or a registry state above that derivation fails closed. Raw parity-ledger labels, catalogue status strings, source files, and historical audit artifacts cannot promote a row. The generated crosswalk rejects missing IDs, borrowed IDs, duplicate edges, and cross-row reuse outside the documented shared contexts. At the recorded checkpoint, no active row derived native- or release-qualified evidence.

`competitor_ready` is currently `false` by design.

The shared Permutation row is now a scoped Standard only for the coupled exactly-two-group MICOM/permutation-MGA v4 workflow and the separately qualified single-model fixed-score Structural Path Randomization v1 workflow. Broader generic permutation claims, measurement-model refits, heteroskedastic or broader non-Gaussian structural inference, multiplicity adjustment, and alternative MGA designs remain excluded.

Final readiness also requires the evidence-derived external-beta gate in [`validation/quickpls_external_beta.json`](../validation/quickpls_external_beta.json) and [`validation/results/quickpls_3_competitor_approval.json`](../validation/results/quickpls_3_competitor_approval.json). The approval file is intentionally absent until final approval. A planned beta contract is structurally valid but not beta-ready; an absent approval is a pending gate, while a present stale or incomplete envelope fails validation.

## Gap inventory

### Current Version 2.53 checkpoint

Capability Registry V2 now projects 41 scoped-Standard exact cells across 27
catalogue rows, 16 Labs cells across 16 rows, and two Legacy cells/rows. The
three additive Version 2.53 cells independently qualify exactly-one-path
mediation bootstrap and bounded true three-way moderation point/bootstrap.
Their promotion does not broaden or relabel existing mediation or moderation
identities.

### Version 2.50 historical checkpoint

At the Version 2.50 checkpoint, Capability Registry V2 projected 38
scoped-Standard exact cells across
27 catalogue rows, 16 Labs cells across 16 rows, and two Legacy cells/rows. The
Rank 0–3 General SEM cells are Standard for their documented bounded predicates:
mediation, simultaneous moderation, higher-order PLS, two-way moderated
mediation, General SEM CB-SEM ML, and recursive-SEM case bootstrap.

This is not unrestricted catalogue parity. Sixteen Labs cells still represent
separately governed incomplete areas such as broader assessment/inference,
model comparison and selection, segmentation/heterogeneity, and advanced
CB-SEM families. A Standard sibling never promotes another option on the same
catalogue row.

The exact current inventory and evidence state come from Capability Registry V2
and its live method manifests. Historical counts below or in milestone ledgers
remain chronology only and must not override that authority.

### Intentionally deferred

- Blindfolding remains outside the 3.0 claim gate as a legacy redundancy-analysis workflow superseded in the product strategy by higher-priority prediction assessment. Existing Q-squared output is not silently promoted as this official workflow.
- PLS Goodness of Fit (GoF) remains outside the 3.0 claim gate because the official catalogue calls it legacy and advises against relying on it. QuickPLS should disclose this decision rather than implementing a low-value method merely for list parity.

## Release train

| Release | Program outcome |
| --- | --- |
| `2.47.0` | Establish signed distribution, clean install/upgrade/uninstall certification, and release operations. |
| `2.48.0` | Promote accepted native methods and bounded assessment capabilities with method-scoped release evidence. |
| `2.49.0` | Complete diagnostic and extended-relationship native workflows. |
| `2.50.0` | Integrate the Rank 0–3 General SEM upgrade across PLS mediation, moderation, higher-order constructs, moderated mediation, and bounded CB-SEM workflows. |
| `2.51.0` | Unify Canvas, the 18-method Calculate catalogue, advanced parameter editing, and categorized Results while retaining existing bounded power and consistent-method cells under their recorded identities. |
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
10. Promote the exact method manifest from current scoped evidence carrying the capability ID, method version, and catalogue snapshot identity, then update the exact Registry V2 option cell only after the live registry-to-manifest cross-check passes. The historical parity ledger cannot promote the cell.

Unsupported variants must remain blocked, hidden, or clearly disclosed. A descriptive comparison screen, source-code path, or historical passing report is not enough to promote a method.

## Competitor claim gate

The competitor claim is allowed only when:

- all 43 non-deferred catalogue entries are `release-qualified` for their documented QuickPLS scope;
- the fail-closed commercial-readiness contract independently derives `release_ready: true`, including Authenticode signing and timestamping;
- clean install, upgrade, recovery, repair, and uninstall are certified on supported Windows configurations;
- public support, security, vulnerability-response, update, known-issues, and rollback policies operate;
- high-risk scientific method families receive independent review; and
- the external beta exit criteria pass without unresolved P0/P1 defects or reproducible data loss;
- the external-beta validator derives `beta_ready: true` from the frozen privacy, cohort, journey, signed-candidate, lifecycle-rerun, and independent-decision evidence;
- the method-manifest factory validates successfully, every mapped capability has a manifest, and each is evidence-derived as release-qualified; and
- the catalogue, parity ledger, and every method manifest carry the exact same canonical catalogue snapshot date; and
- a final aggregate approval envelope cryptographically binds the exact catalogue, parity ledger, evidence-derived parity report, commercial-readiness contract and report, complete method-manifest file set, and method-manifest report.

These conditions authorize only the bounded competitor statement for documented
workflows. They do not authorize a literal fully-offline, no-telemetry, or
zero-egress process-tree claim. That stronger statement requires the separate
OS-enforced fixed-WebView2 containment gate to pass; application-level browser
arguments, CSP, and a rejection proxy are insufficient by themselves.

### Aggregate approval envelope

The final approval envelope is deliberately separate from all inputs it hashes, preventing circular digests. Its nine closed bindings are:

1. `competitor_catalogue`: exact repository-relative catalogue path and SHA-256.
2. `parity_ledger`: exact repository-relative ledger path and SHA-256.
3. `parity_report`: canonical evidence-derived report SHA-256 plus parity-validator path and SHA-256.
4. `commercial_readiness_contract`: exact contract path and SHA-256.
5. `commercial_readiness_report`: canonical derived report SHA-256 plus commercial-validator path and SHA-256.
6. `external_beta_contract`: exact external-beta contract path and SHA-256.
7. `external_beta_report`: canonical evidence-derived beta report SHA-256 plus beta-validator path and SHA-256.
8. `method_manifest_set`: every sorted `validation/methods/*.manifest.json` path and SHA-256 plus a file-set digest.
9. `method_manifest_report`: canonical factory-report SHA-256 plus factory-validator and schema paths and SHA-256 values.

Repository-local absolute paths in derived reports are normalized to repository-relative POSIX paths before canonical JSON hashing, so the approval is portable across clean workspaces while still binding the same repository inputs. The assembly timestamp must not predate catalogue reverification, any method-manifest freeze timestamp, or final commercial release approval. The separate final approval timestamp must strictly postdate digest assembly. Future timestamps are rejected.

Any later catalogue, parity evidence, readiness contract, external-beta contract, validator, schema, method-manifest file, or evidence-derived report change invalidates the envelope and requires fresh assembly and approval. Aggregate assembly must postdate both the commercial release decision and external-beta approval. Merely editing a completion flag cannot satisfy this gate.

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
- a row status that is not derived from its exact validated capability evidence;
- missing, borrowed, duplicate, or unexpectedly shared capability mappings;
- a mapped status that contradicts the exact factory manifest, or for one of the established 17 capabilities, the parity evaluator's evidence-derived state;
- missing or unexpected parity capabilities, factory manifests, or auxiliary factory contracts; or
- invalid commercial-readiness, external-beta, or method-manifest reports; missing manifest coverage; snapshot drift; or invalid competitor claim-gate references.
- a present aggregate approval with missing bindings, digest drift, unsafe timing, an unapproved commercial decision, or any bound input/report mismatch.

To update the frozen vendor baseline, review the official catalogue, change the snapshot and closed expected catalogue together, explain additions/removals in this document, and add mutation tests. To promote a QuickPLS row, first promote its exact method manifest and then the exact Registry V2 option cell. Changing only this generated catalogue or the historical parity ledger must fail validation.
