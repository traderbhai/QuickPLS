# Recipe-v4 core PLS execution adapter

`recipe_v4_pls_execution.rs` is an opt-in runner-facing bridge for the exact
`smartpls.pls_algorithm` / `qpls3.pls.algorithm` capability cell. It executes
only the bounded, non-resampled `CompiledPlsPlanV2` slice.

Before calling the existing production PLS estimator, the bridge:

- deterministically recompiles and compares the complete recipe-v4 plan and
  receipt;
- verifies the compiler target and exact capability-cell identity;
- verifies the recipe document, analytical recipe, model document, scientific
  model, plan, and analytical-identity digests through that recompilation;
- binds the receipt fingerprint and compiled dataset id to the concrete raw
  dataset;
- projects only compiled Mode A/Mode B blocks and recursive structural paths
  into the current estimator contract; and
- returns the exact compilation receipt, adapter version, projected-recipe
  digest, dataset id, and estimator version as deterministic provenance.

Non-executable recipe metadata stays in the compilation receipt and is not
copied into the legacy estimator projection. This prevents a future metadata
key from silently becoming an execution option at this compatibility boundary.

The PLS compiler rejects semantics this production estimator would otherwise
ignore, including scientific covariances, fixed/derived parameters, start
values, bounds, equality labels, unattached parameters, groups, weights,
derived terms, observed structural variables, and matrix input.

## Activation boundary

The adapter is executable only through its explicit Rust library function.
The current `run_pls_analysis` path, CLI, Tauri commands, project archive
runner, GUI, recipe schema constant (v3), and archive schema constant (v5) are
unchanged. The adapter produces point-estimation output only; it does not claim
recipe-v4 bootstrap, permutation, assessment, reporting, CB-SEM, persistence,
or UI support.
