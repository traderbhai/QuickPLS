import type { CapabilityCellReferenceV2 } from "./canonicalResultDocumentV2";

export const MGA_MULTIGROUP_V1_SCHEMA_VERSION = 1 as const;
export const PLS_HETEROGENEITY_V2_SCHEMA_VERSION = 2 as const;
export const GENERAL_SEM_CONDITIONAL_PROCESS_V2_SCHEMA_VERSION = 2 as const;
export const INTERVENTIONAL_CAUSAL_MEDIATION_V1_SCHEMA_VERSION = 1 as const;
export const CONDITIONAL_RAW_PROBE_TRANSFORMATION_CONTRACT_V2 =
  "qpls.conditional.raw_probe.sample_standardization.v1" as const;
export const CONDITIONAL_RAW_PROBE_FIT_METRIC_CONTRACT_V2 =
  "qpls.conditional.raw_probe.fit_metric.v2" as const;
export const MULTIMOD_RESULT_ATTACHMENT_SCHEMA_VERSION_V1 = 1 as const;
export const MULTIMOD_RESULT_SIDECAR_DESCRIPTOR_V1_SCHEMA_VERSION = 1 as const;
export const MULTIMOD_SIDECAR_WARN_BYTES_V1 = 128 * 1024 * 1024;
export const MULTIMOD_SIDECAR_MAX_BYTES_V1 = 512 * 1024 * 1024;

const MGA_NUMERICAL_TIE_ULPS_V1 = 64;

function mgaGreaterOrTiedV1(left: number, right: number): boolean {
  const tolerance =
    MGA_NUMERICAL_TIE_ULPS_V1 *
    Number.EPSILON *
    Math.max(1, Math.abs(left), Math.abs(right));
  return left >= right - tolerance;
}

export type InferenceAlternativeV1 = "two_sided" | "less" | "greater";
export type MultiplicityAdjustmentV1 =
  | "holm"
  | "bonferroni"
  | "sidak"
  | "benjamini_hochberg_exploratory"
  | "none_explicit";

export type TypedGroupValueV1 =
  | { kind: "text"; value: string }
  | { kind: "integer"; value: number }
  | { kind: "number"; value: number }
  | { kind: "boolean"; value: boolean };

export interface SelectedGroupV1 {
  group_id: string;
  label: string;
  value: TypedGroupValueV1;
}

export interface MicomConfiguralChecklistV1 {
  identical_indicators_and_coding: boolean;
  identical_data_treatment: boolean;
  identical_algorithm_settings: boolean;
  identical_model_specification: boolean;
  deterministic_sign_orientation_reviewed: boolean;
  analyst_review_confirmed: boolean;
}

export type MgaProcedureV1 =
  | "micom_pairwise"
  | "pairwise_permutation"
  | "omnibus_max_spread_permutation"
  | "henseler_pls_mga"
  | "bootstrap_difference_bc"
  | "parametric_pooled_variance"
  | "parametric_welch_satterthwaite"
  | "parametric_wald_omnibus";

export type MgaModelProfileV1 =
  | "general_sem_pls"
  | "multiple_two_way_moderation"
  | "bounded_three_way_moderation"
  | "bounded_two_way_moderated_mediation"
  | "multiple_nonnested_hoc"
  | "case_weighted_pls"
  | "frequency_weighted_pls"
  | "reflective_plsc";

export interface GroupPairV1 {
  left_group_id: string;
  right_group_id: string;
}

export type MgaComparisonPlanV1 =
  | { kind: "reference_vs_rest"; reference_group_id: string }
  | { kind: "selected_pairs"; pairs: GroupPairV1[] }
  | { kind: "all_pairs"; heavy_run_confirmed: boolean };

export type AnalysisWeightBindingV1 =
  { kind: "case"; column: string } | { kind: "frequency"; column: string };

export interface MgaMultigroupV1 {
  schema_version: typeof MGA_MULTIGROUP_V1_SCHEMA_VERSION;
  profile: MgaModelProfileV1;
  grouping_column: string;
  groups: SelectedGroupV1[];
  comparison_plan: MgaComparisonPlanV1;
  procedures: MgaProcedureV1[];
  permutation_samples: number;
  bootstrap_samples: number;
  seed: number;
  confidence_level: number;
  alpha: number;
  alternative: InferenceAlternativeV1;
  multiplicity: MultiplicityAdjustmentV1;
  configural_checklist: MicomConfiguralChecklistV1;
  weight?: AnalysisWeightBindingV1;
  selected_parameter_ids: string[];
}

export type HeterogeneityInteractionProfileV2 =
  "p0_structural" | "p2_multi_two_way" | "p23_all_current";

export type HeterogeneityAlgorithmV2 =
  | "fimix_pls_v2"
  | "pls_pos_published_v2"
  | "pls_pos_destination_scored_interactions_v2";

const HETEROGENEITY_ALGORITHMS_V2 = [
  "fimix_pls_v2",
  "pls_pos_published_v2",
  "pls_pos_destination_scored_interactions_v2",
] as const;

export interface FimixSettingsV2 {
  starts: number;
  max_iterations: number;
  relative_log_likelihood_tolerance: number;
  consecutive_converged_iterations: number;
  likelihood_decrease_tolerance: number;
  residual_variance_floor: number;
  rank_tolerance: number;
  minimum_class_share: number;
  required_reproducing_starts: number;
  optimum_relative_log_likelihood_tolerance: number;
  optimum_maximum_coefficient_difference: number;
  optimum_mean_posterior_difference: number;
}

export interface PlsPosSettingsV2 {
  starts: number;
  strict_improvement_tolerance: number;
  stable_objective_tolerance: number;
  minimum_reproducing_starts: number;
}

export interface PosCommonMetricComparabilityV1 {
  schema_version: 1;
  request_segment_contrasts: boolean;
  permutation_samples: number;
  configural_checklist: MicomConfiguralChecklistV1;
  require_partial_compositional_invariance: boolean;
}

export interface SegmentationBootstrapV2 {
  resamples: number;
  seed: number;
  confidence_level: number;
}

export interface HeterogeneityInferenceLockReceiptV2 {
  schema_version: 1;
  discovery_result_identity_sha256: string;
  discovery_candidate_k: number[];
  discovery_algorithms: HeterogeneityAlgorithmV2[];
  selected_algorithm: HeterogeneityAlgorithmV2;
  selected_k: number;
  analyst_lock_confirmed: true;
  tandem_fimix_same_k_start_required: boolean;
}

export type HeterogeneityPhaseV2 =
  | {
      kind: "discovery";
      candidate_k: number[];
      algorithms: HeterogeneityAlgorithmV2[];
    }
  | {
      kind: "inference";
      lock: HeterogeneityInferenceLockReceiptV2;
    };

export interface PlsUnobservedHeterogeneityConfigV2 {
  schema_version: typeof PLS_HETEROGENEITY_V2_SCHEMA_VERSION;
  profile: HeterogeneityInteractionProfileV2;
  phase: HeterogeneityPhaseV2;
  seed: number;
  fimix: FimixSettingsV2;
  pls_pos: PlsPosSettingsV2;
  pos_common_metric?: PosCommonMetricComparabilityV1;
  bootstrap?: SegmentationBootstrapV2;
}

export type ConditionalProcessProfileV2 =
  | "multi_two_way_percentile"
  | "multi_two_way_bca"
  | "multi_two_way_studentized"
  | "bounded_three_way_percentile"
  | "multiple_hoc_percentile"
  | "grouped_percentile"
  | "case_weighted_percentile"
  | "frequency_weighted_percentile";

export interface ConditionalProcessPathV2 {
  path_id: string;
  ordered_relation_ids: string[];
}

export type ConditionalProbeScaleV2 =
  "standardized_score" | "raw_observed_with_transformation_receipt";

export interface ConditionalRawProbeTransformationReceiptV2 {
  contract: typeof CONDITIONAL_RAW_PROBE_TRANSFORMATION_CONTRACT_V2;
  moderator_id: string;
  source_column: string;
  dataset_fingerprint: string;
  analysis_row_mask_sha256: string;
  center: number;
  sample_standard_deviation: number;
  orientation_sign: -1 | 1;
}

export type ConditionalRawProbeFitScopeV2 =
  | { kind: "analysis_fit" }
  | { kind: "group_fit"; group_id: string };

export type ConditionalRawProbeMetricBasisV2 =
  | "unweighted_sample"
  | "case_weighted_effective_df"
  | "frequency_expanded_sample";

export interface ConditionalRawProbeFitMetricReceiptV2 {
  contract: typeof CONDITIONAL_RAW_PROBE_FIT_METRIC_CONTRACT_V2;
  moderator_id: string;
  source_column: string;
  dataset_fingerprint: string;
  analysis_row_mask_sha256: string;
  fit_scope: ConditionalRawProbeFitScopeV2;
  metric_basis: ConditionalRawProbeMetricBasisV2;
  weight_column?: string;
  row_mass_sha256: string;
  compact_row_count: number;
  mass_sum: number;
  mass_squared_sum: number;
  effective_degrees_of_freedom: number;
  frequency_total?: number;
  center: number;
  standard_deviation: number;
  orientation_sign: -1 | 1;
}

export interface ConditionalModeratorProbeV2 {
  probe_id: string;
  moderator_id: string;
  scale: ConditionalProbeScaleV2;
  values: number[];
  raw_transformation_receipt?: ConditionalRawProbeTransformationReceiptV2;
  raw_fit_metric_receipts?: ConditionalRawProbeFitMetricReceiptV2[];
}

export function parseConditionalRawProbeFitMetricReceiptV2(
  value: unknown,
  moderatorId: string,
  path = "rawProbeMetric",
): ConditionalRawProbeFitMetricReceiptV2 {
  const item = exactRecordAt(
    value,
    [
      "contract",
      "moderator_id",
      "source_column",
      "dataset_fingerprint",
      "analysis_row_mask_sha256",
      "fit_scope",
      "metric_basis",
      "row_mass_sha256",
      "compact_row_count",
      "mass_sum",
      "mass_squared_sum",
      "effective_degrees_of_freedom",
      "center",
      "standard_deviation",
      "orientation_sign",
    ],
    ["weight_column", "frequency_total"],
    path,
  );
  if (item.contract !== CONDITIONAL_RAW_PROBE_FIT_METRIC_CONTRACT_V2)
    fail(
      "general_sem_conditional_process_v2.raw_probe_fit_metric_contract",
      `${path}.contract`,
      "Raw probes require the frozen fit-metric V2 contract.",
    );
  const receiptModerator = textAt(item.moderator_id, `${path}.moderator_id`, true);
  if (receiptModerator !== moderatorId)
    fail(
      "general_sem_conditional_process_v2.raw_probe_moderator",
      `${path}.moderator_id`,
      "Fit-metric receipt moderator must match its probe moderator.",
    );
  const scope = recordAt(item.fit_scope, `${path}.fit_scope`);
  const fitScope: ConditionalRawProbeFitScopeV2 = scope.kind === "analysis_fit"
    ? (exactRecordAt(scope, ["kind"], [], `${path}.fit_scope`), { kind: "analysis_fit" })
    : scope.kind === "group_fit"
      ? (exactRecordAt(scope, ["kind", "group_id"], [], `${path}.fit_scope`), {
          kind: "group_fit",
          group_id: textAt(scope.group_id, `${path}.fit_scope.group_id`, true),
        })
      : fail(
          "general_sem_conditional_process_v2.raw_probe_fit_scope",
          `${path}.fit_scope.kind`,
          "Fit-metric scope must be analysis_fit or group_fit.",
        );
  const metricBasis = enumAt(
    item.metric_basis,
    [
      "unweighted_sample",
      "case_weighted_effective_df",
      "frequency_expanded_sample",
    ] as const,
    `${path}.metric_basis`,
  );
  const compactRows = countAt(item.compact_row_count, `${path}.compact_row_count`);
  const massSum = finiteAt(item.mass_sum, `${path}.mass_sum`);
  const massSquaredSum = finiteAt(item.mass_squared_sum, `${path}.mass_squared_sum`);
  const effectiveDf = finiteAt(
    item.effective_degrees_of_freedom,
    `${path}.effective_degrees_of_freedom`,
  );
  const standardDeviation = finiteAt(item.standard_deviation, `${path}.standard_deviation`);
  const orientation = finiteAt(item.orientation_sign, `${path}.orientation_sign`);
  if (compactRows < 2 || massSum <= 0 || massSquaredSum <= 0 || effectiveDf <= 0
    || standardDeviation <= 0 || (orientation !== -1 && orientation !== 1))
    fail(
      "general_sem_conditional_process_v2.raw_probe_fit_metric_parameters",
      path,
      "Fit-metric mass, degrees of freedom, scale, or orientation is invalid.",
    );
  const weightColumn = item.weight_column == null
    ? undefined
    : textAt(item.weight_column, `${path}.weight_column`, true);
  const frequencyTotal = item.frequency_total == null
    ? undefined
    : countAt(item.frequency_total, `${path}.frequency_total`);
  const approximately = (left: number, right: number) =>
    Math.abs(left - right) <= 1e-12 * Math.max(Math.abs(left), Math.abs(right), 1);
  if (metricBasis === "unweighted_sample"
    ? weightColumn !== undefined || frequencyTotal !== undefined
      || !approximately(massSum, compactRows)
      || !approximately(massSquaredSum, compactRows)
      || !approximately(effectiveDf, compactRows - 1)
    : metricBasis === "case_weighted_effective_df"
      ? weightColumn === undefined || frequencyTotal !== undefined
        || !approximately(effectiveDf, massSum - massSquaredSum / massSum)
      : weightColumn === undefined || frequencyTotal === undefined || frequencyTotal < 2
        || !approximately(massSum, frequencyTotal)
        || !approximately(effectiveDf, frequencyTotal - 1))
    fail(
      `general_sem_conditional_process_v2.raw_probe_${metricBasis}_metric`,
      path,
      "Fit-metric receipt denominator semantics do not match its declared basis.",
    );
  return {
    contract: CONDITIONAL_RAW_PROBE_FIT_METRIC_CONTRACT_V2,
    moderator_id: receiptModerator,
    source_column: textAt(item.source_column, `${path}.source_column`, true),
    dataset_fingerprint: textAt(item.dataset_fingerprint, `${path}.dataset_fingerprint`, true),
    analysis_row_mask_sha256: shaAt(item.analysis_row_mask_sha256, `${path}.analysis_row_mask_sha256`),
    fit_scope: fitScope,
    metric_basis: metricBasis,
    ...(weightColumn === undefined ? {} : { weight_column: weightColumn }),
    row_mass_sha256: shaAt(item.row_mass_sha256, `${path}.row_mass_sha256`),
    compact_row_count: compactRows,
    mass_sum: massSum,
    mass_squared_sum: massSquaredSum,
    effective_degrees_of_freedom: effectiveDf,
    ...(frequencyTotal === undefined ? {} : { frequency_total: frequencyTotal }),
    center: finiteAt(item.center, `${path}.center`),
    standard_deviation: standardDeviation,
    orientation_sign: orientation as -1 | 1,
  };
}

export interface ConditionalJointProbeTupleV2 {
  tuple_id: string;
  values_by_moderator: Record<string, number>;
}

export interface ConditionalProbeContrastV2 {
  contrast_id: string;
  left_tuple_id: string;
  right_tuple_id: string;
}

export interface ConditionalGroupContrastV2 {
  contrast_id: string;
  left_group_id: string;
  right_group_id: string;
}

export interface ConditionalProcessEstimandsV2 {
  conditional_specific_indirect: boolean;
  conditional_total_indirect: boolean;
  conditional_total_effect: boolean;
  scalar_index_when_affine: boolean;
  local_first_derivatives: boolean;
  local_second_and_cross_derivatives: boolean;
  finite_probe_contrasts: boolean;
}

export type ConditionalProcessIntervalV2 = "percentile" | "bca" | "studentized";

export interface ConditionalProcessInferenceV2 {
  interval: ConditionalProcessIntervalV2;
  alternative: InferenceAlternativeV1;
  outer_resamples: number;
  inner_resamples: number;
  seed: number;
  confidence_level: number;
}

export interface GeneralSemConditionalProcessConfigV2 {
  schema_version: typeof GENERAL_SEM_CONDITIONAL_PROCESS_V2_SCHEMA_VERSION;
  profile: ConditionalProcessProfileV2;
  paths: ConditionalProcessPathV2[];
  declared_interaction_ids: string[];
  three_way_interaction_id?: string;
  hoc_ids: string[];
  moderator_ids: string[];
  probes: ConditionalModeratorProbeV2[];
  explicit_joint_tuples: ConditionalJointProbeTupleV2[];
  probe_contrasts: ConditionalProbeContrastV2[];
  grouping_column?: string;
  groups: SelectedGroupV1[];
  group_contrasts: ConditionalGroupContrastV2[];
  weight?: AnalysisWeightBindingV1;
  estimands: ConditionalProcessEstimandsV2;
  inference: ConditionalProcessInferenceV2;
}

export type ObservedTreatmentContrastV1 =
  | { kind: "binary"; control: number; treated: number }
  | { kind: "continuous"; x0: number; x1: number };

export interface CausalIdentificationChecklistV1 {
  temporal_order_declared: boolean;
  adjustment_set_justified: boolean;
  consistency_assumption_acknowledged: boolean;
  no_unmeasured_treatment_outcome_confounding_acknowledged: boolean;
  no_unmeasured_treatment_mediator_confounding_acknowledged: boolean;
  no_unmeasured_mediator_outcome_confounding_acknowledged: boolean;
  no_exposure_induced_mediator_outcome_confounder_confirmed: boolean;
  no_recanting_witness_confirmed: boolean;
  linear_model_specification_reviewed: boolean;
  positivity_reviewed: boolean;
}

export interface ObservedCausalPathV1 {
  path_id: string;
  ordered_variable_ids: string[];
  equations: CausalLinearEquationV1[];
}

export interface CausalLinearTermV1 {
  term_id: string;
  factor_variable_ids: string[];
}

export interface CausalLinearEquationV1 {
  equation_id: string;
  outcome_variable_id: string;
  terms: CausalLinearTermV1[];
}

export interface CausalPositivityPolicyV1 {
  minimum_binary_arm_count: number;
  maximum_binary_arm_ratio: number;
  positivity_strata_variable_ids: string[];
  minimum_count_per_binary_stratum_arm: number;
  continuous_neighborhood_fraction_of_range: number;
  minimum_continuous_neighborhood_count: number;
}

export interface InterventionalCausalMediationConfigV1 {
  schema_version: typeof INTERVENTIONAL_CAUSAL_MEDIATION_V1_SCHEMA_VERSION;
  treatment: string;
  treatment_contrast: ObservedTreatmentContrastV1;
  outcome: string;
  mediators: string[];
  baseline_moderators: string[];
  adjustment_covariates: string[];
  paths: ObservedCausalPathV1[];
  positivity_policy: CausalPositivityPolicyV1;
  identification: CausalIdentificationChecklistV1;
  bootstrap_resamples: number;
  seed: number;
  confidence_level: number;
}

export class MultiModContractErrorV1 extends Error {
  readonly code: string;
  readonly path: string;

  constructor(code: string, path: string, message: string) {
    super(message);
    this.name = "MultiModContractErrorV1";
    this.code = code;
    this.path = path;
  }
}

type UnknownRecord = Record<string, unknown>;

function fail(code: string, path: string, message: string): never {
  throw new MultiModContractErrorV1(code, path, message);
}

function recordAt(value: unknown, path: string): UnknownRecord {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail("multimod.shape.object", path, `${path} must be an object.`);
  }
  return value as UnknownRecord;
}

function hasOwn(record: UnknownRecord, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(record, key);
}

function exactRecordAt(
  value: unknown,
  required: readonly string[],
  optional: readonly string[],
  path: string,
): UnknownRecord {
  const record = recordAt(value, path);
  const allowed = new Set([...required, ...optional]);
  const unknown = Object.keys(record).find((key) => !allowed.has(key));
  if (unknown)
    fail(
      "multimod.shape.unknown_field",
      `${path}.${unknown}`,
      `${path}.${unknown} is not part of this versioned contract.`,
    );
  const missing = required.find((key) => !hasOwn(record, key));
  if (missing)
    fail(
      "multimod.shape.missing_field",
      `${path}.${missing}`,
      `${path}.${missing} is required.`,
    );
  return record;
}

function textAt(value: unknown, path: string, stable = false): string {
  if (
    typeof value !== "string" ||
    (stable &&
      (!value ||
        value.trim() !== value ||
        /[\u0000-\u001f\u007f]/u.test(value)))
  ) {
    fail(
      "multimod.value.text",
      path,
      `${path} must be ${stable ? "a stable nonempty identifier" : "text"}.`,
    );
  }
  return value;
}

