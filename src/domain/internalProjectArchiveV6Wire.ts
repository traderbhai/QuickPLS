import {
  validateCanonicalResultDocumentV2,
  type CanonicalResultDocumentV2,
} from "./canonicalResultDocumentV2";
import {
  parseSemModelV4,
  parseSemModelV4AuthoringDraft,
  type SemModelV4,
} from "./semModelV4";
import type { NativeCanonicalModelSpec } from "../types";

const LOWER_SHA256 = /^[0-9a-f]{64}$/;
const CANONICAL_UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const RFC3339 = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/;
const NIL_UUID = "00000000-0000-0000-0000-000000000000";
const U32_MAX = 0xffff_ffff;

type WireRecord = Record<string, unknown>;

export class InternalProjectArchiveV6WireError extends Error {
  constructor(
    public readonly code: string,
    public readonly path: string,
    message: string,
  ) {
    super(message);
    this.name = "InternalProjectArchiveV6WireError";
  }
}

function fail(code: string, path: string, message: string): never {
  throw new InternalProjectArchiveV6WireError(code, path, message);
}

function recordAt(value: unknown, path: string): WireRecord {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return fail("project_archive_v6.object_required", path, `${path} must be an object.`);
  }
  return value as WireRecord;
}

function exactRecordAt(
  value: unknown,
  required: readonly string[],
  optional: readonly string[],
  path: string,
): WireRecord {
  const record = recordAt(value, path);
  const allowed = new Set([...required, ...optional]);
  for (const key of required) {
    if (!Object.prototype.hasOwnProperty.call(record, key)) {
      fail("project_archive_v6.field_missing", `${path}.${key}`, `${path}.${key} is required.`);
    }
  }
  for (const key of Object.keys(record)) {
    if (!allowed.has(key)) {
      fail("project_archive_v6.field_unknown", `${path}.${key}`, `${path}.${key} is not part of the schema-v6 wire contract.`);
    }
  }
  return record;
}

function arrayAt(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) fail("project_archive_v6.array_required", path, `${path} must be an array.`);
  return value;
}

function textAt(value: unknown, path: string, allowEmpty = false): string {
  if (typeof value !== "string" || (!allowEmpty && !value.trim())) {
    fail("project_archive_v6.text_required", path, `${path} must be a${allowEmpty ? "" : " nonempty"} string.`);
  }
  return value;
}

function optionalTextAt(value: unknown, path: string): string | null {
  return value == null ? null : textAt(value, path, true);
}

function u32At(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value) || Object.is(value, -0) || (value as number) < 0 || (value as number) > U32_MAX) {
    fail("project_archive_v6.u32_required", path, `${path} must be an unsigned 32-bit integer.`);
  }
  return value as number;
}

function countAt(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value) || Object.is(value, -0) || (value as number) < 0) {
    fail("project_archive_v6.count_required", path, `${path} must be a nonnegative safe integer.`);
  }
  return value as number;
}

function finiteOrNullAt(value: unknown, path: string): number | null {
  if (value === null || value === undefined) return null;
  if (typeof value !== "number" || !Number.isFinite(value) || Object.is(value, -0)) {
    fail("project_archive_v6.finite_required", path, `${path} must be finite or null.`);
  }
  return value;
}

function finiteAt(value: unknown, path: string): number {
  const parsed = finiteOrNullAt(value, path);
  if (parsed === null) fail("project_archive_v6.finite_required", path, `${path} must be finite.`);
  return parsed;
}

function booleanAt(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") {
    fail("project_archive_v6.boolean_required", path, `${path} must be a boolean.`);
  }
  return value;
}

function uuidAt(value: unknown, path: string): string {
  const parsed = textAt(value, path);
  if (!CANONICAL_UUID.test(parsed)) {
    fail("project_archive_v6.uuid_invalid", path, `${path} must be a canonical lowercase UUID.`);
  }
  return parsed;
}

function sha256At(value: unknown, path: string): string {
  const parsed = textAt(value, path);
  if (!LOWER_SHA256.test(parsed)) {
    fail("project_archive_v6.sha256_invalid", path, `${path} must be a lowercase SHA-256 digest.`);
  }
  return parsed;
}

function timestampAt(value: unknown, path: string): string {
  const parsed = textAt(value, path);
  if (!RFC3339.test(parsed) || !Number.isFinite(Date.parse(parsed))) {
    fail("project_archive_v6.timestamp_invalid", path, `${path} must be an RFC 3339 timestamp.`);
  }
  return parsed;
}

function enumAt<T extends string>(value: unknown, allowed: readonly T[], path: string): T {
  if (typeof value !== "string" || !allowed.includes(value as T)) {
    fail("project_archive_v6.enum_invalid", path, `${path} has an unsupported discriminator.`);
  }
  return value as T;
}

function stringArrayAt(value: unknown, path: string): string[] {
  return arrayAt(value, path).map((entry, index) => textAt(entry, `${path}[${index}]`, true));
}

function stringMapAt(value: unknown, path: string): Record<string, string> {
  const record = recordAt(value, path);
  return Object.fromEntries(Object.entries(record).map(([key, item]) => [key, textAt(item, `${path}.${key}`, true)]));
}

function hasOwn(record: WireRecord, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(record, key);
}

export type ProjectArchiveSchemaAccessV6Wire =
  | "historical_upgrade_copy_required"
  | "current_editable"
  | "future_read_only";

export function classifyInternalProjectArchiveSchemaV6(schemaVersion: unknown): ProjectArchiveSchemaAccessV6Wire {
  const version = u32At(schemaVersion, "schema_version");
  if (version === 0) fail("project_archive_v6.schema_zero", "schema_version", "Project archive schema version 0 is unsupported.");
  if (version <= 5) return "historical_upgrade_copy_required";
  if (version === 6) return "current_editable";
  return "future_read_only";
}

export interface ProjectUpgradeLineageV6Wire {
  source_project_id: string;
  source_archive_schema_version: number;
  source_archive_sha256: string;
  source_archive_path: string;
  destination_archive_path: string;
  upgraded_at: string;
  source_preservation: "required";
  write_policy: "new_archive_only";
  historical_results_immutable: true;
}

export type ProjectOriginV6Wire =
  | { kind: "new_project" }
  | { kind: "upgraded_copy"; lineage: ProjectUpgradeLineageV6Wire };

function normalizePathForComparison(value: string): string {
  return value.trim().replace(/[\\/]+$/, "").replaceAll("/", "\\").toLowerCase();
}

function parseUpgradeLineage(value: unknown, path: string): ProjectUpgradeLineageV6Wire {
  const lineage = exactRecordAt(value, [
    "source_project_id",
    "source_archive_schema_version",
    "source_archive_sha256",
    "source_archive_path",
    "destination_archive_path",
    "upgraded_at",
    "source_preservation",
    "write_policy",
    "historical_results_immutable",
  ], [], path);
  const sourceArchiveSchemaVersion = u32At(lineage.source_archive_schema_version, `${path}.source_archive_schema_version`);
  if (sourceArchiveSchemaVersion < 1 || sourceArchiveSchemaVersion > 5) {
    fail("project_archive_v6.upgrade_schema_invalid", `${path}.source_archive_schema_version`, "Upgrade lineage must reference source schema 1 through 5.");
  }
  const sourceArchivePath = textAt(lineage.source_archive_path, `${path}.source_archive_path`);
  const destinationArchivePath = textAt(lineage.destination_archive_path, `${path}.destination_archive_path`);
  const normalizedSourcePath = normalizePathForComparison(sourceArchivePath);
  const normalizedDestinationPath = normalizePathForComparison(destinationArchivePath);
  if (!normalizedSourcePath || !normalizedDestinationPath) {
    fail("project_archive_v6.upgrade_path_empty", path, "Upgrade source and destination paths must remain nonempty after slash normalization.");
  }
  if (normalizedSourcePath === normalizedDestinationPath) {
    fail("project_archive_v6.upgrade_destination_not_new", path, "Upgrade source and destination paths must differ.");
  }
  if (lineage.source_preservation !== "required"
    || lineage.write_policy !== "new_archive_only"
    || lineage.historical_results_immutable !== true) {
    fail("project_archive_v6.upgrade_policy_unsafe", path, "Upgrade lineage must preserve the source, write a new archive, and keep historical results immutable.");
  }
  return {
    source_project_id: uuidAt(lineage.source_project_id, `${path}.source_project_id`),
    source_archive_schema_version: sourceArchiveSchemaVersion,
    source_archive_sha256: sha256At(lineage.source_archive_sha256, `${path}.source_archive_sha256`),
    source_archive_path: sourceArchivePath,
    destination_archive_path: destinationArchivePath,
    upgraded_at: timestampAt(lineage.upgraded_at, `${path}.upgraded_at`),
    source_preservation: "required",
    write_policy: "new_archive_only",
    historical_results_immutable: true,
  };
}

