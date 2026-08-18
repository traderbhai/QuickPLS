import type {
  InternalLabsRecipeV4CbsemExecutionRequestV1,
  InternalRecipeV4CbsemCompletedResultV1,
  InternalRecipeV4CbsemJobSnapshotV1,
} from "./internalRecipeV4CbsemExecution";
import {
  INTERNAL_RECIPE_V4_CBSEM_BOOTSTRAP_CAPABILITY_CELL,
  INTERNAL_RECIPE_V4_CBSEM_CAPABILITY_CELL,
} from "./internalRecipeV4CbsemExecution";
import type {
  AnalysisRecipeV4,
  AnalysisRecipeV4MissingDataPolicy,
  AnalysisRecipeV4Settings,
  InternalRecipeV4ExecutionFailureV1,
} from "./internalRecipeV4PlsExecution";
import type {
  InternalProjectSchema6ResultAppendOutcomeV1,
  InternalProjectSchema6ResultAppendRequestV1,
} from "./internalProjectSchema6ResultAppend";
import type {
  InternalProjectSchema6CanonicalResultEntryV1,
  InternalProjectSchema6ResultReadOutcomeV1,
  InternalProjectSchema6ResultReadRequestV1,
} from "./internalProjectSchema6ResultRead";
import type { ProjectUpgradeInspectionV1 } from "./internalProjectUpgradeV6";
import {
  canonicalizeSemModelV4,
  hasStructuralFeedbackV4,
  validateSemModelV4,
  type SemCovarianceDenominatorV4,
  type SemModelV4,
} from "./semModelV4";
import {
  resolveSemWeightDeclarationV1,
  weightCapabilityIssueV1,
  type WeightCapabilityIssueV1,
  type WeightCapabilityTargetV1,
} from "./semWeightDeclarationV1";
import type { Dataset } from "../types";

export const INTERNAL_CBSEM_MEAN_REPLACEMENT_VARIABLE_WARNING_RATE_V1 = 0.05 as const;
export const INTERNAL_CBSEM_MEAN_REPLACEMENT_HIGH_WARNING_RATE_V1 = 0.15 as const;

export type InternalRecipeV4CbsemPreflightStageV1 =
  | "access"
  | "project"
  | "model"
  | "dataset"
  | "input"
  | "recipe";

export interface InternalRecipeV4CbsemPreflightIssueV1 {
  stage: InternalRecipeV4CbsemPreflightStageV1;
  code: string;
  subject: string;
  message: string;
  correctiveAction: string;
}

export interface InternalRecipeV4CbsemPreflightWarningV1 {
  stage: "input";
  severity: "warning" | "high";
  code: string;
  subject: string;
  message: string;
}

export interface InternalRecipeV4CbsemPreflightLayerV1 {
  stage: InternalRecipeV4CbsemPreflightStageV1;
  label: string;
  status: "ready" | "blocked";
  issues: InternalRecipeV4CbsemPreflightIssueV1[];
  warnings: InternalRecipeV4CbsemPreflightWarningV1[];
}

export interface InternalRecipeV4CbsemPreflightV1 {
  ready: boolean;
  layers: InternalRecipeV4CbsemPreflightLayerV1[];
  issues: InternalRecipeV4CbsemPreflightIssueV1[];
  warnings: InternalRecipeV4CbsemPreflightWarningV1[];
}

export interface InternalRecipeV4CbsemEngineOptionsV1 {
  tolerance: number;
  maxIterations: number;
  seed: number;
  workers: number;
  confidenceLevel: number;
  bootstrapSamples?: number;
  bootstrapInterval?: "percentile_type7" | "analytic_studentized_type7" | "bca_type7";
  bootstrapTestTail?: "two_sided" | "one_sided_greater" | "one_sided_less";
}

export interface BindInternalRecipeV4CbsemDatasetV1 {
  covarianceDenominator: SemCovarianceDenominatorV4;
  missingDataPolicy: AnalysisRecipeV4MissingDataPolicy;
  /** Keys are stable observed-variable IDs, never display labels. */
  correlationStandardDeviations?: Readonly<Record<string, number>>;
}

export class InternalRecipeV4CbsemWorkspaceError extends Error {
  constructor(
    public readonly code: string,
    public readonly subject: string,
    message: string,
    public readonly correctiveAction: string,
  ) {
    super(message);
    this.name = "InternalRecipeV4CbsemWorkspaceError";
  }
}

const STAGES: ReadonlyArray<{ stage: InternalRecipeV4CbsemPreflightStageV1; label: string }> = [
  { stage: "access", label: "Method access" },
  { stage: "project", label: "Active project" },
  { stage: "model", label: "Scientific model" },
  { stage: "dataset", label: "Resident dataset" },
  { stage: "input", label: "Estimator input" },
  { stage: "recipe", label: "Recipe settings" },
];

const issue = (
  stage: InternalRecipeV4CbsemPreflightStageV1,
  code: string,
  subject: string,
  message: string,
  correctiveAction: string,
): InternalRecipeV4CbsemPreflightIssueV1 => ({ stage, code, subject, message, correctiveAction });

function observedVariables(model: SemModelV4) {
  return model.variables.filter((variable) => variable.kind === "observed");
}

