# PLS score execution v2 validation strategy

Snapshot date: 2026-08-15

This document freezes the validation plan for `pls_score_execution_v2`. The
current NumPy oracle and its tests are independent, development-only work. They
do not execute a QuickPLS binary, attach a QualificationSpec V2 receipt, alter
the Capability Registry or a method manifest, or support promotion. The lane
remains `qualification_ready=false` until the same frozen cases are compared
with a current source-bound product build.

## Bounded scientific contract

The v2 contract separates model scoring semantics from run initialization:

| Concern | Exact v2 behavior |
|---|---|
| Standard initialization | Every estimated indicator requests `+1`; effective starts follow the legacy orientation and unit-variance normalization path. A configured Standard product result must be bit-identical to the legacy Standard result apart from the new identity/provenance payload. |
| Individual initialization | Finite weights cover every estimated indicator exactly once, in strict stable construct/indicator ID order, with at least one non-zero value per estimated block. One positive unit-variance scalar preserves requested ratios and signs. Fixed blocks are excluded. |
| Unit fixed scoring | Every block indicator requests `+1`; a positive unit-variance scalar produces the effective weights without sign reorientation. |
| Custom fixed scoring | The SemModel stable-ID map covers the block exactly. A positive unit-variance scalar preserves requested ratios and signs. |
| Mixed scoring | Fixed scores participate in inner proxies. Fixed weights never update; only estimated blocks enter the convergence maximum and update count. |
| Fixed-only scoring | Scores are returned directly with zero performed outer iterations and zero estimated-block updates. |
| Iteration accounting | `estimated_block_updates = performed_iterations * estimated_block_count`; fixed and estimated counts remain separate. |
| Execution envelope | Raw standardized/unit-variance point estimate; Path or Factor weighting; exactly 3,000 maximum iterations and `1e-7` stop criterion. |
| Fail-closed exclusions | `none` and `sum_to_one` fixed normalization; bootstrap, studentized, or permutation resampling; case weights; higher-order constructs; interactions; PCA weighting; mean-centered or unstandardized result location. |

`none` and `sum_to_one` remain valid universal-model representations. Their
typed execution rejection is intentional; v2 must not reinterpret either as
unit variance.

## Independent oracle

[`validation/pls_score_execution_v2_oracle.py`](../../validation/pls_score_execution_v2_oracle.py)
implements the equations directly with NumPy. It imports no QuickPLS crate,
module, shared numerical helper, or product result parser. The oracle:

- standardizes every complete indicator with sample standard deviation;
- resolves requested and effective initial/fixed weights by stable ID;
- applies Path or Factor inner proxies;
- updates only Mode A or Mode B estimated blocks;
- preserves fixed weights while including their scores in inner proxies;
- calculates final construct scores, structural paths, R-squared, and exact
  iteration accounting;
- records deterministic hashes of the resolved initial state and iteration
  trajectory.

This is a transparent second implementation, not a runtime dependency. Its
work report advertises `independent_implementation` and
`development_validation_only`, but explicitly marks the report as an
inadmissible qualification receipt.

## Test layers and acceptance rules

| Layer | Cases | Acceptance |
|---|---|---|
| Oracle unit/micro | Standard, Individual, Unit, Custom, mixed, fixed-only, invalid boundaries | Exact identifiers, requested weights, typed error codes, scoring kinds, block counts, and iteration accounting. Hand-computed fixed scores use absolute tolerance `2e-14`. |
| Oracle deterministic | Same input repeated; Standard and Individual trajectories | Each run is bit-repeatable. Both ordinary starts converge to the same stationary solution within `2e-8`; their resolved starts, iteration counts, and trajectory hashes differ. |
| Adversarial initialization | Collinear block where requested `[+1, -1]` cancels exactly | Standard converges; Individual fails with `pls_score_execution_v2_zero_variance_requested_score`. No fallback or sign rewrite. |
| Metamorphic | Stable-ID renaming, model/indicator/path declaration order, variable order, row order with stable row IDs, positive affine indicator transforms | Keyed scores, paths, weights, and R-squared agree within `2e-10`. |
| Product integration, pending | Current engine output versus the exact oracle cases | Exact method/contract identity and provenance; exact stable-ID/order/accounting fields; numerical rules above; configured Standard versus legacy Standard bit identity. |
| Archive/native/E2E, pending | Schema-6 append/reopen, desktop/native equality, export readback, installed/portable Windows | Required by QualificationSpec V2 before qualification or promotion. The evidence-absent customer CLI is not an alternate Internal execution route. |

The ordinary Individual fixture deliberately uses signed, unequal starting
weights. It converges deterministically along a trajectory different from the
Standard `+1` start. The adversarial cancellation fixture proves that starting
weights can also change executability, so initialization is scientific input
and must remain in the method identity/provenance surface.

## QualificationSpec V2 alignment

The current work covers numerical candidate estimands for resolved initial
weights, fixed unit-variance weights, construct scores, structural paths, and
iteration accounting. It exercises portions of the model-topology,
measurement-model, data-distribution, input-type, and workload axes.

It does not satisfy the complete QualificationSpec V2 evidence contract. A
future product comparison must be generated only after relevant product sources
freeze and must bind, at minimum:

- exact capability/cell/method identity without changing its current evidence
  or distribution state;
- source-set and scenario-set SHA-256 values;
- source-bound `quickpls-desktop` lib-test executable fingerprint and exact ignored Internal harness identity;
- commands, timestamps, strict stdout/stderr, and immutable write-new output;
- product-versus-oracle numerical comparisons for every case in this lane;
- configured Standard versus legacy Standard bit identity;
- typed product rejections for every unsupported boundary;
- archive, native, cross-format export, packaged Windows, cancellation,
  performance, accessibility, soak, and independent-review evidence.

Until that comparison exists, the work report must retain all of these values:

```text
work_evidence_only=true
qualification_ready=false
product_comparison_performed=false
receipt_attached=false
promotion_requested=false
```

## Focused verification

```powershell
python -m pytest -q validation/test_pls_score_execution_v2_oracle.py
python -m ruff check validation/pls_score_execution_v2_oracle.py validation/test_pls_score_execution_v2_oracle.py
python validation/pls_score_execution_v2_oracle.py
```

An optional deterministic work report can be written to a non-receipt work
location with `--write-work-report`. That output must not be copied into a
candidate-receipt directory or referenced by a Registry/manifest promotion.
