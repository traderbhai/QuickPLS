import type { CapabilityCellReferenceV2 } from "./canonicalResultDocumentV2";
import { sha256HexBytesV1, sha256HexUtf8V1 } from "./sha256V1";

export const CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION = 1 as const;
export const GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1 = "general_sem_pls_full_model_case_bootstrap_v1" as const;
export const GENERAL_SEM_PLS_CASE_BOOTSTRAP_OPERATION_VERSION_V1 = "general_sem_pls_case_bootstrap_v1" as const;
export const GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1 = "indexed_case_resampling_v1" as const;
export const GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1 = "type7_quantile_v1" as const;
export const GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1 = "sample_standard_error_b_minus_1_v1" as const;
export const GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1 = "neumaier_compensated_sum_v1" as const;
export const GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1 = "null_centered_plus_one_v1" as const;
export const GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1 = "minimum_usable_fraction_0_9_v1" as const;

export interface CanonicalGeneralSemResultTraceV1 {
  model_id: string;
  capability_cell: CapabilityCellReferenceV2;
}

export interface CanonicalGeneralSemEstimateV1 {
  estimate: number;
  bootstrap_mean?: number | null;
  bootstrap_bias?: number | null;
  standard_error?: number | null;
  lower?: number | null;
  upper?: number | null;
  p_value?: number | null;
  bootstrap_usable_replicates?: number | null;
  bootstrap_two_sided_exceedances?: number | null;
}

export type CanonicalGeneralSemInferenceKindV1 = "case_bootstrap";
export type CanonicalGeneralSemBootstrapIntervalV1 = "percentile_type7" | "bca";
export type CanonicalGeneralSemInferenceTailV1 = "two_sided" | "one_sided_lower" | "one_sided_upper";

export type CanonicalGeneralSemFailedReplicateReasonV1 =
  | "insufficient_observations"
  | "constant_indicator"
  | "rank_deficient"
  | "isolated_construct"
  | "estimation_nonconvergence"
  | "numerical_failure";

export interface CanonicalGeneralSemFailedReplicateV1 {
  replicate_index: number;
  reason_code: CanonicalGeneralSemFailedReplicateReasonV1;
  message: string;
}

export interface CanonicalGeneralSemInferenceReceiptV1 {
  kind: CanonicalGeneralSemInferenceKindV1;
  capability_cell: CapabilityCellReferenceV2;
  method_version: typeof GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1;
  resampling_operation_version: typeof GENERAL_SEM_PLS_CASE_BOOTSTRAP_OPERATION_VERSION_V1;
  resampling_stream_version: typeof GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1;
  quantile_method_version: typeof GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1;
  standard_error_method_version: typeof GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1;
  summation_method_version: typeof GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1;
  p_value_method_version: typeof GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1;
  failure_policy_version: typeof GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1;
  compilation_artifact_identity_sha256: string;
  compiled_plan_sha256: string;
  general_sem_config_sha256: string;
  recipe_analytical_sha256: string;
  model_scientific_sha256: string;
  source_dataset_fingerprint: string;
  complete_case_frame_sha256: string;
  usable_replicate_indices_sha256: string;
  effect_identity_set_sha256: string;
  effect_ids: string[];
  interval: CanonicalGeneralSemBootstrapIntervalV1;
  tail: CanonicalGeneralSemInferenceTailV1;
  confidence_level: number;
  resamples_requested: number;
  resamples_usable: number;
  minimum_usable_resamples: number;
  /** Canonical decimal wire form bounded to JavaScript's nonnegative safe-integer range. */
  seed: string;
  workers: number;
  complete_model_reestimated_per_replicate: boolean;
  failed_replicates: CanonicalGeneralSemFailedReplicateV1[];
}

export interface CanonicalSpecificIndirectEffectResultV1 {
  effect_id: string;
  estimand_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  source_id: string;
  target_id: string;
  ordered_relation_ids: string[];
  value: CanonicalGeneralSemEstimateV1;
}

export type CanonicalAggregateEffectKindV1 = "total_indirect" | "total_effect";

export interface CanonicalAggregateEffectResultV1 {
  effect_id: string;
  estimand_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  kind: CanonicalAggregateEffectKindV1;
  source_id: string;
  target_id: string;
  direct_relation_ids: string[];
  contributing_path_identities: string[];
  value: CanonicalGeneralSemEstimateV1;
}

export type CanonicalGeneralSemEffectIdentityV1 =
  | {
      kind: "specific_indirect";
      effect_id: string;
      estimand_id: string;
      source_id: string;
      target_id: string;
      ordered_relation_ids: string[];
    }
  | {
      kind: "total_indirect";
      effect_id: string;
      estimand_id: string;
      source_id: string;
      target_id: string;
      contributing_path_identities: string[];
    }
  | {
      kind: "total_effect";
      effect_id: string;
      estimand_id: string;
      source_id: string;
      target_id: string;
      direct_relation_ids: string[];
      contributing_path_identities: string[];
    };

export type CanonicalConditionalProbeValuesResultV1 =
  | { kind: "data_derived_mean_plus_minus_one_sd"; mean: number; standard_deviation: number }
  | { kind: "explicit"; values: number[] };

export interface CanonicalConditionalEffectProbeResultV1 {
  probe_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  moderator_id: string;
  values: CanonicalConditionalProbeValuesResultV1;
}

export interface CanonicalConditionalEffectResultV1 {
  effect_id: string;
  estimand_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  interaction_id: string;
  focal_relation_id: string;
  probe_id: string;
  moderator_id: string;
  probe_value_index: number;
  moderator_value: number;
  value: CanonicalGeneralSemEstimateV1;
}

export interface CanonicalInteractionPlotPointV1 {
  focal_value: number;
  predicted_value: number;
  lower?: number | null;
  upper?: number | null;
}

export interface CanonicalInteractionPlotSeriesV1 {
  series_id: string;
  probe_id: string;
  probe_value_index: number;
  moderator_value: number;
  points: CanonicalInteractionPlotPointV1[];
}

export interface CanonicalInteractionPlotResultV1 {
  plot_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  interaction_id: string;
  focal_relation_id: string;
  focal_predictor_id: string;
  moderator_id: string;
  outcome_id: string;
  series: CanonicalInteractionPlotSeriesV1[];
}

export type CanonicalHocStageKindV1 =
  | "lower_order_score_estimation"
  | "higher_order_estimation";

export interface CanonicalHocRelationEstimateV1 {
  relation_id: string;
  source_id: string;
  target_id: string;
  value: CanonicalGeneralSemEstimateV1;
}

export interface CanonicalHocStageResultV1 {
  stage_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  higher_order_construct_id: string;
  stage_number: number;
  kind: CanonicalHocStageKindV1;
  input_construct_ids: string[];
  output_variable_ids: string[];
  relation_estimates?: CanonicalHocRelationEstimateV1[];
}

export interface CanonicalGeneralSemIntervalV1 {
  confidence_level: number;
  lower: number;
  upper: number;
}

export interface CanonicalCbsemFitResultV1 {
  fit_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  chi_square: number;
  degrees_of_freedom: number;
  chi_square_p_value?: number | null;
  rmsea?: number | null;
  rmsea_interval?: CanonicalGeneralSemIntervalV1 | null;
  cfi?: number | null;
  tli?: number | null;
  srmr?: number | null;
  aic?: number | null;
  bic?: number | null;
}

export type CanonicalIdentificationScopeV1 =
  | "model"
  | "variable"
  | "relation"
  | "interaction"
  | "higher_order_construct";

export type CanonicalIdentificationStatusV1 =
  | "identified"
  | "underidentified"
  | "locally_underidentified"
  | "boundary_condition";

export interface CanonicalIdentificationDiagnosticV1 {
  diagnostic_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  scope: CanonicalIdentificationScopeV1;
  subject_id: string;
  status: CanonicalIdentificationStatusV1;
  code: string;
  message: string;
  degrees_of_freedom?: number | null;
}