function datasetKind(dataset: Dataset): "raw" | "covariance" | "correlation" {
  return dataset.kind === "covariance" || dataset.kind === "correlation" ? dataset.kind : "raw";
}

function cbsemWeightTargetV1(missingDataPolicy: AnalysisRecipeV4MissingDataPolicy): WeightCapabilityTargetV1 {
  return missingDataPolicy === "mean_replacement"
    ? "cbsem_ml_mean_replacement_v1"
    : "cbsem_ml_v1";
}

function cbsemWeightIssueV1(
  model: SemModelV4,
  missingDataPolicy: AnalysisRecipeV4MissingDataPolicy,
): WeightCapabilityIssueV1 | null {
  const declaration = resolveSemWeightDeclarationV1(model);
  return declaration ? weightCapabilityIssueV1(cbsemWeightTargetV1(missingDataPolicy), declaration) : null;
}

function cbsemWeightIssueMessageV1(diagnostic: WeightCapabilityIssueV1): string {
  if (diagnostic.code === "case_weight_unsupported") return "This CB-SEM estimator does not execute case weights.";
  if (diagnostic.code === "frequency_weight_unsupported") return "This CB-SEM estimator does not execute frequency weights.";
  if (diagnostic.code === "sampling_weight_unsupported") return "This CB-SEM estimator does not execute sampling weights or sampling-design normalization.";
  if (diagnostic.code === "sampling_weight_normalization_unsupported") return "This CB-SEM estimator does not execute the requested sampling-weight normalization.";
  return "The legacy case-weight setting is not bound unambiguously to this SemModelV4.";
}

/**
 * Rebinds an already valid authoritative SemModelV4 to one exact resident
 * dataset. Matrix variable order follows the native dataset declaration.
 */
export function bindInternalRecipeV4CbsemDatasetV1(
  model: SemModelV4,
  dataset: Dataset,
  options: BindInternalRecipeV4CbsemDatasetV1,
): SemModelV4 {
  const kind = datasetKind(dataset);
  if (kind === "raw") return canonicalizeSemModelV4({
    ...model,
    // Import markers have already become Arrow nulls. The listwise compiler
    // consumes that null bitmap and keeps marker metadata fail-closed; the
    // mean-replacement receipt instead preserves and exact-checks the markers.
    variables: model.variables.map((variable) => variable.kind === "observed" && options.missingDataPolicy === "listwise_deletion"
      ? { ...variable, missing_markers: [] }
      : variable),
    data_binding: {
      kind: "raw",
      dataset_id: dataset.id,
      missing_data: options.missingDataPolicy,
      weight: model.data_binding.kind === "raw" ? model.data_binding.weight ?? null : null,
      cluster_variable: null,
      strata_variable: null,
    },
  });

  const matrixWeightIssue = cbsemWeightIssueV1(model, "listwise_deletion");
  if (matrixWeightIssue) throw new InternalRecipeV4CbsemWorkspaceError(
    matrixWeightIssue.code,
    matrixWeightIssue.declaration?.binding.source_column ?? "weight",
    "Matrix rebinding cannot discard an authored raw-data weight declaration.",
    matrixWeightIssue.corrective_action,
  );

  if (options.missingDataPolicy !== "listwise_deletion") throw new InternalRecipeV4CbsemWorkspaceError(
    "recipe_v4.cbsem.matrix_missing_data_treatment_unsupported",
    dataset.id,
    "Mean replacement is available only for raw continuous data in this exact CB-SEM workspace.",
    "Use listwise deletion for matrix input or choose a raw resident dataset.",
  );

  const bySource = new Map(observedVariables(model).map((variable) => [variable.source_column, variable.id]));
  const variables = dataset.columns.map((column) => bySource.get(column));
  const missingColumn = dataset.columns.find((_, index) => !variables[index]);
  if (missingColumn) throw new InternalRecipeV4CbsemWorkspaceError(
    "recipe_v4.cbsem.matrix_column_unbound",
    missingColumn,
    `Matrix column ${missingColumn} is not bound to an observed variable in this model.`,
    "Choose a matrix whose rows and columns exactly match the model indicators.",
  );

  const standardDeviations = kind === "correlation"
    ? Object.fromEntries((variables as string[]).map((id) => [id, options.correlationStandardDeviations?.[id] ?? Number.NaN]))
    : null;
  return canonicalizeSemModelV4({
    ...model,
    data_binding: {
      kind,
      dataset_id: dataset.id,
      variables: variables as string[],
      means: null,
      standard_deviations: standardDeviations,
      sample: {
        sample_size: dataset.sampleSize ?? 0,
        covariance_denominator: options.covarianceDenominator,
        effective_sample_size: null,
        degrees_of_freedom: null,
        group_sample_sizes: {},
      },
    },
  });
}

export interface InternalRecipeV4CbsemPreflightInputV1 {
  experimentalLabsEnabled: boolean;
  projectName: string;
  projectPath: string | null;
  dataset: Dataset | null;
  model: SemModelV4 | null;
  modelDiagnostics?: ReadonlyArray<{ code: string; subject: string; message: string; correctiveAction: string }>;
  missingDataPolicy: AnalysisRecipeV4MissingDataPolicy;
  engine: InternalRecipeV4CbsemEngineOptionsV1;
}

