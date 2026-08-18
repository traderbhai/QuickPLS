# Method-to-capability bridge V2

`src/domain/methodCapabilityRegistryV2.ts` is the fail-closed boundary between
the current `AnalysisMethodId` / `AnalysisUiSettings` recipe surface and
Capability Registry V2.

## Identity and aggregation rules

- A requested option is identified by `capability_id` plus `cell_id`. A cell ID
  is not unique across the official catalogue: PCA, PLSpredict/CVPAT, PROCESS,
  and MICOM/MGA intentionally share QuickPLS implementation cells.
- A method is Standard only when every exact requested option is available in
  Standard.
- A method is Experimental only when every requested option is visible with
  Experimental Labs enabled and at least one is Experimental.
- Any missing, absent, Legacy, Internal, or disabled requested option hides the
  combined request. A ready base estimator cannot mask an absent add-on.
- The product projection contains customer labels and actionable blocked IDs,
  but no evidence-maturity or qualification labels.
- Unknown methods, option tokens, capability rows, and capability/cell pairings
  fail closed.
- Availability is read from the matched `option_cells[]` entry. The bridge
  never reads or copies the parent row's conservative coverage/evidence/surface
  projection.

## Current mapping

| Request | Registry option cells |
|---|---|
| PLS algorithm | `smartpls.pls_algorithm` / `qpls3.pls.algorithm` |
| PLS bootstrap | PLS algorithm plus `smartpls.pls_bootstrapping` / `qpls3.inference.bootstrap` |
| Structural-path permutation | PLS algorithm plus `smartpls.permutation` / `qpls3.inference.structural_path_randomization` |
| Prospective sample size and power | `smartpls.pls_power_analysis` / `qpls3.pls.sample_size_power` (bounded v2 scoped Standard; historical v1 is read-only) |
| Post-hoc technical minimum result | Same official row / `qpls3.pls.posthoc_technical_minimum_sample_size` (partial scoped Standard result linked to a PLS run; numeric output requires complete PLS bootstrap significance, not a separate prospective-power method) |
| PLSc, WPLS | Their consistent or weighted PLS cells |
| CCA, CTA-PLS, IPMA | PLS algorithm plus the selected assessment cell |
| Endogeneity, nonlinear effects | PLS algorithm plus the selected advanced-relationship cell |
| Moderated mediation | PLS algorithm plus both moderation and mediation cells |
| Prediction | PLS algorithm plus both PLSpredict and CVPAT catalogue rows; add PLS-POS or FIMIX when selected |
| Groups | PLS algorithm plus every requested MICOM, MGA, PLS-POS, or FIMIX row |
| CB-SEM / CFA | The selected ML catalogue row; add bootstrap, multigroup, and invariance cells when requested |
| PCA | Both official PCA catalogue rows bound to the shared PCA cell |
| GSCA, NCA | Their standalone technique cell |
| Regression | OLS, logistic, or PROCESS according to `regressionType`; add the applicable regression or PROCESS bootstrap row when requested |

The current registry projects PCA, GSCA, NCA, and Regression into Standard.
OLS is the default Regression option and binary logistic is also a Standard
option. Each is release-qualified for an exact documented scope; partial
coverage remains explicit and does not imply complete SmartPLS breadth.

## Integration boundary

The bridge deliberately has no UI, store, native-controller, or runner
dependency. Those consumers should use `methodCapabilityAvailabilityV2` and
must not reconstruct availability from old method status fields or catalogue
arrays.