/** Empty collections are omitted by Rust, so every result family is optional on the wire. */
export interface CanonicalGeneralSemResultsV1 {
  schema_version: typeof CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION;
  inference_receipt?: CanonicalGeneralSemInferenceReceiptV1 | null;
  specific_indirect_effects?: CanonicalSpecificIndirectEffectResultV1[];
  aggregate_effects?: CanonicalAggregateEffectResultV1[];
  conditional_effect_probes?: CanonicalConditionalEffectProbeResultV1[];
  conditional_effects?: CanonicalConditionalEffectResultV1[];
  interaction_plots?: CanonicalInteractionPlotResultV1[];
  higher_order_stages?: CanonicalHocStageResultV1[];
  cbsem_fit?: CanonicalCbsemFitResultV1[];
  identification_diagnostics?: CanonicalIdentificationDiagnosticV1[];
}

export interface CanonicalGeneralSemResultsV1Context {
  modelId: string;
  modelDigest: string;
  datasetFingerprint: string;
  recipeDigest: string;
  seed: number | null;
  workers: number;
  capabilityCells: readonly CapabilityCellReferenceV2[];
}

export type CanonicalGeneralSemResultsV1ParseErrorCode =
  | "schema.invalid_shape"
  | "schema.unknown_field"
  | "schema.invalid_discriminator"
  | "schema.version_unsupported"
  | "schema.non_finite"
  | "schema.integer_invalid"
  | "document.invalid";

export class CanonicalGeneralSemResultsV1ParseError extends Error {
  constructor(
    public readonly code: CanonicalGeneralSemResultsV1ParseErrorCode,
    public readonly path: string,
    message: string,
  ) {
    super(message);
    this.name = "CanonicalGeneralSemResultsV1ParseError";
  }
}

type StrictWireRecord = Record<string, unknown>;

const GENERAL_SEM_STABLE_ID = /^[a-z0-9][a-z0-9_.:-]*$/;
const GENERAL_SEM_LOWERCASE_SHA256 = /^[a-f0-9]{64}$/;
const GENERAL_SEM_DATASET_FINGERPRINT_V1 = /^(?:v2:)?[a-f0-9]{64}$/;
const GENERAL_SEM_CANONICAL_DECIMAL_U64 = /^(?:0|[1-9][0-9]*)$/;
const GENERAL_SEM_MAX_SAFE_SEED = 9_007_199_254_740_991n;
const GENERAL_SEM_PLS_RECURSIVE_EFFECTS_CAPABILITY_CELL_V1: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.mediation",
  cell_id: "qpls3.pls.mediation",
  capability_version: "pls_mediation_v1",
};
const GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.mediation",
  cell_id: "qpls3.pls.general_sem_multiple_mediation_bootstrap",
  capability_version: "general_sem_pls_full_model_case_bootstrap_v1",
};

function capabilityCellIdentity(reference: CapabilityCellReferenceV2): string {
  return `${reference.registry_schema_version}:${reference.capability_id}:${reference.cell_id}:${reference.capability_version}`;
}

function hasNonFiniteNumber(value: unknown, seen = new Set<object>()): boolean {
  if (typeof value === "number") return !Number.isFinite(value);
  if (!value || typeof value !== "object") return false;
  if (seen.has(value)) return true;
  seen.add(value);
  const invalid = Object.values(value).some((child) => hasNonFiniteNumber(child, seen));
  seen.delete(value);
  return invalid;
}

function generalSemSerializedSha256(value: unknown): string {
  return sha256HexUtf8V1(JSON.stringify(value));
}

function generalSemSpecificDirectedPathIdentityV1(relationIds: readonly string[]): string {
  const encoder = new TextEncoder();
  const domain = encoder.encode("qpls.compiled-sem-topology-v1.specific-directed-path\0");
  const encodedIds = relationIds.map((relationId) => encoder.encode(relationId));
  const totalLength = domain.length + encodedIds.reduce((total, bytes) => total + 8 + bytes.length, 0);
  const identityInput = new Uint8Array(totalLength);
  identityInput.set(domain);
  let offset = domain.length;
  for (const bytes of encodedIds) {
    const lengthView = new DataView(identityInput.buffer, offset, 8);
    const length = BigInt(bytes.length);
    lengthView.setUint32(0, Number(length >> 32n), false);
    lengthView.setUint32(4, Number(length & 0xffff_ffffn), false);
    offset += 8;
    identityInput.set(bytes, offset);
    offset += bytes.length;
  }
  return `sem_specific_path_v1_${sha256HexBytesV1(identityInput)}`;
}

function wireFail(
  code: CanonicalGeneralSemResultsV1ParseErrorCode,
  path: string,
  message: string,
): never {
  throw new CanonicalGeneralSemResultsV1ParseError(code, path, message);
}

function strictWireRecord(value: unknown, path: string): StrictWireRecord {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return wireFail("schema.invalid_shape", path, `${path} must be an object.`);
  }
  return value as StrictWireRecord;
}

function exactWireRecord(
  value: unknown,
  required: readonly string[],
  optional: readonly string[],
  path: string,
): StrictWireRecord {
  const record = strictWireRecord(value, path);
  const allowed = new Set([...required, ...optional]);
  const unknown = Object.keys(record).find((key) => !allowed.has(key));
  if (unknown) return wireFail("schema.unknown_field", `${path}.${unknown}`, `${path}.${unknown} is not supported.`);
  const missing = required.find((key) => !Object.prototype.hasOwnProperty.call(record, key));
  if (missing) return wireFail("schema.invalid_shape", `${path}.${missing}`, `${path}.${missing} is required.`);
  return record;
}

function wireArray(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) return wireFail("schema.invalid_shape", path, `${path} must be an array.`);
  return value;
}

function optionalWireArray(record: StrictWireRecord, key: string, path: string): unknown[] {
  if (!Object.prototype.hasOwnProperty.call(record, key)) return [];
  return wireArray(record[key], `${path}.${key}`);
}

function wireText(value: unknown, path: string): string {
  if (typeof value !== "string" || !value.trim()) {
    return wireFail("schema.invalid_shape", path, `${path} must be nonempty text.`);
  }
  return value;
}

function wireStableId(value: unknown, path: string): string {
  const id = wireText(value, path);
  if (!GENERAL_SEM_STABLE_ID.test(id)) {
    return wireFail("document.invalid", path, `${path} must be a stable lowercase identifier.`);
  }
  return id;
}

function wireFinite(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return wireFail("schema.non_finite", path, `${path} must be a finite number.`);
  }
  return value;
}

function optionalWireFinite(record: StrictWireRecord, key: string, path: string): number | null | undefined {
  if (!Object.prototype.hasOwnProperty.call(record, key)) return undefined;
  if (record[key] === null) return null;
  return wireFinite(record[key], `${path}.${key}`);
}

function wireU32(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0 || (value as number) > 0xffff_ffff) {
    return wireFail("schema.integer_invalid", path, `${path} must be an unsigned 32-bit integer.`);
  }
  return value as number;
}

function optionalWireSafeInteger(record: StrictWireRecord, key: string, path: string): number | null | undefined {
  if (!Object.prototype.hasOwnProperty.call(record, key)) return undefined;
  if (record[key] === null) return null;
  if (!Number.isSafeInteger(record[key])) {
    return wireFail("schema.integer_invalid", `${path}.${key}`, `${path}.${key} must be a safe integer.`);
  }
  return record[key] as number;
}

function optionalWireU32(record: StrictWireRecord, key: string, path: string): number | null | undefined {
  if (!Object.prototype.hasOwnProperty.call(record, key)) return undefined;
  if (record[key] === null) return null;
  return wireU32(record[key], `${path}.${key}`);
}

function wireEnum<T extends string>(value: unknown, values: readonly T[], path: string): T {
  if (typeof value !== "string" || !values.includes(value as T)) {
    return wireFail("schema.invalid_discriminator", path, `${path} has an unsupported discriminator.`);
  }
  return value as T;
}