export function preflightInternalRecipeV4CbsemWorkspaceV1(
  input: InternalRecipeV4CbsemPreflightInputV1,
): InternalRecipeV4CbsemPreflightV1 {
  const issues: InternalRecipeV4CbsemPreflightIssueV1[] = [];
  const warnings: InternalRecipeV4CbsemPreflightWarningV1[] = [];
  const add = (...diagnostic: Parameters<typeof issue>) => issues.push(issue(...diagnostic));
  const bootstrapSamples = input.engine.bootstrapSamples ?? 0;
  const bootstrapInterval = input.engine.bootstrapInterval ?? "percentile_type7";
  const bootstrapTestTail = input.engine.bootstrapTestTail ?? "two_sided";
  const bootstrapEnabled = bootstrapSamples > 0;
  const boundedInterval = bootstrapInterval === "analytic_studentized_type7" || bootstrapInterval === "bca_type7";
  if (!input.projectName.trim() || input.projectName === "No project open") add("project", "recipe_v4.project_required", "project", "Open a project before starting this job.", "Open the project that owns the model and resident dataset.");
  if (!input.projectPath?.trim()) add("project", "recipe_v4.saved_project_required", "project", "The active project needs a stable saved path for identity monitoring.", "Save the active project, then reopen this workspace.");

  if (!input.model) {
    if (input.modelDiagnostics?.length) for (const diagnostic of input.modelDiagnostics) add("model", diagnostic.code, diagnostic.subject, diagnostic.message, diagnostic.correctiveAction);
    else add("model", "recipe_v4.cbsem.model_required", "model", "No authoritative SemModelV4 is available.", "Complete the scientific decisions in the Parameter Table.");
  } else {
    for (const diagnostic of validateSemModelV4(input.model)) add("model", diagnostic.code, diagnostic.subject ?? "model", diagnostic.message, "Correct the named model object in the Parameter Table.");
    if (!input.model.variables.some((variable) => variable.kind === "common_factor")) add(
      "model",
      "recipe_v4.cbsem.common_factor_required",
      "model",
      "Exact CB-SEM requires at least one authored common-factor construct.",
      "Create a reflective construct, assign its indicators, and confirm its common-factor decisions before starting this job.",
    );
    if (input.model.group.kind !== "single_group") add("model", "recipe_v4.cbsem.groups_not_available", "group", "This CB-SEM job supports one group.", "Use a single-group model for this job.");
    const groupOverride = input.model.parameters.find((parameter) => parameter.group_overrides?.length);
    if (groupOverride) add("model", "recipe_v4.cbsem.parameter_group_overrides_not_available", groupOverride.id, "Group-specific parameter overrides are not available in this job.", "Remove the group override and use one shared parameter specification.");
    if (hasStructuralFeedbackV4(input.model)) add("model", "recipe_v4.cbsem.feedback_not_available", "structural_model", "Reciprocal structural paths are not available in this job.", "Remove one path from each feedback loop and run preflight again.");
    const unsupported = input.model.variables.find((variable) => variable.kind === "composite" || variable.kind === "derived");
    if (unsupported) add("model", "recipe_v4.cbsem.common_factors_required", unsupported.id, "CB-SEM requires common factors and observed variables in this workspace.", "Confirm Common factor for each construct in the Parameter Table.");
    try {
      const weightIssue = cbsemWeightIssueV1(input.model, input.missingDataPolicy);
      if (weightIssue) add(
        "recipe",
        weightIssue.code,
        weightIssue.declaration?.binding.source_column ?? "weight",
        cbsemWeightIssueMessageV1(weightIssue),
        weightIssue.corrective_action,
      );
    } catch {
      // Malformed scientific bindings are already reported by validateSemModelV4.
      // Never derive a capability declaration from invalid input.
    }
  }

  const dataset = input.dataset;
  if (!dataset) add("dataset", "recipe_v4.cbsem.dataset_required", "dataset", "Choose a resident project dataset.", "Select a raw, covariance, or correlation dataset from this project.");
  else {
    if (!dataset.id.trim() || !dataset.fingerprint?.trim()) add("dataset", "recipe_v4.cbsem.dataset_identity_missing", dataset.id || "dataset", "The selected dataset does not have a complete resident identity.", "Reimport or reactivate the dataset so its ID and fingerprint are available.");
    if (input.model && input.model.data_binding.dataset_id !== dataset.id) add("dataset", "recipe_v4.cbsem.dataset_binding_mismatch", dataset.id, "The model binding does not match the selected resident dataset.", "Re-select the dataset to rebuild the model binding.");
    const kind = datasetKind(dataset);
    if (input.model && input.model.data_binding.kind !== kind) add("input", "recipe_v4.cbsem.input_kind_mismatch", dataset.id, "The model input kind does not match the selected dataset.", "Re-select the dataset and run preflight again.");
    const modelObserved = input.model ? observedVariables(input.model) : [];
    const sourceColumns = modelObserved.map((variable) => variable.source_column);
    const missingColumns = sourceColumns.filter((column) => !dataset.columns.includes(column));
    if (missingColumns.length) add("dataset", "recipe_v4.cbsem.source_column_missing", missingColumns[0], `The selected dataset is missing modeled column ${missingColumns[0]}.`, "Choose the exact resident dataset used to author this model.");
    const metadata = new Map((dataset.columnMetadata ?? []).map((column) => [column.name, column]));
    const nonContinuous = sourceColumns.find((column) => {
      const details = metadata.get(column);
      return details && (details.column_type !== "numeric" || details.scale_type !== "continuous");
    });
    if (nonContinuous) add("input", "recipe_v4.cbsem.continuous_numeric_required", nonContinuous, `${nonContinuous} must be a continuous numeric variable for this estimator.`, "Correct its data metadata or choose another dataset.");

    if (kind === "raw") {
      const rows = dataset.rowCount ?? dataset.rows.length;
      if (!Number.isInteger(rows) || rows < 10) add("input", "recipe_v4.cbsem.raw_sample_too_small", dataset.id, "Raw CB-SEM input requires at least 10 resident observations.", "Choose a dataset with at least 10 observations.");
      if (input.model?.data_binding.kind === "raw") {
        const boundPolicy = input.model.data_binding.missing_data;
        if (boundPolicy !== "listwise_deletion" && boundPolicy !== "mean_replacement") {
          add("recipe", "recipe_v4.cbsem.missing_data_policy_unsupported", "missing_data", "This CB-SEM workspace supports only listwise deletion or mean replacement.", "Choose one of the two available missing-data treatments.");
        } else if (boundPolicy !== input.missingDataPolicy) {
          add("recipe", "recipe_v4.cbsem.missing_data_policy_mismatch", "missing_data", "The SemModelV4 raw-data binding and Recipe-v4 setting do not name the same missing-data treatment.", "Re-select the treatment so the model binding and recipe setting are rebuilt together.");
        }
      }
      if (input.missingDataPolicy === "mean_replacement" && Number.isInteger(rows) && rows > 0) {
        for (const sourceColumn of sourceColumns) {
          const missingCount = dataset.missingByColumn?.[sourceColumn];
          if (missingCount == null) continue;
          if (!Number.isSafeInteger(missingCount) || missingCount < 0 || missingCount > rows) {
            add("dataset", "recipe_v4.cbsem.missing_count_invalid", sourceColumn, `The resident missing-value count for ${sourceColumn} is invalid.`, "Refresh or reimport the dataset before estimating a replacement mean.");
            continue;
          }
          if (missingCount === rows) {
            add("input", "recipe_v4.cbsem.mean_replacement_no_observed_value", sourceColumn, `${sourceColumn} has no observed value from which to calculate a replacement mean.`, "Provide at least one finite observed value for this modeled variable or remove it from the model.");
            continue;
          }
          const rate = missingCount / rows;
          if (rate > INTERNAL_CBSEM_MEAN_REPLACEMENT_HIGH_WARNING_RATE_V1) {
            warnings.push({
              stage: "input",
              severity: "high",
              code: "recipe_v4.cbsem.variable_missing_rate_above_15_percent",
              subject: sourceColumn,
              message: `${sourceColumn} has ${missingCount} of ${rows} values missing (${(rate * 100).toFixed(1)}%), above the 15% high-warning threshold.`,
            });
          } else if (rate >= INTERNAL_CBSEM_MEAN_REPLACEMENT_VARIABLE_WARNING_RATE_V1) {
            warnings.push({
              stage: "input",
              severity: "warning",
              code: "recipe_v4.cbsem.variable_missing_rate_at_least_5_percent",
              subject: sourceColumn,
              message: `${sourceColumn} has ${missingCount} of ${rows} values missing (${(rate * 100).toFixed(1)}%), at or above the 5% warning threshold.`,
            });
          }
        }
      }
    } else if (input.model && input.model.data_binding.kind !== "raw") {
      if (input.missingDataPolicy !== "listwise_deletion") add("recipe", "recipe_v4.cbsem.matrix_missing_data_treatment_unsupported", dataset.id, "Mean replacement is available only for raw continuous data in this CB-SEM workspace.", "Use listwise deletion for matrix input or choose a raw resident dataset.");
      const binding = input.model.data_binding;
      if (!Number.isInteger(dataset.sampleSize) || (dataset.sampleSize ?? 0) < 10) add("input", "recipe_v4.cbsem.matrix_sample_size_invalid", dataset.id, "Matrix input requires a declared integer sample size of at least 10.", "Reimport the matrix with its study sample size.");
      if (binding.sample.sample_size !== dataset.sampleSize) add("input", "recipe_v4.cbsem.matrix_sample_size_mismatch", dataset.id, "The model and resident matrix declare different sample sizes.", "Re-select the matrix so the binding uses its current sample size.");
      if ((dataset.rowCount ?? dataset.rows.length) !== dataset.columns.length) add("input", "recipe_v4.cbsem.matrix_shape_invalid", dataset.id, "The resident matrix must be square.", "Choose a square covariance or correlation matrix.");
      const boundSources = binding.variables.map((id) => modelObserved.find((variable) => variable.id === id)?.source_column ?? "");
      if (boundSources.length !== dataset.columns.length || boundSources.some((column, index) => column !== dataset.columns[index])) add("input", "recipe_v4.cbsem.matrix_order_mismatch", dataset.id, "Matrix variables must exactly follow the resident row and column order.", "Rebuild the binding from the selected matrix.");
      if (binding.means) add("input", "recipe_v4.cbsem.matrix_means_not_available", dataset.id, "Mean vectors are not supported by this job.", "Clear matrix means and use covariance-structure estimation.");
      if (binding.sample.effective_sample_size != null || binding.sample.degrees_of_freedom != null || Object.keys(binding.sample.group_sample_sizes ?? {}).length) add("input", "recipe_v4.cbsem.matrix_sample_metadata_not_available", dataset.id, "Effective sample size, supplied degrees of freedom, and group sizes are not supported here.", "Use only sample size and covariance denominator.");
      if (kind === "covariance" && binding.standard_deviations) add("input", "recipe_v4.cbsem.covariance_scales_not_allowed", dataset.id, "A covariance matrix must not provide separate scales.", "Clear the scale fields or choose correlation input.");
      if (kind === "correlation") {
        const scales = binding.standard_deviations ?? {};
        const invalidScale = binding.variables.find((id) => !Number.isFinite(scales[id]) || scales[id] <= 0);
        if (invalidScale || Object.keys(scales).length !== binding.variables.length) add("input", "recipe_v4.cbsem.correlation_scales_required", invalidScale ?? dataset.id, "Correlation input requires one finite positive standard deviation for every modeled variable.", "Enter every scale from the study covariance metadata.");
      }
    }
  }

  if (!Number.isFinite(input.engine.tolerance) || input.engine.tolerance <= 0 || input.engine.tolerance > 0.01) add("recipe", "recipe_v4.cbsem.tolerance_invalid", "tolerance", "Tolerance must be greater than zero and no more than 0.01.", "Enter a supported convergence tolerance.");
  if (!Number.isInteger(input.engine.maxIterations) || input.engine.maxIterations < 100 || input.engine.maxIterations > 100_000) add("recipe", "recipe_v4.cbsem.iterations_invalid", "maxIterations", "Maximum iterations must be between 100 and 100,000.", "Enter a supported iteration limit.");
  if (!Number.isInteger(input.engine.seed) || input.engine.seed < 0 || input.engine.seed > 4_294_967_295) add("recipe", "recipe_v4.cbsem.seed_invalid", "seed", "Seed must be an unsigned 32-bit integer.", "Enter a seed from 0 through 4,294,967,295.");
  if (!Number.isInteger(input.engine.workers) || input.engine.workers < 1 || input.engine.workers > 64) add("recipe", "recipe_v4.cbsem.workers_invalid", "workers", "Worker count must be between 1 and 64.", "Choose a supported worker count.");
  if (!Number.isFinite(input.engine.confidenceLevel) || input.engine.confidenceLevel < 0.8 || input.engine.confidenceLevel > 0.999) add("recipe", "recipe_v4.cbsem.confidence_invalid", "confidenceLevel", "Confidence level must be between 0.8 and 0.999.", "Choose a supported confidence level.");
  if (!Number.isInteger(bootstrapSamples) || (bootstrapSamples !== 0 && (bootstrapSamples < 500 || bootstrapSamples > 10_000))) add("recipe", "recipe_v4.cbsem.bootstrap_samples_invalid", "bootstrapSamples", "Exact CFA bootstrap samples must be 0 or an integer from 500 through 10,000.", "Choose 0 to disable bootstrap, or 500 through 10,000 indexed case refits.");
  if (!["percentile_type7", "analytic_studentized_type7", "bca_type7"].includes(bootstrapInterval)) add("recipe", "recipe_v4.cbsem.bootstrap_interval_invalid", "bootstrapInterval", "Choose percentile Type-7, analytic studentized Type-7, or BCa Type-7.", "Choose one supported exact-CFA interval.");
  if (!["two_sided", "one_sided_greater", "one_sided_less"].includes(bootstrapTestTail)) add("recipe", "recipe_v4.cbsem.bootstrap_tail_invalid", "bootstrapTestTail", "Choose a supported exact-CFA zero-null test tail.", "Choose two-sided, one-sided greater, or one-sided less.");
  if (bootstrapEnabled) {
    if (input.engine.confidenceLevel !== 0.95) add("recipe", "recipe_v4.cbsem.bootstrap_confidence_fixed", "confidenceLevel", "Exact CFA bootstrap intervals use the fixed 95% confidence level.", "Set confidence to 0.95.");
    if (input.missingDataPolicy !== "listwise_deletion") add("recipe", "recipe_v4.cbsem.bootstrap_listwise_required", "missingDataPolicy", "Exact CFA bootstrap requires one fixed listwise-complete sampling frame.", "Choose listwise deletion.");
    if (input.model?.data_binding.kind !== "raw") add("input", "recipe_v4.cbsem.bootstrap_raw_required", "dataset", "Exact CFA bootstrap requires raw case-level data.", "Choose a resident raw dataset.");
    if (input.model?.relations.some((relation) => relation.kind === "structural")) add("model", "recipe_v4.cbsem.bootstrap_cfa_required", "structural_model", "The promoted exact bootstrap family is confined to confirmatory factor analysis.", "Remove structural paths or run point-only CB-SEM.");
    if (boundedInterval && bootstrapTestTail !== "two_sided") add("recipe", "recipe_v4.cbsem.bootstrap_two_sided_required", "bootstrapTestTail", "Analytic studentized and BCa intervals use the fixed two-sided exact-CFA contract.", "Choose the two-sided test tail.");
    if (boundedInterval && input.engine.workers > 12) add("recipe", "recipe_v4.cbsem.bootstrap_workers_bounded", "workers", "Analytic studentized and BCa exact-CFA bootstrap support at most 12 workers.", "Choose 1 through 12 workers.");
    const modeledVariables = input.model ? observedVariables(input.model).length : 0;
    if (boundedInterval && modeledVariables > 9) add("model", "recipe_v4.cbsem.bootstrap_variables_bounded", "model", "Analytic studentized and BCa exact-CFA bootstrap support at most 9 modeled observed variables.", "Use at most 9 observed variables.");
    const residentRows = input.dataset?.rowCount ?? input.dataset?.rows.length ?? 0;
    if (boundedInterval && residentRows > 180) add("input", "recipe_v4.cbsem.bootstrap_cases_bounded", "dataset", "Analytic studentized and BCa exact-CFA bootstrap support at most 180 resident cases before listwise filtering.", "Use a dataset with at most 180 rows; the runner also verifies the complete-case and compiled parameter limits.");
  }

  const layers = STAGES.map(({ stage, label }) => {
    const stageIssues = issues.filter((diagnostic) => diagnostic.stage === stage);
    const stageWarnings = warnings.filter((diagnostic) => diagnostic.stage === stage);
    return { stage, label, status: stageIssues.length ? "blocked" as const : "ready" as const, issues: stageIssues, warnings: stageWarnings };
  });
  return { ready: issues.length === 0, layers, issues, warnings };
}

