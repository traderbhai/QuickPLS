# MICOM v3.1

Status: exact internal work implementation; not qualification-ready and not available in Standard Calculate. The Capability Registry cell remains `coverage_state=absent`, `evidence_state=absent`, `surface=labs`. No manifest or promotion state is changed by this implementation.

QuickPLS now has a distinct `method_config.kind=micom` execution contract with method identity `micom_v3_1`. It does not run structural-path permutation MGA or consistent permutation. The former combined `micom_v3` plus `pls_mga_permutation_v3` path remains readable only as historical output; it is not evidence for v3.1.

## Three-step contract

- Step 1 is an explicit researcher review of equivalent indicator meaning, coding, data treatment, model specification, and algorithm settings across the selected groups. QuickPLS records the confirmation and `step1_computed=false`; it does not infer configural invariance from the data.
- Step 2 estimates group-specific composite weights, aligns their orientation to the pooled-data reference, and compares the correlation of the resulting pooled scores with the lower tail of a deterministic permutation distribution.
- Step 3 uses pooled-model composite scores. The obtained Group A minus Group B mean difference and log variance ratio are compared with their permutation intervals. Following the current SmartPLS correction to the original article wording, equality is decided by whether the obtained difference lies within the interval, not by a generic test that the interval contains zero.
- Partial invariance requires Steps 1 and 2. Full invariance additionally requires equality of both means and variances in Step 3.

## Exact resampling and accounting

- The selected groups retain their observed sizes in every permutation.
- Stable case identities, the seed, and the replicate index determine each partition.
- Exchanging Group A and Group B yields the coupled complementary partition and reverses signed statistics without changing two-sided decisions.
- Every requested replicate is attempted exactly once. There are no replacement retries.
- Step 2 and Step 3 have separate usable and failed counts because Step 3 uses the pooled scores and can remain usable when a group refit needed by Step 2 fails.
- Every requested replicate has a ledger row with its index, partition hash, preserved group sizes, Step 2/3 status, and typed failure code.

The bounded internal setup currently requires two selected groups, at least 10 complete model cases per group, a maximum 10:1 complete-case size ratio, path weighting, standardized preprocessing, listwise deletion, no case weights, and 5,000–10,000 requested attempts. Interactions and higher-order constructs are blocked. These bounds are implementation limits, not general scientific MICOM rules.

## Product boundaries

The exact configuration flows through recipe validation, estimation, runner provenance, project schema-6 validation, CLI/native serialization, result tables, and archive reopening. New v3.1 results require:

- method version `micom_v3_1`;
- MICOM-only payloads (`mga=null`, `mga_permutation=null`);
- explicit Step 1 review provenance;
- attempted count equal to the requested count;
- `retry_policy=none`;
- complete, sequential ledger accounting;
- coherent Step 2/3 counts and hierarchical construct decisions.

Historical v1, v2, and combined v3 results remain readable under their original identities. They are not silently recalculated or reinterpreted as v3.1.

## Verification status

The transparent independent NumPy work oracle is `validation/micom_v3_oracle.py`. It covers deterministic size-preserving partitions, pooled-reference orientation, corrected Step 3 decisions, exact no-retry accounting, group-swap/reorder/seed metamorphics, and typed empty/small/imbalanced/degenerate failures. This is source work, not an immutable qualification receipt.

Focused product checks cover the wire contract, semantic validation, deterministic partition plan, runner version selection, schema-6 append/reopen validation, native preflight, exact recipe construction, request parsing, result-ledger validation, and customer wording. The full 5,000-attempt runner test is deliberately marked qualification-scale rather than placed in the fast unit lane.

Remaining qualification blockers include:

- no frozen current-product result compared with the NumPy oracle under the same canonical partition plan;
- no second independently maintained computational MICOM implementation;
- no qualification-sized 5,000/10,000 simulation, calibration, power, and failure-rate campaign;
- Mode B/mixed composites, controls, interactions, higher-order constructs, broader missing-data policies, arbitrary declared groups, and pairwise orchestration remain unqualified;
- archive/export, packaged Windows, accessibility, performance, cancellation, soak, and independent scientific-review receipts are absent;
- the registry and legacy manifest still use one combined MICOM/permutation-MGA cell and must be split before qualification or promotion.

Source-work entry points:

```powershell
python validation/micom_v3_qualification_factory.py --verify
python validation/micom_v3_oracle.py --permutations 39
npx vitest run src/native/nativeAnalysisRecipe.test.ts src/native/nativePlsReadiness.test.ts src/native/nativeMga.test.ts src/native/nativeResults.test.ts
```

Primary and official references:

- Henseler, Ringle, and Sarstedt (2016), DOI `10.1108/IMR-09-2014-0304`.
- SmartPLS Measurement Invariance Assessment (MICOM): https://smartpls.com/documentation/algorithms-and-techniques/heterogeneity-and-multigroup/micom/
- SmartPLS Permutation: https://smartpls.com/documentation/algorithms-and-techniques/resampling-and-inference/permutation/