function validateWireCapabilityCell(value: unknown, path: string): CapabilityCellReferenceV2 {
  const cell = exactWireRecord(
    value,
    ["registry_schema_version", "capability_id", "cell_id", "capability_version"],
    [],
    path,
  );
  if (cell.registry_schema_version !== 2) {
    return wireFail("schema.version_unsupported", `${path}.registry_schema_version`, `${path}.registry_schema_version must equal 2.`);
  }
  wireStableId(cell.capability_id, `${path}.capability_id`);
  wireStableId(cell.cell_id, `${path}.cell_id`);
  wireText(cell.capability_version, `${path}.capability_version`);
  return value as CapabilityCellReferenceV2;
}

function validateCanonicalWireIds(
  values: readonly unknown[],
  key: string,
  path: string,
): string[] {
  const ids = values.map((value, index) => wireStableId(
    strictWireRecord(value, `${path}[${index}]`)[key],
    `${path}[${index}].${key}`,
  ));
  if (new Set(ids).size !== ids.length) {
    wireFail("document.invalid", path, `${path} contains duplicate stable identifiers.`);
  }
  const sorted = [...ids].sort();
  if (!ids.every((id, index) => id === sorted[index])) {
    wireFail("document.invalid", path, `${path} must be ordered by exact stable identifier.`);
  }
  return ids;
}

function validateStableIdArray(
  value: unknown,
  path: string,
  options: { minimum?: number; canonical?: boolean } = {},
): string[] {
  const values = wireArray(value, path).map((item, index) => wireStableId(item, `${path}[${index}]`));
  if (values.length < (options.minimum ?? 0)) {
    wireFail("document.invalid", path, `${path} requires at least ${options.minimum ?? 0} values.`);
  }
  if (new Set(values).size !== values.length) {
    wireFail("document.invalid", path, `${path} must not contain duplicate identifiers.`);
  }
  if (options.canonical) {
    const sorted = [...values].sort();
    if (!values.every((id, index) => id === sorted[index])) {
      wireFail("document.invalid", path, `${path} must use canonical stable-ID order.`);
    }
  }
  return values;
}

function validateGeneralSemBounds(
  lower: number | null | undefined,
  upper: number | null | undefined,
  path: string,
): void {
  if (lower != null && upper != null && lower > upper) {
    wireFail("document.invalid", path, `${path}.lower must not exceed upper.`);
  }
}

function validateGeneralSemEstimate(value: unknown, path: string): void {
  const estimate = exactWireRecord(
    value,
    ["estimate"],
    [
      "bootstrap_mean", "bootstrap_bias", "standard_error", "lower", "upper", "p_value",
      "bootstrap_usable_replicates", "bootstrap_two_sided_exceedances",
    ],
    path,
  );
  const pointEstimate = wireFinite(estimate.estimate, `${path}.estimate`);
  const bootstrapMean = optionalWireFinite(estimate, "bootstrap_mean", path);
  const bootstrapBias = optionalWireFinite(estimate, "bootstrap_bias", path);
  const standardError = optionalWireFinite(estimate, "standard_error", path);
  const lower = optionalWireFinite(estimate, "lower", path);
  const upper = optionalWireFinite(estimate, "upper", path);
  const pValue = optionalWireFinite(estimate, "p_value", path);
  const usableReplicates = optionalWireU32(estimate, "bootstrap_usable_replicates", path);
  const twoSidedExceedances = optionalWireU32(estimate, "bootstrap_two_sided_exceedances", path);
  if (standardError != null && standardError < 0) {
    wireFail("document.invalid", `${path}.standard_error`, `${path}.standard_error must be nonnegative.`);
  }
  if (pValue != null && (pValue < 0 || pValue > 1)) {
    wireFail("document.invalid", `${path}.p_value`, `${path}.p_value must be between 0 and 1.`);
  }
  validateGeneralSemBounds(lower, upper, path);
  const inferenceValues = [
    bootstrapMean,
    bootstrapBias,
    standardError,
    lower,
    upper,
    pValue,
    usableReplicates,
    twoSidedExceedances,
  ];
  const inferenceFieldCount = inferenceValues.filter((item) => item != null).length;
  if (inferenceFieldCount !== 0 && inferenceFieldCount !== inferenceValues.length) {
    wireFail(
      "document.invalid",
      path,
      `${path} bootstrap inference fields must be either all absent or all present.`,
    );
  }
  if (bootstrapMean != null && bootstrapBias != null
    && !approximatelyEqualGeneralSem(bootstrapMean - pointEstimate, bootstrapBias)) {
    wireFail(
      "document.invalid",
      `${path}.bootstrap_bias`,
      `${path}.bootstrap_bias must equal bootstrap_mean minus estimate.`,
    );
  }
}

function generalSemEstimateHasInference(value: unknown): boolean {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const estimate = value as StrictWireRecord;
  return [
    "bootstrap_mean", "bootstrap_bias", "standard_error", "lower", "upper", "p_value",
    "bootstrap_usable_replicates", "bootstrap_two_sided_exceedances",
  ]
    .some((key) => estimate[key] != null);
}

interface GeneralSemWireContext {
  readonly modelId: string;
  readonly modelDigest: string;
  readonly datasetFingerprint: string;
  readonly recipeDigest: string;
  readonly seed: number | null;
  readonly workers: number;
  readonly capabilityIds: ReadonlySet<string>;
}

function validateGeneralSemTrace(value: unknown, path: string, context: GeneralSemWireContext): void {
  const trace = exactWireRecord(value, ["model_id", "capability_cell"], [], path);
  const modelId = wireStableId(trace.model_id, `${path}.model_id`);
  if (modelId !== context.modelId) {
    wireFail("document.invalid", `${path}.model_id`, `${path}.model_id must equal provenance.model_id.`);
  }
  const cell = validateWireCapabilityCell(trace.capability_cell, `${path}.capability_cell`);
  const identity = capabilityCellIdentity(cell);
  if (!context.capabilityIds.has(identity)) {
    wireFail("document.invalid", `${path}.capability_cell`, `${path}.capability_cell is not declared by the document.`);
  }
}

function approximatelyEqualGeneralSem(left: number, right: number): boolean {
  return left === right
    || Math.abs(left - right) <= Number.EPSILON * 8 * Math.max(Math.abs(left), Math.abs(right), 1);
}

function canonicalGeneralSemEffectIdentitiesV1(
  specific: readonly unknown[],
  aggregate: readonly unknown[],
): CanonicalGeneralSemEffectIdentityV1[] {
  const identities: CanonicalGeneralSemEffectIdentityV1[] = specific.map((value, index) => {
    const path = `general_sem_results.specific_indirect_effects[${index}]`;
    const effect = strictWireRecord(value, path);
    return {
      kind: "specific_indirect",
      effect_id: wireStableId(effect.effect_id, `${path}.effect_id`),
      estimand_id: wireStableId(effect.estimand_id, `${path}.estimand_id`),
      source_id: wireStableId(effect.source_id, `${path}.source_id`),
      target_id: wireStableId(effect.target_id, `${path}.target_id`),
      ordered_relation_ids: validateStableIdArray(
        effect.ordered_relation_ids,
        `${path}.ordered_relation_ids`,
        { minimum: 2 },
      ),
    };
  });
  for (let index = 0; index < aggregate.length; index += 1) {
    const path = `general_sem_results.aggregate_effects[${index}]`;
    const effect = strictWireRecord(aggregate[index], path);
    const kind = wireEnum(effect.kind, ["total_indirect", "total_effect"] as const, `${path}.kind`);
    const common = {
      effect_id: wireStableId(effect.effect_id, `${path}.effect_id`),
      estimand_id: wireStableId(effect.estimand_id, `${path}.estimand_id`),
      source_id: wireStableId(effect.source_id, `${path}.source_id`),
      target_id: wireStableId(effect.target_id, `${path}.target_id`),
    };
    const contributingPathIdentities = validateStableIdArray(
      effect.contributing_path_identities,
      `${path}.contributing_path_identities`,
      { canonical: true },
    );
    if (kind === "total_indirect") {
      identities.push({
        kind,
        ...common,
        contributing_path_identities: contributingPathIdentities,
      });
    } else {
      identities.push({
        kind,
        ...common,
        direct_relation_ids: validateStableIdArray(
          effect.direct_relation_ids,
          `${path}.direct_relation_ids`,
          { canonical: true },
        ),
        contributing_path_identities: contributingPathIdentities,
      });
    }
  }
  identities.sort((left, right) => (
    left.effect_id < right.effect_id ? -1 : left.effect_id > right.effect_id ? 1 : 0
  ));
  return identities;
}