function parseOrigin(value: unknown, path: string): ProjectOriginV6Wire {
  const origin = recordAt(value, path);
  if (origin.kind === "new_project") {
    exactRecordAt(origin, ["kind"], [], path);
    return { kind: "new_project" };
  }
  if (origin.kind === "upgraded_copy") {
    exactRecordAt(origin, ["kind", "lineage"], [], path);
    return { kind: "upgraded_copy", lineage: parseUpgradeLineage(origin.lineage, `${path}.lineage`) };
  }
  return fail("project_archive_v6.origin_invalid", `${path}.kind`, `${path}.kind must be new_project or upgraded_copy.`);
}

export interface ProjectDatasetColumnV6Wire {
  name: string;
  label: string | null;
  column_type: "numeric" | "text" | "boolean";
  scale_type: "continuous" | "ordinal" | "nominal" | "binary" | "identifier";
  missing_markers: string[];
  theoretical_min: number | null;
  theoretical_max: number | null;
  value_labels: Record<string, string>;
}

export interface ProjectDatasetDescriptorV6Wire {
  id: string;
  name: string;
  schema: {
    version: number;
    kind: "raw" | "covariance" | "correlation";
    columns: ProjectDatasetColumnV6Wire[];
    case_count: number;
    sample_size: number | null;
  };
  fingerprint: string;
}

function parseDatasetColumn(value: unknown, path: string): ProjectDatasetColumnV6Wire {
  // qpls-data structs do not deny unknown fields. Parse their known fields and
  // normalize exactly as a Rust deserialize/serialize cycle would.
  const column = recordAt(value, path);
  const missingMarkers = stringArrayAt(column.missing_markers, `${path}.missing_markers`);
  return {
    name: textAt(column.name, `${path}.name`),
    label: optionalTextAt(column.label, `${path}.label`),
    column_type: enumAt(column.column_type, ["numeric", "text", "boolean"] as const, `${path}.column_type`),
    scale_type: enumAt(column.scale_type, ["continuous", "ordinal", "nominal", "binary", "identifier"] as const, `${path}.scale_type`),
    missing_markers: missingMarkers,
    theoretical_min: finiteOrNullAt(column.theoretical_min, `${path}.theoretical_min`),
    theoretical_max: finiteOrNullAt(column.theoretical_max, `${path}.theoretical_max`),
    value_labels: stringMapAt(column.value_labels, `${path}.value_labels`),
  };
}

function parseDataset(value: unknown, path: string): ProjectDatasetDescriptorV6Wire {
  const dataset = recordAt(value, path);
  const schema = recordAt(dataset.schema, `${path}.schema`);
  const sampleSize = schema.sample_size == null ? null : countAt(schema.sample_size, `${path}.schema.sample_size`);
  return {
    id: uuidAt(dataset.id, `${path}.id`),
    name: textAt(dataset.name, `${path}.name`, true),
    schema: {
      version: u32At(schema.version, `${path}.schema.version`),
      kind: enumAt(schema.kind, ["raw", "covariance", "correlation"] as const, `${path}.schema.kind`),
      columns: arrayAt(schema.columns, `${path}.schema.columns`).map((column, index) => parseDatasetColumn(column, `${path}.schema.columns[${index}]`)),
      case_count: countAt(schema.case_count, `${path}.schema.case_count`),
      sample_size: sampleSize,
    },
    fingerprint: textAt(dataset.fingerprint, `${path}.fingerprint`),
  };
}

export interface LegacyDisplayCovarianceV4Wire {
  id: string;
  left_construct: string;
  right_construct: string;
  label: string | null;
}

export type ProjectModelPayloadV6Wire =
  | { kind: "sem_model_v4"; model: SemModelV4; scientific_sha256: string }
  | { kind: "sem_model_v4_draft"; model: SemModelV4; model_document_sha256: string }
  | {
      kind: "legacy_estimand_unspecified";
      legacy_model: NativeCanonicalModelSpec;
      legacy_model_sha256: string;
      display_covariances: LegacyDisplayCovarianceV4Wire[];
      automatic_conversion_blocker?: string;
    };

export interface ProjectModelRecordV6Wire {
  model_id: string;
  payload: ProjectModelPayloadV6Wire;
}

export type ExecutableProjectModelRecordV6Wire = ProjectModelRecordV6Wire & {
  payload: Extract<ProjectModelPayloadV6Wire, { kind: "sem_model_v4" }>;
};

export function isExecutableProjectModelRecordV6Wire(
  record: ProjectModelRecordV6Wire,
): record is ExecutableProjectModelRecordV6Wire {
  return record.payload.kind === "sem_model_v4";
}

function parseDisplayCovariance(value: unknown, path: string): LegacyDisplayCovarianceV4Wire {
  const covariance = exactRecordAt(value, ["id", "left_construct", "right_construct"], ["label"], path);
  return {
    id: textAt(covariance.id, `${path}.id`),
    left_construct: textAt(covariance.left_construct, `${path}.left_construct`),
    right_construct: textAt(covariance.right_construct, `${path}.right_construct`),
    label: optionalTextAt(covariance.label, `${path}.label`),
  };
}

function parseLegacyModel(value: unknown, path: string): NativeCanonicalModelSpec {
  const model = recordAt(value, path);
  const id = uuidAt(model.id, `${path}.id`);
  const constructs = arrayAt(model.constructs, `${path}.constructs`);
  const paths = arrayAt(model.paths, `${path}.paths`);
  const controls = arrayAt(hasOwn(model, "controls") ? model.controls : [], `${path}.controls`);
  const higherOrder = arrayAt(hasOwn(model, "higher_order_constructs") ? model.higher_order_constructs : [], `${path}.higher_order_constructs`);
  const interactions = arrayAt(hasOwn(model, "interactions") ? model.interactions : [], `${path}.interactions`);
  return {
    ...model,
    id,
    name: textAt(model.name, `${path}.name`, true),
    constructs,
    paths,
    controls,
    higher_order_constructs: higherOrder,
    interactions,
  } as unknown as NativeCanonicalModelSpec;
}

