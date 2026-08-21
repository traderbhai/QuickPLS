import type {
  CapabilityCellReferenceV2,
  CanonicalResultDocumentV2,
} from "./canonicalResultDocumentV2";
import type { GeneralSemConfigV1 } from "./generalSemConfigV1";
import { validateCanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import { validateArchivedPlsScoreExecutionV2 } from "./internalProjectSchema6ResultRead";
import { compareUtf8StringsV1, type SemModelV4 } from "./semModelV4";
import type {
  AnalysisEngineSettingsSnapshot,
  NativeAnalysisMethodConfig,
  PlsResolvedScoreExecutionV2,
  PlsResolvedScoreWeightV2,
  PlsResult,
} from "../types";

export const INTERNAL_RECIPE_V4_PLS_COMMAND_SCHEMA_VERSION = 1 as const;
export interface InternalRecipeV4PlsCapabilityCellV1 extends CapabilityCellReferenceV2 {
  registry_schema_version: 2;
  capability_id: "smartpls.pls_algorithm";
  cell_id: "qpls3.pls.algorithm";
  capability_version: "pls_pm_v1";
}

export const INTERNAL_RECIPE_V4_PLS_CAPABILITY_CELL: InternalRecipeV4PlsCapabilityCellV1 = {
  registry_schema_version: 2,
  capability_id: "smartpls.pls_algorithm",
  cell_id: "qpls3.pls.algorithm",
  capability_version: "pls_pm_v1",
};

export interface InternalRecipeV4PlsNonlinearCapabilityCellV1 extends CapabilityCellReferenceV2 {
  registry_schema_version: 2;
  capability_id: "smartpls.nonlinear_relationships";
  cell_id: "qpls3.pls.nonlinear_quadratic";
  capability_version: "pls_quadratic_nonlinear_effects_v1";
}

export const INTERNAL_RECIPE_V4_PLS_NONLINEAR_CAPABILITY_CELL: InternalRecipeV4PlsNonlinearCapabilityCellV1 = {
  registry_schema_version: 2,
  capability_id: "smartpls.nonlinear_relationships",
  cell_id: "qpls3.pls.nonlinear_quadratic",
  capability_version: "pls_quadratic_nonlinear_effects_v1",
};

export type InternalRecipeV4PlsExecutionCapabilityCellV1 =
  | InternalRecipeV4PlsCapabilityCellV1
  | InternalRecipeV4PlsNonlinearCapabilityCellV1;

export interface PlsPosthocTechnicalMinimumSampleSizeCapabilityCellV2 extends CapabilityCellReferenceV2 {
  registry_schema_version: 2;
  capability_id: "smartpls.pls_power_analysis";
  cell_id: "qpls3.pls.posthoc_technical_minimum_sample_size";
  capability_version: "pls_posthoc_technical_minimum_sample_size_v2";
}

interface PlsPosthocTechnicalMinimumSampleSizeConfigIdentityV2 {
  capability_cell: PlsPosthocTechnicalMinimumSampleSizeCapabilityCellV2;
  method_version: "inverse_square_root_posthoc_v2";
}

export type PlsPosthocTechnicalMinimumSampleSizeConfigV2 =
  PlsPosthocTechnicalMinimumSampleSizeConfigIdentityV2 & (
    | { base_analysis: "pls_algorithm"; inference: "point_estimate_only" }
    | { base_analysis: "pls_bootstrap"; inference: "case_bootstrap_normal_reference_two_sided" }
  );

export const INTERNAL_RECIPE_V4_PLS_POSTHOC_TECHNICAL_MINIMUM_SAMPLE_SIZE_V2 = {
  capability_cell: {
    registry_schema_version: 2,
    capability_id: "smartpls.pls_power_analysis",
    cell_id: "qpls3.pls.posthoc_technical_minimum_sample_size",
    capability_version: "pls_posthoc_technical_minimum_sample_size_v2",
  },
  method_version: "inverse_square_root_posthoc_v2",
  base_analysis: "pls_algorithm",
  inference: "point_estimate_only",
} as const satisfies PlsPosthocTechnicalMinimumSampleSizeConfigV2;

export type LegacyEstimandConfirmationV4 =
  | "not_legacy"
  | "legacy_estimand_unspecified"
  | "confirmed_composite"
  | "confirmed_common_factor";

export type AnalysisRecipeModelBindingV4 =
  | {
      kind: "embedded_sem_model_v4";
      model: SemModelV4;
      scientific_sha256: string;
    }
  | {
      kind: "project_sem_model_v4_reference";
      model_id: string;
      scientific_sha256: string;
    }
  | {
      kind: "legacy_estimand_unspecified";
      legacy_model_id: string;
      legacy_model_sha256: string;
    };

/**
 * Recipe-v4 can name mean replacement only for the bounded Internal CB-SEM
 * Labs path. Standard analysis settings remain listwise-only in `types.ts`.
 */
export type AnalysisRecipeV4MissingDataPolicy =
  | "listwise_deletion"
  | "mean_replacement";

export type AnalysisRecipeV4Settings<
  TMissingData extends AnalysisRecipeV4MissingDataPolicy = AnalysisRecipeV4MissingDataPolicy,
> = Omit<
  AnalysisEngineSettingsSnapshot,
  "missing_data"
> & {
  missing_data: TMissingData;
};

export interface AnalysisRecipeV4<
  TMissingData extends AnalysisRecipeV4MissingDataPolicy = "listwise_deletion",
> {
  schema_version: 4;
  id: string;
  created_at: string;
  dataset_fingerprint: string;
  model_binding: AnalysisRecipeModelBindingV4;
  estimand_confirmation: LegacyEstimandConfirmationV4;
  settings: AnalysisRecipeV4Settings<TMissingData>;
  method_config?: NativeAnalysisMethodConfig | null;
  general_sem_config?: GeneralSemConfigV1 | null;
  metadata: Record<string, string>;
  legacy_source?: {
    source_schema_version: 1 | 2 | 3;
    source_recipe_sha256: string;
  } | null;
}

/** PLS keeps its existing fail-closed listwise-only Recipe-v4 surface. */
export type InternalLabsPlsAnalysisRecipeV4 = AnalysisRecipeV4<"listwise_deletion">;

/**
 * Deliberately internal/Labs-only request. `residentData` means raw rows stay
 * inside the active native project and only their exact identity crosses IPC.
 */
export interface InternalLabsRecipeV4PlsExecutionRequestV1 {
  surface: "internal_labs";
  experimentalLabsEnabled: true;
  residentData: "project_resident";
  datasetId: string;
  datasetFingerprint: string;
  recipe: InternalLabsPlsAnalysisRecipeV4;
  model: SemModelV4;
  compilerTarget: "pls_plan_v2";
  capabilityCell: InternalRecipeV4PlsExecutionCapabilityCellV1;
  /** Explicit scoped Standard add-on. Absence means no post-hoc payload. */
  posthocTechnicalMinimumSampleSize?: PlsPosthocTechnicalMinimumSampleSizeConfigV2;
}

export interface RecipeV4CompilationReceiptV1 {
  schema_version: 1;
  recipe_id: string;
  recipe_document_sha256: string;
  recipe_analytical_sha256: string;
  model_id: string;
  model_document_sha256: string;
  model_scientific_sha256: string;
  dataset_fingerprint: string;
  compiler_target: "pls_plan_v2";
  compiler_version: string;
  capability_cell: InternalRecipeV4PlsExecutionCapabilityCellV1;
  plan_sha256: string;
  analytical_identity_sha256: string;
}

export interface InternalRecipeV4PlsExecutionProvenanceV1 {
  adapter_version: string;
  compilation_receipt: RecipeV4CompilationReceiptV1;
  projected_recipe_schema_version: 3;
  projected_recipe_sha256: string;
  projected_initialization_sha256?: string;
  dataset_id: string;
  estimator_method_version: string;
  posthoc_technical_minimum_sample_size?: PlsPosthocTechnicalMinimumSampleSizeConfigV2;
}

export interface InternalRecipeV4PlsExecutionResultV1 {
  schema_version: typeof INTERNAL_RECIPE_V4_PLS_COMMAND_SCHEMA_VERSION;
  provenance: InternalRecipeV4PlsExecutionProvenanceV1;
  estimation: PlsResult;
}

/**
 * A terminal job payload. The canonical document is produced and validated by
 * the native worker from the same immutable analytical result before either is
 * exposed, so callers never reconstruct scientific tables in the UI.
 */
export interface InternalRecipeV4CompletedResultV1 {
  schemaVersion: typeof INTERNAL_RECIPE_V4_PLS_COMMAND_SCHEMA_VERSION;
  analyticalResult: InternalRecipeV4PlsExecutionResultV1;
  canonicalDocument: CanonicalResultDocumentV2;
}

export type InternalRecipeV4ExecutionStageV1 =
  | "access"
  | "capability"
  | "data_resolution"
  | "compilation"
  | "projection"
  | "estimation"
  | "integrity";

/** Serialized Tauri rejection payload for internal tooling and Labs callers. */
export interface InternalRecipeV4ExecutionFailureV1 {
  schemaVersion: 1;
  stage: InternalRecipeV4ExecutionStageV1;
  subject: string;
  code: string;
  message: string;
  correctiveAction: string;
  issues?: Array<{
    code: string;
    subject: string;
    message: string;
  }>;
}

export type InternalRecipeV4PlsJobStateV1 =
  | "queued"
  | "running"
  | "cancelling"
  | "completed"
  | "failed"
  | "cancelled";

/** Internal-only lifecycle. A scientific payload exists only in `completed`. */
export interface InternalRecipeV4PlsJobSnapshotV1 {
  schemaVersion: 1;
  jobId: string;
  state: InternalRecipeV4PlsJobStateV1;
  phase: string;
  completedUnits: number;
  totalUnits: number;
  message: string | null;
  failure: InternalRecipeV4ExecutionFailureV1 | null;
  queuedAt: string;
  startedAt: string | null;
  completedAt: string | null;
}

const SCORE_EXECUTION_CONTRACT_VERSION_V2 = "pls_score_execution_v2" as const;
export const PLS_NONLINEAR_METHOD_VERSION_V1 = "pls_quadratic_nonlinear_effects_v1" as const;
export const PLS_NONLINEAR_TERM_V1 = "centered_squared_construct_score_v1" as const;
export const PLS_NONLINEAR_ENGINE_WARNING_V1 = "Nonlinear effects are validated for the documented QuickPLS v1.2.3 fixed-score quadratic diagnostic scope; diagnostics use fixed PLS construct scores and centered squared score terms." as const;
const LEGACY_PLS_ADAPTER_VERSION = "compiled_recipe_v4_pls_plan_v2_execution_v3";
const LEGACY_SCORE_EXECUTION_ADAPTER_VERSION_V2 = "compiled_recipe_v4_pls_plan_v2_execution_v4";
const PLS_ADAPTER_VERSION = "compiled_recipe_v4_pls_plan_v2_execution_v5";
const SCORE_EXECUTION_ADAPTER_VERSION_V2 = "compiled_recipe_v4_pls_plan_v2_execution_v6";
export const PLS_NONLINEAR_ADAPTER_VERSION_V7 = "compiled_recipe_v4_pls_plan_v2_execution_v7" as const;
const SHA256_HEX = /^[a-f0-9]{64}$/;
const VERSIONED_DATASET_SHA256 = /^v2:([a-f0-9]{64})$/;

function scoreContractFail(path: string, message: string): never {
  throw new Error(`Recipe-v4 PLS ${path}: ${message}`);
}

function scoreRecord(value: unknown, path: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    scoreContractFail(path, "must be an object");
  }
  return value as Record<string, unknown>;
}

