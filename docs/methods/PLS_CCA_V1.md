# CCA Composite Residual Diagnostics v1

Numerical status: validated only for the bounded descriptive contract below. Native workflow promotion additionally requires a current packaged-Tauri run, export, save, and reopen acceptance artifact.

`AnalysisMethod::Cca` runs the ordinary QuickPLS estimator and then compares observed composite-score correlations with correlations reproduced by one recursive standardized composite path model. It emits:

- estimation and payload method version `cca_composite_residual_v1`;
- nested model identifier `recursive_standardized_composite_path_model_v1`;
- exactly one row for each unordered pair of constructs;
- observed correlation, reproduced correlation, signed residual, and absolute residual for each pair; and
- the maximum absolute residual across those rows.

## Supported input contract

The calculation is accepted only when all of these conditions hold:

- raw numeric data are available;
- preprocessing is `standardized`;
- missing data use `listwise_deletion`;
- weighting is `path` or `factor`, never PCA;
- the model is recursive and contains at least two constructs and one structural path;
- every construct is reflective and has at least one observed indicator; and
- the recipe has no control-path declarations, interactions, higher-order constructs, case weights, bootstrap samples, studentized inner samples, or permutation samples.

Single-indicator reflective constructs are representable but retain the general single-item measurement warning. They are not evidence of a multi-item measurement-quality assessment.

## Reproduced-correlation identity

Construct scores and structural estimates come from the same immutable execution recipe as the completed run. Let `B` contain the standardized structural path coefficients. The reproduced composite correlation matrix is:

```text
(I - B)^-1 Psi (I - B)^-T
```

For exogenous constructs, `Psi` retains their observed composite correlations. For endogenous constructs, its diagonal uses `1 - R2`; off-diagonal endogenous disturbance covariances are fixed to zero. Each reported signed residual is `observed - reproduced`, its absolute residual is the absolute value of that difference, and the summary maximum is the largest row-level absolute residual.

## Interpretation boundary

These values are descriptive diagnostics. A larger absolute residual means that the bounded recursive composite path model reproduces that observed composite correlation less closely. QuickPLS does not convert the residuals into a confirmatory verdict.

No threshold, pass/fail decision, adequacy classification, p value, confidence interval, or bootstrap discrepancy test is produced. The workflow does not support formative or mixed measurement models, control paths, higher-order constructs, interactions, case weights, nonstandardized preprocessing, pairwise missing-data handling, covariance/correlation-only input, or resampled CCA inference.

Despite the historical `CCA` identifier, this is not a full implementation of SmartPLS CCA. SmartPLS describes confirmatory composite analysis as a broader series of steps for confirming reflective and formative measurement models within a nomological network. QuickPLS independently implements only the descriptive residual calculation specified here and does not claim SmartPLS project compatibility, workflow identity, or numerical equivalence. See the [official SmartPLS CCA description](https://smartpls.com/documentation/algorithms-and-techniques/validity-and-model-fit/confirmatory-composite-analysis/) for the scope of that product's workflow.

## Result and export contract

The native Results tree exposes an `Assessment` group only when a completed immutable run contains a valid CCA payload:

- `Residual summary` reports the nested model identifier, number of correlation pairs, and finite maximum absolute residual.
- `Composite residuals` reports only finite pair rows with immutable model display labels.

The same two tables and run provenance participate in CSV, XLSX, HTML, reviewer-pack, and Print/PDF paths. Unavailable inference or classifications are omitted rather than represented with `N/A`. A run is not promoted as native-ready unless its exact payload survives project commit, explicit save, load/reopen, and tamper validation.

## Validation evidence

- `npm run qpls:cca:reference` generates a deterministic three-construct, two-path `X -> Z -> Y` fixture. The omitted `X -> Y` relationship makes the model non-saturated and produces a genuine non-zero residual.
- The independent Python reference estimates PLS scores, rebuilds the recursive reproduced-correlation identity, verifies every pair and residual identity within `1e-10`, checks exact method/nested-model provenance, and exercises the bounded input guards.
- `npm run qpls:promotion:cca` additionally requires the native catalog/readiness/result contracts, qpls-project persistence support, current three-viewport browser evidence for catalog/setup/readiness with an explicit no-synthetic-results follow-up, and packaged-Tauri evidence for a genuine run, export, save, and reopen.

Publication use remains limited to reporting these descriptive composite-correlation residuals and their exact method version. Broader confirmatory composite analysis claims remain unsupported.