function booleanAt(value: unknown, path: string): boolean {
  if (typeof value !== "boolean")
    fail("multimod.value.boolean", path, `${path} must be true or false.`);
  return value;
}

function finiteAt(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value))
    fail("multimod.value.finite", path, `${path} must be finite.`);
  return value;
}

function countAt(
  value: unknown,
  path: string,
  maximum = Number.MAX_SAFE_INTEGER,
): number {
  if (
    !Number.isSafeInteger(value) ||
    (value as number) < 0 ||
    (value as number) > maximum
  ) {
    fail(
      "multimod.value.count",
      path,
      `${path} must be a nonnegative safe integer no greater than ${maximum}.`,
    );
  }
  return value as number;
}

function signedSafeIntegerAt(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value)) {
    fail(
      "multimod.value.integer",
      path,
      `${path} must be a signed integer that can be represented without loss in JavaScript.`,
    );
  }
  return value as number;
}

function probabilityAt(value: unknown, path: string): number {
  const probability = finiteAt(value, path);
  if (!(probability > 0 && probability < 1))
    fail(
      "multimod.value.probability",
      path,
      `${path} must be strictly between zero and one.`,
    );
  return probability;
}

function unitProbabilityAt(value: unknown, path: string): number {
  const probability = finiteAt(value, path);
  if (probability < 0 || probability > 1)
    fail(
      "multimod.value.unit_probability",
      path,
      `${path} must be between zero and one.`,
    );
  return probability;
}

function enumAt<T extends string>(
  value: unknown,
  values: readonly T[],
  path: string,
): T {
  if (typeof value !== "string" || !values.includes(value as T)) {
    fail("multimod.value.enum", path, `${path} has an unsupported value.`);
  }
  return value as T;
}

function arrayAt(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value))
    fail("multimod.shape.array", path, `${path} must be an array.`);
  return value;
}

function uniqueStableTextArray(value: unknown, path: string): string[] {
  const values = arrayAt(value, path).map((item, index) =>
    textAt(item, `${path}[${index}]`, true),
  );
  if (new Set(values).size !== values.length)
    fail(
      "multimod.value.duplicate",
      path,
      `${path} must not contain duplicate identifiers.`,
    );
  return values;
}

function finiteMapAt(value: unknown, path: string): Record<string, number> {
  return Object.fromEntries(
    Object.entries(recordAt(value, path)).map(([key, item]) => [
      textAt(key, `${path}.key`, true),
      finiteAt(item, `${path}.${key}`),
    ]),
  );
}

function parseTypedGroupValueV1(
  value: unknown,
  path: string,
): TypedGroupValueV1 {
  const candidate = recordAt(value, path);
  if (candidate.kind === "text") {
    const item = exactRecordAt(candidate, ["kind", "value"], [], path);
    const text = textAt(item.value, `${path}.value`);
    if (!text)
      fail(
        "multimod.group_value_empty",
        `${path}.value`,
        "Text group values cannot be empty.",
      );
    return { kind: "text", value: text };
  }
  if (candidate.kind === "number") {
    const item = exactRecordAt(candidate, ["kind", "value"], [], path);
    return { kind: "number", value: finiteAt(item.value, `${path}.value`) };
  }
  if (candidate.kind === "integer") {
    const item = exactRecordAt(candidate, ["kind", "value"], [], path);
    return {
      kind: "integer",
      value: signedSafeIntegerAt(item.value, `${path}.value`),
    };
  }
  if (candidate.kind === "boolean") {
    const item = exactRecordAt(candidate, ["kind", "value"], [], path);
    return { kind: "boolean", value: booleanAt(item.value, `${path}.value`) };
  }
  return fail(
    "multimod.group_value_kind",
    `${path}.kind`,
    "Group values must be typed as text, integer, number, or boolean.",
  );
}

function typedGroupKey(value: TypedGroupValueV1): string {
  if (value.kind === "number")
    return `number:${Object.is(value.value, -0) ? 0 : value.value}`;
  return `${value.kind}:${String(value.value)}`;
}

function parseSelectedGroups(value: unknown, path: string): SelectedGroupV1[] {
  const groups = arrayAt(value, path).map((candidate, index) => {
    const itemPath = `${path}[${index}]`;
    const item = exactRecordAt(
      candidate,
      ["group_id", "label", "value"],
      [],
      itemPath,
    );
    const label = textAt(item.label, `${itemPath}.label`);
    if (!label.trim())
      fail(
        "multimod.group_label",
        `${itemPath}.label`,
        "Group labels cannot be empty.",
      );
    return {
      group_id: textAt(item.group_id, `${itemPath}.group_id`, true),
      label,
      value: parseTypedGroupValueV1(item.value, `${itemPath}.value`),
    };
  });
  if (new Set(groups.map((group) => group.group_id)).size !== groups.length)
    fail(
      "multimod.group_id_duplicate",
      path,
      "Group identifiers must be unique.",
    );
  if (
    new Set(groups.map((group) => typedGroupKey(group.value))).size !==
    groups.length
  )
    fail(
      "multimod.group_value_duplicate",
      path,
      "Typed group values must be unique.",
    );
  return groups;
}

export function parseMicomConfiguralChecklistV1(
  value: unknown,
  path = "configural_checklist",
): MicomConfiguralChecklistV1 {
  const fields = [
    "identical_indicators_and_coding",
    "identical_data_treatment",
    "identical_algorithm_settings",
    "identical_model_specification",
    "deterministic_sign_orientation_reviewed",
    "analyst_review_confirmed",
  ] as const;
  const item = exactRecordAt(value, fields, [], path);
  return Object.fromEntries(
    fields.map((field) => [field, booleanAt(item[field], `${path}.${field}`)]),
  ) as unknown as MicomConfiguralChecklistV1;
}

export function micomConfiguralChecklistCompleteV1(
  value: MicomConfiguralChecklistV1,
): boolean {
  return Object.values(value).every(Boolean);
}

function parseWeight(value: unknown, path: string): AnalysisWeightBindingV1 {
  const item = exactRecordAt(value, ["kind", "column"], [], path);
  const kind = enumAt(
    item.kind,
    ["case", "frequency"] as const,
    `${path}.kind`,
  );
  return { kind, column: textAt(item.column, `${path}.column`, true) };
}

function parseComparisonPlan(
  value: unknown,
  path: string,
): MgaComparisonPlanV1 {
  const candidate = recordAt(value, path);
  if (candidate.kind === "reference_vs_rest") {
    const item = exactRecordAt(
      candidate,
      ["kind", "reference_group_id"],
      [],
      path,
    );
    return {
      kind: "reference_vs_rest",
      reference_group_id: textAt(
        item.reference_group_id,
        `${path}.reference_group_id`,
        true,
      ),
    };
  }
  if (candidate.kind === "selected_pairs") {
    const item = exactRecordAt(candidate, ["kind", "pairs"], [], path);
    const pairs = arrayAt(item.pairs, `${path}.pairs`).map((pair, index) => {
      const pairPath = `${path}.pairs[${index}]`;
      const parsed = exactRecordAt(
        pair,
        ["left_group_id", "right_group_id"],
        [],
        pairPath,
      );
      return {
        left_group_id: textAt(
          parsed.left_group_id,
          `${pairPath}.left_group_id`,
          true,
        ),
        right_group_id: textAt(
          parsed.right_group_id,
          `${pairPath}.right_group_id`,
          true,
        ),
      };
    });
    if (!pairs.length)
      fail(
        "mga_multigroup_v1.pairs_empty",
        `${path}.pairs`,
        "Select at least one pair.",
      );
    return { kind: "selected_pairs", pairs };
  }
  if (candidate.kind === "all_pairs") {
    const item = exactRecordAt(
      candidate,
      ["kind", "heavy_run_confirmed"],
      [],
      path,
    );
    return {
      kind: "all_pairs",
      heavy_run_confirmed: booleanAt(
        item.heavy_run_confirmed,
        `${path}.heavy_run_confirmed`,
      ),
    };
  }
  return fail(
    "mga_multigroup_v1.comparison_plan",
    `${path}.kind`,
    "The MGA comparison plan is unsupported.",
  );
}

export function parseMgaMultigroupV1(
  value: unknown,
  path = "mga_multigroup",
): MgaMultigroupV1 {
  const item = exactRecordAt(
    value,
    [
      "schema_version",
      "profile",
      "grouping_column",
      "groups",
      "comparison_plan",
      "procedures",
      "permutation_samples",
      "bootstrap_samples",
      "seed",
      "confidence_level",
      "alpha",
      "alternative",
      "multiplicity",
      "configural_checklist",
    ],
    ["weight", "selected_parameter_ids"],
    path,
  );
  if (countAt(item.schema_version, `${path}.schema_version`) !== 1)
    fail(
      "mga_multigroup_v1.schema",
      `${path}.schema_version`,
      "MGA multigroup configuration requires schema version 1.",
    );
  const groups = parseSelectedGroups(item.groups, `${path}.groups`);
  if (groups.length < 2 || groups.length > 20)
    fail(
      "mga_multigroup_v1.group_count",
      `${path}.groups`,
      "Select between 2 and 20 groups.",
    );
  const procedures = arrayAt(item.procedures, `${path}.procedures`).map(
    (procedure, index) =>
      enumAt(
        procedure,
        [
          "micom_pairwise",
          "pairwise_permutation",
          "omnibus_max_spread_permutation",
          "henseler_pls_mga",
          "bootstrap_difference_bc",
          "parametric_pooled_variance",
          "parametric_welch_satterthwaite",
          "parametric_wald_omnibus",
        ] as const,
        `${path}.procedures[${index}]`,
      ),
  );
  if (!procedures.length || new Set(procedures).size !== procedures.length)
    fail(
      "mga_multigroup_v1.procedures",
      `${path}.procedures`,
      "Select one or more unique MGA procedures.",
    );
  const permutationSamples = countAt(
    item.permutation_samples,
    `${path}.permutation_samples`,
    10_000,
  );
  const bootstrapSamples = countAt(
    item.bootstrap_samples,
    `${path}.bootstrap_samples`,
    10_000,
  );
  if (permutationSamples < 5_000 || bootstrapSamples < 5_000)
    fail(
      "mga_multigroup_v1.resamples",
      path,
      "MGA resample counts must be between 5,000 and 10,000.",
    );
  const checklist = parseMicomConfiguralChecklistV1(
    item.configural_checklist,
    `${path}.configural_checklist`,
  );
  if (!micomConfiguralChecklistCompleteV1(checklist))
    fail(
      "mga_multigroup_v1.micom_step1_incomplete",
      `${path}.configural_checklist`,
      "Every MGA run requires every configural-invariance confirmation.",
    );
  const comparisonPlan = parseComparisonPlan(
    item.comparison_plan,
    `${path}.comparison_plan`,
  );
  const groupIds = new Set(groups.map((group) => group.group_id));
  if (
    comparisonPlan.kind === "reference_vs_rest" &&
    !groupIds.has(comparisonPlan.reference_group_id)
  )
    fail(
      "mga_multigroup_v1.reference_unknown",
      `${path}.comparison_plan.reference_group_id`,
      "The reference must be a selected group.",
    );
  if (comparisonPlan.kind === "selected_pairs") {
    const keys = new Set<string>();
    comparisonPlan.pairs.forEach((pair, index) => {
      if (
        pair.left_group_id === pair.right_group_id ||
        !groupIds.has(pair.left_group_id) ||
        !groupIds.has(pair.right_group_id)
      )
        fail(
          "mga_multigroup_v1.pair_invalid",
          `${path}.comparison_plan.pairs[${index}]`,
          "Each pair must contain two distinct selected groups.",
        );
      const key = [pair.left_group_id, pair.right_group_id]
        .sort()
        .join("\u0000");
      if (keys.has(key))
        fail(
          "mga_multigroup_v1.pair_duplicate",
          `${path}.comparison_plan.pairs[${index}]`,
          "Pairwise comparisons must be unique.",
        );
      keys.add(key);
    });
  }
  if (
    comparisonPlan.kind === "all_pairs" &&
    groups.length === 20 &&
    !comparisonPlan.heavy_run_confirmed
  )
    fail(
      "mga_multigroup_v1.heavy_run_confirmation",
      `${path}.comparison_plan.heavy_run_confirmed`,
      "All 190 comparisons require explicit confirmation.",
    );
  const profile = enumAt(
    item.profile,
    [
      "general_sem_pls",
      "multiple_two_way_moderation",
      "bounded_three_way_moderation",
      "bounded_two_way_moderated_mediation",
      "multiple_nonnested_hoc",
      "case_weighted_pls",
      "frequency_weighted_pls",
      "reflective_plsc",
    ] as const,
    `${path}.profile`,
  );
  if (
    groups.length < 3 &&
    procedures.includes("omnibus_max_spread_permutation")
  ) {
    fail(
      "mga_multigroup_v1.omnibus_requires_three_groups",
      `${path}.procedures`,
      "The max-spread omnibus procedure requires at least three selected groups.",
    );
  }
  if (
    groups.length >= 3 &&
    procedures.some((procedure) =>
      [
        "micom_pairwise",
        "pairwise_permutation",
        "henseler_pls_mga",
        "bootstrap_difference_bc",
        "parametric_pooled_variance",
        "parametric_welch_satterthwaite",
      ].includes(procedure),
    ) &&
    !procedures.includes("omnibus_max_spread_permutation")
  ) {
    fail(
      "mga_multigroup_v1.omnibus_required",
      `${path}.procedures`,
      "Three through twenty groups require the max-spread omnibus permutation before pairwise follow-up.",
    );
  }
  const confidenceLevel = probabilityAt(
    item.confidence_level,
    `${path}.confidence_level`,
  );
  const alpha = probabilityAt(item.alpha, `${path}.alpha`);
  if (confidenceLevel !== 0.95 || alpha !== 0.05)
    fail(
      "mga_multigroup_v1.fixed_error_rates",
      `${path}.confidence_level`,
      "MGA V1 uses the frozen 95% confidence and alpha .05 contract.",
    );
  const weight =
    item.weight == null
      ? undefined
      : parseWeight(item.weight, `${path}.weight`);
  if (
    (profile === "case_weighted_pls" && weight?.kind !== "case") ||
    (profile === "frequency_weighted_pls" && weight?.kind !== "frequency") ||
    (!profile.includes("weighted") && weight)
  )
    fail(
      "mga_multigroup_v1.weight_profile_mismatch",
      `${path}.weight`,
      "The selected profile and weight binding do not match.",
    );
  return {
    schema_version: 1,
    profile,
    grouping_column: textAt(
      item.grouping_column,
      `${path}.grouping_column`,
      true,
    ),
    groups,
    comparison_plan: comparisonPlan,
    procedures,
    permutation_samples: permutationSamples,
    bootstrap_samples: bootstrapSamples,
    seed: countAt(item.seed, `${path}.seed`),
    confidence_level: confidenceLevel,
    alpha,
    alternative: enumAt(
      item.alternative,
      ["two_sided", "less", "greater"] as const,
      `${path}.alternative`,
    ),
    multiplicity: enumAt(
      item.multiplicity,
      [
        "holm",
        "bonferroni",
        "sidak",
        "benjamini_hochberg_exploratory",
        "none_explicit",
      ] as const,
      `${path}.multiplicity`,
    ),
    configural_checklist: checklist,
    ...(weight ? { weight } : {}),
    selected_parameter_ids: uniqueStableTextArray(
      hasOwn(item, "selected_parameter_ids") ? item.selected_parameter_ids : [],
      `${path}.selected_parameter_ids`,
    ),
  };
}

function parseFimixSettings(value: unknown, path: string): FimixSettingsV2 {
  const item = exactRecordAt(
    value,
    [
      "starts",
      "max_iterations",
      "relative_log_likelihood_tolerance",
      "consecutive_converged_iterations",
      "likelihood_decrease_tolerance",
      "residual_variance_floor",
      "rank_tolerance",
      "minimum_class_share",
      "required_reproducing_starts",
      "optimum_relative_log_likelihood_tolerance",
      "optimum_maximum_coefficient_difference",
      "optimum_mean_posterior_difference",
    ],
    [],
    path,
  );
  const settings = {
    starts: countAt(item.starts, `${path}.starts`, 30),
    max_iterations: countAt(item.max_iterations, `${path}.max_iterations`),
    relative_log_likelihood_tolerance: finiteAt(
      item.relative_log_likelihood_tolerance,
      `${path}.relative_log_likelihood_tolerance`,
    ),
    consecutive_converged_iterations: countAt(
      item.consecutive_converged_iterations,
      `${path}.consecutive_converged_iterations`,
    ),
    likelihood_decrease_tolerance: finiteAt(
      item.likelihood_decrease_tolerance,
      `${path}.likelihood_decrease_tolerance`,
    ),
    residual_variance_floor: finiteAt(
      item.residual_variance_floor,
      `${path}.residual_variance_floor`,
    ),
    rank_tolerance: finiteAt(item.rank_tolerance, `${path}.rank_tolerance`),
    minimum_class_share: finiteAt(
      item.minimum_class_share,
      `${path}.minimum_class_share`,
    ),
    required_reproducing_starts: countAt(
      item.required_reproducing_starts,
      `${path}.required_reproducing_starts`,
      30,
    ),
    optimum_relative_log_likelihood_tolerance: finiteAt(
      item.optimum_relative_log_likelihood_tolerance,
      `${path}.optimum_relative_log_likelihood_tolerance`,
    ),
    optimum_maximum_coefficient_difference: finiteAt(
      item.optimum_maximum_coefficient_difference,
      `${path}.optimum_maximum_coefficient_difference`,
    ),
    optimum_mean_posterior_difference: finiteAt(
      item.optimum_mean_posterior_difference,
      `${path}.optimum_mean_posterior_difference`,
    ),
  };
  if (
    settings.starts !== 30 ||
    settings.max_iterations === 0 ||
    settings.max_iterations > 5_000 ||
    settings.relative_log_likelihood_tolerance !== 1e-10 ||
    settings.consecutive_converged_iterations !== 3 ||
    settings.likelihood_decrease_tolerance !== 1e-9 ||
    settings.residual_variance_floor < 1e-8 ||
    settings.rank_tolerance <= 0 ||
    !(
      settings.minimum_class_share >= 0.05 && settings.minimum_class_share < 0.5
    ) ||
    settings.required_reproducing_starts < 2 ||
    settings.required_reproducing_starts > settings.starts ||
    settings.optimum_relative_log_likelihood_tolerance <= 0 ||
    settings.optimum_maximum_coefficient_difference <= 0 ||
    settings.optimum_mean_posterior_difference <= 0
  ) {
    fail(
      "pls_heterogeneity_v2.fimix_settings",
      path,
      "FIMIX settings are outside the qualified numerical envelope.",
    );
  }
  return settings;
}

function parsePlsPosSettings(value: unknown, path: string): PlsPosSettingsV2 {
  const item = exactRecordAt(
    value,
    [
      "starts",
      "strict_improvement_tolerance",
      "stable_objective_tolerance",
      "minimum_reproducing_starts",
    ],
    [],
    path,
  );
  const settings = {
    starts: countAt(item.starts, `${path}.starts`),
    strict_improvement_tolerance: finiteAt(
      item.strict_improvement_tolerance,
      `${path}.strict_improvement_tolerance`,
    ),
    stable_objective_tolerance: finiteAt(
      item.stable_objective_tolerance,
      `${path}.stable_objective_tolerance`,
    ),
    minimum_reproducing_starts: countAt(
      item.minimum_reproducing_starts,
      `${path}.minimum_reproducing_starts`,
    ),
  };
  if (
    settings.starts !== 10 ||
    settings.strict_improvement_tolerance <= 0 ||
    settings.stable_objective_tolerance <= 0 ||
    settings.minimum_reproducing_starts !== 2
  ) {
    fail(
      "pls_heterogeneity_v2.pos_settings",
      path,
      "PLS-POS requires ten starts, a positive tolerance, and two reproducing starts.",
    );
  }
  return settings;
}

function parseHeterogeneityPhase(
  value: unknown,
  path: string,
): HeterogeneityPhaseV2 {
  const candidate = recordAt(value, path);
  if (candidate.kind === "discovery") {
    const item = exactRecordAt(
      candidate,
      ["kind", "candidate_k", "algorithms"],
      [],
      path,
    );
    const candidateK = arrayAt(item.candidate_k, `${path}.candidate_k`).map(
      (entry, index) => countAt(entry, `${path}.candidate_k[${index}]`, 5),
    );
    if (
      !candidateK.length ||
      candidateK.some((k) => k < 2) ||
      !candidateK.every((k, index) => index === 0 || candidateK[index - 1] < k)
    ) {
      fail(
        "pls_heterogeneity_v2.candidate_k",
        `${path}.candidate_k`,
        "Candidate K values must be ascending, unique, and between 2 and 5.",
      );
    }
    const algorithms = arrayAt(item.algorithms, `${path}.algorithms`).map(
      (entry, index) =>
        enumAt(entry, HETEROGENEITY_ALGORITHMS_V2, `${path}.algorithms[${index}]`),
    );
    if (!algorithms.length || new Set(algorithms).size !== algorithms.length)
      fail(
        "pls_heterogeneity_v2.algorithms",
        `${path}.algorithms`,
        "Select one or more unique segmentation algorithms.",
      );
    return { kind: "discovery", candidate_k: candidateK, algorithms };
  }
  if (candidate.kind === "inference") {
    const item = exactRecordAt(candidate, ["kind", "lock"], [], path);
    return {
      kind: "inference",
      lock: parseHeterogeneityInferenceLockReceiptV2(
        item.lock,
        `${path}.lock`,
      ),
    };
  }
  return fail(
    "pls_heterogeneity_v2.phase",
    `${path}.kind`,
    "Heterogeneity phase must be discovery or inference.",
  );
}