function scoreExactRecord(
  value: unknown,
  required: readonly string[],
  optional: readonly string[],
  path: string,
): Record<string, unknown> {
  const record = scoreRecord(value, path);
  const allowed = new Set([...required, ...optional]);
  const unknown = Object.keys(record).filter((key) => !allowed.has(key));
  const missing = required.filter((key) => !Object.prototype.hasOwnProperty.call(record, key));
  if (unknown.length > 0 || missing.length > 0) {
    scoreContractFail(
      path,
      `has a drifted key contract${unknown.length ? `; unknown ${unknown.join(", ")}` : ""}${missing.length ? `; missing ${missing.join(", ")}` : ""}`,
    );
  }
  return record;
}

function scoreText(value: unknown, path: string): string {
  if (typeof value !== "string" || !value.trim()) {
    scoreContractFail(path, "must be a nonempty string");
  }
  return value;
}

function scoreNumber(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    scoreContractFail(path, "must be a finite number");
  }
  return value;
}

function scoreCount(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    scoreContractFail(path, "must be a nonnegative safe integer");
  }
  return value as number;
}

function recordedDatasetSha256(value: unknown, path: string): string {
  const fingerprint = scoreText(value, path);
  if (SHA256_HEX.test(fingerprint)) return fingerprint;
  const versioned = VERSIONED_DATASET_SHA256.exec(fingerprint);
  if (versioned) return versioned[1];
  scoreContractFail(path, "must be a lowercase SHA-256 or v2-prefixed lowercase SHA-256");
}

function scoreWeights(
  value: unknown,
  indicatorIds: readonly string[],
  path: string,
): PlsResolvedScoreWeightV2[] {
  if (!Array.isArray(value) || value.length !== indicatorIds.length) {
    scoreContractFail(path, "must exactly cover the block indicator identities");
  }
  return value.map((item, index) => {
    const weight = scoreExactRecord(item, ["indicator_id", "value"], [], `${path}[${index}]`);
    const indicatorId = scoreText(weight.indicator_id, `${path}[${index}].indicator_id`);
    if (indicatorId !== indicatorIds[index]) {
      scoreContractFail(`${path}[${index}].indicator_id`, "differs from block indicator order");
    }
    return {
      indicator_id: indicatorId,
      value: scoreNumber(weight.value, `${path}[${index}].value`),
    };
  });
}

function positiveScalarProportional(
  requested: readonly PlsResolvedScoreWeightV2[],
  resolved: readonly PlsResolvedScoreWeightV2[],
): boolean {
  const anchor = requested.findIndex((weight) => weight.value !== 0);
  if (anchor < 0) return false;
  const scale = resolved[anchor].value / requested[anchor].value;
  return Number.isFinite(scale) && scale > 0 && requested.every((weight, index) => {
    const expected = weight.value * scale;
    return Math.abs(resolved[index].value - expected)
      <= 1e-12 * Math.max(Math.abs(resolved[index].value), Math.abs(expected), 1);
  });
}