function parseModelRecord(value: unknown, path: string): ProjectModelRecordV6Wire {
  const record = exactRecordAt(value, ["model_id", "payload"], [], path);
  const modelId = textAt(record.model_id, `${path}.model_id`);
  const payload = recordAt(record.payload, `${path}.payload`);
  if (payload.kind === "sem_model_v4") {
    exactRecordAt(payload, ["kind", "model", "scientific_sha256"], [], `${path}.payload`);
    const model = parseSemModelV4(payload.model);
    if (model.id !== modelId) fail("project_archive_v6.model_identity", path, `${path} model_id differs from the SemModelV4 id.`);
    return {
      model_id: modelId,
      payload: {
        kind: "sem_model_v4",
        model,
        scientific_sha256: sha256At(payload.scientific_sha256, `${path}.payload.scientific_sha256`),
      },
    };
  }
  if (payload.kind === "sem_model_v4_draft") {
    exactRecordAt(payload, ["kind", "model", "model_document_sha256"], [], `${path}.payload`);
    const model = parseSemModelV4AuthoringDraft(payload.model);
    if (model.id !== modelId) fail("project_archive_v6.model_identity", path, `${path} model_id differs from the draft SemModelV4 id.`);
    return {
      model_id: modelId,
      payload: {
        kind: "sem_model_v4_draft",
        model,
        model_document_sha256: sha256At(payload.model_document_sha256, `${path}.payload.model_document_sha256`),
      },
    };
  }
  if (payload.kind === "legacy_estimand_unspecified") {
    exactRecordAt(payload, ["kind", "legacy_model", "legacy_model_sha256"], ["display_covariances", "automatic_conversion_blocker"], `${path}.payload`);
    const legacyModel = parseLegacyModel(payload.legacy_model, `${path}.payload.legacy_model`);
    if (legacyModel.id !== modelId) fail("project_archive_v6.model_identity", path, `${path} model_id differs from the legacy model id.`);
    const blocker = payload.automatic_conversion_blocker == null
      ? null
      : textAt(payload.automatic_conversion_blocker, `${path}.payload.automatic_conversion_blocker`, true);
    const normalized: Extract<ProjectModelPayloadV6Wire, { kind: "legacy_estimand_unspecified" }> = {
      kind: "legacy_estimand_unspecified",
      legacy_model: legacyModel,
      legacy_model_sha256: sha256At(payload.legacy_model_sha256, `${path}.payload.legacy_model_sha256`),
      display_covariances: arrayAt(hasOwn(payload, "display_covariances") ? payload.display_covariances : [], `${path}.payload.display_covariances`)
        .map((item, index) => parseDisplayCovariance(item, `${path}.payload.display_covariances[${index}]`)),
    };
    if (blocker !== null) normalized.automatic_conversion_blocker = blocker;
    return { model_id: modelId, payload: normalized };
  }
  return fail("project_archive_v6.model_kind", `${path}.payload.kind`, `${path}.payload.kind is unsupported.`);
}

export interface ImmutableHistoricalRecipeV6Wire {
  readonly recipe_id: string;
  readonly source_recipe_schema_version: number;
  readonly recipe_document: Readonly<WireRecord>;
  readonly recipe_document_sha256: string;
}

function parseHistoricalRecipe(value: unknown, path: string): ImmutableHistoricalRecipeV6Wire {
  const recipe = exactRecordAt(value, ["recipe_id", "source_recipe_schema_version", "recipe_document", "recipe_document_sha256"], [], path);
  const recipeId = uuidAt(recipe.recipe_id, `${path}.recipe_id`);
  const sourceVersion = u32At(recipe.source_recipe_schema_version, `${path}.source_recipe_schema_version`);
  if (sourceVersion < 1 || sourceVersion > 3) {
    fail("project_archive_v6.historical_recipe_schema", `${path}.source_recipe_schema_version`, "Historical recipe source schema must be 1 through 3.");
  }
  const document = recordAt(recipe.recipe_document, `${path}.recipe_document`);
  if (u32At(document.schema_version, `${path}.recipe_document.schema_version`) !== sourceVersion) {
    fail("project_archive_v6.historical_recipe_schema", path, "Historical recipe envelope and document schema versions differ.");
  }
  if (uuidAt(document.id, `${path}.recipe_document.id`) !== recipeId) {
    fail("project_archive_v6.historical_recipe_identity", path, "Historical recipe envelope and document ids differ.");
  }
  return {
    recipe_id: recipeId,
    source_recipe_schema_version: sourceVersion,
    recipe_document: document,
    recipe_document_sha256: sha256At(recipe.recipe_document_sha256, `${path}.recipe_document_sha256`),
  };
}

export type HistoricalResultRecipeBindingV6Wire =
  | { kind: "bound"; source_recipe_id: string; recipe_document_sha256: string }
  | { kind: "unbound_legacy" };

export interface ImmutableHistoricalResultV6Wire {
  readonly result_id: string;
  readonly source_result_schema_version: number;
  readonly result: Readonly<WireRecord>;
  readonly result_sha256: string;
  readonly source_recipe: HistoricalResultRecipeBindingV6Wire;
}

function parseHistoricalResultBinding(value: unknown, path: string): HistoricalResultRecipeBindingV6Wire {
  const binding = recordAt(value, path);
  if (binding.kind === "bound") {
    exactRecordAt(binding, ["kind", "source_recipe_id", "recipe_document_sha256"], [], path);
    return {
      kind: "bound",
      source_recipe_id: uuidAt(binding.source_recipe_id, `${path}.source_recipe_id`),
      recipe_document_sha256: sha256At(binding.recipe_document_sha256, `${path}.recipe_document_sha256`),
    };
  }
  if (binding.kind === "unbound_legacy") {
    exactRecordAt(binding, ["kind"], [], path);
    return { kind: "unbound_legacy" };
  }
  return fail("project_archive_v6.historical_result_binding_kind", `${path}.kind`, `${path}.kind must be bound or unbound_legacy.`);
}

function parseHistoricalResult(value: unknown, path: string): ImmutableHistoricalResultV6Wire {
  const envelope = exactRecordAt(value, ["result_id", "source_result_schema_version", "result", "result_sha256"], ["source_recipe"], path);
  const resultId = uuidAt(envelope.result_id, `${path}.result_id`);
  const sourceVersion = u32At(envelope.source_result_schema_version, `${path}.source_result_schema_version`);
  const result = recordAt(envelope.result, `${path}.result`);
  if (u32At(result.schema_version, `${path}.result.schema_version`) !== sourceVersion) {
    fail("project_archive_v6.historical_result_schema", path, "Historical result envelope and document schema versions differ.");
  }
  if (uuidAt(result.id, `${path}.result.id`) !== resultId) {
    fail("project_archive_v6.historical_result_identity", path, "Historical result envelope and document ids differ.");
  }
  return {
    result_id: resultId,
    source_result_schema_version: sourceVersion,
    result,
    result_sha256: sha256At(envelope.result_sha256, `${path}.result_sha256`),
    source_recipe: envelope.source_recipe === undefined
      ? { kind: "unbound_legacy" }
      : parseHistoricalResultBinding(envelope.source_recipe, `${path}.source_recipe`),
  };
}

function embeddedHistoricalRecipeId(result: ImmutableHistoricalResultV6Wire): string | null {
  const provenance = result.result.provenance;
  if (provenance === null || typeof provenance !== "object" || Array.isArray(provenance)) return null;
  const candidate = (provenance as WireRecord).recipe_id;
  if (typeof candidate !== "string" || !CANONICAL_UUID.test(candidate.toLowerCase())) return null;
  return candidate.toLowerCase();
}

function validateHistoricalResultBinding(
  result: ImmutableHistoricalResultV6Wire,
  historicalRecipes: ReadonlyMap<string, ImmutableHistoricalRecipeV6Wire>,
  sourceProvenanceCanBind: boolean,
  path: string,
): void {
  const embedded = embeddedHistoricalRecipeId(result);
  if (result.source_recipe.kind === "bound") {
    const recipe = historicalRecipes.get(result.source_recipe.source_recipe_id);
    if (!sourceProvenanceCanBind
      || embedded !== result.source_recipe.source_recipe_id
      || recipe?.recipe_document_sha256 !== result.source_recipe.recipe_document_sha256) {
      fail("project_archive_v6.historical_result_recipe_binding", path, "Historical result has a missing, invented, or mismatched source recipe binding.");
    }
    return;
  }
  if (sourceProvenanceCanBind && embedded && embedded !== NIL_UUID && historicalRecipes.has(embedded)) {
    fail("project_archive_v6.historical_result_recipe_binding", path, "A historical result with an available source recipe must use an explicit bound source_recipe.");
  }
}

