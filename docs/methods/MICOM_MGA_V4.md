# Combined MICOM and Two-Group Permutation MGA v4

Status: Supported scoped Standard workflow. Current execution identities are
`pls_mga_two_group_v4`, `pls_mga_permutation_v4`, and `micom_v4`. The recipe
schema remains version 3. Historical v1-v3 combined results remain readable
under their original identities but are not appendable or relabeled as v4.

## Supported scientific scope

The workflow compares exactly two explicitly ordered groups using the same reflective PLS model,
path weighting, standardized data, listwise deletion, and a researcher-confirmed MICOM Step 1.
It reports group path coefficients, outer loadings, outer weights, MICOM Steps 1–3, two-tailed
permutation comparisons, and complete fixed-plan accounting.

The requested 5,000–10,000 partitions are preplanned from the seed. Every partition preserves the
two group sizes and is executed exactly once. Failed fits remain in the ledger with a typed reason;
they are never replaced. MICOM Steps 2–3 and MGA path/loading/weight comparisons share the same
partition identity. The result records requested, attempted, usable, and failed counts, every
indexed status, every partition digest, and a digest of the complete plan.

Changing Group A and Group B reverses signed differences while preserving the partition plan,
two-tailed probabilities, failure accounting, and MICOM decisions. Changing worker count does not
change the analytical payload for the same recipe and seed.

## Persistence and compatibility

Only the three v4 identities form appendable combined MGA evidence. Project validation binds the
recipe and result identity, ordered groups, direction, counts, fixed ledger, recomputed plan digest,
probabilities, MICOM decisions, and dataset fingerprint before appending anything. A failed check is
atomic: neither the recipe nor result is added.

Historical combined v1, v2, and v3 payloads remain readable under their original identities. They
cannot be appended as new evidence and are never relabeled as v4. Standalone `micom_v3_1` remains a
separate MICOM-only contract.

## Exclusions

More than two simultaneous groups, case weights, interactions, higher-order constructs, consistent
PLS, parametric or bootstrap MGA, one-tailed MGA, omnibus tests, and multiplicity adjustment are
outside this workflow. MICOM does not establish semantic equivalence, validity, causality, or
representativeness. Interpret group differences only where the relevant invariance assumptions are
adequately supported.

## Qualification commands

```powershell
python validation/micom_mga_v4_reference.py --permutations 39
python validation/micom_mga_v4_reference.py --permutations 5000 --run-quickpls
cargo test -p qpls-estimation micom_and_permutation_mga_v4_emit_one_fixed_swap_coupled_contract
cargo test -p qpls-project validated_append_and_archive_round_trip_preserve_exact_combined_mga_v4_contract
powershell -NoProfile -ExecutionPolicy Bypass -File validation/run_v247_mga_native_acceptance.ps1
```
