import type { CanonicalResultDocumentV2, CapabilityCellReferenceV2 } from "./canonicalResultDocumentV2";
import { canonicalResultDocumentJson, validateCanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import {
  capabilityOptionCellAvailabilityV2,
  capabilityRegistryV2,
  type CapabilityOptionCellV2,
} from "./capabilityRegistryV2";
import {
  defaultGeneralSemConfigV1,
  parseGeneralSemConfigV1,
  type GeneralSemConfigV1,
  type GeneralSemEffectEstimandV1,
} from "./generalSemConfigV1";
import {
  GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1,
  GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1,
  GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1,
  GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1,
  GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1,
  GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1,
  GENERAL_SEM_PLS_MULTIPLE_MODERATION_SIGN_ALIGNMENT_VERSION_V1,
  GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
  GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1,
  GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
  GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
  GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1,
  GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1,
} from "./canonicalGeneralSemResultsV1";
import {
  GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_CELL_V1,
  GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_CELL_V1,
  preflightGeneralSemPlsV1,
} from "./generalSemCapabilityPreflightV1";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "./internalProjectArchiveV6Read";
import type {
  InternalProjectSchema6CanonicalResultEntryV1,
  InternalProjectSchema6ResultReadOutcomeV1,
  InternalProjectSchema6ResultReadRequestV1,
} from "./internalProjectSchema6ResultRead";
import type {
  InternalProjectSchema6ResultAppendOutcomeV1,
  InternalProjectSchema6ResultAppendRequestV1,
} from "./internalProjectSchema6ResultAppend";
import type {
  AnalysisRecipeV4,
  AnalysisRecipeV4Settings,
} from "./internalRecipeV4PlsExecution";
import { parseSemCapabilityDecisionV1, type SemCapabilityDecisionV1 } from "./semCapabilityDecisionV1";
import {
  canonicalizeSemModelV4,
  compareUtf8StringsV1,
  validateSemModelV4,
  type SemModelV4,
} from "./semModelV4";
import { sha256HexUtf8V1 } from "./sha256V1";
import type { Dataset } from "../types";

export const GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1 = Object.freeze({
  registry_schema_version: 2,
  capability_id: "smartpls.mediation",
  cell_id: "qpls3.pls.mediation",
  capability_version: "pls_mediation_v1",
} as const satisfies CapabilityCellReferenceV2);

export const GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1 = Object.freeze({
  registry_schema_version: 2,
  capability_id: "smartpls.mediation",
  cell_id: "qpls3.pls.general_sem_multiple_mediation_bootstrap",
  capability_version: "general_sem_pls_full_model_case_bootstrap_v1",
} as const satisfies CapabilityCellReferenceV2);

export const GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1 =
  GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_CELL_V1;

export const GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1 =
  GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_CELL_V1;

const GENERAL_SEM_PLS_BASE_CAPABILITY_CELL_V1 = Object.freeze({
  registry_schema_version: 2,
  capability_id: "smartpls.pls_algorithm",
  cell_id: "qpls3.pls.algorithm",
  capability_version: "pls_pm_v1",
} as const satisfies CapabilityCellReferenceV2);

const GENERAL_SEM_PLS_POINT_METHOD_VERSION_V1 = "general_sem_effects_v1" as const;
const GENERAL_SEM_PLS_POINT_ADAPTER_VERSION_V1 = "compiled_general_sem_pls_recipe_v1_point_execution_v1" as const;
const GENERAL_SEM_PLS_BOOTSTRAP_ADAPTER_VERSION_V1 = "compiled_general_sem_pls_recipe_v1_percentile_bootstrap_execution_v1" as const;
const GENERAL_SEM_PLS_MODERATION_POINT_ADAPTER_VERSION_V1 = "compiled_general_sem_pls_recipe_v1_multiple_two_way_moderation_point_execution_v1" as const;
const GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_ADAPTER_VERSION_V1 = "compiled_general_sem_pls_recipe_v1_multiple_two_way_moderation_percentile_bootstrap_execution_v1" as const;

export interface GeneralSemPlsEngineOptionsV1 {
  tolerance: number;
  maxIterations: number;
  inference: "none" | "percentile_case_bootstrap";
  bootstrapSamples: number;
  seed: number;
  workers: number;
  confidenceLevel: number;
  maxMaterializedSpecificPaths: number;
}

export const defaultGeneralSemPlsEngineOptionsV1 = (): GeneralSemPlsEngineOptionsV1 => ({
  tolerance: 1e-7,
  maxIterations: 1_000,
  inference: "none",
  bootstrapSamples: 500,
  seed: 42,
  workers: 1,
  confidenceLevel: 0.95,
  maxMaterializedSpecificPaths: 10_000,
});

export interface GeneralSemWorkspaceIssueV1 {
  code: string;
  subject: string;
  message: string;
  correctiveAction: string;
}

export interface GeneralSemWorkspacePreflightV1 {
  ready: boolean;
  decision: SemCapabilityDecisionV1 | null;
  issues: GeneralSemWorkspaceIssueV1[];
}

export class GeneralSemWorkspaceErrorV1 extends Error {
  constructor(
    public readonly code: string,
    public readonly subject: string,
    message: string,
    public readonly correctiveAction: string,
  ) {
    super(message);
    this.name = "GeneralSemWorkspaceErrorV1";
  }
}

export interface GeneralSemCapabilityRegistryReaderV1 {
  requireOptionCell(capabilityId: string, cellId: string): CapabilityOptionCellV2;
}

export type GeneralSemExecutionAccessV1 =
  | { readonly surface: "standard"; readonly experimentalLabsEnabled: false }
  | { readonly surface: "internal_labs"; readonly experimentalLabsEnabled: true };

export type GeneralSemReadAccessV1 = {
  readonly surface: "standard" | "internal_labs";
  readonly experimentalLabsEnabled: false;
};

/**
 * Immutable RecipeV4 metadata identities. The Labs value is historical and
 * must remain readable after a cell is promoted; newly built recipes always
 * use the identity selected from the current exact Registry V2 cell.
 */
export const GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1 =
  "native_general_sem_pls_labs_v1" as const;
export const GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1 =
  "native_general_sem_pls_standard_v1" as const;
export type GeneralSemPlsRecipeExecutionSurfaceV1 =
  | typeof GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1
  | typeof GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1;

function generalSemRecipeExecutionSurfaceV1(
  access: GeneralSemExecutionAccessV1,
): GeneralSemPlsRecipeExecutionSurfaceV1 {
  return access.surface === "standard"
    ? GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1
    : GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1;
}

/**
 * Resolves one exact Registry V2 cell to its stable execution surface. The
 * native command repeats this decision against its embedded registry, so this
 * frontend selection improves workflow without becoming an authorization
 * authority.
 */
export function selectGeneralSemExecutionAccessV1(input: {
  capabilityCell: CapabilityCellReferenceV2;
  experimentalLabsEnabled: boolean;
  registry?: GeneralSemCapabilityRegistryReaderV1;
}): GeneralSemExecutionAccessV1 {
  const registry = input.registry ?? capabilityRegistryV2;
  let cell: CapabilityOptionCellV2;
  try {
    cell = registry.requireOptionCell(
      input.capabilityCell.capability_id,
      input.capabilityCell.cell_id,
    );
  } catch {
    throw new GeneralSemWorkspaceErrorV1(
      "general_sem.access.capability_unavailable",
      input.capabilityCell.cell_id,
      "The exact General SEM option cell is absent from Capability Registry V2.",
      "Refresh the verified capability registry and rerun exact estimator preflight.",
    );
  }
  if (cell.capability_id !== input.capabilityCell.capability_id
    || cell.cell_id !== input.capabilityCell.cell_id
    || cell.capability_version !== input.capabilityCell.capability_version
    || input.capabilityCell.registry_schema_version !== 2) {
    throw new GeneralSemWorkspaceErrorV1(
      "general_sem.access.capability_unavailable",
      input.capabilityCell.cell_id,
      "The requested General SEM option-cell identity differs from Capability Registry V2.",
      "Refresh the verified capability registry and rerun exact estimator preflight.",
    );
  }
  const availability = capabilityOptionCellAvailabilityV2(
    cell,
    input.experimentalLabsEnabled,
  );
  if (availability.reason === "standard_ready") {
    return { surface: "standard", experimentalLabsEnabled: false };
  }
  if (availability.reason === "labs_ready") {
    return { surface: "internal_labs", experimentalLabsEnabled: true };
  }
  if (availability.reason === "labs_disabled") {
    throw new GeneralSemWorkspaceErrorV1(
      "general_sem.access.experimental_labs_required",
      "experimentalLabsEnabled",
      "The exact General SEM option cell is available only through Experimental Labs.",
      "Enable Experimental Labs, or choose a Standard-qualified General SEM cell.",
    );
  }
  throw new GeneralSemWorkspaceErrorV1(
    "general_sem.access.capability_unavailable",
    input.capabilityCell.cell_id,
    "The exact General SEM option cell is not executable on its registered surface.",
    "Keep the project unchanged and use a qualified Capability Registry V2 cell.",
  );
}

/** Product-level navigation access; the selected calculation cell is rechecked separately. */
export function generalSemWorkspaceProductAccessV1(
  experimentalLabsEnabled: boolean,
  registry: GeneralSemCapabilityRegistryReaderV1 = capabilityRegistryV2,
): GeneralSemExecutionAccessV1 | null {
  const cells = [
    GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1,
    GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1,
    GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1,
    GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
  ];
  for (const capabilityCell of cells) {
    try {
      const access = selectGeneralSemExecutionAccessV1({
        capabilityCell,
        experimentalLabsEnabled: false,
        registry,
      });
      if (access.surface === "standard") return access;
    } catch {
      // Product navigation stays fail-closed until at least one exact cell is available.
    }
  }
  if (!experimentalLabsEnabled) return null;
  for (const capabilityCell of cells) {
    try {
      const access = selectGeneralSemExecutionAccessV1({
        capabilityCell,
        experimentalLabsEnabled: true,
        registry,
      });
      if (access.surface === "internal_labs") return access;
    } catch {
      // Keep searching the bounded General SEM option-cell inventory.
    }
  }
  return null;
}

const SHA256 = /^[a-f0-9]{64}$/;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

function issue(
  code: string,
  subject: string,
  message: string,
  correctiveAction: string,
): GeneralSemWorkspaceIssueV1 {
  return { code, subject, message, correctiveAction };
}

export function bindGeneralSemPlsModelToDatasetV1(model: SemModelV4, dataset: Dataset): SemModelV4 {
  if (dataset.kind === "covariance" || dataset.kind === "correlation") {
    throw new GeneralSemWorkspaceErrorV1(
      "general_sem_pls.raw_data_required",
      dataset.id,
      "The current General SEM PLS estimator requires resident raw case-level data.",
      "Choose a raw resident dataset. Matrix input remains available to qualified CB-SEM cells.",
    );
  }
  if (model.data_binding.kind !== "raw") {
    throw new GeneralSemWorkspaceErrorV1(
      "general_sem_pls.raw_model_binding_required",
      model.id,
      "The authored SEM model does not use a raw-data binding.",
      "Choose a raw-data SEM model. Unsupported data semantics are preserved and must be changed explicitly in the model editor.",
    );
  }
  return canonicalizeSemModelV4({
    ...model,
    data_binding: {
      ...model.data_binding,
      dataset_id: dataset.id,
    },
  });
}

export function generalSemConfigFromEngineV1(
  engine: GeneralSemPlsEngineOptionsV1,
  requestedEffectEstimands: readonly GeneralSemEffectEstimandV1[] = [],
): GeneralSemConfigV1 {
  return parseGeneralSemConfigV1({
    ...defaultGeneralSemConfigV1(),
    requested_effect_estimands: [...requestedEffectEstimands],
    inference: engine.inference === "none"
      ? { kind: "none" }
      : {
          kind: "case_bootstrap",
          resamples: engine.bootstrapSamples,
          seed: engine.seed,
          confidence_level: engine.confidenceLevel,
          interval: "percentile",
          tail: "two_sided",
        },
    output_policy: {
      max_materialized_specific_paths: engine.maxMaterializedSpecificPaths,
      lazy_specific_path_materialization: false,
      when_specific_path_limit_exceeded: "error",
    },
  });
}

export function preflightGeneralSemWorkspaceV1(input: {
  experimentalLabsEnabled: boolean;
  capabilityRegistry?: GeneralSemCapabilityRegistryReaderV1;
  sourceProjectId: string | null;
  dataset: Dataset | null;
  model: SemModelV4 | null;
  config: GeneralSemConfigV1;
  engine: GeneralSemPlsEngineOptionsV1;
}): GeneralSemWorkspacePreflightV1 {
  const issues: GeneralSemWorkspaceIssueV1[] = [];
  if (!input.sourceProjectId || !UUID.test(input.sourceProjectId)) issues.push(issue(
    "general_sem.project.stable_identity_required",
    "project",
    "The active project does not expose a stable native project identity.",
    "Save or reopen the active native project before creating a General SEM calculation project.",
  ));
  if (!input.dataset) issues.push(issue(
    "general_sem.dataset.required",
    "dataset",
    "A resident raw dataset is required.",
    "Import or select a raw dataset in the active project.",
  ));
  else {
    if (!input.dataset.fingerprint?.trim()) issues.push(issue(
      "general_sem.dataset.fingerprint_required",
      input.dataset.id,
      "The resident dataset has no stable fingerprint.",
      "Reimport or reactivate the dataset before calculation.",
    ));
    if (input.dataset.kind === "covariance" || input.dataset.kind === "correlation") issues.push(issue(
      "general_sem.dataset.raw_required",
      input.dataset.id,
      "The current General SEM PLS slice requires raw case-level data.",
      "Choose a raw dataset.",
    ));
  }
  if (!input.model) issues.push(issue(
    "general_sem.model.required",
    "model",
    "The canvas cannot currently produce a valid SemModelV4 authority.",
    "Resolve the scientific authoring diagnostics on the canvas and Parameter Table.",
  ));
  else {
    for (const diagnostic of validateSemModelV4(input.model)) issues.push(issue(
      diagnostic.code,
      diagnostic.subject ?? "model",
      diagnostic.message,
      "Correct the named scientific object and run compatibility preflight again.",
    ));
  }
  if (input.dataset && input.model) {
    if (input.model.data_binding.kind !== "raw"
      || input.model.data_binding.dataset_id !== input.dataset.id) issues.push(issue(
      "general_sem.dataset.binding_mismatch",
      input.dataset.id,
      "The model and resident dataset identities or binding kinds differ.",
      "Bind the model explicitly to the selected resident raw dataset.",
    ));
    for (const variable of input.model.variables) {
      if (variable.kind !== "observed") continue;
      if (!input.dataset.columns.includes(variable.source_column)) {
        issues.push(issue(
          "general_sem.dataset.observed_column_missing",
          variable.id,
          `Observed source column ${variable.source_column} is absent from the resident dataset.`,
          "Restore the exact source column or correct the observed-variable binding.",
        ));
        continue;
      }
      const metadata = input.dataset.columnMetadata?.find((column) => column.name === variable.source_column);
      if (!metadata || metadata.column_type !== "numeric" || metadata.scale_type !== "continuous") issues.push(issue(
        "general_sem.dataset.continuous_numeric_required",
        variable.id,
        `Observed source column ${variable.source_column} must have continuous numeric metadata for this exact PLS cell.`,
        "Correct the column metadata through an explicit dataset operation, or use a future qualified estimator cell.",
      ));
    }
  }
  if (!Number.isFinite(input.engine.tolerance) || input.engine.tolerance <= 0 || input.engine.tolerance > 0.01) issues.push(issue(
    "general_sem.settings.tolerance_invalid", "tolerance", "Tolerance must be greater than zero and no more than 0.01.", "Enter a supported convergence tolerance.",
  ));
  if (!Number.isSafeInteger(input.engine.maxIterations) || input.engine.maxIterations < 100 || input.engine.maxIterations > 100_000) issues.push(issue(
    "general_sem.settings.iterations_invalid", "maxIterations", "Maximum iterations must be between 100 and 100,000.", "Enter a supported iteration limit.",
  ));
  if (!Number.isSafeInteger(input.engine.seed) || input.engine.seed < 0 || input.engine.seed > Number.MAX_SAFE_INTEGER) issues.push(issue(
    "general_sem.settings.seed_invalid", "seed", "Seed must be a nonnegative safe integer.", "Enter a seed from 0 through 9,007,199,254,740,991.",
  ));
  if (!Number.isSafeInteger(input.engine.workers) || input.engine.workers < 1 || input.engine.workers > 64) issues.push(issue(
    "general_sem.settings.workers_invalid", "workers", "Worker count must be between 1 and 64.", "Choose a supported worker count.",
  ));
  if (input.engine.inference === "percentile_case_bootstrap"
    && (!Number.isSafeInteger(input.engine.bootstrapSamples) || input.engine.bootstrapSamples < 2 || input.engine.bootstrapSamples > 10_000)) issues.push(issue(
      "general_sem.settings.bootstrap_samples_invalid", "bootstrapSamples", "Bootstrap samples must be an integer from 2 through 10,000.", "Choose a supported full-model case-bootstrap count.",
    ));

  let decision: SemCapabilityDecisionV1 | null = null;
  if (input.model) {
    try {
      selectGeneralSemExecutionAccessV1({
        capabilityCell: generalSemPlsRequestedCapabilityCellV1(input.model, input.config),
        experimentalLabsEnabled: input.experimentalLabsEnabled,
        registry: input.capabilityRegistry,
      });
      decision = preflightGeneralSemPlsV1(input.model, input.config);
      for (const diagnostic of decision.diagnostics.filter((item) => item.severity === "error")) issues.push(issue(
        diagnostic.code,
        diagnostic.subject ?? "model",
        diagnostic.message,
        diagnostic.corrections[0] ?? "Correct the model or calculation configuration.",
      ));
    } catch (error) {
      issues.push(error instanceof GeneralSemWorkspaceErrorV1
        ? issue(error.code, error.subject, error.message, error.correctiveAction)
        : issue(
          "general_sem.preflight.contract_invalid",
          "preflight",
          error instanceof Error ? error.message : "The General SEM capability decision could not be validated.",
          "Keep the model unchanged and retry after reopening the current project.",
        ));
    }
  }
  return {
    ready: issues.length === 0
      && (decision?.status === "experimental" || decision?.status === "supported"),
    decision,
    issues,
  };
}

export interface BuildGeneralSemRecipeV1Input {
  recipeId: string;
  createdAt: string;
  dataset: Dataset;
  model: SemModelV4;
  nativeScientificSha256: string;
  config: GeneralSemConfigV1;
  engine: GeneralSemPlsEngineOptionsV1;
  capabilityCell: CapabilityCellReferenceV2;
  experimentalLabsEnabled: boolean;
  capabilityRegistry?: GeneralSemCapabilityRegistryReaderV1;
}

export function buildGeneralSemRecipeV1(input: BuildGeneralSemRecipeV1Input): AnalysisRecipeV4 {
  if (!SHA256.test(input.nativeScientificSha256)) throw new GeneralSemWorkspaceErrorV1(
    "general_sem.model.native_digest_invalid", input.model.id, "The native model digest is invalid.", "Re-run native scientific validation.",
  );
  if (!input.dataset.fingerprint?.trim() || input.model.data_binding.dataset_id !== input.dataset.id) throw new GeneralSemWorkspaceErrorV1(
    "general_sem.dataset.binding_mismatch", input.dataset.id, "The model and resident dataset identities differ.", "Rebind the model to the selected dataset.",
  );
  const config = parseGeneralSemConfigV1(input.config);
  const access = selectGeneralSemExecutionAccessV1({
    capabilityCell: input.capabilityCell,
    experimentalLabsEnabled: input.experimentalLabsEnabled,
    registry: input.capabilityRegistry,
  });
  const requestedCell = generalSemPlsRequestedCapabilityCellV1(input.model, config);
  if (requestedCell.registry_schema_version !== input.capabilityCell.registry_schema_version
    || requestedCell.capability_id !== input.capabilityCell.capability_id
    || requestedCell.cell_id !== input.capabilityCell.cell_id
    || requestedCell.capability_version !== input.capabilityCell.capability_version) {
    throw new GeneralSemWorkspaceErrorV1(
      "general_sem.recipe.capability_cell_mismatch",
      input.capabilityCell.cell_id,
      "The RecipeV4 capability cell differs from the exact model and General SEM configuration.",
      "Rerun exact estimator preflight and rebuild the recipe from the unchanged model.",
    );
  }
  const bootstrapInference = config.inference.kind === "case_bootstrap" ? config.inference : null;
  const settings: AnalysisRecipeV4Settings<"listwise_deletion"> = {
    method: "pls_pm",
    weighting_scheme: "path",
    tolerance: input.engine.tolerance,
    max_iterations: input.engine.maxIterations,
    bootstrap_samples: bootstrapInference?.resamples ?? 0,
    bootstrap_test_tail: "two_sided",
    studentized_inner_samples: 0,
    permutation_samples: 0,
    seed: input.engine.seed,
    workers: input.engine.workers,
    confidence_level: input.engine.confidenceLevel,
    preprocessing: "standardized",
    missing_data: "listwise_deletion",
    case_weight_column: null,
  };
  return {
    schema_version: 4,
    id: input.recipeId,
    created_at: input.createdAt,
    dataset_fingerprint: input.dataset.fingerprint,
    model_binding: {
      kind: "project_sem_model_v4_reference",
      model_id: input.model.id,
      scientific_sha256: input.nativeScientificSha256,
    },
    estimand_confirmation: "not_legacy",
    settings,
    method_config: { kind: "pls_algorithm" },
    general_sem_config: config,
    metadata: {
      execution_surface: generalSemRecipeExecutionSurfaceV1(access),
      general_sem_generation: "general_sem_v1",
    },
    legacy_source: null,
  };
}

export type GeneralSemProjectBootstrapRequestV1 = GeneralSemExecutionAccessV1 & {
  capabilityCell: CapabilityCellReferenceV2;
  destinationPath: string;
  projectId: string;
  name: string;
  createdAt: string;
  sourceProjectId: string;
  sourceDatasetId: string;
  sourceDatasetFingerprint: string;
  model: SemModelV4;
  recipe: AnalysisRecipeV4;
};

export interface GeneralSemProjectBootstrapReceiptV1 {
  schemaVersion: 1;
  archiveSchemaVersion: 6;
  projectId: string;
  name: string;
  createdAt: string;
  destinationArchivePath: string;
  destinationArchiveSha256: string;
  destinationArchiveBytes: number;
  strictReopenValidated: true;
  residentDatasetId: string;
  residentDatasetFingerprint: string;
  residentModelId: string;
  residentModelScientificSha256: string;
  residentRecipeId: string;
  residentRecipeDocumentSha256: string;
}

export interface RehydratedGeneralSemExecutionAuthorityV1 {
  receipt: GeneralSemProjectBootstrapReceiptV1;
  engine: GeneralSemPlsEngineOptionsV1;
  /** Exact resident config; never reconstructed from UI defaults. */
  config: GeneralSemConfigV1;
  /** Current Registry-selected access for this exact resident cell. */
  executionAccess: GeneralSemExecutionAccessV1;
  /** Stored recipe surface used only for immutable strict result readback. */
  readAccess: GeneralSemReadAccessV1;
  capabilityCell: CapabilityCellReferenceV2;
  /** Immutable metadata read from the resident RecipeV4. */
  recipeExecutionSurface: GeneralSemPlsRecipeExecutionSurfaceV1;
  /** True only when a historical Labs recipe is read after cell promotion. */
  legacyLabsRecipeOnStandardCell: boolean;
}

/** Restores the exact native RecipeV4 authority after remount or process restart. */
export function rehydrateGeneralSemExecutionAuthorityV1(
  snapshot: InternalProjectArchiveV6ReadSnapshotV1,
  registry: GeneralSemCapabilityRegistryReaderV1 = capabilityRegistryV2,
): RehydratedGeneralSemExecutionAuthorityV1 {
  const authority = snapshot.generalSemExecutionAuthority;
  if (!authority) throw new GeneralSemWorkspaceErrorV1(
    "general_sem.rehydrate.native_authority_required",
    snapshot.archivePath,
    "The marked QuickPLS project did not expose a native General SEM execution authority.",
    "Preserve the file unchanged and reopen it with the matching QuickPLS version.",
  );
  const recipe = authority.recipe;
  const config = recipe.general_sem_config
    ? parseGeneralSemConfigV1(recipe.general_sem_config)
    : null;
  const methodConfig = recipe.method_config as { kind?: unknown } | undefined;
  const recipeExecutionSurface = recipe.metadata.execution_surface;
  if (!config
    || recipe.settings.method !== "pls_pm"
    || recipe.settings.weighting_scheme !== "path"
    || recipe.settings.preprocessing !== "standardized"
    || recipe.settings.missing_data !== "listwise_deletion"
    || recipe.settings.case_weight_column !== null
    || methodConfig?.kind !== "pls_algorithm"
    || (recipeExecutionSurface !== GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1
      && recipeExecutionSurface !== GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1)
    || recipe.metadata.general_sem_generation !== "general_sem_v1") {
    throw new GeneralSemWorkspaceErrorV1(
      "general_sem.rehydrate.recipe_scope_mismatch",
      authority.recipeId,
      "The resident RecipeV4 is outside the bounded General SEM PLS execution scope.",
      "Keep the archive unchanged and use an estimator cell that explicitly supports its recipe.",
    );
  }
  const modelRecord = snapshot.project.models.find((record) => record.model_id === authority.modelId);
  if (!modelRecord || modelRecord.payload.kind !== "sem_model_v4") {
    throw new GeneralSemWorkspaceErrorV1(
      "general_sem.rehydrate.model_authority_required",
      authority.modelId,
      "The resident RecipeV4 model reference does not resolve to one promoted SemModelV4 authority.",
      "Keep the archive unchanged and reopen it with the matching QuickPLS version.",
    );
  }
  const capabilityCell = generalSemPlsRequestedCapabilityCellV1(modelRecord.payload.model, config);
  const executionAccess = selectGeneralSemExecutionAccessV1({
    capabilityCell,
    // Rehydration is read compatibility, not permission to execute. Discover
    // the cell's current Registry surface while preserving historical bytes.
    experimentalLabsEnabled: true,
    registry,
  });
  const expectedRecipeSurface = generalSemRecipeExecutionSurfaceV1(executionAccess);
  const legacyLabsRecipeOnStandardCell =
    recipeExecutionSurface === GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1
    && executionAccess.surface === "standard";
  if (recipeExecutionSurface !== expectedRecipeSurface && !legacyLabsRecipeOnStandardCell) {
    throw new GeneralSemWorkspaceErrorV1(
      "general_sem.rehydrate.execution_surface_mismatch",
      authority.recipeId,
      "The resident RecipeV4 execution-surface identity disagrees with its exact Capability Registry V2 cell.",
      "Preserve the archive unchanged and report the authority mismatch.",
    );
  }
  const inference = config.inference;
  if ((inference.kind === "none" && recipe.settings.bootstrap_samples !== 0)
    || (inference.kind === "case_bootstrap" && (
      inference.interval !== "percentile"
      || inference.tail !== "two_sided"
      || inference.resamples !== recipe.settings.bootstrap_samples
      || inference.seed !== recipe.settings.seed
      || inference.confidence_level !== recipe.settings.confidence_level
    ))) {
    throw new GeneralSemWorkspaceErrorV1(
      "general_sem.rehydrate.inference_mismatch",
      authority.recipeId,
      "The resident RecipeV4 settings and GeneralSemConfigV1 inference authority differ.",
      "Preserve the archive unchanged and report the authority mismatch.",
    );
  }
  return {
    receipt: {
      schemaVersion: 1,
      archiveSchemaVersion: 6,
      projectId: authority.projectId,
      name: snapshot.project.name,
      createdAt: snapshot.project.created_at,
      destinationArchivePath: snapshot.archivePath,
      destinationArchiveSha256: snapshot.archiveSha256,
      destinationArchiveBytes: snapshot.archiveBytes,
      strictReopenValidated: true,
      residentDatasetId: authority.datasetId,
      residentDatasetFingerprint: authority.datasetFingerprint,
      residentModelId: authority.modelId,
      residentModelScientificSha256: authority.modelScientificSha256,
      residentRecipeId: authority.recipeId,
      residentRecipeDocumentSha256: authority.recipeDocumentSha256,
    },
    engine: {
      tolerance: recipe.settings.tolerance,
      maxIterations: recipe.settings.max_iterations,
      inference: inference.kind === "none" ? "none" : "percentile_case_bootstrap",
      bootstrapSamples: inference.kind === "none" ? 0 : inference.resamples,
      seed: recipe.settings.seed,
      workers: recipe.settings.workers,
      confidenceLevel: recipe.settings.confidence_level,
      maxMaterializedSpecificPaths: config.output_policy.max_materialized_specific_paths,
    },
    config,
    executionAccess,
    readAccess: {
      surface: recipeExecutionSurface === GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1
        ? "internal_labs"
        : "standard",
      experimentalLabsEnabled: false,
    },
    capabilityCell,
    recipeExecutionSurface,
    legacyLabsRecipeOnStandardCell,
  };
}

export type GeneralSemProjectBootstrapOutcomeV1 =
  | { status: "ok"; value: { schemaVersion: 1; receipt: GeneralSemProjectBootstrapReceiptV1 } }
  | { status: "blocked"; diagnostic: { code: string; message: string; correctiveAction: string } };

export interface GeneralSemNativePreflightOutcomeV1 {
  status: "ok";
  value: { schemaVersion: 1; pls: SemCapabilityDecisionV1; cbsem: SemCapabilityDecisionV1 };
}

export interface GeneralSemNativePreflightBlockedV1 {
  status: "blocked";
  diagnostic: { code: string; message: string; correctiveAction: string };
}

export type GeneralSemEstimatorPreflightOutcomeV1 = GeneralSemNativePreflightOutcomeV1 | GeneralSemNativePreflightBlockedV1;

export type GeneralSemPlsJobRequestV1 = GeneralSemExecutionAccessV1 & {
  archivePath: string;
  expectedArchiveSha256: string;
  projectId: string;
  datasetId: string;
  datasetFingerprint: string;
  modelId: string;
  modelScientificSha256: string;
  recipeId: string;
  recipeDocumentSha256: string;
  capabilityCell: CapabilityCellReferenceV2;
};

export type GeneralSemPlsJobStateV1 = "queued" | "running" | "cancelling" | "completed" | "failed" | "cancelled";
export interface GeneralSemPlsJobFailureV1 {
  schemaVersion: 1;
  stage: "access" | "archive_authority" | "capability" | "compilation" | "estimation" | "canonicalization" | "integrity";
  subject: string;
  code: string;
  message: string;
  correctiveAction: string;
  issues: Array<{ code: string; subject: string; message: string }>;
}

export interface GeneralSemPlsJobSnapshotV1 {
  schemaVersion: 1;
  jobId: string;
  state: GeneralSemPlsJobStateV1;
  phase: string;
  completedUnits: number;
  totalUnits: number;
  message: string | null;
  failure: GeneralSemPlsJobFailureV1 | null;
  queuedAt: string;
  startedAt: string | null;
  completedAt: string | null;
}

export interface GeneralSemArchiveIdentityV1 {
  archivePath: string;
  archiveSha256: string;
  projectId: string;
  datasetId: string;
  datasetFingerprint: string;
  modelId: string;
  modelScientificSha256: string;
  recipeId: string;
  recipeDocumentSha256: string;
}

export interface GeneralSemPlsCompletedResultV1 {
  schemaVersion: 1;
  archiveIdentity: GeneralSemArchiveIdentityV1;
  analyticalResult: unknown;
  canonicalDocument: CanonicalResultDocumentV2;
}

type UnknownRecord = Record<string, unknown>;
function recordAt(value: unknown, path: string): UnknownRecord {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new GeneralSemWorkspaceErrorV1(
    "general_sem.wire.object_required", path, `${path} must be an object.`, "Retry from the installed desktop application.",
  );
  return value as UnknownRecord;
}
function exactKeysAt(record: UnknownRecord, allowed: readonly string[], path: string): void {
  const allowedSet = new Set(allowed);
  const unexpected = Object.keys(record).filter((key) => !allowedSet.has(key)).sort();
  if (unexpected.length > 0) throw new GeneralSemWorkspaceErrorV1(
    "general_sem.wire.field_unknown",
    path,
    `${path} contains unknown fields: ${unexpected.join(", ")}.`,
    "Retry from the matching installed QuickPLS version.",
  );
}
function textAt(value: unknown, path: string): string {
  if (typeof value !== "string" || !value.trim()) throw new GeneralSemWorkspaceErrorV1(
    "general_sem.wire.text_required", path, `${path} must be nonempty text.`, "Retry from the installed desktop application.",
  );
  return value;
}
function uuidAt(value: unknown, path: string): string {
  const uuid = textAt(value, path);
  if (!UUID.test(uuid)) throw new GeneralSemWorkspaceErrorV1(
    "general_sem.wire.uuid_invalid", path, `${path} must be a lowercase UUID.`, "Retry from the installed desktop application.",
  );
  return uuid;
}
function timestampAt(value: unknown, path: string): string {
  const timestamp = textAt(value, path);
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/u.test(timestamp)
    || !Number.isFinite(Date.parse(timestamp))) throw new GeneralSemWorkspaceErrorV1(
      "general_sem.wire.timestamp_invalid", path, `${path} must be an RFC 3339 timestamp.`, "Retry from the installed desktop application.",
    );
  return timestamp;
}
function countAt(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) throw new GeneralSemWorkspaceErrorV1(
    "general_sem.wire.count_invalid", path, `${path} must be a nonnegative safe integer.`, "Retry from the installed desktop application.",
  );
  return value as number;
}
function digestAt(value: unknown, path: string): string {
  const digest = textAt(value, path);
  if (!SHA256.test(digest)) throw new GeneralSemWorkspaceErrorV1(
    "general_sem.wire.digest_invalid", path, `${path} must be a lowercase SHA-256 digest.`, "Retry from the installed desktop application.",
  );
  return digest;
}
function diagnosticAt(value: unknown, path: string) {
  const diagnostic = recordAt(value, path);
  exactKeysAt(diagnostic, ["code", "message", "correctiveAction"], path);
  return {
    code: textAt(diagnostic.code, `${path}.code`),
    message: textAt(diagnostic.message, `${path}.message`),
    correctiveAction: textAt(diagnostic.correctiveAction, `${path}.correctiveAction`),
  };
}

