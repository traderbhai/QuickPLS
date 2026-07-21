# Mediation V1

`pls_mediation_v1` is validated for the documented QuickPLS v1.2.1 PLS mediation scope. It reports direct, indirect, total, VAF, and descriptive mediation classification from the validated PLS path/effect matrix.

Supported interpretation requires the relevant indirect effect to be paired with validated QuickPLS bootstrap or permutation inference. The scope excludes PROCESS-style observed-variable workflows, moderated mediation, unvalidated experimental base estimators, and unsupported model shapes.

Evidence:

- `validation/results/mediation_reference_report.json`
- `validation/results/mediation_r_reference_report.json`
- `validation/results/mediation_published_example_report.json`
- `validation/results/mediation_metamorphic_report.json`
- `validation/results/mediation_randomization_report.json`

The implementation details remain in `PLS_MEDIATION_V1.md`.