export interface BuildInternalLabsRecipeV4CbsemRequestV1 {
  recipeId: string;
  createdAt: string;
  dataset: Dataset;
  model: SemModelV4;
  nativeScientificSha256: string;
  engine: InternalRecipeV4CbsemEngineOptionsV1;
}

/** Builds only the typed request; native code remains the compilation authority. */
export async function buildInternalLabsRecipeV4CbsemRequestV1(
  input: BuildInternalLabsRecipeV4CbsemRequestV1,
): Promise<InternalLabsRecipeV4CbsemExecutionRequestV1> {
  const residentKind = datasetKind(input.dataset);
  if (input.model.data_binding.dataset_id !== input.dataset.id || input.model.data_binding.kind !== residentKind) {
    throw new InternalRecipeV4CbsemWorkspaceError(
      "recipe_v4.cbsem.dataset_binding_mismatch",
      input.dataset.id,
      "The SemModelV4 binding does not match the exact resident dataset selected for this recipe.",
      "Re-select the resident dataset and run preflight again.",
    );
  }
  let missingDataPolicy: AnalysisRecipeV4MissingDataPolicy = "listwise_deletion";
  if (input.model.data_binding.kind === "raw") {
    const boundPolicy = input.model.data_binding.missing_data;
    if (boundPolicy !== "listwise_deletion" && boundPolicy !== "mean_replacement") {
      throw new InternalRecipeV4CbsemWorkspaceError(
        "recipe_v4.cbsem.missing_data_policy_unsupported",
        "missing_data",
        "This CB-SEM recipe supports only listwise deletion or mean replacement.",
        "Choose one of the two available missing-data treatments.",
      );
    }
    missingDataPolicy = boundPolicy;
  }
  const weightIssue = cbsemWeightIssueV1(input.model, missingDataPolicy);
  if (weightIssue) throw new InternalRecipeV4CbsemWorkspaceError(
    weightIssue.code,
    weightIssue.declaration?.binding.source_column ?? "weight",
    cbsemWeightIssueMessageV1(weightIssue),
    weightIssue.corrective_action,
  );
  if (!/^[a-f0-9]{64}$/.test(input.nativeScientificSha256)) {
    throw new InternalRecipeV4CbsemWorkspaceError(
      "recipe_v4.cbsem.native_scientific_digest_invalid",
      "model",
      "The native SemModelV4 scientific digest is not an exact lowercase SHA-256 value.",
      "Re-run validation in the installed desktop application before starting this recipe.",
    );
  }
  const settings: AnalysisRecipeV4Settings = {
    method: "cbsem",
    weighting_scheme: "path",
    tolerance: input.engine.tolerance,
    max_iterations: input.engine.maxIterations,
    bootstrap_samples: input.engine.bootstrapSamples ?? 0,
    studentized_inner_samples: 0,
    permutation_samples: 0,
    seed: input.engine.seed,
    workers: input.engine.workers,
    confidence_level: input.engine.confidenceLevel,
    preprocessing: "unstandardized",
    missing_data: missingDataPolicy,
    case_weight_column: null,
  };
  const inputKind = input.model.data_binding.kind;
  const modelType = input.model.relations.some((relation) => relation.kind === "structural") ? "sem" : "cfa";
  const bootstrapSamples = input.engine.bootstrapSamples ?? 0;
  const bootstrapInterval = input.engine.bootstrapInterval ?? "percentile_type7";
  const bootstrapTestTail = input.engine.bootstrapTestTail ?? "two_sided";
  return {
    surface: "standard",
    experimentalLabsEnabled: false,
    residentData: "project_resident",
    datasetId: input.dataset.id,
    datasetFingerprint: input.dataset.fingerprint ?? "",
    recipe: {
      schema_version: 4,
      id: input.recipeId,
      created_at: input.createdAt,
      dataset_fingerprint: input.dataset.fingerprint ?? "",
      model_binding: {
        kind: "embedded_sem_model_v4",
        model: input.model,
        scientific_sha256: input.nativeScientificSha256,
      },
      estimand_confirmation: "not_legacy",
      settings,
      method_config: {
        kind: "cbsem",
        model_type: modelType,
        estimator: "ml",
        input: inputKind,
        mean_structure: false,
        bootstrap_samples: bootstrapSamples,
        ...(bootstrapSamples > 0 ? {
          bootstrap_v2: {
            algorithm: "case_resampling_full_ml",
            interval: bootstrapInterval,
            ...(bootstrapTestTail === "two_sided" ? {} : { test_tail: bootstrapTestTail }),
          },
        } : {}),
      },
      metadata: { execution_surface: bootstrapSamples > 0 ? "native_cbsem_exact_bootstrap_v1" : "native_cbsem_recipe_v4_labs_v1" },
      legacy_source: null,
    },
    model: input.model,
    compilerTarget: "cbsem_plan_v2",
    capabilityCell: bootstrapSamples > 0
      ? INTERNAL_RECIPE_V4_CBSEM_BOOTSTRAP_CAPABILITY_CELL
      : INTERNAL_RECIPE_V4_CBSEM_CAPABILITY_CELL,
  };
}