export function parseGeneralSemProjectBootstrapOutcomeV1(value: unknown): GeneralSemProjectBootstrapOutcomeV1 {
  const outcome = recordAt(value, "outcome");
  if (outcome.status === "blocked") {
    exactKeysAt(outcome, ["status", "diagnostic"], "outcome");
    return { status: "blocked", diagnostic: diagnosticAt(outcome.diagnostic, "outcome.diagnostic") };
  }
  if (outcome.status !== "ok") throw new GeneralSemWorkspaceErrorV1("general_sem.wire.status_invalid", "outcome.status", "Bootstrap outcome status is invalid.", "Retry from the installed desktop application.");
  exactKeysAt(outcome, ["status", "value"], "outcome");
  const body = recordAt(outcome.value, "outcome.value");
  const receipt = recordAt(body.receipt, "outcome.value.receipt");
  exactKeysAt(body, ["schemaVersion", "receipt"], "outcome.value");
  exactKeysAt(receipt, [
    "schemaVersion", "archiveSchemaVersion", "projectId", "name", "createdAt",
    "destinationArchivePath", "destinationArchiveSha256", "destinationArchiveBytes",
    "strictReopenValidated", "residentDatasetId", "residentDatasetFingerprint",
    "residentModelId", "residentModelScientificSha256", "residentRecipeId",
    "residentRecipeDocumentSha256",
  ], "outcome.value.receipt");
  if (body.schemaVersion !== 1 || receipt.schemaVersion !== 1 || receipt.archiveSchemaVersion !== 6 || receipt.strictReopenValidated !== true) {
    throw new GeneralSemWorkspaceErrorV1("general_sem.wire.bootstrap_contract_invalid", "outcome.value", "The native bootstrap receipt is not a strict schema-6 v1 receipt.", "Do not use the created file; retry after updating QuickPLS.");
  }
  return { status: "ok", value: { schemaVersion: 1, receipt: {
    schemaVersion: 1,
    archiveSchemaVersion: 6,
    projectId: uuidAt(receipt.projectId, "receipt.projectId"),
    name: textAt(receipt.name, "receipt.name"),
    createdAt: timestampAt(receipt.createdAt, "receipt.createdAt"),
    destinationArchivePath: textAt(receipt.destinationArchivePath, "receipt.destinationArchivePath"),
    destinationArchiveSha256: digestAt(receipt.destinationArchiveSha256, "receipt.destinationArchiveSha256"),
    destinationArchiveBytes: countAt(receipt.destinationArchiveBytes, "receipt.destinationArchiveBytes"),
    strictReopenValidated: true,
    residentDatasetId: textAt(receipt.residentDatasetId, "receipt.residentDatasetId"),
    residentDatasetFingerprint: textAt(receipt.residentDatasetFingerprint, "receipt.residentDatasetFingerprint"),
    residentModelId: textAt(receipt.residentModelId, "receipt.residentModelId"),
    residentModelScientificSha256: digestAt(receipt.residentModelScientificSha256, "receipt.residentModelScientificSha256"),
    residentRecipeId: textAt(receipt.residentRecipeId, "receipt.residentRecipeId"),
    residentRecipeDocumentSha256: digestAt(receipt.residentRecipeDocumentSha256, "receipt.residentRecipeDocumentSha256"),
  } } };
}

