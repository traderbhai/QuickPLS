# Two-Group MGA v3 (historical)

This identity is archive-readable only. New combined executions use `pls_mga_two_group_v4`; see
`MICOM_MGA_V4.md`.

Status: implemented as the point-estimate envelope for the coupled MICOM/permutation-MGA v3 workflow.

The workflow estimates the two selected groups independently and reports group paths, R², outer loadings, outer weights, and explicit A-minus-B comparisons. Both group solutions are sign-aligned to one pooled selected-group reference. The pooled row order is canonical for the unordered group pair, so exchanging Group A and Group B swaps the fitted summaries and exactly reverses signed comparisons.

The top-level result identity is `pls_mga_two_group_v3`. It must be accompanied by `pls_mga_permutation_v3` and `micom_v3` for current native execution. Earlier v1 and v2 bundles remain distinct archive formats.