export interface MonitorInternalLabsRecipeV4CbsemJobV1 {
  initial: InternalRecipeV4CbsemJobSnapshotV1;
  getStatus: (jobId: string) => Promise<InternalRecipeV4CbsemJobSnapshotV1>;
  getResult: (jobId: string) => Promise<InternalRecipeV4CbsemCompletedResultV1>;
  onSnapshot?: (snapshot: InternalRecipeV4CbsemJobSnapshotV1) => void;
  wait?: () => Promise<void>;
  signal?: AbortSignal;
}

export type InternalRecipeV4CbsemMonitorOutcomeV1 =
  | { status: "completed"; snapshot: InternalRecipeV4CbsemJobSnapshotV1; completed: InternalRecipeV4CbsemCompletedResultV1 }
  | { status: "terminal_without_result"; snapshot: InternalRecipeV4CbsemJobSnapshotV1 }
  | { status: "aborted"; snapshot: InternalRecipeV4CbsemJobSnapshotV1 };

const activeJobState = (state: InternalRecipeV4CbsemJobSnapshotV1["state"]) => state === "queued" || state === "running" || state === "cancelling";
const defaultPollWait = () => new Promise<void>((resolve) => globalThis.setTimeout(resolve, 250));

export async function monitorInternalLabsRecipeV4CbsemJobV1(
  input: MonitorInternalLabsRecipeV4CbsemJobV1,
): Promise<InternalRecipeV4CbsemMonitorOutcomeV1> {
  let snapshot = input.initial;
  input.onSnapshot?.(snapshot);
  const wait = input.wait ?? defaultPollWait;
  while (activeJobState(snapshot.state)) {
    if (input.signal?.aborted) return { status: "aborted", snapshot };
    await wait();
    if (input.signal?.aborted) return { status: "aborted", snapshot };
    snapshot = await input.getStatus(snapshot.jobId);
    input.onSnapshot?.(snapshot);
  }
  if (snapshot.state !== "completed") return { status: "terminal_without_result", snapshot };
  if (input.signal?.aborted) return { status: "aborted", snapshot };
  const completed = await input.getResult(snapshot.jobId);
  return { status: "completed", snapshot, completed };
}