export function parseGeneralSemEstimatorPreflightOutcomeV1(value: unknown): GeneralSemEstimatorPreflightOutcomeV1 {
  const outcome = recordAt(value, "outcome");
  if (outcome.status === "blocked") {
    exactKeysAt(outcome, ["status", "diagnostic"], "outcome");
    return { status: "blocked", diagnostic: diagnosticAt(outcome.diagnostic, "outcome.diagnostic") };
  }
  if (outcome.status !== "ok") throw new GeneralSemWorkspaceErrorV1("general_sem.wire.status_invalid", "outcome.status", "Estimator preflight status is invalid.", "Retry from the installed desktop application.");
  exactKeysAt(outcome, ["status", "value"], "outcome");
  const body = recordAt(outcome.value, "outcome.value");
  exactKeysAt(body, ["schemaVersion", "pls", "cbsem"], "outcome.value");
  if (body.schemaVersion !== 1) throw new GeneralSemWorkspaceErrorV1("general_sem.wire.preflight_schema_invalid", "outcome.value.schemaVersion", "Estimator preflight requires schema version 1.", "Update QuickPLS and retry.");
  return { status: "ok", value: {
    schemaVersion: 1,
    pls: parseSemCapabilityDecisionV1(body.pls),
    cbsem: parseSemCapabilityDecisionV1(body.cbsem),
  } };
}