export function parseHeterogeneityInferenceLockReceiptV2(
  value: unknown,
  path = "pls_heterogeneity.phase.lock",
): HeterogeneityInferenceLockReceiptV2 {
  const item = exactRecordAt(
    value,
    [
      "schema_version",
      "discovery_result_identity_sha256",
      "discovery_candidate_k",
      "discovery_algorithms",
      "selected_algorithm",
      "selected_k",
      "analyst_lock_confirmed",
      "tandem_fimix_same_k_start_required",
    ],
    [],
    path,
  );
  if (countAt(item.schema_version, `${path}.schema_version`) !== 1)
    fail(
      "pls_heterogeneity_v2.inference_lock_schema",
      `${path}.schema_version`,
      "Heterogeneity inference lock requires schema version 1.",
    );
  const candidateK = arrayAt(
    item.discovery_candidate_k,
    `${path}.discovery_candidate_k`,
  ).map((entry, index) =>
    countAt(entry, `${path}.discovery_candidate_k[${index}]`, 5),
  );
  if (
    !candidateK.length ||
    candidateK.some((k) => k < 2) ||
    !candidateK.every((k, index) => index === 0 || candidateK[index - 1] < k)
  )
    fail(
      "pls_heterogeneity_v2.inference_lock_candidate_k",
      `${path}.discovery_candidate_k`,
      "Locked discovery K values must be unique, ascending, and between 2 and 5.",
    );
  const algorithms = arrayAt(
    item.discovery_algorithms,
    `${path}.discovery_algorithms`,
  ).map((entry, index) =>
    enumAt(
      entry,
      HETEROGENEITY_ALGORITHMS_V2,
      `${path}.discovery_algorithms[${index}]`,
    ),
  );
  if (
    !algorithms.length ||
    !algorithms.every(
      (algorithm, index) =>
        index === 0 ||
        HETEROGENEITY_ALGORITHMS_V2.indexOf(algorithms[index - 1]) <
          HETEROGENEITY_ALGORITHMS_V2.indexOf(algorithm),
    )
  )
    fail(
      "pls_heterogeneity_v2.inference_lock_algorithms",
      `${path}.discovery_algorithms`,
      "Locked discovery algorithms must be unique and canonical.",
    );
  const selectedAlgorithm = enumAt(
    item.selected_algorithm,
    HETEROGENEITY_ALGORITHMS_V2,
    `${path}.selected_algorithm`,
  );
  const selectedK = countAt(item.selected_k, `${path}.selected_k`, 5);
  if (!algorithms.includes(selectedAlgorithm))
    fail(
      "pls_heterogeneity_v2.inference_lock_algorithms",
      `${path}.selected_algorithm`,
      "Selected algorithm must be present in the locked discovery inventory.",
    );
  if (!candidateK.includes(selectedK))
    fail(
      "pls_heterogeneity_v2.inference_lock_selected_k",
      `${path}.selected_k`,
      "Selected K must be present in the locked discovery inventory and cannot be K=1.",
    );
  if (!booleanAt(item.analyst_lock_confirmed, `${path}.analyst_lock_confirmed`))
    fail(
      "pls_heterogeneity_v2.inference_lock_confirmation",
      `${path}.analyst_lock_confirmed`,
      "Fixed-K inference requires explicit analyst confirmation.",
    );
  const tandemRequired =
    selectedAlgorithm !== "fimix_pls_v2" &&
    algorithms.includes("fimix_pls_v2");
  const tandem = booleanAt(
    item.tandem_fimix_same_k_start_required,
    `${path}.tandem_fimix_same_k_start_required`,
  );
  if (tandem !== tandemRequired)
    fail(
      "pls_heterogeneity_v2.inference_lock_tandem_fimix",
      `${path}.tandem_fimix_same_k_start_required`,
      "Tandem FIMIX start authority must exactly reflect the discovery inventory.",
    );
  return {
    schema_version: 1,
    discovery_result_identity_sha256: shaAt(
      item.discovery_result_identity_sha256,
      `${path}.discovery_result_identity_sha256`,
    ),
    discovery_candidate_k: candidateK,
    discovery_algorithms: algorithms,
    selected_algorithm: selectedAlgorithm,
    selected_k: selectedK,
    analyst_lock_confirmed: true,
    tandem_fimix_same_k_start_required: tandem,
  };
}

export function parsePlsUnobservedHeterogeneityConfigV2(
  value: unknown,
  path = "pls_heterogeneity",
): PlsUnobservedHeterogeneityConfigV2 {
  const item = exactRecordAt(
    value,
    ["schema_version", "profile", "phase", "seed", "fimix", "pls_pos"],
    ["pos_common_metric", "bootstrap"],
    path,
  );
  if (countAt(item.schema_version, `${path}.schema_version`) !== 2)
    fail(
      "pls_heterogeneity_v2.schema",
      `${path}.schema_version`,
      "PLS heterogeneity configuration requires schema version 2.",
    );
  const profile = enumAt(
    item.profile,
    ["p0_structural", "p2_multi_two_way", "p23_all_current"] as const,
    `${path}.profile`,
  );
  const phase = parseHeterogeneityPhase(item.phase, `${path}.phase`);
  const algorithms =
    phase.kind === "discovery"
      ? phase.algorithms
      : [phase.lock.selected_algorithm];
  if (phase.kind === "discovery" && item.bootstrap != null)
    fail(
      "pls_heterogeneity_v2.discovery_bootstrap",
      `${path}.bootstrap`,
      "Bootstrap inference requires a locked algorithm and K.",
    );
  if (
    profile !== "p0_structural" &&
    algorithms.includes("pls_pos_published_v2")
  )
    fail(
      "pls_heterogeneity_v2.published_pos_interaction",
      `${path}.profile`,
      "Publication-faithful PLS-POS supports P0 only.",
    );
  if (
    profile === "p0_structural" &&
    algorithms.includes("pls_pos_destination_scored_interactions_v2")
  )
    fail(
      "pls_heterogeneity_v2.pos_extension_without_interaction",
      `${path}.profile`,
      "Destination-scored PLS-POS is reserved for interaction profiles.",
    );

  let posCommonMetric: PosCommonMetricComparabilityV1 | undefined;
  if (item.pos_common_metric != null) {
    const gate = exactRecordAt(
      item.pos_common_metric,
      [
        "schema_version",
        "request_segment_contrasts",
        "permutation_samples",
        "configural_checklist",
        "require_partial_compositional_invariance",
      ],
      [],
      `${path}.pos_common_metric`,
    );
    if (
      countAt(
        gate.schema_version,
        `${path}.pos_common_metric.schema_version`,
      ) !== 1
    )
      fail(
        "pls_heterogeneity_v2.common_metric",
        `${path}.pos_common_metric.schema_version`,
        "Common-metric comparability requires schema version 1.",
      );
    const permutationSamples = countAt(
      gate.permutation_samples,
      `${path}.pos_common_metric.permutation_samples`,
      10_000,
    );
    const checklist = parseMicomConfiguralChecklistV1(
      gate.configural_checklist,
      `${path}.pos_common_metric.configural_checklist`,
    );
    const requestContrasts = booleanAt(
      gate.request_segment_contrasts,
      `${path}.pos_common_metric.request_segment_contrasts`,
    );
    const requirePartial = booleanAt(
      gate.require_partial_compositional_invariance,
      `${path}.pos_common_metric.require_partial_compositional_invariance`,
    );
    if (
      permutationSamples < 5_000 ||
      (requestContrasts &&
        (!micomConfiguralChecklistCompleteV1(checklist) || !requirePartial))
    )
      fail(
        "pls_heterogeneity_v2.common_metric",
        `${path}.pos_common_metric`,
        "POS contrasts require the complete common-metric comparability contract.",
      );
    posCommonMetric = {
      schema_version: 1,
      request_segment_contrasts: requestContrasts,
      permutation_samples: permutationSamples,
      configural_checklist: checklist,
      require_partial_compositional_invariance: requirePartial,
    };
  }

  let bootstrap: SegmentationBootstrapV2 | undefined;
  if (item.bootstrap != null) {
    const inference = exactRecordAt(
      item.bootstrap,
      ["resamples", "seed", "confidence_level"],
      [],
      `${path}.bootstrap`,
    );
    const resamples = countAt(
      inference.resamples,
      `${path}.bootstrap.resamples`,
      10_000,
    );
    if (resamples < 500)
      fail(
        "pls_heterogeneity_v2.bootstrap_resamples",
        `${path}.bootstrap.resamples`,
        "Segmentation bootstrap requires 500 through 10,000 resamples.",
      );
    bootstrap = {
      resamples,
      seed: countAt(inference.seed, `${path}.bootstrap.seed`),
      confidence_level: probabilityAt(
        inference.confidence_level,
        `${path}.bootstrap.confidence_level`,
      ),
    };
  }
  return {
    schema_version: 2,
    profile,
    phase,
    seed: countAt(item.seed, `${path}.seed`),
    fimix: parseFimixSettings(item.fimix, `${path}.fimix`),
    pls_pos: parsePlsPosSettings(item.pls_pos, `${path}.pls_pos`),
    ...(posCommonMetric ? { pos_common_metric: posCommonMetric } : {}),
    ...(bootstrap ? { bootstrap } : {}),
  };
}

function parseConditionalEstimands(
  value: unknown,
  path: string,
): ConditionalProcessEstimandsV2 {
  const fields = [
    "conditional_specific_indirect",
    "conditional_total_indirect",
    "conditional_total_effect",
    "scalar_index_when_affine",
    "local_first_derivatives",
    "local_second_and_cross_derivatives",
    "finite_probe_contrasts",
  ] as const;
  const item = exactRecordAt(value, fields, [], path);
  const parsed = Object.fromEntries(
    fields.map((field) => [field, booleanAt(item[field], `${path}.${field}`)]),
  ) as unknown as ConditionalProcessEstimandsV2;
  if (!Object.values(parsed).some(Boolean))
    fail(
      "general_sem_conditional_process_v2.estimands_empty",
      path,
      "Select at least one conditional-process estimand.",
    );
  return parsed;
}

function parseConditionalInference(
  value: unknown,
  path: string,
): ConditionalProcessInferenceV2 {
  const item = exactRecordAt(
    value,
    [
      "interval",
      "alternative",
      "outer_resamples",
      "inner_resamples",
      "seed",
      "confidence_level",
    ],
    [],
    path,
  );
  return {
    interval: enumAt(
      item.interval,
      ["percentile", "bca", "studentized"] as const,
      `${path}.interval`,
    ),
    alternative: enumAt(
      item.alternative,
      ["two_sided", "less", "greater"] as const,
      `${path}.alternative`,
    ),
    outer_resamples: countAt(
      item.outer_resamples,
      `${path}.outer_resamples`,
      10_000,
    ),
    inner_resamples: countAt(
      item.inner_resamples,
      `${path}.inner_resamples`,
      1_000,
    ),
    seed: countAt(item.seed, `${path}.seed`),
    confidence_level: probabilityAt(
      item.confidence_level,
      `${path}.confidence_level`,
    ),
  };
}

