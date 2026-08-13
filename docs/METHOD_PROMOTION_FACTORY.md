# QuickPLS Method-Promotion Factory

This factory makes every scientific capability earn its product claim from the
same fail-closed evidence contract. It supplements the QuickPLS 3 parity ledger;
it does not replace that ledger or independently make a marketing claim.

The strict command, `python validation/method_promotion_manifest.py`, is the
only factory validator that derives qualification from current hash-bound
evidence. It deliberately fails when local release artifacts are missing or
changed. Clean-checkout CI uses
`python validation/method_promotion_contracts.py` to validate only the closed
JSON, schema, and semantic contracts. That portable command explicitly reports
`evidence_verified: false` and `claim_authorized: false`; it cannot promote a
method or authorize parity, release, or competitor claims. A trusted release
evidence job must restore the exact digest-bound artifact bundle before running
the strict validator. CI also runs
`python validation/quickpls_3_competitor_contracts.py` plus the portable
factory and catalogue mutation runners. Those runners exercise the strict
contract rules with non-claiming fixtures; they never substitute for the local
materialized-evidence gate.

## Manifest contract

Each capability owns one `validation/methods/*.manifest.json` document validated
against `validation/methods/method_promotion_manifest.schema.json` by a
self-contained Python standard-library validator. A manifest freezes:

- a stable `qpls3.*` feature ID, immutable method version, method kind, official
  catalogue snapshot date, and source URL;
- the bounded claim, estimand, assumptions, exclusions, and mandatory warnings;
- traceable equations and at least two genuinely independent reference groups,
  including a primary paper and a computational reference;
- preregistered simulations with designs, seeds, metrics, and acceptance rules;
- data-pathology, unsupported-scope, metamorphic, determinism, and tamper
  boundaries;
- recipe/result versions, payload identity, legacy policy, and all six archive
  tamper classes;
- native setup, applicability blockers, result surfaces, GUI/CLI parity, and
  accessibility;
- same-run export tables and provenance;
- installed Windows workflow, the three supported acceptance viewports, offline
  execution, cancellation for stochastic methods, and process cleanup; and
- current audit reports with exact identity fields, source hashes, and freshness.

JSON is read strictly. Duplicate keys, `NaN`/`Infinity`, unknown fields, unsafe
repository paths, malformed JSON pointers, duplicate scientific IDs, repeated
stage roles, incomplete boundary categories, and incomplete tamper coverage fail
validation.

## Evidence ladder

Promotion remains sequential:

```text
absent -> engine_only -> archive_qualified -> native_qualified -> release_qualified
```

| State | Required evidence roles |
| --- | --- |
| `absent` | A complete planned contract is allowed with no implementation evidence. |
| `engine_only` | Method specification, independent reference, simulation report, and boundary report. |
| `archive_qualified` | All engine evidence plus strict persistence/tamper evidence. |
| `native_qualified` | All archive evidence plus native frontend and export evidence. |
| `release_qualified` | All native evidence plus separate current method-audit and packaged-acceptance reports. |

Every evidence artifact must be a strict JSON identity report. Existence-only
checks are prohibited, including for the method specification. A report may
cover several roles in its own stage, but every required role must appear
exactly once. Every report binds `passed=true`, feature ID, method version, and
catalogue snapshot date.

Each report also exposes a `source_artifacts` array. Every descriptor has exactly
`path`, byte `size`, and lowercase `sha256`. The validator resolves each path
inside the repository and rehashes its exact bytes. Every report must bind:

- its capability manifest;
- the shared manifest schema;
- the manifest validator;
- the focused mutation test; and
- every role-specific source path declared by the manifest.

The report timestamp must be at or after the manifest's contract-freeze time.
This makes a report stale whenever its governing contract is refrozen, while
hashes protect content independently of checkout filesystem timestamps. A
method specification is therefore proven only through a current report that
hash-binds it; the Markdown file itself is never evidence.

Later-stage evidence cannot compensate for a failed or missing earlier stage.
The validator derives the highest state on disk and rejects a higher declared
state.