export function parseGeneralSemPlsJobSnapshotV1(value: unknown): GeneralSemPlsJobSnapshotV1 {
  const snapshot = recordAt(value, "snapshot");
  exactKeysAt(snapshot, [
    "schemaVersion", "jobId", "state", "phase", "completedUnits", "totalUnits",
    "message", "failure", "queuedAt", "startedAt", "completedAt",
  ], "snapshot");
  const states = ["queued", "running", "cancelling", "completed", "failed", "cancelled"] as const;
  if (snapshot.schemaVersion !== 1 || !states.includes(snapshot.state as GeneralSemPlsJobStateV1)) throw new GeneralSemWorkspaceErrorV1(
    "general_sem.wire.job_snapshot_invalid", "snapshot", "The General SEM job snapshot contract is invalid.", "Dismiss the job and retry after updating QuickPLS.",
  );
  const completedUnits = countAt(snapshot.completedUnits, "snapshot.completedUnits");
  const totalUnits = countAt(snapshot.totalUnits, "snapshot.totalUnits");
  if (completedUnits > totalUnits) throw new GeneralSemWorkspaceErrorV1(
    "general_sem.wire.job_progress_invalid", "snapshot.completedUnits", "Completed job units cannot exceed total units.", "Dismiss the job and retry after updating QuickPLS.",
  );
  const failure = snapshot.failure === null ? null : (() => {
    const item = recordAt(snapshot.failure, "snapshot.failure");
    exactKeysAt(item, ["schemaVersion", "stage", "subject", "code", "message", "correctiveAction", "issues"], "snapshot.failure");
    const stages: readonly GeneralSemPlsJobFailureV1["stage"][] = ["access", "archive_authority", "capability", "compilation", "estimation", "canonicalization", "integrity"];
    if (item.schemaVersion !== 1 || !stages.includes(item.stage as GeneralSemPlsJobFailureV1["stage"])) throw new GeneralSemWorkspaceErrorV1(
      "general_sem.wire.job_failure_invalid", "snapshot.failure", "The General SEM job failure contract is invalid.", "Retry from the installed desktop application.",
    );
    return {
      schemaVersion: 1 as const,
      stage: textAt(item.stage, "snapshot.failure.stage") as GeneralSemPlsJobFailureV1["stage"],
      subject: textAt(item.subject, "snapshot.failure.subject"),
      code: textAt(item.code, "snapshot.failure.code"),
      message: textAt(item.message, "snapshot.failure.message"),
      correctiveAction: textAt(item.correctiveAction, "snapshot.failure.correctiveAction"),
      issues: Array.isArray(item.issues) ? item.issues.map((raw, index) => {
        const nested = recordAt(raw, `snapshot.failure.issues[${index}]`);
        exactKeysAt(nested, ["code", "subject", "message"], `snapshot.failure.issues[${index}]`);
        return { code: textAt(nested.code, `snapshot.failure.issues[${index}].code`), subject: textAt(nested.subject, `snapshot.failure.issues[${index}].subject`), message: textAt(nested.message, `snapshot.failure.issues[${index}].message`) };
      }) : [],
    };
  })();
  return {
    schemaVersion: 1,
    jobId: textAt(snapshot.jobId, "snapshot.jobId"),
    state: snapshot.state as GeneralSemPlsJobStateV1,
    phase: textAt(snapshot.phase, "snapshot.phase"),
    completedUnits,
    totalUnits,
    message: snapshot.message === null ? null : textAt(snapshot.message, "snapshot.message"),
    failure,
    queuedAt: timestampAt(snapshot.queuedAt, "snapshot.queuedAt"),
    startedAt: snapshot.startedAt === null ? null : timestampAt(snapshot.startedAt, "snapshot.startedAt"),
    completedAt: snapshot.completedAt === null ? null : timestampAt(snapshot.completedAt, "snapshot.completedAt"),
  };
}

export function parseGeneralSemPlsCompletedResultV1(value: unknown): GeneralSemPlsCompletedResultV1 {
  const completed = recordAt(value, "completed");
  exactKeysAt(completed, ["schemaVersion", "archiveIdentity", "analyticalResult", "canonicalDocument"], "completed");
  const archive = recordAt(completed.archiveIdentity, "completed.archiveIdentity");
  exactKeysAt(archive, [
    "archivePath", "archiveSha256", "projectId", "datasetId", "datasetFingerprint",
    "modelId", "modelScientificSha256", "recipeId", "recipeDocumentSha256",
  ], "completed.archiveIdentity");
  const canonicalDocument = completed.canonicalDocument as CanonicalResultDocumentV2;
  const validation = validateCanonicalResultDocumentV2(canonicalDocument);
  if (!validation.passed || !canonicalDocument.general_sem_results) throw new GeneralSemWorkspaceErrorV1(
    "general_sem.wire.canonical_invalid", "completed.canonicalDocument", validation.errors.join("; ") || "The completed document has no General SEM result section.", "Discard this completed job and retry after updating QuickPLS.",
  );
  if (completed.schemaVersion !== 1) throw new GeneralSemWorkspaceErrorV1("general_sem.wire.result_schema_invalid", "completed.schemaVersion", "Completed General SEM result requires schema version 1.", "Update QuickPLS and retry.");
  const archiveIdentity: GeneralSemArchiveIdentityV1 = {
    archivePath: textAt(archive.archivePath, "archiveIdentity.archivePath"),
    archiveSha256: digestAt(archive.archiveSha256, "archiveIdentity.archiveSha256"),
    projectId: uuidAt(archive.projectId, "archiveIdentity.projectId"),
    datasetId: textAt(archive.datasetId, "archiveIdentity.datasetId"),
    datasetFingerprint: textAt(archive.datasetFingerprint, "archiveIdentity.datasetFingerprint"),
    modelId: textAt(archive.modelId, "archiveIdentity.modelId"),
    modelScientificSha256: digestAt(archive.modelScientificSha256, "archiveIdentity.modelScientificSha256"),
    recipeId: textAt(archive.recipeId, "archiveIdentity.recipeId"),
    recipeDocumentSha256: digestAt(archive.recipeDocumentSha256, "archiveIdentity.recipeDocumentSha256"),
  };
  if (canonicalDocument.provenance.project_id !== archiveIdentity.projectId
    || canonicalDocument.provenance.dataset_id !== archiveIdentity.datasetId
    || canonicalDocument.provenance.dataset_fingerprint !== archiveIdentity.datasetFingerprint
    || canonicalDocument.provenance.model_id !== archiveIdentity.modelId
    || canonicalDocument.provenance.model_digest !== archiveIdentity.modelScientificSha256
    || canonicalDocument.provenance.recipe_id !== archiveIdentity.recipeId) throw new GeneralSemWorkspaceErrorV1(
      "general_sem.wire.result_authority_mismatch", "completed.canonicalDocument.provenance", "The canonical result differs from the archive authority returned by the job.", "Discard the job; do not append this result.",
    );
  const analytical = recordAt(completed.analyticalResult, "completed.analyticalResult");
  if (Object.prototype.hasOwnProperty.call(analytical, "moderation_bootstrap_inference")
    && analytical.moderation_bootstrap_inference !== null) {
    validateCompletedModerationBootstrapInferenceV1({
      value: analytical.moderation_bootstrap_inference,
      analytical,
      interactionPoint: recordAt(
        analytical.interaction_point_estimation,
        "completed.analyticalResult.interaction_point_estimation",
      ),
      generalSemResults: recordAt(
        canonicalDocument.general_sem_results,
        "completed.canonicalDocument.general_sem_results",
      ),
    });
  }
  return { schemaVersion: 1, archiveIdentity, analyticalResult: completed.analyticalResult, canonicalDocument };
}

type GeneralSemCapabilityCellIdentityV1 = {
  readonly registry_schema_version: number;
  readonly capability_id: string;
  readonly cell_id: string;
  readonly capability_version: string;
};

function sameCapabilityCellV1(
  left: GeneralSemCapabilityCellIdentityV1,
  right: GeneralSemCapabilityCellIdentityV1,
): boolean {
  return left.registry_schema_version === right.registry_schema_version
    && left.capability_id === right.capability_id
    && left.cell_id === right.cell_id
    && left.capability_version === right.capability_version;
}

function compareCapabilityCellsV1(
  left: GeneralSemCapabilityCellIdentityV1,
  right: GeneralSemCapabilityCellIdentityV1,
): number {
  return left.registry_schema_version - right.registry_schema_version
    || compareUtf8StringsV1(left.capability_id, right.capability_id)
    || compareUtf8StringsV1(left.cell_id, right.cell_id)
    || compareUtf8StringsV1(left.capability_version, right.capability_version);
}

export type GeneralSemPlsExecutionKindV1 =
  | "mediation_point"
  | "mediation_bootstrap"
  | "multiple_two_way_moderation_point"
  | "multiple_two_way_moderation_bootstrap";

export interface GeneralSemPlsExecutionCapabilityV1 {
  readonly kind: GeneralSemPlsExecutionKindV1;
  readonly capabilityCell: CapabilityCellReferenceV2;
  readonly interactionIds: readonly string[];
  readonly focalRelationIds: readonly string[];
}

export function generalSemPlsRequestedCapabilityCellV1(
  model: SemModelV4,
  config: GeneralSemConfigV1,
): CapabilityCellReferenceV2 {
  const moderation = model.derived_terms.some((term) => term.kind === "interaction_v2");
  return moderation
    ? config.inference.kind === "case_bootstrap"
      ? GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1
      : GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1
    : config.inference.kind === "case_bootstrap"
      ? GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1
      : GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1;
}

interface GeneralSemCompletedInteractionIdentityV1 {
  readonly interactionId: string;
  readonly focalRelationId: string;
}