function resolvedWeightsMatchNormalization(
  normalization: "none" | "sum_to_one" | "unit_variance",
  requested: readonly PlsResolvedScoreWeightV2[],
  resolved: readonly PlsResolvedScoreWeightV2[],
): boolean {
  if (normalization === "none") {
    return requested.every((weight, index) => Object.is(weight.value, resolved[index].value));
  }
  if (normalization === "sum_to_one") {
    const sum = requested.reduce((total, weight) => total + weight.value, 0);
    return Number.isFinite(sum) && sum !== 0 && requested.every((weight, index) => (
      Object.is(weight.value / sum, resolved[index].value)
    ));
  }
  return positiveScalarProportional(requested, resolved);
}

/** Strict parser for the untrusted native `score_execution` wire payload. */
export function parsePlsResolvedScoreExecutionV2(
  input: unknown,
): PlsResolvedScoreExecutionV2 {
  const execution = scoreExactRecord(
    input,
    ["contract_version", "blocks", "iteration_accounting"],
    [],
    "score_execution",
  );
  if (execution.contract_version !== SCORE_EXECUTION_CONTRACT_VERSION_V2) {
    scoreContractFail("score_execution.contract_version", "must equal pls_score_execution_v2");
  }
  if (!Array.isArray(execution.blocks) || execution.blocks.length === 0) {
    scoreContractFail("score_execution.blocks", "must be a nonempty array");
  }
  const constructIds = new Set<string>();
  let estimatedBlocks = 0;
  let fixedBlocks = 0;
  const blocks = execution.blocks.map((item, blockIndex) => {
    const path = `score_execution.blocks[${blockIndex}]`;
    const block = scoreExactRecord(item, ["construct_id", "indicator_ids", "scoring"], [], path);
    const constructId = scoreText(block.construct_id, `${path}.construct_id`);
    if (constructIds.has(constructId)) scoreContractFail(`${path}.construct_id`, "is duplicated");
    constructIds.add(constructId);
    if (!Array.isArray(block.indicator_ids) || block.indicator_ids.length === 0) {
      scoreContractFail(`${path}.indicator_ids`, "must be a nonempty array");
    }
    const indicatorIds = block.indicator_ids.map((value, index) => (
      scoreText(value, `${path}.indicator_ids[${index}]`)
    ));
    if (new Set(indicatorIds).size !== indicatorIds.length) {
      scoreContractFail(`${path}.indicator_ids`, "contains duplicate identities");
    }
    const scoring = scoreRecord(block.scoring, `${path}.scoring`);
    if (scoring.kind === "estimated") {
      estimatedBlocks += 1;
      scoreExactRecord(
        scoring,
        ["kind", "mode", "requested_initialization", "resolved_initial_weights"],
        [],
        `${path}.scoring`,
      );
      if (scoring.mode !== "mode_a" && scoring.mode !== "mode_b") {
        scoreContractFail(`${path}.scoring.mode`, "must equal mode_a or mode_b");
      }
      const requestedInitialization = scoreRecord(
        scoring.requested_initialization,
        `${path}.scoring.requested_initialization`,
      );
      if (requestedInitialization.kind !== "standard" && requestedInitialization.kind !== "individual") {
        scoreContractFail(
          `${path}.scoring.requested_initialization.kind`,
          "must equal standard or individual",
        );
      }
      scoreExactRecord(
        requestedInitialization,
        ["kind", "weights"],
        [],
        `${path}.scoring.requested_initialization`,
      );
      const requested = scoreWeights(
        requestedInitialization.weights,
        indicatorIds,
        `${path}.scoring.requested_initialization.weights`,
      );
      if (
        requestedInitialization.kind === "standard"
        && requested.some((weight) => weight.value !== 1)
      ) {
        scoreContractFail(
          `${path}.scoring.requested_initialization.weights`,
          "standard initialization must request exact +1 weights",
        );
      }
      const resolved = scoreWeights(
        scoring.resolved_initial_weights,
        indicatorIds,
        `${path}.scoring.resolved_initial_weights`,
      );
      if (!positiveScalarProportional(requested, resolved)) {
        scoreContractFail(`${path}.scoring`, "resolved initialization changed block orientation");
      }
    } else if (scoring.kind === "fixed_unit" || scoring.kind === "fixed_custom") {
      fixedBlocks += 1;
      scoreExactRecord(
        scoring,
        ["kind", "normalization", "requested_weights", "resolved_effective_weights"],
        [],
        `${path}.scoring`,
      );
      const normalization = scoreText(
        scoring.normalization,
        `${path}.scoring.normalization`,
      );
      if (!(["none", "sum_to_one", "unit_variance"] as const).includes(
        normalization as "none" | "sum_to_one" | "unit_variance",
      )) {
        scoreContractFail(
          `${path}.scoring.normalization`,
          "must equal none, sum_to_one, or unit_variance",
        );
      }
      const requested = scoreWeights(
        scoring.requested_weights,
        indicatorIds,
        `${path}.scoring.requested_weights`,
      );
      if (scoring.kind === "fixed_unit" && requested.some((weight) => weight.value !== 1)) {
        scoreContractFail(`${path}.scoring.requested_weights`, "fixed_unit must request exact +1 weights");
      }
      const resolved = scoreWeights(
        scoring.resolved_effective_weights,
        indicatorIds,
        `${path}.scoring.resolved_effective_weights`,
      );
      if (!resolvedWeightsMatchNormalization(
        normalization as "none" | "sum_to_one" | "unit_variance",
        requested,
        resolved,
      )) {
        scoreContractFail(
          `${path}.scoring`,
          `resolved fixed weights violate the ${normalization} normalization contract`,
        );
      }
    } else {
      scoreContractFail(`${path}.scoring.kind`, "is unsupported");
    }
    return item;
  });

  const accounting = scoreExactRecord(
    execution.iteration_accounting,
    [
      "maximum_iterations",
      "stop_criterion",
      "estimated_block_count",
      "fixed_block_count",
      "performed_iterations",
      "estimated_block_updates",
    ],
    [],
    "score_execution.iteration_accounting",
  );
  const maximumIterations = scoreCount(
    accounting.maximum_iterations,
    "score_execution.iteration_accounting.maximum_iterations",
  );
  const stopCriterion = scoreNumber(
    accounting.stop_criterion,
    "score_execution.iteration_accounting.stop_criterion",
  );
  const estimatedBlockCount = scoreCount(
    accounting.estimated_block_count,
    "score_execution.iteration_accounting.estimated_block_count",
  );
  const fixedBlockCount = scoreCount(
    accounting.fixed_block_count,
    "score_execution.iteration_accounting.fixed_block_count",
  );
  const performedIterations = scoreCount(
    accounting.performed_iterations,
    "score_execution.iteration_accounting.performed_iterations",
  );
  const estimatedBlockUpdates = scoreCount(
    accounting.estimated_block_updates,
    "score_execution.iteration_accounting.estimated_block_updates",
  );
  if (
    maximumIterations !== 3_000
    || stopCriterion !== 1e-7
    || estimatedBlockCount !== estimatedBlocks
    || fixedBlockCount !== fixedBlocks
    || estimatedBlockUpdates !== performedIterations * estimatedBlockCount
    || (estimatedBlockCount === 0 && performedIterations !== 0)
    || (estimatedBlockCount > 0 && (performedIterations < 1 || performedIterations > 3_000))
  ) {
    scoreContractFail("score_execution.iteration_accounting", "differs from the exact v2 contract");
  }
  return {
    contract_version: SCORE_EXECUTION_CONTRACT_VERSION_V2,
    blocks: blocks as PlsResolvedScoreExecutionV2["blocks"],
    iteration_accounting: {
      maximum_iterations: maximumIterations,
      stop_criterion: stopCriterion,
      estimated_block_count: estimatedBlockCount,
      fixed_block_count: fixedBlockCount,
      performed_iterations: performedIterations,
      estimated_block_updates: estimatedBlockUpdates,
    },
  };
}

