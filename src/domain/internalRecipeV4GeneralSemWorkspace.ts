import type { CanonicalResultDocumentV2, CapabilityCellReferenceV2 } from "./canonicalResultDocumentV2";
import { canonicalResultDocumentJson, validateCanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import {
  defaultGeneralSemConfigV1,
  parseGeneralSemConfigV1,
  type GeneralSemConfigV1,
  type GeneralSemEffectEstimandV1,
} from "./generalSemConfigV1";
import { preflightGeneralSemPlsV1 } from "./generalSemCapabilityPreflightV1";
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
  validateSemModelV4,
  type SemModelV4,
} from "./semModelV4";
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
  sourceProjectId: string | null;
  dataset: Dataset | null;
  model: SemModelV4 | null;
  config: GeneralSemConfigV1;
  engine: GeneralSemPlsEngineOptionsV1;
}): GeneralSemWorkspacePreflightV1 {
  const issues: GeneralSemWorkspaceIssueV1[] = [];
  if (!input.experimentalLabsEnabled) issues.push(issue(
    "general_sem.access.experimental_labs_required",
    "experimentalLabsEnabled",
    "General SEM calculation is available only in Experimental Labs.",
    "Enable Experimental Labs in Preferences.",
  ));
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
      decision = preflightGeneralSemPlsV1(input.model, input.config);
      for (const diagnostic of decision.diagnostics.filter((item) => item.severity === "error")) issues.push(issue(
        diagnostic.code,
        diagnostic.subject ?? "model",
        diagnostic.message,
        diagnostic.corrections[0] ?? "Correct the model or calculation configuration.",
      ));
    } catch (error) {
      issues.push(issue(
        "general_sem.preflight.contract_invalid",
        "preflight",
        error instanceof Error ? error.message : "The General SEM capability decision could not be validated.",
        "Keep the model unchanged and retry after reopening the current project.",
      ));
    }
  }
  return { ready: issues.length === 0 && decision?.status === "experimental", decision, issues };
}

export interface BuildGeneralSemRecipeV1Input {
  recipeId: string;
  createdAt: string;
  dataset: Dataset;
  model: SemModelV4;
  nativeScientificSha256: string;
  config: GeneralSemConfigV1;
  engine: GeneralSemPlsEngineOptionsV1;
}

export function buildGeneralSemRecipeV1(input: BuildGeneralSemRecipeV1Input): AnalysisRecipeV4 {
  if (!SHA256.test(input.nativeScientificSha256)) throw new GeneralSemWorkspaceErrorV1(
    "general_sem.model.native_digest_invalid", input.model.id, "The native model digest is invalid.", "Re-run native scientific validation.",
  );
  if (!input.dataset.fingerprint?.trim() || input.model.data_binding.dataset_id !== input.dataset.id) throw new GeneralSemWorkspaceErrorV1(
    "general_sem.dataset.binding_mismatch", input.dataset.id, "The model and resident dataset identities differ.", "Rebind the model to the selected dataset.",
  );
  const config = parseGeneralSemConfigV1(input.config);
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
      execution_surface: "native_general_sem_pls_labs_v1",
      general_sem_generation: "general_sem_v1",
    },
    legacy_source: null,
  };
}

export interface GeneralSemProjectBootstrapRequestV1 {
  surface: "internal_labs";
  experimentalLabsEnabled: true;
  destinationPath: string;
  projectId: string;
  name: string;
  createdAt: string;
  sourceProjectId: string;
  sourceDatasetId: string;
  sourceDatasetFingerprint: string;
  model: SemModelV4;
  recipe: AnalysisRecipeV4;
}

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
}

/** Restores the exact native RecipeV4 authority after remount or process restart. */
export function rehydrateGeneralSemExecutionAuthorityV1(
  snapshot: InternalProjectArchiveV6ReadSnapshotV1,
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
  if (!config
    || recipe.settings.method !== "pls_pm"
    || recipe.settings.weighting_scheme !== "path"
    || recipe.settings.preprocessing !== "standardized"
    || recipe.settings.missing_data !== "listwise_deletion"
    || recipe.settings.case_weight_column !== null
    || methodConfig?.kind !== "pls_algorithm"
    || recipe.metadata.execution_surface !== "native_general_sem_pls_labs_v1"
    || recipe.metadata.general_sem_generation !== "general_sem_v1") {
    throw new GeneralSemWorkspaceErrorV1(
      "general_sem.rehydrate.recipe_scope_mismatch",
      authority.recipeId,
      "The resident RecipeV4 is outside the bounded General SEM PLS execution scope.",
      "Keep the archive unchanged and use an estimator cell that explicitly supports its recipe.",
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

export interface GeneralSemPlsJobRequestV1 {
  surface: "internal_labs";
  experimentalLabsEnabled: true;
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
}

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
  return { schemaVersion: 1, archiveIdentity, analyticalResult: completed.analyticalResult, canonicalDocument };
}

export function generalSemJobRequestFromReceiptV1(
  receipt: GeneralSemProjectBootstrapReceiptV1,
  config: GeneralSemConfigV1,
  expectedArchiveSha256 = receipt.destinationArchiveSha256,
): GeneralSemPlsJobRequestV1 {
  return {
    surface: "internal_labs",
    experimentalLabsEnabled: true,
    archivePath: receipt.destinationArchivePath,
    expectedArchiveSha256,
    projectId: receipt.projectId,
    datasetId: receipt.residentDatasetId,
    datasetFingerprint: receipt.residentDatasetFingerprint,
    modelId: receipt.residentModelId,
    modelScientificSha256: receipt.residentModelScientificSha256,
    recipeId: receipt.residentRecipeId,
    recipeDocumentSha256: receipt.residentRecipeDocumentSha256,
    capabilityCell: config.inference.kind === "case_bootstrap"
      ? GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1
      : GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1,
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
  append: (request: InternalProjectSchema6ResultAppendRequestV1) => Promise<InternalProjectSchema6ResultAppendOutcomeV1>,
): Promise<InternalProjectSchema6ResultAppendOutcomeV1> {
  return append({
    surface: "internal_labs",
    experimentalLabsEnabled: true,
    archivePath: completed.archiveIdentity.archivePath,
    expectedSourceSha256: completed.archiveIdentity.archiveSha256,
    canonicalDocument: completed.canonicalDocument,
  });
}

export async function reopenGeneralSemResultV1(
  completed: GeneralSemPlsCompletedResultV1,
  updatedArchiveSha256: string,
  read: (request: InternalProjectSchema6ResultReadRequestV1) => Promise<InternalProjectSchema6ResultReadOutcomeV1>,
): Promise<{ outcome: InternalProjectSchema6ResultReadOutcomeV1; entry: InternalProjectSchema6CanonicalResultEntryV1 | null }> {
  const outcome = await read({
    surface: "internal_labs",
    experimentalLabsEnabled: true,
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
) {
  return {
    surface: "internal_labs" as const,
    experimentalLabsEnabled: true as const,
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