function wireGeneralSemSha256(value: unknown, path: string): string {
  const digest = wireText(value, path);
  if (!GENERAL_SEM_LOWERCASE_SHA256.test(digest)) {
    return wireFail("document.invalid", path, `${path} must be a lowercase SHA-256.`);
  }
  return digest;
}

function wireGeneralSemDatasetFingerprint(value: unknown, path: string): string {
  const fingerprint = wireText(value, path);
  if (!GENERAL_SEM_DATASET_FINGERPRINT_V1.test(fingerprint)) {
    return wireFail(
      "document.invalid",
      path,
      `${path} must be a bare lowercase SHA-256 or v2:<lowercase SHA-256>.`,
    );
  }
  return fingerprint;
}

function wireGeneralSemDecimalSafeSeed(value: unknown, path: string): string {
  if (typeof value !== "string"
    || !GENERAL_SEM_CANONICAL_DECIMAL_U64.test(value)
    || BigInt(value) > GENERAL_SEM_MAX_SAFE_SEED) {
    return wireFail(
      "document.invalid",
      path,
      `${path} must be a canonical decimal integer no greater than 9007199254740991.`,
    );
  }
  return value;
}

function validateGeneralSemInferenceReceiptV1(
  receiptValue: unknown,
  specific: readonly unknown[],
  aggregate: readonly unknown[],
  conditional: readonly unknown[],
  hocStages: readonly unknown[],
  context: GeneralSemWireContext,
): void {
  const coveredEffects = [
    ...specific.map((effect, index) => ({
      path: `general_sem_results.specific_indirect_effects[${index}]`,
      effect: strictWireRecord(effect, `general_sem_results.specific_indirect_effects[${index}]`),
    })),
    ...aggregate.map((effect, index) => ({
      path: `general_sem_results.aggregate_effects[${index}]`,
      effect: strictWireRecord(effect, `general_sem_results.aggregate_effects[${index}]`),
    })),
  ];
  const coveredEffectValues = coveredEffects.map(({ effect }) => effect.value);
  const conditionalInference = conditional.some((effect, index) => generalSemEstimateHasInference(
    strictWireRecord(effect, `general_sem_results.conditional_effects[${index}]`).value,
  ));
  const higherOrderInference = hocStages.some((stage, stageIndex) => {
    const record = strictWireRecord(stage, `general_sem_results.higher_order_stages[${stageIndex}]`);
    return Array.isArray(record.relation_estimates) && record.relation_estimates.some((relation, relationIndex) => (
      generalSemEstimateHasInference(strictWireRecord(
        relation,
        `general_sem_results.higher_order_stages[${stageIndex}].relation_estimates[${relationIndex}]`,
      ).value)
    ));
  });
  const uncoveredInference = conditionalInference || higherOrderInference;

  if (receiptValue == null) {
    if (coveredEffectValues.some(generalSemEstimateHasInference) || uncoveredInference) {
      wireFail(
        "document.invalid",
        "general_sem_results.inference_receipt",
        "general_sem_results inference fields require inference_receipt.",
      );
    }
    return;
  }

  const path = "general_sem_results.inference_receipt";
  const receipt = exactWireRecord(receiptValue, [
    "kind",
    "capability_cell",
    "method_version",
    "resampling_operation_version",
    "resampling_stream_version",
    "quantile_method_version",
    "standard_error_method_version",
    "summation_method_version",
    "p_value_method_version",
    "failure_policy_version",
    "compilation_artifact_identity_sha256",
    "compiled_plan_sha256",
    "general_sem_config_sha256",
    "recipe_analytical_sha256",
    "model_scientific_sha256",
    "source_dataset_fingerprint",
    "complete_case_frame_sha256",
    "usable_replicate_indices_sha256",
    "effect_identity_set_sha256",
    "effect_ids",
    "interval",
    "tail",
    "confidence_level",
    "resamples_requested",
    "resamples_usable",
    "minimum_usable_resamples",
    "seed",
    "workers",
    "complete_model_reestimated_per_replicate",
    "failed_replicates",
  ], [], path);

  wireEnum(receipt.kind, ["case_bootstrap"] as const, `${path}.kind`);
  const capabilityCell = validateWireCapabilityCell(receipt.capability_cell, `${path}.capability_cell`);
  if (capabilityCellIdentity(capabilityCell)
    !== capabilityCellIdentity(GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1)) {
    wireFail(
      "document.invalid",
      `${path}.capability_cell`,
      `${path}.capability_cell must equal the General SEM multiple-mediation full-model case-bootstrap option cell.`,
    );
  }
  if (!context.capabilityIds.has(capabilityCellIdentity(capabilityCell))) {
    wireFail(
      "document.invalid",
      `${path}.capability_cell`,
      `${path}.capability_cell references an undeclared option cell.`,
    );
  }

  const versions = [
    ["method_version", GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1],
    ["resampling_operation_version", GENERAL_SEM_PLS_CASE_BOOTSTRAP_OPERATION_VERSION_V1],
    ["resampling_stream_version", GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1],
    ["quantile_method_version", GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1],
    ["standard_error_method_version", GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1],
    ["summation_method_version", GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1],
    ["p_value_method_version", GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1],
    ["failure_policy_version", GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1],
  ] as const;
  for (const [field, expected] of versions) {
    const actual = wireStableId(receipt[field], `${path}.${field}`);
    if (actual !== expected) {
      wireFail("document.invalid", `${path}.${field}`, `${path}.${field} must equal ${expected}.`);
    }
  }

  for (const field of [
    "compilation_artifact_identity_sha256",
    "compiled_plan_sha256",
    "general_sem_config_sha256",
    "recipe_analytical_sha256",
    "model_scientific_sha256",
    "complete_case_frame_sha256",
    "usable_replicate_indices_sha256",
    "effect_identity_set_sha256",
  ] as const) {
    wireGeneralSemSha256(receipt[field], `${path}.${field}`);
  }
  const sourceDatasetFingerprint = wireGeneralSemDatasetFingerprint(
    receipt.source_dataset_fingerprint,
    `${path}.source_dataset_fingerprint`,
  );
  if (receipt.model_scientific_sha256 !== context.modelDigest) {
    wireFail(
      "document.invalid",
      `${path}.model_scientific_sha256`,
      `${path}.model_scientific_sha256 must equal provenance.model_digest.`,
    );
  }
  if (sourceDatasetFingerprint !== context.datasetFingerprint) {
    wireFail(
      "document.invalid",
      `${path}.source_dataset_fingerprint`,
      `${path}.source_dataset_fingerprint must equal provenance.dataset_fingerprint.`,
    );
  }
  if (receipt.recipe_analytical_sha256 !== context.recipeDigest) {
    wireFail(
      "document.invalid",
      `${path}.recipe_analytical_sha256`,
      `${path}.recipe_analytical_sha256 must equal provenance.recipe_digest.`,
    );
  }

  const effectIds = validateStableIdArray(receipt.effect_ids, `${path}.effect_ids`, { minimum: 1, canonical: true });
  const expectedEffectIds = [...specific, ...aggregate]
    .map((effect, index) => wireStableId(
      strictWireRecord(effect, `general_sem_results.covered_effects[${index}]`).effect_id,
      `general_sem_results.covered_effects[${index}].effect_id`,
    ))
    .sort();
  if (effectIds.length !== expectedEffectIds.length
    || effectIds.some((effectId, index) => effectId !== expectedEffectIds[index])) {
    wireFail(
      "document.invalid",
      `${path}.effect_ids`,
      `${path}.effect_ids must exactly cover specific and aggregate effect rows.`,
    );
  }
  const effectIdentities = canonicalGeneralSemEffectIdentitiesV1(specific, aggregate);
  if (receipt.effect_identity_set_sha256 !== generalSemSerializedSha256(effectIdentities)) {
    wireFail(
      "document.invalid",
      `${path}.effect_identity_set_sha256`,
      `${path}.effect_identity_set_sha256 does not match the typed effect identity set.`,
    );
  }

  wireEnum(receipt.interval, ["percentile_type7", "bca"] as const, `${path}.interval`);
  if (receipt.interval !== "percentile_type7") {
    wireFail("document.invalid", `${path}.interval`, `${path}.interval must equal percentile_type7 for the v1 executor.`);
  }
  wireEnum(receipt.tail, ["two_sided", "one_sided_lower", "one_sided_upper"] as const, `${path}.tail`);
  if (receipt.tail !== "two_sided") {
    wireFail("document.invalid", `${path}.tail`, `${path}.tail must equal two_sided for the v1 executor.`);
  }
  const confidenceLevel = wireFinite(receipt.confidence_level, `${path}.confidence_level`);
  if (confidenceLevel <= 0 || confidenceLevel >= 1) {
    wireFail(
      "document.invalid",
      `${path}.confidence_level`,
      `${path}.confidence_level must be finite and strictly between 0 and 1.`,
    );
  }

  const resamplesRequested = wireU32(receipt.resamples_requested, `${path}.resamples_requested`);
  const resamplesUsable = wireU32(receipt.resamples_usable, `${path}.resamples_usable`);
  const minimumUsableResamples = wireU32(receipt.minimum_usable_resamples, `${path}.minimum_usable_resamples`);
  if (resamplesRequested < 2 || resamplesRequested > 10_000) {
    wireFail(
      "document.invalid",
      `${path}.resamples_requested`,
      `${path}.resamples_requested must be between 2 and 10000.`,
    );
  }
  const expectedMinimumUsable = Math.max(2, Math.ceil(resamplesRequested * 0.9));
  if (minimumUsableResamples !== expectedMinimumUsable) {
    wireFail(
      "document.invalid",
      `${path}.minimum_usable_resamples`,
      `${path}.minimum_usable_resamples must equal the 90 percent usable gate.`,
    );
  }
  if (resamplesUsable < minimumUsableResamples || resamplesUsable > resamplesRequested) {
    wireFail(
      "document.invalid",
      `${path}.resamples_usable`,
      `${path}.resamples_usable must satisfy the declared usable gate.`,
    );
  }

  const seed = wireGeneralSemDecimalSafeSeed(receipt.seed, `${path}.seed`);
  if (context.seed == null || seed !== String(context.seed)) {
    wireFail("document.invalid", `${path}.seed`, `${path}.seed must equal provenance.seed.`);
  }
  const workers = wireU32(receipt.workers, `${path}.workers`);
  if (workers < 1 || workers > 64) {
    wireFail("document.invalid", `${path}.workers`, `${path}.workers must be between 1 and 64.`);
  }
  if (workers !== context.workers) {
    wireFail("document.invalid", `${path}.workers`, `${path}.workers must equal provenance.workers.`);
  }
  if (typeof receipt.complete_model_reestimated_per_replicate !== "boolean") {
    wireFail(
      "schema.invalid_shape",
      `${path}.complete_model_reestimated_per_replicate`,
      `${path}.complete_model_reestimated_per_replicate must be a boolean.`,
    );
  }
  if (!receipt.complete_model_reestimated_per_replicate) {
    wireFail(
      "document.invalid",
      `${path}.complete_model_reestimated_per_replicate`,
      `${path}.complete_model_reestimated_per_replicate must be true.`,
    );
  }

  const failures = wireArray(receipt.failed_replicates, `${path}.failed_replicates`);
  if (resamplesUsable + failures.length !== resamplesRequested) {
    wireFail(
      "document.invalid",
      path,
      `${path} requested count must equal usable plus failed replicates.`,
    );
  }
  const failedIndices = new Set<number>();
  let previousFailureIndex: number | null = null;
  failures.forEach((failureValue, index) => {
    const failurePath = `${path}.failed_replicates[${index}]`;
    const failure = exactWireRecord(failureValue, ["replicate_index", "reason_code", "message"], [], failurePath);
    const replicateIndex = wireU32(failure.replicate_index, `${failurePath}.replicate_index`);
    if (replicateIndex >= resamplesRequested) {
      wireFail(
        "document.invalid",
        `${failurePath}.replicate_index`,
        `${failurePath}.replicate_index is outside the requested plan.`,
      );
    }
    if (previousFailureIndex != null && previousFailureIndex >= replicateIndex) {
      wireFail(
        "document.invalid",
        `${path}.failed_replicates`,
        `${path}.failed_replicates must be strictly ordered by replicate_index.`,
      );
    }
    previousFailureIndex = replicateIndex;
    failedIndices.add(replicateIndex);
    wireEnum(failure.reason_code, [
      "insufficient_observations",
      "constant_indicator",
      "rank_deficient",
      "isolated_construct",
      "estimation_nonconvergence",
      "numerical_failure",
    ] as const, `${failurePath}.reason_code`);
    wireText(failure.message, `${failurePath}.message`);
  });
  const usableReplicateIndices = Array.from({ length: resamplesRequested }, (_, index) => index)
    .filter((replicateIndex) => !failedIndices.has(replicateIndex));
  if (usableReplicateIndices.length !== resamplesUsable) {
    wireFail(
      "document.invalid",
      `${path}.resamples_usable`,
      `${path}.resamples_usable contradicts the failure ledger.`,
    );
  }
  if (receipt.usable_replicate_indices_sha256 !== generalSemSerializedSha256(usableReplicateIndices)) {
    wireFail(
      "document.invalid",
      `${path}.usable_replicate_indices_sha256`,
      `${path}.usable_replicate_indices_sha256 does not match the failure ledger.`,
    );
  }
  if (coveredEffectValues.some((estimate) => !generalSemEstimateHasInference(estimate))) {
    wireFail(
      "document.invalid",
      path,
      `${path} requires complete inference fields for every covered effect.`,
    );
  }
  if (uncoveredInference) {
    wireFail(
      "document.invalid",
      path,
      `${path} v1 does not cover conditional or higher-order estimate inference.`,
    );
  }
  const expectedEffectCapability = capabilityCellIdentity(
    GENERAL_SEM_PLS_RECURSIVE_EFFECTS_CAPABILITY_CELL_V1,
  );
  for (const { path: effectPath, effect } of coveredEffects) {
    const effectId = wireStableId(effect.effect_id, `${effectPath}.effect_id`);
    const trace = strictWireRecord(effect.trace, `${effectPath}.trace`);
    const traceCapability = validateWireCapabilityCell(
      trace.capability_cell,
      `${effectPath}.trace.capability_cell`,
    );
    if (capabilityCellIdentity(traceCapability) !== expectedEffectCapability) {
      wireFail(
        "document.invalid",
        `${effectPath}.trace.capability_cell`,
        `${path} effect ${effectId} trace.capability_cell must equal the PLS recursive-effects option cell.`,
      );
    }
    const value = strictWireRecord(effect.value, `${effectPath}.value`);
    const usable = optionalWireU32(value, "bootstrap_usable_replicates", `${effectPath}.value`);
    const exceedances = optionalWireU32(value, "bootstrap_two_sided_exceedances", `${effectPath}.value`);
    if (usable == null || exceedances == null) continue;
    if (usable !== resamplesUsable) {
      wireFail(
        "document.invalid",
        `${effectPath}.value.bootstrap_usable_replicates`,
        `${path} effect ${effectId} usable replicate count contradicts the receipt.`,
      );
    }
    if (exceedances > usable) {
      wireFail(
        "document.invalid",
        `${effectPath}.value.bootstrap_two_sided_exceedances`,
        `${path} effect ${effectId} exceedance count exceeds usable replicates.`,
      );
    }
    const pValue = optionalWireFinite(value, "p_value", `${effectPath}.value`);
    const expectedPValue = (exceedances + 1) / (usable + 1);
    if (pValue != null && !approximatelyEqualGeneralSem(pValue, expectedPValue)) {
      wireFail(
        "document.invalid",
        `${effectPath}.value.p_value`,
        `${path} effect ${effectId} p_value contradicts the plus-one exceedance ledger.`,
      );
    }
  }
}