function validatePlsResultScoreExecution(
  resultValue: unknown,
): PlsResolvedScoreExecutionV2 | null {
  const result = scoreRecord(resultValue, "estimation");
  const methodVersion = scoreText(result.method_version, "estimation.method_version");
  if (methodVersion === "pls_pm_v1" || methodVersion === PLS_NONLINEAR_METHOD_VERSION_V1) {
    if (Object.prototype.hasOwnProperty.call(result, "score_execution")) {
      scoreContractFail(
        "estimation.score_execution",
        methodVersion === "pls_pm_v1"
          ? "must be omitted for legacy pls_pm_v1"
          : `must be omitted for ${methodVersion}`,
      );
    }
    return null;
  }
  if (methodVersion !== SCORE_EXECUTION_CONTRACT_VERSION_V2) {
    scoreContractFail("estimation.method_version", `is unsupported: ${methodVersion}`);
  }
  const execution = parsePlsResolvedScoreExecutionV2(result.score_execution);
  const iterations = scoreCount(result.iterations, "estimation.iterations");
  if (iterations !== execution.iteration_accounting.performed_iterations) {
    scoreContractFail("estimation.iterations", "differs from score-execution accounting");
  }
  return execution;
}

function capabilityCellMatches(
  value: unknown,
  expected: CapabilityCellReferenceV2,
  path: string,
): boolean {
  const cell = scoreExactRecord(
    value,
    ["registry_schema_version", "capability_id", "cell_id", "capability_version"],
    [],
    path,
  );
  return cell.registry_schema_version === expected.registry_schema_version
    && cell.capability_id === expected.capability_id
    && cell.cell_id === expected.cell_id
    && cell.capability_version === expected.capability_version;
}

function parseNonlinearCompilationReceipt(
  value: unknown,
): RecipeV4CompilationReceiptV1 {
  const path = "result.provenance.compilation_receipt";
  const receipt = scoreExactRecord(
    value,
    [
      "schema_version", "recipe_id", "recipe_document_sha256", "recipe_analytical_sha256",
      "model_id", "model_document_sha256", "model_scientific_sha256", "dataset_fingerprint",
      "compiler_target", "compiler_version", "capability_cell", "plan_sha256",
      "analytical_identity_sha256",
    ],
    [],
    path,
  );
  if (receipt.schema_version !== 1 || receipt.compiler_target !== "pls_plan_v2") {
    scoreContractFail(path, "has an unsupported schema or compiler target");
  }
  for (const key of ["recipe_id", "model_id", "dataset_fingerprint", "compiler_version"] as const) {
    scoreText(receipt[key], `${path}.${key}`);
  }
  recordedDatasetSha256(receipt.dataset_fingerprint, `${path}.dataset_fingerprint`);
  for (const key of [
    "recipe_document_sha256", "recipe_analytical_sha256", "model_document_sha256",
    "model_scientific_sha256", "plan_sha256", "analytical_identity_sha256",
  ] as const) {
    if (!SHA256_HEX.test(scoreText(receipt[key], `${path}.${key}`))) {
      scoreContractFail(`${path}.${key}`, "must be a lowercase SHA-256");
    }
  }
  if (!capabilityCellMatches(
    receipt.capability_cell,
    INTERNAL_RECIPE_V4_PLS_NONLINEAR_CAPABILITY_CELL,
    `${path}.capability_cell`,
  )) scoreContractFail(`${path}.capability_cell`, "must equal the primary nonlinear capability cell");
  return value as RecipeV4CompilationReceiptV1;
}

function validateNonlinearEffectsResult(
  estimation: Record<string, unknown>,
  nonlinear: boolean,
): void {
  const path = "result.estimation.nonlinear_effects";
  const raw = estimation.nonlinear_effects;
  if (!nonlinear) {
    if (raw !== undefined && raw !== null) {
      scoreContractFail(path, "must be absent outside the v7 nonlinear adapter");
    }
    return;
  }
  const analysis = scoreExactRecord(raw, ["method_version", "term", "estimates", "warnings"], [], path);
  if (
    analysis.method_version !== PLS_NONLINEAR_METHOD_VERSION_V1
    || analysis.term !== PLS_NONLINEAR_TERM_V1
    || !Array.isArray(analysis.warnings)
    || analysis.warnings.length !== 1
    || analysis.warnings[0] !== PLS_NONLINEAR_ENGINE_WARNING_V1
    || !Array.isArray(analysis.estimates)
    || analysis.estimates.length === 0
  ) scoreContractFail(path, "has a drifted method, term, warning, or empty estimate family");

  if (!Array.isArray(estimation.paths) || estimation.paths.length === 0) {
    scoreContractFail("result.estimation.paths", "must be a nonempty structural path family");
  }
  const pathCoefficients = new Map<string, number>();
  estimation.paths.forEach((rawPath, index) => {
    const candidate = scoreRecord(rawPath, `result.estimation.paths[${index}]`);
    const source = scoreText(candidate.source, `result.estimation.paths[${index}].source`);
    const target = scoreText(candidate.target, `result.estimation.paths[${index}].target`);
    const coefficient = scoreNumber(candidate.coefficient, `result.estimation.paths[${index}].coefficient`);
    const identity = `${target}\u0000${source}`;
    if (pathCoefficients.has(identity)) {
      scoreContractFail("result.estimation.paths", "contains duplicate endpoints");
    }
    pathCoefficients.set(identity, coefficient);
  });

  const identities = new Set<string>();
  const equationFits = new Map<string, readonly [number, number, number]>();
  analysis.estimates.forEach((rawEstimate, index) => {
    const estimatePath = `${path}.estimates[${index}]`;
    const estimate = scoreExactRecord(
      rawEstimate,
      [
        "source", "target", "linear_coefficient", "quadratic_coefficient",
        "standard_error", "t_statistic", "p_value_two_sided", "linear_r_squared",
        "augmented_r_squared", "delta_r_squared", "warning",
      ],
      [],
      estimatePath,
    );
    const source = scoreText(estimate.source, `${estimatePath}.source`);
    const target = scoreText(estimate.target, `${estimatePath}.target`);
    const linear = scoreNumber(estimate.linear_coefficient, `${estimatePath}.linear_coefficient`);
    const quadratic = scoreNumber(estimate.quadratic_coefficient, `${estimatePath}.quadratic_coefficient`);
    const standardError = scoreNumber(estimate.standard_error, `${estimatePath}.standard_error`);
    const tStatistic = scoreNumber(estimate.t_statistic, `${estimatePath}.t_statistic`);
    const pValue = scoreNumber(estimate.p_value_two_sided, `${estimatePath}.p_value_two_sided`);
    const linearR2 = scoreNumber(estimate.linear_r_squared, `${estimatePath}.linear_r_squared`);
    const augmentedR2 = scoreNumber(estimate.augmented_r_squared, `${estimatePath}.augmented_r_squared`);
    const deltaR2 = scoreNumber(estimate.delta_r_squared, `${estimatePath}.delta_r_squared`);
    if (estimate.warning !== null) scoreText(estimate.warning, `${estimatePath}.warning`);
    const identity = `${target}\u0000${source}`;
    const priorEquation = equationFits.get(target);
    if (
      identities.has(identity)
      || !pathCoefficients.has(identity)
      || !Object.is(pathCoefficients.get(identity), linear)
      || standardError <= 0
      || !Object.is(tStatistic, quadratic / standardError)
      || pValue < 0 || pValue > 1
      || linearR2 < 0 || linearR2 > 1
      || augmentedR2 < 0 || augmentedR2 > 1
      || !Object.is(deltaR2, Math.max(augmentedR2 - linearR2, 0))
      || (priorEquation && (
        !Object.is(priorEquation[0], linearR2)
        || !Object.is(priorEquation[1], augmentedR2)
        || !Object.is(priorEquation[2], deltaR2)
      ))
    ) scoreContractFail(estimatePath, "has drifted numerical or structural invariants");
    identities.add(identity);
    equationFits.set(target, [linearR2, augmentedR2, deltaR2]);
  });
  if (identities.size !== pathCoefficients.size) {
    scoreContractFail(`${path}.estimates`, "must exactly cover structural path endpoints");
  }
}