export function parseGeneralSemConditionalProcessConfigV2(
  value: unknown,
  path = "conditional_process",
): GeneralSemConditionalProcessConfigV2 {
  const item = exactRecordAt(
    value,
    [
      "schema_version",
      "profile",
      "paths",
      "declared_interaction_ids",
      "moderator_ids",
      "probes",
      "estimands",
      "inference",
    ],
    [
      "three_way_interaction_id",
      "hoc_ids",
      "explicit_joint_tuples",
      "probe_contrasts",
      "grouping_column",
      "groups",
      "group_contrasts",
      "weight",
    ],
    path,
  );
  if (countAt(item.schema_version, `${path}.schema_version`) !== 2)
    fail(
      "general_sem_conditional_process_v2.schema",
      `${path}.schema_version`,
      "Conditional-process configuration requires schema version 2.",
    );
  const profile = enumAt(
    item.profile,
    [
      "multi_two_way_percentile",
      "multi_two_way_bca",
      "multi_two_way_studentized",
      "bounded_three_way_percentile",
      "multiple_hoc_percentile",
      "grouped_percentile",
      "case_weighted_percentile",
      "frequency_weighted_percentile",
    ] as const,
    `${path}.profile`,
  );
  const bounds: Record<
    ConditionalProcessProfileV2,
    readonly [number, number, number, number, number]
  > = {
    multi_two_way_percentile: [8, 8, 2, 6, 4],
    multi_two_way_bca: [8, 8, 2, 6, 4],
    multi_two_way_studentized: [4, 2, 2, 4, 3],
    bounded_three_way_percentile: [8, 4, 2, 5, 4],
    multiple_hoc_percentile: [2, 8, 2, 6, 4],
    grouped_percentile: [4, 4, 2, 4, 4],
    case_weighted_percentile: [4, 4, 2, 4, 4],
    frequency_weighted_percentile: [4, 4, 2, 4, 4],
  };
  const [maxInteractions, maxPaths, minEdges, maxEdges, maxModerators] =
    bounds[profile];
  const paths = arrayAt(item.paths, `${path}.paths`).map((candidate, index) => {
    const itemPath = `${path}.paths[${index}]`;
    const parsed = exactRecordAt(
      candidate,
      ["path_id", "ordered_relation_ids"],
      [],
      itemPath,
    );
    const relationIds = uniqueStableTextArray(
      parsed.ordered_relation_ids,
      `${itemPath}.ordered_relation_ids`,
    );
    if (relationIds.length < minEdges || relationIds.length > maxEdges)
      fail(
        "general_sem_conditional_process_v2.path_length",
        `${itemPath}.ordered_relation_ids`,
        `Path length must be between ${minEdges} and ${maxEdges}.`,
      );
    return {
      path_id: textAt(parsed.path_id, `${itemPath}.path_id`, true),
      ordered_relation_ids: relationIds,
    };
  });
  if (
    !paths.length ||
    paths.length > maxPaths ||
    new Set(paths.map((candidate) => candidate.path_id)).size !== paths.length
  )
    fail(
      "general_sem_conditional_process_v2.path_count",
      `${path}.paths`,
      `Select one through ${maxPaths} unique paths.`,
    );
  const interactionIds = uniqueStableTextArray(
    item.declared_interaction_ids,
    `${path}.declared_interaction_ids`,
  );
  if (!interactionIds.length || interactionIds.length > maxInteractions)
    fail(
      "general_sem_conditional_process_v2.interaction_count",
      `${path}.declared_interaction_ids`,
      `Select one through ${maxInteractions} interactions.`,
    );
  const moderatorIds = uniqueStableTextArray(
    item.moderator_ids,
    `${path}.moderator_ids`,
  );
  if (!moderatorIds.length || moderatorIds.length > maxModerators)
    fail(
      "general_sem_conditional_process_v2.moderator_count",
      `${path}.moderator_ids`,
      `Select one through ${maxModerators} moderators.`,
    );
  const threeWay =
    item.three_way_interaction_id == null
      ? undefined
      : textAt(
          item.three_way_interaction_id,
          `${path}.three_way_interaction_id`,
          true,
        );
  if ((profile === "bounded_three_way_percentile") !== Boolean(threeWay))
    fail(
      "general_sem_conditional_process_v2.three_way_profile",
      `${path}.three_way_interaction_id`,
      "Exactly one three-way term is required only for the bounded three-way profile.",
    );
  const hocIds = uniqueStableTextArray(
    hasOwn(item, "hoc_ids") ? item.hoc_ids : [],
    `${path}.hoc_ids`,
  );
  if (
    (profile === "multiple_hoc_percentile" &&
      (hocIds.length < 1 || hocIds.length > 4)) ||
    (profile !== "multiple_hoc_percentile" && hocIds.length)
  )
    fail(
      "general_sem_conditional_process_v2.hoc_profile",
      `${path}.hoc_ids`,
      "HOCs are admitted only by the multiple-HOC profile.",
    );
  const probes = arrayAt(item.probes, `${path}.probes`).map(
    (candidate, index) => {
      const probePath = `${path}.probes[${index}]`;
      const probe = exactRecordAt(
        candidate,
        ["probe_id", "moderator_id", "scale", "values"],
        ["raw_transformation_receipt", "raw_fit_metric_receipts"],
        probePath,
      );
      const values = arrayAt(probe.values, `${probePath}.values`).map(
        (entry, valueIndex) =>
          finiteAt(entry, `${probePath}.values[${valueIndex}]`),
      );
      if (
        !values.length ||
        values.length > 5 ||
        values.some(
          (value, valueIndex) =>
            valueIndex > 0 && values[valueIndex - 1] >= value,
        )
      )
        fail(
          "general_sem_conditional_process_v2.probe_values",
          `${probePath}.values`,
          "Each moderator requires one through five unique ascending probe values.",
        );
      const moderatorId = textAt(
        probe.moderator_id,
        `${probePath}.moderator_id`,
        true,
      );
      if (!moderatorIds.includes(moderatorId))
        fail(
          "general_sem_conditional_process_v2.probe_moderator",
          `${probePath}.moderator_id`,
          "Probe moderator must be declared.",
        );
      const scale = enumAt(
        probe.scale,
        [
          "standardized_score",
          "raw_observed_with_transformation_receipt",
        ] as const,
        `${probePath}.scale`,
      );
      let rawTransformationReceipt:
        ConditionalRawProbeTransformationReceiptV2 | undefined;
      if (
        hasOwn(probe, "raw_transformation_receipt") &&
        probe.raw_transformation_receipt != null
      ) {
        const receiptPath = `${probePath}.raw_transformation_receipt`;
        const receipt = exactRecordAt(
          probe.raw_transformation_receipt,
          [
            "contract",
            "moderator_id",
            "source_column",
            "dataset_fingerprint",
            "analysis_row_mask_sha256",
            "center",
            "sample_standard_deviation",
            "orientation_sign",
          ],
          [],
          receiptPath,
        );
        const contract = textAt(
          receipt.contract,
          `${receiptPath}.contract`,
          true,
        );
        if (contract !== CONDITIONAL_RAW_PROBE_TRANSFORMATION_CONTRACT_V2)
          fail(
            "general_sem_conditional_process_v2.raw_probe_contract",
            `${receiptPath}.contract`,
            "Raw probes require the frozen sample-standardization contract.",
          );
        const receiptModeratorId = textAt(
          receipt.moderator_id,
          `${receiptPath}.moderator_id`,
          true,
        );
        if (receiptModeratorId !== moderatorId)
          fail(
            "general_sem_conditional_process_v2.raw_probe_moderator",
            `${receiptPath}.moderator_id`,
            "Receipt moderator must match its probe moderator.",
          );
        const sampleStandardDeviation = finiteAt(
          receipt.sample_standard_deviation,
          `${receiptPath}.sample_standard_deviation`,
        );
        if (sampleStandardDeviation <= 0)
          fail(
            "general_sem_conditional_process_v2.raw_probe_scale",
            `${receiptPath}.sample_standard_deviation`,
            "Raw-probe sample standard deviation must be positive.",
          );
        const orientationValue = finiteAt(
          receipt.orientation_sign,
          `${receiptPath}.orientation_sign`,
        );
        if (orientationValue !== -1 && orientationValue !== 1)
          fail(
            "general_sem_conditional_process_v2.raw_probe_orientation",
            `${receiptPath}.orientation_sign`,
            "Raw-probe orientation must be -1 or +1.",
          );
        const orientationSign: -1 | 1 = orientationValue;
        rawTransformationReceipt = {
          contract: CONDITIONAL_RAW_PROBE_TRANSFORMATION_CONTRACT_V2,
          moderator_id: receiptModeratorId,
          source_column: textAt(
            receipt.source_column,
            `${receiptPath}.source_column`,
            true,
          ),
          dataset_fingerprint: textAt(
            receipt.dataset_fingerprint,
            `${receiptPath}.dataset_fingerprint`,
            true,
          ),
          analysis_row_mask_sha256: shaAt(
            receipt.analysis_row_mask_sha256,
            `${receiptPath}.analysis_row_mask_sha256`,
          ),
          center: finiteAt(receipt.center, `${receiptPath}.center`),
          sample_standard_deviation: sampleStandardDeviation,
          orientation_sign: orientationSign,
        };
      }
      const rawFitMetricReceipts = arrayAt(
        hasOwn(probe, "raw_fit_metric_receipts")
          ? probe.raw_fit_metric_receipts
          : [],
        `${probePath}.raw_fit_metric_receipts`,
      ).map((receipt, receiptIndex) =>
        parseConditionalRawProbeFitMetricReceiptV2(
          receipt,
          moderatorId,
          `${probePath}.raw_fit_metric_receipts[${receiptIndex}]`,
        ));
      if (scale === "standardized_score"
        ? Boolean(rawTransformationReceipt) || rawFitMetricReceipts.length > 0
        : Boolean(rawTransformationReceipt) === (rawFitMetricReceipts.length > 0))
        fail(
          "general_sem_conditional_process_v2.raw_probe_receipt",
          probePath,
          "Standardized probes accept no raw receipt; raw-unit probes require exactly one receipt family.",
        );
      return {
        probe_id: textAt(probe.probe_id, `${probePath}.probe_id`, true),
        moderator_id: moderatorId,
        scale,
        values,
        ...(rawTransformationReceipt
          ? { raw_transformation_receipt: rawTransformationReceipt }
          : {}),
        ...(rawFitMetricReceipts.length
          ? { raw_fit_metric_receipts: rawFitMetricReceipts }
          : {}),
      };
    },
  );
  if (
    probes.length !== moderatorIds.length ||
    new Set(probes.map((probe) => probe.probe_id)).size !== probes.length ||
    new Set(probes.map((probe) => probe.moderator_id)).size !== probes.length
  )
    fail(
      "general_sem_conditional_process_v2.probes",
      `${path}.probes`,
      "Provide exactly one uniquely identified probe for every moderator.",
    );
  const cartesianPoints = probes.reduce(
    (total, probe) => total * probe.values.length,
    1,
  );
  if (cartesianPoints > 81)
    fail(
      "general_sem_conditional_process_v2.cartesian_cap",
      `${path}.probes`,
      "The Cartesian probe grid exceeds 81 points.",
    );

  const explicitJointTuples = arrayAt(
    hasOwn(item, "explicit_joint_tuples") ? item.explicit_joint_tuples : [],
    `${path}.explicit_joint_tuples`,
  ).map((candidate, index) => {
    const tuplePath = `${path}.explicit_joint_tuples[${index}]`;
    const tuple = exactRecordAt(
      candidate,
      ["tuple_id", "values_by_moderator"],
      [],
      tuplePath,
    );
    const values = finiteMapAt(
      tuple.values_by_moderator,
      `${tuplePath}.values_by_moderator`,
    );
    if (
      Object.keys(values).length !== moderatorIds.length ||
      !moderatorIds.every((id) => hasOwn(values, id))
    )
      fail(
        "general_sem_conditional_process_v2.tuple_shape",
        `${tuplePath}.values_by_moderator`,
        "A joint tuple requires one value for every moderator.",
      );
    return {
      tuple_id: textAt(tuple.tuple_id, `${tuplePath}.tuple_id`, true),
      values_by_moderator: values,
    };
  });
  if (
    explicitJointTuples.length > 100 ||
    new Set(explicitJointTuples.map((tuple) => tuple.tuple_id)).size !==
      explicitJointTuples.length
  )
    fail(
      "general_sem_conditional_process_v2.tuple_cap",
      `${path}.explicit_joint_tuples`,
      "Use at most 100 uniquely identified explicit tuples.",
    );
  const tupleIds = new Set(explicitJointTuples.map((tuple) => tuple.tuple_id));
  const probeContrasts = arrayAt(
    hasOwn(item, "probe_contrasts") ? item.probe_contrasts : [],
    `${path}.probe_contrasts`,
  ).map((candidate, index) => {
    const contrastPath = `${path}.probe_contrasts[${index}]`;
    const contrast = exactRecordAt(
      candidate,
      ["contrast_id", "left_tuple_id", "right_tuple_id"],
      [],
      contrastPath,
    );
    const parsed = {
      contrast_id: textAt(
        contrast.contrast_id,
        `${contrastPath}.contrast_id`,
        true,
      ),
      left_tuple_id: textAt(
        contrast.left_tuple_id,
        `${contrastPath}.left_tuple_id`,
        true,
      ),
      right_tuple_id: textAt(
        contrast.right_tuple_id,
        `${contrastPath}.right_tuple_id`,
        true,
      ),
    };
    if (
      parsed.left_tuple_id === parsed.right_tuple_id ||
      !tupleIds.has(parsed.left_tuple_id) ||
      !tupleIds.has(parsed.right_tuple_id)
    )
      fail(
        "general_sem_conditional_process_v2.probe_contrast",
        contrastPath,
        "Probe contrasts require two distinct explicit tuples.",
      );
    return parsed;
  });
  if (
    probeContrasts.length > 16 ||
    new Set(probeContrasts.map((contrast) => contrast.contrast_id)).size !==
      probeContrasts.length
  )
    fail(
      "general_sem_conditional_process_v2.probe_contrast_cap",
      `${path}.probe_contrasts`,
      "Use at most 16 uniquely identified probe contrasts.",
    );

  const groups = parseSelectedGroups(
    hasOwn(item, "groups") ? item.groups : [],
    `${path}.groups`,
  );
  const groupContrasts = arrayAt(
    hasOwn(item, "group_contrasts") ? item.group_contrasts : [],
    `${path}.group_contrasts`,
  ).map((candidate, index) => {
    const contrastPath = `${path}.group_contrasts[${index}]`;
    const contrast = exactRecordAt(
      candidate,
      ["contrast_id", "left_group_id", "right_group_id"],
      [],
      contrastPath,
    );
    return {
      contrast_id: textAt(
        contrast.contrast_id,
        `${contrastPath}.contrast_id`,
        true,
      ),
      left_group_id: textAt(
        contrast.left_group_id,
        `${contrastPath}.left_group_id`,
        true,
      ),
      right_group_id: textAt(
        contrast.right_group_id,
        `${contrastPath}.right_group_id`,
        true,
      ),
    };
  });
  if (
    new Set(groupContrasts.map((contrast) => contrast.contrast_id)).size !==
    groupContrasts.length
  ) {
    fail(
      "general_sem_conditional_process_v2.group_contrast_cap",
      `${path}.group_contrasts`,
      "Group-contrast identifiers must be unique.",
    );
  }
  const groupingColumn =
    item.grouping_column == null
      ? undefined
      : textAt(item.grouping_column, `${path}.grouping_column`, true);
  if (profile === "grouped_percentile") {
    const maximumPairs = (groups.length * (groups.length - 1)) / 2;
    if (
      !groupingColumn ||
      groups.length < 2 ||
      groups.length > 20 ||
      groupContrasts.length > maximumPairs
    )
      fail(
        "general_sem_conditional_process_v2.group_profile",
        `${path}.groups`,
        "Grouped inference requires a grouping column, 2 through 20 groups, and no more contrasts than unique group pairs.",
      );
    const groupIds = new Set(groups.map((group) => group.group_id));
    const groupPairs = new Set<string>();
    groupContrasts.forEach((contrast, index) => {
      if (
        contrast.left_group_id === contrast.right_group_id ||
        !groupIds.has(contrast.left_group_id) ||
        !groupIds.has(contrast.right_group_id)
      ) {
        fail(
          "general_sem_conditional_process_v2.group_contrast",
          `${path}.group_contrasts[${index}]`,
          "Group contrasts require two distinct selected groups.",
        );
      }
      const pair = [contrast.left_group_id, contrast.right_group_id]
        .sort()
        .join("\u0000");
      if (groupPairs.has(pair))
        fail(
          "general_sem_conditional_process_v2.group_contrast_duplicate",
          `${path}.group_contrasts[${index}]`,
          "Each selected-group pair can appear only once.",
        );
      groupPairs.add(pair);
    });
  } else if (groupingColumn || groups.length || groupContrasts.length)
    fail(
      "general_sem_conditional_process_v2.group_profile",
      `${path}.groups`,
      "Groups are admitted only by the grouped profile.",
    );
  const weight =
    item.weight == null
      ? undefined
      : parseWeight(item.weight, `${path}.weight`);
  if (
    (profile === "case_weighted_percentile" && weight?.kind !== "case") ||
    (profile === "frequency_weighted_percentile" &&
      weight?.kind !== "frequency") ||
    (!profile.includes("weighted") && weight)
  )
    fail(
      "general_sem_conditional_process_v2.weight_profile",
      `${path}.weight`,
      "The profile and weight binding do not match.",
    );
  for (const [probeIndex, probe] of probes.entries()) {
    if (probe.scale !== "raw_observed_with_transformation_receipt") continue;
    if (probe.raw_transformation_receipt
      && ["grouped_percentile", "case_weighted_percentile", "frequency_weighted_percentile"].includes(profile))
      fail(
        "general_sem_conditional_process_v2.raw_probe_fit_metric_required",
        `${path}.probes[${probeIndex}]`,
        "Grouped and weighted raw probes require scoped fit-metric receipts.",
      );
    const receipts = probe.raw_fit_metric_receipts ?? [];
    if (!receipts.length) continue;
    const expectedBasis: ConditionalRawProbeMetricBasisV2 = profile === "case_weighted_percentile"
      ? "case_weighted_effective_df"
      : profile === "frequency_weighted_percentile"
        ? "frequency_expanded_sample"
        : "unweighted_sample";
    const expectedWeight = profile === "case_weighted_percentile" || profile === "frequency_weighted_percentile"
      ? weight?.column
      : undefined;
    const expectedScopes = profile === "grouped_percentile"
      ? groups.map((group) => `group:${group.group_id}`).sort()
      : ["analysis"];
    const actualScopes = receipts.map((receipt) => receipt.fit_scope.kind === "analysis_fit"
      ? "analysis"
      : `group:${receipt.fit_scope.group_id}`).sort();
    const first = receipts[0]!;
    if (new Set(actualScopes).size !== actualScopes.length
      || JSON.stringify(actualScopes) !== JSON.stringify(expectedScopes)
      || receipts.some((receipt) => receipt.metric_basis !== expectedBasis
        || receipt.weight_column !== expectedWeight
        || receipt.source_column !== first.source_column
        || receipt.dataset_fingerprint !== first.dataset_fingerprint
        || receipt.analysis_row_mask_sha256 !== first.analysis_row_mask_sha256))
      fail(
        "general_sem_conditional_process_v2.raw_probe_scope_profile",
        `${path}.probes[${probeIndex}].raw_fit_metric_receipts`,
        "Fit-metric receipt scope, basis, weight, or original-sample authority differs from the selected profile.",
      );
  }
  const inference = parseConditionalInference(
    item.inference,
    `${path}.inference`,
  );
  const expectedInterval =
    profile === "multi_two_way_bca"
      ? "bca"
      : profile === "multi_two_way_studentized"
        ? "studentized"
        : "percentile";
  if (inference.interval !== expectedInterval)
    fail(
      "general_sem_conditional_process_v2.interval_profile",
      `${path}.inference.interval`,
      "The selected interval family is not admitted by this profile.",
    );
  if (
    [
      "multiple_hoc_percentile",
      "grouped_percentile",
      "case_weighted_percentile",
      "frequency_weighted_percentile",
    ].includes(profile) &&
    inference.alternative !== "two_sided"
  )
    fail(
      "general_sem_conditional_process_v2.alternative_profile",
      `${path}.inference.alternative`,
      "This profile is qualified for two-sided inference only.",
    );
  if (inference.outer_resamples < 500)
    fail(
      "general_sem_conditional_process_v2.outer_resamples",
      `${path}.inference.outer_resamples`,
      "Outer resamples must be between 500 and 10,000.",
    );
  if (inference.interval === "studentized") {
    if (
      inference.inner_resamples < 100 ||
      inference.outer_resamples > 5_000 ||
      (inference.outer_resamples + 1) * inference.inner_resamples > 1_000_000
    )
      fail(
        "general_sem_conditional_process_v2.studentized_budget",
        `${path}.inference`,
        "Studentized inference exceeds its outer/inner refit envelope.",
      );
  } else if (inference.inner_resamples !== 0)
    fail(
      "general_sem_conditional_process_v2.inner_resamples",
      `${path}.inference.inner_resamples`,
      "Inner resamples are reserved for studentized inference.",
    );
  const estimands = parseConditionalEstimands(
    item.estimands,
    `${path}.estimands`,
  );
  if (estimands.finite_probe_contrasts !== probeContrasts.length > 0)
    fail(
      "general_sem_conditional_process_v2.probe_contrast_estimand",
      `${path}.estimands.finite_probe_contrasts`,
      "Finite probe contrasts require declared contrasts, and declared contrasts must be selected as an estimand.",
    );
  const pointCount = explicitJointTuples.length || cartesianPoints;
  if (profile === "multi_two_way_studentized" && pointCount > 27)
    fail(
      "general_sem_conditional_process_v2.studentized_probe_cap",
      `${path}.probes`,
      "Studentized inference supports at most 27 joint probe points.",
    );
  const groupFactor = profile === "grouped_percentile" ? groups.length : 1;
  const cells = paths.length * pointCount * groupFactor;
  if (cells > 512)
    fail(
      "general_sem_conditional_process_v2.cell_cap",
      path,
      "Path × group × probe cells exceed 512.",
    );
  const cellEffects =
    Number(estimands.conditional_specific_indirect) +
    Number(estimands.conditional_total_indirect) +
    Number(estimands.conditional_total_effect) +
    Number(estimands.local_first_derivatives) * moderatorIds.length +
    (Number(estimands.local_second_and_cross_derivatives) *
      moderatorIds.length *
      (moderatorIds.length + 1)) /
      2;
  const targetEstimate =
    cells * cellEffects +
    Number(estimands.scalar_index_when_affine) * paths.length * groupFactor +
    Number(estimands.finite_probe_contrasts) *
      paths.length *
      groupFactor *
      probeContrasts.length +
    groupContrasts.length *
      paths.length *
      pointCount *
      (cellEffects + Number(estimands.scalar_index_when_affine));
  const targetLimit = profile === "multi_two_way_studentized" ? 256 : 1_024;
  if (targetEstimate < 1 || targetEstimate > targetLimit)
    fail(
      "general_sem_conditional_process_v2.target_cap",
      path,
      `The inferential target estimate must be between 1 and ${targetLimit}.`,
    );
  return {
    schema_version: 2,
    profile,
    paths,
    declared_interaction_ids: interactionIds,
    ...(threeWay ? { three_way_interaction_id: threeWay } : {}),
    hoc_ids: hocIds,
    moderator_ids: moderatorIds,
    probes,
    explicit_joint_tuples: explicitJointTuples,
    probe_contrasts: probeContrasts,
    ...(groupingColumn ? { grouping_column: groupingColumn } : {}),
    groups,
    group_contrasts: groupContrasts,
    ...(weight ? { weight } : {}),
    estimands,
    inference,
  };
}