function completedExecutionIdentityV1(value: unknown, path: string): string {
  if (typeof value !== "string" || !value.trim() || value.trim() !== value) {
    completedExecutionMismatchV1(path, `${path} must be a nonempty identity without surrounding whitespace.`);
  }
  return value;
}

function exactSortedDistinctExecutionInventoryV1(
  values: readonly string[],
  path: string,
): readonly string[] {
  if (!Array.isArray(values)) {
    completedExecutionMismatchV1(path, `${path} must be an identity array.`);
  }
  const parsed = values.map((value, index) => completedExecutionIdentityV1(value, `${path}[${index}]`));
  const sorted = [...parsed].sort(compareUtf8StringsV1);
  if (new Set(sorted).size !== sorted.length
    || sorted.some((value, index) => value !== parsed[index])) {
    completedExecutionMismatchV1(path, `${path} must be canonically sorted and distinct.`);
  }
  return sorted;
}

function completedInteractionIdentitiesV1(
  value: unknown,
  path: string,
): readonly GeneralSemCompletedInteractionIdentityV1[] {
  if (!Array.isArray(value)) {
    completedExecutionMismatchV1(path, `${path} must be an interaction identity array.`);
  }
  const identities = value.map((entry, index) => {
    const entryPath = `${path}[${index}]`;
    const record = entry && typeof entry === "object" && !Array.isArray(entry)
      ? entry as UnknownRecord
      : completedExecutionMismatchV1(entryPath, `${entryPath} must be an interaction identity object.`);
    return {
      interactionId: completedExecutionIdentityV1(record.interaction_id, `${entryPath}.interaction_id`),
      focalRelationId: completedExecutionIdentityV1(record.focal_relation_id, `${entryPath}.focal_relation_id`),
    };
  });
  const interactionIds = identities.map((identity) => identity.interactionId);
  if (new Set(interactionIds).size !== interactionIds.length) {
    completedExecutionMismatchV1(path, `${path} contains a duplicated interaction identity.`);
  }
  return identities;
}

function sortedDistinctIdentityInventoryV1(values: readonly string[]): readonly string[] {
  return [...new Set(values)].sort(compareUtf8StringsV1);
}

function requireSameCompletedInventoryV1(
  observed: readonly string[],
  expected: readonly string[],
  path: string,
): void {
  const canonicalObserved = sortedDistinctIdentityInventoryV1(observed);
  if (canonicalObserved.length !== expected.length
    || canonicalObserved.some((value, index) => value !== expected[index])) {
    completedExecutionMismatchV1(path, `${path} differs from the exact current compiled execution inventory.`);
  }
}

function requireSameCompletedInteractionMappingV1(
  left: readonly GeneralSemCompletedInteractionIdentityV1[],
  right: readonly GeneralSemCompletedInteractionIdentityV1[],
  path: string,
): void {
  const canonical = (values: readonly GeneralSemCompletedInteractionIdentityV1[]) => values
    .map((identity) => `${identity.interactionId}\u0000${identity.focalRelationId}`)
    .sort(compareUtf8StringsV1);
  const canonicalLeft = canonical(left);
  const canonicalRight = canonical(right);
  if (canonicalLeft.length !== canonicalRight.length
    || canonicalLeft.some((value, index) => value !== canonicalRight[index])) {
    completedExecutionMismatchV1(path, "The analytical and canonical interaction-to-focal identities differ.");
  }
}

/**
 * Chooses a runnable cell only from the exact native estimator decision and the
 * unchanged compiled-graph shape. A config toggle alone can never promote a
 * capability, and a stale/mismatched native preflight fails closed.
 */
export function selectGeneralSemPlsExecutionCapabilityV1(input: {
  model: SemModelV4;
  config: GeneralSemConfigV1;
  decision: SemCapabilityDecisionV1;
}): GeneralSemPlsExecutionCapabilityV1 {
  const interactionTerms = input.model.derived_terms
    .filter((term): term is Extract<SemModelV4["derived_terms"][number], { kind: "interaction_v2" }> => (
      term.kind === "interaction_v2"
    ))
    .slice()
    .sort((left, right) => compareUtf8StringsV1(left.id, right.id));
  const moderation = interactionTerms.length > 0;
  const capabilityCell = generalSemPlsRequestedCapabilityCellV1(input.model, input.config);
  if (input.decision.estimator_id !== "qpls.pls_sem.v3"
    || (input.decision.status !== "experimental" && input.decision.status !== "supported")) {
    throw new GeneralSemWorkspaceErrorV1(
      "general_sem.capability.native_preflight_not_runnable",
      input.decision.estimator_id,
      "The exact native PLS estimator preflight is not runnable for the current model and resident RecipeV4.",
      "Keep the project unchanged, apply the native corrective diagnostics, and rerun preflight before calculation.",
    );
  }
  const expectedCapabilityCells = [
    moderation ? GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1 : GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1,
    ...(input.config.inference.kind === "case_bootstrap"
      ? [moderation
        ? GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1
        : GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1]
      : []),
  ].sort(compareCapabilityCellsV1);
  if (input.decision.capability_cells.length !== expectedCapabilityCells.length
    || input.decision.capability_cells.some((cell, index) => (
      !sameCapabilityCellV1(cell, expectedCapabilityCells[index]!)
    ))) {
    throw new GeneralSemWorkspaceErrorV1(
      "general_sem.capability.native_preflight_cell_mismatch",
      capabilityCell.cell_id,
      "The native PLS preflight capability-cell set or canonical UTF-8 order differs from the current compiled graph and inference config.",
      "Do not calculate from this stale authority. Reopen the exact marked project and rerun native estimator preflight.",
    );
  }
  return {
    kind: moderation
      ? input.config.inference.kind === "case_bootstrap"
        ? "multiple_two_way_moderation_bootstrap"
        : "multiple_two_way_moderation_point"
      : input.config.inference.kind === "case_bootstrap"
        ? "mediation_bootstrap"
        : "mediation_point",
    capabilityCell,
    interactionIds: interactionTerms.map((term) => term.id),
    focalRelationIds: [...new Set(interactionTerms.map((term) => term.focal_relation))]
      .sort(compareUtf8StringsV1),
  };
}

interface GeneralSemCompletedModerationGammaTargetV1 {
  readonly kind: "interaction_scientific_rescaled_gamma";
  readonly target_version: typeof GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1;
  readonly target_id: string;
  readonly interaction_id: string;
  readonly focal_relation_id: string;
  readonly interaction_effect_relation_id: string;
  readonly interaction_effect_parameter_id: string;
  readonly generated_product_column_id: string;
  readonly focal_predictor_id: string;
  readonly moderator_id: string;
  readonly outcome_id: string;
  readonly stage_one_model_scientific_sha256: string;
  readonly product_scale_version: typeof GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1;
  readonly method_version: typeof GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1;
}

function completedRecordAtV1(value: unknown, path: string): UnknownRecord {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    completedExecutionMismatchV1(path, `${path} must be an object.`);
  }
  return value as UnknownRecord;
}

function completedArrayAtV1(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) completedExecutionMismatchV1(path, `${path} must be an array.`);
  return value;
}

function completedExactKeysAtV1(record: UnknownRecord, expected: readonly string[], path: string): void {
  const actual = Object.keys(record).sort(compareUtf8StringsV1);
  const canonicalExpected = [...expected].sort(compareUtf8StringsV1);
  if (actual.length !== canonicalExpected.length
    || actual.some((key, index) => key !== canonicalExpected[index])) {
    completedExecutionMismatchV1(path, `${path} must contain exactly the moderation-bootstrap v1 fields.`);
  }
}

function completedFiniteAtV1(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    completedExecutionMismatchV1(path, `${path} must be a finite number.`);
  }
  return value;
}

function completedCountAtV1(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    completedExecutionMismatchV1(path, `${path} must be a nonnegative safe integer.`);
  }
  return value as number;
}

function completedTrueAtV1(value: unknown, path: string): void {
  if (value !== true) completedExecutionMismatchV1(path, `${path} must be true.`);
}

function completedModerationGammaTargetAtV1(
  value: unknown,
  path: string,
): GeneralSemCompletedModerationGammaTargetV1 {
  const target = completedRecordAtV1(value, path);
  completedExactKeysAtV1(target, [
    "kind", "target_version", "target_id", "interaction_id", "focal_relation_id",
    "interaction_effect_relation_id", "interaction_effect_parameter_id",
    "generated_product_column_id", "focal_predictor_id", "moderator_id", "outcome_id",
    "stage_one_model_scientific_sha256", "product_scale_version", "method_version",
  ], path);
  const parsed = {
    kind: completedExecutionIdentityV1(target.kind, `${path}.kind`),
    target_version: completedExecutionIdentityV1(target.target_version, `${path}.target_version`),
    target_id: completedExecutionIdentityV1(target.target_id, `${path}.target_id`),
    interaction_id: completedExecutionIdentityV1(target.interaction_id, `${path}.interaction_id`),
    focal_relation_id: completedExecutionIdentityV1(target.focal_relation_id, `${path}.focal_relation_id`),
    interaction_effect_relation_id: completedExecutionIdentityV1(
      target.interaction_effect_relation_id,
      `${path}.interaction_effect_relation_id`,
    ),
    interaction_effect_parameter_id: completedExecutionIdentityV1(
      target.interaction_effect_parameter_id,
      `${path}.interaction_effect_parameter_id`,
    ),
    generated_product_column_id: completedExecutionIdentityV1(
      target.generated_product_column_id,
      `${path}.generated_product_column_id`,
    ),
    focal_predictor_id: completedExecutionIdentityV1(target.focal_predictor_id, `${path}.focal_predictor_id`),
    moderator_id: completedExecutionIdentityV1(target.moderator_id, `${path}.moderator_id`),
    outcome_id: completedExecutionIdentityV1(target.outcome_id, `${path}.outcome_id`),
    stage_one_model_scientific_sha256: digestAt(
      target.stage_one_model_scientific_sha256,
      `${path}.stage_one_model_scientific_sha256`,
    ),
    product_scale_version: completedExecutionIdentityV1(
      target.product_scale_version,
      `${path}.product_scale_version`,
    ),
    method_version: completedExecutionIdentityV1(target.method_version, `${path}.method_version`),
  };
  if (parsed.kind !== "interaction_scientific_rescaled_gamma"
    || parsed.target_version !== GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1
    || parsed.target_id !== parsed.interaction_effect_relation_id
    || parsed.product_scale_version !== GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1
    || parsed.method_version !== GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1) {
    completedExecutionMismatchV1(path, `${path} is not the exact compiled scientific gamma target identity.`);
  }
  return parsed as GeneralSemCompletedModerationGammaTargetV1;
}

function canonicalModerationGammaIdentityV1(effect: UnknownRecord, path: string) {
  return {
    kind: "interaction_scientific_rescaled_gamma" as const,
    effect_id: completedExecutionIdentityV1(effect.effect_id, `${path}.effect_id`),
    interaction_id: completedExecutionIdentityV1(effect.interaction_id, `${path}.interaction_id`),
    focal_relation_id: completedExecutionIdentityV1(effect.focal_relation_id, `${path}.focal_relation_id`),
    interaction_effect_relation_id: completedExecutionIdentityV1(
      effect.interaction_effect_relation_id,
      `${path}.interaction_effect_relation_id`,
    ),
    interaction_effect_parameter_id: completedExecutionIdentityV1(
      effect.interaction_effect_parameter_id,
      `${path}.interaction_effect_parameter_id`,
    ),
    generated_product_column_id: completedExecutionIdentityV1(
      effect.generated_product_column_id,
      `${path}.generated_product_column_id`,
    ),
    focal_predictor_id: completedExecutionIdentityV1(effect.focal_predictor_id, `${path}.focal_predictor_id`),
    moderator_id: completedExecutionIdentityV1(effect.moderator_id, `${path}.moderator_id`),
    outcome_id: completedExecutionIdentityV1(effect.outcome_id, `${path}.outcome_id`),
    stage_one_model_scientific_sha256: digestAt(
      effect.stage_one_model_scientific_sha256,
      `${path}.stage_one_model_scientific_sha256`,
    ),
    product_scale_version: completedExecutionIdentityV1(
      effect.product_scale_version,
      `${path}.product_scale_version`,
    ),
    method_version: completedExecutionIdentityV1(effect.method_version, `${path}.method_version`),
  };
}