export interface InternalSchema6ArchiveIdentityV1 {
  archivePath: string;
  sourceSha256: string;
  projectId: string;
}

export function schema6ArchiveIdentityFromInspectionV1(
  inspection: ProjectUpgradeInspectionV1,
): InternalSchema6ArchiveIdentityV1 {
  if (inspection.access !== "current_v6_archive" || inspection.sourceKind !== "project_archive" || inspection.schemaVersion !== 6 || !inspection.projectId) throw new InternalRecipeV4CbsemWorkspaceError(
    "schema6.cbsem.current_archive_required",
    inspection.sourceArchivePath,
    "The selected file is not a current schema-6 project document.",
    "Create or choose the schema-6 copy for this project.",
  );
  return {
    archivePath: inspection.sourceArchivePath,
    sourceSha256: inspection.sourceArchiveSha256,
    projectId: inspection.projectId,
  };
}

export async function appendInternalLabsRecipeV4CbsemResultV1(
  completed: InternalRecipeV4CbsemCompletedResultV1,
  recipe: AnalysisRecipeV4<AnalysisRecipeV4MissingDataPolicy>,
  archive: InternalSchema6ArchiveIdentityV1,
  append: (request: InternalProjectSchema6ResultAppendRequestV1) => Promise<InternalProjectSchema6ResultAppendOutcomeV1>,
): Promise<InternalProjectSchema6ResultAppendOutcomeV1> {
  if (completed.canonicalDocument.provenance.project_id !== archive.projectId) return {
    status: "blocked",
    diagnostic: {
      code: "schema6.cbsem.project_identity_mismatch",
      message: "The selected schema-6 project does not own this result.",
      correctiveAction: "Choose the schema-6 copy created from the active project.",
    },
  };
  return append({
    surface: "standard_exact_cbsem",
    experimentalLabsEnabled: false,
    archivePath: archive.archivePath,
    expectedSourceSha256: archive.sourceSha256,
    recipe,
    canonicalDocument: completed.canonicalDocument,
  });
}