export function parseInterventionalCausalMediationConfigV1(
  value: unknown,
  path = "interventional_mediation",
): InterventionalCausalMediationConfigV1 {
  const item = exactRecordAt(
    value,
    [
      "schema_version",
      "treatment",
      "treatment_contrast",
      "outcome",
      "mediators",
      "paths",
      "adjustment_covariates",
      "positivity_policy",
      "identification",
      "bootstrap_resamples",
      "seed",
      "confidence_level",
    ],
    ["baseline_moderators"],
    path,
  );
  if (countAt(item.schema_version, `${path}.schema_version`) !== 1)
    fail(
      "interventional_causal_mediation_v1.schema",
      `${path}.schema_version`,
      "Interventional causal mediation requires schema version 1.",
    );
  const treatment = textAt(item.treatment, `${path}.treatment`, true);
  const outcome = textAt(item.outcome, `${path}.outcome`, true);
  if (treatment === outcome)
    fail(
      "interventional_causal_mediation_v1.endpoint",
      path,
      "Treatment and outcome must differ.",
    );
  const contrastCandidate = recordAt(
    item.treatment_contrast,
    `${path}.treatment_contrast`,
  );
  let treatmentContrast: ObservedTreatmentContrastV1;
  if (contrastCandidate.kind === "binary") {
    const contrast = exactRecordAt(
      contrastCandidate,
      ["kind", "control", "treated"],
      [],
      `${path}.treatment_contrast`,
    );
    treatmentContrast = {
      kind: "binary",
      control: finiteAt(contrast.control, `${path}.treatment_contrast.control`),
      treated: finiteAt(contrast.treated, `${path}.treatment_contrast.treated`),
    };
    if (treatmentContrast.control === treatmentContrast.treated)
      fail(
        "interventional_causal_mediation_v1.treatment_contrast",
        `${path}.treatment_contrast`,
        "Treatment endpoints must differ.",
      );
  } else if (contrastCandidate.kind === "continuous") {
    const contrast = exactRecordAt(
      contrastCandidate,
      ["kind", "x0", "x1"],
      [],
      `${path}.treatment_contrast`,
    );
    treatmentContrast = {
      kind: "continuous",
      x0: finiteAt(contrast.x0, `${path}.treatment_contrast.x0`),
      x1: finiteAt(contrast.x1, `${path}.treatment_contrast.x1`),
    };
    if (treatmentContrast.x0 === treatmentContrast.x1)
      fail(
        "interventional_causal_mediation_v1.treatment_contrast",
        `${path}.treatment_contrast`,
        "Treatment endpoints must differ.",
      );
  } else
    return fail(
      "interventional_causal_mediation_v1.treatment_contrast",
      `${path}.treatment_contrast.kind`,
      "Treatment contrast must be binary or continuous.",
    );
  const mediators = uniqueStableTextArray(item.mediators, `${path}.mediators`);
  if (!mediators.length)
    fail(
      "interventional_causal_mediation_v1.mediators",
      `${path}.mediators`,
      "Declare at least one observed mediator.",
    );
  const baselineModerators = uniqueStableTextArray(
    hasOwn(item, "baseline_moderators") ? item.baseline_moderators : [],
    `${path}.baseline_moderators`,
  );
  const adjustmentCovariates = uniqueStableTextArray(
    item.adjustment_covariates,
    `${path}.adjustment_covariates`,
  );
  if (!adjustmentCovariates.length)
    fail(
      "interventional_causal_mediation_v1.adjustment_set_missing",
      `${path}.adjustment_covariates`,
      "Declare a nonempty explicit adjustment set.",
    );
  const roles = [
    treatment,
    outcome,
    ...mediators,
    ...baselineModerators,
    ...adjustmentCovariates,
  ];
  if (new Set(roles).size !== roles.length)
    fail(
      "interventional_causal_mediation_v1.role_overlap",
      path,
      "Each observed variable must have exactly one causal role.",
    );
  const paths = arrayAt(item.paths, `${path}.paths`).map((candidate, index) => {
    const itemPath = `${path}.paths[${index}]`;
    const parsed = exactRecordAt(
      candidate,
      ["path_id", "ordered_variable_ids", "equations"],
      [],
      itemPath,
    );
    const ids = uniqueStableTextArray(
      parsed.ordered_variable_ids,
      `${itemPath}.ordered_variable_ids`,
    );
    if (
      ids.length < 3 ||
      ids.length > 5 ||
      ids[0] !== treatment ||
      ids.at(-1) !== outcome ||
      ids.slice(1, -1).some((id) => !mediators.includes(id))
    )
      fail(
        "interventional_causal_mediation_v1.path",
        itemPath,
        "Paths require 2 through 4 edges from treatment through declared mediators to outcome.",
      );
    const baseline = new Set([...baselineModerators, ...adjustmentCovariates]);
    const equations = arrayAt(parsed.equations, `${itemPath}.equations`).map(
      (entry, equationIndex) => {
        const equationPath = `${itemPath}.equations[${equationIndex}]`;
        const equation = exactRecordAt(
          entry,
          ["equation_id", "outcome_variable_id", "terms"],
          [],
          equationPath,
        );
        const terms = arrayAt(equation.terms, `${equationPath}.terms`).map(
          (termEntry, termIndex) => {
            const termPath = `${equationPath}.terms[${termIndex}]`;
            const term = exactRecordAt(
              termEntry,
              ["term_id", "factor_variable_ids"],
              [],
              termPath,
            );
            const factors = uniqueStableTextArray(
              term.factor_variable_ids,
              `${termPath}.factor_variable_ids`,
            );
            if (factors.length < 1 || factors.length > 3)
              fail(
                "interventional_causal_mediation_v1.equation_term",
                termPath,
                "Equation terms require one through three unique factors.",
              );
            return {
              term_id: textAt(term.term_id, `${termPath}.term_id`, true),
              factor_variable_ids: factors,
            };
          },
        );
        if (
          !terms.length ||
          new Set(terms.map((term) => term.term_id)).size !== terms.length
        )
          fail(
            "interventional_causal_mediation_v1.equation_terms",
            `${equationPath}.terms`,
            "Each equation requires uniquely identified terms.",
          );
        const allowed = new Set([
          ...baseline,
          ...ids.slice(0, equationIndex + 1),
        ]);
        const mains = new Set(
          terms
            .filter((term) => term.factor_variable_ids.length === 1)
            .map((term) => term.factor_variable_ids[0]),
        );
        const factorSets = new Set<string>();
        terms.forEach((term, termIndex) => {
          const key = [...term.factor_variable_ids].sort().join("\u0000");
          const postTreatmentCount = term.factor_variable_ids.filter((factor) =>
            mediators.includes(factor),
          ).length;
          if (
            factorSets.has(key) ||
            term.factor_variable_ids.some((factor) => !allowed.has(factor)) ||
            postTreatmentCount > 1 ||
            (term.factor_variable_ids.length > 1 &&
              term.factor_variable_ids.some((factor) => !mains.has(factor)))
          )
            fail(
              "interventional_causal_mediation_v1.equation_term",
              `${equationPath}.terms[${termIndex}]`,
              "Terms must be unique recursive products with strong-hierarchy main effects and at most one post-treatment mediator.",
            );
          factorSets.add(key);
        });
        if (
          !mains.has(ids[equationIndex]) ||
          adjustmentCovariates.some((covariate) => !mains.has(covariate))
        )
          fail(
            "interventional_causal_mediation_v1.required_main_effect",
            `${equationPath}.terms`,
            "Every equation requires its path predecessor and all adjustment covariates as main effects.",
          );
        return {
          equation_id: textAt(
            equation.equation_id,
            `${equationPath}.equation_id`,
            true,
          ),
          outcome_variable_id: textAt(
            equation.outcome_variable_id,
            `${equationPath}.outcome_variable_id`,
            true,
          ),
          terms,
        };
      },
    );
    if (
      equations.length !== ids.length - 1 ||
      equations.some(
        (equation, equationIndex) =>
          equation.outcome_variable_id !== ids[equationIndex + 1],
      ) ||
      new Set(equations.map((equation) => equation.equation_id)).size !==
        equations.length
    )
      fail(
        "interventional_causal_mediation_v1.equation_order",
        `${itemPath}.equations`,
        "Each path requires ordered mediator equations followed by its outcome equation.",
      );
    return {
      path_id: textAt(parsed.path_id, `${itemPath}.path_id`, true),
      ordered_variable_ids: ids,
      equations,
    };
  });
  if (
    !paths.length ||
    paths.length > 8 ||
    new Set(paths.map((candidate) => candidate.path_id)).size !== paths.length
  )
    fail(
      "interventional_causal_mediation_v1.path_count",
      `${path}.paths`,
      "Declare one through eight unique causal paths.",
    );
  const positivityItem = exactRecordAt(
    item.positivity_policy,
    [
      "minimum_binary_arm_count",
      "maximum_binary_arm_ratio",
      "positivity_strata_variable_ids",
      "minimum_count_per_binary_stratum_arm",
      "continuous_neighborhood_fraction_of_range",
      "minimum_continuous_neighborhood_count",
    ],
    [],
    `${path}.positivity_policy`,
  );
  const positivityPolicy: CausalPositivityPolicyV1 = {
    minimum_binary_arm_count: countAt(
      positivityItem.minimum_binary_arm_count,
      `${path}.positivity_policy.minimum_binary_arm_count`,
    ),
    maximum_binary_arm_ratio: finiteAt(
      positivityItem.maximum_binary_arm_ratio,
      `${path}.positivity_policy.maximum_binary_arm_ratio`,
    ),
    positivity_strata_variable_ids: uniqueStableTextArray(
      positivityItem.positivity_strata_variable_ids,
      `${path}.positivity_policy.positivity_strata_variable_ids`,
    ),
    minimum_count_per_binary_stratum_arm: countAt(
      positivityItem.minimum_count_per_binary_stratum_arm,
      `${path}.positivity_policy.minimum_count_per_binary_stratum_arm`,
    ),
    continuous_neighborhood_fraction_of_range: finiteAt(
      positivityItem.continuous_neighborhood_fraction_of_range,
      `${path}.positivity_policy.continuous_neighborhood_fraction_of_range`,
    ),
    minimum_continuous_neighborhood_count: countAt(
      positivityItem.minimum_continuous_neighborhood_count,
      `${path}.positivity_policy.minimum_continuous_neighborhood_count`,
    ),
  };
  const positivityBaseline = new Set([
    ...baselineModerators,
    ...adjustmentCovariates,
  ]);
  if (
    positivityPolicy.minimum_binary_arm_count === 0 ||
    positivityPolicy.maximum_binary_arm_ratio < 1 ||
    positivityPolicy.maximum_binary_arm_ratio > 10 ||
    positivityPolicy.positivity_strata_variable_ids.some(
      (variable) => !positivityBaseline.has(variable),
    ) ||
    positivityPolicy.minimum_count_per_binary_stratum_arm === 0 ||
    positivityPolicy.continuous_neighborhood_fraction_of_range <= 0 ||
    positivityPolicy.continuous_neighborhood_fraction_of_range > 0.5 ||
    positivityPolicy.minimum_continuous_neighborhood_count === 0
  )
    fail(
      "interventional_causal_mediation_v1.positivity_policy",
      `${path}.positivity_policy`,
      "The positivity screen requires positive bounded thresholds and baseline-only unique strata.",
    );
  const identificationFields = [
    "temporal_order_declared",
    "adjustment_set_justified",
    "consistency_assumption_acknowledged",
    "no_unmeasured_treatment_outcome_confounding_acknowledged",
    "no_unmeasured_treatment_mediator_confounding_acknowledged",
    "no_unmeasured_mediator_outcome_confounding_acknowledged",
    "no_exposure_induced_mediator_outcome_confounder_confirmed",
    "no_recanting_witness_confirmed",
    "linear_model_specification_reviewed",
    "positivity_reviewed",
  ] as const;
  const identificationItem = exactRecordAt(
    item.identification,
    identificationFields,
    [],
    `${path}.identification`,
  );
  const identification = Object.fromEntries(
    identificationFields.map((field) => [
      field,
      booleanAt(identificationItem[field], `${path}.identification.${field}`),
    ]),
  ) as unknown as CausalIdentificationChecklistV1;
  if (!Object.values(identification).every(Boolean))
    fail(
      "interventional_causal_mediation_v1.identification",
      `${path}.identification`,
      "All identification declarations require explicit acknowledgement.",
    );
  const bootstrapResamples = countAt(
    item.bootstrap_resamples,
    `${path}.bootstrap_resamples`,
    10_000,
  );
  if (bootstrapResamples < 500)
    fail(
      "interventional_causal_mediation_v1.bootstrap_resamples",
      `${path}.bootstrap_resamples`,
      "Bootstrap resamples must be between 500 and 10,000.",
    );
  return {
    schema_version: 1,
    treatment,
    treatment_contrast: treatmentContrast,
    outcome,
    mediators,
    baseline_moderators: baselineModerators,
    adjustment_covariates: adjustmentCovariates,
    paths,
    positivity_policy: positivityPolicy,
    identification,
    bootstrap_resamples: bootstrapResamples,
    seed: countAt(item.seed, `${path}.seed`),
    confidence_level: probabilityAt(
      item.confidence_level,
      `${path}.confidence_level`,
    ),
  };
}

export type MultimodQualificationStateV1 =
  "unqualified_labs" | "release_qualified_candidate" | "failed_closed";

export interface MultimodCandidateQualificationReceiptV1 {
  schema_version: 1;
  authority_binding_sha256: string;
  candidate_commit_sha: string;
  candidate_version: string;
  qualification_plan_sha256: string;
  gate_binding_sha256: string;
  capability_index_sha256: string;
  prepackage_manifest_set_sha256: string;
  required_profile_cells: string[];
}

export interface MultimodProvenanceV1 {
  method_version: string;
  recipe_id: string;
  recipe_analytical_sha256: string;
  config_sha256: string;
  model_id: string;
  model_scientific_sha256: string;
  dataset_id: string;
  dataset_fingerprint: string;
  engine_version: string;
  seed: number;
  capability_cell: CapabilityCellReferenceV2;
  qualification: MultimodQualificationStateV1;
  candidate_qualification_receipt?: MultimodCandidateQualificationReceiptV1;
}

export interface MultimodResultSidecarDescriptorV1 {
  schema_version: typeof MULTIMOD_RESULT_SIDECAR_DESCRIPTOR_V1_SCHEMA_VERSION;
  entry_name: string;
  evidence_role: string;
  arrow_schema_contract_id: string;
  arrow_schema_contract_version: 1;
  media_type: "application/vnd.apache.arrow.stream";
  compression: "zip_deflate";
  arrow_schema_sha256: string;
  row_count: number;
  column_count: number;
  uncompressed_bytes: number;
  sha256: string;
  identity_sha256: string;
  required_for_scientific_reopen: true;
}

export type MultimodReplicateFailureKindV1 =
  | "cancelled"
  | "empty_group"
  | "insufficient_cases"
  | "rank_deficient"
  | "constant_score"
  | "constant_product"
  | "nonfinite_estimate"
  | "estimator_did_not_converge"
  | "class_collapsed"
  | "variance_collapsed"
  | "unstable_multistart"
  | "ambiguous_label_alignment"
  | "comparability_failed"
  | "target_inventory_mismatch"
  | "inner_standard_error_unavailable"
  | "other";

export interface MultimodReplicateFailureV1 {
  replicate_index: number;
  kind: MultimodReplicateFailureKindV1;
  stable_code: string;
  detail: string;
}

export interface MultimodReplicateLedgerSummaryV1 {
  requested: number;
  usable: number;
  minimum_required: number;
  usable_fraction: number;
  complete: boolean;
  ledger_sha256: string;
  failure_counts: Record<string, number>;
  failures: MultimodReplicateFailureV1[];
}

export interface MultimodIntervalV1 {
  confidence_level: number;
  lower?: number;
  upper?: number;
  family: string;
  alternative: InferenceAlternativeV1;
}

export interface MultimodParameterEstimateV1 {
  target_id: string;
  target_kind: string;
  estimate: number;
  standard_error?: number;
  p_value?: number;
  interval?: MultimodIntervalV1;
}

export interface MgaGroupEligibilityV1 {
  group_id: string;
  label: string;
  complete_cases: number;
  selected_rows: number;
  eligible: boolean;
  warnings: string[];
  blockers: string[];
}

export interface MgaGroupParameterV1 {
  group_id: string;
  parameter: MultimodParameterEstimateV1;
}
export type MicomInvarianceInterpretationV1 = "composite_invariance";
export interface MicomPairResultV1 {
  left_group_id: string;
  right_group_id: string;
  construct_id: string;
  interpretation: MicomInvarianceInterpretationV1;
  configural_invariance_confirmed: boolean;
  compositional_correlation: number;
  compositional_lower_quantile: number;
  compositional_p_value: number;
  compositional_invariance: boolean;
  partial_invariance: boolean;
  equal_mean_p_value: number;
  equal_variance_p_value: number;
}
export interface MgaPairwiseComparisonV1 {
  procedure: string;
  left_group_id: string;
  right_group_id: string;
  target_id: string;
  difference_left_minus_right: number;
  raw_p_value?: number;
  adjusted_p_value?: number;
  directional_probability?: number;
  interval?: MultimodIntervalV1;
  measurement_comparability_satisfied: boolean;
  interpretation_blocked: boolean;
}
export interface MgaOmnibusComparisonV1 {
  procedure: string;
  target_id: string;
  statistic: number;
  degrees_of_freedom: number;
  p_value: number;
}
export type ExcludedRowReasonV1 =
  | "unselected_group_value"
  | "missing_group_value"
  | "missing_model_value"
  | "invalid_weight"
  | "nonfinite_value";
export interface ExcludedRowReceiptV1 {
  stable_row_token: string;
  typed_group_value: string;
  reason: ExcludedRowReasonV1;
}

export interface PlsMultigroupAnalysisV1 {
  schema_version: 1;
  provenance: MultimodProvenanceV1;
  profile: MgaModelProfileV1;
  group_eligibility: MgaGroupEligibilityV1[];
  group_parameters: MgaGroupParameterV1[];
  micom_pairs: MicomPairResultV1[];
  omnibus: MgaOmnibusComparisonV1[];
  pairwise: MgaPairwiseComparisonV1[];
  multiplicity: MultiplicityAdjustmentV1;
  replicate_ledgers: MultimodReplicateLedgerSummaryV1[];
  excluded_rows: ExcludedRowReceiptV1[];
  sidecars: MultimodResultSidecarDescriptorV1[];
}

export type HeterogeneityCandidateStateV2 =
  "eligible" | "converged_stable" | "ineligible" | "failed" | "unstable";
export type HeterogeneityCandidateMethodV2 =
  | { kind: "pooled_baseline_v1" }
  | { kind: "segmentation"; algorithm: HeterogeneityAlgorithmV2 };
export interface HeterogeneityCandidateV2 {
  method: HeterogeneityCandidateMethodV2;
  k: number;
  state: HeterogeneityCandidateStateV2;
  converged_starts: number;
  stable_starts: number;
  log_likelihood?: number;
  objective?: number;
  criteria: Record<string, number>;
  class_or_segment_shares: number[];
  pooled_parameters: MultimodParameterEstimateV1[];
  blockers: string[];
}
export interface HeterogeneityClassParameterV2 {
  class_id: number;
  parameter: MultimodParameterEstimateV1;
  metric: string;
}
export interface HeterogeneityClassContrastV2 {
  left_class_id: number;
  right_class_id: number;
  target_id: string;
  difference: number;
  p_value?: number;
  interval?: MultimodIntervalV1;
  common_metric_comparability_satisfied: boolean;
  inferential_interpretation_blocked: boolean;
}
export interface PlsHeterogeneityAnalysisV2 {
  schema_version: 2;
  provenance: MultimodProvenanceV1;
  profile: HeterogeneityInteractionProfileV2;
  candidates: HeterogeneityCandidateV2[];
  discovery_result_identity_sha256: string;
  inference_lock?: HeterogeneityInferenceLockReceiptV2;
  locked_algorithm?: HeterogeneityAlgorithmV2;
  locked_k?: number;
  parameters: HeterogeneityClassParameterV2[];
  contrasts: HeterogeneityClassContrastV2[];
  bootstrap_ledger?: MultimodReplicateLedgerSummaryV1;
  sidecars: MultimodResultSidecarDescriptorV1[];
  descriptive_only: boolean;
}

export type ConditionalProcessTargetKindV2 =
  | "conditional_specific_indirect"
  | "conditional_total_indirect"
  | "conditional_total_effect"
  | "scalar_index_of_moderated_mediation"
  | "local_first_derivative"
  | "local_second_derivative"
  | "local_cross_derivative"
  | "probe_contrast"
  | "group_contrast";
export interface ConditionalProcessTargetResultV2 {
  target_id: string;
  kind: ConditionalProcessTargetKindV2;
  path_id: string;
  group_id?: string;
  probe_values: Record<string, number>;
  derivative_variables: string[];
  estimate: number;
  p_value?: number;
  interval?: MultimodIntervalV1;
  usable_replicates: number;
}
export interface GeneralSemConditionalProcessResultV2 {
  schema_version: 2;
  provenance: MultimodProvenanceV1;
  profile_id: string;
  targets: ConditionalProcessTargetResultV2[];
  replicate_ledger: MultimodReplicateLedgerSummaryV1;
  sidecars: MultimodResultSidecarDescriptorV1[];
  warnings: string[];
}

export interface CausalPositivityDiagnosticV1 {
  variable_id: string;
  observed_minimum: number;
  observed_maximum: number;
  requested_value: number;
  support_count: number;
  minimum_required_count: number;
  support_rule: string;
  supported: boolean;
}
export interface InterventionalEffectResultV1 {
  target_id: string;
  path_id: string;
  estimand: string;
  estimate: number;
  p_value?: number;
  interval?: MultimodIntervalV1;
}
export interface InterventionalMediationResultV1 {
  schema_version: 1;
  provenance: MultimodProvenanceV1;
  interpretation_label: string;
  identification_assumptions: string[];
  positivity: CausalPositivityDiagnosticV1[];
  effects: InterventionalEffectResultV1[];
  replicate_ledger: MultimodReplicateLedgerSummaryV1;
  sidecars: MultimodResultSidecarDescriptorV1[];
}

export type MultiModAnalysisResultV1 =
  | { kind: "pls_multigroup_analysis_v1"; analysis: PlsMultigroupAnalysisV1 }
  | {
      kind: "pls_heterogeneity_analysis_v2";
      analysis: PlsHeterogeneityAnalysisV2;
    }
  | {
      kind: "general_sem_conditional_process_result_v2";
      analysis: GeneralSemConditionalProcessResultV2;
    }
  | {
      kind: "interventional_mediation_result_v1";
      analysis: InterventionalMediationResultV1;
    };

export interface MultiModResultAttachmentV1 {
  schema_version: typeof MULTIMOD_RESULT_ATTACHMENT_SCHEMA_VERSION_V1;
  result_id: string;
  recipe_id: string;
  result: MultiModAnalysisResultV1;
  result_sha256: string;
  identity_sha256: string;
  sidecars: MultimodResultSidecarDescriptorV1[];
}

const SHA256 = /^[a-f0-9]{64}$/u;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/iu;

function shaAt(value: unknown, path: string): string {
  const text = textAt(value, path);
  if (!SHA256.test(text))
    fail("multimod.value.sha256", path, `${path} must be a lowercase SHA-256.`);
  return text;
}

function uuidAt(value: unknown, path: string): string {
  const text = textAt(value, path);
  if (!UUID.test(text))
    fail("multimod.value.uuid", path, `${path} must be a UUID.`);
  return text;
}

function textArrayAt(value: unknown, path: string): string[] {
  return arrayAt(value, path).map((entry, index) =>
    textAt(entry, `${path}[${index}]`),
  );
}

function countMapAt(value: unknown, path: string): Record<string, number> {
  return Object.fromEntries(
    Object.entries(recordAt(value, path)).map(([key, entry]) => [
      textAt(key, `${path}.key`, true),
      countAt(entry, `${path}.${key}`),
    ]),
  );
}

function optionalFinite(
  record: UnknownRecord,
  field: string,
  path: string,
): number | undefined {
  return record[field] == null
    ? undefined
    : finiteAt(record[field], `${path}.${field}`);
}

function optionalProbability(
  record: UnknownRecord,
  field: string,
  path: string,
): number | undefined {
  return record[field] == null
    ? undefined
    : unitProbabilityAt(record[field], `${path}.${field}`);
}

function parseCapabilityCell(
  value: unknown,
  path: string,
): CapabilityCellReferenceV2 {
  const item = exactRecordAt(
    value,
    [
      "registry_schema_version",
      "capability_id",
      "cell_id",
      "capability_version",
    ],
    [],
    path,
  );
  if (
    countAt(item.registry_schema_version, `${path}.registry_schema_version`) !==
    2
  )
    fail(
      "multimod.capability_cell.schema",
      `${path}.registry_schema_version`,
      "Capability Registry schema must equal 2.",
    );
  return {
    registry_schema_version: 2,
    capability_id: textAt(item.capability_id, `${path}.capability_id`, true),
    cell_id: textAt(item.cell_id, `${path}.cell_id`, true),
    capability_version: textAt(
      item.capability_version,
      `${path}.capability_version`,
      true,
    ),
  };
}

export function parseMultimodCandidateQualificationReceiptV1(
  value: unknown,
  path = "multimod.candidate_qualification_receipt",
): MultimodCandidateQualificationReceiptV1 {
  const receipt = exactRecordAt(
    value,
    [
      "schema_version",
      "authority_binding_sha256",
      "candidate_commit_sha",
      "candidate_version",
      "qualification_plan_sha256",
      "gate_binding_sha256",
      "capability_index_sha256",
      "prepackage_manifest_set_sha256",
      "required_profile_cells",
    ],
    [],
    path,
  );
  if (countAt(receipt.schema_version, `${path}.schema_version`) !== 1) {
    fail("multimod.candidate_receipt.schema", `${path}.schema_version`, "Candidate receipt schema must equal 1.");
  }
  const candidateCommit = textAt(receipt.candidate_commit_sha, `${path}.candidate_commit_sha`, true);
  if (!/^[a-f0-9]{40}$/u.test(candidateCommit)) {
    fail("multimod.candidate_receipt.commit", `${path}.candidate_commit_sha`, "Candidate commit must be one lowercase Git SHA.");
  }
  const requiredCells = arrayAt(receipt.required_profile_cells, `${path}.required_profile_cells`)
    .map((cell, index) => textAt(cell, `${path}.required_profile_cells[${index}]`, true));
  const exactCells = requiredCells.length > 0
    && requiredCells.every((cell) => /^[a-z0-9][a-z0-9._-]*::[a-z0-9][a-z0-9._-]*$/u.test(cell))
    && requiredCells.every((cell, index) => index === 0 || requiredCells[index - 1]! < cell);
  if (!exactCells) {
    fail("multimod.candidate_receipt.cells", `${path}.required_profile_cells`, "Candidate cells must be nonempty, exact, sorted, unique, and wildcard-free.");
  }
  return {
    schema_version: 1,
    authority_binding_sha256: shaAt(receipt.authority_binding_sha256, `${path}.authority_binding_sha256`),
    candidate_commit_sha: candidateCommit,
    candidate_version: textAt(receipt.candidate_version, `${path}.candidate_version`, true),
    qualification_plan_sha256: shaAt(receipt.qualification_plan_sha256, `${path}.qualification_plan_sha256`),
    gate_binding_sha256: shaAt(receipt.gate_binding_sha256, `${path}.gate_binding_sha256`),
    capability_index_sha256: shaAt(receipt.capability_index_sha256, `${path}.capability_index_sha256`),
    prepackage_manifest_set_sha256: shaAt(receipt.prepackage_manifest_set_sha256, `${path}.prepackage_manifest_set_sha256`),
    required_profile_cells: requiredCells,
  };
}