function validateFixedScoreScaleReceipt(
  estimation: Record<string, unknown>,
  execution: PlsResolvedScoreExecutionV2 | null,
  currentAdapter: boolean,
): void {
  const fixedBlocks = execution?.blocks.filter((block) => block.scoring.kind !== "estimated") ?? [];
  const present = Object.prototype.hasOwnProperty.call(estimation, "fixed_score_scale_receipt");
  if (fixedBlocks.length === 0) {
    if (present) {
      scoreContractFail(
        "result.estimation.fixed_score_scale_receipt",
        "must be omitted when no fixed score blocks were executed",
      );
    }
    return;
  }
  if (!present) {
    if (currentAdapter) {
      scoreContractFail(
        "result.estimation.fixed_score_scale_receipt",
        "is required for a current fixed-score adapter",
      );
    }
    return;
  }
  const receipt = scoreExactRecord(
    estimation.fixed_score_scale_receipt,
    ["contract_version", "blocks"],
    [],
    "result.estimation.fixed_score_scale_receipt",
  );
  if (receipt.contract_version !== "pls_fixed_score_scale_receipt_v1") {
    scoreContractFail(
      "result.estimation.fixed_score_scale_receipt.contract_version",
      "must equal pls_fixed_score_scale_receipt_v1",
    );
  }
  if (!Array.isArray(receipt.blocks) || receipt.blocks.length !== fixedBlocks.length) {
    scoreContractFail(
      "result.estimation.fixed_score_scale_receipt.blocks",
      "must exactly cover fixed blocks in score-execution order",
    );
  }
  receipt.blocks.forEach((value, blockIndex) => {
    const path = `result.estimation.fixed_score_scale_receipt.blocks[${blockIndex}]`;
    const block = scoreExactRecord(
      value,
      [
        "construct_id",
        "indicator_ids",
        "pre_standardization_center",
        "pre_standardization_scale",
        "effective_unit_score_weights",
      ],
      [],
      path,
    );
    const expected = fixedBlocks[blockIndex];
    if (scoreText(block.construct_id, `${path}.construct_id`) !== expected.construct_id) {
      scoreContractFail(`${path}.construct_id`, "differs from fixed score-execution order");
    }
    if (
      !Array.isArray(block.indicator_ids)
      || block.indicator_ids.length !== expected.indicator_ids.length
      || block.indicator_ids.some((id, index) => (
        scoreText(id, `${path}.indicator_ids[${index}]`) !== expected.indicator_ids[index]
      ))
    ) {
      scoreContractFail(`${path}.indicator_ids`, "differs from fixed indicator order");
    }
    scoreNumber(block.pre_standardization_center, `${path}.pre_standardization_center`);
    const scale = scoreNumber(
      block.pre_standardization_scale,
      `${path}.pre_standardization_scale`,
    );
    if (scale <= Number.EPSILON) {
      scoreContractFail(`${path}.pre_standardization_scale`, "must be finite and positive");
    }
    const effective = scoreWeights(
      block.effective_unit_score_weights,
      expected.indicator_ids,
      `${path}.effective_unit_score_weights`,
    );
    if (expected.scoring.kind === "estimated") {
      scoreContractFail(path, "references an estimated block");
    }
    const resolved = expected.scoring.resolved_effective_weights;
    if (effective.some((weight, index) => (
      !Object.is(weight.value, resolved[index].value / scale)
    ))) {
      scoreContractFail(path, "effective weights differ from resolved coefficient / scale");
    }
  });
}

function validatePointEstimateAttribution(
  estimation: Record<string, unknown>,
  currentAdapter: boolean,
): void {
  const path = "result.estimation.point_estimate_attribution";
  const present = Object.prototype.hasOwnProperty.call(estimation, "point_estimate_attribution");
  if (!present) {
    if (currentAdapter) scoreContractFail(path, "is required for a current PLS adapter");
    return;
  }
  const value = scoreExactRecord(
    estimation.point_estimate_attribution,
    [
      "contract_version",
      "preprocessing",
      "indicator_centering",
      "indicator_scaling",
      "outer_weights",
      "outer_loadings",
      "construct_scores",
      "structural_paths",
      "effects",
    ],
    [],
    path,
  );
  const preprocessing = scoreText(value.preprocessing, `${path}.preprocessing`);
  const preprocessingTokens = {
    standardized: ["sample_mean", "sample_standard_deviation"],
    mean_centered: ["sample_mean", "unit_scale"],
    unstandardized: ["no_centering", "unit_scale"],
  } as const;
  const expected = preprocessingTokens[preprocessing as keyof typeof preprocessingTokens];
  if (
    value.contract_version !== "pls_point_estimate_attribution_v1"
    || !expected
    || scoreText(value.indicator_centering, `${path}.indicator_centering`) !== expected[0]
    || scoreText(value.indicator_scaling, `${path}.indicator_scaling`) !== expected[1]
    || scoreText(value.outer_weights, `${path}.outer_weights`)
      !== "preprocessed_indicator_to_unit_variance_construct_score"
    || scoreText(value.outer_loadings, `${path}.outer_loadings`)
      !== "indicator_construct_score_correlation"
    || scoreText(value.construct_scores, `${path}.construct_scores`)
      !== "zero_mean_unit_variance_construct_score"
    || scoreText(value.structural_paths, `${path}.structural_paths`)
      !== "standardized_construct_score_regression"
    || scoreText(value.effects, `${path}.effects`)
      !== "standardized_structural_path_decomposition"
  ) scoreContractFail(path, "differs from the exact preprocessing and point-estimate scale contract");
}