type LegacyEstimandConfirmationV4Wire =
  | "not_legacy"
  | "legacy_estimand_unspecified"
  | "confirmed_composite"
  | "confirmed_common_factor";

type ProjectAnalysisRecipeModelBindingV4Wire =
  | { kind: "embedded_sem_model_v4"; model: SemModelV4; scientific_sha256: string }
  | { kind: "project_sem_model_v4_reference"; model_id: string; scientific_sha256: string }
  | { kind: "legacy_estimand_unspecified"; legacy_model_id: string; legacy_model_sha256: string };

export interface ProjectAnalysisSettingsV4Wire {
  method: "pls_pm" | "bootstrap" | "pls_sample_size_power" | "plsc" | "wpls" | "cca" | "cta_pls" | "endogeneity" | "nonlinear_effects" | "moderated_mediation" | "predict" | "mga" | "ipma" | "cbsem" | "pca" | "gsca" | "regression" | "nca" | "legacy";
  weighting_scheme: "path" | "factor" | "pca";
  tolerance: number;
  max_iterations: number;
  bootstrap_samples: number;
  bootstrap_test_tail?: "one_sided_greater" | "one_sided_less";
  studentized_inner_samples: number;
  permutation_samples: number;
  seed: number;
  workers: number;
  confidence_level: number;
  preprocessing: "standardized" | "mean_centered" | "unstandardized";
  missing_data: "listwise_deletion" | "mean_replacement";
  case_weight_column: string | null;
}

export interface ProjectAnalysisRecipeV4Wire {
  schema_version: 4;
  id: string;
  created_at: string;
  dataset_fingerprint: string;
  model_binding: ProjectAnalysisRecipeModelBindingV4Wire;
  estimand_confirmation: LegacyEstimandConfirmationV4Wire;
  settings: ProjectAnalysisSettingsV4Wire;
  method_config?: Readonly<WireRecord>;
  metadata: Record<string, string>;
  legacy_source?: { source_schema_version: 1 | 2 | 3; source_recipe_sha256: string };
}

function parseRecipeModelBinding(value: unknown, path: string): ProjectAnalysisRecipeModelBindingV4Wire {
  const binding = recordAt(value, path);
  if (binding.kind === "embedded_sem_model_v4") {
    exactRecordAt(binding, ["kind", "model", "scientific_sha256"], [], path);
    return {
      kind: "embedded_sem_model_v4",
      model: parseSemModelV4(binding.model),
      scientific_sha256: sha256At(binding.scientific_sha256, `${path}.scientific_sha256`),
    };
  }
  if (binding.kind === "project_sem_model_v4_reference") {
    exactRecordAt(binding, ["kind", "model_id", "scientific_sha256"], [], path);
    return {
      kind: "project_sem_model_v4_reference",
      model_id: textAt(binding.model_id, `${path}.model_id`),
      scientific_sha256: sha256At(binding.scientific_sha256, `${path}.scientific_sha256`),
    };
  }
  if (binding.kind === "legacy_estimand_unspecified") {
    exactRecordAt(binding, ["kind", "legacy_model_id", "legacy_model_sha256"], [], path);
    return {
      kind: "legacy_estimand_unspecified",
      legacy_model_id: textAt(binding.legacy_model_id, `${path}.legacy_model_id`),
      legacy_model_sha256: sha256At(binding.legacy_model_sha256, `${path}.legacy_model_sha256`),
    };
  }
  return fail("project_archive_v6.recipe_model_kind", `${path}.kind`, `${path}.kind is unsupported.`);
}

function parseRecipeSettings(value: unknown, path: string): ProjectAnalysisSettingsV4Wire {
  // AnalysisSettings intentionally lacks deny_unknown_fields. A Rust
  // deserialize/serialize cycle drops extensions while filling only missing
  // #[serde(default)] fields. Explicit null still fails for non-Option fields.
  const settings = recordAt(value, path);
  const bootstrapTestTail = hasOwn(settings, "bootstrap_test_tail")
    ? enumAt(settings.bootstrap_test_tail, ["two_sided", "one_sided_greater", "one_sided_less"] as const, `${path}.bootstrap_test_tail`)
    : "two_sided";
  return {
    method: enumAt(settings.method, [
      "pls_pm", "bootstrap", "pls_sample_size_power", "plsc", "wpls", "cca", "cta_pls",
      "endogeneity", "nonlinear_effects", "moderated_mediation", "predict", "mga", "ipma",
      "cbsem", "pca", "gsca", "regression", "nca", "legacy",
    ] as const, `${path}.method`),
    weighting_scheme: enumAt(settings.weighting_scheme, ["path", "factor", "pca"] as const, `${path}.weighting_scheme`),
    tolerance: finiteAt(settings.tolerance, `${path}.tolerance`),
    max_iterations: u32At(settings.max_iterations, `${path}.max_iterations`),
    bootstrap_samples: u32At(settings.bootstrap_samples, `${path}.bootstrap_samples`),
    ...(bootstrapTestTail === "two_sided" ? {} : { bootstrap_test_tail: bootstrapTestTail }),
    studentized_inner_samples: hasOwn(settings, "studentized_inner_samples")
      ? u32At(settings.studentized_inner_samples, `${path}.studentized_inner_samples`) : 0,
    permutation_samples: hasOwn(settings, "permutation_samples")
      ? u32At(settings.permutation_samples, `${path}.permutation_samples`) : 0,
    // JSON numbers above the safe-integer boundary cannot be mirrored without
    // changing the public representation, so fail closed instead of rounding.
    seed: countAt(settings.seed, `${path}.seed`),
    workers: hasOwn(settings, "workers") ? countAt(settings.workers, `${path}.workers`) : 1,
    confidence_level: hasOwn(settings, "confidence_level")
      ? finiteAt(settings.confidence_level, `${path}.confidence_level`) : 0.95,
    preprocessing: hasOwn(settings, "preprocessing")
      ? enumAt(settings.preprocessing, ["standardized", "mean_centered", "unstandardized"] as const, `${path}.preprocessing`)
      : "standardized",
    missing_data: hasOwn(settings, "missing_data")
      ? enumAt(settings.missing_data, ["listwise_deletion", "mean_replacement"] as const, `${path}.missing_data`)
      : "listwise_deletion",
    case_weight_column: settings.case_weight_column == null
      ? null
      : textAt(settings.case_weight_column, `${path}.case_weight_column`, true),
  };
}

function parseSegmentationConfig(value: unknown, path: string): WireRecord {
  const config = exactRecordAt(value, ["segments", "starts", "minimum_segment_share"], [], path);
  return {
    segments: u32At(config.segments, `${path}.segments`),
    starts: u32At(config.starts, `${path}.starts`),
    minimum_segment_share: finiteAt(config.minimum_segment_share, `${path}.minimum_segment_share`),
  };
}