The repository currently carries 40 method contracts. Nine derive
`release_qualified`: PLS Algorithm, WPLS, PLSc, PLS Bootstrapping, PCA, CTA-PLS,
Structural Path Randomization, Binary Logistic Regression, and Regression
Bootstrapping. Graph-defined PROCESS v2 derives `native_qualified`; the other 30
contracts remain `absent`. These states come from the strict evidence validator,
not from this inventory paragraph.

`pls_sample_size_power_v1.manifest.json` is one example of a complete future
contract that truthfully remains `absent` until its implementation evidence is
produced. Planned contracts retain exact source requirements and empty evidence
ladders; they cannot inherit qualification from legacy reports or UI labels.

The existing parity ledger retains its own qualification state and validation
rules. Factory promotion requires newly generated, factory-bound reports; it
does not silently revoke or inherit the separate ledger state.

## Adding or promoting a method

1. Copy the planned manifest pattern and freeze the claim before implementing.
2. Give the capability its own feature ID and method version; do not share
   qualification merely because two capabilities use one launcher.
3. Implement the engine and generate the required reports one stage at a time.
4. Use independent equations and fixtures. External R/GPL tools remain
   development-time validation only and are never bundled or called at runtime.
5. Add exact failure, metamorphic, deterministic, persistence, and tamper tests.
6. Complete the native setup-to-results flow and verify accessible tables,
   same-run exports, explicit save, close, and same-run reopen.
7. Run acceptance against the packaged offline Windows binary at all declared
   viewports; include cancellation/retry for stochastic methods.
8. Generate every report after the current contract freeze. Include exact
   identity fields and exact source descriptors for all governance and
   role-specific sources.
9. Add distinct method-audit and packaged-acceptance reports; neither may reuse
   the other role.
10. Advance `declared_state` only after the CLI derives that state.
11. Update the parity ledger and public compatibility documentation separately.

## Commands

Validate every manifest and its current evidence:

```powershell
python validation\method_promotion_manifest.py
```

Inspect the full evaluation report:

```powershell
python validation\method_promotion_manifest.py --json
```

Run the fail-closed mutation tests:

```powershell
python -m unittest validation.test_method_promotion_manifest -v
```

The CLI exits nonzero for schema, semantic, evidence, identity, state, or
cross-manifest feature-ID contradictions. A planned `absent` method exits zero
when its contract is complete and it makes no unsupported qualification claim.

## Promotion gate

A method is a release **GO** only when the manifest derives
`release_qualified`, the existing parity ledger independently passes, and the
app, exports, documentation, compatibility matrix, and public wording describe
the same bounded scope. Any unexplained deterministic difference above `1e-6`,
failed simulation acceptance rule, missing independent reference, stochastic
nondeterminism, silent legacy reinterpretation, invalid archive acceptance,
dev-only packaged evidence, or broader product wording is a **NO-GO**.

## Prioritized batches and effort

Effort bands assume one experienced product/statistical engineer using the
existing QuickPLS infrastructure: **S** 2-3 person-weeks, **M** 4-7, **L** 8-12,
and **XL** 13-20. They exclude external academic review and installer/signing
work.

| Priority | Batch | Typical effort |
| --- | --- | --- |
| 0 | Give every currently native-qualified marketed capability a dedicated current method audit and packaged acceptance report. | 10-18 person-weeks total |
| 1 | Promote bounded CTA-PLS, Gaussian-copula, nonlinear, moderated-mediation, and higher-order native workflows. | M to L each |
| 2 | Promote deterministic PLS-POS, then bounded FIMIX-PLS. | L, then XL |
| 3 | Build consistent bootstrapping, consistent permutation, then consistent MGA on their qualified foundations. | L, L, then XL |
| 4 | Build sample-size/power, saved-model comparison, prediction-oriented selection, and saved-model CVPAT. | L to XL each |
| 5 | Build CB-SEM bootstrap, multigroup/invariance, model comparison, then moderator analysis. | L to XL each |
| 6 | Add legacy PLS GoF only if validated user demand justifies it, and label it as descriptive rather than global validity. | S to M |

High-risk stochastic or latent-class methods should receive independent
statistical review before release promotion even when the automated contract
passes.