function parseProvenance(value: unknown, path: string): MultimodProvenanceV1 {
  const item = exactRecordAt(
    value,
    [
      "method_version",
      "recipe_id",
      "recipe_analytical_sha256",
      "config_sha256",
      "model_id",
      "model_scientific_sha256",
      "dataset_id",
      "dataset_fingerprint",
      "engine_version",
      "seed",
      "capability_cell",
      "qualification",
    ],
    ["candidate_qualification_receipt"],
    path,
  );
  const qualification = enumAt(
    item.qualification,
    [
      "unqualified_labs",
      "release_qualified_candidate",
      "failed_closed",
    ] as const,
    `${path}.qualification`,
  );
  let candidateReceipt: MultimodCandidateQualificationReceiptV1 | undefined;
  if (hasOwn(item, "candidate_qualification_receipt") && item.candidate_qualification_receipt != null) {
    candidateReceipt = parseMultimodCandidateQualificationReceiptV1(
      item.candidate_qualification_receipt,
      `${path}.candidate_qualification_receipt`,
    );
  }
  if ((qualification === "release_qualified_candidate") !== (candidateReceipt !== undefined)) {
    fail(
      "multimod.candidate_receipt.state_coupling",
      `${path}.candidate_qualification_receipt`,
      "Only release-qualified-candidate provenance must carry exactly one candidate authority receipt.",
    );
  }
  return {
    method_version: textAt(item.method_version, `${path}.method_version`, true),
    recipe_id: textAt(item.recipe_id, `${path}.recipe_id`, true),
    recipe_analytical_sha256: shaAt(
      item.recipe_analytical_sha256,
      `${path}.recipe_analytical_sha256`,
    ),
    config_sha256: shaAt(item.config_sha256, `${path}.config_sha256`),
    model_id: textAt(item.model_id, `${path}.model_id`, true),
    model_scientific_sha256: shaAt(
      item.model_scientific_sha256,
      `${path}.model_scientific_sha256`,
    ),
    dataset_id: textAt(item.dataset_id, `${path}.dataset_id`, true),
    dataset_fingerprint: textAt(
      item.dataset_fingerprint,
      `${path}.dataset_fingerprint`,
      true,
    ),
    engine_version: textAt(item.engine_version, `${path}.engine_version`, true),
    seed: countAt(item.seed, `${path}.seed`),
    capability_cell: parseCapabilityCell(
      item.capability_cell,
      `${path}.capability_cell`,
    ),
    qualification,
    ...(candidateReceipt !== undefined
      ? { candidate_qualification_receipt: candidateReceipt }
      : {}),
  };
}

export function parseMultimodResultSidecarDescriptorV1(
  value: unknown,
  resultId: string,
  path: string,
): MultimodResultSidecarDescriptorV1 {
  const item = exactRecordAt(
    value,
    [
      "schema_version",
      "entry_name",
      "evidence_role",
      "arrow_schema_contract_id",
      "arrow_schema_contract_version",
      "media_type",
      "compression",
      "arrow_schema_sha256",
      "row_count",
      "column_count",
      "uncompressed_bytes",
      "sha256",
      "identity_sha256",
      "required_for_scientific_reopen",
    ],
    [],
    path,
  );
  if (countAt(item.schema_version, `${path}.schema_version`) !== 1)
    fail(
      "multimod.sidecar.schema",
      `${path}.schema_version`,
      "Sidecar descriptor schema must equal 1.",
    );
  const entryName = textAt(item.entry_name, `${path}.entry_name`);
  if (
    !entryName.startsWith(`results/${resultId}/`) ||
    !entryName.endsWith(".arrow") ||
    entryName.includes("..") ||
    entryName.includes("\\") ||
    entryName.startsWith("/")
  )
    fail(
      "multimod.sidecar.entry",
      `${path}.entry_name`,
      "Sidecar entry identity is invalid.",
    );
  const mediaType = enumAt(
    item.media_type,
    ["application/vnd.apache.arrow.stream"] as const,
    `${path}.media_type`,
  );
  const compression = enumAt(
    item.compression,
    ["zip_deflate"] as const,
    `${path}.compression`,
  );
  const evidenceRole = textAt(item.evidence_role, `${path}.evidence_role`);
  const roleMatch = /^([a-z0-9_-]+):([a-z0-9_-]+)$/u.exec(evidenceRole);
  const leafStem = entryName.slice(`results/${resultId}/`.length, -".arrow".length);
  if (!roleMatch
    || (!(leafStem.startsWith(`${roleMatch[1]}-`) || leafStem.includes(`-${roleMatch[1]}-`)))
    || !leafStem.endsWith(`-${roleMatch[2]}`)) {
    fail(
      "multimod.sidecar.evidence_role",
      `${path}.evidence_role`,
      "Sidecar evidence role must be a stable kind:table identity bound to its archive leaf.",
    );
  }
  const schemaContractId = textAt(
    item.arrow_schema_contract_id,
    `${path}.arrow_schema_contract_id`,
  );
  const contractPrefix = `qpls.multimod.arrow.${evidenceRole}.v1.`;
  const contractSchemaSha256 = schemaContractId.startsWith(contractPrefix)
    ? schemaContractId.slice(contractPrefix.length)
    : "";
  if (!SHA256.test(contractSchemaSha256)
    || countAt(item.arrow_schema_contract_version, `${path}.arrow_schema_contract_version`) !== 1) {
    fail(
      "multimod.sidecar.schema_contract",
      path,
      "Sidecar schema-contract identity or version is invalid.",
    );
  }
  const rowCount = countAt(item.row_count, `${path}.row_count`);
  const columnCount = countAt(item.column_count, `${path}.column_count`);
  const uncompressedBytes = countAt(
    item.uncompressed_bytes,
    `${path}.uncompressed_bytes`,
  );
  if (
    !rowCount ||
    !columnCount ||
    !uncompressedBytes ||
    uncompressedBytes > MULTIMOD_SIDECAR_MAX_BYTES_V1
  )
    fail(
      "multimod.sidecar.bounds",
      path,
      "Sidecar shape or size exceeds the scientific archive contract.",
    );
  if (item.required_for_scientific_reopen !== true)
    fail(
      "multimod.sidecar.required",
      `${path}.required_for_scientific_reopen`,
      "Every declared sidecar is required for scientific reopen.",
    );
  return {
    schema_version: 1,
    entry_name: entryName,
    evidence_role: evidenceRole,
    arrow_schema_contract_id: schemaContractId,
    arrow_schema_contract_version: 1,
    media_type: mediaType,
    compression,
    arrow_schema_sha256: shaAt(
      item.arrow_schema_sha256,
      `${path}.arrow_schema_sha256`,
    ),
    row_count: rowCount,
    column_count: columnCount,
    uncompressed_bytes: uncompressedBytes,
    sha256: shaAt(item.sha256, `${path}.sha256`),
    identity_sha256: shaAt(item.identity_sha256, `${path}.identity_sha256`),
    required_for_scientific_reopen: true,
  };
}

function parseSidecars(
  value: unknown,
  resultId: string,
  path: string,
): MultimodResultSidecarDescriptorV1[] {
  const sidecars = arrayAt(value, path).map((entry, index) =>
    parseMultimodResultSidecarDescriptorV1(
      entry,
      resultId,
      `${path}[${index}]`,
    ),
  );
  if (
    new Set(sidecars.map((entry) => entry.entry_name)).size !== sidecars.length
  )
    fail("multimod.sidecar.duplicate", path, "Sidecar entries must be unique.");
  const totalBytes = sidecars.reduce(
    (total, sidecar) => total + sidecar.uncompressed_bytes,
    0,
  );
  if (
    !Number.isSafeInteger(totalBytes) ||
    totalBytes > MULTIMOD_SIDECAR_MAX_BYTES_V1
  )
    fail(
      "multimod.sidecar.total",
      path,
      "Aggregate sidecar evidence exceeds the 512 MiB per-run cap.",
    );
  return sidecars;
}

export function parseMultimodIntervalV1(
  value: unknown,
  path = "interval",
): MultimodIntervalV1 {
  const item = exactRecordAt(
    value,
    ["confidence_level", "family", "alternative"],
    ["lower", "upper"],
    path,
  );
  const alternative = enumAt(
    item.alternative,
    ["two_sided", "less", "greater"] as const,
    `${path}.alternative`,
  );
  const lower =
    item.lower == null ? undefined : finiteAt(item.lower, `${path}.lower`);
  const upper =
    item.upper == null ? undefined : finiteAt(item.upper, `${path}.upper`);
  const endpointsValid =
    alternative === "two_sided"
      ? lower !== undefined && upper !== undefined && lower <= upper
      : alternative === "less"
        ? lower === undefined && upper !== undefined
        : lower !== undefined && upper === undefined;
  if (!endpointsValid)
    fail(
      "multimod_result.interval",
      path,
      "Interval endpoints do not match the declared alternative.",
    );
  return {
    confidence_level: probabilityAt(
      item.confidence_level,
      `${path}.confidence_level`,
    ),
    ...(lower === undefined ? {} : { lower }),
    ...(upper === undefined ? {} : { upper }),
    family: textAt(item.family, `${path}.family`, true),
    alternative,
  };
}

function parseEstimate(
  value: unknown,
  path: string,
): MultimodParameterEstimateV1 {
  const item = exactRecordAt(
    value,
    ["target_id", "target_kind", "estimate"],
    ["standard_error", "p_value", "interval"],
    path,
  );
  const interval =
    item.interval == null
      ? undefined
      : parseMultimodIntervalV1(item.interval, `${path}.interval`);
  const standardError = optionalFinite(item, "standard_error", path);
  const pValue = optionalProbability(item, "p_value", path);
  if (standardError !== undefined && standardError < 0) {
    fail(
      "multimod_result.parameter",
      `${path}.standard_error`,
      "A standard error cannot be negative.",
    );
  }
  return {
    target_id: textAt(item.target_id, `${path}.target_id`, true),
    target_kind: textAt(item.target_kind, `${path}.target_kind`, true),
    estimate: finiteAt(item.estimate, `${path}.estimate`),
    ...(standardError === undefined ? {} : { standard_error: standardError }),
    ...(pValue === undefined ? {} : { p_value: pValue }),
    ...(interval ? { interval } : {}),
  };
}

function parseLedger(
  value: unknown,
  path: string,
): MultimodReplicateLedgerSummaryV1 {
  const item = exactRecordAt(
    value,
    [
      "requested",
      "usable",
      "minimum_required",
      "usable_fraction",
      "complete",
      "ledger_sha256",
      "failure_counts",
    ],
    ["failures"],
    path,
  );
  const requested = countAt(item.requested, `${path}.requested`);
  const usable = countAt(item.usable, `${path}.usable`);
  const minimumRequired = countAt(
    item.minimum_required,
    `${path}.minimum_required`,
  );
  const usableFraction = finiteAt(
    item.usable_fraction,
    `${path}.usable_fraction`,
  );
  const expectedFraction = requested === 0 ? 0 : usable / requested;
  if (
    requested === 0 ||
    usable > requested ||
    minimumRequired === 0 ||
    minimumRequired > requested ||
    usableFraction < 0 ||
    usableFraction > 1 ||
    Math.abs(usableFraction - expectedFraction) > 1e-12
  ) {
    fail(
      "multimod_result.ledger",
      path,
      "Replicate-ledger counts or usable fraction are inconsistent.",
    );
  }
  const failures = arrayAt(
    hasOwn(item, "failures") ? item.failures : [],
    `${path}.failures`,
  ).map((entry, index) => {
    const failurePath = `${path}.failures[${index}]`;
    const failure = exactRecordAt(
      entry,
      ["replicate_index", "kind", "stable_code", "detail"],
      [],
      failurePath,
    );
    const replicateIndex = countAt(
      failure.replicate_index,
      `${failurePath}.replicate_index`,
    );
    const detail = textAt(failure.detail, `${failurePath}.detail`);
    if (replicateIndex >= requested || detail.trim() !== detail) {
      fail(
        "multimod_result.ledger",
        failurePath,
        "Replicate failure identity is inconsistent with the ledger.",
      );
    }
    return {
      replicate_index: replicateIndex,
      kind: enumAt(
        failure.kind,
        [
          "cancelled",
          "empty_group",
          "insufficient_cases",
          "rank_deficient",
          "constant_score",
          "constant_product",
          "nonfinite_estimate",
          "estimator_did_not_converge",
          "class_collapsed",
          "variance_collapsed",
          "unstable_multistart",
          "ambiguous_label_alignment",
          "comparability_failed",
          "target_inventory_mismatch",
          "inner_standard_error_unavailable",
          "other",
        ] as const,
        `${failurePath}.kind`,
      ),
      stable_code: textAt(
        failure.stable_code,
        `${failurePath}.stable_code`,
        true,
      ),
      detail,
    };
  });
  const failureCounts = countMapAt(
    item.failure_counts,
    `${path}.failure_counts`,
  );
  const failed = requested - usable;
  const inventoriedFailures = Object.values(failureCounts).reduce(
    (total, count) => total + count,
    0,
  );
  const complete = booleanAt(item.complete, `${path}.complete`);
  if (
    inventoriedFailures !== failed ||
    complete !== usable >= minimumRequired
  ) {
    fail(
      "multimod_result.ledger",
      path,
      "Replicate completion or failure inventory is inconsistent.",
    );
  }
  return {
    requested,
    usable,
    minimum_required: minimumRequired,
    usable_fraction: usableFraction,
    complete,
    ledger_sha256: shaAt(item.ledger_sha256, `${path}.ledger_sha256`),
    failure_counts: failureCounts,
    failures,
  };
}