function parseRegressionRelationship(value: unknown, path: string): WireRecord {
  const candidate = recordAt(value, path);
  if (candidate.model === "mediation") {
    const relationship = exactRecordAt(candidate, ["model", "x", "mediator"], [], path);
    return { model: "mediation", x: textAt(relationship.x, `${path}.x`, true), mediator: textAt(relationship.mediator, `${path}.mediator`, true) };
  }
  if (candidate.model === "moderation") {
    const relationship = exactRecordAt(candidate, ["model", "x", "moderator"], [], path);
    return { model: "moderation", x: textAt(relationship.x, `${path}.x`, true), moderator: textAt(relationship.moderator, `${path}.moderator`, true) };
  }
  if (candidate.model === "moderated_mediation") {
    const relationship = exactRecordAt(candidate, ["model", "x", "mediator", "moderator"], [], path);
    return {
      model: "moderated_mediation",
      x: textAt(relationship.x, `${path}.x`, true),
      mediator: textAt(relationship.mediator, `${path}.mediator`, true),
      moderator: textAt(relationship.moderator, `${path}.moderator`, true),
    };
  }
  if (candidate.model === "graph") {
    const relationship = exactRecordAt(candidate, [
      "model", "focal_predictor", "paths", "moderators", "moderations", "continuous_product_centering",
    ], [], path);
    return {
      model: "graph",
      focal_predictor: textAt(relationship.focal_predictor, `${path}.focal_predictor`, true),
      paths: arrayAt(relationship.paths, `${path}.paths`).map((item, index) => {
        const itemPath = `${path}.paths[${index}]`;
        const edge = exactRecordAt(item, ["from", "to"], [], itemPath);
        return { from: textAt(edge.from, `${itemPath}.from`, true), to: textAt(edge.to, `${itemPath}.to`, true) };
      }),
      moderators: arrayAt(relationship.moderators, `${path}.moderators`).map((item, index) => {
        const itemPath = `${path}.moderators[${index}]`;
        const moderator = exactRecordAt(item, ["variable", "scale"], [], itemPath);
        return {
          variable: textAt(moderator.variable, `${itemPath}.variable`, true),
          scale: enumAt(moderator.scale, ["continuous", "binary_0_1"] as const, `${itemPath}.scale`),
        };
      }),
      moderations: arrayAt(relationship.moderations, `${path}.moderations`).map((item, index) => {
        const itemPath = `${path}.moderations[${index}]`;
        const moderation = exactRecordAt(item, ["from", "to", "moderator"], ["conditioning_moderator"], itemPath);
        const normalized: WireRecord = {
          from: textAt(moderation.from, `${itemPath}.from`, true),
          to: textAt(moderation.to, `${itemPath}.to`, true),
          moderator: textAt(moderation.moderator, `${itemPath}.moderator`, true),
        };
        if (moderation.conditioning_moderator != null) {
          normalized.conditioning_moderator = textAt(moderation.conditioning_moderator, `${itemPath}.conditioning_moderator`, true);
        }
        return normalized;
      }),
      continuous_product_centering: enumAt(
        relationship.continuous_product_centering,
        ["equation_complete_case_mean_v1"] as const,
        `${path}.continuous_product_centering`,
      ),
    };
  }
  return fail("project_archive_v6.regression_relationship_kind", `${path}.model`, `${path}.model is unsupported.`);
}

function parseRegressionModel(value: unknown, path: string): WireRecord {
  const candidate = recordAt(value, path);
  if (candidate.type === "ols") {
    const model = exactRecordAt(candidate, ["type", "robust_se"], [], path);
    return { type: "ols", robust_se: enumAt(model.robust_se, ["hc3"] as const, `${path}.robust_se`) };
  }
  if (candidate.type === "logistic") {
    exactRecordAt(candidate, ["type"], [], path);
    return { type: "logistic" };
  }
  if (candidate.type === "process") {
    const model = exactRecordAt(candidate, ["type", "relationship"], [], path);
    return { type: "process", relationship: parseRegressionRelationship(model.relationship, `${path}.relationship`) };
  }
  return fail("project_archive_v6.regression_model_kind", `${path}.type`, `${path}.type is unsupported.`);
}