export interface ReopenInternalLabsRecipeV4CbsemResultV1 {
  outcome: InternalProjectSchema6ResultReadOutcomeV1;
  entry: InternalProjectSchema6CanonicalResultEntryV1 | null;
}

export interface ReadStoredInternalLabsRecipeV4CbsemResultsV1 {
  outcome: InternalProjectSchema6ResultReadOutcomeV1;
  entries: InternalProjectSchema6CanonicalResultEntryV1[];
}

export function storedExactCaseBootstrapEntriesV1(
  documents: readonly InternalProjectSchema6CanonicalResultEntryV1[],
): InternalProjectSchema6CanonicalResultEntryV1[] {
  return documents.filter(({ canonicalDocument }) => {
    const capability = canonicalDocument.provenance.capability_cell;
    return capability.registry_schema_version === INTERNAL_RECIPE_V4_CBSEM_BOOTSTRAP_CAPABILITY_CELL.registry_schema_version
      && capability.capability_id === INTERNAL_RECIPE_V4_CBSEM_BOOTSTRAP_CAPABILITY_CELL.capability_id
      && capability.cell_id === INTERNAL_RECIPE_V4_CBSEM_BOOTSTRAP_CAPABILITY_CELL.cell_id
      && capability.capability_version === INTERNAL_RECIPE_V4_CBSEM_BOOTSTRAP_CAPABILITY_CELL.capability_version;
  });
}