function parseMgaResult(
  value: unknown,
  resultId: string,
  path: string,
): PlsMultigroupAnalysisV1 {
  const item = exactRecordAt(
    value,
    [
      "schema_version",
      "provenance",
      "profile",
      "group_eligibility",
      "group_parameters",
      "micom_pairs",
      "omnibus",
      "pairwise",
      "multiplicity",
      "replicate_ledgers",
      "excluded_rows",
    ],
    ["sidecars"],
    path,
  );
  if (countAt(item.schema_version, `${path}.schema_version`) !== 1)
    fail(
      "multimod.result.mga_schema",
      `${path}.schema_version`,
      "PLS multigroup result schema must equal 1.",
    );
  const groupEligibility = arrayAt(
    item.group_eligibility,
    `${path}.group_eligibility`,
  ).map((entry, index) => {
    const entryPath = `${path}.group_eligibility[${index}]`;
    const row = exactRecordAt(
      entry,
      ["group_id", "label", "complete_cases", "selected_rows", "eligible"],
      ["warnings", "blockers"],
      entryPath,
    );
    return {
      group_id: textAt(row.group_id, `${entryPath}.group_id`, true),
      label: textAt(row.label, `${entryPath}.label`, true),
      complete_cases: countAt(
        row.complete_cases,
        `${entryPath}.complete_cases`,
      ),
      selected_rows: countAt(row.selected_rows, `${entryPath}.selected_rows`),
      eligible: booleanAt(row.eligible, `${entryPath}.eligible`),
      warnings: textArrayAt(
        hasOwn(row, "warnings") ? row.warnings : [],
        `${entryPath}.warnings`,
      ),
      blockers: textArrayAt(
        hasOwn(row, "blockers") ? row.blockers : [],
        `${entryPath}.blockers`,
      ),
    };
  });
  if (
    groupEligibility.length < 2 ||
    groupEligibility.length > 20 ||
    new Set(groupEligibility.map((group) => group.group_id)).size !==
      groupEligibility.length ||
    groupEligibility.some(
      (group) =>
        group.complete_cases > group.selected_rows ||
        group.eligible !== (group.blockers.length === 0),
    )
  ) {
    fail(
      "multimod_result.mga_eligibility",
      `${path}.group_eligibility`,
      "MGA group identities, counts, or blocker state are inconsistent.",
    );
  }
  const groupParameters = arrayAt(
    item.group_parameters,
    `${path}.group_parameters`,
  ).map((entry, index) => {
    const entryPath = `${path}.group_parameters[${index}]`;
    const row = exactRecordAt(entry, ["group_id", "parameter"], [], entryPath);
    return {
      group_id: textAt(row.group_id, `${entryPath}.group_id`, true),
      parameter: parseEstimate(row.parameter, `${entryPath}.parameter`),
    };
  });
  if (
    new Set(
      groupParameters.map(
        (row) => `${row.group_id}\u0000${row.parameter.target_id}`,
      ),
    ).size !== groupParameters.length
  ) {
    fail(
      "multimod_result.mga_parameter_duplicate",
      `${path}.group_parameters`,
      "Group-parameter identities must be unique.",
    );
  }
  const micomPairs = arrayAt(item.micom_pairs, `${path}.micom_pairs`).map(
    (entry, index) => {
      const entryPath = `${path}.micom_pairs[${index}]`;
      const row = exactRecordAt(
        entry,
        [
          "left_group_id",
          "right_group_id",
          "construct_id",
          "interpretation",
          "configural_invariance_confirmed",
          "compositional_correlation",
          "compositional_lower_quantile",
          "compositional_p_value",
          "compositional_invariance",
          "partial_invariance",
          "equal_mean_p_value",
          "equal_variance_p_value",
        ],
        [],
        entryPath,
      );
      const parsed = {
        left_group_id: textAt(
          row.left_group_id,
          `${entryPath}.left_group_id`,
          true,
        ),
        right_group_id: textAt(
          row.right_group_id,
          `${entryPath}.right_group_id`,
          true,
        ),
        construct_id: textAt(
          row.construct_id,
          `${entryPath}.construct_id`,
          true,
        ),
        interpretation: enumAt(
          row.interpretation,
          ["composite_invariance"] as const,
          `${entryPath}.interpretation`,
        ),
        configural_invariance_confirmed: booleanAt(
          row.configural_invariance_confirmed,
          `${entryPath}.configural_invariance_confirmed`,
        ),
        compositional_correlation: finiteAt(
          row.compositional_correlation,
          `${entryPath}.compositional_correlation`,
        ),
        compositional_lower_quantile: finiteAt(
          row.compositional_lower_quantile,
          `${entryPath}.compositional_lower_quantile`,
        ),
        compositional_p_value: unitProbabilityAt(
          row.compositional_p_value,
          `${entryPath}.compositional_p_value`,
        ),
        compositional_invariance: booleanAt(
          row.compositional_invariance,
          `${entryPath}.compositional_invariance`,
        ),
        partial_invariance: booleanAt(
          row.partial_invariance,
          `${entryPath}.partial_invariance`,
        ),
        equal_mean_p_value: unitProbabilityAt(
          row.equal_mean_p_value,
          `${entryPath}.equal_mean_p_value`,
        ),
        equal_variance_p_value: unitProbabilityAt(
          row.equal_variance_p_value,
          `${entryPath}.equal_variance_p_value`,
        ),
      };
      if (
        parsed.left_group_id === parsed.right_group_id ||
        parsed.compositional_correlation < -1 ||
        parsed.compositional_correlation > 1 ||
        parsed.compositional_lower_quantile < -1 ||
        parsed.compositional_lower_quantile > 1 ||
        parsed.compositional_invariance !==
          mgaGreaterOrTiedV1(
            parsed.compositional_correlation,
            parsed.compositional_lower_quantile,
          ) ||
        parsed.partial_invariance !==
          (parsed.configural_invariance_confirmed &&
            parsed.compositional_invariance)
      ) {
        fail(
          "multimod_result.micom_pair",
          entryPath,
          "MICOM pair identity, probability, or partial-invariance state is invalid.",
        );
      }
      return parsed;
    },
  );
  const omnibus = arrayAt(item.omnibus, `${path}.omnibus`).map(
    (entry, index) => {
      const entryPath = `${path}.omnibus[${index}]`;
      const row = exactRecordAt(
        entry,
        [
          "procedure",
          "target_id",
          "statistic",
          "degrees_of_freedom",
          "p_value",
        ],
        [],
        entryPath,
      );
      const degreesOfFreedom = countAt(
        row.degrees_of_freedom,
        `${entryPath}.degrees_of_freedom`,
      );
      if (degreesOfFreedom === 0)
        fail(
          "multimod_result.mga_omnibus",
          entryPath,
          "Omnibus degrees of freedom must be positive.",
        );
      return {
        procedure: textAt(row.procedure, `${entryPath}.procedure`, true),
        target_id: textAt(row.target_id, `${entryPath}.target_id`, true),
        statistic: finiteAt(row.statistic, `${entryPath}.statistic`),
        degrees_of_freedom: degreesOfFreedom,
        p_value: unitProbabilityAt(row.p_value, `${entryPath}.p_value`),
      };
    },
  );
  const pairwise = arrayAt(item.pairwise, `${path}.pairwise`).map(
    (entry, index) => {
      const entryPath = `${path}.pairwise[${index}]`;
      const row = exactRecordAt(
        entry,
        [
          "procedure",
          "left_group_id",
          "right_group_id",
          "target_id",
          "difference_left_minus_right",
          "measurement_comparability_satisfied",
          "interpretation_blocked",
        ],
        [
          "raw_p_value",
          "adjusted_p_value",
          "directional_probability",
          "interval",
        ],
        entryPath,
      );
      const rawP = optionalProbability(row, "raw_p_value", entryPath);
      const adjustedP = optionalProbability(row, "adjusted_p_value", entryPath);
      const directional = optionalProbability(
        row,
        "directional_probability",
        entryPath,
      );
      const leftGroupId = textAt(
        row.left_group_id,
        `${entryPath}.left_group_id`,
        true,
      );
      const rightGroupId = textAt(
        row.right_group_id,
        `${entryPath}.right_group_id`,
        true,
      );
      const comparability = booleanAt(
        row.measurement_comparability_satisfied,
        `${entryPath}.measurement_comparability_satisfied`,
      );
      const blocked = booleanAt(
        row.interpretation_blocked,
        `${entryPath}.interpretation_blocked`,
      );
      if (leftGroupId === rightGroupId || blocked === comparability) {
        fail(
          "multimod_result.mga_pairwise",
          entryPath,
          "Pairwise group identity or comparability state is invalid.",
        );
      }
      return {
        procedure: textAt(row.procedure, `${entryPath}.procedure`, true),
        left_group_id: leftGroupId,
        right_group_id: rightGroupId,
        target_id: textAt(row.target_id, `${entryPath}.target_id`, true),
        difference_left_minus_right: finiteAt(
          row.difference_left_minus_right,
          `${entryPath}.difference_left_minus_right`,
        ),
        ...(rawP === undefined ? {} : { raw_p_value: rawP }),
        ...(adjustedP === undefined ? {} : { adjusted_p_value: adjustedP }),
        ...(directional === undefined
          ? {}
          : { directional_probability: directional }),
        ...(row.interval == null
          ? {}
          : {
              interval: parseMultimodIntervalV1(
                row.interval,
                `${entryPath}.interval`,
              ),
            }),
        measurement_comparability_satisfied: comparability,
        interpretation_blocked: blocked,
      };
    },
  );
  const excludedRows = arrayAt(item.excluded_rows, `${path}.excluded_rows`).map(
    (entry, index) => {
      const entryPath = `${path}.excluded_rows[${index}]`;
      const row = exactRecordAt(
        entry,
        ["stable_row_token", "typed_group_value", "reason"],
        [],
        entryPath,
      );
      return {
        stable_row_token: textAt(
          row.stable_row_token,
          `${entryPath}.stable_row_token`,
          true,
        ),
        typed_group_value: textAt(
          row.typed_group_value,
          `${entryPath}.typed_group_value`,
        ),
        reason: enumAt(
          row.reason,
          [
            "unselected_group_value",
            "missing_group_value",
            "missing_model_value",
            "invalid_weight",
            "nonfinite_value",
          ] as const,
          `${entryPath}.reason`,
        ),
      };
    },
  );
  if (
    new Set(excludedRows.map((row) => row.stable_row_token)).size !==
    excludedRows.length
  ) {
    fail(
      "multimod_result.target_identity",
      `${path}.excluded_rows`,
      "Excluded-row tokens must be unique.",
    );
  }
  const replicateLedgers = arrayAt(
    item.replicate_ledgers,
    `${path}.replicate_ledgers`,
  ).map((entry, index) =>
    parseLedger(entry, `${path}.replicate_ledgers[${index}]`),
  );
  if (
    replicateLedgers.some((ledger) => !ledger.complete) &&
    (micomPairs.length > 0 ||
      omnibus.length > 0 ||
      pairwise.some(
        (row) =>
          row.raw_p_value !== undefined ||
          row.adjusted_p_value !== undefined ||
          row.directional_probability !== undefined ||
          row.interval !== undefined,
      ))
  ) {
    fail(
      "multimod_result.mga_inference_requires_complete_ledger",
      `${path}.replicate_ledgers`,
      "Incomplete MGA resampling ledgers cannot publish MICOM, permutation/bootstrap, or omnibus inference.",
    );
  }
  return {
    schema_version: 1,
    provenance: parseProvenance(item.provenance, `${path}.provenance`),
    profile: enumAt(
      item.profile,
      [
        "general_sem_pls",
        "multiple_two_way_moderation",
        "bounded_three_way_moderation",
        "bounded_two_way_moderated_mediation",
        "multiple_nonnested_hoc",
        "case_weighted_pls",
        "frequency_weighted_pls",
        "reflective_plsc",
      ] as const,
      `${path}.profile`,
    ),
    group_eligibility: groupEligibility,
    group_parameters: groupParameters,
    micom_pairs: micomPairs,
    omnibus,
    pairwise,
    multiplicity: enumAt(
      item.multiplicity,
      [
        "holm",
        "bonferroni",
        "sidak",
        "benjamini_hochberg_exploratory",
        "none_explicit",
      ] as const,
      `${path}.multiplicity`,
    ),
    replicate_ledgers: replicateLedgers,
    excluded_rows: excludedRows,
    sidecars: parseSidecars(
      hasOwn(item, "sidecars") ? item.sidecars : [],
      resultId,
      `${path}.sidecars`,
    ),
  };
}

function parseHeterogeneityResult(
  value: unknown,
  resultId: string,
  path: string,
): PlsHeterogeneityAnalysisV2 {
  const item = exactRecordAt(
    value,
    [
      "schema_version",
      "provenance",
      "profile",
      "candidates",
      "discovery_result_identity_sha256",
      "parameters",
      "contrasts",
      "descriptive_only",
    ],
    [
      "inference_lock",
      "locked_algorithm",
      "locked_k",
      "bootstrap_ledger",
      "sidecars",
    ],
    path,
  );
  if (countAt(item.schema_version, `${path}.schema_version`) !== 2)
    fail(
      "multimod.result.heterogeneity_schema",
      `${path}.schema_version`,
      "PLS heterogeneity result schema must equal 2.",
    );
  const candidates = arrayAt(item.candidates, `${path}.candidates`).map(
    (entry, index) => {
      const entryPath = `${path}.candidates[${index}]`;
      const row = exactRecordAt(
        entry,
        ["method", "k", "state", "converged_starts", "stable_starts"],
        [
          "log_likelihood",
          "objective",
          "criteria",
          "class_or_segment_shares",
          "pooled_parameters",
          "blockers",
        ],
        entryPath,
      );
      const methodRow = recordAt(row.method, `${entryPath}.method`);
      const method: HeterogeneityCandidateMethodV2 =
        methodRow.kind === "pooled_baseline_v1"
          ? (() => {
              exactRecordAt(
                methodRow,
                ["kind"],
                [],
                `${entryPath}.method`,
              );
              return { kind: "pooled_baseline_v1" } as const;
            })()
          : methodRow.kind === "segmentation"
            ? (() => {
                const parsed = exactRecordAt(
                  methodRow,
                  ["kind", "algorithm"],
                  [],
                  `${entryPath}.method`,
                );
                return {
                  kind: "segmentation",
                  algorithm: enumAt(
                    parsed.algorithm,
                    HETEROGENEITY_ALGORITHMS_V2,
                    `${entryPath}.method.algorithm`,
                  ),
                } as const;
              })()
            : fail(
                "multimod_result.heterogeneity_candidate",
                `${entryPath}.method.kind`,
                "Candidate method must be pooled baseline or segmentation.",
              );
      const logLikelihood = optionalFinite(row, "log_likelihood", entryPath);
      const objective = optionalFinite(row, "objective", entryPath);
      const k = countAt(row.k, `${entryPath}.k`, 5);
      const state = enumAt(
        row.state,
        [
          "eligible",
          "converged_stable",
          "ineligible",
          "failed",
          "unstable",
        ] as const,
        `${entryPath}.state`,
      );
      const convergedStarts = countAt(
        row.converged_starts,
        `${entryPath}.converged_starts`,
      );
      const stableStarts = countAt(
        row.stable_starts,
        `${entryPath}.stable_starts`,
      );
      const shares = arrayAt(
        hasOwn(row, "class_or_segment_shares")
          ? row.class_or_segment_shares
          : [],
        `${entryPath}.class_or_segment_shares`,
      ).map((share, shareIndex) =>
        unitProbabilityAt(
          share,
          `${entryPath}.class_or_segment_shares[${shareIndex}]`,
        ),
      );
      const pooledParameters = arrayAt(
        hasOwn(row, "pooled_parameters") ? row.pooled_parameters : [],
        `${entryPath}.pooled_parameters`,
      ).map((parameter, parameterIndex) =>
        parseEstimate(
          parameter,
          `${entryPath}.pooled_parameters[${parameterIndex}]`,
        ),
      );
      if (
        (method.kind === "pooled_baseline_v1" &&
          (k !== 1 ||
            state !== "eligible" ||
            convergedStarts !== 0 ||
            stableStarts !== 0 ||
            logLikelihood !== undefined ||
            objective !== undefined ||
            shares.length !== 0 ||
            pooledParameters.length === 0 ||
            (hasOwn(row, "blockers") &&
              arrayAt(row.blockers, `${entryPath}.blockers`).length > 0))) ||
        (method.kind === "segmentation" &&
          (k < 2 ||
            k > 5 ||
            pooledParameters.length !== 0 ||
            (shares.length !== 0 && shares.length !== k))) ||
        (shares.length &&
          Math.abs(shares.reduce((total, share) => total + share, 0) - 1) >
            1e-8) ||
        new Set(
          pooledParameters.map((parameter) => parameter.target_id),
        ).size !== pooledParameters.length
      ) {
        fail(
          "multimod_result.heterogeneity_candidate",
          entryPath,
          "Candidate K or class/segment shares are invalid.",
        );
      }
      return {
        method,
        k,
        state,
        converged_starts: convergedStarts,
        stable_starts: stableStarts,
        ...(logLikelihood === undefined
          ? {}
          : { log_likelihood: logLikelihood }),
        ...(objective === undefined ? {} : { objective }),
        criteria: finiteMapAt(
          hasOwn(row, "criteria") ? row.criteria : {},
          `${entryPath}.criteria`,
        ),
        class_or_segment_shares: shares,
        pooled_parameters: pooledParameters,
        blockers: textArrayAt(
          hasOwn(row, "blockers") ? row.blockers : [],
          `${entryPath}.blockers`,
        ),
      };
    },
  );
  if (
    new Set(
      candidates.map((candidate) =>
        candidate.method.kind === "pooled_baseline_v1"
          ? `pooled_baseline_v1\u0000${candidate.k}`
          : `segmentation\u0000${candidate.method.algorithm}\u0000${candidate.k}`,
      ),
    ).size !== candidates.length ||
    candidates.filter(
      (candidate) => candidate.method.kind === "pooled_baseline_v1",
    ).length !== 1
  ) {
    fail(
      "multimod_result.heterogeneity_candidate",
      `${path}.candidates`,
      "Candidate algorithm and K identities must be unique.",
    );
  }
  const parameters = arrayAt(item.parameters, `${path}.parameters`).map(
    (entry, index) => {
      const entryPath = `${path}.parameters[${index}]`;
      const row = exactRecordAt(
        entry,
        ["class_id", "parameter", "metric"],
        [],
        entryPath,
      );
      return {
        class_id: countAt(row.class_id, `${entryPath}.class_id`, 255),
        parameter: parseEstimate(row.parameter, `${entryPath}.parameter`),
        metric: textAt(row.metric, `${entryPath}.metric`, true),
      };
    },
  );
  if (
    parameters.some((row) => row.class_id === 0) ||
    new Set(
      parameters.map(
        (row) => `${row.class_id}\u0000${row.parameter.target_id}`,
      ),
    ).size !== parameters.length
  ) {
    fail(
      "multimod_result.heterogeneity_parameter",
      `${path}.parameters`,
      "Class parameter identities must be positive and unique.",
    );
  }
  const contrasts = arrayAt(item.contrasts, `${path}.contrasts`).map(
    (entry, index) => {
      const entryPath = `${path}.contrasts[${index}]`;
      const row = exactRecordAt(
        entry,
        [
          "left_class_id",
          "right_class_id",
          "target_id",
          "difference",
          "common_metric_comparability_satisfied",
          "inferential_interpretation_blocked",
        ],
        ["p_value", "interval"],
        entryPath,
      );
      const pValue = optionalProbability(row, "p_value", entryPath);
      const leftClassId = countAt(
        row.left_class_id,
        `${entryPath}.left_class_id`,
        255,
      );
      const rightClassId = countAt(
        row.right_class_id,
        `${entryPath}.right_class_id`,
        255,
      );
      const comparability = booleanAt(
        row.common_metric_comparability_satisfied,
        `${entryPath}.common_metric_comparability_satisfied`,
      );
      const blocked = booleanAt(
        row.inferential_interpretation_blocked,
        `${entryPath}.inferential_interpretation_blocked`,
      );
      if (
        !leftClassId ||
        !rightClassId ||
        leftClassId === rightClassId ||
        blocked === comparability
      ) {
        fail(
          "multimod_result.heterogeneity_contrast",
          entryPath,
          "Class contrast identity or common-metric gate is invalid.",
        );
      }
      return {
        left_class_id: leftClassId,
        right_class_id: rightClassId,
        target_id: textAt(row.target_id, `${entryPath}.target_id`, true),
        difference: finiteAt(row.difference, `${entryPath}.difference`),
        ...(pValue === undefined ? {} : { p_value: pValue }),
        ...(row.interval == null
          ? {}
          : {
              interval: parseMultimodIntervalV1(
                row.interval,
                `${entryPath}.interval`,
              ),
            }),
        common_metric_comparability_satisfied: comparability,
        inferential_interpretation_blocked: blocked,
      };
    },
  );
  const lockedAlgorithm =
    item.locked_algorithm == null
      ? undefined
      : enumAt(
          item.locked_algorithm,
          HETEROGENEITY_ALGORITHMS_V2,
          `${path}.locked_algorithm`,
        );
  const lockedK =
    item.locked_k == null
      ? undefined
      : countAt(item.locked_k, `${path}.locked_k`, 255);
  const descriptiveOnly = booleanAt(
    item.descriptive_only,
    `${path}.descriptive_only`,
  );
  const discoveryResultIdentity = shaAt(
    item.discovery_result_identity_sha256,
    `${path}.discovery_result_identity_sha256`,
  );
  const inferenceLock =
    item.inference_lock == null
      ? undefined
      : parseHeterogeneityInferenceLockReceiptV2(
          item.inference_lock,
          `${path}.inference_lock`,
        );
  if (
    (lockedAlgorithm === undefined) !== (lockedK === undefined) ||
    (inferenceLock === undefined) !== (lockedAlgorithm === undefined) ||
    (lockedK !== undefined && (lockedK < 2 || lockedK > 5)) ||
    (inferenceLock !== undefined &&
      (lockedAlgorithm !== inferenceLock.selected_algorithm ||
        lockedK !== inferenceLock.selected_k ||
        discoveryResultIdentity !==
          inferenceLock.discovery_result_identity_sha256)) ||
    (descriptiveOnly &&
      contrasts.some(
        (contrast) =>
          !contrast.inferential_interpretation_blocked ||
          contrast.p_value !== undefined ||
          contrast.interval !== undefined,
      ))
  ) {
    fail(
      "multimod_result.heterogeneity_lock_or_gate",
      path,
      "Locked algorithm/K or descriptive-only comparability gate is inconsistent.",
    );
  }
  if (inferenceLock) {
    const actualInventory = new Set(
      candidates.flatMap((candidate) =>
        candidate.method.kind === "segmentation"
          ? [`${candidate.method.algorithm}\u0000${candidate.k}`]
          : [],
      ),
    );
    const expectedInventory = new Set(
      inferenceLock.discovery_algorithms.flatMap((algorithm) =>
        inferenceLock.discovery_candidate_k.map((k) => `${algorithm}\u0000${k}`),
      ),
    );
    if (
      actualInventory.size !== expectedInventory.size ||
      [...actualInventory].some((identity) => !expectedInventory.has(identity))
    )
      fail(
        "multimod_result.heterogeneity_inference_inventory",
        `${path}.candidates`,
        "Reproduced candidates differ from the locked discovery inventory.",
      );
  }
  const bootstrapLedger = item.bootstrap_ledger == null
    ? undefined
    : parseLedger(item.bootstrap_ledger, `${path}.bootstrap_ledger`);
  if (
    bootstrapLedger &&
    !bootstrapLedger.complete &&
    contrasts.some(
      (contrast) =>
        contrast.p_value !== undefined ||
        contrast.interval !== undefined ||
        !contrast.inferential_interpretation_blocked,
    )
  ) {
    fail(
      "multimod_result.heterogeneity_inference_requires_complete_ledger",
      `${path}.bootstrap_ledger`,
      "An incomplete fixed-K bootstrap ledger permits descriptive segment estimates only.",
    );
  }
  return {
    schema_version: 2,
    provenance: parseProvenance(item.provenance, `${path}.provenance`),
    profile: enumAt(
      item.profile,
      ["p0_structural", "p2_multi_two_way", "p23_all_current"] as const,
      `${path}.profile`,
    ),
    candidates,
    discovery_result_identity_sha256: discoveryResultIdentity,
    ...(inferenceLock ? { inference_lock: inferenceLock } : {}),
    ...(lockedAlgorithm ? { locked_algorithm: lockedAlgorithm } : {}),
    ...(lockedK === undefined ? {} : { locked_k: lockedK }),
    parameters,
    contrasts,
    ...(bootstrapLedger ? { bootstrap_ledger: bootstrapLedger } : {}),
    sidecars: parseSidecars(
      hasOwn(item, "sidecars") ? item.sidecars : [],
      resultId,
      `${path}.sidecars`,
    ),
    descriptive_only: descriptiveOnly,
  };
}