function validateCompletedModerationBootstrapInferenceV1(input: {
  value: unknown;
  analytical: UnknownRecord;
  interactionPoint: UnknownRecord;
  generalSemResults: UnknownRecord;
}): void {
  const path = "completed.analyticalResult.moderation_bootstrap_inference";
  const bootstrap = completedRecordAtV1(input.value, path);
  completedExactKeysAtV1(bootstrap, [
    "schema_version", "method_version", "point_method_version", "resampling_operation_version",
    "resampling_stream_version", "quantile_method_version", "standard_error_method_version",
    "summation_method_version", "p_value_method_version", "failure_policy_version",
    "sign_alignment_method_version", "product_scale_version", "gamma_target_version",
    "general_sem_config_sha256", "compiled_plan_sha256", "model_scientific_sha256",
    "stage_one_model_scientific_sha256", "source_dataset_fingerprint",
    "complete_case_frame_sha256", "usable_replicate_indices_sha256",
    "gamma_target_identity_set_sha256", "gamma_target_ids", "interval", "tail",
    "confidence_level", "resamples_requested", "resamples_usable",
    "minimum_usable_resamples", "seed", "workers",
    "complete_model_reestimated_per_replicate",
    "shared_stage_one_reestimated_per_replicate",
    "score_vectors_sign_aligned_before_products",
    "product_scaling_recomputed_per_replicate",
    "joint_stage_two_reestimated_per_replicate",
    "complete_joint_point_contract_validated_per_replicate",
    "failed_replicates", "interaction_gammas",
  ], path);
  const exactVersions = {
    method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
    point_method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
    resampling_operation_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1,
    resampling_stream_version: GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1,
    quantile_method_version: GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1,
    standard_error_method_version: GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1,
    summation_method_version: GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1,
    p_value_method_version: GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1,
    failure_policy_version: GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1,
    sign_alignment_method_version: GENERAL_SEM_PLS_MULTIPLE_MODERATION_SIGN_ALIGNMENT_VERSION_V1,
    product_scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
    gamma_target_version: GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1,
  } as const;
  if (bootstrap.schema_version !== 1) {
    completedExecutionMismatchV1(`${path}.schema_version`, `${path}.schema_version must equal 1.`);
  }
  for (const [field, expected] of Object.entries(exactVersions)) {
    if (bootstrap[field] !== expected) {
      completedExecutionMismatchV1(`${path}.${field}`, `${path}.${field} must equal ${expected}.`);
    }
  }
  for (const field of [
    "general_sem_config_sha256", "compiled_plan_sha256", "model_scientific_sha256",
    "stage_one_model_scientific_sha256", "complete_case_frame_sha256",
    "usable_replicate_indices_sha256", "gamma_target_identity_set_sha256",
  ] as const) digestAt(bootstrap[field], `${path}.${field}`);
  if (bootstrap.general_sem_config_sha256 !== input.analytical.general_sem_config_sha256
    || bootstrap.compiled_plan_sha256 !== input.analytical.compiled_plan_sha256
    || bootstrap.model_scientific_sha256 !== input.analytical.model_scientific_sha256
    || bootstrap.stage_one_model_scientific_sha256 !== input.analytical.stage_one_model_scientific_sha256
    || bootstrap.source_dataset_fingerprint !== input.analytical.source_dataset_fingerprint) {
    completedExecutionMismatchV1(path, `${path} provenance differs from the compiled analytical result.`);
  }
  if (bootstrap.model_scientific_sha256 === bootstrap.stage_one_model_scientific_sha256) {
    completedExecutionMismatchV1(
      `${path}.stage_one_model_scientific_sha256`,
      `${path} requires a distinct interaction-free stage-one model digest.`,
    );
  }
  if (bootstrap.interval !== "percentile" || bootstrap.tail !== "two_sided") {
    completedExecutionMismatchV1(path, `${path} must use percentile two-sided inference.`);
  }
  const confidenceLevel = completedFiniteAtV1(bootstrap.confidence_level, `${path}.confidence_level`);
  if (confidenceLevel <= 0 || confidenceLevel >= 1) {
    completedExecutionMismatchV1(`${path}.confidence_level`, `${path}.confidence_level is outside (0, 1).`);
  }
  const requested = completedCountAtV1(bootstrap.resamples_requested, `${path}.resamples_requested`);
  const usable = completedCountAtV1(bootstrap.resamples_usable, `${path}.resamples_usable`);
  const minimumUsable = completedCountAtV1(
    bootstrap.minimum_usable_resamples,
    `${path}.minimum_usable_resamples`,
  );
  if (requested < 2 || requested > 10_000
    || minimumUsable !== Math.max(2, Math.ceil(requested * 0.9))
    || usable < minimumUsable
    || usable > requested) {
    completedExecutionMismatchV1(path, `${path} violates the exact resample plan or 90 percent usable gate.`);
  }
  const seed = completedExecutionIdentityV1(bootstrap.seed, `${path}.seed`);
  if (!/^(?:0|[1-9][0-9]*)$/u.test(seed) || BigInt(seed) > BigInt(Number.MAX_SAFE_INTEGER)) {
    completedExecutionMismatchV1(`${path}.seed`, `${path}.seed must be a canonical JavaScript-safe decimal integer.`);
  }
  const workers = completedCountAtV1(bootstrap.workers, `${path}.workers`);
  if (workers < 1 || workers > 64) {
    completedExecutionMismatchV1(`${path}.workers`, `${path}.workers must be between 1 and 64.`);
  }
  for (const field of [
    "complete_model_reestimated_per_replicate",
    "shared_stage_one_reestimated_per_replicate",
    "score_vectors_sign_aligned_before_products",
    "product_scaling_recomputed_per_replicate",
    "joint_stage_two_reestimated_per_replicate",
    "complete_joint_point_contract_validated_per_replicate",
  ] as const) completedTrueAtV1(bootstrap[field], `${path}.${field}`);

  const failures = completedArrayAtV1(bootstrap.failed_replicates, `${path}.failed_replicates`);
  const failureReasons = new Set([
    "insufficient_observations", "constant_indicator", "stage_one_rank_deficient",
    "isolated_construct", "stage_one_nonconvergence", "indeterminate_score_sign",
    "constant_construct_score", "constant_interaction_product", "joint_stage_rank_deficient",
    "numerical_failure",
  ]);
  const failedIndices = new Set<number>();
  let previousFailureIndex = -1;
  failures.forEach((value, index) => {
    const failurePath = `${path}.failed_replicates[${index}]`;
    const failure = completedRecordAtV1(value, failurePath);
    completedExactKeysAtV1(failure, ["replicate_index", "reason_code", "message"], failurePath);
    const replicateIndex = completedCountAtV1(failure.replicate_index, `${failurePath}.replicate_index`);
    const reason = completedExecutionIdentityV1(failure.reason_code, `${failurePath}.reason_code`);
    completedExecutionIdentityV1(failure.message, `${failurePath}.message`);
    if (replicateIndex >= requested || replicateIndex <= previousFailureIndex || !failureReasons.has(reason)) {
      completedExecutionMismatchV1(failurePath, `${failurePath} is not a canonical moderation failure entry.`);
    }
    previousFailureIndex = replicateIndex;
    failedIndices.add(replicateIndex);
  });
  if (usable + failures.length !== requested) {
    completedExecutionMismatchV1(path, `${path} usable and failed ledgers do not cover the requested plan.`);
  }
  const usableIndices = Array.from({ length: requested }, (_, index) => index)
    .filter((index) => !failedIndices.has(index));
  if (usableIndices.length !== usable
    || bootstrap.usable_replicate_indices_sha256 !== sha256HexUtf8V1(JSON.stringify(usableIndices))) {
    completedExecutionMismatchV1(
      `${path}.usable_replicate_indices_sha256`,
      `${path}.usable_replicate_indices_sha256 contradicts the failure ledger.`,
    );
  }

  const targetIds = exactSortedDistinctExecutionInventoryV1(
    completedArrayAtV1(bootstrap.gamma_target_ids, `${path}.gamma_target_ids`) as string[],
    `${path}.gamma_target_ids`,
  );
  if (targetIds.length === 0) {
    completedExecutionMismatchV1(`${path}.gamma_target_ids`, `${path}.gamma_target_ids must not be empty.`);
  }
  const canonicalEffects = completedArrayAtV1(
    input.generalSemResults.interaction_effects,
    "completed.canonicalDocument.general_sem_results.interaction_effects",
  ).map((value, index) => completedRecordAtV1(
    value,
    `completed.canonicalDocument.general_sem_results.interaction_effects[${index}]`,
  ));
  const canonicalById = new Map(canonicalEffects.map((effect, index) => [
    completedExecutionIdentityV1(
      effect.effect_id,
      `completed.canonicalDocument.general_sem_results.interaction_effects[${index}].effect_id`,
    ),
    { effect, index },
  ]));
  const pointCoefficients = completedArrayAtV1(
    input.interactionPoint.interaction_coefficients,
    "completed.analyticalResult.interaction_point_estimation.interaction_coefficients",
  ).map((value, index) => completedRecordAtV1(
    value,
    `completed.analyticalResult.interaction_point_estimation.interaction_coefficients[${index}]`,
  ));
  const pointByTargetId = new Map(pointCoefficients.map((coefficient, index) => [
    completedExecutionIdentityV1(
      coefficient.interaction_effect_relation_id,
      `completed.analyticalResult.interaction_point_estimation.interaction_coefficients[${index}].interaction_effect_relation_id`,
    ),
    { coefficient, index },
  ]));
  const gammaRows = completedArrayAtV1(bootstrap.interaction_gammas, `${path}.interaction_gammas`);
  if (gammaRows.length !== targetIds.length || canonicalById.size !== targetIds.length) {
    completedExecutionMismatchV1(path, `${path} must exactly cover the canonical interaction gamma inventory.`);
  }
  const rawTargets: GeneralSemCompletedModerationGammaTargetV1[] = [];
  gammaRows.forEach((value, index) => {
    const rowPath = `${path}.interaction_gammas[${index}]`;
    const row = completedRecordAtV1(value, rowPath);
    completedExactKeysAtV1(row, [
      "target", "original", "bootstrap_mean", "bootstrap_bias", "standard_error",
      "lower", "upper", "p_value_two_sided", "usable_replicates", "two_sided_exceedances",
    ], rowPath);
    const target = completedModerationGammaTargetAtV1(row.target, `${rowPath}.target`);
    rawTargets.push(target);
    if (target.target_id !== targetIds[index]) {
      completedExecutionMismatchV1(`${rowPath}.target.target_id`, `${rowPath} is outside canonical target order.`);
    }
    const canonical = canonicalById.get(target.target_id);
    const point = pointByTargetId.get(target.target_id);
    if (!canonical || !point) {
      completedExecutionMismatchV1(rowPath, `${rowPath} has no canonical and point-estimation authority.`);
    }
    const canonicalPath = `completed.canonicalDocument.general_sem_results.interaction_effects[${canonical.index}]`;
    const canonicalIdentity = canonicalModerationGammaIdentityV1(canonical.effect, canonicalPath);
    const pointPath = `completed.analyticalResult.interaction_point_estimation.interaction_coefficients[${point.index}]`;
    const matchingFields = [
      "interaction_id", "focal_relation_id", "interaction_effect_relation_id",
      "interaction_effect_parameter_id", "focal_predictor_id", "moderator_id", "outcome_id",
    ] as const;
    if (canonicalIdentity.effect_id !== target.target_id
      || canonicalIdentity.generated_product_column_id !== target.generated_product_column_id
      || canonicalIdentity.stage_one_model_scientific_sha256 !== target.stage_one_model_scientific_sha256
      || canonicalIdentity.product_scale_version !== target.product_scale_version
      || canonicalIdentity.method_version !== target.method_version
      || matchingFields.some((field) => canonicalIdentity[field] !== target[field])
      || matchingFields.some((field) => completedExecutionIdentityV1(
        point.coefficient[field],
        `${pointPath}.${field}`,
      ) !== target[field])) {
      completedExecutionMismatchV1(rowPath, `${rowPath} target differs from its point and canonical interaction authority.`);
    }
    const original = completedFiniteAtV1(row.original, `${rowPath}.original`);
    const bootstrapMean = completedFiniteAtV1(row.bootstrap_mean, `${rowPath}.bootstrap_mean`);
    const bootstrapBias = completedFiniteAtV1(row.bootstrap_bias, `${rowPath}.bootstrap_bias`);
    const standardError = completedFiniteAtV1(row.standard_error, `${rowPath}.standard_error`);
    const lower = completedFiniteAtV1(row.lower, `${rowPath}.lower`);
    const upper = completedFiniteAtV1(row.upper, `${rowPath}.upper`);
    const pValue = completedFiniteAtV1(row.p_value_two_sided, `${rowPath}.p_value_two_sided`);
    const rowUsable = completedCountAtV1(row.usable_replicates, `${rowPath}.usable_replicates`);
    const exceedances = completedCountAtV1(row.two_sided_exceedances, `${rowPath}.two_sided_exceedances`);
    const canonicalGamma = completedRecordAtV1(
      canonical.effect.scientific_rescaled_gamma,
      `${canonicalPath}.scientific_rescaled_gamma`,
    );
    const canonicalValues = [
      ["estimate", original], ["bootstrap_mean", bootstrapMean], ["bootstrap_bias", bootstrapBias],
      ["standard_error", standardError], ["lower", lower], ["upper", upper],
      ["p_value", pValue], ["bootstrap_usable_replicates", rowUsable],
      ["bootstrap_two_sided_exceedances", exceedances],
    ] as const;
    if (standardError < 0 || lower > upper || rowUsable !== usable || exceedances > rowUsable
      || Math.abs(bootstrapBias - (bootstrapMean - original)) > Number.EPSILON * 8 * Math.max(1, Math.abs(bootstrapBias))
      || Math.abs(pValue - ((exceedances + 1) / (rowUsable + 1))) > Number.EPSILON * 8
      || completedFiniteAtV1(point.coefficient.raw_product_estimate, `${pointPath}.raw_product_estimate`) !== original
      || canonicalValues.some(([field, expected]) => canonicalGamma[field] !== expected)) {
      completedExecutionMismatchV1(rowPath, `${rowPath} inference differs from its point estimate, ledger, or canonical gamma row.`);
    }
  });
  if (bootstrap.gamma_target_identity_set_sha256 !== sha256HexUtf8V1(JSON.stringify(rawTargets))) {
    completedExecutionMismatchV1(
      `${path}.gamma_target_identity_set_sha256`,
      `${path}.gamma_target_identity_set_sha256 contradicts the typed raw target ledger.`,
    );
  }

  const receiptPath = "completed.canonicalDocument.general_sem_results.inference_receipt";
  const receipt = completedRecordAtV1(input.generalSemResults.inference_receipt, receiptPath);
  const receiptCell = capabilityCellAtV1(receipt.capability_cell, `${receiptPath}.capability_cell`);
  const canonicalIdentities = canonicalEffects
    .map((effect, index) => canonicalModerationGammaIdentityV1(
      effect,
      `completed.canonicalDocument.general_sem_results.interaction_effects[${index}]`,
    ))
    .sort((left, right) => compareUtf8StringsV1(left.effect_id, right.effect_id));
  const exactReceiptValues = [
    ["method_version", bootstrap.method_version],
    ["resampling_operation_version", bootstrap.resampling_operation_version],
    ["resampling_stream_version", bootstrap.resampling_stream_version],
    ["quantile_method_version", bootstrap.quantile_method_version],
    ["standard_error_method_version", bootstrap.standard_error_method_version],
    ["summation_method_version", bootstrap.summation_method_version],
    ["p_value_method_version", bootstrap.p_value_method_version],
    ["failure_policy_version", bootstrap.failure_policy_version],
    ["compiled_plan_sha256", bootstrap.compiled_plan_sha256],
    ["general_sem_config_sha256", bootstrap.general_sem_config_sha256],
    ["model_scientific_sha256", bootstrap.model_scientific_sha256],
    ["source_dataset_fingerprint", bootstrap.source_dataset_fingerprint],
    ["complete_case_frame_sha256", bootstrap.complete_case_frame_sha256],
    ["usable_replicate_indices_sha256", bootstrap.usable_replicate_indices_sha256],
    ["confidence_level", confidenceLevel], ["resamples_requested", requested],
    ["resamples_usable", usable], ["minimum_usable_resamples", minimumUsable],
    ["seed", seed], ["workers", workers],
  ] as const;
  if (!sameCapabilityCellV1(receiptCell, GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1)
    || receipt.kind !== "case_bootstrap"
    || receipt.interval !== "percentile_type7"
    || receipt.tail !== "two_sided"
    || receipt.complete_model_reestimated_per_replicate !== true
    || receipt.compilation_artifact_identity_sha256 !== input.analytical.compilation_artifact_identity_sha256
    || receipt.recipe_analytical_sha256 !== input.analytical.recipe_analytical_sha256
    || exactReceiptValues.some(([field, expected]) => receipt[field] !== expected)
    || JSON.stringify(receipt.effect_ids) !== JSON.stringify(targetIds)
    || receipt.effect_identity_set_sha256 !== sha256HexUtf8V1(JSON.stringify(canonicalIdentities))
    || JSON.stringify(receipt.failed_replicates) !== JSON.stringify(failures)) {
    completedExecutionMismatchV1(receiptPath, `${receiptPath} differs from the exact raw gamma bootstrap receipt.`);
  }
}