function validateConditionalProbeValues(value: unknown, path: string): number[] {
  const record = strictWireRecord(value, path);
  const kind = wireEnum(record.kind, ["data_derived_mean_plus_minus_one_sd", "explicit"] as const, `${path}.kind`);
  if (kind === "data_derived_mean_plus_minus_one_sd") {
    const values = exactWireRecord(value, ["kind", "mean", "standard_deviation"], [], path);
    const mean = wireFinite(values.mean, `${path}.mean`);
    const standardDeviation = wireFinite(values.standard_deviation, `${path}.standard_deviation`);
    if (standardDeviation < 0) {
      wireFail("document.invalid", `${path}.standard_deviation`, `${path}.standard_deviation must be nonnegative.`);
    }
    return [mean - standardDeviation, mean, mean + standardDeviation];
  }
  const values = exactWireRecord(value, ["kind", "values"], [], path);
  const explicit = wireArray(values.values, `${path}.values`).map((item, index) => wireFinite(item, `${path}.values[${index}]`));
  if (explicit.length === 0) wireFail("document.invalid", `${path}.values`, `${path}.values must not be empty.`);
  for (let index = 1; index < explicit.length; index += 1) {
    if (explicit[index - 1]! >= explicit[index]!) {
      wireFail("document.invalid", `${path}.values`, `${path}.values must be strictly increasing.`);
    }
  }
  return explicit;
}

