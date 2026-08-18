# PLS model-fit v2 qualification status

Status date: 15 August 2026

This page records validation progress for `pls_model_fit_v2` and the bounded
`pls_model_fit_exact_v1` source lane. It is an internal qualification status,
not a product-availability or release-readiness claim.

## Current truth

| Contract surface | Current state |
|---|---|
| CapabilityRegistryV2 cell | `coverage_state=partial`, `evidence_state=absent`, `surface=labs` |
| v1 promotion manifest | `declared_state=absent`; every evidence array is empty |
| QualificationSpecV2 migration | `compatibility_only` |
| Receipts attached to QualificationSpecV2 | 0 |
| Candidate receipt descriptors emitted by the validation factory | 1: `method_contract` |
| Overall qualification | Not ready |
| Promotion | Not allowed |

The method-contract candidate is deliberately not attached to the
QualificationSpec. Attachment remains a separate reviewed action and cannot
make the method qualified while any required role is absent.

## Required receipt roles

| Role | Factory status | Evidence and exact blocker |
|---|---|---|
| `method_contract` | Candidate receipt emitted | Frozen spec, registry link, manifest state, source descriptors, and contract sections pass. |
| `kernel_execution` | Blocked | Product source and test files exist, but there is no immutable, build-bound product execution envelope covering point fit, separate exact-fit refits, Type-7 bounds, ledgers, PLS-PM/PLSc breadth, seeds, workers, and typed failures. |
| `oracle_independence` | Work evidence only | The transparent NumPy/SciPy oracle independently refits a bounded recursive PLS-PM model and matches the frozen product point fixture. Full PLSc and advanced supported-shape breadth plus a second independent implementation or approved exception are missing. |
| `generative_recovery` | Work evidence only | The deterministic harness runs 96 point-recovery samples and 12 exact-fit datasets per condition with two distributions. Exact-fit work uses only 19 draws; the maximum Wilson half-width is about 0.246, not the required 0.01, and product/worker/breadth matrices are unrun. |
| `adversarial_boundaries` | Work evidence only | Fourteen independent-oracle boundary and metamorphic cases pass. Matching product, archive, native, worker, GUI/CLI, and supported-shape executions are missing. |
| `archive_persistence` | Blocked | No immutable real-runner append, save, close, reopen, cancellation, recovery, legacy/future-read, and tamper-rejection execution envelope exists. |
| `cross_format_export` | Blocked | No same-run semantic readback envelope exists for CSV, XLSX, HTML, SVG, PDF, and PNG identity. |
| `frontend_contract` | Blocked | Test source presence is not execution evidence. No build-bound setup, result linkage, accessibility, Labs-warning, or cancellation execution envelope exists. |
| `packaged_windows_e2e` | Blocked | Installed/portable offline, viewport, scaling, keyboard, pointer, cancellation, save/reopen, export, and clean-exit matrices have not been captured. |
| `performance_scale` | Blocked | Applied, large, maximum-axis, compound-stress, cancellation, memory, regression, and soak profiles have not been executed on the declared hardware classes. |

Independent scientific review remains an additional red finalization gate even
though it is not one of the ten receipt roles in the current QualificationSpec.

## Validation-only evidence added

The independent full-refit oracle computes observed, saturated-implied, and
estimated-implied correlation matrices and independently evaluates SRMR,
d_ULS, natural-log d_G, ML-function chi-square, degrees of freedom, and NFI. It
also performs two domain-separated adapted Bollen-Stine null transformations,
indexed case resampling, full PLS-PM refits, Type-7 HI95/HI99 summaries, fixed
requested/usable/failure ledgers, seed/index identity checks, row/column
permutation checks, and typed singular/non-positive-definite failures.

The generative and adversarial reports are intentionally marked:

```text
qualification_role_satisfied=false
receipt_eligible=false
```

Passing these small work checks means the validation machinery is functioning;
it does not mean Type-I error, power, coverage, failure rate, breadth, packaged
behavior, or performance has met the qualification contract.

## Fail-closed factory behavior

The factory reads exact source descriptors and hashes, the frozen scenario
contract hash, registry and manifest states, and optional immutable execution
envelopes. A non-contract role is admitted only when its envelope has:

- the exact role and stage identity;
- the current complete source set and scenario hashes;
- a non-validation product build fingerprint;
- the exact required check IDs, all passing;
- successful timestamped command proof;
- hash-current output artifacts; and
- the required hardware fingerprint.

Test-source presence, a console statement that a test passed, the small
independent work reports, or a stale artifact cannot produce a product,
archive, native, packaged, or performance receipt.

The factory never edits the QualificationSpec, CapabilityRegistryV2, or v1
promotion manifest.

## Reproduction

```powershell
python validation\pls_model_fit_full_refit_oracle.py --draws 12
python validation\pls_model_fit_v2_qualification_evidence.py --write
python validation\pls_model_fit_v2_qualification_factory.py --write
python validation\pls_model_fit_v2_qualification_factory.py --verify
python -m pytest validation\test_pls_model_fit_v2_reference.py validation\test_pls_model_fit_exact_v1_reference.py validation\test_pls_model_fit_full_refit_oracle.py validation\test_pls_model_fit_v2_qualification_evidence.py validation\test_pls_model_fit_exact_v1_qualification.py validation\test_pls_model_fit_v2_qualification_factory.py -q
```

The checked-in factory audit is
`validation/results/method_factory/pls_model_fit_v2/qualification_factory_audit.json`.
Any source-set change invalidates it until the relevant work artifacts are
rerun and the factory is regenerated.