function parseMethodConfig(value: unknown, path: string): WireRecord {
  const candidate = recordAt(value, path);
  const kind = textAt(candidate.kind, `${path}.kind`, true);
  const unitKinds = [
    "pls_algorithm", "pls_bootstrap", "pls_permutation", "plsc", "wpls", "cca", "cta_pls",
    "endogeneity", "nonlinear_effects", "moderated_mediation", "gsca", "legacy",
  ];
  if (unitKinds.includes(kind)) {
    exactRecordAt(candidate, ["kind"], [], path);
    return { kind };
  }
  if (kind === "pls_algorithm_configured_v2") {
    const config = exactRecordAt(candidate, ["kind", "initialization_contract_version", "initial_outer_weights"], [], path);
    const weightsCandidate = recordAt(config.initial_outer_weights, `${path}.initial_outer_weights`);
    let initialOuterWeights: WireRecord;
    if (weightsCandidate.kind === "standard") {
      exactRecordAt(weightsCandidate, ["kind"], [], `${path}.initial_outer_weights`);
      initialOuterWeights = { kind: "standard" };
    } else if (weightsCandidate.kind === "individual") {
      const individual = exactRecordAt(weightsCandidate, ["kind", "weights"], [], `${path}.initial_outer_weights`);
      initialOuterWeights = {
        kind: "individual",
        weights: arrayAt(individual.weights, `${path}.initial_outer_weights.weights`).map((item, index) => {
          const itemPath = `${path}.initial_outer_weights.weights[${index}]`;
          const weight = exactRecordAt(item, ["construct_id", "indicator_id", "value"], [], itemPath);
          return { construct_id: textAt(weight.construct_id, `${itemPath}.construct_id`, true), indicator_id: textAt(weight.indicator_id, `${itemPath}.indicator_id`, true), value: finiteAt(weight.value, `${itemPath}.value`) };
        }),
      };
    } else {
      return fail("project_archive_v6.initial_weights_kind", `${path}.initial_outer_weights.kind`, "Initial outer-weight kind is unsupported.");
    }
    return { kind, initialization_contract_version: textAt(config.initialization_contract_version, `${path}.initialization_contract_version`, true), initial_outer_weights: initialOuterWeights };
  }
  if (kind === "pls_posthoc_technical_minimum_sample_size") {
    const config = exactRecordAt(candidate, ["kind", "capability_cell", "method_version", "base_analysis", "inference"], [], path);
    const cell = exactRecordAt(config.capability_cell, ["registry_schema_version", "capability_id", "cell_id", "capability_version"], [], `${path}.capability_cell`);
    return {
      kind,
      capability_cell: {
        registry_schema_version: u32At(cell.registry_schema_version, `${path}.capability_cell.registry_schema_version`),
        capability_id: textAt(cell.capability_id, `${path}.capability_cell.capability_id`, true),
        cell_id: textAt(cell.cell_id, `${path}.capability_cell.cell_id`, true),
        capability_version: textAt(cell.capability_version, `${path}.capability_cell.capability_version`, true),
      },
      method_version: textAt(config.method_version, `${path}.method_version`, true),
      base_analysis: enumAt(config.base_analysis, ["pls_algorithm", "pls_bootstrap"] as const, `${path}.base_analysis`),
      inference: enumAt(config.inference, ["point_estimate_only", "case_bootstrap_normal_reference_two_sided"] as const, `${path}.inference`),
    };
  }
  if (kind === "pls_sample_size_power") {
    const required = [
      "kind", "scenario_identity", "predictor_construct", "outcome_construct", "predictor_indicator_loadings",
      "outcome_indicator_loadings", "population_path", "exogenous_distribution",
      "structural_disturbance_distribution", "indicator_error_distribution", "missing_data", "inference",
      "sample_size_grid", "alpha", "target_power", "interval_confidence_level", "monte_carlo_replicates",
      "bootstrap_replicates",
    ];
    const config = exactRecordAt(candidate, required, [], path);
    return {
      kind,
      scenario_identity: textAt(config.scenario_identity, `${path}.scenario_identity`, true),
      predictor_construct: textAt(config.predictor_construct, `${path}.predictor_construct`, true),
      outcome_construct: textAt(config.outcome_construct, `${path}.outcome_construct`, true),
      predictor_indicator_loadings: arrayAt(config.predictor_indicator_loadings, `${path}.predictor_indicator_loadings`).map((item, index) => finiteAt(item, `${path}.predictor_indicator_loadings[${index}]`)),
      outcome_indicator_loadings: arrayAt(config.outcome_indicator_loadings, `${path}.outcome_indicator_loadings`).map((item, index) => finiteAt(item, `${path}.outcome_indicator_loadings[${index}]`)),
      population_path: finiteAt(config.population_path, `${path}.population_path`),
      exogenous_distribution: enumAt(config.exogenous_distribution, ["standard_normal"] as const, `${path}.exogenous_distribution`),
      structural_disturbance_distribution: enumAt(config.structural_disturbance_distribution, ["standard_normal"] as const, `${path}.structural_disturbance_distribution`),
      indicator_error_distribution: enumAt(config.indicator_error_distribution, ["standard_normal"] as const, `${path}.indicator_error_distribution`),
      missing_data: enumAt(config.missing_data, ["none"] as const, `${path}.missing_data`),
      inference: enumAt(config.inference, [
        "case_bootstrap_normal_reference_two_sided",
        "case_bootstrap_null_centered_two_sided_plus_one",
      ] as const, `${path}.inference`),
      sample_size_grid: arrayAt(config.sample_size_grid, `${path}.sample_size_grid`).map((item, index) => u32At(item, `${path}.sample_size_grid[${index}]`)),
      alpha: finiteAt(config.alpha, `${path}.alpha`),
      target_power: finiteAt(config.target_power, `${path}.target_power`),
      interval_confidence_level: finiteAt(config.interval_confidence_level, `${path}.interval_confidence_level`),
      monte_carlo_replicates: u32At(config.monte_carlo_replicates, `${path}.monte_carlo_replicates`),
      bootstrap_replicates: u32At(config.bootstrap_replicates, `${path}.bootstrap_replicates`),
    };
  }
  if (kind === "plsc_permutation") {
    const config = exactRecordAt(candidate, ["kind", "group_column", "group_a", "group_b"], ["test_tail"], path);
    const testTail = hasOwn(config, "test_tail")
      ? enumAt(config.test_tail, ["two_sided", "group_a_greater", "group_a_less"] as const, `${path}.test_tail`)
      : "two_sided";
    return {
      kind,
      group_column: textAt(config.group_column, `${path}.group_column`, true),
      group_a: textAt(config.group_a, `${path}.group_a`, true),
      group_b: textAt(config.group_b, `${path}.group_b`, true),
      ...(testTail === "two_sided" ? {} : { test_tail: testTail }),
    };
  }
  if (kind === "predict") {
    const config = exactRecordAt(candidate, ["kind"], ["pls_pos", "fimix"], path);
    const normalized: WireRecord = { kind };
    if (config.pls_pos != null) normalized.pls_pos = parseSegmentationConfig(config.pls_pos, `${path}.pls_pos`);
    if (config.fimix != null) normalized.fimix = parseSegmentationConfig(config.fimix, `${path}.fimix`);
    return normalized;
  }
  if (kind === "mga") {
    const config = exactRecordAt(candidate, ["kind", "group_column", "group_a", "group_b", "methods", "permutation_samples", "configural_invariance_confirmed"], [], path);
    return {
      kind,
      group_column: textAt(config.group_column, `${path}.group_column`, true),
      group_a: textAt(config.group_a, `${path}.group_a`, true),
      group_b: textAt(config.group_b, `${path}.group_b`, true),
      methods: arrayAt(config.methods, `${path}.methods`).map((item, index) => enumAt(item, ["micom", "mga_permutation"] as const, `${path}.methods[${index}]`)),
      permutation_samples: u32At(config.permutation_samples, `${path}.permutation_samples`),
      configural_invariance_confirmed: booleanAt(config.configural_invariance_confirmed, `${path}.configural_invariance_confirmed`),
    };
  }
  if (kind === "micom") {
    const config = exactRecordAt(candidate, ["kind", "group_column", "group_a", "group_b", "permutation_samples", "configural_invariance_confirmed"], [], path);
    return { kind, group_column: textAt(config.group_column, `${path}.group_column`, true), group_a: textAt(config.group_a, `${path}.group_a`, true), group_b: textAt(config.group_b, `${path}.group_b`, true), permutation_samples: u32At(config.permutation_samples, `${path}.permutation_samples`), configural_invariance_confirmed: booleanAt(config.configural_invariance_confirmed, `${path}.configural_invariance_confirmed`) };
  }
  if (kind === "ipma") {
    const config = exactRecordAt(candidate, ["kind", "targets"], [], path);
    return { kind, targets: stringArrayAt(config.targets, `${path}.targets`) };
  }
  if (kind === "cbsem") {
    const config = exactRecordAt(candidate, ["kind", "model_type", "estimator", "input", "mean_structure", "bootstrap_samples"], ["bootstrap_v2", "group_column", "invariance_steps"], path);
    const normalized: WireRecord = {
      kind,
      model_type: enumAt(config.model_type, ["cfa", "sem"] as const, `${path}.model_type`),
      estimator: enumAt(config.estimator, ["ml", "robust_ml", "wlsmv"] as const, `${path}.estimator`),
      input: enumAt(config.input, ["raw", "covariance", "correlation"] as const, `${path}.input`),
      mean_structure: booleanAt(config.mean_structure, `${path}.mean_structure`),
      bootstrap_samples: u32At(config.bootstrap_samples, `${path}.bootstrap_samples`),
    };
    if (config.bootstrap_v2 != null) {
      const bootstrap = exactRecordAt(config.bootstrap_v2, ["algorithm", "interval"], ["test_tail"], `${path}.bootstrap_v2`);
      const testTail = hasOwn(bootstrap, "test_tail")
        ? enumAt(bootstrap.test_tail, ["two_sided", "one_sided_greater", "one_sided_less"] as const, `${path}.bootstrap_v2.test_tail`)
        : "two_sided";
      normalized.bootstrap_v2 = {
        algorithm: enumAt(bootstrap.algorithm, ["case_resampling_full_ml"] as const, `${path}.bootstrap_v2.algorithm`),
        interval: enumAt(bootstrap.interval, ["percentile_type7"] as const, `${path}.bootstrap_v2.interval`),
        ...(testTail === "two_sided" ? {} : { test_tail: testTail }),
      };
    }
    if (config.group_column != null) normalized.group_column = textAt(config.group_column, `${path}.group_column`, true);
    const invarianceSteps = hasOwn(config, "invariance_steps")
      ? arrayAt(config.invariance_steps, `${path}.invariance_steps`).map((item, index) => enumAt(item, ["configural", "metric", "scalar"] as const, `${path}.invariance_steps[${index}]`))
      : [];
    if (invarianceSteps.length) normalized.invariance_steps = invarianceSteps;
    return normalized;
  }
  if (kind === "pca") {
    const config = exactRecordAt(candidate, ["kind", "variables", "retention"], [], path);
    const retentionCandidate = recordAt(config.retention, `${path}.retention`);
    let retention: WireRecord;
    if (retentionCandidate.rule === "kaiser") {
      exactRecordAt(retentionCandidate, ["rule"], [], `${path}.retention`);
      retention = { rule: "kaiser" };
    } else if (retentionCandidate.rule === "fixed") {
      const fixed = exactRecordAt(retentionCandidate, ["rule", "components"], [], `${path}.retention`);
      retention = { rule: "fixed", components: u32At(fixed.components, `${path}.retention.components`) };
    } else if (retentionCandidate.rule === "variance_threshold") {
      const threshold = exactRecordAt(retentionCandidate, ["rule", "threshold"], [], `${path}.retention`);
      retention = { rule: "variance_threshold", threshold: finiteAt(threshold.threshold, `${path}.retention.threshold`) };
    } else {
      return fail("project_archive_v6.pca_retention_kind", `${path}.retention.rule`, "PCA retention rule is unsupported.");
    }
    return { kind, variables: stringArrayAt(config.variables, `${path}.variables`), retention };
  }
  if (kind === "regression") {
    const config = exactRecordAt(candidate, ["kind", "outcome", "predictors", "model"], ["controls", "bootstrap"], path);
    const normalized: WireRecord = {
      kind,
      outcome: textAt(config.outcome, `${path}.outcome`, true),
      predictors: stringArrayAt(config.predictors, `${path}.predictors`),
      model: parseRegressionModel(config.model, `${path}.model`),
    };
    const controls = hasOwn(config, "controls") ? stringArrayAt(config.controls, `${path}.controls`) : [];
    if (controls.length) normalized.controls = controls;
    if (config.bootstrap != null) {
      const bootstrap = exactRecordAt(config.bootstrap, ["algorithm", "intervals"], [], `${path}.bootstrap`);
      normalized.bootstrap = {
        algorithm: enumAt(bootstrap.algorithm, ["case_resampling"] as const, `${path}.bootstrap.algorithm`),
        intervals: arrayAt(bootstrap.intervals, `${path}.bootstrap.intervals`).map((item, index) => enumAt(item, ["percentile", "bca"] as const, `${path}.bootstrap.intervals[${index}]`)),
      };
    }
    return normalized;
  }
  if (kind === "nca") {
    const config = exactRecordAt(candidate, ["kind", "condition", "outcome", "ceiling", "permutation_samples"], [], path);
    return { kind, condition: textAt(config.condition, `${path}.condition`, true), outcome: textAt(config.outcome, `${path}.outcome`, true), ceiling: enumAt(config.ceiling, ["ce_fdh", "cr_fdh", "both"] as const, `${path}.ceiling`), permutation_samples: u32At(config.permutation_samples, `${path}.permutation_samples`) };
  }
  return fail("project_archive_v6.method_config_kind", `${path}.kind`, `${path}.kind is unsupported.`);
}

