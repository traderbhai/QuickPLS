import fs from "node:fs/promises";
import { constants as fsConstants } from "node:fs";
import path from "node:path";
import { createHash } from "node:crypto";
import { execFile, spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { chromium } from "playwright";
import {
  enumerateQuickPlsCdpPages,
  inspectQuickPlsCdpPage,
  setActualTauriClientViewport as resizeActualTauriClientViewport,
} from "./v247_cdp_package_helpers.mjs";

const execFileAsync = promisify(execFile);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const bundledSampleCatalogPath = path.join(root, "src", "data", "bundledSampleProjects.v1.json");
const legacyBundledSampleResultTables = Object.freeze({
  corporate_reputation: "Path coefficients",
  simple_pls: "Path coefficients",
  mediation: "Direct effects",
  organizational_identification: "Path coefficients",
});
const requestedOiBundledSampleIds = Object.freeze([
  "organizational_identification_mediation",
  "organizational_identification_moderated_mediation",
  "organizational_identification_higher_order",
]);
const excludedOiBundledSampleId = "organizational_identification_moderation";
const expectedBundledSampleScopeSubstitutionIds = Object.freeze([
  "organizational_identification_moderated_mediation",
  "organizational_identification_higher_order",
]);

function bundledSampleContractsFromCatalog(catalog) {
  if (catalog?.schemaVersion !== 1 || !Array.isArray(catalog.datasets)
    || !catalog.constructSets || typeof catalog.constructSets !== "object" || Array.isArray(catalog.constructSets)
    || !Array.isArray(catalog.samples) || catalog.samples.length === 0) {
    throw new Error("The bundled sample catalog must be a non-empty schema-version-1 document.");
  }
  const datasets = new Map(catalog.datasets.map((dataset) => [dataset?.id, dataset]));
  if (datasets.size !== catalog.datasets.length || [...datasets.keys()].some((id) => typeof id !== "string" || !id)) {
    throw new Error("The bundled sample catalog must contain unique, non-empty dataset identities.");
  }
  const sampleIds = catalog.samples.map((sample) => sample?.id);
  if (new Set(sampleIds).size !== sampleIds.length || sampleIds.some((id) => typeof id !== "string" || !id)) {
    throw new Error("The bundled sample catalog must contain unique, non-empty sample identities.");
  }
  const legacyIds = Object.keys(legacyBundledSampleResultTables);
  const expectedSampleIds = [...legacyIds, ...requestedOiBundledSampleIds];
  if (JSON.stringify(sampleIds) !== JSON.stringify(expectedSampleIds)
    || sampleIds.includes(excludedOiBundledSampleId)
    || catalog.defaultSampleId !== "corporate_reputation") {
    throw new Error("The shared catalog must preserve the legacy four and contain only the three selected OI additions.");
  }
  const contracts = catalog.samples.map((sample) => {
    const dataset = datasets.get(sample.datasetId);
    const acceptance = sample.acceptance;
    const model = sample.model;
    const runs = sample.runs;
    const baseConstructs = model && Array.isArray(catalog.constructSets[model.constructSetId])
      ? catalog.constructSets[model.constructSetId]
      : null;
    if (!dataset || !acceptance || !model || !Array.isArray(model.paths)
      || !Array.isArray(model.extraConstructs) || !Array.isArray(model.interactions)
      || !Array.isArray(model.higher_order_constructs) || !Array.isArray(baseConstructs)
      || !Array.isArray(runs) || runs.length !== 1) {
      throw new Error(`Bundled sample ${sample.id} is missing its dataset, model, acceptance, or single completed-run contract.`);
    }
    const constructs = [...baseConstructs, ...model.extraConstructs];
    const constructIds = constructs.map((construct) => construct?.id);
    if (model.paths.length !== acceptance.pathCount || constructs.length !== acceptance.constructCount
      || new Set(constructIds).size !== constructIds.length
      || constructIds.some((id) => typeof id !== "string" || !id)) {
      throw new Error(`Bundled sample ${sample.id} acceptance counts or construct identities do not match its model.`);
    }
    const generatedInteractionIds = new Set(model.interactions.map((interaction) => interaction.product_construct));
    const higherOrderIds = new Set(model.higher_order_constructs.map((higherOrder) => higherOrder.id));
    if (generatedInteractionIds.size !== model.interactions.length
      || model.interactions.some((interaction) => !constructIds.includes(interaction.product_construct)
        || !constructIds.includes(interaction.outcome)
        || !model.paths.some((candidate) => candidate.source === interaction.product_construct
          && candidate.target === interaction.outcome))
      || higherOrderIds.size !== model.higher_order_constructs.length
      || model.higher_order_constructs.some((higherOrder) => !constructIds.includes(higherOrder.id)
        || !Array.isArray(higherOrder.components) || higherOrder.components.length < 2
        || higherOrder.components.some((component) => !constructIds.includes(component)))) {
      throw new Error(`Bundled sample ${sample.id} has an invalid generated interaction or higher-order contract.`);
    }
    const diagramConstructs = constructs.length - generatedInteractionIds.size;
    const resultPaths = higherOrderIds.size > 0
      ? model.paths.filter((candidate) => higherOrderIds.has(candidate.source) || higherOrderIds.has(candidate.target)).length
      : model.paths.filter((candidate) => !generatedInteractionIds.has(candidate.source) && !generatedInteractionIds.has(candidate.target)).length;
    const run = runs[0];
    const runLabel = acceptance.runLabel
      ?? (run?.methodConfig?.kind === "pls_algorithm" ? "PLS-SEM Algorithm run" : null);
    const resultTable = acceptance.resultTable
      ?? legacyBundledSampleResultTables[sample.id]
      ?? (higherOrderIds.size > 0 ? "Higher-order structural paths" : null);
    const referencePath = acceptance.referencePath ?? null;
    const referenceScope = sample.metadata?.reference_scope ?? null;
    const evidenceBoundary = sample.metadata?.evidence_boundary ?? null;
    if (!Number.isInteger(acceptance.caseCount) || !Number.isInteger(acceptance.constructCount)
      || !Number.isInteger(acceptance.pathCount) || diagramConstructs < 1 || resultPaths < 1
      || typeof sample.label !== "string" || !sample.label.trim()
      || typeof sample.detail !== "string" || !sample.detail.trim()
      || typeof sample.projectName !== "string" || !sample.projectName
      || typeof dataset.fileName !== "string" || !dataset.fileName || !runLabel) {
      throw new Error(`Bundled sample ${sample.id} has an incomplete packaged-acceptance contract.`);
    }
    if (referencePath !== null && (typeof referencePath !== "string" || !referencePath
      || sample.metadata?.smartpls_reference !== referencePath
      || typeof referenceScope !== "string" || !referenceScope
      || typeof evidenceBoundary !== "string" || !evidenceBoundary)) {
      throw new Error(`Bundled sample ${sample.id} has an incomplete scientific-reference disclosure.`);
    }
    return Object.freeze({
      id: sample.id,
      label: sample.label,
      detail: sample.detail,
      project: sample.projectName,
      datasetId: sample.datasetId,
      dataset: dataset.fileName,
      cases: acceptance.caseCount,
      constructs: constructs.length,
      diagramConstructs,
      paths: model.paths.length,
      resultPaths,
      runLabel,
      resultTable,
      referencePath,
      referenceScope,
      evidenceBoundary,
      scopeSubstitution: typeof evidenceBoundary === "string"
        && /not_claimed|qualified_scope|close_to/.test(evidenceBoundary),
    });
  });
  const scopeSubstitutionIds = contracts.filter((sample) => sample.scopeSubstitution).map((sample) => sample.id);
  if (JSON.stringify(scopeSubstitutionIds) !== JSON.stringify(expectedBundledSampleScopeSubstitutionIds)) {
    throw new Error("The bundled sample catalog must retain exactly the moderated-mediation and higher-order non-parity disclosures.");
  }
  return contracts;
}

const bundledSampleCatalogBytes = await fs.readFile(bundledSampleCatalogPath);
const bundledSampleCatalogSha256 = createHash("sha256").update(bundledSampleCatalogBytes).digest("hex");
const bundledSampleCatalog = JSON.parse(bundledSampleCatalogBytes.toString("utf8"));
const bundledSampleContracts = bundledSampleContractsFromCatalog(bundledSampleCatalog);
const screenshotDir = path.join(root, "validation", "results", "screens", "v247-native-desktop-acceptance");
const reportPath = path.join(root, "validation", "results", "v247_tauri_native_acceptance.json");
const logisticPackagedReportPath = path.join(root, "validation", "results", "logistic_v2_packaged_acceptance.json");
const regressionBootstrapPackagedReportPath = path.join(root, "validation", "results", "regression_bootstrap_v1_packaged_acceptance.json");
const processV2PackagedReportPath = path.join(root, "validation", "results", "process_v2_packaged_acceptance.json");
const structuralPathRandomizationPackagedReportPath = path.join(
  root,
  "validation",
  "results",
  "structural_path_randomization_v1_packaged_acceptance.json",
);
const cbsemExactBootstrapPackagedReportPath = process.env.QUICKPLS_CBSEM_EXACT_PACKAGED_REPORT_PATH?.trim()
  ? path.resolve(process.env.QUICKPLS_CBSEM_EXACT_PACKAGED_REPORT_PATH.trim())
  : path.join(root, "validation", "results", "cbsem_exact_case_bootstrap_v1_packaged_acceptance.json");
const validationResultsDir = path.join(root, "validation", "results");
const windowsNativeSaveHelperPath = path.join(root, "validation", "windows_native_save_export.py");
const endpoint = process.env.QUICKPLS_CDP_ENDPOINT ?? "http://127.0.0.1:9222";
const packagedTauriOrigin = "http://tauri.localhost";
const packagedTauriIpcOrigin = "http://ipc.localhost";
const acceptanceScope = process.env.QUICKPLS_ACCEPTANCE_SCOPE?.trim().toLocaleLowerCase() || "full";
if (!["full", "mga", "nca", "prediction", "hoc", "pca", "cta_pls", "ols", "logistic", "regression_bootstrap", "process_v2", "structural_path_randomization", "cbsem", "cbsem_exact_bootstrap", "gsca", "plsc_bootstrap", "pls_sample_size_power"].includes(acceptanceScope)) {
  throw new Error(`QUICKPLS_ACCEPTANCE_SCOPE must be a registered focused acceptance scope; received ${acceptanceScope}.`);
}
const ncaOnly = acceptanceScope === "nca";
const mgaOnly = acceptanceScope === "mga";
const predictionOnly = acceptanceScope === "prediction";
const hocOnly = acceptanceScope === "hoc";
const pcaOnly = acceptanceScope === "pca";
const ctaPlsOnly = acceptanceScope === "cta_pls";
const olsOnly = acceptanceScope === "ols";
const logisticOnly = acceptanceScope === "logistic";
const regressionBootstrapOnly = acceptanceScope === "regression_bootstrap";
const processV2Only = acceptanceScope === "process_v2";
const structuralPathRandomizationOnly = acceptanceScope === "structural_path_randomization";
const cbsemOnly = acceptanceScope === "cbsem";
const cbsemExactBootstrapOnly = acceptanceScope === "cbsem_exact_bootstrap";
const gscaOnly = acceptanceScope === "gsca";
const plscBootstrapOnly = acceptanceScope === "plsc_bootstrap";
const plsSampleSizePowerOnly = acceptanceScope === "pls_sample_size_power";
const focusedOnly = ncaOnly || mgaOnly || predictionOnly || hocOnly || pcaOnly || olsOnly || logisticOnly
  || ctaPlsOnly || regressionBootstrapOnly || processV2Only || structuralPathRandomizationOnly || cbsemOnly || gscaOnly
  || cbsemExactBootstrapOnly || plscBootstrapOnly || plsSampleSizePowerOnly;
const scopedReportPath = focusedOnly
  ? path.join(root, "validation", "results", `v247_tauri_native_acceptance_${acceptanceScope}.json`)
  : reportPath;
const recentProjectsKey = "quickpls.native.recent-projects.v1";
const uiPreferencesKey = "quickpls:native-ui-preferences:v1";
let priorUiPreferencesRaw = null;
let uiPreferencesSeeded = false;
const fixtureCsvPath = path.join(root, "validation", "results", "wpls_reference.csv");
const predictionFixtureCsvPath = path.join(root, "validation", "results", "prediction_native_reference.csv");
const mediationFixtureCsvPath = path.join(root, "validation", "results", "lavaan_latent_mediation_sem.csv");
const moderationFixtureCsvPath = path.join(root, "validation", "results", "moderation_reference_base.csv");
const hocFixtureCsvPath = path.join(root, "validation", "results", "higher_order_two_stage_base.csv");
const mgaFixtureCsvPath = path.join(root, "validation", "results", "mga_reference.csv");
const ccaFixtureCsvPath = path.join(root, "validation", "results", "cca_reference.csv");
const ipmaFixtureCsvPath = path.join(root, "validation", "results", "ipma_reference.csv");
const qplsCliPath = process.env.QUICKPLS_CLI_PATH ?? path.join(root, "target", "debug", "qpls.exe");
const disposableProjectPath = path.join(root, "validation", "results", `v247-native-methods-${Date.now()}-${process.pid}.qpls`);
const disposableProjectName = "Native Methods Acceptance";
const disposableModelName = "WPLS Structural Model";
const plscBootstrapProjectPath = path.join(root, "validation", "results", `v247-native-plsc-bootstrap-${Date.now()}-${process.pid}.qpls`);
const plscBootstrapProjectName = "Native PLSc Bootstrap Acceptance";
const plscBootstrapModelName = "PLSc Bootstrap Structural Model";
const plscBootstrapFeatureId = "qpls3.inference.consistent_bootstrap";
const plscBootstrapMethodVersion = "plsc_bootstrap_v1";
const plscBootstrapCatalogueSnapshotDate = "2026-08-12";
const plscBootstrapSamples = 10_000;
const plscBootstrapCancellationSamples = plscBootstrapSamples;
const plscBootstrapSeed = 20_260_818;
const plscBootstrapWorkers = 2;
const plsSampleSizePowerProjectPath = path.join(root, "validation", "results", `v247-native-pls-sample-size-power-${Date.now()}-${process.pid}.qpls`);
const plsSampleSizePowerProjectName = "Native Prospective PLS Power Acceptance";
const plsSampleSizePowerModelName = "Prospective PLS Power Model";
const plsSampleSizePowerFeatureId = "qpls3.pls.sample_size_power";
const plsSampleSizePowerMethodVersion = "pls_sample_size_power_v2";
const plsSampleSizePowerCatalogueSnapshotDate = "2026-08-12";
const plsSampleSizePowerGrid = "30,40";
const plsSampleSizePowerMonteCarloReplicates = 100;
const plsSampleSizePowerBootstrapReplicates = 99;
const plsSampleSizePowerSeed = 20_260_818;
const plsSampleSizePowerWorkers = 2;
const mediationProjectPath = path.join(root, "validation", "results", `v247-native-mediation-${Date.now()}-${process.pid}.qpls`);
const mediationProjectName = "Native Mediation Acceptance";
const mediationModelName = "Mediation Structural Model";
const moderationProjectPath = path.join(root, "validation", "results", `v247-native-moderation-${Date.now()}-${process.pid}.qpls`);
const moderationProjectName = "Native Moderation Acceptance";
const moderationModelName = "Moderation Structural Model";
const hocProjectPath = path.join(root, "validation", "results", `v247-native-hoc-${Date.now()}-${process.pid}.qpls`);
const hocProjectName = "Native Higher-Order Acceptance";
const hocModelName = "Higher-Order Structural Model";
const mgaProjectPath = path.join(root, "validation", "results", `v247-native-mga-${Date.now()}-${process.pid}.qpls`);
const mgaProjectName = "Native MGA Acceptance";
const mgaModelName = "Two-Group Structural Model";
const mgaRuntimePermutationSamples = 5_000;
const mgaCompletionTimeoutMs = 1_800_000;
const mgaMethodVersion = "pls_mga_two_group_v4";
const mgaPermutationMethodVersion = "pls_mga_permutation_v4";
const micomMethodVersion = "micom_v4";
const structuralPathRandomizationProjectPath = path.join(
  root,
  "validation",
  "results",
  `v247-native-structural-path-randomization-${Date.now()}-${process.pid}.qpls`,
);
const structuralPathRandomizationCancellationSnapshotPrefix = structuralPathRandomizationProjectPath.replace(
  /\.qpls$/,
  "-cancellation",
);
const structuralPathRandomizationProjectName = "Native Structural Path Randomization Acceptance";
const structuralPathRandomizationModelName = "Structural Path Randomization Model";
const structuralPathRandomizationFeatureId = "qpls3.inference.structural_path_randomization";
const structuralPathRandomizationMethodVersion = "freedman_lane_permutation_v1";
const structuralPathRandomizationOperation = "pls_pm_freedman_lane_v1";
const structuralPathRandomizationCatalogueSnapshotDate = "2026-08-12";
const structuralPathRandomizationEvidenceKind = "quickpls3_scoped_tauri_structural_path_randomization_v1_acceptance";
const structuralPathRandomizationPermutations = 10_000;
const structuralPathRandomizationSeed = 20_260_718;
const structuralPathRandomizationCancellationWorkers = 1;
const structuralPathRandomizationWorkers = 4;
const structuralPathRandomizationWarning = "Supported for the documented bounded scope: single-model Freedman-Lane randomization holds the original PLS construct scores fixed and reports unadjusted pathwise two-sided plus-one p values. Interpret these as conditional, approximate inference under exchangeable reduced-model residuals. Measurement-score uncertainty is not re-estimated, no multiplicity adjustment is applied, and current calibration covers homoscedastic Gaussian errors only.";
const structuralPathRandomizationProbabilityDisclosure = "Conditional/approximate two-sided plus-one probability under exchangeable reduced-model residuals; no multiplicity adjustment";
const structuralPathRandomizationQualificationDisclosure = "Supported within the documented fixed-score scope";
const structuralPathRandomizationExpectedColumns = ["Path", "Original", "Exceedances", "Permutations", "Raw two-sided p"];
const structuralPathRandomizationExpectedPathLabels = ["X -> Y", "Z -> Y"];
const structuralPathRandomizationExpectedCheckNames = [
  "runtimePreflight",
  "structuralPathRandomizationFixtureProvisioning",
  "structuralPathRandomizationSetup",
  "structuralPathRandomizationCancellation",
  "structuralPathRandomizationResults",
  "structuralPathRandomizationExport",
  "structuralPathRandomizationArchive",
  "structuralPathRandomizationSaveReopen",
  "resources",
  "cleanup",
];
const ccaProjectPath = path.join(root, "validation", "results", `v247-native-cca-${Date.now()}-${process.pid}.qpls`);
const ccaProjectName = "Native CCA Acceptance";
const ccaModelName = "CCA Residual Model";
const ccaMethodVersion = "cca_composite_residual_v1";
const ccaProvenanceMethodVersion = "pls_pm_v1+cca_composite_residual_v1+cca_residual_diagnostics_v1+pls_mediation_v1+pls_assessment_v8";
const ccaNestedModelVersion = "recursive_standardized_composite_path_model_v1";
const ipmaProjectPath = path.join(root, "validation", "results", `v247-native-ipma-${Date.now()}-${process.pid}.qpls`);
const ipmaProjectName = "Native IPMA Acceptance";
const ipmaModelName = "Importance-Performance Structural Model";
const ipmaMethodVersion = "ipma_v1";
const ipmaProvenanceMethodVersion = "pls_pm_v1+ipma_v1+pls_mediation_v1+pls_assessment_v8";
const ipmaPerformanceScale = "min_max_0_100_from_standardized_scores_v1";
const ncaFixtureCsvPath = path.join(root, "validation", "results", "nca_native_reference.csv");
const ncaProjectPath = path.join(root, "validation", "results", `v247-native-nca-${Date.now()}-${process.pid}.qpls`);
const ncaProjectName = "Native NCA Acceptance";
const ncaMethodVersion = "nca_v2";
const ncaObservations = 1_024;
const ncaPermutationSamples = 9_999;
const ncaSeed = 20_260_811;
const ncaTolerance = 1e-9;
const ncaScopeWarning = "NCA v2 supports one observed numeric condition/outcome pair with CE-FDH and CR-FDH ceilings, seeded one-sided permutation evidence, and observed-range bottlenecks. Multiple conditions, latent-score NCA, cIPMA, and additional ceiling variants are not available.";
const pcaFixtureCsvPath = path.join(root, "validation", "results", "v08_extended_methods_fixture.csv");
const pcaProjectPath = path.join(root, "validation", "results", `v247-native-pca-${Date.now()}-${process.pid}.qpls`);
const pcaProjectName = "Native PCA Acceptance";
const pcaMethodVersion = "pca_v1";
const pcaScopeWarning = "Standalone PCA v1 supports the model, data, and settings listed in Method Details; incompatible setups remain blocked.";
const pcaValidatedScope = "Correlation-matrix PCA of 2 to 50 selected numeric variables with listwise deletion, deterministic component orientation, and no rotation or inferential resampling.";
const pcaVariables = ["x", "m", "w", "y", "z"];
const pcaVarianceThreshold = 0.95;
const ctaPlsFixtureCsvPath = path.join(root, "validation", "results", "cta_pls_reference.csv");
const ctaPlsProjectPath = path.join(root, "validation", "results", `v247-native-cta-pls-${Date.now()}-${process.pid}.qpls`);
const ctaPlsProjectName = "Native CTA-PLS Acceptance";
const ctaPlsModelName = "CTA-PLS Descriptive Tetrad Model";
const ctaPlsMethodVersion = "cta_pls_tetrad_v1";
const ctaPlsProvenanceMethodVersion = "pls_pm_v1+cta_pls_tetrad_v1+pls_mediation_v1+pls_assessment_v8";
const ctaPlsCovarianceVersion = "sample_covariance_of_preprocessed_indicators_v1";
const ctaPlsResultWarning = "CTA-PLS tetrad bootstrap/permutation inference is outside the validated QuickPLS v1.2.3 descriptive scope.";
const ctaPlsScopeNote = "Descriptive sample-covariance tetrads only. QuickPLS reports all three pairings for every four-indicator subset; it does not classify blocks or calculate bootstrap, permutation, asymptotic, or vanishing-tetrad decisions.";
const ctaPlsPairings = ["ab_cd_minus_ac_bd", "ac_bd_minus_ad_bc", "ad_bc_minus_ab_cd"];
const ctaPlsViewports = [
  { id: "1024x700", width: 1024, height: 700 },
  { id: "1280x720", width: 1280, height: 720 },
  { id: "1440x900", width: 1440, height: 900 },
];
const olsFixtureCsvPath = path.join(root, "validation", "results", "v08_extended_methods_fixture.csv");
const olsProjectPath = path.join(root, "validation", "results", `v247-native-ols-${Date.now()}-${process.pid}.qpls`);
const olsProjectName = "Native OLS Acceptance";
const olsMethodVersion = "regression_ols_v1";
const olsScopeWarning = "OLS regression v1 requires numeric complete-case variables and HC3 robust standard errors; incompatible configurations are blocked before calculation.";
const olsValidatedScope = "Raw numeric ordinary least squares with an intercept, listwise deletion, HC3 robust standard errors, and fixed two-sided 95% confidence intervals. Optional regression case-resampling reports percentile-primary and conditional BCa inference. Categorical encoding, weights, clusters, generic PLS resampling, logistic regression, and PROCESS models are not included.";
const olsOutcome = "y";
const olsPredictors = ["x", "m"];
const olsControls = ["z"];
const logisticFixtureCsvPath = path.join(root, "validation", "results", "v08_extended_methods_fixture.csv");
const logisticProjectPath = path.join(root, "validation", "results", `v247-native-logistic-${Date.now()}-${process.pid}.qpls`);
const logisticProjectName = "Native Logistic Acceptance";
const logisticMethodVersion = "regression_logistic_v2";
const logisticFeatureId = "qpls3.standalone.logistic";
const logisticCatalogueSnapshotDate = "2026-08-12";
const logisticEvidenceKind = "quickpls3_scoped_tauri_logistic_v2_acceptance";
const logisticOutcome = "bin_y";
const logisticPredictors = ["x", "z"];
const logisticControls = ["w"];
const logisticObservations = 140;
const logisticZeroCases = 71;
const logisticOneCases = 69;
const logisticClassificationDisclaimer = "In-sample descriptive classification; not out-of-sample predictive performance.";
const logisticScopeWarning = "Logistic regression v2 requires a binary numeric outcome and numeric complete-case predictors; multinomial, ordinal, weighted, clustered, automatic categorical encoding, and Firth-corrected models are not available.";
const logisticValidatedScope = "Binary logistic regression with an intercept, raw numeric predictors, listwise deletion, deterministic maximum-likelihood estimation, Wald inference, odds ratios, fitted probabilities, and fixed two-sided 95% confidence intervals. Optional regression case-resampling reports percentile-primary and conditional BCa coefficient and odds-ratio inference. The outcome must be coded exactly 0/1. Multinomial, ordinal, weighted, clustered, penalized, generic PLS resampling, and Firth-corrected models are not included.";
const regressionBootstrapFixtureCsvPath = path.join(root, "validation", "results", "v08_extended_methods_fixture.csv");
const regressionBootstrapProjectPath = path.join(root, "validation", "results", `v247-native-regression-bootstrap-${Date.now()}-${process.pid}.qpls`);
const regressionBootstrapProjectName = "Native Regression Bootstrap Acceptance";
const regressionBootstrapFeatureId = "qpls3.standalone.regression_bootstrap";
const regressionBootstrapMethodVersion = "regression_bootstrap_v1";
const regressionBootstrapWitnessVersion = "regression_bootstrap_validation_witness_v1";
const regressionBootstrapCatalogueSnapshotDate = "2026-08-12";
const regressionBootstrapEvidenceKind = "quickpls3_scoped_tauri_regression_bootstrap_v1_acceptance";
const regressionBootstrapDefaultTableId = "regression_bootstrap_summary";
const regressionBootstrapSamples = 10_000;
const regressionBootstrapSeed = 20_260_812;
const regressionBootstrapWorkers = 2;
const regressionBootstrapPredictors = ["x", "z"];
const regressionBootstrapControls = ["w"];
const regressionBootstrapTerms = ["intercept", ...regressionBootstrapPredictors, ...regressionBootstrapControls];
const regressionBootstrapObservations = 140;
const processV2FixtureCsvPath = path.join(root, "validation", "results", "process_v2_native_reference.csv");
const processV2ReferenceContractPath = path.join(root, "validation", "results", "process_v2_native_reference_contract.json");
let processV2ExpectedGraphCounts = null;
let processV2ExpectedJohnsonNeymanAnalysisKeys = null;
const processV2ProjectPath = path.join(root, "validation", "results", `v247-native-process-v2-${Date.now()}-${process.pid}.qpls`);
const processV2ResetProjectPath = path.join(root, "validation", "results", `v247-native-process-v2-reset-${Date.now()}-${process.pid}.qpls`);
const processV2ResourceSnapshotPrefix = path.join(
  validationResultsDir,
  `process-v2-resource-snapshot-${Date.now()}-${process.pid}`,
);
const processV2ProjectName = "Native Graph-Defined PROCESS v2 Acceptance";
const processV2FeatureId = "qpls3.standalone.process";
const processV2MethodVersion = "regression_process_v2";
const processV2BootstrapMethodVersion = "regression_process_bootstrap_v1";
const processV2WitnessVersion = "regression_process_bootstrap_validation_witness_v1";
const processV2ReplicateFailureReasonCodes = new Set([
  "rank_deficient_equation",
  "invalid_binary_profile",
  "high_leverage_hc3_instability",
  "invalid_hc3_covariance",
  "degenerate_simple_slope_variance",
  "nonfinite_estimate",
]);
const processV2CatalogueSnapshotDate = "2026-08-12";
const processV2EvidenceKind = "quickpls3_scoped_tauri_process_v2_acceptance";
const processV2DefaultTableId = "process_model_summary";
const processV2ReferenceColumns = ["Effect ID", "Kind", "Path", "Estimate", "Reference condition"];
const processV2ReferenceCondition = "Continuous moderators are evaluated at their original complete-sample raw means (coded 0); binary moderators are evaluated at 0.";
const processV2PolicyKeys = ["centering", "confidence_level", "covariance", "inference_reference"];
const processV2ResultStatus = "validated";
const processV2CurveWarningDisclosure = "Exact engine-persisted Johnson-Neyman curve points; internal bootstrap refit diagnostics are not exported.";
const processV2PrivateWitnessWireToken = /regression_process_bootstrap_validation_witness_v1|validation_witness|successful_bootstrap|successful_jackknife|failed_jackknife/i;
const processV2Samples = 10_000;
const processV2Seed = 20_260_812;
const processV2Workers = 2;
const processV2Observations = 175;
const processV2Omitted = 5;
const processV2ResourcePhasesPath = process.env.QUICKPLS_PROCESS_V2_RESOURCE_PHASES_PATH?.trim()
  ? path.resolve(process.env.QUICKPLS_PROCESS_V2_RESOURCE_PHASES_PATH.trim())
  : "";
const processV2ResourcePhases = {};
const processV2IdleSettleMilliseconds = 5_000;
const processV2ResourceSampleCaptureMilliseconds = 500;
const processV2ResourceSampleWindowMilliseconds = 10_000;
const processV2ResourcePostMarkerHoldMilliseconds = processV2ResourceSampleCaptureMilliseconds
  + processV2ResourceSampleWindowMilliseconds;
const processV2ExpectedTableIds = [
  "process_model_summary", "process_paths", "process_equation_coefficients", "process_equation_fit",
  "process_reference_effects", "process_conditional_indirect_effects", "process_moderated_mediation_indices",
  "process_simple_slopes", "process_conditional_plot_points", "process_johnson_neyman",
  "process_johnson_neyman_curve_points", "process_bootstrap_summary",
  "process_bootstrap_failures", "process_bootstrap_inference", "process_bootstrap_bca", "process_scope",
];

function processV2PoliciesExact(policies) {
  if (policies === null || typeof policies !== "object" || Array.isArray(policies)) return false;
  return JSON.stringify(Object.keys(policies).sort()) === JSON.stringify(processV2PolicyKeys)
    && policies.centering === "equation_complete_case_mean_v1"
    && policies.confidence_level === 0.95
    && policies.covariance === "hc3_v1"
    && policies.inference_reference === "student_t_residual_df_v1";
}
const testedDesktopExecutablePath = process.env.QUICKPLS_DESKTOP_EXE_PATH?.trim()
  ? path.resolve(process.env.QUICKPLS_DESKTOP_EXE_PATH.trim())
  : path.join(root, "target", "release", "quickpls-desktop.exe");
const testedDistDirectory = path.join(root, "dist");
const cbsemFixtureCsvPath = path.join(root, "validation", "results", "lavaan_latent_regression_sem.csv");
const cbsemProjectPath = path.join(root, "validation", "results", `v247-native-cbsem-${Date.now()}-${process.pid}.qpls`);
const cbsemProjectName = "Native CB-SEM Acceptance";
const cbsemModelName = "CB-SEM Structural Model";
const cbsemMethodVersion = "cbsem_ml_v1";
const cbsemFitMethodVersion = "cbsem_fit_v1";
const cbsemModificationMethodVersion = "cbsem_modification_indices_v1";
const cbsemProvenanceMethodVersion = "pls_pm_v1+cbsem_ml_v1+cbsem_fit_v1+cbsem_modification_indices_v1+pls_mediation_v1+pls_assessment_v8";
const cbsemExactBootstrapFeatureId = "qpls3.cbsem.bootstrap";
const cbsemExactBootstrapMethodVersion = "cbsem_exact_case_bootstrap_v1";
const cbsemExactBootstrapCatalogueSnapshotDate = "2026-08-12";
const cbsemExactBootstrapFixtureCsvPath = path.join(root, "validation", "results", "v08_extended_methods_fixture.csv");
const cbsemExactBootstrapProjectPath = process.env.QUICKPLS_CBSEM_EXACT_PROJECT_PATH?.trim()
  ? path.resolve(process.env.QUICKPLS_CBSEM_EXACT_PROJECT_PATH.trim())
  : path.join(root, "validation", "results", "cbsem-exact-bootstrap-runtime.qpls");
const cbsemExactBootstrapSchema6Path = process.env.QUICKPLS_CBSEM_EXACT_SCHEMA6_PATH?.trim()
  ? path.resolve(process.env.QUICKPLS_CBSEM_EXACT_SCHEMA6_PATH.trim())
  : path.join(root, "validation", "results", "cbsem-exact-bootstrap-runtime-v6.qpls");
const cbsemExactBootstrapCheckpointPath = process.env.QUICKPLS_CBSEM_EXACT_CHECKPOINT_PATH?.trim()
  ? path.resolve(process.env.QUICKPLS_CBSEM_EXACT_CHECKPOINT_PATH.trim())
  : path.join(root, "validation", "results", "cbsem-exact-bootstrap-runtime.checkpoint.json");
const cbsemExactBootstrapPhase = process.env.QUICKPLS_CBSEM_EXACT_PHASE?.trim().toLocaleLowerCase() || "execute";
if (cbsemExactBootstrapOnly && !["execute", "reopen"].includes(cbsemExactBootstrapPhase)) {
  throw new Error(`QUICKPLS_CBSEM_EXACT_PHASE must be execute or reopen; received ${cbsemExactBootstrapPhase}.`);
}
const cbsemExactBootstrapProjectName = "Native Exact CB-SEM Bootstrap Acceptance";
const cbsemExactBootstrapModelName = "Exact CFA Bootstrap Model";
const cbsemExactBootstrapSamples = 1_000;
const cbsemExactBootstrapSeed = 20_260_818;
const cbsemExactBootstrapWorkers = 2;
const gscaFixtureCsvPath = path.join(root, "validation", "results", "v08_extended_methods_fixture.csv");
const gscaReferenceOutputPath = path.join(root, "validation", "results", "gsca_als_v2_quickpls.json");
const gscaProjectPath = path.join(root, "validation", "results", `v247-native-gsca-${Date.now()}-${process.pid}.qpls`);
const gscaProjectName = "Native GSCA Acceptance";
const gscaModelName = "GSCA Mixed-Block Model";
const gscaMethodVersion = "gsca_als_v2";
const gscaAlgorithmVersion = "alternating_least_squares_v1";
const predictionMethodVersion = "plspredict_indicator_v2";
const predictionRepeatedMethodVersion = "plspredict_repeated_kfold_indicator_v2";
const predictionCvpatMethodVersion = "cvpat_indicator_benchmarks_v2";
const predictionProvenanceMethodVersion = `pls_pm_v1+${predictionMethodVersion}+pls_mediation_v1+pls_assessment_v8`;
const predictionObservations = 8_192;
const predictionAssignment = "seeded_sha256_source_row_order_round_robin_10_v1";
const predictionFolds = 10;
const predictionRepeats = 10;
const predictionConfidenceLevel = 0.95;
const packageVersion = JSON.parse(await fs.readFile(path.join(root, "package.json"), "utf8")).version;
const requestedNativeExportPath = process.env.QUICKPLS_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedPlscNativeExportPath = process.env.QUICKPLS_PLSC_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedWplsNativeExportPath = process.env.QUICKPLS_WPLS_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedBootstrapNativeExportPath = process.env.QUICKPLS_BOOTSTRAP_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedMgaNativeExportPath = process.env.QUICKPLS_MGA_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedCcaNativeExportPath = process.env.QUICKPLS_CCA_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedIpmaNativeExportPath = process.env.QUICKPLS_IPMA_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedNcaNativeExportPath = process.env.QUICKPLS_NCA_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedPredictionNativeExportPath = process.env.QUICKPLS_PREDICTION_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedHocNativeExportPath = process.env.QUICKPLS_HOC_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedPcaNativeExportPath = process.env.QUICKPLS_PCA_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedCtaPlsNativeExportPath = process.env.QUICKPLS_CTA_PLS_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedOlsNativeExportPath = process.env.QUICKPLS_OLS_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedLogisticNativeExportPath = process.env.QUICKPLS_LOGISTIC_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedRegressionBootstrapOlsExportPath = process.env.QUICKPLS_REGRESSION_BOOTSTRAP_OLS_EXPORT_PATH?.trim() ?? "";
const requestedRegressionBootstrapLogisticExportPath = process.env.QUICKPLS_REGRESSION_BOOTSTRAP_LOGISTIC_EXPORT_PATH?.trim() ?? "";
const requestedProcessV2ExportPath = process.env.QUICKPLS_PROCESS_V2_EXPORT_PATH?.trim() ?? "";
const requestedStructuralPathRandomizationExportPath = process.env.QUICKPLS_STRUCTURAL_PATH_RANDOMIZATION_EXPORT_PATH?.trim() ?? "";
const requestedCbsemNativeExportPath = process.env.QUICKPLS_CBSEM_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedCbsemExactBootstrapExportPath = process.env.QUICKPLS_CBSEM_EXACT_EXPORT_PATH?.trim() ?? "";
const requestedGscaNativeExportPath = process.env.QUICKPLS_GSCA_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedPlscBootstrapNativeExportPath = process.env.QUICKPLS_PLSC_BOOTSTRAP_NATIVE_EXPORT_PATH?.trim() ?? "";
const requestedPlsSampleSizePowerNativeExportPath = process.env.QUICKPLS_PLS_SAMPLE_SIZE_POWER_NATIVE_EXPORT_PATH?.trim() ?? "";
const pythonExecutable = process.env.QUICKPLS_PYTHON?.trim() || "python";

const isolatedFocusedOnly = mgaOnly || hocOnly || predictionOnly || cbsemOnly || cbsemExactBootstrapOnly || pcaOnly || olsOnly
  || logisticOnly || regressionBootstrapOnly || ncaOnly || ctaPlsOnly || processV2Only
  || structuralPathRandomizationOnly || gscaOnly || plscBootstrapOnly || plsSampleSizePowerOnly;
const inheritPriorEvidence = focusedOnly && !isolatedFocusedOnly;
let priorEvidence = null;
if (cbsemExactBootstrapOnly && cbsemExactBootstrapPhase === "reopen") {
  try {
    priorEvidence = JSON.parse(await fs.readFile(scopedReportPath, "utf8"));
  } catch {
    priorEvidence = null;
  }
  if (!priorEvidence?.checks?.cbsemExactBootstrapCheckpoint) {
    throw new Error("Exact-CB reopen phase requires the successful execute-phase scoped evidence.");
  }
} else if (inheritPriorEvidence) {
  try {
    priorEvidence = JSON.parse(await fs.readFile(reportPath, "utf8"));
  } catch {
    priorEvidence = null;
  }
}

const evidence = {
  ...(cbsemExactBootstrapOnly ? {
    schema_version: "quickpls.packaged_acceptance.v1",
    feature_id: cbsemExactBootstrapFeatureId,
    method_version: cbsemExactBootstrapMethodVersion,
    catalogue_snapshot_date: cbsemExactBootstrapCatalogueSnapshotDate,
    acceptance_scope: "cbsem_exact_bootstrap",
    phase: cbsemExactBootstrapPhase,
  } : plsSampleSizePowerOnly ? {
    schema_version: "quickpls.packaged_acceptance.v1",
    feature_id: plsSampleSizePowerFeatureId,
    method_version: plsSampleSizePowerMethodVersion,
    catalogue_snapshot_date: plsSampleSizePowerCatalogueSnapshotDate,
    acceptance_scope: "pls_sample_size_power",
  } : plscBootstrapOnly ? {
    schema_version: "quickpls.packaged_acceptance.v1",
    feature_id: plscBootstrapFeatureId,
    method_version: plscBootstrapMethodVersion,
    catalogue_snapshot_date: plscBootstrapCatalogueSnapshotDate,
    acceptance_scope: "plsc_bootstrap",
  } : ctaPlsOnly ? {
    schema_version: "quickpls.packaged_acceptance.v1",
    feature_id: "qpls3.assessment.cta_pls",
    method_version: ctaPlsMethodVersion,
    catalogue_snapshot_date: "2026-08-12",
    acceptance_scope: "cta_pls",
  } : logisticOnly ? {
    schema_version: "quickpls.packaged_acceptance.v1",
    feature_id: logisticFeatureId,
    method_version: logisticMethodVersion,
    catalogue_snapshot_date: logisticCatalogueSnapshotDate,
  } : regressionBootstrapOnly ? {
    schema_version: "quickpls.packaged_acceptance.v1",
    feature_id: regressionBootstrapFeatureId,
    method_version: regressionBootstrapMethodVersion,
    catalogue_snapshot_date: regressionBootstrapCatalogueSnapshotDate,
  } : processV2Only ? {
    schema_version: "quickpls.packaged_acceptance.v1",
    feature_id: processV2FeatureId,
    method_version: processV2MethodVersion,
    bootstrap_method_version: processV2BootstrapMethodVersion,
    catalogue_snapshot_date: processV2CatalogueSnapshotDate,
  } : structuralPathRandomizationOnly ? {
    schema_version: "quickpls.packaged_acceptance.v1",
    feature_id: structuralPathRandomizationFeatureId,
    method_version: structuralPathRandomizationMethodVersion,
    catalogue_snapshot_date: structuralPathRandomizationCatalogueSnapshotDate,
    acceptance_scope: "structural_path_randomization",
  } : {}),
  passed: false,
  generatedAt: new Date().toISOString(),
  endpoint,
  runtime: "tauri-webview2-cdp",
  focusedRun: focusedOnly ? {
    scope: acceptanceScope,
    priorGeneratedAt: priorEvidence?.generatedAt ?? null,
    completedAt: null,
  } : null,
  checks: (inheritPriorEvidence || (cbsemExactBootstrapOnly && cbsemExactBootstrapPhase === "reopen")) && priorEvidence?.checks ? { ...priorEvidence.checks } : {},
  screenshots: (inheritPriorEvidence || (cbsemExactBootstrapOnly && cbsemExactBootstrapPhase === "reopen")) && Array.isArray(priorEvidence?.screenshots)
    ? priorEvidence.screenshots.filter((file) => acceptanceScope === "nca"
      ? !/\\(?:84|85|86|87|88|89)[a-z]?-tauri-native-nca-/i.test(file)
      : acceptanceScope === "plsc_bootstrap"
        ? !/\\17[3-9][a-z]?-tauri-native-plsc-bootstrap-/i.test(file)
      : acceptanceScope === "prediction"
        ? !/\\9[0-7][a-z]?-tauri-native-prediction-/i.test(file)
        : acceptanceScope === "hoc"
          ? !/\\10[0-6]-tauri-native-hoc-/i.test(file)
          : acceptanceScope === "pca"
            ? !/\\11[0-7]-tauri-native-pca-/i.test(file)
          : acceptanceScope === "ols"
            ? !/\\12[0-7]-tauri-native-ols-/i.test(file)
          : acceptanceScope === "logistic"
            ? !/\\15[0-9]-tauri-native-logistic-/i.test(file)
          : acceptanceScope === "regression_bootstrap"
            ? !/\\(?:16[0-9]|17[0-2])-tauri-native-regression-bootstrap-/i.test(file)
          : acceptanceScope === "process_v2"
            ? !/\\18[0-9]-tauri-native-process-v2-/i.test(file)
          : acceptanceScope === "cbsem"
            ? !/\\13[0-6][a-z]?-tauri-native-cbsem-/i.test(file)
          : acceptanceScope === "gsca"
            ? !/\\14[0-6][a-z]?-tauri-native-gsca-/i.test(file)
          : !/\\6[0-9]-tauri-native-mga-/i.test(file))
    : [],
  screenshotArtifacts: [],
  consoleErrors: [],
  failures: [],
};
evidence.screenshots = evidence.screenshots.filter((file) => (
  !/[\\/]99-tauri-native-failure-state-1440x900\.png$/i.test(file)
));

async function writeEvidenceFile(filePath, contents) {
  const retryableWindowsCodes = new Set(["UNKNOWN", "EBUSY", "EPERM", "EACCES"]);
  for (let attempt = 0; attempt < 8; attempt += 1) {
    try {
      await fs.writeFile(filePath, contents, "utf8");
      return;
    } catch (error) {
      if (!retryableWindowsCodes.has(error?.code) || attempt === 7) throw error;
      await new Promise((resolve) => setTimeout(resolve, 100 * (attempt + 1)));
    }
  }
}

async function writeAcceptanceEvidence() {
  evidence.screenshotArtifacts = (await Promise.all(evidence.screenshots.map(artifactDigest))).filter(Boolean);
  const screenshotDescriptorFailure = "One or more acceptance screenshots were missing, empty, duplicated, or could not be hash-bound at report publication.";
  const uniqueScreenshotPaths = new Set(evidence.screenshotArtifacts.map((row) => row.path));
  if ((evidence.screenshotArtifacts.length !== evidence.screenshots.length
      || uniqueScreenshotPaths.size !== evidence.screenshotArtifacts.length)
    && !evidence.failures.includes(screenshotDescriptorFailure)) {
    evidence.failures.push(screenshotDescriptorFailure);
  }
  evidence.passed = evidence.failures.length === 0 && evidence.consoleErrors.length === 0;
  const serialized = JSON.stringify(evidence, null, 2) + "\n";
  if (isolatedFocusedOnly && scopedReportPath !== reportPath) {
    await writeEvidenceFile(scopedReportPath, serialized);
  } else {
    await writeEvidenceFile(reportPath, serialized);
    if (scopedReportPath !== reportPath) await writeEvidenceFile(scopedReportPath, serialized);
  }
  if (logisticOnly) await writeLogisticPackagedEvidence();
  if (regressionBootstrapOnly) await writeRegressionBootstrapPackagedEvidence();
  if (processV2Only) await writeProcessV2PackagedEvidence();
  if (structuralPathRandomizationOnly) await writeStructuralPathRandomizationPackagedEvidence();
  if (cbsemExactBootstrapOnly) await writeCbsemExactBootstrapPackagedEvidence();
}

async function artifactDigest(filePath) {
  if (!filePath) return null;
  try {
    const [file, contents] = await Promise.all([fs.stat(filePath), fs.readFile(filePath)]);
    if (!file.isFile() || file.size <= 0) return null;
    return {
      path: path.relative(root, filePath).replaceAll("\\", "/"),
      size: file.size,
      sha256: createHash("sha256").update(contents).digest("hex"),
    };
  } catch {
    return null;
  }
}

async function writeCbsemExactBootstrapPackagedEvidence() {
  const source = evidence.checks;
  const checks = {
    setup: { passed: source.cbsemExactBootstrapSetup?.passed === true, evidence: source.cbsemExactBootstrapSetup ?? null },
    invalid_setup_blocked: { passed: source.cbsemExactBootstrapInvalidSetup?.passed === true, evidence: source.cbsemExactBootstrapInvalidSetup ?? null },
    execute_percentile: { passed: source.cbsemExactBootstrapPercentile?.passed === true, evidence: source.cbsemExactBootstrapPercentile ?? null },
    execute_studentized: { passed: source.cbsemExactBootstrapStudentized?.passed === true, evidence: source.cbsemExactBootstrapStudentized ?? null },
    execute_bca: { passed: source.cbsemExactBootstrapBca?.passed === true, evidence: source.cbsemExactBootstrapBca ?? null },
    cancellation_retry: { passed: source.cbsemExactBootstrapCancellationRetry?.passed === true, evidence: source.cbsemExactBootstrapCancellationRetry ?? null },
    result_identity: { passed: source.cbsemExactBootstrapResultIdentity?.passed === true, evidence: source.cbsemExactBootstrapResultIdentity ?? null },
    xlsx_same_run: { passed: source.cbsemExactBootstrapXlsx?.passed === true, evidence: source.cbsemExactBootstrapXlsx ?? null },
    save_reopen_same_run: { passed: source.cbsemExactBootstrapSaveReopen?.passed === true, evidence: source.cbsemExactBootstrapSaveReopen ?? null },
    offline: { passed: source.cbsemExactBootstrapOffline?.passed === true, evidence: source.cbsemExactBootstrapOffline ?? null },
    viewports: { passed: source.cbsemExactBootstrapViewports?.passed === true, evidence: source.cbsemExactBootstrapViewports ?? null },
    process_cleanup: { passed: false, pending_wrapper_supervisor: true },
  };
  const manifest = JSON.parse(await fs.readFile(
    path.join(root, "validation", "methods", "cbsem_exact_case_bootstrap_v1.manifest.json"),
    "utf8",
  ));
  const sourcePaths = new Set([
    manifest.governance.manifest_path,
    manifest.governance.schema_path,
    manifest.governance.validator_path,
    manifest.governance.focused_test_path,
    "validation/cbsem_exact_case_bootstrap_v1_factory.py",
    "validation/test_cbsem_exact_case_bootstrap_v1_factory.py",
    "validation/test_cbsem_exact_case_bootstrap_v1_packaged_producer.py",
    ...(manifest.qualification.source_requirements.packaged_acceptance ?? []),
    "validation/v247_tauri_native_acceptance.mjs",
    "validation/windows_native_save_export.py",
    "validation/results/v08_extended_methods_fixture.csv",
    "src/native/NativeRecipeV4CbsemWorkspace.tsx",
    "src/domain/internalRecipeV4CbsemWorkspace.ts",
    "src/domain/internalProjectSchema6ResultAppend.ts",
    "src-tauri/src/project_schema6_result_append.rs",
    "crates/qpls-project/src/project_schema_v6.rs",
  ]);
  const sourceArtifacts = (await Promise.all([...sourcePaths].sort().map((relative) => artifactDigest(path.join(root, relative))))).filter(Boolean);
  if (sourceArtifacts.length !== sourcePaths.size) {
    throw new Error("Exact-CB packaged report could not bind every required producer/product source.");
  }
  const [desktop, cli] = await Promise.all([
    artifactDigest(testedDesktopExecutablePath),
    artifactDigest(qplsCliPath),
  ]);
  if (!desktop || !cli) throw new Error("Exact-CB packaged report could not bind both frozen release binaries.");
  const report = {
    passed: Object.values(checks).every((check) => check.passed === true),
    feature_id: cbsemExactBootstrapFeatureId,
    method_version: cbsemExactBootstrapMethodVersion,
    catalogue_snapshot_date: cbsemExactBootstrapCatalogueSnapshotDate,
    scope: cbsemExactBootstrapMethodVersion,
    generated_at_utc: new Date().toISOString(),
    checks,
    binary_artifacts: { desktop, cli },
    source_artifacts: sourceArtifacts,
  };
  await writeEvidenceFile(cbsemExactBootstrapPackagedReportPath, JSON.stringify(report, null, 2) + "\n");
}

async function processV2ResourceArtifactState(filePath) {
  try {
    const stat = await fs.stat(filePath);
    return { path: path.relative(root, filePath).replaceAll("\\", "/"), bytes: stat.isFile() ? stat.size : null };
  } catch {
    return { path: path.relative(root, filePath).replaceAll("\\", "/"), bytes: 0 };
  }
}

async function snapshotProcessV2ResourceArchive(name, sourcePath) {
  if (!/^[a-z0-9_]+$/.test(name)) {
    throw new Error(`Unsafe PROCESS v2 resource snapshot name: ${name}`);
  }
  const snapshotPath = `${processV2ResourceSnapshotPrefix}-${name}.qpls`;
  const temporaryPath = `${snapshotPath}.copying`;
  await fs.mkdir(path.dirname(snapshotPath), { recursive: true });
  const existing = await Promise.all([
    fs.stat(snapshotPath).then(() => true).catch(() => false),
    fs.stat(temporaryPath).then(() => true).catch(() => false),
  ]);
  if (existing.some(Boolean)) {
    throw new Error(`PROCESS v2 resource snapshot target was not exclusive: ${snapshotPath}`);
  }
  const sourceStatBefore = await fs.stat(sourcePath, { bigint: true });
  const sourceDigestBefore = await artifactDigest(sourcePath);
  if (!sourceStatBefore.isFile() || !sourceDigestBefore) {
    throw new Error(`PROCESS v2 resource snapshot source was not a non-empty file: ${sourcePath}`);
  }
  try {
    await fs.copyFile(sourcePath, temporaryPath, fsConstants.COPYFILE_EXCL);
    const sourceStatAfter = await fs.stat(sourcePath, { bigint: true });
    const sourceDigestAfter = await artifactDigest(sourcePath);
    const temporaryDigest = await artifactDigest(temporaryPath);
    const sourceStable = sourceStatAfter.isFile()
      && sourceStatBefore.size === sourceStatAfter.size
      && sourceStatBefore.mtimeNs === sourceStatAfter.mtimeNs
      && sourceDigestAfter !== null
      && sourceDigestBefore.size === sourceDigestAfter.size
      && sourceDigestBefore.sha256 === sourceDigestAfter.sha256;
    if (!sourceStable || !temporaryDigest
      || temporaryDigest.size !== sourceDigestBefore.size
      || temporaryDigest.sha256 !== sourceDigestBefore.sha256) {
      throw new Error(`PROCESS v2 resource snapshot source changed during its exclusive copy: ${JSON.stringify({ sourceDigestBefore, sourceDigestAfter, temporaryDigest })}`);
    }
    await fs.link(temporaryPath, snapshotPath);
    await fs.rm(temporaryPath, { force: true });
    const snapshotDigest = await artifactDigest(snapshotPath);
    if (!snapshotDigest || snapshotDigest.size !== sourceDigestBefore.size
      || snapshotDigest.sha256 !== sourceDigestBefore.sha256) {
      throw new Error(`PROCESS v2 resource snapshot identity changed after atomic publication: ${snapshotPath}`);
    }
    const logicalState = await inspectProcessV2LogicalArchiveState(snapshotPath);
    return {
      path: snapshotDigest.path,
      bytes: snapshotDigest.size,
      sha256: snapshotDigest.sha256,
      source_path: sourceDigestBefore.path,
      source_before: {
        bytes: sourceDigestBefore.size,
        sha256: sourceDigestBefore.sha256,
        mtime_ns: sourceStatBefore.mtimeNs.toString(),
      },
      source_after: {
        bytes: sourceDigestAfter.size,
        sha256: sourceDigestAfter.sha256,
        mtime_ns: sourceStatAfter.mtimeNs.toString(),
      },
      source_stable_during_copy: true,
      exclusive_atomic_copy: true,
      application_opened: false,
      logical_state: logicalState,
    };
  } catch (error) {
    await fs.rm(temporaryPath, { force: true });
    await fs.rm(snapshotPath, { force: true });
    throw error;
  }
}

async function markProcessV2ResourcePhase(name, state, effectiveArchivePath) {
  if (!processV2Only || !processV2ResourcePhasesPath) return;
  const stateKeys = ["completed_result_count", "selected_run_id", "state_kind", "surface", "witness_count"];
  if (!state || JSON.stringify(Object.keys(state).sort()) !== JSON.stringify(stateKeys)
    || !["data", "results"].includes(state.surface)
    || !Number.isInteger(state.completed_result_count) || state.completed_result_count < 0
    || !Number.isInteger(state.witness_count) || state.witness_count < 0
    || (state.selected_run_id !== null && (typeof state.selected_run_id !== "string" || !state.selected_run_id.trim()))
    || typeof state.state_kind !== "string" || !state.state_kind.trim()) {
    throw new Error(`Invalid PROCESS v2 resource logical state for ${name}: ${JSON.stringify(state)}`);
  }
  const effectiveArchive = await snapshotProcessV2ResourceArchive(name, effectiveArchivePath);
  const archiveState = effectiveArchive.logical_state;
  if (!archiveState.manifestValid
    || archiveState.completedResultCount !== state.completed_result_count
    || archiveState.witnessCount !== state.witness_count
    || archiveState.selectedRunId !== state.selected_run_id) {
    throw new Error(`PROCESS v2 phase ${name} logical state did not match its effective archive: ${JSON.stringify({ state, archiveState })}`);
  }
  processV2ResourcePhases[name] = {
    recorded_at_utc: new Date().toISOString(),
    idle_settle_milliseconds: processV2IdleSettleMilliseconds,
    capture_delay_milliseconds: processV2ResourceSampleCaptureMilliseconds,
    sample_window_milliseconds: processV2ResourceSampleWindowMilliseconds,
    logical_state: state,
    effective_archive: effectiveArchive,
    primary_archive: await processV2ResourceArtifactState(processV2ProjectPath),
    export: await processV2ResourceArtifactState(requestedProcessV2ExportPath),
  };
  const temporary = `${processV2ResourcePhasesPath}.${process.pid}.tmp`;
  await fs.mkdir(path.dirname(processV2ResourcePhasesPath), { recursive: true });
  await fs.writeFile(temporary, JSON.stringify({
    schema_version: 2,
    feature_id: processV2FeatureId,
    method_version: processV2MethodVersion,
    phases: processV2ResourcePhases,
  }, null, 2) + "\n", "utf8");
  await fs.rename(temporary, processV2ResourcePhasesPath);
}

async function directoryManifestDigest(directoryPath) {
  const visit = async (current) => {
    const entries = await fs.readdir(current, { withFileTypes: true });
    const files = [];
    for (const entry of entries) {
      const absolute = path.join(current, entry.name);
      if (entry.isDirectory()) files.push(...await visit(absolute));
      else if (entry.isFile()) files.push(absolute);
    }
    return files;
  };
  try {
    const files = (await visit(directoryPath)).sort((left, right) => left < right ? -1 : left > right ? 1 : 0);
    if (files.length === 0) return null;
    const manifest = [];
    let size = 0;
    for (const file of files) {
      const contents = await fs.readFile(file);
      const relative = path.relative(directoryPath, file).replaceAll("\\", "/");
      const sha256 = createHash("sha256").update(contents).digest("hex");
      size += contents.length;
      manifest.push({ path: relative, size: contents.length, sha256 });
    }
    const canonical = manifest.map((item) => `${item.path}\0${item.size}\0${item.sha256}\n`).join("");
    return {
      path: path.relative(root, directoryPath).replaceAll("\\", "/"),
      size,
      file_count: manifest.length,
      sha256: createHash("sha256").update(canonical).digest("hex"),
      manifest,
    };
  } catch {
    return null;
  }
}

async function writeLogisticPackagedEvidence() {
  const source = evidence.checks;
  const workflowPassed = source.logisticWorkflow?.passed === true
    && source.logisticWorkflow?.feature_id === logisticFeatureId
    && source.logisticWorkflow?.method_version === logisticMethodVersion
    && source.logisticWorkflow?.catalogue_snapshot_date === logisticCatalogueSnapshotDate;
  const resultsPassed = Boolean(source.logisticResult)
    && source.logisticResult?.initialSelectedTable === "logistic_coefficients"
    && source.logisticResult?.noPlaceholder === true
    && source.logisticResult?.noSemResultGroups === true
    && source.logisticResult?.editModelCommand?.count === 0;
  const exportPassed = source.logisticExport?.nativeXlsx?.attempted === true
    && source.logisticExport?.nativeXlsx?.file?.isFile === true
    && source.logisticExport?.nativeXlsx?.file?.size > 0
    && source.logisticExport?.inSampleDisclaimerIncluded === true;
  const saveReopenPassed = source.logisticSaveReopen?.sameRunRestored === true
    && source.logisticSaveReopen?.archive?.manifest?.projectChecksumMatches === true;
  const failureLifecyclePassed = source.logisticFailureLifecycle?.passed === true;
  const legacyV1Passed = source.logisticLegacyV1?.passed === true;
  const xlsxPath = source.logisticExport?.nativeXlsx?.targetPath ?? "";
  const screenshotPaths = evidence.screenshots.filter((file) => /\\15[0-9]-tauri-native-logistic-/i.test(file));
  const [xlsx, projectArchive, ...screenshots] = await Promise.all([
    artifactDigest(xlsxPath),
    artifactDigest(logisticProjectPath),
    ...screenshotPaths.map(artifactDigest),
  ]);
  const checks = {
    workflow: {
      passed: workflowPassed,
      full_data_profiled: source.logisticWorkflow?.fullDataProfiled === true,
      active_lifecycle_captured: source.logisticWorkflow?.activeLifecycleCaptured === true,
      model_free: source.logisticWorkflow?.modelFree === true,
      source_check: "logisticWorkflow",
    },
    results: {
      passed: resultsPassed,
      coefficient_rows: source.logisticResult?.coefficients?.rows ?? null,
      probability_rows: source.logisticResult?.probabilities?.totalRows ?? null,
      in_sample_disclaimer: source.logisticResult?.classification?.warning ?? null,
      source_check: "logisticResult",
    },
    export: {
      passed: exportPassed && xlsx !== null,
      workbook_sheets: source.logisticExport?.nativeXlsx?.workbookSheets ?? [],
      in_sample_disclaimer_included: source.logisticExport?.inSampleDisclaimerIncluded === true,
      artifact_sha256: xlsx?.sha256 ?? null,
      source_check: "logisticExport",
    },
    save_reopen: {
      passed: saveReopenPassed && projectArchive !== null,
      same_run_restored: source.logisticSaveReopen?.sameRunRestored === true,
      project_checksum_matches: source.logisticSaveReopen?.archive?.manifest?.projectChecksumMatches === true,
      archive_sha256: projectArchive?.sha256 ?? null,
      source_check: "logisticSaveReopen",
    },
    failure_lifecycle: {
      passed: failureLifecyclePassed,
      boundary: "strict_nonbinary_outcome_profile_rejected_then_valid_profile_recovered",
      source_check: "logisticFailureLifecycle",
    },
    legacy_v1: {
      passed: legacyV1Passed,
      boundary: "current_packaged_v2_result_is_not_reinterpreted_as_historical_v1",
      historical_archive_readability_evidence: "backend_persistence_gate",
      source_check: "logisticLegacyV1",
    },
  };
  const report = {
    schema_version: "quickpls.packaged_acceptance.v1",
    kind: logisticEvidenceKind,
    passed: evidence.passed && Object.values(checks).every((check) => check.passed),
    generated_at_utc: evidence.generatedAt,
    completed_at_utc: evidence.focusedRun?.completedAt ?? null,
    feature_id: logisticFeatureId,
    method_version: logisticMethodVersion,
    catalogue_snapshot_date: logisticCatalogueSnapshotDate,
    target: "windows_10_11_x64_packaged_tauri",
    runtime: evidence.runtime,
    endpoint: evidence.endpoint,
    generator: "validation/v247_tauri_native_acceptance.mjs",
    checks,
    artifacts: {
      xlsx,
      project_archive: projectArchive,
      screenshots: screenshots.filter(Boolean),
    },
    console_errors: evidence.consoleErrors,
    failures: evidence.failures,
    source_report: path.relative(root, scopedReportPath).replaceAll("\\", "/"),
  };
  await fs.writeFile(logisticPackagedReportPath, JSON.stringify(report, null, 2) + "\n", "utf8");
}

async function writeRegressionBootstrapPackagedEvidence() {
  const source = evidence.checks;
  const workflowPassed = source.regressionBootstrapWorkflow?.passed === true
    && source.regressionBootstrapWorkflow?.feature_id === regressionBootstrapFeatureId
    && source.regressionBootstrapWorkflow?.method_version === regressionBootstrapMethodVersion
    && source.regressionBootstrapWorkflow?.catalogue_snapshot_date === regressionBootstrapCatalogueSnapshotDate;
  const resultsPassed = source.regressionBootstrapResults?.passed === true
    && source.regressionBootstrapResults?.olsInitialSelectedTable === regressionBootstrapDefaultTableId
    && source.regressionBootstrapResults?.logisticInitialSelectedTable === regressionBootstrapDefaultTableId;
  const olsExportPassed = source.regressionBootstrapOlsExport?.passed === true
    && source.regressionBootstrapOlsExport?.nativeXlsx?.attempted === true
    && source.regressionBootstrapOlsExport?.nativeXlsx?.file?.isFile === true
    && source.regressionBootstrapOlsExport?.nativeXlsx?.file?.size > 0
    && source.regressionBootstrapOlsExport?.validationWitnessExcluded === true;
  const logisticExportPassed = source.regressionBootstrapLogisticExport?.passed === true
    && source.regressionBootstrapLogisticExport?.nativeXlsx?.attempted === true
    && source.regressionBootstrapLogisticExport?.nativeXlsx?.file?.isFile === true
    && source.regressionBootstrapLogisticExport?.nativeXlsx?.file?.size > 0
    && source.regressionBootstrapLogisticExport?.validationWitnessExcluded === true;
  const saveReopenPassed = source.regressionBootstrapSaveReopen?.passed === true
    && source.regressionBootstrapSaveReopen?.archive?.manifest?.projectChecksumMatches === true
    && source.regressionBootstrapSaveReopen?.initialSelectedTables?.ols === regressionBootstrapDefaultTableId
    && source.regressionBootstrapSaveReopen?.initialSelectedTables?.logistic === regressionBootstrapDefaultTableId;
  const cancellationPassed = source.regressionBootstrapCancellation?.passed === true;
  const witnessPassed = source.regressionBootstrapWitnessBoundary?.passed === true;
  const screenshotPaths = evidence.screenshots.filter((file) => /\\(?:16[0-9]|17[0-2])-tauri-native-regression-bootstrap-/i.test(file));
  const [olsXlsx, logisticXlsx, projectArchive, testedDesktopExecutable, testedDistBundle, ...screenshots] = await Promise.all([
    artifactDigest(source.regressionBootstrapOlsExport?.nativeXlsx?.targetPath ?? ""),
    artifactDigest(source.regressionBootstrapLogisticExport?.nativeXlsx?.targetPath ?? ""),
    artifactDigest(regressionBootstrapProjectPath),
    artifactDigest(testedDesktopExecutablePath),
    directoryManifestDigest(testedDistDirectory),
    ...screenshotPaths.map(artifactDigest),
  ]);
  const checks = {
    workflow: {
      passed: workflowPassed,
      ols_completed: source.regressionBootstrapWorkflow?.olsCompleted === true,
      logistic_completed: source.regressionBootstrapWorkflow?.logisticCompleted === true,
      active_lifecycle_captured: source.regressionBootstrapWorkflow?.activeLifecycleCaptured === true,
      model_free: source.regressionBootstrapWorkflow?.modelFree === true,
      source_check: "regressionBootstrapWorkflow",
    },
    results: {
      passed: resultsPassed,
      ols_initial_selected_table: source.regressionBootstrapResults?.olsInitialSelectedTable ?? null,
      logistic_initial_selected_table: source.regressionBootstrapResults?.logisticInitialSelectedTable ?? null,
      ols_coefficient_rows: source.regressionBootstrapResults?.olsCoefficientRows ?? null,
      logistic_coefficient_rows: source.regressionBootstrapResults?.logisticCoefficientRows ?? null,
      percentile_primary_present: source.regressionBootstrapResults?.percentilePrimaryPresent === true,
      bca_conditional_present: source.regressionBootstrapResults?.bcaConditionalPresent === true,
      failure_disclosure_truthful: source.regressionBootstrapResults?.failureDisclosureTruthful === true,
      validation_witness_not_rendered: source.regressionBootstrapResults?.validationWitnessNotRendered === true,
      no_na_fabrication: source.regressionBootstrapResults?.noNaFabrication === true,
      source_check: "regressionBootstrapResults",
    },
    ols_export: {
      passed: olsExportPassed && olsXlsx !== null,
      workbook_sheets: source.regressionBootstrapOlsExport?.nativeXlsx?.workbookSheets ?? [],
      validation_witness_excluded: source.regressionBootstrapOlsExport?.validationWitnessExcluded === true,
      witness_scan: source.regressionBootstrapOlsExport?.nativeXlsx?.witnessScan ?? null,
      artifact_sha256: olsXlsx?.sha256 ?? null,
      source_check: "regressionBootstrapOlsExport",
    },
    logistic_export: {
      passed: logisticExportPassed && logisticXlsx !== null,
      workbook_sheets: source.regressionBootstrapLogisticExport?.nativeXlsx?.workbookSheets ?? [],
      validation_witness_excluded: source.regressionBootstrapLogisticExport?.validationWitnessExcluded === true,
      witness_scan: source.regressionBootstrapLogisticExport?.nativeXlsx?.witnessScan ?? null,
      artifact_sha256: logisticXlsx?.sha256 ?? null,
      source_check: "regressionBootstrapLogisticExport",
    },
    save_reopen: {
      passed: saveReopenPassed && projectArchive !== null,
      ols_same_run_restored: source.regressionBootstrapSaveReopen?.olsSameRunRestored === true,
      logistic_same_run_restored: source.regressionBootstrapSaveReopen?.logisticSameRunRestored === true,
      ols_initial_selected_table: source.regressionBootstrapSaveReopen?.initialSelectedTables?.ols ?? null,
      logistic_initial_selected_table: source.regressionBootstrapSaveReopen?.initialSelectedTables?.logistic ?? null,
      project_checksum_matches: source.regressionBootstrapSaveReopen?.archive?.manifest?.projectChecksumMatches === true,
      archive_witness_validated: source.regressionBootstrapSaveReopen?.archive?.witnessBoundary?.passed === true,
      archive_sha256: projectArchive?.sha256 ?? null,
      source_check: "regressionBootstrapSaveReopen",
    },
    cancellation: {
      passed: cancellationPassed,
      active_lifecycle_captured: source.regressionBootstrapCancellation?.activeLifecycleCaptured === true,
      no_partial_result: source.regressionBootstrapCancellation?.noPartialResult === true,
      source_check: "regressionBootstrapCancellation",
    },
    witness_boundary: {
      passed: witnessPassed,
      archive_only: source.regressionBootstrapWitnessBoundary?.archiveOnly === true,
      term_order_exact: source.regressionBootstrapWitnessBoundary?.termOrderExact === true,
      bootstrap_index_partition_exact: source.regressionBootstrapWitnessBoundary?.bootstrapIndexPartitionExact === true,
      jackknife_index_partition_exact: source.regressionBootstrapWitnessBoundary?.jackknifeIndexPartitionExact === true,
      excluded_from_results: source.regressionBootstrapWitnessBoundary?.excludedFromResults === true,
      excluded_from_exports: source.regressionBootstrapWitnessBoundary?.excludedFromExports === true,
      source_check: "regressionBootstrapWitnessBoundary",
    },
  };
  const report = {
    schema_version: "quickpls.packaged_acceptance.v1",
    kind: regressionBootstrapEvidenceKind,
    passed: evidence.passed && Object.values(checks).every((check) => check.passed)
      && testedDesktopExecutable !== null && testedDistBundle !== null,
    generated_at_utc: evidence.generatedAt,
    completed_at_utc: evidence.focusedRun?.completedAt ?? null,
    feature_id: regressionBootstrapFeatureId,
    method_version: regressionBootstrapMethodVersion,
    catalogue_snapshot_date: regressionBootstrapCatalogueSnapshotDate,
    target: "windows_10_11_x64_packaged_tauri",
    runtime: evidence.runtime,
    endpoint: evidence.endpoint,
    generator: "validation/v247_tauri_native_acceptance.mjs",
    tested_product: {
      quickpls_desktop_exe: testedDesktopExecutable,
      dist_bundle: testedDistBundle,
    },
    checks,
    artifacts: {
      ols_xlsx: olsXlsx,
      logistic_xlsx: logisticXlsx,
      project_archive: projectArchive,
      screenshots: screenshots.filter(Boolean),
    },
    console_errors: evidence.consoleErrors,
    failures: evidence.failures,
    source_report: path.relative(root, scopedReportPath).replaceAll("\\", "/"),
  };
  await fs.writeFile(regressionBootstrapPackagedReportPath, JSON.stringify(report, null, 2) + "\n", "utf8");
}

async function writeProcessV2PackagedEvidence() {
  const source = evidence.checks;
  const screenshotPaths = evidence.screenshots.filter((file) => /\\18[0-9]-tauri-native-process-v2-/i.test(file));
  const [xlsx, projectArchive, testedCliExecutable, testedDesktopExecutable, testedDistBundle, ...screenshots] = await Promise.all([
    artifactDigest(source.processV2Export?.nativeXlsx?.targetPath ?? ""),
    artifactDigest(processV2ProjectPath),
    artifactDigest(qplsCliPath),
    artifactDigest(testedDesktopExecutablePath),
    directoryManifestDigest(testedDistDirectory),
    ...screenshotPaths.map(artifactDigest),
  ]);
  const workflow = source.processV2Workflow ?? {};
  const setup = source.processV2Setup ?? {};
  const results = source.processV2Results ?? {};
  const exported = source.processV2Export ?? {};
  const reopened = source.processV2SaveReopen ?? {};
  const cancellation = source.processV2Cancellation ?? {};
  const cancelledRetrySetup = source.processV2CancelledRetrySetup ?? {};
  const witness = source.processV2WitnessBoundary ?? {};
  const resetClone = source.processV2ResourceResetClone ?? {};
  const runtimePreflight = source.runtimePreflight ?? {};
  const packagedPageState = (state) => state && typeof state === "object" ? {
    index: state.index ?? null,
    url: state.url ?? null,
    origin: state.origin ?? null,
    title: state.title ?? null,
    shell_visible: state.shellVisible === true,
    tauri_runtime: state.tauriRuntime === true,
  } : null;
  const checks = {
    runtime_preflight: {
      passed: runtimePreflight.passed === true,
      expected_origin: runtimePreflight.expectedOrigin ?? null,
      enumerated_pages: Array.isArray(runtimePreflight.enumeratedPages)
        ? runtimePreflight.enumeratedPages.map(packagedPageState)
        : [],
      qualifying_page_count: runtimePreflight.qualifyingPageCount ?? null,
      pre_reload: packagedPageState(runtimePreflight.preReload),
      reload_count: runtimePreflight.reloadCount ?? null,
      post_reload: packagedPageState(runtimePreflight.postReload),
      same_origin: runtimePreflight.sameOrigin === true,
      source_check: "runtimePreflight",
    },
    workflow: {
      passed: workflow.passed === true,
      completed: workflow.completed === true,
      active_lifecycle_captured: workflow.activeLifecycleCaptured === true,
      model_free: workflow.modelFree === true,
      graph_defined_without_numbered_templates: workflow.graphDefinedWithoutNumberedTemplates === true,
      source_check: "processV2Workflow",
    },
    setup: {
      passed: setup.passed === true,
      outcome: setup.outcome ?? null,
      focal_predictor: setup.focalPredictor ?? null,
      top_level_predictors: setup.topLevelPredictors ?? null,
      top_level_predictors_maximum: setup.topLevelPredictorsMaximum ?? null,
      paths: setup.paths ?? null,
      moderators: setup.moderators ?? null,
      moderations: setup.moderations ?? null,
      controls: setup.controls ?? null,
      controls_maximum: setup.controlsMaximum ?? null,
      equation_non_intercept_terms_maximum: setup.equationNonInterceptTermsMaximum ?? null,
      bootstrap_replicates: setup.bootstrapReplicates ?? null,
      workers: setup.workers ?? null,
      seed: setup.seed ?? null,
      source_check: "processV2Setup",
    },
    results: {
      passed: results.passed === true,
      initial_selected_table: results.initialSelectedTable ?? null,
      table_ids: results.tableIds ?? [],
      exact_table_ids: results.exactTableIds === true,
      equation_count: results.equationCount ?? null,
      reference_effect_rows: results.referenceEffectRows ?? null,
      conditional_indirect_rows: results.conditionalIndirectRows ?? null,
      moderated_mediation_index_rows: results.moderatedMediationIndexRows ?? null,
      simple_slope_rows: results.simpleSlopeRows ?? null,
      conditional_plot_point_rows: results.conditionalPlotPointRows ?? null,
      johnson_neyman_rows: results.johnsonNeymanRows ?? null,
      johnson_neyman_analysis_count: results.johnsonNeymanAnalysisCount ?? null,
      johnson_neyman_analysis_keys: results.johnsonNeymanAnalysisKeys ?? [],
      johnson_neyman_curve_point_rows: results.johnsonNeymanCurvePointRows ?? null,
      bootstrap_estimand_rows: results.bootstrapEstimandRows ?? null,
      accessible_non_color_plot_semantics: results.accessibleNonColorPlotSemantics === true,
      reference_effect_columns_exact: results.referenceEffectColumnsExact === true,
      reference_condition_rows_exact: results.referenceConditionRowsExact === true,
      promotion_pending_warning_absent: results.promotionPendingWarningAbsent === true,
      curve_warning_disclosure_exact: results.curveWarningDisclosureExact === true,
      failure_disclosure_truthful: results.failureDisclosureTruthful === true,
      validation_witness_not_rendered: results.validationWitnessNotRendered === true,
      no_na_fabrication: results.noNaFabrication === true,
      generic_regression_shell_not_applicable: results.genericRegressionShellNotApplicable === true,
      expected_counts_source: results.expectedCountsSource ?? null,
      expected_graph_counts: results.expectedGraphCounts ?? null,
      source_check: "processV2Results",
    },
    export: {
      passed: exported.passed === true && xlsx !== null,
      workbook_sheets: exported.nativeXlsx?.workbookSheets ?? [],
      validation_witness_excluded: exported.validationWitnessExcluded === true,
      witness_scan: exported.nativeXlsx?.witnessScan ?? null,
      process_table_contract: exported.nativeXlsx?.processTableContract ?? null,
      artifact_sha256: xlsx?.sha256 ?? null,
      source_check: "processV2Export",
    },
    save_reopen: {
      passed: reopened.passed === true && projectArchive !== null,
      same_run_restored: reopened.sameRunRestored === true,
      initial_selected_table: reopened.initialSelectedTable ?? null,
      project_checksum_matches: reopened.archive?.manifest?.projectChecksumMatches === true,
      archive_witness_validated: reopened.archive?.witnessBoundary?.passed === true,
      archive_sha256: projectArchive?.sha256 ?? null,
      cycle_1_settled_autosave: reopened.settledAutosave ?? null,
      cycle_1_autosave_after_checkpoint: reopened.autosaveAfterCheckpoint ?? null,
      source_check: "processV2SaveReopen",
    },
    cancellation: {
      passed: cancellation.passed === true,
      active_lifecycle_captured: cancellation.activeLifecycleCaptured === true,
      no_partial_result: cancellation.noPartialResult === true,
      source_check: "processV2Cancellation",
    },
    cancelled_retry_setup: {
      passed: cancelledRetrySetup.passed === true,
      read_only: cancelledRetrySetup.readOnly === true,
      exact_frozen_setup_match: cancelledRetrySetup.exactFrozenSetupMatch === true,
      snapshot: cancelledRetrySetup.snapshot ?? null,
      frozen_setup: cancelledRetrySetup.frozenSetup ?? null,
      source_check: "processV2CancelledRetrySetup",
    },
    witness_boundary: {
      passed: witness.passed === true,
      archive_only: witness.archiveOnly === true,
      witness_method_version: witness.witnessMethodVersion ?? null,
      estimand_order_exact: witness.estimandOrderExact === true,
      bootstrap_index_partition_exact: witness.bootstrapIndexPartitionExact === true,
      jackknife_index_partition_exact: witness.jackknifeIndexPartitionExact === true,
      excluded_from_results: witness.excludedFromResults === true,
      excluded_from_exports: witness.excludedFromExports === true,
      source_check: "processV2WitnessBoundary",
    },
    resource_reset: {
      passed: resetClone.passed === true,
      original_path: resetClone.originalPath ?? null,
      reset_path: resetClone.resetPath ?? null,
      distinct_path: resetClone.distinctPath === true,
      original_archive: resetClone.originalArchive ?? null,
      reset_archive: resetClone.resetArchive ?? null,
      result_id: resetClone.identity?.resultId ?? null,
      recipe_id: resetClone.identity?.recipeId ?? null,
      run_id: resetClone.identity?.runId ?? null,
      completed_result_count: resetClone.logicalState?.completedResultCount ?? null,
      witness_count: resetClone.logicalState?.witnessCount ?? null,
      no_sidecars_before_copy: resetClone.sidecarsBeforeCopy?.present?.length === 0,
      no_sidecars_after_copy: resetClone.sidecarsAfterCopy?.present?.length === 0,
      no_sidecars_before_open: resetClone.sidecarsBeforeOpen?.present?.length === 0,
      settled_autosave_sidecars_exact: resetClone.settledAutosave?.exactAllowedIdentity === true,
      autosave_sidecars_stable_after_checkpoint: resetClone.autosaveAfterCheckpoint?.exactAllowedIdentity === true
        && JSON.stringify(resetClone.autosaveAfterCheckpoint?.artifacts ?? null)
          === JSON.stringify(resetClone.settledAutosave?.artifacts ?? null),
      recovery_disclosure_absent: resetClone.recoveryDisclosureAbsent === true,
      table_ids: resetClone.resetTableIds ?? [],
      selected_run_id: resetClone.selectedRunId ?? null,
      selected_table_id: resetClone.selectedTableId ?? null,
      source_check: "processV2ResourceResetClone",
    },
  };
  const report = {
    schema_version: "quickpls.packaged_acceptance.v1",
    kind: processV2EvidenceKind,
    passed: evidence.passed && Object.values(checks).every((check) => check.passed)
      && testedDesktopExecutable !== null && testedDistBundle !== null
      && xlsx !== null && projectArchive !== null && screenshots.filter(Boolean).length >= 5,
    generated_at_utc: evidence.generatedAt,
    completed_at_utc: evidence.focusedRun?.completedAt ?? null,
    feature_id: processV2FeatureId,
    method_version: processV2MethodVersion,
    bootstrap_method_version: processV2BootstrapMethodVersion,
    catalogue_snapshot_date: processV2CatalogueSnapshotDate,
    target: "windows_10_11_x64_packaged_tauri",
    runtime: evidence.runtime,
    endpoint: evidence.endpoint,
    generator: "validation/v247_tauri_native_acceptance.mjs",
    tested_product: {
      qpls_cli_exe: testedCliExecutable,
      quickpls_desktop_exe: testedDesktopExecutable,
      dist_bundle: testedDistBundle,
    },
    checks,
    artifacts: {
      xlsx,
      project_archive: projectArchive,
      screenshots: screenshots.filter(Boolean),
    },
    console_errors: evidence.consoleErrors,
    failures: evidence.failures,
    source_report: path.relative(root, scopedReportPath).replaceAll("\\", "/"),
  };
  await fs.writeFile(processV2PackagedReportPath, JSON.stringify(report, null, 2) + "\n", "utf8");
}

async function writeStructuralPathRandomizationPackagedEvidence() {
  const source = evidence.checks;
  const screenshotPaths = evidence.screenshots.filter((file) => /\\(?:19[0-9]|20[0-9])-tauri-native-structural-path-randomization-/i.test(file));
  const cancellationBeforeReported = source.structuralPathRandomizationCancellation?.archiveBeforeSnapshot?.artifact ?? null;
  const cancellationAfterReported = source.structuralPathRandomizationCancellation?.archiveAfterSnapshot?.artifact ?? null;
  const [
    xlsx,
    projectArchive,
    testedCliExecutable,
    testedDesktopExecutable,
    testedDistBundle,
    cancellationArchiveBefore,
    cancellationArchiveAfter,
    ...screenshots
  ] = await Promise.all([
    artifactDigest(source.structuralPathRandomizationExport?.nativeXlsx?.targetPath ?? ""),
    artifactDigest(structuralPathRandomizationProjectPath),
    artifactDigest(qplsCliPath),
    artifactDigest(testedDesktopExecutablePath),
    directoryManifestDigest(testedDistDirectory),
    artifactDigest(cancellationBeforeReported?.path ? path.resolve(root, cancellationBeforeReported.path) : ""),
    artifactDigest(cancellationAfterReported?.path ? path.resolve(root, cancellationAfterReported.path) : ""),
    ...screenshotPaths.map(artifactDigest),
  ]);
  const checks = {
    runtimePreflight: source.runtimePreflight ?? { passed: false, pending: "harness" },
    structuralPathRandomizationFixtureProvisioning: source.structuralPathRandomizationFixtureProvisioning ?? { passed: false, pending: "harness" },
    structuralPathRandomizationSetup: source.structuralPathRandomizationSetup ?? { passed: false, pending: "harness" },
    structuralPathRandomizationCancellation: source.structuralPathRandomizationCancellation ?? { passed: false, pending: "harness" },
    structuralPathRandomizationResults: source.structuralPathRandomizationResults ?? { passed: false, pending: "harness" },
    structuralPathRandomizationExport: source.structuralPathRandomizationExport ?? { passed: false, pending: "harness" },
    structuralPathRandomizationArchive: source.structuralPathRandomizationArchive ?? { passed: false, pending: "harness" },
    structuralPathRandomizationSaveReopen: source.structuralPathRandomizationSaveReopen ?? { passed: false, pending: "harness" },
    resources: source.resources ?? { passed: false, pending: "wrapper" },
    cleanup: source.cleanup ?? { passed: false, pending: "wrapper" },
  };
  const exactCheckNames = JSON.stringify(Object.keys(checks)) === JSON.stringify(structuralPathRandomizationExpectedCheckNames);
  const cancellationArtifactsExact = cancellationArchiveBefore !== null
    && cancellationArchiveAfter !== null
    && JSON.stringify(cancellationArchiveBefore) === JSON.stringify(cancellationBeforeReported)
    && JSON.stringify(cancellationArchiveAfter) === JSON.stringify(cancellationAfterReported);
  const report = {
    schema_version: "quickpls.packaged_acceptance.v1",
    kind: structuralPathRandomizationEvidenceKind,
    passed: evidence.passed && exactCheckNames
      && Object.values(checks).every((check) => check?.passed === true)
      && xlsx !== null && projectArchive !== null && testedCliExecutable !== null
      && testedDesktopExecutable !== null && testedDistBundle !== null
      && cancellationArtifactsExact
      && screenshots.filter(Boolean).length >= 6,
    generated_at_utc: evidence.generatedAt,
    completed_at_utc: evidence.focusedRun?.completedAt ?? null,
    feature_id: structuralPathRandomizationFeatureId,
    method_version: structuralPathRandomizationMethodVersion,
    catalogue_snapshot_date: structuralPathRandomizationCatalogueSnapshotDate,
    target: "windows_10_11_x64_packaged_tauri",
    runtime: evidence.runtime,
    endpoint: evidence.endpoint,
    generator: "validation/v247_tauri_native_acceptance.mjs",
    acceptance_scope: "structural_path_randomization",
    tested_product: {
      qpls_cli_exe: testedCliExecutable,
      quickpls_desktop_exe: testedDesktopExecutable,
      dist_bundle: testedDistBundle,
    },
    checks,
    artifacts: {
      xlsx,
      project_archive: projectArchive,
      resource_samples: source.resources?.artifacts?.samples ?? null,
      resource_report: source.resources?.artifacts?.report ?? null,
      cleanup_report: source.cleanup?.artifact ?? null,
      cancellation_archive_before: cancellationArchiveBefore,
      cancellation_archive_after: cancellationArchiveAfter,
      screenshots: screenshots.filter(Boolean),
    },
    console_errors: evidence.consoleErrors,
    failures: evidence.failures,
    source_report: path.relative(root, scopedReportPath).replaceAll("\\", "/"),
  };
  await fs.writeFile(structuralPathRandomizationPackagedReportPath, JSON.stringify(report, null, 2) + "\n", "utf8");
}

async function canonicalNativeAnalysisCatalog() {
  const catalogSource = await fs.readFile(path.join(root, "src", "native", "nativeAnalysisCatalog.ts"), "utf8");
  const recipeSource = await fs.readFile(path.join(root, "src", "native", "nativeAnalysisRecipe.ts"), "utf8");
  const calculationModeSource = await fs.readFile(path.join(root, "src", "native", "nativeCalculationMode.ts"), "utf8");
  const catalogMatch = catalogSource.match(/const CATALOG_DRAFTS[^=]*= \[([\s\S]*?)\n\] as const;/);
  if (!catalogMatch) throw new Error("Could not locate the canonical native analysis catalogue declaration.");
  const establishedMatch = catalogSource.match(/export const NATIVE_ESTABLISHED_WORKING_ANALYSIS_KINDS_V1\s*=\s*\[([\s\S]*?)\n\] as const/);
  if (!establishedMatch) throw new Error("Could not locate the established working analysis catalogue declaration.");

  const catalogueKindOrder = [...catalogMatch[1].matchAll(/^[ \t]{4}kind:\s*"([a-z_]+)",\r?$/gm)]
    .map((match) => match[1]);
  const catalogueKinds = new Set(catalogueKindOrder);
  const establishedKinds = [...establishedMatch[1].matchAll(/^[ \t]*"([a-z_]+)",\r?$/gm)]
    .map((match) => match[1]);
  const standardSupplementalKinds = [
    "cta_pls",
    "plsc_bootstrap",
    "pls_posthoc_technical_minimum_sample_size",
    "pls_sample_size_power",
  ];
  const expectedKindSet = new Set([...establishedKinds, ...standardSupplementalKinds]);
  const kinds = catalogueKindOrder.filter((kind) => expectedKindSet.has(kind));
  if (kinds.length === 0 || new Set(kinds).size !== kinds.length) {
    throw new Error(`The production native execution-adapter order must be non-empty and unique: ${JSON.stringify(kinds)}`);
  }
  if (kinds.length !== expectedKindSet.size || [...expectedKindSet].some((kind) => !catalogueKinds.has(kind))) {
    throw new Error(`The established working catalogue contains an unknown analysis kind: ${JSON.stringify(kinds)}`);
  }

  const labelsByKind = new Map(
    [...recipeSource.matchAll(/\{\s*kind:\s*"([a-z_]+)"[^{}]*?\blabel:\s*"([^"]+)"/g)]
      .map((match) => [match[1], match[2]]),
  );
  const predictionLabel = calculationModeSource.match(/export const NATIVE_PREDICTION_METHOD_LABEL\s*=\s*"([^"]+)";/)?.[1];
  const regressionLabel = catalogSource.match(/item\.kind\s*===\s*"regression"\s*\?\s*"([^"]+)"/)?.[1];
  if (!predictionLabel || !regressionLabel) {
    throw new Error("Could not resolve the canonical Prediction or Regression catalogue label.");
  }
  labelsByKind.set("predict", predictionLabel);
  labelsByKind.set("regression", regressionLabel);
  labelsByKind.set("mga", "MICOM and Two-Group Permutation MGA");

  const methods = kinds.map((kind) => ({ kind, label: labelsByKind.get(kind) ?? null }));
  if (methods.some((method) => !method.label)
    || new Set(methods.map((method) => method.label)).size !== methods.length) {
    throw new Error(`The canonical native analysis catalogue has missing or duplicate labels: ${JSON.stringify(methods)}`);
  }
  return methods;
}

const nativeCalculationMethods = await canonicalNativeAnalysisCatalog();
const expectedOptionLabels = nativeCalculationMethods.map((method) => method.label);

async function canonicalNativeAnalysisCatalogKinds() {
  return nativeCalculationMethods.map((method) => method.kind);
}

await fs.mkdir(screenshotDir, { recursive: true });

async function validateRequestedNativeExportPath(value, environmentVariable = "QUICKPLS_NATIVE_EXPORT_PATH") {
  if (!path.isAbsolute(value)) {
    throw new Error(`${environmentVariable} must be an absolute .xlsx path under validation/results.`);
  }
  const targetPath = path.resolve(value);
  if (path.extname(targetPath).toLocaleLowerCase() !== ".xlsx") {
    throw new Error(`${environmentVariable} must use the .xlsx extension.`);
  }
  const realResultsDir = await fs.realpath(validationResultsDir);
  const realTargetParent = await fs.realpath(path.dirname(targetPath));
  const relativeParent = path.relative(realResultsDir, realTargetParent);
  if (relativeParent === ".." || relativeParent.startsWith(`..${path.sep}`) || path.isAbsolute(relativeParent)) {
    throw new Error(`${environmentVariable} must resolve inside validation/results.`);
  }
  try {
    await fs.access(targetPath);
    throw new Error(`${environmentVariable} must not already exist: ${targetPath}`);
  } catch (error) {
    if (error instanceof Error && error.message.startsWith(environmentVariable)) throw error;
    if (error?.code !== "ENOENT") throw error;
  }
  return targetPath;
}

function startWindowsNativeSaveExportHelper({
  targetPath,
  windowTitle,
  expectedSheets = [],
  expectedSharedStrings = [],
}) {
  const helperArguments = [
    windowsNativeSaveHelperPath,
    "--target",
    targetPath,
    "--results-root",
    validationResultsDir,
    "--window-title",
    windowTitle,
    "--timeout-seconds",
    "45",
  ];
  for (const expectedSheet of expectedSheets) {
    helperArguments.push("--expected-sheet", expectedSheet);
  }
  for (const expectedSharedString of expectedSharedStrings) {
    helperArguments.push("--expected-shared-string", expectedSharedString);
  }
  const child = spawn(pythonExecutable, helperArguments, {
    cwd: root,
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
  });

  const events = [];
  const protocolErrors = [];
  let stdoutBuffer = "";
  let stderr = "";
  let readyResolved = false;
  let completedResolved = false;
  let exited = false;
  let finalEvent = null;
  let resolveReady;
  let resolveCompleted;
  const ready = new Promise((resolve) => { resolveReady = resolve; });
  const completed = new Promise((resolve) => { resolveCompleted = resolve; });

  const failureEvent = (message, context = {}) => ({
    event: "complete",
    passed: false,
    phase: "helper_transport",
    error: { type: "HelperTransportError", message },
    ...context,
  });
  const settleReady = (value) => {
    if (readyResolved) return;
    readyResolved = true;
    resolveReady(value);
  };
  const settleCompleted = (value) => {
    if (completedResolved) return;
    completedResolved = true;
    resolveCompleted(value);
  };
  const consumeLine = (line) => {
    if (!line.trim()) return;
    try {
      const event = JSON.parse(line);
      events.push(event);
      if (event?.event === "ready") settleReady(event);
      if (event?.event === "complete") {
        finalEvent = event;
        if (!event.passed) settleReady(event);
      }
    } catch (error) {
      protocolErrors.push({ line, error: error instanceof Error ? error.message : String(error) });
    }
  };

  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    stdoutBuffer += chunk;
    const lines = stdoutBuffer.split(/\r?\n/);
    stdoutBuffer = lines.pop() ?? "";
    lines.forEach(consumeLine);
  });
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (chunk) => { stderr += chunk; });

  child.on("error", (error) => {
    const failure = failureEvent(error.message, { code: error.code ?? null });
    settleReady(failure);
    settleCompleted({ ...failure, transport: { stderr, events, protocolErrors } });
  });
  child.on("close", (code, signal) => {
    exited = true;
    if (stdoutBuffer.trim()) consumeLine(stdoutBuffer);
    let result = finalEvent ?? failureEvent("The native Save helper exited without a completion event.");
    if (result.passed && (code !== 0 || protocolErrors.length > 0)) {
      result = failureEvent("The native Save helper reported success with an invalid process or JSON-lines outcome.", {
        reportedCompletion: result,
      });
    }
    const withTransport = { ...result, transport: { exitCode: code, signal, stderr: stderr.trim(), events, protocolErrors } };
    settleReady(result.passed ? failureEvent("The native Save helper exited before its readiness event.") : result);
    settleCompleted(withTransport);
  });

  const timeout = setTimeout(() => {
    if (exited) return;
    child.kill();
    const failure = failureEvent("The native Save helper exceeded its 70 second transport timeout.");
    settleReady(failure);
    settleCompleted({ ...failure, transport: { stderr: stderr.trim(), events, protocolErrors } });
  }, 70_000);
  completed.finally(() => clearTimeout(timeout));

  return {
    ready,
    completed,
    stop: () => { if (!exited) child.kill(); },
  };
}

async function provisionDisposableProject({ sourceCsv, projectPath, projectName }) {
  try {
    await fs.access(sourceCsv);
  } catch {
    throw new Error(`The tracked native acceptance fixture is missing: ${sourceCsv}`);
  }
  try {
    await fs.access(qplsCliPath);
  } catch {
    throw new Error(`QuickPLS CLI is required to provision the disposable acceptance project. Build it first with cargo build -p qpls-cli, or set QUICKPLS_CLI_PATH. Expected: ${qplsCliPath}`);
  }
  const importArguments = ["import", sourceCsv, projectPath, "--name", projectName];
  let importResult = null;
  let importAttempts = 0;
  for (let attempt = 1; attempt <= 3; attempt += 1) {
    importAttempts = attempt;
    try {
      importResult = await execFileAsync(qplsCliPath, importArguments, {
        cwd: root,
        windowsHide: true,
        maxBuffer: 1024 * 1024,
      });
      break;
    } catch (error) {
      const diagnostic = `${error?.message ?? ""}\n${error?.stderr ?? ""}`;
      const transientOpenFailure = /unknown error, open|resource busy|sharing violation|EBUSY|EPERM/i.test(diagnostic);
      if (!transientOpenFailure || attempt === 3) throw error;
      await new Promise((resolve) => setTimeout(resolve, 250 * attempt));
    }
  }
  if (!importResult) throw new Error(`QuickPLS CLI import did not produce a result after ${importAttempts} attempts.`);
  const { stdout, stderr } = importResult;
  return {
    sourceCsv,
    project: projectPath,
    projectName,
    cli: qplsCliPath,
    stdout: stdout.trim(),
    stderr: stderr.trim(),
    importAttempts,
  };
}

async function provisionProcessV2ReferenceFixture(filePath) {
  const source = [
    "from pathlib import Path",
    "import json",
    "import sys",
    "sys.path.insert(0, str(Path.cwd() / 'validation'))",
    "from process_v2_reference import write_fixture, complete_case_columns, variable_profiles, reference_graph, OUTCOME",
    "write_fixture(Path(sys.argv[1]))",
    "columns, total_rows = complete_case_columns(Path(sys.argv[1]))",
    "graph = reference_graph(columns, raw_probe_profiles=variable_profiles(columns))",
    "counts = {'completeCases': len(columns[OUTCOME]), 'omittedCases': total_rows - len(columns[OUTCOME]), 'equations': len(graph['equations']), 'paths': len(graph['paths']), 'moderations': len(graph['moderations']), 'referenceEffects': len(graph['reference_effects']), 'conditionalIndirectEffects': len(graph['conditional_indirect_effects']), 'moderatedMediationIndices': len(graph['moderated_mediation_indices']), 'simpleSlopes': len(graph['simple_slopes']), 'plots': len(graph['plots']), 'conditionalPlotPoints': sum(len(series['points']) for plot in graph['plots'] for series in plot['series']), 'johnsonNeyman': len(graph['johnson_neyman']), 'johnsonNeymanRegionRows': sum(len(row['regions']) if row['status'] == 'available' else 1 for row in graph['johnson_neyman']), 'availableJohnsonNeyman': sum(row['status'] == 'available' for row in graph['johnson_neyman']), 'johnsonNeymanCurvePoints': sum(len(row.get('curve_points', [])) for row in graph['johnson_neyman']), 'estimands': len(graph['reference_effects']) + len(graph['conditional_indirect_effects']) + len(graph['moderated_mediation_indices']) + len(graph['simple_slopes'])}",
    "analysis_keys = [[row['moderation_id'], row['solved_moderator'], '; '.join(f\"{value['variable']} = {value['raw_value']:.4f} (coded {value['coded_value']:.4f})\" for value in row['conditioning_values'])] for row in graph['johnson_neyman']]",
    "Path(sys.argv[2]).write_text(json.dumps({'schema_version': 1, 'source': 'validation/process_v2_reference.py:reference_graph', 'counts': counts, 'johnson_neyman_analysis_keys': analysis_keys}, indent=2) + '\\n', encoding='utf-8')",
  ].join("; ");
  const { stdout, stderr } = await execFileAsync(pythonExecutable, ["-c", source, filePath, processV2ReferenceContractPath], {
    cwd: root,
    windowsHide: true,
    maxBuffer: 1024 * 1024,
  });
  const contents = await fs.readFile(filePath, "utf8");
  const lines = contents.trimEnd().split(/\r?\n/);
  const header = lines[0]?.split(",") ?? [];
  if (lines.length !== 181 || JSON.stringify(header) !== JSON.stringify(["X", "M1", "M2", "M3", "M4", "W", "B", "C", "Y"])) {
    throw new Error(`The PROCESS v2 reference fixture identity drifted: ${JSON.stringify({ rows: lines.length - 1, header })}`);
  }
  const expected = JSON.parse(await fs.readFile(processV2ReferenceContractPath, "utf8"));
  if (expected?.schema_version !== 1 || expected?.source !== "validation/process_v2_reference.py:reference_graph"
    || !expected?.counts || Object.values(expected.counts).some((value) => !Number.isInteger(value) || value < 0)
    || expected.counts.johnsonNeyman !== 4 || expected.counts.johnsonNeymanRegionRows !== 7
    || expected.counts.availableJohnsonNeyman !== 3
    || !Array.isArray(expected.johnson_neyman_analysis_keys)
    || expected.johnson_neyman_analysis_keys.length !== expected.counts.johnsonNeyman
    || expected.johnson_neyman_analysis_keys.some((row) => (
      !Array.isArray(row) || row.length !== 3 || row.some((value) => typeof value !== "string")
    ))) {
    throw new Error(`The independent PROCESS v2 expected-count contract was invalid: ${JSON.stringify(expected)}`);
  }
  processV2ExpectedGraphCounts = expected.counts;
  processV2ExpectedJohnsonNeymanAnalysisKeys = expected.johnson_neyman_analysis_keys;
  return {
    path: filePath, rows: 180, columns: header, stdout: stdout.trim(), stderr: stderr.trim(),
    expectedCountsSource: expected.source, expectedGraphCounts: expected.counts,
    expectedJohnsonNeymanAnalysisKeys: expected.johnson_neyman_analysis_keys,
  };
}

async function provisionMgaReferenceFixture(filePath) {
  const columns = ["group", "x1", "x2", "z1", "z2", "y1", "y2"];
  const rows = [];
  const format = (value) => value.toFixed(8);
  for (let index = 0; index < 180; index += 1) {
    const observation = index + 1;
    const group = index < 90 ? "A" : "B";
    const x = Math.sin(observation * 0.37) + 0.35 * Math.cos(observation * 0.11);
    const z = Math.cos(observation * 0.29) + 0.25 * Math.sin(observation * 0.17);
    const disturbance = 0.18 * Math.sin(observation * 1.37) + 0.08 * Math.cos(observation * 0.73);
    const y = group === "A"
      ? 0.78 * x + 0.18 * z + disturbance
      : 0.24 * x + 0.72 * z + disturbance;
    rows.push([
      group,
      format(x + 0.10 * Math.sin(observation * 0.83)),
      format(0.91 * x + 0.13 * Math.cos(observation * 0.61)),
      format(z + 0.11 * Math.cos(observation * 0.79)),
      format(0.89 * z + 0.12 * Math.sin(observation * 0.67)),
      format(y + 0.09 * Math.cos(observation * 0.97)),
      format(0.93 * y + 0.10 * Math.sin(observation * 1.09)),
    ].join(","));
  }
  await fs.writeFile(filePath, `${columns.join(",")}\n${rows.join("\n")}\n`, "utf8");
  return {
    path: filePath,
    columns,
    rows: rows.length,
    groups: { A: 90, B: 90 },
    deterministic: true,
  };
}

async function provisionNcaReferenceFixture(filePath) {
  const referenceRows = [[0, 1], [1, 3], [2, 2], [3, 4]];
  const repeatedRows = Array.from({ length: ncaObservations }, (_, index) => referenceRows[index % referenceRows.length]);
  const csv = `x,y\n${repeatedRows.map((row) => row.join(",")).join("\n")}\n`;
  await fs.writeFile(filePath, csv, "utf8");
  return {
    path: filePath,
    columns: ["x", "y"],
    rows: ncaObservations,
    completeCases: ncaObservations,
    deterministic: true,
    repeatedReferencePattern: referenceRows,
    expected: {
      scope: { minimum_x: 0, maximum_x: 3, minimum_y: 1, maximum_y: 4 },
      ceFdhPeers: [{ x: 0, y: 1 }, { x: 1, y: 3 }, { x: 3, y: 4 }],
      ceFdhEffectSize: 5 / 9,
      crFdh: { slope: 13 / 14, intercept: 10 / 7, effectSize: 36 / 91 },
      ceiling: "both",
      permutationSamples: ncaPermutationSamples,
      seed: ncaSeed,
    },
  };
}

async function provisionPredictionReferenceFixture(sourcePath, targetPath) {
  const source = await fs.readFile(sourcePath, "utf8");
  const lines = source.trim().split(/\r?\n/);
  const [header, ...referenceRows] = lines;
  if (!header || referenceRows.length !== 128) {
    throw new Error(`The prediction reference source must contain one header and exactly 128 data rows; found ${referenceRows.length}.`);
  }
  const repeatedRows = Array.from(
    { length: predictionObservations },
    (_, index) => referenceRows[index % referenceRows.length],
  );
  await fs.writeFile(targetPath, `${header}\n${repeatedRows.join("\n")}\n`, "utf8");
  return {
    path: targetPath,
    sourcePath,
    columns: header.split(","),
    rows: predictionObservations,
    deterministic: true,
    repeatedReferenceRows: referenceRows.length,
    repeatCount: predictionObservations / referenceRows.length,
  };
}

try {
  if (plsSampleSizePowerOnly) {
    evidence.checks.plsSampleSizePowerFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: mediationFixtureCsvPath,
      projectPath: plsSampleSizePowerProjectPath,
      projectName: plsSampleSizePowerProjectName,
    });
  } else if (plscBootstrapOnly) {
    evidence.checks.plscBootstrapFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: fixtureCsvPath,
      projectPath: plscBootstrapProjectPath,
      projectName: plscBootstrapProjectName,
    });
  } else if (gscaOnly) {
    evidence.checks.gscaFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: gscaFixtureCsvPath,
      projectPath: gscaProjectPath,
      projectName: gscaProjectName,
    });
  } else if (cbsemExactBootstrapOnly) {
    if (cbsemExactBootstrapPhase === "execute") {
      evidence.checks.cbsemExactBootstrapFixtureProvisioning = await provisionDisposableProject({
        sourceCsv: cbsemExactBootstrapFixtureCsvPath,
        projectPath: cbsemExactBootstrapProjectPath,
        projectName: cbsemExactBootstrapProjectName,
      });
    }
  } else if (cbsemOnly) {
    evidence.checks.cbsemFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: cbsemFixtureCsvPath,
      projectPath: cbsemProjectPath,
      projectName: cbsemProjectName,
    });
  } else if (olsOnly) {
    evidence.checks.olsFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: olsFixtureCsvPath,
      projectPath: olsProjectPath,
      projectName: olsProjectName,
    });
  } else if (logisticOnly) {
    evidence.checks.logisticFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: logisticFixtureCsvPath,
      projectPath: logisticProjectPath,
      projectName: logisticProjectName,
    });
  } else if (regressionBootstrapOnly) {
    evidence.checks.regressionBootstrapFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: regressionBootstrapFixtureCsvPath,
      projectPath: regressionBootstrapProjectPath,
      projectName: regressionBootstrapProjectName,
    });
  } else if (processV2Only) {
    // PROCESS v2 provisioning is deferred until the packaged Tauri page passes
    // its production-origin and reload preflight.
  } else if (structuralPathRandomizationOnly) {
    // Structural Path Randomization provisioning is deferred until the packaged
    // Tauri page passes its production-origin and reload preflight.
  } else if (pcaOnly) {
    evidence.checks.pcaFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: pcaFixtureCsvPath,
      projectPath: pcaProjectPath,
      projectName: pcaProjectName,
    });
  } else if (ctaPlsOnly) {
    evidence.checks.ctaPlsFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: ctaPlsFixtureCsvPath,
      projectPath: ctaPlsProjectPath,
      projectName: ctaPlsProjectName,
    });
  } else if (predictionOnly) {
    const predictionReferenceFixture = await provisionPredictionReferenceFixture(
      fixtureCsvPath,
      predictionFixtureCsvPath,
    );
    evidence.checks.fixtureProvisioning = await provisionDisposableProject({
      sourceCsv: predictionFixtureCsvPath,
      projectPath: disposableProjectPath,
      projectName: disposableProjectName,
    });
    evidence.checks.fixtureProvisioning.predictionReferenceFixture = predictionReferenceFixture;
  } else if (hocOnly) {
    evidence.checks.hocFixtureProvisioning = await provisionDisposableProject({
      sourceCsv: hocFixtureCsvPath,
      projectPath: hocProjectPath,
      projectName: hocProjectName,
    });
  } else {
    if (!mgaOnly) {
      evidence.checks.ncaReferenceFixture = await provisionNcaReferenceFixture(ncaFixtureCsvPath);
    }
    if (!ncaOnly) {
      evidence.checks.mgaReferenceFixture = await provisionMgaReferenceFixture(mgaFixtureCsvPath);
      if (!mgaOnly) {
        evidence.checks.fixtureProvisioning = await provisionDisposableProject({
          sourceCsv: fixtureCsvPath,
          projectPath: disposableProjectPath,
          projectName: disposableProjectName,
        });
        evidence.checks.mediationFixtureProvisioning = await provisionDisposableProject({
          sourceCsv: mediationFixtureCsvPath,
          projectPath: mediationProjectPath,
          projectName: mediationProjectName,
        });
        evidence.checks.moderationFixtureProvisioning = await provisionDisposableProject({
          sourceCsv: moderationFixtureCsvPath,
          projectPath: moderationProjectPath,
          projectName: moderationProjectName,
        });
      }
      evidence.checks.mgaFixtureProvisioning = await provisionDisposableProject({
        sourceCsv: mgaFixtureCsvPath,
        projectPath: mgaProjectPath,
        projectName: mgaProjectName,
      });
      if (!mgaOnly) {
        evidence.checks.ccaFixtureProvisioning = await provisionDisposableProject({
          sourceCsv: ccaFixtureCsvPath,
          projectPath: ccaProjectPath,
          projectName: ccaProjectName,
        });
        evidence.checks.ipmaFixtureProvisioning = await provisionDisposableProject({
          sourceCsv: ipmaFixtureCsvPath,
          projectPath: ipmaProjectPath,
          projectName: ipmaProjectName,
        });
      }
    }
    if (!mgaOnly) {
      evidence.checks.ncaFixtureProvisioning = await provisionDisposableProject({
        sourceCsv: ncaFixtureCsvPath,
        projectPath: ncaProjectPath,
        projectName: ncaProjectName,
      });
      evidence.checks.ncaFixtureProvisioning.initialArchive = await inspectInitialNcaArchive(ncaProjectPath);
    }
  }
} catch (error) {
  evidence.failures.push(error instanceof Error ? error.message : String(error));
  await writeAcceptanceEvidence();
  console.error(evidence.failures[0]);
  process.exit(1);
}

async function inspectCdpPage(candidate, index) {
  return inspectQuickPlsCdpPage(candidate, index);
}

async function enumerateCdpPages(browserInstance) {
  return enumerateQuickPlsCdpPages(browserInstance);
}

let browser = null;
let page = null;
const observedBrowserRequests = [];
evidence.checks.runtimePreflight = {
  passed: false,
  expectedOrigin: packagedTauriOrigin,
  enumeratedPages: [],
  qualifyingPageCount: 0,
  preReload: null,
  reloadCount: 0,
  postReload: null,
  sameOrigin: false,
};
try {
  browser = await chromium.connectOverCDP(endpoint);
  let pageEntries = [];
  for (let attempt = 0; attempt < 60; attempt += 1) {
    pageEntries = await enumerateCdpPages(browser);
    const qualifying = pageEntries.filter(({ state }) => state.shellVisible && state.tauriRuntime);
    if (qualifying.length > 0) break;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  const qualifying = pageEntries.filter(({ state }) => state.shellVisible && state.tauriRuntime);
  evidence.checks.runtimePreflight.enumeratedPages = pageEntries.map(({ state }) => state);
  evidence.checks.runtimePreflight.qualifyingPageCount = qualifying.length;
  if (qualifying.length !== 1) {
    throw new Error(`Expected exactly one QuickPLS shell+Tauri CDP page; found ${qualifying.length}: ${JSON.stringify(evidence.checks.runtimePreflight.enumeratedPages)}`);
  }
  page = qualifying[0].candidate;
  evidence.checks.runtimePreflight.preReload = qualifying[0].state;
  if (qualifying[0].state.origin !== packagedTauriOrigin) {
    throw new Error(`QuickPLS packaged preflight expected origin ${packagedTauriOrigin}; received ${qualifying[0].state.origin ?? "invalid"} at ${qualifying[0].state.url}.`);
  }

  if (ctaPlsOnly || gscaOnly || ncaOnly || hocOnly || cbsemExactBootstrapOnly || plscBootstrapOnly || acceptanceScope === "full") {
    page.on("request", (request) => {
      const url = request.url();
      let origin = null;
      try {
        origin = new URL(url).origin;
      } catch {
        origin = null;
      }
      observedBrowserRequests.push({
        method: request.method(),
        resourceType: request.resourceType(),
        url,
        origin,
      });
    });
  }

  page.on("pageerror", (error) => evidence.consoleErrors.push({ type: "pageerror", message: error.message }));
  page.on("console", (message) => {
    if (message.type() === "error") evidence.consoleErrors.push({ type: "console", message: message.text() });
  });

  let reloadFailure = null;
  evidence.checks.runtimePreflight.reloadCount = 1;
  try {
    priorUiPreferencesRaw = await page.evaluate((key) => window.localStorage.getItem(key), uiPreferencesKey);
    await page.evaluate((key) => {
      let preferences = {};
      try {
        const parsed = JSON.parse(window.localStorage.getItem(key) ?? "{}");
        if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) preferences = parsed;
      } catch {
        preferences = {};
      }
      window.localStorage.setItem(key, JSON.stringify({ ...preferences, experimentalLabsEnabled: true }));
    }, uiPreferencesKey);
    uiPreferencesSeeded = true;
    await page.reload({ waitUntil: "domcontentloaded", timeout: 30_000 });
    await page.locator(".nd-app[data-native-desktop-shell='true']").waitFor({ state: "visible", timeout: 15_000 });
  } catch (error) {
    reloadFailure = error instanceof Error ? error.message : String(error);
  }
  const postReload = await inspectCdpPage(page, qualifying[0].state.index);
  evidence.checks.runtimePreflight.postReload = postReload;
  evidence.checks.runtimePreflight.sameOrigin = postReload.origin === qualifying[0].state.origin;
  evidence.checks.runtimePreflight.passed = reloadFailure === null
    && postReload.origin === packagedTauriOrigin
    && postReload.shellVisible
    && postReload.tauriRuntime
    && evidence.checks.runtimePreflight.sameOrigin;
  if (!evidence.checks.runtimePreflight.passed) {
    throw new Error(`QuickPLS packaged reload preflight failed: ${JSON.stringify({ reloadFailure, ...evidence.checks.runtimePreflight })}`);
  }
} catch (error) {
  evidence.failures.push(error instanceof Error ? error.message : String(error));
  await writeAcceptanceEvidence();
  console.error(evidence.failures[0]);
  process.exit(1);
}

if (processV2Only) {
  try {
    evidence.checks.processV2ReferenceFixture = await provisionProcessV2ReferenceFixture(processV2FixtureCsvPath);
    evidence.checks.processV2FixtureProvisioning = await provisionDisposableProject({
      sourceCsv: processV2FixtureCsvPath,
      projectPath: processV2ProjectPath,
      projectName: processV2ProjectName,
    });
  } catch (error) {
    evidence.failures.push(error instanceof Error ? error.message : String(error));
    await writeAcceptanceEvidence();
    console.error(evidence.failures[0]);
    process.exit(1);
  }
}

if (structuralPathRandomizationOnly) {
  try {
    const fixture = await provisionMgaReferenceFixture(mgaFixtureCsvPath);
    const project = await provisionDisposableProject({
      sourceCsv: mgaFixtureCsvPath,
      projectPath: structuralPathRandomizationProjectPath,
      projectName: structuralPathRandomizationProjectName,
    });
    evidence.checks.structuralPathRandomizationFixtureProvisioning = {
      passed: fixture.deterministic === true && fixture.rows === 180
        && JSON.stringify(fixture.columns) === JSON.stringify(["group", "x1", "x2", "z1", "z2", "y1", "y2"]),
      fixture,
      project,
      model_name: structuralPathRandomizationModelName,
    };
    if (!evidence.checks.structuralPathRandomizationFixtureProvisioning.passed) {
      throw new Error(`Structural Path Randomization fixture identity drifted: ${JSON.stringify(evidence.checks.structuralPathRandomizationFixtureProvisioning)}`);
    }
  } catch (error) {
    evidence.failures.push(error instanceof Error ? error.message : String(error));
    await writeAcceptanceEvidence();
    console.error(evidence.failures[0]);
    process.exit(1);
  }
}

let priorRecentProjectsRaw = null;
let recentProjectsSeeded = false;
let nativeViewportLabel = "current-viewport";

async function capture(name) {
  const file = path.join(screenshotDir, name);
  await page.screenshot({ path: file, animations: "disabled" });
  evidence.screenshots.push(file);
  return file;
}

function windowBoundsEqual(left, right, tolerancePixels = 0) {
  if (!left || !right) return false;
  const leftState = left.windowState ?? "normal";
  const rightState = right.windowState ?? "normal";
  if (leftState !== rightState) return false;
  if (leftState !== "normal") return true;
  return ["left", "top", "width", "height"].every((key) => (
    Number.isInteger(left[key]) && Number.isInteger(right[key])
      && Math.abs(left[key] - right[key]) <= tolerancePixels
  ));
}

async function setActualTauriClientViewport(viewport, reason) {
  return resizeActualTauriClientViewport(page, viewport, reason);
}

async function captureActualTauriViewportMatrix({
  checkName,
  methodSlug,
  methodVersion,
  methodEvidenceCheck,
  expectedRunId,
  expectedRunLabel,
  expectedTableId,
  capturePrefix,
  captureSequence,
  exactWorkspace = false,
}) {
  const cdp = await page.context().newCDPSession(page);
  const contract = {
    passed: false,
    actualTauriWindow: true,
    resizeMechanism: "Browser.setWindowBounds",
    targetIdentity: null,
    deviceMetricsOverride: {
      clearCommand: "Emulation.clearDeviceMetricsOverride",
      cleared: false,
      pageSetViewportSizeUsed: false,
      emulationOnly: false,
    },
    method: {
      slug: methodSlug,
      version: methodVersion,
      evidenceCheck: methodEvidenceCheck,
      expectedRunId,
      expectedRunLabel,
      expectedTableId,
    },
    outerBoundsBefore: null,
    exactViewports: [],
    restoredFinalWindowState: null,
  };
  evidence.checks[checkName] = contract;
  let initialWindow = null;
  let targetId = null;
  try {
    contract.deviceMetricsOverride.playwrightViewportBefore = page.viewportSize();
    if (contract.deviceMetricsOverride.playwrightViewportBefore !== null) {
      throw new Error(`${checkName} found an active Playwright viewport override: ${JSON.stringify(contract.deviceMetricsOverride.playwrightViewportBefore)}`);
    }
    const target = await cdp.send("Target.getTargetInfo");
    targetId = target?.targetInfo?.targetId ?? null;
    if (!targetId) throw new Error(`${checkName} could not resolve the actual WebView2 target identity.`);
    initialWindow = await cdp.send("Browser.getWindowForTarget", { targetId });
    if (!Number.isInteger(initialWindow?.windowId) || !initialWindow?.bounds) {
      throw new Error(`${checkName} could not bind the WebView2 target to an actual desktop window.`);
    }
    contract.targetIdentity = {
      targetId,
      windowId: initialWindow.windowId,
      type: target.targetInfo.type ?? null,
      title: target.targetInfo.title ?? null,
      url: target.targetInfo.url ?? null,
      origin: new URL(page.url()).origin,
      lookupCommand: "Browser.getWindowForTarget",
    };
    contract.outerBoundsBefore = initialWindow.bounds;

    await cdp.send("Emulation.clearDeviceMetricsOverride");
    contract.deviceMetricsOverride.cleared = true;
    if (initialWindow.bounds.windowState !== "normal") {
      await cdp.send("Browser.setWindowBounds", {
        windowId: initialWindow.windowId,
        bounds: { windowState: "normal" },
      });
      await page.waitForTimeout(250);
    }

    let priorOuter = (await cdp.send("Browser.getWindowBounds", { windowId: initialWindow.windowId })).bounds;
    let requestedLeft = Number.isInteger(initialWindow.bounds.left) ? initialWindow.bounds.left : null;
    let requestedTop = Number.isInteger(initialWindow.bounds.top) ? initialWindow.bounds.top : null;
    for (const viewport of ctaPlsViewports) {
      const requestedClientViewport = { width: viewport.width, height: viewport.height };
      const attempts = [];
      let dom = null;
      let afterOuter = null;
      for (let attempt = 1; attempt <= 6; attempt += 1) {
        dom = await page.evaluate(() => ({ innerWidth, innerHeight }));
        const current = (await cdp.send("Browser.getWindowBounds", { windowId: initialWindow.windowId })).bounds;
        if (dom.innerWidth === viewport.width && dom.innerHeight === viewport.height) {
          afterOuter = current;
          break;
        }
        const requestedOuterBounds = {
          width: Math.max(300, current.width + viewport.width - dom.innerWidth),
          height: Math.max(300, current.height + viewport.height - dom.innerHeight),
        };
        if (requestedLeft !== null) requestedOuterBounds.left = requestedLeft;
        if (requestedTop !== null) requestedOuterBounds.top = requestedTop;
        await cdp.send("Browser.setWindowBounds", {
          windowId: initialWindow.windowId,
          bounds: requestedOuterBounds,
        });
        await page.waitForFunction(
          ([width, height]) => innerWidth === width && innerHeight === height,
          [viewport.width, viewport.height],
          { timeout: 1_500 },
        ).catch(() => undefined);
        await page.waitForTimeout(150);
        const observedDom = await page.evaluate(() => ({ innerWidth, innerHeight }));
        const observedOuter = (await cdp.send("Browser.getWindowBounds", { windowId: initialWindow.windowId })).bounds;
        attempts.push({ attempt, requestedOuterBounds, observedOuterBounds: observedOuter, observedDomInnerDimensions: observedDom });
        if (requestedLeft !== null && Number.isInteger(observedOuter.left)) requestedLeft += initialWindow.bounds.left - observedOuter.left;
        if (requestedTop !== null && Number.isInteger(observedOuter.top)) requestedTop += initialWindow.bounds.top - observedOuter.top;
        dom = observedDom;
        afterOuter = observedOuter;
        if (dom.innerWidth === viewport.width && dom.innerHeight === viewport.height) break;
      }

      const state = await page.evaluate(({ runId, runLabel, tableId, version, versionCheck, exactWorkspace }) => {
        const app = document.querySelector(".nd-app[data-native-desktop-shell='true']");
        if (exactWorkspace) {
          const details = Object.fromEntries(Array.from(document.querySelectorAll(".nd-cbsem-v4-run-details dl > div")).map((row) => [
            row.querySelector("dt")?.textContent?.trim() ?? "",
            row.querySelector("dd")?.textContent?.trim() ?? "",
          ]));
          const tableWrap = Array.from(document.querySelectorAll(".nd-cbsem-v4-table-wrap"))
            .find((candidate) => candidate.getAttribute("data-canonical-table-id") === tableId);
          const resultTable = tableWrap?.querySelector("table") ?? null;
          const heading = document.querySelector("#nd-cbsem-v4-results-heading");
          return {
            origin: location.origin,
            tauriRuntime: Boolean(window.__TAURI_INTERNALS__),
            surface: app?.getAttribute("data-surface") ?? null,
            domInnerDimensions: { width: innerWidth, height: innerHeight },
            documentNoHorizontalOverflow: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
            appNoHorizontalOverflow: Boolean(app && app.scrollWidth <= app.clientWidth + 1),
            selectedRunId: details.Run ?? null,
            selectedRunLabel: heading?.textContent?.replace(/\s+/g, " ").trim() ?? null,
            selectedTableId: tableWrap?.getAttribute("data-canonical-table-id") ?? null,
            resultRows: resultTable?.querySelectorAll("tbody tr").length ?? 0,
            resultTableVisible: Boolean(resultTable && resultTable.getClientRects().length > 0),
            expected: { runId, runLabel, tableId, version, versionCheck },
          };
        }
        const selectedRun = document.querySelector(".nd-run-select select");
        const selectedOption = selectedRun?.selectedOptions?.[0];
        const selectedTable = document.querySelector('.nd-result-tree [role="treeitem"][aria-selected="true"]');
        const resultTable = document.querySelector(".nd-result-table");
        return {
          origin: location.origin,
          tauriRuntime: Boolean(window.__TAURI_INTERNALS__),
          surface: app?.getAttribute("data-surface") ?? null,
          domInnerDimensions: { width: innerWidth, height: innerHeight },
          documentNoHorizontalOverflow: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
          appNoHorizontalOverflow: Boolean(app && app.scrollWidth <= app.clientWidth + 1),
          selectedRunId: selectedRun?.value ?? null,
          selectedRunLabel: selectedOption?.textContent?.replace(/\s+/g, " ").trim() ?? null,
          selectedTableId: selectedTable?.getAttribute("data-result-tree-item-id") ?? null,
          resultRows: resultTable?.querySelectorAll("tbody tr").length ?? 0,
          resultTableVisible: Boolean(resultTable && resultTable.getClientRects().length > 0),
          expected: { runId, runLabel, tableId, version, versionCheck },
        };
      }, {
        runId: expectedRunId,
        runLabel: expectedRunLabel,
        tableId: expectedTableId,
        version: methodVersion,
        versionCheck: methodEvidenceCheck,
        exactWorkspace,
      });
      const methodVersionEvidenceBound = JSON.stringify(evidence.checks[methodEvidenceCheck] ?? {}).includes(methodVersion);
      const methodRunLinkage = state.selectedRunId === expectedRunId
        && state.selectedRunLabel?.includes(expectedRunLabel)
        && state.selectedTableId === expectedTableId
        && state.resultRows > 0 && state.resultTableVisible
        && methodVersionEvidenceBound;
      const passed = state.domInnerDimensions.width === viewport.width
        && state.domInnerDimensions.height === viewport.height
        && state.origin === packagedTauriOrigin && state.tauriRuntime
        && state.surface === (exactWorkspace ? "model" : "results")
        && state.documentNoHorizontalOverflow && state.appNoHorizontalOverflow
        && methodRunLinkage;
      const screenshot = await capture(
        `${captureSequence}v-tauri-native-${capturePrefix}-packaged-viewport-${viewport.id}.png`,
      );
      contract.exactViewports.push({
        id: viewport.id,
        requestedClientViewport,
        domInnerDimensions: state.domInnerDimensions,
        outerBoundsBefore: priorOuter,
        outerBoundsAfter: afterOuter,
        outerBoundsChanged: priorOuter?.width !== afterOuter?.width || priorOuter?.height !== afterOuter?.height,
        resizeAttempts: attempts,
        origin: state.origin,
        tauriRuntime: state.tauriRuntime,
        surface: state.surface,
        noHorizontalOverflow: state.documentNoHorizontalOverflow && state.appNoHorizontalOverflow,
        methodRunLinkage,
        methodVersionEvidenceBound,
        selectedRunId: state.selectedRunId,
        selectedRunLabel: state.selectedRunLabel,
        selectedTableId: state.selectedTableId,
        resultRows: state.resultRows,
        screenshot,
        passed,
      });
      priorOuter = afterOuter;
      if (!passed) {
        throw new Error(`${checkName} failed at ${viewport.id}: ${JSON.stringify(contract.exactViewports.at(-1))}`);
      }
    }
  } finally {
    if (Number.isInteger(initialWindow?.windowId) && initialWindow.bounds) {
      try {
        const restoreNormalBounds = Object.fromEntries(
          ["left", "top", "width", "height"]
            .filter((key) => Number.isInteger(initialWindow.bounds[key]))
            .map((key) => [key, initialWindow.bounds[key]]),
        );
        let restored = null;
        let restoreRequest = { ...restoreNormalBounds };
        for (let attempt = 1; attempt <= 8 && Object.keys(restoreRequest).length; attempt += 1) {
          await cdp.send("Browser.setWindowBounds", { windowId: initialWindow.windowId, bounds: restoreRequest });
          await page.waitForTimeout(200);
          restored = (await cdp.send("Browser.getWindowBounds", { windowId: initialWindow.windowId })).bounds;
          if (windowBoundsEqual(initialWindow.bounds, restored)) break;
          for (const key of ["left", "top", "width", "height"]) {
            if (Number.isInteger(restoreRequest[key]) && Number.isInteger(initialWindow.bounds[key]) && Number.isInteger(restored[key])) {
              restoreRequest[key] += initialWindow.bounds[key] - restored[key];
            }
          }
        }
        if (initialWindow.bounds.windowState && initialWindow.bounds.windowState !== "normal") {
          await cdp.send("Browser.setWindowBounds", {
            windowId: initialWindow.windowId,
            bounds: { windowState: initialWindow.bounds.windowState },
          });
          await page.waitForTimeout(200);
        }
        restored = (await cdp.send("Browser.getWindowBounds", { windowId: initialWindow.windowId })).bounds;
        contract.restoredFinalWindowState = {
          requested: initialWindow.bounds,
          actual: restored,
          tolerancePixels: 1,
          passed: windowBoundsEqual(initialWindow.bounds, restored, 1),
        };
      } catch (restoreError) {
        contract.restoredFinalWindowState = {
          requested: initialWindow.bounds,
          actual: null,
          passed: false,
          error: restoreError instanceof Error ? restoreError.message : String(restoreError),
        };
      }
    }
    await cdp.detach().catch(() => undefined);
  }
  const distinctOuterSizes = new Set(contract.exactViewports.map((row) => (
    `${row.outerBoundsAfter?.width ?? "?"}x${row.outerBoundsAfter?.height ?? "?"}`
  )));
  contract.passed = contract.deviceMetricsOverride.cleared
    && contract.exactViewports.length === ctaPlsViewports.length
    && contract.exactViewports.every((row) => row.passed)
    && distinctOuterSizes.size === ctaPlsViewports.length
    && contract.restoredFinalWindowState?.passed === true;
  if (!contract.passed) {
    throw new Error(`${checkName} did not satisfy the actual packaged Tauri window matrix and restoration contract: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function openMenuItem(menu, item) {
  await page.getByRole("menuitem", { name: menu, exact: true }).click();
  await page.getByRole("menuitem", { name: item, exact: true }).click();
}

async function openBundledSampleResultTable(sample) {
  let title = sample.resultTable;
  if (!title) {
    const availableTitles = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents())
      .map((value) => value.trim());
    title = ["Direct effects", "Path coefficients"].find((candidate) => availableTitles.includes(candidate)) ?? null;
  }
  if (!title) {
    throw new Error(`The live ${sample.id} bundled result did not expose a structural-result table.`);
  }
  return { title, rows: await openResultTable(title) };
}

async function inspectBundledSample(sample) {
  await waitForSurface("launcher");
  const launcher = page.locator('.nd-launcher[aria-label="Project launcher"]');
  await launcher.waitFor({ state: "visible", timeout: 15_000 });
  const sampleCards = launcher.locator('.nd-sample-project-list button[data-sample-id]');
  const visibleSampleIds = await sampleCards.evaluateAll((elements) => elements.map((element) => (
    element.getAttribute("data-sample-id")
  )));
  const selectedCard = launcher.locator(`.nd-sample-project-list button[data-sample-id="${sample.id}"]`);
  if (await selectedCard.count() !== 1) {
    throw new Error(`The live launcher did not expose exactly one ${sample.id} sample card.`);
  }
  const cardLabel = (await selectedCard.locator("strong").textContent())?.trim() ?? "";
  const cardDetail = (await selectedCard.locator("small").textContent())?.trim() ?? "";
  await selectedCard.click();
  await waitForSurface("model");
  await page.locator(".react-flow__node-latent").nth(sample.diagramConstructs - 1)
    .waitFor({ state: "visible", timeout: 30_000 });
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const selectedRun = page.locator(".nd-run-select select");
  await selectedRun.waitFor({ state: "visible", timeout: 30_000 });
  const runOptions = (await selectedRun.locator("option").allTextContents()).map((value) => value.trim());
  const resultTable = await openBundledSampleResultTable(sample);
  const statusItems = page.locator(".nd-statusbar > span");
  const observed = {
    sampleId: sample.id,
    visibleSampleIds,
    cardLabel,
    cardDetail,
    project: (await page.locator(".nd-window-project").textContent())?.trim() ?? "",
    dataset: (await statusItems.nth(2).textContent())?.trim() ?? "",
    cases: (await statusItems.nth(3).textContent())?.trim() ?? "",
    renderedConstructs: (await statusItems.nth(4).textContent())?.trim() ?? "",
    runOptions,
    selectedRunId: await selectedRun.inputValue(),
    pathTable: resultTable.title,
    pathRows: resultTable.rows,
    scientificReference: {
      path: sample.referencePath,
      scope: sample.referenceScope,
      boundary: sample.evidenceBoundary,
      deliberateScopeSubstitution: sample.scopeSubstitution,
    },
  };
  if (JSON.stringify(visibleSampleIds) !== JSON.stringify(bundledSampleContracts.map((candidate) => candidate.id))
    || observed.cardLabel !== sample.label
    || observed.cardDetail !== sample.detail
    || observed.project !== sample.project
    || observed.dataset !== sample.dataset
    || observed.cases !== `${sample.cases} cases`
    || observed.renderedConstructs !== `${sample.constructs} constructs`
    || runOptions.length !== 1
    || runOptions[0] !== sample.runLabel
    || !observed.selectedRunId
    || resultTable.rows !== sample.resultPaths) {
    throw new Error(`The live ${sample.id} launcher card did not hydrate its exact bundled project/result contract: ${JSON.stringify(observed)}`);
  }
  await openMenuItem("View", "Edit Model");
  await waitForSurface("model");
  const renderedPaths = await structuralPaths().count();
  if (await page.locator(".react-flow__node-latent").count() !== sample.diagramConstructs
    || renderedPaths !== sample.paths) {
    throw new Error(`The live ${sample.id} sample model did not render its exact construct/path shape.`);
  }
  return { ...observed, renderedPaths };
}

async function waitForSurface(surface, timeout = 15_000) {
  await page.locator(`.nd-app[data-surface="${surface}"]`).waitFor({ state: "visible", timeout });
}

async function waitForResultsOrCalculationFailure(dialog, calculationLabel, timeout = 120_000) {
  await page.waitForFunction(() => (
    document.querySelector('.nd-app[data-surface="results"]')
    || document.querySelector('.nd-run-progress.failed')
  ), null, { timeout });
  const failure = dialog.locator(".nd-run-progress.failed");
  if (await failure.isVisible().catch(() => false)) {
    const status = compactVisibleText(await failure.textContent().catch(() => ""));
    throw new Error(`${calculationLabel} failed before opening Results: ${status || "no failure detail was rendered"}`);
  }
  await waitForSurface("results");
}

async function captureActiveCalculation(
  dialog,
  name,
  methodLabel,
  { allowTerminalTransitionAfterCapture = false } = {},
) {
  const progress = dialog.locator(
    '.nd-run-progress[aria-busy="true"]:is(.queued,.validating,.running,.cancelling)',
  );
  if (await progress.count() > 1) {
    throw new Error(`${methodLabel} exposed more than one active calculation state.`);
  }
  await progress.waitFor({ state: "visible", timeout: 5_000 });
  const state = await progress.evaluate((element) => ({
    ariaBusy: element.getAttribute("aria-busy"),
    status: [...element.classList].find((className) => ["queued", "validating", "running", "cancelling"].includes(className)) ?? null,
    phase: element.querySelector("strong")?.textContent?.trim() ?? "",
    message: element.querySelector("p")?.textContent?.trim() ?? "",
    progressValue: element.querySelector("progress")?.getAttribute("value") ?? null,
    progressMax: element.querySelector("progress")?.getAttribute("max") ?? null,
    logEntries: element.querySelectorAll("ol li").length,
  }));
  if (state.ariaBusy !== "true" || !state.status) throw new Error(`${methodLabel} did not expose a genuine active calculation state.`);
  await capture(name);
  const postCapture = await progress.evaluate((element) => ({
    ariaBusy: element.getAttribute("aria-busy"),
    status: [...element.classList].find((className) => ["queued", "validating", "running", "cancelling"].includes(className)) ?? null,
  })).catch(() => ({ ariaBusy: null, status: null }));
  const stillActive = postCapture.ariaBusy === "true" && Boolean(postCapture.status);
  if (!stillActive && !allowTerminalTransitionAfterCapture) {
    throw new Error(`${methodLabel} left its active lifecycle before the evidence snapshot completed.`);
  }
  return {
    ...state,
    postCapture: {
      ...postCapture,
      active: stillActive,
      terminalTransitionObserved: !stillActive,
      terminalTransitionAllowed: allowTerminalTransitionAfterCapture,
    },
  };
}

async function openResultTable(title) {
  const escapedTitle = title.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const item = page.locator('.nd-result-tree [role="treeitem"]').filter({ hasText: new RegExp(`^${escapedTitle}$`) });
  await item.waitFor({ state: "visible", timeout: 15_000 });
  await item.click();
  await page.getByRole("heading", { name: title, exact: true }).waitFor({ state: "visible", timeout: 15_000 });
  return page.locator(".nd-result-table tbody tr").count();
}

async function openCalculationFromToolbar() {
  const command = page.locator(".nd-commandbar button").filter({ hasText: /^Calculate/i });
  if (await command.count() !== 1) throw new Error("The active workspace did not expose exactly one generic Calculate command.");
  await command.click();
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.getByRole("listbox", { name: "Available calculation methods", exact: true })
    .getByRole("option").first().waitFor({ state: "visible", timeout: 10_000 });
  return dialog;
}

async function openAnalysisFromDataToolbar() {
  const command = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Analyze(?:\u2026|\.\.\.)?$/i });
  const calculateCommands = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Calculate(?:\u2026|\.\.\.)?$/i });
  if (await command.count() !== 1 || await calculateCommands.count() !== 0) {
    throw new Error("The data-only workspace did not expose exactly one shared Analyze command without a duplicate Calculate command.");
  }
  await command.click();
  const dialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.getByRole("listbox", { name: "Available calculation methods", exact: true })
    .getByRole("option").first().waitFor({ state: "visible", timeout: 10_000 });
  return dialog;
}

async function seedRecentProject(project) {
  if (!recentProjectsSeeded) {
    priorRecentProjectsRaw = await page.evaluate((key) => window.localStorage.getItem(key), recentProjectsKey);
  }
  await page.evaluate(({ key, project }) => {
    let existing = [];
    try {
      const parsed = JSON.parse(window.localStorage.getItem(key) ?? "[]");
      if (Array.isArray(parsed)) existing = parsed;
    } catch {
      existing = [];
    }
    const normalized = project.path.toLocaleLowerCase();
    window.localStorage.setItem(key, JSON.stringify([
      project,
      ...existing.filter((entry) => typeof entry?.path === "string" && entry.path.toLocaleLowerCase() !== normalized),
    ].slice(0, 8)));
  }, { key: recentProjectsKey, project });
  recentProjectsSeeded = true;
}

async function seedDisposableRecentProject() {
  await seedRecentProject({
    name: disposableProjectName,
    path: disposableProjectPath,
    openedAt: "2026-08-10T00:00:00.000Z",
  });
}

async function reloadToLauncher() {
  await page.reload({ waitUntil: "domcontentloaded", timeout: 30_000 });
  await page.locator(".nd-app[data-native-desktop-shell='true']").waitFor({ state: "visible", timeout: 15_000 });
  await waitForSurface("launcher");
}

function exactRecentProjectRow(projectName, projectPath) {
  if (!projectPath) {
    throw new Error(`Recent-project selection for ${projectName} requires an exact project path.`);
  }
  return page.locator(".nd-recent-projects .nd-project-row").filter({
    has: page.locator("strong").filter({ hasText: exactVisibleText(projectName) }),
  }).filter({
    has: page.locator("small").filter({ hasText: exactVisibleText(projectPath) }),
  });
}

async function openRecentProject(projectName, projectPath) {
  const row = exactRecentProjectRow(projectName, projectPath);
  await row.waitFor({ state: "visible", timeout: 10_000 });
  if (await row.count() !== 1) {
    throw new Error(`${projectName} at ${projectPath} was not exposed as exactly one visible Recent Projects row.`);
  }
  await row.click();
  await page.locator(".nd-window-project").filter({ hasText: projectName }).waitFor({ state: "visible", timeout: 15_000 });
}

async function openProjectAtExactPath(projectName, projectPath) {
  await page.evaluate(({ path }) => {
    window.dispatchEvent(new CustomEvent("quickpls:open-project-path", { detail: { path } }));
  }, { path: projectPath });
  await page.locator(".nd-window-project").filter({ hasText: projectName })
    .waitFor({ state: "visible", timeout: 30_000 });
}

async function createExactCbsemSchema6Copy(sourceArchivePath, destinationArchivePath) {
  const outcome = await page.evaluate(async ({ sourceArchivePath, destinationArchivePath }) => {
    const invoke = window.__TAURI_INTERNALS__?.invoke;
    if (typeof invoke !== "function") return { error: "Tauri invoke is unavailable." };
    const access = { surface: "internal_labs", experimentalLabsEnabled: true };
    const inspection = await invoke("inspect_internal_project_upgrade_v6", {
      request: { ...access, sourceArchivePath },
    });
    if (inspection?.status !== "ok") return { error: "inspection_blocked", inspection };
    const planRequest = {
      ...access,
      sourceArchivePath,
      destinationArchivePath,
      expectedSourceArchiveSha256: inspection.value.sourceArchiveSha256,
      legacyDisplayCovariances: {},
      estimandConfirmations: {},
    };
    let plan = await invoke("plan_internal_project_upgrade_v6", { request: planRequest });
    if (plan?.status === "ok" && plan.value?.state === "confirmation_required") {
      planRequest.estimandConfirmations = Object.fromEntries(
        plan.value.prompts.map((prompt) => [prompt.modelId, "common_factor"]),
      );
      plan = await invoke("plan_internal_project_upgrade_v6", { request: planRequest });
    }
    if (plan?.status !== "ok" || plan.value?.state !== "ready") return { error: "plan_not_ready", inspection, plan };
    const execution = await invoke("execute_internal_project_upgrade_v6", {
      request: {
        ...access,
        planId: plan.value.planId,
        expectedPlanSha256: plan.value.planSha256,
        confirmNewDestination: true,
      },
    });
    if (execution?.status !== "ok") return { error: "execution_blocked", inspection, plan, execution };
    const destinationInspection = await invoke("inspect_internal_project_upgrade_v6", {
      request: { ...access, sourceArchivePath: destinationArchivePath },
    });
    return { inspection, plan, execution, destinationInspection };
  }, { sourceArchivePath, destinationArchivePath });
  if (outcome.error || outcome.destinationInspection?.status !== "ok"
    || outcome.destinationInspection.value?.access !== "current_v6_archive") {
    throw new Error(`Exact-CB schema-6 copy could not be created and inspected: ${JSON.stringify(outcome)}`);
  }
  return outcome;
}

async function openDisposableRecentProject() {
  await openRecentProject(disposableProjectName, disposableProjectPath);
}

function exactVisibleText(value) {
  const escaped = value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return new RegExp(`^\\s*${escaped}\\s*$`);
}

function compactVisibleText(value) {
  return String(value ?? "").replace(/\s+/g, " ").trim();
}

function workspaceTreeItem(kind, label) {
  const item = page.locator(`.nd-project-treeitem[data-kind="${kind}"]`);
  return label ? item.filter({ hasText: exactVisibleText(label) }) : item;
}

async function currentModelDocumentName() {
  const title = page.locator(".nd-model-document-title");
  await title.waitFor({ state: "visible", timeout: 15_000 });
  return (await title.textContent())?.trim() ?? "";
}

async function openWorkspaceExplorer(projectName = disposableProjectName) {
  await openMenuItem("View", "Project");
  await waitForSurface("launcher");
  await page.locator(".nd-workspace-explorer").waitFor({ state: "visible", timeout: 15_000 });
  return page.getByRole("tree", { name: `${projectName} project contents`, exact: true });
}

async function submitNamedExplorerDialog(title, value, actionLabel) {
  const dialog = page.getByRole("dialog", { name: title, exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.getByLabel("Name", { exact: true }).fill(value);
  await dialog.getByRole("button", { name: actionLabel, exact: true }).click();
  await dialog.waitFor({ state: "hidden", timeout: 15_000 });
}

async function confirmExplorerRemoval(title, actionLabel) {
  const dialog = page.getByRole("dialog", { name: title, exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  await dialog.getByRole("button", { name: actionLabel, exact: true }).click();
  await dialog.waitFor({ state: "hidden", timeout: 15_000 });
}

async function createInitialEditableModel(projectName, modelName) {
  const tree = await openWorkspaceExplorer(projectName);
  await tree.waitFor({ state: "visible", timeout: 15_000 });
  const importedEditableModels = await workspaceTreeItem("model").count();
  if (importedEditableModels !== 0) {
    throw new Error(`${projectName} unexpectedly exposed ${importedEditableModels} editable models before native authoring.`);
  }
  await workspaceTreeItem("models", "Models").click();
  const newModelCommand = page.locator(".nd-explorer-detail-actions").getByRole("button", { name: "New Model", exact: true });
  if (!await newModelCommand.isEnabled()) {
    throw new Error(`${projectName} did not expose an enabled New Model action in Project Explorer.`);
  }
  await newModelCommand.click();
  await submitNamedExplorerDialog("New Model", modelName, "Create");
  await waitForSurface("model");
  await page.locator(".react-flow__pane").waitFor({ state: "visible", timeout: 15_000 });
  const created = {
    projectName,
    importedEditableModels,
    name: await currentModelDocumentName(),
    constructs: await page.locator(".react-flow__node-latent").count(),
    structuralPaths: await structuralPaths().count(),
  };
  if (created.name !== modelName || created.constructs !== 0 || created.structuralPaths !== 0) {
    throw new Error(`Project Explorer did not create the expected empty ${modelName} model: ${JSON.stringify(created)}`);
  }
  return created;
}

async function clickIndicator(name) {
  const indicator = page.locator(".nd-variable-item").filter({ hasText: new RegExp(`^${name}$`) });
  await indicator.waitFor({ state: "visible", timeout: 10_000 });
  if (await indicator.count() !== 1) throw new Error(`Expected exactly one visible ${name} indicator in the Model navigator.`);
  await indicator.click();
}

async function clearModelSelection() {
  const pane = page.locator(".react-flow__pane");
  await pane.waitFor({ state: "visible", timeout: 10_000 });
  const box = await pane.boundingBox();
  if (!box) throw new Error("The model canvas pane did not expose screen bounds.");
  await pane.click({ position: { x: Math.max(8, box.width - 24), y: 24 } });
  await page.locator(".react-flow__node-latent.selected").waitFor({ state: "hidden", timeout: 5_000 }).catch(() => undefined);
}

function structuralPaths() {
  return page.locator('.react-flow__edge[data-id]:not([data-id^="measurement::"])');
}

function modelInspector() {
  return page.locator("aside.nd-model-inspector");
}

async function selectModelInspectorTab(label) {
  const tab = modelInspector().getByRole("tab", { name: label, exact: true });
  await tab.waitFor({ state: "visible", timeout: 5_000 });
  if (await tab.getAttribute("aria-selected") !== "true") await tab.click();
}

async function setSelectedMeasurementMode(label) {
  await selectModelInspectorTab("Parameter");
  const option = modelInspector().getByLabel(label, { exact: true });
  await option.waitFor({ state: "visible", timeout: 5_000 });
  await option.check();
}

async function buildTwoConstructModel({ firstIndicatorAlreadyAssigned = false } = {}) {
  if (!firstIndicatorAlreadyAssigned) {
    await clickIndicator("x1");
    await page.locator(".react-flow__node-latent").nth(0).waitFor({ state: "visible", timeout: 10_000 });
  } else {
    const firstNode = page.locator(".react-flow__node-latent").nth(0);
    await firstNode.waitFor({ state: "visible", timeout: 10_000 });
    await firstNode.dispatchEvent("click");
  }
  await clickIndicator("x2");
  await clearModelSelection();
  await clickIndicator("y1");
  await page.locator(".react-flow__node-latent").nth(1).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("y2");

  const nodes = page.locator(".react-flow__node-latent");
  if (await nodes.count() !== 2) throw new Error("Visible indicator actions did not create exactly two constructs.");
  const pathCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Path$/ });
  if (await pathCommand.count() !== 1) throw new Error("The Model toolbar did not expose exactly one Path command.");
  await pathCommand.click();
  await nodes.nth(0).dispatchEvent("click");
  await nodes.nth(1).dispatchEvent("click");
  await structuralPaths().first().waitFor({ state: "attached", timeout: 10_000 });
  if (await structuralPaths().count() !== 1) throw new Error("The visible Path workflow did not create exactly one structural path.");
}

async function buildProspectivePlsPowerModel() {
  const nodes = page.locator(".react-flow__node-latent");
  for (const [index, definition] of [
    { indicators: ["x1", "x2", "x3"], name: "Predictor", shortName: "X" },
    { indicators: ["y1", "y2", "y3"], name: "Outcome", shortName: "Y" },
  ].entries()) {
    await clickIndicator(definition.indicators[0]);
    await nodes.nth(index).waitFor({ state: "visible", timeout: 10_000 });
    for (const indicator of definition.indicators.slice(1)) await clickIndicator(indicator);
    await renameSelectedConstruct(definition.name, definition.shortName);
    await clearModelSelection();
  }
  await createStructuralPath(nodes, 0, 1, 1);
  const contract = {
    constructs: await nodes.count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    labels: (await nodes.allTextContents()).map(compactVisibleText),
  };
  if (contract.constructs !== 2 || contract.assignedIndicators !== 6 || contract.structuralPaths !== 1
    || !contract.labels.some((label) => /Predictor/.test(label))
    || !contract.labels.some((label) => /Outcome/.test(label))) {
    throw new Error(`Prospective PLS-power authoring did not retain its two-construct, six-indicator, one-path design: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function buildCtaPlsModel() {
  const nodes = page.locator(".react-flow__node-latent");
  await clickIndicator("x1");
  await nodes.nth(0).waitFor({ state: "visible", timeout: 10_000 });
  for (const indicator of ["x2", "x3", "x4"]) await clickIndicator(indicator);
  await renameSelectedConstruct("Predictor", "X");
  await clearModelSelection();
  await clickIndicator("y1");
  await nodes.nth(1).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("y2");
  await renameSelectedConstruct("Outcome", "Y");
  await clearModelSelection();

  if (await nodes.count() !== 2) throw new Error("CTA-PLS authoring did not create exactly two constructs.");
  await createStructuralPath(nodes, 0, 1, 1);
  const constructIds = await nodes.evaluateAll((elements) => elements.map((element) => element.getAttribute("data-id")));
  const contract = {
    constructs: await nodes.count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    constructIds,
    labels: (await nodes.allTextContents()).map(compactVisibleText),
  };
  if (contract.constructs !== 2 || contract.assignedIndicators !== 6 || contract.structuralPaths !== 1
    || constructIds.some((id) => !id) || !contract.labels.some((label) => /Predictor/.test(label))
    || !contract.labels.some((label) => /Outcome/.test(label))) {
    throw new Error(`CTA-PLS model authoring did not retain the exact two-construct, six-indicator, one-path contract: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function buildTwoConstructGscaModel() {
  const nodes = page.locator(".react-flow__node-latent");
  await clickIndicator("g1");
  await nodes.nth(0).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("g2");
  await clickIndicator("g3");
  await renameSelectedConstruct("G formative component", "G");
  await setSelectedMeasurementMode("Formative");
  await clearModelSelection();

  await clickIndicator("h1");
  await nodes.nth(1).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("h2");
  await renameSelectedConstruct("H reflective component", "H");
  await setSelectedMeasurementMode("Reflective");
  await clearModelSelection();

  if (await nodes.count() !== 2) throw new Error("Visible indicator actions did not create the two GSCA components.");
  await createStructuralPath(nodes, 0, 1, 1);
}

async function createStructuralPath(nodes, sourceIndex, targetIndex, expectedPathCount) {
  const pathCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Path$/ });
  if (await pathCommand.count() !== 1) throw new Error("The Model toolbar did not expose exactly one Path command.");
  await pathCommand.click();
  await nodes.nth(sourceIndex).dispatchEvent("click");
  await nodes.nth(targetIndex).dispatchEvent("click");
  await structuralPaths().nth(expectedPathCount - 1).waitFor({ state: "attached", timeout: 10_000 });
  if (await structuralPaths().count() !== expectedPathCount) {
    throw new Error(`The visible Path workflow did not create exactly ${expectedPathCount} structural path(s).`);
  }
}

async function buildThreeConstructMediationModel() {
  await clickIndicator("x1");
  await page.locator(".react-flow__node-latent").nth(0).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("x2");
  await clearModelSelection();

  await clickIndicator("m1");
  await page.locator(".react-flow__node-latent").nth(1).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("m2");
  await clearModelSelection();

  await clickIndicator("y1");
  await page.locator(".react-flow__node-latent").nth(2).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("y2");

  const nodes = page.locator(".react-flow__node-latent");
  if (await nodes.count() !== 3) throw new Error("Visible indicator actions did not create exactly three mediation constructs.");
  await createStructuralPath(nodes, 0, 1, 1);
  await createStructuralPath(nodes, 1, 2, 2);
}

async function buildThreeConstructMgaModel() {
  const definitions = [
    { indicators: ["x1", "x2"], name: "X" },
    { indicators: ["z1", "z2"], name: "Z" },
    { indicators: ["y1", "y2"], name: "Y" },
  ];
  const nodes = page.locator(".react-flow__node-latent");
  for (let index = 0; index < definitions.length; index += 1) {
    const definition = definitions[index];
    await clickIndicator(definition.indicators[0]);
    await nodes.nth(index).waitFor({ state: "visible", timeout: 10_000 });
    await clickIndicator(definition.indicators[1]);
    await renameSelectedConstruct(definition.name, definition.name);
    await nodes.nth(index).filter({ hasText: definition.name }).waitFor({ state: "visible", timeout: 5_000 });
    await clearModelSelection();
  }
  if (await nodes.count() !== 3) throw new Error("Visible indicator actions did not create exactly the X, Z, and Y MGA constructs.");
  await createStructuralPath(nodes, 0, 2, 1);
  await createStructuralPath(nodes, 1, 2, 2);
}

async function buildThreeConstructCcaModel() {
  const definitions = [
    { indicators: ["x1", "x2"], name: "X" },
    { indicators: ["z1", "z2"], name: "Z" },
    { indicators: ["y1", "y2"], name: "Y" },
  ];
  const nodes = page.locator(".react-flow__node-latent");
  for (let index = 0; index < definitions.length; index += 1) {
    const definition = definitions[index];
    await clickIndicator(definition.indicators[0]);
    await nodes.nth(index).waitFor({ state: "visible", timeout: 10_000 });
    await clickIndicator(definition.indicators[1]);
    await renameSelectedConstruct(definition.name, definition.name);
    await nodes.nth(index).filter({ hasText: definition.name }).waitFor({ state: "visible", timeout: 5_000 });
    await clearModelSelection();
  }
  if (await nodes.count() !== 3) throw new Error("Visible indicator actions did not create exactly the X, Z, and Y CCA constructs.");
  await createStructuralPath(nodes, 0, 1, 1);
  await createStructuralPath(nodes, 1, 2, 2);
}

async function buildThreeConstructCbsemModel() {
  const definitions = [
    { indicators: ["x1", "x2", "x3"], name: "X" },
    { indicators: ["m1", "m2", "m3"], name: "M" },
    { indicators: ["y1", "y2", "y3"], name: "Y" },
  ];
  const nodes = page.locator(".react-flow__node-latent");
  for (let index = 0; index < definitions.length; index += 1) {
    const definition = definitions[index];
    await clickIndicator(definition.indicators[0]);
    await nodes.nth(index).waitFor({ state: "visible", timeout: 10_000 });
    for (const indicator of definition.indicators.slice(1)) await clickIndicator(indicator);
    await renameSelectedConstruct(definition.name, definition.name);
    await nodes.nth(index).filter({ hasText: definition.name }).waitFor({ state: "visible", timeout: 5_000 });
    await clearModelSelection();
  }
  if (await nodes.count() !== 3) throw new Error("Visible indicator actions did not create exactly the X, M, and Y CB-SEM factors.");
  await createStructuralPath(nodes, 0, 1, 1);
  await createStructuralPath(nodes, 1, 2, 2);
}

async function confirmSelectedConstructAsCommonFactor() {
  const expert = modelInspector().getByRole("button", { name: "Expert", exact: true });
  await expert.waitFor({ state: "visible", timeout: 10_000 });
  if (await expert.getAttribute("aria-pressed") !== "true") await expert.click();
  await selectModelInspectorTab("Parameter");
  const representation = modelInspector().getByLabel("Representation", { exact: true });
  await representation.waitFor({ state: "visible", timeout: 10_000 });
  await representation.selectOption("common_factor");
  await page.waitForFunction(() => {
    const select = document.querySelector('aside.nd-model-inspector select[id$="-representation"]');
    return select?.value === "common_factor";
  }, null, { timeout: 10_000 });
  return {
    representation: await representation.inputValue(),
    marker: await modelInspector().getByLabel("Marker indicator", { exact: true }).inputValue(),
  };
}

async function buildOneFactorExactCbsemModel() {
  const nodes = page.locator(".react-flow__node-latent");
  await clickIndicator("g1");
  await nodes.nth(0).waitFor({ state: "visible", timeout: 10_000 });
  await clickIndicator("g2");
  await clickIndicator("g3");
  await renameSelectedConstruct("Exact CFA factor", "F");
  const scientific = await confirmSelectedConstructAsCommonFactor();
  await clearModelSelection();
  const contract = {
    constructs: await nodes.count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    label: compactVisibleText(await nodes.nth(0).textContent()),
    scientific,
  };
  if (contract.constructs !== 1 || contract.assignedIndicators !== 3 || contract.structuralPaths !== 0
    || !contract.label.includes("Exact CFA factor") || contract.scientific.representation !== "common_factor"
    || contract.scientific.marker !== "g1") {
    throw new Error(`Exact-CB authoring did not retain the one-factor, three-indicator CFA contract: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function buildSixConstructIpmaModelWithDisconnectedBranch() {
  const definitions = [
    { indicator: "x1", name: "X" },
    { indicator: "z1", name: "Z" },
    { indicator: "m1", name: "M" },
    { indicator: "y1", name: "Y" },
    { indicator: "u1", name: "U" },
    { indicator: "v1", name: "V" },
  ];
  const nodes = page.locator(".react-flow__node-latent");
  const constructIds = {};
  for (let index = 0; index < definitions.length; index += 1) {
    const definition = definitions[index];
    await clickIndicator(definition.indicator);
    const node = nodes.nth(index);
    await node.waitFor({ state: "visible", timeout: 10_000 });
    await renameSelectedConstruct(definition.name, definition.name);
    await node.filter({ hasText: definition.name }).waitFor({ state: "visible", timeout: 5_000 });
    const id = await node.getAttribute("data-id");
    if (!id) throw new Error(`The visible ${definition.name} IPMA construct did not expose an immutable identifier.`);
    constructIds[definition.name.toLocaleLowerCase()] = id;
    await clearModelSelection();
  }
  if (await nodes.count() !== 6) throw new Error("Visible indicator actions did not create exactly the X, Z, M, Y, U, and V IPMA constructs.");
  await createStructuralPath(nodes, 0, 2, 1);
  await createStructuralPath(nodes, 1, 2, 2);
  await createStructuralPath(nodes, 0, 3, 3);
  await createStructuralPath(nodes, 1, 3, 4);
  await createStructuralPath(nodes, 2, 3, 5);
  await createStructuralPath(nodes, 4, 5, 6);
  return { constructIds };
}

async function renameSelectedConstruct(name, shortName) {
  await selectModelInspectorTab("Model");
  const properties = modelInspector();
  const nameInput = properties.getByLabel("Name", { exact: true });
  const shortNameInput = properties.getByLabel("Short name", { exact: true });
  await nameInput.waitFor({ state: "visible", timeout: 5_000 });
  await nameInput.fill(name);
  await nameInput.press("Enter");
  await shortNameInput.fill(shortName);
  await shortNameInput.press("Enter");
  await page.waitForFunction(({ name, shortName }) => {
    const propertiesPane = document.querySelector("aside.nd-model-inspector");
    const inputs = propertiesPane?.querySelectorAll('input[type="text"]') ?? [];
    return inputs[0]?.value === name && inputs[1]?.value === shortName;
  }, { name, shortName }, { timeout: 5_000 });
}

async function buildThreeConstructModerationModel() {
  const definitions = [
    { indicator: "x", name: "X", shortName: "X" },
    { indicator: "m", name: "M", shortName: "M" },
    { indicator: "y", name: "Y", shortName: "Y" },
  ];
  const nodes = page.locator(".react-flow__node-latent");
  for (let index = 0; index < definitions.length; index += 1) {
    const definition = definitions[index];
    await clickIndicator(definition.indicator);
    await nodes.nth(index).waitFor({ state: "visible", timeout: 10_000 });
    await renameSelectedConstruct(definition.name, definition.shortName);
    await nodes.nth(index).filter({ hasText: definition.name }).waitFor({ state: "visible", timeout: 5_000 });
    await clearModelSelection();
  }
  if (await nodes.count() !== 3) throw new Error("Visible indicator actions did not create exactly the X, M, and Y constructs.");
  const nodeIds = await nodes.evaluateAll((elements) => elements.map((element) => element.getAttribute("data-id")));
  if (nodeIds.some((id) => !id)) throw new Error(`The X, M, and Y model nodes did not expose stable React Flow identifiers: ${JSON.stringify(nodeIds)}`);
  await createStructuralPath(nodes, 0, 2, 1);
  const basePathId = await structuralPaths().first().getAttribute("data-id");
  if (!basePathId) throw new Error("The visible X-to-Y structural relationship had no React Flow identifier.");
  return { nodes, xId: nodeIds[0], mId: nodeIds[1], yId: nodeIds[2], basePathId };
}

async function buildThreeConstructHigherOrderModel() {
  const definitions = [
    { indicator: "x1", name: "Capability", shortName: "CAP" },
    { indicator: "z1", name: "Resources", shortName: "RES" },
    { indicator: "y1", name: "Performance", shortName: "PERF" },
  ];
  const nodes = page.locator(".react-flow__node-latent");
  const ids = {};
  for (let index = 0; index < definitions.length; index += 1) {
    const definition = definitions[index];
    await clickIndicator(definition.indicator);
    const node = nodes.nth(index);
    await node.waitFor({ state: "visible", timeout: 10_000 });
    await renameSelectedConstruct(definition.name, definition.shortName);
    await node.filter({ hasText: definition.name }).waitFor({ state: "visible", timeout: 5_000 });
    const id = await node.getAttribute("data-id");
    if (!id) throw new Error(`The visible ${definition.name} construct did not expose an immutable identifier.`);
    ids[definition.name.toLocaleLowerCase()] = id;
    await clearModelSelection();
  }
  if (await nodes.count() !== 3 || await structuralPaths().count() !== 0) {
    throw new Error("Visible indicator actions did not create exactly three measurement-only HOC input constructs.");
  }
  return { nodes, ids };
}

async function selectVisibleStructuralPath(edge) {
  await edge.waitFor({ state: "attached", timeout: 10_000 });
  if (await edge.count() !== 1) throw new Error("Expected exactly one visible structural relationship for path selection.");
  const hitTarget = edge.locator(".react-flow__edge-interaction");
  if (await hitTarget.count() !== 1) throw new Error("The structural relationship did not expose one React Flow interaction target.");
  const edgeId = await edge.getAttribute("data-id");
  if (!edgeId) throw new Error("The structural relationship did not expose a React Flow identifier for selection.");
  await hitTarget.dispatchEvent("click");
  await page.waitForFunction((selectedEdgeId) => document.querySelector(`.react-flow__edge[data-id="${CSS.escape(selectedEdgeId)}"]`)?.classList.contains("selected"), edgeId, { timeout: 5_000 });
}

async function inspectVisibleStructuralPath(edge) {
  await selectVisibleStructuralPath(edge);
  const properties = modelInspector().locator(".nd-property-list").first();
  await properties.waitFor({ state: "visible", timeout: 5_000 });
  return properties.evaluate((element) => Object.fromEntries(Array.from(element.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
}

const mediationBaseTableTitles = [
  "Direct effects",
  "Specific indirect effects",
  "Total indirect effects",
  "Total effects",
];
const mediationBootstrapTableTitle = "Aggregate mediation effects bootstrap inference";

async function inspectMediationResultTree({ withBootstrap }) {
  const group = page.getByRole("treeitem", { name: "Mediation", exact: true });
  await group.waitFor({ state: "visible", timeout: 15_000 });
  if (await group.getAttribute("aria-expanded") === "false") await group.click();

  const requiredTitles = withBootstrap
    ? [...mediationBaseTableTitles, mediationBootstrapTableTitle]
    : mediationBaseTableTitles;
  const rowCounts = {};
  const tableText = {};
  for (const title of requiredTitles) {
    rowCounts[title] = await openResultTable(title);
    tableText[title] = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
    if (!rowCounts[title] || !tableText[title]) throw new Error(`The native ${title} mediation table was empty.`);
  }

  const bootstrapTreeItems = await page.getByRole("treeitem", { name: mediationBootstrapTableTitle, exact: true }).count();
  if (withBootstrap && bootstrapTreeItems !== 1) {
    throw new Error(`The completed Bootstrap run exposed ${bootstrapTreeItems} Aggregate mediation effects bootstrap inference tree items instead of one.`);
  }
  if (!withBootstrap && bootstrapTreeItems !== 0) {
    throw new Error("The non-resampled PLS run exposed fabricated mediation bootstrap inference.");
  }

  await openResultTable("Specific indirect effects");
  return {
    groupTitle: (await group.textContent())?.trim() ?? "",
    requiredTitles,
    treeItems: (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map((label) => label.trim()),
    rowCounts,
    tableText,
    bootstrapTreeItems,
    selectedTable: (await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').textContent())?.trim() ?? "",
  };
}

const moderationBaseTableTitles = ["Moderation effects", "Simple slope analysis"];
const moderationBootstrapTableTitle = "Interaction effect bootstrap inference";

async function inspectModerationResultTree({ withBootstrap }) {
  const group = page.getByRole("treeitem", { name: "Moderation", exact: true });
  await group.waitFor({ state: "visible", timeout: 15_000 });
  if (await group.getAttribute("aria-expanded") === "false") await group.click();

  const requiredTitles = withBootstrap
    ? [...moderationBaseTableTitles, moderationBootstrapTableTitle]
    : moderationBaseTableTitles;
  const rowCounts = {};
  const tableText = {};
  for (const title of requiredTitles) {
    rowCounts[title] = await openResultTable(title);
    tableText[title] = (await page.locator(".nd-result-table tbody").textContent())?.replace(/\s+/g, " ").trim() ?? "";
    if (!rowCounts[title] || !tableText[title] || /\bN\/A\b/i.test(tableText[title])) {
      throw new Error(`The native ${title} moderation table was empty or contained placeholder N/A output: ${tableText[title]}`);
    }
  }

  const bootstrapTreeItems = await page.getByRole("treeitem", { name: moderationBootstrapTableTitle, exact: true }).count();
  if (withBootstrap && bootstrapTreeItems !== 1) {
    throw new Error(`The completed moderation Bootstrap run exposed ${bootstrapTreeItems} interaction-effect inference tree items instead of one.`);
  }
  if (!withBootstrap && bootstrapTreeItems !== 0) {
    throw new Error("The non-resampled moderation PLS run exposed fabricated interaction-effect bootstrap inference.");
  }

  await openResultTable("Simple slope analysis");
  const plot = page.locator(".nd-moderation-plot");
  const plotContract = {
    figures: await plot.count(),
    accessibleSvgs: await plot.locator('svg[role="img"][aria-labelledby]').count(),
    lines: await plot.locator("polyline.slope").count(),
    points: await plot.locator("circle").count(),
    caption: (await plot.locator("figcaption").textContent())?.replace(/\s+/g, " ").trim() ?? "",
  };
  if (plotContract.figures !== 1 || plotContract.accessibleSvgs !== 1 || plotContract.lines !== 1 || plotContract.points < 3 || !/Conditional effect plot/i.test(plotContract.caption)) {
    throw new Error(`Simple slope analysis did not expose one accessible conditional-effect plot backed by the reported slope points: ${JSON.stringify(plotContract)}`);
  }

  return {
    groupTitle: (await group.textContent())?.trim() ?? "",
    requiredTitles,
    treeItems: (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map((label) => label.trim()),
    rowCounts,
    tableText,
    bootstrapTreeItems,
    plot: plotContract,
    selectedTable: (await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').textContent())?.trim() ?? "",
  };
}

const mgaTableContracts = [
  {
    title: "Two-group sample summary",
    rows: 2,
    columns: ["Group column", "Role", "Group value", "Analyzed observations"],
  },
  {
    title: "MICOM invariance summary",
    rows: 3,
    columns: ["Construct", "Configural", "Compositional", "Partial invariance", "Equal means", "Equal variances", "Full invariance", "Confidence", "Usable permutations"],
  },
  {
    title: "MICOM Step 1 - configural invariance",
    rows: 3,
    columns: ["Construct", "Configural invariance"],
  },
  {
    title: "MICOM Step 2 - compositional invariance",
    rows: 3,
    columns: ["Construct", "Original correlation", "Lower confidence bound", "Permutation p", "Compositional invariance"],
  },
  {
    title: "MICOM Step 3 - equality of composite means",
    rows: 3,
    columns: ["Construct", "Mean A", "Mean B", "Mean difference (A - B)", "Lower confidence bound", "Upper confidence bound", "Two-tailed p", "Equal means"],
  },
  {
    title: "MICOM Step 3 - equality of composite variances",
    rows: 3,
    columns: ["Construct", "Variance A", "Variance B", "Log variance ratio (A/B)", "Lower confidence bound", "Upper confidence bound", "Two-tailed p", "Equal variances"],
  },
  {
    title: "MICOM permutation accounting",
    rows: 10,
    columns: ["Field", "Value"],
  },
  {
    title: "Group path coefficients",
    rows: 4,
    columns: ["Role", "Group value", "Path", "Coefficient"],
  },
  {
    title: "Group R-square",
    rows: 2,
    columns: ["Role", "Group value", "Construct", "R²"],
  },
  {
    title: "Group outer loadings",
    rows: 12,
    columns: ["Role", "Group value", "Construct", "Indicator", "Outer loading"],
  },
  {
    title: "Group outer weights",
    rows: 12,
    columns: ["Role", "Group value", "Construct", "Indicator", "Outer weight"],
  },
  {
    title: "Group A minus Group B path differences",
    rows: 2,
    columns: ["Path", "Group A", "Coefficient A", "Group B", "Coefficient B", "A − B"],
  },
  {
    title: "Two-tailed permutation path differences",
    rows: 2,
    columns: ["Path", "A − B", "Two-tailed p", "Percentile rank", "Requested permutations", "Usable permutations"],
  },
  {
    title: "Group A minus Group B outer-loading differences",
    rows: 6,
    columns: ["Construct", "Indicator", "Group A", "Outer loading A", "Group B", "Outer loading B", "A - B"],
  },
  {
    title: "Group A minus Group B outer-weight differences",
    rows: 6,
    columns: ["Construct", "Indicator", "Group A", "Outer weight A", "Group B", "Outer weight B", "A - B"],
  },
  {
    title: "Two-tailed permutation outer-loading differences",
    rows: 6,
    columns: ["Construct", "Indicator", "A - B", "Two-tailed p", "Percentile rank", "Requested permutations", "Usable permutations"],
  },
  {
    title: "Two-tailed permutation outer-weight differences",
    rows: 6,
    columns: ["Construct", "Indicator", "A - B", "Two-tailed p", "Percentile rank", "Requested permutations", "Usable permutations"],
  },
  {
    title: "Combined permutation plan and provenance",
    rows: 10,
    columns: ["Field", "Value"],
  },
];

async function inspectMgaResultTree(expectedPermutationSamples) {
  const group = page.getByRole("treeitem", { name: "Groups", exact: true });
  await group.waitFor({ state: "visible", timeout: 15_000 });
  if (await group.getAttribute("aria-expanded") === "false") await group.click();

  const tables = {};
  for (const contract of mgaTableContracts) {
    const rows = await openResultTable(contract.title);
    const headers = (await page.locator(".nd-result-table thead th").allTextContents()).map((value) => value.replace(/\s+/g, " ").trim());
    const rowValues = await page.locator(".nd-result-table tbody tr").evaluateAll((elements) => elements.map((row) => (
      Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
    )));
    const text = (await page.locator(".nd-result-table tbody").textContent())?.replace(/\s+/g, " ").trim() ?? "";
    const viewText = (await page.locator(".nd-result-table-view").textContent())?.replace(/\s+/g, " ").trim() ?? "";
    tables[contract.title] = { rows, headers, rowValues, text, viewText };
    if (rows !== contract.rows || JSON.stringify(headers) !== JSON.stringify(contract.columns) || !text || /\bN\/A\b/i.test(text)) {
      throw new Error(`The native ${contract.title} MGA table did not match its exact non-placeholder contract: ${JSON.stringify(tables[contract.title])}`);
    }
  }

  const sample = tables["Two-group sample summary"];
  const micomSummary = tables["MICOM invariance summary"];
  const micomConfigural = tables["MICOM Step 1 - configural invariance"];
  const micomComposition = tables["MICOM Step 2 - compositional invariance"];
  const micomMeans = tables["MICOM Step 3 - equality of composite means"];
  const micomVariances = tables["MICOM Step 3 - equality of composite variances"];
  const micomAccounting = tables["MICOM permutation accounting"];
  const paths = tables["Group path coefficients"];
  const loadings = tables["Group outer loadings"];
  const weights = tables["Group outer weights"];
  const rSquared = tables["Group R-square"];
  const differences = tables["Group A minus Group B path differences"];
  const loadingDifferences = tables["Group A minus Group B outer-loading differences"];
  const weightDifferences = tables["Group A minus Group B outer-weight differences"];
  const permutation = tables["Two-tailed permutation path differences"];
  const permutationLoadings = tables["Two-tailed permutation outer-loading differences"];
  const permutationWeights = tables["Two-tailed permutation outer-weight differences"];
  const combinedAccounting = tables["Combined permutation plan and provenance"];
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map((label) => label.replace(/\s+/g, " ").trim());
  const resultsWorkspaceText = (await page.locator(".nd-results-workspace").textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const allGroupTableText = Object.values(tables).map((table) => table.viewText).join(" ");
  const selectedTable = (await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const contract = {
    groupTitle: (await group.textContent())?.replace(/\s+/g, " ").trim() ?? "",
    selectedTable,
    treeItems,
    tables,
    noPooledDiagram: !treeItems.includes("Graphical results")
      && !treeItems.includes("Model estimates")
      && await page.locator(".nd-result-diagram-view").count() === 0,
    noApproximateNormalInference: !/(?:approximate normal|normal approximation|t[- ]?statistic|standard error)/i.test(`${allGroupTableText} ${resultsWorkspaceText}`),
    allRequiredTablesVisible: mgaTableContracts.every((table) => treeItems.includes(table.title)),
    noPlaceholderNa: !/\bN\/A\b/i.test(`${allGroupTableText} ${resultsWorkspaceText}`),
  };

  if (JSON.stringify(sample.rowValues) !== JSON.stringify([
    ["group", "Group A", "A", "90"],
    ["group", "Group B", "B", "90"],
  ])) {
    throw new Error(`The two-group sample summary did not expose both 90-case A/B samples: ${sample.text}`);
  }
  if (![paths.text, differences.text, permutation.text].every((text) => /X\s*(?:→|->)\s*Y/i.test(text) && /Z\s*(?:→|->)\s*Y/i.test(text))) {
    throw new Error(`The MGA structural tables did not expose both X -> Y and Z -> Y paths: ${JSON.stringify({ paths: paths.text, differences: differences.text, permutation: permutation.text })}`);
  }
  const expectedConstructs = ["X", "Y", "Z"];
  const expectedIndicators = ["x1", "x2", "y1", "y2", "z1", "z2"];
  const tableConstructs = (table) => [...new Set(table.rowValues.map((row) => row[0]))].sort();
  const measurementIndicators = (table, index) => [...new Set(table.rowValues.map((row) => row[index]))].sort();
  const micomConstructsValid = [micomSummary, micomConfigural, micomComposition, micomMeans, micomVariances]
    .every((table) => JSON.stringify(tableConstructs(table)) === JSON.stringify(expectedConstructs));
  const groupMeasurementRowsValid = [loadings, weights].every((table) => (
    JSON.stringify(measurementIndicators(table, 3)) === JSON.stringify(expectedIndicators)
    && table.rowValues.filter((row) => row[0] === "Group A" && row[1] === "A").length === 6
    && table.rowValues.filter((row) => row[0] === "Group B" && row[1] === "B").length === 6
  ));
  const differenceMeasurementRowsValid = [loadingDifferences, weightDifferences].every((table) => (
    JSON.stringify(measurementIndicators(table, 1)) === JSON.stringify(expectedIndicators)
    && table.rowValues.every((row) => row[2] === "A" && row[4] === "B")
  ));
  const micomSummaryValid = micomSummary.rowValues.every((row) => (
    row[1] === "Confirmed"
    && ["Established", "Not established"].includes(row[2])
    && row[3] === row[2]
    && ["Equal", "Different"].includes(row[4])
    && ["Equal", "Different"].includes(row[5])
    && ["Established", "Not established"].includes(row[6])
    && /^95(?:\.0)?%$/.test(row[7])
    && row[8] === String(expectedPermutationSamples)
  ));
  const micomConfiguralValid = micomConfigural.rowValues.every((row) => row[1] === "Confirmed");
  const visibleProbability = (value) => /^(?:<0\.0001|0(?:\.\d+)?|1(?:\.0+)?)$/.test(value);
  const micomCompositionValid = micomComposition.rowValues.every((row) => (
    parseVisibleNumber(row[1]) !== null
    && parseVisibleNumber(row[2]) !== null
    && visibleProbability(row[3])
    && ["Established", "Not established"].includes(row[4])
  ));
  const micomStep3Valid = [micomMeans, micomVariances].every((table) => table.rowValues.every((row) => (
    row.slice(1, 6).every((value) => parseVisibleNumber(value) !== null)
    && visibleProbability(row[6])
    && ["Equal", "Different"].includes(row[7])
  )));
  const accountingValues = (table) => Object.fromEntries(table.rowValues.map(([field, value]) => [field, value]));
  const micomAccountingValues = accountingValues(micomAccounting);
  const combinedAccountingValues = accountingValues(combinedAccounting);
  const accountingValid = micomAccountingValues["Requested permutations"] === String(expectedPermutationSamples)
    && micomAccountingValues["Attempted permutations"] === String(expectedPermutationSamples)
    && micomAccountingValues["Retry policy"] === "none"
    && micomAccountingValues["Ledger rows"] === String(expectedPermutationSamples)
    && /^sha256:[0-9a-f]{64}$/.test(micomAccountingValues["Permutation plan"] ?? "")
    && combinedAccountingValues["MGA method version"] === mgaMethodVersion
    && combinedAccountingValues["Permutation method version"] === mgaPermutationMethodVersion
    && combinedAccountingValues["MICOM method version"] === micomMethodVersion
    && combinedAccountingValues["Requested partitions"] === String(expectedPermutationSamples)
    && combinedAccountingValues["Attempted partitions"] === String(expectedPermutationSamples)
    && combinedAccountingValues["Retry policy"] === "none"
    && combinedAccountingValues["Partition plan digest"] === micomAccountingValues["Permutation plan"];
  const rSquaredConstructIndex = rSquared.headers.indexOf("Construct");
  const rSquaredOutcomeValid = rSquaredConstructIndex >= 0
    && rSquared.rowValues.length === 2
    && rSquared.rowValues.every((row) => row[rSquaredConstructIndex] === "Y");
  const permutationCountsValid = [permutation, permutationLoadings, permutationWeights].every((table) => {
    const requested = table.headers.indexOf("Requested permutations");
    const usable = table.headers.indexOf("Usable permutations");
    return requested >= 0 && usable >= 0 && table.rowValues.every((row) => (
      row[requested] === String(expectedPermutationSamples)
      && row[usable] === String(expectedPermutationSamples)
    ));
  });
  const permutationMeasurementRowsValid = [permutationLoadings, permutationWeights].every((table) => (
    JSON.stringify(measurementIndicators(table, 1)) === JSON.stringify(expectedIndicators)
    && table.rowValues.every((row) => visibleProbability(row[3]))
  ));
  if (!rSquaredOutcomeValid || !permutationCountsValid || !micomConstructsValid
    || !groupMeasurementRowsValid || !differenceMeasurementRowsValid || !permutationMeasurementRowsValid
    || !micomSummaryValid || !micomConfiguralValid || !micomCompositionValid || !micomStep3Valid || !accountingValid) {
    throw new Error(`The MICOM v4 and permutation MGA v4 tables did not expose complete, finite, fixed-plan group inference: ${JSON.stringify({ rSquared: rSquared.text, micomSummary: micomSummary.rowValues, permutationRows: permutation.rowValues, micomAccountingValues, combinedAccountingValues })}`);
  }
  if (!contract.noPooledDiagram || !contract.noApproximateNormalInference || !contract.allRequiredTablesVisible || !contract.noPlaceholderNa) {
    throw new Error(`The completed MICOM/MGA Results workspace omitted a required v4 table or exposed a pooled, approximate-normal, or placeholder surface: ${JSON.stringify(contract)}`);
  }
  await openResultTable("MICOM invariance summary");
  return contract;
}

function parseVisibleNumber(value) {
  const normalized = value.replace(/\u2212/g, "-").replace(/,/g, "").trim();
  const number = Number(normalized);
  return Number.isFinite(number) ? number : null;
}

async function inspectCcaResultTree() {
  const assessment = page.getByRole("treeitem", { name: "Assessment", exact: true });
  await assessment.waitFor({ state: "visible", timeout: 15_000 });
  if (await assessment.getAttribute("aria-expanded") === "false") await assessment.click();

  const initialSelectedTable = (await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const summaryRowCount = await openResultTable("Residual summary");
  const summaryHeaders = (await page.locator(".nd-result-table thead th").allTextContents()).map((value) => value.replace(/\s+/g, " ").trim());
  const summaryRows = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => (
    Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
  )));
  const summary = Object.fromEntries(summaryRows.map((row) => [row[0], row[1]]));

  const residualRowCount = await openResultTable("Composite residuals");
  const residualHeaders = (await page.locator(".nd-result-table thead th").allTextContents()).map((value) => value.replace(/\s+/g, " ").trim());
  const residualRows = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => (
    Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
  )));
  const pairLabels = residualRows.map((row) => row[0]).sort();
  const expectedPairLabels = ["X ↔ Y", "X ↔ Z", "Z ↔ Y"].sort();
  const finiteAndConsistent = residualRows.every((row) => {
    const [observed, reproduced, residual, absoluteResidual] = row.slice(1).map(parseVisibleNumber);
    return [observed, reproduced, residual, absoluteResidual].every((value) => value !== null)
      && Math.abs(observed - reproduced - residual) <= 0.000002
      && Math.abs(Math.abs(residual) - absoluteResidual) <= 0.000002;
  });
  const maximumAbsoluteResidual = parseVisibleNumber(summary["Maximum absolute residual"] ?? "");
  const detailMaximum = Math.max(...residualRows.map((row) => parseVisibleNumber(row[4]) ?? Number.NEGATIVE_INFINITY));
  const ctaOrInferenceTreeItems = await page.locator('.nd-result-tree [role="treeitem"]')
    .filter({ hasText: /threshold|classification|p-value|confidence interval|bootstrap/i }).count();
  const renderedCcaText = `${summaryRows.flat().join(" ")} ${residualRows.flat().join(" ")}`;
  const contract = {
    groupTitle: (await assessment.textContent())?.replace(/\s+/g, " ").trim() ?? "",
    initialSelectedTable,
    summary: { headers: summaryHeaders, rows: summaryRows, rowCount: summaryRowCount },
    residuals: { headers: residualHeaders, rows: residualRows, rowCount: residualRowCount },
    nestedModelLabel: summary.Model ?? "",
    correlationPairs: Number(summary["Correlation pairs"] ?? Number.NaN),
    maximumAbsoluteResidual,
    pairLabels,
    finiteAndConsistent,
    maximumMatchesRows: maximumAbsoluteResidual !== null && Number.isFinite(detailMaximum)
      && Math.abs(maximumAbsoluteResidual - detailMaximum) <= 0.000001,
    noInventedInferenceOrClassification: ctaOrInferenceTreeItems === 0
      && !/threshold|pass\/fail|fit classification|p[- ]?value|confidence interval|bootstrap/i.test(renderedCcaText),
  };

  if (initialSelectedTable !== "Residual summary"
    || summaryRowCount !== 3
    || JSON.stringify(summaryHeaders) !== JSON.stringify(["Metric", "Value"])
    || summary.Model !== "Recursive standardized composite path model"
    || summary["Correlation pairs"] !== "3"
    || maximumAbsoluteResidual === null
    || maximumAbsoluteResidual <= 0
    || residualRowCount !== 3
    || JSON.stringify(residualHeaders) !== JSON.stringify(["Composite pair", "Observed correlation", "Reproduced correlation", "Residual", "Absolute residual"])
    || JSON.stringify(pairLabels) !== JSON.stringify(expectedPairLabels)
    || !finiteAndConsistent
    || !contract.maximumMatchesRows
    || !contract.noInventedInferenceOrClassification) {
    throw new Error(`The completed CCA result did not match its exact finite descriptive residual contract: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectIpmaResultTree() {
  const group = page.getByRole("treeitem", { name: "Importance-performance map", exact: true });
  await group.waitFor({ state: "visible", timeout: 15_000 });
  if (await group.getAttribute("aria-expanded") === "false") await group.click();

  const initialSelectedTable = (await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]')
    .textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const readTable = async (title) => {
    const rowCount = await openResultTable(title);
    const headers = (await page.locator(".nd-result-table thead th").allTextContents())
      .map((value) => value.replace(/\s+/g, " ").trim());
    const rows = await page.locator(".nd-result-table tbody tr").evaluateAll((elements) => elements.map((row) => (
      Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
    )));
    return { title, rowCount, headers, rows };
  };

  const constructs = await readTable("Construct importance and performance");
  const plot = page.locator(".nd-ipma-plot");
  const plotSvg = plot.locator('svg[role="img"][aria-labelledby]');
  const directPlotTitles = plotSvg.locator(":scope > title");
  const directPlotTitleCount = await directPlotTitles.count();
  const captionTitles = plot.locator("figcaption > strong");
  const captionTargets = plot.locator("figcaption > span");
  const captionTitleCount = await captionTitles.count();
  const captionTargetCount = await captionTargets.count();
  const plotContract = {
    figures: await plot.count(),
    accessibleSvgs: await plotSvg.count(),
    directTitleCount: directPlotTitleCount,
    title: directPlotTitleCount === 1
      ? compactVisibleText(await directPlotTitles.textContent())
      : "",
    captionTitleCount,
    captionTargetCount,
    captionTitle: captionTitleCount === 1
      ? compactVisibleText(await captionTitles.textContent())
      : "",
    captionTarget: captionTargetCount === 1
      ? compactVisibleText(await captionTargets.textContent())
      : "",
    points: await plot.locator("circle").count(),
    pointLabels: (await plot.locator(".point-label").allTextContents()).map(compactVisibleText).sort(),
    scope: compactVisibleText(await plot.locator(":scope > p").textContent().catch(() => "")),
  };
  const indicators = await readTable("Indicator performance");
  const scope = await readTable("Analysis details");
  const scopeValues = Object.fromEntries(scope.rows.map((row) => [row[0], row[1]]));
  const constructLabels = constructs.rows.map((row) => row[1]).sort();
  const indicatorConstructLabels = indicators.rows.map((row) => row[1]).sort();
  const indicatorLabels = indicators.rows.map((row) => row[2]).sort();
  const excludedConstructLabels = ["U", "V", "Y"];
  const excludedIndicatorLabels = ["u1", "v1", "y1"];
  const constructValuesFinite = constructs.rows.every((row) => {
    const importance = parseVisibleNumber(row[2]);
    const performance = parseVisibleNumber(row[3]);
    return importance !== null && performance !== null && performance >= 0 && performance <= 100;
  });
  const indicatorValuesFinite = indicators.rows.every((row) => {
    const values = [row[3], row[4], row[5], row[6]].map(parseVisibleNumber);
    return values.every((value) => value !== null)
      && values[2] >= 0 && values[2] <= 100;
  });
  const renderedText = [constructs, indicators, scope]
    .flatMap((table) => [table.headers, ...table.rows]).flat().join(" ")
    + ` ${Object.values(plotContract).join(" ")}`;
  const inferenceTreeItems = await page.locator('.nd-result-tree [role="treeitem"]')
    .filter({ hasText: /bootstrap|permutation|confidence interval|p-value|cIPMA/i }).count();
  const contract = {
    groupTitle: compactVisibleText(await group.textContent()),
    initialSelectedTable,
    treeItems: (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText),
    constructs,
    indicators,
    scope,
    scopeValues,
    plot: plotContract,
    constructLabels,
    indicatorConstructLabels,
    indicatorLabels,
    constructValuesFinite,
    indicatorValuesFinite,
    predecessorOnly: JSON.stringify(constructLabels) === JSON.stringify(["M", "X", "Z"])
      && JSON.stringify(indicatorConstructLabels) === JSON.stringify(["M", "X", "Z"])
      && excludedConstructLabels.every((label) => !constructLabels.includes(label)
        && !indicatorConstructLabels.includes(label)),
    excludesTargetAndUnrelatedConstructRows: excludedConstructLabels.every((label) => !constructLabels.includes(label)),
    excludesTargetAndUnrelatedIndicatorRows: excludedConstructLabels.every((label) => !indicatorConstructLabels.includes(label))
      && excludedIndicatorLabels.every((label) => !indicatorLabels.includes(label)),
    noPlaceholderOrUnsupportedClaims: inferenceTreeItems === 0
      && !/\bN\/A\b|\bcIPMA\b|p[- ]?value|confidence interval|bootstrap|permutation/i.test(renderedText),
  };

  if (initialSelectedTable !== "Construct importance and performance"
    || constructs.rowCount !== 3
    || JSON.stringify(constructs.headers) !== JSON.stringify(["Target", "Predecessor construct", "Total importance", "Performance"])
    || !constructs.rows.every((row) => row[0] === "Y")
    || !contract.predecessorOnly
    || !constructValuesFinite
    || indicators.rowCount !== 3
    || JSON.stringify(indicators.headers) !== JSON.stringify(["Target", "Construct", "Indicator", "Construct importance", "Loading", "Performance", "Standardized score mean"])
    || !indicators.rows.every((row) => row[0] === "Y")
    || JSON.stringify(indicatorLabels) !== JSON.stringify(["m1", "x1", "z1"])
    || !indicatorValuesFinite
    || !contract.excludesTargetAndUnrelatedConstructRows
    || !contract.excludesTargetAndUnrelatedIndicatorRows
    || scope.rowCount !== 5
    || JSON.stringify(scope.headers) !== JSON.stringify(["Field", "Value"])
    || scopeValues.Target !== "Y"
    || scopeValues["Method version"] !== ipmaMethodVersion
    || !/observed-range|min(?:-|\s)?max/i.test(scopeValues.Performance ?? "")
    || scopeValues["Missing data"] !== "Listwise deletion"
    || scopeValues["Theoretical-range correction"] !== "Not applied"
    || plotContract.figures !== 1
    || plotContract.accessibleSvgs !== 1
    || plotContract.directTitleCount !== 1
    || plotContract.title !== "Importance-performance map for Y"
    || plotContract.captionTitleCount !== 1
    || plotContract.captionTargetCount !== 1
    || plotContract.captionTitle !== "Importance-performance map"
    || plotContract.captionTarget !== "Target: Y"
    || plotContract.points !== 3
    || JSON.stringify(plotContract.pointLabels) !== JSON.stringify(["M", "X", "Z"])
    || !plotContract.scope.includes("observed-range")
    || !plotContract.scope.includes("listwise-standardized composite scores")
    || !plotContract.scope.includes("No theoretical-range correction is applied")
    || !contract.noPlaceholderOrUnsupportedClaims) {
    throw new Error(`The completed IPMA result did not match its exact predecessor-only descriptive map contract: ${JSON.stringify(contract)}`);
  }
  await openResultTable("Construct importance and performance");
  return contract;
}

async function inspectNcaResultTree() {
  const group = page.getByRole("treeitem", { name: "Necessary conditions", exact: true });
  await group.waitFor({ state: "visible", timeout: 15_000 });
  if (await group.getAttribute("aria-expanded") === "false") await group.click();

  const initialSelectedTable = compactVisibleText(await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').textContent());
  const readTable = async (title) => {
    const rowCount = await openResultTable(title);
    const headers = (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText);
    const rows = await page.locator(".nd-result-table tbody tr").evaluateAll((elements) => elements.map((row) => (
      Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
    )));
    const warningLocator = page.locator(".nd-result-table-view > .nd-inline-warning");
    const warning = await warningLocator.count() === 1
      ? compactVisibleText(await warningLocator.textContent())
      : "";
    return { title, rowCount, headers, rows, warning };
  };

  const effects = await readTable("Ceiling effect sizes and permutation inference");
  const plot = page.locator(".nd-nca-plot");
  const plotSvg = plot.locator('svg[role="img"][aria-labelledby]');
  const directTitle = plotSvg.locator(":scope > title");
  const description = plotSvg.locator(":scope > desc");
  const plotContract = {
    figures: await plot.count(),
    accessibleSvgs: await plotSvg.count(),
    namedImages: await plot.getByRole("img", { name: /Necessary condition ceiling plot for x and y/i }).count(),
    labelledBy: await plotSvg.getAttribute("aria-labelledby"),
    directTitleCount: await directTitle.count(),
    title: compactVisibleText(await directTitle.textContent().catch(() => "")),
    descriptionCount: await description.count(),
    description: compactVisibleText(await description.textContent().catch(() => "")),
    captionTitle: compactVisibleText(await plot.locator("figcaption > strong").textContent().catch(() => "")),
    captionPair: compactVisibleText(await plot.locator("figcaption > span").textContent().catch(() => "")),
    ceFdhPaths: await plot.locator(".ceiling.ce-fdh").count(),
    crFdhLines: await plot.locator(".ceiling.cr-fdh").count(),
    ceFdhPeers: await plot.locator("circle.ce-peer").count(),
  };
  const peers = await readTable("CE-FDH frontier peers");
  const crLine = await readTable("CR-FDH ceiling coefficients");
  const bottlenecks = await readTable("Observed-range bottlenecks");
  const scope = await readTable("Analysis details");
  const scopeValues = Object.fromEntries(scope.rows.map((row) => [row[0], row[1]]));
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const expectedTreeItems = [
    "Necessary conditions",
    "Ceiling effect sizes and permutation inference",
    "CE-FDH frontier peers",
    "CR-FDH ceiling coefficients",
    "Observed-range bottlenecks",
    "Analysis details",
  ];
  const effectValues = Object.fromEntries(effects.rows.map((row) => [row[0], {
    effectSize: parseVisibleNumber(row[1]),
    permutationP: parseVisibleNumber(row[2]),
  }]));
  const pValueLattice = Object.values(effectValues).every((row) => row.permutationP !== null
    && row.permutationP >= 1 / (ncaPermutationSamples + 1) - ncaTolerance
    && row.permutationP <= 1 + ncaTolerance
    && Math.abs(row.permutationP * (ncaPermutationSamples + 1) - Math.round(row.permutationP * (ncaPermutationSamples + 1))) <= ncaTolerance);
  const displayedNumberClose = (actual, expected, tolerance) => Number.isFinite(actual)
    && Math.abs(actual - expected) <= tolerance;
  const expectedCeRequirements = [
    "33.3333% of observed X range", "33.3333% of observed X range", "33.3333% of observed X range",
    "33.3333% of observed X range", "33.3333% of observed X range", "33.3333% of observed X range",
    "100.0000% of observed X range", "100.0000% of observed X range", "100.0000% of observed X range",
  ];
  const expectedCrRequirements = [
    "Not necessary", "6.1538% of observed X range", "16.9231% of observed X range",
    "27.6923% of observed X range", "38.4615% of observed X range", "49.2308% of observed X range",
    "60.0000% of observed X range", "70.7692% of observed X range", "81.5385% of observed X range",
  ];
  const ceRows = bottlenecks.rows.filter((row) => row[0] === "CE-FDH");
  const crRows = bottlenecks.rows.filter((row) => row[0] === "CR-FDH");
  const bottlenecksMatch = ceRows.length === 9 && crRows.length === 9
    && ceRows.every((row, index) => row[1] === `${(index + 1) * 10}%` && row[2] === expectedCeRequirements[index])
    && crRows.every((row, index) => row[1] === `${(index + 1) * 10}%` && row[2] === expectedCrRequirements[index]);
  const renderedText = [effects, peers, crLine, bottlenecks, scope]
    .flatMap((table) => [table.headers, ...table.rows]).flat().join(" ")
    + ` ${Object.values(plotContract).join(" ")}`;
  const contract = {
    groupTitle: compactVisibleText(await group.textContent()),
    initialSelectedTable,
    treeItems,
    effects,
    peers,
    crLine,
    bottlenecks,
    scope,
    scopeValues,
    plot: plotContract,
    effectValues,
    pValueLattice,
    bottlenecksMatch,
    noModelOrQualityTree: !treeItems.some((label) => ["Graphical results", "Model estimates", "Quality criteria"].includes(label))
      && await page.locator(".nd-result-diagram-view").count() === 0,
    noPlaceholder: !/\bN\/A\b/i.test(renderedText),
    noBroaderNcaClaim: !/multiple conditions (?:are )?(?:included|supported)|latent-score NCA (?:is )?(?:included|supported)|cIPMA (?:is )?(?:included|supported)/i.test(renderedText),
  };

  if (initialSelectedTable !== "Ceiling effect sizes and permutation inference"
    || JSON.stringify(treeItems) !== JSON.stringify(expectedTreeItems)
    || effects.rowCount !== 2
    || JSON.stringify(effects.headers) !== JSON.stringify(["Ceiling line", "Effect size", "Permutation p"])
    || JSON.stringify(effects.rows.map((row) => row[0])) !== JSON.stringify(["CE-FDH", "CR-FDH"])
    || !displayedNumberClose(effectValues["CE-FDH"]?.effectSize, 5 / 9, 0.00005)
    || !displayedNumberClose(effectValues["CR-FDH"]?.effectSize, 36 / 91, 0.00005)
    || !pValueLattice
    || peers.rowCount !== 3
    || JSON.stringify(peers.headers) !== JSON.stringify(["Peer identity", "Condition variable (X)", "Condition value", "Outcome variable (Y)", "Outcome value"])
    || JSON.stringify(peers.rows) !== JSON.stringify([
      ["CE-FDH peer 1", "x", "0.0000", "y", "1.0000"],
      ["CE-FDH peer 2", "x", "1.0000", "y", "3.0000"],
      ["CE-FDH peer 3", "x", "3.0000", "y", "4.0000"],
    ])
    || !peers.warning.includes("does not retain original source-row identifiers")
    || crLine.rowCount !== 1
    || JSON.stringify(crLine.headers) !== JSON.stringify(["Ceiling line", "Slope", "Intercept"])
    || crLine.rows[0]?.[0] !== "CR-FDH"
    || !displayedNumberClose(parseVisibleNumber(crLine.rows[0]?.[1] ?? ""), 13 / 14, 0.00005)
    || !displayedNumberClose(parseVisibleNumber(crLine.rows[0]?.[2] ?? ""), 10 / 7, 0.00005)
    || bottlenecks.rowCount !== 18
    || JSON.stringify(bottlenecks.headers) !== JSON.stringify(["Ceiling line", "Outcome (% observed range)", "Condition requirement"])
    || !bottlenecksMatch
    || scope.rowCount !== 10 || JSON.stringify(scope.headers) !== JSON.stringify(["Field", "Value"])
    || scopeValues["Condition variable (X)"] !== "x" || scopeValues["Outcome variable (Y)"] !== "y"
    || scopeValues["Analyzed observations"] !== String(ncaObservations) || scopeValues["X observed range"] !== "0.000000 to 3.000000"
    || scopeValues["Y observed range"] !== "1.000000 to 4.000000" || scopeValues["Ceiling lines"] !== "CE-FDH and CR-FDH"
    || scopeValues["Requested permutations"] !== String(ncaPermutationSamples)
    || scopeValues["Usable permutations"] !== String(ncaPermutationSamples)
    || scopeValues["Missing data"] !== "Listwise deletion" || scopeValues["Method version"] !== ncaMethodVersion
    || plotContract.figures !== 1 || plotContract.accessibleSvgs !== 1 || plotContract.namedImages !== 1
    || plotContract.labelledBy !== "nd-nca-plot-title nd-nca-plot-description" || plotContract.directTitleCount !== 1
    || plotContract.title !== "Necessary condition ceiling plot for x and y" || plotContract.descriptionCount !== 1
    || plotContract.captionTitle !== "Necessary condition ceiling plot" || plotContract.captionPair !== "x -> y"
    || plotContract.ceFdhPaths !== 2 || plotContract.crFdhLines !== 2 || plotContract.ceFdhPeers !== 3
    || !plotContract.description.includes("CE-FDH peer 0, 1")
    || !plotContract.description.includes("CE-FDH peer 1, 3")
    || !plotContract.description.includes("CE-FDH peer 3, 4")
    || !plotContract.description.includes("CR-FDH slope 0.9286 and intercept 1.4286")
    || !contract.noModelOrQualityTree || !contract.noPlaceholder || !contract.noBroaderNcaClaim) {
    throw new Error(`The completed NCA result did not match its exact standalone nca_v2 tables and accessible ceiling plot: ${JSON.stringify(contract)}`);
  }
  await openResultTable("Ceiling effect sizes and permutation inference");
  return contract;
}

async function inspectCurrentRunDetails() {
  const command = page.locator(".nd-commandbar button").filter({ hasText: /^Run Details and Log/ });
  if (await command.count() !== 1 || !await command.isEnabled()) {
    throw new Error("The selected completed run did not expose exactly one enabled Run Details and Log command.");
  }
  await command.click();
  const dialog = page.getByRole("dialog", { name: "Run Details", exact: true });
  await dialog.waitFor({ state: "visible", timeout: 10_000 });
  const properties = await dialog.locator(".nd-property-list").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  const logEntries = await dialog.locator(".nd-run-details ol li").count();
  await dialog.getByRole("button", { name: "Close dialog", exact: true }).click();
  await dialog.waitFor({ state: "hidden", timeout: 10_000 });
  return { properties, logEntries };
}

async function inspectSavedPredictionArchive(projectPath, runId) {
  const { stdout } = await execFileAsync("tar", ["-xOf", projectPath, "project.json"], {
    cwd: root,
    windowsHide: true,
    maxBuffer: 64 * 1024 * 1024,
  });
  const project = JSON.parse(stdout);
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved prediction archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = project.layouts?.workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const usedObservations = estimation?.used_observations;
  const predict = estimation?.predict;
  const repeated = predict?.repeated_kfold;
  const indicatorRows = Array.isArray(repeated?.indicator_targets) ? repeated.indicator_targets : [];
  const constructRows = Array.isArray(repeated?.targets) ? repeated.targets : [];
  const cvpatRows = Array.isArray(repeated?.cvpat_benchmark_assessments) ? repeated.cvpat_benchmark_assessments : [];
  const endogenousConstructs = Array.isArray(recipe?.model?.constructs)
    ? recipe.model.constructs.filter((construct) => recipe.model.paths?.some((path) => path.target === construct.id))
    : [];
  const expectedIndicators = endogenousConstructs.flatMap((construct) => construct.indicators ?? []).sort();
  const exactVersions = result.provenance?.method_version === predictionProvenanceMethodVersion
    && estimation?.method_version === predictionMethodVersion
    && predict?.method_version === predictionMethodVersion
    && predict?.primary_analysis === predictionRepeatedMethodVersion
    && repeated?.method_version === predictionRepeatedMethodVersion
    && cvpatRows.every((row) => row.method_version === predictionCvpatMethodVersion);
  const exactRepeatedPlan = repeated?.folds === predictionFolds
    && repeated?.repeats === predictionRepeats
    && repeated?.seed === recipe?.settings?.seed
    && repeated?.assignment === predictionAssignment
    && /^sha256:[0-9a-f]{64}$/.test(repeated?.assignment_digest ?? "")
    && repeated?.total_test_observations === usedObservations * predictionRepeats;
  const exactIndicatorRows = indicatorRows.length === expectedIndicators.length
    && JSON.stringify(indicatorRows.map((row) => row.indicator).sort()) === JSON.stringify(expectedIndicators)
    && indicatorRows.every((row) => row.predictor_scope === "earliest_antecedent_indicators"
      && Number.isInteger(row.predictor_count) && row.predictor_count > 0
      && Number.isFinite(row.q_squared_predict)
      && row.pls?.observations === usedObservations * predictionRepeats
      && row.indicator_average?.observations === usedObservations * predictionRepeats
      && ["available", "unavailable"].includes(row.linear_model?.status));
  const exactCvpatRows = cvpatRows.length === 2
    && JSON.stringify(cvpatRows.map((row) => row.benchmark).sort()) === JSON.stringify(["indicator_average", "linear_model"])
    && cvpatRows.every((row) => row.comparison_kind === "benchmark_assessment"
      && row.target_scope === "all_endogenous_indicators"
      && row.loss === "mean_squared_error_across_indicators_per_observation"
      && row.alternative === "pls_loss_less_than_benchmark"
      && row.confidence_level === predictionConfidenceLevel
      && row.observations === usedObservations
      && row.indicator_count === expectedIndicators.length
      && ["available", "inferential_test_unavailable", "benchmark_unavailable"].includes(row.status));
  const contract = {
    resultStatus: result.status ?? null,
    resultMethod: result.provenance?.method ?? null,
    resultMethodVersion: result.provenance?.method_version ?? null,
    usedObservations: usedObservations ?? null,
    exactVersions,
    exactRepeatedPlan,
    exactIndicatorRows,
    exactCvpatRows,
    indicatorCount: indicatorRows.length,
    constructCount: constructRows.length,
    cvpatBenchmarks: cvpatRows.map((row) => row.benchmark),
    noLegacyRelabel: Array.isArray(repeated?.cvpat) && repeated.cvpat.length === 0
      && Array.isArray(repeated?.paired_loss_diagnostics) && repeated.paired_loss_diagnostics.length === 0,
    recipe: recipe ? {
      status: recipe.metadata?.status ?? null,
      method: recipe.settings?.method ?? null,
      confidenceLevel: recipe.settings?.confidence_level ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      seed: recipe.settings?.seed ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      paths: recipe.model?.paths?.length ?? null,
    } : null,
    run: run ? {
      method: run.method ?? null,
      status: run.status ?? null,
      modelId: run.modelId ?? null,
      snapshotNodes: run.modelSnapshot?.nodes?.length ?? null,
      logs: run.logs?.length ?? 0,
    } : null,
  };
  if (contract.resultStatus !== "completed" || contract.resultMethod !== "predict"
    || contract.usedObservations !== predictionObservations || !contract.exactVersions || !contract.exactRepeatedPlan
    || !contract.exactIndicatorRows || !contract.exactCvpatRows || contract.indicatorCount !== 2
    || contract.constructCount !== 1 || !contract.noLegacyRelabel
    || contract.recipe?.status !== "validated_plspredict_indicator_v2_and_cvpat_indicator_benchmarks_v2_bounded_scope"
    || contract.recipe?.method !== "predict" || contract.recipe?.confidenceLevel !== predictionConfidenceLevel
    || contract.recipe?.bootstrapSamples !== 0 || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null || !Number.isInteger(contract.recipe?.seed)
    || contract.recipe?.constructs !== 2 || contract.recipe?.paths !== 1
    || contract.run?.method !== "PLSpredict / CVPAT" || contract.run?.status !== "completed"
    || !contract.run?.modelId || contract.run?.snapshotNodes !== 2 || contract.run.logs < 1) {
    throw new Error(`The saved prediction archive did not retain the exact current indicator-level PLSpredict/CVPAT contract: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedMgaArchive(projectPath, runId) {
  const { stdout } = await execFileAsync("tar", ["-xOf", projectPath, "project.json"], {
    cwd: root,
    windowsHide: true,
    maxBuffer: 64 * 1024 * 1024,
  });
  const project = JSON.parse(stdout);
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved MICOM/MGA archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = project.layouts?.workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const mga = estimation?.mga;
  const permutation = estimation?.mga_permutation;
  const micom = estimation?.micom;
  const groups = Array.isArray(mga?.groups) ? mga.groups : [];
  const measurementComparisons = Array.isArray(mga?.measurement_comparisons) ? mga.measurement_comparisons : [];
  const permutationMeasurementComparisons = Array.isArray(permutation?.measurement_comparisons) ? permutation.measurement_comparisons : [];
  const micomConstructs = Array.isArray(micom?.constructs) ? micom.constructs : [];
  const permutationLedger = Array.isArray(permutation?.permutation_ledger) ? permutation.permutation_ledger : [];
  const micomLedger = Array.isArray(micom?.permutation_ledger) ? micom.permutation_ledger : [];
  const ledgerPlanDigest = (ledger) => {
    const digest = createHash("sha256");
    for (const entry of ledger) {
      const index = Buffer.alloc(8);
      index.writeBigUInt64LE(BigInt(entry.replicate));
      digest.update(index);
      digest.update(entry.partition_sha256, "utf8");
    }
    return `sha256:${digest.digest("hex")}`;
  };
  const exactLedger = (ledger) => ledger.length === mgaRuntimePermutationSamples
    && ledger.every((entry, replicate) => entry.replicate === replicate
      && /^[0-9a-f]{64}$/.test(entry.partition_sha256 ?? "")
      && entry.group_a_rows === 90 && entry.group_b_rows === 90
      && ["usable", "failed"].includes(entry.step2_status)
      && ["usable", "failed"].includes(entry.step3_status)
      && (entry.step2_status === "usable") === (entry.step2_failure_code == null)
      && (entry.step3_status === "usable") === (entry.step3_failure_code == null));
  const methodConfig = recipe?.method_config;
  const exactMethodConfigKeys = methodConfig && JSON.stringify(Object.keys(methodConfig).sort()) === JSON.stringify([
    "configural_invariance_confirmed",
    "group_a",
    "group_b",
    "group_column",
    "kind",
    "methods",
    "permutation_samples",
  ]);
  const exactGroupMethods = Array.isArray(methodConfig?.methods)
    && JSON.stringify(methodConfig.methods) === JSON.stringify(["micom", "mga_permutation"]);
  const provenanceTokens = String(result.provenance?.method_version ?? "").split("+");
  const exactCurrentVersions = [mgaMethodVersion, mgaPermutationMethodVersion, micomMethodVersion]
    .every((version) => provenanceTokens.filter((token) => token === version).length === 1)
    && !provenanceTokens.some((token) => /(?:pls_mga_two_group|pls_mga_permutation|micom)_v[1-3]$/.test(token));
  const exactGroupPayload = groups.length === 2
    && groups[0]?.group === "A" && groups[0]?.observations === 90
    && groups[1]?.group === "B" && groups[1]?.observations === 90
    && groups.every((group) => Array.isArray(group.paths) && group.paths.length === 2
      && Array.isArray(group.outer_estimates) && group.outer_estimates.length === 6
      && Array.isArray(group.transforms) && group.transforms.length === 6)
    && Array.isArray(mga?.comparisons) && mga.comparisons.length === 2
    && measurementComparisons.filter((row) => row.parameter === "outer_loading").length === 6
    && measurementComparisons.filter((row) => row.parameter === "outer_weight").length === 6;
  const exactPermutationPayload = permutation?.method_version === mgaPermutationMethodVersion
    && permutation?.permutation_samples === mgaRuntimePermutationSamples
    && permutation?.usable_permutations === mgaRuntimePermutationSamples
    && permutation?.attempted_permutations === mgaRuntimePermutationSamples
    && permutation?.failed_permutations === 0
    && permutation?.retry_policy === "none"
    && /^sha256:[0-9a-f]{64}$/.test(permutation?.permutation_plan_sha256 ?? "")
    && exactLedger(permutationLedger)
    && permutation.permutation_plan_sha256 === ledgerPlanDigest(permutationLedger)
    && Array.isArray(permutation?.comparisons) && permutation.comparisons.length === 2
    && permutationMeasurementComparisons.filter((row) => row.parameter === "outer_loading").length === 6
    && permutationMeasurementComparisons.filter((row) => row.parameter === "outer_weight").length === 6;
  const exactMicomPayload = micom?.method_version === micomMethodVersion
    && micom?.permutation_samples === mgaRuntimePermutationSamples
    && micom?.usable_permutations === mgaRuntimePermutationSamples
    && micom?.confidence_level === 0.95
    && micom?.attempted_permutations === mgaRuntimePermutationSamples
    && micom?.failed_permutations === 0
    && micom?.retry_policy === "none"
    && micom?.step1_status === "confirmed_by_researcher_review"
    && micom?.step1_computed === false
    && micom?.step2_usable_permutations === mgaRuntimePermutationSamples
    && micom?.step2_failed_permutations === 0
    && micom?.step3_usable_permutations === mgaRuntimePermutationSamples
    && micom?.step3_failed_permutations === 0
    && exactLedger(micomLedger)
    && JSON.stringify(micomLedger) === JSON.stringify(permutationLedger)
    && micom?.permutation_plan_sha256 === permutation?.permutation_plan_sha256
    && Array.isArray(micom?.groups) && micom.groups.length === 2
    && micom.groups[0]?.group === "A" && micom.groups[0]?.observations === 90
    && micom.groups[1]?.group === "B" && micom.groups[1]?.observations === 90
    && micomConstructs.length === 3
    && micomConstructs.every((row) => row.configural_invariance === true && [
      row.compositional_correlation,
      row.compositional_correlation_lower,
      row.compositional_p_value,
      row.mean_a,
      row.mean_b,
      row.mean_difference,
      row.mean_difference_lower,
      row.mean_difference_upper,
      row.mean_p_value,
      row.variance_a,
      row.variance_b,
      row.variance_difference,
      row.variance_difference_lower,
      row.variance_difference_upper,
      row.variance_p_value,
    ].every(Number.isFinite)
      && [row.partial_invariance, row.equal_means, row.equal_variances, row.full_invariance].every((value) => typeof value === "boolean"));
  const contract = {
    resultStatus: result.status ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    exactCurrentVersions,
    estimationMethodVersion: estimation?.method_version ?? null,
    mgaMethodVersion: mga?.method_version ?? null,
    permutationMethodVersion: permutation?.method_version ?? null,
    micomMethodVersion: micom?.method_version ?? null,
    exactGroupPayload,
    exactPermutationPayload,
    exactMicomPayload,
    recipe: recipe ? {
      schemaVersion: recipe.schema_version ?? null,
      status: recipe.metadata?.status ?? null,
      exactMethodConfigKeys,
      methodConfigKind: methodConfig?.kind ?? null,
      groupMethods: methodConfig?.methods ?? null,
      exactGroupMethods,
      groupPermutationSamples: methodConfig?.permutation_samples ?? null,
      configuralConfirmed: methodConfig?.configural_invariance_confirmed ?? null,
      groupColumn: methodConfig?.group_column ?? null,
      groupA: methodConfig?.group_a ?? null,
      groupB: methodConfig?.group_b ?? null,
      method: recipe.settings?.method ?? null,
      weighting: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      confidenceLevel: recipe.settings?.confidence_level ?? null,
    } : null,
    run: run ? { method: run.method, status: run.status, logs: run.logs?.length ?? 0 } : null,
  };
  if (contract.resultStatus !== "completed" || contract.provenanceMethod !== "mga"
    || !contract.exactCurrentVersions || contract.estimationMethodVersion !== mgaMethodVersion
    || contract.mgaMethodVersion !== mgaMethodVersion || !contract.exactGroupPayload
    || !contract.exactPermutationPayload || !contract.exactMicomPayload
    || contract.recipe?.schemaVersion !== 3 || !contract.recipe?.exactMethodConfigKeys
    || contract.recipe?.methodConfigKind !== "mga" || !contract.recipe?.exactGroupMethods
    || contract.recipe?.status !== "validated_micom_v4_and_permutation_mga_v4_fixed_plan_scope"
    || contract.recipe?.groupPermutationSamples !== mgaRuntimePermutationSamples
    || contract.recipe?.configuralConfirmed !== true
    || contract.recipe?.groupColumn !== "group" || contract.recipe?.groupA !== "A" || contract.recipe?.groupB !== "B"
    || contract.recipe?.method !== "mga" || contract.recipe?.weighting !== "path"
    || contract.recipe?.preprocessing !== "standardized" || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.bootstrapSamples !== 0 || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null || contract.recipe?.confidenceLevel !== 0.95
    || contract.run?.method !== "MICOM and Two-Group Permutation MGA" || contract.run?.status !== "completed"
    || !Number.isInteger(contract.run?.logs) || contract.run.logs < 1) {
    throw new Error(`The saved group-analysis archive did not retain the exact current MICOM v4 and permutation MGA v4 fixed-plan contract: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedStructuralPathRandomizationArchive(projectPath, runId) {
  const { project, manifest, projectText } = await readNcaArchive(projectPath);
  const projectChecksum = createHash("sha256").update(Buffer.from(projectText, "utf8")).digest("hex");
  const matchingResults = (project.results ?? []).filter((candidate) => candidate.id === runId);
  if (matchingResults.length !== 1) {
    throw new Error(`The saved Structural Path Randomization archive contained ${matchingResults.length} results for ${runId}.`);
  }
  const result = matchingResults[0];
  const matchingRecipes = (project.recipes ?? []).filter((candidate) => candidate.id === result.provenance?.recipe_id);
  if (matchingRecipes.length !== 1) {
    throw new Error(`The saved Structural Path Randomization archive did not bind exactly one schema-v3 recipe to ${runId}.`);
  }
  const recipe = matchingRecipes[0];
  const matchingRuns = (project.layouts?.workspace?.runs ?? []).filter((candidate) => candidate.id === runId);
  const run = matchingRuns[0];
  const estimation = result.payload?.estimation;
  const permutation = result.payload?.permutation;
  const constructs = Array.isArray(recipe.model?.constructs) ? recipe.model.constructs : [];
  const constructLabels = new Map(constructs.map((construct) => [construct.id, construct.name]));
  const canonicalRecipePaths = constructs.flatMap((construct) => (
    (recipe.model?.paths ?? []).filter((candidate) => candidate.target === construct.id)
  ));
  const resultPaths = Array.isArray(estimation?.paths) ? estimation.paths : [];
  const parameters = Array.isArray(permutation?.parameters) ? permutation.parameters : [];
  const labelPath = (candidate) => [constructLabels.get(candidate?.source) ?? null, constructLabels.get(candidate?.target) ?? null];
  const expectedLabelPairs = [["X", "Y"], ["Z", "Y"]];
  const pathOrderExact = JSON.stringify(canonicalRecipePaths.map(labelPath)) === JSON.stringify(expectedLabelPairs)
    && JSON.stringify(resultPaths.map(labelPath)) === JSON.stringify(expectedLabelPairs);
  const parameterContract = parameters.length === resultPaths.length && parameters.every((parameter, index) => {
    const resultPath = resultPaths[index];
    const expectedIdentity = JSON.stringify(["path", [resultPath?.source, resultPath?.target]]);
    const expectedP = (parameter?.exceedances + 1) / (structuralPathRandomizationPermutations + 1);
    return parameter && JSON.stringify(Object.keys(parameter).sort()) === JSON.stringify([
      "exceedances", "original", "p_value_two_sided", "parameter", "permutations",
    ])
      && parameter.parameter === expectedIdentity
      && Number.isFinite(parameter.original) && Object.is(parameter.original, resultPath?.coefficient)
      && Number.isInteger(parameter.exceedances) && parameter.exceedances >= 0
      && parameter.exceedances <= structuralPathRandomizationPermutations
      && parameter.permutations === structuralPathRandomizationPermutations
      && Number.isFinite(parameter.p_value_two_sided)
      && Object.is(parameter.p_value_two_sided, expectedP);
  });
  const settings = recipe.settings ?? {};
  const provenanceSettings = result.provenance?.settings ?? {};
  const sameSettings = [
    "method", "weighting_scheme", "tolerance", "max_iterations", "bootstrap_samples",
    "studentized_inner_samples", "permutation_samples", "seed", "workers", "confidence_level",
    "preprocessing", "missing_data", "case_weight_column",
  ].every((key) => Object.is(settings[key], provenanceSettings[key]));
  const contract = {
    manifest: {
      schemaVersion: manifest.schema_version ?? null,
      engineVersion: manifest.engine_version ?? null,
      checksumAlgorithm: manifest.checksum_algorithm ?? null,
      declaredProjectChecksum: manifest.checksums?.["project.json"] ?? null,
      projectChecksum,
      projectChecksumMatches: manifest.checksums?.["project.json"] === projectChecksum,
    },
    project: {
      modelCount: project.models?.length ?? null,
      recipeCount: project.recipes?.length ?? null,
      resultCount: project.results?.length ?? null,
      matchingRunCount: matchingRuns.length,
    },
    resultId: result.id ?? null,
    resultSchemaVersion: result.schema_version ?? null,
    resultStatus: result.status ?? null,
    payloadKind: result.payload?.kind ?? null,
    estimationMethodVersion: estimation?.method_version ?? null,
    bootstrapAbsent: result.payload?.bootstrap == null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    recipeId: recipe.id ?? null,
    recipeSchemaVersion: recipe.schema_version ?? null,
    recipeMethodConfig: recipe.method_config ?? null,
    recipeMethodConfigExact: recipe.method_config?.kind === "pls_permutation"
      && Object.keys(recipe.method_config).length === 1,
    recipeStatus: recipe.metadata?.status ?? null,
    settings: {
      method: settings.method ?? null,
      weightingScheme: settings.weighting_scheme ?? null,
      preprocessing: settings.preprocessing ?? null,
      missingData: settings.missing_data ?? null,
      bootstrapSamples: settings.bootstrap_samples ?? null,
      studentizedInnerSamples: settings.studentized_inner_samples ?? null,
      permutationSamples: settings.permutation_samples ?? null,
      seed: settings.seed ?? null,
      workers: settings.workers ?? null,
      confidenceLevel: settings.confidence_level ?? null,
      caseWeightColumn: settings.case_weight_column ?? null,
      exactProvenanceMatch: sameSettings,
    },
    constructs: constructs.map((construct) => ({ id: construct.id, name: construct.name, indicators: construct.indicators })),
    recipePathIds: canonicalRecipePaths.map((candidate) => [candidate.source, candidate.target]),
    resultPathIds: resultPaths.map((candidate) => [candidate.source, candidate.target]),
    pathLabels: resultPaths.map((candidate) => `${constructLabels.get(candidate.source)} -> ${constructLabels.get(candidate.target)}`),
    pathOrderExact,
    permutation: permutation ? {
      exactKeys: JSON.stringify(Object.keys(permutation).sort()) === JSON.stringify(["method_version", "parameters", "plan"]),
      methodVersion: permutation.method_version ?? null,
      planExactKeys: JSON.stringify(Object.keys(permutation.plan ?? {}).sort()) === JSON.stringify(["master_seed", "operation", "permutations"]),
      permutations: permutation.plan?.permutations ?? null,
      masterSeed: permutation.plan?.master_seed ?? null,
      operation: permutation.plan?.operation ?? null,
      parameterCount: parameters.length,
      parameterIds: parameters.map((parameter) => parameter.parameter),
      exceedances: parameters.map((parameter) => parameter.exceedances),
      pValues: parameters.map((parameter) => parameter.p_value_two_sided),
      parameterContract,
    } : null,
    run: run ? {
      id: run.id ?? null,
      method: run.method ?? null,
      status: run.status ?? null,
      modelId: run.modelId ?? null,
      snapshotNodes: run.modelSnapshot?.nodes?.length ?? null,
      snapshotEdges: run.modelSnapshot?.edges?.length ?? null,
      logs: run.logs?.length ?? 0,
    } : null,
  };
  const expectedProvenanceVersion = `pls_pm_v1+pls_mediation_v1+pls_assessment_v8+${structuralPathRandomizationMethodVersion}`;
  const valid = contract.manifest.schemaVersion === 5
    && contract.manifest.engineVersion === packageVersion
    && contract.manifest.checksumAlgorithm === "sha256"
    && contract.manifest.projectChecksumMatches
    && contract.project.modelCount === 1 && contract.project.recipeCount === 1
    && contract.project.resultCount === 1 && contract.project.matchingRunCount === 1
    && contract.resultSchemaVersion === 1 && contract.resultStatus === "completed"
    && contract.payloadKind === "pls_pm_v3" && contract.estimationMethodVersion === "pls_pm_v1"
    && contract.bootstrapAbsent && contract.provenanceMethod === "pls_pm"
    && contract.provenanceMethodVersion === expectedProvenanceVersion
    && contract.recipeSchemaVersion === 3 && contract.recipeMethodConfigExact
    && contract.recipeStatus === "candidate_freedman_lane_path_randomization_scope"
    && contract.settings.method === "pls_pm" && contract.settings.weightingScheme === "path"
    && contract.settings.preprocessing === "standardized" && contract.settings.missingData === "listwise_deletion"
    && contract.settings.bootstrapSamples === 0 && contract.settings.studentizedInnerSamples === 0
    && contract.settings.permutationSamples === structuralPathRandomizationPermutations
    && contract.settings.seed === structuralPathRandomizationSeed
    && contract.settings.workers === structuralPathRandomizationWorkers
    && contract.settings.confidenceLevel === 0.95 && contract.settings.caseWeightColumn === null
    && contract.settings.exactProvenanceMatch
    && JSON.stringify(contract.constructs.map((construct) => construct.name)) === JSON.stringify(["X", "Z", "Y"])
    && contract.pathOrderExact
    && JSON.stringify(contract.pathLabels) === JSON.stringify(structuralPathRandomizationExpectedPathLabels)
    && contract.permutation?.exactKeys && contract.permutation?.planExactKeys
    && contract.permutation?.methodVersion === structuralPathRandomizationMethodVersion
    && contract.permutation?.permutations === structuralPathRandomizationPermutations
    && contract.permutation?.masterSeed === structuralPathRandomizationSeed
    && contract.permutation?.operation === structuralPathRandomizationOperation
    && contract.permutation?.parameterCount === 2 && contract.permutation?.parameterContract
    && contract.run?.id === runId && contract.run?.method === "Structural Path Randomization"
    && contract.run?.status === "completed" && Boolean(contract.run?.modelId)
    && contract.run?.snapshotNodes === 3 && contract.run?.snapshotEdges === 2
    && Number.isInteger(contract.run?.logs) && contract.run.logs >= 1;
  contract.passed = valid;
  if (!valid) {
    throw new Error(`The saved Structural Path Randomization archive did not retain the exact schema-v3 recipe, schema-v4 payload, canonical path order, plus-one arithmetic, and no-bootstrap contract: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedCcaArchive(projectPath, runId) {
  const { stdout } = await execFileAsync("tar", ["-xOf", projectPath, "project.json"], {
    cwd: root,
    windowsHide: true,
    maxBuffer: 32 * 1024 * 1024,
  });
  const project = JSON.parse(stdout);
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved CCA archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const cca = result.payload?.estimation?.cca;
  const correlations = Array.isArray(cca?.correlations) ? cca.correlations : [];
  const pairKeys = correlations.map((row) => [row.left, row.right].sort().join("::"));
  const finiteRows = correlations.every((row) => [row.observed, row.reproduced, row.residual, row.absolute_residual].every(Number.isFinite));
  const residualIdentities = correlations.every((row) => Math.abs(row.residual - (row.observed - row.reproduced)) <= 1e-10
    && Math.abs(row.absolute_residual - Math.abs(row.residual)) <= 1e-10);
  const calculatedMaximum = Math.max(...correlations.map((row) => row.absolute_residual));
  const contract = {
    resultId: result.id,
    resultStatus: result.status,
    method: result.provenance?.method ?? null,
    methodVersion: result.provenance?.method_version ?? null,
    nestedModelVersion: cca?.model ?? null,
    payloadMethodVersion: cca?.method_version ?? null,
    correlationPairs: correlations.length,
    uniqueCorrelationPairs: new Set(pairKeys).size,
    finiteRows,
    residualIdentities,
    maximumAbsoluteResidual: cca?.max_absolute_residual ?? null,
    maximumMatchesRows: Number.isFinite(cca?.max_absolute_residual)
      && Number.isFinite(calculatedMaximum)
      && Math.abs(cca.max_absolute_residual - calculatedMaximum) <= 1e-10,
    recipe: recipe ? {
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      paths: recipe.model?.paths?.length ?? null,
      controls: recipe.model?.controls?.length ?? null,
      interactions: recipe.model?.interactions?.length ?? null,
      higherOrderConstructs: recipe.model?.higher_order_constructs?.length ?? null,
    } : null,
  };
  if (contract.resultStatus !== "completed"
    || contract.method !== "cca"
    || contract.methodVersion !== ccaProvenanceMethodVersion
    || contract.payloadMethodVersion !== ccaMethodVersion
    || contract.nestedModelVersion !== ccaNestedModelVersion
    || contract.correlationPairs !== 3
    || contract.uniqueCorrelationPairs !== 3
    || !contract.finiteRows
    || !contract.residualIdentities
    || !contract.maximumMatchesRows
    || contract.recipe?.method !== "cca"
    || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "standardized"
    || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.bootstrapSamples !== 0
    || contract.recipe?.studentizedInnerSamples !== 0
    || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null
    || contract.recipe?.constructs !== 3
    || contract.recipe?.paths !== 2
    || contract.recipe?.controls !== 0
    || contract.recipe?.interactions !== 0
    || contract.recipe?.higherOrderConstructs !== 0) {
    throw new Error(`The saved CCA archive did not retain the exact bounded recipe and residual payload: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedIpmaArchive(projectPath, runId, constructIds) {
  const { stdout } = await execFileAsync("tar", ["-xOf", projectPath, "project.json"], {
    cwd: root,
    windowsHide: true,
    maxBuffer: 32 * 1024 * 1024,
  });
  const project = JSON.parse(stdout);
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved IPMA archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const methodConfig = recipe?.method_config;
  const exactMethodConfigKeys = methodConfig && JSON.stringify(Object.keys(methodConfig).sort()) === JSON.stringify(["kind", "targets"]);
  const estimation = result.payload?.estimation;
  const ipma = estimation?.ipma;
  const constructRows = Array.isArray(ipma?.constructs) ? ipma.constructs : [];
  const indicatorRows = Array.isArray(ipma?.indicators) ? ipma.indicators : [];
  const expectedPredecessors = [constructIds.x, constructIds.z, constructIds.m].sort();
  const actualPredecessors = constructRows.map((row) => row.construct).sort();
  const actualIndicatorConstructs = indicatorRows.map((row) => row.construct).sort();
  const excludedConstructIds = [constructIds.y, constructIds.u, constructIds.v];
  const excludedIndicators = ["y1", "u1", "v1"];
  const finiteConstructRows = constructRows.every((row) => [row.importance, row.performance, row.score_mean].every(Number.isFinite)
    && row.performance >= 0 && row.performance <= 100 && row.target === constructIds.y);
  const finiteIndicatorRows = indicatorRows.every((row) => [row.construct_importance, row.loading, row.performance, row.score_mean].every(Number.isFinite)
    && row.performance >= 0 && row.performance <= 100 && row.target === constructIds.y);
  const forbiddenNestedPayloads = ["plsc", "wpls", "cca", "predict", "mga", "micom", "mga_permutation", "cbsem", "regression", "nca", "gsca"]
    .filter((key) => estimation?.[key] != null);
  const contract = {
    resultId: result.id,
    resultStatus: result.status,
    method: result.provenance?.method ?? null,
    methodVersion: result.provenance?.method_version ?? null,
    payloadMethodVersion: estimation?.method_version ?? null,
    ipmaMethodVersion: ipma?.method_version ?? null,
    performanceScale: ipma?.performance_scale ?? null,
    targets: ipma?.targets ?? null,
    constructRows: constructRows.length,
    indicatorRows: indicatorRows.length,
    expectedPredecessors,
    actualPredecessors,
    actualIndicatorConstructs,
    indicators: indicatorRows.map((row) => row.indicator).sort(),
    finiteConstructRows,
    finiteIndicatorRows,
    excludedConstructIds,
    excludedIndicators,
    excludesTargetAndUnrelatedRows: excludedConstructIds.every((id) => !actualPredecessors.includes(id)
      && !actualIndicatorConstructs.includes(id))
      && excludedIndicators.every((indicator) => !indicatorRows.some((row) => row.indicator === indicator)),
    forbiddenNestedPayloads,
    recipe: recipe ? {
      schemaVersion: recipe.schema_version ?? null,
      exactMethodConfigKeys,
      methodConfigKind: methodConfig?.kind ?? null,
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      ipmaTargets: methodConfig?.targets ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      paths: recipe.model?.paths?.length ?? null,
      controls: recipe.model?.controls?.length ?? null,
      interactions: recipe.model?.interactions?.length ?? null,
      higherOrderConstructs: recipe.model?.higher_order_constructs?.length ?? null,
    } : null,
  };
  if (contract.resultStatus !== "completed"
    || contract.method !== "ipma"
    || contract.methodVersion !== ipmaProvenanceMethodVersion
    || contract.payloadMethodVersion !== ipmaMethodVersion
    || contract.ipmaMethodVersion !== ipmaMethodVersion
    || contract.performanceScale !== ipmaPerformanceScale
    || JSON.stringify(contract.targets) !== JSON.stringify([constructIds.y])
    || contract.constructRows !== 3
    || contract.indicatorRows !== 3
    || JSON.stringify(actualPredecessors) !== JSON.stringify(expectedPredecessors)
    || JSON.stringify(actualIndicatorConstructs) !== JSON.stringify(expectedPredecessors)
    || JSON.stringify(contract.indicators) !== JSON.stringify(["m1", "x1", "z1"])
    || !finiteConstructRows || !finiteIndicatorRows || !contract.excludesTargetAndUnrelatedRows
    || forbiddenNestedPayloads.length !== 0
    || contract.recipe?.schemaVersion !== 3 || !contract.recipe?.exactMethodConfigKeys
    || contract.recipe?.methodConfigKind !== "ipma"
    || contract.recipe?.method !== "ipma"
    || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "standardized"
    || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.bootstrapSamples !== 0
    || contract.recipe?.studentizedInnerSamples !== 0
    || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null
    || JSON.stringify(contract.recipe?.ipmaTargets) !== JSON.stringify([constructIds.y])
    || contract.recipe?.constructs !== 6
    || contract.recipe?.paths !== 6
    || contract.recipe?.controls !== 0
    || contract.recipe?.interactions !== 0
    || contract.recipe?.higherOrderConstructs !== 0) {
    throw new Error(`The saved IPMA archive did not retain the exact bounded recipe and predecessor-only payload: ${JSON.stringify(contract)}`);
  }
  return contract;
}

function ncaNumberClose(actual, expected) {
  return Number.isFinite(actual) && Math.abs(actual - expected) <= ncaTolerance;
}

function ncaOptionalNumberClose(actual, expected) {
  return expected === null ? actual === null : ncaNumberClose(actual, expected);
}

async function readNcaArchive(projectPath) {
  const [{ stdout: projectText }, { stdout: manifestText }] = await Promise.all([
    execFileAsync("tar", ["-xOf", projectPath, "project.json"], {
      cwd: root,
      windowsHide: true,
      maxBuffer: 32 * 1024 * 1024,
    }),
    execFileAsync("tar", ["-xOf", projectPath, "manifest.json"], {
      cwd: root,
      windowsHide: true,
      maxBuffer: 4 * 1024 * 1024,
    }),
  ]);
  return { project: JSON.parse(projectText), manifest: JSON.parse(manifestText), projectText };
}

async function inspectInitialNcaArchive(projectPath) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    models: project.models?.length ?? null,
    recipes: project.recipes?.length ?? null,
    results: project.results?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? 0,
    edges: workspace?.edges?.length ?? 0,
  };
  if (contract.manifestEngineVersion !== packageVersion
    || contract.models !== 0 || contract.recipes !== 0 || contract.results !== 0
    || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0) {
    throw new Error(`The imported NCA fixture was not a canonical data-only project: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectStructuralPathRandomizationCancellationArchive(projectPath) {
  const [{ project }, artifact] = await Promise.all([
    readNcaArchive(projectPath),
    artifactDigest(projectPath),
  ]);
  const workspaceRuns = Array.isArray(project.layouts?.workspace?.runs)
    ? project.layouts.workspace.runs
    : [];
  const recipes = Array.isArray(project.recipes) ? project.recipes : [];
  const results = Array.isArray(project.results) ? project.results : [];
  const datasets = Array.isArray(project.datasets) ? project.datasets : [];
  const models = Array.isArray(project.models) ? project.models : [];
  const model = models[0] ?? null;
  const constructs = Array.isArray(model?.constructs) ? model.constructs : [];
  const constructLabels = new Map(constructs.map((construct) => [construct?.id, construct?.name]));
  const paths = Array.isArray(model?.paths) ? model.paths : [];
  return {
    artifact,
    datasetCount: datasets.length,
    modelCount: models.length,
    modelName: model?.name ?? null,
    constructLabels: constructs.map((construct) => construct?.name ?? null),
    pathLabels: paths.map((candidate) => (
      `${constructLabels.get(candidate?.source) ?? ""} -> ${constructLabels.get(candidate?.target) ?? ""}`
    )),
    recipeCount: recipes.length,
    resultCount: results.length,
    runCount: workspaceRuns.length,
    recipeIds: recipes.map((row) => row?.id ?? null),
    resultIds: results.map((row) => row?.id ?? null),
    runIds: workspaceRuns.map((row) => row?.id ?? null),
  };
}

async function snapshotStructuralPathRandomizationCancellationArchive(phase, sourcePath) {
  if (!new Set(["before", "after"]).has(phase)) {
    throw new Error(`Unsafe Structural Path Randomization cancellation snapshot phase: ${phase}`);
  }
  const snapshotPath = `${structuralPathRandomizationCancellationSnapshotPrefix}-${phase}.qpls`;
  const temporaryPath = `${snapshotPath}.copying`;
  const [snapshotExists, temporaryExists] = await Promise.all([
    fs.stat(snapshotPath).then(() => true).catch(() => false),
    fs.stat(temporaryPath).then(() => true).catch(() => false),
  ]);
  if (snapshotExists || temporaryExists) {
    throw new Error(`Structural Path Randomization cancellation snapshot target was not exclusive: ${snapshotPath}`);
  }
  const sourceStatBefore = await fs.stat(sourcePath, { bigint: true });
  const sourceDigestBefore = await artifactDigest(sourcePath);
  if (!sourceStatBefore.isFile() || !sourceDigestBefore) {
    throw new Error(`Structural Path Randomization cancellation snapshot source was not a non-empty file: ${sourcePath}`);
  }
  try {
    await fs.copyFile(sourcePath, temporaryPath, fsConstants.COPYFILE_EXCL);
    const sourceStatAfter = await fs.stat(sourcePath, { bigint: true });
    const sourceDigestAfter = await artifactDigest(sourcePath);
    const temporaryDigest = await artifactDigest(temporaryPath);
    const sourceStableDuringCopy = sourceStatAfter.isFile()
      && sourceStatBefore.size === sourceStatAfter.size
      && sourceStatBefore.mtimeNs === sourceStatAfter.mtimeNs
      && sourceDigestAfter !== null
      && sourceDigestBefore.size === sourceDigestAfter.size
      && sourceDigestBefore.sha256 === sourceDigestAfter.sha256;
    const snapshotMatchesSource = temporaryDigest !== null
      && temporaryDigest.size === sourceDigestBefore.size
      && temporaryDigest.sha256 === sourceDigestBefore.sha256;
    if (!sourceStableDuringCopy || !snapshotMatchesSource) {
      throw new Error(`Structural Path Randomization cancellation snapshot source changed during copy: ${JSON.stringify({ sourceDigestBefore, sourceDigestAfter, temporaryDigest })}`);
    }
    await fs.link(temporaryPath, snapshotPath);
    await fs.rm(temporaryPath, { force: true });
    const snapshotDigest = await artifactDigest(snapshotPath);
    if (!snapshotDigest || snapshotDigest.size !== sourceDigestBefore.size
      || snapshotDigest.sha256 !== sourceDigestBefore.sha256) {
      throw new Error(`Structural Path Randomization cancellation snapshot identity changed after publication: ${snapshotPath}`);
    }
    const logicalState = await inspectStructuralPathRandomizationCancellationArchive(snapshotPath);
    return {
      phase,
      ...logicalState,
      sourcePath: sourceDigestBefore.path,
      sourceSize: sourceDigestBefore.size,
      sourceSha256: sourceDigestBefore.sha256,
      sourceMtimeNsBefore: sourceStatBefore.mtimeNs.toString(),
      sourceMtimeNsAfter: sourceStatAfter.mtimeNs.toString(),
      sourceStableDuringCopy,
      snapshotMatchesSource,
    };
  } finally {
    await fs.rm(temporaryPath, { force: true }).catch(() => {});
  }
}

async function inspectSavedNcaArchive(projectPath, runId) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved NCA archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const methodConfig = recipe?.method_config;
  const exactMethodConfigKeys = methodConfig && JSON.stringify(Object.keys(methodConfig).sort()) === JSON.stringify([
    "ceiling", "condition", "kind", "outcome", "permutation_samples",
  ]);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const nca = estimation?.nca;
  const assessment = result.payload?.assessment;
  const ceilings = Array.isArray(nca?.ceilings) ? nca.ceilings : [];
  const bottlenecks = Array.isArray(nca?.bottlenecks) ? nca.bottlenecks : [];
  const peers = Array.isArray(nca?.ce_fdh_peers) ? nca.ce_fdh_peers : [];
  const scope = nca?.scope;
  const ce = ceilings.find((row) => row.ceiling === "ce_fdh");
  const cr = ceilings.find((row) => row.ceiling === "cr_fdh");
  const expectedPeers = [{ x: 0, y: 1 }, { x: 1, y: 3 }, { x: 3, y: 4 }];
  const expectedCeBottlenecks = [
    100 / 3, 100 / 3, 100 / 3, 100 / 3, 100 / 3, 100 / 3, 100, 100, 100,
  ];
  const expectedCrBottlenecks = [
    null, 6.153846153846154, 16.923076923076923, 27.692307692307693,
    38.46153846153846, 49.23076923076923, 60, 70.76923076923077, 81.53846153846153,
  ];
  const ceBottlenecks = bottlenecks.filter((row) => row.ceiling === "ce_fdh");
  const crBottlenecks = bottlenecks.filter((row) => row.ceiling === "cr_fdh");
  const pValues = ceilings.map((row) => row.permutation_p_value);
  const pValueLattice = pValues.every((value) => Number.isFinite(value)
    && value >= 1 / (ncaPermutationSamples + 1) - ncaTolerance
    && value <= 1 + ncaTolerance
    && Math.abs(value * (ncaPermutationSamples + 1) - Math.round(value * (ncaPermutationSamples + 1))) <= ncaTolerance);
  const scopeMatches = scope
    && ncaNumberClose(scope.minimum_x, 0) && ncaNumberClose(scope.maximum_x, 3)
    && ncaNumberClose(scope.minimum_y, 1) && ncaNumberClose(scope.maximum_y, 4);
  const peersMatch = peers.length === expectedPeers.length && peers.every((peer, index) => (
    ncaNumberClose(peer.x, expectedPeers[index].x) && ncaNumberClose(peer.y, expectedPeers[index].y)
  ));
  const ceilingGeometryMatches = ceilings.length === 2
    && ce && cr
    && ncaNumberClose(ce.effect_size, 5 / 9) && ce.slope === null && ce.intercept === null
    && ncaNumberClose(cr.effect_size, 36 / 91)
    && ncaNumberClose(cr.slope, 13 / 14) && ncaNumberClose(cr.intercept, 10 / 7)
    && pValueLattice;
  const bottlenecksMatch = bottlenecks.length === 18
    && ceBottlenecks.length === 9 && crBottlenecks.length === 9
    && ceBottlenecks.every((row, index) => row.outcome_percent === (index + 1) * 10
      && row.status === "required"
      && ncaOptionalNumberClose(row.required_x_percent, expectedCeBottlenecks[index]))
    && crBottlenecks.every((row, index) => row.outcome_percent === (index + 1) * 10
      && row.status === (index === 0 ? "not_necessary" : "required")
      && ncaOptionalNumberClose(row.required_x_percent, expectedCrBottlenecks[index]));
  const exactNcaKeys = Object.keys(nca ?? {}).sort();
  const expectedNcaKeys = [
    "bottlenecks", "ce_fdh_peers", "ceiling", "ceilings", "method_version", "observations",
    "permutation_samples", "scope", "usable_permutations", "warnings", "x", "y",
  ].sort();
  const expectedCeilingKeys = ["ceiling", "effect_size", "permutation_p_value", "slope", "intercept"].sort();
  const expectedBottleneckKeys = ["ceiling", "outcome_percent", "required_x_percent", "status"].sort();
  const expectedPeerKeys = ["x", "y"].sort();
  const exactCeilingRows = ceilings.every((row) => (
    JSON.stringify(Object.keys(row).sort()) === JSON.stringify(expectedCeilingKeys)
  ));
  const exactBottleneckRows = bottlenecks.every((row) => (
    JSON.stringify(Object.keys(row).sort()) === JSON.stringify(expectedBottleneckKeys)
  ));
  const exactPeerRows = peers.every((row) => (
    JSON.stringify(Object.keys(row).sort()) === JSON.stringify(expectedPeerKeys)
  ));
  const forbiddenNestedPayloads = [
    "cbsem", "cca", "cta_pls", "endogeneity", "fimix", "gsca", "ipma", "mga", "mga_permutation",
    "micom", "moderated_mediation", "nonlinear_effects", "pca", "plsc", "predict", "regression",
    "segmentation", "wpls",
  ].filter((key) => estimation?.[key] != null);
  const contract = {
    resultId: result.id,
    resultStatus: result.status,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    provenanceEngineVersion: result.provenance?.engine_version ?? null,
    provenanceSeed: result.provenance?.seed ?? null,
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    payloadKind: result.payload?.kind ?? null,
    estimationMethodVersion: estimation?.method_version ?? null,
    usedObservations: estimation?.used_observations ?? null,
    omittedObservations: estimation?.omitted_observations ?? null,
    assessment: assessment ? {
      methodVersion: assessment.method_version ?? null,
      warnings: assessment.warnings ?? null,
    } : null,
    ncaKeys: exactNcaKeys,
    exactCeilingRows,
    exactBottleneckRows,
    exactPeerRows,
    ncaMethodVersion: nca?.method_version ?? null,
    ceiling: nca?.ceiling ?? null,
    permutationSamples: nca?.permutation_samples ?? null,
    usablePermutations: nca?.usable_permutations ?? null,
    x: nca?.x ?? null,
    y: nca?.y ?? null,
    observations: nca?.observations ?? null,
    scope,
    peers,
    ceilings,
    bottlenecks,
    scopeMatches,
    peersMatch,
    ceilingGeometryMatches,
    bottlenecksMatch,
    pValueLattice,
    warnings: nca?.warnings ?? null,
    forbiddenNestedPayloads,
    recipe: recipe ? {
      schemaVersion: recipe.schema_version ?? null,
      exactMethodConfigKeys,
      methodConfigKind: methodConfig?.kind ?? null,
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      seed: recipe.settings?.seed ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      ncaX: methodConfig?.condition ?? null,
      ncaY: methodConfig?.outcome ?? null,
      ncaCeiling: methodConfig?.ceiling ?? null,
      ncaPermutationSamples: methodConfig?.permutation_samples ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      paths: recipe.model?.paths?.length ?? null,
      controls: recipe.model?.controls?.length ?? null,
      interactions: recipe.model?.interactions?.length ?? null,
      higherOrderConstructs: recipe.model?.higher_order_constructs?.length ?? null,
    } : null,
    models: project.models?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? null,
    edges: workspace?.edges?.length ?? null,
    runModelId: run?.modelId ?? null,
    runModelSnapshot: run?.modelSnapshot ?? null,
    runSeed: run?.seed ?? null,
  };
  if (contract.resultStatus !== "completed"
    || contract.provenanceMethod !== "nca"
    || contract.provenanceMethodVersion !== ncaMethodVersion
    || contract.provenanceEngineVersion !== packageVersion
    || contract.provenanceSeed !== ncaSeed
    || contract.manifestEngineVersion !== packageVersion
    || contract.payloadKind !== "pls_pm_v1"
    || contract.estimationMethodVersion !== ncaMethodVersion
    || contract.usedObservations !== ncaObservations || contract.omittedObservations !== 0
    || contract.assessment?.methodVersion !== "assessment_not_applicable_v1"
    || JSON.stringify(contract.assessment?.warnings) !== JSON.stringify(["PLS assessment is not applicable to standalone raw-data analyses."])
    || JSON.stringify(exactNcaKeys) !== JSON.stringify(expectedNcaKeys)
    || !exactCeilingRows || !exactBottleneckRows || !exactPeerRows
    || contract.ncaMethodVersion !== ncaMethodVersion || contract.ceiling !== "both"
    || contract.permutationSamples !== ncaPermutationSamples || contract.usablePermutations !== ncaPermutationSamples
    || contract.x !== "x" || contract.y !== "y" || contract.observations !== ncaObservations
    || !scopeMatches || !peersMatch || !ceilingGeometryMatches || !bottlenecksMatch
    || !Array.isArray(contract.warnings) || contract.warnings.length !== 1
    || contract.warnings[0] !== ncaScopeWarning
    || forbiddenNestedPayloads.length !== 0
    || contract.recipe?.schemaVersion !== 3 || !contract.recipe?.exactMethodConfigKeys
    || contract.recipe?.methodConfigKind !== "nca"
    || contract.recipe?.method !== "nca" || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "unstandardized" || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.seed !== ncaSeed || contract.recipe?.bootstrapSamples !== 0
    || contract.recipe?.studentizedInnerSamples !== 0 || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null || contract.recipe?.ncaX !== "x" || contract.recipe?.ncaY !== "y"
    || contract.recipe?.ncaCeiling !== "both" || contract.recipe?.ncaPermutationSamples !== ncaPermutationSamples
    || contract.recipe?.constructs !== 0 || contract.recipe?.paths !== 0 || contract.recipe?.controls !== 0
    || contract.recipe?.interactions !== 0 || contract.recipe?.higherOrderConstructs !== 0
    || contract.models !== 0 || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0
    || contract.runModelId !== null || contract.runModelSnapshot !== null
    || contract.runSeed !== ncaSeed) {
    throw new Error(`The saved NCA archive did not retain the exact standalone nca_v2 recipe, geometry, and model-free snapshot: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectInitialPcaArchive(projectPath) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    models: project.models?.length ?? null,
    recipes: project.recipes?.length ?? null,
    results: project.results?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? 0,
    edges: workspace?.edges?.length ?? 0,
  };
  if (contract.manifestEngineVersion !== packageVersion
    || contract.models !== 0 || contract.recipes !== 0 || contract.results !== 0
    || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0) {
    throw new Error(`The imported PCA fixture was not a canonical data-only project: ${JSON.stringify(contract)}`);
  }
  return contract;
}

function archiveProjectChecksum(projectText, manifest) {
  const expected = manifest?.checksums?.["project.json"] ?? null;
  const actual = createHash("sha256").update(projectText, "utf8").digest("hex");
  return { expected, actual, matches: expected === actual };
}

async function inspectCtaPlsArchiveCounts(projectPath) {
  const { project, manifest, projectText } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    projectChecksum: archiveProjectChecksum(projectText, manifest),
    datasetCount: project.datasets?.length ?? null,
    modelCount: project.models?.length ?? null,
    recipeCount: project.recipes?.length ?? null,
    resultCount: project.results?.length ?? null,
    runCount: workspace?.runs?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
  };
  if (contract.manifestEngineVersion !== packageVersion || !contract.projectChecksum.matches
    || contract.datasetCount !== 1 || contract.modelCount !== 1
    || contract.recipeCount !== 0 || contract.resultCount !== 0 || contract.runCount !== 0
    || !contract.activeModelId) {
    throw new Error(`The blocked CTA-PLS setup changed the zero-run archive boundary: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectInitialCtaPlsArchive(projectPath) {
  const { project, manifest, projectText } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    projectChecksum: archiveProjectChecksum(projectText, manifest),
    datasetCount: project.datasets?.length ?? null,
    modelCount: project.models?.length ?? null,
    recipeCount: project.recipes?.length ?? null,
    resultCount: project.results?.length ?? null,
    runCount: Array.isArray(workspace?.runs) ? workspace.runs.length : 0,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: Array.isArray(workspace?.nodes) ? workspace.nodes.length : 0,
    edges: Array.isArray(workspace?.edges) ? workspace.edges.length : 0,
  };
  if (contract.manifestEngineVersion !== packageVersion || !contract.projectChecksum.matches
    || contract.datasetCount !== 1 || contract.modelCount !== 0 || contract.recipeCount !== 0
    || contract.resultCount !== 0 || contract.runCount !== 0 || contract.activeModelId !== null
    || contract.nodes !== 0 || contract.edges !== 0) {
    throw new Error(`The CTA-PLS fixture was not a canonical data-only archive: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedCtaPlsArchive(projectPath, runId) {
  const { project, manifest, projectText } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved CTA-PLS archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const cta = estimation?.cta_pls;
  const estimates = Array.isArray(cta?.estimates) ? cta.estimates : [];
  const recipeConstructs = Array.isArray(recipe?.model?.constructs) ? recipe.model.constructs : [];
  const predictor = recipeConstructs.find((construct) => construct.name === "Predictor");
  const outcome = recipeConstructs.find((construct) => construct.name === "Outcome");
  const estimateIdentities = estimates.map((row) => [
    row.construct, row.indicator_a, row.indicator_b, row.indicator_c, row.indicator_d, row.pairing,
  ].join("\u0000"));
  const uniqueEstimateIdentities = new Set(estimateIdentities);
  const exactPairings = estimates.map((row) => row.pairing);
  const finiteTetrads = estimates.every((row) => Number.isFinite(row.tetrad)
    && Number.isFinite(row.absolute_tetrad)
    && Math.abs(row.absolute_tetrad - Math.abs(row.tetrad)) <= 1e-12);
  const maximum = estimates.reduce((value, row) => Math.max(value, row.absolute_tetrad), 0);
  const archivedMaximum = predictor ? cta?.max_absolute_tetrad_by_construct?.[predictor.id] : null;
  const unrelatedPayloads = [
    "cbsem", "cca", "endogeneity", "fimix", "gsca", "ipma", "mga", "mga_permutation", "micom",
    "moderated_mediation", "nca", "nonlinear_effects", "pca", "plsc", "predict", "regression",
    "segmentation", "wpls",
  ].filter((key) => estimation?.[key] != null);
  const exactMethodConfig = recipe?.method_config
    && JSON.stringify(Object.keys(recipe.method_config).sort()) === JSON.stringify(["kind"])
    && recipe.method_config.kind === "cta_pls";
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    projectChecksum: archiveProjectChecksum(projectText, manifest),
    identity: {
      resultId: result.id ?? null,
      resultStatus: result.status ?? null,
      provenanceMethod: result.provenance?.method ?? null,
      provenanceMethodVersion: result.provenance?.method_version ?? null,
      provenanceEngineVersion: result.provenance?.engine_version ?? null,
      payloadKind: result.payload?.kind ?? null,
      estimationMethodVersion: estimation?.method_version ?? null,
      ctaMethodVersion: cta?.method_version ?? null,
      covarianceVersion: cta?.covariance ?? null,
    },
    observations: {
      used: estimation?.used_observations ?? null,
      omitted: estimation?.omitted_observations ?? null,
    },
    tetrads: {
      count: estimates.length,
      identities: estimateIdentities,
      uniqueIdentities: uniqueEstimateIdentities.size,
      pairings: exactPairings,
      finite: finiteTetrads,
      algebraicSum: estimates.reduce((value, row) => value + row.tetrad, 0),
      maximum,
      archivedMaximum,
      maximumMatches: Number.isFinite(archivedMaximum) && Math.abs(maximum - archivedMaximum) <= 1e-12,
    },
    warnings: {
      result: cta?.warnings ?? null,
      estimation: estimation?.warnings ?? null,
    },
    unrelatedPayloads,
    recipe: recipe ? {
      schemaVersion: recipe.schema_version ?? null,
      exactMethodConfig,
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      workers: recipe.settings?.workers ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      constructs: recipeConstructs.map((construct) => ({
        id: construct.id,
        name: construct.name,
        shortName: construct.short_name,
        mode: construct.mode,
        indicators: construct.indicators,
      })),
      paths: recipe.model?.paths ?? null,
      controls: recipe.model?.controls ?? null,
      higherOrderConstructs: recipe.model?.higher_order_constructs ?? null,
      interactions: recipe.model?.interactions ?? null,
    } : null,
    project: {
      datasetCount: project.datasets?.length ?? null,
      modelCount: project.models?.length ?? null,
      recipeCount: project.recipes?.length ?? null,
      resultCount: project.results?.length ?? null,
      runCount: workspace?.runs?.length ?? null,
      activeModelId: workspace?.activeModelId ?? null,
      runModelId: run?.modelId ?? null,
      runSnapshotNodes: run?.modelSnapshot?.nodes?.length ?? null,
      runSnapshotEdges: run?.modelSnapshot?.edges?.length ?? null,
    },
  };
  if (contract.manifestEngineVersion !== packageVersion || !contract.projectChecksum.matches
    || contract.identity.resultId !== runId || contract.identity.resultStatus !== "completed"
    || contract.identity.provenanceMethod !== "cta_pls"
    || contract.identity.provenanceMethodVersion !== ctaPlsProvenanceMethodVersion
    || contract.identity.provenanceEngineVersion !== packageVersion
    || contract.identity.payloadKind !== "pls_pm_v1"
    || contract.identity.estimationMethodVersion !== ctaPlsMethodVersion
    || contract.identity.ctaMethodVersion !== ctaPlsMethodVersion
    || contract.identity.covarianceVersion !== ctaPlsCovarianceVersion
    || contract.observations.used !== 120 || contract.observations.omitted !== 0
    || contract.tetrads.count !== 3 || contract.tetrads.uniqueIdentities !== 3
    || JSON.stringify(contract.tetrads.pairings) !== JSON.stringify(ctaPlsPairings)
    || !contract.tetrads.finite || Math.abs(contract.tetrads.algebraicSum) > 1e-10
    || !contract.tetrads.maximumMatches
    || JSON.stringify(contract.warnings.result) !== JSON.stringify([ctaPlsResultWarning])
    || !Array.isArray(contract.warnings.estimation) || contract.warnings.estimation.length !== 1
    || !/CTA-PLS tetrad diagnostics are validated/.test(contract.warnings.estimation[0])
    || unrelatedPayloads.length !== 0 || contract.recipe?.schemaVersion !== 3
    || !contract.recipe.exactMethodConfig || contract.recipe.method !== "cta_pls"
    || contract.recipe.weightingScheme !== "path" || contract.recipe.preprocessing !== "standardized"
    || contract.recipe.missingData !== "listwise_deletion" || contract.recipe.bootstrapSamples !== 0
    || contract.recipe.studentizedInnerSamples !== 0 || contract.recipe.permutationSamples !== 0
    || contract.recipe.workers !== 1 || contract.recipe.caseWeightColumn !== null
    || JSON.stringify(predictor?.indicators) !== JSON.stringify(["x1", "x2", "x3", "x4"])
    || predictor?.short_name !== "X" || predictor?.mode !== "reflective"
    || JSON.stringify(outcome?.indicators) !== JSON.stringify(["y1", "y2"])
    || outcome?.short_name !== "Y" || outcome?.mode !== "reflective"
    || contract.recipe.paths?.length !== 1 || contract.recipe.controls?.length !== 0
    || contract.recipe.higherOrderConstructs?.length !== 0 || contract.recipe.interactions?.length !== 0
    || estimates.some((row) => row.construct !== predictor?.id
      || JSON.stringify([row.indicator_a, row.indicator_b, row.indicator_c, row.indicator_d]) !== JSON.stringify(["x1", "x2", "x3", "x4"]))
    || contract.project.datasetCount !== 1 || contract.project.modelCount !== 1
    || contract.project.recipeCount !== 1 || contract.project.resultCount !== 1 || contract.project.runCount !== 1
    || !contract.project.activeModelId || contract.project.runModelId !== contract.project.activeModelId
    || contract.project.runSnapshotNodes !== 2 || contract.project.runSnapshotEdges !== 1) {
    throw new Error(`The saved CTA-PLS archive did not retain the exact descriptive tetrad result and model snapshot: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedPcaArchive(projectPath, runId) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved PCA archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const methodConfig = recipe?.method_config;
  const retention = methodConfig?.retention;
  const exactMethodConfigKeys = methodConfig && JSON.stringify(Object.keys(methodConfig).sort()) === JSON.stringify(["kind", "retention", "variables"]);
  const exactRetentionKeys = retention && JSON.stringify(Object.keys(retention).sort()) === JSON.stringify(["rule", "threshold"]);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const pca = estimation?.pca;
  const components = Array.isArray(pca?.components) ? pca.components : [];
  const loadings = Array.isArray(pca?.loadings) ? pca.loadings : [];
  const scores = Array.isArray(pca?.scores) ? pca.scores : [];
  const assessment = result.payload?.assessment;
  const componentIds = components.map((row) => row.component);
  const expectedComponentIds = ["PC1", "PC2", "PC3", "PC4"];
  const uniqueLoadingIdentities = new Set(loadings.map((row) => `${row.variable}\u0000${row.component}`));
  const uniqueScoreIdentities = new Set(scores.map((row) => `${row.observation}\u0000${row.component}`));
  const finiteComponents = components.every((row) => [row.eigenvalue, row.explained_variance, row.cumulative_variance].every(Number.isFinite));
  const finiteLoadings = loadings.every((row) => Number.isFinite(row.loading) && Number.isFinite(row.weight));
  const finiteScores = scores.every((row) => Number.isInteger(row.observation) && Number.isFinite(row.score));
  const thresholdCrossing = components.length === 4
    && components[2].cumulative_variance < pcaVarianceThreshold
    && components[3].cumulative_variance >= pcaVarianceThreshold;
  const unrelatedPayloads = [
    "cbsem", "cca", "cta_pls", "endogeneity", "fimix", "gsca", "ipma", "mga", "mga_permutation",
    "micom", "moderated_mediation", "nca", "nonlinear_effects", "plsc", "predict", "regression", "segmentation", "wpls",
  ].filter((key) => estimation?.[key] != null);
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    resultId: result.id ?? null,
    resultStatus: result.status ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    provenanceEngineVersion: result.provenance?.engine_version ?? null,
    payloadKind: result.payload?.kind ?? null,
    estimationMethodVersion: estimation?.method_version ?? null,
    usedObservations: estimation?.used_observations ?? null,
    omittedObservations: estimation?.omitted_observations ?? null,
    assessment: assessment ? { methodVersion: assessment.method_version ?? null, warnings: assessment.warnings ?? null } : null,
    pcaMethodVersion: pca?.method_version ?? null,
    componentRule: pca?.component_rule ?? null,
    retainedComponents: pca?.retained_components ?? null,
    observations: pca?.observations ?? null,
    variables: pca?.variables ?? null,
    componentIds,
    components,
    loadings: loadings.length,
    scores: scores.length,
    uniqueLoadingIdentities: uniqueLoadingIdentities.size,
    uniqueScoreIdentities: uniqueScoreIdentities.size,
    finiteComponents,
    finiteLoadings,
    finiteScores,
    thresholdCrossing,
    warnings: pca?.warnings ?? null,
    unrelatedPayloads,
    recipe: recipe ? {
      schemaVersion: recipe.schema_version ?? null,
      exactMethodConfigKeys,
      exactRetentionKeys,
      methodConfigKind: methodConfig?.kind ?? null,
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      variables: methodConfig?.variables ?? null,
      componentRule: retention?.rule ?? null,
      varianceThreshold: retention?.threshold ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      paths: recipe.model?.paths?.length ?? null,
    } : null,
    models: project.models?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? null,
    edges: workspace?.edges?.length ?? null,
    runModelId: run?.modelId ?? null,
    runModelSnapshot: run?.modelSnapshot ?? null,
  };
  if (contract.manifestEngineVersion !== packageVersion || contract.provenanceEngineVersion !== packageVersion
    || contract.resultStatus !== "completed" || contract.provenanceMethod !== "pca"
    || contract.provenanceMethodVersion !== pcaMethodVersion || contract.payloadKind !== "pls_pm_v1"
    || contract.estimationMethodVersion !== pcaMethodVersion || contract.pcaMethodVersion !== pcaMethodVersion
    || contract.usedObservations !== 140 || contract.omittedObservations !== 0
    || contract.assessment?.methodVersion !== "assessment_not_applicable_v1"
    || JSON.stringify(contract.assessment?.warnings) !== JSON.stringify(["PLS assessment is not applicable to standalone raw-data analyses."])
    || contract.componentRule !== "variance_threshold" || contract.retainedComponents !== 4
    || contract.observations !== 140 || JSON.stringify(contract.variables) !== JSON.stringify(pcaVariables)
    || JSON.stringify(componentIds) !== JSON.stringify(expectedComponentIds)
    || contract.loadings !== 20 || contract.scores !== 560
    || contract.uniqueLoadingIdentities !== 20 || contract.uniqueScoreIdentities !== 560
    || !finiteComponents || !finiteLoadings || !finiteScores || !thresholdCrossing
    || !Array.isArray(contract.warnings) || contract.warnings.length !== 1
    || contract.warnings[0] !== pcaScopeWarning || unrelatedPayloads.length !== 0
    || contract.recipe?.schemaVersion !== 3 || !contract.recipe?.exactMethodConfigKeys || !contract.recipe?.exactRetentionKeys
    || contract.recipe?.methodConfigKind !== "pca"
    || contract.recipe?.method !== "pca" || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "standardized" || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.bootstrapSamples !== 0 || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null || JSON.stringify(contract.recipe?.variables) !== JSON.stringify(pcaVariables)
    || contract.recipe?.componentRule !== "variance_threshold"
    || contract.recipe?.varianceThreshold !== pcaVarianceThreshold
    || contract.recipe?.constructs !== 0 || contract.recipe?.paths !== 0
    || contract.models !== 0 || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0
    || contract.runModelId !== null || contract.runModelSnapshot !== null) {
    throw new Error(`The saved PCA archive did not retain the exact standalone pca_v1 variance-threshold result and model-free snapshot: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedHigherOrderArchive(projectPath, runId, expected) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved HOC archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  if (!recipe || !run) throw new Error("The saved HOC archive did not retain its canonical recipe and visible run snapshot.");
  const declaration = recipe.model?.higher_order_constructs?.[0];
  const hocConstruct = recipe.model?.constructs?.find((construct) => construct.id === expected.hocId);
  const pathRow = recipe.model?.paths?.[0];
  const estimation = result.payload?.estimation;
  const expectedGeneratedIndicators = expected.componentIds
    .map((componentId) => `__qpls_hoc_${expected.hocId}_${componentId}`)
    .sort();
  const generatedOuterRows = (estimation?.outer_estimates ?? [])
    .filter((row) => row.construct === expected.hocId)
    .sort((left, right) => String(left.indicator).localeCompare(String(right.indicator)));
  const hocSnapshotNode = run.modelSnapshot?.nodes?.find((node) => node.id === expected.hocId);
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    resultStatus: result.status ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    payloadKind: result.payload?.kind ?? null,
    recipeId: recipe.id ?? null,
    recipeMethod: recipe.settings?.method ?? null,
    weightingScheme: recipe.settings?.weighting_scheme ?? null,
    preprocessing: recipe.settings?.preprocessing ?? null,
    missingData: recipe.settings?.missing_data ?? null,
    bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
    studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
    permutationSamples: recipe.settings?.permutation_samples ?? null,
    caseWeightColumn: recipe.settings?.case_weight_column ?? null,
    constructCount: recipe.model?.constructs?.length ?? null,
    pathCount: recipe.model?.paths?.length ?? null,
    higherOrderCount: recipe.model?.higher_order_constructs?.length ?? null,
    declaration: declaration ? {
      id: declaration.id,
      components: declaration.components,
      method: declaration.method,
      stageOneRecipe: declaration.stage_one_recipe,
    } : null,
    hocConstruct: hocConstruct ? {
      mode: hocConstruct.mode,
      indicators: hocConstruct.indicators,
    } : null,
    path: pathRow ? { source: pathRow.source, target: pathRow.target } : null,
    generatedOuterIndicators: generatedOuterRows.map((row) => row.indicator),
    finiteGeneratedOuterRows: generatedOuterRows.every((row) => Number.isFinite(row.loading) && Number.isFinite(row.weight)),
    generatedTransformKeys: (estimation?.transforms ?? [])
      .map((row) => row.indicator)
      .filter((indicator) => String(indicator).startsWith("__qpls_hoc_"))
      .sort(),
    constructScoreIds: Object.keys(estimation?.construct_scores ?? {}).sort(),
    runModelId: run.modelId ?? null,
    runSnapshot: hocSnapshotNode ? {
      semantic: hocSnapshotNode.data?.semantic ?? null,
      indicators: hocSnapshotNode.data?.indicators ?? null,
      declaration: hocSnapshotNode.data?.higherOrder ?? null,
    } : null,
  };
  if (contract.manifestEngineVersion !== packageVersion
    || contract.resultStatus !== "completed"
    || contract.provenanceMethod !== "pls_pm"
    || !String(contract.provenanceMethodVersion).includes("pls_pm_v1")
    || contract.payloadKind !== "pls_pm_v1"
    || contract.recipeMethod !== "pls_pm"
    || contract.weightingScheme !== "path"
    || contract.preprocessing !== "standardized"
    || contract.missingData !== "listwise_deletion"
    || contract.bootstrapSamples !== 0 || contract.studentizedInnerSamples !== 0 || contract.permutationSamples !== 0
    || contract.caseWeightColumn !== null
    || contract.constructCount !== 4 || contract.pathCount !== 1 || contract.higherOrderCount !== 1
    || contract.declaration?.id !== expected.hocId
    || JSON.stringify([...(contract.declaration?.components ?? [])].sort()) !== JSON.stringify([...expected.componentIds].sort())
    || contract.declaration?.method !== "two_stage" || contract.declaration?.stageOneRecipe !== null
    || contract.hocConstruct?.mode !== "reflective" || contract.hocConstruct?.indicators?.length !== 0
    || contract.path?.source !== expected.hocId || contract.path?.target !== expected.outcomeId
    || JSON.stringify(contract.generatedOuterIndicators) !== JSON.stringify(expectedGeneratedIndicators)
    || !contract.finiteGeneratedOuterRows
    || JSON.stringify(contract.generatedTransformKeys) !== JSON.stringify(expectedGeneratedIndicators)
    || !expected.componentIds.every((componentId) => contract.constructScoreIds.includes(componentId))
    || !contract.constructScoreIds.includes(expected.hocId) || !contract.constructScoreIds.includes(expected.outcomeId)
    || !contract.runModelId
    || contract.runSnapshot?.semantic !== "higher_order"
    || contract.runSnapshot?.indicators?.length !== 0
    || contract.runSnapshot?.declaration?.method !== "two_stage") {
    throw new Error(`The saved HOC archive did not retain the exact bounded two-stage model, generated identities, and completed result: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectXlsxWorkbookSheets(filePath) {
  const { stdout } = await execFileAsync("tar", ["-xOf", filePath, "xl/workbook.xml"], {
    cwd: root,
    windowsHide: true,
    maxBuffer: 4 * 1024 * 1024,
  });
  return [...stdout.matchAll(/<sheet\s+name="([^"]+)"/g)].map((match) => match[1]);
}

async function xlsxExcludesValidationWitness(filePath) {
  const listed = await execFileAsync("tar", ["-tf", filePath], {
    cwd: root,
    windowsHide: true,
    maxBuffer: 16 * 1024 * 1024,
  });
  const allMembers = listed.stdout.split(/\r?\n/).map((member) => member.trim()).filter(Boolean);
  const uniqueMembers = [...new Set(allMembers)];
  const scannedMembers = uniqueMembers.filter((member) => /(?:\.xml|\.rels)$/i.test(member));
  const worksheetMembers = scannedMembers.filter((member) => /^xl\/worksheets\/[^/]+\.xml$/i.test(member));
  if (uniqueMembers.length !== allMembers.length
    || !scannedMembers.includes("xl/workbook.xml") || worksheetMembers.length === 0) {
    throw new Error(`The XLSX package inventory is incomplete or ambiguous: ${JSON.stringify({
      totalMembers: allMembers.length,
      uniqueMembers: uniqueMembers.length,
      workbookPresent: scannedMembers.includes("xl/workbook.xml"),
      worksheetMembers,
    })}`);
  }
  const forbidden = /validation_witness|regression_(?:process_)?bootstrap_validation_witness_v1|successful_bootstrap|successful_jackknife|failed_jackknife/i;
  const forbiddenMatches = [];
  const worksheetRowCounts = {};
  const extractionRoot = await fs.mkdtemp(path.join(validationResultsDir, ".regression-bootstrap-xlsx-scan-"));
  const extractionErrors = [];
  try {
    await execFileAsync("tar", ["-xf", filePath, "-C", extractionRoot], {
      cwd: root,
      windowsHide: true,
      maxBuffer: 32 * 1024 * 1024,
    });
    for (const member of scannedMembers) {
      const normalizedMember = member.replaceAll("\\", "/");
      const extractedPath = path.resolve(extractionRoot, ...normalizedMember.split("/"));
      const relative = path.relative(extractionRoot, extractedPath);
      if (relative.startsWith("..") || path.isAbsolute(relative)) {
        extractionErrors.push({ member, reason: "member_resolves_outside_scan_root" });
        continue;
      }
      let extractedText;
      try {
        extractedText = await fs.readFile(extractedPath, "utf8");
      } catch (error) {
        extractionErrors.push({ member, reason: String(error) });
        continue;
      }
      if (forbidden.test(extractedText)) forbiddenMatches.push(member);
      if (worksheetMembers.includes(member)) {
        worksheetRowCounts[member] = [...extractedText.matchAll(/<row(?:\s|>)/g)].length;
      }
    }
  } finally {
    await fs.rm(extractionRoot, { recursive: true, force: true });
  }
  return {
    passed: forbiddenMatches.length === 0 && extractionErrors.length === 0,
    total_members: uniqueMembers.length,
    scanned_xml_and_rels_members: scannedMembers,
    worksheet_members: worksheetMembers,
    worksheet_row_counts: worksheetRowCounts,
    forbidden_matches: forbiddenMatches,
    extraction_errors: extractionErrors,
  };
}

function mediationCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-${state}-${nativeViewportLabel}.png`;
}

async function inspectMediationArchiveRunState(projectPath) {
  const { project } = await readNcaArchive(projectPath);
  const recipes = Array.isArray(project.recipes) ? project.recipes : [];
  const results = Array.isArray(project.results) ? project.results : [];
  const runs = Array.isArray(project.layouts?.workspace?.runs)
    ? project.layouts.workspace.runs
    : [];
  return {
    recipeCount: recipes.length,
    resultCount: results.length,
    runCount: runs.length,
    recipeIds: recipes.map((row) => row?.id ?? null),
    resultIds: results.map((row) => row?.id ?? null),
    runIds: runs.map((row) => row?.id ?? null),
  };
}

async function inspectSavedPlscBootstrapArchive(projectPath, runId) {
  const { project, manifest, projectText } = await readNcaArchive(projectPath);
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved PLSc bootstrap archive omitted result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = project.layouts?.workspace?.runs?.find((candidate) => candidate.id === runId);
  const bootstrap = result.payload?.bootstrap;
  const ledger = Array.isArray(bootstrap?.replicate_ledger) ? bootstrap.replicate_ledger : [];
  const successful = Array.isArray(bootstrap?.successful_replicates) ? bootstrap.successful_replicates : [];
  const failed = Array.isArray(bootstrap?.failed_replicates) ? bootstrap.failed_replicates : [];
  const successfulJackknife = Array.isArray(bootstrap?.successful_jackknife_cases)
    ? bootstrap.successful_jackknife_cases : [];
  const failedJackknife = Array.isArray(bootstrap?.failed_jackknife_cases)
    ? bootstrap.failed_jackknife_cases : [];
  const requested = bootstrap?.plan?.replicates;
  const successIndices = new Set(successful.map((row) => row.replicate_index));
  const failedIndices = new Set(failed.map((row) => row.replicate_index));
  const ledgerPartitionExact = Number.isInteger(requested)
    && ledger.length === requested
    && ledger.every((row, index) => row.replicate_index === index
      && ((row.status === "success" && successIndices.has(index) && !failedIndices.has(index))
        || (row.status === "failed" && failedIndices.has(index) && !successIndices.has(index))));
  const primaryWitnessesValid = successful.every((row) => Number.isInteger(row.replicate_index)
    && row.replicate_index >= 0 && row.replicate_index < requested
    && row.parameters && Object.keys(row.parameters).length > 0
    && Object.values(row.parameters).every(Number.isFinite));
  const jackknifeWitnessesValid = successfulJackknife.every((row) => Number.isInteger(row.omitted_case)
    && row.omitted_case >= 0 && row.parameters && Object.keys(row.parameters).length > 0
    && Object.values(row.parameters).every(Number.isFinite));
  const failureReasonsValid = [...failed, ...failedJackknife].every((row) => (
    typeof row.reason_code === "string" && row.reason_code.trim()
    && typeof row.message === "string" && row.message.trim()
  ));
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    runId,
    resultStatus: result.status ?? null,
    runStatus: run?.status ?? null,
    payloadKind: result.payload?.kind ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    recipeMethod: recipe?.settings?.method ?? null,
    recipeMethodConfig: recipe?.method_config ?? null,
    requested,
    usable: bootstrap?.usable_replicates ?? null,
    failed: failed.length,
    ledgerRows: ledger.length,
    successfulWitnesses: successful.length,
    successfulJackknifeWitnesses: successfulJackknife.length,
    failedJackknife: failedJackknife.length,
    ledgerPartitionExact,
    primaryWitnessesValid,
    jackknifeWitnessesValid,
    failureReasonsValid,
    checksum: createHash("sha256").update(projectText, "utf8").digest("hex"),
    immutableRunChecksum: createHash("sha256")
      .update(JSON.stringify({ recipe, result }), "utf8")
      .digest("hex"),
  };
  const valid = contract.manifestEngineVersion === packageVersion
    && contract.resultStatus === "completed" && contract.runStatus === "completed"
    && contract.payloadKind === "pls_pm_v2"
    && contract.provenanceMethodVersion?.split("+").includes(plscBootstrapMethodVersion)
    && contract.provenanceMethodVersion?.split("+").includes("plsc_v2")
    && contract.recipeMethod === "plsc" && contract.recipeMethodConfig?.kind === "plsc"
    && contract.requested === plscBootstrapSamples
    && contract.usable + contract.failed === contract.requested
    && contract.successfulWitnesses === contract.usable
    && contract.successfulJackknifeWitnesses + contract.failedJackknife > 0
    && contract.ledgerPartitionExact && contract.primaryWitnessesValid
    && contract.jackknifeWitnessesValid && contract.failureReasonsValid;
  if (!valid) throw new Error(`The saved PLSc bootstrap archive failed its replayable witness contract: ${JSON.stringify(contract)}`);
  return contract;
}

async function inspectSavedPlsSampleSizePowerArchive(projectPath, runId) {
  const { project, manifest, projectText } = await readNcaArchive(projectPath);
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved prospective PLS-power archive omitted result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = project.layouts?.workspace?.runs?.find((candidate) => candidate.id === runId);
  const analysis = result.payload?.analysis;
  const rows = Array.isArray(analysis?.rows) ? analysis.rows : [];
  const outcomes = Array.isArray(analysis?.outcomes) ? analysis.outcomes : [];
  const expectedGrid = plsSampleSizePowerGrid.split(",").map(Number);
  const successfulOutcomes = outcomes.filter((row) => row?.successful === true);
  const failedOutcomes = outcomes.filter((row) => row?.successful === false);
  const validOutcomes = outcomes.length === expectedGrid.length * plsSampleSizePowerMonteCarloReplicates
    && expectedGrid.every((sampleSize, gridIndex) => outcomes
      .slice(gridIndex * plsSampleSizePowerMonteCarloReplicates, (gridIndex + 1) * plsSampleSizePowerMonteCarloReplicates)
      .every((outcome, replicateIndex) => outcome?.sample_size === sampleSize
        && outcome?.replicate_index === replicateIndex && outcome?.attempted === true
        && typeof outcome?.stream_identity === "string" && /^[0-9a-f]{64}$/.test(outcome.stream_identity)
        && (outcome.successful === true
          ? outcome.converged === true && Number.isFinite(outcome.target_estimate)
            && Number.isFinite(outcome.p_value_two_sided)
            && outcome.bootstrap_requested_replicates === plsSampleSizePowerBootstrapReplicates
            && Number.isInteger(outcome.bootstrap_usable_replicates)
            && Number.isInteger(outcome.bootstrap_failed_replicates)
            && Number.isInteger(outcome.bootstrap_two_sided_exceedances)
            && outcome.bootstrap_usable_replicates + outcome.bootstrap_failed_replicates === plsSampleSizePowerBootstrapReplicates
            && outcome.bootstrap_usable_replicates >= Math.ceil(plsSampleSizePowerBootstrapReplicates * 0.9)
            && outcome.bootstrap_two_sided_exceedances >= 0
            && outcome.bootstrap_two_sided_exceedances <= outcome.bootstrap_usable_replicates
            && outcome.p_value_two_sided === (outcome.bootstrap_two_sided_exceedances + 1) / (outcome.bootstrap_usable_replicates + 1)
            && outcome.failure_code === null && outcome.failure_message === null
          : outcome.converged === false && outcome.target_estimate === null && outcome.p_value_two_sided === null
            && typeof outcome.failure_code === "string" && Boolean(outcome.failure_code.trim())
            && typeof outcome.failure_message === "string" && Boolean(outcome.failure_message.trim()))));
  const validRows = rows.length === expectedGrid.length && rows.every((row, gridIndex) => {
    const sampleSize = expectedGrid[gridIndex];
    const group = outcomes.slice(gridIndex * plsSampleSizePowerMonteCarloReplicates, (gridIndex + 1) * plsSampleSizePowerMonteCarloReplicates);
    const successful = group.filter((outcome) => outcome.successful).length;
    const rejections = group.filter((outcome) => outcome.rejected).length;
    return row?.sample_size === sampleSize
      && row.requested_replicates === plsSampleSizePowerMonteCarloReplicates
      && row.attempted_replicates === plsSampleSizePowerMonteCarloReplicates
      && row.successful_replicates === successful
      && row.failed_replicates === plsSampleSizePowerMonteCarloReplicates - successful
      && row.rejections === rejections
      && Math.abs(row.achieved_power - rejections / plsSampleSizePowerMonteCarloReplicates) <= 1e-12
      && row.qualifies === (row.confidence_lower >= 0.8);
  });
  const firstQualified = rows.find((row) => row.qualifies)?.sample_size ?? null;
  const decisionValid = firstQualified === null
    ? analysis?.decision?.status === "not_reached"
    : analysis?.decision?.status === "reached" && analysis.decision.sample_size === firstQualified;
  const config = recipe?.method_config;
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    runId,
    resultStatus: result.status ?? null,
    runStatus: run?.status ?? null,
    payloadKind: result.payload?.kind ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    recipeSchemaVersion: recipe?.schema_version ?? null,
    recipeMethod: recipe?.settings?.method ?? null,
    recipeMethodConfig: config ?? null,
    rowCount: rows.length,
    outerRequested: outcomes.length,
    outerAttempted: outcomes.filter((row) => row?.attempted === true).length,
    outerUsable: successfulOutcomes.length,
    outerFailed: failedOutcomes.length,
    innerRequested: successfulOutcomes.reduce((total, row) => total + row.bootstrap_requested_replicates, 0),
    innerUsable: successfulOutcomes.reduce((total, row) => total + row.bootstrap_usable_replicates, 0),
    innerFailed: successfulOutcomes.reduce((total, row) => total + row.bootstrap_failed_replicates, 0),
    innerExceedances: successfulOutcomes.reduce((total, row) => total + row.bootstrap_two_sided_exceedances, 0),
    typedOuterFailures: failedOutcomes.every((row) => typeof row.failure_code === "string" && row.failure_code.trim()
      && typeof row.failure_message === "string" && row.failure_message.trim()),
    validOutcomes,
    validRows,
    decisionValid,
    checksum: createHash("sha256").update(projectText, "utf8").digest("hex"),
    immutableRunChecksum: createHash("sha256").update(JSON.stringify({ recipe, result }), "utf8").digest("hex"),
  };
  const valid = contract.manifestEngineVersion === packageVersion
    && contract.resultStatus === "completed" && contract.runStatus === "completed"
    && contract.payloadKind === "pls_sample_size_power_v2"
    && contract.provenanceMethod === "pls_sample_size_power"
    && contract.provenanceMethodVersion === plsSampleSizePowerMethodVersion
    && contract.recipeSchemaVersion === 3 && contract.recipeMethod === "pls_sample_size_power"
    && config?.kind === "pls_sample_size_power"
    && config?.inference === "case_bootstrap_null_centered_two_sided_plus_one"
    && JSON.stringify(config?.sample_size_grid) === JSON.stringify(expectedGrid)
    && config?.monte_carlo_replicates === plsSampleSizePowerMonteCarloReplicates
    && config?.bootstrap_replicates === plsSampleSizePowerBootstrapReplicates
    && recipe?.settings?.seed === plsSampleSizePowerSeed
    && analysis?.schema_version === 2
    && analysis?.capability_id === plsSampleSizePowerFeatureId
    && analysis?.method_version === plsSampleSizePowerMethodVersion
    && analysis?.inference_method === "pls_pm_case_bootstrap_null_centered_two_sided_plus_one_v2"
    && /^[0-9a-f]{64}$/.test(analysis?.recipe_digest ?? "")
    && /^[0-9a-f]{64}$/.test(analysis?.outcome_digest ?? "")
    && contract.validOutcomes && contract.validRows && contract.decisionValid && contract.typedOuterFailures
    && result.provenance?.dataset_fingerprint === recipe?.dataset_fingerprint;
  if (!valid) throw new Error(`The saved prospective PLS-power archive failed its exact v2 contract: ${JSON.stringify(contract)}`);
  return contract;
}

function moderationCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-${state}-${nativeViewportLabel}.png`;
}

function mgaCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-mga-${state}-${nativeViewportLabel}.png`;
}

function ccaCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-cca-${state}-${nativeViewportLabel}.png`;
}

function ipmaCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-ipma-${state}-${nativeViewportLabel}.png`;
}

function ncaCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-nca-${state}-${nativeViewportLabel}.png`;
}

function predictionCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-prediction-${state}-${nativeViewportLabel}.png`;
}

function hocCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-hoc-${state}-${nativeViewportLabel}.png`;
}

async function inspectInitialOlsArchive(projectPath) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    models: project.models?.length ?? null,
    recipes: project.recipes?.length ?? null,
    results: project.results?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? 0,
    edges: workspace?.edges?.length ?? 0,
  };
  if (contract.manifestEngineVersion !== packageVersion
    || contract.models !== 0 || contract.recipes !== 0 || contract.results !== 0
    || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0) {
    throw new Error(`The imported OLS fixture was not a canonical data-only project: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedOlsArchive(projectPath, runId) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved OLS archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const methodConfig = recipe?.method_config;
  const regressionModel = methodConfig?.model;
  const exactMethodConfigKeys = methodConfig && JSON.stringify(Object.keys(methodConfig).sort()) === JSON.stringify([
    "controls", "kind", "model", "outcome", "predictors",
  ]);
  const exactRegressionModelKeys = regressionModel && JSON.stringify(Object.keys(regressionModel).sort()) === JSON.stringify(["robust_se", "type"]);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const regression = estimation?.regression;
  const coefficients = Array.isArray(regression?.coefficients) ? regression.coefficients : [];
  const predictions = Array.isArray(regression?.predictions) ? regression.predictions : [];
  const assessment = result.payload?.assessment;
  const expectedTerms = ["intercept", ...olsPredictors, ...olsControls];
  const coefficientContract = coefficients.length === expectedTerms.length && coefficients.every((row, index) => (
    row.term === expectedTerms[index]
    && [row.estimate, row.standard_error, row.statistic, row.p_value_two_sided, row.confidence_interval_lower, row.confidence_interval_upper].every(Number.isFinite)
    && row.standard_error > 0 && row.p_value_two_sided >= 0 && row.p_value_two_sided <= 1
    && row.confidence_interval_lower <= row.estimate && row.confidence_interval_upper >= row.estimate
    && row.odds_ratio == null
  ));
  const predictionContract = predictions.length === 140 && predictions.every((row, index) => (
    row.observation === index && Number.isFinite(row.fitted) && Number.isFinite(row.residual) && row.probability == null
  ));
  const fit = regression?.fit;
  const fitContract = fit
    && [fit.r_squared, fit.adjusted_r_squared, fit.f_statistic, fit.aic, fit.bic, fit.rmse].every(Number.isFinite)
    && fit.log_likelihood == null && fit.pseudo_r_squared == null;
  const unrelatedPayloads = [
    "cbsem", "cca", "cta_pls", "endogeneity", "fimix", "gsca", "ipma", "mga", "mga_permutation",
    "micom", "moderated_mediation", "nca", "nonlinear_effects", "pca", "plsc", "predict", "segmentation", "wpls",
  ].filter((key) => estimation?.[key] != null);
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    resultId: result.id ?? null,
    resultStatus: result.status ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    provenanceEngineVersion: result.provenance?.engine_version ?? null,
    payloadKind: result.payload?.kind ?? null,
    estimationMethodVersion: estimation?.method_version ?? null,
    usedObservations: estimation?.used_observations ?? null,
    omittedObservations: estimation?.omitted_observations ?? null,
    assessment: assessment ? { methodVersion: assessment.method_version ?? null, warnings: assessment.warnings ?? null } : null,
    regressionMethodVersion: regression?.method_version ?? null,
    regressionType: regression?.regression_type ?? null,
    outcome: regression?.outcome ?? null,
    predictors: regression?.predictors ?? null,
    controls: regression?.controls ?? null,
    observations: regression?.observations ?? null,
    coefficientCount: coefficients.length,
    predictionCount: predictions.length,
    coefficientContract,
    predictionContract,
    fitContract: Boolean(fitContract),
    process: regression?.process ?? null,
    warnings: regression?.warnings ?? null,
    unrelatedPayloads,
    recipe: recipe ? {
      schemaVersion: recipe.schema_version ?? null,
      status: recipe.metadata?.status ?? null,
      exactMethodConfigKeys,
      exactRegressionModelKeys,
      methodConfigKind: methodConfig?.kind ?? null,
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      confidenceLevel: recipe.settings?.confidence_level ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      regressionType: regressionModel?.type ?? null,
      outcome: methodConfig?.outcome ?? null,
      predictors: methodConfig?.predictors ?? null,
      controls: methodConfig?.controls ?? null,
      robustSe: regressionModel?.robust_se ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      paths: recipe.model?.paths?.length ?? null,
      controlsCount: recipe.model?.controls?.length ?? null,
      interactions: recipe.model?.interactions?.length ?? null,
      higherOrderConstructs: recipe.model?.higher_order_constructs?.length ?? null,
    } : null,
    models: project.models?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? null,
    edges: workspace?.edges?.length ?? null,
    runModelId: run?.modelId ?? null,
    runModelSnapshot: run?.modelSnapshot ?? null,
  };
  if (contract.manifestEngineVersion !== packageVersion || contract.provenanceEngineVersion !== packageVersion
    || contract.resultStatus !== "completed" || contract.provenanceMethod !== "regression"
    || contract.provenanceMethodVersion !== olsMethodVersion || contract.payloadKind !== "pls_pm_v1"
    || contract.estimationMethodVersion !== olsMethodVersion || contract.regressionMethodVersion !== olsMethodVersion
    || contract.usedObservations !== 140 || contract.omittedObservations !== 0
    || contract.assessment?.methodVersion !== "assessment_not_applicable_v1"
    || JSON.stringify(contract.assessment?.warnings) !== JSON.stringify(["PLS assessment is not applicable to standalone raw-data analyses."])
    || contract.regressionType !== "ols" || contract.outcome !== olsOutcome
    || JSON.stringify(contract.predictors) !== JSON.stringify(olsPredictors)
    || JSON.stringify(contract.controls) !== JSON.stringify(olsControls)
    || contract.observations !== 140 || contract.coefficientCount !== 4 || contract.predictionCount !== 140
    || !contract.coefficientContract || !contract.predictionContract || !contract.fitContract || contract.process !== null
    || !Array.isArray(contract.warnings) || contract.warnings.length !== 1 || contract.warnings[0] !== olsScopeWarning
    || contract.unrelatedPayloads.length !== 0
    || contract.recipe?.schemaVersion !== 3 || !contract.recipe?.exactMethodConfigKeys || !contract.recipe?.exactRegressionModelKeys
    || contract.recipe?.methodConfigKind !== "regression"
    || contract.recipe?.status !== "validated_regression_ols_v1_bounded_scope"
    || contract.recipe?.method !== "regression" || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "unstandardized" || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.confidenceLevel !== 0.95 || contract.recipe?.bootstrapSamples !== 0
    || contract.recipe?.studentizedInnerSamples !== 0 || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null || contract.recipe?.regressionType !== "ols"
    || contract.recipe?.outcome !== olsOutcome || JSON.stringify(contract.recipe?.predictors) !== JSON.stringify(olsPredictors)
    || JSON.stringify(contract.recipe?.controls) !== JSON.stringify(olsControls) || contract.recipe?.robustSe !== "hc3"
    || contract.recipe?.constructs !== 0 || contract.recipe?.paths !== 0 || contract.recipe?.controlsCount !== 0
    || contract.recipe?.interactions !== 0 || contract.recipe?.higherOrderConstructs !== 0
    || contract.models !== 0 || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0
    || contract.runModelId !== null || contract.runModelSnapshot !== null) {
    throw new Error(`The saved OLS archive did not retain the exact standalone regression_ols_v1 result and model-free snapshot: ${JSON.stringify(contract)}`);
  }
  return contract;
}

function logisticNumberClose(actual, expected, tolerance = 1e-8) {
  return Number.isFinite(actual) && Number.isFinite(expected)
    && Math.abs(actual - expected) <= tolerance * Math.max(1, Math.abs(actual), Math.abs(expected));
}

async function inspectInitialLogisticArchive(projectPath) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const contract = {
    manifestSchemaVersion: manifest.schema_version ?? null,
    manifestEngineVersion: manifest.engine_version ?? null,
    checksumAlgorithm: manifest.checksum_algorithm ?? null,
    packageVersion,
    models: project.models?.length ?? null,
    recipes: project.recipes?.length ?? null,
    results: project.results?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? 0,
    edges: workspace?.edges?.length ?? 0,
  };
  if (contract.manifestSchemaVersion !== 5 || contract.manifestEngineVersion !== packageVersion
    || contract.checksumAlgorithm !== "sha256"
    || contract.models !== 0 || contract.recipes !== 0 || contract.results !== 0
    || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0) {
    throw new Error(`The imported logistic fixture was not a canonical checksummed data-only project: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedLogisticArchive(projectPath, runId) {
  const { project, manifest, projectText } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved logistic archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const regression = estimation?.regression;
  const coefficients = Array.isArray(regression?.coefficients) ? regression.coefficients : [];
  const predictions = Array.isArray(regression?.predictions) ? regression.predictions : [];
  const diagnostics = regression?.logistic;
  const profile = diagnostics?.outcome_profile;
  const convergence = diagnostics?.convergence;
  const classification = diagnostics?.classification;
  const fit = regression?.fit;
  const assessment = result.payload?.assessment;
  const expectedTerms = ["intercept", ...logisticPredictors, ...logisticControls];

  const coefficientContract = coefficients.length === expectedTerms.length && coefficients.every((row, index) => (
    row.term === expectedTerms[index]
    && [
      row.estimate,
      row.standard_error,
      row.statistic,
      row.p_value_two_sided,
      row.confidence_interval_lower,
      row.confidence_interval_upper,
      row.odds_ratio,
      row.odds_ratio_confidence_interval_lower,
      row.odds_ratio_confidence_interval_upper,
    ].every(Number.isFinite)
    && row.standard_error > 0
    && row.p_value_two_sided >= 0 && row.p_value_two_sided <= 1
    && logisticNumberClose(row.statistic, row.estimate / row.standard_error)
    && row.confidence_interval_lower <= row.estimate && row.confidence_interval_upper >= row.estimate
    && row.odds_ratio > 0
    && logisticNumberClose(row.odds_ratio, Math.exp(row.estimate))
    && logisticNumberClose(row.odds_ratio_confidence_interval_lower, Math.exp(row.confidence_interval_lower))
    && logisticNumberClose(row.odds_ratio_confidence_interval_upper, Math.exp(row.confidence_interval_upper))
  ));

  let reconstructedLogLikelihood = 0;
  let reconstructedTruePositive = 0;
  let reconstructedTrueNegative = 0;
  let reconstructedFalsePositive = 0;
  let reconstructedFalseNegative = 0;
  let reconstructedZeroCases = 0;
  let reconstructedOneCases = 0;
  const predictionContract = predictions.length === logisticObservations && predictions.every((row, index) => {
    if (row.observation !== index || !Number.isFinite(row.fitted) || !Number.isFinite(row.residual)
      || !Number.isFinite(row.probability) || row.probability < 0 || row.probability > 1
      || !logisticNumberClose(row.fitted, row.probability)) return false;
    const observed = row.probability + row.residual;
    const observedOne = logisticNumberClose(observed, 1);
    const observedZero = logisticNumberClose(observed, 0);
    if (!observedOne && !observedZero) return false;
    if (observedOne) reconstructedOneCases += 1;
    else reconstructedZeroCases += 1;
    reconstructedLogLikelihood += observedOne ? Math.log(row.probability) : Math.log(1 - row.probability);
    const predictedOne = row.probability >= 0.5;
    if (observedOne && predictedOne) reconstructedTruePositive += 1;
    else if (observedZero && !predictedOne) reconstructedTrueNegative += 1;
    else if (observedZero && predictedOne) reconstructedFalsePositive += 1;
    else reconstructedFalseNegative += 1;
    return true;
  });

  const parameterCount = expectedTerms.length;
  const fitContract = Boolean(fit)
    && [
      fit.log_likelihood,
      fit.null_log_likelihood,
      fit.pseudo_r_squared,
      fit.deviance,
      fit.null_deviance,
      fit.likelihood_ratio_chi_square,
      fit.likelihood_ratio_p_value,
      fit.aic,
      fit.bic,
    ].every(Number.isFinite)
    && fit.r_squared == null && fit.adjusted_r_squared == null && fit.f_statistic == null && fit.rmse == null
    && fit.pseudo_r_squared_method === "mcfadden_v1"
    && fit.likelihood_ratio_degrees_of_freedom === parameterCount - 1
    && fit.likelihood_ratio_p_value >= 0 && fit.likelihood_ratio_p_value <= 1
    && logisticNumberClose(fit.log_likelihood, reconstructedLogLikelihood)
    && logisticNumberClose(fit.pseudo_r_squared, 1 - fit.log_likelihood / fit.null_log_likelihood)
    && logisticNumberClose(fit.deviance, -2 * fit.log_likelihood)
    && logisticNumberClose(fit.null_deviance, -2 * fit.null_log_likelihood)
    && logisticNumberClose(fit.likelihood_ratio_chi_square, fit.null_deviance - fit.deviance)
    && logisticNumberClose(fit.aic, fit.deviance + 2 * parameterCount)
    && logisticNumberClose(fit.bic, fit.deviance + Math.log(logisticObservations) * parameterCount);

  const profileContract = Boolean(profile)
    && profile.outcome === logisticOutcome
    && profile.coding === "numeric_0_1_exact_v1"
    && profile.complete_cases === logisticObservations
    && profile.omitted_cases === 0
    && profile.zero_count === logisticZeroCases
    && profile.one_count === logisticOneCases
    && profile.invalid_count === 0
    && profile.readiness === "ready"
    && logisticNumberClose(profile.prevalence, logisticOneCases / logisticObservations)
    && profile.zero_count === reconstructedZeroCases
    && profile.one_count === reconstructedOneCases;

  const convergenceContract = Boolean(convergence)
    && convergence.algorithm === "deterministic_newton_irls_v1"
    && convergence.converged === true
    && Number.isInteger(convergence.iterations) && convergence.iterations > 0 && convergence.iterations <= 100
    && convergence.max_iterations === 100
    && logisticNumberClose(convergence.tolerance, 1e-8)
    && Number.isFinite(convergence.final_max_abs_step) && convergence.final_max_abs_step >= 0
    && convergence.final_max_abs_step < convergence.tolerance
    && logisticNumberClose(convergence.separation_probability_tolerance, 1e-9);

  const classificationContract = Boolean(classification)
    && logisticNumberClose(classification.threshold, 0.5)
    && classification.true_positive === reconstructedTruePositive
    && classification.true_negative === reconstructedTrueNegative
    && classification.false_positive === reconstructedFalsePositive
    && classification.false_negative === reconstructedFalseNegative
    && logisticNumberClose(classification.accuracy, (reconstructedTruePositive + reconstructedTrueNegative) / logisticObservations)
    && logisticNumberClose(classification.sensitivity, reconstructedTruePositive / (reconstructedTruePositive + reconstructedFalseNegative))
    && logisticNumberClose(classification.specificity, reconstructedTrueNegative / (reconstructedTrueNegative + reconstructedFalsePositive));

  const unrelatedPayloads = [
    "cbsem", "cca", "cta_pls", "endogeneity", "fimix", "gsca", "ipma", "mga", "mga_permutation",
    "micom", "moderated_mediation", "nca", "nonlinear_effects", "pca", "plsc", "predict", "segmentation", "wpls",
  ].filter((key) => estimation?.[key] != null);
  const projectChecksum = createHash("sha256").update(projectText, "utf8").digest("hex");
  const methodConfig = recipe?.method_config;
  const contract = {
    manifest: {
      schemaVersion: manifest.schema_version ?? null,
      engineVersion: manifest.engine_version ?? null,
      checksumAlgorithm: manifest.checksum_algorithm ?? null,
      declaredProjectChecksum: manifest.checksums?.["project.json"] ?? null,
      calculatedProjectChecksum: projectChecksum,
      projectChecksumMatches: manifest.checksums?.["project.json"] === projectChecksum,
    },
    packageVersion,
    resultId: result.id ?? null,
    resultStatus: result.status ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    provenanceEngineVersion: result.provenance?.engine_version ?? null,
    payloadKind: result.payload?.kind ?? null,
    estimationMethodVersion: estimation?.method_version ?? null,
    usedObservations: estimation?.used_observations ?? null,
    omittedObservations: estimation?.omitted_observations ?? null,
    assessment: assessment ? { methodVersion: assessment.method_version ?? null, warnings: assessment.warnings ?? null } : null,
    regressionMethodVersion: regression?.method_version ?? null,
    regressionType: regression?.regression_type ?? null,
    outcome: regression?.outcome ?? null,
    predictors: regression?.predictors ?? null,
    controls: regression?.controls ?? null,
    observations: regression?.observations ?? null,
    coefficientCount: coefficients.length,
    predictionCount: predictions.length,
    coefficientContract,
    predictionContract,
    fitContract,
    profileContract,
    convergenceContract,
    classificationContract,
    profile,
    convergence,
    classification,
    process: regression?.process ?? null,
    warnings: regression?.warnings ?? null,
    unrelatedPayloads,
    recipe: recipe ? {
      id: recipe.id ?? null,
      schemaVersion: recipe.schema_version ?? null,
      status: recipe.metadata?.status ?? null,
      methodConfig,
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      confidenceLevel: recipe.settings?.confidence_level ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      workers: recipe.settings?.workers ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      paths: recipe.model?.paths?.length ?? null,
      controlsCount: recipe.model?.controls?.length ?? null,
      interactions: recipe.model?.interactions?.length ?? null,
      higherOrderConstructs: recipe.model?.higher_order_constructs?.length ?? null,
    } : null,
    identity: {
      resultId: result.id ?? null,
      recipeId: recipe?.id ?? null,
      runId: run?.id ?? null,
      resultCount: project.results?.length ?? null,
      recipeCount: project.recipes?.length ?? null,
    },
    project: {
      models: project.models?.length ?? null,
      activeModelId: workspace?.activeModelId ?? null,
      nodes: workspace?.nodes?.length ?? null,
      edges: workspace?.edges?.length ?? null,
    },
    run: run ? {
      method: run.method ?? null,
      status: run.status ?? null,
      logs: run.logs?.length ?? 0,
      modelId: run.modelId ?? null,
      modelSnapshot: run.modelSnapshot ?? null,
    } : null,
  };

  if (contract.manifest.schemaVersion !== 5 || contract.manifest.engineVersion !== packageVersion
    || contract.manifest.checksumAlgorithm !== "sha256" || !contract.manifest.projectChecksumMatches
    || contract.resultStatus !== "completed" || contract.provenanceMethod !== "regression"
    || contract.provenanceMethodVersion !== logisticMethodVersion || contract.provenanceEngineVersion !== packageVersion
    || contract.payloadKind !== "pls_pm_v1" || contract.estimationMethodVersion !== logisticMethodVersion
    || contract.regressionMethodVersion !== logisticMethodVersion || contract.regressionType !== "logistic"
    || contract.usedObservations !== logisticObservations || contract.omittedObservations !== 0
    || contract.assessment?.methodVersion !== "assessment_not_applicable_v1"
    || JSON.stringify(contract.assessment?.warnings) !== JSON.stringify(["PLS assessment is not applicable to standalone raw-data analyses."])
    || contract.outcome !== logisticOutcome
    || JSON.stringify(contract.predictors) !== JSON.stringify(logisticPredictors)
    || JSON.stringify(contract.controls) !== JSON.stringify(logisticControls)
    || contract.observations !== logisticObservations || contract.coefficientCount !== expectedTerms.length
    || contract.predictionCount !== logisticObservations || !contract.coefficientContract || !contract.predictionContract
    || !contract.fitContract || !contract.profileContract || !contract.convergenceContract || !contract.classificationContract
    || contract.process !== null || JSON.stringify(contract.warnings) !== JSON.stringify([logisticScopeWarning])
    || contract.unrelatedPayloads.length !== 0
    || contract.recipe?.schemaVersion !== 3 || contract.recipe?.status !== "validated_regression_logistic_v2_bounded_scope"
    || JSON.stringify(contract.recipe?.methodConfig) !== JSON.stringify({
      kind: "regression",
      outcome: logisticOutcome,
      predictors: logisticPredictors,
      controls: logisticControls,
      model: { type: "logistic" },
    })
    || contract.recipe?.method !== "regression" || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "unstandardized" || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.confidenceLevel !== 0.95 || contract.recipe?.bootstrapSamples !== 0
    || contract.recipe?.studentizedInnerSamples !== 0 || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.workers !== 1 || contract.recipe?.caseWeightColumn !== null
    || contract.recipe?.constructs !== 0 || contract.recipe?.paths !== 0 || contract.recipe?.controlsCount !== 0
    || contract.recipe?.interactions !== 0 || contract.recipe?.higherOrderConstructs !== 0
    || contract.project.models !== 0 || contract.project.activeModelId !== null
    || contract.project.nodes !== 0 || contract.project.edges !== 0
    || contract.run?.method !== "Binary Logistic Regression" || contract.run?.status !== "completed"
    || !Number.isInteger(contract.run?.logs) || contract.run.logs < 1
    || contract.run?.modelId !== null || contract.run?.modelSnapshot !== null) {
    throw new Error(`The saved logistic archive did not retain the exact typed v3, arithmetic, checksum, and model-free v2 contract: ${JSON.stringify(contract)}`);
  }
  return contract;
}

function exactZeroBasedPartition(successes, failures, total, key) {
  const success = successes.map((row) => row?.[key]);
  const failed = failures.map((row) => row?.[key]);
  const ascending = (values) => values.every((value) => Number.isInteger(value) && value >= 0)
    && values.every((value, index) => index === 0 || values[index - 1] < value);
  const combined = [...success, ...failed].sort((left, right) => left - right);
  return ascending(success)
    && ascending(failed)
    && combined.length === total
    && combined.every((value, index) => value === index);
}

async function inspectSavedRegressionBootstrapArchive(projectPath, runIds) {
  const { project, manifest, projectText } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const projectChecksum = createHash("sha256").update(projectText, "utf8").digest("hex");
  const inspectRun = (model, runId) => {
    const result = project.results?.find((candidate) => candidate.id === runId);
    if (!result) throw new Error(`The saved regression bootstrap archive did not contain ${model} result ${runId}.`);
    const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
    const run = workspace?.runs?.find((candidate) => candidate.id === runId);
    const estimation = result.payload?.estimation;
    const regression = estimation?.regression;
    const bootstrap = regression?.bootstrap;
    const witness = bootstrap?.validation_witness;
    const coefficients = Array.isArray(bootstrap?.coefficients) ? bootstrap.coefficients : [];
    const failedReplicates = Array.isArray(bootstrap?.failed_replicates) ? bootstrap.failed_replicates : [];
    const successfulBootstrap = Array.isArray(witness?.successful_bootstrap) ? witness.successful_bootstrap : [];
    const successfulJackknife = Array.isArray(witness?.successful_jackknife) ? witness.successful_jackknife : [];
    const failedJackknife = Array.isArray(witness?.failed_jackknife) ? witness.failed_jackknife : [];
    const logistic = model === "logistic";
    const baseMethodVersion = logistic ? "regression_logistic_v2" : "regression_ols_v1";
    const provenanceMethodVersion = `${baseMethodVersion}+${regressionBootstrapMethodVersion}`;
    const outcome = logistic ? "bin_y" : "y";
    const vectorsValid = [...successfulBootstrap, ...successfulJackknife].every((row) => (
      Array.isArray(row.coefficients)
      && row.coefficients.length === regressionBootstrapTerms.length
      && row.coefficients.every((value) => Number.isFinite(value) && (!logistic || Number.isFinite(Math.exp(value))))
    ));
    const reasonsValid = [...failedReplicates, ...failedJackknife].every((row) => (
      typeof row.reason_code === "string" && row.reason_code.trim()
      && typeof row.message === "string" && row.message.trim()
    ));
    const bootstrapPartitionExact = exactZeroBasedPartition(
      successfulBootstrap,
      failedReplicates,
      regressionBootstrapSamples,
      "replicate_index",
    );
    const jackknifePartitionExact = exactZeroBasedPartition(
      successfulJackknife,
      failedJackknife,
      regressionBootstrapObservations,
      "omitted_case",
    );
    const publicCoefficientContract = coefficients.length === regressionBootstrapTerms.length
      && coefficients.every((row, index) => (
        row.term === regressionBootstrapTerms[index]
        && [
          row.original, row.bootstrap_mean, row.bias, row.standard_error, row.replicate_max_abs,
          row.test_tolerance, row.percentile_lower, row.percentile_upper,
        ].every(Number.isFinite)
        && row.usable_replicates === bootstrap.usable_replicates
        && ["available", "unavailable"].includes(row.test?.status)
        && ["available", "unavailable"].includes(row.bca?.status)
        && (logistic
          ? row.odds_ratio != null
            && [row.odds_ratio.original, row.odds_ratio.percentile_lower, row.odds_ratio.percentile_upper].every(Number.isFinite)
            && ["available", "unavailable"].includes(row.odds_ratio.bca?.status)
          : row.odds_ratio == null)
      ));
    const methodConfig = recipe?.method_config;
    const contract = {
      model,
      runId,
      baseMethodVersion,
      provenanceMethodVersion: result.provenance?.method_version ?? null,
      estimationMethodVersion: estimation?.method_version ?? null,
      regressionMethodVersion: regression?.method_version ?? null,
      resultStatus: result.status ?? null,
      runStatus: run?.status ?? null,
      payloadKind: result.payload?.kind ?? null,
      genericPlsBootstrapAbsent: result.payload?.bootstrap == null,
      processAbsent: regression?.process == null,
      outcome: regression?.outcome ?? null,
      predictors: regression?.predictors ?? null,
      controls: regression?.controls ?? null,
      observations: regression?.observations ?? null,
      bootstrap: bootstrap ? {
        methodVersion: bootstrap.method_version ?? null,
        algorithm: bootstrap.algorithm ?? null,
        streamToken: bootstrap.stream_token ?? null,
        intervalPolicy: bootstrap.interval_policy ?? null,
        testReference: bootstrap.test_reference ?? null,
        testTolerancePolicy: bootstrap.test_tolerance_policy ?? null,
        alternative: bootstrap.alternative ?? null,
        confidenceLevel: bootstrap.confidence_level ?? null,
        requestedReplicates: bootstrap.requested_replicates ?? null,
        usableReplicates: bootstrap.usable_replicates ?? null,
        failedReplicates: failedReplicates.length,
        minimumUsableFraction: bootstrap.minimum_usable_fraction ?? null,
        jackknifeCases: bootstrap.jackknife_cases ?? null,
        usableJackknifeCases: bootstrap.usable_jackknife_cases ?? null,
        seed: bootstrap.seed ?? null,
        workers: bootstrap.workers ?? null,
        coefficientCount: coefficients.length,
        publicCoefficientContract,
      } : null,
      witness: witness ? {
        methodVersion: witness.method_version ?? null,
        terms: witness.terms ?? null,
        successfulBootstrap: successfulBootstrap.length,
        successfulJackknife: successfulJackknife.length,
        failedJackknife: failedJackknife.length,
        vectorsValid,
        reasonsValid,
        bootstrapPartitionExact,
        jackknifePartitionExact,
      } : null,
      recipe: recipe ? {
        schemaVersion: recipe.schema_version ?? null,
        status: recipe.metadata?.status ?? null,
        bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
        workers: recipe.settings?.workers ?? null,
        seed: recipe.settings?.seed ?? null,
        confidenceLevel: recipe.settings?.confidence_level ?? null,
        preprocessing: recipe.settings?.preprocessing ?? null,
        missingData: recipe.settings?.missing_data ?? null,
        methodConfig,
      } : null,
      modelFree: project.models?.length === 0
        && workspace?.activeModelId == null
        && (workspace?.nodes?.length ?? 0) === 0
        && (workspace?.edges?.length ?? 0) === 0
        && run?.modelId == null
        && run?.modelSnapshot == null,
    };
    const expectedMethodConfig = {
      kind: "regression",
      outcome,
      predictors: regressionBootstrapPredictors,
      controls: regressionBootstrapControls,
      model: logistic ? { type: "logistic" } : { type: "ols", robust_se: "hc3" },
      bootstrap: { algorithm: "case_resampling", intervals: ["percentile", "bca"] },
    };
    const valid = contract.resultStatus === "completed"
      && contract.runStatus === "completed"
      && result.provenance?.method === "regression"
      && contract.provenanceMethodVersion === provenanceMethodVersion
      && contract.estimationMethodVersion === baseMethodVersion
      && contract.regressionMethodVersion === baseMethodVersion
      && contract.payloadKind === "pls_pm_v1"
      && contract.genericPlsBootstrapAbsent
      && contract.processAbsent
      && contract.outcome === outcome
      && JSON.stringify(contract.predictors) === JSON.stringify(regressionBootstrapPredictors)
      && JSON.stringify(contract.controls) === JSON.stringify(regressionBootstrapControls)
      && contract.observations === regressionBootstrapObservations
      && contract.bootstrap?.methodVersion === regressionBootstrapMethodVersion
      && contract.bootstrap?.algorithm === "indexed_case_resampling_v1"
      && contract.bootstrap?.streamToken === "quickpls_indexed_resampling_v1"
      && contract.bootstrap?.intervalPolicy === "percentile_primary_bca_conditional_v1"
      && contract.bootstrap?.testReference === "standard_normal_bootstrap_ratio_v1"
      && contract.bootstrap?.testTolerancePolicy === "64eps_max_1_original_replicates_v1"
      && contract.bootstrap?.alternative === "two_sided"
      && contract.bootstrap?.confidenceLevel === 0.95
      && contract.bootstrap?.requestedReplicates === regressionBootstrapSamples
      && contract.bootstrap?.usableReplicates + contract.bootstrap?.failedReplicates === regressionBootstrapSamples
      && contract.bootstrap?.usableReplicates >= Math.ceil(0.9 * regressionBootstrapSamples)
      && contract.bootstrap?.minimumUsableFraction === 0.9
      && contract.bootstrap?.jackknifeCases === regressionBootstrapObservations
      && contract.bootstrap?.usableJackknifeCases === successfulJackknife.length
      && contract.bootstrap?.seed === regressionBootstrapSeed
      && contract.bootstrap?.workers === regressionBootstrapWorkers
      && contract.bootstrap?.publicCoefficientContract
      && contract.witness?.methodVersion === regressionBootstrapWitnessVersion
      && JSON.stringify(contract.witness?.terms) === JSON.stringify(regressionBootstrapTerms)
      && contract.witness?.successfulBootstrap === contract.bootstrap?.usableReplicates
      && contract.witness?.successfulJackknife === contract.bootstrap?.usableJackknifeCases
      && contract.witness?.failedJackknife === regressionBootstrapObservations - contract.bootstrap?.usableJackknifeCases
      && contract.witness?.vectorsValid
      && contract.witness?.reasonsValid
      && contract.witness?.bootstrapPartitionExact
      && contract.witness?.jackknifePartitionExact
      && contract.recipe?.schemaVersion === 3
      && contract.recipe?.status === "validated_regression_bootstrap_v1_bounded_scope"
      && contract.recipe?.bootstrapSamples === regressionBootstrapSamples
      && contract.recipe?.workers === regressionBootstrapWorkers
      && contract.recipe?.seed === regressionBootstrapSeed
      && contract.recipe?.confidenceLevel === 0.95
      && contract.recipe?.preprocessing === "unstandardized"
      && contract.recipe?.missingData === "listwise_deletion"
      && JSON.stringify(contract.recipe?.methodConfig) === JSON.stringify(expectedMethodConfig)
      && contract.modelFree;
    if (!valid) {
      throw new Error(`The saved ${model} regression bootstrap archive contract was invalid: ${JSON.stringify(contract)}`);
    }
    return contract;
  };
  const ols = inspectRun("ols", runIds.ols);
  const logistic = inspectRun("logistic", runIds.logistic);
  const witnessBoundary = {
    passed: ols.witness?.methodVersion === regressionBootstrapWitnessVersion
      && logistic.witness?.methodVersion === regressionBootstrapWitnessVersion
      && JSON.stringify(ols.witness?.terms) === JSON.stringify(regressionBootstrapTerms)
      && JSON.stringify(logistic.witness?.terms) === JSON.stringify(regressionBootstrapTerms)
      && ols.witness?.bootstrapPartitionExact === true
      && logistic.witness?.bootstrapPartitionExact === true
      && ols.witness?.jackknifePartitionExact === true
      && logistic.witness?.jackknifePartitionExact === true,
    termOrderExact: JSON.stringify(ols.witness?.terms) === JSON.stringify(regressionBootstrapTerms)
      && JSON.stringify(logistic.witness?.terms) === JSON.stringify(regressionBootstrapTerms),
    bootstrapIndexPartitionExact: ols.witness?.bootstrapPartitionExact === true
      && logistic.witness?.bootstrapPartitionExact === true,
    jackknifeIndexPartitionExact: ols.witness?.jackknifePartitionExact === true
      && logistic.witness?.jackknifePartitionExact === true,
  };
  const contract = {
    manifest: {
      schemaVersion: manifest.schema_version ?? null,
      engineVersion: manifest.engine_version ?? null,
      checksumAlgorithm: manifest.checksum_algorithm ?? null,
      declaredProjectChecksum: manifest.checksums?.["project.json"] ?? null,
      calculatedProjectChecksum: projectChecksum,
      projectChecksumMatches: manifest.checksums?.["project.json"] === projectChecksum,
    },
    resultCount: project.results?.length ?? null,
    recipeCount: project.recipes?.length ?? null,
    modelCount: project.models?.length ?? null,
    ols,
    logistic,
    witnessBoundary,
  };
  if (contract.manifest.schemaVersion !== 5
    || contract.manifest.engineVersion !== packageVersion
    || contract.manifest.checksumAlgorithm !== "sha256"
    || !contract.manifest.projectChecksumMatches
    || contract.resultCount !== 2 || contract.recipeCount !== 2 || contract.modelCount !== 0
    || !witnessBoundary.passed) {
    throw new Error(`The saved regression bootstrap project did not retain two exact checksummed model-free runs: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedProcessV2Archive(projectPath, runId) {
  if (!processV2ExpectedGraphCounts) {
    throw new Error("The independent PROCESS v2 expected-count contract was not provisioned before archive inspection.");
  }
  const { project, manifest, projectText } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved PROCESS v2 archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const regression = estimation?.regression;
  const process = regression?.process;
  const graph = process?.graph_v2;
  const bootstrap = graph?.bootstrap;
  const witness = bootstrap?.validation_witness;
  const reference = Array.isArray(graph?.reference_effects) ? graph.reference_effects : [];
  const conditional = Array.isArray(graph?.conditional_indirect_effects) ? graph.conditional_indirect_effects : [];
  const indices = Array.isArray(graph?.moderated_mediation_indices) ? graph.moderated_mediation_indices : [];
  const slopes = Array.isArray(graph?.simple_slopes) ? graph.simple_slopes : [];
  const estimandIds = [...reference, ...conditional, ...indices, ...slopes].map((row) => row.effect_id);
  const successfulBootstrap = Array.isArray(witness?.successful_bootstrap) ? witness.successful_bootstrap : [];
  const failedReplicates = Array.isArray(bootstrap?.failed_replicates) ? bootstrap.failed_replicates : [];
  const successfulJackknife = Array.isArray(witness?.successful_jackknife) ? witness.successful_jackknife : [];
  const failedJackknife = Array.isArray(witness?.failed_jackknife) ? witness.failed_jackknife : [];
  const bootstrapPartitionExact = exactZeroBasedPartition(
    successfulBootstrap, failedReplicates, processV2Samples, "replicate_index",
  );
  const jackknifePartitionExact = exactZeroBasedPartition(
    successfulJackknife, failedJackknife, processV2Observations, "omitted_case",
  );
  const witnessVectorsValid = [...successfulBootstrap, ...successfulJackknife].every((row) => (
    Array.isArray(row.estimates) && row.estimates.length === estimandIds.length && row.estimates.every(Number.isFinite)
  ));
  const reasonsValid = [...failedReplicates, ...failedJackknife].every((row) => (
    processV2ReplicateFailureReasonCodes.has(row.reason_code)
    && typeof row.message === "string" && row.message.trim()
  ));
  const methodConfig = recipe?.method_config;
  const relationship = methodConfig?.model?.relationship;
  const expectedPaths = [
    { from: "X", to: "Y" }, { from: "X", to: "M1" }, { from: "M1", to: "M2" },
    { from: "M2", to: "Y" }, { from: "X", to: "M3" }, { from: "M3", to: "Y" },
    { from: "X", to: "M4" }, { from: "M4", to: "Y" },
  ];
  const expectedModerators = [
    { variable: "W", scale: "continuous" }, { variable: "B", scale: "binary_0_1" },
  ];
  const expectedModerations = [
    { from: "X", to: "Y", moderator: "W", conditioning_moderator: "B" },
    { from: "X", to: "M3", moderator: "W" },
    { from: "M4", to: "Y", moderator: "B" },
  ];
  const projectChecksum = createHash("sha256").update(projectText, "utf8").digest("hex");
  const manifestContract = {
    schemaVersion: manifest.schema_version ?? null,
    engineVersion: manifest.engine_version ?? null,
    checksumAlgorithm: manifest.checksum_algorithm ?? null,
    declaredProjectChecksum: manifest.checksums?.["project.json"] ?? null,
    calculatedProjectChecksum: projectChecksum,
    projectChecksumMatches: manifest.checksums?.["project.json"] === projectChecksum,
  };
  const graphCounts = {
    completeCases: graph?.complete_cases ?? null,
    omittedCases: graph?.omitted_cases ?? null,
    equations: graph?.equations?.length ?? null,
    paths: graph?.paths?.length ?? null,
    moderations: graph?.moderations?.length ?? null,
    referenceEffects: reference.length,
    conditionalIndirectEffects: conditional.length,
    moderatedMediationIndices: indices.length,
    simpleSlopes: slopes.length,
    plots: graph?.plots?.length ?? null,
    conditionalPlotPoints: Array.isArray(graph?.plots) ? graph.plots.reduce((total, plot) => (
      total + (Array.isArray(plot.series) ? plot.series.reduce((seriesTotal, series) => (
        seriesTotal + (Array.isArray(series.points) ? series.points.length : 0)
      ), 0) : 0)
    ), 0) : null,
    johnsonNeyman: graph?.johnson_neyman?.length ?? null,
    johnsonNeymanRegionRows: Array.isArray(graph?.johnson_neyman) ? graph.johnson_neyman.reduce((total, row) => (
      total + (row.status === "available" && Array.isArray(row.regions) ? row.regions.length : 1)
    ), 0) : null,
    availableJohnsonNeyman: Array.isArray(graph?.johnson_neyman)
      ? graph.johnson_neyman.filter((row) => row.status === "available").length : null,
    johnsonNeymanCurvePoints: Array.isArray(graph?.johnson_neyman) ? graph.johnson_neyman.reduce((total, row) => (
      total + (row.status === "available" && Array.isArray(row.curve_points) ? row.curve_points.length : 0)
    ), 0) : null,
    estimands: bootstrap?.estimands?.length ?? null,
  };
  const genericRegressionShellNotApplicable = regression?.method_version === processV2MethodVersion
    && regression?.regression_type === "process"
    && regression?.observations === graph?.complete_cases
    && Array.isArray(regression?.coefficients) && regression.coefficients.length === 0
    && regression?.fit === null
    && Array.isArray(regression?.predictions) && regression.predictions.length === 0
    && !("logistic" in regression)
    && !("bootstrap" in regression)
    && !("mediation" in estimation)
    && !("moderation" in estimation);
  const witnessBoundary = {
    passed: witness?.method_version === processV2WitnessVersion
      && estimandIds.length === 24 && new Set(estimandIds).size === 24
      && JSON.stringify(witness?.estimand_ids) === JSON.stringify(estimandIds)
      && JSON.stringify(bootstrap?.estimands?.map((row) => row.effect_id)) === JSON.stringify(estimandIds)
      && bootstrapPartitionExact && jackknifePartitionExact && witnessVectorsValid && reasonsValid,
    witnessMethodVersion: witness?.method_version ?? null,
    estimandIds,
    estimandOrderExact: JSON.stringify(witness?.estimand_ids) === JSON.stringify(estimandIds)
      && JSON.stringify(bootstrap?.estimands?.map((row) => row.effect_id)) === JSON.stringify(estimandIds),
    bootstrapIndexPartitionExact: bootstrapPartitionExact,
    jackknifeIndexPartitionExact: jackknifePartitionExact,
  };
  const contract = {
    manifest: manifestContract,
    resultStatus: result.status ?? null,
    runStatus: run?.status ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    estimationMethodVersion: result.payload?.estimation?.method_version ?? null,
    processMethodVersion: process?.method_version ?? null,
    model: process?.model ?? null,
    outcome: regression?.outcome ?? null,
    predictors: regression?.predictors ?? null,
    controls: regression?.controls ?? null,
    graphCounts,
    genericRegressionShellNotApplicable,
    policies: graph?.policies ?? null,
    bootstrap: bootstrap ? {
      methodVersion: bootstrap.method_version ?? null,
      algorithm: bootstrap.algorithm ?? null,
      intervalPolicy: bootstrap.interval_policy ?? null,
      testReference: bootstrap.test_reference ?? null,
      streamToken: bootstrap.stream_token ?? null,
      requestedReplicates: bootstrap.requested_replicates ?? null,
      usableReplicates: bootstrap.usable_replicates ?? null,
      failedReplicates: failedReplicates.length,
      jackknifeCases: bootstrap.jackknife_cases ?? null,
      usableJackknifeCases: bootstrap.usable_jackknife_cases ?? null,
      seed: bootstrap.seed ?? null,
      workers: bootstrap.workers ?? null,
    } : null,
    witnessBoundary,
    identity: {
      resultId: result.id ?? null,
      recipeId: recipe?.id ?? null,
      runId: run?.id ?? null,
      resultCount: project.results?.length ?? null,
      recipeCount: project.recipes?.length ?? null,
      witnessCount: witness ? 1 : 0,
    },
    recipe: recipe ? {
      schemaVersion: recipe.schema_version ?? null,
      status: recipe.metadata?.status ?? null,
      settings: recipe.settings ?? null,
      methodConfig,
    } : null,
    modelFree: project.models?.length === 0 && workspace?.activeModelId == null
      && (workspace?.nodes?.length ?? 0) === 0 && (workspace?.edges?.length ?? 0) === 0
      && run?.modelId == null && run?.modelSnapshot == null,
  };
  const valid = manifestContract.schemaVersion === 5 && manifestContract.engineVersion === packageVersion
    && manifestContract.checksumAlgorithm === "sha256" && manifestContract.projectChecksumMatches
    && project.results?.length === 1 && project.recipes?.length === 1 && project.models?.length === 0
    && contract.resultStatus === "completed" && contract.runStatus === "completed"
    && contract.provenanceMethod === "regression"
    && contract.provenanceMethodVersion === `${processV2MethodVersion}+${processV2BootstrapMethodVersion}`
    && contract.estimationMethodVersion === processV2MethodVersion
    && contract.processMethodVersion === processV2MethodVersion && contract.model === "graph"
    && contract.outcome === "Y"
    && JSON.stringify(contract.predictors) === JSON.stringify(["X", "M1", "M2", "M3", "M4", "W", "B"])
    && JSON.stringify(contract.controls) === JSON.stringify(["C"])
    && contract.genericRegressionShellNotApplicable
    && processV2PoliciesExact(contract.policies)
    && JSON.stringify(graphCounts) === JSON.stringify(processV2ExpectedGraphCounts)
    && contract.bootstrap?.methodVersion === processV2BootstrapMethodVersion
    && contract.bootstrap?.algorithm === "indexed_case_resampling_v1"
    && contract.bootstrap?.intervalPolicy === "percentile_primary_bca_conditional_v1"
    && contract.bootstrap?.testReference === "standard_normal_bootstrap_ratio_v1"
    && contract.bootstrap?.streamToken === "process_indexed_case_stream_v1"
    && contract.bootstrap?.requestedReplicates === processV2Samples
    && contract.bootstrap?.usableReplicates + contract.bootstrap?.failedReplicates === processV2Samples
    && contract.bootstrap?.usableReplicates >= Math.ceil(0.9 * processV2Samples)
    && contract.bootstrap?.jackknifeCases === processV2Observations
    && contract.bootstrap?.usableJackknifeCases === successfulJackknife.length
    && contract.bootstrap?.seed === processV2Seed && contract.bootstrap?.workers === processV2Workers
    && witnessBoundary.passed
    && contract.identity.resultId === runId
    && contract.identity.recipeId === result.provenance?.recipe_id
    && contract.identity.runId === runId
    && contract.identity.resultCount === 1
    && contract.identity.recipeCount === 1
    && contract.identity.witnessCount === 1
    && contract.recipe?.schemaVersion === 3
    && contract.recipe?.status === "validated_regression_process_v2_plus_bootstrap_v1_bounded_scope"
    && contract.recipe?.settings?.bootstrap_samples === processV2Samples
    && contract.recipe?.settings?.workers === processV2Workers && contract.recipe?.settings?.seed === processV2Seed
    && contract.recipe?.settings?.preprocessing === "unstandardized"
    && contract.recipe?.settings?.missing_data === "listwise_deletion"
    && methodConfig?.kind === "regression" && methodConfig?.outcome === "Y"
    && JSON.stringify(methodConfig?.predictors) === JSON.stringify(["X", "M1", "M2", "M3", "M4", "W", "B"])
    && JSON.stringify(methodConfig?.controls) === JSON.stringify(["C"])
    && methodConfig?.model?.type === "process" && relationship?.model === "graph"
    && relationship?.focal_predictor === "X"
    && JSON.stringify(relationship?.paths) === JSON.stringify(expectedPaths)
    && JSON.stringify(relationship?.moderators) === JSON.stringify(expectedModerators)
    && JSON.stringify(relationship?.moderations) === JSON.stringify(expectedModerations)
    && relationship?.continuous_product_centering === "equation_complete_case_mean_v1"
    && JSON.stringify(methodConfig?.bootstrap) === JSON.stringify({ algorithm: "case_resampling", intervals: ["percentile", "bca"] })
    && contract.modelFree;
  if (!valid) throw new Error(`The saved PROCESS v2 archive contract was invalid: ${JSON.stringify(contract)}`);
  return contract;
}

async function inspectProcessV2LogicalArchiveState(projectPath) {
  const { project, manifest, projectText } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const results = Array.isArray(project.results) ? project.results : [];
  const recipes = Array.isArray(project.recipes) ? project.recipes : [];
  const runs = Array.isArray(workspace?.runs) ? workspace.runs : [];
  const completedResults = results.filter((result) => result?.status === "completed");
  const completedRunIds = completedResults.map((result) => result.id);
  const witnessRunIds = completedResults.filter((result) => (
    result.payload?.estimation?.regression?.process?.graph_v2?.bootstrap?.validation_witness?.method_version
      === processV2WitnessVersion
  )).map((result) => result.id);
  const recipeIds = completedResults.map((result) => result.provenance?.recipe_id ?? null);
  const workspaceRunIds = runs.filter((run) => run?.status === "completed").map((run) => run.id);
  const projectChecksum = createHash("sha256").update(projectText, "utf8").digest("hex");
  return {
    manifestValid: manifest.schema_version === 5
      && manifest.checksum_algorithm === "sha256"
      && manifest.checksums?.["project.json"] === projectChecksum,
    completedResultCount: completedResults.length,
    witnessCount: witnessRunIds.length,
    completedRunIds,
    witnessRunIds,
    recipeIds,
    recipeCount: recipes.length,
    workspaceRunIds,
    selectedRunId: workspace?.diagramOverlaySettings?.selectedRunId ?? null,
  };
}

async function processV2SidecarState(projectPath) {
  const directory = path.dirname(projectPath);
  const basename = path.basename(projectPath);
  const entries = await fs.readdir(directory, { withFileTypes: true });
  const absolute = entries.filter((entry) => entry.isFile() && entry.name.startsWith(`${basename}.`))
    .map((entry) => path.join(directory, entry.name))
    .sort();
  const artifacts = (await Promise.all(absolute.map(artifactDigest))).filter(Boolean);
  const present = artifacts.map((entry) => entry.path);
  return {
    prefix: path.relative(root, projectPath).replaceAll("\\", "/"),
    coversEverySiblingPrefix: true,
    present,
    artifacts,
  };
}

async function processV2SettledAutosaveState(projectPath, { primaryDurability }) {
  const relative = (candidate) => path.relative(root, candidate).replaceAll("\\", "/");
  const autosavePath = `${projectPath}.autosave`;
  const required = [relative(autosavePath), relative(`${autosavePath}.identity.json`)];
  const allowed = [
    ...required,
    relative(`${autosavePath}.bak`),
    ...(primaryDurability ? [relative(`${projectPath}.bak`), relative(`${projectPath}.identity.json`)] : []),
  ].sort();
  const sidecars = await processV2SidecarState(projectPath);
  const missing = required.filter((entry) => !sidecars.present.includes(entry));
  const forbidden = sidecars.present.filter((entry) => !allowed.includes(entry));
  const logicalState = missing.length === 0
    ? await inspectProcessV2LogicalArchiveState(autosavePath)
    : null;
  return {
    ...sidecars,
    required,
    allowed,
    missing,
    forbidden,
    exactAllowedIdentity: missing.length === 0 && forbidden.length === 0,
    autosavePath: relative(autosavePath),
    logicalState,
  };
}

async function captureProcessV2SidecarEvidence(label, state) {
  const captures = [];
  for (const [index, source] of state.artifacts.entries()) {
    const sourcePath = path.resolve(root, source.path);
    const snapshotPath = `${processV2ResourceSnapshotPrefix}-sidecar-${label}-${index}.bin`;
    const temporaryPath = `${snapshotPath}.copying`;
    try {
      await fs.copyFile(sourcePath, temporaryPath, fsConstants.COPYFILE_EXCL);
      const temporary = await artifactDigest(temporaryPath);
      if (!temporary || temporary.size !== source.size || temporary.sha256 !== source.sha256) {
        throw new Error(`PROCESS v2 sidecar evidence copy drifted: ${JSON.stringify({ source, temporary })}`);
      }
      await fs.link(temporaryPath, snapshotPath);
      await fs.rm(temporaryPath, { force: true });
      const snapshot = await artifactDigest(snapshotPath);
      if (!snapshot || snapshot.size !== source.size || snapshot.sha256 !== source.sha256) {
        throw new Error(`PROCESS v2 sidecar evidence publication drifted: ${JSON.stringify({ source, snapshot })}`);
      }
      captures.push({ source_path: source.path, source_size: source.size, source_sha256: source.sha256, snapshot });
    } catch (error) {
      await fs.rm(temporaryPath, { force: true });
      await fs.rm(snapshotPath, { force: true });
      throw error;
    }
  }
  return captures;
}

async function clearProcessV2ResetArtifacts(projectPath) {
  const directory = path.dirname(projectPath);
  const basename = path.basename(projectPath);
  const entries = await fs.readdir(directory, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.isFile() && (entry.name === basename || entry.name.startsWith(`${basename}.`))) {
      await fs.rm(path.join(directory, entry.name), { force: true });
    }
  }
}

async function inspectInitialCbsemArchive(projectPath) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    models: project.models?.length ?? null,
    recipes: project.recipes?.length ?? null,
    results: project.results?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? 0,
    edges: workspace?.edges?.length ?? 0,
  };
  if (contract.manifestEngineVersion !== packageVersion
    || contract.models !== 0 || contract.recipes !== 0 || contract.results !== 0
    || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0) {
    throw new Error(`The imported CB-SEM fixture was not a canonical data-only project: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectSavedCbsemArchive(projectPath, runId) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved CB-SEM archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const methodConfig = recipe?.method_config;
  const exactMethodConfigKeys = methodConfig && JSON.stringify(Object.keys(methodConfig).sort()) === JSON.stringify([
    "bootstrap_samples", "estimator", "input", "kind", "mean_structure", "model_type",
  ]);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const cbsem = estimation?.cbsem;
  const parameters = Array.isArray(cbsem?.parameters) ? cbsem.parameters : [];
  const standardized = Array.isArray(cbsem?.standardized) ? cbsem.standardized : [];
  const implied = Array.isArray(cbsem?.implied_covariance) ? cbsem.implied_covariance : [];
  const residualCovariance = Array.isArray(cbsem?.residual_covariance) ? cbsem.residual_covariance : [];
  const residualCorrelation = Array.isArray(cbsem?.residual_correlation) ? cbsem.residual_correlation : [];
  const modification = Array.isArray(cbsem?.modification_indices) ? cbsem.modification_indices : [];
  const parameterNames = parameters.map((row) => row.name);
  const standardizedNames = standardized.map((row) => row.name);
  const matrixContract = (rows) => rows.length === 81 && rows.every((row) => (
    typeof row.row === "string" && row.row.length > 0
    && typeof row.column === "string" && row.column.length > 0
    && Number.isFinite(row.value)
  ));
  const fit = cbsem?.fit;
  const fitContract = fit
    && fit.method_version === cbsemFitMethodVersion
    && [fit.chi_square, fit.srmr, fit.aic, fit.bic, fit.baseline_chi_square].every(Number.isFinite)
    && Number.isInteger(fit.degrees_of_freedom) && fit.degrees_of_freedom >= 0
    && Number.isInteger(fit.baseline_degrees_of_freedom) && fit.baseline_degrees_of_freedom >= 0
    && [fit.p_value, fit.cfi, fit.tli, fit.rmsea, fit.rmsea_ci_lower, fit.rmsea_ci_upper].every((value) => value === null || Number.isFinite(value));
  const unrelatedPayloads = [
    "cca", "cta_pls", "endogeneity", "fimix", "gsca", "ipma", "mga", "mga_permutation", "micom",
    "moderated_mediation", "nca", "nonlinear_effects", "pca", "plsc", "predict", "regression", "segmentation", "wpls",
  ].filter((key) => estimation?.[key] != null);
  const constructIndicators = recipe?.model?.constructs?.map((construct) => ({
    name: construct.name,
    mode: construct.mode,
    indicators: construct.indicators?.length ?? 0,
  })) ?? [];
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    resultId: result.id ?? null,
    resultStatus: result.status ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    provenanceEngineVersion: result.provenance?.engine_version ?? null,
    payloadKind: result.payload?.kind ?? null,
    estimationMethodVersion: estimation?.method_version ?? null,
    usedObservations: estimation?.used_observations ?? null,
    omittedObservations: estimation?.omitted_observations ?? null,
    cbsem: cbsem ? {
      methodVersion: cbsem.method_version ?? null,
      modelType: cbsem.model_type ?? null,
      estimator: cbsem.estimator ?? null,
      input: cbsem.input ?? null,
      meanStructure: cbsem.mean_structure ?? null,
      converged: cbsem.converged ?? null,
      iterations: cbsem.iterations ?? null,
      objective: cbsem.objective ?? null,
      gradientNorm: cbsem.gradient_norm ?? null,
      sampleSize: cbsem.sample_size ?? null,
      parameterCount: parameters.length,
      uniqueParameterNames: new Set(parameterNames).size,
      standardizedCount: standardized.length,
      standardizedIdentityMatch: JSON.stringify(standardizedNames) === JSON.stringify(parameterNames),
      impliedCount: implied.length,
      residualCovarianceCount: residualCovariance.length,
      residualCorrelationCount: residualCorrelation.length,
      matrixContract: matrixContract(implied) && matrixContract(residualCovariance) && matrixContract(residualCorrelation),
      modificationCount: modification.length,
      modificationContract: modification.length > 0 && modification.every((row) => (
        row.method_version === cbsemModificationMethodVersion
        && typeof row.kind === "string" && typeof row.lhs === "string" && typeof row.rhs === "string"
        && Number.isFinite(row.modification_index)
        && (row.expected_parameter_change === null || Number.isFinite(row.expected_parameter_change))
      )),
      fitContract: Boolean(fitContract),
      diagnostics: cbsem.diagnostics ?? null,
      warnings: cbsem.warnings ?? null,
      bootstrap: cbsem.bootstrap ?? null,
      multigroup: cbsem.multigroup ?? null,
    } : null,
    unrelatedPayloads,
    recipe: recipe ? {
      schemaVersion: recipe.schema_version ?? null,
      status: recipe.metadata?.status ?? null,
      exactMethodConfigKeys,
      methodConfigKind: methodConfig?.kind ?? null,
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      workers: recipe.settings?.workers ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      modelType: methodConfig?.model_type ?? null,
      estimator: methodConfig?.estimator ?? null,
      input: methodConfig?.input ?? null,
      meanStructure: methodConfig?.mean_structure ?? null,
      methodBootstrapSamples: methodConfig?.bootstrap_samples ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      constructIndicators,
      paths: recipe.model?.paths?.length ?? null,
      controls: recipe.model?.controls?.length ?? null,
      interactions: recipe.model?.interactions?.length ?? null,
      higherOrderConstructs: recipe.model?.higher_order_constructs?.length ?? null,
    } : null,
    models: project.models?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? null,
    edges: workspace?.edges?.length ?? null,
    runModelId: run?.modelId ?? null,
    runSnapshotNodes: run?.modelSnapshot?.nodes?.length ?? null,
    runSnapshotEdges: run?.modelSnapshot?.edges?.length ?? null,
  };
  if (contract.manifestEngineVersion !== packageVersion || contract.provenanceEngineVersion !== packageVersion
    || contract.resultStatus !== "completed" || contract.provenanceMethod !== "cbsem"
    || contract.provenanceMethodVersion !== cbsemProvenanceMethodVersion || contract.payloadKind !== "pls_pm_v1"
    || contract.estimationMethodVersion !== cbsemMethodVersion || contract.usedObservations !== 240 || contract.omittedObservations !== 0
    || !contract.cbsem || contract.cbsem.methodVersion !== cbsemMethodVersion || contract.cbsem.modelType !== "sem"
    || contract.cbsem.estimator !== "ml" || contract.cbsem.input !== "raw" || contract.cbsem.meanStructure !== false
    || contract.cbsem.converged !== true || !Number.isInteger(contract.cbsem.iterations) || contract.cbsem.iterations < 1
    || !Number.isFinite(contract.cbsem.objective) || !Number.isFinite(contract.cbsem.gradientNorm) || contract.cbsem.sampleSize !== 240
    || contract.cbsem.parameterCount !== 23 || contract.cbsem.uniqueParameterNames !== 23
    || contract.cbsem.standardizedCount !== 23 || !contract.cbsem.standardizedIdentityMatch
    || contract.cbsem.impliedCount !== 81 || contract.cbsem.residualCovarianceCount !== 81 || contract.cbsem.residualCorrelationCount !== 81
    || !contract.cbsem.matrixContract || contract.cbsem.modificationCount !== 50 || !contract.cbsem.modificationContract
    || !contract.cbsem.fitContract || !Array.isArray(contract.cbsem.diagnostics) || !Array.isArray(contract.cbsem.warnings)
    || contract.cbsem.bootstrap !== null || contract.cbsem.multigroup !== null || contract.unrelatedPayloads.length !== 0
    || contract.recipe?.schemaVersion !== 3 || !contract.recipe?.exactMethodConfigKeys
    || contract.recipe?.methodConfigKind !== "cbsem"
    || contract.recipe?.status !== "validated_v1_2_4_cbsem_single_group_bounded_scope"
    || contract.recipe?.method !== "cbsem" || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "standardized" || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.workers !== 1 || contract.recipe?.bootstrapSamples !== 0
    || contract.recipe?.studentizedInnerSamples !== 0 || contract.recipe?.permutationSamples !== 0
    || contract.recipe?.caseWeightColumn !== null || contract.recipe?.modelType !== "sem"
    || contract.recipe?.estimator !== "ml" || contract.recipe?.input !== "raw" || contract.recipe?.meanStructure !== false
    || contract.recipe?.methodBootstrapSamples !== 0
    || contract.recipe?.constructs !== 3 || contract.recipe?.paths !== 2 || contract.recipe?.controls !== 0
    || contract.recipe?.interactions !== 0 || contract.recipe?.higherOrderConstructs !== 0
    || constructIndicators.length !== 3 || constructIndicators.some((construct) => construct.mode !== "reflective" || construct.indicators !== 3)
    || contract.models !== 1 || !contract.activeModelId || contract.nodes !== 3 || contract.edges !== 2
    || contract.runModelId !== contract.activeModelId || contract.runSnapshotNodes !== 3 || contract.runSnapshotEdges !== 2) {
    throw new Error(`The saved CB-SEM archive did not retain the exact bounded ML recipe, payload, and model snapshot: ${JSON.stringify(contract)}`);
  }
  return contract;
}

async function inspectInitialGscaArchive(projectPath) {
  const { project, manifest } = await readNcaArchive(projectPath);
  const workspace = project.layouts?.workspace;
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    models: project.models?.length ?? null,
    recipes: project.recipes?.length ?? null,
    results: project.results?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? 0,
    edges: workspace?.edges?.length ?? 0,
  };
  if (contract.manifestEngineVersion !== packageVersion
    || contract.models !== 0 || contract.recipes !== 0 || contract.results !== 0
    || contract.activeModelId !== null || contract.nodes !== 0 || contract.edges !== 0) {
    throw new Error(`The imported GSCA fixture was not a canonical data-only project: ${JSON.stringify(contract)}`);
  }
  return contract;
}

function gscaRowsMatch(actualRows, expectedRows, identity) {
  if (!Array.isArray(actualRows) || !Array.isArray(expectedRows) || actualRows.length !== expectedRows.length) return false;
  const expectedById = new Map(expectedRows.map((row) => [identity(row), row]));
  return actualRows.every((row) => {
    const expected = expectedById.get(identity(row));
    if (!expected) return false;
    return Object.keys(expected).every((key) => typeof expected[key] === "number"
      ? ncaNumberClose(row[key], expected[key])
      : row[key] === expected[key]);
  });
}

async function inspectSavedGscaArchive(projectPath, runId) {
  const [{ project, manifest }, referenceEnvelope] = await Promise.all([
    readNcaArchive(projectPath),
    fs.readFile(gscaReferenceOutputPath, "utf8").then(JSON.parse),
  ]);
  const workspace = project.layouts?.workspace;
  const result = project.results?.find((candidate) => candidate.id === runId);
  if (!result) throw new Error(`The saved GSCA archive did not contain result ${runId}.`);
  const recipe = project.recipes?.find((candidate) => candidate.id === result.provenance?.recipe_id);
  const run = workspace?.runs?.find((candidate) => candidate.id === runId);
  const estimation = result.payload?.estimation;
  const gsca = estimation?.gsca;
  const expected = referenceEnvelope.payload?.estimation?.gsca;
  const assessment = result.payload?.assessment;
  const gscaKeys = Object.keys(gsca ?? {}).sort();
  const expectedGscaKeys = [
    "adjusted_fit", "algorithm", "bootstrap_intervals", "converged", "covariance_discrepancy",
    "covariance_sample_total", "final_change", "fit", "free_parameters", "gfi", "iterations",
    "loadings", "measurement_fit", "method_version", "objective", "observations", "paths",
    "r_squared", "srmr", "standardized_residual_sum", "stop_criterion", "structural_fit", "warnings", "weights",
  ].sort();
  const metricKeys = [
    "objective", "fit", "measurement_fit", "structural_fit", "adjusted_fit", "gfi", "srmr",
    "covariance_discrepancy", "covariance_sample_total", "standardized_residual_sum", "final_change",
  ];
  const metricsMatch = Boolean(gsca && expected && metricKeys.every((key) => ncaNumberClose(gsca[key], expected[key])));
  const gConstructId = recipe?.model?.constructs?.find((construct) => (
    construct.mode === "formative" && JSON.stringify(construct.indicators ?? []) === JSON.stringify(["g1", "g2", "g3"])
  ))?.id;
  const hConstructId = recipe?.model?.constructs?.find((construct) => (
    construct.mode === "reflective" && JSON.stringify(construct.indicators ?? []) === JSON.stringify(["h1", "h2"])
  ))?.id;
  const expectedConstructId = (referenceId) => referenceId === "g" ? gConstructId : referenceId === "h" ? hConstructId : referenceId;
  const remapExpectedRows = (rows, fields) => Array.isArray(rows) ? rows.map((row) => ({
    ...row,
    ...Object.fromEntries(fields.map((field) => [field, expectedConstructId(row[field])])),
  })) : rows;
  const expectedWeights = remapExpectedRows(expected?.weights, ["construct"]);
  const expectedLoadings = remapExpectedRows(expected?.loadings, ["construct"]);
  const expectedPaths = remapExpectedRows(expected?.paths, ["source", "target"]);
  const expectedRSquared = expected && Object.fromEntries(Object.entries(expected.r_squared ?? {})
    .map(([construct, value]) => [expectedConstructId(construct), value]));
  const weightsMatch = gscaRowsMatch(gsca?.weights, expectedWeights, (row) => `${row.construct}\u0000${row.indicator}`);
  const loadingsMatch = gscaRowsMatch(gsca?.loadings, expectedLoadings, (row) => `${row.construct}\u0000${row.indicator}`);
  const pathsMatch = gscaRowsMatch(gsca?.paths, expectedPaths, (row) => `${row.source}\u0000${row.target}`);
  const rSquaredMatch = gsca && expectedRSquared
    && JSON.stringify(Object.keys(gsca.r_squared ?? {}).sort()) === JSON.stringify(Object.keys(expectedRSquared).sort())
    && Object.keys(expectedRSquared).every((key) => ncaNumberClose(gsca.r_squared[key], expectedRSquared[key]));
  const unrelatedPayloads = [
    "cbsem", "cca", "cta_pls", "endogeneity", "fimix", "ipma", "mga", "mga_permutation", "micom",
    "moderated_mediation", "nca", "nonlinear_effects", "pca", "plsc", "predict", "regression", "segmentation", "wpls",
  ].filter((key) => estimation?.[key] != null);
  const constructIndicators = recipe?.model?.constructs?.map((construct) => ({
    name: construct.name,
    mode: construct.mode,
    indicators: construct.indicators ?? [],
  })) ?? [];
  const contract = {
    manifestEngineVersion: manifest.engine_version ?? null,
    packageVersion,
    resultId: result.id ?? null,
    resultStatus: result.status ?? null,
    provenanceMethod: result.provenance?.method ?? null,
    provenanceMethodVersion: result.provenance?.method_version ?? null,
    provenanceEngineVersion: result.provenance?.engine_version ?? null,
    payloadKind: result.payload?.kind ?? null,
    estimationMethodVersion: estimation?.method_version ?? null,
    usedObservations: estimation?.used_observations ?? null,
    omittedObservations: estimation?.omitted_observations ?? null,
    converged: estimation?.converged ?? null,
    iterations: estimation?.iterations ?? null,
    assessment: assessment ? { methodVersion: assessment.method_version ?? null, warnings: assessment.warnings ?? null } : null,
    gsca: gsca ? {
      keys: gscaKeys,
      methodVersion: gsca.method_version ?? null,
      algorithm: gsca.algorithm ?? null,
      converged: gsca.converged ?? null,
      iterations: gsca.iterations ?? null,
      stopCriterion: gsca.stop_criterion ?? null,
      finalChange: gsca.final_change ?? null,
      observations: gsca.observations ?? null,
      freeParameters: gsca.free_parameters ?? null,
      metricsMatch,
      weightsMatch,
      loadingsMatch,
      pathsMatch,
      rSquaredMatch,
      bootstrapIntervals: gsca.bootstrap_intervals ?? null,
      warnings: gsca.warnings ?? null,
    } : null,
    unrelatedPayloads,
    noPlsArtifacts: Array.isArray(estimation?.effects) && estimation.effects.length === 0
      && Array.isArray(estimation?.control_estimates) && estimation.control_estimates.length === 0
      && (estimation?.mediation == null
        || (Array.isArray(estimation.mediation.estimates) && estimation.mediation.estimates.length === 0))
      && (estimation?.moderation == null
        || (Array.isArray(estimation.moderation.estimates) && estimation.moderation.estimates.length === 0)),
    recipe: recipe ? {
      status: recipe.metadata?.status ?? null,
      method: recipe.settings?.method ?? null,
      weightingScheme: recipe.settings?.weighting_scheme ?? null,
      preprocessing: recipe.settings?.preprocessing ?? null,
      missingData: recipe.settings?.missing_data ?? null,
      maxIterations: recipe.settings?.max_iterations ?? null,
      tolerance: recipe.settings?.tolerance ?? null,
      workers: recipe.settings?.workers ?? null,
      bootstrapSamples: recipe.settings?.bootstrap_samples ?? null,
      studentizedInnerSamples: recipe.settings?.studentized_inner_samples ?? null,
      permutationSamples: recipe.settings?.permutation_samples ?? null,
      caseWeightColumn: recipe.settings?.case_weight_column ?? null,
      constructs: recipe.model?.constructs?.length ?? null,
      constructIndicators,
      paths: recipe.model?.paths?.length ?? null,
      controls: recipe.model?.controls?.length ?? null,
      interactions: recipe.model?.interactions?.length ?? null,
      higherOrderConstructs: recipe.model?.higher_order_constructs?.length ?? null,
    } : null,
    models: project.models?.length ?? null,
    activeModelId: workspace?.activeModelId ?? null,
    nodes: workspace?.nodes?.length ?? null,
    edges: workspace?.edges?.length ?? null,
    runModelId: run?.modelId ?? null,
    runSnapshotNodes: run?.modelSnapshot?.nodes?.length ?? null,
    runSnapshotEdges: run?.modelSnapshot?.edges?.length ?? null,
  };
  if (contract.manifestEngineVersion !== packageVersion || contract.provenanceEngineVersion !== packageVersion
    || contract.resultStatus !== "completed" || contract.provenanceMethod !== "gsca"
    || contract.provenanceMethodVersion !== gscaMethodVersion || contract.payloadKind !== "pls_pm_v1"
    || contract.estimationMethodVersion !== gscaMethodVersion || contract.usedObservations !== 140 || contract.omittedObservations !== 0
    || contract.converged !== true || contract.iterations !== 4
    || contract.assessment?.methodVersion !== "assessment_not_applicable_v1"
    || JSON.stringify(contract.assessment?.warnings) !== JSON.stringify(["PLS assessment is not applicable to GSCA ALS component-model estimation."])
    || !contract.gsca || JSON.stringify(contract.gsca.keys) !== JSON.stringify(expectedGscaKeys)
    || contract.gsca.methodVersion !== gscaMethodVersion || contract.gsca.algorithm !== gscaAlgorithmVersion
    || contract.gsca.converged !== true || contract.gsca.iterations !== 4 || contract.gsca.stopCriterion !== 1e-7
    || !Number.isFinite(contract.gsca.finalChange) || contract.gsca.finalChange > 1e-7
    || contract.gsca.observations !== 140 || contract.gsca.freeParameters !== 6
    || !contract.gsca.metricsMatch || !contract.gsca.weightsMatch || !contract.gsca.loadingsMatch
    || !contract.gsca.pathsMatch || !contract.gsca.rSquaredMatch
    || !Array.isArray(contract.gsca.bootstrapIntervals) || contract.gsca.bootstrapIntervals.length !== 0
    || !Array.isArray(contract.gsca.warnings) || contract.gsca.warnings.length !== 1
    || !/GSCA ALS v2 is bounded/i.test(contract.gsca.warnings[0])
    || contract.unrelatedPayloads.length !== 0 || !contract.noPlsArtifacts
    || contract.recipe?.status !== "validated_gsca_als_v2_bounded_scope"
    || contract.recipe?.method !== "gsca" || contract.recipe?.weightingScheme !== "path"
    || contract.recipe?.preprocessing !== "standardized" || contract.recipe?.missingData !== "listwise_deletion"
    || contract.recipe?.maxIterations !== 3_000 || contract.recipe?.tolerance !== 1e-7 || contract.recipe?.workers !== 1
    || contract.recipe?.bootstrapSamples !== 0 || contract.recipe?.studentizedInnerSamples !== 0
    || contract.recipe?.permutationSamples !== 0 || contract.recipe?.caseWeightColumn !== null
    || contract.recipe?.constructs !== 2 || contract.recipe?.paths !== 1 || contract.recipe?.controls !== 0
    || contract.recipe?.interactions !== 0 || contract.recipe?.higherOrderConstructs !== 0
    || JSON.stringify(constructIndicators) !== JSON.stringify([
      { name: "G formative component", mode: "formative", indicators: ["g1", "g2", "g3"] },
      { name: "H reflective component", mode: "reflective", indicators: ["h1", "h2"] },
    ])
    || contract.models !== 1 || !contract.activeModelId || contract.nodes !== 2 || contract.edges !== 1
    || contract.runModelId !== contract.activeModelId || contract.runSnapshotNodes !== 2 || contract.runSnapshotEdges !== 1) {
    throw new Error(`The saved GSCA archive did not retain the exact ALS v2 recipe, numerical payload, and model snapshot: ${JSON.stringify(contract)}`);
  }
  return contract;
}

function olsCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-ols-${state}-${nativeViewportLabel}.png`;
}

function logisticCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-logistic-${state}-${nativeViewportLabel}.png`;
}

function regressionBootstrapCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-regression-bootstrap-${state}-${nativeViewportLabel}.png`;
}

function processV2CaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-process-v2-${state}-${nativeViewportLabel}.png`;
}

function structuralPathRandomizationCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-structural-path-randomization-${state}-${nativeViewportLabel}.png`;
}

function pcaCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-pca-${state}-${nativeViewportLabel}.png`;
}

function ctaPlsCaptureName(sequence, state, viewport = nativeViewportLabel) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-cta-pls-${state}-${viewport}.png`;
}

function cbsemCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-cbsem-${state}-${nativeViewportLabel}.png`;
}

function gscaCaptureName(sequence, state) {
  return `${String(sequence).padStart(2, "0")}-tauri-native-gsca-${state}-${nativeViewportLabel}.png`;
}

async function runFocusedStructuralPathRandomizationAcceptance() {
  if (!requestedStructuralPathRandomizationExportPath) {
    throw new Error("QUICKPLS_STRUCTURAL_PATH_RANDOMIZATION_EXPORT_PATH is required; an enabled XLSX button is not packaged export evidence.");
  }
  const exportTargetPath = await validateRequestedNativeExportPath(
    requestedStructuralPathRandomizationExportPath,
    "QUICKPLS_STRUCTURAL_PATH_RANDOMIZATION_EXPORT_PATH",
  );
  const fixtureProvisioning = evidence.checks.structuralPathRandomizationFixtureProvisioning;
  if (fixtureProvisioning?.passed !== true) {
    throw new Error(`Structural Path Randomization fixture provisioning did not pass: ${JSON.stringify(fixtureProvisioning)}`);
  }
  await seedRecentProject({
    name: structuralPathRandomizationProjectName,
    path: structuralPathRandomizationProjectPath,
    openedAt: "2026-08-13T00:00:00.000Z",
  });
  await reloadToLauncher();
  await openRecentProject(structuralPathRandomizationProjectName, structuralPathRandomizationProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const status = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const columns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  const initialArchive = await inspectInitialNcaArchive(structuralPathRandomizationProjectPath);
  Object.assign(fixtureProvisioning, {
    passed: fixtureProvisioning.passed === true
      && status.includes("180 cases")
      && JSON.stringify(columns) === JSON.stringify(["#", "group", "x1", "x2", "z1", "z2", "y1", "y2"])
      && initialArchive.models === 0 && initialArchive.activeModelId === null,
    status,
    columns,
    observations: 180,
    initialArchive,
  });
  if (!fixtureProvisioning.passed) {
    throw new Error(`The focused Structural Path Randomization fixture was not the exact 180-row data-only MGA reference project: ${JSON.stringify(fixtureProvisioning)}`);
  }
  await capture(structuralPathRandomizationCaptureName(190, "fixture-data"));

  const modelCreation = await createInitialEditableModel(
    structuralPathRandomizationProjectName,
    structuralPathRandomizationModelName,
  );
  await buildThreeConstructMgaModel();
  const model = {
    creation: modelCreation,
    constructs: await page.locator(".react-flow__node-latent").count(),
    constructLabels: (await page.locator(".react-flow__node-latent").allTextContents()).map(compactVisibleText),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    unassignedGroupColumn: await page.locator(".nd-variable-item").filter({ hasText: /^group$/ }).evaluate((element) => !element.classList.contains("assigned")),
  };
  if (model.constructs !== 3 || model.assignedIndicators !== 6 || model.structuralPaths !== 2
    || !model.unassignedGroupColumn
    || !["X", "Z", "Y"].every((name) => model.constructLabels.some((label) => label.includes(name)))) {
    throw new Error(`The visible authoring flow did not produce the exact X -> Y and Z -> Y model: ${JSON.stringify(model)}`);
  }
  await capture(structuralPathRandomizationCaptureName(191, "model"));
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const readSetup = async (dialog, action) => {
    const listbox = dialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
    const permutations = dialog.locator("#nd-calculation-permutations");
    const workers = dialog.locator("#nd-calculation-workers");
    const seed = dialog.locator("#nd-calculation-seed");
    return {
      catalogCount: await listbox.getByRole("option").count(),
      selectedMethod: compactVisibleText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent()),
      permutations: {
        count: await permutations.count(),
        type: await permutations.getAttribute("type"),
        minimum: await permutations.getAttribute("min"),
        maximum: await permutations.getAttribute("max"),
        step: await permutations.getAttribute("step"),
        value: await permutations.inputValue(),
      },
      workers: {
        count: await workers.count(),
        type: await workers.getAttribute("type"),
        minimum: await workers.getAttribute("min"),
        maximum: await workers.getAttribute("max"),
        value: await workers.inputValue(),
      },
      seed: {
        count: await seed.count(),
        type: await seed.getAttribute("type"),
        minimum: await seed.getAttribute("min"),
        maximum: await seed.getAttribute("max"),
        value: await seed.inputValue(),
      },
      bootstrapControls: await dialog.locator("#nd-calculation-bootstrap-samples, #nd-calculation-studentized").count(),
      groupControls: await dialog.locator("#nd-calculation-group-column, #nd-calculation-group-a, #nd-calculation-group-b").count(),
      scopeLabel: compactVisibleText(await dialog.locator("#nd-calculation-permutation-scope span").textContent()),
      scope: compactVisibleText(await dialog.locator("#nd-calculation-permutation-scope strong").textContent()),
      blockers: (await dialog.locator(".nd-blocker li").allTextContents()).map(compactVisibleText),
      startLabel: compactVisibleText(await action.textContent()),
      startEnabled: await action.isEnabled(),
    };
  };
  const expectedSetupContract = (workerCount, startLabel) => ({
    catalogCount: expectedOptionLabels.length,
    selectedMethod: "Structural Path Randomization",
    permutations: {
      count: 1, type: "number", minimum: "99", maximum: "10000", step: "1", value: "10000",
    },
    workers: {
      count: 1, type: "number", minimum: "1", maximum: "64", value: String(workerCount),
    },
    seed: {
      count: 1, type: "number", minimum: "0", maximum: "4294967295", value: "20260718",
    },
    bootstrapControls: 0,
    groupControls: 0,
    scopeLabel: "Validated scope",
    scope: structuralPathRandomizationWarning,
    blockers: [],
    startLabel,
    startEnabled: true,
  });
  const configure = async (workerCount) => {
    const dialog = await openCalculationFromToolbar();
    await dialog.locator("#nd-calculation-method-pls_permutation").click();
    const permutations = dialog.locator("#nd-calculation-permutations");
    const workers = dialog.locator("#nd-calculation-workers");
    const seed = dialog.locator("#nd-calculation-seed");
    await permutations.fill(String(structuralPathRandomizationPermutations));
    await workers.fill(String(workerCount));
    await seed.fill(String(structuralPathRandomizationSeed));
    const start = dialog.getByRole("button", { name: "Start path randomization", exact: true });
    const contract = await readSetup(dialog, start);
    const valid = JSON.stringify(contract)
      === JSON.stringify(expectedSetupContract(workerCount, "Start path randomization"));
    if (!valid) throw new Error(`The Structural Path Randomization setup did not match the exact candidate 10,000-permutation/${workerCount}-worker contract: ${JSON.stringify(contract)}`);
    return { dialog, start, contract };
  };

  const cancellationArchiveBeforeSnapshot = await snapshotStructuralPathRandomizationCancellationArchive(
    "before",
    structuralPathRandomizationProjectPath,
  );
  if (cancellationArchiveBeforeSnapshot.datasetCount !== 1
    || cancellationArchiveBeforeSnapshot.modelCount !== 1
    || cancellationArchiveBeforeSnapshot.modelName !== structuralPathRandomizationModelName
    || JSON.stringify(cancellationArchiveBeforeSnapshot.constructLabels) !== JSON.stringify(["X", "Z", "Y"])
    || JSON.stringify(cancellationArchiveBeforeSnapshot.pathLabels) !== JSON.stringify(structuralPathRandomizationExpectedPathLabels)
    || cancellationArchiveBeforeSnapshot.recipeCount !== 0
    || cancellationArchiveBeforeSnapshot.resultCount !== 0
    || cancellationArchiveBeforeSnapshot.runCount !== 0) {
    throw new Error(`Structural Path Randomization cancellation did not begin from an exact result-free saved archive snapshot: ${JSON.stringify(cancellationArchiveBeforeSnapshot)}`);
  }
  const cancelledSetup = await configure(structuralPathRandomizationCancellationWorkers);
  await capture(structuralPathRandomizationCaptureName(192, "dialog"));
  await cancelledSetup.start.click();
  const activeProgress = cancelledSetup.dialog.locator(
    '.nd-run-progress[aria-busy="true"]:is(.queued,.validating,.running)',
  );
  await activeProgress.waitFor({ state: "visible", timeout: 5_000 });
  const cancel = cancelledSetup.dialog.getByRole("button", { name: "Cancel calculation", exact: true });
  await cancel.waitFor({ state: "visible", timeout: 1_000 });
  const activeCancellation = await cancelledSetup.dialog.evaluate((dialog) => {
    const progress = dialog.querySelector('.nd-run-progress[aria-busy="true"]:is(.queued,.validating,.running)');
    const buttons = [...dialog.querySelectorAll("button")].filter(
      (button) => button.textContent?.replace(/\s+/g, " ").trim() === "Cancel calculation",
    );
    const button = buttons[0] ?? null;
    return {
      ariaBusy: progress?.getAttribute("aria-busy") ?? null,
      status: progress
        ? [...progress.classList].find((className) => ["queued", "validating", "running"].includes(className)) ?? null
        : null,
      phase: progress?.querySelector("strong")?.textContent?.trim() ?? "",
      message: progress?.querySelector("p")?.textContent?.trim() ?? "",
      progressValue: progress?.querySelector("progress")?.getAttribute("value") ?? null,
      progressMax: progress?.querySelector("progress")?.getAttribute("max") ?? null,
      logEntries: progress?.querySelectorAll("ol li").length ?? 0,
      cancelButtonCount: buttons.length,
      cancelButtonEnabled: button instanceof HTMLButtonElement && !button.disabled,
    };
  });
  const {
    cancelButtonCount,
    cancelButtonEnabled,
    ...activeCancellationState
  } = activeCancellation;
  if (activeCancellationState.ariaBusy !== "true" || !activeCancellationState.status
    || cancelButtonCount !== 1 || !cancelButtonEnabled) {
    throw new Error(`Structural Path Randomization cancellation did not expose one genuine active state and enabled Cancel calculation button: ${JSON.stringify(activeCancellation)}`);
  }
  const terminalStatePromise = page.waitForFunction(() => {
    if (document.querySelector('.nd-app[data-surface="results"]')) return "results_surface";
    const dialog = document.querySelector('.nd-dialog-calculation[role="dialog"]');
    if (!dialog) return "dialog_detached";
    if (dialog.querySelector('.nd-run-progress.cancelled[aria-busy="false"]')) return "cancelled";
    if (dialog.querySelector(".nd-run-progress.completed")) return "completed";
    return null;
  }, null, { timeout: 60_000 });
  const [terminalStateHandle] = await Promise.all([
    terminalStatePromise,
    cancel.click({ timeout: 1_000 }),
  ]);
  const terminalOutcome = await terminalStateHandle.jsonValue();
  if (terminalOutcome !== "cancelled") {
    throw new Error(`completion_won_race: Structural Path Randomization reached ${terminalOutcome} before terminal cancellation became authoritative.`);
  }
  const cancelledState = cancelledSetup.dialog.locator('.nd-run-progress.cancelled[aria-busy="false"]');
  const cancelledLifecycle = await cancelledState.evaluate((element) => ({
    ariaBusy: element.getAttribute("aria-busy"),
    status: [...element.classList].find((className) => className === "cancelled") ?? null,
    phase: element.querySelector("strong")?.textContent?.trim() ?? "",
    message: element.querySelector("p")?.textContent?.trim() ?? "",
    progressValue: element.querySelector("progress")?.getAttribute("value") ?? null,
    progressMax: element.querySelector("progress")?.getAttribute("max") ?? null,
    logEntries: element.querySelectorAll("ol li").length,
    logMessages: [...element.querySelectorAll("ol li span")].map((row) => row.textContent?.replace(/\s+/g, " ").trim() ?? ""),
  }));
  const cancellationLogNewest = cancelledLifecycle.logMessages.slice(0, 2);
  const cancellationLogExact = JSON.stringify(cancellationLogNewest)
    === JSON.stringify(["Calculation cancelled.", "Cancellation requested."]);
  if (cancelledLifecycle.ariaBusy !== "false" || cancelledLifecycle.status !== "cancelled"
    || cancelledLifecycle.phase !== "Cancelled" || cancelledLifecycle.message !== "Calculation cancelled."
    || !cancellationLogExact) {
    throw new Error(`Structural Path Randomization cancellation did not reach the exact terminal cancelled contract: ${JSON.stringify({ cancelledLifecycle, cancellationLogNewest })}`);
  }
  const cancellationArchiveAfterSnapshot = await snapshotStructuralPathRandomizationCancellationArchive(
    "after",
    structuralPathRandomizationProjectPath,
  );
  const zeroResultRecipeRunDelta = ["recipeCount", "resultCount", "runCount"].every(
    (key) => cancellationArchiveAfterSnapshot[key] === 0
      && cancellationArchiveBeforeSnapshot[key] === 0,
  ) && [cancellationArchiveBeforeSnapshot, cancellationArchiveAfterSnapshot].every(
    (snapshot) => snapshot.recipeIds.length === 0
      && snapshot.resultIds.length === 0
      && snapshot.runIds.length === 0,
  );
  const archiveSnapshotsByteIdentical = cancellationArchiveAfterSnapshot.artifact.size
    === cancellationArchiveBeforeSnapshot.artifact.size
    && cancellationArchiveAfterSnapshot.artifact.sha256
      === cancellationArchiveBeforeSnapshot.artifact.sha256;
  const retry = cancelledSetup.dialog.getByRole("button", { name: "Retry path randomization", exact: true });
  await retry.waitFor({ state: "visible", timeout: 10_000 });
  const retrySetup = await readSetup(cancelledSetup.dialog, retry);
  const retrySetupMatches = JSON.stringify({ ...retrySetup, startLabel: "Start path randomization" })
    === JSON.stringify(cancelledSetup.contract);
  await capture(structuralPathRandomizationCaptureName(194, "cancelled"));
  const completionWorkers = cancelledSetup.dialog.locator("#nd-calculation-workers");
  await completionWorkers.fill(String(structuralPathRandomizationWorkers));
  const completionSetup = await readSetup(cancelledSetup.dialog, retry);
  const completionSetupExact = JSON.stringify(completionSetup)
    === JSON.stringify(expectedSetupContract(structuralPathRandomizationWorkers, "Retry path randomization"));
  const normalizedCompletionSetup = { ...completionSetup, startLabel: "Start path randomization" };
  evidence.checks.structuralPathRandomizationSetup = {
    passed: completionSetupExact,
    model,
    ...normalizedCompletionSetup,
    feature_id: structuralPathRandomizationFeatureId,
    method_version: structuralPathRandomizationMethodVersion,
    catalogue_snapshot_date: structuralPathRandomizationCatalogueSnapshotDate,
  };
  evidence.checks.structuralPathRandomizationCancellation = {
    passed: false,
    activeLifecycleCaptured: true,
    activeState: activeCancellationState,
    cancelButtonCount,
    cancelButtonEnabled,
    cancelClickDispatched: true,
    terminalOutcome,
    completionWonRace: false,
    cancelledState: cancelledLifecycle,
    cancelledMessage: cancelledLifecycle.message,
    cancellationLogNewest,
    cancellationLogExact,
    archiveBeforeSnapshot: cancellationArchiveBeforeSnapshot,
    archiveAfterSnapshot: cancellationArchiveAfterSnapshot,
    archiveSnapshotsByteIdentical,
    zeroResultRecipeRunDelta,
    noPartialResult: archiveSnapshotsByteIdentical && zeroResultRecipeRunDelta,
    cancellationSetup: cancelledSetup.contract,
    exactFrozenSetupOnRetry: retrySetupMatches,
    retrySetup,
    completionSetup,
    completionSetupExact,
    retryCompleted: false,
    retryRunId: null,
    retryNewIdentity: false,
  };
  if (!archiveSnapshotsByteIdentical || !zeroResultRecipeRunDelta || !retrySetupMatches
    || !completionSetupExact || !await retry.isEnabled()) {
    throw new Error(`Structural Path Randomization cancellation/retry did not preserve setup or discard partial output: ${JSON.stringify(evidence.checks.structuralPathRandomizationCancellation)}`);
  }
  const activeCompletionPromise = captureActiveCalculation(
    cancelledSetup.dialog,
    structuralPathRandomizationCaptureName(195, "running"),
    "Structural Path Randomization 10,000-permutation run",
    { allowTerminalTransitionAfterCapture: true },
  ).then((activeState) => ({ captured: true, ...activeState }))
    .catch((error) => ({ captured: false, detail: error instanceof Error ? error.message : String(error) }));
  const completionProofPromise = page.waitForFunction(() => {
    if (document.querySelector('.nd-app[data-surface="results"]') === null) return null;
    const options = [...document.querySelectorAll(".nd-run-select select option")]
      .filter((option) => /Structural Path Randomization/i.test(option.textContent ?? ""));
    if (options.length !== 1 || !(options[0] instanceof HTMLOptionElement) || !options[0].value) return null;
    return {
      surface: "results",
      completedRunCount: options.length,
      runId: options[0].value,
    };
  }, null, { timeout: 900_000 });
  await retry.click();
  const completionProof = await completionProofPromise.then((handle) => handle.jsonValue());
  const activeCompletion = await activeCompletionPromise;
  if (!activeCompletion.captured) {
    throw new Error(`Structural Path Randomization did not expose a genuine active retry lifecycle: ${JSON.stringify(activeCompletion)}`);
  }
  const runSelect = page.locator(".nd-run-select select");
  const runOptions = runSelect.locator("option").filter({ hasText: /Structural Path Randomization/i });
  await runOptions.first().waitFor({ state: "attached", timeout: 30_000 });
  if (await runOptions.count() !== 1) {
    throw new Error(`Structural Path Randomization exposed ${await runOptions.count()} completed options after one cancelled and one completed run.`);
  }
  const runId = await runOptions.first().getAttribute("value");
  if (!runId) throw new Error("The completed Structural Path Randomization option had no run identifier.");
  if (completionProof?.surface !== "results" || completionProof.completedRunCount !== 1
    || completionProof.runId !== runId) {
    throw new Error(`Structural Path Randomization completion proof did not bind the exact completed result identity: ${JSON.stringify({ completionProof, runId })}`);
  }
  const retryNewIdentity = ![
    ...cancellationArchiveBeforeSnapshot.resultIds,
    ...cancellationArchiveBeforeSnapshot.runIds,
    ...cancellationArchiveAfterSnapshot.resultIds,
    ...cancellationArchiveAfterSnapshot.runIds,
  ].includes(runId);
  Object.assign(evidence.checks.structuralPathRandomizationCancellation, {
    retryCompleted: activeCompletion.captured === true,
    retryRunId: runId,
    retryNewIdentity,
  });
  evidence.checks.structuralPathRandomizationCancellation.passed = Boolean(
    evidence.checks.structuralPathRandomizationCancellation.noPartialResult
      && evidence.checks.structuralPathRandomizationCancellation.exactFrozenSetupOnRetry
      && evidence.checks.structuralPathRandomizationCancellation.completionSetupExact
      && activeCompletion.captured === true
      && retryNewIdentity,
  );
  if (!evidence.checks.structuralPathRandomizationCancellation.passed) {
    throw new Error(`Structural Path Randomization terminal cancellation/retry identity contract failed: ${JSON.stringify(evidence.checks.structuralPathRandomizationCancellation)}`);
  }
  const selectedRunId = await runSelect.inputValue();
  const selectedRunLabel = compactVisibleText(await runSelect.locator("option:checked").textContent());
  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]')
    .getAttribute("data-result-tree-item-id");
  const inferenceGroup = page.getByRole("treeitem", { name: "Inference", exact: true });
  await inferenceGroup.waitFor({ state: "visible", timeout: 15_000 });
  if (await inferenceGroup.getAttribute("aria-expanded") === "false") await inferenceGroup.click();
  const tableItem = page.locator('.nd-result-tree [role="treeitem"][data-result-tree-item-id="permutation"]');
  await tableItem.waitFor({ state: "visible", timeout: 15_000 });
  await tableItem.click();
  await page.getByRole("heading", { name: "Structural path randomization", exact: true }).waitFor({ state: "visible", timeout: 15_000 });
  const tableColumns = (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText);
  const tableRows = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => (
    Array.from(row.querySelectorAll("th, td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
  )));
  const warning = compactVisibleText(await page.locator(".nd-inline-warning").textContent());
  const pGridExact = tableRows.every((row) => {
    const exceedances = Number(row[2]);
    const permutations = Number(row[3]);
    const probability = Number(row[4]);
    return Number.isInteger(exceedances) && exceedances >= 0 && exceedances <= structuralPathRandomizationPermutations
      && permutations === structuralPathRandomizationPermutations
      && Number.isFinite(probability) && Object.is(probability, (exceedances + 1) / (permutations + 1));
  });
  const allTreeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const runDetails = await inspectCurrentRunDetails();
  evidence.checks.structuralPathRandomizationResults = {
    passed: selectedRunId === runId && /Structural Path Randomization/i.test(selectedRunLabel)
      && initialSelectedTable === "model_estimates"
      && JSON.stringify(tableColumns) === JSON.stringify(structuralPathRandomizationExpectedColumns)
      && tableRows.length === 2
      && JSON.stringify(tableRows.map((row) => row[0])) === JSON.stringify(structuralPathRandomizationExpectedPathLabels)
      && tableRows.every((row) => row.length === 5 && Number.isFinite(Number(row[1])))
      && pGridExact && warning === structuralPathRandomizationWarning
      && !allTreeItems.some((label) => /Bootstrap confidence|Bootstrap inference|Studentized confidence/i.test(label))
      && !/\bN\/?A\b|NaN|Infinity/i.test(tableRows.flat().join(" "))
      && runDetails.properties.Method === "Structural Path Randomization"
      && runDetails.properties["Recorded seed"] === String(structuralPathRandomizationSeed)
      && runDetails.properties["Method version"] === `pls_pm_v1+pls_mediation_v1+pls_assessment_v8+${structuralPathRandomizationMethodVersion}`
      && runDetails.properties.Engine === packageVersion
      && runDetails.properties.Weighting === "path"
      && runDetails.properties.Preprocessing === "standardized"
      && typeof runDetails.properties.Recipe === "string" && runDetails.properties.Recipe.length > 0
      && typeof runDetails.properties["Dataset fingerprint"] === "string"
      && runDetails.properties["Dataset fingerprint"].length > 0
      && Number.isInteger(runDetails.logEntries) && runDetails.logEntries >= 1,
    runId,
    selectedRunId,
    selectedRunLabel,
    initialSelectedTable,
    group: compactVisibleText(await inferenceGroup.textContent()),
    tableId: await tableItem.getAttribute("data-result-tree-item-id"),
    title: "Structural path randomization",
    warning,
    columns: tableColumns,
    rows: tableRows,
    pathOrder: tableRows.map((row) => row[0]),
    plusOneProbabilityGridExact: pGridExact,
    noBootstrapTables: !allTreeItems.some((label) => /Bootstrap confidence|Bootstrap inference|Studentized confidence/i.test(label)),
    noPlaceholderValues: !/\bN\/?A\b|NaN|Infinity/i.test(tableRows.flat().join(" ")),
    activeLifecycle: activeCompletion,
    runDetails,
  };
  if (!evidence.checks.structuralPathRandomizationResults.passed) {
    throw new Error(`The Structural Path Randomization Results table did not match the exact candidate path order and plus-one grid: ${JSON.stringify(evidence.checks.structuralPathRandomizationResults)}`);
  }
  await capture(structuralPathRandomizationCaptureName(196, "results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  await xlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const expectedSheets = ["Structural path randomization", "Run provenance"];
  const expectedSharedStrings = [
    "Structural path randomization",
    "Run provenance",
    structuralPathRandomizationWarning,
    "Randomization method",
    structuralPathRandomizationMethodVersion,
    "Randomization operation",
    structuralPathRandomizationOperation,
    "Randomized structural paths",
    "2",
    "Requested path permutations",
    String(structuralPathRandomizationPermutations),
    "Randomization estimand",
    "Structural path coefficients conditional on fixed original PLS construct scores",
    "Pathwise probability",
    structuralPathRandomizationProbabilityDisclosure,
    "Availability",
    structuralPathRandomizationQualificationDisclosure,
  ];
  evidence.checks.structuralPathRandomizationExport = {
    passed: false,
    xlsxEnabled: await xlsxExport.isEnabled(),
    buttonCount: await exportDialog.locator(".nd-export-list button").count(),
    expectedSheets,
    expectedSharedStrings,
    nativeXlsx: null,
  };
  if (!evidence.checks.structuralPathRandomizationExport.xlsxEnabled) {
    throw new Error("The completed Structural Path Randomization result did not enable native XLSX export.");
  }
  const nativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: exportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets,
    expectedSharedStrings,
  });
  let helperCompleted = false;
  try {
    const ready = await nativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") {
      throw new Error(`Native Structural Path Randomization XLSX helper did not become ready: ${JSON.stringify(ready)}`);
    }
    await xlsxExport.click();
    const completion = await nativeSaveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) {
      throw new Error(`Native Structural Path Randomization XLSX save failed: ${JSON.stringify(completion)}`);
    }
    const feedbackText = `Saved ${path.basename(exportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: feedbackText });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(exportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(exportTargetPath);
    const exactRequiredSheets = expectedSheets.every((name) => workbookSheets.filter((candidate) => candidate === name).length === 1);
    const noBootstrapSheets = !workbookSheets.some((name) => /bootstrap|studentized/i.test(name));
    evidence.checks.structuralPathRandomizationExport.nativeXlsx = {
      attempted: true,
      targetPath: exportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
      exactRequiredSheets,
      noBootstrapSheets,
    };
    evidence.checks.structuralPathRandomizationExport.passed = file.isFile() && file.size > 0
      && evidence.checks.structuralPathRandomizationExport.nativeXlsx.appFeedback === feedbackText
      && exactRequiredSheets && noBootstrapSheets;
    if (!evidence.checks.structuralPathRandomizationExport.passed) {
      throw new Error(`The native Structural Path Randomization XLSX did not preserve its candidate table and provenance contract: ${JSON.stringify(evidence.checks.structuralPathRandomizationExport)}`);
    }
  } finally {
    if (!helperCompleted) nativeSaveHelper.stop();
  }
  await capture(structuralPathRandomizationCaptureName(197, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const archive = await inspectSavedStructuralPathRandomizationArchive(structuralPathRandomizationProjectPath, runId);
  evidence.checks.structuralPathRandomizationArchive = {
    passed: archive.passed === true,
    ...archive,
  };
  await reloadToLauncher();
  await openRecentProject(structuralPathRandomizationProjectName, structuralPathRandomizationProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOptions = page.locator(".nd-run-select select option").filter({ hasText: /Structural Path Randomization/i });
  await reopenedOptions.first().waitFor({ state: "attached", timeout: 30_000 });
  if (await reopenedOptions.count() !== 1) {
    throw new Error(`The reopened archive exposed ${await reopenedOptions.count()} Structural Path Randomization runs instead of one.`);
  }
  const reopenedRunId = await reopenedOptions.first().getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened Structural Path Randomization option had no run identifier.");
  const reopenedSelect = page.locator(".nd-run-select select");
  if (await reopenedSelect.inputValue() !== reopenedRunId) await reopenedSelect.selectOption(reopenedRunId);
  const reopenedInitialTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]')
    .getAttribute("data-result-tree-item-id");
  const reopenedInference = page.getByRole("treeitem", { name: "Inference", exact: true });
  if (await reopenedInference.getAttribute("aria-expanded") === "false") await reopenedInference.click();
  const reopenedRows = await openResultTable("Structural path randomization");
  const reopenedColumns = (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText);
  const reopenedValues = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => (
    Array.from(row.querySelectorAll("th, td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
  )));
  const reopenedWarning = compactVisibleText(await page.locator(".nd-inline-warning").textContent());
  evidence.checks.structuralPathRandomizationSaveReopen = {
    passed: reopenedRunId === runId && await reopenedSelect.inputValue() === runId
      && reopenedInitialTable === "model_estimates" && reopenedRows === 2
      && JSON.stringify(reopenedColumns) === JSON.stringify(tableColumns)
      && JSON.stringify(reopenedValues) === JSON.stringify(tableRows)
      && reopenedWarning === structuralPathRandomizationWarning,
    expectedRunId: runId,
    reopenedRunId,
    selectedRunId: await reopenedSelect.inputValue(),
    sameRunRestored: reopenedRunId === runId && await reopenedSelect.inputValue() === runId,
    initialSelectedTable: reopenedInitialTable,
    rows: reopenedRows,
    columns: reopenedColumns,
    values: reopenedValues,
    warning: reopenedWarning,
    archiveChecksumMatches: archive.manifest.projectChecksumMatches,
  };
  if (!evidence.checks.structuralPathRandomizationSaveReopen.passed) {
    throw new Error(`The exact Structural Path Randomization run did not survive explicit save and same-run reopen: ${JSON.stringify(evidence.checks.structuralPathRandomizationSaveReopen)}`);
  }
  await capture(structuralPathRandomizationCaptureName(198, "reopened"));
}

const cbsemExactBootstrapBaseTableIds = [
  "exact_case_bootstrap_summary",
  "exact_case_bootstrap_parameter_intervals",
  "exact_case_bootstrap_successful_refits",
  "exact_case_bootstrap_failures",
  "exact_case_bootstrap_hypothesis_tests",
];
const cbsemExactBootstrapStudentizedTableIds = [
  "exact_case_bootstrap_studentized_summary",
  "exact_case_bootstrap_studentized_point_standard_errors",
  "exact_case_bootstrap_studentized_parameter_intervals",
  "exact_case_bootstrap_studentized_refit_standard_errors",
];
const cbsemExactBootstrapBcaTableIds = [
  "exact_case_bootstrap_bca_summary",
  "exact_case_bootstrap_bca_parameter_intervals",
  "exact_case_bootstrap_bca_successful_delete_one_refits",
  "exact_case_bootstrap_bca_failures",
];

function exactCbsemNumeric(value) {
  return Number(String(value ?? "").replaceAll(",", ""));
}

async function inspectExactCbsemCanonicalResult(requiredTableIds) {
  const results = page.locator(".nd-cbsem-v4-results");
  await page.waitForFunction(() => {
    const result = document.querySelector(".nd-cbsem-v4-results");
    const failure = document.querySelector(".nd-cbsem-v4-failure");
    const state = document.querySelector(".nd-cbsem-v4-state")?.textContent?.trim().toLocaleLowerCase();
    return Boolean(result || failure || state === "failed" || state === "cancelled");
  }, null, { timeout: 180_000 });
  if (!await results.isVisible()) {
    const terminal = await page.evaluate(() => ({
        state: document.querySelector(".nd-cbsem-v4-state")?.textContent?.replace(/\s+/g, " ").trim() ?? "missing",
        monitor: document.querySelector(".nd-cbsem-v4-monitor")?.textContent?.replace(/\s+/g, " ").trim() ?? "missing",
        failure: document.querySelector(".nd-cbsem-v4-failure")?.textContent?.replace(/\s+/g, " ").trim() ?? "missing",
      }));
    throw new Error(`Exact-CB native job terminated without a result: ${JSON.stringify(terminal)}`);
  }
  const inspected = await results.evaluate((element) => {
    const tables = {};
    for (const wrap of element.querySelectorAll(".nd-cbsem-v4-table-wrap")) {
      const id = wrap.getAttribute("data-canonical-table-id");
      if (!id) continue;
      const table = wrap.querySelector("table");
      const headers = Array.from(table?.querySelectorAll("thead th") ?? [], (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "");
      const rows = Array.from(table?.querySelectorAll("tbody tr") ?? [], (row) => Array.from(
        row.querySelectorAll("th,td"),
        (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "",
      ));
      tables[id] = {
        title: table?.querySelector("caption strong")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
        headers,
        rows,
      };
    }
    const details = Object.fromEntries(Array.from(element.querySelectorAll(".nd-cbsem-v4-run-details dl > div")).map((row) => [
      row.querySelector("dt")?.textContent?.trim() ?? "",
      row.querySelector("dd")?.textContent?.trim() ?? "",
    ]));
    return {
      title: element.querySelector("#nd-cbsem-v4-results-heading")?.textContent?.replace(/\s+/g, " ").trim() ?? "",
      reopened: element.textContent?.includes("Reopened immutable schema-6 document") ?? false,
      tableIds: Object.keys(tables),
      tables,
      details,
    };
  });
  const missing = requiredTableIds.filter((id) => !(id in inspected.tables));
  if (missing.length) throw new Error(`Exact-CB result omitted canonical tables: ${missing.join(", ")}.`);
  const summary = inspected.tables.exact_case_bootstrap_summary;
  const summaryValues = Object.fromEntries(summary.headers.map((header, index) => [header, summary.rows[0]?.[index] ?? ""]));
  return { ...inspected, summaryValues };
}

async function configureExactCbsemBootstrap(interval, { workers = cbsemExactBootstrapWorkers } = {}) {
  const enabled = page.locator("#nd-cbsem-v4-bootstrap-enabled");
  if (!await enabled.isChecked()) await enabled.check();
  await page.locator("#nd-cbsem-v4-bootstrap-samples").fill(String(cbsemExactBootstrapSamples));
  await page.locator("#nd-cbsem-v4-bootstrap-interval").selectOption(interval);
  const engineDetails = page.locator("#nd-model-cbsem-labs-panel details").filter({ hasText: /^Engine settings/i });
  if ((await engineDetails.getAttribute("open")) === null) await engineDetails.locator("summary").click();
  await engineDetails.getByLabel("Workers", { exact: true }).fill(String(workers));
  await engineDetails.getByLabel("Seed", { exact: true }).fill(String(cbsemExactBootstrapSeed));
  const setup = {
    interval: await page.locator("#nd-cbsem-v4-bootstrap-interval").inputValue(),
    samples: await page.locator("#nd-cbsem-v4-bootstrap-samples").inputValue(),
    tail: await page.locator("#nd-cbsem-v4-bootstrap-tail").inputValue(),
    tailDisabled: await page.locator("#nd-cbsem-v4-bootstrap-tail").isDisabled(),
    workers: await engineDetails.getByLabel("Workers", { exact: true }).inputValue(),
    seed: await engineDetails.getByLabel("Seed", { exact: true }).inputValue(),
    startEnabled: await page.getByRole("button", { name: "Start native job", exact: true }).isEnabled(),
  };
  if (setup.interval !== interval || setup.samples !== String(cbsemExactBootstrapSamples)
    || setup.tail !== "two_sided" || setup.workers !== String(workers)
    || setup.seed !== String(cbsemExactBootstrapSeed) || !setup.startEnabled
    || (interval !== "percentile_type7" && !setup.tailDisabled)) {
    throw new Error(`Exact-CB bootstrap controls did not retain the requested contract: ${JSON.stringify(setup)}`);
  }
  return setup;
}

async function runExactCbsemBootstrapInterval({ interval, requiredTableIds, checkName, workers = cbsemExactBootstrapWorkers }) {
  const setup = await configureExactCbsemBootstrap(interval, { workers });
  await page.getByRole("button", { name: "Start native job", exact: true }).click();
  const result = await inspectExactCbsemCanonicalResult(requiredTableIds);
  const summary = result.summaryValues;
  const requested = exactCbsemNumeric(summary.Requested);
  const attempted = exactCbsemNumeric(summary.Attempted);
  const usable = exactCbsemNumeric(summary.Usable);
  const failed = exactCbsemNumeric(summary.Failed);
  const minimumUsable = exactCbsemNumeric(summary["Minimum usable"]);
  const expectedMinimumUsable = Math.max(1_000, Math.ceil(0.9 * requested));
  const inferenceAvailable = usable >= minimumUsable;
  const parameterIntervals = result.tables.exact_case_bootstrap_parameter_intervals;
  const inferenceContractPassed = inferenceAvailable
    ? summary["Inference status"] === "available" && parameterIntervals.rows.length > 0
    : summary["Inference status"] === "unavailable"
      && summary["Unavailable reason"] === "insufficient_usable_refits"
      && parameterIntervals.rows.length === 0;
  const check = {
    passed: result.title === "CB-SEM CFA results"
      && summary["Method version"] === cbsemExactBootstrapMethodVersion
      && requested === cbsemExactBootstrapSamples && attempted === cbsemExactBootstrapSamples
      && usable + failed === attempted && usable >= Math.ceil(0.9 * requested)
      && minimumUsable === expectedMinimumUsable && inferenceContractPassed
      && result.tables.exact_case_bootstrap_successful_refits.rows.length === usable
      && result.tables.exact_case_bootstrap_failures.rows.length === failed
      && result.details.Method === "cbsem_ml_exact_parameter_table_v3"
      && Boolean(result.details.Run),
    setup,
    requested,
    attempted,
    usable,
    failed,
    minimumUsable,
    inferenceStatus: summary["Inference status"],
    unavailableReason: summary["Unavailable reason"],
    methodVersion: summary["Method version"],
    runId: result.details.Run,
    projectId: result.details.Project,
    modelId: result.details.Model,
    datasetId: result.details.Dataset,
    tableIds: result.tableIds,
    requiredTableIds,
    tables: Object.fromEntries(requiredTableIds.map((id) => [id, {
      title: result.tables[id].title,
      rows: result.tables[id].rows.length,
      headers: result.tables[id].headers,
    }])),
  };
  evidence.checks[checkName] = check;
  if (!check.passed) throw new Error(`Exact-CB ${interval} execution did not satisfy its canonical contract: ${JSON.stringify(check)}`);
  return { result, check };
}

async function runFocusedExactCbsemBootstrapExecute() {
  if (!requestedCbsemExactBootstrapExportPath) {
    throw new Error("QUICKPLS_CBSEM_EXACT_EXPORT_PATH is required for genuine same-run native XLSX acceptance.");
  }
  const exportTargetPath = await validateRequestedNativeExportPath(
    requestedCbsemExactBootstrapExportPath,
    "QUICKPLS_CBSEM_EXACT_EXPORT_PATH",
  );
  await seedRecentProject({
    name: cbsemExactBootstrapProjectName,
    path: cbsemExactBootstrapProjectPath,
    openedAt: "2026-08-18T00:00:00.000Z",
  });
  await reloadToLauncher();
  await openRecentProject(cbsemExactBootstrapProjectName, cbsemExactBootstrapProjectPath);
  await waitForSurface("data");
  const fixtureStatus = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const fixtureColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  evidence.checks.cbsemExactBootstrapInitialModelCreation = await createInitialEditableModel(
    cbsemExactBootstrapProjectName,
    cbsemExactBootstrapModelName,
  );

  await page.locator("#nd-model-cbsem-labs-tab").click();
  const invalidStart = page.getByRole("button", { name: "Start native job", exact: true });
  const invalidIssues = (await page.locator("#nd-cbsem-v4-preflight .blocked p").allTextContents()).map(compactVisibleText);
  evidence.checks.cbsemExactBootstrapInvalidSetup = {
    passed: await invalidStart.isDisabled()
      && invalidIssues.length > 0
      && await page.locator(".nd-cbsem-v4-results").count() === 0,
    startDisabled: await invalidStart.isDisabled(),
    issues: invalidIssues,
    noResult: await page.locator(".nd-cbsem-v4-results").count() === 0,
  };
  if (!evidence.checks.cbsemExactBootstrapInvalidSetup.passed) {
    throw new Error(`Empty exact-CB setup was not blocked: ${JSON.stringify(evidence.checks.cbsemExactBootstrapInvalidSetup)}`);
  }
  await capture("180-tauri-native-cbsem-exact-invalid-setup.png");

  await page.locator("#nd-model-canvas-tab").click();
  const authored = await buildOneFactorExactCbsemModel();
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const schema6 = await createExactCbsemSchema6Copy(cbsemExactBootstrapProjectPath, cbsemExactBootstrapSchema6Path);
  await page.locator("#nd-model-cbsem-labs-tab").click();
  const readySetup = await configureExactCbsemBootstrap("percentile_type7", { workers: 1 });
  evidence.checks.cbsemExactBootstrapSetup = {
    passed: fixtureStatus.includes("140 cases")
      && ["g1", "g2", "g3"].every((column) => fixtureColumns.includes(column))
      && authored.constructs === 1 && authored.structuralPaths === 0
      && authored.scientific.representation === "common_factor"
      && schema6.destinationInspection.value.schemaVersion === 6
      && readySetup.startEnabled,
    fixture: { path: cbsemExactBootstrapFixtureCsvPath, status: fixtureStatus, columns: fixtureColumns },
    authored,
    schema6,
    controls: readySetup,
  };
  if (!evidence.checks.cbsemExactBootstrapSetup.passed) {
    throw new Error(`Exact-CB setup did not reach a ready bounded CFA: ${JSON.stringify(evidence.checks.cbsemExactBootstrapSetup)}`);
  }

  const settingsBeforeCancel = await configureExactCbsemBootstrap("percentile_type7", { workers: 1 });
  await page.getByRole("button", { name: "Start native job", exact: true }).click();
  const runningState = page.locator(".nd-cbsem-v4-state.running");
  await runningState.waitFor({ state: "visible", timeout: 30_000 });
  await page.waitForFunction(() => {
    const progress = document.querySelector('progress[aria-label="CB-SEM Recipe-v4 job progress"]');
    return Number(progress?.getAttribute("value") ?? 0) > 0;
  }, null, { timeout: 60_000 });
  const cancelledProgress = await page.locator('progress[aria-label="CB-SEM Recipe-v4 job progress"]').getAttribute("value");
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await page.locator(".nd-cbsem-v4-state.cancelled").waitFor({ state: "visible", timeout: 60_000 });
  const noCancelledResult = await page.locator(".nd-cbsem-v4-results").count() === 0;
  const appendDisabledAfterCancel = await page.getByRole("button", { name: "Append exact native document", exact: true }).isDisabled();
  const clear = page.getByRole("button", { name: "Clear", exact: true });
  await clear.click();
  const retainedSettings = {
    interval: await page.locator("#nd-cbsem-v4-bootstrap-interval").inputValue(),
    samples: await page.locator("#nd-cbsem-v4-bootstrap-samples").inputValue(),
    seed: await page.getByLabel("Seed", { exact: true }).inputValue(),
    workers: await page.getByLabel("Workers", { exact: true }).inputValue(),
  };
  const percentile = await runExactCbsemBootstrapInterval({
    interval: "percentile_type7",
    requiredTableIds: cbsemExactBootstrapBaseTableIds,
    checkName: "cbsemExactBootstrapPercentile",
    workers: 1,
  });
  evidence.checks.cbsemExactBootstrapCancellationRetry = {
    passed: Number(cancelledProgress) > 0 && noCancelledResult && appendDisabledAfterCancel
      && retainedSettings.interval === settingsBeforeCancel.interval
      && retainedSettings.samples === settingsBeforeCancel.samples
      && retainedSettings.seed === settingsBeforeCancel.seed
      && retainedSettings.workers === settingsBeforeCancel.workers
      && percentile.check.runId.length > 0,
    progressAtCancellation: Number(cancelledProgress),
    noPartialResult: noCancelledResult,
    appendDisabledAfterCancel,
    settingsBeforeCancel,
    retainedSettings,
    completedRetryRunId: percentile.check.runId,
  };
  if (!evidence.checks.cbsemExactBootstrapCancellationRetry.passed) {
    throw new Error(`Exact-CB cancellation/retry linkage failed: ${JSON.stringify(evidence.checks.cbsemExactBootstrapCancellationRetry)}`);
  }
  await capture("181-tauri-native-cbsem-exact-percentile-retry.png");

  await runExactCbsemBootstrapInterval({
    interval: "analytic_studentized_type7",
    requiredTableIds: [...cbsemExactBootstrapBaseTableIds, ...cbsemExactBootstrapStudentizedTableIds],
    checkName: "cbsemExactBootstrapStudentized",
  });
  await capture("182-tauri-native-cbsem-exact-studentized.png");
  const bca = await runExactCbsemBootstrapInterval({
    interval: "bca_type7",
    requiredTableIds: [...cbsemExactBootstrapBaseTableIds, ...cbsemExactBootstrapBcaTableIds],
    checkName: "cbsemExactBootstrapBca",
  });
  await capture("183-tauri-native-cbsem-exact-bca.png");

  const archiveInput = page.getByLabel("Schema-6 archive path", { exact: true });
  await archiveInput.fill(cbsemExactBootstrapSchema6Path);
  await page.getByRole("button", { name: "Inspect", exact: true }).click();
  await page.locator(".nd-cbsem-v4-receipt").waitFor({ state: "visible", timeout: 30_000 });
  const append = page.getByRole("button", { name: "Append exact native document", exact: true });
  await append.waitFor({ state: "visible", timeout: 10_000 });
  if (!await append.isEnabled()) throw new Error("Exact-CB schema-6 append did not become enabled after inspection.");
  await append.click();
  const appendSuccess = page.locator(".nd-cbsem-v4-success").filter({ hasText: /^Appended document/i });
  await appendSuccess.waitFor({ state: "visible", timeout: 120_000 });
  const appendText = compactVisibleText(await appendSuccess.textContent());
  const documentId = /Appended document ([^;]+);/.exec(appendText)?.[1] ?? "";
  await page.getByRole("button", { name: "Reopen and verify completed run", exact: true }).click();
  await page.locator(".nd-cbsem-v4-success").filter({ hasText: /Reopened immutable document/i }).waitFor({ state: "visible", timeout: 120_000 });
  const reopened = await inspectExactCbsemCanonicalResult([...cbsemExactBootstrapBaseTableIds, ...cbsemExactBootstrapBcaTableIds]);
  const storedDocumentId = await page.locator("#nd-cbsem-v4-stored-results").inputValue();
  const runId = reopened.details.Run;
  evidence.checks.cbsemExactBootstrapResultIdentity = {
    passed: Boolean(documentId) && storedDocumentId === documentId && runId === bca.check.runId
      && reopened.reopened && reopened.summaryValues["Method version"] === cbsemExactBootstrapMethodVersion,
    documentId,
    storedDocumentId,
    runId,
    expectedRunId: bca.check.runId,
    projectId: reopened.details.Project,
    methodVersion: reopened.summaryValues["Method version"],
    estimatorMethodVersion: reopened.details.Method,
    appendText,
  };
  if (!evidence.checks.cbsemExactBootstrapResultIdentity.passed) {
    throw new Error(`Exact-CB immutable result identity did not survive append/reopen: ${JSON.stringify(evidence.checks.cbsemExactBootstrapResultIdentity)}`);
  }

  const expectedSheets = [
    "Exact case-bootstrap summary",
    "BCa bootstrap summary",
    "BCa parameter intervals",
    "Successful delete-one refits",
    "Failed delete-one refits",
    "Canonical run provenance",
    "Canonical result notes",
  ];
  const saveHelper = startWindowsNativeSaveExportHelper({
    targetPath: exportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets,
    expectedSharedStrings: [
      "Exact case-bootstrap summary", "BCa bootstrap summary", "BCa parameter intervals",
      "Successful delete-one refits", "Canonical run provenance", cbsemExactBootstrapMethodVersion,
      documentId, runId,
    ],
  });
  let saveCompleted = false;
  try {
    const ready = await saveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Exact-CB XLSX helper did not become ready: ${JSON.stringify(ready)}`);
    await page.locator("#nd-cbsem-v4-export-xlsx").click();
    const completion = await saveHelper.completed;
    saveCompleted = true;
    if (!completion.passed) throw new Error(`Exact-CB XLSX helper failed: ${JSON.stringify(completion)}`);
    const file = await fs.stat(exportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(exportTargetPath);
    const feedback = compactVisibleText(await page.locator(".nd-cbsem-v4-export-actions [role='status']").textContent());
    evidence.checks.cbsemExactBootstrapXlsx = {
      passed: file.isFile() && file.size > 0 && expectedSheets.every((sheet) => workbookSheets.includes(sheet))
        && feedback === `Saved ${exportTargetPath}.` && reopened.details.Run === runId,
      selectedRunId: reopened.details.Run,
      expectedRunId: runId,
      targetPath: exportTargetPath,
      workbookSheets,
      expectedSheets,
      feedback,
      file: { size: file.size, isFile: file.isFile() },
      helper: { ready, completion },
    };
  } finally {
    if (!saveCompleted) saveHelper.stop();
  }
  if (!evidence.checks.cbsemExactBootstrapXlsx.passed) {
    throw new Error(`Exact-CB XLSX was not bound to the selected immutable run: ${JSON.stringify(evidence.checks.cbsemExactBootstrapXlsx)}`);
  }
  await capture("184-tauri-native-cbsem-exact-appended-exported.png");

  const checkpoint = {
    schema_version: 1,
    projectPath: cbsemExactBootstrapProjectPath,
    projectName: cbsemExactBootstrapProjectName,
    modelName: cbsemExactBootstrapModelName,
    schema6Path: cbsemExactBootstrapSchema6Path,
    documentId,
    runId,
    projectId: reopened.details.Project,
    methodVersion: reopened.summaryValues["Method version"],
    tableIds: reopened.tableIds,
    archive: await artifactDigest(cbsemExactBootstrapSchema6Path),
    export: await artifactDigest(exportTargetPath),
  };
  await writeEvidenceFile(cbsemExactBootstrapCheckpointPath, JSON.stringify(checkpoint, null, 2) + "\n");
  evidence.checks.cbsemExactBootstrapCheckpoint = checkpoint;
}

async function runFocusedExactCbsemBootstrapReopen() {
  const checkpoint = JSON.parse(await fs.readFile(cbsemExactBootstrapCheckpointPath, "utf8"));
  if (checkpoint.projectPath !== cbsemExactBootstrapProjectPath || checkpoint.schema6Path !== cbsemExactBootstrapSchema6Path
    || checkpoint.methodVersion !== cbsemExactBootstrapMethodVersion) {
    throw new Error(`Exact-CB reopen checkpoint identity drifted: ${JSON.stringify(checkpoint)}`);
  }
  await seedRecentProject({ name: checkpoint.projectName, path: checkpoint.projectPath, openedAt: new Date().toISOString() });
  await reloadToLauncher();
  await openRecentProject(checkpoint.projectName, checkpoint.projectPath);
  const currentSurface = await page.locator(".nd-app").getAttribute("data-surface");
  if (currentSurface !== "model") {
    await openMenuItem("View", "Edit Model");
    await waitForSurface("model");
  }
  await page.locator("#nd-model-cbsem-labs-tab").click();
  await page.getByLabel("Schema-6 archive path", { exact: true }).fill(checkpoint.schema6Path);
  await page.getByRole("button", { name: "Inspect", exact: true }).click();
  await page.locator("#nd-cbsem-v4-stored-results").waitFor({ state: "visible", timeout: 60_000 });
  const reopened = await inspectExactCbsemCanonicalResult([...cbsemExactBootstrapBaseTableIds, ...cbsemExactBootstrapBcaTableIds]);
  const selectedDocumentId = await page.locator("#nd-cbsem-v4-stored-results").inputValue();
  const archive = await artifactDigest(checkpoint.schema6Path);
  evidence.checks.cbsemExactBootstrapSaveReopen = {
    passed: selectedDocumentId === checkpoint.documentId && reopened.details.Run === checkpoint.runId
      && reopened.details.Project === checkpoint.projectId && reopened.reopened
      && reopened.summaryValues["Method version"] === checkpoint.methodVersion
      && archive?.sha256 === checkpoint.archive?.sha256,
    distinctDesktopProcessRequired: true,
    selectedDocumentId,
    expectedDocumentId: checkpoint.documentId,
    selectedRunId: reopened.details.Run,
    expectedRunId: checkpoint.runId,
    projectId: reopened.details.Project,
    methodVersion: reopened.summaryValues["Method version"],
    archiveBeforeClose: checkpoint.archive,
    archiveAfterRelaunch: archive,
    tableIds: reopened.tableIds,
  };
  if (!evidence.checks.cbsemExactBootstrapSaveReopen.passed) {
    throw new Error(`Exact-CB run did not survive a fresh desktop process: ${JSON.stringify(evidence.checks.cbsemExactBootstrapSaveReopen)}`);
  }
  await capture("185-tauri-native-cbsem-exact-fresh-process-reopen.png");
  await captureActualTauriViewportMatrix({
    checkName: "cbsemExactBootstrapViewports",
    methodSlug: cbsemExactBootstrapMethodVersion,
    methodVersion: cbsemExactBootstrapMethodVersion,
    methodEvidenceCheck: "cbsemExactBootstrapResultIdentity",
    expectedRunId: checkpoint.runId,
    expectedRunLabel: "CB-SEM CFA results",
    expectedTableId: "exact_case_bootstrap_bca_summary",
    capturePrefix: "cbsem-exact",
    captureSequence: "185",
    exactWorkspace: true,
  });
  const internalOrigins = new Set([packagedTauriOrigin, packagedTauriIpcOrigin]);
  const externalRequests = observedBrowserRequests.filter((request) => request.origin
    && request.origin !== "null" && !internalOrigins.has(request.origin));
  evidence.checks.cbsemExactBootstrapOffline = {
    passed: externalRequests.length === 0,
    analyticalWorkflowRequiresInternet: false,
    observedRequestCount: observedBrowserRequests.length,
    externalRequestCount: externalRequests.length,
    origins: [...new Set(observedBrowserRequests.map((request) => request.origin))].sort(),
    externalRequests,
  };
  if (!evidence.checks.cbsemExactBootstrapOffline.passed) {
    throw new Error(`Exact-CB packaged workflow crossed its functional-offline boundary: ${JSON.stringify(evidence.checks.cbsemExactBootstrapOffline)}`);
  }
}

async function runFocusedExactCbsemBootstrapAcceptance() {
  if (cbsemExactBootstrapPhase === "execute") await runFocusedExactCbsemBootstrapExecute();
  else await runFocusedExactCbsemBootstrapReopen();
}

async function runFocusedCbsemAcceptance() {
  if (!requestedCbsemNativeExportPath) {
    throw new Error("QUICKPLS_CBSEM_NATIVE_EXPORT_PATH is required for focused packaged CB-SEM acceptance; enabled-button assertions do not replace a genuine native XLSX Save and workbook-content check.");
  }
  const exportTargetPath = await validateRequestedNativeExportPath(
    requestedCbsemNativeExportPath,
    "QUICKPLS_CBSEM_NATIVE_EXPORT_PATH",
  );
  await seedRecentProject({ name: cbsemProjectName, path: cbsemProjectPath, openedAt: "2026-08-12T00:00:00.000Z" });
  await reloadToLauncher();
  await openRecentProject(cbsemProjectName, cbsemProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const status = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const columns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  const initialArchive = await inspectInitialCbsemArchive(cbsemProjectPath);
  evidence.checks.cbsemFixture = {
    projectPath: cbsemProjectPath,
    sourceCsv: cbsemFixtureCsvPath,
    status,
    cases: status.includes("240 cases") ? 240 : null,
    columns,
    initialArchive,
  };
  const expectedColumns = ["#", "x1", "x2", "x3", "m1", "m2", "m3", "y1", "y2", "y3"];
  if (evidence.checks.cbsemFixture.cases !== 240 || JSON.stringify(columns) !== JSON.stringify(expectedColumns)
    || initialArchive.models !== 0 || initialArchive.activeModelId !== null) {
    throw new Error(`The focused CB-SEM fixture did not expose the canonical 240-row data-only project: ${JSON.stringify(evidence.checks.cbsemFixture)}`);
  }
  await capture(cbsemCaptureName(130, "fixture-data"));

  evidence.checks.cbsemInitialModelCreation = await createInitialEditableModel(cbsemProjectName, cbsemModelName);
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const cbsemInvalidArchiveBefore = await inspectMediationArchiveRunState(cbsemProjectPath);
  const cbsemInvalidDialog = await openCalculationFromToolbar();
  const cbsemInvalidListbox = cbsemInvalidDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await cbsemInvalidDialog.locator("#nd-calculation-method-cbsem").click();
  const cbsemInvalidStart = cbsemInvalidDialog.getByRole("button", { name: "Start CB-SEM / CFA", exact: true });
  const cbsemInvalidBlockers = (await cbsemInvalidDialog.locator(".nd-blocker li").allTextContents()).map(compactVisibleText);
  evidence.checks.cbsemInvalidSetup = {
    attempted: true,
    selectedMethod: compactVisibleText(await cbsemInvalidListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    startEnabled: await cbsemInvalidStart.isEnabled(),
    blockers: cbsemInvalidBlockers,
    emptyModelBlocker: cbsemInvalidBlockers.some((row) => /requires at least one latent factor/i.test(row)),
    archiveBefore: cbsemInvalidArchiveBefore,
    archiveAfter: null,
    archiveStateUnchanged: false,
    resultCreated: false,
  };
  await capture(cbsemCaptureName("130a", "invalid-setup"));
  await cbsemInvalidDialog.getByRole("button", { name: "Close", exact: true }).click();
  await cbsemInvalidDialog.waitFor({ state: "hidden", timeout: 10_000 });
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const cbsemInvalidArchiveAfter = await inspectMediationArchiveRunState(cbsemProjectPath);
  evidence.checks.cbsemInvalidSetup.archiveAfter = cbsemInvalidArchiveAfter;
  evidence.checks.cbsemInvalidSetup.archiveStateUnchanged = JSON.stringify(cbsemInvalidArchiveAfter) === JSON.stringify(cbsemInvalidArchiveBefore);
  evidence.checks.cbsemInvalidSetup.resultCreated = cbsemInvalidArchiveAfter.resultCount > cbsemInvalidArchiveBefore.resultCount;
  if (evidence.checks.cbsemInvalidSetup.selectedMethod !== "CB-SEM / CFA"
    || evidence.checks.cbsemInvalidSetup.startEnabled
    || !evidence.checks.cbsemInvalidSetup.emptyModelBlocker
    || !evidence.checks.cbsemInvalidSetup.archiveStateUnchanged
    || evidence.checks.cbsemInvalidSetup.resultCreated) {
    throw new Error(`The empty-model packaged CB-SEM setup did not fail closed without creating calculation state: ${JSON.stringify(evidence.checks.cbsemInvalidSetup)}`);
  }
  await buildThreeConstructCbsemModel();
  evidence.checks.cbsemModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    constructLabels: (await page.locator(".react-flow__node-latent").allTextContents()).map(compactVisibleText),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
  };
  if (evidence.checks.cbsemModel.constructs !== 3 || evidence.checks.cbsemModel.assignedIndicators !== 9
    || evidence.checks.cbsemModel.structuralPaths !== 2
    || !["X", "M", "Y"].every((name) => evidence.checks.cbsemModel.constructLabels.some((label) => label.includes(name)))) {
    throw new Error(`The visible CB-SEM authoring workflow did not create X -> M -> Y with three reflective indicators per factor: ${JSON.stringify(evidence.checks.cbsemModel)}`);
  }
  await capture(cbsemCaptureName(131, "model"));
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const calculation = await openCalculationFromToolbar();
  const listbox = calculation.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const options = listbox.getByRole("option");
  await calculation.locator("#nd-calculation-method-cbsem").click();
  const modelType = calculation.locator("#nd-calculation-cbsem-model-type");
  await modelType.waitFor({ state: "visible", timeout: 10_000 });
  await modelType.selectOption("sem");
  const maximumIterations = calculation.locator("#nd-calculation-max-iterations");
  const tolerance = calculation.locator("#nd-calculation-tolerance");
  const start = calculation.getByRole("button", { name: "Start CB-SEM / CFA", exact: true });
  const blockers = await calculation.locator(".nd-blocker li").allTextContents();
  evidence.checks.cbsemDialog = {
    catalogCount: await options.count(),
    selectedMethod: compactVisibleText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    category: compactVisibleText(await calculation.locator("#nd-calculation-category-covariance").textContent()),
    modelTypeOptions: await modelType.locator("option").evaluateAll((entries) => entries.map((entry) => ({ value: entry.value, text: entry.textContent?.trim() ?? "" }))),
    modelType: await modelType.inputValue(),
    weighting: compactVisibleText(await calculation.locator(".nd-setting-note").filter({ hasText: "Weighting scheme" }).locator("strong").textContent()),
    resultData: compactVisibleText(await calculation.locator(".nd-setting-note").filter({ hasText: "Result data" }).locator("strong").textContent()),
    editableWeightingControls: await calculation.locator("#nd-calculation-weighting, #nd-calculation-preprocessing").count(),
    maximumIterations: await maximumIterations.inputValue(),
    tolerance: await tolerance.inputValue(),
    estimator: compactVisibleText(await calculation.locator("#nd-calculation-cbsem-estimator").textContent()),
    scope: compactVisibleText(await calculation.locator("#nd-calculation-cbsem-scope").textContent()),
    unsupportedControls: await calculation.locator([
      "#nd-calculation-bootstrap-samples", "#nd-calculation-permutations", "#nd-calculation-nca-permutations",
      "#nd-calculation-seed", "#nd-calculation-workers", "#nd-calculation-case-weight", "#nd-calculation-group-column",
      "#nd-calculation-cbsem-bootstrap", "#nd-calculation-cbsem-group", "#nd-calculation-cbsem-mean-structure",
    ].join(", ")).count(),
    blockers,
    startEnabled: await start.isEnabled(),
  };
  if (evidence.checks.cbsemDialog.catalogCount !== expectedOptionLabels.length || evidence.checks.cbsemDialog.selectedMethod !== "CB-SEM / CFA"
    || evidence.checks.cbsemDialog.category !== "Covariance-based SEM"
    || JSON.stringify(evidence.checks.cbsemDialog.modelTypeOptions) !== JSON.stringify([
      { value: "sem", text: "Structural equation model (paths required)" },
      { value: "cfa", text: "Confirmatory factor analysis (no paths)" },
    ])
    || evidence.checks.cbsemDialog.modelType !== "sem" || evidence.checks.cbsemDialog.weighting !== "Path weighting (fixed)"
    || evidence.checks.cbsemDialog.resultData !== "Standardized (fixed)"
    || evidence.checks.cbsemDialog.editableWeightingControls !== 0
    || !Number.isInteger(Number(evidence.checks.cbsemDialog.maximumIterations)) || Number(evidence.checks.cbsemDialog.maximumIterations) < 1
    || !Number.isFinite(Number(evidence.checks.cbsemDialog.tolerance)) || Number(evidence.checks.cbsemDialog.tolerance) <= 0
    || !/Maximum likelihood; first loading fixed to 1/i.test(evidence.checks.cbsemDialog.estimator)
    || !/Single-group reflective raw-data CFA or recursive SEM/i.test(evidence.checks.cbsemDialog.scope)
    || !/listwise-standardized indicators/i.test(evidence.checks.cbsemDialog.scope)
    || evidence.checks.cbsemDialog.unsupportedControls !== 0 || blockers.length !== 0 || !evidence.checks.cbsemDialog.startEnabled) {
    throw new Error(`The focused CB-SEM dialog did not match the exact bounded raw/listwise/ML contract: ${JSON.stringify(evidence.checks.cbsemDialog)}`);
  }
  await capture(cbsemCaptureName(132, "dialog"));

  const activeCapture = captureActiveCalculation(calculation, cbsemCaptureName(133, "running"), "CB-SEM / CFA")
    .then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, completedBeforeCapture: true, detail: error instanceof Error ? error.message : String(error) }));
  await start.click();
  await waitForSurface("results", 180_000);
  evidence.checks.cbsemProgress = await activeCapture;
  const selectedRun = page.locator(".nd-run-select select option:checked");
  await selectedRun.waitFor({ state: "attached", timeout: 30_000 });
  const runId = await page.locator(".nd-run-select select").inputValue();
  if (!runId) throw new Error("The completed CB-SEM run had no identifier.");
  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const readTable = async (title) => {
    const rows = await openResultTable(title);
    return {
      rows,
      headers: (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText),
      values: await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => Array.from(
        row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "",
      ))),
    };
  };
  const fit = await readTable("Model fit");
  const standardized = await readTable("Standardized parameters");
  const unstandardized = await readTable("Unstandardized parameters");
  const residualCorrelations = await readTable("Residual correlations");
  const residualCovariances = await readTable("Residual covariances");
  const impliedCovariances = await readTable("Model-implied covariances");
  const modificationDiagnostics = await readTable("Residual-based modification diagnostics");
  const scope = await readTable("Calculation scope");
  const fitValues = Object.fromEntries(fit.values.map((row) => [row[0], row[1]]));
  const scopeValues = Object.fromEntries(scope.values.map((row) => [row[0], row[1]]));
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const resultsText = [fit, standardized, unstandardized, residualCorrelations, residualCovariances, impliedCovariances, modificationDiagnostics, scope]
    .flatMap((table) => [table.headers, ...table.values]).flat().join(" ");
  const runDetails = await inspectCurrentRunDetails();
  const resultProperties = await page.locator(".nd-properties .nd-property-list").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  const modelEstimateItem = page.locator('.nd-result-tree [role="treeitem"]').filter({ hasText: /^Model estimates$/ });
  await modelEstimateItem.click();
  await page.getByRole("heading", { name: "Standardized model estimates", exact: true }).waitFor({ state: "visible", timeout: 10_000 });
  const diagram = {
    heading: compactVisibleText(await page.getByRole("heading", { name: "Standardized model estimates", exact: true }).textContent()),
    imageAlt: await page.locator(".nd-result-diagram-canvas img").getAttribute("alt"),
  };
  evidence.checks.cbsemResult = {
    runId,
    runLabel: compactVisibleText(await selectedRun.textContent()),
    initialSelectedTable,
    treeItems,
    fit,
    standardized,
    unstandardized,
    residualCorrelations,
    residualCovariances,
    impliedCovariances,
    modificationDiagnostics,
    scope,
    fitValues,
    scopeValues,
    runDetails,
    resultProperties,
    diagram,
    noPlaceholder: !/\bN\/?A\b/i.test(resultsText),
    noGenericPlsTables: !/Path coefficients|Outer loadings|Construct reliability|Mediation|Prediction/i.test(treeItems.join(" ")),
  };
  const expectedTree = [
    "Graphical results", "Model estimates", "CB-SEM / CFA", "Model fit", "Standardized parameters", "Unstandardized parameters",
    "Residual correlations", "Residual covariances", "Model-implied covariances", "Residual-based modification diagnostics", "Calculation scope",
  ];
  if (initialSelectedTable !== "cbsem_fit" || JSON.stringify(treeItems) !== JSON.stringify(expectedTree)
    || fit.rows !== 13 || standardized.rows !== 23 || unstandardized.rows !== 23
    || residualCorrelations.rows !== 45 || residualCovariances.rows !== 45 || impliedCovariances.rows !== 45
    || modificationDiagnostics.rows !== 50 || scope.rows !== 15
    || JSON.stringify(fit.headers) !== JSON.stringify(["Fit measure", "Value"])
    || JSON.stringify(standardized.headers) !== JSON.stringify(["Parameter", "Type", "Std. LV", "Std. all"])
    || JSON.stringify(unstandardized.headers) !== JSON.stringify(["Parameter", "Type", "Estimate", "SE", "z", "p (two-sided)", "Status"])
    || !["CFI", "TLI", "RMSEA", "SRMR", "AIC", "BIC"].every((key) => Object.hasOwn(fitValues, key) && fitValues[key] !== "")
    || !standardized.values.some((row) => row[1] === "Structural path") || !standardized.values.some((row) => row[1] === "Loading")
    || !unstandardized.values.some((row) => row[6] === "Fixed for marker identification")
    || !unstandardized.values.some((row) => row[6] === "Estimated")
    || scopeValues["Model type"] !== "Recursive structural equation model" || scopeValues.Estimator !== "Maximum likelihood"
    || scopeValues.Input !== "Raw case-level data; indicators standardized after listwise filtering"
    || scopeValues.Identification !== "First loading fixed to 1 for each latent factor"
    || scopeValues["Mean structure"] !== "Not estimated" || scopeValues["Analyzed observations"] !== "240"
    || scopeValues.Converged !== "Yes" || scopeValues["Estimator method version"] !== cbsemMethodVersion
    || scopeValues["Fit method version"] !== cbsemFitMethodVersion
    || scopeValues["Modification-diagnostic version"] !== cbsemModificationMethodVersion
    || scopeValues["CB-SEM bootstrap"] !== "Not requested"
    || scopeValues["Unsupported in this workflow"] !== "Multigroup/invariance, robust/ordinal/FIML estimators, interactions, higher-order constructs, and mean structures"
    || runDetails.properties.Method !== "CB-SEM / CFA" || runDetails.properties["Method version"] !== cbsemProvenanceMethodVersion
    || runDetails.properties["Model type"] !== "Recursive structural equation model"
    || runDetails.properties.Estimator !== "Maximum likelihood" || runDetails.properties["Complete cases"] !== "240"
    || runDetails.properties.Converged !== "Yes" || runDetails.properties.Input !== "Raw case-level data"
    || runDetails.properties["Missing data"] !== "Listwise deletion"
    || Object.hasOwn(runDetails.properties, "Weighting") || Object.hasOwn(runDetails.properties, "Preprocessing")
    || resultProperties.Method !== "CB-SEM / CFA" || resultProperties["Model type"] !== "Recursive structural equation model"
    || resultProperties.Estimator !== "Maximum likelihood" || resultProperties["Complete cases"] !== "240"
    || diagram.heading !== "Standardized model estimates" || !/Standardized model estimates/i.test(diagram.imageAlt ?? "")
    || !evidence.checks.cbsemResult.noPlaceholder || !evidence.checks.cbsemResult.noGenericPlsTables) {
    throw new Error(`The completed CB-SEM result did not expose the exact ML fit, parameter, residual, diagnostic, scope, and standardized-diagram contract: ${JSON.stringify(evidence.checks.cbsemResult)}`);
  }
  await openResultTable("Standardized parameters");
  await capture(cbsemCaptureName(134, "results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  await xlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const exportButtonTexts = await exportDialog.locator(".nd-export-list button").evaluateAll((buttons) => buttons.map((button) => (
    button.innerText.replace(/\s+/g, " ").trim()
  )));
  const expectedFormats = ["CSV tables", "HTML report", "Reviewer pack", "XLSX workbook", "Model diagram", "Print / PDF"];
  const tableTitles = [
    "Model fit", "Standardized parameters", "Unstandardized parameters", "Residual correlations", "Residual covariances",
    "Model-implied covariances", "Residual-based modification diagnostics", "Calculation scope",
  ];
  const expectedSheets = [
    "Model fit", "Standardized parameters", "Unstandardized parameters", "Residual correlations", "Residual covariances",
    "Model-implied covariances", "Residual-based modification dia", "Calculation scope", "Run provenance",
  ];
  evidence.checks.cbsemExport = {
    formats: expectedFormats,
    buttonTexts: exportButtonTexts,
    buttonCount: await exportDialog.locator(".nd-export-list button").count(),
    everyFormatPresentOnce: expectedFormats.every((label) => exportButtonTexts.filter((text) => text.startsWith(label)).length === 1),
    xlsxEnabled: await xlsxExport.isEnabled(),
    nativeXlsx: null,
  };
  if (evidence.checks.cbsemExport.buttonCount !== 6 || !evidence.checks.cbsemExport.everyFormatPresentOnce
    || !evidence.checks.cbsemExport.xlsxEnabled) {
    throw new Error(`The completed CB-SEM result did not expose exactly six model-and-table export formats: ${JSON.stringify(evidence.checks.cbsemExport)}`);
  }
  const nativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: exportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets,
    expectedSharedStrings: [...tableTitles, "Run provenance", "Maximum likelihood", cbsemMethodVersion, cbsemFitMethodVersion],
  });
  let helperCompleted = false;
  try {
    const ready = await nativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native CB-SEM XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await nativeSaveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`Native CB-SEM XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(exportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(exportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(exportTargetPath);
    evidence.checks.cbsemExport.nativeXlsx = {
      attempted: true,
      targetPath: exportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    if (!file.isFile() || file.size <= 0 || evidence.checks.cbsemExport.nativeXlsx.appFeedback !== expectedFeedback
      || JSON.stringify(workbookSheets) !== JSON.stringify(expectedSheets)) {
      throw new Error(`The genuine CB-SEM XLSX did not contain every result and provenance sheet exactly once: ${JSON.stringify(evidence.checks.cbsemExport)}`);
    }
  } finally {
    if (!helperCompleted) nativeSaveHelper.stop();
  }
  await capture(cbsemCaptureName(135, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedArchive = await inspectSavedCbsemArchive(cbsemProjectPath, runId);
  await reloadToLauncher();
  await openRecentProject(cbsemProjectName, cbsemProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /CB-SEM \/ CFA/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened CB-SEM result had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedFitRows = await openResultTable("Model fit");
  const reopenedStandardizedRows = await openResultTable("Standardized parameters");
  const reopenedModificationRows = await openResultTable("Residual-based modification diagnostics");
  const reopenedScopeRows = await openResultTable("Calculation scope");
  evidence.checks.cbsemSaveReopen = {
    expectedRunId: runId,
    selectedRunId: reopenedRunId,
    sameRunRestored: reopenedRunId === runId,
    fitRows: reopenedFitRows,
    standardizedRows: reopenedStandardizedRows,
    modificationRows: reopenedModificationRows,
    scopeRows: reopenedScopeRows,
    archive: savedArchive,
  };
  if (!evidence.checks.cbsemSaveReopen.sameRunRestored || reopenedFitRows !== 13 || reopenedStandardizedRows !== 23
    || reopenedModificationRows !== 50 || reopenedScopeRows !== 15) {
    throw new Error(`The exact CB-SEM ML run did not survive explicit save/reload/reopen: ${JSON.stringify(evidence.checks.cbsemSaveReopen)}`);
  }
  await openResultTable("Standardized parameters");
  await capture(cbsemCaptureName(136, "reopened"));
  await captureActualTauriViewportMatrix({
    checkName: "cbsemPackagedViewports",
    methodSlug: "cbsem_ml_v1",
    methodVersion: cbsemMethodVersion,
    methodEvidenceCheck: "cbsemResult",
    expectedRunId: runId,
    expectedRunLabel: "CB-SEM / CFA",
    expectedTableId: "cbsem_standardized_parameters",
    capturePrefix: "cbsem",
    captureSequence: "136",
  });
}

async function runFocusedGscaAcceptance() {
  if (!requestedGscaNativeExportPath) {
    throw new Error("QUICKPLS_GSCA_NATIVE_EXPORT_PATH is required for focused packaged GSCA acceptance; enabled-button assertions do not replace a genuine native XLSX Save and workbook-content check.");
  }
  const exportTargetPath = await validateRequestedNativeExportPath(
    requestedGscaNativeExportPath,
    "QUICKPLS_GSCA_NATIVE_EXPORT_PATH",
  );
  await seedRecentProject({ name: gscaProjectName, path: gscaProjectPath, openedAt: "2026-08-12T00:00:00.000Z" });
  await reloadToLauncher();
  await openRecentProject(gscaProjectName, gscaProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const status = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const columns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  const initialArchive = await inspectInitialGscaArchive(gscaProjectPath);
  evidence.checks.gscaFixture = {
    projectPath: gscaProjectPath,
    sourceCsv: gscaFixtureCsvPath,
    status,
    cases: status.includes("140 cases") ? 140 : null,
    columns,
    initialArchive,
  };
  const expectedColumns = ["#", "x", "m", "w", "y", "z", "bin_y", "g1", "g2", "g3", "h1", "h2"];
  if (evidence.checks.gscaFixture.cases !== 140 || JSON.stringify(columns) !== JSON.stringify(expectedColumns)
    || initialArchive.models !== 0 || initialArchive.activeModelId !== null) {
    throw new Error(`The focused GSCA fixture did not expose the canonical 140-row data-only project: ${JSON.stringify(evidence.checks.gscaFixture)}`);
  }
  await capture(gscaCaptureName(140, "fixture-data"));

  evidence.checks.gscaInitialModelCreation = await createInitialEditableModel(gscaProjectName, gscaModelName);
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const gscaInvalidArchiveBefore = await inspectMediationArchiveRunState(gscaProjectPath);
  const gscaInvalidDialog = await openCalculationFromToolbar();
  const gscaInvalidListbox = gscaInvalidDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await gscaInvalidDialog.locator("#nd-calculation-method-gsca").click();
  const gscaInvalidStart = gscaInvalidDialog.getByRole("button", { name: "Start GSCA", exact: true });
  const gscaInvalidBlockers = (await gscaInvalidDialog.locator(".nd-blocker li").allTextContents()).map(compactVisibleText);
  evidence.checks.gscaInvalidSetup = {
    attempted: true,
    selectedMethod: compactVisibleText(await gscaInvalidListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    startEnabled: await gscaInvalidStart.isEnabled(),
    blockers: gscaInvalidBlockers,
    emptyModelBlocker: gscaInvalidBlockers.some((row) => /requires at least two component constructs/i.test(row)),
    archiveBefore: gscaInvalidArchiveBefore,
    archiveAfter: null,
    archiveStateUnchanged: false,
    resultCreated: false,
  };
  await capture(gscaCaptureName("140a", "invalid-setup"));
  await gscaInvalidDialog.getByRole("button", { name: "Close", exact: true }).click();
  await gscaInvalidDialog.waitFor({ state: "hidden", timeout: 10_000 });
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const gscaInvalidArchiveAfter = await inspectMediationArchiveRunState(gscaProjectPath);
  evidence.checks.gscaInvalidSetup.archiveAfter = gscaInvalidArchiveAfter;
  evidence.checks.gscaInvalidSetup.archiveStateUnchanged = JSON.stringify(gscaInvalidArchiveAfter) === JSON.stringify(gscaInvalidArchiveBefore);
  evidence.checks.gscaInvalidSetup.resultCreated = gscaInvalidArchiveAfter.resultCount > gscaInvalidArchiveBefore.resultCount;
  if (evidence.checks.gscaInvalidSetup.selectedMethod !== "GSCA"
    || evidence.checks.gscaInvalidSetup.startEnabled
    || !evidence.checks.gscaInvalidSetup.emptyModelBlocker
    || !evidence.checks.gscaInvalidSetup.archiveStateUnchanged
    || evidence.checks.gscaInvalidSetup.resultCreated) {
    throw new Error(`The empty-model packaged GSCA setup did not fail closed without creating calculation state: ${JSON.stringify(evidence.checks.gscaInvalidSetup)}`);
  }
  await buildTwoConstructGscaModel();
  const nodes = page.locator(".react-flow__node-latent");
  const formativeChecked = await nodes.nth(0).locator(".smartpls-latent-node.formative").count() === 1;
  const reflectiveChecked = await nodes.nth(1).locator(".smartpls-latent-node.reflective").count() === 1;
  evidence.checks.gscaModel = {
    constructs: await nodes.count(),
    constructLabels: (await nodes.allTextContents()).map(compactVisibleText),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    formativeChecked,
    reflectiveChecked,
  };
  if (evidence.checks.gscaModel.constructs !== 2 || evidence.checks.gscaModel.assignedIndicators !== 5
    || evidence.checks.gscaModel.structuralPaths !== 1 || !formativeChecked || !reflectiveChecked
    || !evidence.checks.gscaModel.constructLabels.some((label) => label.includes("G formative component"))
    || !evidence.checks.gscaModel.constructLabels.some((label) => label.includes("H reflective component"))) {
    throw new Error(`The visible GSCA authoring workflow did not create the exact formative G -> reflective H model: ${JSON.stringify(evidence.checks.gscaModel)}`);
  }
  await capture(gscaCaptureName(141, "model"));
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const calculation = await openCalculationFromToolbar();
  const listbox = calculation.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const options = listbox.getByRole("option");
  await calculation.locator("#nd-calculation-method-gsca").click();
  const start = calculation.getByRole("button", { name: "Start GSCA", exact: true });
  const blockers = await calculation.locator(".nd-blocker li").allTextContents();
  evidence.checks.gscaDialog = {
    catalogCount: await options.count(),
    optionLabels: (await options.locator("strong").allTextContents()).map(compactVisibleText),
    selectedMethod: compactVisibleText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    category: compactVisibleText(await calculation.locator("#nd-calculation-category-component_models").textContent()),
    weighting: compactVisibleText(await calculation.locator(".nd-setting-note").filter({ hasText: "Weighting scheme" }).locator("strong").textContent()),
    resultData: compactVisibleText(await calculation.locator(".nd-setting-note").filter({ hasText: "Result data" }).locator("strong").textContent()),
    estimator: compactVisibleText(await calculation.locator("#nd-calculation-gsca-estimator").textContent()),
    scope: compactVisibleText(await calculation.locator("#nd-calculation-gsca-scope").textContent()),
    unsupportedControls: await calculation.locator([
      "#nd-calculation-weighting", "#nd-calculation-preprocessing", "#nd-calculation-max-iterations", "#nd-calculation-tolerance",
      "#nd-calculation-bootstrap-samples", "#nd-calculation-permutations", "#nd-calculation-nca-permutations",
      "#nd-calculation-seed", "#nd-calculation-workers", "#nd-calculation-case-weight", "#nd-calculation-group-column",
    ].join(", ")).count(),
    blockers,
    startEnabled: await start.isEnabled(),
  };
  if (evidence.checks.gscaDialog.catalogCount !== evidence.checks.gscaDialog.optionLabels.length
    || new Set(evidence.checks.gscaDialog.optionLabels).size !== evidence.checks.gscaDialog.optionLabels.length
    || evidence.checks.gscaDialog.optionLabels.filter((label) => label === "GSCA").length !== 1
    || evidence.checks.gscaDialog.selectedMethod !== "GSCA" || evidence.checks.gscaDialog.category !== "Component models"
    || evidence.checks.gscaDialog.weighting !== "Path weighting (fixed)"
    || evidence.checks.gscaDialog.resultData !== "Standardized (fixed)"
    || !/Joint global least-squares alternating least squares; fixed \+1 initialization/i.test(evidence.checks.gscaDialog.estimator)
    || !/3,000 maximum iterations/i.test(evidence.checks.gscaDialog.scope)
    || !/1e-7 objective-and-weight stop criterion/i.test(evidence.checks.gscaDialog.scope)
    || !/No controls, covariance paths, interactions, higher-order constructs, case weights, multigroup analysis, GSCA bootstrapping, or other inference/i.test(evidence.checks.gscaDialog.scope)
    || evidence.checks.gscaDialog.unsupportedControls !== 0 || blockers.length !== 0 || !evidence.checks.gscaDialog.startEnabled) {
    throw new Error(`The focused GSCA dialog did not match the exact bounded ALS v2 contract: ${JSON.stringify(evidence.checks.gscaDialog)}`);
  }
  await capture(gscaCaptureName(142, "dialog"));

  const activeCapture = captureActiveCalculation(
    calculation,
    gscaCaptureName(143, "running"),
    "GSCA",
    { allowTerminalTransitionAfterCapture: true },
  )
    .then((state) => ({ captured: true, ...state }));
  await start.click();
  await waitForSurface("results", 180_000);
  evidence.checks.gscaProgress = await activeCapture;
  if (!evidence.checks.gscaProgress.captured || !evidence.checks.gscaProgress.status
    || !evidence.checks.gscaProgress.phase || !evidence.checks.gscaProgress.message) {
    throw new Error(`GSCA did not expose a genuine queued, validating, or running lifecycle state: ${JSON.stringify(evidence.checks.gscaProgress)}`);
  }

  const selectedRun = page.locator(".nd-run-select select option:checked");
  await selectedRun.waitFor({ state: "attached", timeout: 30_000 });
  const runId = await page.locator(".nd-run-select select").inputValue();
  if (!runId) throw new Error("The completed GSCA run had no identifier.");
  const gscaRunLabel = compactVisibleText(await selectedRun.textContent());
  evidence.checks.gscaProgress.completedRunProof = {
    runId,
    runLabel: gscaRunLabel,
    matched: /GSCA/i.test(gscaRunLabel),
  };
  if (!evidence.checks.gscaProgress.completedRunProof.matched) {
    throw new Error(`The focused GSCA lifecycle did not resolve to its matching completed run: ${JSON.stringify(evidence.checks.gscaProgress)}`);
  }
  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const readTable = async (title) => {
    const rows = await openResultTable(title);
    return {
      rows,
      headers: (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText),
      values: await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => Array.from(
        row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "",
      ))),
    };
  };
  const fit = await readTable("Model fit and convergence");
  const paths = await readTable("Structural path coefficients");
  const rSquared = await readTable("Endogenous construct R²");
  const loadings = await readTable("Measurement loadings");
  const weights = await readTable("Component weights");
  const scope = await readTable("Analysis details");
  const fitValues = Object.fromEntries(fit.values.map((row) => [row[0], row[1]]));
  const scopeValues = Object.fromEntries(scope.values.map((row) => [row[0], row[1]]));
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const resultsText = [fit, paths, rSquared, loadings, weights, scope].flatMap((table) => [table.headers, ...table.values]).flat().join(" ");
  const runDetails = await inspectCurrentRunDetails();
  const resultProperties = await page.locator(".nd-properties .nd-property-list").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  await page.getByRole("treeitem", { name: "Model estimates", exact: true }).click();
  await page.getByRole("heading", { name: "Model estimates", exact: true }).waitFor({ state: "visible", timeout: 10_000 });
  const diagram = {
    heading: compactVisibleText(await page.getByRole("heading", { name: "Model estimates", exact: true }).textContent()),
    imageAlt: await page.locator(".nd-result-diagram-canvas img").getAttribute("alt"),
  };
  evidence.checks.gscaResult = {
    runId,
    runLabel: gscaRunLabel,
    initialSelectedTable,
    treeItems,
    fit,
    paths,
    rSquared,
    loadings,
    weights,
    scope,
    fitValues,
    scopeValues,
    runDetails,
    resultProperties,
    diagram,
    noPlaceholder: !/\bN\/?A\b/i.test(resultsText),
    noGenericPlsOrInference: !/Final results|Quality criteria|Assessment|Inference|Bootstrap|Permutation/i.test(treeItems.join(" ")),
  };
  const expectedTree = [
    "Graphical results", "Model estimates", "GSCA component model", "Model fit and convergence", "Structural path coefficients",
    "Endogenous construct R²", "Measurement loadings", "Component weights", "Analysis details",
  ];
  if (initialSelectedTable !== "gsca_fit" || JSON.stringify(treeItems) !== JSON.stringify(expectedTree)
    || fit.rows !== 12 || paths.rows !== 1 || rSquared.rows !== 1 || loadings.rows !== 5 || weights.rows !== 5 || scope.rows !== 11
    || JSON.stringify(fit.headers) !== JSON.stringify(["Measure", "Value"])
    || JSON.stringify(paths.headers) !== JSON.stringify(["Path", "Coefficient"])
    || JSON.stringify(loadings.headers) !== JSON.stringify(["Construct", "Indicator", "Measurement model", "Loading"])
    || JSON.stringify(weights.headers) !== JSON.stringify(["Construct", "Indicator", "Measurement model", "Weight"])
    || fitValues.Converged !== "Yes" || fitValues["ALS iterations"] !== "4"
    || !["Global FIT", "Adjusted FIT", "Measurement FIT", "Structural FIT", "GFI", "SRMR", "Objective", "Final objective-and-weight change"].every((key) => Object.hasOwn(fitValues, key) && Number.isFinite(Number(fitValues[key])))
    || paths.values[0]?.[0] !== "H reflective component ← G formative component"
    || !loadings.values.every((row) => row[2] === (row[0] === "G formative component" ? "Formative" : "Reflective"))
    || !weights.values.every((row) => row[2] === (row[0] === "G formative component" ? "Formative" : "Reflective"))
    || scopeValues.Estimator !== "Joint global least-squares alternating least squares"
    || scopeValues["Method version"] !== gscaMethodVersion || scopeValues["Algorithm version"] !== gscaAlgorithmVersion
    || scopeValues.Inference !== "Point estimates only; no bootstrap or permutation inference"
    || runDetails.properties.Method !== "GSCA" || runDetails.properties["Method version"] !== gscaMethodVersion
    || runDetails.properties.Estimator !== "Joint global least-squares ALS" || runDetails.properties["Complete cases"] !== "140"
    || runDetails.properties.Converged !== "Yes" || runDetails.properties["ALS iterations"] !== "4"
    || Object.hasOwn(runDetails.properties, "Weighting") || Object.hasOwn(runDetails.properties, "Preprocessing") || Object.hasOwn(runDetails.properties, "Recorded seed")
    || resultProperties.Method !== "GSCA" || resultProperties.Estimator !== "Joint global least-squares ALS"
    || resultProperties["Complete cases"] !== "140" || resultProperties.Converged !== "Yes" || resultProperties["ALS iterations"] !== "4"
    || diagram.heading !== "Model estimates" || !/Model estimates for GSCA run/i.test(diagram.imageAlt ?? "")
    || !evidence.checks.gscaResult.noPlaceholder || !evidence.checks.gscaResult.noGenericPlsOrInference) {
    throw new Error(`The completed GSCA result did not expose the exact ALS fit, path, measurement, scope, and diagram contract: ${JSON.stringify(evidence.checks.gscaResult)}`);
  }
  await openResultTable("Model fit and convergence");
  await capture(gscaCaptureName(144, "results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  await xlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const exportButtonTexts = await exportDialog.locator(".nd-export-list button").evaluateAll((buttons) => buttons.map((button) => button.innerText.replace(/\s+/g, " ").trim()));
  const expectedFormats = ["CSV tables", "HTML report", "Reviewer pack", "XLSX workbook", "Model diagram", "Print / PDF"];
  const tableTitles = ["Model fit and convergence", "Structural path coefficients", "Endogenous construct R²", "Measurement loadings", "Component weights", "Analysis details"];
  const expectedSheets = [...tableTitles, "Run provenance"];
  evidence.checks.gscaExport = {
    formats: expectedFormats,
    buttonTexts: exportButtonTexts,
    buttonCount: await exportDialog.locator(".nd-export-list button").count(),
    everyFormatPresentOnce: expectedFormats.every((label) => exportButtonTexts.filter((text) => text.startsWith(label)).length === 1),
    xlsxEnabled: await xlsxExport.isEnabled(),
    nativeXlsx: null,
  };
  if (evidence.checks.gscaExport.buttonCount !== 6 || !evidence.checks.gscaExport.everyFormatPresentOnce || !evidence.checks.gscaExport.xlsxEnabled) {
    throw new Error(`The completed GSCA result did not expose exactly six model-and-table export formats: ${JSON.stringify(evidence.checks.gscaExport)}`);
  }
  const nativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: exportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets,
    expectedSharedStrings: [...tableTitles, "Run provenance", "Joint global least-squares alternating least squares", gscaMethodVersion, gscaAlgorithmVersion],
  });
  let helperCompleted = false;
  try {
    const ready = await nativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native GSCA XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await nativeSaveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`Native GSCA XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(exportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(exportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(exportTargetPath);
    evidence.checks.gscaExport.nativeXlsx = {
      attempted: true,
      targetPath: exportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    if (!file.isFile() || file.size <= 0 || evidence.checks.gscaExport.nativeXlsx.appFeedback !== expectedFeedback
      || JSON.stringify(workbookSheets) !== JSON.stringify(expectedSheets)) {
      throw new Error(`The genuine GSCA XLSX did not contain every result and provenance sheet exactly once: ${JSON.stringify(evidence.checks.gscaExport)}`);
    }
  } finally {
    if (!helperCompleted) nativeSaveHelper.stop();
  }
  await capture(gscaCaptureName(145, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedArchive = await inspectSavedGscaArchive(gscaProjectPath, runId);
  await reloadToLauncher();
  await openRecentProject(gscaProjectName, gscaProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /^GSCA run$/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened GSCA result had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedFitRows = await openResultTable("Model fit and convergence");
  const reopenedLoadingRows = await openResultTable("Measurement loadings");
  const reopenedScopeRows = await openResultTable("Analysis details");
  evidence.checks.gscaSaveReopen = {
    expectedRunId: runId,
    selectedRunId: reopenedRunId,
    sameRunRestored: reopenedRunId === runId,
    fitRows: reopenedFitRows,
    loadingRows: reopenedLoadingRows,
    scopeRows: reopenedScopeRows,
    archive: savedArchive,
  };
  if (!evidence.checks.gscaSaveReopen.sameRunRestored || reopenedFitRows !== 12 || reopenedLoadingRows !== 5 || reopenedScopeRows !== 11) {
    throw new Error(`The exact GSCA ALS v2 run did not survive explicit save/reload/reopen: ${JSON.stringify(evidence.checks.gscaSaveReopen)}`);
  }
  await openResultTable("Model fit and convergence");
  await capture(gscaCaptureName(146, "reopened"));
  await captureActualTauriViewportMatrix({
    checkName: "gscaPackagedViewports",
    methodSlug: "gsca_als_v2",
    methodVersion: gscaMethodVersion,
    methodEvidenceCheck: "gscaResult",
    expectedRunId: runId,
    expectedRunLabel: "GSCA run",
    expectedTableId: "gsca_fit",
    capturePrefix: "gsca",
    captureSequence: "146",
  });
  const gscaInternalOrigins = new Set([packagedTauriOrigin, packagedTauriIpcOrigin]);
  const gscaExternalRequests = observedBrowserRequests.filter((request) => request.origin
    && request.origin !== "null" && !gscaInternalOrigins.has(request.origin));
  evidence.checks.gscaFunctionalOffline = {
    passed: observedBrowserRequests.length > 0 && gscaExternalRequests.length === 0,
    analyticalWorkflowRequiresInternet: false,
    strictZeroProcessEgressClaimed: false,
    platformBackgroundEgressOutsidePageRequestScope: true,
    observedRequestCount: observedBrowserRequests.length,
    externalRequestCount: gscaExternalRequests.length,
    origins: [...new Set(observedBrowserRequests.map((request) => request.origin))].sort(),
    externalRequests: gscaExternalRequests,
  };
  if (!evidence.checks.gscaFunctionalOffline.passed) {
    throw new Error(`GSCA packaged browser/app workflow crossed its functional-offline request boundary: ${JSON.stringify(evidence.checks.gscaFunctionalOffline)}`);
  }
}

async function runFocusedOlsAcceptance() {
  if (!requestedOlsNativeExportPath) {
    throw new Error("QUICKPLS_OLS_NATIVE_EXPORT_PATH is required for focused packaged OLS acceptance; enabled-button assertions do not replace a genuine native XLSX Save and workbook-content check.");
  }
  const exportTargetPath = await validateRequestedNativeExportPath(
    requestedOlsNativeExportPath,
    "QUICKPLS_OLS_NATIVE_EXPORT_PATH",
  );
  await seedRecentProject({ name: olsProjectName, path: olsProjectPath, openedAt: "2026-08-12T00:00:00.000Z" });
  await reloadToLauncher();
  await openRecentProject(olsProjectName, olsProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const status = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const columns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  const initialArchive = await inspectInitialOlsArchive(olsProjectPath);
  evidence.checks.olsFixture = {
    projectPath: olsProjectPath,
    status,
    cases: status.includes("140 cases") ? 140 : null,
    columns,
    initialArchive,
  };
  const expectedColumns = ["#", "x", "m", "w", "y", "z", "bin_y", "g1", "g2", "g3", "h1", "h2"];
  if (evidence.checks.olsFixture.cases !== 140 || JSON.stringify(columns) !== JSON.stringify(expectedColumns)
    || initialArchive.models !== 0 || initialArchive.activeModelId !== null) {
    throw new Error(`The focused OLS fixture did not expose the canonical 140-row data-only project: ${JSON.stringify(evidence.checks.olsFixture)}`);
  }
  await capture(olsCaptureName(120, "fixture-data"));

  const calculation = await openAnalysisFromDataToolbar();
  const listbox = calculation.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const options = listbox.getByRole("option");
  await calculation.locator("#nd-calculation-method-regression").click();
  const olsSettings = calculation.locator(".nd-ols-settings");
  await olsSettings.waitFor({ state: "visible", timeout: 10_000 });
  const regressionType = calculation.locator("#nd-calculation-regression-type");
  if (await regressionType.inputValue().catch(() => "") !== "ols") await regressionType.selectOption("ols");
  const outcome = calculation.locator("#nd-calculation-regression-outcome");
  await outcome.selectOption(olsOutcome);
  const roleFieldsets = olsSettings.locator("fieldset.nd-pca-variables");
  const roleLabels = (fieldset) => fieldset.locator("label");
  for (const fieldset of [roleFieldsets.nth(0), roleFieldsets.nth(1)]) {
    const checked = fieldset.locator('input[type="checkbox"]:checked');
    while (await checked.count()) await checked.first().uncheck();
  }
  for (const variable of olsPredictors) {
    const label = roleLabels(roleFieldsets.nth(0)).filter({ hasText: new RegExp(`^\\s*${variable}\\s*$`) });
    if (await label.count() !== 1) throw new Error(`OLS predictor ${variable} was not exposed as exactly one checkbox.`);
    await label.getByRole("checkbox").check();
  }
  for (const variable of olsControls) {
    const label = roleLabels(roleFieldsets.nth(1)).filter({ hasText: new RegExp(`^\\s*${variable}\\s*$`) });
    if (await label.count() !== 1) throw new Error(`OLS control ${variable} was not exposed as exactly one checkbox.`);
    await label.getByRole("checkbox").check();
  }
  const inspectRole = async (fieldset) => fieldset.locator("label").evaluateAll((labels) => labels.filter((label) => (
    label.querySelector('input[type="checkbox"]')?.checked
  )).map((label) => label.querySelector("span")?.textContent?.replace(/\s+/g, " ").trim() ?? ""));
  const selectedPredictors = await inspectRole(roleFieldsets.nth(0));
  const selectedControls = await inspectRole(roleFieldsets.nth(1));
  const noteValue = async (label) => compactVisibleText(await calculation.locator(".nd-setting-note")
    .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
  const start = calculation.getByRole("button", { name: "Start OLS regression", exact: true });
  const blockerText = compactVisibleText(await calculation.locator('.nd-blocker[role="alert"]').textContent().catch(() => ""));
  evidence.checks.olsDialog = {
    catalogCount: await options.count(),
    selectedMethod: compactVisibleText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    category: compactVisibleText(await calculation.locator("#nd-calculation-category-standalone").textContent()),
    outcome: await outcome.inputValue(),
    selectedPredictors,
    selectedControls,
    calculationBasis: await noteValue("Calculation basis"),
    variableData: await noteValue("Variable data"),
    uncertainty: await noteValue("Uncertainty"),
    validatedScope: await noteValue("Validated scope"),
    unsupportedControls: await calculation.locator([
      "#nd-calculation-weighting", "#nd-calculation-preprocessing", "#nd-calculation-max-iterations",
      "#nd-calculation-tolerance", "#nd-calculation-bootstrap-samples", "#nd-calculation-permutations",
      "#nd-calculation-seed", "#nd-calculation-workers", "#nd-calculation-case-weight",
    ].join(", ")).count(),
    blockers: await calculation.locator(".nd-blocker li").allTextContents(),
    blockerText,
    noModelBlocker: !/construct|structural path|editable model|active model/i.test(blockerText),
    startEnabled: await start.isEnabled(),
  };
  if (evidence.checks.olsDialog.catalogCount !== expectedOptionLabels.length
    || evidence.checks.olsDialog.selectedMethod !== "Regression"
    || evidence.checks.olsDialog.category !== "Standalone analysis"
    || evidence.checks.olsDialog.outcome !== olsOutcome
    || JSON.stringify(selectedPredictors) !== JSON.stringify(olsPredictors)
    || JSON.stringify(selectedControls) !== JSON.stringify(olsControls)
    || evidence.checks.olsDialog.calculationBasis !== "Raw-value OLS with intercept (fixed)"
    || evidence.checks.olsDialog.variableData !== "Unstandardized numeric values (fixed)"
    || evidence.checks.olsDialog.uncertainty !== "HC3 robust SE; two-sided 95% CI (fixed)"
    || !/Raw numeric ordinary least squares with an intercept/i.test(evidence.checks.olsDialog.validatedScope)
    || evidence.checks.olsDialog.unsupportedControls !== 0 || evidence.checks.olsDialog.blockers.length !== 0
    || !evidence.checks.olsDialog.noModelBlocker || !evidence.checks.olsDialog.startEnabled) {
    throw new Error(`The focused OLS dialog did not match the exact raw/listwise/HC3 model-free contract: ${JSON.stringify(evidence.checks.olsDialog)}`);
  }
  await capture(olsCaptureName(121, "dialog"));

  const activeCapture = captureActiveCalculation(calculation, olsCaptureName(122, "running"), "standalone OLS")
    .then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, completedBeforeCapture: true, detail: error instanceof Error ? error.message : String(error) }));
  await start.click();
  await waitForResultsOrCalculationFailure(calculation, "Packaged OLS calculation");
  evidence.checks.olsProgress = await activeCapture;
  const selectedRun = page.locator(".nd-run-select select option:checked");
  await selectedRun.waitFor({ state: "attached", timeout: 30_000 });
  const runId = await page.locator(".nd-run-select select").inputValue();
  if (!runId) throw new Error("The completed OLS run had no identifier.");
  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const readTable = async (title) => {
    const rows = await openResultTable(title);
    return {
      rows,
      headers: (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText),
      values: await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => Array.from(
        row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "",
      ))),
    };
  };
  const coefficients = await readTable("Coefficients");
  const fit = await readTable("Model fit");
  const scope = await readTable("Calculation scope");
  const coefficientValuesValid = coefficients.values.every((row) => row.slice(1).every((value, index) => (
    index === 3
      ? (/^<0\.0001$/.test(value) || (Number.isFinite(Number(value)) && Number(value) >= 0 && Number(value) <= 1))
      : Number.isFinite(Number(value))
  )));
  const scopeValues = Object.fromEntries(scope.values.map((row) => [row[0], row[1]]));
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const resultsText = [coefficients, fit, scope].flatMap((table) => [table.headers, ...table.values]).flat().join(" ");
  const runDetails = await inspectCurrentRunDetails();
  const editDataCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Data$/i });
  const editModelCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Model$/i });
  evidence.checks.olsResult = {
    runId,
    runLabel: compactVisibleText(await selectedRun.textContent()),
    initialSelectedTable,
    treeItems,
    coefficients,
    fit,
    scope,
    scopeValues,
    runDetails,
    editDataCommand: { count: await editDataCommand.count(), enabled: await editDataCommand.isEnabled().catch(() => false) },
    editModelCommand: { count: await editModelCommand.count() },
    noPlaceholder: !/\bN\/?A\b/i.test(resultsText),
    noSemResultGroups: !/Model estimates|Quality criteria|Mediation|Moderation|Prediction/i.test(treeItems.join(" ")),
  };
  if (initialSelectedTable !== "ols_coefficients"
    || JSON.stringify(treeItems) !== JSON.stringify(["OLS regression", "Coefficients", "Model fit", "Calculation scope"])
    || coefficients.rows !== 4
    || JSON.stringify(coefficients.headers) !== JSON.stringify(["Term", "Estimate", "HC3 SE", "t", "p (two-sided)", "95% CI lower", "95% CI upper"])
    || JSON.stringify(coefficients.values.map((row) => row[0])) !== JSON.stringify(["Intercept", "x", "m", "z"])
    || !coefficientValuesValid
    || fit.rows !== 1 || JSON.stringify(fit.headers) !== JSON.stringify(["Observations", "R\u00B2", "Adjusted R\u00B2", "F", "RMSE", "AIC", "BIC"])
    || fit.values[0]?.[0] !== "140" || fit.values[0]?.slice(1).some((value) => !Number.isFinite(Number(value)))
    || scope.rows !== 12 || scopeValues.Outcome !== olsOutcome || scopeValues.Predictors !== olsPredictors.join(", ")
    || scopeValues.Controls !== olsControls.join(", ") || scopeValues["Analyzed observations"] !== "140"
    || scopeValues["Standard errors"] !== "HC3 heteroskedasticity-consistent"
    || scopeValues["Confidence intervals"] !== "Two-sided 95%" || scopeValues["Validated scope"] !== olsValidatedScope
    || scopeValues["Method version"] !== olsMethodVersion
    || runDetails.properties.Method !== "Ordinary Least Squares Regression"
    || runDetails.properties["Method version"] !== olsMethodVersion || runDetails.properties.Outcome !== olsOutcome
    || runDetails.properties.Predictors !== olsPredictors.join(", ") || runDetails.properties.Controls !== olsControls.join(", ")
    || runDetails.properties.Observations !== "140" || runDetails.properties["Standard errors"] !== "HC3 robust"
    || Object.hasOwn(runDetails.properties, "Weighting") || Object.hasOwn(runDetails.properties, "Preprocessing")
    || evidence.checks.olsResult.editDataCommand.count !== 1 || !evidence.checks.olsResult.editDataCommand.enabled
    || evidence.checks.olsResult.editModelCommand.count !== 0
    || !evidence.checks.olsResult.noPlaceholder || !evidence.checks.olsResult.noSemResultGroups) {
    throw new Error(`The completed OLS result did not expose the exact HC3 coefficient, fit, scope, and model-free return boundary: ${JSON.stringify(evidence.checks.olsResult)}`);
  }
  await openResultTable("Coefficients");
  await capture(olsCaptureName(123, "results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  const tableTitles = ["Coefficients", "Model fit", "Calculation scope", "Fitted values and residuals"];
  const expectedSheets = [...tableTitles, "Run provenance"];
  evidence.checks.olsExport = {
    xlsxEnabled: await xlsxExport.isEnabled(),
    buttonCount: await exportDialog.locator(".nd-export-list button").count(),
    modelDiagramFormats: await exportDialog.getByRole("button", { name: /diagram|svg/i }).count(),
    nativeXlsx: null,
  };
  if (!evidence.checks.olsExport.xlsxEnabled || evidence.checks.olsExport.buttonCount !== 5
    || evidence.checks.olsExport.modelDiagramFormats !== 0) {
    throw new Error(`The model-free OLS result did not expose exactly five table-only export formats: ${JSON.stringify(evidence.checks.olsExport)}`);
  }
  const nativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: exportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets,
    expectedSharedStrings: [...tableTitles, "Run provenance", "HC3 SE", "Validated scope", olsValidatedScope, olsMethodVersion],
  });
  let helperCompleted = false;
  try {
    const ready = await nativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native OLS XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await nativeSaveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`Native OLS XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(exportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(exportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(exportTargetPath);
    evidence.checks.olsExport.nativeXlsx = {
      attempted: true,
      targetPath: exportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    if (!file.isFile() || file.size <= 0 || evidence.checks.olsExport.nativeXlsx.appFeedback !== expectedFeedback
      || !expectedSheets.every((sheet) => workbookSheets.includes(sheet))) {
      throw new Error(`The genuine OLS XLSX did not contain every result, fitted/residual, and provenance sheet: ${JSON.stringify(evidence.checks.olsExport)}`);
    }
  } finally {
    if (!helperCompleted) nativeSaveHelper.stop();
  }
  await capture(olsCaptureName(124, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedArchive = await inspectSavedOlsArchive(olsProjectPath, runId);
  await reloadToLauncher();
  await openRecentProject(olsProjectName, olsProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /Ordinary Least Squares Regression/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened OLS result had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedCoefficientRows = await openResultTable("Coefficients");
  const reopenedFitRows = await openResultTable("Model fit");
  const reopenedScopeRows = await openResultTable("Calculation scope");
  evidence.checks.olsSaveReopen = {
    expectedRunId: runId,
    selectedRunId: reopenedRunId,
    sameRunRestored: reopenedRunId === runId,
    coefficientRows: reopenedCoefficientRows,
    fitRows: reopenedFitRows,
    scopeRows: reopenedScopeRows,
    archive: savedArchive,
  };
  if (!evidence.checks.olsSaveReopen.sameRunRestored || reopenedCoefficientRows !== 4
    || reopenedFitRows !== 1 || reopenedScopeRows !== 12) {
    throw new Error(`The exact model-free OLS run did not survive explicit save/reload/reopen: ${JSON.stringify(evidence.checks.olsSaveReopen)}`);
  }
  await openResultTable("Coefficients");
  await capture(olsCaptureName(125, "reopened"));
  const olsInternalOrigins = new Set([packagedTauriOrigin, packagedTauriIpcOrigin]);
  const olsExternalRequests = observedBrowserRequests.filter((request) => request.origin
    && request.origin !== "null" && !olsInternalOrigins.has(request.origin));
  evidence.checks.olsFunctionalOffline = {
    passed: olsExternalRequests.length === 0,
    analyticalWorkflowRequiresInternet: false,
    strictZeroProcessEgressClaimed: false,
    platformBackgroundEgressOutsidePageRequestScope: true,
    observedRequestCount: observedBrowserRequests.length,
    externalRequestCount: olsExternalRequests.length,
    origins: [...new Set(observedBrowserRequests.map((request) => request.origin))].sort(),
    externalRequests: olsExternalRequests,
  };
  if (!evidence.checks.olsFunctionalOffline.passed) {
    throw new Error(`OLS packaged browser/app workflow crossed its functional-offline request boundary: ${JSON.stringify(evidence.checks.olsFunctionalOffline)}`);
  }
}

async function runFocusedLogisticAcceptance() {
  evidence.checks.logisticWorkflow = {
    passed: false,
    feature_id: logisticFeatureId,
    method_version: logisticMethodVersion,
    catalogue_snapshot_date: logisticCatalogueSnapshotDate,
  };
  if (!requestedLogisticNativeExportPath) {
    throw new Error("QUICKPLS_LOGISTIC_NATIVE_EXPORT_PATH is required for focused packaged logistic acceptance; enabled-button assertions do not replace a genuine native XLSX Save and workbook-content check.");
  }
  const exportTargetPath = await validateRequestedNativeExportPath(
    requestedLogisticNativeExportPath,
    "QUICKPLS_LOGISTIC_NATIVE_EXPORT_PATH",
  );
  await seedRecentProject({ name: logisticProjectName, path: logisticProjectPath, openedAt: "2026-08-12T00:00:00.000Z" });
  await reloadToLauncher();
  await openRecentProject(logisticProjectName, logisticProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const status = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const columns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  const initialArchive = await inspectInitialLogisticArchive(logisticProjectPath);
  const expectedColumns = ["#", "x", "m", "w", "y", "z", "bin_y", "g1", "g2", "g3", "h1", "h2"];
  evidence.checks.logisticFixture = {
    projectPath: logisticProjectPath,
    sourceCsv: logisticFixtureCsvPath,
    status,
    cases: status.includes(`${logisticObservations} cases`) ? logisticObservations : null,
    columns,
    visibleModelNodes: await page.locator(".react-flow__node-latent").count(),
    initialArchive,
  };
  if (evidence.checks.logisticFixture.cases !== logisticObservations
    || JSON.stringify(columns) !== JSON.stringify(expectedColumns)
    || evidence.checks.logisticFixture.visibleModelNodes !== 0
    || initialArchive.models !== 0 || initialArchive.activeModelId !== null) {
    throw new Error(`The focused logistic fixture did not expose the canonical 140-row data-only project: ${JSON.stringify(evidence.checks.logisticFixture)}`);
  }
  await capture(logisticCaptureName(150, "fixture-data"));

  const calculation = await openAnalysisFromDataToolbar();
  const listbox = calculation.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const options = listbox.getByRole("option");
  await calculation.locator("#nd-calculation-method-regression").click();
  const regressionSettings = calculation.locator(".nd-ols-settings");
  await regressionSettings.waitFor({ state: "visible", timeout: 10_000 });
  const regressionType = calculation.locator("#nd-calculation-regression-type");
  await regressionType.selectOption("logistic");
  const outcome = calculation.locator("#nd-calculation-regression-outcome");
  await outcome.selectOption("y");
  const failureProfile = calculation.locator("#nd-calculation-logistic-profile");
  await failureProfile.waitFor({ state: "visible", timeout: 10_000 });
  await page.waitForFunction(() => {
    const node = document.querySelector("#nd-calculation-logistic-profile");
    return node?.getAttribute("aria-busy") === "false"
      && node.textContent?.replace(/\s+/g, " ").includes("140 complete cases: 0 class 0 and 0 class 1");
  }, null, { timeout: 30_000 });
  const failureBlockers = (await calculation.locator(".nd-blocker li").allTextContents()).map(compactVisibleText);
  const failureStart = calculation.getByRole("button", { name: "Start binary logistic regression", exact: true });
  evidence.checks.logisticFailureLifecycle = {
    passed: await failureStart.isDisabled()
      && failureBlockers.some((message) => /not coded exactly 0 or 1/i.test(message))
      && failureBlockers.some((message) => /must contain both class 0 and class 1/i.test(message)),
    rejectedOutcome: "y",
    profileText: compactVisibleText(await failureProfile.textContent()),
    blockers: failureBlockers,
    startDisabled: await failureStart.isDisabled(),
    resultCountBeforeRecovery: await page.locator(".nd-run-select select option").count(),
  };
  if (!evidence.checks.logisticFailureLifecycle.passed
    || evidence.checks.logisticFailureLifecycle.resultCountBeforeRecovery !== 0) {
    throw new Error(`The packaged strict-0/1 logistic failure boundary did not reject the full nonbinary outcome without creating a result: ${JSON.stringify(evidence.checks.logisticFailureLifecycle)}`);
  }
  await outcome.selectOption(logisticOutcome);
  const roleFieldsets = regressionSettings.locator("fieldset.nd-pca-variables");
  for (const fieldset of [roleFieldsets.nth(0), roleFieldsets.nth(1)]) {
    const checked = fieldset.locator('input[type="checkbox"]:checked');
    while (await checked.count()) await checked.first().uncheck();
  }
  for (const variable of logisticPredictors) {
    const label = roleFieldsets.nth(0).locator("label").filter({ hasText: new RegExp(`^\\s*${variable}\\s*$`) });
    if (await label.count() !== 1) throw new Error(`Logistic predictor ${variable} was not exposed as exactly one checkbox.`);
    await label.getByRole("checkbox").check();
  }
  for (const variable of logisticControls) {
    const label = roleFieldsets.nth(1).locator("label").filter({ hasText: new RegExp(`^\\s*${variable}\\s*$`) });
    if (await label.count() !== 1) throw new Error(`Logistic control ${variable} was not exposed as exactly one checkbox.`);
    await label.getByRole("checkbox").check();
  }
  const profile = calculation.locator("#nd-calculation-logistic-profile");
  await profile.waitFor({ state: "visible", timeout: 10_000 });
  await page.waitForFunction(({ expected }) => {
    const node = document.querySelector("#nd-calculation-logistic-profile");
    return node?.getAttribute("aria-busy") === "false"
      && node.textContent?.replace(/\s+/g, " ").includes(expected);
  }, { expected: `${logisticObservations} complete cases: ${logisticZeroCases} class 0 and ${logisticOneCases} class 1; 0 omitted by listwise deletion` }, { timeout: 30_000 });
  const inspectRole = async (fieldset) => fieldset.locator("label").evaluateAll((labels) => labels.filter((label) => (
    label.querySelector('input[type="checkbox"]')?.checked
  )).map((label) => label.querySelector("span")?.textContent?.replace(/\s+/g, " ").trim() ?? ""));
  const selectedPredictors = await inspectRole(roleFieldsets.nth(0));
  const selectedControls = await inspectRole(roleFieldsets.nth(1));
  const noteValue = async (label) => compactVisibleText(await calculation.locator(".nd-setting-note")
    .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
  const start = calculation.getByRole("button", { name: "Start binary logistic regression", exact: true });
  const blockerText = compactVisibleText(await calculation.locator('.nd-blocker[role="alert"]').textContent().catch(() => ""));
  evidence.checks.logisticDialog = {
    catalogCount: await options.count(),
    selectedMethod: compactVisibleText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    category: compactVisibleText(await calculation.locator("#nd-calculation-category-standalone").textContent()),
    regressionType: await regressionType.inputValue(),
    outcome: await outcome.inputValue(),
    selectedPredictors,
    selectedControls,
    calculationBasis: await noteValue("Calculation basis"),
    variableData: await noteValue("Variable data"),
    uncertainty: await noteValue("Uncertainty"),
    profile: {
      text: compactVisibleText(await profile.textContent()),
      ariaBusy: await profile.getAttribute("aria-busy"),
      ariaLive: await profile.getAttribute("aria-live"),
    },
    validatedScope: await noteValue("Validated scope"),
    unsupportedControls: await calculation.locator([
      "#nd-calculation-weighting", "#nd-calculation-preprocessing", "#nd-calculation-max-iterations",
      "#nd-calculation-tolerance", "#nd-calculation-bootstrap-samples", "#nd-calculation-permutations",
      "#nd-calculation-seed", "#nd-calculation-workers", "#nd-calculation-case-weight",
    ].join(", ")).count(),
    blockers: await calculation.locator(".nd-blocker li").allTextContents(),
    blockerText,
    noModelBlocker: !/construct|structural path|editable model|active model/i.test(blockerText),
    startEnabled: await start.isEnabled(),
  };
  if (evidence.checks.logisticDialog.catalogCount !== expectedOptionLabels.length
    || evidence.checks.logisticDialog.selectedMethod !== "Regression"
    || evidence.checks.logisticDialog.category !== "Standalone analysis"
    || evidence.checks.logisticDialog.regressionType !== "logistic"
    || evidence.checks.logisticDialog.outcome !== logisticOutcome
    || JSON.stringify(selectedPredictors) !== JSON.stringify(logisticPredictors)
    || JSON.stringify(selectedControls) !== JSON.stringify(logisticControls)
    || evidence.checks.logisticDialog.calculationBasis !== "Binary logistic maximum likelihood with intercept (fixed)"
    || evidence.checks.logisticDialog.variableData !== "Unstandardized numeric values (fixed)"
    || evidence.checks.logisticDialog.uncertainty !== "Maximum-likelihood SE; Wald z and two-sided 95% CI; odds ratios (fixed)"
    || !evidence.checks.logisticDialog.profile.text.includes(`${logisticZeroCases} class 0 and ${logisticOneCases} class 1`)
    || evidence.checks.logisticDialog.profile.ariaBusy !== "false" || evidence.checks.logisticDialog.profile.ariaLive !== "polite"
    || !/^Binary logistic regression with an intercept, raw numeric predictors, listwise deletion/i.test(evidence.checks.logisticDialog.validatedScope)
    || evidence.checks.logisticDialog.unsupportedControls !== 0 || evidence.checks.logisticDialog.blockers.length !== 0
    || !evidence.checks.logisticDialog.noModelBlocker || !evidence.checks.logisticDialog.startEnabled) {
    throw new Error(`The focused logistic dialog did not match the exact typed-v3 strict-0/1 model-free contract: ${JSON.stringify(evidence.checks.logisticDialog)}`);
  }
  await capture(logisticCaptureName(151, "dialog"));

  const activeCapture = captureActiveCalculation(
    calculation,
    logisticCaptureName(152, "running"),
    "binary logistic regression",
    { allowTerminalTransitionAfterCapture: true },
  )
    .then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, detail: error instanceof Error ? error.message : String(error) }));
  await start.click();
  await waitForResultsOrCalculationFailure(calculation, "Packaged logistic calculation");
  evidence.checks.logisticProgress = await activeCapture;
  if (!evidence.checks.logisticProgress.captured
    || !["queued", "validating", "running", "cancelling"].includes(evidence.checks.logisticProgress.status)) {
    throw new Error(`The packaged logistic run did not expose a genuine active lifecycle state: ${JSON.stringify(evidence.checks.logisticProgress)}`);
  }

  const selectedRun = page.locator(".nd-run-select select option:checked");
  await selectedRun.waitFor({ state: "attached", timeout: 30_000 });
  const runId = await page.locator(".nd-run-select select").inputValue();
  if (!runId) throw new Error("The completed logistic run had no identifier.");
  const logisticRunLabel = compactVisibleText(await selectedRun.textContent());
  evidence.checks.logisticProgress.completedRunProof = {
    runId,
    runLabel: logisticRunLabel,
    matched: logisticRunLabel === "Binary Logistic Regression run",
  };
  if (!evidence.checks.logisticProgress.completedRunProof.matched) {
    throw new Error(`The focused logistic lifecycle did not resolve to its matching completed run: ${JSON.stringify(evidence.checks.logisticProgress)}`);
  }
  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const readTable = async (title) => {
    const rows = await openResultTable(title);
    return {
      rows,
      headers: (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText),
      values: await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => Array.from(
        row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "",
      ))),
      warning: compactVisibleText(await page.locator(".nd-inline-warning").textContent().catch(() => "")),
    };
  };
  const coefficients = await readTable("Coefficients, Wald inference, and odds ratios");
  const fit = await readTable("Model fit and likelihood-ratio inference");
  const classification = await readTable("Classification at probability threshold 0.5");
  const outcomeProfile = await readTable("Binary outcome profile");
  const convergence = await readTable("Estimator convergence");
  const probabilities = await readTable("Complete-case fitted probabilities");
  const scope = await readTable("Calculation scope");
  const scopeValues = Object.fromEntries(scope.values.map((row) => [row[0], row[1]]));
  const fitValues = Object.fromEntries(fit.values.map((row) => [row[0], row[1]]));
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const runDetails = await inspectCurrentRunDetails();
  const editDataCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Data$/i });
  const editModelCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Model$/i });
  const visibleNumber = (value) => Number.isFinite(Number(value));
  const visiblePValue = (value) => /^<0\.0001$/.test(value) || (visibleNumber(value) && Number(value) >= 0 && Number(value) <= 1);
  const coefficientValuesValid = coefficients.values.every((row) => row.length === 10 && row.slice(1).every((value, index) => (
    index === 3 ? visiblePValue(value) : visibleNumber(value)
  )) && Number(row[7]) > 0 && Number(row[8]) > 0 && Number(row[9]) > 0);
  const classificationValues = classification.values[0] ?? [];
  const classificationCount = classificationValues.slice(0, 4).reduce((sum, value) => sum + Number(value), 0);
  const classificationMetricsValid = classificationValues.slice(4).every((value) => visibleNumber(value) && Number(value) >= 0 && Number(value) <= 1);
  const probabilityValuesValid = probabilities.values.every((row, index) => (
    row[0] === String(index + 1) && visibleNumber(row[1]) && Number(row[1]) >= 0 && Number(row[1]) <= 1 && visibleNumber(row[2])
  ));
  const convergenceValues = convergence.values[0] ?? [];
  const resultText = [coefficients, fit, classification, outcomeProfile, convergence, probabilities, scope]
    .flatMap((table) => [table.headers, ...table.values]).flat().join(" ");
  evidence.checks.logisticResult = {
    runId,
    runLabel: compactVisibleText(await selectedRun.textContent()),
    initialSelectedTable,
    treeItems,
    coefficients,
    fit,
    classification,
    outcomeProfile,
    convergence,
    probabilities: { ...probabilities, values: probabilities.values.slice(0, 3), totalRows: probabilities.rows },
    scope,
    scopeValues,
    fitValues,
    runDetails,
    editDataCommand: { count: await editDataCommand.count(), enabled: await editDataCommand.isEnabled().catch(() => false) },
    editModelCommand: { count: await editModelCommand.count() },
    noPlaceholder: !/\bN\/?A\b|NaN|Infinity/i.test(resultText),
    noSemResultGroups: !/Model estimates|Quality criteria|Mediation|Moderation|Prediction/i.test(treeItems.join(" ")),
  };
  const expectedTree = [
    "Binary logistic regression",
    "Coefficients, Wald inference, and odds ratios",
    "Model fit and likelihood-ratio inference",
    "Classification at probability threshold 0.5",
    "Binary outcome profile",
    "Estimator convergence",
    "Complete-case fitted probabilities",
    "Calculation scope",
  ];
  if (initialSelectedTable !== "logistic_coefficients" || JSON.stringify(treeItems) !== JSON.stringify(expectedTree)
    || coefficients.rows !== 4
    || JSON.stringify(coefficients.headers) !== JSON.stringify(["Term", "Estimate", "ML SE", "Wald z", "p (two-sided)", "95% CI lower", "95% CI upper", "Odds ratio", "OR 95% CI lower", "OR 95% CI upper"])
    || JSON.stringify(coefficients.values.map((row) => row[0])) !== JSON.stringify(["Intercept", ...logisticPredictors, ...logisticControls])
    || !coefficientValuesValid || fit.rows !== 11
    || fitValues["Analyzed observations"] !== String(logisticObservations)
    || fitValues["Likelihood-ratio df"] !== "3" || !visiblePValue(fitValues["Likelihood-ratio p"])
    || !Object.entries(fitValues).filter(([label]) => label !== "Likelihood-ratio p").every(([, value]) => visibleNumber(value))
    || classification.rows !== 1 || classification.warning !== logisticClassificationDisclaimer
    || JSON.stringify(classification.headers) !== JSON.stringify(["True positive", "True negative", "False positive", "False negative", "Accuracy", "Sensitivity", "Specificity"])
    || classificationCount !== logisticObservations || !classificationMetricsValid
    || outcomeProfile.rows !== 1
    || JSON.stringify(outcomeProfile.values[0]) !== JSON.stringify([logisticOutcome, "Numeric 0/1 (exact)", "140", "0", "71", "69", "0.4929", "Ready"])
    || convergence.rows !== 1 || convergenceValues[0] !== "Deterministic Newton IRLS" || convergenceValues[1] !== "Yes"
    || !(Number(convergenceValues[2]) > 0 && Number(convergenceValues[2]) <= 100)
    || convergenceValues[3] !== "100" || Number(convergenceValues[4]) !== 1e-8
    || !(Number(convergenceValues[5]) >= 0 && Number(convergenceValues[5]) < 1e-8)
    || Number(convergenceValues[6]) !== 1e-9
    || probabilities.rows !== logisticObservations
    || JSON.stringify(probabilities.headers) !== JSON.stringify(["Complete-case observation", "Fitted probability", "Residual"])
    || !probabilityValuesValid || scope.rows !== 12 || scopeValues.Outcome !== logisticOutcome
    || scopeValues.Predictors !== logisticPredictors.join(", ") || scopeValues.Controls !== logisticControls.join(", ")
    || scopeValues.Estimator !== "Binary logistic maximum likelihood with intercept"
    || scopeValues.Execution !== "Deterministic Newton IRLS; one worker"
    || scopeValues["Classification threshold"] !== "0.5"
    || scopeValues["Classification interpretation"] !== logisticClassificationDisclaimer
    || scopeValues["Validated scope"] !== logisticValidatedScope
    || scopeValues["Method version"] !== logisticMethodVersion
    || runDetails.properties.Method !== "Binary Logistic Regression"
    || runDetails.properties["Method version"] !== logisticMethodVersion
    || runDetails.properties.Weighting !== "path" || runDetails.properties.Preprocessing !== "unstandardized"
    || evidence.checks.logisticResult.editDataCommand.count !== 1 || !evidence.checks.logisticResult.editDataCommand.enabled
    || evidence.checks.logisticResult.editModelCommand.count !== 0
    || !evidence.checks.logisticResult.noPlaceholder || !evidence.checks.logisticResult.noSemResultGroups) {
    throw new Error(`The completed logistic result did not expose the exact Wald, odds-ratio, fit, profile, convergence, classification, probability, disclaimer, and model-free contract: ${JSON.stringify(evidence.checks.logisticResult)}`);
  }
  await openResultTable("Coefficients, Wald inference, and odds ratios");
  await capture(logisticCaptureName(153, "results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  const tableTitles = [
    "Coefficients, Wald inference, and odds ratios",
    "Model fit and likelihood-ratio inference",
    "Classification at probability threshold 0.5",
    "Binary outcome profile",
    "Estimator convergence",
    "Complete-case fitted probabilities",
    "Calculation scope",
  ];
  const expectedSheets = [...tableTitles.map((title) => title.slice(0, 31).trimEnd()), "Run provenance"];
  evidence.checks.logisticExport = {
    xlsxEnabled: await xlsxExport.isEnabled(),
    buttonCount: await exportDialog.locator(".nd-export-list button").count(),
    modelDiagramFormats: await exportDialog.getByRole("button", { name: /diagram|svg/i }).count(),
    inSampleDisclaimerExpected: classification.warning === logisticClassificationDisclaimer,
    inSampleDisclaimerIncluded: false,
    nativeXlsx: null,
  };
  if (!evidence.checks.logisticExport.xlsxEnabled || evidence.checks.logisticExport.buttonCount !== 5
    || evidence.checks.logisticExport.modelDiagramFormats !== 0 || !evidence.checks.logisticExport.inSampleDisclaimerExpected) {
    throw new Error(`The model-free logistic result did not expose exactly five table-only export formats with its in-sample disclaimer: ${JSON.stringify(evidence.checks.logisticExport)}`);
  }
  const nativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: exportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets,
    expectedSharedStrings: [
      "Run provenance", "ML SE", "Odds ratio", "Deterministic Newton IRLS", logisticClassificationDisclaimer,
      "Validated scope", logisticValidatedScope, logisticMethodVersion,
    ],
  });
  let helperCompleted = false;
  try {
    const ready = await nativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native logistic XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await nativeSaveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`Native logistic XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(exportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(exportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(exportTargetPath);
    evidence.checks.logisticExport.nativeXlsx = {
      attempted: true,
      targetPath: exportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    evidence.checks.logisticExport.inSampleDisclaimerIncluded = completion.passed;
    if (!file.isFile() || file.size <= 0 || evidence.checks.logisticExport.nativeXlsx.appFeedback !== expectedFeedback
      || JSON.stringify(workbookSheets) !== JSON.stringify(expectedSheets)) {
      throw new Error(`The genuine logistic XLSX did not contain every diagnostic, probability, scope, and provenance sheet exactly once: ${JSON.stringify(evidence.checks.logisticExport)}`);
    }
  } finally {
    if (!helperCompleted) nativeSaveHelper.stop();
  }
  await capture(logisticCaptureName(154, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedArchive = await inspectSavedLogisticArchive(logisticProjectPath, runId);
  evidence.checks.logisticLegacyV1 = {
    passed: savedArchive.provenanceMethodVersion === logisticMethodVersion
      && savedArchive.regressionMethodVersion === logisticMethodVersion
      && savedArchive.recipe?.schemaVersion === 3
      && savedArchive.recipe?.methodConfig?.model?.type === "logistic"
      && !treeItems.some((item) => /legacy|regression_logistic_v1/i.test(item)),
    currentResultVersion: savedArchive.provenanceMethodVersion,
    typedRecipeSchema: savedArchive.recipe?.schemaVersion ?? null,
    legacyResultTreeItems: treeItems.filter((item) => /legacy|regression_logistic_v1/i.test(item)),
    scope: "This packaged v2 gate proves that current typed-v3 output is not reinterpreted as historical v1. Historical v1 archive readability is covered by the backend persistence gate.",
  };
  if (!evidence.checks.logisticLegacyV1.passed) {
    throw new Error(`The packaged current logistic result was confused with historical v1 output: ${JSON.stringify(evidence.checks.logisticLegacyV1)}`);
  }
  await reloadToLauncher();
  await openRecentProject(logisticProjectName, logisticProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /Binary Logistic Regression/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened logistic result had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedRows = {
    coefficients: await openResultTable("Coefficients, Wald inference, and odds ratios"),
    fit: await openResultTable("Model fit and likelihood-ratio inference"),
    classification: await openResultTable("Classification at probability threshold 0.5"),
    profile: await openResultTable("Binary outcome profile"),
    convergence: await openResultTable("Estimator convergence"),
    probabilities: await openResultTable("Complete-case fitted probabilities"),
    scope: await openResultTable("Calculation scope"),
  };
  evidence.checks.logisticSaveReopen = {
    expectedRunId: runId,
    selectedRunId: reopenedRunId,
    sameRunRestored: reopenedRunId === runId,
    rows: reopenedRows,
    archive: savedArchive,
  };
  if (!evidence.checks.logisticSaveReopen.sameRunRestored
    || JSON.stringify(reopenedRows) !== JSON.stringify({
      coefficients: 4,
      fit: 11,
      classification: 1,
      profile: 1,
      convergence: 1,
      probabilities: logisticObservations,
      scope: 12,
    })) {
    throw new Error(`The exact model-free logistic v2 run did not survive explicit save, checksum inspection, reload, and reopen: ${JSON.stringify(evidence.checks.logisticSaveReopen)}`);
  }
  await openResultTable("Coefficients, Wald inference, and odds ratios");
  await capture(logisticCaptureName(155, "reopened"));
  evidence.checks.logisticWorkflow = {
    passed: true,
    feature_id: logisticFeatureId,
    method_version: logisticMethodVersion,
    catalogue_snapshot_date: logisticCatalogueSnapshotDate,
    fullDataProfiled: true,
    activeLifecycleCaptured: true,
    modelFree: true,
    realXlsxSaved: true,
    explicitSaveAndSameRunReopen: true,
  };
}

async function runFocusedRegressionBootstrapAcceptance() {
  evidence.checks.regressionBootstrapWorkflow = {
    passed: false,
    feature_id: regressionBootstrapFeatureId,
    method_version: regressionBootstrapMethodVersion,
    catalogue_snapshot_date: regressionBootstrapCatalogueSnapshotDate,
  };
  if (!requestedRegressionBootstrapOlsExportPath || !requestedRegressionBootstrapLogisticExportPath) {
    throw new Error("QUICKPLS_REGRESSION_BOOTSTRAP_OLS_EXPORT_PATH and QUICKPLS_REGRESSION_BOOTSTRAP_LOGISTIC_EXPORT_PATH are both required; enabled-button assertions do not replace two genuine native XLSX saves.");
  }
  const olsExportTarget = await validateRequestedNativeExportPath(
    requestedRegressionBootstrapOlsExportPath,
    "QUICKPLS_REGRESSION_BOOTSTRAP_OLS_EXPORT_PATH",
  );
  const logisticExportTarget = await validateRequestedNativeExportPath(
    requestedRegressionBootstrapLogisticExportPath,
    "QUICKPLS_REGRESSION_BOOTSTRAP_LOGISTIC_EXPORT_PATH",
  );
  if (olsExportTarget.toLocaleLowerCase() === logisticExportTarget.toLocaleLowerCase()) {
    throw new Error("Regression bootstrap OLS and logistic exports must use different .xlsx paths.");
  }

  await seedRecentProject({
    name: regressionBootstrapProjectName,
    path: regressionBootstrapProjectPath,
    openedAt: "2026-08-12T00:00:00.000Z",
  });
  await reloadToLauncher();
  await openRecentProject(regressionBootstrapProjectName, regressionBootstrapProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const fixtureStatus = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const fixtureColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  const initialArchive = await inspectInitialLogisticArchive(regressionBootstrapProjectPath);
  evidence.checks.regressionBootstrapFixture = {
    projectPath: regressionBootstrapProjectPath,
    sourceCsv: regressionBootstrapFixtureCsvPath,
    status: fixtureStatus,
    cases: fixtureStatus.includes(`${regressionBootstrapObservations} cases`) ? regressionBootstrapObservations : null,
    columns: fixtureColumns,
    modelFree: initialArchive.models === 0 && initialArchive.activeModelId === null,
    initialArchive,
  };
  if (evidence.checks.regressionBootstrapFixture.cases !== regressionBootstrapObservations
    || !fixtureColumns.includes("y") || !fixtureColumns.includes("bin_y")
    || !fixtureColumns.includes("x") || !fixtureColumns.includes("z") || !fixtureColumns.includes("w")
    || !evidence.checks.regressionBootstrapFixture.modelFree) {
    throw new Error(`The regression bootstrap fixture was not a canonical 140-row model-free project: ${JSON.stringify(evidence.checks.regressionBootstrapFixture)}`);
  }
  await capture(regressionBootstrapCaptureName(160, "fixture-data"));

  const configure = async (model, samples = regressionBootstrapSamples) => {
    const calculation = await openAnalysisFromDataToolbar();
    const listbox = calculation.getByRole("listbox", { name: "Available calculation methods", exact: true });
    await calculation.locator("#nd-calculation-method-regression").click();
    const settings = calculation.locator(".nd-ols-settings");
    await settings.waitFor({ state: "visible", timeout: 10_000 });
    const regressionType = calculation.locator("#nd-calculation-regression-type");
    await regressionType.selectOption(model);
    const outcome = calculation.locator("#nd-calculation-regression-outcome");
    await outcome.selectOption(model === "logistic" ? "bin_y" : "y");
    const roleFieldsets = settings.locator("fieldset.nd-pca-variables");
    for (const fieldset of [roleFieldsets.nth(0), roleFieldsets.nth(1)]) {
      const checked = fieldset.locator('input[type="checkbox"]:checked');
      while (await checked.count()) await checked.first().uncheck();
    }
    const selectRole = async (fieldset, variables, role) => {
      for (const variable of variables) {
        const label = fieldset.locator("label").filter({ hasText: new RegExp(`^\\s*${variable}\\s*$`) });
        if (await label.count() !== 1) throw new Error(`Regression bootstrap ${role} ${variable} was not exposed exactly once.`);
        await label.getByRole("checkbox").check();
      }
    };
    await selectRole(roleFieldsets.nth(0), regressionBootstrapPredictors, "predictor");
    await selectRole(roleFieldsets.nth(1), regressionBootstrapControls, "control");
    await calculation.locator("#nd-calculation-regression-bootstrap").selectOption("enabled");
    await calculation.locator("#nd-calculation-regression-bootstrap-samples").fill(String(samples));
    await calculation.locator("#nd-calculation-regression-bootstrap-workers").fill(String(regressionBootstrapWorkers));
    await calculation.locator("#nd-calculation-seed").fill(String(regressionBootstrapSeed));
    if (model === "logistic") {
      await page.waitForFunction(({ expected }) => {
        const node = document.querySelector("#nd-calculation-logistic-profile");
        return node?.getAttribute("aria-busy") === "false" && node.textContent?.replace(/\s+/g, " ").includes(expected);
      }, { expected: "140 complete cases: 71 class 0 and 69 class 1; 0 omitted by listwise deletion" }, { timeout: 30_000 });
    }
    const selectedRole = async (fieldset) => fieldset.locator("label").evaluateAll((labels) => labels.filter((label) => (
      label.querySelector('input[type="checkbox"]')?.checked
    )).map((label) => label.querySelector("span")?.textContent?.replace(/\s+/g, " ").trim() ?? ""));
    const scope = compactVisibleText(await calculation.locator("#nd-calculation-regression-bootstrap-scope strong").textContent());
    const startLabel = model === "logistic"
      ? "Start binary logistic regression with bootstrap"
      : "Start OLS regression with bootstrap";
    const start = calculation.getByRole("button", { name: startLabel, exact: true });
    const contract = {
      catalogCount: await listbox.getByRole("option").count(),
      selectedMethod: compactVisibleText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent()),
      regressionType: await regressionType.inputValue(),
      outcome: await outcome.inputValue(),
      predictors: await selectedRole(roleFieldsets.nth(0)),
      controls: await selectedRole(roleFieldsets.nth(1)),
      bootstrap: await calculation.locator("#nd-calculation-regression-bootstrap").inputValue(),
      samples: await calculation.locator("#nd-calculation-regression-bootstrap-samples").inputValue(),
      workers: await calculation.locator("#nd-calculation-regression-bootstrap-workers").inputValue(),
      seed: await calculation.locator("#nd-calculation-seed").inputValue(),
      scope,
      blockers: (await calculation.locator(".nd-blocker li").allTextContents()).map(compactVisibleText),
      startEnabled: await start.isEnabled(),
      modelFree: !/construct|structural path|active model/i.test(compactVisibleText(await calculation.locator('.nd-blocker[role="alert"]').textContent().catch(() => ""))),
    };
    if (contract.catalogCount !== expectedOptionLabels.length || contract.selectedMethod !== "Regression"
      || contract.regressionType !== model || contract.outcome !== (model === "logistic" ? "bin_y" : "y")
      || JSON.stringify(contract.predictors) !== JSON.stringify(regressionBootstrapPredictors)
      || JSON.stringify(contract.controls) !== JSON.stringify(regressionBootstrapControls)
      || contract.bootstrap !== "enabled" || contract.samples !== String(samples)
      || contract.workers !== String(regressionBootstrapWorkers) || contract.seed !== String(regressionBootstrapSeed)
      || !/10,000 resamples are recommended for final results/i.test(contract.scope)
      || !/Percentile intervals are primary/i.test(contract.scope)
      || !/BCa is reported when delete-one refits support it/i.test(contract.scope)
      || !/worker-invariant/i.test(contract.scope)
      || contract.blockers.length !== 0 || !contract.startEnabled || !contract.modelFree) {
      throw new Error(`The ${model} regression bootstrap dialog contract was invalid: ${JSON.stringify(contract)}`);
    }
    return { calculation, start, contract };
  };

  const cancelledSetup = await configure("logistic");
  const cancellationActive = captureActiveCalculation(
    cancelledSetup.calculation,
    regressionBootstrapCaptureName(161, "cancellation-running"),
    "binary logistic regression bootstrap cancellation",
  ).then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, detail: error instanceof Error ? error.message : String(error) }));
  await cancelledSetup.start.click();
  const activeState = await cancellationActive;
  const cancelButton = cancelledSetup.calculation.getByRole("button", { name: "Cancel calculation", exact: true });
  await cancelButton.waitFor({ state: "visible", timeout: 15_000 });
  await cancelButton.click();
  const cancelled = cancelledSetup.calculation.locator(".nd-run-progress.cancelled");
  await cancelled.waitFor({ state: "visible", timeout: 30_000 });
  const partialResults = await page.locator(".nd-run-select select option").count();
  evidence.checks.regressionBootstrapCancellation = {
    passed: activeState.captured && partialResults === 0,
    activeLifecycleCaptured: activeState.captured,
    activeState,
    cancelledMessage: compactVisibleText(await cancelled.textContent()),
    noPartialResult: partialResults === 0,
    partialResults,
  };
  if (!evidence.checks.regressionBootstrapCancellation.passed) {
    throw new Error(`Regression bootstrap cancellation left a partial result or missed active lifecycle evidence: ${JSON.stringify(evidence.checks.regressionBootstrapCancellation)}`);
  }
  await capture(regressionBootstrapCaptureName(162, "cancelled"));
  await cancelledSetup.calculation.getByRole("button", { name: "Close", exact: true }).click();

  const readTable = async (title) => {
    const rows = await openResultTable(title);
    return {
      rows,
      headers: (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText),
      values: await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => Array.from(
        row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "",
      ))),
      warning: compactVisibleText(await page.locator(".nd-inline-warning").textContent().catch(() => "")),
    };
  };
  const inspectCompletedResult = async (model) => {
    const selectedRun = page.locator(".nd-run-select select option:checked");
    await selectedRun.waitFor({ state: "attached", timeout: 30_000 });
    const runId = await page.locator(".nd-run-select select").inputValue();
    const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
    const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
    const summary = await readTable("Regression bootstrap summary");
    const coefficientInference = await readTable("Bootstrap coefficient inference");
    const percentile = await readTable("Percentile confidence intervals (primary)");
    const bca = await readTable("BCa confidence intervals (conditional)");
    const oddsRatio = model === "logistic" ? await readTable("Bootstrap odds-ratio intervals") : null;
    const summaryValues = Object.fromEntries(summary.values.map((row) => [row[0], row[1]]));
    const failedCount = Number(summaryValues["Failed replicates"]);
    const failureTreePresent = treeItems.includes("Failed bootstrap replicates");
    const failures = failureTreePresent ? await readTable("Failed bootstrap replicates") : null;
    const failureDisclosureTruthful = Number.isInteger(failedCount) && failedCount >= 0
      && (failedCount === 0
        ? !failureTreePresent
        : failureTreePresent && failures?.rows === failedCount && failures.values.every((row) => row.length === 3 && row.every(Boolean)));
    const allVisibleText = [summary, coefficientInference, percentile, bca, ...(oddsRatio ? [oddsRatio] : []), ...(failures ? [failures] : [])]
      .flatMap((table) => [table.headers, ...table.values, table.warning]).flat().join(" ");
    const contract = {
      model,
      runId,
      runLabel: compactVisibleText(await selectedRun.textContent()),
      initialSelectedTable,
      treeItems,
      summary,
      summaryValues,
      coefficientInference,
      percentile,
      bca,
      oddsRatio,
      failures,
      failureDisclosureTruthful,
      validationWitnessNotRendered: !/validation[_ ]witness|regression_bootstrap_validation_witness_v1|successful_bootstrap|successful_jackknife|failed_jackknife/i.test(`${treeItems.join(" ")} ${allVisibleText}`),
      noNaFabrication: !/\bN\/?A\b|NaN|Infinity/i.test(allVisibleText),
      modelFree: await page.locator(".nd-commandbar button").filter({ hasText: /^Edit Model$/i }).count() === 0,
    };
    const baseGroup = model === "logistic" ? "Binary logistic regression with bootstrap" : "OLS regression with bootstrap";
    const expectedBaseTitles = model === "logistic"
      ? [
          "Coefficients, Wald inference, and odds ratios", "Model fit and likelihood-ratio inference",
          "Classification at probability threshold 0.5", "Binary outcome profile", "Estimator convergence",
          "Complete-case fitted probabilities", "Calculation scope",
        ]
      : ["Coefficients", "Model fit", "Calculation scope"];
    const expectedBootstrapTitles = [
      "Regression bootstrap summary",
      ...(failedCount > 0 ? ["Failed bootstrap replicates"] : []),
      "Bootstrap coefficient inference",
      "Percentile confidence intervals (primary)",
      "BCa confidence intervals (conditional)",
      ...(model === "logistic" ? ["Bootstrap odds-ratio intervals"] : []),
    ];
    const expectedTree = [baseGroup, ...expectedBaseTitles, ...expectedBootstrapTitles];
    const valid = Boolean(runId)
      && initialSelectedTable === "regression_bootstrap_summary"
      && JSON.stringify(treeItems) === JSON.stringify(expectedTree)
      && summary.rows === 17
      && summaryValues["Method version"] === regressionBootstrapMethodVersion
      && summaryValues.Sampling === "Case resampling with replacement"
      && summaryValues.Algorithm === "indexed_case_resampling_v1"
      && summaryValues.Stream === "quickpls_indexed_resampling_v1"
      && summaryValues.Alternative === "Two-sided"
      && summaryValues["Test reference"] === "Standard normal bootstrap ratio"
      && summaryValues["Confidence level"] === "95% (fixed)"
      && summaryValues["Interval policy"] === "Percentile primary; BCa conditional"
      && summaryValues["Requested replicates"] === String(regressionBootstrapSamples)
      && Number(summaryValues["Usable replicates"]) + failedCount === regressionBootstrapSamples
      && summaryValues["Delete-one fits required"] === String(regressionBootstrapObservations)
      && summaryValues.Seed === String(regressionBootstrapSeed)
      && summaryValues.Workers === String(regressionBootstrapWorkers)
      && coefficientInference.rows === regressionBootstrapTerms.length
      && percentile.rows === regressionBootstrapTerms.length
      && bca.rows === regressionBootstrapTerms.length
      && (model !== "logistic" || oddsRatio?.rows === regressionBootstrapTerms.length)
      && contract.failureDisclosureTruthful
      && contract.validationWitnessNotRendered
      && contract.noNaFabrication
      && contract.modelFree;
    if (!valid) throw new Error(`The completed ${model} regression bootstrap result was invalid: ${JSON.stringify(contract)}`);
    return contract;
  };

  const exportCompletedResult = async (model, targetPath, result) => {
    const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
    await exportCommand.click();
    const dialog = page.locator('.nd-dialog-export[role="dialog"]');
    await dialog.waitFor({ state: "visible", timeout: 10_000 });
    const xlsx = dialog.getByRole("button", { name: /XLSX workbook/i });
    const baseTitles = model === "logistic"
      ? [
          "Coefficients, Wald inference, and odds ratios", "Model fit and likelihood-ratio inference",
          "Classification at probability threshold 0.5", "Binary outcome profile", "Estimator convergence",
          "Complete-case fitted probabilities", "Calculation scope",
        ]
      : ["Coefficients", "Model fit", "Calculation scope"];
    const bootstrapTitles = [
      "Regression bootstrap summary",
      ...(result.failures ? ["Failed bootstrap replicates"] : []),
      "Bootstrap coefficient inference", "Percentile confidence intervals (primary)",
      "BCa confidence intervals (conditional)",
      ...(model === "logistic" ? ["Bootstrap odds-ratio intervals"] : []),
    ];
    const supplemental = model === "ols" ? ["Fitted values and residuals"] : [];
    const expectedSheets = [...baseTitles, ...bootstrapTitles, ...supplemental]
      .map((title) => title.slice(0, 31).trimEnd()).concat("Run provenance");
    const helper = startWindowsNativeSaveExportHelper({
      targetPath,
      windowTitle: evidence.checks.runtime.title,
      expectedSheets,
      expectedSharedStrings: [
        "Regression bootstrap summary", "Bootstrap coefficient inference", regressionBootstrapMethodVersion,
        "Case resampling with replacement", "Percentile primary; BCa conditional", "Run provenance",
      ],
    });
    let helperCompleted = false;
    let nativeXlsx = null;
    try {
      const ready = await helper.ready;
      if (!ready.passed || ready.event !== "ready") throw new Error(`Native ${model} regression bootstrap XLSX helper did not become ready: ${JSON.stringify(ready)}`);
      await xlsx.click();
      const completion = await helper.completed;
      helperCompleted = true;
      if (!completion.passed) throw new Error(`Native ${model} regression bootstrap XLSX verification failed: ${JSON.stringify(completion)}`);
      const expectedFeedback = `Saved ${path.basename(targetPath)}.`;
      const feedback = dialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
      await feedback.waitFor({ state: "visible", timeout: 15_000 });
      const file = await fs.stat(targetPath);
      const workbookSheets = await inspectXlsxWorkbookSheets(targetPath);
      nativeXlsx = {
        attempted: true,
        targetPath,
        helper: { ready, completion },
        appFeedback: compactVisibleText(await feedback.textContent()),
        file: { size: file.size, isFile: file.isFile() },
        workbookSheets,
      };
      if (!file.isFile() || file.size <= 0 || nativeXlsx.appFeedback !== expectedFeedback
        || JSON.stringify(workbookSheets) !== JSON.stringify(expectedSheets)) {
        throw new Error(`The genuine ${model} regression bootstrap XLSX sheets were invalid: ${JSON.stringify(nativeXlsx)}`);
      }
    } finally {
      if (!helperCompleted) helper.stop();
    }
    const witnessScan = await xlsxExcludesValidationWitness(targetPath);
    const validationWitnessExcluded = witnessScan.passed;
    nativeXlsx.witnessScan = witnessScan;
    if (!validationWitnessExcluded) throw new Error(`The internal validation witness leaked into the ${model} XLSX.`);
    await dialog.getByRole("button", { name: "Close", exact: true }).click();
    return { passed: true, nativeXlsx, validationWitnessExcluded, expectedSheets };
  };

  const olsSetup = await configure("ols");
  await capture(regressionBootstrapCaptureName(163, "ols-dialog"));
  const olsActive = captureActiveCalculation(
    olsSetup.calculation,
    regressionBootstrapCaptureName(164, "ols-running"),
    "OLS regression with bootstrap",
  ).then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, detail: error instanceof Error ? error.message : String(error) }));
  await olsSetup.start.click();
  await waitForResultsOrCalculationFailure(olsSetup.calculation, "Packaged OLS regression bootstrap", 600_000);
  const olsActiveState = await olsActive;
  if (!olsActiveState.captured) throw new Error(`The OLS regression bootstrap active lifecycle was not captured: ${JSON.stringify(olsActiveState)}`);
  const olsResult = await inspectCompletedResult("ols");
  await openResultTable("Bootstrap coefficient inference");
  await capture(regressionBootstrapCaptureName(165, "ols-results"));
  evidence.checks.regressionBootstrapOlsExport = await exportCompletedResult("ols", olsExportTarget, olsResult);
  await capture(regressionBootstrapCaptureName(166, "ols-export"));

  await openMenuItem("View", "Data");
  await waitForSurface("data");
  const logisticSetup = await configure("logistic");
  await capture(regressionBootstrapCaptureName(167, "logistic-dialog"));
  const logisticActive = captureActiveCalculation(
    logisticSetup.calculation,
    regressionBootstrapCaptureName(168, "logistic-running"),
    "binary logistic regression with bootstrap",
  ).then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, detail: error instanceof Error ? error.message : String(error) }));
  await logisticSetup.start.click();
  await waitForResultsOrCalculationFailure(logisticSetup.calculation, "Packaged logistic regression bootstrap", 600_000);
  const logisticActiveState = await logisticActive;
  if (!logisticActiveState.captured) throw new Error(`The logistic regression bootstrap active lifecycle was not captured: ${JSON.stringify(logisticActiveState)}`);
  const logisticResult = await inspectCompletedResult("logistic");
  await openResultTable("Bootstrap odds-ratio intervals");
  await capture(regressionBootstrapCaptureName(169, "logistic-results"));
  evidence.checks.regressionBootstrapLogisticExport = await exportCompletedResult("logistic", logisticExportTarget, logisticResult);
  await capture(regressionBootstrapCaptureName(170, "logistic-export"));

  evidence.checks.regressionBootstrapResults = {
    passed: olsResult.failureDisclosureTruthful && logisticResult.failureDisclosureTruthful
      && olsResult.validationWitnessNotRendered && logisticResult.validationWitnessNotRendered
      && olsResult.noNaFabrication && logisticResult.noNaFabrication
      && olsResult.initialSelectedTable === regressionBootstrapDefaultTableId
      && logisticResult.initialSelectedTable === regressionBootstrapDefaultTableId,
    olsInitialSelectedTable: olsResult.initialSelectedTable,
    logisticInitialSelectedTable: logisticResult.initialSelectedTable,
    olsCoefficientRows: olsResult.coefficientInference.rows,
    logisticCoefficientRows: logisticResult.coefficientInference.rows,
    percentilePrimaryPresent: olsResult.percentile.rows === regressionBootstrapTerms.length
      && logisticResult.percentile.rows === regressionBootstrapTerms.length,
    bcaConditionalPresent: olsResult.bca.rows === regressionBootstrapTerms.length
      && logisticResult.bca.rows === regressionBootstrapTerms.length,
    failureDisclosureTruthful: olsResult.failureDisclosureTruthful && logisticResult.failureDisclosureTruthful,
    validationWitnessNotRendered: olsResult.validationWitnessNotRendered && logisticResult.validationWitnessNotRendered,
    noNaFabrication: olsResult.noNaFabrication && logisticResult.noNaFabrication,
    ols: olsResult,
    logistic: logisticResult,
  };
  if (!evidence.checks.regressionBootstrapResults.passed) {
    throw new Error(`Regression bootstrap result evidence was incomplete: ${JSON.stringify(evidence.checks.regressionBootstrapResults)}`);
  }

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 30_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 30_000 });
  const archive = await inspectSavedRegressionBootstrapArchive(regressionBootstrapProjectPath, {
    ols: olsResult.runId,
    logistic: logisticResult.runId,
  });
  evidence.checks.regressionBootstrapWitnessBoundary = {
    passed: archive.witnessBoundary.passed
      && olsResult.validationWitnessNotRendered && logisticResult.validationWitnessNotRendered
      && evidence.checks.regressionBootstrapOlsExport.validationWitnessExcluded
      && evidence.checks.regressionBootstrapLogisticExport.validationWitnessExcluded,
    archiveOnly: archive.witnessBoundary.passed,
    termOrderExact: archive.witnessBoundary.termOrderExact,
    bootstrapIndexPartitionExact: archive.witnessBoundary.bootstrapIndexPartitionExact,
    jackknifeIndexPartitionExact: archive.witnessBoundary.jackknifeIndexPartitionExact,
    excludedFromResults: olsResult.validationWitnessNotRendered && logisticResult.validationWitnessNotRendered,
    excludedFromExports: evidence.checks.regressionBootstrapOlsExport.validationWitnessExcluded
      && evidence.checks.regressionBootstrapLogisticExport.validationWitnessExcluded,
  };
  if (!evidence.checks.regressionBootstrapWitnessBoundary.passed) {
    throw new Error(`The regression bootstrap witness crossed its archive-only boundary: ${JSON.stringify(evidence.checks.regressionBootstrapWitnessBoundary)}`);
  }

  await reloadToLauncher();
  await openRecentProject(regressionBootstrapProjectName, regressionBootstrapProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const runSelect = page.locator(".nd-run-select select");
  const olsOption = runSelect.locator("option").filter({ hasText: /Ordinary Least Squares Regression with Bootstrap/i }).first();
  const logisticOption = runSelect.locator("option").filter({ hasText: /Binary Logistic Regression with Bootstrap/i }).first();
  await olsOption.waitFor({ state: "attached", timeout: 30_000 });
  await logisticOption.waitFor({ state: "attached", timeout: 30_000 });
  const reopenedOlsId = await olsOption.getAttribute("value");
  const reopenedLogisticId = await logisticOption.getAttribute("value");
  await runSelect.selectOption(reopenedOlsId);
  await page.waitForFunction((runId) => document.querySelector(".nd-run-select select")?.value === runId, reopenedOlsId, { timeout: 30_000 });
  const reopenedOlsSelectedTable = page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]');
  await reopenedOlsSelectedTable.waitFor({ state: "visible", timeout: 30_000 });
  const reopenedOlsInitialSelectedTable = await reopenedOlsSelectedTable.getAttribute("data-result-tree-item-id");
  const reopenedOlsSummaryRows = await openResultTable("Regression bootstrap summary");
  const reopenedOlsCoefficientRows = await openResultTable("Bootstrap coefficient inference");
  await runSelect.selectOption(reopenedLogisticId);
  await page.waitForFunction((runId) => document.querySelector(".nd-run-select select")?.value === runId, reopenedLogisticId, { timeout: 30_000 });
  const reopenedLogisticSelectedTable = page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]');
  await reopenedLogisticSelectedTable.waitFor({ state: "visible", timeout: 30_000 });
  const reopenedLogisticInitialSelectedTable = await reopenedLogisticSelectedTable.getAttribute("data-result-tree-item-id");
  const reopenedLogisticSummaryRows = await openResultTable("Regression bootstrap summary");
  const reopenedLogisticOddsRows = await openResultTable("Bootstrap odds-ratio intervals");
  evidence.checks.regressionBootstrapSaveReopen = {
    passed: reopenedOlsId === olsResult.runId
      && reopenedLogisticId === logisticResult.runId
      && reopenedOlsSummaryRows === 17
      && reopenedOlsCoefficientRows === regressionBootstrapTerms.length
      && reopenedLogisticSummaryRows === 17
      && reopenedLogisticOddsRows === regressionBootstrapTerms.length
      && reopenedOlsInitialSelectedTable === regressionBootstrapDefaultTableId
      && reopenedLogisticInitialSelectedTable === regressionBootstrapDefaultTableId
      && archive.manifest.projectChecksumMatches
      && archive.witnessBoundary.passed,
    olsSameRunRestored: reopenedOlsId === olsResult.runId,
    logisticSameRunRestored: reopenedLogisticId === logisticResult.runId,
    initialSelectedTables: {
      ols: reopenedOlsInitialSelectedTable,
      logistic: reopenedLogisticInitialSelectedTable,
    },
    rows: {
      olsSummary: reopenedOlsSummaryRows,
      olsCoefficients: reopenedOlsCoefficientRows,
      logisticSummary: reopenedLogisticSummaryRows,
      logisticOddsRatios: reopenedLogisticOddsRows,
    },
    archive,
  };
  if (!evidence.checks.regressionBootstrapSaveReopen.passed) {
    throw new Error(`The two exact regression bootstrap runs did not survive save/reopen: ${JSON.stringify(evidence.checks.regressionBootstrapSaveReopen)}`);
  }
  await capture(regressionBootstrapCaptureName(171, "reopened"));
  evidence.checks.regressionBootstrapWorkflow = {
    passed: true,
    feature_id: regressionBootstrapFeatureId,
    method_version: regressionBootstrapMethodVersion,
    catalogue_snapshot_date: regressionBootstrapCatalogueSnapshotDate,
    olsCompleted: true,
    logisticCompleted: true,
    activeLifecycleCaptured: activeState.captured && olsActiveState.captured && logisticActiveState.captured,
    modelFree: olsResult.modelFree && logisticResult.modelFree,
    realXlsxSaved: true,
    explicitSaveAndSameRunReopen: true,
  };
}

async function runFocusedProcessV2Acceptance() {
  evidence.checks.processV2Workflow = {
    passed: false,
    feature_id: processV2FeatureId,
    method_version: processV2MethodVersion,
    bootstrap_method_version: processV2BootstrapMethodVersion,
    catalogue_snapshot_date: processV2CatalogueSnapshotDate,
  };
  if (!requestedProcessV2ExportPath) {
    throw new Error("QUICKPLS_PROCESS_V2_EXPORT_PATH is required; enabled-button assertions do not replace a genuine native PROCESS v2 XLSX save.");
  }
  if (!processV2ExpectedGraphCounts) {
    throw new Error("PROCESS v2 packaged acceptance requires independently derived expected graph counts from process_v2_reference.py.");
  }
  const exportTarget = await validateRequestedNativeExportPath(
    requestedProcessV2ExportPath,
    "QUICKPLS_PROCESS_V2_EXPORT_PATH",
  );
  await seedRecentProject({
    name: processV2ProjectName,
    path: processV2ProjectPath,
    openedAt: "2026-08-12T00:00:00.000Z",
  });
  await reloadToLauncher();
  await openRecentProject(processV2ProjectName, processV2ProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const fixtureStatus = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const fixtureColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  const initialArchive = await inspectInitialNcaArchive(processV2ProjectPath);
  evidence.checks.processV2Fixture = {
    passed: fixtureStatus.includes("180 cases")
      && JSON.stringify(fixtureColumns.slice(-9)) === JSON.stringify(["X", "M1", "M2", "M3", "M4", "W", "B", "C", "Y"])
      && fixtureColumns.length === 10
      && initialArchive.models === 0 && initialArchive.activeModelId === null,
    status: fixtureStatus,
    columns: fixtureColumns,
    initialArchive,
  };
  if (!evidence.checks.processV2Fixture.passed) {
    throw new Error(`The PROCESS v2 fixture was not an exact 180-row, nine-variable, model-free project: ${JSON.stringify(evidence.checks.processV2Fixture)}`);
  }
  await page.waitForTimeout(processV2IdleSettleMilliseconds);
  await markProcessV2ResourcePhase("initial_idle", {
    surface: "data",
    completed_result_count: 0,
    witness_count: 0,
    selected_run_id: null,
    state_kind: "model_free_fixture",
  }, processV2ProjectPath);
  await page.waitForTimeout(processV2ResourcePostMarkerHoldMilliseconds);
  await capture(processV2CaptureName(180, "fixture-data"));

  const desiredPaths = [
    ["X", "Y"], ["X", "M1"], ["M1", "M2"], ["M2", "Y"],
    ["X", "M3"], ["M3", "Y"], ["X", "M4"], ["M4", "Y"],
  ];
  const desiredModerations = [
    { edge: "X -> Y", primary: "W", conditioning: "B" },
    { edge: "X -> M3", primary: "W", conditioning: "" },
    { edge: "M4 -> Y", primary: "B", conditioning: "" },
  ];
  const readProcessV2SetupSnapshot = async (calculation, action) => {
    const listbox = calculation.getByRole("listbox", { name: "Available calculation methods", exact: true });
    const graph = calculation.locator("#nd-calculation-process-graph");
    const profileNode = calculation.locator("#nd-calculation-process-profile");
    const pathLegend = compactVisibleText(await graph.locator("fieldset").nth(0).locator("legend").textContent());
    const controlLegend = compactVisibleText(await calculation.locator(".nd-process-controls legend").textContent());
    const scope = compactVisibleText(await calculation.locator("#nd-calculation-process-scope strong").textContent());
    const bootstrapScope = compactVisibleText(await calculation.locator("#nd-calculation-regression-bootstrap-scope strong").textContent());
    const predictorCapacity = pathLegend.match(/(\d+)\/(\d+) graph predictors/i);
    const controlCapacity = controlLegend.match(/Controls \((\d+)\/(\d+)/i);
    const equationTermCapacity = scope.match(/the (\d+)-term ceiling/i);
    return {
      catalogCount: await listbox.getByRole("option").count(),
      selectedMethod: compactVisibleText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent()),
      regressionType: await calculation.locator("#nd-calculation-regression-type").inputValue(),
      outcome: await calculation.locator("#nd-process-outcome").inputValue(),
      focalPredictor: await calculation.locator("#nd-process-focal").inputValue(),
      paths: await calculation.locator("[data-process-path-row]").evaluateAll((rows) => rows.map((row) => ({
        from: row.querySelector("select[id^='nd-process-path-from-']")?.value ?? "",
        to: row.querySelector("select[id^='nd-process-path-to-']")?.value ?? "",
      }))),
      moderators: await calculation.locator("[data-process-moderator-row]").evaluateAll((rows) => rows.map((row) => ({
        variable: row.querySelector("select[id^='nd-process-moderator-variable-']")?.value ?? "",
        scale: row.querySelector("select[id^='nd-process-moderator-scale-']")?.value ?? "",
      }))),
      moderations: await calculation.locator("[data-process-moderation-row]").evaluateAll((rows) => rows.map((row) => ({
        edge: row.querySelector("select[id^='nd-process-moderation-edge-']")?.selectedOptions[0]?.textContent?.replace(/\s+/g, " ").trim() ?? "",
        primary: row.querySelector("select[id^='nd-process-moderation-primary-']")?.value ?? "",
        conditioning: row.querySelector("select[id^='nd-process-moderation-conditioning-']")?.value ?? "",
      }))),
      selectedControls: await calculation.locator("[data-process-control]:checked").evaluateAll((inputs) => inputs.map((input) => (
        input.closest("label")?.querySelector("span")?.textContent?.trim() ?? ""
      ))),
      samples: await calculation.locator("#nd-calculation-regression-bootstrap-samples").inputValue(),
      workers: await calculation.locator("#nd-calculation-regression-bootstrap-workers").inputValue(),
      seed: await calculation.locator("#nd-calculation-seed").inputValue(),
      profile: compactVisibleText(await profileNode.textContent()),
      profileAriaBusy: await profileNode.getAttribute("aria-busy"),
      scope,
      bootstrapScope,
      blockers: (await calculation.locator(".nd-blocker li").allTextContents()).map(compactVisibleText),
      startEnabled: await action.isEnabled(),
      capacity: {
        topLevelPredictors: predictorCapacity ? Number(predictorCapacity[1]) : null,
        topLevelPredictorsMaximum: predictorCapacity ? Number(predictorCapacity[2]) : null,
        controls: controlCapacity ? Number(controlCapacity[1]) : null,
        controlsMaximum: controlCapacity ? Number(controlCapacity[2]) : null,
        equationNonInterceptTermsMaximum: equationTermCapacity ? Number(equationTermCapacity[1]) : null,
      },
      graphDefinedWithoutNumberedTemplates: /does not execute numbered PROCESS templates/i.test(compactVisibleText(await graph.textContent())),
    };
  };
  const configure = async () => {
    const calculation = await openAnalysisFromDataToolbar();
    const listbox = calculation.getByRole("listbox", { name: "Available calculation methods", exact: true });
    await calculation.locator("#nd-calculation-method-regression").click();
    const regressionType = calculation.locator("#nd-calculation-regression-type");
    await regressionType.selectOption("process");
    const graph = calculation.locator("#nd-calculation-process-graph");
    await graph.waitFor({ state: "visible", timeout: 10_000 });
    await calculation.locator("#nd-process-outcome").selectOption("Y");
    await calculation.locator("#nd-process-focal").selectOption("X");
    const waitForRows = async (selector, count) => page.waitForFunction(({ selector: value, count: expected }) => (
      document.querySelectorAll(value).length === expected
    ), { selector, count }, { timeout: 10_000 });
    const setPath = async (index, from, to) => {
      await calculation.locator(`#nd-process-path-to-${index}`).selectOption(to);
      await calculation.locator(`#nd-process-path-from-${index}`).selectOption(from);
    };
    const readPaths = () => calculation.locator("[data-process-path-row]").evaluateAll((rows) => rows.map((row) => ({
      from: row.querySelector("select[id^='nd-process-path-from-']")?.value ?? "",
      to: row.querySelector("select[id^='nd-process-path-to-']")?.value ?? "",
    })));
    const expectedPaths = desiredPaths.map(([from, to]) => ({ from, to }));
    const initialPathCount = await calculation.locator("[data-process-path-row]").count();
    if (initialPathCount !== 1 && initialPathCount !== desiredPaths.length) {
      throw new Error(`PROCESS v2 setup exposed ${initialPathCount} path rows; expected a fresh row or the exact persisted graph.`);
    }
    if (initialPathCount === desiredPaths.length) {
      const persistedPaths = await readPaths();
      if (JSON.stringify(persistedPaths) !== JSON.stringify(expectedPaths)) {
        throw new Error(`PROCESS v2 persisted path rows drifted before retry: ${JSON.stringify(persistedPaths)}`);
      }
    } else {
      await waitForRows("[data-process-path-row]", initialPathCount);
      for (let index = initialPathCount; index < desiredPaths.length; index += 1) {
        await calculation.locator("#nd-process-add-path").click();
        await waitForRows("[data-process-path-row]", index + 1);
      }
      for (let index = 0; index < desiredPaths.length; index += 1) {
        await setPath(index, ...desiredPaths[index]);
      }
    }
    const initialModeratorCount = await calculation.locator("[data-process-moderator-row]").count();
    if (initialModeratorCount !== 0 && initialModeratorCount !== 2) {
      throw new Error(`PROCESS v2 setup exposed ${initialModeratorCount} moderator rows; expected none or the exact persisted pair.`);
    }
    const expectedModerators = [{ variable: "W", scale: "continuous" }, { variable: "B", scale: "binary_0_1" }];
    if (initialModeratorCount === 2) {
      const persistedModerators = await calculation.locator("[data-process-moderator-row]").evaluateAll((rows) => rows.map((row) => ({
        variable: row.querySelector("select[id^='nd-process-moderator-variable-']")?.value ?? "",
        scale: row.querySelector("select[id^='nd-process-moderator-scale-']")?.value ?? "",
      })));
      if (JSON.stringify(persistedModerators) !== JSON.stringify(expectedModerators)) {
        throw new Error(`PROCESS v2 persisted moderator rows drifted before retry: ${JSON.stringify(persistedModerators)}`);
      }
    } else {
      for (let index = initialModeratorCount; index < 2; index += 1) {
        await calculation.locator("#nd-process-add-moderator").click();
        await waitForRows("[data-process-moderator-row]", index + 1);
      }
      await calculation.locator("#nd-process-moderator-variable-0").selectOption("W");
      await calculation.locator("#nd-process-moderator-scale-0").selectOption("continuous");
      await calculation.locator("#nd-process-moderator-variable-1").selectOption("B");
      await calculation.locator("#nd-process-moderator-scale-1").selectOption("binary_0_1");
    }
    const initialModerationCount = await calculation.locator("[data-process-moderation-row]").count();
    if (initialModerationCount !== 0 && initialModerationCount !== desiredModerations.length) {
      throw new Error(`PROCESS v2 setup exposed ${initialModerationCount} moderation rows; expected none or the exact persisted set.`);
    }
    if (initialModerationCount === desiredModerations.length) {
      const persistedModerations = await calculation.locator("[data-process-moderation-row]").evaluateAll((rows) => rows.map((row) => ({
        edge: row.querySelector("select[id^='nd-process-moderation-edge-']")?.selectedOptions[0]?.textContent?.replace(/\s+/g, " ").trim() ?? "",
        primary: row.querySelector("select[id^='nd-process-moderation-primary-']")?.value ?? "",
        conditioning: row.querySelector("select[id^='nd-process-moderation-conditioning-']")?.value ?? "",
      })));
      if (JSON.stringify(persistedModerations) !== JSON.stringify(desiredModerations)) {
        throw new Error(`PROCESS v2 persisted moderation rows drifted before retry: ${JSON.stringify(persistedModerations)}`);
      }
    } else {
      for (let index = initialModerationCount; index < desiredModerations.length; index += 1) {
        await calculation.locator("#nd-process-add-moderation").click();
        await waitForRows("[data-process-moderation-row]", index + 1);
      }
      for (let index = 0; index < desiredModerations.length; index += 1) {
        const row = desiredModerations[index];
        await calculation.locator(`#nd-process-moderation-edge-${index}`).selectOption({ label: row.edge });
        await calculation.locator(`#nd-process-moderation-primary-${index}`).selectOption(row.primary);
        const conditioning = calculation.locator(`#nd-process-moderation-conditioning-${index}`);
        if (!await conditioning.isDisabled()) await conditioning.selectOption(row.conditioning);
      }
    }
    const control = calculation.locator(".nd-process-controls label").filter({ hasText: /^C$/ }).locator("[data-process-control]");
    if (await control.count() !== 1) throw new Error("PROCESS v2 did not expose C as exactly one eligible control.");
    await control.check();
    await calculation.locator("#nd-calculation-regression-bootstrap").selectOption("enabled");
    await calculation.locator("#nd-calculation-regression-bootstrap-samples").fill(String(processV2Samples));
    await calculation.locator("#nd-calculation-regression-bootstrap-workers").fill(String(processV2Workers));
    await calculation.locator("#nd-calculation-seed").fill(String(processV2Seed));
    await page.waitForFunction(({ expected }) => {
      const node = document.querySelector("#nd-calculation-process-profile");
      return node?.getAttribute("aria-busy") === "false" && node.textContent?.replace(/\s+/g, " ").includes(expected);
    }, { expected: `${processV2Observations} global listwise-complete cases; ${processV2Omitted} rows omitted; 5 OLS equations verified` }, { timeout: 60_000 });
    const start = calculation.getByRole("button", { name: "Start graph-defined path analysis with bootstrap", exact: true });
    const contract = await readProcessV2SetupSnapshot(calculation, start);
    const valid = contract.catalogCount === expectedOptionLabels.length && contract.selectedMethod === "Regression"
      && contract.regressionType === "process" && contract.outcome === "Y" && contract.focalPredictor === "X"
      && JSON.stringify(contract.paths) === JSON.stringify(expectedPaths)
      && JSON.stringify(contract.moderators) === JSON.stringify(expectedModerators)
      && JSON.stringify(contract.moderations) === JSON.stringify(desiredModerations)
      && JSON.stringify(contract.selectedControls) === JSON.stringify(["C"])
      && contract.samples === String(processV2Samples) && contract.workers === String(processV2Workers)
      && contract.seed === String(processV2Seed) && contract.blockers.length === 0 && contract.startEnabled
      && contract.profileAriaBusy === "false"
      && contract.profile.includes(`${processV2Observations} global listwise-complete cases; ${processV2Omitted} rows omitted; 5 OLS equations verified`)
      && JSON.stringify(contract.capacity) === JSON.stringify({
        topLevelPredictors: 7, topLevelPredictorsMaximum: 8, controls: 1,
        controlsMaximum: 1, equationNonInterceptTermsMaximum: 50,
      })
      && contract.graphDefinedWithoutNumberedTemplates
      && /up to 8 selected predictors/i.test(contract.scope) && /one control entered in every equation/i.test(contract.scope)
      && /50-term ceiling is an equation-design safety bound/i.test(contract.scope)
      && /original sample raw mean/i.test(contract.scope)
      && /Resamples and delete-one fits re-center their equations internally while retaining that original raw probe grid/i.test(contract.scope)
      && /10,000 complete-case resamples are recommended for final results/i.test(contract.bootstrapScope)
      && /Percentile intervals are primary/i.test(contract.bootstrapScope) && /worker-invariant/i.test(contract.bootstrapScope);
    if (!valid) throw new Error(`The PROCESS v2 setup contract was invalid: ${JSON.stringify(contract)}`);
    return { calculation, start, contract };
  };

  const cancelledSetup = await configure();
  await capture(processV2CaptureName(181, "cancellation-dialog"));
  const cancellationActivePromise = captureActiveCalculation(
    cancelledSetup.calculation,
    processV2CaptureName(182, "cancellation-running"),
    "PROCESS v2 cancellation",
  ).then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, detail: error instanceof Error ? error.message : String(error) }));
  await cancelledSetup.start.click();
  const cancellationActive = await cancellationActivePromise;
  const cancelButton = cancelledSetup.calculation.getByRole("button", { name: "Cancel calculation", exact: true });
  await cancelButton.waitFor({ state: "visible", timeout: 15_000 });
  await cancelButton.click();
  const cancelled = cancelledSetup.calculation.locator(".nd-run-progress.cancelled");
  await cancelled.waitFor({ state: "visible", timeout: 60_000 });
  const partialResults = await page.locator(".nd-run-select select option").count();
  evidence.checks.processV2Cancellation = {
    passed: cancellationActive.captured && partialResults === 0,
    activeLifecycleCaptured: cancellationActive.captured,
    noPartialResult: partialResults === 0,
    activeState: cancellationActive,
    cancelledMessage: compactVisibleText(await cancelled.textContent()),
  };
  if (!evidence.checks.processV2Cancellation.passed) {
    throw new Error(`PROCESS v2 cancellation evidence failed: ${JSON.stringify(evidence.checks.processV2Cancellation)}`);
  }
  await capture(processV2CaptureName(183, "cancelled"));
  await page.waitForTimeout(processV2IdleSettleMilliseconds);
  await markProcessV2ResourcePhase("post_cancellation_idle", {
    surface: "data",
    completed_result_count: 0,
    witness_count: 0,
    selected_run_id: null,
    state_kind: "cancelled_setup_no_result",
  }, processV2ProjectPath);
  await page.waitForTimeout(processV2ResourcePostMarkerHoldMilliseconds);

  const retry = cancelledSetup.calculation.getByRole("button", {
    name: "Retry graph-defined path analysis with bootstrap",
    exact: true,
  });
  await retry.waitFor({ state: "visible", timeout: 10_000 });
  if (!await retry.isEnabled()) {
    throw new Error("The cancelled PROCESS v2 calculation did not expose an enabled retry action with its frozen setup.");
  }
  const preRetrySnapshot = await readProcessV2SetupSnapshot(cancelledSetup.calculation, retry);
  const preRetryMatchesFrozenSetup = JSON.stringify(preRetrySnapshot) === JSON.stringify(cancelledSetup.contract);
  evidence.checks.processV2CancelledRetrySetup = {
    passed: preRetryMatchesFrozenSetup,
    readOnly: true,
    exactFrozenSetupMatch: preRetryMatchesFrozenSetup,
    snapshot: preRetrySnapshot,
    frozenSetup: cancelledSetup.contract,
  };
  if (!evidence.checks.processV2CancelledRetrySetup.passed) {
    throw new Error(`The cancelled PROCESS v2 dialog did not retain the exact frozen setup before Retry: ${JSON.stringify(evidence.checks.processV2CancelledRetrySetup)}`);
  }
  const fullSetup = {
    calculation: cancelledSetup.calculation,
    start: retry,
    contract: cancelledSetup.contract,
  };
  evidence.checks.processV2Setup = {
    passed: true,
    outcome: fullSetup.contract.outcome,
    focalPredictor: fullSetup.contract.focalPredictor,
    topLevelPredictors: fullSetup.contract.capacity.topLevelPredictors,
    topLevelPredictorsMaximum: fullSetup.contract.capacity.topLevelPredictorsMaximum,
    paths: fullSetup.contract.paths.length,
    moderators: fullSetup.contract.moderators.length,
    moderations: fullSetup.contract.moderations.length,
    controls: fullSetup.contract.selectedControls.length,
    controlsMaximum: fullSetup.contract.capacity.controlsMaximum,
    equationNonInterceptTermsMaximum: fullSetup.contract.capacity.equationNonInterceptTermsMaximum,
    bootstrapReplicates: Number(fullSetup.contract.samples),
    workers: Number(fullSetup.contract.workers),
    seed: Number(fullSetup.contract.seed),
    contract: fullSetup.contract,
  };
  await capture(processV2CaptureName(184, "dialog"));
  const activePromise = captureActiveCalculation(
    fullSetup.calculation,
    processV2CaptureName(185, "running"),
    "PROCESS v2 10,000-resample run",
  ).then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, detail: error instanceof Error ? error.message : String(error) }));
  await fullSetup.start.click();
  await waitForResultsOrCalculationFailure(fullSetup.calculation, "Packaged PROCESS v2 calculation", 900_000);
  const activeState = await activePromise;
  if (!activeState.captured) throw new Error(`PROCESS v2 did not expose a genuine active lifecycle: ${JSON.stringify(activeState)}`);

  const runSelect = page.locator(".nd-run-select select");
  const runOption = runSelect.locator("option").filter({ hasText: /Graph-defined path analysis with bootstrap/i }).last();
  await runOption.waitFor({ state: "attached", timeout: 30_000 });
  const runId = await runOption.getAttribute("value");
  if (!runId) throw new Error("The completed PROCESS v2 option had no run identifier.");
  await runSelect.selectOption(runId);
  const selectedTreeItem = page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]');
  await selectedTreeItem.waitFor({ state: "visible", timeout: 30_000 });
  await page.waitForFunction((expected) => (
    document.querySelectorAll('.nd-result-tree [role="treeitem"][aria-level="2"]').length === expected
  ), processV2ExpectedTableIds.length, { timeout: 30_000 });
  const initialSelectedTable = await selectedTreeItem.getAttribute("data-result-tree-item-id");
  const group = page.locator('.nd-result-tree [role="treeitem"][aria-level="1"]');
  const groupId = await group.getAttribute("data-result-tree-item-id");
  const groupTitle = compactVisibleText(await group.textContent());
  const tableItems = page.locator('.nd-result-tree [role="treeitem"][aria-level="2"]');
  const tableIds = await tableItems.evaluateAll((items) => items.map((item) => item.getAttribute("data-result-tree-item-id")));
  const titleById = {
    process_model_summary: "Model summary", process_paths: "Directed paths",
    process_equation_coefficients: "Equation coefficients", process_equation_fit: "Equation fit",
    process_reference_effects: "Reference effects", process_conditional_indirect_effects: "Conditional indirect effects",
    process_moderated_mediation_indices: "Moderated-mediation indices",
    process_simple_slopes: "Simple slopes and conditional plots",
    process_conditional_plot_points: "Conditional outcome plot data",
    process_johnson_neyman: "Johnson-Neyman regions",
    process_johnson_neyman_curve_points: "Johnson-Neyman curve data",
    process_bootstrap_summary: "Bootstrap summary", process_bootstrap_failures: "Bootstrap failures",
    process_bootstrap_inference: "Bootstrap inference", process_bootstrap_bca: "Bootstrap BCa intervals",
    process_scope: "Scope and provenance",
  };
  const rendered = {};
  let renderedText = "";
  for (const id of processV2ExpectedTableIds) {
    const item = page.locator(`.nd-result-tree [role="treeitem"][data-result-tree-item-id="${id}"]`);
    await item.click();
    await page.getByRole("heading", { name: titleById[id], exact: true }).waitFor({ state: "visible", timeout: 15_000 });
    const rows = await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => Array.from(
      row.querySelectorAll("th, td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "",
    )));
    const columns = (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText);
    const warning = compactVisibleText(await page.locator(".nd-inline-warning").textContent().catch(() => ""));
    rendered[id] = { rows: rows.length, columns, warning, values: rows };
    renderedText += `\n${titleById[id]}\n${warning}\n${columns.join("\n")}\n${rows.flat().join("\n")}`;
  }
  const summary = Object.fromEntries(rendered.process_bootstrap_summary.values.map((row) => [row[0], row[1]]));
  await page.locator('.nd-result-tree [role="treeitem"][data-result-tree-item-id="process_simple_slopes"]').click();
  await page.getByRole("heading", { name: "Simple slopes and conditional plots", exact: true }).waitFor({ state: "visible", timeout: 15_000 });
  const conditionalPlotAccessibility = await page.locator("[data-process-plot-id]").evaluateAll((figures) => ({
    figureCount: figures.length,
    seriesCounts: figures.map((figure) => figure.querySelectorAll(".nd-process-plot-legend li").length),
    pointDisclosures: figures.map((figure) => figure.querySelector("p")?.textContent?.replace(/\s+/g, " ").trim() ?? ""),
    everySvgNamed: figures.every((figure) => {
      const svg = figure.querySelector('svg[role="img"]');
      const ids = svg?.getAttribute("aria-labelledby")?.split(/\s+/).filter(Boolean) ?? [];
      return ids.length === 2 && ids.every((id) => Boolean(document.getElementById(id)));
    }),
    everyLegendNamed: figures.every((figure) => Boolean(figure.querySelector(".nd-process-plot-legend[aria-label]"))),
    everySeriesHasMarkerText: figures.every((figure) => Array.from(figure.querySelectorAll(".nd-process-plot-legend li span")).every((node) => (
      /circle|square|triangle/i.test(node.textContent ?? "")
    ))),
    dashPatternsPresent: figures.every((figure) => figure.querySelectorAll("polyline.process-estimate[stroke-dasharray]").length >= 1),
  }));
  await page.locator('.nd-result-tree [role="treeitem"][data-result-tree-item-id="process_johnson_neyman"]').click();
  await page.getByRole("heading", { name: "Johnson-Neyman regions", exact: true }).waitFor({ state: "visible", timeout: 15_000 });
  const johnsonNeymanAccessibility = await page.locator("[data-process-jn-moderation]").evaluateAll((figures) => ({
    figureCount: figures.length,
    totalCurveMarkers: figures.reduce((total, figure) => total + figure.querySelectorAll('svg[role="img"] circle').length, 0),
    everySvgNamed: figures.every((figure) => {
      const svg = figure.querySelector('svg[role="img"]');
      const ids = svg?.getAttribute("aria-labelledby")?.split(/\s+/).filter(Boolean) ?? [];
      return ids.length === 2 && ids.every((id) => Boolean(document.getElementById(id)));
    }),
    everyCurveDisclosure: figures.every((figure) => /All 101 curve points, intervals, roots, and regions were persisted by the engine/i.test(figure.querySelector("p")?.textContent ?? "")),
    textualRootsRegions: figures.every((figure) => /Roots:.*Regions:/i.test(figure.querySelector("desc")?.textContent ?? "")),
  }));
  const accessibleNonColorPlotSemantics = conditionalPlotAccessibility.figureCount === 3
    && JSON.stringify(conditionalPlotAccessibility.seriesCounts) === JSON.stringify([3, 6, 2])
    && conditionalPlotAccessibility.pointDisclosures.every((value) => /All 25 points per series/i.test(value))
    && conditionalPlotAccessibility.everySvgNamed && conditionalPlotAccessibility.everyLegendNamed
    && conditionalPlotAccessibility.everySeriesHasMarkerText && conditionalPlotAccessibility.dashPatternsPresent
    && johnsonNeymanAccessibility.figureCount === 3 && johnsonNeymanAccessibility.totalCurveMarkers === 303
    && johnsonNeymanAccessibility.everySvgNamed && johnsonNeymanAccessibility.everyCurveDisclosure
    && johnsonNeymanAccessibility.textualRootsRegions;
  const failedCount = Number(summary["Failed replicates"]);
  const failureRows = rendered.process_bootstrap_failures.values;
  const failureDisclosureTruthful = Number.isInteger(failedCount) && failedCount >= 0
    && (failedCount === 0
      ? failureRows.length === 1 && failureRows[0][1] === "No failed replicates"
      : failureRows.length === failedCount && failureRows.every((row) => /^\d+$/.test(row[0]) && row[1] && row[2]));
  const renderedPrivateDataText = [
    ...tableIds,
    ...Object.values(titleById),
    ...processV2ExpectedTableIds.flatMap((id) => [
      ...rendered[id].columns,
      ...rendered[id].values.flat(),
    ]),
  ].join("\n");
  const validationWitnessNotRendered = !processV2PrivateWitnessWireToken.test(renderedPrivateDataText);
  const noNaFabrication = !/\bN\/?A\b/i.test(renderedText);
  const referenceEffectColumnsExact = JSON.stringify(rendered.process_reference_effects.columns)
    === JSON.stringify(processV2ReferenceColumns);
  const referenceConditionRowsExact = rendered.process_reference_effects.values.length
    === processV2ExpectedGraphCounts.referenceEffects
    && rendered.process_reference_effects.values.every((row) => (
      row.length === processV2ReferenceColumns.length && row[4] === processV2ReferenceCondition
    ));
  const promotionPendingWarningAbsent = processV2ExpectedTableIds.every((id) => (
    !rendered[id].warning.toLocaleLowerCase().startsWith("experimental ")
  ));
  const curveWarningDisclosureExact = rendered.process_johnson_neyman_curve_points.warning
    === processV2CurveWarningDisclosure;
  const johnsonNeymanAnalysisKeys = [...new Map(
    rendered.process_johnson_neyman.values.map((row) => {
      const key = row.slice(0, 3);
      return [JSON.stringify(key), key];
    }),
  ).values()];
  const rowCountsValid = rendered.process_model_summary.rows === 11
    && rendered.process_paths.rows === processV2ExpectedGraphCounts.paths
    && rendered.process_equation_coefficients.rows === 27
    && rendered.process_equation_fit.rows === processV2ExpectedGraphCounts.equations
    && rendered.process_reference_effects.rows === processV2ExpectedGraphCounts.referenceEffects
    && rendered.process_conditional_indirect_effects.rows === processV2ExpectedGraphCounts.conditionalIndirectEffects
    && rendered.process_moderated_mediation_indices.rows === processV2ExpectedGraphCounts.moderatedMediationIndices
    && rendered.process_simple_slopes.rows === processV2ExpectedGraphCounts.simpleSlopes
    && rendered.process_conditional_plot_points.rows === processV2ExpectedGraphCounts.conditionalPlotPoints
    && rendered.process_johnson_neyman.rows === processV2ExpectedGraphCounts.johnsonNeymanRegionRows
    && johnsonNeymanAnalysisKeys.length === processV2ExpectedGraphCounts.johnsonNeyman
    && JSON.stringify(johnsonNeymanAnalysisKeys) === JSON.stringify(processV2ExpectedJohnsonNeymanAnalysisKeys)
    && rendered.process_johnson_neyman_curve_points.rows === processV2ExpectedGraphCounts.johnsonNeymanCurvePoints
    && rendered.process_bootstrap_summary.rows === 13
    && rendered.process_bootstrap_failures.rows >= 1
    && rendered.process_bootstrap_inference.rows === processV2ExpectedGraphCounts.estimands
    && rendered.process_bootstrap_bca.rows === processV2ExpectedGraphCounts.estimands
    && rendered.process_scope.rows === 4;
  const summaryValid = summary["Method version"] === processV2BootstrapMethodVersion
    && summary.Algorithm === "Indexed case resampling with replacement"
    && summary["Interval policy"] === "Percentile primary; BCa conditional"
    && summary["Test reference"] === "Two-sided standard-normal bootstrap ratio"
    && summary["Confidence level"] === "0.95 (fixed)"
    && summary["Requested replicates"] === String(processV2Samples)
    && Number(summary["Usable replicates"]) + failedCount === processV2Samples
    && summary["Seed"] === String(processV2Seed) && summary.Workers === String(processV2Workers)
    && summary.Stream === "process_indexed_case_stream_v1"
    && summary["Probe grid"] === "Original-sample raw moderator probes; each resample and delete-one equation re-centered internally";
  evidence.checks.processV2Results = {
    passed: initialSelectedTable === processV2DefaultTableId && groupId === "process"
      && /Graph-defined path analysis with bootstrap/i.test(groupTitle)
      && JSON.stringify(tableIds) === JSON.stringify(processV2ExpectedTableIds)
      && rowCountsValid && summaryValid && failureDisclosureTruthful
      && validationWitnessNotRendered && noNaFabrication && accessibleNonColorPlotSemantics
      && referenceEffectColumnsExact && referenceConditionRowsExact && promotionPendingWarningAbsent
      && curveWarningDisclosureExact,
    initialSelectedTable, tableIds, exactTableIds: JSON.stringify(tableIds) === JSON.stringify(processV2ExpectedTableIds),
    equationCount: rendered.process_equation_fit.rows,
    referenceEffectRows: rendered.process_reference_effects.rows,
    conditionalIndirectRows: rendered.process_conditional_indirect_effects.rows,
    moderatedMediationIndexRows: rendered.process_moderated_mediation_indices.rows,
    simpleSlopeRows: rendered.process_simple_slopes.rows,
    conditionalPlotPointRows: rendered.process_conditional_plot_points.rows,
    johnsonNeymanRows: rendered.process_johnson_neyman.rows,
    johnsonNeymanAnalysisCount: johnsonNeymanAnalysisKeys.length,
    johnsonNeymanAnalysisKeys,
    renderedJohnsonNeymanRows: rendered.process_johnson_neyman.rows,
    johnsonNeymanCurvePointRows: rendered.process_johnson_neyman_curve_points.rows,
    bootstrapEstimandRows: rendered.process_bootstrap_inference.rows,
    failureDisclosureTruthful, validationWitnessNotRendered, noNaFabrication,
    referenceEffectColumnsExact, referenceConditionRowsExact, promotionPendingWarningAbsent,
    curveWarningDisclosureExact,
    accessibleNonColorPlotSemantics,
    plotAccessibility: { conditional: conditionalPlotAccessibility, johnsonNeyman: johnsonNeymanAccessibility },
    genericRegressionShellNotApplicable: null,
    expectedCountsSource: "validation/process_v2_reference.py:reference_graph",
    expectedGraphCounts: processV2ExpectedGraphCounts,
    summary, rows: rendered,
  };
  if (!evidence.checks.processV2Results.passed) {
    throw new Error(`PROCESS v2 result tables were invalid: ${JSON.stringify(evidence.checks.processV2Results)}`);
  }
  await openResultTable("Bootstrap inference");
  await capture(processV2CaptureName(186, "results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const expectedSheets = [...Object.values(titleById).map((title) => title.slice(0, 31).trimEnd()), "Run provenance"];
  const expectedSharedStrings = [
    "Graph-Defined Path Analysis with Bootstrap", processV2MethodVersion, processV2BootstrapMethodVersion,
    "Original-sample raw moderator probes; each resample and delete-one equation re-centered internally",
    "Run provenance", ...processV2ReferenceColumns,
    processV2ReferenceCondition, processV2ResultStatus, processV2CurveWarningDisclosure,
  ];
  const helper = startWindowsNativeSaveExportHelper({
    targetPath: exportTarget,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets,
    expectedSharedStrings,
  });
  let helperCompleted = false;
  let nativeXlsx;
  try {
    const ready = await helper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`PROCESS v2 XLSX helper did not become ready: ${JSON.stringify(ready)}`);
    await exportDialog.getByRole("button", { name: /XLSX workbook/i }).click();
    const completion = await helper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`PROCESS v2 XLSX verification failed: ${JSON.stringify(completion)}`);
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: `Saved ${path.basename(exportTarget)}.` });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(exportTarget);
    const workbookSheets = await inspectXlsxWorkbookSheets(exportTarget);
    const requiredSharedStrings = completion.workbook?.requiredSharedStrings ?? [];
    const processTableContract = {
      passed: JSON.stringify(requiredSharedStrings) === JSON.stringify(expectedSharedStrings),
      reference_sheet: "Reference effects",
      reference_columns: processV2ReferenceColumns,
      reference_effect_rows: processV2ExpectedGraphCounts.referenceEffects,
      reference_condition: processV2ReferenceCondition,
      result_status: processV2ResultStatus,
      promotion_pending_warning_absent: true,
      curve_warning_disclosure: processV2CurveWarningDisclosure,
      curve_warning_disclosure_exact: requiredSharedStrings.includes(processV2CurveWarningDisclosure),
      required_shared_strings_verified: JSON.stringify(requiredSharedStrings) === JSON.stringify(expectedSharedStrings),
    };
    nativeXlsx = {
      attempted: true, targetPath: exportTarget, helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() }, workbookSheets, processTableContract,
    };
    if (!file.isFile() || file.size <= 0 || JSON.stringify(workbookSheets) !== JSON.stringify(expectedSheets)
      || !processTableContract.passed) {
      throw new Error(`PROCESS v2 XLSX sheet identity drifted: ${JSON.stringify(nativeXlsx)}`);
    }
  } finally {
    if (!helperCompleted) helper.stop();
  }
  const witnessScan = await xlsxExcludesValidationWitness(exportTarget);
  nativeXlsx.witnessScan = witnessScan;
  evidence.checks.processV2Export = {
    passed: witnessScan.passed && nativeXlsx.processTableContract.passed,
    nativeXlsx,
    validationWitnessExcluded: witnessScan.passed,
    expectedSheets,
  };
  if (!evidence.checks.processV2Export.passed) throw new Error("The PROCESS v2 archive-only witness leaked into XLSX.");
  await capture(processV2CaptureName(187, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 30_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 30_000 });
  const archive = await inspectSavedProcessV2Archive(processV2ProjectPath, runId);
  if (path.resolve(processV2ResetProjectPath) === path.resolve(processV2ProjectPath)
    || path.dirname(path.resolve(processV2ResetProjectPath)) !== path.resolve(validationResultsDir)) {
    throw new Error(`PROCESS v2 reset clone path was not a distinct validation/results path: ${processV2ResetProjectPath}`);
  }
  await clearProcessV2ResetArtifacts(processV2ResetProjectPath);
  const resetSidecarsBeforeCopy = await processV2SidecarState(processV2ResetProjectPath);
  await fs.copyFile(processV2ProjectPath, processV2ResetProjectPath, fsConstants.COPYFILE_EXCL);
  const [originalArchiveArtifact, resetArchiveArtifact] = await Promise.all([
    artifactDigest(processV2ProjectPath), artifactDigest(processV2ResetProjectPath),
  ]);
  const resetArchive = await inspectSavedProcessV2Archive(processV2ResetProjectPath, runId);
  const resetLogicalState = await inspectProcessV2LogicalArchiveState(processV2ResetProjectPath);
  const resetSidecarsAfterCopy = await processV2SidecarState(processV2ResetProjectPath);
  evidence.checks.processV2ResourceResetClone = {
    passed: resetSidecarsBeforeCopy.present.length === 0 && resetSidecarsAfterCopy.present.length === 0
      && originalArchiveArtifact !== null && resetArchiveArtifact !== null
      && originalArchiveArtifact.size === resetArchiveArtifact.size
      && originalArchiveArtifact.sha256 === resetArchiveArtifact.sha256
      && resetArchive.identity.resultId === archive.identity.resultId
      && resetArchive.identity.recipeId === archive.identity.recipeId
      && resetArchive.identity.runId === archive.identity.runId
      && resetLogicalState.manifestValid
      && resetLogicalState.completedResultCount === 1 && resetLogicalState.witnessCount === 1
      && resetLogicalState.selectedRunId === runId
      && JSON.stringify(resetLogicalState.completedRunIds) === JSON.stringify([runId])
      && JSON.stringify(resetLogicalState.witnessRunIds) === JSON.stringify([runId]),
    originalPath: path.relative(root, processV2ProjectPath).replaceAll("\\", "/"),
    resetPath: path.relative(root, processV2ResetProjectPath).replaceAll("\\", "/"),
    distinctPath: path.resolve(processV2ResetProjectPath) !== path.resolve(processV2ProjectPath),
    originalArchive: originalArchiveArtifact,
    resetArchive: resetArchiveArtifact,
    identity: resetArchive.identity,
    logicalState: resetLogicalState,
    sidecarsBeforeCopy: resetSidecarsBeforeCopy,
    sidecarsAfterCopy: resetSidecarsAfterCopy,
    sidecarsBeforeOpen: null,
    settledAutosave: null,
    autosaveAfterCheckpoint: null,
    recoveryDisclosureAbsent: null,
  };
  if (!evidence.checks.processV2ResourceResetClone.passed) {
    throw new Error(`PROCESS v2 one-result reset clone was not exact and sidecar-free: ${JSON.stringify(evidence.checks.processV2ResourceResetClone)}`);
  }
  evidence.checks.processV2Results.genericRegressionShellNotApplicable = archive.genericRegressionShellNotApplicable;
  const liveArchiveCountsAgree = archive.graphCounts.equations === evidence.checks.processV2Results.equationCount
    && archive.graphCounts.referenceEffects === evidence.checks.processV2Results.referenceEffectRows
    && archive.graphCounts.conditionalIndirectEffects === evidence.checks.processV2Results.conditionalIndirectRows
    && archive.graphCounts.moderatedMediationIndices === evidence.checks.processV2Results.moderatedMediationIndexRows
    && archive.graphCounts.simpleSlopes === evidence.checks.processV2Results.simpleSlopeRows
    && archive.graphCounts.conditionalPlotPoints === evidence.checks.processV2Results.conditionalPlotPointRows
    && archive.graphCounts.johnsonNeyman === evidence.checks.processV2Results.johnsonNeymanAnalysisCount
    && archive.graphCounts.johnsonNeymanRegionRows === evidence.checks.processV2Results.johnsonNeymanRows
    && archive.graphCounts.availableJohnsonNeyman === johnsonNeymanAccessibility.figureCount
    && archive.graphCounts.johnsonNeymanCurvePoints === evidence.checks.processV2Results.johnsonNeymanCurvePointRows
    && archive.graphCounts.estimands === evidence.checks.processV2Results.bootstrapEstimandRows;
  evidence.checks.processV2Results.liveArchiveCountsAgree = liveArchiveCountsAgree;
  evidence.checks.processV2Results.passed = evidence.checks.processV2Results.passed
    && archive.genericRegressionShellNotApplicable && liveArchiveCountsAgree;
  if (!evidence.checks.processV2Results.passed) {
    throw new Error(`PROCESS v2 live result and saved graph counts disagreed: ${JSON.stringify(evidence.checks.processV2Results)}`);
  }
  evidence.checks.processV2WitnessBoundary = {
    passed: archive.witnessBoundary.passed
      && evidence.checks.processV2Results.validationWitnessNotRendered
      && evidence.checks.processV2Export.validationWitnessExcluded,
    archiveOnly: archive.witnessBoundary.passed,
    witnessMethodVersion: archive.witnessBoundary.witnessMethodVersion,
    estimandOrderExact: archive.witnessBoundary.estimandOrderExact,
    bootstrapIndexPartitionExact: archive.witnessBoundary.bootstrapIndexPartitionExact,
    jackknifeIndexPartitionExact: archive.witnessBoundary.jackknifeIndexPartitionExact,
    excludedFromResults: evidence.checks.processV2Results.validationWitnessNotRendered,
    excludedFromExports: evidence.checks.processV2Export.validationWitnessExcluded,
  };
  if (!evidence.checks.processV2WitnessBoundary.passed) {
    throw new Error(`PROCESS v2 witness boundary failed: ${JSON.stringify(evidence.checks.processV2WitnessBoundary)}`);
  }

  await reloadToLauncher();
  await openRecentProject(processV2ProjectName, processV2ProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /Graph-defined path analysis with bootstrap/i }).last();
  await reopenedOption.waitFor({ state: "attached", timeout: 30_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  const reopenedAutoSelectedRunId = await page.locator(".nd-run-select select").inputValue();
  const reopenedSelected = page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]');
  await reopenedSelected.waitFor({ state: "visible", timeout: 30_000 });
  await page.waitForFunction((expected) => (
    document.querySelectorAll('.nd-result-tree [role="treeitem"][aria-level="2"]').length === expected
  ), processV2ExpectedTableIds.length, { timeout: 30_000 });
  const reopenedInitialSelectedTable = await reopenedSelected.getAttribute("data-result-tree-item-id");
  const reopenedTableIds = await page.locator('.nd-result-tree [role="treeitem"][aria-level="2"]').evaluateAll((items) => (
    items.map((item) => item.getAttribute("data-result-tree-item-id"))
  ));
  const cycle1LogicalState = await inspectProcessV2LogicalArchiveState(processV2ProjectPath);
  if (reopenedRunId !== runId || reopenedAutoSelectedRunId !== runId
    || reopenedInitialSelectedTable !== processV2DefaultTableId
    || JSON.stringify(reopenedTableIds) !== JSON.stringify(processV2ExpectedTableIds)
    || cycle1LogicalState.completedResultCount !== 1 || cycle1LogicalState.witnessCount !== 1
    || cycle1LogicalState.selectedRunId !== runId) {
    throw new Error(`PROCESS v2 original one-result checkpoint was not auto-restored in the frozen default state: ${JSON.stringify({ reopenedRunId, reopenedAutoSelectedRunId, reopenedInitialSelectedTable, reopenedTableIds, cycle1LogicalState })}`);
  }
  await page.waitForTimeout(processV2IdleSettleMilliseconds);
  const cycle1AutosaveState = await processV2SettledAutosaveState(
    processV2ProjectPath,
    { primaryDurability: true },
  );
  const cycle1EffectiveState = cycle1AutosaveState.logicalState;
  if (!cycle1AutosaveState.exactAllowedIdentity || !cycle1EffectiveState?.manifestValid
    || cycle1EffectiveState.completedResultCount !== 1 || cycle1EffectiveState.witnessCount !== 1
    || cycle1EffectiveState.selectedRunId !== runId
    || JSON.stringify(cycle1EffectiveState.completedRunIds) !== JSON.stringify([runId])) {
    throw new Error(`PROCESS v2 original one-result autosave did not settle to the exact allowed state: ${JSON.stringify(cycle1AutosaveState)}`);
  }
  await markProcessV2ResourcePhase("post_completed_cycle_1_idle", {
    surface: "results",
    completed_result_count: cycle1EffectiveState.completedResultCount,
    witness_count: cycle1EffectiveState.witnessCount,
    selected_run_id: runId,
    state_kind: "one_result_reopened_original",
  }, `${processV2ProjectPath}.autosave`);
  await page.waitForTimeout(processV2ResourcePostMarkerHoldMilliseconds);
  const cycle1AutosaveStateAfterCheckpoint = await processV2SettledAutosaveState(
    processV2ProjectPath,
    { primaryDurability: true },
  );
  if (!cycle1AutosaveStateAfterCheckpoint.exactAllowedIdentity
    || cycle1AutosaveStateAfterCheckpoint.autosavePath !== cycle1AutosaveState.autosavePath
    || JSON.stringify(cycle1AutosaveStateAfterCheckpoint.artifacts)
      !== JSON.stringify(cycle1AutosaveState.artifacts)
    || cycle1AutosaveStateAfterCheckpoint.logicalState?.completedResultCount !== 1
    || cycle1AutosaveStateAfterCheckpoint.logicalState?.witnessCount !== 1
    || cycle1AutosaveStateAfterCheckpoint.logicalState?.selectedRunId !== runId) {
    throw new Error(`PROCESS v2 original one-result autosave changed during its idle window: ${JSON.stringify(cycle1AutosaveStateAfterCheckpoint)}`);
  }
  const cycle1SidecarEvidence = await captureProcessV2SidecarEvidence(
    "cycle1",
    cycle1AutosaveStateAfterCheckpoint,
  );
  const cycle1SettledAutosaveEvidence = {
    ...cycle1AutosaveState,
    capturedArtifacts: cycle1SidecarEvidence,
  };
  const cycle1AutosaveAfterCheckpointEvidence = {
    ...cycle1AutosaveStateAfterCheckpoint,
    capturedArtifacts: cycle1SidecarEvidence,
  };
  const reopenedConditionalPlotRows = await openResultTable("Conditional outcome plot data");
  const reopenedJohnsonNeymanCurveRows = await openResultTable("Johnson-Neyman curve data");
  const reopenedInferenceRows = await openResultTable("Bootstrap inference");
  evidence.checks.processV2SaveReopen = {
    passed: reopenedRunId === runId && reopenedInitialSelectedTable === processV2DefaultTableId
      && JSON.stringify(reopenedTableIds) === JSON.stringify(processV2ExpectedTableIds)
      && reopenedConditionalPlotRows === archive.graphCounts.conditionalPlotPoints
      && reopenedJohnsonNeymanCurveRows === archive.graphCounts.johnsonNeymanCurvePoints
      && reopenedInferenceRows === 24 && archive.manifest.projectChecksumMatches && archive.witnessBoundary.passed,
    sameRunRestored: reopenedRunId === runId,
    initialSelectedTable: reopenedInitialSelectedTable,
    tableIds: reopenedTableIds,
    rows: {
      conditionalPlotPoints: reopenedConditionalPlotRows,
      johnsonNeymanCurvePoints: reopenedJohnsonNeymanCurveRows,
      bootstrapInference: reopenedInferenceRows,
    },
    settledAutosave: cycle1SettledAutosaveEvidence,
    autosaveAfterCheckpoint: cycle1AutosaveAfterCheckpointEvidence,
    archive,
  };
  if (!evidence.checks.processV2SaveReopen.passed) {
    throw new Error(`PROCESS v2 did not survive exact save/reopen: ${JSON.stringify(evidence.checks.processV2SaveReopen)}`);
  }
  await capture(processV2CaptureName(188, "reopened"));
  const completedRunIdsBefore = await page.locator(".nd-run-select select option")
    .filter({ hasText: /Graph-defined path analysis with bootstrap/i })
    .evaluateAll((options) => options.map((option) => option.value));

  await openMenuItem("View", "Data");
  await waitForSurface("data");
  const repeatedSetup = await configure();
  const repeatedActivePromise = captureActiveCalculation(
    repeatedSetup.calculation,
    processV2CaptureName(189, "repeated-running"),
    "PROCESS v2 repeated 10,000-resample run",
  ).then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, detail: error instanceof Error ? error.message : String(error) }));
  await repeatedSetup.start.click();
  await waitForSurface("results", 900_000);
  const repeatedActiveState = await repeatedActivePromise;
  const repeatedRunOptions = page.locator(".nd-run-select select option").filter({ hasText: /Graph-defined path analysis with bootstrap/i });
  await page.waitForFunction((priorCount) => (
    Array.from(document.querySelectorAll(".nd-run-select select option"))
      .filter((option) => /Graph-defined path analysis with bootstrap/i.test(option.textContent ?? "")).length >= priorCount + 1
  ), completedRunIdsBefore.length, { timeout: 30_000 });
  const completedRunIdsAfter = await repeatedRunOptions.evaluateAll((options) => options.map((option) => option.value));
  const addedRunIds = completedRunIdsAfter.filter((id) => !completedRunIdsBefore.includes(id));
  const repeatedRunId = addedRunIds.length === 1 ? addedRunIds[0] : null;
  const autoSelectedRunId = await runSelect.inputValue();
  if (repeatedRunId) await runSelect.selectOption(repeatedRunId);
  const explicitlySelectedRunId = await runSelect.inputValue();
  await page.waitForFunction((expected) => (
    document.querySelectorAll('.nd-result-tree [role="treeitem"][aria-level="2"]').length === expected
  ), processV2ExpectedTableIds.length, { timeout: 30_000 });
  const repeatedInitialTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]')
    .getAttribute("data-result-tree-item-id");
  const completedRunIdsBeforeUnique = completedRunIdsBefore.length === new Set(completedRunIdsBefore).size;
  const completedRunIdsAfterUnique = completedRunIdsAfter.length === new Set(completedRunIdsAfter).size;
  const priorIdsPreserved = completedRunIdsBefore.every((id) => completedRunIdsAfter.includes(id));
  evidence.checks.processV2RepeatedCompletion = {
    passed: repeatedActiveState.captured && Boolean(repeatedRunId) && repeatedRunId !== runId
      && completedRunIdsBefore.length === 1 && completedRunIdsBefore[0] === runId
      && completedRunIdsBefore.every(Boolean) && completedRunIdsBeforeUnique
      && completedRunIdsAfter.length === completedRunIdsBefore.length + 1
      && completedRunIdsAfter.every(Boolean) && completedRunIdsAfterUnique && priorIdsPreserved
      && addedRunIds.length === 1 && addedRunIds[0] === repeatedRunId
      && autoSelectedRunId === repeatedRunId && explicitlySelectedRunId === repeatedRunId
      && repeatedInitialTable === processV2DefaultTableId,
    activeLifecycleCaptured: repeatedActiveState.captured,
    priorRunId: runId,
    repeatedRunId,
    completedRunIdsBefore,
    completedRunIdsAfter,
    addedRunIds,
    completedRunCountBefore: completedRunIdsBefore.length,
    completedRunCount: completedRunIdsAfter.length,
    uniqueCompletedRunCount: new Set(completedRunIdsAfter).size,
    autoSelectedRunId,
    explicitlySelectedRunId,
    initialSelectedTable: repeatedInitialTable,
  };
  if (!evidence.checks.processV2RepeatedCompletion.passed) {
    throw new Error(`PROCESS v2 did not complete a second genuine resource cycle: ${JSON.stringify(evidence.checks.processV2RepeatedCompletion)}`);
  }
  const historyDefaultTable = page.locator(`.nd-result-tree [data-result-tree-item-id="${processV2DefaultTableId}"]`);
  await historyDefaultTable.click();
  await page.waitForFunction((tableId) => (
    document.querySelector('.nd-result-tree [role="treeitem"][aria-selected="true"]')?.getAttribute("data-result-tree-item-id") === tableId
  ), processV2DefaultTableId, { timeout: 10_000 });
  await page.waitForTimeout(processV2IdleSettleMilliseconds);
  const historyAutosavePath = `${processV2ProjectPath}.autosave`;
  const historyLogicalState = await inspectProcessV2LogicalArchiveState(historyAutosavePath);
  if (!historyLogicalState.manifestValid || historyLogicalState.completedResultCount !== 2
    || historyLogicalState.witnessCount !== 2
    || historyLogicalState.completedRunIds.length !== 2
    || !historyLogicalState.completedRunIds.includes(runId)
    || !historyLogicalState.completedRunIds.includes(repeatedRunId)
    || JSON.stringify([...historyLogicalState.completedRunIds].sort())
      !== JSON.stringify([...historyLogicalState.witnessRunIds].sort())) {
    throw new Error(`PROCESS v2 retained-history autosave did not contain two exact witnessed results: ${JSON.stringify(historyLogicalState)}`);
  }
  await markProcessV2ResourcePhase("post_completed_history_2_idle", {
    surface: "results",
    completed_result_count: historyLogicalState.completedResultCount,
    witness_count: historyLogicalState.witnessCount,
    selected_run_id: repeatedRunId,
    state_kind: "two_results_retained_history",
  }, historyAutosavePath);
  await page.waitForTimeout(processV2ResourcePostMarkerHoldMilliseconds);

  await reloadToLauncher();
  const resetSidecarsBeforeOpen = await processV2SidecarState(processV2ResetProjectPath);
  if (resetSidecarsBeforeOpen.present.length !== 0) {
    throw new Error(`PROCESS v2 reset clone acquired a sidecar before open: ${JSON.stringify(resetSidecarsBeforeOpen)}`);
  }
  await openProjectAtExactPath(processV2ProjectName, processV2ResetProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const resetRunOptions = page.locator(".nd-run-select select option")
    .filter({ hasText: /Graph-defined path analysis with bootstrap/i });
  await resetRunOptions.first().waitFor({ state: "attached", timeout: 30_000 });
  const resetRunIds = await resetRunOptions.evaluateAll((options) => options.map((option) => option.value));
  if (JSON.stringify(resetRunIds) !== JSON.stringify([runId])) {
    throw new Error(`PROCESS v2 reset clone did not expose exactly the original run: ${JSON.stringify(resetRunIds)}`);
  }
  const resetAutoSelectedRunId = await runSelect.inputValue();
  await page.waitForFunction((expected) => (
    document.querySelectorAll('.nd-result-tree [role="treeitem"][aria-level="2"]').length === expected
  ), processV2ExpectedTableIds.length, { timeout: 30_000 });
  const resetTableIds = await page.locator('.nd-result-tree [role="treeitem"][aria-level="2"]')
    .evaluateAll((items) => items.map((item) => item.getAttribute("data-result-tree-item-id")));
  const resetInitialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]')
    .getAttribute("data-result-tree-item-id");
  const resetRecoveryText = compactVisibleText(await page.locator(".nd-toast").allTextContents());
  const resetArchiveAfterOpen = await inspectProcessV2LogicalArchiveState(processV2ResetProjectPath);
  const resetReopenPassed = resetAutoSelectedRunId === runId
    && resetInitialSelectedTable === processV2DefaultTableId
    && JSON.stringify(resetTableIds) === JSON.stringify(processV2ExpectedTableIds)
    && !/recover(?:y|ed)|autosave/i.test(resetRecoveryText)
    && resetArchiveAfterOpen.manifestValid && resetArchiveAfterOpen.completedResultCount === 1
    && resetArchiveAfterOpen.witnessCount === 1
    && JSON.stringify(resetArchiveAfterOpen.completedRunIds) === JSON.stringify([runId]);
  evidence.checks.processV2ResourceResetClone.sidecarsBeforeOpen = resetSidecarsBeforeOpen;
  evidence.checks.processV2ResourceResetClone.recoveryDisclosureAbsent = !/recover(?:y|ed)|autosave/i.test(resetRecoveryText);
  evidence.checks.processV2ResourceResetClone.resetTableIds = resetTableIds;
  evidence.checks.processV2ResourceResetClone.selectedRunId = resetAutoSelectedRunId;
  evidence.checks.processV2ResourceResetClone.selectedTableId = resetInitialSelectedTable;
  evidence.checks.processV2ResourceResetClone.passed = evidence.checks.processV2ResourceResetClone.passed && resetReopenPassed;
  if (!evidence.checks.processV2ResourceResetClone.passed) {
    throw new Error(`PROCESS v2 reset clone did not reopen as the exact one-result state without recovery disclosure: ${JSON.stringify(evidence.checks.processV2ResourceResetClone)}`);
  }
  await page.waitForTimeout(processV2IdleSettleMilliseconds);
  const resetSettledAutosave = await processV2SettledAutosaveState(
    processV2ResetProjectPath,
    { primaryDurability: false },
  );
  const resetEffectiveState = resetSettledAutosave.logicalState;
  if (!resetSettledAutosave.exactAllowedIdentity || !resetEffectiveState?.manifestValid
    || resetEffectiveState.completedResultCount !== 1 || resetEffectiveState.witnessCount !== 1
    || resetEffectiveState.selectedRunId !== runId
    || JSON.stringify(resetEffectiveState.completedRunIds) !== JSON.stringify([runId])) {
    throw new Error(`PROCESS v2 reset clone autosave did not settle to the exact allowed state: ${JSON.stringify(resetSettledAutosave)}`);
  }
  evidence.checks.processV2ResourceResetClone.settledAutosave = resetSettledAutosave;
  await markProcessV2ResourcePhase("post_completed_cycle_2_idle", {
    surface: "results",
    completed_result_count: resetEffectiveState.completedResultCount,
    witness_count: resetEffectiveState.witnessCount,
    selected_run_id: runId,
    state_kind: "one_result_reopened_reset_clone",
  }, `${processV2ResetProjectPath}.autosave`);
  await page.waitForTimeout(processV2ResourcePostMarkerHoldMilliseconds);
  const resetAutosaveAfterCheckpoint = await processV2SettledAutosaveState(
    processV2ResetProjectPath,
    { primaryDurability: false },
  );
  evidence.checks.processV2ResourceResetClone.autosaveAfterCheckpoint = resetAutosaveAfterCheckpoint;
  evidence.checks.processV2ResourceResetClone.passed = evidence.checks.processV2ResourceResetClone.passed
    && resetSidecarsBeforeOpen.present.length === 0
    && resetAutosaveAfterCheckpoint.exactAllowedIdentity
    && JSON.stringify(resetAutosaveAfterCheckpoint.artifacts)
      === JSON.stringify(resetSettledAutosave.artifacts)
    && resetAutosaveAfterCheckpoint.logicalState?.completedResultCount === 1
    && resetAutosaveAfterCheckpoint.logicalState?.witnessCount === 1
    && resetAutosaveAfterCheckpoint.logicalState?.selectedRunId === runId;
  if (!evidence.checks.processV2ResourceResetClone.passed) {
    throw new Error(`PROCESS v2 reset clone autosave drifted or acquired forbidden recovery artifacts during its idle window: ${JSON.stringify(evidence.checks.processV2ResourceResetClone)}`);
  }
  evidence.checks.processV2Workflow = {
    passed: true,
    feature_id: processV2FeatureId,
    method_version: processV2MethodVersion,
    bootstrap_method_version: processV2BootstrapMethodVersion,
    catalogue_snapshot_date: processV2CatalogueSnapshotDate,
    completed: true,
    activeLifecycleCaptured: activeState.captured && repeatedActiveState.captured,
    completedRuns: 2,
    modelFree: archive.modelFree,
    graphDefinedWithoutNumberedTemplates: fullSetup.contract.graphDefinedWithoutNumberedTemplates,
    realXlsxSaved: true,
    explicitSaveAndSameRunReopen: true,
  };
}

async function runFocusedCtaPlsAcceptance() {
  if (!requestedCtaPlsNativeExportPath) {
    throw new Error("QUICKPLS_CTA_PLS_NATIVE_EXPORT_PATH is required for focused packaged CTA-PLS acceptance; enabled-button assertions do not replace a genuine native XLSX Save and workbook-content check.");
  }
  const exportTargetPath = await validateRequestedNativeExportPath(
    requestedCtaPlsNativeExportPath,
    "QUICKPLS_CTA_PLS_NATIVE_EXPORT_PATH",
  );

  await seedRecentProject({
    name: ctaPlsProjectName,
    path: ctaPlsProjectPath,
    openedAt: "2026-08-13T00:00:00.000Z",
  });
  await reloadToLauncher();
  await openRecentProject(ctaPlsProjectName, ctaPlsProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const status = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const columns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  const initialArchive = await inspectInitialCtaPlsArchive(ctaPlsProjectPath);
  evidence.checks.ctaPlsFixture = {
    projectPath: ctaPlsProjectPath,
    status,
    cases: status.includes("120 cases") ? 120 : null,
    columns,
    initialArchive,
  };
  if (evidence.checks.ctaPlsFixture.cases !== 120
    || JSON.stringify(columns) !== JSON.stringify(["#", "x1", "x2", "x3", "x4", "y1", "y2"])) {
    throw new Error(`The focused CTA-PLS fixture did not expose the canonical 120-row reference data: ${JSON.stringify(evidence.checks.ctaPlsFixture)}`);
  }
  await capture(ctaPlsCaptureName(200, "fixture-data"));

  evidence.checks.ctaPlsInitialModel = await createInitialEditableModel(ctaPlsProjectName, ctaPlsModelName);
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const invalidArchiveBefore = await inspectCtaPlsArchiveCounts(ctaPlsProjectPath);
  const invalidDialog = await openCalculationFromToolbar();
  const invalidOption = invalidDialog.locator("#nd-calculation-method-cta_pls");
  await invalidOption.click();
  const invalidStart = invalidDialog.getByRole("button", { name: "Start tetrad diagnostics", exact: true });
  const invalidBlockers = (await invalidDialog.locator(".nd-blocker li").allTextContents()).map(compactVisibleText);
  const invalidEligibleText = compactVisibleText(await invalidDialog.locator("#nd-calculation-cta-pls-scope strong").textContent());
  evidence.checks.ctaPlsInvalidSetup = {
    attempted: true,
    selectedMethod: compactVisibleText(await invalidDialog.getByRole("option", { selected: true }).locator("strong").textContent()),
    startEnabled: await invalidStart.isEnabled(),
    blockers: invalidBlockers,
    eligibleBlockText: invalidEligibleText,
    archiveBefore: invalidArchiveBefore,
    archiveAfter: null,
    runStateUnchanged: false,
    resultCreated: false,
  };
  const ctaSpecificInvalidBlockers = invalidBlockers.filter((blocker) => (
    /CTA-PLS requires at least one ordinary construct with four or more assigned indicators/i.test(blocker)
  ));
  if (evidence.checks.ctaPlsInvalidSetup.selectedMethod !== "Confirmatory Tetrad Analysis"
    || evidence.checks.ctaPlsInvalidSetup.startEnabled
    || ctaSpecificInvalidBlockers.length !== 1
    || invalidEligibleText !== "None - assign at least four indicators to one ordinary construct") {
    throw new Error(`The invalid CTA-PLS setup did not fail closed at the exact applicability boundary: ${JSON.stringify(evidence.checks.ctaPlsInvalidSetup)}`);
  }
  await capture(ctaPlsCaptureName(201, "invalid-setup"));
  await invalidDialog.getByRole("button", { name: "Close", exact: true }).click();
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const invalidArchiveAfter = await inspectCtaPlsArchiveCounts(ctaPlsProjectPath);
  evidence.checks.ctaPlsInvalidSetup.archiveAfter = invalidArchiveAfter;
  evidence.checks.ctaPlsInvalidSetup.runStateUnchanged = ["recipeCount", "resultCount", "runCount"]
    .every((field) => invalidArchiveBefore[field] === 0 && invalidArchiveAfter[field] === 0);
  evidence.checks.ctaPlsInvalidSetup.resultCreated = invalidArchiveAfter.resultCount !== 0;
  if (!evidence.checks.ctaPlsInvalidSetup.runStateUnchanged || evidence.checks.ctaPlsInvalidSetup.resultCreated) {
    throw new Error(`The blocked CTA-PLS attempt created persisted calculation state: ${JSON.stringify(evidence.checks.ctaPlsInvalidSetup)}`);
  }

  evidence.checks.ctaPlsModel = await buildCtaPlsModel();
  await capture(ctaPlsCaptureName(202, "model"));
  const calculation = await openCalculationFromToolbar();
  const listbox = calculation.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const options = listbox.getByRole("option");
  await calculation.locator("#nd-calculation-method-cta_pls").click();
  const eligibleBlockText = compactVisibleText(await calculation.locator("#nd-calculation-cta-pls-scope strong").textContent());
  const scopeText = compactVisibleText(await calculation.locator("#nd-calculation-cta-pls-scope small").textContent());
  const weighting = calculation.locator("#nd-calculation-weighting");
  const preprocessing = calculation.locator("#nd-calculation-preprocessing");
  const pcaWeightingDisabled = await weighting.locator('option[value="pca"]').evaluate((option) => option.disabled);
  const start = calculation.getByRole("button", { name: "Start tetrad diagnostics", exact: true });
  const canonicalCatalogKinds = await canonicalNativeAnalysisCatalogKinds();
  const catalogKinds = await options.evaluateAll((elements) => elements.map((element) => (
    element.id.startsWith("nd-calculation-method-") ? element.id.slice("nd-calculation-method-".length) : null
  )));
  evidence.checks.ctaPlsDialog = {
    catalogCount: await options.count(),
    canonicalCatalogKinds,
    catalogKinds,
    selectedMethod: compactVisibleText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    category: compactVisibleText(await calculation.locator("#nd-calculation-category-assessment").textContent()),
    eligibleBlockText,
    scopeText,
    weighting: await weighting.inputValue(),
    pcaWeightingDisabled,
    preprocessing: await preprocessing.inputValue(),
    maximumIterations: await calculation.locator("#nd-calculation-max-iterations").inputValue(),
    tolerance: await calculation.locator("#nd-calculation-tolerance").inputValue(),
    unsupportedControls: await calculation.locator([
      "#nd-calculation-bootstrap-samples", "#nd-calculation-confidence", "#nd-calculation-studentized",
      "#nd-calculation-permutations", "#nd-calculation-workers", "#nd-calculation-case-weight",
      "#nd-calculation-group-column", "#nd-calculation-ipma-target", "#nd-calculation-nca-permutations",
    ].join(", ")).count(),
    blockers: (await calculation.locator(".nd-blocker li").allTextContents()).map(compactVisibleText),
    startEnabled: await start.isEnabled(),
  };
  if (evidence.checks.ctaPlsDialog.catalogCount !== expectedOptionLabels.length
    || JSON.stringify(catalogKinds) !== JSON.stringify(canonicalCatalogKinds)
    || evidence.checks.ctaPlsDialog.selectedMethod !== "Confirmatory Tetrad Analysis"
    || evidence.checks.ctaPlsDialog.category !== "Assessment"
    || eligibleBlockText !== "Predictor: 4 indicators, 3 tetrads" || scopeText !== ctaPlsScopeNote
    || evidence.checks.ctaPlsDialog.weighting !== "path" || !pcaWeightingDisabled
    || evidence.checks.ctaPlsDialog.preprocessing !== "standardized"
    || evidence.checks.ctaPlsDialog.maximumIterations !== "3000"
    || Number(evidence.checks.ctaPlsDialog.tolerance) !== 1e-7
    || evidence.checks.ctaPlsDialog.unsupportedControls !== 0
    || evidence.checks.ctaPlsDialog.blockers.length !== 0 || !evidence.checks.ctaPlsDialog.startEnabled) {
    throw new Error(`The focused CTA-PLS dialog did not match the exact bounded descriptive contract: ${JSON.stringify(evidence.checks.ctaPlsDialog)}`);
  }
  await capture(ctaPlsCaptureName(203, "dialog"));

  const activeCapture = captureActiveCalculation(calculation, ctaPlsCaptureName(204, "running"), "CTA-PLS descriptive tetrads")
    .then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, completedBeforeCapture: true, detail: error instanceof Error ? error.message : String(error) }));
  await start.click();
  await waitForSurface("results", 120_000);
  evidence.checks.ctaPlsProgress = await activeCapture;

  const selectedRun = page.locator(".nd-run-select select option:checked");
  await selectedRun.waitFor({ state: "attached", timeout: 30_000 });
  const runId = await page.locator(".nd-run-select select").inputValue();
  if (!runId) throw new Error("The completed CTA-PLS run had no identifier.");
  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const readTable = async (title) => {
    const rows = await openResultTable(title);
    return {
      rows,
      headers: (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText),
      values: await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => Array.from(
        row.querySelectorAll("th,td"),
        (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "",
      ))),
    };
  };
  const summary = await readTable("CTA-PLS tetrad summary");
  const tetrads = await readTable("CTA-PLS tetrads");
  const scope = await readTable("CTA-PLS requirements and exclusions");
  const scopeValues = Object.fromEntries(scope.values.map((row) => [row[0], row[1]]));
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const runDetails = await inspectCurrentRunDetails();
  const numericTetrads = tetrads.values.map((row) => [Number(row[6]), Number(row[7])]);
  const ctaText = [summary, tetrads, scope].flatMap((table) => [table.headers, ...table.values]).flat().join(" ");
  evidence.checks.ctaPlsResult = {
    runId,
    runLabel: compactVisibleText(await selectedRun.textContent()),
    initialSelectedTable,
    treeItems,
    summary,
    tetrads,
    scope,
    scopeValues,
    runDetails,
    allTetradsFinite: numericTetrads.every((row) => row.every(Number.isFinite)),
    absoluteColumnsMatch: numericTetrads.every(([signed, absolute]) => Math.abs(Math.abs(signed) - absolute) <= 1e-4),
    noInferentialDecision: !/\bp[- ]?value\b|confidence interval|statistically significant|vanishing decision/i.test(ctaText),
  };
  if (initialSelectedTable !== "cta_pls_summary"
    || ["CTA-PLS tetrad summary", "CTA-PLS tetrads", "CTA-PLS requirements and exclusions"].some((title) => treeItems.filter((item) => item === title).length !== 1)
    || summary.rows !== 1
    || JSON.stringify(summary.headers) !== JSON.stringify(["Construct", "Indicators", "Four-indicator subsets", "Tetrads", "Maximum absolute tetrad"])
    || summary.values[0]?.[0] !== "Predictor" || summary.values[0]?.[1] !== "x1, x2, x3, x4"
    || summary.values[0]?.[2] !== "1" || summary.values[0]?.[3] !== "3" || !Number.isFinite(Number(summary.values[0]?.[4]))
    || tetrads.rows !== 3
    || JSON.stringify(tetrads.headers) !== JSON.stringify(["Construct", "Indicator A", "Indicator B", "Indicator C", "Indicator D", "Pairing", "Tetrad", "Absolute tetrad"])
    || tetrads.values.some((row) => row[0] !== "Predictor"
      || JSON.stringify(row.slice(1, 5)) !== JSON.stringify(["x1", "x2", "x3", "x4"]))
    || new Set(tetrads.values.map((row) => row[5])).size !== 3
    || scope.rows !== 6 || scopeValues["Method version"] !== ctaPlsMethodVersion
    || scopeValues["Covariance convention"] !== ctaPlsCovarianceVersion
    || scopeValues["Complete cases"] !== "120" || scopeValues["Omitted cases"] !== "0"
    || scopeValues.Interpretation !== ctaPlsScopeNote
    || scopeValues["Excluded inference"] !== "Bootstrap, permutation, asymptotic, and vanishing-tetrad decisions"
    || runDetails.properties.Method !== "Confirmatory Tetrad Analysis"
    || runDetails.properties["Method version"] !== ctaPlsProvenanceMethodVersion
    || runDetails.properties.Weighting !== "path" || runDetails.properties.Preprocessing !== "standardized"
    || Object.hasOwn(runDetails.properties, "Recorded seed") || runDetails.logEntries < 1
    || !evidence.checks.ctaPlsResult.allTetradsFinite || !evidence.checks.ctaPlsResult.absoluteColumnsMatch
    || !evidence.checks.ctaPlsResult.noInferentialDecision) {
    throw new Error(`The completed CTA-PLS Results did not expose the exact three descriptive tetrads and scope: ${JSON.stringify(evidence.checks.ctaPlsResult)}`);
  }
  await openResultTable("CTA-PLS tetrad summary");
  await capture(ctaPlsCaptureName(205, "results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  const ctaPlsTableTitles = ["CTA-PLS tetrad summary", "CTA-PLS tetrads", "CTA-PLS requirements and exclusions"];
  const expectedSheets = [...ctaPlsTableTitles.map((title) => title.slice(0, 31).trimEnd()), "Run provenance"];
  const expectedSharedStrings = [
    ...ctaPlsTableTitles,
    "Run provenance",
    "Confirmatory Tetrad Analysis",
    ctaPlsMethodVersion,
    ctaPlsCovarianceVersion,
    ctaPlsResultWarning,
    "Predictor",
  ];
  evidence.checks.ctaPlsExport = {
    selectedRunId: runId,
    xlsxEnabled: await xlsxExport.isEnabled(),
    buttonCount: await exportDialog.locator(".nd-export-list button").count(),
    nativeXlsx: null,
  };
  if (!evidence.checks.ctaPlsExport.xlsxEnabled || evidence.checks.ctaPlsExport.buttonCount < 5) {
    throw new Error(`The CTA-PLS result did not expose its table exports: ${JSON.stringify(evidence.checks.ctaPlsExport)}`);
  }
  const saveHelper = startWindowsNativeSaveExportHelper({
    targetPath: exportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets,
    expectedSharedStrings,
  });
  let helperCompleted = false;
  try {
    const ready = await saveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native CTA-PLS XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await saveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`Native CTA-PLS XLSX verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(exportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(exportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(exportTargetPath);
    evidence.checks.ctaPlsExport.nativeXlsx = {
      attempted: true,
      targetPath: exportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    if (!file.isFile() || file.size <= 0 || evidence.checks.ctaPlsExport.nativeXlsx.appFeedback !== expectedFeedback
      || !expectedSheets.every((sheet) => workbookSheets.includes(sheet))) {
      throw new Error(`The genuine CTA-PLS XLSX did not contain every descriptive tetrad and provenance sheet: ${JSON.stringify(evidence.checks.ctaPlsExport)}`);
    }
  } finally {
    if (!helperCompleted) saveHelper.stop();
  }
  await capture(ctaPlsCaptureName(206, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedArchive = await inspectSavedCtaPlsArchive(ctaPlsProjectPath, runId);
  await reloadToLauncher();
  await openRecentProject(ctaPlsProjectName, ctaPlsProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /Confirmatory Tetrad Analysis/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened CTA-PLS result had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedSummary = await readTable("CTA-PLS tetrad summary");
  const reopenedTetrads = await readTable("CTA-PLS tetrads");
  const reopenedScope = await readTable("CTA-PLS requirements and exclusions");
  evidence.checks.ctaPlsSaveReopen = {
    expectedRunId: runId,
    selectedRunId: reopenedRunId,
    sameRunRestored: reopenedRunId === runId && await page.locator(".nd-run-select select").inputValue() === runId,
    sameVisibleValuesRestored: JSON.stringify({ reopenedSummary, reopenedTetrads, reopenedScope })
      === JSON.stringify({ reopenedSummary: summary, reopenedTetrads: tetrads, reopenedScope: scope }),
    archive: savedArchive,
  };
  if (!evidence.checks.ctaPlsSaveReopen.sameRunRestored || !evidence.checks.ctaPlsSaveReopen.sameVisibleValuesRestored) {
    throw new Error(`The exact CTA-PLS run and visible tetrads did not survive save/reload/reopen: ${JSON.stringify(evidence.checks.ctaPlsSaveReopen)}`);
  }

  const responsive = [];
  for (const viewport of ctaPlsViewports) {
    await setActualTauriClientViewport(viewport, `CTA-PLS responsive result ${viewport.id}`);
    await openResultTable("CTA-PLS tetrad summary");
    const metrics = await page.evaluate(() => {
      const app = document.querySelector(".nd-app");
      const selected = document.querySelector('.nd-result-tree [role="treeitem"][aria-selected="true"]');
      return {
        innerWidth,
        innerHeight,
        documentNoHorizontalOverflow: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
        appNoHorizontalOverflow: Boolean(app && app.scrollWidth <= app.clientWidth + 1),
        selectedTableId: selected?.getAttribute("data-result-tree-item-id") ?? null,
        summaryRows: document.querySelectorAll(".nd-result-table tbody tr").length,
        treeVisible: Boolean(document.querySelector('.nd-result-tree[role="tree"]')),
      };
    });
    const passed = metrics.innerWidth === viewport.width && metrics.innerHeight === viewport.height
      && metrics.documentNoHorizontalOverflow && metrics.appNoHorizontalOverflow
      && metrics.selectedTableId === "cta_pls_summary" && metrics.summaryRows === 1 && metrics.treeVisible;
    responsive.push({ ...viewport, passed, metrics });
    await capture(ctaPlsCaptureName(207, "reopened", viewport.id));
  }
  await setActualTauriClientViewport({ width: 1440, height: 900 }, "CTA-PLS responsive result restoration");
  evidence.checks.ctaPlsResponsiveViewports = {
    passed: responsive.length === ctaPlsViewports.length && responsive.every((row) => row.passed),
    exactViewports: responsive,
  };
  if (!evidence.checks.ctaPlsResponsiveViewports.passed) {
    throw new Error(`CTA-PLS result responsiveness failed at a required viewport: ${JSON.stringify(evidence.checks.ctaPlsResponsiveViewports)}`);
  }

  const packagedInternalOrigins = new Set([packagedTauriOrigin, packagedTauriIpcOrigin]);
  const externalRequests = observedBrowserRequests.filter((request) => request.origin
    && request.origin !== "null" && !packagedInternalOrigins.has(request.origin));
  evidence.checks.ctaPlsBrowserNetwork = {
    passed: observedBrowserRequests.length > 0 && externalRequests.length === 0,
    observedRequestCount: observedBrowserRequests.length,
    externalRequestCount: externalRequests.length,
    origins: [...new Set(observedBrowserRequests.map((request) => request.origin))].sort(),
    externalRequests,
  };
  if (!evidence.checks.ctaPlsBrowserNetwork.passed) {
    throw new Error(`CTA-PLS packaged browser traffic crossed the offline boundary: ${JSON.stringify(evidence.checks.ctaPlsBrowserNetwork)}`);
  }
}

async function runFocusedPcaAcceptance() {
  if (!requestedPcaNativeExportPath) {
    throw new Error("QUICKPLS_PCA_NATIVE_EXPORT_PATH is required for focused packaged PCA acceptance; enabled-button assertions do not replace a genuine native XLSX Save and workbook-content check.");
  }
  const pcaExportTargetPath = await validateRequestedNativeExportPath(
    requestedPcaNativeExportPath,
    "QUICKPLS_PCA_NATIVE_EXPORT_PATH",
  );

  await seedRecentProject({
    name: pcaProjectName,
    path: pcaProjectPath,
    openedAt: "2026-08-12T00:00:00.000Z",
  });
  await reloadToLauncher();
  await openRecentProject(pcaProjectName, pcaProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const status = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const columns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  const initialArchive = await inspectInitialPcaArchive(pcaProjectPath);
  evidence.checks.pcaFixture = {
    projectPath: pcaProjectPath,
    status,
    cases: status.includes("140 cases") ? 140 : null,
    columns,
    initialArchive,
  };
  const expectedColumns = ["#", "x", "m", "w", "y", "z", "bin_y", "g1", "g2", "g3", "h1", "h2"];
  if (evidence.checks.pcaFixture.cases !== 140 || JSON.stringify(columns) !== JSON.stringify(expectedColumns)
    || initialArchive.models !== 0 || initialArchive.activeModelId !== null) {
    throw new Error(`The focused PCA fixture did not expose the canonical 140-row data-only project: ${JSON.stringify(evidence.checks.pcaFixture)}`);
  }
  await capture(pcaCaptureName(110, "fixture-data"));

  const calculation = await openAnalysisFromDataToolbar();
  const listbox = calculation.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const options = listbox.getByRole("option");
  const pcaOption = calculation.locator("#nd-calculation-method-pca");
  await pcaOption.click();
  await calculation.locator(".nd-pca-settings").waitFor({ state: "visible", timeout: 10_000 });

  const variableLabels = calculation.locator(".nd-pca-variable-list label");
  const availableVariables = await variableLabels.evaluateAll((labels) => labels.map((label) => (
    label.querySelector("span")?.textContent?.replace(/\s+/g, " ").trim() ?? ""
  )));
  await calculation.getByRole("button", { name: "Clear", exact: true }).click();
  for (const variable of pcaVariables) {
    const label = variableLabels.filter({ hasText: new RegExp(`^\\s*${variable}\\s*$`) });
    if (await label.count() !== 1) throw new Error(`PCA variable ${variable} was not exposed as exactly one checkbox.`);
    await label.getByRole("checkbox").check();
  }
  const selectedVariables = await variableLabels.evaluateAll((labels) => labels.filter((label) => (
    label.querySelector('input[type="checkbox"]')?.checked
  )).map((label) => label.querySelector("span")?.textContent?.replace(/\s+/g, " ").trim() ?? ""));
  const retention = calculation.locator("#nd-calculation-pca-rule");
  await retention.selectOption("variance_threshold");
  const threshold = calculation.locator("#nd-calculation-pca-threshold");
  await threshold.fill(String(pcaVarianceThreshold * 100));
  const noteValue = async (label) => compactVisibleText(await calculation.locator(".nd-setting-note")
    .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
  const start = calculation.getByRole("button", { name: "Start principal component analysis", exact: true });
  const blockerText = compactVisibleText(await calculation.locator('.nd-blocker[role="alert"]').textContent().catch(() => ""));
  const canonicalCatalogKinds = await canonicalNativeAnalysisCatalogKinds();
  const catalogOptionIds = await options.evaluateAll((elements) => elements.map((element) => element.id));
  const catalogKinds = catalogOptionIds.map((id) => (
    id.startsWith("nd-calculation-method-") ? id.slice("nd-calculation-method-".length) : null
  ));
  evidence.checks.pcaDialog = {
    catalogCount: await options.count(),
    canonicalCatalogKinds,
    catalogKinds,
    selectedMethod: compactVisibleText(await listbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    category: compactVisibleText(await calculation.locator("#nd-calculation-category-standalone").textContent()),
    availableVariables,
    selectedVariables,
    componentRule: await retention.inputValue(),
    varianceThresholdPercent: await threshold.inputValue(),
    calculationBasis: await noteValue("Calculation basis"),
    variableData: await noteValue("Variable data"),
    validatedScope: await noteValue("Validated scope"),
    maxIterationsCount: await calculation.locator("#nd-calculation-max-iterations").count(),
    toleranceCount: await calculation.locator("#nd-calculation-tolerance").count(),
    unsupportedControls: await calculation.locator([
      "#nd-calculation-weighting", "#nd-calculation-preprocessing", "#nd-calculation-bootstrap-samples",
      "#nd-calculation-permutations", "#nd-calculation-nca-permutations", "#nd-calculation-seed",
      "#nd-calculation-workers", "#nd-calculation-case-weight", "#nd-calculation-group-column",
    ].join(", ")).count(),
    blockers: await calculation.locator(".nd-blocker li").allTextContents(),
    blockerText,
    noModelBlocker: !/construct|structural path|editable model|active model/i.test(blockerText),
    startEnabled: await start.isEnabled(),
  };
  if (evidence.checks.pcaDialog.catalogCount !== expectedOptionLabels.length
    || JSON.stringify(catalogKinds) !== JSON.stringify(canonicalCatalogKinds)
    || evidence.checks.pcaDialog.selectedMethod !== "Principal Component Analysis"
    || evidence.checks.pcaDialog.category !== "Standalone analysis"
    || JSON.stringify(availableVariables) !== JSON.stringify(expectedColumns.slice(1))
    || JSON.stringify(selectedVariables) !== JSON.stringify(pcaVariables)
    || evidence.checks.pcaDialog.componentRule !== "variance_threshold"
    || evidence.checks.pcaDialog.varianceThresholdPercent !== "95"
    || evidence.checks.pcaDialog.calculationBasis !== "Correlation matrix (fixed)"
    || evidence.checks.pcaDialog.variableData !== "Standardized numeric values (fixed)"
    || !/Correlation-matrix PCA of 2 to 50 selected numeric variables/i.test(evidence.checks.pcaDialog.validatedScope)
    || evidence.checks.pcaDialog.maxIterationsCount !== 0 || evidence.checks.pcaDialog.toleranceCount !== 0
    || evidence.checks.pcaDialog.unsupportedControls !== 0 || evidence.checks.pcaDialog.blockers.length !== 0
    || !evidence.checks.pcaDialog.noModelBlocker || !evidence.checks.pcaDialog.startEnabled) {
    throw new Error(`The focused PCA dialog did not match the exact model-free variance-threshold contract: ${JSON.stringify(evidence.checks.pcaDialog)}`);
  }
  await capture(pcaCaptureName(111, "dialog"));

  const activeCapture = captureActiveCalculation(calculation, pcaCaptureName(112, "running"), "standalone PCA")
    .then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, completedBeforeCapture: true, detail: error instanceof Error ? error.message : String(error) }));
  await start.click();
  await waitForSurface("results", 120_000);
  evidence.checks.pcaProgress = await activeCapture;
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const selectedRun = page.locator(".nd-run-select select option:checked");
  await selectedRun.waitFor({ state: "attached", timeout: 30_000 });
  const pcaRunId = await page.locator(".nd-run-select select").inputValue();
  if (!pcaRunId) throw new Error("The completed PCA run had no identifier.");
  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const readTable = async (title) => {
    const rows = await openResultTable(title);
    return {
      rows,
      headers: (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText),
      values: await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => Array.from(
        row.querySelectorAll("th,td"),
        (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "",
      ))),
    };
  };
  const summary = await readTable("Component summary");
  const loadings = await readTable("Component loadings and weights");
  const scope = await readTable("Calculation scope");
  const scopeValues = Object.fromEntries(scope.values.map((row) => [row[0], row[1]]));
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const resultsText = [summary, loadings, scope].flatMap((table) => [table.headers, ...table.values]).flat().join(" ");
  const runDetails = await inspectCurrentRunDetails();
  const editDataCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Data$/i });
  const editModelCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Model$/i });
  evidence.checks.pcaResult = {
    runId: pcaRunId,
    runLabel: compactVisibleText(await selectedRun.textContent()),
    initialSelectedTable,
    treeItems,
    summary,
    loadings,
    scope,
    scopeValues,
    runDetails,
    editDataCommand: { count: await editDataCommand.count(), enabled: await editDataCommand.isEnabled().catch(() => false) },
    editModelCommand: { count: await editModelCommand.count() },
    noPlaceholder: !/\bN\/A\b/i.test(resultsText),
    noSemResultGroups: !/Model estimates|Quality criteria|Mediation|Moderation|Prediction/i.test(treeItems.join(" ")),
  };
  const cumulativePercentages = summary.values.map((row) => Number(row[3]?.replace("%", "")));
  const loadingIdentities = new Set(loadings.values.map((row) => `${row[0]}\u0000${row[1]}`));
  if (initialSelectedTable !== "pca_component_summary"
    || JSON.stringify(treeItems) !== JSON.stringify(["Principal components", "Component summary", "Component loadings and weights", "Calculation scope"])
    || summary.rows !== 4
    || JSON.stringify(summary.headers) !== JSON.stringify(["Component", "Eigenvalue", "Explained variance", "Cumulative variance"])
    || JSON.stringify(summary.values.map((row) => row[0])) !== JSON.stringify(["PC1", "PC2", "PC3", "PC4"])
    || !cumulativePercentages.every(Number.isFinite) || cumulativePercentages[2] >= 95 || cumulativePercentages[3] < 95
    || loadings.rows !== 20 || loadingIdentities.size !== 20
    || JSON.stringify([...new Set(loadings.values.map((row) => row[0]))]) !== JSON.stringify(pcaVariables)
    || scope.rows !== 11 || scopeValues.Variables !== "5" || scopeValues["Analyzed observations"] !== "140"
    || scopeValues["Retention rule"] !== "Cumulative variance threshold" || scopeValues["Retained components"] !== "4"
    || scopeValues["Stored component scores"] !== "560" || scopeValues.Rotation !== "None"
    || scopeValues["Validated scope"] !== pcaValidatedScope || scopeValues["Method version"] !== pcaMethodVersion
    || runDetails.properties.Method !== "Principal Component Analysis" || runDetails.properties["Method version"] !== pcaMethodVersion
    || runDetails.properties.Variables !== "5" || runDetails.properties["Retention rule"] !== "Cumulative variance threshold"
    || runDetails.properties["Retained components"] !== "4" || runDetails.properties.Observations !== "140"
    || Object.hasOwn(runDetails.properties, "Weighting") || Object.hasOwn(runDetails.properties, "Preprocessing")
    || evidence.checks.pcaResult.editDataCommand.count !== 1 || !evidence.checks.pcaResult.editDataCommand.enabled
    || evidence.checks.pcaResult.editModelCommand.count !== 0
    || !evidence.checks.pcaResult.noPlaceholder || !evidence.checks.pcaResult.noSemResultGroups) {
    throw new Error(`The completed PCA result did not expose the exact four-component crossing result, tables, and model-free return boundary: ${JSON.stringify(evidence.checks.pcaResult)}`);
  }
  await openResultTable("Component summary");
  await capture(pcaCaptureName(113, "results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  const pcaTableTitles = ["Component summary", "Component loadings and weights", "Calculation scope", "Component scores"];
  const expectedPcaSheets = [...pcaTableTitles, "Run provenance"];
  const expectedExportFormats = ["CSV tables", "HTML report", "Reviewer pack", "XLSX workbook", "Print / PDF"];
  const exportButtonTexts = await exportDialog.locator(".nd-export-list button").evaluateAll((buttons) => buttons.map((button) => (
    button.innerText.replace(/\s+/g, " ").trim()
  )));
  evidence.checks.pcaExport = {
    xlsxEnabled: await xlsxExport.isEnabled(),
    formats: expectedExportFormats,
    buttonTexts: exportButtonTexts,
    everyFormatPresentOnceWhenReadable: exportButtonTexts.length === 0 ? null : expectedExportFormats.every((label) => (
      exportButtonTexts.filter((text) => text.startsWith(label)).length === 1
    )),
    buttonCount: await exportDialog.locator(".nd-export-list button").count(),
    modelDiagramFormats: await exportDialog.getByRole("button", { name: /diagram|svg/i }).count(),
    nativeXlsx: null,
  };
  if (!evidence.checks.pcaExport.xlsxEnabled || evidence.checks.pcaExport.buttonCount !== 5
    || evidence.checks.pcaExport.everyFormatPresentOnceWhenReadable === false
    || evidence.checks.pcaExport.modelDiagramFormats !== 0) {
    throw new Error(`The model-free PCA result did not expose exactly five table-only export formats: ${JSON.stringify(evidence.checks.pcaExport)}`);
  }
  const nativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: pcaExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: expectedPcaSheets,
    expectedSharedStrings: [...pcaTableTitles, "Run provenance", "Cumulative variance threshold", "Validated scope", pcaValidatedScope, pcaMethodVersion],
  });
  let helperCompleted = false;
  try {
    const ready = await nativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native PCA XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await nativeSaveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`Native PCA XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(pcaExportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(pcaExportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(pcaExportTargetPath);
    evidence.checks.pcaExport.nativeXlsx = {
      attempted: true,
      targetPath: pcaExportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    if (!file.isFile() || file.size <= 0 || evidence.checks.pcaExport.nativeXlsx.appFeedback !== expectedFeedback
      || !expectedPcaSheets.every((sheet) => workbookSheets.includes(sheet))) {
      throw new Error(`The genuine PCA XLSX did not contain every result, full score, and provenance sheet: ${JSON.stringify(evidence.checks.pcaExport)}`);
    }
  } finally {
    if (!helperCompleted) nativeSaveHelper.stop();
  }
  await capture(pcaCaptureName(114, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedArchive = await inspectSavedPcaArchive(pcaProjectPath, pcaRunId);
  await reloadToLauncher();
  await openRecentProject(pcaProjectName, pcaProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /Principal Component Analysis/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened PCA result had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedSummaryRows = await openResultTable("Component summary");
  const reopenedLoadingRows = await openResultTable("Component loadings and weights");
  const reopenedScopeRows = await openResultTable("Calculation scope");
  evidence.checks.pcaSaveReopen = {
    expectedRunId: pcaRunId,
    selectedRunId: reopenedRunId,
    sameRunRestored: reopenedRunId === pcaRunId,
    summaryRows: reopenedSummaryRows,
    loadingRows: reopenedLoadingRows,
    scopeRows: reopenedScopeRows,
    archive: savedArchive,
  };
  if (!evidence.checks.pcaSaveReopen.sameRunRestored || reopenedSummaryRows !== 4
    || reopenedLoadingRows !== 20 || reopenedScopeRows !== 11) {
    throw new Error(`The exact model-free PCA run did not survive explicit save/reload/reopen: ${JSON.stringify(evidence.checks.pcaSaveReopen)}`);
  }
  await openResultTable("Component summary");
  await capture(pcaCaptureName(115, "reopened"));
}

async function runFocusedHigherOrderAcceptance() {
  if (!requestedHocNativeExportPath) {
    throw new Error("QUICKPLS_HOC_NATIVE_EXPORT_PATH is required for focused packaged HOC acceptance; enabled-button assertions do not replace a genuine native XLSX Save and workbook-content check.");
  }
  const hocExportTargetPath = await validateRequestedNativeExportPath(
    requestedHocNativeExportPath,
    "QUICKPLS_HOC_NATIVE_EXPORT_PATH",
  );

  await seedRecentProject({
    name: hocProjectName,
    path: hocProjectPath,
    openedAt: "2026-08-12T00:00:00.000Z",
  });
  await reloadToLauncher();
  await openRecentProject(hocProjectName, hocProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const status = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const columns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  evidence.checks.hocFixture = {
    projectPath: hocProjectPath,
    status,
    cases: status.includes("120 cases") ? 120 : null,
    columns,
  };
  if (evidence.checks.hocFixture.cases !== 120 || JSON.stringify(columns) !== JSON.stringify(["#", "x1", "z1", "y1"])) {
    throw new Error(`The focused HOC fixture did not expose the expected 120-row x1/z1/y1 data: ${JSON.stringify(evidence.checks.hocFixture)}`);
  }
  await capture(hocCaptureName(100, "fixture-data"));

  evidence.checks.hocInitialModel = await createInitialEditableModel(hocProjectName, hocModelName);
  const authored = await buildThreeConstructHigherOrderModel();
  const hocCommand = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Higher-Order Construct/i });
  if (await hocCommand.count() !== 1 || !await hocCommand.isEnabled()) {
    throw new Error("The three eligible measurement-only constructs did not enable exactly one Higher-Order Construct toolbar command.");
  }
  await hocCommand.click();
  const hocDialog = page.locator('.nd-dialog-higher-order[role="dialog"]');
  await hocDialog.waitFor({ state: "visible", timeout: 10_000 });
  await hocDialog.getByLabel("Name", { exact: true }).fill("Organizational Capability");
  await hocDialog.getByLabel("Short name", { exact: true }).fill("OC");
  const capabilityCheckbox = hocDialog.getByRole("checkbox", { name: /Capability/ });
  const resourcesCheckbox = hocDialog.getByRole("checkbox", { name: /Resources/ });
  const performanceCheckbox = hocDialog.getByRole("checkbox", { name: /Performance/ });
  await capabilityCheckbox.check();
  await capabilityCheckbox.uncheck();
  if (await capabilityCheckbox.isChecked()) throw new Error("The HOC Capability checkbox did not preserve a user deselection.");
  await capabilityCheckbox.check();
  await resourcesCheckbox.check();
  await performanceCheckbox.check();
  if (!await performanceCheckbox.isChecked()) throw new Error("The HOC Performance checkbox did not preserve a user selection.");
  await performanceCheckbox.uncheck();
  if (await performanceCheckbox.isChecked()) throw new Error("The HOC Performance checkbox did not preserve the final unselected outcome state.");
  const createHoc = hocDialog.getByRole("button", { name: "Create higher-order construct", exact: true });
  const hocDialogText = compactVisibleText(await hocDialog.textContent());
  evidence.checks.hocDialog = {
    componentCount: await hocDialog.locator('input[type="checkbox"]').count(),
    capabilitySelected: await capabilityCheckbox.isChecked(),
    resourcesSelected: await resourcesCheckbox.isChecked(),
    performanceSelected: await performanceCheckbox.isChecked(),
    method: compactVisibleText(await hocDialog.locator(".nd-hoc-summary").textContent()),
    scope: compactVisibleText(await hocDialog.locator(".nd-dialog-note").textContent()),
    inferenceControls: await hocDialog.locator('[id*="bootstrap"], [id*="permutation"], [id*="confidence"], input[type="number"]').count(),
    createEnabled: await createHoc.isEnabled(),
    noBroaderClaim: !/repeated indicators|hybrid|bootstrapping available|permutation available/i.test(hocDialogText),
  };
  if (evidence.checks.hocDialog.componentCount !== 3
    || !evidence.checks.hocDialog.capabilitySelected || !evidence.checks.hocDialog.resourcesSelected || evidence.checks.hocDialog.performanceSelected
    || !/Reflective.+reflective disjoint two-stage/i.test(evidence.checks.hocDialog.method)
    || !/Stage 1.*component scores.*Stage 2.*generated HOC indicators/i.test(evidence.checks.hocDialog.method)
    || !/one HOC-to-outcome relationship/i.test(evidence.checks.hocDialog.scope)
    || !/no other structural path/i.test(evidence.checks.hocDialog.scope)
    || !/HOC bootstrapping and permutation inference remain unavailable/i.test(evidence.checks.hocDialog.scope)
    || evidence.checks.hocDialog.inferenceControls !== 0 || !evidence.checks.hocDialog.createEnabled || !evidence.checks.hocDialog.noBroaderClaim) {
    throw new Error(`The focused HOC dialog did not match the bounded disjoint two-stage point-estimate contract: ${JSON.stringify(evidence.checks.hocDialog)}`);
  }
  await capture(hocCaptureName(101, "dialog"));
  await createHoc.click();
  await hocDialog.waitFor({ state: "hidden", timeout: 10_000 });

  const allNodes = page.locator(".react-flow__node-latent");
  if (await allNodes.count() !== 4) throw new Error("HOC creation did not add exactly one generated higher-order construct.");
  const hocNode = allNodes.filter({ hasText: "Organizational Capability" });
  if (await hocNode.count() !== 1) throw new Error("The generated HOC was not visible under its authored name.");
  const hocId = await hocNode.getAttribute("data-id");
  if (!hocId) throw new Error("The generated HOC did not expose an immutable identifier.");
  const hocProperties = await modelInspector().locator(".nd-property-list").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const invalidArchiveBefore = await inspectMediationArchiveRunState(hocProjectPath);
  const invalidCalculation = await openCalculationFromToolbar();
  const invalidListbox = invalidCalculation.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await invalidListbox.getByRole("option", { name: /PLS-SEM Algorithm/i }).click();
  const invalidStart = invalidCalculation.getByRole("button", { name: "Start calculation", exact: true });
  const invalidBlockers = (await invalidCalculation.locator(".nd-blocker li").allTextContents()).map(compactVisibleText);
  evidence.checks.hocInvalidSetup = {
    attempted: true,
    selectedMethod: compactVisibleText(await invalidListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    startEnabled: await invalidStart.isEnabled(),
    blockers: invalidBlockers,
    missingHocPathBlocked: invalidBlockers.some((row) => /Connect the higher-order construct to at least one measured outcome/i.test(row)),
    archiveBefore: invalidArchiveBefore,
    archiveAfter: null,
    archiveStateUnchanged: false,
    resultCreated: false,
  };
  await capture(hocCaptureName("101a", "invalid-setup"));
  await invalidCalculation.getByRole("button", { name: "Close", exact: true }).click();
  await invalidCalculation.waitFor({ state: "hidden", timeout: 10_000 });
  evidence.checks.hocInvalidSetup.archiveAfter = await inspectMediationArchiveRunState(hocProjectPath);
  evidence.checks.hocInvalidSetup.archiveStateUnchanged = JSON.stringify(evidence.checks.hocInvalidSetup.archiveAfter)
    === JSON.stringify(evidence.checks.hocInvalidSetup.archiveBefore);
  evidence.checks.hocInvalidSetup.resultCreated = evidence.checks.hocInvalidSetup.archiveAfter.resultCount
    !== evidence.checks.hocInvalidSetup.archiveBefore.resultCount;
  if (evidence.checks.hocInvalidSetup.selectedMethod !== "PLS-SEM Algorithm"
    || evidence.checks.hocInvalidSetup.startEnabled || !evidence.checks.hocInvalidSetup.missingHocPathBlocked
    || !evidence.checks.hocInvalidSetup.archiveStateUnchanged || evidence.checks.hocInvalidSetup.resultCreated) {
    throw new Error(`The path-free HOC setup did not fail closed without creating a recipe/run/result: ${JSON.stringify(evidence.checks.hocInvalidSetup)}`);
  }
  await createStructuralPath(allNodes, 3, 2, 1);
  const generatedPath = await structuralPaths().first().getAttribute("data-id");
  evidence.checks.hocModel = {
    constructs: await allNodes.count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    paths: await structuralPaths().count(),
    hocId,
    componentIds: [authored.ids.capability, authored.ids.resources],
    outcomeId: authored.ids.performance,
    generatedPath,
    properties: hocProperties,
  };
  if (evidence.checks.hocModel.constructs !== 4 || evidence.checks.hocModel.assignedIndicators !== 3 || evidence.checks.hocModel.paths !== 1
    || hocProperties.Type !== "Reflective–reflective higher-order construct" || hocProperties.Method !== "Disjoint two-stage"
    || !hocProperties.Components?.includes("Capability") || !hocProperties.Components?.includes("Resources")
    || hocProperties.Indicators !== "Generated component scores") {
    throw new Error(`The visible HOC model did not retain its exact indicator-free semantics and single outgoing relationship: ${JSON.stringify(evidence.checks.hocModel)}`);
  }
  await capture(hocCaptureName(102, "model"));

  const calculation = await openCalculationFromToolbar();
  await calculation.getByRole("listbox", { name: "Available calculation methods", exact: true })
    .getByRole("option", { name: /PLS-SEM Algorithm/i }).click();
  const start = calculation.getByRole("button", { name: "Start calculation", exact: true });
  evidence.checks.hocCalculation = {
    selectedMethod: compactVisibleText(await calculation.getByRole("option", { selected: true }).locator("strong").textContent()),
    blockers: await calculation.locator(".nd-blocker li").allTextContents(),
    startEnabled: await start.isEnabled(),
    bootstrapControls: await calculation.locator("#nd-calculation-bootstrap-samples").count(),
    permutationControls: await calculation.locator("#nd-calculation-permutations").count(),
  };
  if (evidence.checks.hocCalculation.selectedMethod !== "PLS-SEM Algorithm"
    || evidence.checks.hocCalculation.blockers.length !== 0 || !evidence.checks.hocCalculation.startEnabled
    || evidence.checks.hocCalculation.bootstrapControls !== 0 || evidence.checks.hocCalculation.permutationControls !== 0) {
    throw new Error(`The bounded HOC model was not runnable only through the ordinary PLS-SEM Algorithm workflow: ${JSON.stringify(evidence.checks.hocCalculation)}`);
  }
  const activeCapture = captureActiveCalculation(calculation, hocCaptureName(103, "running"), "two-stage HOC")
    .then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, completedBeforeCapture: true, detail: error instanceof Error ? error.message : String(error) }));
  await start.click();
  await waitForSurface("results", 120_000);
  evidence.checks.hocProgress = await activeCapture;

  const selectedRun = page.locator(".nd-run-select select option:checked");
  await selectedRun.waitFor({ state: "attached", timeout: 30_000 });
  const hocRunId = await page.locator(".nd-run-select select").inputValue();
  if (!hocRunId) throw new Error("The completed HOC run had no identifier.");
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  evidence.checks.hocCompletedRunSaved = { runId: hocRunId, savedBeforeResultInspection: true };
  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const componentRows = await openResultTable("Higher-order component relationships");
  const componentHeaders = (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText);
  const componentValues = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")));
  const structuralRows = await openResultTable("Higher-order structural paths");
  const structuralValues = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")));
  const scopeRows = await openResultTable("Higher-order calculation scope");
  const scopeValues = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")));
  const outerRows = await openResultTable("Outer loadings");
  const outerText = compactVisibleText(await page.locator(".nd-result-table tbody").textContent());
  const resultText = compactVisibleText(await page.locator(".nd-results-workspace").textContent());
  const runDetails = await inspectCurrentRunDetails();
  evidence.checks.hocResult = {
    runId: hocRunId,
    runLabel: compactVisibleText(await selectedRun.textContent()),
    initialSelectedTable,
    component: { rows: componentRows, headers: componentHeaders, values: componentValues },
    structural: { rows: structuralRows, values: structuralValues },
    scope: { rows: scopeRows, values: scopeValues },
    ordinaryOuterRows: outerRows,
    ordinaryOuterText: outerText,
    runDetails,
    noTechnicalIds: !/__qpls_hoc_/i.test(resultText),
    noPlaceholder: !/\bN\/A\b/i.test(resultText),
  };
  if (initialSelectedTable !== "hoc_component_relationships"
    || componentRows !== 2
    || JSON.stringify(componentHeaders) !== JSON.stringify(["Higher-order construct", "Lower-order component", "Method", "Loading", "Weight"])
    || !componentValues.every((row) => row.includes("Organizational Capability") && row.includes("Disjoint two-stage"))
    || !componentValues.some((row) => row.includes("Capability")) || !componentValues.some((row) => row.includes("Resources"))
    || structuralRows !== 1 || !structuralValues[0]?.some((value) => value.includes("Organizational Capability")) || !structuralValues[0]?.some((value) => value.includes("Performance"))
    || scopeRows !== 1 || !scopeValues[0]?.includes("Reflective-reflective disjoint two-stage")
    || !scopeValues[0]?.some((value) => value.includes("Point estimates only in the bounded native workflow"))
    || !scopeValues[0]?.some((value) => value.includes("HOC bootstrapping and permutation inference remain unavailable"))
    || outerRows !== 3 || /Organizational Capability|__qpls_hoc_/i.test(outerText)
    || runDetails.properties.Method !== "PLS-SEM Algorithm" || !String(runDetails.properties["Method version"]).includes("pls_pm_v1")
    || !evidence.checks.hocResult.noTechnicalIds || !evidence.checks.hocResult.noPlaceholder) {
    throw new Error(`The completed HOC result did not expose the exact component/path/scope tables without technical pseudo-indicators: ${JSON.stringify(evidence.checks.hocResult)}`);
  }
  await openResultTable("Higher-order component relationships");
  await capture(hocCaptureName(104, "results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  const hocTableTitles = [
    "Higher-order component relationships",
    "Higher-order structural paths",
    "Higher-order calculation scope",
  ];
  const expectedHocSheets = [...hocTableTitles.map((title) => title.slice(0, 31)), "Run provenance"];
  evidence.checks.hocExport = {
    xlsxEnabled: await xlsxExport.isEnabled(),
    formats: (await exportDialog.locator(".nd-export-list button strong").allTextContents()).map(compactVisibleText),
    nativeXlsx: null,
  };
  if (!evidence.checks.hocExport.xlsxEnabled) throw new Error("The HOC result did not enable native XLSX export.");
  const nativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: hocExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: expectedHocSheets,
    expectedSharedStrings: [
      ...hocTableTitles,
      "Run provenance",
      "Disjoint two-stage",
      "Point estimates only in the bounded native workflow; HOC bootstrapping and permutation inference remain unavailable",
    ],
  });
  let helperCompleted = false;
  try {
    const ready = await nativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native HOC XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await nativeSaveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`Native HOC XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(hocExportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(hocExportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(hocExportTargetPath);
    evidence.checks.hocExport.nativeXlsx = {
      attempted: true,
      targetPath: hocExportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    if (!file.isFile() || file.size <= 0 || evidence.checks.hocExport.nativeXlsx.appFeedback !== expectedFeedback
      || !expectedHocSheets.every((sheet) => workbookSheets.includes(sheet))) {
      throw new Error(`The genuine HOC XLSX did not contain every bounded result and provenance sheet: ${JSON.stringify(evidence.checks.hocExport)}`);
    }
  } finally {
    if (!helperCompleted) nativeSaveHelper.stop();
  }
  await capture(hocCaptureName(105, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedArchive = await inspectSavedHigherOrderArchive(hocProjectPath, hocRunId, {
    hocId,
    componentIds: [authored.ids.capability, authored.ids.resources],
    outcomeId: authored.ids.performance,
  });
  await reloadToLauncher();
  await openRecentProject(hocProjectName, hocProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /PLS-SEM Algorithm/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened HOC result had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedComponentRows = await openResultTable("Higher-order component relationships");
  const reopenedStructuralRows = await openResultTable("Higher-order structural paths");
  const reopenedScopeRows = await openResultTable("Higher-order calculation scope");
  evidence.checks.hocSaveReopen = {
    expectedRunId: hocRunId,
    selectedRunId: reopenedRunId,
    sameRunRestored: reopenedRunId === hocRunId,
    componentRows: reopenedComponentRows,
    structuralRows: reopenedStructuralRows,
    scopeRows: reopenedScopeRows,
    archive: savedArchive,
  };
  if (!evidence.checks.hocSaveReopen.sameRunRestored || reopenedComponentRows !== 2 || reopenedStructuralRows !== 1 || reopenedScopeRows !== 1) {
    throw new Error(`The exact HOC model and completed result did not survive explicit save/reload/reopen: ${JSON.stringify(evidence.checks.hocSaveReopen)}`);
  }
  await openResultTable("Higher-order component relationships");
  await capture(hocCaptureName(106, "reopened"));
  await captureActualTauriViewportMatrix({
    checkName: "hocPackagedViewports",
    methodSlug: "higher_order_v1",
    methodVersion: "pls_pm_v1",
    methodEvidenceCheck: "hocResult",
    expectedRunId: hocRunId,
    expectedRunLabel: "PLS-SEM Algorithm",
    expectedTableId: "hoc_component_relationships",
    capturePrefix: "hoc",
    captureSequence: "106",
  });
  const hocInternalOrigins = new Set([packagedTauriOrigin, packagedTauriIpcOrigin]);
  const hocExternalRequests = observedBrowserRequests.filter((request) => request.origin
    && request.origin !== "null" && !hocInternalOrigins.has(request.origin));
  evidence.checks.hocFunctionalOffline = {
    passed: observedBrowserRequests.length > 0 && hocExternalRequests.length === 0,
    analyticalWorkflowRequiresInternet: false,
    strictZeroProcessEgressClaimed: false,
    platformBackgroundEgressOutsidePageRequestScope: true,
    observedRequestCount: observedBrowserRequests.length,
    externalRequestCount: hocExternalRequests.length,
    origins: [...new Set(observedBrowserRequests.map((request) => request.origin))].sort(),
    externalRequests: hocExternalRequests,
  };
  if (!evidence.checks.hocFunctionalOffline.passed) {
    throw new Error(`HOC packaged browser/app workflow crossed its functional-offline request boundary: ${JSON.stringify(evidence.checks.hocFunctionalOffline)}`);
  }
}

async function runFocusedPredictionAcceptance() {
  if (!requestedPredictionNativeExportPath) {
    throw new Error("QUICKPLS_PREDICTION_NATIVE_EXPORT_PATH is required for focused packaged PLSpredict/CVPAT acceptance; enabled-button assertions do not replace a genuine native XLSX Save and workbook-content check.");
  }
  const predictionExportTargetPath = await validateRequestedNativeExportPath(
    requestedPredictionNativeExportPath,
    "QUICKPLS_PREDICTION_NATIVE_EXPORT_PATH",
  );

  await seedDisposableRecentProject();
  await reloadToLauncher();
  await openDisposableRecentProject();
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const statusText = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const dataColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  evidence.checks.predictionFixture = {
    projectPath: disposableProjectPath,
    status: statusText,
    cases: statusText.includes(`${predictionObservations} cases`) ? predictionObservations : null,
    columns: dataColumns,
  };
  if (evidence.checks.predictionFixture.cases !== predictionObservations
    || !["x1", "x2", "y1", "y2"].every((column) => dataColumns.includes(column))) {
    throw new Error(`The focused prediction fixture did not expose the expected ${predictionObservations}-row x/y indicator data: ${JSON.stringify(evidence.checks.predictionFixture)}`);
  }
  await capture(predictionCaptureName(90, "fixture-data"));

  evidence.checks.predictionInitialModel = await createInitialEditableModel(disposableProjectName, disposableModelName);
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const predictionInvalidArchiveBefore = await inspectMediationArchiveRunState(disposableProjectPath);
  const predictionInvalidDialog = await openCalculationFromToolbar();
  const predictionInvalidListbox = predictionInvalidDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await predictionInvalidListbox.getByRole("option", { name: /PLSpredict \/ CVPAT/i }).click();
  const predictionInvalidStart = predictionInvalidDialog.getByRole("button", { name: "Start prediction", exact: true });
  const predictionInvalidBlockers = (await predictionInvalidDialog.locator(".nd-blocker li").allTextContents()).map(compactVisibleText);
  evidence.checks.predictionInvalidSetup = {
    attempted: true,
    selectedMethod: compactVisibleText(await predictionInvalidListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    startEnabled: await predictionInvalidStart.isEnabled(),
    blockers: predictionInvalidBlockers,
    emptyModelBlocker: predictionInvalidBlockers.some((row) => /requires at least one structural path/i.test(row)),
    archiveBefore: predictionInvalidArchiveBefore,
    archiveAfter: null,
    archiveStateUnchanged: false,
    resultCreated: false,
  };
  await capture(predictionCaptureName("90a", "invalid-setup"));
  await predictionInvalidDialog.getByRole("button", { name: "Close", exact: true }).click();
  await predictionInvalidDialog.waitFor({ state: "hidden", timeout: 10_000 });
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const predictionInvalidArchiveAfter = await inspectMediationArchiveRunState(disposableProjectPath);
  evidence.checks.predictionInvalidSetup.archiveAfter = predictionInvalidArchiveAfter;
  evidence.checks.predictionInvalidSetup.archiveStateUnchanged = JSON.stringify(predictionInvalidArchiveAfter) === JSON.stringify(predictionInvalidArchiveBefore);
  evidence.checks.predictionInvalidSetup.resultCreated = predictionInvalidArchiveAfter.resultCount > predictionInvalidArchiveBefore.resultCount;
  if (evidence.checks.predictionInvalidSetup.selectedMethod !== "PLSpredict / CVPAT"
    || evidence.checks.predictionInvalidSetup.startEnabled
    || !evidence.checks.predictionInvalidSetup.emptyModelBlocker
    || !evidence.checks.predictionInvalidSetup.archiveStateUnchanged
    || evidence.checks.predictionInvalidSetup.resultCreated) {
    throw new Error(`The empty-model packaged PLSpredict/CVPAT setup did not fail closed without creating calculation state: ${JSON.stringify(evidence.checks.predictionInvalidSetup)}`);
  }
  await buildTwoConstructModel();
  evidence.checks.predictionModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    paths: await structuralPaths().count(),
  };
  if (JSON.stringify(evidence.checks.predictionModel) !== JSON.stringify({ constructs: 2, assignedIndicators: 4, paths: 1 })) {
    throw new Error(`The focused prediction model was not the expected two-construct, four-indicator, one-path model: ${JSON.stringify(evidence.checks.predictionModel)}`);
  }
  await capture(predictionCaptureName(91, "model"));

  const dialog = await openCalculationFromToolbar();
  const methodListbox = dialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const methodLabels = (await methodListbox.getByRole("option").locator("strong").allTextContents()).map(compactVisibleText);
  await methodListbox.getByRole("option", { name: /PLSpredict \/ CVPAT/i }).click();
  const start = dialog.getByRole("button", { name: "Start prediction", exact: true });
  const selectedPanel = dialog.locator("#nd-calculation-panel");
  const dialogContract = {
    methods: methodLabels,
    selectedMethod: compactVisibleText(await methodListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    plan: compactVisibleText(await dialog.locator("#nd-calculation-prediction-plan strong").textContent()),
    targets: compactVisibleText(await dialog.locator("#nd-calculation-prediction-targets strong").textContent()),
    benchmarks: compactVisibleText(await dialog.locator("#nd-calculation-prediction-benchmarks strong").textContent()),
    cvpat: compactVisibleText(await dialog.locator("#nd-calculation-prediction-cvpat strong").textContent()),
    seed: Number(await dialog.locator("#nd-calculation-seed").inputValue()),
    confidenceControls: await dialog.locator("#nd-calculation-confidence").count(),
    workerControls: await dialog.locator("#nd-calculation-workers").count(),
    bootstrapControls: await dialog.locator("#nd-calculation-bootstrap-samples").count(),
    permutationControls: await dialog.locator("#nd-calculation-permutations").count(),
    startEnabled: await start.isEnabled(),
    blockers: await dialog.locator(".nd-blocker li").allTextContents(),
    noLegacyClaim: !/5\s*(?:folds?|×|x)\s*3|construct-score-only|bounded paired-loss/i.test(compactVisibleText(await selectedPanel.textContent())),
  };
  evidence.checks.predictionV2Dialog = dialogContract;
  if (JSON.stringify(dialogContract.methods) !== JSON.stringify(expectedOptionLabels)
    || dialogContract.selectedMethod !== "PLSpredict / CVPAT"
    || dialogContract.plan !== "Complete cases; seeded balanced 10-fold × 10-repeat cross-validation; deterministic modulo-4 holdout retained as a secondary check"
    || !/endogenous indicators.*primary.*construct-score metrics.*supplementary/i.test(dialogContract.targets)
    || !/indicator average \(IA\).*linear model \(LM(?:, where estimable)?\)/i.test(dialogContract.benchmarks)
    || !/single fitted model versus IA\/LM benchmarks/i.test(dialogContract.cvpat)
    || !/one-sided test, 95% confidence/i.test(dialogContract.cvpat)
    || !/not a comparison of saved models/i.test(dialogContract.cvpat)
    || !Number.isInteger(dialogContract.seed) || dialogContract.seed < 0
    || dialogContract.confidenceControls !== 0 || dialogContract.workerControls !== 0
    || dialogContract.bootstrapControls !== 0 || dialogContract.permutationControls !== 0
    || !dialogContract.startEnabled || dialogContract.blockers.length !== 0 || !dialogContract.noLegacyClaim) {
    throw new Error(`The focused prediction dialog did not match the exact indicator-level seeded 10x10 PLSpredict/CVPAT contract: ${JSON.stringify(dialogContract)}`);
  }
  await capture(predictionCaptureName(92, "dialog"));

  const predictionCancellationArchiveBefore = await inspectMediationArchiveRunState(disposableProjectPath);
  const predictionTerminalStatePromise = page.waitForFunction(() => {
    if (document.querySelector('.nd-app[data-surface="results"]')) return "results_surface";
    const calculationDialog = document.querySelector('.nd-dialog-calculation[role="dialog"]');
    if (!calculationDialog) return "dialog_detached";
    if (calculationDialog.querySelector('.nd-run-progress.cancelled[aria-busy="false"]')) return "cancelled";
    if (calculationDialog.querySelector(".nd-run-progress.completed")) return "completed";
    return null;
  }, null, { timeout: 60_000 });
  const predictionCancellationRequestPromise = page.waitForFunction(() => {
    if (document.querySelector('.nd-app[data-surface="results"]')) return { outcome: "results_surface" };
    const calculationDialog = document.querySelector('.nd-dialog-calculation[role="dialog"]');
    if (!calculationDialog) return null;
    if (calculationDialog.querySelector(".nd-run-progress.completed")) return { outcome: "completed" };
    const progress = calculationDialog.querySelector(
      '.nd-run-progress[aria-busy="true"]:is(.queued,.validating,.running)',
    );
    if (!progress) return null;
    const message = progress.querySelector("p")?.textContent?.trim() ?? "";
    if (message !== "Native engine accepted the calculation job.") return null;
    const cancelButtons = Array.from(calculationDialog.querySelectorAll("button"))
      .filter((button) => button.textContent?.trim() === "Cancel calculation" && !button.disabled);
    if (cancelButtons.length !== 1) return null;
    const snapshot = {
      outcome: "cancel_requested",
      ariaBusy: progress.getAttribute("aria-busy"),
      status: [...progress.classList].find((className) => ["queued", "validating", "running"].includes(className)) ?? null,
      phase: progress.querySelector("strong")?.textContent?.trim() ?? "",
      message,
      progressValue: progress.querySelector("progress")?.getAttribute("value") ?? null,
      progressMax: progress.querySelector("progress")?.getAttribute("max") ?? null,
      logEntries: progress.querySelectorAll("ol li").length,
      cancelButtonCount: cancelButtons.length,
    };
    cancelButtons[0].click();
    return snapshot;
  }, null, { timeout: 5_000 });
  await start.click();
  const predictionCancellationRequestHandle = await predictionCancellationRequestPromise;
  const predictionCancellationSnapshot = await predictionCancellationRequestHandle.jsonValue();
  await predictionCancellationRequestHandle.dispose();
  if (predictionCancellationSnapshot?.outcome !== "cancel_requested") {
    throw new Error(`completion_won_race: PLSpredict / CVPAT reached ${predictionCancellationSnapshot?.outcome ?? "unknown"} before post-acceptance cancellation could be requested.`);
  }
  const predictionCancellationCapturePromise = capture(predictionCaptureName("92a", "cancellation-running"))
    .then(() => ({ captured: true, detail: null }))
    .catch((error) => ({ captured: false, detail: error instanceof Error ? error.message : String(error) }));
  const predictionTerminalStateHandle = await predictionTerminalStatePromise;
  const predictionTerminalOutcome = await predictionTerminalStateHandle.jsonValue();
  await predictionTerminalStateHandle.dispose();
  const predictionCancellationCapture = await predictionCancellationCapturePromise;
  const predictionCancellationActive = {
    captured: predictionCancellationCapture.captured,
    ...predictionCancellationSnapshot,
    screenshot: predictionCancellationCapture,
  };
  if (predictionTerminalOutcome !== "cancelled") {
    throw new Error(`completion_won_race: PLSpredict / CVPAT reached ${predictionTerminalOutcome} before terminal cancellation became authoritative.`);
  }
  const predictionCancelled = dialog.locator(".nd-run-progress.cancelled");
  await predictionCancelled.waitFor({ state: "visible", timeout: 60_000 });
  const predictionCancelledMessage = compactVisibleText(await predictionCancelled.textContent());
  const predictionPartialResults = await page.locator(".nd-run-select select option").count();
  const predictionCancellationArchiveAfter = await inspectMediationArchiveRunState(disposableProjectPath);
  const predictionCancellationArchiveUnchanged = JSON.stringify(predictionCancellationArchiveAfter)
    === JSON.stringify(predictionCancellationArchiveBefore);
  const predictionRetrySettings = {
    selectedMethod: compactVisibleText(await methodListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    plan: compactVisibleText(await dialog.locator("#nd-calculation-prediction-plan strong").textContent()),
    seed: Number(await dialog.locator("#nd-calculation-seed").inputValue()),
  };
  const predictionRetry = dialog.getByRole("button", { name: "Retry prediction", exact: true });
  await predictionRetry.waitFor({ state: "visible", timeout: 15_000 });
  const predictionRetryEnabled = await predictionRetry.isEnabled();
  if (!predictionCancellationActive.captured || predictionPartialResults !== 0
    || !predictionCancellationArchiveUnchanged
    || predictionRetrySettings.selectedMethod !== dialogContract.selectedMethod
    || predictionRetrySettings.plan !== dialogContract.plan
    || predictionRetrySettings.seed !== dialogContract.seed
    || !predictionRetryEnabled) {
    throw new Error(`PLSpredict/CVPAT cancellation did not terminate cleanly with its exact setup available for retry: ${JSON.stringify({ predictionCancellationActive, predictionPartialResults, predictionCancellationArchiveBefore, predictionCancellationArchiveAfter, predictionRetrySettings, predictionRetryEnabled })}`);
  }
  await capture(predictionCaptureName("92b", "cancelled"));
  const activeCapture = captureActiveCalculation(
    dialog,
    predictionCaptureName(93, "running"),
    "PLSpredict / CVPAT retry",
    { allowTerminalTransitionAfterCapture: true },
  );
  await predictionRetry.click();
  evidence.checks.predictionV2Progress = await activeCapture;
  await waitForSurface("results", 180_000);
  const selectedRunOption = page.locator(".nd-run-select select option:checked").filter({ hasText: /PLSpredict \/ CVPAT/i });
  await selectedRunOption.waitFor({ state: "attached", timeout: 180_000 });
  const predictionRunId = await page.locator(".nd-run-select select").inputValue();
  if (!predictionRunId) throw new Error("The completed focused prediction run had no run identifier.");
  const predictionRunLabel = compactVisibleText(await selectedRunOption.textContent());
  evidence.checks.predictionV2Progress.completedRunProof = {
    runId: predictionRunId,
    runLabel: predictionRunLabel,
    matched: /PLSpredict \/ CVPAT/i.test(predictionRunLabel),
  };
  if (!evidence.checks.predictionV2Progress.completedRunProof.matched) {
    throw new Error(`The focused prediction lifecycle did not resolve to its matching completed run: ${JSON.stringify(evidence.checks.predictionV2Progress)}`);
  }
  evidence.checks.predictionCancellationRetry = {
    passed: predictionCancellationActive.captured
      && predictionPartialResults === 0
      && predictionCancellationArchiveUnchanged
      && predictionRetryEnabled
      && predictionRetrySettings.selectedMethod === dialogContract.selectedMethod
      && predictionRetrySettings.plan === dialogContract.plan
      && predictionRetrySettings.seed === dialogContract.seed
      && Boolean(predictionRunId)
      && /PLSpredict \/ CVPAT/i.test(predictionRunLabel),
    cancelledMethod: dialogContract.selectedMethod,
    cancelledSettings: { plan: dialogContract.plan, seed: dialogContract.seed },
    activeLifecycleCaptured: predictionCancellationActive.captured,
    activeLifecycle: predictionCancellationActive,
    terminalMessage: predictionCancelledMessage,
    noPartialVisibleResult: predictionPartialResults === 0,
    noPartialCommittedResult: predictionCancellationArchiveUnchanged,
    archiveStateUnchanged: predictionCancellationArchiveUnchanged,
    archiveBefore: predictionCancellationArchiveBefore,
    archiveAfter: predictionCancellationArchiveAfter,
    retrySettings: predictionRetrySettings,
    retryEnabled: predictionRetryEnabled,
    completedRetryRunId: predictionRunId,
    completedRetryRunLabel: predictionRunLabel,
  };
  if (!evidence.checks.predictionCancellationRetry.passed) {
    throw new Error(`PLSpredict/CVPAT cancellation/retry identity linkage failed: ${JSON.stringify(evidence.checks.predictionCancellationRetry)}`);
  }

  const predictionTreeIds = await page.locator('.nd-result-tree [role="treeitem"][data-result-tree-item-id]').evaluateAll((items) => items.map((item) => item.getAttribute("data-result-tree-item-id")));
  const requiredPredictionTreeIds = [
    "plspredict_indicator_summary",
    "cvpat_benchmark_assessment",
    "plspredict_validation_plan",
    "plspredict_construct_summary",
    "plspredict_holdout_indicator_summary",
    "plspredict_holdout_construct_summary",
    "plspredict_holdout_split",
  ];
  const initialPredictionSelection = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const indicatorRowCount = await openResultTable("Indicator prediction summary (10-fold × 10-repeat)");
  const indicatorHeaders = (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText);
  const indicatorRows = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")));
  await capture(predictionCaptureName(94, "indicator-results"));

  const cvpatRowCount = await openResultTable("CVPAT benchmark assessment (single model)");
  const cvpatHeaders = (await page.locator(".nd-result-table thead th").allTextContents()).map(compactVisibleText);
  const cvpatRows = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")));
  const validationPlanRows = await openResultTable("Prediction validation plan");
  const validationPlanText = compactVisibleText(await page.locator(".nd-result-table tbody").textContent());
  const resultProperties = await page.locator('aside[aria-label="Result properties"] .nd-property-list').evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  const runDetails = await inspectCurrentRunDetails();
  const allPredictionText = `${indicatorRows.flat().join(" ")} ${cvpatRows.flat().join(" ")} ${validationPlanText}`;
  evidence.checks.predictionV2Result = {
    runId: predictionRunId,
    runLabel: compactVisibleText(await selectedRunOption.textContent()),
    treeIds: predictionTreeIds,
    initialSelection: initialPredictionSelection,
    indicator: { rows: indicatorRowCount, headers: indicatorHeaders, values: indicatorRows },
    cvpat: { rows: cvpatRowCount, headers: cvpatHeaders, values: cvpatRows },
    validationPlan: { rows: validationPlanRows, text: validationPlanText },
    properties: resultProperties,
    runDetails,
    noPlaceholderOrLegacyClaim: !/\bN\/A\b|construct-score-only|5\s*(?:folds?|×|x)\s*3|comparison of saved models/i.test(allPredictionText),
  };
  if (!requiredPredictionTreeIds.every((id) => predictionTreeIds.includes(id))
    || initialPredictionSelection !== "plspredict_indicator_summary"
    || indicatorRowCount !== 2 || !indicatorHeaders.includes("Indicator") || !indicatorHeaders.includes("Q²_predict")
    || !indicatorHeaders.includes("PLS-SEM RMSE") || !indicatorHeaders.includes("IA RMSE") || !indicatorHeaders.includes("LM RMSE")
    || !indicatorRows.every((row) => row.includes("Construct 2") && row.some((cell) => /^y[12]$/.test(cell)))
    || cvpatRowCount !== 2 || !cvpatHeaders.includes("Mean loss difference (PLS-SEM − benchmark)")
    || !cvpatRows.some((row) => row.includes("Indicator average (IA)"))
    || !cvpatRows.some((row) => row.includes("Linear model (LM)"))
    || !cvpatRows.every((row) => row.includes("PLS-SEM loss < benchmark") && row.includes("95%"))
    || validationPlanRows !== 1 || !validationPlanText.includes("Primary repeated cross-validation")
    || !validationPlanText.includes("10") || !/sha256:[0-9a-f]{64}/.test(validationPlanText)
    || resultProperties.Method !== "PLSpredict / CVPAT" || resultProperties["Complete cases"] !== String(predictionObservations)
    || resultProperties.Folds !== "10" || resultProperties.Repeats !== "10"
    || resultProperties.CVPAT !== "One-sided, 95% confidence"
    || runDetails.properties.Method !== "PLSpredict / CVPAT"
    || runDetails.properties["Method version"] !== predictionProvenanceMethodVersion
    || runDetails.properties["Recorded seed"] !== String(dialogContract.seed)
    || runDetails.logEntries < 1 || !evidence.checks.predictionV2Result.noPlaceholderOrLegacyClaim) {
    throw new Error(`The focused packaged prediction result did not expose the exact indicator, two-row CVPAT, validation-plan, provenance, and no-placeholder contract: ${JSON.stringify(evidence.checks.predictionV2Result)}`);
  }
  await openResultTable("CVPAT benchmark assessment (single model)");
  await capture(predictionCaptureName(95, "cvpat-results"));

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  await xlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const exportLabels = (await exportDialog.locator(".nd-export-list button strong").allTextContents()).map(compactVisibleText);
  const expectedPredictionSheets = [
    "Indicator prediction summary (1",
    "CVPAT benchmark assessment (sin",
    "Prediction validation plan",
    "Supplementary construct-score p",
    "Secondary holdout indicator sum",
    "Secondary holdout construct-sco",
    "Secondary deterministic holdout",
    "Run provenance",
  ];
  evidence.checks.predictionV2Export = {
    formats: exportLabels,
    xlsxEnabled: await xlsxExport.isEnabled(),
    nativeXlsx: null,
  };
  if (!["CSV tables", "HTML report", "Reviewer pack", "XLSX workbook", "Model diagram", "Print / PDF"].every((label) => exportLabels.includes(label))
    || !evidence.checks.predictionV2Export.xlsxEnabled) {
    throw new Error(`The focused prediction result did not expose all table, workbook, diagram, and print exports: ${JSON.stringify(evidence.checks.predictionV2Export)}`);
  }
  const nativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: predictionExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: expectedPredictionSheets,
    expectedSharedStrings: [
      "Indicator prediction summary (10-fold × 10-repeat)",
      "CVPAT benchmark assessment (single model)",
      "Prediction validation plan",
      "Run provenance",
      "Indicator average (IA)",
      "Linear model (LM)",
      "Assignment digest",
      predictionProvenanceMethodVersion,
      predictionRepeatedMethodVersion,
      predictionCvpatMethodVersion,
    ],
  });
  let helperCompleted = false;
  try {
    const ready = await nativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native prediction XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await nativeSaveHelper.completed;
    helperCompleted = true;
    if (!completion.passed) throw new Error(`Native prediction XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(predictionExportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(predictionExportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(predictionExportTargetPath);
    evidence.checks.predictionV2Export.nativeXlsx = {
      attempted: true,
      targetPath: predictionExportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    if (!file.isFile() || file.size <= 0 || evidence.checks.predictionV2Export.nativeXlsx.appFeedback !== expectedFeedback
      || !expectedPredictionSheets.every((sheet) => workbookSheets.includes(sheet))) {
      throw new Error(`The genuine prediction XLSX did not contain every current v2 result and provenance sheet: ${JSON.stringify(evidence.checks.predictionV2Export)}`);
    }
  } finally {
    if (!helperCompleted) nativeSaveHelper.stop();
  }
  await capture(predictionCaptureName(96, "export"));
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedArchive = await inspectSavedPredictionArchive(disposableProjectPath, predictionRunId);
  await reloadToLauncher();
  await openDisposableRecentProject();
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /PLSpredict \/ CVPAT/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened prediction option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedIndicatorRows = await openResultTable("Indicator prediction summary (10-fold × 10-repeat)");
  const reopenedCvpatRows = await openResultTable("CVPAT benchmark assessment (single model)");
  evidence.checks.predictionV2SaveReopen = {
    expectedRunId: predictionRunId,
    selectedRunId: reopenedRunId,
    sameRunRestored: reopenedRunId === predictionRunId,
    indicatorRows: reopenedIndicatorRows,
    cvpatRows: reopenedCvpatRows,
    archive: savedArchive,
  };
  if (!evidence.checks.predictionV2SaveReopen.sameRunRestored || reopenedIndicatorRows !== 2 || reopenedCvpatRows !== 2) {
    throw new Error(`The focused v2 prediction run did not survive explicit save/reload/reopen: ${JSON.stringify(evidence.checks.predictionV2SaveReopen)}`);
  }
  await capture(predictionCaptureName(97, "reopened"));
  await captureActualTauriViewportMatrix({
    checkName: "predictionPackagedViewports",
    methodSlug: "plspredict_cvpat_v2",
    methodVersion: predictionMethodVersion,
    methodEvidenceCheck: "predictionV2Result",
    expectedRunId: predictionRunId,
    expectedRunLabel: "PLSpredict / CVPAT",
    expectedTableId: "cvpat_benchmark_assessment",
    capturePrefix: "prediction",
    captureSequence: "97",
  });
}

async function runFocusedPlsSampleSizePowerAcceptance() {
  if (!requestedPlsSampleSizePowerNativeExportPath) {
    throw new Error("QUICKPLS_PLS_SAMPLE_SIZE_POWER_NATIVE_EXPORT_PATH is required; an enabled XLSX button is not packaged prospective-power evidence.");
  }
  const exportTargetPath = await validateRequestedNativeExportPath(
    requestedPlsSampleSizePowerNativeExportPath,
    "QUICKPLS_PLS_SAMPLE_SIZE_POWER_NATIVE_EXPORT_PATH",
  );
  await seedRecentProject({
    name: plsSampleSizePowerProjectName,
    path: plsSampleSizePowerProjectPath,
    openedAt: "2026-08-18T00:00:00.000Z",
  });
  await reloadToLauncher();
  await openRecentProject(plsSampleSizePowerProjectName, plsSampleSizePowerProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const fixtureStatus = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const fixtureColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  evidence.checks.plsSampleSizePowerFixture = {
    projectPath: plsSampleSizePowerProjectPath,
    status: fixtureStatus,
    columns: fixtureColumns,
  };
  if (!/240 cases/i.test(fixtureStatus) || !["x1", "x2", "x3", "y1", "y2", "y3"].every((name) => fixtureColumns.includes(name))) {
    throw new Error(`The focused prospective-power anchor fixture is incomplete: ${JSON.stringify(evidence.checks.plsSampleSizePowerFixture)}`);
  }

  evidence.checks.plsSampleSizePowerInitialModel = await createInitialEditableModel(
    plsSampleSizePowerProjectName,
    plsSampleSizePowerModelName,
  );
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const invalidArchiveBefore = await inspectMediationArchiveRunState(plsSampleSizePowerProjectPath);
  const invalidDialog = await openCalculationFromToolbar();
  const invalidListbox = invalidDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await invalidDialog.locator("#nd-calculation-method-pls_sample_size_power").click();
  const invalidStart = invalidDialog.getByRole("button", { name: "Start prospective power analysis", exact: true });
  const invalidBlockers = (await invalidDialog.locator(".nd-blocker li").allTextContents()).map(compactVisibleText).filter(Boolean);
  const invalidSelected = invalidListbox.getByRole("option", { selected: true });
  evidence.checks.plsSampleSizePowerInvalidSetup = {
    attempted: true,
    selectedMethod: compactVisibleText(await invalidSelected.locator("strong").textContent()),
    startEnabled: await invalidStart.isEnabled(),
    blockers: invalidBlockers,
    modelShapeBlocker: invalidBlockers.some((row) => /exactly two|reflective constructs|structural path|predictor|outcome/i.test(row)),
    archiveBefore: invalidArchiveBefore,
    archiveAfter: null,
    archiveStateUnchanged: false,
    resultCreated: false,
  };
  await capture("200-tauri-native-pls-sample-size-power-invalid-setup-1440x900.png");
  await invalidDialog.getByRole("button", { name: "Close", exact: true }).click();
  await invalidDialog.waitFor({ state: "hidden", timeout: 10_000 });
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const invalidArchiveAfter = await inspectMediationArchiveRunState(plsSampleSizePowerProjectPath);
  Object.assign(evidence.checks.plsSampleSizePowerInvalidSetup, {
    archiveAfter: invalidArchiveAfter,
    archiveStateUnchanged: JSON.stringify(invalidArchiveAfter) === JSON.stringify(invalidArchiveBefore),
    resultCreated: invalidArchiveAfter.resultCount > invalidArchiveBefore.resultCount,
  });
  if (evidence.checks.plsSampleSizePowerInvalidSetup.selectedMethod !== "PLS-SEM Sample Size and Power"
    || evidence.checks.plsSampleSizePowerInvalidSetup.startEnabled
    || !evidence.checks.plsSampleSizePowerInvalidSetup.modelShapeBlocker
    || !evidence.checks.plsSampleSizePowerInvalidSetup.archiveStateUnchanged
    || evidence.checks.plsSampleSizePowerInvalidSetup.resultCreated) {
    throw new Error(`Prospective-power invalid setup did not fail closed: ${JSON.stringify(evidence.checks.plsSampleSizePowerInvalidSetup)}`);
  }

  evidence.checks.plsSampleSizePowerModel = await buildProspectivePlsPowerModel();
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const configure = async (workers) => {
    const dialog = await openCalculationFromToolbar();
    const listbox = dialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
    await dialog.locator("#nd-calculation-method-pls_sample_size_power").click();
    await dialog.locator("#nd-calculation-pls-power-scenario").fill("packaged_two_construct_path_v2");
    await dialog.locator("#nd-calculation-pls-power-predictor").selectOption({ label: "Predictor (3 indicators)" });
    await dialog.locator("#nd-calculation-pls-power-outcome").selectOption({ label: "Outcome (3 indicators)" });
    await dialog.locator("#nd-calculation-pls-power-predictor-loadings").fill("0.80,0.80,0.80");
    await dialog.locator("#nd-calculation-pls-power-outcome-loadings").fill("0.80,0.80,0.80");
    await dialog.locator("#nd-calculation-pls-power-path").fill("0.30");
    await dialog.locator("#nd-calculation-pls-power-grid").fill(plsSampleSizePowerGrid);
    await dialog.locator("#nd-calculation-pls-power-alpha").fill("0.05");
    await dialog.locator("#nd-calculation-pls-power-target").fill("0.80");
    await dialog.locator("#nd-calculation-pls-power-confidence").fill("95");
    await dialog.locator("#nd-calculation-pls-power-mc").fill(String(plsSampleSizePowerMonteCarloReplicates));
    await dialog.locator("#nd-calculation-pls-power-bootstrap").fill(String(plsSampleSizePowerBootstrapReplicates));
    await dialog.locator("#nd-calculation-seed").fill(String(plsSampleSizePowerSeed));
    await dialog.locator("#nd-calculation-pls-power-workers").fill(String(workers));
    const start = dialog.getByRole("button", { name: "Start prospective power analysis", exact: true });
    const selected = listbox.getByRole("option", { selected: true });
    const selectedText = compactVisibleText(await selected.textContent());
    const contract = {
      selectedMethod: compactVisibleText(await selected.locator("strong").textContent()),
      selectedText,
      scenario: await dialog.locator("#nd-calculation-pls-power-scenario").inputValue(),
      predictor: compactVisibleText(await dialog.locator("#nd-calculation-pls-power-predictor option:checked").textContent()),
      outcome: compactVisibleText(await dialog.locator("#nd-calculation-pls-power-outcome option:checked").textContent()),
      grid: await dialog.locator("#nd-calculation-pls-power-grid").inputValue(),
      monteCarloReplicates: await dialog.locator("#nd-calculation-pls-power-mc").inputValue(),
      bootstrapReplicates: await dialog.locator("#nd-calculation-pls-power-bootstrap").inputValue(),
      seed: await dialog.locator("#nd-calculation-seed").inputValue(),
      workers: await dialog.locator("#nd-calculation-pls-power-workers").inputValue(),
      scope: compactVisibleText(await dialog.locator("#nd-calculation-pls-power-scope").textContent()),
      workload: compactVisibleText(await dialog.locator("#nd-calculation-pls-power-workload").textContent()),
      blockers: (await dialog.locator(".nd-blocker li").allTextContents()).map(compactVisibleText),
      startEnabled: await start.isEnabled(),
      standardSurface: !/Experimental|Limited scope/i.test(selectedText),
    };
    if (contract.selectedMethod !== "PLS-SEM Sample Size and Power" || contract.scenario !== "packaged_two_construct_path_v2"
      || contract.predictor !== "Predictor (3 indicators)" || contract.outcome !== "Outcome (3 indicators)"
      || contract.grid !== plsSampleSizePowerGrid
      || contract.monteCarloReplicates !== String(plsSampleSizePowerMonteCarloReplicates)
      || contract.bootstrapReplicates !== String(plsSampleSizePowerBootstrapReplicates)
      || contract.seed !== String(plsSampleSizePowerSeed) || contract.workers !== String(workers)
      || !/exactly two ordinary reflective constructs/i.test(contract.scope)
      || !/20,000 PLS fits/i.test(contract.workload)
      || !contract.standardSurface || contract.blockers.length !== 0 || !contract.startEnabled) {
      throw new Error(`Prospective-power setup differs from its exact bounded v2 contract: ${JSON.stringify(contract)}`);
    }
    return { dialog, start, contract };
  };

  const cancellationArchiveBefore = await inspectMediationArchiveRunState(plsSampleSizePowerProjectPath);
  const cancellationSetup = await configure(1);
  const activeCapture = captureActiveCalculation(
    cancellationSetup.dialog,
    "201-tauri-native-pls-sample-size-power-cancellation-running-1440x900.png",
    "Prospective PLS-power cancellation",
  ).then((state) => ({ captured: true, ...state }))
    .catch((error) => ({ captured: false, detail: error instanceof Error ? error.message : String(error) }));
  await cancellationSetup.start.click();
  const activeState = await activeCapture;
  const cancel = cancellationSetup.dialog.getByRole("button", { name: "Cancel calculation", exact: true });
  await cancel.waitFor({ state: "visible", timeout: 15_000 });
  await cancel.click();
  const cancelled = cancellationSetup.dialog.locator(".nd-run-progress.cancelled");
  await cancelled.waitFor({ state: "visible", timeout: 60_000 });
  const cancelledMessage = compactVisibleText(await cancelled.textContent());
  const partialVisibleResults = await page.locator(".nd-run-select select option").count();
  await cancellationSetup.dialog.getByRole("button", { name: "Close", exact: true }).click();
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const cancellationArchiveAfter = await inspectMediationArchiveRunState(plsSampleSizePowerProjectPath);
  evidence.checks.plsSampleSizePowerCancellation = {
    passed: activeState.captured && partialVisibleResults === 0
      && JSON.stringify(cancellationArchiveAfter) === JSON.stringify(cancellationArchiveBefore),
    activeState,
    cancelledMessage,
    cancelledSettings: cancellationSetup.contract,
    noPartialVisibleResult: partialVisibleResults === 0,
    noPartialCommittedResult: cancellationArchiveAfter.resultCount === cancellationArchiveBefore.resultCount,
    archiveStateUnchanged: JSON.stringify(cancellationArchiveAfter) === JSON.stringify(cancellationArchiveBefore),
    archiveBefore: cancellationArchiveBefore,
    archiveAfter: cancellationArchiveAfter,
  };
  if (!evidence.checks.plsSampleSizePowerCancellation.passed) {
    throw new Error(`Cancelled prospective power appended partial evidence: ${JSON.stringify(evidence.checks.plsSampleSizePowerCancellation)}`);
  }

  const retry = await configure(plsSampleSizePowerWorkers);
  evidence.checks.plsSampleSizePowerDialog = retry.contract;
  const progressCapture = captureActiveCalculation(
    retry.dialog,
    "202-tauri-native-pls-sample-size-power-running-1440x900.png",
    "Prospective PLS sample-size and power",
    { allowTerminalTransitionAfterCapture: true },
  );
  await retry.start.click();
  evidence.checks.plsSampleSizePowerProgress = { captured: true, ...await progressCapture };
  await waitForSurface("results", 600_000);
  const selectedRun = page.locator(".nd-run-select select option:checked");
  await selectedRun.waitFor({ state: "attached", timeout: 600_000 });
  const runId = await page.locator(".nd-run-select select").inputValue();
  const runLabel = compactVisibleText(await selectedRun.textContent());
  evidence.checks.plsSampleSizePowerProgress.completedRunProof = { matched: Boolean(runId), runId };
  Object.assign(evidence.checks.plsSampleSizePowerCancellation, {
    retryEnabled: retry.contract.startEnabled,
    retrySettings: retry.contract,
    completedRetryRunId: runId,
  });
  evidence.checks.plsSampleSizePowerCancellation.passed = evidence.checks.plsSampleSizePowerCancellation.passed
    && retry.contract.grid === cancellationSetup.contract.grid
    && retry.contract.monteCarloReplicates === cancellationSetup.contract.monteCarloReplicates
    && retry.contract.bootstrapReplicates === cancellationSetup.contract.bootstrapReplicates
    && retry.contract.seed === cancellationSetup.contract.seed && Boolean(runId);

  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const powerRows = await openResultTable("Power by sample size");
  const powerValues = await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => (
    Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
  )));
  const tailRows = await openResultTable("Bootstrap tail accounting");
  const tailValues = await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => (
    Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
  )));
  const failureRows = await openResultTable("Simulation failures");
  const failureValues = await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => (
    Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
  )));
  const assumptionRows = await openResultTable("Design assumptions");
  const provenanceRows = await openResultTable("Run provenance");
  const provenanceText = compactVisibleText(await page.locator(".nd-result-table tbody").textContent());
  const outerAccountingCloses = powerValues.every((row) => Number(row[1]) === plsSampleSizePowerMonteCarloReplicates
    && Number(row[2]) === Number(row[1]) && Number(row[3]) + Number(row[4]) === Number(row[1]));
  const tailAccountingCloses = tailValues.every((row) => Number(row[2]) === Number(row[1]) * plsSampleSizePowerBootstrapReplicates
    && Number(row[3]) + Number(row[4]) === Number(row[2])
    && Number(row[5]) >= 0 && Number(row[5]) <= Number(row[3]));
  const typedFailures = failureValues.every((row) => row.length === 5 && row.every(Boolean));
  evidence.checks.plsSampleSizePowerResult = {
    runId,
    runLabel,
    methodVersion: plsSampleSizePowerMethodVersion,
    initialSelectedTable,
    treeItems,
    powerRows,
    powerValues,
    tailRows,
    tailValues,
    failureRows,
    typedFailures,
    assumptionRows,
    provenanceRows,
    provenanceMethodVersionPresent: provenanceText.includes(plsSampleSizePowerMethodVersion),
    outerAccountingCloses,
    tailAccountingCloses,
  };
  if (!runId || !/Sample Size and Power Analysis/i.test(runLabel)
    || initialSelectedTable !== "pls_power_by_sample_size"
    || !["Power by sample size", "Bootstrap tail accounting", "Simulation failures", "Design assumptions", "Run provenance"].every((title) => treeItems.includes(title))
    || powerRows !== 2 || tailRows !== 2 || assumptionRows <= 0 || provenanceRows <= 0
    || !outerAccountingCloses || !tailAccountingCloses || !typedFailures
    || !evidence.checks.plsSampleSizePowerResult.provenanceMethodVersionPresent) {
    throw new Error(`Prospective-power result omitted exact grid, tail accounting, failures, design, or provenance: ${JSON.stringify(evidence.checks.plsSampleSizePowerResult)}`);
  }

  const exportButton = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportButton.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsx = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  const expectedSheets = ["Power by sample size", "Bootstrap tail accounting", "Simulation failures", "Design assumptions", "Run provenance"];
  const saveHelper = startWindowsNativeSaveExportHelper({
    targetPath: exportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets,
    expectedSharedStrings: [plsSampleSizePowerMethodVersion, "Bootstrap tail accounting", "Run provenance"],
  });
  let saveCompleted = false;
  try {
    const ready = await saveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Prospective-power XLSX helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsx.click();
    const completion = await saveHelper.completed;
    saveCompleted = true;
    if (!completion.passed) throw new Error(`Prospective-power XLSX verification failed: ${JSON.stringify(completion)}`);
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: `Saved ${path.basename(exportTargetPath)}.` });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(exportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(exportTargetPath);
    evidence.checks.plsSampleSizePowerExport = {
      selectedRunId: await page.locator(".nd-run-select select").inputValue(),
      expectedRunId: runId,
      nativeXlsx: { attempted: true, targetPath: exportTargetPath, file: { size: file.size, isFile: file.isFile() }, workbookSheets, expectedSheets, helper: { ready, completion } },
    };
    if (evidence.checks.plsSampleSizePowerExport.selectedRunId !== runId || !file.isFile() || file.size <= 0
      || !expectedSheets.every((sheet) => workbookSheets.includes(sheet))) {
      throw new Error(`Prospective-power XLSX is not bound to the selected immutable run: ${JSON.stringify(evidence.checks.plsSampleSizePowerExport)}`);
    }
  } finally {
    if (!saveCompleted) saveHelper.stop();
  }
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const archiveBeforeReopen = await inspectSavedPlsSampleSizePowerArchive(plsSampleSizePowerProjectPath, runId);
  await reloadToLauncher();
  await openRecentProject(plsSampleSizePowerProjectName, plsSampleSizePowerProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /Sample Size and Power Analysis/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 30_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("Reopened prospective-power option has no immutable run ID.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedPowerRows = await openResultTable("Power by sample size");
  const reopenedTailRows = await openResultTable("Bootstrap tail accounting");
  const archiveAfterReopen = await inspectSavedPlsSampleSizePowerArchive(plsSampleSizePowerProjectPath, reopenedRunId);
  evidence.checks.plsSampleSizePowerSaveReopen = {
    expectedRunId: runId,
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    sameRunRestored: reopenedRunId === runId && await page.locator(".nd-run-select select").inputValue() === runId,
    reopenedPowerRows,
    reopenedTailRows,
    archiveBeforeReopen,
    archiveAfterReopen,
    archive: archiveAfterReopen,
  };
  if (!evidence.checks.plsSampleSizePowerSaveReopen.sameRunRestored || reopenedPowerRows !== powerRows || reopenedTailRows !== tailRows
    || archiveAfterReopen.immutableRunChecksum !== archiveBeforeReopen.immutableRunChecksum) {
    throw new Error(`The exact prospective-power run did not survive save/reopen: ${JSON.stringify(evidence.checks.plsSampleSizePowerSaveReopen)}`);
  }

  await captureActualTauriViewportMatrix({
    checkName: "plsSampleSizePowerPackagedViewports",
    methodSlug: "pls_sample_size_power_v2",
    methodVersion: plsSampleSizePowerMethodVersion,
    methodEvidenceCheck: "plsSampleSizePowerResult",
    expectedRunId: runId,
    expectedRunLabel: "PLS-SEM Sample Size and Power Analysis",
    expectedTableId: "pls_power_bootstrap_tail_accounting",
    capturePrefix: "pls-sample-size-power",
    captureSequence: "209",
  });
  const internalOrigins = new Set([packagedTauriOrigin, packagedTauriIpcOrigin]);
  const externalRequests = observedBrowserRequests.filter((request) => request.origin
    && request.origin !== "null" && !internalOrigins.has(request.origin));
  evidence.checks.plsSampleSizePowerFunctionalOffline = {
    passed: externalRequests.length === 0,
    analyticalWorkflowRequiresInternet: false,
    strictZeroProcessEgressClaimed: false,
    observedRequestCount: observedBrowserRequests.length,
    externalRequestCount: externalRequests.length,
    origins: [...new Set(observedBrowserRequests.map((request) => request.origin))].sort(),
    externalRequests,
  };
  if (!evidence.checks.plsSampleSizePowerFunctionalOffline.passed) {
    throw new Error(`Prospective power crossed its functional-offline boundary: ${JSON.stringify(evidence.checks.plsSampleSizePowerFunctionalOffline)}`);
  }
}

async function runFocusedPlscBootstrapAcceptance() {
  if (!requestedPlscBootstrapNativeExportPath) {
    throw new Error("QUICKPLS_PLSC_BOOTSTRAP_NATIVE_EXPORT_PATH is required; an enabled XLSX button is not packaged PLSc-bootstrap export evidence.");
  }
  const exportTargetPath = await validateRequestedNativeExportPath(
    requestedPlscBootstrapNativeExportPath,
    "QUICKPLS_PLSC_BOOTSTRAP_NATIVE_EXPORT_PATH",
  );
  await seedRecentProject({
    name: plscBootstrapProjectName,
    path: plscBootstrapProjectPath,
    openedAt: "2026-08-18T00:00:00.000Z",
  });
  await reloadToLauncher();
  await openRecentProject(plscBootstrapProjectName, plscBootstrapProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const fixtureStatus = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  evidence.checks.plscBootstrapFixture = {
    projectPath: plscBootstrapProjectPath,
    status: fixtureStatus,
    columns: (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText),
  };
  if (!/128 cases/i.test(fixtureStatus)) {
    throw new Error(`The focused PLSc-bootstrap fixture did not expose 128 cases: ${JSON.stringify(evidence.checks.plscBootstrapFixture)}`);
  }

  evidence.checks.plscBootstrapInitialModel = await createInitialEditableModel(
    plscBootstrapProjectName,
    plscBootstrapModelName,
  );
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const invalidArchiveBefore = await inspectMediationArchiveRunState(plscBootstrapProjectPath);
  const invalidDialog = await openCalculationFromToolbar();
  const invalidListbox = invalidDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await invalidDialog.locator("#nd-calculation-method-plsc_bootstrap").click();
  const invalidStart = invalidDialog.getByRole("button", { name: "Start consistent bootstrapping", exact: true });
  const invalidBlockers = (await invalidDialog.locator(".nd-blocker li").allTextContents()).map(compactVisibleText).filter(Boolean);
  const invalidSelected = invalidListbox.getByRole("option", { selected: true });
  evidence.checks.plscBootstrapInvalidSetup = {
    attempted: true,
    selectedMethod: compactVisibleText(await invalidSelected.locator("strong").textContent()),
    startEnabled: await invalidStart.isEnabled(),
    blockers: invalidBlockers,
    emptyModelBlocker:
      invalidBlockers.some((row) => /does not contain any constructs|requires at least two constructs/i.test(row))
      && invalidBlockers.some((row) => /does not contain any assigned indicators|requires at least two indicators/i.test(row)),
    archiveBefore: invalidArchiveBefore,
    archiveAfter: null,
    archiveStateUnchanged: false,
    resultCreated: false,
  };
  await capture("172-tauri-native-plsc-bootstrap-invalid-setup-1440x900.png");
  await invalidDialog.getByRole("button", { name: "Close", exact: true }).click();
  await invalidDialog.waitFor({ state: "hidden", timeout: 10_000 });
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const invalidArchiveAfter = await inspectMediationArchiveRunState(plscBootstrapProjectPath);
  evidence.checks.plscBootstrapInvalidSetup.archiveAfter = invalidArchiveAfter;
  evidence.checks.plscBootstrapInvalidSetup.archiveStateUnchanged = JSON.stringify(invalidArchiveAfter) === JSON.stringify(invalidArchiveBefore);
  evidence.checks.plscBootstrapInvalidSetup.resultCreated = invalidArchiveAfter.resultCount > invalidArchiveBefore.resultCount;
  if (evidence.checks.plscBootstrapInvalidSetup.selectedMethod !== "PLSc Consistent Bootstrapping"
    || evidence.checks.plscBootstrapInvalidSetup.startEnabled
    || !evidence.checks.plscBootstrapInvalidSetup.emptyModelBlocker
    || !evidence.checks.plscBootstrapInvalidSetup.archiveStateUnchanged
    || evidence.checks.plscBootstrapInvalidSetup.resultCreated) {
    throw new Error(`The focused PLSc-bootstrap invalid setup did not fail closed: ${JSON.stringify(evidence.checks.plscBootstrapInvalidSetup)}`);
  }

  await buildTwoConstructModel();
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const configure = async (samples, workers) => {
    const dialog = await openCalculationFromToolbar();
    const listbox = dialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
    await dialog.locator("#nd-calculation-method-plsc_bootstrap").click();
    await dialog.locator("#nd-calculation-plsc-bootstrap-samples").fill(String(samples));
    await dialog.locator("#nd-calculation-seed").fill(String(plscBootstrapSeed));
    await dialog.locator("#nd-calculation-workers").fill(String(workers));
    const start = dialog.getByRole("button", { name: "Start consistent bootstrapping", exact: true });
    const selected = listbox.getByRole("option", { selected: true });
    const selectedText = compactVisibleText(await selected.textContent());
    const contract = {
      selectedMethod: compactVisibleText(await selected.locator("strong").textContent()),
      selectedText,
      samples: await dialog.locator("#nd-calculation-plsc-bootstrap-samples").inputValue(),
      seed: await dialog.locator("#nd-calculation-seed").inputValue(),
      workers: await dialog.locator("#nd-calculation-workers").inputValue(),
      inference: compactVisibleText(await dialog.locator("#nd-calculation-plsc-bootstrap-inference").textContent()),
      blockers: (await dialog.locator(".nd-blocker li").allTextContents()).map(compactVisibleText),
      startEnabled: await start.isEnabled(),
      standardSurface: !/Experimental|Limited scope/i.test(selectedText),
    };
    if (contract.selectedMethod !== "PLSc Consistent Bootstrapping"
      || contract.samples !== String(samples) || contract.seed !== String(plscBootstrapSeed)
      || contract.workers !== String(workers) || !contract.standardSurface
      || !/Two-sided normal-reference diagnostics/i.test(contract.inference)
      || !/percentile intervals/i.test(contract.inference)
      || !/BCa when every required full-PLSc delete-one refit is usable/i.test(contract.inference)
      || contract.blockers.length !== 0 || !contract.startEnabled) {
      throw new Error(`The focused PLSc-bootstrap setup did not match its bounded Standard contract: ${JSON.stringify(contract)}`);
    }
    return { dialog, start, contract };
  };

  const cancellationArchiveBefore = await inspectMediationArchiveRunState(plscBootstrapProjectPath);
  const cancellationSetup = await configure(plscBootstrapCancellationSamples, 1);
  const cancellationTerminalStatePromise = page.waitForFunction(() => {
    if (document.querySelector('.nd-app[data-surface="results"]')) return "results_surface";
    const calculationDialog = document.querySelector('.nd-dialog-calculation[role="dialog"]');
    if (!calculationDialog) return "dialog_detached";
    if (calculationDialog.querySelector('.nd-run-progress.cancelled[aria-busy="false"]')) return "cancelled";
    if (calculationDialog.querySelector(".nd-run-progress.completed")) return "completed";
    return null;
  }, null, { timeout: 60_000 });
  const cancellationRequestPromise = page.waitForFunction(() => {
    if (document.querySelector('.nd-app[data-surface="results"]')) return { outcome: "results_surface" };
    const calculationDialog = document.querySelector('.nd-dialog-calculation[role="dialog"]');
    if (!calculationDialog) return null;
    if (calculationDialog.querySelector(".nd-run-progress.completed")) return { outcome: "completed" };
    const progress = calculationDialog.querySelector(
      '.nd-run-progress[aria-busy="true"]:is(.queued,.validating,.running)',
    );
    if (!progress) return null;
    const cancelButtons = Array.from(calculationDialog.querySelectorAll("button"))
      .filter((button) => button.textContent?.trim() === "Cancel calculation" && !button.disabled);
    if (cancelButtons.length !== 1) return null;
    const snapshot = {
      outcome: "cancel_requested",
      ariaBusy: progress.getAttribute("aria-busy"),
      status: [...progress.classList].find((className) => ["queued", "validating", "running"].includes(className)) ?? null,
      phase: progress.querySelector("strong")?.textContent?.trim() ?? "",
      message: progress.querySelector("p")?.textContent?.trim() ?? "",
      progressValue: progress.querySelector("progress")?.getAttribute("value") ?? null,
      progressMax: progress.querySelector("progress")?.getAttribute("max") ?? null,
      logEntries: progress.querySelectorAll("ol li").length,
      cancelButtonCount: cancelButtons.length,
    };
    cancelButtons[0].click();
    return snapshot;
  }, null, { timeout: 5_000 });
  await cancellationSetup.start.click();
  const cancellationRequestHandle = await cancellationRequestPromise;
  const cancellationSnapshot = await cancellationRequestHandle.jsonValue();
  await cancellationRequestHandle.dispose();
  if (cancellationSnapshot?.outcome !== "cancel_requested") {
    throw new Error(`completion_won_race: PLSc consistent bootstrapping reached ${cancellationSnapshot?.outcome ?? "unknown"} before cancellation could be requested.`);
  }
  const cancellationCapturePromise = capture("173-tauri-native-plsc-bootstrap-cancellation-running-1440x900.png")
    .then(() => ({ captured: true, detail: null }))
    .catch((error) => ({ captured: false, detail: error instanceof Error ? error.message : String(error) }));
  const cancellationTerminalStateHandle = await cancellationTerminalStatePromise;
  const cancellationTerminalOutcome = await cancellationTerminalStateHandle.jsonValue();
  await cancellationTerminalStateHandle.dispose();
  const cancellationCapture = await cancellationCapturePromise;
  const activeState = {
    captured: cancellationCapture.captured,
    ...cancellationSnapshot,
    screenshot: cancellationCapture,
  };
  if (cancellationTerminalOutcome !== "cancelled") {
    throw new Error(`completion_won_race: PLSc consistent bootstrapping reached ${cancellationTerminalOutcome} before terminal cancellation became authoritative.`);
  }
  const cancelled = cancellationSetup.dialog.locator(".nd-run-progress.cancelled");
  await cancelled.waitFor({ state: "visible", timeout: 60_000 });
  const cancelledMessage = compactVisibleText(await cancelled.textContent());
  const partialResults = await page.locator(".nd-run-select select option").count();
  await cancellationSetup.dialog.getByRole("button", { name: "Close", exact: true }).click();
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const cancellationArchiveAfter = await inspectMediationArchiveRunState(plscBootstrapProjectPath);
  evidence.checks.plscBootstrapCancellation = {
    passed: activeState.captured && partialResults === 0
      && cancellationArchiveAfter.recipeCount === cancellationArchiveBefore.recipeCount
      && cancellationArchiveAfter.resultCount === cancellationArchiveBefore.resultCount,
    activeState,
    cancelledMessage,
    terminalMessage: cancelledMessage,
    cancelledMethod: cancellationSetup.contract.selectedMethod,
    cancelledSettings: {
      samples: cancellationSetup.contract.samples,
      seed: cancellationSetup.contract.seed,
      workers: cancellationSetup.contract.workers,
    },
    noPartialResult: partialResults === 0,
    noPartialVisibleResult: partialResults === 0,
    noPartialCommittedResult: cancellationArchiveAfter.resultCount === cancellationArchiveBefore.resultCount,
    archiveStateUnchanged: JSON.stringify(cancellationArchiveAfter) === JSON.stringify(cancellationArchiveBefore),
    archiveBefore: cancellationArchiveBefore,
    archiveAfter: cancellationArchiveAfter,
  };
  if (!evidence.checks.plscBootstrapCancellation.passed) {
    throw new Error(`Cancelled PLSc bootstrapping appended partial evidence: ${JSON.stringify(evidence.checks.plscBootstrapCancellation)}`);
  }

  const retry = await configure(plscBootstrapSamples, plscBootstrapWorkers);
  evidence.checks.plscBootstrapDialog = retry.contract;
  const progressCapture = captureActiveCalculation(
    retry.dialog,
    "174-tauri-native-plsc-bootstrap-running-1440x900.png",
    "PLSc consistent bootstrapping",
    { allowTerminalTransitionAfterCapture: true },
  );
  await retry.start.click();
  evidence.checks.plscBootstrapProgress = { captured: true, ...await progressCapture };
  await waitForSurface("results", 300_000);
  const selectedRun = page.locator(".nd-run-select select option:checked");
  await selectedRun.waitFor({ state: "attached", timeout: 300_000 });
  const runId = await page.locator(".nd-run-select select").inputValue();
  const runLabel = compactVisibleText(await selectedRun.textContent());
  evidence.checks.plscBootstrapProgress.completedRunProof = {
    matched: Boolean(runId),
    runId,
  };
  Object.assign(evidence.checks.plscBootstrapCancellation, {
    retryEnabled: retry.contract.startEnabled,
    retrySettings: {
      selectedMethod: retry.contract.selectedMethod,
      samples: retry.contract.samples,
      seed: retry.contract.seed,
      workers: retry.contract.workers,
    },
    completedRetryRunId: runId,
  });
  evidence.checks.plscBootstrapCancellation.passed = evidence.checks.plscBootstrapCancellation.passed
    && retry.contract.startEnabled
    && retry.contract.samples === evidence.checks.plscBootstrapCancellation.cancelledSettings.samples
    && retry.contract.seed === evidence.checks.plscBootstrapCancellation.cancelledSettings.seed
    && Boolean(runId);
  const initialSelectedTable = await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').getAttribute("data-result-tree-item-id");
  const treeItems = (await page.locator('.nd-result-tree [role="treeitem"]').allTextContents()).map(compactVisibleText);
  const accountingRows = await openResultTable("PLSc bootstrap replicate accounting");
  const accountingValues = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => Object.fromEntries(rows.map((row) => {
    const cells = Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "");
    return [cells[0], cells[1]];
  })));
  const requested = Number(accountingValues["Requested case resamples"]);
  const attempted = Number(accountingValues["Attempted preplanned full-PLSc refits"]);
  const usable = Number(accountingValues["Usable full-PLSc refits"]);
  const failed = Number(accountingValues["Failed full-PLSc refits"]);
  const successfulWitnesses = Number(accountingValues["Replayable successful-refit witnesses"]);
  const jackknife = Number(accountingValues["Delete-one PLSc fits"]);
  const successfulJackknife = Number(accountingValues["Replayable successful delete-one witnesses"]);
  const failedJackknife = Number(accountingValues["Failed delete-one fits"]);
  const percentileRows = await openResultTable("PLSc consistent bootstrapping");
  const bcaRows = await openResultTable("Bias-corrected and accelerated intervals");
  let failureDisclosure = { rows: 0, typed: true };
  if (failed > 0) {
    const rows = await openResultTable("PLSc bootstrap failed refits");
    const values = await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => (
      Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
    )));
    failureDisclosure = { rows, typed: values.every((row) => row.length === 4 && row.every(Boolean)) };
  }
  let jackknifeFailureDisclosure = { rows: 0, typed: true };
  if (failedJackknife > 0) {
    const rows = await openResultTable("PLSc bootstrap failed delete-one fits");
    const values = await page.locator(".nd-result-table tbody tr").evaluateAll((tableRows) => tableRows.map((row) => (
      Array.from(row.querySelectorAll("th,td"), (cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
    )));
    jackknifeFailureDisclosure = { rows, typed: values.every((row) => row.length === 3 && row.every(Boolean)) };
  }
  evidence.checks.plscBootstrapResult = {
    runId,
    runLabel,
    methodVersion: plscBootstrapMethodVersion,
    initialSelectedTable,
    treeItems,
    accountingRows,
    accountingValues,
    requested,
    attempted,
    usable,
    failed,
    successfulWitnesses,
    jackknife,
    successfulJackknife,
    failedJackknife,
    percentileRows,
    bcaRows,
    failureDisclosure,
    jackknifeFailureDisclosure,
  };
  if (!runId || !/consistent bootstrapping/i.test(runLabel)
    || !treeItems.includes("PLSc bootstrap replicate accounting")
    || !treeItems.includes("PLSc consistent bootstrapping")
    || requested !== plscBootstrapSamples || attempted !== requested
    || usable + failed !== requested || successfulWitnesses !== usable
    || successfulJackknife + failedJackknife !== jackknife
    || accountingRows < 10 || percentileRows <= 0 || bcaRows <= 0
    || failureDisclosure.rows !== failed || !failureDisclosure.typed
    || jackknifeFailureDisclosure.rows !== failedJackknife || !jackknifeFailureDisclosure.typed) {
    throw new Error(`The completed PLSc-bootstrap result omitted required accounting, witness, interval, or failure nodes: ${JSON.stringify(evidence.checks.plscBootstrapResult)}`);
  }

  const exportButton = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportButton.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsx = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  const expectedSheets = [
    "PLSc bootstrap replicate accounting",
    "PLSc consistent bootstrapping",
    "Bias-corrected and accelerated intervals",
    "Run provenance",
  ].map((title) => title.slice(0, 31).trimEnd());
  const saveHelper = startWindowsNativeSaveExportHelper({
    targetPath: exportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets,
    expectedSharedStrings: [
      plscBootstrapMethodVersion,
      "Replayable successful-refit witnesses",
      "Replayable successful delete-one witnesses",
      "Run provenance",
    ],
  });
  let saveCompleted = false;
  try {
    const ready = await saveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`PLSc-bootstrap native XLSX helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsx.click();
    const completion = await saveHelper.completed;
    saveCompleted = true;
    if (!completion.passed) throw new Error(`PLSc-bootstrap native XLSX verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(exportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(exportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(exportTargetPath);
    evidence.checks.plscBootstrapExport = {
      selectedRunId: await page.locator(".nd-run-select select").inputValue(),
      expectedRunId: runId,
      nativeXlsx: {
        attempted: true,
        targetPath: exportTargetPath,
        file: { size: file.size, isFile: file.isFile() },
        workbookSheets,
        expectedSheets,
        helper: { ready, completion },
      },
    };
    if (evidence.checks.plscBootstrapExport.selectedRunId !== runId || !file.isFile() || file.size <= 0
      || !expectedSheets.every((sheet) => workbookSheets.includes(sheet))) {
      throw new Error(`The PLSc-bootstrap XLSX was not bound to the selected immutable run: ${JSON.stringify(evidence.checks.plscBootstrapExport)}`);
    }
  } finally {
    if (!saveCompleted) saveHelper.stop();
  }
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const archiveBeforeReopen = await inspectSavedPlscBootstrapArchive(plscBootstrapProjectPath, runId);
  await reloadToLauncher();
  await openRecentProject(plscBootstrapProjectName, plscBootstrapProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedOption = page.locator(".nd-run-select select option").filter({ hasText: /consistent bootstrapping/i }).first();
  await reopenedOption.waitFor({ state: "attached", timeout: 30_000 });
  const reopenedRunId = await reopenedOption.getAttribute("value");
  if (!reopenedRunId) throw new Error("The reopened PLSc-bootstrap option had no immutable run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedRunId);
  const reopenedAccountingRows = await openResultTable("PLSc bootstrap replicate accounting");
  const reopenedAccountingText = compactVisibleText(await page.locator(".nd-result-table tbody").textContent());
  const archiveAfterReopen = await inspectSavedPlscBootstrapArchive(plscBootstrapProjectPath, reopenedRunId);
  evidence.checks.plscBootstrapSaveReopen = {
    expectedRunId: runId,
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    sameRunRestored: reopenedRunId === runId && await page.locator(".nd-run-select select").inputValue() === runId,
    reopenedAccountingRows,
    witnessRowsRestored: reopenedAccountingText.includes("Replayable successful-refit witnesses")
      && reopenedAccountingText.includes("Replayable successful delete-one witnesses"),
    archiveBeforeReopen,
    archiveAfterReopen,
    archive: archiveAfterReopen,
  };
  if (!evidence.checks.plscBootstrapSaveReopen.sameRunRestored || reopenedAccountingRows !== accountingRows
    || !evidence.checks.plscBootstrapSaveReopen.witnessRowsRestored
    || archiveAfterReopen.immutableRunChecksum !== archiveBeforeReopen.immutableRunChecksum) {
    throw new Error(`The exact PLSc-bootstrap run did not survive save/reopen unchanged: ${JSON.stringify(evidence.checks.plscBootstrapSaveReopen)}`);
  }

  await captureActualTauriViewportMatrix({
    checkName: "plscBootstrapPackagedViewports",
    methodSlug: "plsc_bootstrap_v1",
    methodVersion: plscBootstrapMethodVersion,
    methodEvidenceCheck: "plscBootstrapResult",
    expectedRunId: runId,
    expectedRunLabel: "PLSc Consistent Bootstrapping",
    expectedTableId: "plsc_bootstrap_accounting",
    capturePrefix: "plsc-bootstrap",
    captureSequence: "179",
  });
  const internalOrigins = new Set([packagedTauriOrigin, packagedTauriIpcOrigin]);
  const externalRequests = observedBrowserRequests.filter((request) => request.origin
    && request.origin !== "null" && !internalOrigins.has(request.origin));
  evidence.checks.plscBootstrapFunctionalOffline = {
    passed: externalRequests.length === 0,
    analyticalWorkflowRequiresInternet: false,
    strictZeroProcessEgressClaimed: false,
    observedRequestCount: observedBrowserRequests.length,
    externalRequestCount: externalRequests.length,
    origins: [...new Set(observedBrowserRequests.map((request) => request.origin))].sort(),
    externalRequests,
  };
  if (!evidence.checks.plscBootstrapFunctionalOffline.passed) {
    throw new Error(`PLSc-bootstrap crossed its packaged functional-offline boundary: ${JSON.stringify(evidence.checks.plscBootstrapFunctionalOffline)}`);
  }
}

try {
  await page.locator(".nd-app[data-native-desktop-shell='true']").waitFor({ state: "visible", timeout: 15_000 });
  evidence.checks.runtime = await page.evaluate(() => ({
    title: document.title,
    tauriRuntime: Boolean(window.__TAURI_INTERNALS__),
    viewport: { width: innerWidth, height: innerHeight, dpr: devicePixelRatio },
    surface: document.querySelector(".nd-app")?.getAttribute("data-surface"),
  }));
  if (!evidence.checks.runtime.tauriRuntime) throw new Error("The inspected page is not running inside Tauri.");
  nativeViewportLabel = `${evidence.checks.runtime.viewport.width}x${evidence.checks.runtime.viewport.height}`;
  if (plsSampleSizePowerOnly) {
    await runFocusedPlsSampleSizePowerAcceptance();
  } else if (plscBootstrapOnly) {
    await runFocusedPlscBootstrapAcceptance();
  } else if (gscaOnly) {
    await runFocusedGscaAcceptance();
  } else if (cbsemExactBootstrapOnly) {
    await runFocusedExactCbsemBootstrapAcceptance();
  } else if (cbsemOnly) {
    await runFocusedCbsemAcceptance();
  } else if (logisticOnly) {
    await runFocusedLogisticAcceptance();
  } else if (regressionBootstrapOnly) {
    await runFocusedRegressionBootstrapAcceptance();
  } else if (processV2Only) {
    await runFocusedProcessV2Acceptance();
  } else if (structuralPathRandomizationOnly) {
    await runFocusedStructuralPathRandomizationAcceptance();
  } else if (olsOnly) {
    await runFocusedOlsAcceptance();
  } else if (pcaOnly) {
    await runFocusedPcaAcceptance();
  } else if (ctaPlsOnly) {
    await runFocusedCtaPlsAcceptance();
  } else if (hocOnly) {
    await runFocusedHigherOrderAcceptance();
  } else if (predictionOnly) {
    await runFocusedPredictionAcceptance();
  } else {
  if (ncaOnly) {
    await reloadToLauncher();
  } else {
    if (!mgaOnly) {
    await capture("12-tauri-native-launcher-1440x900.png");

    const bundledSamples = [];
    for (const sampleContract of bundledSampleContracts) {
      await reloadToLauncher();
      bundledSamples.push(await inspectBundledSample(sampleContract));
      await capture(`12-tauri-native-sample-${sampleContract.id}-${nativeViewportLabel}.png`);
    }
    evidence.checks.bundledSampleGallery = {
      passed: bundledSamples.length === bundledSampleContracts.length,
      catalogSchemaVersion: bundledSampleCatalog.schemaVersion,
      catalogSha256: bundledSampleCatalogSha256,
      defaultSampleId: bundledSampleCatalog.defaultSampleId,
      datasetCount: bundledSampleCatalog.datasets.length,
      catalogSampleCount: bundledSampleContracts.length,
      sampleIds: bundledSamples.map((sample) => sample.sampleId),
      samples: bundledSamples,
      deliberateScopeSubstitutions: bundledSamples
        .filter((sample) => sample.scientificReference.deliberateScopeSubstitution)
        .map((sample) => ({
          sampleId: sample.sampleId,
          scope: sample.scientificReference.scope,
          boundary: sample.scientificReference.boundary,
        })),
      liveLauncher: true,
      typedSelector: true,
      completedCanonicalResults: true,
    };
    if (!evidence.checks.bundledSampleGallery.passed
      || JSON.stringify(evidence.checks.bundledSampleGallery.sampleIds)
        !== JSON.stringify(bundledSampleContracts.map((sample) => sample.id))) {
      throw new Error(`The packaged bundled-sample gallery was incomplete: ${JSON.stringify(evidence.checks.bundledSampleGallery)}`);
    }

    await reloadToLauncher();

    await openMenuItem("File", "Open Sample Project");
  await waitForSurface("model");
  await page.locator(".react-flow__node-latent").first().waitFor({ state: "visible", timeout: 15_000 });
  evidence.checks.sample = {
    project: (await page.locator(".nd-window-project").textContent())?.trim(),
    constructs: await page.locator(".react-flow__node-latent").count(),
  };
  await capture("13-tauri-native-model-1440x900.png");

  await openMenuItem("View", "Data");
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  await capture("14-tauri-native-data-1440x900.png");

  const recodeCommand = page.locator(".nd-commandbar button").filter({ hasText: /Recode Variable/i });
  if (await recodeCommand.count() !== 1) throw new Error("The Data toolbar did not expose exactly one Recode Variable command.");
  await recodeCommand.click();
  const recodeDialog = page.locator('.nd-dialog-recode-data[role="dialog"]');
  await recodeDialog.waitFor({ state: "visible", timeout: 5_000 });
  await recodeDialog.getByLabel("New indicator", { exact: true }).fill("COMP1_recode");
  await recodeDialog.getByLabel("Type", { exact: true }).selectOption("numeric");
  await recodeDialog.getByLabel("Scale", { exact: true }).selectOption("continuous");
  await recodeDialog.getByLabel("Mapping 1 source value", { exact: true }).fill("6");
  await recodeDialog.getByLabel(/Mapping 1 new value/i).fill("60");
  const createRecode = recodeDialog.getByRole("button", { name: "Create Recode", exact: true });
  evidence.checks.recodeDialog = {
    source: await recodeDialog.getByLabel("Source indicator", { exact: true }).inputValue(),
    browserOnlyNoticeCount: await recodeDialog.getByText(/Browser preview cannot write dataset versions/i).count(),
    createEnabled: await createRecode.isEnabled(),
  };
  if (!evidence.checks.recodeDialog.createEnabled) throw new Error("Native Recode remained disabled after a valid mapping was entered.");
  await capture("15-tauri-native-recode-dialog-1440x900.png");
  await createRecode.click();
  await recodeDialog.waitFor({ state: "hidden", timeout: 15_000 });
  await page.getByText("Versions (2)", { exact: true }).waitFor({ state: "visible", timeout: 15_000 });
  evidence.checks.recodeCompleted = {
    versionCountText: (await page.getByText("Versions (2)", { exact: true }).textContent())?.trim(),
    derivedVariableVisible: await page.getByText("COMP1_recode", { exact: true }).count(),
  };
  if (!evidence.checks.recodeCompleted.derivedVariableVisible) throw new Error("The derived COMP1_recode indicator was not visible after the native mutation completed.");
  await capture("16-tauri-native-data-version-1440x900.png");

  await openMenuItem("View", "Model");
  await waitForSurface("model");
  const calculate = page.locator(".nd-commandbar button").filter({ hasText: /^Calculate/i });
  if (await calculate.count() !== 1) throw new Error("The Model toolbar did not expose exactly one generic Calculate command.");

  const calculateMenuTrigger = page.getByRole("menuitem", { name: "Calculate", exact: true });
  await calculateMenuTrigger.click();
  const calculateMenuId = await calculateMenuTrigger.getAttribute("aria-controls");
  if (!calculateMenuId) throw new Error("The Calculate menu trigger did not reference its popup.");
  const calculateMenu = page.locator(`#${calculateMenuId}`);
  await calculateMenu.waitFor({ state: "visible", timeout: 5_000 });
  const calculateMenuLabels = (await calculateMenu.getByRole("menuitem").allTextContents()).map((label) => label.trim());
  const methodSpecificMenuLabels = calculateMenuLabels.filter((label) => /PLS Algorithm|Bootstrapp|Permutation|Randomization|Construct Prediction|Prediction|Consistent PLS|Weighted PLS|CCA|composite residual|CTA|Tetrad|Importance-Performance|IPMA|MGA|Multi-Group|Necessary Condition|NCA/i.test(label));
  evidence.checks.calculationCommands = {
    toolbarGenericCommands: await calculate.count(),
    calculateMenuEntries: calculateMenuLabels,
    methodSpecificMenuEntries: methodSpecificMenuLabels,
  };
  if (calculateMenuLabels.length !== 1 || methodSpecificMenuLabels.length !== 0 || !/^Calculate/i.test(calculateMenuLabels[0])) {
    throw new Error(`The Calculate menu must contain one generic command, not per-method commands: ${calculateMenuLabels.join(" | ")}`);
  }
  await calculateMenu.getByRole("menuitem").click();

  const calculationDialog = page.locator('.nd-dialog-calculation[role="dialog"]');
  await calculationDialog.waitFor({ state: "visible", timeout: 5_000 });
  const methodListbox = calculationDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const methodOptions = methodListbox.getByRole("option");
  await methodOptions.first().waitFor({ state: "visible", timeout: 10_000 });
  const optionLabels = (await methodOptions.locator("strong").allTextContents()).map((label) => label.trim());
  const pathRandomizationOption = methodListbox.getByRole("option", { name: /Structural Path Randomization/i });
  const pathRandomizationDescription = (await pathRandomizationOption.textContent())?.replace(/\s+/g, " ").trim() ?? "";
  evidence.checks.calculationCatalog = {
    options: optionLabels,
    optionCount: optionLabels.length,
    legacyTabCount: await calculationDialog.getByRole("tab").count(),
    structuralPathRandomization: {
      optionCount: await pathRandomizationOption.count(),
      description: pathRandomizationDescription,
      singleModelFreedmanLane: /single-model Freedman(?:\u2013|-|\s)Lane randomization/i.test(pathRandomizationDescription)
        && /structural paths/i.test(pathRandomizationDescription)
        && /fixed original PLS construct scores/i.test(pathRandomizationDescription)
        && /unadjusted pathwise p values/i.test(pathRandomizationDescription),
      mentionsMgaOrMicom: /\bMGA\b|\bMICOM\b/i.test(pathRandomizationDescription),
    },
  };
  if (JSON.stringify(optionLabels) !== JSON.stringify(expectedOptionLabels)) {
    throw new Error(`The calculation browser did not expose the expected ${expectedOptionLabels.length}-method catalog: ${optionLabels.join(" | ")}`);
  }
  if (evidence.checks.calculationCatalog.legacyTabCount !== 0) throw new Error("The extracted calculation browser still exposed legacy method tabs.");
  if (evidence.checks.calculationCatalog.structuralPathRandomization.optionCount !== 1
    || !evidence.checks.calculationCatalog.structuralPathRandomization.singleModelFreedmanLane
    || evidence.checks.calculationCatalog.structuralPathRandomization.mentionsMgaOrMicom) {
    throw new Error(`Structural Path Randomization did not preserve its required single-model Freedman-Lane structural-path, fixed-score, unadjusted pathwise scope, or mentioned MGA/MICOM: ${pathRandomizationDescription}`);
  }

  const predictionOption = calculationDialog.getByRole("option", { name: /PLSpredict \/ CVPAT/i });
  const predictionOptionText = (await predictionOption.textContent())?.replace(/\s+/g, " ").trim() ?? "";
  await predictionOption.click();
  const startPrediction = calculationDialog.getByRole("button", { name: "Start prediction", exact: true });
  const expectedPredictionPlan = "Complete cases; seeded balanced 10-fold × 10-repeat cross-validation; deterministic modulo-4 holdout retained as a secondary check";
  evidence.checks.predictionDialog = {
    selectedMethod: (await methodListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    catalogDescription: predictionOptionText,
    startEnabled: await startPrediction.isEnabled(),
    blockers: await calculationDialog.locator(".nd-blocker li").allTextContents(),
    plan: (await calculationDialog.locator("#nd-calculation-prediction-plan strong").textContent())?.trim(),
    targets: (await calculationDialog.locator("#nd-calculation-prediction-targets strong").textContent())?.trim(),
    benchmarks: (await calculationDialog.locator("#nd-calculation-prediction-benchmarks strong").textContent())?.trim(),
    cvpatScope: (await calculationDialog.locator("#nd-calculation-prediction-cvpat strong").textContent())?.trim(),
    seedControls: await calculationDialog.locator("#nd-calculation-seed").count(),
    workerControls: await calculationDialog.locator("#nd-calculation-workers").count(),
  };
  evidence.checks.predictionDialog.truthfulBoundedLabel = /PLSpredict \/ CVPAT/i.test(evidence.checks.predictionDialog.selectedMethod ?? "")
    && /endogenous-indicator prediction/i.test(predictionOptionText)
    && /10-fold × 10-repeat/i.test(predictionOptionText)
    && evidence.checks.predictionDialog.plan === expectedPredictionPlan
    && /endogenous indicators.*primary.*construct-score metrics.*supplementary/i.test(evidence.checks.predictionDialog.targets ?? "")
    && /indicator average \(IA\).*linear model \(LM(?:, where estimable)?\)/i.test(evidence.checks.predictionDialog.benchmarks ?? "")
    && /single fitted model versus IA\/LM benchmarks/i.test(evidence.checks.predictionDialog.cvpatScope ?? "")
    && /one-sided test, 95% confidence/i.test(evidence.checks.predictionDialog.cvpatScope ?? "")
    && /not a comparison of saved models/i.test(evidence.checks.predictionDialog.cvpatScope ?? "")
    && evidence.checks.predictionDialog.seedControls === 1
    && evidence.checks.predictionDialog.workerControls === 0;
  evidence.checks.predictionDialog.expectedSampleScopeBlock = !evidence.checks.predictionDialog.startEnabled
    && evidence.checks.predictionDialog.blockers.some((blocker) => /at least 20 observations/i.test(blocker));
  if (!evidence.checks.predictionDialog.truthfulBoundedLabel) throw new Error(`Prediction did not expose the bounded indicator-level PLSpredict/CVPAT label, seeded plan, IA/LM benchmarks, and one-sided single-model scope: ${JSON.stringify(evidence.checks.predictionDialog)}`);
  if (!evidence.checks.predictionDialog.expectedSampleScopeBlock) throw new Error(`The 12-row bundled sample did not expose the expected 20-observation Prediction scope blocker: ${evidence.checks.predictionDialog.blockers.join(" | ")}`);
  await capture("17-tauri-native-prediction-dialog-1440x900.png");
  await calculationDialog.getByRole("button", { name: "Close dialog", exact: true }).click();
  await openMenuItem("View", "Project");
  await waitForSurface("launcher");

  await seedDisposableRecentProject();
  await reloadToLauncher();
  const seededRecentRow = exactRecentProjectRow(disposableProjectName, disposableProjectPath);
  await seededRecentRow.waitFor({ state: "visible", timeout: 10_000 });
  evidence.checks.disposableRecentProject = {
    visibleRows: await seededRecentRow.count(),
    pathVisible: (await seededRecentRow.textContent())?.includes(disposableProjectPath) ?? false,
    projectPath: disposableProjectPath,
  };
  if (evidence.checks.disposableRecentProject.visibleRows !== 1 || !evidence.checks.disposableRecentProject.pathVisible) {
    throw new Error("The disposable 128-row project was not exposed through one truthful visible Recent Projects row.");
  }
  await capture("18-tauri-native-seeded-recent-project-1440x900.png");

  await openDisposableRecentProject();
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const largeDatasetStatus = (await page.locator(".nd-statusbar").textContent())?.trim() ?? "";
  evidence.checks.nativeMethodDataset = {
    cases: largeDatasetStatus.includes("128 cases") ? 128 : null,
    columns: await page.locator(".nd-data-table thead th").allTextContents(),
    status: largeDatasetStatus,
  };
  if (evidence.checks.nativeMethodDataset.cases !== 128 || !evidence.checks.nativeMethodDataset.columns.some((column) => column.trim() === "case_wt")) {
    throw new Error(`The visible disposable project did not load the 128-row case-weight dataset: ${JSON.stringify(evidence.checks.nativeMethodDataset)}`);
  }
  await capture("19-tauri-native-method-fixture-data-1440x900.png");

  const dataGrid = page.locator('.nd-data-table[role="grid"]');
  const dataCell = dataGrid.locator('[role="gridcell"][data-native-grid-row="8"][data-native-grid-column="1"]');
  await dataCell.click();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("Control+C");
  await page.waitForFunction(() => Boolean(document.querySelector('.nd-data-grid [role="status"][aria-live="polite"]')?.textContent?.trim()), null, { timeout: 5_000 });
  const activeDataCell = dataGrid.locator('[role="gridcell"][tabindex="0"]');
  evidence.checks.dataGridKeyboard = await page.evaluate(() => {
    const grid = document.querySelector('.nd-data-table[role="grid"]');
    const active = grid?.querySelector('[role="gridcell"][tabindex="0"]');
    const viewport = grid?.closest('.nd-table-scroll');
    const pager = document.querySelector('.nd-data-pager');
    const selectedHeader = grid?.querySelector('th.selected[data-native-variable]');
    const style = active ? getComputedStyle(active) : null;
    return {
      activeCells: grid?.querySelectorAll('[role="gridcell"][tabindex="0"]').length ?? 0,
      activeRow: active?.getAttribute('data-native-grid-row') ?? null,
      activeColumn: active?.getAttribute('data-native-grid-column') ?? null,
      activeVariable: active?.getAttribute('data-native-variable') ?? null,
      selectedHeader: selectedHeader?.getAttribute('data-native-variable') ?? null,
      activeCellFocused: document.activeElement === active,
      activeBoxShadow: style?.boxShadow ?? null,
      pagerVisible: Boolean(pager && pager.getBoundingClientRect().height > 0),
      viewportOverflowX: viewport ? getComputedStyle(viewport).overflowX : null,
      pageHorizontalOverflow: document.documentElement.scrollWidth > document.documentElement.clientWidth,
    };
  });
  evidence.checks.dataGridKeyboard.copyAnnouncement = (await page.locator('.nd-data-grid [role="status"][aria-live="polite"]').textContent())?.trim() ?? "";
  if (evidence.checks.dataGridKeyboard.activeCells !== 1
    || evidence.checks.dataGridKeyboard.activeRow !== "9"
    || evidence.checks.dataGridKeyboard.activeColumn !== "2"
    || evidence.checks.dataGridKeyboard.activeVariable !== evidence.checks.dataGridKeyboard.selectedHeader
    || !evidence.checks.dataGridKeyboard.activeCellFocused
    || !evidence.checks.dataGridKeyboard.activeBoxShadow
    || evidence.checks.dataGridKeyboard.activeBoxShadow === "none"
    || !evidence.checks.dataGridKeyboard.pagerVisible
    || evidence.checks.dataGridKeyboard.viewportOverflowX !== "auto"
    || evidence.checks.dataGridKeyboard.pageHorizontalOverflow
    || !evidence.checks.dataGridKeyboard.copyAnnouncement) {
    throw new Error(`The native Data grid did not satisfy its keyboard, selection, copy, or contained-scroll contract: ${JSON.stringify(evidence.checks.dataGridKeyboard)}`);
  }
  await activeDataCell.waitFor({ state: "visible", timeout: 5_000 });
  await capture("19a-tauri-native-data-grid-active-cell-1440x900.png");

  // A CLI-imported dataset truthfully starts without a canonical model. Create
  // the first editable model through the same Project Explorer flow exposed to
  // desktop users before entering the model workbench.
  evidence.checks.initialEditableModelCreation = await createInitialEditableModel(disposableProjectName, disposableModelName);
  await clickIndicator("x1");
  await page.locator(".react-flow__node-latent").nth(0).waitFor({ state: "visible", timeout: 10_000 });
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const plscInvalidArchiveBefore = await inspectMediationArchiveRunState(disposableProjectPath);
  const plscInvalidDialog = await openCalculationFromToolbar();
  const plscInvalidListbox = plscInvalidDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await plscInvalidListbox.locator("#nd-calculation-method-plsc").click();
  const plscInvalidStart = plscInvalidDialog.getByRole("button", { name: "Start consistent PLS", exact: true });
  const plscInvalidBlockers = (await plscInvalidDialog.locator(".nd-blocker li").allTextContents()).map((row) => row.trim()).filter(Boolean);
  evidence.checks.plscInvalidSetup = {
    attempted: true,
    selectedMethod: (await plscInvalidListbox.getByRole("option", { selected: true }).textContent())?.trim() ?? "",
    startEnabled: await plscInvalidStart.isEnabled(),
    blockers: plscInvalidBlockers,
    underspecifiedReflectiveBlocker: plscInvalidBlockers.some((row) => /Consistent PLS requires at least two indicators per construct/i.test(row)),
    archiveBefore: plscInvalidArchiveBefore,
    archiveAfter: null,
    runStateUnchanged: false,
    resultCreated: false,
  };
  await capture("19b-tauri-native-plsc-invalid-setup-1440x900.png");
  await plscInvalidDialog.getByRole("button", { name: "Close", exact: true }).click();
  await plscInvalidDialog.waitFor({ state: "hidden", timeout: 10_000 });
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const plscInvalidArchiveAfter = await inspectMediationArchiveRunState(disposableProjectPath);
  evidence.checks.plscInvalidSetup.archiveAfter = plscInvalidArchiveAfter;
  evidence.checks.plscInvalidSetup.runStateUnchanged = JSON.stringify(plscInvalidArchiveAfter) === JSON.stringify(plscInvalidArchiveBefore);
  evidence.checks.plscInvalidSetup.resultCreated = plscInvalidArchiveAfter.resultCount > plscInvalidArchiveBefore.resultCount;
  if (!/Consistent PLS/i.test(evidence.checks.plscInvalidSetup.selectedMethod)
    || evidence.checks.plscInvalidSetup.startEnabled
    || !evidence.checks.plscInvalidSetup.underspecifiedReflectiveBlocker
    || plscInvalidArchiveBefore.recipeCount !== 0
    || plscInvalidArchiveBefore.resultCount !== 0
    || plscInvalidArchiveBefore.runCount !== 0
    || !evidence.checks.plscInvalidSetup.runStateUnchanged
    || evidence.checks.plscInvalidSetup.resultCreated) {
    throw new Error(`The underspecified packaged PLSc setup did not fail closed without creating calculation state: ${JSON.stringify(evidence.checks.plscInvalidSetup)}`);
  }

  const wplsInvalidArchiveBefore = await inspectMediationArchiveRunState(disposableProjectPath);
  const wplsInvalidDialog = await openCalculationFromToolbar();
  const wplsInvalidListbox = wplsInvalidDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await wplsInvalidListbox.getByRole("option", { name: /Weighted PLS/i }).click();
  const wplsInvalidWeight = wplsInvalidDialog.locator("#nd-calculation-case-weight");
  const wplsInvalidStart = wplsInvalidDialog.getByRole("button", { name: "Start weighted PLS", exact: true });
  const wplsInvalidBlockers = (await wplsInvalidDialog.locator(".nd-blocker li").allTextContents()).map((row) => row.trim()).filter(Boolean);
  evidence.checks.wplsInvalidSetup = {
    attempted: true,
    selectedMethod: (await wplsInvalidListbox.getByRole("option", { selected: true }).textContent())?.trim() ?? "",
    caseWeightColumn: await wplsInvalidWeight.inputValue(),
    startEnabled: await wplsInvalidStart.isEnabled(),
    blockers: wplsInvalidBlockers,
    missingWeightBlocker: wplsInvalidBlockers.some((row) => /Choose a positive numeric case-weight variable/i.test(row)),
    archiveBefore: wplsInvalidArchiveBefore,
    archiveAfter: null,
    runStateUnchanged: false,
    resultCreated: false,
  };
  await capture("19c-tauri-native-wpls-invalid-setup-1440x900.png");
  await wplsInvalidDialog.getByRole("button", { name: "Close", exact: true }).click();
  await wplsInvalidDialog.waitFor({ state: "hidden", timeout: 10_000 });
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const wplsInvalidArchiveAfter = await inspectMediationArchiveRunState(disposableProjectPath);
  evidence.checks.wplsInvalidSetup.archiveAfter = wplsInvalidArchiveAfter;
  evidence.checks.wplsInvalidSetup.runStateUnchanged = JSON.stringify(wplsInvalidArchiveAfter) === JSON.stringify(wplsInvalidArchiveBefore);
  evidence.checks.wplsInvalidSetup.resultCreated = wplsInvalidArchiveAfter.resultCount > wplsInvalidArchiveBefore.resultCount;
  if (!/Weighted PLS/i.test(evidence.checks.wplsInvalidSetup.selectedMethod)
    || evidence.checks.wplsInvalidSetup.caseWeightColumn !== ""
    || evidence.checks.wplsInvalidSetup.startEnabled
    || !evidence.checks.wplsInvalidSetup.missingWeightBlocker
    || wplsInvalidArchiveBefore.recipeCount !== 0
    || wplsInvalidArchiveBefore.resultCount !== 0
    || wplsInvalidArchiveBefore.runCount !== 0
    || !evidence.checks.wplsInvalidSetup.runStateUnchanged
    || evidence.checks.wplsInvalidSetup.resultCreated) {
    throw new Error(`The missing-weight packaged WPLS setup did not fail closed without creating calculation state: ${JSON.stringify(evidence.checks.wplsInvalidSetup)}`);
  }

  await buildTwoConstructModel({ firstIndicatorAlreadyAssigned: true });
  evidence.checks.visibleModelBuild = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    unassignedCaseWeight: await page.locator(".nd-variable-item").filter({ hasText: /^case_wt$/ }).evaluate((element) => !element.classList.contains("assigned")),
  };
  if (evidence.checks.visibleModelBuild.constructs !== 2
    || evidence.checks.visibleModelBuild.assignedIndicators !== 4
    || evidence.checks.visibleModelBuild.structuralPaths !== 1
    || !evidence.checks.visibleModelBuild.unassignedCaseWeight) {
    throw new Error(`The visible Model workflow did not create the expected x1/x2 -> y1/y2 model: ${JSON.stringify(evidence.checks.visibleModelBuild)}`);
  }
  await capture("20-tauri-native-method-fixture-model-built-1440x900.png");

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  await reloadToLauncher();
  await openDisposableRecentProject();
  await waitForSurface("model");
  await page.locator(".react-flow__node-latent").nth(1).waitFor({ state: "visible", timeout: 15_000 });
  evidence.checks.projectSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    projectPath: disposableProjectPath,
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    cases: ((await page.locator(".nd-statusbar").textContent()) ?? "").includes("128 cases") ? 128 : null,
  };
  if (evidence.checks.projectSaveReopen.constructs !== 2
    || evidence.checks.projectSaveReopen.assignedIndicators !== 4
    || evidence.checks.projectSaveReopen.structuralPaths !== 1
    || evidence.checks.projectSaveReopen.cases !== 128) {
    throw new Error(`The visible save/reload/recent-row workflow did not restore the native method project: ${JSON.stringify(evidence.checks.projectSaveReopen)}`);
  }
  await capture("21-tauri-native-method-fixture-model-reopened-1440x900.png");

  const plscDialog = await openCalculationFromToolbar();
  const plscListbox = plscDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const reopenedOptionLabels = (await plscListbox.getByRole("option").locator("strong").allTextContents())
    .map((label) => label.trim());
  if (reopenedOptionLabels.length !== expectedOptionLabels.length
    || JSON.stringify(reopenedOptionLabels) !== JSON.stringify(expectedOptionLabels)) {
    throw new Error(`The reopened calculation browser did not retain its expected ${expectedOptionLabels.length}-method catalog: ${reopenedOptionLabels.join(" | ")}`);
  }
  await plscListbox.locator("#nd-calculation-method-plsc").click();
  const startPlsc = plscDialog.getByRole("button", { name: "Start consistent PLS", exact: true });
  evidence.checks.plscDialog = {
    selectedMethod: (await plscListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    startEnabled: await startPlsc.isEnabled(),
    blockers: await plscDialog.locator(".nd-blocker li").allTextContents(),
  };
  if (!evidence.checks.plscDialog.startEnabled) throw new Error(`Native Consistent PLS was blocked on the 128-row project: ${evidence.checks.plscDialog.blockers.join(" | ")}`);
  await capture("22-tauri-native-plsc-dialog-1440x900.png");

  const plscProgressCapture = captureActiveCalculation(
    plscDialog,
    "23-tauri-native-running-plsc-1440x900.png",
    "Consistent PLS",
    { allowTerminalTransitionAfterCapture: true },
  );
  await startPlsc.click();
  evidence.checks.plscProgress = await plscProgressCapture;

  await waitForSurface("results", 120_000);
  await page.locator(".nd-run-select select option:checked").filter({ hasText: /Consistent PLS/i }).waitFor({ state: "attached", timeout: 120_000 });
  const plscRunId = await page.locator(".nd-run-select select").inputValue();
  const plscRunLabel = (await page.locator(".nd-run-select select option:checked").textContent())?.trim();
  evidence.checks.plscProgress.completedRunProof = {
    runId: plscRunId,
    runLabel: plscRunLabel,
    matched: Boolean(plscRunId) && /Consistent PLS/i.test(plscRunLabel ?? ""),
  };
  if (!evidence.checks.plscProgress.completedRunProof.matched) {
    throw new Error(`Consistent PLS did not expose its matching completed run immediately after the active lifecycle: ${JSON.stringify(evidence.checks.plscProgress)}`);
  }
  const plscReliabilityRows = await openResultTable("PLSc correction reliability");
  const plscReliabilityText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  const plscCorrelationRows = await openResultTable("PLSc construct correlations");
  const plscCorrelationText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  evidence.checks.plscResult = {
    runId: plscRunId,
    runLabel: plscRunLabel,
    selectedItem: (await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').textContent())?.trim(),
    reliabilityRows: plscReliabilityRows,
    correlationRows: plscCorrelationRows,
    reliabilityText: plscReliabilityText,
    correlationText: plscCorrelationText,
    recordedSeedLabel: await page.getByText("Recorded seed", { exact: true }).count(),
  };
  if (!/Consistent PLS/i.test(plscRunLabel ?? "")) throw new Error(`The selected completed run was not Consistent PLS: ${plscRunLabel ?? "missing label"}`);
  if (!plscReliabilityRows || !plscCorrelationRows) throw new Error("The completed PLSc result did not expose non-empty correction reliability and construct-correlation tables.");
  if (![plscReliabilityText, plscCorrelationText].every((text) => text.includes("Construct 1") && text.includes("Construct 2") && !/construct-/i.test(text))) {
    throw new Error(`The completed PLSc result exposed internal construct identifiers instead of immutable model labels: ${JSON.stringify(evidence.checks.plscResult)}`);
  }
  if (evidence.checks.plscResult.recordedSeedLabel !== 0) throw new Error("The deterministic PLSc result exposed an inapplicable Recorded seed property.");
  await capture("24-tauri-native-plsc-results-1440x900.png");

  const exportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await exportCommand.click();
  const exportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const xlsxExport = exportDialog.getByRole("button", { name: /XLSX workbook/i });
  await xlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const plscExpectedSheets = ["PLSc correction reliability", "PLSc construct correlations", "Run provenance"];
  evidence.checks.plscExport = {
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    expectedRunId: plscRunId,
    formats: await exportDialog.locator(".nd-export-list button").count(),
    xlsxEnabled: await xlsxExport.isEnabled(),
    expectedSheets: plscExpectedSheets,
    nativeXlsx: null,
  };
  if (evidence.checks.plscExport.selectedRunId !== plscRunId
    || evidence.checks.plscExport.formats < 5
    || !evidence.checks.plscExport.xlsxEnabled) {
    throw new Error(`The exact completed PLSc run did not expose the expected enabled output formats: ${JSON.stringify(evidence.checks.plscExport)}`);
  }
  const plscExportTargetPath = await validateRequestedNativeExportPath(
    requestedPlscNativeExportPath,
    "QUICKPLS_PLSC_NATIVE_EXPORT_PATH",
  );
  const plscSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: plscExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: plscExpectedSheets,
    expectedSharedStrings: ["rho_A"],
  });
  let plscHelperCompleted = false;
  try {
    const ready = await plscSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`PLSc native XLSX helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await plscSaveHelper.completed;
    plscHelperCompleted = true;
    if (!completion.passed) throw new Error(`PLSc native XLSX export failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(plscExportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(plscExportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(plscExportTargetPath);
    evidence.checks.plscExport.nativeXlsx = {
      attempted: true,
      targetPath: plscExportTargetPath,
      helper: { ready, completion },
      appFeedback: (await feedback.textContent())?.trim() ?? "",
      file: { path: plscExportTargetPath, size: file.size, isFile: file.isFile() },
      workbookSheets,
      methodSheetsPresentExactlyOnce: plscExpectedSheets.every((sheet) => workbookSheets.filter((candidate) => candidate === sheet).length === 1),
    };
    if (!file.isFile() || file.size <= 0
      || evidence.checks.plscExport.nativeXlsx.appFeedback !== expectedFeedback
      || !evidence.checks.plscExport.nativeXlsx.methodSheetsPresentExactlyOnce) {
      throw new Error(`The packaged PLSc XLSX was not exact and readable: ${JSON.stringify(evidence.checks.plscExport.nativeXlsx)}`);
    }
  } finally {
    if (!plscHelperCompleted) plscSaveHelper.stop();
  }
  await capture("25-tauri-native-export-dialog-1440x900.png");
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.locator(".nd-commandbar button").filter({ hasText: /^Edit Model$/ }).click();
  await waitForSurface("model");
  evidence.checks.returnToModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
  };
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  await page.getByRole("heading", { name: "PLSc construct correlations", exact: true }).waitFor({ state: "visible", timeout: 15_000 });
  evidence.checks.returnToModel.selectedRunRetained = await page.locator(".nd-run-select select").inputValue() === plscRunId;
  if (!evidence.checks.returnToModel.selectedRunRetained) throw new Error("Returning from Results to Model did not retain the completed run selection.");
  await capture("26-tauri-native-returned-plsc-results-1440x900.png");

  const wplsDialog = await openCalculationFromToolbar();
  const wplsListbox = wplsDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await wplsListbox.getByRole("option", { name: /Weighted PLS/i }).click();
  const caseWeight = wplsDialog.locator("#nd-calculation-case-weight");
  await caseWeight.locator('option[value="case_wt"]').waitFor({ state: "attached", timeout: 10_000 });
  const weightOptions = await caseWeight.locator("option").evaluateAll((options) => options.map((option) => ({
    label: option.textContent?.trim() ?? "",
    value: option.value,
    disabled: option.disabled,
  })));
  if (!weightOptions.some((option) => option.value === "case_wt" && !option.disabled)) {
    throw new Error(`The positive numeric case_wt variable was not available for WPLS: ${JSON.stringify(weightOptions)}`);
  }
  await caseWeight.selectOption("case_wt");
  const startWpls = wplsDialog.getByRole("button", { name: "Start weighted PLS", exact: true });
  evidence.checks.wplsDialog = {
    selectedMethod: (await wplsListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    caseWeightColumn: await caseWeight.inputValue(),
    numericWeightOptions: weightOptions.filter((option) => option.value && !option.disabled).map((option) => option.value),
    standardized: (await wplsDialog.locator(".nd-setting-note").filter({ hasText: /Result data/i }).locator("strong").textContent())?.trim(),
    startEnabled: await startWpls.isEnabled(),
    blockers: await wplsDialog.locator(".nd-blocker li").allTextContents(),
  };
  if (!evidence.checks.wplsDialog.startEnabled) throw new Error(`Native Weighted PLS was blocked: ${evidence.checks.wplsDialog.blockers.join(" | ")}`);
  if (!/Standardized \(fixed\)/i.test(evidence.checks.wplsDialog.standardized ?? "")) throw new Error("Weighted PLS did not expose its fixed standardized preprocessing contract.");
  await capture("27-tauri-native-wpls-dialog-1440x900.png");

  const wplsProgressCapture = captureActiveCalculation(
    wplsDialog,
    "28-tauri-native-running-wpls-1440x900.png",
    "Weighted PLS",
    { allowTerminalTransitionAfterCapture: true },
  );
  await startWpls.click();
  evidence.checks.wplsProgress = await wplsProgressCapture;

  await waitForSurface("results");
  await page.locator(".nd-run-select select option:checked").filter({ hasText: /Weighted PLS/i }).waitFor({ state: "attached", timeout: 120_000 });
  const wplsRunId = await page.locator(".nd-run-select select").inputValue();
  const wplsRunLabel = (await page.locator(".nd-run-select select option:checked").textContent())?.trim();
  evidence.checks.wplsProgress.completedRunProof = {
    runId: wplsRunId,
    runLabel: wplsRunLabel,
    matched: Boolean(wplsRunId) && /Weighted PLS/i.test(wplsRunLabel ?? ""),
  };
  if (!evidence.checks.wplsProgress.completedRunProof.matched) {
    throw new Error(`Weighted PLS did not expose its matching completed run immediately after the active lifecycle: ${JSON.stringify(evidence.checks.wplsProgress)}`);
  }
  const weightedPathRows = await openResultTable("Path coefficients");
  const weightedPathText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  evidence.checks.wplsResult = {
    runId: wplsRunId,
    runLabel: wplsRunLabel,
    pathRows: weightedPathRows,
    pathText: weightedPathText,
  };
  if (!/Weighted PLS/i.test(wplsRunLabel ?? "")) throw new Error(`The selected completed run was not Weighted PLS: ${wplsRunLabel ?? "missing label"}`);
  if (!weightedPathRows) throw new Error("The completed Weighted PLS result contained no path-coefficient rows.");
  if (!weightedPathText.includes("Construct 1") || !weightedPathText.includes("Construct 2") || /construct-/i.test(weightedPathText)) {
    throw new Error(`The completed Weighted PLS path table exposed internal construct identifiers: ${weightedPathText}`);
  }
  await capture("29-tauri-native-wpls-results-1440x900.png");

  const resultsGrid = page.locator('.nd-result-table[role="grid"]');
  await resultsGrid.locator('[role="gridcell"][data-native-grid-row="0"][data-native-grid-column="0"]').click();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Control+C");
  await page.waitForFunction(() => Boolean(document.querySelector('.nd-result-table-view [role="status"][aria-live="polite"]')?.textContent?.trim()), null, { timeout: 5_000 });
  evidence.checks.resultsGridKeyboard = await page.evaluate(() => {
    const grid = document.querySelector('.nd-result-table[role="grid"]');
    const active = grid?.querySelector('[role="gridcell"][tabindex="0"]');
    const viewport = grid?.closest('.nd-table-scroll');
    const style = active ? getComputedStyle(active) : null;
    return {
      activeCells: grid?.querySelectorAll('[role="gridcell"][tabindex="0"]').length ?? 0,
      activeRow: active?.getAttribute('data-native-grid-row') ?? null,
      activeColumn: active?.getAttribute('data-native-grid-column') ?? null,
      activeCellFocused: document.activeElement === active,
      activeBoxShadow: style?.boxShadow ?? null,
      viewportOverflowX: viewport ? getComputedStyle(viewport).overflowX : null,
      pageHorizontalOverflow: document.documentElement.scrollWidth > document.documentElement.clientWidth,
    };
  });
  evidence.checks.resultsGridKeyboard.copyAnnouncement = (await page.locator('.nd-result-table-view [role="status"][aria-live="polite"]').textContent())?.trim() ?? "";
  if (evidence.checks.resultsGridKeyboard.activeCells !== 1
    || evidence.checks.resultsGridKeyboard.activeRow !== "0"
    || evidence.checks.resultsGridKeyboard.activeColumn !== "1"
    || !evidence.checks.resultsGridKeyboard.activeCellFocused
    || !evidence.checks.resultsGridKeyboard.activeBoxShadow
    || evidence.checks.resultsGridKeyboard.activeBoxShadow === "none"
    || evidence.checks.resultsGridKeyboard.viewportOverflowX !== "auto"
    || evidence.checks.resultsGridKeyboard.pageHorizontalOverflow
    || !evidence.checks.resultsGridKeyboard.copyAnnouncement) {
    throw new Error(`The native Results grid did not satisfy its keyboard, copy, focus, or contained-scroll contract: ${JSON.stringify(evidence.checks.resultsGridKeyboard)}`);
  }
  await capture("29a-tauri-native-results-grid-active-cell-1440x900.png");

  const wplsDiagnosticRows = await openResultTable("WPLS case-weight diagnostics");
  const wplsDiagnosticText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  evidence.checks.wpls_weights = {
    tableId: "wpls_weights",
    title: "WPLS case-weight diagnostics",
    rows: wplsDiagnosticRows,
    caseWeightColumnVisible: /case_wt/.test(wplsDiagnosticText),
  };
  if (!wplsDiagnosticRows || !evidence.checks.wpls_weights.caseWeightColumnVisible) {
    throw new Error(`The completed WPLS result did not expose truthful wpls_weights diagnostics for case_wt: ${wplsDiagnosticText}`);
  }
  await capture("30-tauri-native-wpls-weights-1440x900.png");

  await exportCommand.click();
  await exportDialog.waitFor({ state: "visible", timeout: 10_000 });
  await xlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const wplsExpectedSheets = ["WPLS case-weight diagnostics", "Run provenance"];
  evidence.checks.wplsExport = {
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    expectedRunId: wplsRunId,
    formats: await exportDialog.locator(".nd-export-list button").count(),
    xlsxEnabled: await xlsxExport.isEnabled(),
    expectedSheets: wplsExpectedSheets,
    nativeXlsx: null,
  };
  if (evidence.checks.wplsExport.selectedRunId !== wplsRunId
    || evidence.checks.wplsExport.formats < 5
    || !evidence.checks.wplsExport.xlsxEnabled) {
    throw new Error(`The exact completed WPLS run did not expose the expected enabled output formats: ${JSON.stringify(evidence.checks.wplsExport)}`);
  }
  const wplsExportTargetPath = await validateRequestedNativeExportPath(
    requestedWplsNativeExportPath,
    "QUICKPLS_WPLS_NATIVE_EXPORT_PATH",
  );
  const wplsSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: wplsExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: wplsExpectedSheets,
    expectedSharedStrings: ["case_wt"],
  });
  let wplsHelperCompleted = false;
  try {
    const ready = await wplsSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`WPLS native XLSX helper did not become ready: ${JSON.stringify(ready)}`);
    await xlsxExport.click();
    const completion = await wplsSaveHelper.completed;
    wplsHelperCompleted = true;
    if (!completion.passed) throw new Error(`WPLS native XLSX export failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(wplsExportTargetPath)}.`;
    const feedback = exportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(wplsExportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(wplsExportTargetPath);
    evidence.checks.wplsExport.nativeXlsx = {
      attempted: true,
      targetPath: wplsExportTargetPath,
      helper: { ready, completion },
      appFeedback: (await feedback.textContent())?.trim() ?? "",
      file: { path: wplsExportTargetPath, size: file.size, isFile: file.isFile() },
      workbookSheets,
      methodSheetsPresentExactlyOnce: wplsExpectedSheets.every((sheet) => workbookSheets.filter((candidate) => candidate === sheet).length === 1),
    };
    if (!file.isFile() || file.size <= 0
      || evidence.checks.wplsExport.nativeXlsx.appFeedback !== expectedFeedback
      || !evidence.checks.wplsExport.nativeXlsx.methodSheetsPresentExactlyOnce) {
      throw new Error(`The packaged WPLS XLSX was not exact and readable: ${JSON.stringify(evidence.checks.wplsExport.nativeXlsx)}`);
    }
  } finally {
    if (!wplsHelperCompleted) wplsSaveHelper.stop();
  }
  await capture("30a-tauri-native-wpls-export-dialog-1440x900.png");
  await exportDialog.getByRole("button", { name: "Close", exact: true }).click();

  const predictionRunDialog = await openCalculationFromToolbar();
  const predictionListbox = predictionRunDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await predictionListbox.getByRole("option", { name: /PLSpredict \/ CVPAT/i }).click();
  const startLargePrediction = predictionRunDialog.getByRole("button", { name: "Start prediction", exact: true });
  evidence.checks.predictionRunnableDialog = {
    selectedMethod: (await predictionListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    startEnabled: await startLargePrediction.isEnabled(),
    blockers: await predictionRunDialog.locator(".nd-blocker li").allTextContents(),
    plan: (await predictionRunDialog.locator("#nd-calculation-prediction-plan strong").textContent())?.trim(),
    seed: Number(await predictionRunDialog.locator("#nd-calculation-seed").inputValue()),
  };
  if (!evidence.checks.predictionRunnableDialog.startEnabled) {
    throw new Error(`Native Prediction was blocked on the 128-row project: ${evidence.checks.predictionRunnableDialog.blockers.join(" | ")}`);
  }
  await capture("31-tauri-native-prediction-runnable-dialog-1440x900.png");

  const predictionProgressCapture = captureActiveCalculation(
    predictionRunDialog,
    "32-tauri-native-running-prediction-1440x900.png",
    "PLSpredict / CVPAT",
    { allowTerminalTransitionAfterCapture: true },
  );
  await startLargePrediction.click();
  evidence.checks.predictionProgress = await predictionProgressCapture;
  await waitForSurface("results");
  await page.locator(".nd-run-select select option:checked").filter({ hasText: /PLSpredict \/ CVPAT/i }).waitFor({ state: "attached", timeout: 120_000 });
  const predictionRunId = await page.locator(".nd-run-select select").inputValue();
  const predictionRunLabel = (await page.locator(".nd-run-select select option:checked").textContent())?.trim();
  evidence.checks.predictionProgress.completedRunProof = {
    runId: predictionRunId,
    runLabel: predictionRunLabel,
    matched: Boolean(predictionRunId) && /PLSpredict \/ CVPAT/i.test(predictionRunLabel ?? ""),
  };
  if (!evidence.checks.predictionProgress.completedRunProof.matched) {
    throw new Error(`PLSpredict / CVPAT did not expose its matching completed run immediately after the active lifecycle: ${JSON.stringify(evidence.checks.predictionProgress)}`);
  }
  const indicatorRows = await openResultTable("Indicator prediction summary (10-fold × 10-repeat)");
  const indicatorText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  const indicatorCells = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => (
    Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
  )));
  await capture("33-tauri-native-prediction-indicator-results-1440x900.png");
  const cvpatRows = await openResultTable("CVPAT benchmark assessment (single model)");
  const cvpatText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  const cvpatCells = await page.locator(".nd-result-table tbody tr").evaluateAll((rows) => rows.map((row) => (
    Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
  )));
  const displayedIndicators = indicatorCells.map((cells) => cells[1] ?? "");
  const noInternalConstructIds = [...indicatorCells, ...cvpatCells]
    .flat()
    .every((cell) => !/construct-/i.test(cell));
  evidence.checks.predictionResult = {
    runId: predictionRunId,
    runLabel: predictionRunLabel,
    indicatorRows,
    cvpatRows,
    indicatorText,
    cvpatText,
    indicatorCells,
    cvpatCells,
    displayedIndicators,
    noInternalConstructIds,
    noPlaceholder: !/\bN\/A\b/i.test(`${indicatorText} ${cvpatText}`),
    singleModelScope: /Indicator average \(IA\)/i.test(cvpatText)
      && /Linear model \(LM\)/i.test(cvpatText)
      && /PLS-SEM loss < benchmark/i.test(cvpatText),
  };
  if (!/PLSpredict \/ CVPAT/i.test(predictionRunLabel ?? "") || !indicatorRows || cvpatRows !== 2
    || !evidence.checks.predictionResult.noPlaceholder || !evidence.checks.predictionResult.singleModelScope) {
    throw new Error(`The completed Prediction run did not expose genuine indicator prediction and two-row single-model CVPAT benchmark outputs: ${JSON.stringify(evidence.checks.predictionResult)}`);
  }
  if (!noInternalConstructIds
    || !indicatorCells.every((cells) => cells[0] === "Construct 2")
    || JSON.stringify(displayedIndicators) !== JSON.stringify(["y1", "y2"])) {
    throw new Error(`The completed Prediction tables exposed internal construct identifiers instead of immutable model labels: ${JSON.stringify(evidence.checks.predictionResult)}`);
  }
  await capture("33a-tauri-native-prediction-cvpat-results-1440x900.png");

  await page.waitForTimeout(2_500);
  const archiveFailureToasts = page.locator(".nd-toast").filter({ hasText: /Recovery save failed|project archive is invalid/i });
  evidence.checks.predictionAutosave = {
    attempted: true,
    archiveFailureToasts: await archiveFailureToasts.allTextContents(),
  };
  if (evidence.checks.predictionAutosave.archiveFailureToasts.length) {
    throw new Error(`Completed Prediction could not be recovery-saved: ${evidence.checks.predictionAutosave.archiveFailureToasts.join(" | ")}`);
  }

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await reloadToLauncher();
  await openDisposableRecentProject();
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  await page.locator(".nd-results-nav").waitFor({ state: "visible", timeout: 15_000 });
  await page.locator(".nd-run-select select option").first().waitFor({ state: "attached", timeout: 15_000 });
  const reopenedRunOptions = await page.locator(".nd-run-select select option").allTextContents();
  evidence.checks.completedResultsSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    runOptions: reopenedRunOptions.map((label) => label.trim()),
    hasPlsc: reopenedRunOptions.some((label) => /Consistent PLS/i.test(label)),
    hasWpls: reopenedRunOptions.some((label) => /Weighted PLS/i.test(label)),
    hasPrediction: reopenedRunOptions.some((label) => /PLSpredict \/ CVPAT/i.test(label)),
  };
  if (!evidence.checks.completedResultsSaveReopen.hasPlsc
    || !evidence.checks.completedResultsSaveReopen.hasWpls
    || !evidence.checks.completedResultsSaveReopen.hasPrediction) {
    throw new Error(`Completed method results did not survive explicit save/reload/reopen: ${JSON.stringify(evidence.checks.completedResultsSaveReopen)}`);
  }

  const reopenedPlscOption = page.locator(".nd-run-select select option").filter({ hasText: /Consistent PLS/i }).first();
  await reopenedPlscOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedPlscRunId = await reopenedPlscOption.getAttribute("value");
  if (!reopenedPlscRunId) throw new Error("The reopened PLSc option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedPlscRunId);
  const reopenedPlscReliabilityRows = await openResultTable("PLSc correction reliability");
  const reopenedPlscCorrelationRows = await openResultTable("PLSc construct correlations");
  const reopenedPlscText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  evidence.checks.plscSaveReopen = {
    attempted: true,
    expectedRunId: plscRunId,
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    sameRunRestored: reopenedPlscRunId === plscRunId && await page.locator(".nd-run-select select").inputValue() === plscRunId,
    reliabilityRows: reopenedPlscReliabilityRows,
    correlationRows: reopenedPlscCorrelationRows,
    immutableLabelsRestored: reopenedPlscText.includes("Construct 1") && reopenedPlscText.includes("Construct 2") && !/construct-/i.test(reopenedPlscText),
  };
  if (!evidence.checks.plscSaveReopen.sameRunRestored
    || !reopenedPlscReliabilityRows
    || !reopenedPlscCorrelationRows
    || !evidence.checks.plscSaveReopen.immutableLabelsRestored) {
    throw new Error(`The exact PLSc run and result tables did not survive save/reload/reopen: ${JSON.stringify(evidence.checks.plscSaveReopen)}`);
  }

  const reopenedWplsOption = page.locator(".nd-run-select select option").filter({ hasText: /Weighted PLS/i }).first();
  await reopenedWplsOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedWplsRunId = await reopenedWplsOption.getAttribute("value");
  if (!reopenedWplsRunId) throw new Error("The reopened WPLS option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedWplsRunId);
  const reopenedWplsPathRows = await openResultTable("Path coefficients");
  const reopenedWplsPathText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  const reopenedWplsDiagnosticRows = await openResultTable("WPLS case-weight diagnostics");
  const reopenedWplsDiagnosticText = (await page.locator(".nd-result-table tbody").textContent())?.trim() ?? "";
  evidence.checks.wplsSaveReopen = {
    attempted: true,
    expectedRunId: wplsRunId,
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    sameRunRestored: reopenedWplsRunId === wplsRunId && await page.locator(".nd-run-select select").inputValue() === wplsRunId,
    pathRows: reopenedWplsPathRows,
    diagnosticRows: reopenedWplsDiagnosticRows,
    immutableLabelsRestored: reopenedWplsPathText.includes("Construct 1") && reopenedWplsPathText.includes("Construct 2") && !/construct-/i.test(reopenedWplsPathText),
    caseWeightColumnRestored: /case_wt/.test(reopenedWplsDiagnosticText),
  };
  if (!evidence.checks.wplsSaveReopen.sameRunRestored
    || !reopenedWplsPathRows
    || !reopenedWplsDiagnosticRows
    || !evidence.checks.wplsSaveReopen.immutableLabelsRestored
    || !evidence.checks.wplsSaveReopen.caseWeightColumnRestored) {
    throw new Error(`The exact WPLS run and result tables did not survive save/reload/reopen: ${JSON.stringify(evidence.checks.wplsSaveReopen)}`);
  }

  const reopenedPredictionOption = page.locator(".nd-run-select select option").filter({ hasText: /PLSpredict \/ CVPAT/i }).first();
  await reopenedPredictionOption.waitFor({ state: "attached", timeout: 15_000 });
  await page.locator(".nd-run-select select").selectOption(await reopenedPredictionOption.getAttribute("value"));
  const reopenedPredictionRows = await openResultTable("Indicator prediction summary (10-fold × 10-repeat)");
  const reopenedCvpatRows = await openResultTable("CVPAT benchmark assessment (single model)");
  if (!reopenedPredictionRows || reopenedCvpatRows !== 2) throw new Error("The reopened Prediction run did not retain indicator-level PLSpredict and two-row CVPAT benchmark output.");
  await capture("34-tauri-native-prediction-results-reopened-1440x900.png");

  const bootstrapDialog = await openCalculationFromToolbar();
  const bootstrapListbox = bootstrapDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await bootstrapListbox.getByRole("option", { name: /PLS-SEM Bootstrapping/i }).click();
  await bootstrapDialog.locator("#nd-calculation-bootstrap-samples").fill("1000");
  await bootstrapDialog.locator("#nd-calculation-studentized").selectOption("999");
  const cancelledBootstrapSetup = {
    bootstrapSamples: await bootstrapDialog.locator("#nd-calculation-bootstrap-samples").inputValue(),
    confidenceLevel: await bootstrapDialog.locator("#nd-calculation-confidence").inputValue(),
    studentizedInnerSamples: await bootstrapDialog.locator("#nd-calculation-studentized").inputValue(),
    seed: await bootstrapDialog.locator("#nd-calculation-seed").inputValue(),
    workers: await bootstrapDialog.locator("#nd-calculation-workers").inputValue(),
  };
  const startBootstrap = bootstrapDialog.getByRole("button", { name: "Start bootstrapping", exact: true });
  if (!await startBootstrap.isEnabled()) {
    throw new Error(`The native studentized Bootstrap acceptance job was blocked: ${(await bootstrapDialog.locator(".nd-blocker li").allTextContents()).join(" | ")}`);
  }
  const bootstrapRunning = bootstrapDialog.locator(".nd-run-progress.running");
  const bootstrapRunningWait = bootstrapRunning.waitFor({ state: "visible", timeout: 20_000 });
  await startBootstrap.click();
  await bootstrapRunningWait;
  evidence.checks.bootstrapRunning = await bootstrapRunning.evaluate((element) => ({
    phase: element.querySelector("strong")?.textContent?.trim() ?? "",
    message: element.querySelector("p")?.textContent?.trim() ?? "",
    progressValue: element.querySelector("progress")?.getAttribute("value") ?? null,
    progressMax: element.querySelector("progress")?.getAttribute("max") ?? null,
  }));
  evidence.checks.bootstrapRunning.requestedSettings = cancelledBootstrapSetup;
  await capture("35-tauri-native-running-bootstrap-1440x900.png");
  await bootstrapDialog.getByRole("button", { name: "Cancel calculation", exact: true }).click();
  const cancelledBootstrap = bootstrapDialog.locator(".nd-run-progress.cancelled");
  await cancelledBootstrap.waitFor({ state: "visible", timeout: 30_000 });
  evidence.checks.bootstrapCancelled = {
    status: "cancelled",
    partialRunVisible: await page.locator(".nd-run-select select option").filter({ hasText: /Bootstrapping/i }).count(),
  };
  if (evidence.checks.bootstrapCancelled.partialRunVisible !== 0) {
    throw new Error("The cancelled Bootstrap job appeared as a completed Results run.");
  }
  await capture("36-tauri-native-cancelled-bootstrap-1440x900.png");
  await bootstrapDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.locator(".nd-run-select select").selectOption(wplsRunId);
  const finalWplsRows = await openResultTable("WPLS case-weight diagnostics");
  evidence.checks.finalNativeState = {
    runId: await page.locator(".nd-run-select select").inputValue(),
    title: (await page.getByRole("heading", { name: "WPLS case-weight diagnostics", exact: true }).textContent())?.trim(),
    rows: finalWplsRows,
    status: (await page.locator(".nd-statusbar strong").textContent())?.trim(),
  };
  if (evidence.checks.finalNativeState.runId !== wplsRunId || !finalWplsRows || evidence.checks.finalNativeState.status !== "Ready") {
    throw new Error(`The final native evidence state was not the completed WPLS diagnostics: ${JSON.stringify(evidence.checks.finalNativeState)}`);
  }
  await capture("37-tauri-native-final-wpls-results-1440x900.png");

  // Project Explorer milestone. All mutations use visible native commands and
  // the disposable project already exercised above. The original model is
  // deleted only after every scientific method check is complete, so this
  // section cannot weaken or short-circuit the calculation evidence.
  const initialReportName = "WPLS Review";
  const persistedReportName = "Weighted Diagnostics Review";
  const initialSecondModelName = "Alternative Model";
  const persistedSecondModelName = "Exploratory Model";
  const saveReportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Save Report/i });
  if (await saveReportCommand.count() !== 1 || !await saveReportCommand.isEnabled()) {
    throw new Error("The completed unsaved WPLS result did not expose exactly one enabled Save Report command.");
  }
  await saveReportCommand.click();
  await submitNamedExplorerDialog("Save Report", initialReportName, "Save");

  let workspaceTree = await openWorkspaceExplorer();
  await workspaceTree.waitFor({ state: "visible", timeout: 15_000 });
  const initialModelItems = workspaceTreeItem("model");
  const originalModelName = disposableModelName;
  if (await initialModelItems.count() !== 1 || await workspaceTreeItem("model", originalModelName).count() !== 1) {
    throw new Error(`The disposable project did not begin the multi-model Explorer workflow with exactly one ${originalModelName} model; found ${(await initialModelItems.allTextContents()).map((label) => label.trim()).join(", ") || "none"}.`);
  }

  const savedReportItem = workspaceTreeItem("report", initialReportName);
  await savedReportItem.waitFor({ state: "visible", timeout: 10_000 });
  await savedReportItem.click();
  await savedReportItem.focus();
  await page.keyboard.press("F2");
  await submitNamedExplorerDialog("Rename Report", persistedReportName, "Rename");
  const renamedReportItem = workspaceTreeItem("report", persistedReportName);
  await renamedReportItem.waitFor({ state: "visible", timeout: 10_000 });

  await renamedReportItem.focus();
  await page.keyboard.press("Shift+F10");
  const reportContextMenu = page.getByRole("menu", { name: "Project item commands", exact: true });
  await reportContextMenu.waitFor({ state: "visible", timeout: 5_000 });
  const reportContextLabels = (await reportContextMenu.getByRole("menuitem").allTextContents()).map((label) => label.trim());
  evidence.checks.workspaceExplorerContextMenu = {
    commands: reportContextLabels,
    openReport: reportContextLabels.some((label) => /^Open Report/i.test(label)),
    renameReport: reportContextLabels.some((label) => /^Rename Report/i.test(label)),
    removeReport: reportContextLabels.some((label) => /^Remove Report/i.test(label)),
  };
  await page.keyboard.press("Escape");
  await reportContextMenu.waitFor({ state: "hidden", timeout: 5_000 });
  evidence.checks.workspaceExplorerContextMenu.focusRestored = await renamedReportItem.evaluate((item) => document.activeElement === item);
  if (!evidence.checks.workspaceExplorerContextMenu.openReport
    || !evidence.checks.workspaceExplorerContextMenu.renameReport
    || !evidence.checks.workspaceExplorerContextMenu.removeReport
    || !evidence.checks.workspaceExplorerContextMenu.focusRestored) {
    throw new Error(`The saved-report context menu was incomplete or did not restore tree focus: ${JSON.stringify(evidence.checks.workspaceExplorerContextMenu)}`);
  }

  await workspaceTreeItem("models", "Models").click();
  const newModelDetailCommand = page.locator(".nd-explorer-detail-actions").getByRole("button", { name: "New Model", exact: true });
  if (!await newModelDetailCommand.isEnabled()) throw new Error("The writable Models folder did not expose an enabled New Model action.");
  await newModelDetailCommand.click();
  await submitNamedExplorerDialog("New Model", initialSecondModelName, "Create");
  await waitForSurface("model");
  await page.locator(".react-flow__pane").waitFor({ state: "visible", timeout: 15_000 });
  const createdSecondModel = {
    tab: await currentModelDocumentName(),
    constructs: await page.locator(".react-flow__node-latent").count(),
    structuralPaths: await structuralPaths().count(),
  };
  if (createdSecondModel.tab !== initialSecondModelName || createdSecondModel.constructs !== 0 || createdSecondModel.structuralPaths !== 0) {
    throw new Error(`The newly created model was not an independent empty canvas: ${JSON.stringify(createdSecondModel)}`);
  }
  await capture("37a-tauri-native-workspace-explorer-new-empty-model-1440x900.png");

  workspaceTree = await openWorkspaceExplorer();
  const initialSecondModelItem = workspaceTreeItem("model", initialSecondModelName);
  await initialSecondModelItem.waitFor({ state: "visible", timeout: 10_000 });
  await initialSecondModelItem.click();
  await initialSecondModelItem.focus();
  await page.keyboard.press("F2");
  await submitNamedExplorerDialog("Rename Model", persistedSecondModelName, "Rename");
  const persistedSecondModelItem = workspaceTreeItem("model", persistedSecondModelName);
  await persistedSecondModelItem.waitFor({ state: "visible", timeout: 10_000 });

  const explorerTreeContract = await workspaceTree.evaluate((tree) => ({
    treeItems: tree.querySelectorAll('[role="treeitem"]').length,
    tabStops: tree.querySelectorAll('[role="treeitem"][tabindex="0"]').length,
    selectedItems: tree.querySelectorAll('[role="treeitem"][aria-selected="true"]').length,
    activeModels: tree.querySelectorAll('[role="treeitem"].active-model').length,
    labels: Array.from(tree.querySelectorAll('.nd-tree-label')).map((label) => label.textContent?.trim() ?? ""),
    levels: Array.from(tree.querySelectorAll('[role="treeitem"]')).map((item) => item.getAttribute("aria-level")),
  }));
  evidence.checks.workspaceExplorerCatalog = {
    originalModelName,
    secondModelName: persistedSecondModelName,
    reportName: persistedReportName,
    ...explorerTreeContract,
  };
  if (explorerTreeContract.tabStops !== 1
    || explorerTreeContract.selectedItems !== 1
    || explorerTreeContract.activeModels !== 1
    || ![path.basename(fixtureCsvPath), "Models", "Reports", originalModelName, persistedSecondModelName, persistedReportName]
      .every((label) => explorerTreeContract.labels.includes(label))) {
    throw new Error(`The two-model Project Explorer did not satisfy its compact tree contract: ${JSON.stringify(evidence.checks.workspaceExplorerCatalog)}`);
  }
  await capture("37b-tauri-native-workspace-explorer-two-models-report-1440x900.png");

  const originalModelItem = workspaceTreeItem("model", originalModelName);
  await originalModelItem.dblclick();
  await waitForSurface("model");
  await page.locator(".react-flow__node-latent").first().waitFor({ state: "visible", timeout: 15_000 });
  const originalPresentationBeforeSave = {
    tab: await currentModelDocumentName(),
    constructs: await page.locator(".react-flow__node-latent").count(),
    structuralPaths: await structuralPaths().count(),
  };
  workspaceTree = await openWorkspaceExplorer();
  await workspaceTreeItem("model", persistedSecondModelName).dblclick();
  await waitForSurface("model");
  await page.locator(".react-flow__pane").waitFor({ state: "visible", timeout: 15_000 });
  const secondPresentationBeforeSave = {
    tab: await currentModelDocumentName(),
    constructs: await page.locator(".react-flow__node-latent").count(),
    structuralPaths: await structuralPaths().count(),
  };
  if (originalPresentationBeforeSave.tab !== originalModelName
    || originalPresentationBeforeSave.constructs !== 2
    || originalPresentationBeforeSave.structuralPaths !== 1
    || secondPresentationBeforeSave.tab !== persistedSecondModelName
    || secondPresentationBeforeSave.constructs !== 0
    || secondPresentationBeforeSave.structuralPaths !== 0) {
    throw new Error(`Switching models did not restore two distinct live presentations: ${JSON.stringify({ originalPresentationBeforeSave, secondPresentationBeforeSave })}`);
  }

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await reloadToLauncher();
  await openDisposableRecentProject();
  workspaceTree = await openWorkspaceExplorer();
  await workspaceTreeItem("model", originalModelName).waitFor({ state: "visible", timeout: 15_000 });
  await workspaceTreeItem("model", persistedSecondModelName).waitFor({ state: "visible", timeout: 15_000 });
  await workspaceTreeItem("report", persistedReportName).waitFor({ state: "visible", timeout: 15_000 });

  await workspaceTreeItem("model", originalModelName).dblclick();
  await waitForSurface("model");
  await page.locator(".react-flow__node-latent").first().waitFor({ state: "visible", timeout: 15_000 });
  const originalPresentationAfterReopen = {
    tab: await currentModelDocumentName(),
    constructs: await page.locator(".react-flow__node-latent").count(),
    structuralPaths: await structuralPaths().count(),
  };
  workspaceTree = await openWorkspaceExplorer();
  await workspaceTreeItem("model", persistedSecondModelName).dblclick();
  await waitForSurface("model");
  await page.locator(".react-flow__pane").waitFor({ state: "visible", timeout: 15_000 });
  const secondPresentationAfterReopen = {
    tab: await currentModelDocumentName(),
    constructs: await page.locator(".react-flow__node-latent").count(),
    structuralPaths: await structuralPaths().count(),
  };
  evidence.checks.workspaceExplorerSaveReopen = {
    reopenedThroughRecentProjectRow: true,
    models: [originalModelName, persistedSecondModelName],
    report: persistedReportName,
    originalPresentationBeforeSave,
    secondPresentationBeforeSave,
    originalPresentationAfterReopen,
    secondPresentationAfterReopen,
  };
  if (JSON.stringify(originalPresentationAfterReopen) !== JSON.stringify(originalPresentationBeforeSave)
    || JSON.stringify(secondPresentationAfterReopen) !== JSON.stringify(secondPresentationBeforeSave)) {
    throw new Error(`The named model catalog or distinct presentations did not survive save/reopen: ${JSON.stringify(evidence.checks.workspaceExplorerSaveReopen)}`);
  }

  workspaceTree = await openWorkspaceExplorer();
  const deletableOriginalModel = workspaceTreeItem("model", originalModelName);
  await deletableOriginalModel.click();
  await deletableOriginalModel.focus();
  await page.keyboard.press("Delete");
  const deleteModelDialog = page.getByRole("dialog", { name: "Delete Model", exact: true });
  await deleteModelDialog.waitFor({ state: "visible", timeout: 10_000 });
  const deleteModelDescription = (await deleteModelDialog.textContent())?.replace(/\s+/g, " ").trim() ?? "";
  if (!/Completed results remain in run history/i.test(deleteModelDescription)) {
    throw new Error(`The Delete Model confirmation did not preserve-result semantics: ${deleteModelDescription}`);
  }
  await confirmExplorerRemoval("Delete Model", "Delete");
  await waitForSurface("launcher");
  await workspaceTreeItem("model", originalModelName).waitFor({ state: "hidden", timeout: 10_000 });
  await workspaceTreeItem("report", persistedReportName).waitFor({ state: "visible", timeout: 10_000 });
  const remainingModelNames = (await workspaceTreeItem("model").allTextContents()).map((label) => label.trim());
  if (remainingModelNames.length !== 1 || remainingModelNames[0] !== persistedSecondModelName) {
    throw new Error(`Deleting the historical model did not leave the independent model intact: ${JSON.stringify(remainingModelNames)}`);
  }
  await capture("37c-tauri-native-workspace-explorer-model-deleted-report-preserved-1440x900.png");

  await workspaceTreeItem("report", persistedReportName).dblclick();
  await waitForSurface("results");
  await page.locator(".nd-run-select select").waitFor({ state: "visible", timeout: 15_000 });
  const preservedReportRunId = await page.locator(".nd-run-select select").inputValue();
  const preservedReportRows = await openResultTable("WPLS case-weight diagnostics");
  const resultModelCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Model$/ });
  const resultDataCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Data$/ });
  const editModelCount = await resultModelCommand.count();
  const editDataCount = await resultDataCommand.count();
  evidence.checks.workspaceExplorerHistoricalResult = {
    deletedModel: originalModelName,
    remainingModels: remainingModelNames,
    report: persistedReportName,
    selectedRunId: preservedReportRunId,
    expectedRunId: wplsRunId,
    rows: preservedReportRows,
    editModelCount,
    editDataCount,
    editDataVisible: editDataCount === 1 && await resultDataCommand.isVisible(),
    editDataEnabled: editDataCount === 1 && await resultDataCommand.isEnabled(),
  };
  if (preservedReportRunId !== wplsRunId
    || !preservedReportRows
    || evidence.checks.workspaceExplorerHistoricalResult.editModelCount !== 0
    || evidence.checks.workspaceExplorerHistoricalResult.editDataCount !== 1
    || !evidence.checks.workspaceExplorerHistoricalResult.editDataVisible
    || !evidence.checks.workspaceExplorerHistoricalResult.editDataEnabled) {
    throw new Error(`Deleting a model damaged its historical result/report contract: ${JSON.stringify(evidence.checks.workspaceExplorerHistoricalResult)}`);
  }
  await capture("37d-tauri-native-workspace-explorer-historical-report-open-1440x900.png");

  workspaceTree = await openWorkspaceExplorer();
  const removableReport = workspaceTreeItem("report", persistedReportName);
  await removableReport.click();
  await removableReport.focus();
  await page.keyboard.press("Delete");
  const removeReportDialog = page.getByRole("dialog", { name: "Remove Report", exact: true });
  await removeReportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const removeReportDescription = (await removeReportDialog.textContent())?.replace(/\s+/g, " ").trim() ?? "";
  if (!/completed result remains in run history/i.test(removeReportDescription)) {
    throw new Error(`The Remove Report confirmation did not preserve-result semantics: ${removeReportDescription}`);
  }
  await confirmExplorerRemoval("Remove Report", "Remove");
  await workspaceTreeItem("report").waitFor({ state: "hidden", timeout: 10_000 });
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const postRemovalRunOptions = await page.locator(".nd-run-select select option").evaluateAll((options) => options.map((option) => ({
    value: option.value,
    label: option.textContent?.trim() ?? "",
  })));
  if (!postRemovalRunOptions.some((option) => option.value === wplsRunId)) {
    throw new Error(`Removing a report alias also removed its canonical WPLS result: ${JSON.stringify(postRemovalRunOptions)}`);
  }
  await page.locator(".nd-run-select select").selectOption(wplsRunId);
  const postRemovalRows = await openResultTable("WPLS case-weight diagnostics");
  const postRemovalSaveReportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Save Report/i });
  evidence.checks.workspaceExplorerReportRemoval = {
    aliasesRemaining: 0,
    canonicalRunRetained: postRemovalRunOptions.some((option) => option.value === wplsRunId),
    rows: postRemovalRows,
    saveReportAvailableAgain: await postRemovalSaveReportCommand.count() === 1 && await postRemovalSaveReportCommand.isEnabled(),
  };
  if (!postRemovalRows || !evidence.checks.workspaceExplorerReportRemoval.saveReportAvailableAgain) {
    throw new Error(`The canonical result was not independently usable after removing its report alias: ${JSON.stringify(evidence.checks.workspaceExplorerReportRemoval)}`);
  }
  await capture("37e-tauri-native-workspace-explorer-report-removed-result-retained-1440x900.png");

  // Genuine mediation milestone. This uses the checked-in 240-row latent
  // mediation dataset and creates every construct, indicator assignment, and
  // structural path through the same visible native workbench actions a user
  // performs. x3/m3/y3 intentionally remain unassigned.
  await openMenuItem("View", "Project");
  await waitForSurface("launcher");
  await seedRecentProject({
    name: mediationProjectName,
    path: mediationProjectPath,
    openedAt: "2026-08-11T00:00:00.000Z",
  });
  await reloadToLauncher();
  const mediationRecentRow = exactRecentProjectRow(mediationProjectName, mediationProjectPath);
  await mediationRecentRow.waitFor({ state: "visible", timeout: 10_000 });
  evidence.checks.mediationRecentProject = {
    visibleRows: await mediationRecentRow.count(),
    pathVisible: (await mediationRecentRow.textContent())?.includes(mediationProjectPath) ?? false,
    projectPath: mediationProjectPath,
  };
  if (evidence.checks.mediationRecentProject.visibleRows !== 1 || !evidence.checks.mediationRecentProject.pathVisible) {
    throw new Error("The disposable mediation project was not exposed through one truthful visible Recent Projects row.");
  }
  await capture(mediationCaptureName(38, "mediation-seeded-recent-project"));

  await openRecentProject(mediationProjectName, mediationProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const mediationDatasetStatus = (await page.locator(".nd-statusbar").textContent())?.trim() ?? "";
  const mediationColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map((column) => column.trim());
  evidence.checks.mediationDataset = {
    cases: mediationDatasetStatus.includes("240 cases") ? 240 : null,
    columns: mediationColumns,
    status: mediationDatasetStatus,
    sourceCsv: mediationFixtureCsvPath,
  };
  const requiredMediationColumns = ["x1", "x2", "m1", "m2", "y1", "y2"];
  if (evidence.checks.mediationDataset.cases !== 240
    || requiredMediationColumns.some((column) => !mediationColumns.includes(column))) {
    throw new Error(`The visible mediation project did not load the tracked 240-row fixture: ${JSON.stringify(evidence.checks.mediationDataset)}`);
  }
  await capture(mediationCaptureName(39, "mediation-fixture-data"));

  evidence.checks.initialMediationModelCreation = await createInitialEditableModel(mediationProjectName, mediationModelName);
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const invalidPlsArchiveBefore = await inspectMediationArchiveRunState(mediationProjectPath);

  const invalidBootstrapDialog = await openCalculationFromToolbar();
  const invalidBootstrapListbox = invalidBootstrapDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await invalidBootstrapListbox.getByRole("option", { name: /PLS-SEM Bootstrapping/i }).click();
  const invalidBootstrapStart = invalidBootstrapDialog.getByRole("button", { name: "Start bootstrapping", exact: true });
  const invalidBootstrapSelectedMethod = (await invalidBootstrapListbox.getByRole("option", { selected: true }).textContent())?.trim() ?? "";
  const invalidBootstrapStartEnabled = await invalidBootstrapStart.isEnabled();
  const invalidBootstrapBlockers = (await invalidBootstrapDialog.locator(".nd-blocker li").allTextContents()).map((row) => row.trim()).filter(Boolean);
  await capture(mediationCaptureName(39, "mediation-bootstrap-invalid-setup"));
  await invalidBootstrapDialog.getByRole("button", { name: "Close", exact: true }).click();
  await invalidBootstrapDialog.waitFor({ state: "hidden", timeout: 10_000 });

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const invalidBootstrapArchiveAfter = await inspectMediationArchiveRunState(mediationProjectPath);
  const invalidBootstrapRunStateUnchanged = JSON.stringify(invalidBootstrapArchiveAfter) === JSON.stringify(invalidPlsArchiveBefore);
  const invalidBootstrapResultCreated = invalidBootstrapArchiveAfter.resultCount > invalidPlsArchiveBefore.resultCount
    || invalidBootstrapArchiveAfter.resultIds.some((runId) => !invalidPlsArchiveBefore.resultIds.includes(runId));
  evidence.checks.bootstrapInvalidSetup = {
    attempted: true,
    selectedMethod: invalidBootstrapSelectedMethod,
    startEnabled: invalidBootstrapStartEnabled,
    blockers: invalidBootstrapBlockers,
    archiveBefore: invalidPlsArchiveBefore,
    archiveAfter: invalidBootstrapArchiveAfter,
    runStateUnchanged: invalidBootstrapRunStateUnchanged,
    resultCreated: invalidBootstrapResultCreated,
  };
  if (!invalidBootstrapSelectedMethod.includes("PLS-SEM Bootstrapping")
    || invalidBootstrapStartEnabled || invalidBootstrapBlockers.length === 0
    || invalidPlsArchiveBefore.recipeCount !== 0 || invalidPlsArchiveBefore.resultCount !== 0
    || invalidPlsArchiveBefore.runCount !== 0 || !invalidBootstrapRunStateUnchanged
    || invalidBootstrapResultCreated) {
    throw new Error(`Invalid empty-model Bootstrap setup did not remain blocked without creating a run or result: ${JSON.stringify(evidence.checks.bootstrapInvalidSetup)}`);
  }

  const invalidPlsDialog = await openCalculationFromToolbar();
  const invalidPlsListbox = invalidPlsDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await invalidPlsListbox.getByRole("option", { name: /PLS-SEM Algorithm/i }).click();
  const invalidPlsStart = invalidPlsDialog.getByRole("button", { name: "Start calculation", exact: true });
  const invalidPlsSelectedMethod = (await invalidPlsListbox.getByRole("option", { selected: true }).textContent())?.trim() ?? "";
  const invalidPlsStartEnabled = await invalidPlsStart.isEnabled();
  const invalidPlsBlockers = (await invalidPlsDialog.locator(".nd-blocker li").allTextContents()).map((row) => row.trim()).filter(Boolean);
  await capture(mediationCaptureName(39, "mediation-pls-invalid-setup"));
  await invalidPlsDialog.getByRole("button", { name: "Close", exact: true }).click();
  await invalidPlsDialog.waitFor({ state: "hidden", timeout: 10_000 });

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const invalidPlsArchiveAfter = await inspectMediationArchiveRunState(mediationProjectPath);
  const invalidPlsRunStateUnchanged = JSON.stringify(invalidPlsArchiveAfter) === JSON.stringify(invalidPlsArchiveBefore);
  const invalidPlsResultCreated = invalidPlsArchiveAfter.resultCount > invalidPlsArchiveBefore.resultCount
    || invalidPlsArchiveAfter.resultIds.some((runId) => !invalidPlsArchiveBefore.resultIds.includes(runId));
  evidence.checks.plsAlgorithmInvalidSetup = {
    attempted: true,
    selectedMethod: invalidPlsSelectedMethod,
    startEnabled: invalidPlsStartEnabled,
    blockers: invalidPlsBlockers,
    archiveBefore: invalidPlsArchiveBefore,
    archiveAfter: invalidPlsArchiveAfter,
    runStateUnchanged: invalidPlsRunStateUnchanged,
    resultCreated: invalidPlsResultCreated,
  };
  if (!invalidPlsSelectedMethod.includes("PLS-SEM Algorithm")
    || invalidPlsStartEnabled
    || invalidPlsBlockers.length === 0
    || invalidPlsArchiveBefore.recipeCount !== 0
    || invalidPlsArchiveBefore.resultCount !== 0
    || invalidPlsArchiveBefore.runCount !== 0
    || !invalidPlsRunStateUnchanged
    || invalidPlsResultCreated) {
    throw new Error(`Invalid empty-model PLS setup did not remain blocked without creating a run or result: ${JSON.stringify(evidence.checks.plsAlgorithmInvalidSetup)}`);
  }

  await buildThreeConstructMediationModel();
  const spareIndicators = page.locator(".nd-variable-item").filter({ hasText: /^(x3|m3|y3)$/ });
  evidence.checks.visibleMediationModelBuild = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    constructLabels: (await page.locator(".react-flow__node-latent").allTextContents()).map((label) => label.trim()),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    unassignedSpareIndicators: await spareIndicators.evaluateAll((elements) => elements.filter((element) => !element.classList.contains("assigned")).map((element) => element.textContent?.trim() ?? "")),
  };
  if (evidence.checks.visibleMediationModelBuild.constructs !== 3
    || evidence.checks.visibleMediationModelBuild.assignedIndicators !== 6
    || evidence.checks.visibleMediationModelBuild.structuralPaths !== 2
    || evidence.checks.visibleMediationModelBuild.unassignedSpareIndicators.length !== 3) {
    throw new Error(`The visible Model workflow did not create the expected x1/x2 -> m1/m2 -> y1/y2 chain: ${JSON.stringify(evidence.checks.visibleMediationModelBuild)}`);
  }
  await capture(mediationCaptureName(40, "mediation-model-built"));

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const mediationPlsDialog = await openCalculationFromToolbar();
  const mediationPlsListbox = mediationPlsDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await mediationPlsListbox.getByRole("option", { name: /PLS-SEM Algorithm/i }).click();
  const startMediationPls = mediationPlsDialog.getByRole("button", { name: "Start calculation", exact: true });
  evidence.checks.mediationPlsDialog = {
    selectedMethod: (await mediationPlsListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    startEnabled: await startMediationPls.isEnabled(),
    blockers: await mediationPlsDialog.locator(".nd-blocker li").allTextContents(),
  };
  if (!evidence.checks.mediationPlsDialog.startEnabled) {
    throw new Error(`Native PLS was blocked on the 240-row mediation project: ${evidence.checks.mediationPlsDialog.blockers.join(" | ")}`);
  }
  await capture(mediationCaptureName(41, "mediation-pls-dialog"));
  await startMediationPls.click();

  await waitForSurface("results", 120_000);
  await page.locator(".nd-run-select select option:checked").filter({ hasText: /PLS-SEM Algorithm/i }).waitFor({ state: "attached", timeout: 120_000 });
  const mediationPlsRunId = await page.locator(".nd-run-select select").inputValue();
  evidence.checks.mediationPlsResult = {
    runId: mediationPlsRunId,
    runLabel: (await page.locator(".nd-run-select select option:checked").textContent())?.trim() ?? "",
    navigation: await inspectMediationResultTree({ withBootstrap: false }),
  };
  const plsSpecificText = evidence.checks.mediationPlsResult.navigation.tableText["Specific indirect effects"];
  if (!["Construct 1", "Construct 2", "Construct 3"].every((construct) => plsSpecificText.includes(construct)) || /\bN\/A\b/i.test(plsSpecificText)) {
    throw new Error(`The PLS specific-indirect table did not identify the three visible indicator-block constructs: ${plsSpecificText}`);
  }
  await capture(mediationCaptureName(42, "mediation-pls-results"));

  const mediationExportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await mediationExportCommand.click();
  const mediationExportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await mediationExportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const mediationXlsxExport = mediationExportDialog.getByRole("button", { name: /XLSX workbook/i });
  await mediationXlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  evidence.checks.mediationExport = {
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    formats: await mediationExportDialog.locator(".nd-export-list button").count(),
    xlsxEnabled: await mediationXlsxExport.isEnabled(),
    selectedResultTable: evidence.checks.mediationPlsResult.navigation.selectedTable,
  };
  if (evidence.checks.mediationExport.selectedRunId !== mediationPlsRunId
    || evidence.checks.mediationExport.formats < 5
    || !evidence.checks.mediationExport.xlsxEnabled) {
    throw new Error(`The exact completed PLS Algorithm run did not expose the expected enabled native export formats: ${JSON.stringify(evidence.checks.mediationExport)}`);
  }

  if (requestedNativeExportPath) {
    const targetPath = await validateRequestedNativeExportPath(requestedNativeExportPath);
    const nativeSaveHelper = startWindowsNativeSaveExportHelper({
      targetPath,
      windowTitle: evidence.checks.runtime.title,
      expectedSheets: [
        "Direct effects",
        "Outer loadings",
        "Outer weights",
        "R-square",
        "Specific indirect effects",
        "Total indirect effects",
        "Total effects",
        "Construct reliability and valid",
        "Cross loadings",
        "Fornell-Larcker criterion",
        "HTMT+",
        "Original HTMT",
        "Structural model",
        "Inner VIF values",
        "f-square effect sizes",
        "Model fit",
        "Construct cross-validated redun",
        "Run provenance",
      ],
      expectedSharedStrings: [
        "Direct effects",
        "Specific indirect effects",
        "Run provenance",
      ],
    });
    let helperCompleted = false;
    try {
      const ready = await nativeSaveHelper.ready;
      evidence.checks.mediationExport.nativeXlsx = {
        attempted: true,
        targetPath,
        helper: { ready, completion: null },
        appFeedback: null,
        file: null,
      };
      if (!ready.passed || ready.event !== "ready") {
        throw new Error(`Native XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
      }

      await mediationXlsxExport.click();
      const completion = await nativeSaveHelper.completed;
      helperCompleted = true;
      evidence.checks.mediationExport.nativeXlsx.helper.completion = completion;
      if (!completion.passed) {
        throw new Error(`Native PLS Algorithm XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
      }

      const expectedFeedback = `Saved ${path.basename(targetPath)}.`;
      const feedback = mediationExportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
      await feedback.waitFor({ state: "visible", timeout: 15_000 });
      const file = await fs.stat(targetPath);
      evidence.checks.mediationExport.nativeXlsx.appFeedback = (await feedback.textContent())?.trim() ?? "";
      evidence.checks.mediationExport.nativeXlsx.file = {
        path: targetPath,
        size: file.size,
        isFile: file.isFile(),
      };
      if (!file.isFile() || file.size <= 0 || evidence.checks.mediationExport.nativeXlsx.appFeedback !== expectedFeedback) {
        throw new Error(`The packaged app did not confirm the verified PLS Algorithm XLSX export: ${JSON.stringify(evidence.checks.mediationExport.nativeXlsx)}`);
      }
    } finally {
      if (!helperCompleted) nativeSaveHelper.stop();
    }
  } else {
    evidence.checks.mediationExport.nativeXlsx = {
      attempted: false,
      reason: "QUICKPLS_NATIVE_EXPORT_PATH was not set; the harness retained the enabled PLS Algorithm XLSX UI assertion without opening the Windows Save dialog.",
    };
  }
  await capture(mediationCaptureName(42, "mediation-pls-export-dialog"));
  await mediationExportDialog.getByRole("button", { name: "Close", exact: true }).click();

  const mediationBootstrapDialog = await openCalculationFromToolbar();
  const mediationBootstrapListbox = mediationBootstrapDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await mediationBootstrapListbox.getByRole("option", { name: /PLS-SEM Bootstrapping/i }).click();
  await mediationBootstrapDialog.locator("#nd-calculation-bootstrap-samples").fill("100");
  await mediationBootstrapDialog.locator("#nd-calculation-studentized").selectOption("0");
  const startMediationBootstrap = mediationBootstrapDialog.getByRole("button", { name: "Start bootstrapping", exact: true });
  evidence.checks.mediationBootstrapDialog = {
    selectedMethod: (await mediationBootstrapListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    bootstrapSamples: await mediationBootstrapDialog.locator("#nd-calculation-bootstrap-samples").inputValue(),
    studentizedInnerSamples: await mediationBootstrapDialog.locator("#nd-calculation-studentized").inputValue(),
    startEnabled: await startMediationBootstrap.isEnabled(),
    blockers: await mediationBootstrapDialog.locator(".nd-blocker li").allTextContents(),
    confidenceLevel: await mediationBootstrapDialog.locator("#nd-calculation-confidence").inputValue(),
    seed: await mediationBootstrapDialog.locator("#nd-calculation-seed").inputValue(),
    workers: await mediationBootstrapDialog.locator("#nd-calculation-workers").inputValue(),
  };
  if (!evidence.checks.mediationBootstrapDialog.startEnabled) {
    throw new Error(`Native Bootstrap was blocked on the 240-row mediation project: ${evidence.checks.mediationBootstrapDialog.blockers.join(" | ")}`);
  }
  const bootstrapResponsiveSetup = [];
  for (const viewport of ctaPlsViewports) {
    await setActualTauriClientViewport(viewport, `Bootstrap responsive setup ${viewport.id}`);
    const metrics = await mediationBootstrapDialog.evaluate((element) => {
      const bounds = element.getBoundingClientRect();
      return {
        innerWidth,
        innerHeight,
        documentNoHorizontalOverflow: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
        dialogWithinViewport: bounds.left >= 0 && bounds.right <= innerWidth + 1 && bounds.top >= 0 && bounds.bottom <= innerHeight + 1,
        startVisible: Boolean(element.querySelector('button[type="submit"]')),
      };
    });
    bootstrapResponsiveSetup.push({ ...viewport, passed: metrics.innerWidth === viewport.width
      && metrics.innerHeight === viewport.height && metrics.documentNoHorizontalOverflow
      && metrics.dialogWithinViewport && metrics.startVisible, metrics });
    await capture(`43-tauri-native-mediation-bootstrap-dialog-${viewport.id}.png`);
  }
  await setActualTauriClientViewport({ width: 1440, height: 900 }, "Bootstrap responsive setup restoration");
  await capture(mediationCaptureName(43, "mediation-bootstrap-dialog"));
  await startMediationBootstrap.click();

  await waitForSurface("results", 180_000);
  await page.locator(".nd-run-select select option:checked").filter({ hasText: /PLS-SEM Bootstrapping/i }).waitFor({ state: "attached", timeout: 180_000 });
  const mediationBootstrapRunId = await page.locator(".nd-run-select select").inputValue();
  evidence.checks.mediationBootstrapResult = {
    runId: mediationBootstrapRunId,
    runLabel: (await page.locator(".nd-run-select select option:checked").textContent())?.trim() ?? "",
    navigation: await inspectMediationResultTree({ withBootstrap: true }),
  };
  const bootstrapInferenceText = evidence.checks.mediationBootstrapResult.navigation.tableText[mediationBootstrapTableTitle];
  if (!bootstrapInferenceText.includes("Total indirect effect (aggregate)")
    || !bootstrapInferenceText.includes("Construct 1")
    || !bootstrapInferenceText.includes("Construct 3")
    || /\bN\/A\b/i.test(bootstrapInferenceText)) {
    throw new Error(`Aggregate mediation effects bootstrap inference did not contain the Construct 1 -> Construct 3 aggregate indirect-effect estimate: ${bootstrapInferenceText}`);
  }
  await capture(mediationCaptureName(44, "mediation-bootstrap-results"));

  await mediationExportCommand.click();
  await mediationExportDialog.waitFor({ state: "visible", timeout: 10_000 });
  await mediationXlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  evidence.checks.mediationExport.bootstrap = {
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    formats: await mediationExportDialog.locator(".nd-export-list button").count(),
    xlsxEnabled: await mediationXlsxExport.isEnabled(),
    selectedResultTable: evidence.checks.mediationBootstrapResult.navigation.selectedTable,
    nativeXlsx: null,
  };
  if (evidence.checks.mediationExport.bootstrap.selectedRunId !== mediationBootstrapRunId
    || evidence.checks.mediationExport.bootstrap.formats < 5
    || !evidence.checks.mediationExport.bootstrap.xlsxEnabled) {
    throw new Error(`The completed Bootstrap run did not retain its expected enabled native export formats: ${JSON.stringify(evidence.checks.mediationExport.bootstrap)}`);
  }
  if (!requestedBootstrapNativeExportPath) {
    throw new Error("QUICKPLS_BOOTSTRAP_NATIVE_EXPORT_PATH is required for authoritative packaged PLS Bootstrap XLSX acceptance; the PLS Algorithm export cannot stand in for the selected Bootstrap run.");
  }
  const bootstrapExportTargetPath = await validateRequestedNativeExportPath(
    requestedBootstrapNativeExportPath,
    "QUICKPLS_BOOTSTRAP_NATIVE_EXPORT_PATH",
  );
  const bootstrapSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: bootstrapExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: [
      "Aggregate mediation effects boo",
      "Bootstrapping",
      "Bias-corrected and accelerated",
      "Run provenance",
    ],
    expectedSharedStrings: [
      "Aggregate mediation effects bootstrap inference",
      "Total indirect effect (aggregate)",
      "Run provenance",
    ],
  });
  let bootstrapHelperCompleted = false;
  try {
    const ready = await bootstrapSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") {
      throw new Error(`Native Bootstrap XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    }
    await mediationXlsxExport.click();
    const completion = await bootstrapSaveHelper.completed;
    bootstrapHelperCompleted = true;
    if (!completion.passed) throw new Error(`Native Bootstrap XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(bootstrapExportTargetPath)}.`;
    const feedback = mediationExportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(bootstrapExportTargetPath);
    evidence.checks.mediationExport.bootstrap.nativeXlsx = {
      attempted: true,
      selectedRunId: await page.locator(".nd-run-select select").inputValue(),
      targetPath: bootstrapExportTargetPath,
      helper: { ready, completion },
      appFeedback: (await feedback.textContent())?.trim() ?? "",
      file: { path: bootstrapExportTargetPath, size: file.size, isFile: file.isFile() },
    };
    if (!file.isFile() || file.size <= 0
      || evidence.checks.mediationExport.bootstrap.nativeXlsx.selectedRunId !== mediationBootstrapRunId
      || evidence.checks.mediationExport.bootstrap.nativeXlsx.appFeedback !== expectedFeedback) {
      throw new Error(`The packaged app did not confirm the exact selected Bootstrap XLSX export: ${JSON.stringify(evidence.checks.mediationExport.bootstrap.nativeXlsx)}`);
    }
  } finally {
    if (!bootstrapHelperCompleted) bootstrapSaveHelper.stop();
  }
  await capture(mediationCaptureName(45, "mediation-bootstrap-export-dialog"));
  await mediationExportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  await reloadToLauncher();
  await openRecentProject(mediationProjectName, mediationProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  await page.locator(".nd-run-select select option").first().waitFor({ state: "attached", timeout: 15_000 });
  const reopenedMediationRuns = await page.locator(".nd-run-select select option").allTextContents();
  const reopenedMediationPlsOption = page.locator(".nd-run-select select option").filter({ hasText: /PLS-SEM Algorithm/i }).first();
  await reopenedMediationPlsOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedMediationPlsRunId = await reopenedMediationPlsOption.getAttribute("value");
  if (!reopenedMediationPlsRunId) throw new Error("The reopened mediation PLS Algorithm run option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedMediationPlsRunId);
  const reopenedMediationPlsNavigation = await inspectMediationResultTree({ withBootstrap: false });
  const reopenedMediationBootstrapOption = page.locator(".nd-run-select select option").filter({ hasText: /PLS-SEM Bootstrapping/i }).first();
  await reopenedMediationBootstrapOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedMediationBootstrapRunId = await reopenedMediationBootstrapOption.getAttribute("value");
  if (!reopenedMediationBootstrapRunId) throw new Error("The reopened mediation Bootstrap run option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedMediationBootstrapRunId);
  const reopenedMediationNavigation = await inspectMediationResultTree({ withBootstrap: true });

  await openMenuItem("View", "Edit Model");
  await waitForSurface("model");
  await page.locator(".react-flow__node-latent").nth(2).waitFor({ state: "visible", timeout: 15_000 });
  const reopenedMediationModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
  };
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  await page.locator(".nd-run-select select").selectOption(reopenedMediationPlsRunId);
  const reopenedMediationPlsFinalNavigation = await inspectMediationResultTree({ withBootstrap: false });
  await page.locator(".nd-run-select select").selectOption(reopenedMediationBootstrapRunId);
  const reopenedMediationFinalNavigation = await inspectMediationResultTree({ withBootstrap: true });
  evidence.checks.mediationSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    runOptions: reopenedMediationRuns.map((label) => label.trim()),
    hasPlsAlgorithm: reopenedMediationRuns.some((label) => /PLS-SEM Algorithm/i.test(label)),
    hasBootstrap: reopenedMediationRuns.some((label) => /PLS-SEM Bootstrapping/i.test(label)),
    selectedPlsRunId: reopenedMediationPlsRunId,
    expectedPlsRunId: mediationPlsRunId,
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    expectedBootstrapRunId: mediationBootstrapRunId,
    model: reopenedMediationModel,
    plsNavigation: reopenedMediationPlsNavigation,
    plsFinalNavigation: reopenedMediationPlsFinalNavigation,
    navigation: reopenedMediationNavigation,
    finalNavigation: reopenedMediationFinalNavigation,
  };
  if (!evidence.checks.mediationSaveReopen.hasPlsAlgorithm
    || !evidence.checks.mediationSaveReopen.hasBootstrap
    || evidence.checks.mediationSaveReopen.selectedPlsRunId !== mediationPlsRunId
    || evidence.checks.mediationSaveReopen.selectedRunId !== mediationBootstrapRunId
    || reopenedMediationModel.constructs !== 3
    || reopenedMediationModel.assignedIndicators !== 6
    || reopenedMediationModel.structuralPaths !== 2) {
    throw new Error(`The mediation model, runs, or native results did not survive save/reload/reopen: ${JSON.stringify(evidence.checks.mediationSaveReopen)}`);
  }
  const bootstrapResponsiveResults = [];
  for (const viewport of ctaPlsViewports) {
    await setActualTauriClientViewport(viewport, `Bootstrap responsive result ${viewport.id}`);
    const rowCount = await openResultTable(mediationBootstrapTableTitle);
    const metrics = await page.evaluate(() => {
      const app = document.querySelector(".nd-app");
      return {
        innerWidth,
        innerHeight,
        documentNoHorizontalOverflow: document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1,
        appNoHorizontalOverflow: Boolean(app && app.scrollWidth <= app.clientWidth + 1),
        treeVisible: Boolean(document.querySelector('.nd-result-tree[role="tree"]')),
      };
    });
    bootstrapResponsiveResults.push({ ...viewport, passed: metrics.innerWidth === viewport.width
      && metrics.innerHeight === viewport.height && metrics.documentNoHorizontalOverflow
      && metrics.appNoHorizontalOverflow && metrics.treeVisible && rowCount === 6
      && await page.locator(".nd-run-select select").inputValue() === mediationBootstrapRunId,
    metrics: { ...metrics, rowCount } });
    await capture(`46-tauri-native-mediation-bootstrap-results-reopened-${viewport.id}.png`);
  }
  await setActualTauriClientViewport({ width: 1440, height: 900 }, "Bootstrap responsive result restoration");
  evidence.checks.bootstrapResponsiveViewports = {
    passed: bootstrapResponsiveSetup.length === ctaPlsViewports.length
      && bootstrapResponsiveResults.length === ctaPlsViewports.length
      && bootstrapResponsiveSetup.every((row) => row.passed)
      && bootstrapResponsiveResults.every((row) => row.passed),
    setup: bootstrapResponsiveSetup,
    results: bootstrapResponsiveResults,
  };
  if (!evidence.checks.bootstrapResponsiveViewports.passed) {
    throw new Error(`PLS Bootstrap setup/results responsiveness failed at a required viewport: ${JSON.stringify(evidence.checks.bootstrapResponsiveViewports)}`);
  }
  evidence.checks.bootstrapCancellationRetry = {
    passed: evidence.checks.bootstrapCancelled.status === "cancelled"
      && evidence.checks.bootstrapCancelled.partialRunVisible === 0
      && cancelledBootstrapSetup.bootstrapSamples === "1000"
      && cancelledBootstrapSetup.studentizedInnerSamples === "999"
      && evidence.checks.mediationBootstrapDialog.bootstrapSamples === "100"
      && evidence.checks.mediationBootstrapDialog.studentizedInnerSamples === "0"
      && Boolean(mediationBootstrapRunId)
      && evidence.checks.mediationExport.bootstrap.selectedRunId === mediationBootstrapRunId
      && evidence.checks.mediationSaveReopen.selectedRunId === mediationBootstrapRunId,
    cancelledSettings: cancelledBootstrapSetup,
    cancelledPartialRunVisible: evidence.checks.bootstrapCancelled.partialRunVisible,
    retrySettings: evidence.checks.mediationBootstrapDialog,
    completedRetryRunId: mediationBootstrapRunId,
    exportedRunId: evidence.checks.mediationExport.bootstrap.selectedRunId,
    reopenedRunId: evidence.checks.mediationSaveReopen.selectedRunId,
  };
  if (!evidence.checks.bootstrapCancellationRetry.passed) {
    throw new Error(`PLS Bootstrap cancellation/retry identity linkage failed: ${JSON.stringify(evidence.checks.bootstrapCancellationRetry)}`);
  }
  const bootstrapInternalOrigins = new Set([packagedTauriOrigin, packagedTauriIpcOrigin]);
  const bootstrapExternalRequests = observedBrowserRequests.filter((request) => request.origin
    && request.origin !== "null" && !bootstrapInternalOrigins.has(request.origin));
  evidence.checks.bootstrapFunctionalOffline = {
    passed: observedBrowserRequests.length > 0 && bootstrapExternalRequests.length === 0,
    analyticalWorkflowRequiresInternet: false,
    strictZeroProcessEgressClaimed: false,
    platformBackgroundEgressOutsidePageRequestScope: true,
    observedRequestCount: observedBrowserRequests.length,
    externalRequestCount: bootstrapExternalRequests.length,
    origins: [...new Set(observedBrowserRequests.map((request) => request.origin))].sort(),
    externalRequests: bootstrapExternalRequests,
  };
  if (!evidence.checks.bootstrapFunctionalOffline.passed) {
    throw new Error(`PLS Bootstrap browser/app workflow crossed its functional-offline request boundary: ${JSON.stringify(evidence.checks.bootstrapFunctionalOffline)}`);
  }
  await capture(mediationCaptureName(46, "mediation-results-reopened"));

  // Genuine moderation milestone. The checked-in 120-row fixture is imported
  // into its own disposable project. Every construct, assignment, relationship,
  // and moderating effect is created through the visible native workbench.
  await openMenuItem("View", "Project");
  await waitForSurface("launcher");
  await seedRecentProject({
    name: moderationProjectName,
    path: moderationProjectPath,
    openedAt: "2026-08-11T00:05:00.000Z",
  });
  await reloadToLauncher();
  const moderationRecentRow = exactRecentProjectRow(moderationProjectName, moderationProjectPath);
  await moderationRecentRow.waitFor({ state: "visible", timeout: 10_000 });
  evidence.checks.moderationRecentProject = {
    visibleRows: await moderationRecentRow.count(),
    pathVisible: (await moderationRecentRow.textContent())?.includes(moderationProjectPath) ?? false,
    projectPath: moderationProjectPath,
  };
  if (evidence.checks.moderationRecentProject.visibleRows !== 1 || !evidence.checks.moderationRecentProject.pathVisible) {
    throw new Error("The disposable moderation project was not exposed through one truthful visible Recent Projects row.");
  }
  await capture(moderationCaptureName(47, "moderation-seeded-recent-project"));

  await openRecentProject(moderationProjectName, moderationProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const moderationDatasetStatus = (await page.locator(".nd-statusbar").textContent())?.trim() ?? "";
  const moderationColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map((column) => column.trim());
  evidence.checks.moderationDataset = {
    cases: moderationDatasetStatus.includes("120 cases") ? 120 : null,
    columns: moderationColumns,
    status: moderationDatasetStatus,
    sourceCsv: moderationFixtureCsvPath,
  };
  if (evidence.checks.moderationDataset.cases !== 120 || ["x", "m", "y"].some((column) => !moderationColumns.includes(column))) {
    throw new Error(`The visible moderation project did not load the tracked 120-row x/m/y fixture: ${JSON.stringify(evidence.checks.moderationDataset)}`);
  }
  await capture(moderationCaptureName(48, "moderation-fixture-data"));

  evidence.checks.initialModerationModelCreation = await createInitialEditableModel(moderationProjectName, moderationModelName);
  const moderationModel = await buildThreeConstructModerationModel();
  evidence.checks.visibleModerationBaseModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    constructLabels: (await page.locator(".react-flow__node-latent").allTextContents()).map((label) => label.replace(/\s+/g, " ").trim()),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    basePathId: moderationModel.basePathId,
    nodeIds: { x: moderationModel.xId, moderator: moderationModel.mId, outcome: moderationModel.yId },
  };
  if (evidence.checks.visibleModerationBaseModel.constructs !== 3
    || evidence.checks.visibleModerationBaseModel.assignedIndicators !== 3
    || evidence.checks.visibleModerationBaseModel.structuralPaths !== 1
    || !["X", "M", "Y"].every((label) => evidence.checks.visibleModerationBaseModel.constructLabels.some((value) => value.includes(label)))) {
    throw new Error(`The visible Model workflow did not create the named X, M, and Y base model with X -> Y: ${JSON.stringify(evidence.checks.visibleModerationBaseModel)}`);
  }
  await capture(moderationCaptureName(49, "moderation-base-model"));

  const basePath = page.locator(`.react-flow__edge[data-id="${moderationModel.basePathId}"]`);
  await selectVisibleStructuralPath(basePath);
  const moderatingEffectCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Moderating Effect/i });
  if (await moderatingEffectCommand.count() !== 1 || !await moderatingEffectCommand.isEnabled()) {
    throw new Error("Selecting the eligible X-to-Y relationship did not enable exactly one Moderating Effect command.");
  }
  await moderatingEffectCommand.click();
  const moderationDialog = page.locator('.nd-dialog-moderation[role="dialog"]');
  await moderationDialog.waitFor({ state: "visible", timeout: 10_000 });
  const relationshipSelect = moderationDialog.locator("#nd-moderation-relationship");
  const moderatorSelect = moderationDialog.locator("#nd-moderation-moderator");
  const createModeratingEffect = moderationDialog.getByRole("button", { name: "Create moderating effect", exact: true });
  evidence.checks.moderationDialog = {
    relationshipId: await relationshipSelect.inputValue(),
    relationshipLabel: (await relationshipSelect.locator("option:checked").textContent())?.replace(/\s+/g, " ").trim() ?? "",
    moderatorId: await moderatorSelect.inputValue(),
    moderatorLabel: (await moderatorSelect.locator("option:checked").textContent())?.trim() ?? "",
    methodSummary: (await moderationDialog.locator(".nd-moderation-summary").textContent())?.replace(/\s+/g, " ").trim() ?? "",
    automaticMainEffectDisclosure: (await moderationDialog.locator(".nd-dialog-note").textContent())?.replace(/\s+/g, " ").trim() ?? "",
    createEnabled: await createModeratingEffect.isEnabled(),
  };
  if (evidence.checks.moderationDialog.relationshipId !== moderationModel.basePathId
    || !/X/.test(evidence.checks.moderationDialog.relationshipLabel)
    || !/Y/.test(evidence.checks.moderationDialog.relationshipLabel)
    || evidence.checks.moderationDialog.moderatorId !== moderationModel.mId
    || evidence.checks.moderationDialog.moderatorLabel !== "M"
    || !/Two-stage product score/i.test(evidence.checks.moderationDialog.methodSummary)
    || !/adds the moderator(?:'|\u2019)s main-effect path to the outcome when it is missing/i.test(evidence.checks.moderationDialog.automaticMainEffectDisclosure)
    || !evidence.checks.moderationDialog.createEnabled) {
    throw new Error(`The selected-path Moderating Effect dialog did not bind X -> Y, choose M, and disclose automatic M -> Y creation: ${JSON.stringify(evidence.checks.moderationDialog)}`);
  }
  await capture(moderationCaptureName(50, "moderating-effect-dialog"));
  await createModeratingEffect.click();
  await moderationDialog.waitFor({ state: "hidden", timeout: 10_000 });

  const interactionId = `interaction-${moderationModel.xId}-${moderationModel.mId}-${moderationModel.yId}`.replace(/[^a-zA-Z0-9_-]/g, "-");
  const moderatorMainEffectId = `path-${moderationModel.mId}-${moderationModel.yId}`;
  const interactionEffectId = `path-${interactionId}-${moderationModel.yId}`;
  const moderatorMainEffect = page.locator(`.react-flow__edge[data-id="${moderatorMainEffectId}"]`);
  const interactionEffect = page.locator(`.react-flow__edge[data-id="${interactionEffectId}"]`);
  await page.locator(`.react-flow__node-latent[data-id="${interactionId}"]`).waitFor({ state: "visible", timeout: 10_000 });
  await moderatorMainEffect.waitFor({ state: "attached", timeout: 10_000 });
  await interactionEffect.waitFor({ state: "attached", timeout: 10_000 });
  const basePathProperties = await inspectVisibleStructuralPath(basePath);
  const moderatorPathProperties = await inspectVisibleStructuralPath(moderatorMainEffect);
  const interactionPathProperties = await inspectVisibleStructuralPath(interactionEffect);
  evidence.checks.visibleModerationGeneratedModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    constructLabels: (await page.locator(".react-flow__node-latent").allTextContents()).map((label) => label.replace(/\s+/g, " ").trim()),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    edgeIds: await structuralPaths().evaluateAll((elements) => elements.map((element) => element.getAttribute("data-id"))),
    interactionId,
    basePath: basePathProperties,
    moderatorMainEffect: moderatorPathProperties,
    interactionEffect: interactionPathProperties,
    successToast: await page.locator(".nd-toast").filter({ hasText: /Moderating effect created/i }).count(),
  };
  if (evidence.checks.visibleModerationGeneratedModel.constructs !== 4
    || evidence.checks.visibleModerationGeneratedModel.assignedIndicators !== 3
    || evidence.checks.visibleModerationGeneratedModel.structuralPaths !== 3
    || basePathProperties.Source !== "X" || basePathProperties.Target !== "Y"
    || moderatorPathProperties.Source !== "M" || moderatorPathProperties.Target !== "Y"
    || interactionPathProperties.Source !== "X x M" || interactionPathProperties.Target !== "Y"
    || evidence.checks.visibleModerationGeneratedModel.successToast !== 1) {
    throw new Error(`Creating the moderating effect did not preserve X -> Y and visibly add M -> Y plus X x M -> Y: ${JSON.stringify(evidence.checks.visibleModerationGeneratedModel)}`);
  }
  await capture(moderationCaptureName(51, "moderation-generated-model"));

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });

  const moderationPlsDialog = await openCalculationFromToolbar();
  const moderationPlsListbox = moderationPlsDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await moderationPlsListbox.getByRole("option", { name: /PLS-SEM Algorithm/i }).click();
  const startModerationPls = moderationPlsDialog.getByRole("button", { name: "Start calculation", exact: true });
  evidence.checks.moderationPlsDialog = {
    selectedMethod: (await moderationPlsListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    startEnabled: await startModerationPls.isEnabled(),
    blockers: await moderationPlsDialog.locator(".nd-blocker li").allTextContents(),
  };
  if (!evidence.checks.moderationPlsDialog.startEnabled) {
    throw new Error(`Native PLS was blocked on the visible 120-row moderation model: ${evidence.checks.moderationPlsDialog.blockers.join(" | ")}`);
  }
  await capture(moderationCaptureName(52, "moderation-pls-dialog"));
  await startModerationPls.click();

  await waitForSurface("results", 120_000);
  await page.locator(".nd-run-select select option:checked").filter({ hasText: /PLS-SEM Algorithm/i }).waitFor({ state: "attached", timeout: 120_000 });
  const moderationPlsRunId = await page.locator(".nd-run-select select").inputValue();
  const moderationPlsNavigation = await inspectModerationResultTree({ withBootstrap: false });
  evidence.checks.moderationPlsResult = {
    runId: moderationPlsRunId,
    runLabel: (await page.locator(".nd-run-select select option:checked").textContent())?.trim() ?? "",
    navigation: moderationPlsNavigation,
  };
  const moderationEffectsText = moderationPlsNavigation.tableText["Moderation effects"];
  const simpleSlopeText = moderationPlsNavigation.tableText["Simple slope analysis"];
  if (!/Interaction effect/i.test(moderationEffectsText)
    || !["X", "M", "Y"].every((label) => moderationEffectsText.includes(label))
    || moderationPlsNavigation.rowCounts["Simple slope analysis"] < 3
    || !["-1", "0", "1"].every((score) => simpleSlopeText.includes(score))) {
    throw new Error(`The completed moderation PLS run did not expose the expected effect roles and -1/0/+1 simple slopes: ${JSON.stringify(evidence.checks.moderationPlsResult)}`);
  }
  await openResultTable("Moderation effects");
  await capture(moderationCaptureName(53, "moderation-pls-effects"));
  await openResultTable("Simple slope analysis");
  await capture(moderationCaptureName(54, "moderation-pls-conditional-effect-plot"));

  const moderationBootstrapDialog = await openCalculationFromToolbar();
  const moderationBootstrapListbox = moderationBootstrapDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await moderationBootstrapListbox.getByRole("option", { name: /PLS-SEM Bootstrapping/i }).click();
  const moderationBootstrapSamples = moderationBootstrapDialog.locator("#nd-calculation-bootstrap-samples");
  const moderationBootstrapMinimum = await moderationBootstrapSamples.getAttribute("min");
  await moderationBootstrapSamples.fill(moderationBootstrapMinimum ?? "100");
  await moderationBootstrapDialog.locator("#nd-calculation-studentized").selectOption("0");
  const startModerationBootstrap = moderationBootstrapDialog.getByRole("button", { name: "Start bootstrapping", exact: true });
  evidence.checks.moderationBootstrapDialog = {
    selectedMethod: (await moderationBootstrapListbox.getByRole("option", { selected: true }).textContent())?.trim(),
    bootstrapSamples: await moderationBootstrapSamples.inputValue(),
    inputMinimum: moderationBootstrapMinimum,
    minimumValidBootstrapSamples: 100,
    studentizedInnerSamples: await moderationBootstrapDialog.locator("#nd-calculation-studentized").inputValue(),
    startEnabled: await startModerationBootstrap.isEnabled(),
    blockers: await moderationBootstrapDialog.locator(".nd-blocker li").allTextContents(),
  };
  if (!evidence.checks.moderationBootstrapDialog.startEnabled
    || evidence.checks.moderationBootstrapDialog.inputMinimum !== String(evidence.checks.moderationBootstrapDialog.minimumValidBootstrapSamples)
    || evidence.checks.moderationBootstrapDialog.bootstrapSamples !== String(evidence.checks.moderationBootstrapDialog.minimumValidBootstrapSamples)) {
    throw new Error(`Native Bootstrap was blocked at the minimum valid 100 samples on the moderation model: ${JSON.stringify(evidence.checks.moderationBootstrapDialog)}`);
  }
  await capture(moderationCaptureName(55, "moderation-bootstrap-dialog-minimum-samples"));
  await startModerationBootstrap.click();

  await waitForSurface("results", 180_000);
  await page.locator(".nd-run-select select option:checked").filter({ hasText: /PLS-SEM Bootstrapping/i }).waitFor({ state: "attached", timeout: 180_000 });
  const moderationBootstrapRunId = await page.locator(".nd-run-select select").inputValue();
  const moderationBootstrapNavigation = await inspectModerationResultTree({ withBootstrap: true });
  evidence.checks.moderationBootstrapResult = {
    runId: moderationBootstrapRunId,
    runLabel: (await page.locator(".nd-run-select select option:checked").textContent())?.trim() ?? "",
    navigation: moderationBootstrapNavigation,
  };
  const interactionInferenceText = moderationBootstrapNavigation.tableText[moderationBootstrapTableTitle];
  if (!["X", "M", "Y"].every((label) => interactionInferenceText.includes(label))
    || moderationBootstrapNavigation.rowCounts[moderationBootstrapTableTitle] < 1
    || /\bN\/A\b/i.test(interactionInferenceText)) {
    throw new Error(`Interaction effect bootstrap inference did not contain the genuine X x M -> Y estimate: ${interactionInferenceText}`);
  }
  await openResultTable(moderationBootstrapTableTitle);
  await capture(moderationCaptureName(56, "moderation-bootstrap-interaction-inference"));
  await openResultTable("Simple slope analysis");
  await capture(moderationCaptureName(57, "moderation-bootstrap-conditional-effect-plot"));

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  await reloadToLauncher();
  await openRecentProject(moderationProjectName, moderationProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  await page.locator(".nd-run-select select option").first().waitFor({ state: "attached", timeout: 15_000 });
  const reopenedModerationRuns = await page.locator(".nd-run-select select option").allTextContents();
  const reopenedModerationBootstrapOption = page.locator(".nd-run-select select option").filter({ hasText: /PLS-SEM Bootstrapping/i }).first();
  await reopenedModerationBootstrapOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedModerationBootstrapRunId = await reopenedModerationBootstrapOption.getAttribute("value");
  if (!reopenedModerationBootstrapRunId) throw new Error("The reopened moderation Bootstrap run option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedModerationBootstrapRunId);
  const reopenedModerationNavigation = await inspectModerationResultTree({ withBootstrap: true });

  await openMenuItem("View", "Edit Model");
  await waitForSurface("model");
  await page.locator(`.react-flow__node-latent[data-id="${interactionId}"]`).waitFor({ state: "visible", timeout: 15_000 });
  const reopenedInteraction = page.locator(`.react-flow__node-latent[data-id="${interactionId}"]`);
  await reopenedInteraction.click();
  const reopenedInteractionProperties = await modelInspector().locator(".nd-property-list").first().evaluate((element) => Object.fromEntries(Array.from(element.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  await modelInspector().getByRole("tab", { name: "Parameter", exact: true }).click();
  const reopenedInteractionParameters = await modelInspector().locator(".nd-property-list").first().evaluate((element) => Object.fromEntries(Array.from(element.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  const reopenedModerationModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    interactionProperties: reopenedInteractionProperties,
    interactionParameters: reopenedInteractionParameters,
  };
  await capture(moderationCaptureName(58, "moderation-model-reopened"));

  await openMenuItem("View", "Results");
  await waitForSurface("results");
  await page.locator(".nd-run-select select").selectOption(reopenedModerationBootstrapRunId);
  const reopenedModerationFinalNavigation = await inspectModerationResultTree({ withBootstrap: true });
  evidence.checks.moderationSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    runOptions: reopenedModerationRuns.map((label) => label.trim()),
    hasPlsAlgorithm: reopenedModerationRuns.some((label) => /PLS-SEM Algorithm/i.test(label)),
    hasBootstrap: reopenedModerationRuns.some((label) => /PLS-SEM Bootstrapping/i.test(label)),
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    expectedBootstrapRunId: moderationBootstrapRunId,
    model: reopenedModerationModel,
    navigation: reopenedModerationNavigation,
    finalNavigation: reopenedModerationFinalNavigation,
  };
  if (!evidence.checks.moderationSaveReopen.hasPlsAlgorithm
    || !evidence.checks.moderationSaveReopen.hasBootstrap
    || evidence.checks.moderationSaveReopen.selectedRunId !== moderationBootstrapRunId
    || reopenedModerationBootstrapRunId !== moderationBootstrapRunId
    || reopenedModerationModel.constructs !== 4
    || reopenedModerationModel.assignedIndicators !== 3
    || reopenedModerationModel.structuralPaths !== 3
    || reopenedInteractionProperties.Predictor !== "X"
    || reopenedInteractionProperties.Moderator !== "M"
    || reopenedInteractionProperties.Outcome !== "Y"
    || reopenedInteractionParameters.Parameter !== "Two-stage product score") {
    throw new Error(`The moderation model, generated relationships, completed runs, or result navigation did not survive save/reload/reopen: ${JSON.stringify(evidence.checks.moderationSaveReopen)}`);
  }
  await openResultTable(moderationBootstrapTableTitle);
  await capture(moderationCaptureName(59, "moderation-results-reopened"));
    }

  // Genuine two-group permutation MGA milestone. The deterministic 180-row
  // fixture is imported through the CLI, opened through Recent Projects, and
  // configured only through the visible Data workspace grouping command.
  await openMenuItem("View", "Project");
  await waitForSurface("launcher");
  await seedRecentProject({
    name: mgaProjectName,
    path: mgaProjectPath,
    openedAt: "2026-08-11T02:00:00.000Z",
  });
  await reloadToLauncher();
  const mgaRecentRow = exactRecentProjectRow(mgaProjectName, mgaProjectPath);
  await mgaRecentRow.waitFor({ state: "visible", timeout: 10_000 });
  evidence.checks.mgaRecentProject = {
    visibleRows: await mgaRecentRow.count(),
    pathVisible: (await mgaRecentRow.textContent())?.includes(mgaProjectPath) ?? false,
    projectPath: mgaProjectPath,
  };
  if (evidence.checks.mgaRecentProject.visibleRows !== 1 || !evidence.checks.mgaRecentProject.pathVisible) {
    throw new Error("The disposable MGA project was not exposed through one truthful visible Recent Projects row.");
  }

  await openRecentProject(mgaProjectName, mgaProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const mgaDatasetStatus = (await page.locator(".nd-statusbar").textContent())?.trim() ?? "";
  const mgaColumns = (await page.locator(".nd-data-table thead th").allTextContents()).map((column) => column.trim());
  const mgaPreviewRows = await page.locator(".nd-data-table tbody tr").count();
  evidence.checks.mgaDataset = {
    cases: mgaDatasetStatus.includes("180 cases") ? 180 : null,
    columns: mgaColumns,
    previewRows: mgaPreviewRows,
    status: mgaDatasetStatus,
    sourceCsv: mgaFixtureCsvPath,
  };
  if (evidence.checks.mgaDataset.cases !== 180
    || ["group", "x1", "x2", "z1", "z2", "y1", "y2"].some((column) => !mgaColumns.includes(column))
    || mgaPreviewRows <= 0
    || mgaPreviewRows > 100) {
    throw new Error(`The visible MGA project did not load the deterministic 180-row fixture through its bounded Data preview: ${JSON.stringify(evidence.checks.mgaDataset)}`);
  }
  await capture(mgaCaptureName(60, "data"));

  evidence.checks.initialMgaModelCreation = await createInitialEditableModel(mgaProjectName, mgaModelName);
  await buildThreeConstructMgaModel();
  const mgaGroupVariable = page.locator(".nd-variable-item").filter({ hasText: /^group$/i });
  evidence.checks.visibleMgaModelBuild = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    constructLabels: (await page.locator(".react-flow__node-latent").allTextContents()).map((label) => label.replace(/\s+/g, " ").trim()),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    groupVariableVisible: await mgaGroupVariable.count(),
    groupVariableUnassigned: await mgaGroupVariable.evaluate((element) => !element.classList.contains("assigned")),
  };
  if (evidence.checks.visibleMgaModelBuild.constructs !== 3
    || evidence.checks.visibleMgaModelBuild.assignedIndicators !== 6
    || evidence.checks.visibleMgaModelBuild.structuralPaths !== 2
    || evidence.checks.visibleMgaModelBuild.groupVariableVisible !== 1
    || !evidence.checks.visibleMgaModelBuild.groupVariableUnassigned
    || !["X", "Z", "Y"].every((label) => evidence.checks.visibleMgaModelBuild.constructLabels.some((value) => value.includes(label)))) {
    throw new Error(`The visible MGA authoring workflow did not create X[x1,x2], Z[z1,z2], Y[y1,y2] with X -> Y and Z -> Y: ${JSON.stringify(evidence.checks.visibleMgaModelBuild)}`);
  }
  await capture(mgaCaptureName(61, "model"));

  await openMenuItem("View", "Data");
  await waitForSurface("data");
  const groupVariableInData = page.locator('.nd-variable-list button[data-native-variable="group"]');
  await groupVariableInData.waitFor({ state: "visible", timeout: 10_000 });
  await groupVariableInData.click();
  const configureGroupsCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Use as Grouping Variable…$/ });
  if (await configureGroupsCommand.count() !== 1 || !await configureGroupsCommand.isEnabled()) {
    throw new Error("Selecting group in Data did not expose exactly one enabled Use as Grouping Variable… command.");
  }
  await configureGroupsCommand.click();
  const groupSetupDialog = page.getByRole("dialog", { name: "Configure Groups", exact: true });
  await groupSetupDialog.waitFor({ state: "visible", timeout: 10_000 });
  const groupColumnSelect = groupSetupDialog.locator('select[id$="-column"]');
  const groupASelect = groupSetupDialog.locator('select[id$="-a"]');
  const groupBSelect = groupSetupDialog.locator('select[id$="-b"]');
  await groupASelect.locator('option[value="A"]').waitFor({ state: "attached", timeout: 30_000 });
  await groupBSelect.locator('option[value="B"]').waitFor({ state: "attached", timeout: 30_000 });
  await groupASelect.selectOption("A");
  await groupBSelect.selectOption("B");
  const groupCounts = groupSetupDialog.locator(".nd-group-counts");
  await groupCounts.waitFor({ state: "visible", timeout: 10_000 });
  const groupCountHeaders = (await groupCounts.locator("thead th").allTextContents()).map((value) => value.trim());
  const groupCountRows = await groupCounts.locator("tbody tr").evaluateAll((rows) => rows.map((row) => (
    Array.from(row.querySelectorAll("th, td")).map((cell) => cell.textContent?.replace(/\s+/g, " ").trim() ?? "")
  )));
  const applyGroups = groupSetupDialog.getByRole("button", { name: "Apply Groups", exact: true });
  const groupScope = (await groupSetupDialog.locator(".nd-group-scope").textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const observedCount = groupCountRows.reduce((sum, row) => sum + Number(row[2] ?? 0), 0);
  evidence.checks.mgaGroupSetup = {
    command: "Use as Grouping Variable…",
    groupingVariable: await groupColumnSelect.inputValue(),
    groupA: await groupASelect.inputValue(),
    groupB: await groupBSelect.inputValue(),
    headers: groupCountHeaders,
    rows: groupCountRows,
    observedCount,
    previewRows: mgaPreviewRows,
    completeDatasetBeyondPreview: observedCount === 180 && observedCount > 100 && observedCount > mgaPreviewRows,
    scope: groupScope,
    applyEnabled: await applyGroups.isEnabled(),
  };
  if (evidence.checks.mgaGroupSetup.groupingVariable !== "group"
    || evidence.checks.mgaGroupSetup.groupA !== "A"
    || evidence.checks.mgaGroupSetup.groupB !== "B"
    || JSON.stringify(groupCountHeaders) !== JSON.stringify(["Role", "Value", "Observed", "Complete model cases"])
    || JSON.stringify(groupCountRows) !== JSON.stringify([["A", "A", "90", "90"], ["B", "B", "90", "90"]])
    || !evidence.checks.mgaGroupSetup.completeDatasetBeyondPreview
    || !/Group A\s*(?:−|-)\s*Group B/i.test(groupScope)
    || !/combined MICOM and structural-path permutation MGA workflow/i.test(groupScope)
    || !/Step 1 confirmation/i.test(groupScope)
    || !/shared permutation plan/i.test(groupScope)
    || !evidence.checks.mgaGroupSetup.applyEnabled) {
    throw new Error(`The native Data group setup did not prove explicit A/B selection from all 180 rows: ${JSON.stringify(evidence.checks.mgaGroupSetup)}`);
  }
  await capture(mgaCaptureName(62, "group-setup"));
  await applyGroups.click();
  await groupSetupDialog.waitFor({ state: "hidden", timeout: 10_000 });
  await page.locator(".nd-toast").filter({ hasText: /Groups configured/i }).last().waitFor({ state: "visible", timeout: 10_000 });
  const editGroupsCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Groups…$/ });
  evidence.checks.mgaConfiguredData = {
    groupingMarker: await groupVariableInData.locator("small").filter({ hasText: /^Groups$/ }).count(),
    groupingClass: await groupVariableInData.evaluate((element) => element.classList.contains("grouping")),
    editCommandCount: await editGroupsCommand.count(),
    editCommandEnabled: await editGroupsCommand.isEnabled(),
  };
  if (evidence.checks.mgaConfiguredData.groupingMarker !== 1
    || !evidence.checks.mgaConfiguredData.groupingClass
    || evidence.checks.mgaConfiguredData.editCommandCount !== 1
    || !evidence.checks.mgaConfiguredData.editCommandEnabled) {
    throw new Error(`Applied A/B grouping was not reflected truthfully in the Data workspace: ${JSON.stringify(evidence.checks.mgaConfiguredData)}`);
  }
  await capture(mgaCaptureName(63, "groups-applied"));

  await openMenuItem("View", "Model");
  await waitForSurface("model");
  const mgaDialog = await openCalculationFromToolbar();
  const mgaMethodListbox = mgaDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const mgaMethodNames = (await mgaMethodListbox.getByRole("option").locator("strong").allTextContents()).map((label) => label.trim());
  if (JSON.stringify(mgaMethodNames) !== JSON.stringify(expectedOptionLabels)) {
    throw new Error(`The group calculation browser did not preserve the canonical ${expectedOptionLabels.length}-method catalog with the joint MICOM/MGA entry: ${JSON.stringify({ mgaMethodNames })}`);
  }
  await mgaMethodListbox.getByRole("option", { name: /MICOM and Two-Group Permutation MGA/i }).click();
  const mgaGroupColumn = mgaDialog.locator("#nd-calculation-group-column");
  const mgaGroupA = mgaDialog.locator("#nd-calculation-group-a");
  const mgaGroupB = mgaDialog.locator("#nd-calculation-group-b");
  const mgaPermutationInput = mgaDialog.locator("#nd-calculation-group-permutations");
  const micomConfidenceInput = mgaDialog.locator("#nd-calculation-micom-confidence");
  const micomConfiguralCheckbox = mgaDialog.locator("#nd-calculation-micom-configural");
  await mgaGroupA.locator('option[value="A"]').waitFor({ state: "attached", timeout: 30_000 });
  const startMga = mgaDialog.getByRole("button", { name: "Start group analysis", exact: true });
  const mgaMethodScope = (await mgaDialog.locator(".nd-mga-settings").textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const fixedWeighting = (await mgaDialog.locator(".nd-setting-note").filter({ hasText: /^Weighting scheme/ }).locator("strong").textContent())?.trim() ?? "";
  const fixedPreprocessing = (await mgaDialog.locator(".nd-setting-note").filter({ hasText: /^Result data/ }).locator("strong").textContent())?.trim() ?? "";
  const blockersBeforeConfigural = await mgaDialog.locator(".nd-blocker li").allTextContents();
  evidence.checks.mgaCalculationDialog = {
    methods: mgaMethodNames,
    selectedMethod: (await mgaMethodListbox.getByRole("option", { selected: true }).locator("strong").textContent())?.trim() ?? "",
    groupColumn: await mgaGroupColumn.inputValue(),
    groupA: await mgaGroupA.inputValue(),
    groupB: await mgaGroupB.inputValue(),
    defaultPermutations: await mgaPermutationInput.inputValue(),
    permutationMinimum: await mgaPermutationInput.getAttribute("min"),
    permutationMaximum: await mgaPermutationInput.getAttribute("max"),
    confidence: {
      value: await micomConfidenceInput.inputValue(),
      minimum: await micomConfidenceInput.getAttribute("min"),
      maximum: await micomConfidenceInput.getAttribute("max"),
      step: await micomConfidenceInput.getAttribute("step"),
    },
    configuralConfirmation: {
      count: await micomConfiguralCheckbox.count(),
      initiallyChecked: await micomConfiguralCheckbox.isChecked(),
      blockersBefore: blockersBeforeConfigural,
      checkedAfter: null,
      blockersAfter: [],
    },
    weighting: fixedWeighting,
    preprocessing: fixedPreprocessing,
    twoTailedAMinusB: /Two-tailed; Group A\s*(?:−|-)\s*Group B/i.test(mgaMethodScope),
    automaticStepsTwoAndThree: /Step 2 composition and Step 3 pooled-score means and variances are tested with the same deterministic permutations/i.test(mgaMethodScope),
    startEnabledBeforeConfigural: await startMga.isEnabled(),
  };
  if (evidence.checks.mgaCalculationDialog.selectedMethod !== "MICOM and Two-Group Permutation MGA"
    || evidence.checks.mgaCalculationDialog.groupColumn !== "group"
    || evidence.checks.mgaCalculationDialog.groupA !== "A"
    || evidence.checks.mgaCalculationDialog.groupB !== "B"
    || evidence.checks.mgaCalculationDialog.defaultPermutations !== "5000"
    || evidence.checks.mgaCalculationDialog.permutationMinimum !== "5000"
    || evidence.checks.mgaCalculationDialog.permutationMaximum !== "10000"
    || JSON.stringify(evidence.checks.mgaCalculationDialog.confidence) !== JSON.stringify({ value: "95", minimum: "80", maximum: "99.9", step: "0.1" })
    || evidence.checks.mgaCalculationDialog.configuralConfirmation.count !== 1
    || evidence.checks.mgaCalculationDialog.configuralConfirmation.initiallyChecked !== false
    || !evidence.checks.mgaCalculationDialog.configuralConfirmation.blockersBefore.some((blocker) => /Confirm MICOM Step 1/i.test(blocker))
    || evidence.checks.mgaCalculationDialog.weighting !== "Path weighting (fixed)"
    || evidence.checks.mgaCalculationDialog.preprocessing !== "Standardized (fixed)"
    || !evidence.checks.mgaCalculationDialog.twoTailedAMinusB
    || !evidence.checks.mgaCalculationDialog.automaticStepsTwoAndThree
    || evidence.checks.mgaCalculationDialog.startEnabledBeforeConfigural) {
    throw new Error(`The MICOM/MGA calculation dialog did not retain its explicit A/B, fixed path/standardized, 5,000-permutation, and Step 1 confirmation contract: ${JSON.stringify(evidence.checks.mgaCalculationDialog)}`);
  }
  await micomConfiguralCheckbox.check();
  await page.waitForFunction(() => !Array.from(document.querySelectorAll(".nd-dialog-calculation .nd-blocker li"))
    .some((item) => /Confirm MICOM Step 1/i.test(item.textContent ?? "")), undefined, { timeout: 5_000 });
  evidence.checks.mgaCalculationDialog.configuralConfirmation.checkedAfter = await micomConfiguralCheckbox.isChecked();
  evidence.checks.mgaCalculationDialog.configuralConfirmation.blockersAfter = await mgaDialog.locator(".nd-blocker li").allTextContents();
  if (!evidence.checks.mgaCalculationDialog.configuralConfirmation.checkedAfter
    || evidence.checks.mgaCalculationDialog.configuralConfirmation.blockersAfter.some((blocker) => /Confirm MICOM Step 1/i.test(blocker))) {
    throw new Error(`MICOM Step 1 confirmation did not clear its explicit blocker: ${JSON.stringify(evidence.checks.mgaCalculationDialog.configuralConfirmation)}`);
  }
  await capture(mgaCaptureName(64, "micom-v4-dialog"));

  await mgaPermutationInput.fill(String(mgaRuntimePermutationSamples));
  if (!await startMga.isEnabled()) {
    throw new Error(`The native MGA job was blocked at ${mgaRuntimePermutationSamples} valid permutations: ${(await mgaDialog.locator(".nd-blocker li").allTextContents()).join(" | ")}`);
  }
  const mgaRunningWait = mgaDialog.locator(".nd-run-progress.running").waitFor({ state: "visible", timeout: 30_000 });
  await startMga.click();
  await mgaRunningWait;
  const cancelMga = mgaDialog.getByRole("button", { name: "Cancel calculation", exact: true });
  await cancelMga.waitFor({ state: "visible", timeout: 5_000 });
  evidence.checks.mgaRunning = await mgaDialog.locator(".nd-run-progress.running").evaluate((element) => ({
    status: "running",
    phase: element.querySelector("strong")?.textContent?.trim() ?? "",
    message: element.querySelector("p")?.textContent?.trim() ?? "",
    progressValue: element.querySelector("progress")?.getAttribute("value") ?? null,
    progressMax: element.querySelector("progress")?.getAttribute("max") ?? null,
    logEntries: element.querySelectorAll("ol li").length,
  }));
  evidence.checks.mgaRunning.cancelVisible = await cancelMga.count() === 1;
  evidence.checks.mgaRunning.cancelEnabled = await cancelMga.isEnabled();
  evidence.checks.mgaRunning.permutationSamples = mgaRuntimePermutationSamples;
  if (!evidence.checks.mgaRunning.cancelVisible
    || !evidence.checks.mgaRunning.cancelEnabled
    || evidence.checks.mgaRunning.progressMax !== "100") {
    throw new Error(`The native MGA job did not expose a genuine progress and cancellation contract: ${JSON.stringify(evidence.checks.mgaRunning)}`);
  }
  await capture(mgaCaptureName(65, "micom-v4-running"));

  await waitForSurface("results", mgaCompletionTimeoutMs);
  const selectedMgaRunOption = page.locator(".nd-run-select select option:checked").filter({ hasText: /MICOM and Two-Group Permutation MGA/i });
  await selectedMgaRunOption.waitFor({ state: "attached", timeout: mgaCompletionTimeoutMs });
  const mgaRunId = await page.locator(".nd-run-select select").inputValue();
  const initialMgaSelectedTable = (await page.locator('.nd-result-tree [role="treeitem"][aria-selected="true"]').textContent())?.replace(/\s+/g, " ").trim() ?? "";
  if (initialMgaSelectedTable !== "MICOM invariance summary") {
    throw new Error(`The completed MICOM/MGA run did not auto-open its primary invariance summary: ${initialMgaSelectedTable || "no selected table"}`);
  }
  evidence.checks.mgaResult = {
    runId: mgaRunId,
    runLabel: (await selectedMgaRunOption.textContent())?.trim() ?? "",
    autoOpenedSurface: "results",
    autoOpenedTable: initialMgaSelectedTable,
    navigation: await inspectMgaResultTree(mgaRuntimePermutationSamples),
  };
  const mgaRunDetails = await inspectCurrentRunDetails();
  const mgaRunVersionTokens = String(mgaRunDetails.properties["Method version"] ?? "").split("+");
  evidence.checks.mgaResult.runDetails = mgaRunDetails;
  if (mgaRunDetails.properties.Method !== "MICOM and Two-Group Permutation MGA"
    || mgaRunDetails.properties.Weighting !== "path"
    || mgaRunDetails.properties.Preprocessing !== "standardized"
    || ![mgaMethodVersion, mgaPermutationMethodVersion, micomMethodVersion]
      .every((version) => mgaRunVersionTokens.filter((token) => token === version).length === 1)
    || mgaRunVersionTokens.some((token) => /(?:pls_mga_two_group|pls_mga_permutation|micom)_v[1-3]$/.test(token))
    || mgaRunDetails.logEntries < 1) {
    throw new Error(`The completed group run did not expose current v4 provenance, fixed estimation scope, legacy-v1/v2/v3 exclusion, and genuine stored logs: ${JSON.stringify(mgaRunDetails)}`);
  }
  await capture(mgaCaptureName(66, "micom-v4-results"));

  const mgaExportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await mgaExportCommand.click();
  const mgaExportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await mgaExportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const mgaXlsxExport = mgaExportDialog.getByRole("button", { name: /XLSX workbook/i });
  await mgaXlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const mgaReviewerPackText = (await mgaExportDialog.locator(".nd-export-list button").filter({ hasText: "Reviewer pack" }).textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const mgaPrintText = (await mgaExportDialog.locator(".nd-export-list button").filter({ hasText: "Print / PDF" }).textContent())?.replace(/\s+/g, " ").trim() ?? "";
  evidence.checks.mgaExport = {
    selectedRunId: mgaRunId,
    formats: await mgaExportDialog.locator(".nd-export-list button").count(),
    xlsxEnabled: await mgaXlsxExport.isEnabled(),
    modelDiagramCount: await mgaExportDialog.getByRole("button", { name: /Model diagram/i }).count(),
    reviewerPackText: mgaReviewerPackText,
    printText: mgaPrintText,
    selectedTable: initialMgaSelectedTable,
    nativeXlsx: null,
  };
  if (evidence.checks.mgaExport.formats !== 5
    || !evidence.checks.mgaExport.xlsxEnabled
    || evidence.checks.mgaExport.modelDiagramCount !== 0
    || !/Results tables and run provenance/i.test(evidence.checks.mgaExport.reviewerPackText)
    || !/Print the selected MGA results table/i.test(evidence.checks.mgaExport.printText)) {
    throw new Error(`The completed MGA result did not expose the expected native exports: ${JSON.stringify(evidence.checks.mgaExport)}`);
  }
  if (!requestedMgaNativeExportPath) {
    throw new Error("QUICKPLS_MGA_NATIVE_EXPORT_PATH is required for authoritative packaged MICOM/MGA export acceptance; the harness will not replace a genuine export with an enabled-button assertion.");
  }
  const mgaExportTargetPath = await validateRequestedNativeExportPath(requestedMgaNativeExportPath, "QUICKPLS_MGA_NATIVE_EXPORT_PATH");
  const mgaNativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: mgaExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: [
      "Two-group sample summary",
      "MICOM invariance summary",
      "Group path coefficients",
      "Group outer loadings",
      "Group outer weights",
      "Run provenance",
    ],
    expectedSharedStrings: [...mgaTableContracts.map((table) => table.title), "Run provenance"],
  });
  let mgaHelperCompleted = false;
  try {
    const ready = await mgaNativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") {
      throw new Error(`Native MICOM/MGA XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    }
    await mgaXlsxExport.click();
    const completion = await mgaNativeSaveHelper.completed;
    mgaHelperCompleted = true;
    if (!completion.passed) throw new Error(`Native MICOM/MGA XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(mgaExportTargetPath)}.`;
    const feedback = mgaExportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(mgaExportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(mgaExportTargetPath);
    const micomStep3Sheets = workbookSheets.filter((sheet) => sheet.startsWith("MICOM Step 3 - equality"));
    const allSheetsDistinct = new Set(workbookSheets).size === workbookSheets.length;
    evidence.checks.mgaExport.nativeXlsx = {
      attempted: true,
      targetPath: mgaExportTargetPath,
      helper: { ready, completion },
      appFeedback: (await feedback.textContent())?.trim() ?? "",
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
      allSheetsDistinct,
      micomStep3Sheets,
    };
    if (!file.isFile() || file.size <= 0
      || evidence.checks.mgaExport.nativeXlsx.appFeedback !== expectedFeedback
      || workbookSheets.length !== mgaTableContracts.length + 1
      || !allSheetsDistinct || micomStep3Sheets.length !== 2
      || !workbookSheets.includes("Run provenance")) {
      throw new Error(`The packaged MICOM/MGA XLSX did not contain every v4 table, two distinct MICOM Step 3 sheets, and provenance: ${JSON.stringify(evidence.checks.mgaExport.nativeXlsx)}`);
    }
  } finally {
    if (!mgaHelperCompleted) mgaNativeSaveHelper.stop();
  }
  await capture(mgaCaptureName(67, "export"));
  await mgaExportDialog.getByRole("button", { name: "Close", exact: true }).click();

  const editMgaModel = page.locator(".nd-commandbar button").filter({ hasText: /^Edit Model$/ });
  if (await editMgaModel.count() !== 1 || !await editMgaModel.isEnabled()) {
    throw new Error("Completed MGA Results did not expose exactly one enabled Edit Model command.");
  }
  await editMgaModel.click();
  await waitForSurface("model");
  const configuredGroupInModel = page.locator(".nd-variable-item").filter({ hasText: /^group\s*Group$/i });
  evidence.checks.mgaEditModel = {
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await structuralPaths().count(),
    groupingVariableCount: await configuredGroupInModel.count(),
    groupingVariableDisabled: await configuredGroupInModel.isDisabled(),
    selectedResultRetained: true,
  };
  if (evidence.checks.mgaEditModel.constructs !== 3
    || evidence.checks.mgaEditModel.assignedIndicators !== 6
    || evidence.checks.mgaEditModel.structuralPaths !== 2
    || evidence.checks.mgaEditModel.groupingVariableCount !== 1
    || !evidence.checks.mgaEditModel.groupingVariableDisabled) {
    throw new Error(`Edit Model did not restore the same live MGA model with its grouping variable protected from indicator assignment: ${JSON.stringify(evidence.checks.mgaEditModel)}`);
  }
  await capture(mgaCaptureName(68, "edit-model"));

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedMgaArchive = await inspectSavedMgaArchive(mgaProjectPath, mgaRunId);
  await reloadToLauncher();
  await openRecentProject(mgaProjectName, mgaProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedMgaOption = page.locator(".nd-run-select select option").filter({ hasText: /MICOM and Two-Group Permutation MGA/i }).first();
  await reopenedMgaOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedMgaRunId = await reopenedMgaOption.getAttribute("value");
  if (!reopenedMgaRunId) throw new Error("The reopened MGA run option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedMgaRunId);
  const reopenedMgaNavigation = await inspectMgaResultTree(mgaRuntimePermutationSamples);
  evidence.checks.mgaSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    expectedRunId: mgaRunId,
    expectedSelectedTable: evidence.checks.mgaResult.navigation.selectedTable,
    runOptions: (await page.locator(".nd-run-select select option").allTextContents()).map((label) => label.trim()),
    archive: savedMgaArchive,
    navigation: reopenedMgaNavigation,
  };
  if (reopenedMgaRunId !== mgaRunId
    || evidence.checks.mgaSaveReopen.selectedRunId !== mgaRunId
    || !evidence.checks.mgaSaveReopen.runOptions.some((label) => /MICOM and Two-Group Permutation MGA/i.test(label))
    || evidence.checks.mgaSaveReopen.navigation.selectedTable !== evidence.checks.mgaResult.navigation.selectedTable
    || !evidence.checks.mgaSaveReopen.navigation.allRequiredTablesVisible
    || !evidence.checks.mgaSaveReopen.navigation.noPooledDiagram
    || !evidence.checks.mgaSaveReopen.navigation.noPlaceholderNa) {
    throw new Error(`The completed group result did not survive Ctrl+S, reload, and Recent Projects reopen: ${JSON.stringify(evidence.checks.mgaSaveReopen)}`);
  }
  await capture(mgaCaptureName(69, "micom-v4-reopen"));

  if (!mgaOnly) {
  await seedRecentProject({
    name: ccaProjectName,
    path: ccaProjectPath,
    openedAt: "2026-08-11T00:00:00.000Z",
  });
  await reloadToLauncher();
  const ccaRecentRow = exactRecentProjectRow(ccaProjectName, ccaProjectPath);
  await ccaRecentRow.waitFor({ state: "visible", timeout: 10_000 });
  if (await ccaRecentRow.count() !== 1 || !(await ccaRecentRow.textContent())?.includes(ccaProjectPath)) {
    throw new Error("The deterministic CCA project was not exposed through one truthful Recent Projects row.");
  }
  await openRecentProject(ccaProjectName, ccaProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const ccaDatasetStatus = (await page.locator(".nd-statusbar").textContent())?.replace(/\s+/g, " ").trim() ?? "";
  const ccaDataHeaders = (await page.locator(".nd-data-table thead th").allTextContents()).map((value) => value.trim());
  evidence.checks.ccaFixtureProvisioning.visibleDataset = {
    cases: ccaDatasetStatus.includes("132 cases") ? 132 : null,
    columns: ccaDataHeaders,
    status: ccaDatasetStatus,
    deterministicNonSaturatedModel: "X -> Z -> Y",
  };
  if (evidence.checks.ccaFixtureProvisioning.visibleDataset.cases !== 132
    || !["x1", "x2", "z1", "z2", "y1", "y2"].every((column) => ccaDataHeaders.includes(column))) {
    throw new Error(`The visible CCA project did not load the tracked 132-row six-indicator fixture: ${JSON.stringify(evidence.checks.ccaFixtureProvisioning.visibleDataset)}`);
  }
  await capture(ccaCaptureName(70, "fixture-data"));

  const initialCcaModel = await createInitialEditableModel(ccaProjectName, ccaModelName);
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const ccaInvalidArchiveBefore = await inspectMediationArchiveRunState(ccaProjectPath);
  const ccaInvalidDialog = await openCalculationFromToolbar();
  const ccaInvalidListbox = ccaInvalidDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await ccaInvalidListbox.getByRole("option", { name: /CCA composite residual diagnostics/i }).click();
  const ccaInvalidStart = ccaInvalidDialog.getByRole("button", { name: "Start composite diagnostics", exact: true });
  const ccaInvalidBlockers = (await ccaInvalidDialog.locator(".nd-blocker li").allTextContents()).map(compactVisibleText);
  evidence.checks.ccaInvalidSetup = {
    attempted: true,
    selectedMethod: compactVisibleText(await ccaInvalidListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    startEnabled: await ccaInvalidStart.isEnabled(),
    blockers: ccaInvalidBlockers,
    emptyModelBlocker: ccaInvalidBlockers.some((row) => /require at least two constructs/i.test(row)),
    archiveBefore: ccaInvalidArchiveBefore,
    archiveAfter: null,
    archiveStateUnchanged: false,
    resultCreated: false,
  };
  await capture(ccaCaptureName("70a", "invalid-setup"));
  await ccaInvalidDialog.getByRole("button", { name: "Close", exact: true }).click();
  await ccaInvalidDialog.waitFor({ state: "hidden", timeout: 10_000 });
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const ccaInvalidArchiveAfter = await inspectMediationArchiveRunState(ccaProjectPath);
  evidence.checks.ccaInvalidSetup.archiveAfter = ccaInvalidArchiveAfter;
  evidence.checks.ccaInvalidSetup.archiveStateUnchanged = JSON.stringify(ccaInvalidArchiveAfter) === JSON.stringify(ccaInvalidArchiveBefore);
  evidence.checks.ccaInvalidSetup.resultCreated = ccaInvalidArchiveAfter.resultCount > ccaInvalidArchiveBefore.resultCount;
  if (evidence.checks.ccaInvalidSetup.selectedMethod !== "CCA composite residual diagnostics"
    || evidence.checks.ccaInvalidSetup.startEnabled
    || !evidence.checks.ccaInvalidSetup.emptyModelBlocker
    || !evidence.checks.ccaInvalidSetup.archiveStateUnchanged
    || evidence.checks.ccaInvalidSetup.resultCreated) {
    throw new Error(`The empty-model packaged CCA setup did not fail closed without creating calculation state: ${JSON.stringify(evidence.checks.ccaInvalidSetup)}`);
  }
  await buildThreeConstructCcaModel();
  const ccaPaths = structuralPaths();
  const firstCcaPath = await inspectVisibleStructuralPath(ccaPaths.nth(0));
  const secondCcaPath = await inspectVisibleStructuralPath(ccaPaths.nth(1));
  evidence.checks.visibleCcaModelBuild = {
    ...initialCcaModel,
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await ccaPaths.count(),
    paths: [firstCcaPath, secondCcaPath],
    nonSaturated: true,
  };
  if (evidence.checks.visibleCcaModelBuild.constructs !== 3
    || evidence.checks.visibleCcaModelBuild.assignedIndicators !== 6
    || evidence.checks.visibleCcaModelBuild.structuralPaths !== 2
    || firstCcaPath.Source !== "X" || firstCcaPath.Target !== "Z"
    || secondCcaPath.Source !== "Z" || secondCcaPath.Target !== "Y") {
    throw new Error(`The visible CCA authoring workflow did not produce the exact non-saturated X -> Z -> Y model: ${JSON.stringify(evidence.checks.visibleCcaModelBuild)}`);
  }
  await capture(ccaCaptureName(71, "model"));

  const ccaDialog = await openCalculationFromToolbar();
  const ccaListbox = ccaDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const ccaMethods = (await ccaListbox.getByRole("option").locator("strong").allTextContents()).map((label) => label.trim());
  await ccaListbox.getByRole("option", { name: /CCA composite residual diagnostics/i }).click();
  const ccaResultData = ccaDialog.locator(".nd-setting-note").filter({ hasText: "Result data" });
  const ccaScope = ccaDialog.locator(".nd-setting-note").filter({ hasText: "Supported setup" });
  const ccaMissingData = ccaDialog.locator(".nd-setting-note").filter({ hasText: "Missing data" });
  const ccaPcaOption = ccaDialog.locator('#nd-calculation-weighting option[value="pca"]');
  const startCca = ccaDialog.getByRole("button", { name: "Start composite diagnostics", exact: true });
  const ccaSelectedPanelText = (await ccaDialog.locator("#nd-calculation-panel").textContent())?.replace(/\s+/g, " ").trim() ?? "";
  evidence.checks.ccaCalculationDialog = {
    methods: ccaMethods,
    selectedMethod: (await ccaListbox.getByRole("option", { selected: true }).locator("strong").textContent())?.trim() ?? "",
    category: (await ccaListbox.locator("#nd-calculation-category-assessment").textContent())?.trim() ?? "",
    resultData: (await ccaResultData.locator("strong").textContent())?.trim() ?? "",
    validatedScope: (await ccaScope.locator("strong").textContent())?.trim() ?? "",
    missingData: (await ccaMissingData.locator("strong").textContent())?.trim() ?? "",
    weighting: await ccaDialog.locator("#nd-calculation-weighting").inputValue(),
    pcaDisabled: await ccaPcaOption.evaluate((option) => option.disabled),
    maximumIterations: await ccaDialog.locator("#nd-calculation-max-iterations").inputValue(),
    tolerance: await ccaDialog.locator("#nd-calculation-tolerance").inputValue(),
    unsupportedControls: await ccaDialog.locator([
      "#nd-calculation-bootstrap-samples",
      "#nd-calculation-confidence",
      "#nd-calculation-studentized",
      "#nd-calculation-permutations",
      "#nd-calculation-group-permutations",
      "#nd-calculation-seed",
      "#nd-calculation-workers",
      "#nd-calculation-case-weight",
    ].join(", ")).count(),
    startEnabled: await startCca.isEnabled(),
    blockers: await ccaDialog.locator(".nd-blocker li").allTextContents(),
    noInventedDecisionControls: !/threshold|pass\/fail|fit classification|p[- ]?value|confidence interval/i.test(ccaSelectedPanelText),
  };
  if (JSON.stringify(ccaMethods) !== JSON.stringify(expectedOptionLabels)
    || evidence.checks.ccaCalculationDialog.selectedMethod !== "CCA composite residual diagnostics"
    || evidence.checks.ccaCalculationDialog.category !== "Assessment"
    || evidence.checks.ccaCalculationDialog.resultData !== "Standardized (fixed)"
    || evidence.checks.ccaCalculationDialog.validatedScope !== "Reflective composite path model; descriptive residual diagnostics only"
    || evidence.checks.ccaCalculationDialog.missingData !== "Listwise deletion"
    || evidence.checks.ccaCalculationDialog.weighting !== "path"
    || !evidence.checks.ccaCalculationDialog.pcaDisabled
    || evidence.checks.ccaCalculationDialog.maximumIterations !== "3000"
    || evidence.checks.ccaCalculationDialog.tolerance !== "1e-7"
    || evidence.checks.ccaCalculationDialog.unsupportedControls !== 0
    || !evidence.checks.ccaCalculationDialog.startEnabled
    || evidence.checks.ccaCalculationDialog.blockers.length !== 0
    || !evidence.checks.ccaCalculationDialog.noInventedDecisionControls) {
    throw new Error(`The packaged CCA setup did not match its exact bounded ready-state contract: ${JSON.stringify(evidence.checks.ccaCalculationDialog)}`);
  }
  await capture(ccaCaptureName(72, "dialog"));

  const ccaActive = ccaDialog.locator(".nd-run-progress.queued, .nd-run-progress.validating, .nd-run-progress.running").first();
  const ccaActiveWait = ccaActive.waitFor({ state: "visible", timeout: 20_000 });
  await startCca.click();
  await ccaActiveWait;
  evidence.checks.ccaRunning = await ccaActive.evaluate((element) => ({
    status: ["queued", "validating", "running"].find((candidate) => element.classList.contains(candidate)) ?? "unknown",
    phase: element.querySelector("strong")?.textContent?.trim() ?? "",
    message: element.querySelector("p")?.textContent?.trim() ?? "",
    progressValue: element.querySelector("progress")?.getAttribute("value") ?? null,
    progressMax: element.querySelector("progress")?.getAttribute("max") ?? null,
    logEntries: element.querySelectorAll("ol li").length,
    cancelVisible: Array.from(element.closest("form")?.querySelectorAll("button") ?? []).some((button) => button.textContent?.includes("Cancel calculation")),
  }));
  if (!["queued", "validating", "running"].includes(evidence.checks.ccaRunning.status)
    || !evidence.checks.ccaRunning.phase
    || !evidence.checks.ccaRunning.cancelVisible) {
    throw new Error(`The CCA calculation did not expose a genuine active lifecycle state: ${JSON.stringify(evidence.checks.ccaRunning)}`);
  }
  await capture(ccaCaptureName(73, "running"));

  await waitForSurface("results", 120_000);
  const selectedCcaRunOption = page.locator(".nd-run-select select option:checked").filter({ hasText: /CCA composite residual diagnostics/i });
  await selectedCcaRunOption.waitFor({ state: "attached", timeout: 120_000 });
  const ccaRunId = await page.locator(".nd-run-select select").inputValue();
  const ccaNavigation = await inspectCcaResultTree();
  const ccaRunDetails = await inspectCurrentRunDetails();
  const ccaResultProperties = await page.locator(".nd-properties .nd-property-list").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  evidence.checks.ccaResult = {
    runId: ccaRunId,
    runLabel: (await selectedCcaRunOption.textContent())?.trim() ?? "",
    autoOpenedSurface: "results",
    autoOpenedTable: ccaNavigation.initialSelectedTable,
    methodVersion: ccaRunDetails.properties["Method version"] ?? null,
    nestedModelVersion: null,
    nestedModelLabel: ccaNavigation.nestedModelLabel,
    correlationPairs: ccaNavigation.correlationPairs,
    maximumAbsoluteResidual: ccaNavigation.maximumAbsoluteResidual,
    navigation: ccaNavigation,
    runDetails: ccaRunDetails,
    resultProperties: ccaResultProperties,
  };
  if (evidence.checks.ccaResult.methodVersion !== ccaProvenanceMethodVersion
    || ccaRunDetails.properties.Method !== "CCA composite residual diagnostics"
    || ccaRunDetails.properties.Weighting !== "path"
    || ccaRunDetails.properties.Preprocessing !== "standardized"
    || Object.prototype.hasOwnProperty.call(ccaRunDetails.properties, "Recorded seed")
    || Object.prototype.hasOwnProperty.call(ccaResultProperties, "Recorded seed")
    || ccaRunDetails.logEntries < 1) {
    throw new Error(`The completed CCA run details did not expose exact provenance and genuine stored logs: ${JSON.stringify(evidence.checks.ccaResult)}`);
  }
  await capture(ccaCaptureName(74, "results"));

  const ccaExportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await ccaExportCommand.click();
  const ccaExportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await ccaExportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const ccaXlsxExport = ccaExportDialog.getByRole("button", { name: /XLSX workbook/i });
  await ccaXlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const ccaExportLabels = (await ccaExportDialog.locator(".nd-export-list button strong").allTextContents()).map((label) => label.trim());
  evidence.checks.ccaExport = {
    selectedRunId: ccaRunId,
    formats: ccaExportLabels,
    xlsxEnabled: await ccaXlsxExport.isEnabled(),
    residualTablesIncluded: false,
    nativeXlsx: null,
  };
  if (!["CSV tables", "HTML report", "Reviewer pack", "XLSX workbook", "Model diagram", "Print / PDF"].every((label) => ccaExportLabels.includes(label))
    || !evidence.checks.ccaExport.xlsxEnabled) {
    throw new Error(`The completed CCA result did not expose the expected enabled native exports: ${JSON.stringify(evidence.checks.ccaExport)}`);
  }
  if (!requestedCcaNativeExportPath) {
    throw new Error("QUICKPLS_CCA_NATIVE_EXPORT_PATH is required for authoritative packaged CCA export acceptance; the harness will not replace a genuine export with an enabled-button assertion.");
  }
  const ccaExportTargetPath = await validateRequestedNativeExportPath(requestedCcaNativeExportPath, "QUICKPLS_CCA_NATIVE_EXPORT_PATH");
  const ccaNativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: ccaExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: ["Residual summary", "Composite residuals", "Run provenance"],
    expectedSharedStrings: ["Residual summary", "Composite residuals", "Run provenance"],
  });
  let ccaHelperCompleted = false;
  try {
    const ready = await ccaNativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") {
      throw new Error(`Native CCA XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    }
    await ccaXlsxExport.click();
    const completion = await ccaNativeSaveHelper.completed;
    ccaHelperCompleted = true;
    if (!completion.passed) throw new Error(`Native CCA XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(ccaExportTargetPath)}.`;
    const feedback = ccaExportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(ccaExportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(ccaExportTargetPath);
    evidence.checks.ccaExport.nativeXlsx = {
      attempted: true,
      targetPath: ccaExportTargetPath,
      helper: { ready, completion },
      appFeedback: (await feedback.textContent())?.trim() ?? "",
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    evidence.checks.ccaExport.residualTablesIncluded = workbookSheets.includes("Residual summary")
      && workbookSheets.includes("Composite residuals");
    if (!file.isFile() || file.size <= 0
      || evidence.checks.ccaExport.nativeXlsx.appFeedback !== expectedFeedback
      || !evidence.checks.ccaExport.residualTablesIncluded) {
      throw new Error(`The packaged CCA XLSX did not contain both residual tables with confirmed app feedback: ${JSON.stringify(evidence.checks.ccaExport)}`);
    }
  } finally {
    if (!ccaHelperCompleted) ccaNativeSaveHelper.stop();
  }
  await capture(ccaCaptureName(75, "export"));
  await ccaExportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedCcaArchive = await inspectSavedCcaArchive(ccaProjectPath, ccaRunId);
  evidence.checks.ccaResult.nestedModelVersion = savedCcaArchive.nestedModelVersion;
  evidence.checks.ccaResult.archiveMaximumMatchesVisible = Math.abs(
    Number(savedCcaArchive.maximumAbsoluteResidual.toFixed(4))
      - evidence.checks.ccaResult.maximumAbsoluteResidual,
  ) <= Number.EPSILON;
  if (!evidence.checks.ccaResult.archiveMaximumMatchesVisible) {
    throw new Error(`The visible and saved CCA maximum residuals diverged: ${JSON.stringify(evidence.checks.ccaResult)}`);
  }
  await reloadToLauncher();
  await openRecentProject(ccaProjectName, ccaProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedCcaOption = page.locator(".nd-run-select select option").filter({ hasText: /CCA composite residual diagnostics/i }).first();
  await reopenedCcaOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedCcaRunId = await reopenedCcaOption.getAttribute("value");
  if (!reopenedCcaRunId) throw new Error("The reopened CCA run option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedCcaRunId);
  const reopenedCcaNavigation = await inspectCcaResultTree();
  evidence.checks.ccaSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    expectedRunId: ccaRunId,
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    sameRunRestored: reopenedCcaRunId === ccaRunId
      && await page.locator(".nd-run-select select").inputValue() === ccaRunId,
    archive: savedCcaArchive,
    navigation: reopenedCcaNavigation,
  };
  if (!evidence.checks.ccaSaveReopen.sameRunRestored
    || reopenedCcaNavigation.correlationPairs !== 3
    || reopenedCcaNavigation.nestedModelLabel !== "Recursive standardized composite path model") {
    throw new Error(`The exact CCA run and residual Results did not survive explicit save and reopen: ${JSON.stringify(evidence.checks.ccaSaveReopen)}`);
  }
  await capture(ccaCaptureName(76, "reopened"));
  await openResultTable("Residual summary");
  await captureActualTauriViewportMatrix({
    checkName: "ccaPackagedViewports",
    methodSlug: "cca_residuals_v1",
    methodVersion: ccaMethodVersion,
    methodEvidenceCheck: "ccaResult",
    expectedRunId: ccaRunId,
    expectedRunLabel: "CCA composite residual diagnostics",
    expectedTableId: "cca_residual_summary",
    capturePrefix: "cca",
    captureSequence: "76",
  });

  await seedRecentProject({
    name: ipmaProjectName,
    path: ipmaProjectPath,
    openedAt: "2026-08-11T00:00:00.000Z",
  });
  await reloadToLauncher();
  const ipmaRecentRow = exactRecentProjectRow(ipmaProjectName, ipmaProjectPath);
  await ipmaRecentRow.waitFor({ state: "visible", timeout: 10_000 });
  if (await ipmaRecentRow.count() !== 1 || !(await ipmaRecentRow.textContent())?.includes(ipmaProjectPath)) {
    throw new Error("The deterministic IPMA project was not exposed through one truthful Recent Projects row.");
  }
  await openRecentProject(ipmaProjectName, ipmaProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const ipmaDatasetStatus = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const ipmaDataHeaders = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);
  evidence.checks.ipmaFixtureProvisioning.visibleDataset = {
    cases: ipmaDatasetStatus.includes("80 cases") ? 80 : null,
    columns: ipmaDataHeaders,
    status: ipmaDatasetStatus,
    deterministicModel: "X -> M; Z -> M; X -> Y; Z -> Y; M -> Y; disconnected U -> V negative-control branch",
  };
  if (evidence.checks.ipmaFixtureProvisioning.visibleDataset.cases !== 80
    || !["x1", "z1", "m1", "y1", "u1", "v1"].every((column) => ipmaDataHeaders.includes(column))) {
    throw new Error(`The visible IPMA project did not load the tracked 80-row six-indicator fixture: ${JSON.stringify(evidence.checks.ipmaFixtureProvisioning.visibleDataset)}`);
  }
  await capture(ipmaCaptureName(77, "fixture-data"));

  const initialIpmaModel = await createInitialEditableModel(ipmaProjectName, ipmaModelName);
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const ipmaInvalidArchiveBefore = await inspectMediationArchiveRunState(ipmaProjectPath);
  const ipmaInvalidDialog = await openCalculationFromToolbar();
  const ipmaInvalidListbox = ipmaInvalidDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  await ipmaInvalidListbox.getByRole("option", { name: /Importance-Performance Map Analysis/i }).click();
  const ipmaInvalidStart = ipmaInvalidDialog.getByRole("button", { name: "Start importance-performance analysis", exact: true });
  const ipmaInvalidBlockers = (await ipmaInvalidDialog.locator(".nd-blocker li").allTextContents()).map(compactVisibleText);
  evidence.checks.ipmaInvalidSetup = {
    attempted: true,
    selectedMethod: compactVisibleText(await ipmaInvalidListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    startEnabled: await ipmaInvalidStart.isEnabled(),
    blockers: ipmaInvalidBlockers,
    emptyModelBlocker: ipmaInvalidBlockers.some((row) => /requires at least one endogenous construct/i.test(row)),
    archiveBefore: ipmaInvalidArchiveBefore,
    archiveAfter: null,
    archiveStateUnchanged: false,
    resultCreated: false,
  };
  await capture(ipmaCaptureName("77a", "invalid-setup"));
  await ipmaInvalidDialog.getByRole("button", { name: "Close", exact: true }).click();
  await ipmaInvalidDialog.waitFor({ state: "hidden", timeout: 10_000 });
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const ipmaInvalidArchiveAfter = await inspectMediationArchiveRunState(ipmaProjectPath);
  evidence.checks.ipmaInvalidSetup.archiveAfter = ipmaInvalidArchiveAfter;
  evidence.checks.ipmaInvalidSetup.archiveStateUnchanged = JSON.stringify(ipmaInvalidArchiveAfter) === JSON.stringify(ipmaInvalidArchiveBefore);
  evidence.checks.ipmaInvalidSetup.resultCreated = ipmaInvalidArchiveAfter.resultCount > ipmaInvalidArchiveBefore.resultCount;
  if (evidence.checks.ipmaInvalidSetup.selectedMethod !== "Importance-Performance Map Analysis"
    || evidence.checks.ipmaInvalidSetup.startEnabled
    || !evidence.checks.ipmaInvalidSetup.emptyModelBlocker
    || !evidence.checks.ipmaInvalidSetup.archiveStateUnchanged
    || evidence.checks.ipmaInvalidSetup.resultCreated) {
    throw new Error(`The empty-model packaged IPMA setup did not fail closed without creating calculation state: ${JSON.stringify(evidence.checks.ipmaInvalidSetup)}`);
  }
  const { constructIds: ipmaConstructIds } = await buildSixConstructIpmaModelWithDisconnectedBranch();
  const ipmaPaths = structuralPaths();
  const visibleIpmaPaths = [];
  for (let index = 0; index < 6; index += 1) visibleIpmaPaths.push(await inspectVisibleStructuralPath(ipmaPaths.nth(index)));
  const observedIpmaPairs = visibleIpmaPaths.map((pathProperties) => `${pathProperties.Source} -> ${pathProperties.Target}`);
  const expectedIpmaPairs = ["X -> M", "Z -> M", "X -> Y", "Z -> Y", "M -> Y", "U -> V"];
  evidence.checks.visibleIpmaModelBuild = {
    ...initialIpmaModel,
    constructIds: ipmaConstructIds,
    constructs: await page.locator(".react-flow__node-latent").count(),
    assignedIndicators: await page.locator(".nd-variable-item.assigned").count(),
    structuralPaths: await ipmaPaths.count(),
    paths: visibleIpmaPaths,
    observedPairs: observedIpmaPairs,
  };
  if (evidence.checks.visibleIpmaModelBuild.constructs !== 6
    || evidence.checks.visibleIpmaModelBuild.assignedIndicators !== 6
    || evidence.checks.visibleIpmaModelBuild.structuralPaths !== 6
    || JSON.stringify(observedIpmaPairs) !== JSON.stringify(expectedIpmaPairs)
    || new Set(Object.values(ipmaConstructIds)).size !== 6) {
    throw new Error(`The visible IPMA authoring workflow did not produce the exact six-construct six-path model with a disconnected U-to-V negative-control branch: ${JSON.stringify(evidence.checks.visibleIpmaModelBuild)}`);
  }
  await capture(ipmaCaptureName(78, "model"));

  const ipmaDialog = await openCalculationFromToolbar();
  const ipmaListbox = ipmaDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const ipmaMethods = (await ipmaListbox.getByRole("option").locator("strong").allTextContents()).map(compactVisibleText);
  await ipmaListbox.getByRole("option", { name: /Importance-Performance Map Analysis/i }).click();
  const ipmaTarget = ipmaDialog.locator("#nd-calculation-ipma-target");
  await ipmaTarget.waitFor({ state: "visible", timeout: 10_000 });
  const ipmaTargetOptions = await ipmaTarget.locator("option").evaluateAll((options) => options.map((option) => ({
    value: option.value,
    label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
  })));
  const yTargetOption = ipmaTargetOptions.find((option) => option.value === ipmaConstructIds.y);
  if (!yTargetOption) throw new Error(`The explicit IPMA target selector did not expose Y by immutable construct id: ${JSON.stringify({ ipmaTargetOptions, ipmaConstructIds })}`);
  await ipmaTarget.selectOption(ipmaConstructIds.y);
  const ipmaNoteValue = async (label) => compactVisibleText(await ipmaDialog.locator(".nd-setting-note")
    .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
  const ipmaSelectedPanelText = compactVisibleText(await ipmaDialog.locator("#nd-calculation-panel").textContent());
  const startIpma = ipmaDialog.getByRole("button", { name: "Start importance-performance analysis", exact: true });
  evidence.checks.ipmaCalculationDialog = {
    methods: ipmaMethods,
    selectedMethod: compactVisibleText(await ipmaListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    category: compactVisibleText(await ipmaListbox.locator("#nd-calculation-category-assessment").textContent()),
    targetPlaceholder: ipmaTargetOptions.find((option) => option.value === "")?.label ?? null,
    targetOptions: ipmaTargetOptions,
    selectedTargetId: await ipmaTarget.inputValue(),
    selectedTargetLabel: compactVisibleText(await ipmaTarget.locator("option:checked").textContent()),
    weighting: await ipmaNoteValue("Weighting scheme"),
    resultData: await ipmaNoteValue("Result data"),
    missingData: await ipmaNoteValue("Missing data"),
    reportedConstructs: await ipmaNoteValue("Reported constructs"),
    performanceScope: await ipmaNoteValue("Performance definition"),
    maximumIterations: await ipmaDialog.locator("#nd-calculation-max-iterations").inputValue(),
    tolerance: await ipmaDialog.locator("#nd-calculation-tolerance").inputValue(),
    unsupportedControls: await ipmaDialog.locator([
      "#nd-calculation-weighting",
      "#nd-calculation-preprocessing",
      "#nd-calculation-bootstrap-samples",
      "#nd-calculation-confidence",
      "#nd-calculation-studentized",
      "#nd-calculation-permutations",
      "#nd-calculation-group-permutations",
      "#nd-calculation-seed",
      "#nd-calculation-workers",
      "#nd-calculation-case-weight",
      "#nd-calculation-group-column",
    ].join(", ")).count(),
    startEnabled: await startIpma.isEnabled(),
    blockers: await ipmaDialog.locator(".nd-blocker li").allTextContents(),
    predecessorOnly: ipmaSelectedPanelText.includes("Direct and indirect structural predecessors only; the target and unrelated constructs are omitted"),
    observedRange: ipmaSelectedPanelText.includes("observed-range scaling of standardized composite scores; no theoretical-range correction"),
    noCipmaOrInferenceClaim: !/\bcIPMA\b|p[- ]?value|confidence interval|bootstrap|permutation|resampling inference|significance/i.test(ipmaSelectedPanelText),
  };
  if (JSON.stringify(ipmaMethods) !== JSON.stringify(expectedOptionLabels)
    || evidence.checks.ipmaCalculationDialog.selectedMethod !== "Importance-Performance Map Analysis"
    || evidence.checks.ipmaCalculationDialog.category !== "Assessment"
    || evidence.checks.ipmaCalculationDialog.targetPlaceholder !== "Select one endogenous construct"
    || evidence.checks.ipmaCalculationDialog.selectedTargetId !== ipmaConstructIds.y
    || evidence.checks.ipmaCalculationDialog.selectedTargetLabel !== `Y [${ipmaConstructIds.y}]`
    || evidence.checks.ipmaCalculationDialog.weighting !== "Path weighting (fixed)"
    || evidence.checks.ipmaCalculationDialog.resultData !== "Standardized (fixed)"
    || evidence.checks.ipmaCalculationDialog.missingData !== "Listwise deletion"
    || evidence.checks.ipmaCalculationDialog.reportedConstructs !== "Direct and indirect structural predecessors only; the target and unrelated constructs are omitted"
    || !evidence.checks.ipmaCalculationDialog.performanceScope.includes("observed-range scaling of standardized composite scores; no theoretical-range correction")
    || evidence.checks.ipmaCalculationDialog.maximumIterations !== "3000"
    || evidence.checks.ipmaCalculationDialog.tolerance !== "1e-7"
    || evidence.checks.ipmaCalculationDialog.unsupportedControls !== 0
    || !evidence.checks.ipmaCalculationDialog.startEnabled
    || evidence.checks.ipmaCalculationDialog.blockers.length !== 0
    || !evidence.checks.ipmaCalculationDialog.predecessorOnly
    || !evidence.checks.ipmaCalculationDialog.observedRange
    || !evidence.checks.ipmaCalculationDialog.noCipmaOrInferenceClaim) {
    throw new Error(`The packaged IPMA setup did not match its exact single-target bounded ready-state contract: ${JSON.stringify(evidence.checks.ipmaCalculationDialog)}`);
  }
  await capture(ipmaCaptureName(79, "dialog"));

  const ipmaActive = ipmaDialog.locator(".nd-run-progress.queued, .nd-run-progress.validating, .nd-run-progress.running").first();
  const ipmaActiveWait = ipmaActive.waitFor({ state: "visible", timeout: 20_000 });
  await startIpma.click();
  await ipmaActiveWait;
  evidence.checks.ipmaRunning = await ipmaActive.evaluate((element) => ({
    status: ["queued", "validating", "running"].find((candidate) => element.classList.contains(candidate)) ?? "unknown",
    phase: element.querySelector("strong")?.textContent?.trim() ?? "",
    message: element.querySelector("p")?.textContent?.trim() ?? "",
    progressValue: element.querySelector("progress")?.getAttribute("value") ?? null,
    progressMax: element.querySelector("progress")?.getAttribute("max") ?? null,
    logEntries: element.querySelectorAll("ol li").length,
    cancelVisible: Array.from(element.closest("form")?.querySelectorAll("button") ?? []).some((button) => button.textContent?.includes("Cancel calculation")),
  }));
  if (!["queued", "validating", "running"].includes(evidence.checks.ipmaRunning.status)
    || !evidence.checks.ipmaRunning.phase
    || !evidence.checks.ipmaRunning.cancelVisible) {
    throw new Error(`The IPMA calculation did not expose a genuine active lifecycle state: ${JSON.stringify(evidence.checks.ipmaRunning)}`);
  }
  await capture(ipmaCaptureName(80, "running"));

  await waitForSurface("results", 120_000);
  const selectedIpmaRunOption = page.locator(".nd-run-select select option:checked").filter({ hasText: /Importance-Performance Map Analysis/i });
  await selectedIpmaRunOption.waitFor({ state: "attached", timeout: 120_000 });
  const ipmaRunId = await page.locator(".nd-run-select select").inputValue();
  const ipmaNavigation = await inspectIpmaResultTree();
  const ipmaRunDetails = await inspectCurrentRunDetails();
  const ipmaResultProperties = await page.locator(".nd-properties .nd-property-list").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  evidence.checks.ipmaResult = {
    runId: ipmaRunId,
    runLabel: compactVisibleText(await selectedIpmaRunOption.textContent()),
    autoOpenedSurface: "results",
    autoOpenedTable: ipmaNavigation.initialSelectedTable,
    navigation: ipmaNavigation,
    runDetails: ipmaRunDetails,
    resultProperties: ipmaResultProperties,
  };
  if (ipmaRunDetails.properties["Method version"] !== ipmaProvenanceMethodVersion
    || ipmaRunDetails.properties.Method !== "Importance-Performance Map Analysis"
    || ipmaRunDetails.properties.Weighting !== "path"
    || ipmaRunDetails.properties.Preprocessing !== "standardized"
    || Object.prototype.hasOwnProperty.call(ipmaRunDetails.properties, "Recorded seed")
    || Object.prototype.hasOwnProperty.call(ipmaResultProperties, "Recorded seed")
    || ipmaRunDetails.logEntries < 1) {
    throw new Error(`The completed IPMA run details did not expose exact provenance and genuine stored logs: ${JSON.stringify(evidence.checks.ipmaResult)}`);
  }
  await capture(ipmaCaptureName(81, "results"));

  const ipmaExportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await ipmaExportCommand.click();
  const ipmaExportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await ipmaExportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const ipmaXlsxExport = ipmaExportDialog.getByRole("button", { name: /XLSX workbook/i });
  await ipmaXlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const ipmaExportLabels = (await ipmaExportDialog.locator(".nd-export-list button strong").allTextContents()).map(compactVisibleText);
  evidence.checks.ipmaExport = {
    selectedRunId: ipmaRunId,
    formats: ipmaExportLabels,
    xlsxEnabled: await ipmaXlsxExport.isEnabled(),
    ipmaTablesIncluded: null,
    nativeXlsx: null,
  };
  if (!["CSV tables", "HTML report", "Reviewer pack", "XLSX workbook", "Model diagram", "Print / PDF"].every((label) => ipmaExportLabels.includes(label))
    || !evidence.checks.ipmaExport.xlsxEnabled) {
    throw new Error(`The completed IPMA result did not expose the expected enabled native exports: ${JSON.stringify(evidence.checks.ipmaExport)}`);
  }
  if (requestedIpmaNativeExportPath) {
    const ipmaExportTargetPath = await validateRequestedNativeExportPath(requestedIpmaNativeExportPath, "QUICKPLS_IPMA_NATIVE_EXPORT_PATH");
    const ipmaNativeSaveHelper = startWindowsNativeSaveExportHelper({
      targetPath: ipmaExportTargetPath,
      windowTitle: evidence.checks.runtime.title,
      expectedSheets: ["Construct importance and perfor", "Indicator performance", "Analysis details", "Run provenance"],
      expectedSharedStrings: ["Construct importance and performance", "Indicator performance", "Analysis details", "Predecessor construct", "Not applied", "Run provenance"],
    });
    let ipmaHelperCompleted = false;
    try {
      const ready = await ipmaNativeSaveHelper.ready;
      if (!ready.passed || ready.event !== "ready") {
        throw new Error(`Native IPMA XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
      }
      await ipmaXlsxExport.click();
      const completion = await ipmaNativeSaveHelper.completed;
      ipmaHelperCompleted = true;
      if (!completion.passed) throw new Error(`Native IPMA XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
      const expectedFeedback = `Saved ${path.basename(ipmaExportTargetPath)}.`;
      const feedback = ipmaExportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
      await feedback.waitFor({ state: "visible", timeout: 15_000 });
      const file = await fs.stat(ipmaExportTargetPath);
      const workbookSheets = await inspectXlsxWorkbookSheets(ipmaExportTargetPath);
      evidence.checks.ipmaExport.nativeXlsx = {
        attempted: true,
        targetPath: ipmaExportTargetPath,
        helper: { ready, completion },
        appFeedback: compactVisibleText(await feedback.textContent()),
        file: { size: file.size, isFile: file.isFile() },
        workbookSheets,
      };
      evidence.checks.ipmaExport.ipmaTablesIncluded = ["Construct importance and perfor", "Indicator performance", "Analysis details"]
        .every((sheet) => workbookSheets.includes(sheet));
      if (!file.isFile() || file.size <= 0
        || evidence.checks.ipmaExport.nativeXlsx.appFeedback !== expectedFeedback
        || !evidence.checks.ipmaExport.ipmaTablesIncluded) {
        throw new Error(`The packaged IPMA XLSX did not contain all method-specific tables with confirmed app feedback: ${JSON.stringify(evidence.checks.ipmaExport)}`);
      }
    } finally {
      if (!ipmaHelperCompleted) ipmaNativeSaveHelper.stop();
    }
  } else {
    evidence.checks.ipmaExport.nativeXlsx = {
      attempted: false,
      reason: "QUICKPLS_IPMA_NATIVE_EXPORT_PATH was not set; the optional real native XLSX Save-dialog and workbook-content gate was not requested.",
    };
  }
  await capture(ipmaCaptureName(82, "export"));
  await ipmaExportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedIpmaArchive = await inspectSavedIpmaArchive(ipmaProjectPath, ipmaRunId, ipmaConstructIds);
  await reloadToLauncher();
  await openRecentProject(ipmaProjectName, ipmaProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedIpmaOption = page.locator(".nd-run-select select option").filter({ hasText: /Importance-Performance Map Analysis/i }).first();
  await reopenedIpmaOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedIpmaRunId = await reopenedIpmaOption.getAttribute("value");
  if (!reopenedIpmaRunId) throw new Error("The reopened IPMA run option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedIpmaRunId);
  const reopenedIpmaNavigation = await inspectIpmaResultTree();
  evidence.checks.ipmaSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    expectedRunId: ipmaRunId,
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    sameRunRestored: reopenedIpmaRunId === ipmaRunId
      && await page.locator(".nd-run-select select").inputValue() === ipmaRunId,
    archive: savedIpmaArchive,
    navigation: reopenedIpmaNavigation,
  };
  if (!evidence.checks.ipmaSaveReopen.sameRunRestored
    || reopenedIpmaNavigation.initialSelectedTable !== "Construct importance and performance"
    || !reopenedIpmaNavigation.predecessorOnly
    || !reopenedIpmaNavigation.noPlaceholderOrUnsupportedClaims) {
    throw new Error(`The exact IPMA run and predecessor-only Results did not survive explicit save and reopen: ${JSON.stringify(evidence.checks.ipmaSaveReopen)}`);
  }
  await capture(ipmaCaptureName(83, "reopened"));
  await openResultTable("Construct importance and performance");
  await captureActualTauriViewportMatrix({
    checkName: "ipmaPackagedViewports",
    methodSlug: "ipma_v1",
    methodVersion: ipmaMethodVersion,
    methodEvidenceCheck: "ipmaResult",
    expectedRunId: ipmaRunId,
    expectedRunLabel: "Importance-Performance Map Analysis",
    expectedTableId: "ipma_constructs",
    capturePrefix: "ipma",
    captureSequence: "83",
  });
  }
  }

  if (!mgaOnly) {
  await seedRecentProject({
    name: ncaProjectName,
    path: ncaProjectPath,
    openedAt: "2026-08-11T00:00:00.000Z",
  });
  await reloadToLauncher();
  const ncaRecentRow = exactRecentProjectRow(ncaProjectName, ncaProjectPath);
  await ncaRecentRow.waitFor({ state: "visible", timeout: 10_000 });
  if (await ncaRecentRow.count() !== 1 || !(await ncaRecentRow.textContent())?.includes(ncaProjectPath)) {
    throw new Error("The deterministic model-free NCA project was not exposed through one truthful Recent Projects row.");
  }
  await openRecentProject(ncaProjectName, ncaProjectPath);
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const ncaDatasetStatus = compactVisibleText(await page.locator(".nd-statusbar").textContent());
  const ncaDataHeaders = (await page.locator(".nd-data-table thead th").allTextContents()).map(compactVisibleText);

  const ncaTree = await openWorkspaceExplorer(ncaProjectName);
  await ncaTree.waitFor({ state: "visible", timeout: 15_000 });
  await workspaceTreeItem("project", ncaProjectName).click();
  const ncaProjectProperties = await page.locator(".nd-explorer-properties").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  await workspaceTreeItem("models", "Models").click();
  const ncaModelProperties = await page.locator(".nd-explorer-properties").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  const ncaEditableModelTreeItems = await workspaceTreeItem("model").count();
  const ncaDataTreeItem = workspaceTreeItem("data").filter({ hasText: /nca_native_reference\.csv/i });
  if (await ncaDataTreeItem.count() !== 1) throw new Error("The NCA Project Explorer did not expose exactly one tracked data item.");
  await ncaDataTreeItem.click();
  await page.locator(".nd-explorer-detail-actions").getByRole("button", { name: "Open", exact: true }).click();
  await waitForSurface("data");
  await page.locator(".nd-data-table").waitFor({ state: "visible", timeout: 15_000 });
  const ncaDataToolbarLabels = (await page.locator('.nd-commandbar[role="toolbar"] button').allTextContents()).map(compactVisibleText);
  evidence.checks.ncaFixtureProvisioning.visibleDataset = {
    cases: ncaDatasetStatus.includes(`${ncaObservations} cases`) ? ncaObservations : null,
    columns: ncaDataHeaders,
    status: ncaDatasetStatus,
    surface: await page.locator(".nd-app").getAttribute("data-surface"),
    projectProperties: ncaProjectProperties,
    modelProperties: ncaModelProperties,
    editableModelTreeItems: ncaEditableModelTreeItems,
    toolbar: ncaDataToolbarLabels,
  };
  if (evidence.checks.ncaFixtureProvisioning.visibleDataset.cases !== ncaObservations
    || JSON.stringify(ncaDataHeaders) !== JSON.stringify(["#", "x", "y"])
    || ncaProjectProperties.Models !== "0" || ncaModelProperties.Models !== "0"
    || ncaModelProperties["Active model"] !== "None" || ncaEditableModelTreeItems !== 0
    || !ncaDataToolbarLabels.some((label) => /^Analyze/i.test(label))
    || ncaDataToolbarLabels.some((label) => /^Calculate/i.test(label))) {
    throw new Error(`The visible NCA project was not a ${ncaObservations}-case, two-variable, zero-model Data workspace with one Analyze command: ${JSON.stringify(evidence.checks.ncaFixtureProvisioning.visibleDataset)}`);
  }
  await capture(ncaCaptureName(84, "fixture-data-no-model"));

  const ncaDialog = await openAnalysisFromDataToolbar();
  const ncaListbox = ncaDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const ncaMethods = (await ncaListbox.getByRole("option").locator("strong").allTextContents()).map(compactVisibleText);
  const ncaInitiallySelected = compactVisibleText(await ncaListbox.getByRole("option", { selected: true }).locator("strong").textContent());
  const ncaOption = ncaListbox.getByRole("option", { name: /Necessary Condition Analysis/i });
  if (ncaInitiallySelected !== "Necessary Condition Analysis") await ncaOption.click();
  const ncaX = ncaDialog.locator("#nd-calculation-nca-x");
  const ncaY = ncaDialog.locator("#nd-calculation-nca-y");
  const ncaInvalidArchiveBefore = await inspectMediationArchiveRunState(ncaProjectPath);
  const ncaInvalidStart = ncaDialog.getByRole("button", { name: "Start necessary condition analysis", exact: true });
  const ncaInvalidBlockers = (await ncaDialog.locator(".nd-blocker li").allTextContents()).map(compactVisibleText);
  evidence.checks.ncaInvalidSetup = {
    attempted: true,
    selectedMethod: compactVisibleText(await ncaListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    selectedX: await ncaX.inputValue(),
    selectedY: await ncaY.inputValue(),
    startEnabled: await ncaInvalidStart.isEnabled(),
    blockers: ncaInvalidBlockers,
    missingRolesBlocked: ncaInvalidBlockers.some((row) => /Choose a numeric condition variable \(X\)/i.test(row))
      && ncaInvalidBlockers.some((row) => /Choose a numeric outcome variable \(Y\)/i.test(row)),
    archiveBefore: ncaInvalidArchiveBefore,
    archiveAfter: null,
    archiveStateUnchanged: false,
    resultCreated: false,
  };
  await capture(ncaCaptureName("84a", "invalid-setup"));
  await ncaDialog.getByRole("button", { name: "Close", exact: true }).click();
  await ncaDialog.waitFor({ state: "hidden", timeout: 10_000 });
  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const ncaInvalidArchiveAfter = await inspectMediationArchiveRunState(ncaProjectPath);
  evidence.checks.ncaInvalidSetup.archiveAfter = ncaInvalidArchiveAfter;
  evidence.checks.ncaInvalidSetup.archiveStateUnchanged = JSON.stringify(ncaInvalidArchiveAfter) === JSON.stringify(ncaInvalidArchiveBefore);
  evidence.checks.ncaInvalidSetup.resultCreated = ncaInvalidArchiveAfter.resultCount > ncaInvalidArchiveBefore.resultCount;
  if (evidence.checks.ncaInvalidSetup.selectedMethod !== "Necessary Condition Analysis"
    || evidence.checks.ncaInvalidSetup.selectedX !== "" || evidence.checks.ncaInvalidSetup.selectedY !== ""
    || evidence.checks.ncaInvalidSetup.startEnabled
    || !evidence.checks.ncaInvalidSetup.missingRolesBlocked
    || !evidence.checks.ncaInvalidSetup.archiveStateUnchanged
    || evidence.checks.ncaInvalidSetup.resultCreated) {
    throw new Error(`The missing-role packaged NCA setup did not fail closed without creating calculation state: ${JSON.stringify(evidence.checks.ncaInvalidSetup)}`);
  }
  const reopenedNcaDialog = await openAnalysisFromDataToolbar();
  const reopenedNcaListbox = reopenedNcaDialog.getByRole("listbox", { name: "Available calculation methods", exact: true });
  const reopenedNcaSetupOption = reopenedNcaListbox.getByRole("option", { name: /Necessary Condition Analysis/i });
  if (compactVisibleText(await reopenedNcaListbox.getByRole("option", { selected: true }).locator("strong").textContent()) !== "Necessary Condition Analysis") await reopenedNcaSetupOption.click();
  const activeNcaDialog = reopenedNcaDialog;
  const activeNcaListbox = reopenedNcaListbox;
  const activeNcaX = activeNcaDialog.locator("#nd-calculation-nca-x");
  const activeNcaY = activeNcaDialog.locator("#nd-calculation-nca-y");
  const ncaXOptionsBefore = await activeNcaX.locator("option").evaluateAll((options) => options.map((option) => ({
    value: option.value,
    label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
    disabled: option.disabled,
  })));
  await activeNcaX.selectOption("x");
  const ncaYOptionsAfterX = await activeNcaY.locator("option").evaluateAll((options) => options.map((option) => ({
    value: option.value,
    label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
    disabled: option.disabled,
  })));
  await activeNcaY.selectOption("y");
  const ncaCeiling = activeNcaDialog.locator("#nd-calculation-nca-ceiling");
  const ncaCeilingModes = await ncaCeiling.locator("option").evaluateAll((options) => options.map((option) => ({
    value: option.value,
    label: option.textContent?.replace(/\s+/g, " ").trim() ?? "",
  })));
  for (const mode of ["ce_fdh", "cr_fdh", "both"]) await ncaCeiling.selectOption(mode);
  const ncaPermutations = activeNcaDialog.locator("#nd-calculation-nca-permutations");
  await ncaPermutations.fill(String(ncaPermutationSamples));
  const ncaSeedInput = activeNcaDialog.locator("#nd-calculation-seed");
  await ncaSeedInput.fill(String(ncaSeed));
  const ncaNoteValue = async (label) => compactVisibleText(await activeNcaDialog.locator(".nd-setting-note")
    .filter({ hasText: label }).locator("strong").textContent().catch(() => ""));
  const ncaSelectedPanelText = compactVisibleText(await activeNcaDialog.locator("#nd-calculation-panel").textContent());
  const startNca = activeNcaDialog.getByRole("button", { name: "Start necessary condition analysis", exact: true });
  evidence.checks.ncaCalculationDialog = {
    methods: ncaMethods,
    initiallySelectedMethod: ncaInitiallySelected,
    selectedMethod: compactVisibleText(await activeNcaListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    category: compactVisibleText(await activeNcaListbox.locator("#nd-calculation-category-standalone").textContent()),
    xOptions: ncaXOptionsBefore,
    yOptionsAfterX: ncaYOptionsAfterX,
    selectedX: await activeNcaX.inputValue(),
    selectedY: await activeNcaY.inputValue(),
    ceilingModes: ncaCeilingModes,
    selectedCeiling: await ncaCeiling.inputValue(),
    permutations: {
      value: await ncaPermutations.inputValue(),
      min: await ncaPermutations.getAttribute("min"),
      max: await ncaPermutations.getAttribute("max"),
      step: await ncaPermutations.getAttribute("step"),
    },
    seed: {
      value: await ncaSeedInput.inputValue(),
      min: await ncaSeedInput.getAttribute("min"),
      max: await ncaSeedInput.getAttribute("max"),
    },
    variableData: await ncaNoteValue("Variable data"),
    validatedScope: await ncaNoteValue("Supported setup"),
    unsupportedControls: await activeNcaDialog.locator([
      "#nd-calculation-weighting",
      "#nd-calculation-preprocessing",
      "#nd-calculation-max-iterations",
      "#nd-calculation-tolerance",
      "#nd-calculation-bootstrap-samples",
      "#nd-calculation-confidence",
      "#nd-calculation-studentized",
      "#nd-calculation-permutations",
      "#nd-calculation-workers",
      "#nd-calculation-case-weight",
      "#nd-calculation-group-column",
      "#nd-calculation-ipma-target",
    ].join(", ")).count(),
    startEnabled: await startNca.isEnabled(),
    blockers: await activeNcaDialog.locator(".nd-blocker li").allTextContents(),
    noModelBlocker: !/construct|structural path|editable model|active model/i.test(compactVisibleText(await activeNcaDialog.locator(".nd-blocker").textContent().catch(() => ""))),
    noBroaderProductClaim: !/SmartPLS|full NCA|IPMA integration/i.test(ncaSelectedPanelText),
    noIdleRunState: await activeNcaDialog.locator(".nd-run-progress, progress, .nd-run-details").count() === 0,
  };
  const expectedNcaXOptions = [
    { value: "", label: "Select a numeric variable", disabled: false },
    { value: "x", label: "x", disabled: false },
    { value: "y", label: "y", disabled: false },
  ];
  const expectedNcaYOptions = [
    { value: "", label: "Select a different numeric variable", disabled: false },
    { value: "x", label: "x", disabled: true },
    { value: "y", label: "y", disabled: false },
  ];
  const expectedNcaCeilingModes = [
    { value: "both", label: "CE-FDH and CR-FDH" },
    { value: "ce_fdh", label: "CE-FDH" },
    { value: "cr_fdh", label: "CR-FDH" },
  ];
  const ncaMethodOccurrences = ncaMethods.filter((method) => method === "Necessary Condition Analysis").length;
  if (ncaMethodOccurrences !== 1 || new Set(ncaMethods).size !== ncaMethods.length
    || evidence.checks.ncaCalculationDialog.initiallySelectedMethod !== "Necessary Condition Analysis"
    || evidence.checks.ncaCalculationDialog.selectedMethod !== "Necessary Condition Analysis"
    || evidence.checks.ncaCalculationDialog.category !== "Standalone analysis"
    || JSON.stringify(ncaXOptionsBefore) !== JSON.stringify(expectedNcaXOptions)
    || JSON.stringify(ncaYOptionsAfterX) !== JSON.stringify(expectedNcaYOptions)
    || evidence.checks.ncaCalculationDialog.selectedX !== "x" || evidence.checks.ncaCalculationDialog.selectedY !== "y"
    || JSON.stringify(ncaCeilingModes) !== JSON.stringify(expectedNcaCeilingModes)
    || evidence.checks.ncaCalculationDialog.selectedCeiling !== "both"
    || JSON.stringify(evidence.checks.ncaCalculationDialog.permutations) !== JSON.stringify({ value: String(ncaPermutationSamples), min: "1", max: "10000", step: "1" })
    || JSON.stringify(evidence.checks.ncaCalculationDialog.seed) !== JSON.stringify({ value: "20260811", min: "0", max: "4294967295" })
    || evidence.checks.ncaCalculationDialog.variableData !== "Observed numeric values (fixed)"
    || evidence.checks.ncaCalculationDialog.validatedScope !== "Numeric observed-variable CE-FDH and CR-FDH analysis with observed-range bottlenecks. Multiple conditions, latent-score NCA, cIPMA, and broader ceiling variants are not included."
    || evidence.checks.ncaCalculationDialog.unsupportedControls !== 0
    || !evidence.checks.ncaCalculationDialog.startEnabled || evidence.checks.ncaCalculationDialog.blockers.length !== 0
    || !evidence.checks.ncaCalculationDialog.noModelBlocker || !evidence.checks.ncaCalculationDialog.noBroaderProductClaim
    || !evidence.checks.ncaCalculationDialog.noIdleRunState) {
    throw new Error(`The packaged NCA setup did not match its exact model-free numeric X/Y, CE-FDH/CR-FDH, and seeded permutation contract: ${JSON.stringify(evidence.checks.ncaCalculationDialog)}`);
  }
  await capture(ncaCaptureName(85, "dialog"));

  const ncaCancellationArchiveBefore = await inspectMediationArchiveRunState(ncaProjectPath);
  const ncaActive = activeNcaDialog.locator(".nd-run-progress.queued, .nd-run-progress.validating, .nd-run-progress.running").first();
  const ncaActiveWait = ncaActive.waitFor({ state: "visible", timeout: 20_000 });
  await startNca.click();
  await ncaActiveWait;
  const ncaCancellationActive = await ncaActive.evaluate((element) => ({
    status: ["queued", "validating", "running"].find((candidate) => element.classList.contains(candidate)) ?? "unknown",
    phase: element.querySelector("strong")?.textContent?.trim() ?? "",
    message: element.querySelector("p")?.textContent?.trim() ?? "",
    progressValue: element.querySelector("progress")?.getAttribute("value") ?? null,
    progressMax: element.querySelector("progress")?.getAttribute("max") ?? null,
    logEntries: element.querySelectorAll("ol li").length,
    cancelVisible: Array.from(element.closest("form")?.querySelectorAll("button") ?? []).some((button) => button.textContent?.includes("Cancel calculation")),
  }));
  const ncaCancellationProgressValue = Number(ncaCancellationActive.progressValue);
  const ncaCancellationProgressMax = Number(ncaCancellationActive.progressMax);
  if (!["queued", "validating", "running"].includes(ncaCancellationActive.status)
    || !ncaCancellationActive.phase || !ncaCancellationActive.message
    || ncaCancellationActive.logEntries < 1 || !ncaCancellationActive.cancelVisible
    || !Number.isFinite(ncaCancellationProgressValue) || !Number.isFinite(ncaCancellationProgressMax)
    || ncaCancellationProgressMax <= 0 || ncaCancellationProgressValue < 0 || ncaCancellationProgressValue > ncaCancellationProgressMax) {
    throw new Error(`The NCA cancellation attempt did not expose a genuine active lifecycle state: ${JSON.stringify(ncaCancellationActive)}`);
  }
  const ncaCancel = activeNcaDialog.getByRole("button", { name: "Cancel calculation", exact: true });
  const ncaTerminalStatePromise = page.waitForFunction(() => {
    if (document.querySelector('.nd-app[data-surface="results"]')) return "results_surface";
    const dialog = document.querySelector('.nd-dialog-calculation[role="dialog"]');
    if (!dialog) return "dialog_detached";
    if (dialog.querySelector('.nd-run-progress.cancelled[aria-busy="false"]')) return "cancelled";
    if (dialog.querySelector(".nd-run-progress.completed")) return "completed";
    return null;
  }, null, { timeout: 60_000 });
  const [ncaTerminalStateHandle] = await Promise.all([
    ncaTerminalStatePromise,
    ncaCancel.click({ timeout: 1_000 }),
  ]);
  const ncaTerminalOutcome = await ncaTerminalStateHandle.jsonValue();
  if (ncaTerminalOutcome !== "cancelled") {
    throw new Error(`completion_won_race: NCA reached ${ncaTerminalOutcome} before terminal cancellation became authoritative.`);
  }
  const ncaCancelled = activeNcaDialog.locator(".nd-run-progress.cancelled");
  await ncaCancelled.waitFor({ state: "visible", timeout: 5_000 });
  const ncaCancelledMessage = compactVisibleText(await ncaCancelled.textContent());
  const ncaPartialResults = await page.locator(".nd-run-select select option").count();
  const ncaCancellationArchiveAfter = await inspectMediationArchiveRunState(ncaProjectPath);
  const ncaCancellationArchiveUnchanged = JSON.stringify(ncaCancellationArchiveAfter)
    === JSON.stringify(ncaCancellationArchiveBefore);
  const ncaRetrySettings = {
    selectedMethod: compactVisibleText(await activeNcaListbox.getByRole("option", { selected: true }).locator("strong").textContent()),
    x: await activeNcaX.inputValue(),
    y: await activeNcaY.inputValue(),
    ceiling: await ncaCeiling.inputValue(),
    permutations: await ncaPermutations.inputValue(),
    seed: await ncaSeedInput.inputValue(),
  };
  const retryNca = activeNcaDialog.getByRole("button", { name: "Retry necessary condition analysis", exact: true });
  await retryNca.waitFor({ state: "visible", timeout: 15_000 });
  const ncaRetryEnabled = await retryNca.isEnabled();
  if (ncaPartialResults !== 0
    || !ncaCancellationArchiveUnchanged
    || ncaRetrySettings.selectedMethod !== "Necessary Condition Analysis"
    || ncaRetrySettings.x !== "x" || ncaRetrySettings.y !== "y" || ncaRetrySettings.ceiling !== "both"
    || ncaRetrySettings.permutations !== String(ncaPermutationSamples) || ncaRetrySettings.seed !== String(ncaSeed)
    || !ncaRetryEnabled) {
    throw new Error(`NCA cancellation did not terminate cleanly with its exact setup available for retry: ${JSON.stringify({ ncaCancellationActive, ncaCancelledMessage, ncaPartialResults, ncaCancellationArchiveBefore, ncaCancellationArchiveAfter, ncaRetrySettings, ncaRetryEnabled })}`);
  }
  await capture(ncaCaptureName("85a", "cancelled"));
  const retryNcaActive = activeNcaDialog.locator(".nd-run-progress.queued, .nd-run-progress.validating, .nd-run-progress.running").first();
  const retryNcaActiveWait = retryNcaActive.waitFor({ state: "visible", timeout: 20_000 });
  await retryNca.click();
  await retryNcaActiveWait;
  evidence.checks.ncaRunning = await retryNcaActive.evaluate((element) => ({
    status: ["queued", "validating", "running"].find((candidate) => element.classList.contains(candidate)) ?? "unknown",
    phase: element.querySelector("strong")?.textContent?.trim() ?? "",
    message: element.querySelector("p")?.textContent?.trim() ?? "",
    progressValue: element.querySelector("progress")?.getAttribute("value") ?? null,
    progressMax: element.querySelector("progress")?.getAttribute("max") ?? null,
    logEntries: element.querySelectorAll("ol li").length,
    cancelVisible: Array.from(element.closest("form")?.querySelectorAll("button") ?? []).some((button) => button.textContent?.includes("Cancel calculation")),
  }));
  const ncaProgressValue = Number(evidence.checks.ncaRunning.progressValue);
  const ncaProgressMax = Number(evidence.checks.ncaRunning.progressMax);
  if (!["queued", "validating", "running"].includes(evidence.checks.ncaRunning.status)
    || !evidence.checks.ncaRunning.phase || !evidence.checks.ncaRunning.message
    || evidence.checks.ncaRunning.logEntries < 1 || !evidence.checks.ncaRunning.cancelVisible
    || !Number.isFinite(ncaProgressValue) || !Number.isFinite(ncaProgressMax)
    || ncaProgressMax <= 0 || ncaProgressValue < 0 || ncaProgressValue > ncaProgressMax) {
    throw new Error(`The retried NCA calculation did not expose a genuine active lifecycle state: ${JSON.stringify(evidence.checks.ncaRunning)}`);
  }
  await capture(ncaCaptureName(86, "running"));

  await waitForSurface("results", 120_000);
  const selectedNcaRunOption = page.locator(".nd-run-select select option:checked").filter({ hasText: /Necessary Condition Analysis/i });
  await selectedNcaRunOption.waitFor({ state: "attached", timeout: 120_000 });
  const ncaRunId = await page.locator(".nd-run-select select").inputValue();
  evidence.checks.ncaCancellationRetry = {
    passed: ncaPartialResults === 0
      && ncaCancellationArchiveUnchanged
      && ncaRetryEnabled
      && ncaRetrySettings.selectedMethod === "Necessary Condition Analysis"
      && ncaRetrySettings.x === "x" && ncaRetrySettings.y === "y" && ncaRetrySettings.ceiling === "both"
      && ncaRetrySettings.permutations === String(ncaPermutationSamples) && ncaRetrySettings.seed === String(ncaSeed)
      && Boolean(ncaRunId),
    cancelledMethod: "Necessary Condition Analysis",
    cancelledSettings: { x: "x", y: "y", ceiling: "both", permutations: String(ncaPermutationSamples), seed: String(ncaSeed) },
    activeLifecycle: ncaCancellationActive,
    terminalMessage: ncaCancelledMessage,
    noPartialVisibleResult: ncaPartialResults === 0,
    noPartialCommittedResult: ncaCancellationArchiveUnchanged,
    archiveStateUnchanged: ncaCancellationArchiveUnchanged,
    archiveBefore: ncaCancellationArchiveBefore,
    archiveAfter: ncaCancellationArchiveAfter,
    retrySettings: ncaRetrySettings,
    retryEnabled: ncaRetryEnabled,
    completedRetryRunId: ncaRunId,
  };
  if (!evidence.checks.ncaCancellationRetry.passed) {
    throw new Error(`NCA cancellation/retry identity linkage failed: ${JSON.stringify(evidence.checks.ncaCancellationRetry)}`);
  }
  const ncaNavigation = await inspectNcaResultTree();
  const ncaRunDetails = await inspectCurrentRunDetails();
  const ncaResultProperties = await page.locator(".nd-properties .nd-property-list").evaluate((list) => Object.fromEntries(Array.from(list.querySelectorAll(":scope > div")).map((row) => [
    row.querySelector("dt")?.textContent?.trim() ?? "",
    row.querySelector("dd")?.textContent?.trim() ?? "",
  ])));
  const visibleEditModel = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Edit Model$/i });
  const visibleEditData = page.locator('.nd-commandbar[role="toolbar"] button').filter({ hasText: /^Edit Data$/i });
  evidence.checks.ncaResult = {
    runId: ncaRunId,
    runLabel: compactVisibleText(await selectedNcaRunOption.textContent()),
    autoOpenedSurface: "results",
    autoOpenedTable: ncaNavigation.initialSelectedTable,
    navigation: ncaNavigation,
    runDetails: ncaRunDetails,
    resultProperties: ncaResultProperties,
    editModelCommand: {
      count: await visibleEditModel.count(),
      enabled: await visibleEditModel.count() === 1 ? await visibleEditModel.isEnabled() : false,
    },
    editDataCommand: {
      count: await visibleEditData.count(),
      enabled: await visibleEditData.count() === 1 ? await visibleEditData.isEnabled() : false,
    },
  };
  if (ncaRunDetails.properties["Method version"] !== ncaMethodVersion
    || ncaRunDetails.properties.Method !== "Necessary Condition Analysis"
    || ncaRunDetails.properties["Recorded seed"] !== String(ncaSeed)
    || ncaRunDetails.properties["Condition (X)"] !== "x" || ncaRunDetails.properties["Outcome (Y)"] !== "y"
    || ncaRunDetails.properties.Observations !== String(ncaObservations) || ncaRunDetails.properties["Ceiling lines"] !== "CE-FDH and CR-FDH"
    || ncaRunDetails.properties["Requested permutations"] !== String(ncaPermutationSamples)
    || ncaRunDetails.properties["Usable permutations"] !== String(ncaPermutationSamples)
    || ncaRunDetails.properties["Missing data"] !== "Listwise deletion"
    || "Weighting" in ncaRunDetails.properties || "Preprocessing" in ncaRunDetails.properties
    || ncaRunDetails.logEntries < 1
    || ncaResultProperties.Method !== "Necessary Condition Analysis"
    || ncaResultProperties.Status !== "Completed"
    || ncaResultProperties["Condition (X)"] !== "x" || ncaResultProperties["Outcome (Y)"] !== "y"
    || ncaResultProperties.Observations !== String(ncaObservations) || ncaResultProperties["Ceiling lines"] !== "CE-FDH and CR-FDH"
    || ncaResultProperties["Requested permutations"] !== String(ncaPermutationSamples)
    || ncaResultProperties["Usable permutations"] !== String(ncaPermutationSamples)
    || ncaResultProperties["Recorded seed"] !== String(ncaSeed)
    || evidence.checks.ncaResult.editModelCommand.count !== 0
    || evidence.checks.ncaResult.editDataCommand.count !== 1 || !evidence.checks.ncaResult.editDataCommand.enabled) {
    throw new Error(`The completed NCA Results did not expose exact provenance, properties, stored logs, and a model-free return boundary: ${JSON.stringify(evidence.checks.ncaResult)}`);
  }
  await capture(ncaCaptureName(87, "results"));

  const ncaExportCommand = page.locator(".nd-commandbar button").filter({ hasText: /^Export/i });
  await ncaExportCommand.click();
  const ncaExportDialog = page.locator('.nd-dialog-export[role="dialog"]');
  await ncaExportDialog.waitFor({ state: "visible", timeout: 10_000 });
  const ncaXlsxExport = ncaExportDialog.getByRole("button", { name: /XLSX workbook/i });
  await ncaXlsxExport.waitFor({ state: "visible", timeout: 10_000 });
  const ncaExportLabels = (await ncaExportDialog.locator(".nd-export-list button strong").allTextContents()).map(compactVisibleText);
  evidence.checks.ncaExport = {
    selectedRunId: ncaRunId,
    formats: ncaExportLabels,
    xlsxEnabled: await ncaXlsxExport.isEnabled(),
    modelDiagramCount: await ncaExportDialog.getByRole("button", { name: /Model diagram/i }).count(),
    ncaTablesIncluded: null,
    nativeXlsx: null,
  };
  const expectedNcaExportLabels = ["CSV tables", "HTML report", "Reviewer pack", "XLSX workbook", "Print / PDF"];
  if (JSON.stringify(ncaExportLabels) !== JSON.stringify(expectedNcaExportLabels)
    || !evidence.checks.ncaExport.xlsxEnabled || evidence.checks.ncaExport.modelDiagramCount !== 0) {
    throw new Error(`The completed model-free NCA result did not expose the exact table/provenance exports: ${JSON.stringify(evidence.checks.ncaExport)}`);
  }
  if (!requestedNcaNativeExportPath) {
    throw new Error("QUICKPLS_NCA_NATIVE_EXPORT_PATH is required for authoritative packaged NCA export acceptance; the harness will not replace genuine XLSX Save-dialog and workbook-content evidence with enabled-button assertions.");
  }
  const ncaExportTargetPath = await validateRequestedNativeExportPath(requestedNcaNativeExportPath, "QUICKPLS_NCA_NATIVE_EXPORT_PATH");
  const ncaNativeSaveHelper = startWindowsNativeSaveExportHelper({
    targetPath: ncaExportTargetPath,
    windowTitle: evidence.checks.runtime.title,
    expectedSheets: ["Ceiling effect sizes and permut", "CE-FDH frontier peers", "CR-FDH ceiling coefficients", "Observed-range bottlenecks", "Analysis details", "Run provenance"],
    expectedSharedStrings: [
      "Ceiling effect sizes and permutation inference", "CE-FDH frontier peers", "Peer identity", "CE-FDH peer 1",
      "CR-FDH ceiling coefficients", "Observed-range bottlenecks",
      "Analysis details", "Run provenance", "CE-FDH", "CR-FDH", "Condition variable (X)",
      "Necessary Condition Analysis", "nca_v2", String(ncaSeed),
    ],
  });
  let ncaHelperCompleted = false;
  try {
    const ready = await ncaNativeSaveHelper.ready;
    if (!ready.passed || ready.event !== "ready") throw new Error(`Native NCA XLSX Save helper did not become ready: ${JSON.stringify(ready)}`);
    await ncaXlsxExport.click();
    const completion = await ncaNativeSaveHelper.completed;
    ncaHelperCompleted = true;
    if (!completion.passed) throw new Error(`Native NCA XLSX Save/export verification failed: ${JSON.stringify(completion)}`);
    const expectedFeedback = `Saved ${path.basename(ncaExportTargetPath)}.`;
    const feedback = ncaExportDialog.locator("#nd-export-feedback").filter({ hasText: expectedFeedback });
    await feedback.waitFor({ state: "visible", timeout: 15_000 });
    const file = await fs.stat(ncaExportTargetPath);
    const workbookSheets = await inspectXlsxWorkbookSheets(ncaExportTargetPath);
    evidence.checks.ncaExport.nativeXlsx = {
      attempted: true,
      targetPath: ncaExportTargetPath,
      helper: { ready, completion },
      appFeedback: compactVisibleText(await feedback.textContent()),
      file: { size: file.size, isFile: file.isFile() },
      workbookSheets,
    };
    evidence.checks.ncaExport.ncaTablesIncluded = [
      "Ceiling effect sizes and permut", "CE-FDH frontier peers", "CR-FDH ceiling coefficients", "Observed-range bottlenecks", "Analysis details", "Run provenance",
    ].every((sheet) => workbookSheets.includes(sheet));
    if (!file.isFile() || file.size <= 0 || evidence.checks.ncaExport.nativeXlsx.appFeedback !== expectedFeedback
      || !evidence.checks.ncaExport.ncaTablesIncluded) {
      throw new Error(`The packaged NCA XLSX did not contain all exact NCA and provenance sheets: ${JSON.stringify(evidence.checks.ncaExport)}`);
    }
  } finally {
    if (!ncaHelperCompleted) ncaNativeSaveHelper.stop();
  }
  await capture(ncaCaptureName(88, "export"));
  await ncaExportDialog.getByRole("button", { name: "Close", exact: true }).click();

  await page.keyboard.press("Control+S");
  await page.locator(".nd-toast").filter({ hasText: /Project saved/i }).last().waitFor({ state: "visible", timeout: 15_000 });
  await page.waitForFunction(() => !document.title.endsWith(" *"), null, { timeout: 15_000 });
  const savedNcaArchive = await inspectSavedNcaArchive(ncaProjectPath, ncaRunId);
  await reloadToLauncher();
  await openRecentProject(ncaProjectName, ncaProjectPath);
  await openMenuItem("View", "Results");
  await waitForSurface("results");
  const reopenedNcaOption = page.locator(".nd-run-select select option").filter({ hasText: /Necessary Condition Analysis/i }).first();
  await reopenedNcaOption.waitFor({ state: "attached", timeout: 15_000 });
  const reopenedNcaRunId = await reopenedNcaOption.getAttribute("value");
  if (!reopenedNcaRunId) throw new Error("The reopened NCA run option had no run identifier.");
  await page.locator(".nd-run-select select").selectOption(reopenedNcaRunId);
  const reopenedNcaNavigation = await inspectNcaResultTree();
  evidence.checks.ncaSaveReopen = {
    attempted: true,
    reopenedThroughRecentProjectRow: true,
    expectedRunId: ncaRunId,
    selectedRunId: await page.locator(".nd-run-select select").inputValue(),
    sameRunRestored: reopenedNcaRunId === ncaRunId
      && await page.locator(".nd-run-select select").inputValue() === ncaRunId,
    sameVisibleEffectsRestored: JSON.stringify(reopenedNcaNavigation.effects.rows) === JSON.stringify(ncaNavigation.effects.rows),
    sameVisiblePeersRestored: JSON.stringify(reopenedNcaNavigation.peers.rows) === JSON.stringify(ncaNavigation.peers.rows),
    archive: savedNcaArchive,
    navigation: reopenedNcaNavigation,
  };
  if (!evidence.checks.ncaSaveReopen.sameRunRestored
    || !evidence.checks.ncaSaveReopen.sameVisibleEffectsRestored
    || !evidence.checks.ncaSaveReopen.sameVisiblePeersRestored
    || reopenedNcaNavigation.initialSelectedTable !== "Ceiling effect sizes and permutation inference"
    || !reopenedNcaNavigation.bottlenecksMatch || !reopenedNcaNavigation.noModelOrQualityTree
    || !reopenedNcaNavigation.noPlaceholder || !reopenedNcaNavigation.noBroaderNcaClaim) {
    throw new Error(`The exact model-free NCA run and nca_v2 Results did not survive explicit save and reopen: ${JSON.stringify(evidence.checks.ncaSaveReopen)}`);
  }
  await capture(ncaCaptureName(89, "reopened"));
  await openResultTable("Ceiling effect sizes and permutation inference");
  await captureActualTauriViewportMatrix({
    checkName: "ncaPackagedViewports",
    methodSlug: "nca_v2",
    methodVersion: ncaMethodVersion,
    methodEvidenceCheck: "ncaResult",
    expectedRunId: ncaRunId,
    expectedRunLabel: "Necessary Condition Analysis",
    expectedTableId: "nca_ceiling_effects",
    capturePrefix: "nca",
    captureSequence: "89",
  });
  const ncaInternalOrigins = new Set([packagedTauriOrigin, packagedTauriIpcOrigin]);
  const ncaExternalRequests = observedBrowserRequests.filter((request) => request.origin
    && request.origin !== "null" && !ncaInternalOrigins.has(request.origin));
  evidence.checks.ncaFunctionalOffline = {
    passed: observedBrowserRequests.length > 0 && ncaExternalRequests.length === 0,
    analyticalWorkflowRequiresInternet: false,
    strictZeroProcessEgressClaimed: false,
    platformBackgroundEgressOutsidePageRequestScope: true,
    observedRequestCount: observedBrowserRequests.length,
    externalRequestCount: ncaExternalRequests.length,
    origins: [...new Set(observedBrowserRequests.map((request) => request.origin))].sort(),
    externalRequests: ncaExternalRequests,
  };
  if (!evidence.checks.ncaFunctionalOffline.passed) {
    throw new Error(`NCA packaged browser/app workflow crossed its functional-offline request boundary: ${JSON.stringify(evidence.checks.ncaFunctionalOffline)}`);
  }
  }
  }

  if (evidence.consoleErrors.length) throw new Error(`Console errors: ${JSON.stringify(evidence.consoleErrors)}`);
} catch (error) {
  evidence.failures.push(error instanceof Error ? error.message : String(error));
  await capture("99-tauri-native-failure-state-1440x900.png").catch(() => undefined);
  process.exitCode = 1;
} finally {
  if (uiPreferencesSeeded) {
    evidence.checks.runtimePreflight.experimentalLabsPreferenceRestored = await page.evaluate(({ key, prior }) => {
      if (prior === null) window.localStorage.removeItem(key);
      else window.localStorage.setItem(key, prior);
      return window.localStorage.getItem(key) === prior;
    }, { key: uiPreferencesKey, prior: priorUiPreferencesRaw }).catch(() => false);
  }
  if (recentProjectsSeeded) {
    evidence.checks.recentProjectsRestored = await page.evaluate(({ key, prior }) => {
      if (prior === null) window.localStorage.removeItem(key);
      else window.localStorage.setItem(key, prior);
      return window.localStorage.getItem(key) === prior;
    }, { key: recentProjectsKey, prior: priorRecentProjectsRaw }).catch(() => false);
  }
  if (evidence.focusedRun) evidence.focusedRun.completedAt = new Date().toISOString();
  evidence.passed = evidence.failures.length === 0 && evidence.consoleErrors.length === 0;
  await writeAcceptanceEvidence();
}

console.log(JSON.stringify({
  passed: evidence.passed,
  checks: Object.keys(evidence.checks),
  screenshots: evidence.screenshots.length,
  consoleErrors: evidence.consoleErrors.length,
  failures: evidence.failures,
  reportPath,
  scopedReportPath,
}, null, 2));
process.exit(evidence.failures.length ? 1 : 0);