/**
 * Reconciles the one-shot native result with the exact execution cell selected
 * from the current compiled graph. Archive identity guards alone are
 * insufficient because method/capability relabeling can leave those IDs
 * unchanged. The analytical payload remains the authority for its own adapter
 * identity while the canonical document remains the report authority.
 */
export function validateGeneralSemPlsCompletedExecutionV1(
  completed: GeneralSemPlsCompletedResultV1,
  execution: GeneralSemPlsExecutionCapabilityV1,
): void {
  const expected = expectedGeneralSemExecutionAuthorityV1(execution.kind);
  const document = completed.canonicalDocument;
  const analytical = recordAt(completed.analyticalResult, "completed.analyticalResult");
  exactKeysAt(analytical, [
    "schema_version", "adapter_version", "capability_cell",
    "compilation_artifact_identity_sha256", "compiled_plan_sha256",
    "recipe_analytical_sha256", "model_scientific_sha256",
    "stage_one_model_scientific_sha256", "source_dataset_fingerprint",
    "general_sem_config_sha256", "point_estimation", "requested_effects",
    "interaction_point_estimation", "bootstrap_inference", "moderation_bootstrap_inference",
  ], "completed.analyticalResult");
  if (analytical.schema_version !== 1) completedExecutionMismatchV1("completed.analyticalResult.schema_version", "The analytical result schema is not General SEM execution result v1.");
  const analyticalCell = capabilityCellAtV1(analytical.capability_cell, "completed.analyticalResult.capability_cell");
  const adapterVersion = textAt(analytical.adapter_version, "completed.analyticalResult.adapter_version");
  const analyticalRecipeDigest = digestAt(analytical.recipe_analytical_sha256, "completed.analyticalResult.recipe_analytical_sha256");
  const analyticalModelDigest = digestAt(analytical.model_scientific_sha256, "completed.analyticalResult.model_scientific_sha256");
  digestAt(analytical.stage_one_model_scientific_sha256, "completed.analyticalResult.stage_one_model_scientific_sha256");
  digestAt(analytical.compilation_artifact_identity_sha256, "completed.analyticalResult.compilation_artifact_identity_sha256");
  digestAt(analytical.compiled_plan_sha256, "completed.analyticalResult.compiled_plan_sha256");
  digestAt(analytical.general_sem_config_sha256, "completed.analyticalResult.general_sem_config_sha256");
  const analyticalDatasetFingerprint = textAt(analytical.source_dataset_fingerprint, "completed.analyticalResult.source_dataset_fingerprint");
  if (!Array.isArray(analytical.requested_effects)) completedExecutionMismatchV1("completed.analyticalResult.requested_effects", "The analytical result requested-effect ledger is missing.");

  if (!sameCapabilityCellV1(execution.capabilityCell, expected.requestCell)) {
    completedExecutionMismatchV1("execution.capabilityCell", "The requested execution capability cell differs from the exact selected inference option.");
  }
  if (!sameCapabilityCellV1(analyticalCell, expected.analyticalCell)) {
    completedExecutionMismatchV1("completed.analyticalResult.capability_cell", "The analytical result capability cell differs from the exact compiled point-estimation authority.");
  }
  if (adapterVersion !== expected.adapterVersion
    || document.provenance.engine_version !== expected.adapterVersion) {
    completedExecutionMismatchV1("completed.canonicalDocument.provenance.engine_version", "The result adapter or canonical engine version differs from the exact execution kind.");
  }
  if (!sameCapabilityCellV1(document.provenance.capability_cell, expected.primaryDocumentCell)
    || document.provenance.method_version !== expected.methodVersion) {
    completedExecutionMismatchV1("completed.canonicalDocument.provenance", "The canonical primary capability or analytical method differs from the exact execution kind.");
  }

  const documentCells = document.capability_cells ?? [];
  if (documentCells.length !== expected.documentCells.length
    || documentCells.some((cell, index) => !sameCapabilityCellV1(cell, expected.documentCells[index]!))) {
    completedExecutionMismatchV1("completed.canonicalDocument.capability_cells", "The canonical document capability inventory differs from the exact execution cell and its declared dependencies.");
  }
  if (analyticalRecipeDigest !== document.provenance.recipe_digest
    || analyticalModelDigest !== completed.archiveIdentity.modelScientificSha256
    || analyticalModelDigest !== document.provenance.model_digest
    || analyticalDatasetFingerprint !== completed.archiveIdentity.datasetFingerprint
    || analyticalDatasetFingerprint !== document.provenance.dataset_fingerprint) {
    completedExecutionMismatchV1("completed.analyticalResult", "The analytical result digests differ from the returned archive and canonical authorities.");
  }

  const hasInteractionResult = Object.prototype.hasOwnProperty.call(analytical, "interaction_point_estimation")
    && analytical.interaction_point_estimation !== null;
  const hasBootstrapResult = Object.prototype.hasOwnProperty.call(analytical, "bootstrap_inference")
    && analytical.bootstrap_inference !== null;
  const hasModerationBootstrapResult = Object.prototype.hasOwnProperty.call(
    analytical,
    "moderation_bootstrap_inference",
  ) && analytical.moderation_bootstrap_inference !== null;
  const expectedInteractionResult = execution.kind === "multiple_two_way_moderation_point"
    || execution.kind === "multiple_two_way_moderation_bootstrap";
  const expectedBootstrapResult = execution.kind === "mediation_bootstrap";
  const expectedModerationBootstrapResult = execution.kind === "multiple_two_way_moderation_bootstrap";
  if (hasInteractionResult !== expectedInteractionResult
    || hasBootstrapResult !== expectedBootstrapResult
    || hasModerationBootstrapResult !== expectedModerationBootstrapResult) {
    completedExecutionMismatchV1("completed.analyticalResult", "The analytical interaction/bootstrap payload shape contradicts the selected execution kind.");
  }

  const expectedInteractionIds = exactSortedDistinctExecutionInventoryV1(
    execution.interactionIds,
    "execution.interactionIds",
  );
  const expectedFocalRelationIds = exactSortedDistinctExecutionInventoryV1(
    execution.focalRelationIds,
    "execution.focalRelationIds",
  );
  const generalSemResults = document.general_sem_results as UnknownRecord | undefined;
  const hasCanonicalInteractionPayload = Boolean(
    generalSemResults
    && Object.prototype.hasOwnProperty.call(generalSemResults, "interaction_effects")
    && generalSemResults.interaction_effects !== undefined
    && generalSemResults.interaction_effects !== null,
  );

  if (!expectedInteractionResult) {
    if (expectedInteractionIds.length > 0 || expectedFocalRelationIds.length > 0) {
      completedExecutionMismatchV1(
        "execution",
        "A mediation execution must not carry interaction or focal-relation identities.",
      );
    }
    if (hasCanonicalInteractionPayload) {
      completedExecutionMismatchV1(
        "completed.canonicalDocument.general_sem_results.interaction_effects",
        "A mediation result must not carry a canonical interaction-effect payload.",
      );
    }
    return;
  }

  if (expectedInteractionIds.length === 0 || expectedFocalRelationIds.length === 0) {
    completedExecutionMismatchV1(
      "execution",
      "A moderation execution requires nonempty interaction and focal-relation identity inventories.",
    );
  }
  if (!hasCanonicalInteractionPayload) {
    completedExecutionMismatchV1(
      "completed.canonicalDocument.general_sem_results.interaction_effects",
      "The moderation result is missing its canonical interaction-effect inventory.",
    );
  }

  const interactionPoint = analytical.interaction_point_estimation;
  if (!interactionPoint || typeof interactionPoint !== "object" || Array.isArray(interactionPoint)) {
    completedExecutionMismatchV1(
      "completed.analyticalResult.interaction_point_estimation",
      "The moderation analytical payload must be an object.",
    );
  }
  const analyticalIdentities = completedInteractionIdentitiesV1(
    (interactionPoint as UnknownRecord).interaction_coefficients,
    "completed.analyticalResult.interaction_point_estimation.interaction_coefficients",
  );
  const canonicalIdentities = completedInteractionIdentitiesV1(
    generalSemResults!.interaction_effects,
    "completed.canonicalDocument.general_sem_results.interaction_effects",
  );
  for (const [path, identities] of [
    ["completed.analyticalResult.interaction_point_estimation.interaction_coefficients", analyticalIdentities],
    ["completed.canonicalDocument.general_sem_results.interaction_effects", canonicalIdentities],
  ] as const) {
    requireSameCompletedInventoryV1(
      identities.map((identity) => identity.interactionId),
      expectedInteractionIds,
      `${path}.interaction_id`,
    );
    requireSameCompletedInventoryV1(
      identities.map((identity) => identity.focalRelationId),
      expectedFocalRelationIds,
      `${path}.focal_relation_id`,
    );
  }
  requireSameCompletedInteractionMappingV1(
    analyticalIdentities,
    canonicalIdentities,
    "completed.interaction_identity_mapping",
  );
  if (expectedModerationBootstrapResult) {
    validateCompletedModerationBootstrapInferenceV1({
      value: analytical.moderation_bootstrap_inference,
      analytical,
      interactionPoint: interactionPoint as UnknownRecord,
      generalSemResults: generalSemResults!,
    });
  }
}

