# Rank 3 CB-SEM V3 authority foundation

Status: contract/compiler foundation plus an internal source-only point
adapter. The two cells below are candidate identities and are not
Registry-executable or release-qualified yet.

## Frozen candidate identities

- Point: `qpls3.cbsem.general_sem_ml / cbsem_general_sem_ml_v1`
- Recursive bootstrap: `qpls3.cbsem.bootstrap.recursive_sem / cbsem_exact_recursive_sem_case_bootstrap_v1`

`RecipeV4CompilerTarget::CbsemPlanV3` compiles the active resolved
`SemModelV4`; it does not reconstruct a legacy canvas model. Its
`CompiledCbsemPlanV3` embeds the proven `CompiledCbsemPlanV2` optimizer plan
unchanged and binds the complete parameter-table authority, model scientific
digest, data-binding digest, `GeneralSemConfigV1` digest, V2-plan digest,
recursive topology, identification evidence, and exact candidate-cell set.

The compiled v1 predicate is deliberately bounded to unweighted raw
continuous listwise single-group reflective CFA or recursive SEM, ordinary ML,
no mean structure, no feedback, and no derived effect requests. Parameter-row
fixed/free status, finite row bounds, and equality labels are preserved.
Unsupported explicit constraint objects block compilation and are never
discarded.

The internal point adapter deterministically recompiles the V2 artifact,
requires it to equal the V3 embedded base plan and bound model/recipe/dataset
identities, invokes the existing qualified V2 point kernel once, and maps its
stable parameter IDs to the additive canonical CB parameter, fit, and
provisional-identification rows. It is a private runner module with no public
re-export, Registry/native dispatch, schema-6 publication, or archive path.
Unsupported parameter families and mismatched kernel inventories fail closed.

The recursive bootstrap selector additionally requires a structural model,
500–10,000 resamples, fixed 95% two-sided percentile Type-7 inference, and
matching Recipe/General-SEM resampling settings. This branch currently creates
bootstrap authority contracts only; no bootstrap runner, scheduler, native
job, or Registry claim is added in this checkpoint.

Canonical General SEM results now have additive typed CB parameter rows,
existing fit/identification rows, and an optional CB-specific recursive
bootstrap receipt plus per-parameter inference ledger. Empty collections and
the absent receipt are omitted, so historical documents retain their original
wire shape.