function parseRecipe(value: unknown, path: string): ProjectAnalysisRecipeV4Wire {
  const recipe = exactRecordAt(value, [
    "schema_version",
    "id",
    "created_at",
    "dataset_fingerprint",
    "model_binding",
    "estimand_confirmation",
    "settings",
  ], ["method_config", "metadata", "legacy_source"], path);
  if (u32At(recipe.schema_version, `${path}.schema_version`) !== 4) {
    fail("project_archive_v6.recipe_schema", `${path}.schema_version`, "Current project recipes must use schema version 4.");
  }
  const binding = parseRecipeModelBinding(recipe.model_binding, `${path}.model_binding`);
  const confirmation = enumAt(recipe.estimand_confirmation, [
    "not_legacy",
    "legacy_estimand_unspecified",
    "confirmed_composite",
    "confirmed_common_factor",
  ] as const, `${path}.estimand_confirmation`);
  const bindingMatchesConfirmation = binding.kind === "legacy_estimand_unspecified"
    ? confirmation === "legacy_estimand_unspecified"
    : confirmation !== "legacy_estimand_unspecified";
  if (!bindingMatchesConfirmation) {
    fail("project_archive_v6.recipe_estimand_binding", path, "Recipe estimand confirmation does not match its model binding.");
  }
  const legacySource = recipe.legacy_source == null ? null : exactRecordAt(
    recipe.legacy_source,
    ["source_schema_version", "source_recipe_sha256"],
    [],
    `${path}.legacy_source`,
  );
  if (legacySource) {
    const sourceVersion = u32At(legacySource.source_schema_version, `${path}.legacy_source.source_schema_version`);
    if (sourceVersion < 1 || sourceVersion > 3) {
      fail("project_archive_v6.recipe_legacy_source_schema", `${path}.legacy_source.source_schema_version`, "Recipe legacy source schema must be 1 through 3.");
    }
    sha256At(legacySource.source_recipe_sha256, `${path}.legacy_source.source_recipe_sha256`);
  }
  const normalized: ProjectAnalysisRecipeV4Wire = {
    schema_version: 4,
    id: uuidAt(recipe.id, `${path}.id`),
    created_at: timestampAt(recipe.created_at, `${path}.created_at`),
    dataset_fingerprint: textAt(recipe.dataset_fingerprint, `${path}.dataset_fingerprint`),
    model_binding: binding,
    estimand_confirmation: confirmation,
    settings: parseRecipeSettings(recipe.settings, `${path}.settings`),
    metadata: recipe.metadata === undefined ? {} : stringMapAt(recipe.metadata, `${path}.metadata`),
    ...(recipe.method_config == null ? {} : { method_config: parseMethodConfig(recipe.method_config, `${path}.method_config`) }),
    ...(legacySource == null ? {} : {
      legacy_source: {
        source_schema_version: u32At(legacySource.source_schema_version, `${path}.legacy_source.source_schema_version`) as 1 | 2 | 3,
        source_recipe_sha256: sha256At(legacySource.source_recipe_sha256, `${path}.legacy_source.source_recipe_sha256`),
      },
    }),
  };
  return normalized;
}

function validateRecipeModelReference(
  recipe: ProjectAnalysisRecipeV4Wire,
  scientificModels: ReadonlyMap<string, string>,
  pendingModels: ReadonlyMap<string, string>,
  path: string,
): void {
  const binding = recipe.model_binding;
  if (binding.kind === "project_sem_model_v4_reference") {
    if (scientificModels.get(binding.model_id) !== binding.scientific_sha256) {
      fail("project_archive_v6.recipe_model_reference", path, "Recipe references an unavailable SemModelV4 or a mismatched scientific digest.");
    }
  } else if (binding.kind === "legacy_estimand_unspecified"
    && pendingModels.get(binding.legacy_model_id) !== binding.legacy_model_sha256) {
    fail("project_archive_v6.recipe_model_reference", path, "Recipe references an unavailable legacy model or a mismatched legacy digest.");
  }
}

export interface CanonicalResultDocumentAttachmentV2Wire {
  readonly document_id: string;
  readonly run_id: string;
  readonly document_schema_version: 2;
  readonly canonical_document: CanonicalResultDocumentV2;
  readonly canonical_document_sha256: string;
  readonly immutable: true;
}

function parseCanonicalAttachment(value: unknown, projectId: string, path: string): CanonicalResultDocumentAttachmentV2Wire {
  const attachment = exactRecordAt(value, [
    "document_id",
    "run_id",
    "document_schema_version",
    "canonical_document",
    "canonical_document_sha256",
    "immutable",
  ], [], path);
  const document = attachment.canonical_document as CanonicalResultDocumentV2;
  let validation;
  try {
    validation = validateCanonicalResultDocumentV2(document);
  } catch {
    return fail("project_archive_v6.canonical_document_shape", `${path}.canonical_document`, "Canonical result document has an invalid wire shape.");
  }
  if (!validation.passed) {
    fail("project_archive_v6.canonical_document_invalid", `${path}.canonical_document`, validation.errors.join("; "));
  }
  const documentId = textAt(attachment.document_id, `${path}.document_id`);
  const runId = textAt(attachment.run_id, `${path}.run_id`);
  if (attachment.document_schema_version !== 2
    || document.schema_version !== 2
    || document.document_id !== documentId
    || document.provenance.run_id !== runId
    || document.provenance.project_id !== projectId
    || attachment.immutable !== true) {
    fail("project_archive_v6.canonical_attachment_identity", path, "Canonical result attachment identity, project, schema, or immutability differs from its document.");
  }
  return {
    document_id: documentId,
    run_id: runId,
    document_schema_version: 2,
    canonical_document: document,
    canonical_document_sha256: sha256At(attachment.canonical_document_sha256, `${path}.canonical_document_sha256`),
    immutable: true,
  };
}