function validateAlgorithmConvergenceReceipt(
  estimation: Record<string, unknown>,
  execution: PlsResolvedScoreExecutionV2 | null,
  currentAdapter: boolean,
): void {
  const path = "result.estimation.algorithm_convergence_receipt";
  const present = Object.prototype.hasOwnProperty.call(estimation, "algorithm_convergence_receipt");
  if (!present) {
    if (currentAdapter) scoreContractFail(path, "is required for a current PLS adapter");
    return;
  }
  const receipt = scoreExactRecord(
    estimation.algorithm_convergence_receipt,
    [
      "contract_version",
      "weighting_scheme",
      "maximum_iterations",
      "stop_criterion",
      "comparison",
      "performed_iterations",
      "estimated_block_updates",
      "termination_reason",
      "blocks",
    ],
    ["final_max_outer_weight_change"],
    path,
  );
  const weighting = scoreText(receipt.weighting_scheme, `${path}.weighting_scheme`);
  const maximum = scoreCount(receipt.maximum_iterations, `${path}.maximum_iterations`);
  const criterion = scoreNumber(receipt.stop_criterion, `${path}.stop_criterion`);
  const performed = scoreCount(receipt.performed_iterations, `${path}.performed_iterations`);
  const updates = scoreCount(receipt.estimated_block_updates, `${path}.estimated_block_updates`);
  const iterations = scoreCount(estimation.iterations, "result.estimation.iterations");
  if (
    receipt.contract_version !== "pls_algorithm_convergence_receipt_v1"
    || (weighting !== "path" && weighting !== "factor")
    || maximum !== 3_000
    || criterion !== 1e-7
    || receipt.comparison !== "less_than_or_equal"
    || performed !== iterations
    || !Array.isArray(receipt.blocks)
  ) scoreContractFail(path, "has drifted settings, accounting, or block shape");

  const blocks = receipt.blocks.map((blockValue, blockIndex) => {
    const blockPath = `${path}.blocks[${blockIndex}]`;
    const block = scoreExactRecord(
      blockValue,
      ["construct_id", "indicator_order", "update_rule", "initialization"],
      [],
      blockPath,
    );
    const constructId = scoreText(block.construct_id, `${blockPath}.construct_id`);
    if (!Array.isArray(block.indicator_order) || block.indicator_order.length === 0) {
      scoreContractFail(`${blockPath}.indicator_order`, "must be a nonempty ordered array");
    }
    const indicatorOrder = block.indicator_order.map((indicator, indicatorIndex) => (
      scoreText(indicator, `${blockPath}.indicator_order[${indicatorIndex}]`)
    ));
    if (new Set(indicatorOrder).size !== indicatorOrder.length) {
      scoreContractFail(`${blockPath}.indicator_order`, "contains a duplicate indicator");
    }
    const updateRule = scoreText(block.update_rule, `${blockPath}.update_rule`);
    const initialization = scoreText(block.initialization, `${blockPath}.initialization`);
    const coherent = (updateRule === "mode_a_covariance" || updateRule === "mode_b_ols")
      ? initialization === "standard_unit_weights" || initialization === "individual_requested_weights"
      : updateRule === "fixed_no_update"
        && (initialization === "fixed_unit_weights" || initialization === "fixed_custom_weights");
    if (!coherent) scoreContractFail(blockPath, "has an incoherent update rule or initialization");
    return { constructId, indicatorOrder, updateRule, initialization };
  });
  if (new Set(blocks.map((block) => block.constructId)).size !== blocks.length) {
    scoreContractFail(`${path}.blocks`, "contains a duplicate construct");
  }
  if (execution) {
    if (blocks.length !== execution.blocks.length) {
      scoreContractFail(`${path}.blocks`, "differs from score-execution block count");
    }
    blocks.forEach((block, index) => {
      const resolved = execution.blocks[index];
      const expected = resolved.scoring.kind === "estimated"
        ? {
            updateRule: resolved.scoring.mode === "mode_a" ? "mode_a_covariance" : "mode_b_ols",
            initialization: resolved.scoring.requested_initialization.kind === "standard"
              ? "standard_unit_weights"
              : "individual_requested_weights",
          }
        : {
            updateRule: "fixed_no_update",
            initialization: resolved.scoring.kind === "fixed_unit"
              ? "fixed_unit_weights"
              : "fixed_custom_weights",
          };
      if (
        block.constructId !== resolved.construct_id
        || JSON.stringify(block.indicatorOrder) !== JSON.stringify(resolved.indicator_ids)
        || block.updateRule !== expected.updateRule
        || block.initialization !== expected.initialization
      ) scoreContractFail(`${path}.blocks[${index}]`, "differs from score-execution order or semantics");
    });
  }
  const estimatedBlocks = blocks.filter((block) => block.updateRule !== "fixed_no_update").length;
  if (updates !== performed * estimatedBlocks) {
    scoreContractFail(`${path}.estimated_block_updates`, "differs from performed iterations and estimated blocks");
  }
  const termination = scoreText(receipt.termination_reason, `${path}.termination_reason`);
  if (estimatedBlocks === 0) {
    if (
      termination !== "all_blocks_fixed"
      || performed !== 0
      || Object.prototype.hasOwnProperty.call(receipt, "final_max_outer_weight_change")
    ) scoreContractFail(path, "has an incoherent all-fixed termination receipt");
  } else {
    const finalChange = scoreNumber(
      receipt.final_max_outer_weight_change,
      `${path}.final_max_outer_weight_change`,
    );
    if (
      termination !== "converged_tolerance"
      || performed < 1
      || performed > maximum
      || finalChange < 0
      || finalChange > criterion
    ) scoreContractFail(path, "has an incoherent converged termination receipt");
  }
}

/** Validates the score-contract identity of an untrusted native execution result. */
export function parseInternalRecipeV4PlsExecutionResultV1(
  input: unknown,
): InternalRecipeV4PlsExecutionResultV1 {
  const result = scoreExactRecord(input, ["schema_version", "provenance", "estimation"], [], "result");
  if (result.schema_version !== 1) scoreContractFail("result.schema_version", "must equal 1");
  const provenance = scoreExactRecord(
    result.provenance,
    [
      "adapter_version",
      "compilation_receipt",
      "projected_recipe_schema_version",
      "projected_recipe_sha256",
      "dataset_id",
      "estimator_method_version",
    ],
    ["projected_initialization_sha256", "posthoc_technical_minimum_sample_size"],
    "result.provenance",
  );
  const estimation = scoreRecord(result.estimation, "result.estimation");
  const execution = validatePlsResultScoreExecution(estimation);
  if (provenance.projected_recipe_schema_version !== 3) {
    scoreContractFail("result.provenance.projected_recipe_schema_version", "must equal 3");
  }
  const projectedDigest = scoreText(
    provenance.projected_recipe_sha256,
    "result.provenance.projected_recipe_sha256",
  );
  if (!SHA256_HEX.test(projectedDigest)) {
    scoreContractFail("result.provenance.projected_recipe_sha256", "must be a lowercase SHA-256");
  }
  if ("projected_initialization_sha256" in provenance) {
    const digest = scoreText(
      provenance.projected_initialization_sha256,
      "result.provenance.projected_initialization_sha256",
    );
    if (!SHA256_HEX.test(digest)) {
      scoreContractFail(
        "result.provenance.projected_initialization_sha256",
        "must be a lowercase SHA-256",
      );
    }
  }
  const estimatorMethod = scoreText(
    provenance.estimator_method_version,
    "result.provenance.estimator_method_version",
  );
  if (estimatorMethod !== estimation.method_version) {
    scoreContractFail("result.provenance.estimator_method_version", "differs from estimation");
  }
  const adapterVersion = scoreText(provenance.adapter_version, "result.provenance.adapter_version");
  const nonlinear = estimatorMethod === PLS_NONLINEAR_METHOD_VERSION_V1;
  const legacyAdapter = estimatorMethod === SCORE_EXECUTION_CONTRACT_VERSION_V2
    ? LEGACY_SCORE_EXECUTION_ADAPTER_VERSION_V2
    : estimatorMethod === "pls_pm_v1"
      ? LEGACY_PLS_ADAPTER_VERSION
      : null;
  const currentAdapter = nonlinear
    ? PLS_NONLINEAR_ADAPTER_VERSION_V7
    : estimatorMethod === SCORE_EXECUTION_CONTRACT_VERSION_V2
      ? SCORE_EXECUTION_ADAPTER_VERSION_V2
      : estimatorMethod === "pls_pm_v1"
        ? PLS_ADAPTER_VERSION
        : null;
  if (currentAdapter === null || (adapterVersion !== legacyAdapter && adapterVersion !== currentAdapter)) {
    scoreContractFail("result.provenance.adapter_version", "is not an allowlisted PLS adapter generation");
  }
  const isCurrent = adapterVersion === currentAdapter;
  validateNonlinearEffectsResult(estimation, nonlinear);
  if (nonlinear) {
    if ("posthoc_technical_minimum_sample_size" in provenance) {
      scoreContractFail(
        "result.provenance.posthoc_technical_minimum_sample_size",
        "must be omitted for the nonlinear adapter",
      );
    }
    if ("projected_initialization_sha256" in provenance) {
      scoreContractFail(
        "result.provenance.projected_initialization_sha256",
        "must be omitted for the nonlinear adapter",
      );
    }
    if (Object.prototype.hasOwnProperty.call(estimation, "posthoc_minimum_sample_size")) {
      scoreContractFail(
        "result.estimation.posthoc_minimum_sample_size",
        "must be omitted for the nonlinear adapter",
      );
    }
    parseNonlinearCompilationReceipt(provenance.compilation_receipt);
  }
  validateFixedScoreScaleReceipt(estimation, execution, isCurrent);
  validatePointEstimateAttribution(estimation, isCurrent);
  if (nonlinear && Object.prototype.hasOwnProperty.call(estimation, "algorithm_convergence_receipt")) {
    scoreContractFail(
      "result.estimation.algorithm_convergence_receipt",
      "must be omitted for the v7 nonlinear fixed-score diagnostic",
    );
  }
  validateAlgorithmConvergenceReceipt(estimation, execution, isCurrent && !nonlinear);
  return input as InternalRecipeV4PlsExecutionResultV1;
}