function parseConditionalResult(
  value: unknown,
  resultId: string,
  path: string,
): GeneralSemConditionalProcessResultV2 {
  const item = exactRecordAt(
    value,
    [
      "schema_version",
      "provenance",
      "profile_id",
      "targets",
      "replicate_ledger",
    ],
    ["sidecars", "warnings"],
    path,
  );
  if (countAt(item.schema_version, `${path}.schema_version`) !== 2)
    fail(
      "multimod.result.conditional_schema",
      `${path}.schema_version`,
      "Conditional-process result schema must equal 2.",
    );
  const ledger = parseLedger(item.replicate_ledger, `${path}.replicate_ledger`);
  const targets = arrayAt(item.targets, `${path}.targets`).map(
    (entry, index) => {
      const entryPath = `${path}.targets[${index}]`;
      const row = exactRecordAt(
        entry,
        ["target_id", "kind", "path_id", "estimate", "usable_replicates"],
        [
          "group_id",
          "probe_values",
          "derivative_variables",
          "p_value",
          "interval",
        ],
        entryPath,
      );
      const pValue = optionalProbability(row, "p_value", entryPath);
      return {
        target_id: textAt(row.target_id, `${entryPath}.target_id`, true),
        kind: enumAt(
          row.kind,
          [
            "conditional_specific_indirect",
            "conditional_total_indirect",
            "conditional_total_effect",
            "scalar_index_of_moderated_mediation",
            "local_first_derivative",
            "local_second_derivative",
            "local_cross_derivative",
            "probe_contrast",
            "group_contrast",
          ] as const,
          `${entryPath}.kind`,
        ),
        path_id: textAt(row.path_id, `${entryPath}.path_id`, true),
        ...(row.group_id == null
          ? {}
          : { group_id: textAt(row.group_id, `${entryPath}.group_id`, true) }),
        probe_values: finiteMapAt(
          hasOwn(row, "probe_values") ? row.probe_values : {},
          `${entryPath}.probe_values`,
        ),
        derivative_variables: textArrayAt(
          hasOwn(row, "derivative_variables") ? row.derivative_variables : [],
          `${entryPath}.derivative_variables`,
        ),
        estimate: finiteAt(row.estimate, `${entryPath}.estimate`),
        ...(pValue === undefined ? {} : { p_value: pValue }),
        ...(row.interval == null
          ? {}
          : {
              interval: parseMultimodIntervalV1(
                row.interval,
                `${entryPath}.interval`,
              ),
            }),
        usable_replicates: countAt(
          row.usable_replicates,
          `${entryPath}.usable_replicates`,
        ),
      };
    },
  );
  if (
    !targets.length ||
    new Set(targets.map((target) => target.target_id)).size !==
      targets.length ||
    targets.some((target) => target.usable_replicates > ledger.usable)
  ) {
    fail(
      "multimod_result.conditional_schema_or_targets",
      `${path}.targets`,
      "Conditional target identities or usable-replicate counts are invalid.",
    );
  }
  if (
    !ledger.complete &&
    targets.some(
      (target) =>
        target.p_value !== undefined || target.interval !== undefined,
    )
  ) {
    fail(
      "multimod_result.conditional_inference_requires_complete_ledger",
      `${path}.replicate_ledger`,
      "An incomplete shared conditional-process ledger cannot publish probabilities or intervals.",
    );
  }
  return {
    schema_version: 2,
    provenance: parseProvenance(item.provenance, `${path}.provenance`),
    profile_id: textAt(item.profile_id, `${path}.profile_id`, true),
    targets,
    replicate_ledger: ledger,
    sidecars: parseSidecars(
      hasOwn(item, "sidecars") ? item.sidecars : [],
      resultId,
      `${path}.sidecars`,
    ),
    warnings: textArrayAt(
      hasOwn(item, "warnings") ? item.warnings : [],
      `${path}.warnings`,
    ),
  };
}

function parseCausalResult(
  value: unknown,
  resultId: string,
  path: string,
): InterventionalMediationResultV1 {
  const item = exactRecordAt(
    value,
    [
      "schema_version",
      "provenance",
      "interpretation_label",
      "identification_assumptions",
      "positivity",
      "effects",
      "replicate_ledger",
    ],
    ["sidecars"],
    path,
  );
  if (countAt(item.schema_version, `${path}.schema_version`) !== 1)
    fail(
      "multimod.result.causal_schema",
      `${path}.schema_version`,
      "Interventional mediation result schema must equal 1.",
    );
  const positivity = arrayAt(item.positivity, `${path}.positivity`).map(
    (entry, index) => {
      const entryPath = `${path}.positivity[${index}]`;
      const row = exactRecordAt(
        entry,
        [
          "variable_id",
          "observed_minimum",
          "observed_maximum",
          "requested_value",
          "support_count",
          "minimum_required_count",
          "support_rule",
          "supported",
        ],
        [],
        entryPath,
      );
      const parsed = {
        variable_id: textAt(row.variable_id, `${entryPath}.variable_id`, true),
        observed_minimum: finiteAt(
          row.observed_minimum,
          `${entryPath}.observed_minimum`,
        ),
        observed_maximum: finiteAt(
          row.observed_maximum,
          `${entryPath}.observed_maximum`,
        ),
        requested_value: finiteAt(
          row.requested_value,
          `${entryPath}.requested_value`,
        ),
        support_count: countAt(row.support_count, `${entryPath}.support_count`),
        minimum_required_count: countAt(
          row.minimum_required_count,
          `${entryPath}.minimum_required_count`,
        ),
        support_rule: textAt(
          row.support_rule,
          `${entryPath}.support_rule`,
          true,
        ),
        supported: booleanAt(row.supported, `${entryPath}.supported`),
      };
      if (
        parsed.observed_minimum > parsed.observed_maximum ||
        parsed.minimum_required_count === 0 ||
        parsed.supported !==
          (parsed.requested_value >= parsed.observed_minimum &&
            parsed.requested_value <= parsed.observed_maximum &&
            parsed.support_count >= parsed.minimum_required_count)
      ) {
        fail(
          "multimod_result.causal_positivity",
          entryPath,
          "The positivity diagnostic is inconsistent.",
        );
      }
      return parsed;
    },
  );
  const effects = arrayAt(item.effects, `${path}.effects`).map(
    (entry, index) => {
      const entryPath = `${path}.effects[${index}]`;
      const row = exactRecordAt(
        entry,
        ["target_id", "path_id", "estimand", "estimate"],
        ["p_value", "interval"],
        entryPath,
      );
      const pValue = optionalProbability(row, "p_value", entryPath);
      return {
        target_id: textAt(row.target_id, `${entryPath}.target_id`, true),
        path_id: textAt(row.path_id, `${entryPath}.path_id`, true),
        estimand: textAt(row.estimand, `${entryPath}.estimand`, true),
        estimate: finiteAt(row.estimate, `${entryPath}.estimate`),
        ...(pValue === undefined ? {} : { p_value: pValue }),
        ...(row.interval == null
          ? {}
          : {
              interval: parseMultimodIntervalV1(
                row.interval,
                `${entryPath}.interval`,
              ),
            }),
      };
    },
  );
  const interpretationLabel = textAt(
    item.interpretation_label,
    `${path}.interpretation_label`,
  );
  const identificationAssumptions = textArrayAt(
    item.identification_assumptions,
    `${path}.identification_assumptions`,
  );
  if (
    interpretationLabel !== "assumption-dependent interventional estimate" ||
    !identificationAssumptions.length ||
    !effects.length ||
    new Set(effects.map((effect) => effect.target_id)).size !== effects.length
  ) {
    fail(
      "multimod_result.causal_schema_or_label",
      path,
      "The causal result label, assumption inventory, or effect identities are invalid.",
    );
  }
  const ledger = parseLedger(item.replicate_ledger, `${path}.replicate_ledger`);
  if (
    !ledger.complete &&
    effects.some(
      (effect) =>
        effect.p_value !== undefined || effect.interval !== undefined,
    )
  ) {
    fail(
      "multimod_result.causal_inference_requires_complete_ledger",
      `${path}.replicate_ledger`,
      "An incomplete interventional bootstrap ledger cannot publish probabilities or intervals.",
    );
  }
  return {
    schema_version: 1,
    provenance: parseProvenance(item.provenance, `${path}.provenance`),
    interpretation_label: interpretationLabel,
    identification_assumptions: identificationAssumptions,
    positivity,
    effects,
    replicate_ledger: ledger,
    sidecars: parseSidecars(
      hasOwn(item, "sidecars") ? item.sidecars : [],
      resultId,
      `${path}.sidecars`,
    ),
  };
}

export function parseMultiModAnalysisResultV1(
  value: unknown,
  resultId: string,
  path = "result",
): MultiModAnalysisResultV1 {
  const item = exactRecordAt(value, ["kind", "analysis"], [], path);
  if (item.kind === "pls_multigroup_analysis_v1")
    return {
      kind: "pls_multigroup_analysis_v1",
      analysis: parseMgaResult(item.analysis, resultId, `${path}.analysis`),
    };
  if (item.kind === "pls_heterogeneity_analysis_v2")
    return {
      kind: "pls_heterogeneity_analysis_v2",
      analysis: parseHeterogeneityResult(
        item.analysis,
        resultId,
        `${path}.analysis`,
      ),
    };
  if (item.kind === "general_sem_conditional_process_result_v2")
    return {
      kind: "general_sem_conditional_process_result_v2",
      analysis: parseConditionalResult(
        item.analysis,
        resultId,
        `${path}.analysis`,
      ),
    };
  if (item.kind === "interventional_mediation_result_v1")
    return {
      kind: "interventional_mediation_result_v1",
      analysis: parseCausalResult(item.analysis, resultId, `${path}.analysis`),
    };
  return fail(
    "multimod.result.kind",
    `${path}.kind`,
    "MultiMod result kind is unsupported.",
  );
}

export function parseMultiModResultAttachmentV1(
  value: unknown,
  path = "multimod_result",
): MultiModResultAttachmentV1 {
  const item = exactRecordAt(
    value,
    [
      "schema_version",
      "result_id",
      "recipe_id",
      "result",
      "result_sha256",
      "identity_sha256",
    ],
    ["sidecars"],
    path,
  );
  if (countAt(item.schema_version, `${path}.schema_version`) !== 1)
    fail(
      "multimod.attachment.schema",
      `${path}.schema_version`,
      "MultiMod attachment schema must equal 1.",
    );
  const resultId = textAt(item.result_id, `${path}.result_id`, true);
  if (
    resultId.includes("/") ||
    resultId.includes("\\") ||
    resultId.includes("..")
  )
    fail(
      "multimod.attachment.result_id",
      `${path}.result_id`,
      "Result id cannot contain archive path segments.",
    );
  const identitySha256 = shaAt(item.identity_sha256, `${path}.identity_sha256`);
  const sidecars = parseSidecars(
    hasOwn(item, "sidecars") ? item.sidecars : [],
    resultId,
    `${path}.sidecars`,
  );
  if (
    sidecars.some((descriptor) => descriptor.identity_sha256 !== identitySha256)
  )
    fail(
      "multimod.attachment.sidecar_identity",
      `${path}.sidecars`,
      "Every sidecar identity must match its attachment.",
    );
  const result = parseMultiModAnalysisResultV1(
    item.result,
    resultId,
    `${path}.result`,
  );
  if (JSON.stringify(result.analysis.sidecars) !== JSON.stringify(sidecars)) {
    fail(
      "multimod.attachment.sidecar_inventory",
      `${path}.sidecars`,
      "Attachment and scientific-result sidecar inventories must match exactly.",
    );
  }
  return {
    schema_version: 1,
    result_id: resultId,
    recipe_id: uuidAt(item.recipe_id, `${path}.recipe_id`),
    result,
    result_sha256: shaAt(item.result_sha256, `${path}.result_sha256`),
    identity_sha256: identitySha256,
    sidecars,
  };
}

export type MultiModRecipeConfigV1 =
  | { kind: "mga_multigroup_v1"; config: MgaMultigroupV1 }
  | {
      kind: "pls_unobserved_heterogeneity_v2";
      config: PlsUnobservedHeterogeneityConfigV2;
    }
  | {
      kind: "general_sem_conditional_process_v2";
      config: GeneralSemConditionalProcessConfigV2;
    }
  | {
      kind: "interventional_causal_mediation_v1";
      config: InterventionalCausalMediationConfigV1;
    };

export type MultiModSidecarPredictionInputV1 =
  | {
      kind: "mga";
      groupRows: readonly number[];
      profile: MgaModelProfileV1;
      procedures: readonly MgaProcedureV1[];
      pairCount: number;
      permutationSamples: number;
      bootstrapSamples: number;
      targets: number;
      /** Conservative maximum UTF-8 width of one generated target identity. */
      maximumTargetIdBytes?: number;
      micomConstructs: number;
    }
  | {
      kind: "heterogeneity";
      rows: number;
      candidateK: readonly number[];
      algorithms: readonly HeterogeneityAlgorithmV2[];
      fimixStarts: number;
      fimixMaxIterations: number;
      posStarts: number;
      targets: number;
      inference?: {
        selectedK: number;
        bootstrapResamples: number;
        requestCommonMetricContrasts: boolean;
        commonMetricPermutationSamples: number;
      };
    }
  | {
      kind: "conditional";
      rows: number;
      outerResamples: number;
      innerResamples: number;
      targets: number;
      profile: ConditionalProcessProfileV2;
    }
  | {
      kind: "causal";
      rows: number;
      resamples: number;
      targets: number;
    };

function sidecarCountV1(value: number, minimum = 0): number {
  if (!Number.isFinite(value)) return Number.MAX_SAFE_INTEGER;
  return Math.max(minimum, Math.trunc(value));
}

function sidecarAddV1(...values: number[]): number {
  let total = 0;
  for (const value of values) {
    if (!Number.isSafeInteger(value) || value < 0 || total > Number.MAX_SAFE_INTEGER - value) {
      return Number.MAX_SAFE_INTEGER;
    }
    total += value;
  }
  return total;
}

function sidecarMulV1(...values: number[]): number {
  let total = 1;
  for (const value of values) {
    if (!Number.isSafeInteger(value) || value < 0) return Number.MAX_SAFE_INTEGER;
    if (value !== 0 && total > Math.floor(Number.MAX_SAFE_INTEGER / value)) {
      return Number.MAX_SAFE_INTEGER;
    }
    total *= value;
  }
  return total;
}

const MGA_ARROW_STREAM_FIXED_BYTES_V1 = 4 * 1024;
const MGA_DICTIONARY_TARGET_LEDGER_ROW_BYTES_V1 = 21;
const MGA_MICOM_NULL_STATISTIC_ROW_BYTES_V1 = 18;

function dictionaryValueBytesUpperBoundV1(
  uniqueValues: number,
  maximumValueBytes: number,
): number {
  return sidecarAddV1(
    MGA_ARROW_STREAM_FIXED_BYTES_V1,
    sidecarMulV1(uniqueValues, sidecarAddV1(maximumValueBytes, 4)),
  );
}

/**
 * Conservative uncompressed Arrow prediction. MGA and heterogeneity mirror
 * the native runner formula; conditional/causal additionally charge every
 * persisted draw-row identity and profile-specific weight/count coordinate.
 * Checked arithmetic saturates to a fail-closed blocked value.
 */
export function predictMultiModSidecarBytesV1(
  input: MultiModSidecarPredictionInputV1,
): number {
  if (input.kind === "mga") {
    const groupRows = input.groupRows.map((value) => sidecarCountV1(value));
    const rows = sidecarAddV1(...groupRows);
    const groups = groupRows.length;
    const targets = sidecarCountV1(input.targets, 1);
    const maximumTargetIdBytes = sidecarCountV1(
      input.maximumTargetIdBytes ?? 256,
      1,
    );
    const micomConstructs = sidecarCountV1(input.micomConstructs, 1);
    const pairs = sidecarCountV1(input.pairCount);
    const permutations = sidecarCountV1(input.permutationSamples);
    const bootstraps = sidecarCountV1(input.bootstrapSamples);
    const weighted = input.profile === "case_weighted_pls" || input.profile === "frequency_weighted_pls";
    const extraDrawCoordinate = weighted ? 8 : 0;
    const drawRowBytes = 8 + extraDrawCoordinate;
    let total = sidecarAddV1(
      sidecarMulV1(rows, 112 + extraDrawCoordinate),
      sidecarMulV1(groups, targets, 64),
    );
    const usesPairwisePlan = input.procedures.some(
      (procedure) => procedure === "micom_pairwise" || procedure === "pairwise_permutation",
    );
    if (usesPairwisePlan) {
      total = sidecarAddV1(total, sidecarMulV1(pairs, permutations, 80));
    }
    if (input.procedures.includes("pairwise_permutation")) {
      total = sidecarAddV1(
        total,
        sidecarMulV1(pairs, permutations, 224),
        sidecarMulV1(
          pairs,
          dictionaryValueBytesUpperBoundV1(1, maximumTargetIdBytes),
        ),
      );
    }
    if (input.procedures.includes("micom_pairwise")) {
      const nullRowsPerPair = sidecarMulV1(
        permutations,
        sidecarAddV1(micomConstructs, 2),
      );
      total = sidecarAddV1(
        total,
        sidecarMulV1(
          pairs,
          sidecarAddV1(
            sidecarMulV1(permutations, 96),
            MGA_ARROW_STREAM_FIXED_BYTES_V1,
            sidecarMulV1(
              nullRowsPerPair,
              MGA_MICOM_NULL_STATISTIC_ROW_BYTES_V1,
            ),
          ),
        ),
      );
    }
    if (input.procedures.includes("omnibus_max_spread_permutation")) {
      const perDraw = sidecarAddV1(
        sidecarMulV1(rows, drawRowBytes),
        sidecarMulV1(targets, MGA_DICTIONARY_TARGET_LEDGER_ROW_BYTES_V1),
        128,
      );
      total = sidecarAddV1(
        total,
        sidecarMulV1(permutations, perDraw),
        dictionaryValueBytesUpperBoundV1(targets, maximumTargetIdBytes),
      );
    }
    if (
      input.procedures.includes("henseler_pls_mga") ||
      input.procedures.includes("bootstrap_difference_bc")
    ) {
      const targetCells = sidecarMulV1(groups, targets);
      const perDraw = sidecarAddV1(
        sidecarMulV1(targetCells, MGA_DICTIONARY_TARGET_LEDGER_ROW_BYTES_V1),
        sidecarMulV1(groups, 112),
      );
      total = sidecarAddV1(
        total,
        sidecarMulV1(bootstraps, perDraw),
        dictionaryValueBytesUpperBoundV1(
          targetCells,
          sidecarAddV1(maximumTargetIdBytes, 9),
        ),
      );
    }
    return total;
  }

  if (input.kind === "heterogeneity") {
    const rows = sidecarCountV1(input.rows);
    const targets = sidecarCountV1(input.targets, 1);
    let total = sidecarAddV1(
      sidecarMulV1(rows, 112),
      sidecarMulV1(targets, 128),
      sidecarMulV1(rows, targets, 64),
    );
    for (const rawK of input.candidateK) {
      const k = sidecarCountV1(rawK, 1);
      for (const algorithm of input.algorithms) {
        if (algorithm === "fimix_pls_v2") {
          total = sidecarAddV1(
            total,
            sidecarMulV1(rows, k, 8),
            sidecarMulV1(rows, 8),
            sidecarMulV1(
              sidecarCountV1(input.fimixStarts),
              sidecarCountV1(input.fimixMaxIterations),
              sidecarAddV1(40, sidecarMulV1(k, 8)),
            ),
            sidecarMulV1(k, targets, 48),
            2_048,
          );
        } else {
          const starts = sidecarCountV1(input.posStarts);
          const moveCap = Math.max(1_000, sidecarMulV1(2, rows));
          total = sidecarAddV1(
            total,
            sidecarMulV1(starts, rows, 8),
            sidecarMulV1(starts, moveCap, 80),
            sidecarMulV1(rows, 8),
            sidecarMulV1(k, targets, 48),
            sidecarMulV1(rows, targets, 32),
            2_048,
          );
        }
      }
    }
    if (input.inference) {
      const k = sidecarCountV1(input.inference.selectedK, 1);
      const perDraw = sidecarAddV1(
        sidecarMulV1(rows, 8),
        sidecarMulV1(k, targets, 8),
        sidecarMulV1(k, k, 8),
        192,
      );
      total = sidecarAddV1(
        total,
        sidecarMulV1(sidecarCountV1(input.inference.bootstrapResamples), perDraw),
      );
      if (input.inference.requestCommonMetricContrasts) {
        const pairs = sidecarMulV1(k, Math.max(k - 1, 0)) / 2;
        const commonMetricDraw = sidecarAddV1(
          sidecarMulV1(rows, 8),
          sidecarMulV1(targets, 24),
          160,
        );
        total = sidecarAddV1(
          total,
          sidecarMulV1(
            pairs,
            sidecarCountV1(input.inference.commonMetricPermutationSamples),
            commonMetricDraw,
          ),
        );
      }
    }
    return total;
  }

  const rows = sidecarCountV1(input.rows);
  const targets = sidecarCountV1(input.targets, 1);
  const resamples = sidecarCountV1(
    input.kind === "causal" ? input.resamples : input.outerResamples,
  );
  const extraDrawBytes = input.kind === "conditional"
    ? input.profile === "case_weighted_percentile"
      ? 12
      : input.profile === "frequency_weighted_percentile"
        ? 8
        : 0
    : 0;
  const perDraw = sidecarAddV1(
    sidecarMulV1(rows, 8 + extraDrawBytes),
    sidecarMulV1(targets, 32),
    192,
  );
  let total = sidecarAddV1(
    64 * 1024,
    sidecarMulV1(rows, 112 + extraDrawBytes),
    sidecarMulV1(resamples, perDraw),
  );
  if (input.kind === "conditional" && input.profile === "multi_two_way_bca") {
    total = sidecarAddV1(total, sidecarMulV1(rows, perDraw));
  }
  if (input.kind === "conditional" && input.profile === "multi_two_way_studentized") {
    total = sidecarAddV1(
      total,
      sidecarMulV1(
        resamples,
        sidecarCountV1(input.innerResamples),
        perDraw,
      ),
    );
  }
  return total;
}

export function multiModSidecarCostStateV1(
  bytes: number,
): "normal" | "warning" | "blocked" {
  if (bytes > MULTIMOD_SIDECAR_MAX_BYTES_V1) return "blocked";
  if (bytes > MULTIMOD_SIDECAR_WARN_BYTES_V1) return "warning";
  return "normal";
}