export interface InternalProjectArchiveV6Wire {
  schema_version: 6;
  project_id: string;
  name: string;
  created_at: string;
  modified_at: string;
  datasets: ProjectDatasetDescriptorV6Wire[];
  models: ProjectModelRecordV6Wire[];
  recipes: ProjectAnalysisRecipeV4Wire[];
  historical_recipes: ImmutableHistoricalRecipeV6Wire[];
  layouts: Record<string, unknown>;
  historical_results: ImmutableHistoricalResultV6Wire[];
  canonical_result_documents: CanonicalResultDocumentAttachmentV2Wire[];
  origin: ProjectOriginV6Wire;
}

/**
 * Strict compatibility reader for the frozen schema-v6 archive wire.
 *
 * Rust remains the digest authority. JavaScript only validates digest syntax
 * and exact cross-record equality because parsing JSON loses numeric lexical
 * distinctions that participate in Rust's canonical hashes.
 */
export function parseInternalProjectArchiveV6Wire(input: unknown): InternalProjectArchiveV6Wire {
  const rootCandidate = recordAt(input, "project");
  const access = classifyInternalProjectArchiveSchemaV6(rootCandidate.schema_version);
  if (access === "historical_upgrade_copy_required") {
    fail("project_archive_v6.historical_upgrade_required", "project.schema_version", "Historical schemas 1 through 5 require a source-preserving schema-v6 upgrade copy.");
  }
  if (access === "future_read_only") {
    fail("project_archive_v6.future_read_only", "project.schema_version", "Future project schemas are opaque and read-only in this build.");
  }

  const root = exactRecordAt(rootCandidate, [
    "schema_version",
    "project_id",
    "name",
    "created_at",
    "modified_at",
  ], [
    "datasets",
    "models",
    "recipes",
    "historical_recipes",
    "layouts",
    "historical_results",
    "canonical_result_documents",
    "origin",
    "upgrade_lineage",
  ], "project");
  // These are Option<T> fields in the compatibility-only Rust reader, so an
  // explicit JSON null has the same presence semantics as an omitted field.
  const hasOrigin = hasOwn(root, "origin") && root.origin != null;
  const hasLegacyLineage = hasOwn(root, "upgrade_lineage") && root.upgrade_lineage != null;
  if (hasOrigin === hasLegacyLineage) {
    fail("project_archive_v6.origin_ambiguous", "project", "A schema-v6 project must contain exactly one of origin or legacy upgrade_lineage.");
  }
  const projectId = uuidAt(root.project_id, "project.project_id");
  const origin = hasOrigin
    ? parseOrigin(root.origin, "project.origin")
    : { kind: "upgraded_copy", lineage: parseUpgradeLineage(root.upgrade_lineage, "project.upgrade_lineage") } as const;
  if (origin.kind === "upgraded_copy" && origin.lineage.source_project_id !== projectId) {
    fail("project_archive_v6.upgrade_project_identity", "project.origin", "Upgraded-copy source_project_id must match project_id.");
  }

  const datasets = arrayAt(hasOwn(root, "datasets") ? root.datasets : [], "project.datasets")
    .map((dataset, index) => parseDataset(dataset, `project.datasets[${index}]`));
  const models = arrayAt(hasOwn(root, "models") ? root.models : [], "project.models")
    .map((model, index) => parseModelRecord(model, `project.models[${index}]`));
  const modelIds = new Set<string>();
  const scientificModels = new Map<string, string>();
  const pendingModels = new Map<string, string>();
  models.forEach((record, index) => {
    if (modelIds.has(record.model_id)) {
      fail("project_archive_v6.model_id_duplicate", `project.models[${index}].model_id`, "Project model ids must be unique.");
    }
    modelIds.add(record.model_id);
    if (record.payload.kind === "sem_model_v4") scientificModels.set(record.model_id, record.payload.scientific_sha256);
    if (record.payload.kind === "legacy_estimand_unspecified") pendingModels.set(record.model_id, record.payload.legacy_model_sha256);
  });

  const historicalRecipes = arrayAt(hasOwn(root, "historical_recipes") ? root.historical_recipes : [], "project.historical_recipes")
    .map((recipe, index) => parseHistoricalRecipe(recipe, `project.historical_recipes[${index}]`));
  const recipeIds = new Set<string>();
  const historicalRecipeMap = new Map<string, ImmutableHistoricalRecipeV6Wire>();
  historicalRecipes.forEach((recipe, index) => {
    if (recipeIds.has(recipe.recipe_id)) {
      fail("project_archive_v6.recipe_id_duplicate", `project.historical_recipes[${index}].recipe_id`, "Current and historical recipe ids must be unique together.");
    }
    recipeIds.add(recipe.recipe_id);
    historicalRecipeMap.set(recipe.recipe_id, recipe);
  });

  const recipes = arrayAt(hasOwn(root, "recipes") ? root.recipes : [], "project.recipes")
    .map((recipe, index) => parseRecipe(recipe, `project.recipes[${index}]`));
  recipes.forEach((recipe, index) => {
    if (recipeIds.has(recipe.id)) {
      fail("project_archive_v6.recipe_id_duplicate", `project.recipes[${index}].id`, "Current and historical recipe ids must be unique together.");
    }
    recipeIds.add(recipe.id);
    validateRecipeModelReference(recipe, scientificModels, pendingModels, `project.recipes[${index}].model_binding`);
  });

  const historicalResults = arrayAt(hasOwn(root, "historical_results") ? root.historical_results : [], "project.historical_results")
    .map((result, index) => parseHistoricalResult(result, `project.historical_results[${index}]`));
  const sourceProvenanceCanBind = origin.kind === "new_project"
    || origin.lineage.source_archive_schema_version >= 3;
  const resultIds = new Set<string>();
  historicalResults.forEach((result, index) => {
    if (resultIds.has(result.result_id)) {
      fail("project_archive_v6.result_id_duplicate", `project.historical_results[${index}].result_id`, "Historical result ids must be unique.");
    }
    resultIds.add(result.result_id);
    validateHistoricalResultBinding(result, historicalRecipeMap, sourceProvenanceCanBind, `project.historical_results[${index}].source_recipe`);
  });

  const canonicalAttachments = arrayAt(hasOwn(root, "canonical_result_documents") ? root.canonical_result_documents : [], "project.canonical_result_documents")
    .map((attachment, index) => parseCanonicalAttachment(attachment, projectId, `project.canonical_result_documents[${index}]`));
  const canonicalDocumentIds = new Set<string>();
  const canonicalRunIds = new Set<string>();
  canonicalAttachments.forEach((attachment, index) => {
    if (canonicalDocumentIds.has(attachment.document_id)) {
      fail("project_archive_v6.canonical_document_id_duplicate", `project.canonical_result_documents[${index}].document_id`, "Canonical document ids must be unique.");
    }
    if (canonicalRunIds.has(attachment.run_id)) {
      fail("project_archive_v6.canonical_run_id_duplicate", `project.canonical_result_documents[${index}].run_id`, "Canonical run ids must be unique.");
    }
    canonicalDocumentIds.add(attachment.document_id);
    canonicalRunIds.add(attachment.run_id);
  });

  return {
    schema_version: 6,
    project_id: projectId,
    name: textAt(root.name, "project.name", true),
    created_at: timestampAt(root.created_at, "project.created_at"),
    modified_at: timestampAt(root.modified_at, "project.modified_at"),
    datasets,
    models,
    recipes,
    historical_recipes: historicalRecipes,
    layouts: { ...recordAt(hasOwn(root, "layouts") ? root.layouts : {}, "project.layouts") },
    historical_results: historicalResults,
    canonical_result_documents: canonicalAttachments,
    origin,
  };
}