function canonicalReceiptCellMatches(
  cell: CanonicalResultDocumentV2["tables"][number]["rows"][number]["cells"][number],
  expected: string | number | null,
): boolean {
  if (typeof expected === "string") return cell.kind === "text" && cell.value === expected;
  if (typeof expected === "number") return cell.kind === "number" && Object.is(cell.value, expected);
  return cell.kind === "missing" && cell.reason === "not_applicable" && cell.display === undefined;
}

function bindAnalyticalCanonicalReceipts(
  analytical: InternalRecipeV4PlsExecutionResultV1,
  document: CanonicalResultDocumentV2,
): void {
  const estimation = analytical.estimation;
  const attributionTable = document.tables.find((table) => table.id === "point_estimate_attribution");
  const attribution = estimation.point_estimate_attribution;
  if (Boolean(attributionTable) !== Boolean(attribution)) {
    scoreContractFail("completedResult.canonicalDocument.point_estimate_attribution", "differs from analytical receipt presence");
  }
  if (attribution && attributionTable) {
    const expected = [
      attribution.contract_version,
      attribution.preprocessing,
      attribution.indicator_centering,
      attribution.indicator_scaling,
      attribution.outer_weights,
      attribution.outer_loadings,
      attribution.construct_scores,
      attribution.structural_paths,
      attribution.effects,
    ];
    if (attributionTable.rows[0].cells.some((cell, index) => !canonicalReceiptCellMatches(cell, expected[index]))) {
      scoreContractFail("completedResult.canonicalDocument.point_estimate_attribution", "differs from the analytical attribution receipt");
    }
  }

  const convergenceTable = document.tables.find((table) => table.id === "algorithm_convergence_receipt");
  const blockTable = document.tables.find((table) => table.id === "algorithm_block_order");
  const convergence = estimation.algorithm_convergence_receipt;
  if (Boolean(convergenceTable) !== Boolean(convergence) || Boolean(blockTable) !== Boolean(convergence)) {
    scoreContractFail("completedResult.canonicalDocument.algorithm_convergence_receipt", "differs from analytical receipt presence");
  }
  if (convergence && convergenceTable && blockTable) {
    const expectedSummary: Array<string | number | null> = [
      convergence.contract_version,
      convergence.weighting_scheme,
      convergence.maximum_iterations,
      convergence.stop_criterion,
      convergence.comparison,
      convergence.performed_iterations,
      convergence.estimated_block_updates,
      convergence.termination_reason,
      convergence.final_max_outer_weight_change ?? null,
    ];
    if (convergenceTable.rows[0].cells.some((cell, index) => !canonicalReceiptCellMatches(cell, expectedSummary[index]))) {
      scoreContractFail("completedResult.canonicalDocument.algorithm_convergence_receipt", "differs from the analytical convergence receipt");
    }
    const expectedBlocks = convergence.blocks.flatMap((block, blockIndex) => (
      block.indicator_order.map((indicator, indicatorIndex) => ({
        id: `algorithm_block_${blockIndex.toString().padStart(4, "0")}_indicator_${indicatorIndex.toString().padStart(4, "0")}`,
        cells: [blockIndex, block.construct_id, indicatorIndex, indicator, block.update_rule, block.initialization] as Array<string | number>,
      }))
    ));
    if (
      blockTable.rows.length !== expectedBlocks.length
      || blockTable.rows.some((row, rowIndex) => (
        row.id !== expectedBlocks[rowIndex].id
        || row.cells.some((cell, cellIndex) => (
          !canonicalReceiptCellMatches(cell, expectedBlocks[rowIndex].cells[cellIndex])
        ))
      ))
    ) scoreContractFail("completedResult.canonicalDocument.algorithm_block_order", "differs from the analytical block receipt");
  }

  const fixedTable = document.tables.find((table) => table.id === "fixed_score_scale_receipt");
  const fixed = estimation.fixed_score_scale_receipt;
  if (Boolean(fixedTable) !== Boolean(fixed)) {
    scoreContractFail("completedResult.canonicalDocument.fixed_score_scale_receipt", "differs from analytical receipt presence");
  }
  if (fixed && fixedTable && estimation.score_execution) {
    const expectedRows = fixed.blocks.flatMap((block) => {
      const scoring = estimation.score_execution!.blocks.find(
        (candidate) => candidate.construct_id === block.construct_id,
      )?.scoring;
      const resolved = scoring && scoring.kind !== "estimated" ? scoring.resolved_effective_weights : [];
      return block.indicator_ids.map((indicator, index) => [
        fixed.contract_version,
        block.construct_id,
        indicator,
        block.pre_standardization_center,
        block.pre_standardization_scale,
        resolved[index]?.value,
        block.effective_unit_score_weights[index]?.value,
      ] as Array<string | number | undefined>);
    });
    if (
      fixedTable.rows.length !== expectedRows.length
      || expectedRows.some((expected, rowIndex) => expected.some((value, cellIndex) => (
        value === undefined || !canonicalReceiptCellMatches(fixedTable.rows[rowIndex].cells[cellIndex], value)
      )))
    ) scoreContractFail("completedResult.canonicalDocument.fixed_score_scale_receipt", "differs from the analytical fixed-scale receipt");
  }
}