function expectedGeneralSemExecutionAuthorityV1(kind: GeneralSemPlsExecutionKindV1) {
  const moderation = kind === "multiple_two_way_moderation_point"
    || kind === "multiple_two_way_moderation_bootstrap";
  const primaryDocumentCell = moderation
    ? GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1
    : GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1;
  const requestCell = kind === "mediation_bootstrap"
    ? GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1
    : kind === "multiple_two_way_moderation_bootstrap"
      ? GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1
    : primaryDocumentCell;
  const documentCells = [
    GENERAL_SEM_PLS_BASE_CAPABILITY_CELL_V1,
    primaryDocumentCell,
    ...(kind === "mediation_bootstrap"
      ? [GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1]
      : kind === "multiple_two_way_moderation_bootstrap"
        ? [GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1]
        : []),
  ].sort(compareCapabilityCellsV1);
  return {
    primaryDocumentCell,
    requestCell,
    analyticalCell: primaryDocumentCell,
    documentCells,
    methodVersion: kind === "multiple_two_way_moderation_bootstrap"
      ? GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
      : kind === "multiple_two_way_moderation_point"
        ? GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1
      : kind === "mediation_bootstrap"
        ? GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1
        : GENERAL_SEM_PLS_POINT_METHOD_VERSION_V1,
    adapterVersion: kind === "multiple_two_way_moderation_bootstrap"
      ? GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_ADAPTER_VERSION_V1
      : kind === "multiple_two_way_moderation_point"
        ? GENERAL_SEM_PLS_MODERATION_POINT_ADAPTER_VERSION_V1
      : kind === "mediation_bootstrap"
        ? GENERAL_SEM_PLS_BOOTSTRAP_ADAPTER_VERSION_V1
        : GENERAL_SEM_PLS_POINT_ADAPTER_VERSION_V1,
  } as const;
}

function capabilityCellAtV1(value: unknown, path: string): CapabilityCellReferenceV2 {
  const cell = recordAt(value, path);
  exactKeysAt(cell, ["registry_schema_version", "capability_id", "cell_id", "capability_version"], path);
  if (cell.registry_schema_version !== 2) completedExecutionMismatchV1(`${path}.registry_schema_version`, "The result capability registry schema must be version 2.");
  return {
    registry_schema_version: 2,
    capability_id: textAt(cell.capability_id, `${path}.capability_id`),
    cell_id: textAt(cell.cell_id, `${path}.cell_id`),
    capability_version: textAt(cell.capability_version, `${path}.capability_version`),
  };
}

function completedExecutionMismatchV1(subject: string, message: string): never {
  throw new GeneralSemWorkspaceErrorV1(
    "general_sem.wire.completed_execution_mismatch",
    subject,
    message,
    "Discard the completed job and rerun native preflight from the unchanged marked General SEM archive before calculating again.",
  );
}

export function generalSemJobRequestFromReceiptV1(
  receipt: GeneralSemProjectBootstrapReceiptV1,
  model: SemModelV4,
  config: GeneralSemConfigV1,
  decision: SemCapabilityDecisionV1,
  expectedArchiveSha256 = receipt.destinationArchiveSha256,
  experimentalLabsEnabled = true,
  registry: GeneralSemCapabilityRegistryReaderV1 = capabilityRegistryV2,
): GeneralSemPlsJobRequestV1 {
  const execution = selectGeneralSemPlsExecutionCapabilityV1({ model, config, decision });
  const access = selectGeneralSemExecutionAccessV1({
    capabilityCell: execution.capabilityCell,
    experimentalLabsEnabled,
    registry,
  });
  return {
    ...access,
    archivePath: receipt.destinationArchivePath,
    expectedArchiveSha256,
    projectId: receipt.projectId,
    datasetId: receipt.residentDatasetId,
    datasetFingerprint: receipt.residentDatasetFingerprint,
    modelId: receipt.residentModelId,
    modelScientificSha256: receipt.residentModelScientificSha256,
    recipeId: receipt.residentRecipeId,
    recipeDocumentSha256: receipt.residentRecipeDocumentSha256,
    capabilityCell: execution.capabilityCell,
  };
}

export type GeneralSemPlsMonitorOutcomeV1 =
  | { status: "completed"; snapshot: GeneralSemPlsJobSnapshotV1; completed: GeneralSemPlsCompletedResultV1 }
  | { status: "terminal_without_result"; snapshot: GeneralSemPlsJobSnapshotV1 }
  | { status: "aborted"; snapshot: GeneralSemPlsJobSnapshotV1 };

export async function monitorGeneralSemPlsJobV1(input: {
  initial: GeneralSemPlsJobSnapshotV1;
  getStatus: (jobId: string) => Promise<GeneralSemPlsJobSnapshotV1>;
  getResult: (jobId: string) => Promise<GeneralSemPlsCompletedResultV1>;
  onSnapshot?: (snapshot: GeneralSemPlsJobSnapshotV1) => void;
  wait?: () => Promise<void>;
  signal?: AbortSignal;
}): Promise<GeneralSemPlsMonitorOutcomeV1> {
  const wait = input.wait ?? (() => new Promise<void>((resolve) => globalThis.setTimeout(resolve, 250)));
  let snapshot = input.initial;
  input.onSnapshot?.(snapshot);
  while (snapshot.state === "queued" || snapshot.state === "running" || snapshot.state === "cancelling") {
    if (input.signal?.aborted) return { status: "aborted", snapshot };
    await wait();
    if (input.signal?.aborted) return { status: "aborted", snapshot };
    snapshot = await input.getStatus(snapshot.jobId);
    input.onSnapshot?.(snapshot);
  }
  if (snapshot.state !== "completed") return { status: "terminal_without_result", snapshot };
  if (input.signal?.aborted) return { status: "aborted", snapshot };
  return { status: "completed", snapshot, completed: await input.getResult(snapshot.jobId) };
}

export async function appendGeneralSemResultV1(
  completed: GeneralSemPlsCompletedResultV1,
  execution: GeneralSemPlsExecutionCapabilityV1,
  experimentalLabsEnabled: boolean,
  append: (request: InternalProjectSchema6ResultAppendRequestV1) => Promise<InternalProjectSchema6ResultAppendOutcomeV1>,
  registry: GeneralSemCapabilityRegistryReaderV1 = capabilityRegistryV2,
): Promise<InternalProjectSchema6ResultAppendOutcomeV1> {
  const access = selectGeneralSemExecutionAccessV1({
    capabilityCell: execution.capabilityCell,
    experimentalLabsEnabled,
    registry,
  });
  return append({
    ...access,
    capabilityCell: execution.capabilityCell,
    archivePath: completed.archiveIdentity.archivePath,
    expectedSourceSha256: completed.archiveIdentity.archiveSha256,
    canonicalDocument: completed.canonicalDocument,
  });
}

export async function reopenGeneralSemResultV1(
  completed: GeneralSemPlsCompletedResultV1,
  execution: GeneralSemPlsExecutionCapabilityV1,
  updatedArchiveSha256: string,
  read: (request: InternalProjectSchema6ResultReadRequestV1) => Promise<InternalProjectSchema6ResultReadOutcomeV1>,
  registry: GeneralSemCapabilityRegistryReaderV1 = capabilityRegistryV2,
): Promise<{ outcome: InternalProjectSchema6ResultReadOutcomeV1; entry: InternalProjectSchema6CanonicalResultEntryV1 | null }> {
  const access = selectGeneralSemExecutionAccessV1({
    capabilityCell: execution.capabilityCell,
    // Discover the immutable Registry surface. Native readback uses a
    // separate non-mutating policy and never grants execution or append.
    experimentalLabsEnabled: true,
    registry,
  });
  const outcome = await read({
    surface: access.surface,
    experimentalLabsEnabled: false,
    capabilityCell: execution.capabilityCell,
    archivePath: completed.archiveIdentity.archivePath,
    expectedSourceSha256: updatedArchiveSha256,
  });
  if (outcome.status === "blocked") return { outcome, entry: null };
  const entry = outcome.value.documents.find((candidate) => candidate.documentId === completed.canonicalDocument.document_id) ?? null;
  if (entry && canonicalResultDocumentJson(entry.canonicalDocument) !== canonicalResultDocumentJson(completed.canonicalDocument)) {
    throw new GeneralSemWorkspaceErrorV1(
      "general_sem.persisted_result_mismatch",
      completed.canonicalDocument.document_id,
      "The strictly reopened General SEM document differs from the completed native result.",
      "Preserve the archive and completed job evidence; do not export or report this result.",
    );
  }
  return { outcome, entry };
}

export function nativeGeneralSemPreflightRequestV1(
  snapshot: InternalProjectArchiveV6ReadSnapshotV1,
  model: SemModelV4,
  config: GeneralSemConfigV1,
  experimentalLabsEnabled = true,
  registry: GeneralSemCapabilityRegistryReaderV1 = capabilityRegistryV2,
) {
  const capabilityCell = generalSemPlsRequestedCapabilityCellV1(model, config);
  const access = selectGeneralSemExecutionAccessV1({
    capabilityCell,
    experimentalLabsEnabled,
    registry,
  });
  return {
    ...access,
    capabilityCell,
    project: snapshot.project,
    model,
    config,
  };
}

export function generalSemFailureV1(error: unknown): GeneralSemPlsJobFailureV1 {
  if (error && typeof error === "object") {
    const candidate = error as Partial<GeneralSemPlsJobFailureV1>;
    if (candidate.schemaVersion === 1 && typeof candidate.code === "string" && typeof candidate.message === "string" && typeof candidate.correctiveAction === "string" && typeof candidate.subject === "string") {
      return { ...candidate, stage: candidate.stage ?? "integrity", issues: candidate.issues ?? [] } as GeneralSemPlsJobFailureV1;
    }
  }
  if (error instanceof GeneralSemWorkspaceErrorV1) return {
    schemaVersion: 1,
    stage: "integrity",
    subject: error.subject,
    code: error.code,
    message: error.message,
    correctiveAction: error.correctiveAction,
    issues: [],
  };
  return {
    schemaVersion: 1,
    stage: "integrity",
    subject: "general_sem",
    code: "general_sem.workspace_failed",
    message: error instanceof Error && error.message.trim() ? error.message : "The General SEM workflow could not continue.",
    correctiveAction: "Review the compatibility details, preserve the current project, and retry.",
    issues: [],
  };
}