export async function readStoredInternalLabsRecipeV4CbsemResultsV1(
  archive: InternalSchema6ArchiveIdentityV1,
  read: (request: InternalProjectSchema6ResultReadRequestV1) => Promise<InternalProjectSchema6ResultReadOutcomeV1>,
): Promise<ReadStoredInternalLabsRecipeV4CbsemResultsV1> {
  const outcome = await read({
    surface: "standard_exact_cbsem",
    experimentalLabsEnabled: false,
    archivePath: archive.archivePath,
    expectedSourceSha256: archive.sourceSha256,
  });
  if (outcome.status === "blocked") return { outcome, entries: [] };
  if (outcome.value.projectId !== archive.projectId) return {
    outcome: {
      status: "blocked",
      diagnostic: {
        code: "schema6.cbsem.project_identity_mismatch",
        message: "The selected schema-6 project identity changed while its results were being read.",
        correctiveAction: "Inspect the intended schema-6 project again before selecting a stored result.",
      },
    },
    entries: [],
  };
  return { outcome, entries: storedExactCaseBootstrapEntriesV1(outcome.value.documents) };
}

export async function reopenInternalLabsRecipeV4CbsemResultV1(
  completed: InternalRecipeV4CbsemCompletedResultV1,
  archive: InternalSchema6ArchiveIdentityV1,
  read: (request: InternalProjectSchema6ResultReadRequestV1) => Promise<InternalProjectSchema6ResultReadOutcomeV1>,
): Promise<ReopenInternalLabsRecipeV4CbsemResultV1> {
  const outcome = await read({
    surface: "standard_exact_cbsem",
    experimentalLabsEnabled: false,
    archivePath: archive.archivePath,
    expectedSourceSha256: archive.sourceSha256,
  });
  if (outcome.status === "blocked") return { outcome, entry: null };
  const entry = outcome.value.documents.find((candidate) => candidate.documentId === completed.canonicalDocument.document_id) ?? null;
  if (entry) return { outcome, entry };
  return {
    outcome: {
      status: "blocked",
      diagnostic: {
        code: "schema6.cbsem.result_not_found",
        message: "The reopened schema-6 project does not contain this result document.",
        correctiveAction: "Append the completed result, then reopen the updated archive.",
      },
    },
    entry: null,
  };
}

export function internalRecipeV4CbsemFailureV1(
  error: unknown,
  fallbackStage: InternalRecipeV4ExecutionFailureV1["stage"] = "integrity",
): InternalRecipeV4ExecutionFailureV1 {
  if (error && typeof error === "object") {
    const value = error as Partial<InternalRecipeV4ExecutionFailureV1>;
    if (value.schemaVersion === 1 && typeof value.stage === "string" && typeof value.subject === "string" && typeof value.code === "string" && typeof value.message === "string" && typeof value.correctiveAction === "string") return value as InternalRecipeV4ExecutionFailureV1;
  }
  if (error instanceof InternalRecipeV4CbsemWorkspaceError) return {
    schemaVersion: 1,
    stage: fallbackStage,
    subject: error.subject,
    code: error.code,
    message: error.message,
    correctiveAction: error.correctiveAction,
  };
  return {
    schemaVersion: 1,
    stage: fallbackStage,
    subject: "recipe_v4_cbsem",
    code: "recipe_v4.cbsem.workspace_failed",
    message: error instanceof Error && error.message.trim() ? error.message : "The CB-SEM job could not continue.",
    correctiveAction: "Review the preflight details and try again.",
  };
}
