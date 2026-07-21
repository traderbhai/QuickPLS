# Two-Stage Moderation V1

`pls_two_stage_moderation_v1` is validated for the documented QuickPLS v1.2.1 single-interaction two-stage moderation scope. Stage 1 estimates construct scores, stage 2 uses the generated product-score construct, and simple slopes are reported at standardized moderator scores `-1`, `0`, and `+1`.

Supported interpretation requires validated bootstrap or permutation inference for the interaction path. The scope excludes moderated mediation, multiple unqualified interaction systems, higher-order generated constructs, and unsupported weighting/preprocessing combinations.

Evidence:

- `validation/results/moderation_reference_report.json`
- `validation/results/moderation_r_reference_report.json`
- `validation/results/moderation_published_formula_report.json`
- `validation/results/moderation_published_empirical_report.json`
- `validation/results/moderation_simulation_report.json`
- `validation/results/moderation_inference_report.json`
- `validation/results/moderation_inference_qualification_report.json`
- `validation/results/moderation_coverage_qualification_report.json`

The implementation details remain in `PLS_TWO_STAGE_MODERATION_V1.md`.