/** Strict, lossless validator for the optional Rust General SEM result extension. */
export function parseCanonicalGeneralSemResultsV1(
  value: unknown,
  context: CanonicalGeneralSemResultsV1Context,
): CanonicalGeneralSemResultsV1 {
  if (hasNonFiniteNumber(value)) {
    return wireFail("schema.non_finite", "general_sem_results", "general_sem_results contains a non-finite number or cyclic value.");
  }
  const results = exactWireRecord(
    value,
    ["schema_version"],
    [
      "inference_receipt",
      "specific_indirect_effects",
      "aggregate_effects",
      "conditional_effect_probes",
      "conditional_effects",
      "interaction_plots",
      "higher_order_stages",
      "cbsem_fit",
      "identification_diagnostics",
    ],
    "general_sem_results",
  );
  if (results.schema_version !== CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION) {
    return wireFail(
      "schema.version_unsupported",
      "general_sem_results.schema_version",
      "general_sem_results.schema_version must equal 1.",
    );
  }
  if (context.seed != null && (!Number.isSafeInteger(context.seed) || context.seed < 0)) {
    return wireFail(
      "document.invalid",
      "provenance.seed",
      "provenance.seed must be a nonnegative safe integer or null.",
    );
  }
  const provenanceWorkers = wireU32(context.workers, "provenance.workers");
  if (provenanceWorkers < 1) {
    return wireFail("document.invalid", "provenance.workers", "provenance.workers must be positive.");
  }
  const wireContext: GeneralSemWireContext = {
    modelId: wireStableId(context.modelId, "provenance.model_id"),
    modelDigest: wireGeneralSemSha256(context.modelDigest, "provenance.model_digest"),
    datasetFingerprint: wireGeneralSemDatasetFingerprint(
      context.datasetFingerprint,
      "provenance.dataset_fingerprint",
    ),
    recipeDigest: wireGeneralSemSha256(context.recipeDigest, "provenance.recipe_digest"),
    seed: context.seed,
    workers: provenanceWorkers,
    capabilityIds: new Set(context.capabilityCells.map((cell, index) => (
      capabilityCellIdentity(validateWireCapabilityCell(cell, `capability_cells[${index}]`))
    ))),
  };
  if (wireContext.capabilityIds.size === 0) {
    return wireFail("document.invalid", "capability_cells", "general_sem_results requires document capability_cells.");
  }

  const specific = optionalWireArray(results, "specific_indirect_effects", "general_sem_results");
  const aggregate = optionalWireArray(results, "aggregate_effects", "general_sem_results");
  const probes = optionalWireArray(results, "conditional_effect_probes", "general_sem_results");
  const conditional = optionalWireArray(results, "conditional_effects", "general_sem_results");
  const plots = optionalWireArray(results, "interaction_plots", "general_sem_results");
  const hocStages = optionalWireArray(results, "higher_order_stages", "general_sem_results");
  const fits = optionalWireArray(results, "cbsem_fit", "general_sem_results");
  const identification = optionalWireArray(results, "identification_diagnostics", "general_sem_results");
  if ([specific, aggregate, probes, conditional, plots, hocStages, fits, identification]
    .every((collection) => collection.length === 0)) {
    return wireFail("document.invalid", "general_sem_results", "general_sem_results must contain at least one typed result section.");
  }

  validateCanonicalWireIds(specific, "effect_id", "general_sem_results.specific_indirect_effects");
  validateCanonicalWireIds(aggregate, "effect_id", "general_sem_results.aggregate_effects");
  validateCanonicalWireIds(probes, "probe_id", "general_sem_results.conditional_effect_probes");
  validateCanonicalWireIds(conditional, "effect_id", "general_sem_results.conditional_effects");
  validateCanonicalWireIds(plots, "plot_id", "general_sem_results.interaction_plots");
  validateCanonicalWireIds(hocStages, "stage_id", "general_sem_results.higher_order_stages");
  validateCanonicalWireIds(fits, "fit_id", "general_sem_results.cbsem_fit");
  validateCanonicalWireIds(identification, "diagnostic_id", "general_sem_results.identification_diagnostics");

  const effectIds = new Set<string>();
  const specificSignatures = new Set<string>();
  specific.forEach((item, index) => {
    const path = `general_sem_results.specific_indirect_effects[${index}]`;
    const effect = exactWireRecord(item, [
      "effect_id", "estimand_id", "trace", "source_id", "target_id", "ordered_relation_ids", "value",
    ], [], path);
    const effectId = wireStableId(effect.effect_id, `${path}.effect_id`);
    if (effectIds.has(effectId)) wireFail("document.invalid", `${path}.effect_id`, `${path}.effect_id is duplicated across effect sections.`);
    effectIds.add(effectId);
    wireStableId(effect.estimand_id, `${path}.estimand_id`);
    validateGeneralSemTrace(effect.trace, `${path}.trace`, wireContext);
    const sourceId = wireStableId(effect.source_id, `${path}.source_id`);
    const targetId = wireStableId(effect.target_id, `${path}.target_id`);
    if (sourceId === targetId) wireFail("document.invalid", path, `${path} requires distinct source_id and target_id.`);
    const relations = validateStableIdArray(effect.ordered_relation_ids, `${path}.ordered_relation_ids`, { minimum: 2 });
    if (effectId !== generalSemSpecificDirectedPathIdentityV1(relations)) {
      wireFail(
        "document.invalid",
        `${path}.effect_id`,
        `${path}.effect_id must equal the canonical ordered relation-path identity.`,
      );
    }
    const signature = relations.join("\0");
    if (specificSignatures.has(signature)) wireFail("document.invalid", path, `${path} duplicates another specific indirect path.`);
    specificSignatures.add(signature);
    validateGeneralSemEstimate(effect.value, `${path}.value`);
  });

  const aggregateSignatures = new Set<string>();
  aggregate.forEach((item, index) => {
    const path = `general_sem_results.aggregate_effects[${index}]`;
    const effect = exactWireRecord(item, [
      "effect_id", "estimand_id", "trace", "kind", "source_id", "target_id",
      "direct_relation_ids", "contributing_path_identities", "value",
    ], [], path);
    const effectId = wireStableId(effect.effect_id, `${path}.effect_id`);
    if (effectIds.has(effectId)) wireFail("document.invalid", `${path}.effect_id`, `${path}.effect_id is duplicated across effect sections.`);
    effectIds.add(effectId);
    const estimandId = wireStableId(effect.estimand_id, `${path}.estimand_id`);
    validateGeneralSemTrace(effect.trace, `${path}.trace`, wireContext);
    const kind = wireEnum(effect.kind, ["total_indirect", "total_effect"] as const, `${path}.kind`);
    const sourceId = wireStableId(effect.source_id, `${path}.source_id`);
    const targetId = wireStableId(effect.target_id, `${path}.target_id`);
    if (sourceId === targetId) wireFail("document.invalid", path, `${path} requires distinct source_id and target_id.`);
    if (effectId !== estimandId) {
      wireFail("document.invalid", `${path}.effect_id`, `${path}.effect_id must equal estimand_id for aggregate effects.`);
    }
    const directRelationIds = validateStableIdArray(
      effect.direct_relation_ids,
      `${path}.direct_relation_ids`,
      { canonical: true },
    );
    const contributingPathIdentities = validateStableIdArray(
      effect.contributing_path_identities,
      `${path}.contributing_path_identities`,
      { canonical: true },
    );
    if (kind === "total_indirect") {
      if (directRelationIds.length > 0) {
        wireFail(
          "document.invalid",
          `${path}.direct_relation_ids`,
          `${path}.direct_relation_ids must be empty for total indirect effects.`,
        );
      }
      if (contributingPathIdentities.length === 0) {
        wireFail(
          "document.invalid",
          `${path}.contributing_path_identities`,
          `${path}.contributing_path_identities must not be empty.`,
        );
      }
    } else if (directRelationIds.length === 0 && contributingPathIdentities.length === 0) {
      wireFail(
        "document.invalid",
        path,
        `${path} must identify at least one direct relation or indirect path.`,
      );
    }
    const signature = `${kind}\0${sourceId}\0${targetId}`;
    if (aggregateSignatures.has(signature)) wireFail("document.invalid", path, `${path} duplicates another aggregate scientific effect.`);
    aggregateSignatures.add(signature);
    validateGeneralSemEstimate(effect.value, `${path}.value`);
  });

  const probeValues = new Map<string, { moderatorId: string; values: number[] }>();
  probes.forEach((item, index) => {
    const path = `general_sem_results.conditional_effect_probes[${index}]`;
    const probe = exactWireRecord(item, ["probe_id", "trace", "moderator_id", "values"], [], path);
    const probeId = wireStableId(probe.probe_id, `${path}.probe_id`);
    validateGeneralSemTrace(probe.trace, `${path}.trace`, wireContext);
    probeValues.set(probeId, {
      moderatorId: wireStableId(probe.moderator_id, `${path}.moderator_id`),
      values: validateConditionalProbeValues(probe.values, `${path}.values`),
    });
  });

  const conditionalSignatures = new Set<string>();
  conditional.forEach((item, index) => {
    const path = `general_sem_results.conditional_effects[${index}]`;
    const effect = exactWireRecord(item, [
      "effect_id", "estimand_id", "trace", "interaction_id", "focal_relation_id", "probe_id",
      "moderator_id", "probe_value_index", "moderator_value", "value",
    ], [], path);
    const effectId = wireStableId(effect.effect_id, `${path}.effect_id`);
    if (effectIds.has(effectId)) wireFail("document.invalid", `${path}.effect_id`, `${path}.effect_id is duplicated across effect sections.`);
    effectIds.add(effectId);
    const estimandId = wireStableId(effect.estimand_id, `${path}.estimand_id`);
    validateGeneralSemTrace(effect.trace, `${path}.trace`, wireContext);
    const interactionId = wireStableId(effect.interaction_id, `${path}.interaction_id`);
    const focalRelationId = wireStableId(effect.focal_relation_id, `${path}.focal_relation_id`);
    const probeId = wireStableId(effect.probe_id, `${path}.probe_id`);
    const moderatorId = wireStableId(effect.moderator_id, `${path}.moderator_id`);
    const probeValueIndex = wireU32(effect.probe_value_index, `${path}.probe_value_index`);
    const moderatorValue = wireFinite(effect.moderator_value, `${path}.moderator_value`);
    const probe = probeValues.get(probeId);
    if (!probe) wireFail("document.invalid", `${path}.probe_id`, `${path}.probe_id references a missing probe.`);
    if (probe.moderatorId !== moderatorId) wireFail("document.invalid", `${path}.moderator_id`, `${path}.moderator_id contradicts its probe.`);
    const expectedValue = probe.values[probeValueIndex];
    if (expectedValue === undefined || !approximatelyEqualGeneralSem(moderatorValue, expectedValue)) {
      wireFail("document.invalid", `${path}.moderator_value`, `${path}.moderator_value contradicts its probe value.`);
    }
    const signature = `${estimandId}\0${interactionId}\0${focalRelationId}\0${probeId}\0${probeValueIndex}`;
    if (conditionalSignatures.has(signature)) wireFail("document.invalid", path, `${path} duplicates another conditional scientific effect.`);
    conditionalSignatures.add(signature);
    validateGeneralSemEstimate(effect.value, `${path}.value`);
  });

  plots.forEach((item, index) => {
    const path = `general_sem_results.interaction_plots[${index}]`;
    const plot = exactWireRecord(item, [
      "plot_id", "trace", "interaction_id", "focal_relation_id", "focal_predictor_id",
      "moderator_id", "outcome_id", "series",
    ], [], path);
    wireStableId(plot.plot_id, `${path}.plot_id`);
    validateGeneralSemTrace(plot.trace, `${path}.trace`, wireContext);
    wireStableId(plot.interaction_id, `${path}.interaction_id`);
    wireStableId(plot.focal_relation_id, `${path}.focal_relation_id`);
    const focalId = wireStableId(plot.focal_predictor_id, `${path}.focal_predictor_id`);
    const moderatorId = wireStableId(plot.moderator_id, `${path}.moderator_id`);
    const outcomeId = wireStableId(plot.outcome_id, `${path}.outcome_id`);
    if (new Set([focalId, moderatorId, outcomeId]).size !== 3) {
      wireFail("document.invalid", path, `${path} requires distinct focal, moderator, and outcome identities.`);
    }
    const seriesValues = wireArray(plot.series, `${path}.series`);
    if (seriesValues.length === 0) wireFail("document.invalid", `${path}.series`, `${path}.series must not be empty.`);
    validateCanonicalWireIds(seriesValues, "series_id", `${path}.series`);
    let commonGrid: number[] | null = null;
    seriesValues.forEach((seriesValue, seriesIndex) => {
      const seriesPath = `${path}.series[${seriesIndex}]`;
      const series = exactWireRecord(seriesValue, ["series_id", "probe_id", "probe_value_index", "moderator_value", "points"], [], seriesPath);
      wireStableId(series.series_id, `${seriesPath}.series_id`);
      const probeId = wireStableId(series.probe_id, `${seriesPath}.probe_id`);
      const probeValueIndex = wireU32(series.probe_value_index, `${seriesPath}.probe_value_index`);
      const moderatorValue = wireFinite(series.moderator_value, `${seriesPath}.moderator_value`);
      const probe = probeValues.get(probeId);
      if (!probe) wireFail("document.invalid", `${seriesPath}.probe_id`, `${seriesPath}.probe_id references a missing probe.`);
      if (probe.moderatorId !== moderatorId) wireFail("document.invalid", `${seriesPath}.probe_id`, `${seriesPath}.probe_id uses a different moderator.`);
      const expectedValue = probe.values[probeValueIndex];
      if (expectedValue === undefined || !approximatelyEqualGeneralSem(moderatorValue, expectedValue)) {
        wireFail("document.invalid", `${seriesPath}.moderator_value`, `${seriesPath}.moderator_value contradicts its probe value.`);
      }
      const points = wireArray(series.points, `${seriesPath}.points`);
      if (points.length === 0) wireFail("document.invalid", `${seriesPath}.points`, `${seriesPath}.points must not be empty.`);
      const grid = points.map((pointValue, pointIndex) => {
        const pointPath = `${seriesPath}.points[${pointIndex}]`;
        const point = exactWireRecord(pointValue, ["focal_value", "predicted_value"], ["lower", "upper"], pointPath);
        const focalValue = wireFinite(point.focal_value, `${pointPath}.focal_value`);
        wireFinite(point.predicted_value, `${pointPath}.predicted_value`);
        const lower = optionalWireFinite(point, "lower", pointPath);
        const upper = optionalWireFinite(point, "upper", pointPath);
        validateGeneralSemBounds(lower, upper, pointPath);
        return focalValue;
      });
      for (let pointIndex = 1; pointIndex < grid.length; pointIndex += 1) {
        if (grid[pointIndex - 1]! >= grid[pointIndex]!) {
          wireFail("document.invalid", `${seriesPath}.points`, `${seriesPath}.points must use strictly increasing focal values.`);
        }
      }
      if (commonGrid && (commonGrid.length !== grid.length
        || commonGrid.some((expected, gridIndex) => !approximatelyEqualGeneralSem(expected, grid[gridIndex]!)))) {
        wireFail("document.invalid", `${seriesPath}.points`, `${seriesPath}.points must use the plot's common focal-value grid.`);
      }
      commonGrid ??= grid;
    });
  });

  const hocSignatures = new Set<string>();
  hocStages.forEach((item, index) => {
    const path = `general_sem_results.higher_order_stages[${index}]`;
    const stage = exactWireRecord(item, [
      "stage_id", "trace", "higher_order_construct_id", "stage_number", "kind",
      "input_construct_ids", "output_variable_ids",
    ], ["relation_estimates"], path);
    wireStableId(stage.stage_id, `${path}.stage_id`);
    validateGeneralSemTrace(stage.trace, `${path}.trace`, wireContext);
    const hocId = wireStableId(stage.higher_order_construct_id, `${path}.higher_order_construct_id`);
    const stageNumber = wireU32(stage.stage_number, `${path}.stage_number`);
    const kind = wireEnum(stage.kind, ["lower_order_score_estimation", "higher_order_estimation"] as const, `${path}.kind`);
    const expectedStage = kind === "lower_order_score_estimation" ? 1 : 2;
    if (stageNumber !== expectedStage) wireFail("document.invalid", `${path}.stage_number`, `${path}.stage_number contradicts its stage kind.`);
    const signature = `${hocId}\0${stageNumber}`;
    if (hocSignatures.has(signature)) wireFail("document.invalid", path, `${path} duplicates a higher-order construct stage.`);
    hocSignatures.add(signature);
    validateStableIdArray(stage.input_construct_ids, `${path}.input_construct_ids`, { minimum: 1, canonical: true });
    validateStableIdArray(stage.output_variable_ids, `${path}.output_variable_ids`, { minimum: 1, canonical: true });
    const relations = optionalWireArray(stage, "relation_estimates", path);
    validateCanonicalWireIds(relations, "relation_id", `${path}.relation_estimates`);
    relations.forEach((relationValue, relationIndex) => {
      const relationPath = `${path}.relation_estimates[${relationIndex}]`;
      const relation = exactWireRecord(relationValue, ["relation_id", "source_id", "target_id", "value"], [], relationPath);
      wireStableId(relation.relation_id, `${relationPath}.relation_id`);
      const sourceId = wireStableId(relation.source_id, `${relationPath}.source_id`);
      const targetId = wireStableId(relation.target_id, `${relationPath}.target_id`);
      if (sourceId === targetId) wireFail("document.invalid", relationPath, `${relationPath} requires distinct source_id and target_id.`);
      validateGeneralSemEstimate(relation.value, `${relationPath}.value`);
    });
  });

  fits.forEach((item, index) => {
    const path = `general_sem_results.cbsem_fit[${index}]`;
    const fit = exactWireRecord(item, ["fit_id", "trace", "chi_square", "degrees_of_freedom"], [
      "chi_square_p_value", "rmsea", "rmsea_interval", "cfi", "tli", "srmr", "aic", "bic",
    ], path);
    wireStableId(fit.fit_id, `${path}.fit_id`);
    validateGeneralSemTrace(fit.trace, `${path}.trace`, wireContext);
    const chiSquare = wireFinite(fit.chi_square, `${path}.chi_square`);
    if (chiSquare < 0) wireFail("document.invalid", `${path}.chi_square`, `${path}.chi_square must be nonnegative.`);
    const degreesOfFreedom = wireU32(fit.degrees_of_freedom, `${path}.degrees_of_freedom`);
    const pValue = optionalWireFinite(fit, "chi_square_p_value", path);
    if (pValue != null && (pValue < 0 || pValue > 1)) wireFail("document.invalid", `${path}.chi_square_p_value`, `${path}.chi_square_p_value must be between 0 and 1.`);
    if (degreesOfFreedom === 0 && pValue != null) wireFail("document.invalid", `${path}.chi_square_p_value`, `${path}.chi_square_p_value must be absent when degrees_of_freedom is zero.`);
    const rmsea = optionalWireFinite(fit, "rmsea", path);
    const srmr = optionalWireFinite(fit, "srmr", path);
    for (const key of ["cfi", "tli", "aic", "bic"] as const) optionalWireFinite(fit, key, path);
    if (rmsea != null && rmsea < 0) wireFail("document.invalid", `${path}.rmsea`, `${path}.rmsea must be nonnegative.`);
    if (srmr != null && srmr < 0) wireFail("document.invalid", `${path}.srmr`, `${path}.srmr must be nonnegative.`);
    if (Object.prototype.hasOwnProperty.call(fit, "rmsea_interval") && fit.rmsea_interval !== null) {
      if (rmsea == null) wireFail("document.invalid", `${path}.rmsea_interval`, `${path}.rmsea_interval requires rmsea.`);
      const interval = exactWireRecord(fit.rmsea_interval, ["confidence_level", "lower", "upper"], [], `${path}.rmsea_interval`);
      const confidence = wireFinite(interval.confidence_level, `${path}.rmsea_interval.confidence_level`);
      const lower = wireFinite(interval.lower, `${path}.rmsea_interval.lower`);
      const upper = wireFinite(interval.upper, `${path}.rmsea_interval.upper`);
      if (confidence <= 0 || confidence >= 1) wireFail("document.invalid", `${path}.rmsea_interval.confidence_level`, `${path}.rmsea_interval.confidence_level must be between 0 and 1.`);
      if (lower < 0) wireFail("document.invalid", `${path}.rmsea_interval.lower`, `${path}.rmsea_interval.lower must be nonnegative.`);
      validateGeneralSemBounds(lower, upper, `${path}.rmsea_interval`);
    }
  });

  identification.forEach((item, index) => {
    const path = `general_sem_results.identification_diagnostics[${index}]`;
    const diagnostic = exactWireRecord(item, ["diagnostic_id", "trace", "scope", "subject_id", "status", "code", "message"], ["degrees_of_freedom"], path);
    wireStableId(diagnostic.diagnostic_id, `${path}.diagnostic_id`);
    validateGeneralSemTrace(diagnostic.trace, `${path}.trace`, wireContext);
    const scope = wireEnum(diagnostic.scope, ["model", "variable", "relation", "interaction", "higher_order_construct"] as const, `${path}.scope`);
    const subjectId = wireStableId(diagnostic.subject_id, `${path}.subject_id`);
    const status = wireEnum(diagnostic.status, ["identified", "underidentified", "locally_underidentified", "boundary_condition"] as const, `${path}.status`);
    wireStableId(diagnostic.code, `${path}.code`);
    wireText(diagnostic.message, `${path}.message`);
    const degreesOfFreedom = optionalWireSafeInteger(diagnostic, "degrees_of_freedom", path);
    if (scope === "model" && subjectId !== wireContext.modelId) wireFail("document.invalid", `${path}.subject_id`, `${path}.subject_id must equal provenance.model_id for model scope.`);
    if (status === "identified" && degreesOfFreedom != null && degreesOfFreedom < 0) {
      wireFail("document.invalid", `${path}.degrees_of_freedom`, `${path} cannot be identified with negative degrees_of_freedom.`);
    }
  });

  validateGeneralSemInferenceReceiptV1(
    results.inference_receipt,
    specific,
    aggregate,
    conditional,
    hocStages,
    wireContext,
  );

  return value as CanonicalGeneralSemResultsV1;
}