function bindAnalyticalCanonicalNonlinear(
  analytical: InternalRecipeV4PlsExecutionResultV1,
  document: CanonicalResultDocumentV2,
): void {
  const analysis = analytical.estimation.nonlinear_effects;
  if (!analysis) return;
  const diagnostics = document.tables.find((table) => table.id === "nonlinear_quadratic_diagnostics");
  const equations = document.tables.find((table) => table.id === "nonlinear_equation_fit");
  const scope = document.tables.find((table) => table.id === "nonlinear_method_scope");
  if (!diagnostics || !equations || !scope) {
    scoreContractFail("completedResult.canonicalDocument", "omits the nonlinear canonical table family");
  }
  const estimates = [...analysis.estimates].sort((left, right) => (
    compareUtf8StringsV1(left.target, right.target) || compareUtf8StringsV1(left.source, right.source)
  ));
  if (
    diagnostics.rows.length !== estimates.length
    || diagnostics.rows.some((row, index) => {
      const estimate = estimates[index];
      const warning = row.cells[7];
      const warningMatches = estimate.warning === null
        ? warning.kind === "missing"
          && warning.reason === "not_estimated"
          && warning.display === undefined
        : canonicalReceiptCellMatches(warning, estimate.warning);
      const expected: Array<string | number> = [
        estimate.source,
        estimate.target,
        estimate.linear_coefficient,
        estimate.quadratic_coefficient,
        estimate.standard_error,
        estimate.t_statistic,
        estimate.p_value_two_sided,
      ];
      return row.id !== `nonlinear_quadratic_diagnostic_${index.toString().padStart(4, "0")}`
        || expected.some((value, cellIndex) => !canonicalReceiptCellMatches(row.cells[cellIndex], value))
        || !warningMatches;
    })
  ) scoreContractFail("completedResult.canonicalDocument.nonlinear_quadratic_diagnostics", "differs from the analytical nonlinear estimates");

  const equationByTarget = new Map<string, readonly [number, number, number]>();
  estimates.forEach((estimate) => equationByTarget.set(estimate.target, [
    estimate.linear_r_squared,
    estimate.augmented_r_squared,
    estimate.delta_r_squared,
  ]));
  const expectedEquations = [...equationByTarget].sort(([left], [right]) => compareUtf8StringsV1(left, right));
  if (
    equations.rows.length !== expectedEquations.length
    || equations.rows.some((row, index) => {
      const [target, values] = expectedEquations[index];
      const expected: Array<string | number> = [target, ...values];
      return row.id !== `nonlinear_equation_fit_${index.toString().padStart(4, "0")}`
        || expected.some((value, cellIndex) => !canonicalReceiptCellMatches(row.cells[cellIndex], value));
    })
  ) scoreContractFail("completedResult.canonicalDocument.nonlinear_equation_fit", "differs from the analytical nonlinear estimates");
  const scopeRow = scope.rows[0];
  const expectedScope = [analysis.method_version, analysis.term, analysis.warnings[0]];
  if (
    scope.rows.length !== 1
    || scopeRow.id !== "nonlinear_method_scope"
    || expectedScope.some((value, index) => !canonicalReceiptCellMatches(scopeRow.cells[index], value))
  ) scoreContractFail("completedResult.canonicalDocument.nonlinear_method_scope", "differs from the analytical nonlinear scope");
}

/** Validates the completed analytical/canonical pair before it can be persisted. */
export function parseInternalRecipeV4CompletedResultV1(
  input: unknown,
): InternalRecipeV4CompletedResultV1 {
  const completed = scoreExactRecord(
    input,
    ["schemaVersion", "analyticalResult", "canonicalDocument"],
    [],
    "completedResult",
  );
  if (completed.schemaVersion !== 1) scoreContractFail("completedResult.schemaVersion", "must equal 1");
  const analyticalResult = parseInternalRecipeV4PlsExecutionResultV1(completed.analyticalResult);
  const canonicalDocument = completed.canonicalDocument as CanonicalResultDocumentV2;
  const validation = validateCanonicalResultDocumentV2(canonicalDocument);
  if (!validation.passed) {
    scoreContractFail("completedResult.canonicalDocument", validation.errors.join("; "));
  }
  validateArchivedPlsScoreExecutionV2(
    canonicalDocument,
    "completedResult.canonicalDocument",
  );
  const methodVersion = analyticalResult.estimation.method_version;
  const tableIds = new Set(canonicalDocument.tables.map((table) => table.id));
  const hasSummary = tableIds.has("score_execution_summary");
  const hasWeights = tableIds.has("score_execution_weights");
  const hasAttribution = tableIds.has("point_estimate_attribution");
  const hasConvergence = tableIds.has("algorithm_convergence_receipt");
  const hasBlockOrder = tableIds.has("algorithm_block_order");
  const legacyAdditionsCoherent = hasConvergence === hasBlockOrder
    && (!hasConvergence || hasAttribution);
  const currentFamilies = hasAttribution && hasConvergence && hasBlockOrder;
  const adapterVersion = analyticalResult.provenance.adapter_version;
  const canonicalAdapter = canonicalDocument.provenance.engine_version;
  const adapterIdentity = adapterVersion === canonicalAdapter;
  const legacyPlain = methodVersion === "pls_pm_v1"
    && adapterVersion === LEGACY_PLS_ADAPTER_VERSION
    && !hasSummary && !hasWeights && legacyAdditionsCoherent;
  const currentPlain = methodVersion === "pls_pm_v1"
    && adapterVersion === PLS_ADAPTER_VERSION
    && !hasSummary && !hasWeights && currentFamilies;
  const legacyScore = methodVersion === SCORE_EXECUTION_CONTRACT_VERSION_V2
    && adapterVersion === LEGACY_SCORE_EXECUTION_ADAPTER_VERSION_V2
    && hasSummary && hasWeights && legacyAdditionsCoherent;
  const currentScore = methodVersion === SCORE_EXECUTION_CONTRACT_VERSION_V2
    && adapterVersion === SCORE_EXECUTION_ADAPTER_VERSION_V2
    && hasSummary && hasWeights && currentFamilies;
  const nonlinear = methodVersion === PLS_NONLINEAR_METHOD_VERSION_V1
    && adapterVersion === PLS_NONLINEAR_ADAPTER_VERSION_V7
    && !hasSummary && !hasWeights && hasAttribution && !hasConvergence && !hasBlockOrder
    && tableIds.has("nonlinear_quadratic_diagnostics")
    && tableIds.has("nonlinear_equation_fit")
    && tableIds.has("nonlinear_method_scope");
  if (
    canonicalDocument.provenance.method_version !== methodVersion
    || !adapterIdentity
    || (!legacyPlain && !currentPlain && !legacyScore && !currentScore && !nonlinear)
  ) {
    scoreContractFail("completedResult.canonicalDocument", "has drifted PLS score identity");
  }
  if (nonlinear) {
    const receipt = analyticalResult.provenance.compilation_receipt;
    if (
      canonicalDocument.provenance.recipe_id !== receipt.recipe_id
      || canonicalDocument.provenance.recipe_digest !== receipt.recipe_analytical_sha256
      || canonicalDocument.provenance.model_id !== receipt.model_id
      || canonicalDocument.provenance.model_digest !== receipt.model_scientific_sha256
      || canonicalDocument.provenance.dataset_id !== analyticalResult.provenance.dataset_id
      || canonicalDocument.provenance.dataset_fingerprint !== recordedDatasetSha256(
        receipt.dataset_fingerprint,
        "completedResult.analyticalResult.provenance.compilation_receipt.dataset_fingerprint",
      )
    ) scoreContractFail("completedResult.canonicalDocument.provenance", "differs from the v7 compilation and resident-data identity");
  }
  bindAnalyticalCanonicalReceipts(analyticalResult, canonicalDocument);
  bindAnalyticalCanonicalNonlinear(analyticalResult, canonicalDocument);
  return { schemaVersion: 1, analyticalResult, canonicalDocument };
}
