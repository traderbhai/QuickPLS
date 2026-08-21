import type { CapabilityCellReferenceV2 } from "./canonicalResultDocumentV2";
import {
  GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_CELL_V1,
  GENERAL_SEM_PLS_HIGHER_ORDER_POINT_CELL_V1,
} from "./generalSemHigherOrderContractV1";
import type {
  HigherOrderConstructionApproachV4,
  HigherOrderMeasurementTypeV4,
} from "./semModelV4";
import { sha256HexBytesV1, sha256HexUtf8V1 } from "./sha256V1";

export const CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION = 1 as const;
export const GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1 = "general_sem_pls_full_model_case_bootstrap_v1" as const;
export const GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1 = "general_sem_pls_single_mediation_full_model_case_bootstrap_v1" as const;
export const GENERAL_SEM_PLS_CASE_BOOTSTRAP_OPERATION_VERSION_V1 = "general_sem_pls_case_bootstrap_v1" as const;
export const GENERAL_SEM_PLS_SINGLE_MEDIATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1 = "general_sem_pls_single_mediation_case_bootstrap_v1" as const;
export const GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1 = "indexed_case_resampling_v1" as const;
export const GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1 = "type7_quantile_v1" as const;
export const GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1 = "sample_standard_error_b_minus_1_v1" as const;
export const GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1 = "neumaier_compensated_sum_v1" as const;
export const GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1 = "null_centered_plus_one_v1" as const;
export const GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1 = "minimum_usable_fraction_0_9_v1" as const;
export const GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1 = "qpls.general-sem-pls.multiple-two-way.point.v1" as const;
export const GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1 = "qpls.general-sem-pls.multiple-two-way.full-model-case-bootstrap.v1" as const;
export const GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1 = "general_sem_pls_multiple_two_way_moderation_case_bootstrap_v1" as const;
export const GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1 = "general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1" as const;
export const GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1 = "general_sem_pls_two_way_moderated_mediation_case_bootstrap_v1" as const;
export const GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1 = "qpls.general-sem-pls.three-way.point.v1" as const;
export const GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1 = "qpls.general-sem-pls.three-way.full-model-case-bootstrap.v1" as const;
export const GENERAL_SEM_PLS_THREE_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1 = "general_sem_pls_three_way_moderation_case_bootstrap_v1" as const;
export const GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1 = "qpls.general-sem-pls.three-way.fixed-probes.v1" as const;
export const GENERAL_SEM_PLS_MULTIPLE_MODERATION_SIGN_ALIGNMENT_VERSION_V1 = "sampled_original_construct_score_covariance_v1" as const;
export const GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1 = "compiled_interaction_scientific_rescaled_gamma_v1" as const;
export const GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1 = "qpls.general-sem-pls.two-stage-product.sample-standardized.v1" as const;
export const GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1 = "qpls.general-sem-pls.simple-slope.other-moderators-zero.v1" as const;
export const GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1 = "qpls.general-sem-pls.interaction-hierarchy.strong.v1" as const;
export const CBSEM_RECURSIVE_SEM_BOOTSTRAP_METHOD_VERSION_V1 = "cbsem_exact_recursive_sem_case_bootstrap_v1" as const;
export const CBSEM_RECURSIVE_SEM_BOOTSTRAP_OPERATION_VERSION_V1 = "cbsem_recursive_sem_full_ml_case_bootstrap_v1" as const;
export const GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_OPERATION_VERSION_V1 = "general_sem_pls_higher_order_full_model_case_bootstrap_operation_v1" as const;
export const GENERAL_SEM_PLS_DISJOINT_HOC_SIGN_ALIGNMENT_VERSION_V1 = "sampled_original_construct_score_covariance_v1" as const;
export const GENERAL_SEM_PLS_DISJOINT_HOC_TARGET_VERSION_V1 = "compiled_hoc_component_and_structural_relation_target_v1" as const;
export const GENERAL_SEM_PLS_HIGHER_ORDER_POINT_STAGE_RECEIPT_VERSION_V1 = "general_sem_pls_higher_order_point_stage_receipt_v1" as const;
export const GENERAL_SEM_PLS_DISJOINT_HOC_SCORE_DATASET_RECEIPT_VERSION_V1 = "general_sem_pls_disjoint_hoc_score_dataset_receipt_v1" as const;

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
  | "stage_one_rank_deficient"
  | "stage_one_nonconvergence"
  | "indeterminate_score_sign"
  | "constant_construct_score"
  | "constant_interaction_product"
  | "rank_deficient"
  | "joint_stage_rank_deficient"
  | "isolated_construct"
  | "estimation_nonconvergence"
  | "target_inventory_mismatch"
  | "numerical_failure";

export interface CanonicalGeneralSemFailedReplicateV1 {
  replicate_index: number;
  reason_code: CanonicalGeneralSemFailedReplicateReasonV1;
  message: string;
}

export interface CanonicalGeneralSemInferenceReceiptV1 {
  kind: CanonicalGeneralSemInferenceKindV1;
  capability_cell: CapabilityCellReferenceV2;
  capability_dependencies?: CapabilityCellReferenceV2[];
  method_version:
    | typeof GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1
    | typeof GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1
    | typeof GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
    | typeof GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1;
  resampling_operation_version:
    | typeof GENERAL_SEM_PLS_CASE_BOOTSTRAP_OPERATION_VERSION_V1
    | typeof GENERAL_SEM_PLS_SINGLE_MEDIATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1
    | typeof GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1
    | typeof GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1;
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
    }
  | {
      kind: "interaction_scientific_rescaled_gamma";
      effect_id: string;
      interaction_id: string;
      focal_relation_id: string;
      interaction_effect_relation_id: string;
      interaction_effect_parameter_id: string;
      generated_product_column_id: string;
      focal_predictor_id: string;
      moderator_id: string;
      outcome_id: string;
      stage_one_model_scientific_sha256: string;
      product_scale_version: string;
      method_version: string;
    }
  | {
      kind: "conditional_indirect";
      effect_id: string;
      target_id: string;
      estimand_id: string;
      moderated_stage: CanonicalModeratedMediationStageV1;
      interaction_id: string;
      x_id: string;
      mediator_id: string;
      y_id: string;
      moderator_id: string;
      ordered_relation_ids: string[];
      probe_value_index: number;
      moderator_value_bits_hex: string;
    }
  | {
      kind: "moderated_mediation_index";
      effect_id: string;
      target_id: string;
      estimand_id: string;
      moderated_stage: CanonicalModeratedMediationStageV1;
      interaction_id: string;
      x_id: string;
      mediator_id: string;
      y_id: string;
      moderator_id: string;
      ordered_relation_ids: string[];
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

export type CanonicalInteractionHierarchyPolicyV1 = "strong";
export type CanonicalInteractionConstructionMethodV1 = "two_stage";

export interface CanonicalInteractionEffectResultV1 {
  effect_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  interaction_id: string;
  focal_relation_id: string;
  interaction_effect_relation_id: string;
  interaction_effect_parameter_id: string;
  focal_predictor_id: string;
  moderator_id: string;
  outcome_id: string;
  generated_product_column_id: string;
  stage_one_model_scientific_sha256: string;
  method_version: string;
  construction_method: CanonicalInteractionConstructionMethodV1;
  product_scale_version: string;
  hierarchy_policy: CanonicalInteractionHierarchyPolicyV1;
  hierarchy_policy_version: string;
  conditioning_policy_version: string;
  observation_count: number;
  unstandardized_product_mean: number;
  unstandardized_product_sample_standard_deviation: number;
  standardized_product_coefficient: CanonicalGeneralSemEstimateV1;
  scientific_rescaled_gamma: CanonicalGeneralSemEstimateV1;
}

export interface CanonicalThreeWayInteractionEffectResultV1 {
  effect_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  interaction_id: string;
  focal_relation_id: string;
  interaction_effect_relation_id: string;
  interaction_effect_parameter_id: string;
  operand_ids: [string, string, string];
  outcome_id: string;
  generated_product_column_id: string;
  stage_one_model_scientific_sha256: string;
  method_version: typeof GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1;
  product_scale_version: string;
  hierarchy_policy: CanonicalInteractionHierarchyPolicyV1;
  hierarchy_policy_version: string;
  observation_count: number;
  unstandardized_product_mean: number;
  unstandardized_product_sample_standard_deviation: number;
  standardized_product_coefficient: CanonicalGeneralSemEstimateV1;
  scientific_rescaled_delta: CanonicalGeneralSemEstimateV1;
}

export type CanonicalThreeWayModeratorProbeKindV1 =
  | "continuous_standardized"
  | "binary_zero_one";

export interface CanonicalThreeWayConditionalInteractionEffectResultV1 {
  effect_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  interaction_id: string;
  focal_relation_id: string;
  first_moderator_id: string;
  second_moderator_id: string;
  second_moderator_probe_kind: CanonicalThreeWayModeratorProbeKindV1;
  second_moderator_probe_index: number;
  second_moderator_value: number;
  value: CanonicalGeneralSemEstimateV1;
}

export interface CanonicalThreeWaySimpleSlopeResultV1 {
  effect_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  interaction_id: string;
  focal_relation_id: string;
  first_moderator_id: string;
  second_moderator_id: string;
  first_moderator_probe_kind: CanonicalThreeWayModeratorProbeKindV1;
  second_moderator_probe_kind: CanonicalThreeWayModeratorProbeKindV1;
  first_probe_index: number;
  first_moderator_value: number;
  second_probe_index: number;
  second_moderator_value: number;
  value: CanonicalGeneralSemEstimateV1;
}

export interface CanonicalThreeWayModerationBootstrapReceiptV1 {
  capability_cell: CapabilityCellReferenceV2;
  capability_dependencies: CapabilityCellReferenceV2[];
  method_version: typeof GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1;
  point_method_version: typeof GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1;
  resampling_operation_version: typeof GENERAL_SEM_PLS_THREE_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1;
  resampling_stream_version: typeof GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1;
  quantile_method_version: typeof GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1;
  standard_error_method_version: typeof GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1;
  summation_method_version: typeof GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1;
  p_value_method_version: typeof GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1;
  failure_policy_version: typeof GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1;
  sign_alignment_method_version: typeof GENERAL_SEM_PLS_MULTIPLE_MODERATION_SIGN_ALIGNMENT_VERSION_V1;
  product_scale_version: typeof GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1;
  probe_policy_version: typeof GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1;
  compiled_plan_sha256: string;
  general_sem_config_sha256: string;
  model_scientific_sha256: string;
  stage_one_model_scientific_sha256: string;
  source_dataset_fingerprint: string;
  complete_case_frame_sha256: string;
  usable_replicate_indices_sha256: string;
  target_identity_set_sha256: string;
  target_ids: string[];
  interval: CanonicalGeneralSemBootstrapIntervalV1;
  tail: CanonicalGeneralSemInferenceTailV1;
  confidence_level: number;
  resamples_requested: number;
  resamples_usable: number;
  minimum_usable_resamples: number;
  seed: string;
  workers: number;
  complete_model_reestimated_per_replicate: boolean;
  shared_stage_one_reestimated_per_replicate: true;
  score_vectors_sign_aligned_before_products: true;
  all_lower_order_and_three_way_products_recomputed_per_replicate: true;
  joint_stage_two_reestimated_per_replicate: true;
  complete_joint_point_contract_validated_per_replicate: true;
  all_three_way_targets_share_one_replicate_ledger: true;
  failed_replicates: CanonicalGeneralSemFailedReplicateV1[];
}

export type CanonicalStructuralRelationRoleV1 = "structural" | "control";
export type CanonicalStructuralEstimateStageV1 = "joint_stage_two";

export interface CanonicalJointStageStructuralCoefficientResultV1 {
  relation_id: string;
  parameter_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  source_id: string;
  target_id: string;
  role: CanonicalStructuralRelationRoleV1;
  estimate: CanonicalGeneralSemEstimateV1;
  stage: CanonicalStructuralEstimateStageV1;
  method_version:
    | typeof GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1
    | typeof GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1;
}

export interface CanonicalConditionalEffectResultV1 {
  effect_id: string;
  estimand_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  interaction_id: string;
  interaction_effect_id?: string;
  focal_relation_id: string;
  probe_id: string;
  moderator_id: string;
  probe_value_index: number;
  moderator_value: number;
  value: CanonicalGeneralSemEstimateV1;
}

export type CanonicalModeratedMediationStageV1 = "first_stage" | "second_stage";

export interface CanonicalConditionalIndirectEffectResultV1 {
  effect_id: string;
  target_id: string;
  estimand_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  moderated_stage: CanonicalModeratedMediationStageV1;
  interaction_id: string;
  x_id: string;
  mediator_id: string;
  y_id: string;
  moderator_id: string;
  ordered_relation_ids: string[];
  probe_value_index: number;
  moderator_value: number;
  value: CanonicalGeneralSemEstimateV1;
}

export interface CanonicalModeratedMediationIndexResultV1 {
  effect_id: string;
  target_id: string;
  estimand_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  moderated_stage: CanonicalModeratedMediationStageV1;
  interaction_id: string;
  x_id: string;
  mediator_id: string;
  y_id: string;
  moderator_id: string;
  ordered_relation_ids: string[];
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
  interaction_effect_id?: string;
  focal_relation_id: string;
  focal_predictor_id: string;
  moderator_id: string;
  outcome_id: string;
  series: CanonicalInteractionPlotSeriesV1[];
}

export type CanonicalHocStageKindV1 =
  | "lower_order_score_estimation"
  | "higher_order_estimation";

export type CanonicalHocRelationKindV1 =
  | "component_loading"
  | "component_weight"
  | "authored_structural"
  | "authored_control"
  | "technical_structural"
  | "extended_indirect_effect"
  | "extended_total_effect";

export type CompiledPlsHocComponentRelationInterpretationV1 =
  | "loading"
  | "weight_and_collinearity";

export type CompiledPlsHocStageRoleV1 =
  | "repeated_indicator_estimation"
  | "extended_repeated_indicator_estimation"
  | "embedded_repeated_indicator_estimation"
  | "disjoint_lower_order_score_estimation"
  | "higher_order_from_lower_order_scores";

export interface CanonicalHocGeneratedVariableMappingV1 {
  component_id: string;
  generated_score_variable_id: string;
  generated_component_relation_id: string;
  generated_component_parameter_id: string;
  component_relation_source_id: string;
  component_relation_target_id: string;
  relation_interpretation: CompiledPlsHocComponentRelationInterpretationV1;
}

export interface CanonicalHocGeneratedScoreColumnReceiptV1 {
  component_id: string;
  generated_score_variable_id: string;
  observation_count: number;
  values_sha256: string;
}

export interface CanonicalHocGeneratedScoreDatasetReceiptV1 {
  receipt_version: string;
  source_dataset_fingerprint: string;
  complete_case_row_count: number;
  omitted_row_count: number;
  complete_case_rows_sha256: string;
  generated_score_columns: CanonicalHocGeneratedScoreColumnReceiptV1[];
}

export interface CanonicalHocPointStageReceiptV1 {
  receipt_version: string;
  stage_number: number;
  role: CompiledPlsHocStageRoleV1;
  projection_identity_sha256: string;
  model_scientific_sha256: string;
  compiled_plan_sha256: string;
  dataset_fingerprint: string;
  used_observations: number;
  omitted_observations: number;
  generated_score_dataset?: CanonicalHocGeneratedScoreDatasetReceiptV1 | null;
}

export interface CanonicalHocRelationEstimateV1 {
  relation_id: string;
  parameter_id?: string | null;
  source_id: string;
  target_id: string;
  kind?: CanonicalHocRelationKindV1 | null;
  value: CanonicalGeneralSemEstimateV1;
  collinearity_vif?: number | null;
}

export interface CanonicalHocStageResultV1 {
  stage_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  higher_order_construct_id: string;
  stage_number: number;
  kind: CanonicalHocStageKindV1;
  input_construct_ids: string[];
  output_variable_ids: string[];
  approach?: HigherOrderConstructionApproachV4 | null;
  measurement_type?: HigherOrderMeasurementTypeV4 | null;
  generated_variable_mappings?: CanonicalHocGeneratedVariableMappingV1[];
  receipt?: CanonicalHocPointStageReceiptV1 | null;
  relation_estimates?: CanonicalHocRelationEstimateV1[];
}

export type CanonicalHocBootstrapTargetKindV1 =
  | "component_loading"
  | "component_weight"
  | "hoc_structural_path"
  | "extended_total_effect";

export interface CanonicalHocBootstrapTargetIdentityV1 {
  kind: CanonicalHocBootstrapTargetKindV1;
  target_version: string;
  target_id: string;
  relation_id: string;
  parameter_id: string;
  source_id: string;
  target_variable_id: string;
  point_method_version: string;
}

export type CanonicalHocBootstrapFailureReasonV1 =
  | "insufficient_observations"
  | "constant_indicator"
  | "stage_one_rank_deficient"
  | "isolated_construct"
  | "stage_one_nonconvergence"
  | "indeterminate_score_sign"
  | "constant_component_score"
  | "stage_two_rank_deficient"
  | "stage_two_nonconvergence"
  | "component_collinearity"
  | "numerical_failure";

export interface CanonicalHocBootstrapFailedReplicateV1 {
  replicate_index: number;
  reason_code: CanonicalHocBootstrapFailureReasonV1;
  message: string;
}

export interface CanonicalHocBootstrapReceiptV1 {
  schema_version: 1;
  capability_cell: CapabilityCellReferenceV2;
  method_version: string;
  point_method_version: string;
  resampling_operation_version: string;
  resampling_stream_version: string;
  quantile_method_version: string;
  standard_error_method_version: string;
  summation_method_version: string;
  p_value_method_version: string;
  failure_policy_version: string;
  sign_alignment_method_version: string;
  target_version: string;
  general_sem_config_sha256: string;
  compiled_plan_sha256: string;
  hoc_stage_plan_sha256: string;
  model_scientific_sha256: string;
  stage_one_model_scientific_sha256: string;
  stage_two_model_scientific_sha256: string;
  source_dataset_fingerprint: string;
  complete_case_frame_sha256: string;
  usable_replicate_indices_sha256: string;
  target_identity_set_sha256: string;
  target_ids: string[];
  target_identities: CanonicalHocBootstrapTargetIdentityV1[];
  interval: CanonicalGeneralSemBootstrapIntervalV1;
  tail: CanonicalGeneralSemInferenceTailV1;
  confidence_level: number;
  resamples_requested: number;
  resamples_usable: number;
  minimum_usable_resamples: number;
  seed: string;
  workers: number;
  complete_model_reestimated_per_replicate: boolean;
  stage_one_reestimated_per_replicate: boolean;
  generated_component_values_recalculated_per_replicate: boolean;
  stage_one_scores_sign_aligned_per_replicate: boolean;
  stage_two_reestimated_per_replicate: boolean;
  stage_two_scores_sign_aligned_per_replicate: boolean;
  complete_point_contract_validated_per_replicate: boolean;
  failed_replicates: CanonicalHocBootstrapFailedReplicateV1[];
}

export interface CanonicalGeneralSemIntervalV1 {
  confidence_level: number;
  lower: number;
  upper: number;
}

export type CanonicalCbsemParameterRoleV1 = "loading" | "regression" | "covariance" | "variance";

export type CanonicalCbsemEndpointV1 =
  | { kind: "variable"; variable_id: string }
  | { kind: "residual"; variable_id: string }
  | { kind: "disturbance"; variable_id: string };

export type CanonicalCbsemParameterTargetV1 =
  | { kind: "loading"; factor_id: string; indicator_id: string }
  | { kind: "regression"; source_id: string; target_id: string }
  | { kind: "covariance"; left: CanonicalCbsemEndpointV1; right: CanonicalCbsemEndpointV1 }
  | { kind: "variance"; endpoint: CanonicalCbsemEndpointV1 };

export type CanonicalCbsemParameterStateV1 =
  | { kind: "fixed"; value: number }
  | { kind: "free"; equality_label?: string | null; lower?: number | null; upper?: number | null };

export interface CanonicalCbsemParameterResultV1 {
  parameter_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  role: CanonicalCbsemParameterRoleV1;
  target: CanonicalCbsemParameterTargetV1;
  relation_id?: string | null;
  state: CanonicalCbsemParameterStateV1;
  estimate: number;
  standard_error?: number | null;
  z_value?: number | null;
  p_value?: number | null;
  standardized_estimate?: number | null;
}

export type CanonicalCbsemBootstrapFailedReplicateReasonV1 =
  | "insufficient_observations"
  | "nonpositive_definite_sample_covariance"
  | "nonconvergence"
  | "nonfinite_estimate"
  | "parameter_inventory_mismatch"
  | "numerical_failure";

export interface CanonicalCbsemBootstrapFailedReplicateV1 {
  replicate_index: number;
  reason_code: CanonicalCbsemBootstrapFailedReplicateReasonV1;
  message: string;
}

export interface CanonicalCbsemBootstrapReceiptV1 {
  capability_cell: CapabilityCellReferenceV2;
  method_version: typeof CBSEM_RECURSIVE_SEM_BOOTSTRAP_METHOD_VERSION_V1;
  resampling_operation_version: typeof CBSEM_RECURSIVE_SEM_BOOTSTRAP_OPERATION_VERSION_V1;
  quantile_method_version: typeof GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1;
  compiled_plan_sha256: string;
  base_plan_sha256: string;
  parameter_inventory_sha256: string;
  model_scientific_sha256: string;
  general_sem_config_sha256: string;
  recipe_analytical_sha256: string;
  source_dataset_fingerprint: string;
  complete_case_frame_sha256: string;
  usable_replicate_indices_sha256: string;
  confidence_level: number;
  resamples_requested: number;
  resamples_usable: number;
  minimum_usable_resamples: number;
  seed: string;
  workers: number;
  complete_model_reestimated_per_replicate: boolean;
  failed_replicates: CanonicalCbsemBootstrapFailedReplicateV1[];
}

export type CanonicalCbsemBootstrapInferenceOutcomeV1 =
  | { kind: "available"; value: CanonicalGeneralSemEstimateV1 }
  | { kind: "unavailable"; reason: "insufficient_usable_replicates" | "parameter_not_eligible" };

export interface CanonicalCbsemBootstrapParameterInferenceV1 {
  parameter_id: string;
  trace: CanonicalGeneralSemResultTraceV1;
  point_estimate: number;
  outcome: CanonicalCbsemBootstrapInferenceOutcomeV1;
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
  | "provisional"
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
  joint_stage_structural_coefficients?: CanonicalJointStageStructuralCoefficientResultV1[];
  interaction_effects?: CanonicalInteractionEffectResultV1[];
  three_way_interaction_effects?: CanonicalThreeWayInteractionEffectResultV1[];
  three_way_conditional_interaction_effects?: CanonicalThreeWayConditionalInteractionEffectResultV1[];
  three_way_simple_slopes?: CanonicalThreeWaySimpleSlopeResultV1[];
  three_way_moderation_bootstrap_receipt?: CanonicalThreeWayModerationBootstrapReceiptV1 | null;
  conditional_effect_probes?: CanonicalConditionalEffectProbeResultV1[];
  conditional_effects?: CanonicalConditionalEffectResultV1[];
  conditional_indirect_effects?: CanonicalConditionalIndirectEffectResultV1[];
  moderated_mediation_indices?: CanonicalModeratedMediationIndexResultV1[];
  interaction_plots?: CanonicalInteractionPlotResultV1[];
  higher_order_stages?: CanonicalHocStageResultV1[];
  cbsem_parameters?: CanonicalCbsemParameterResultV1[];
  higher_order_inference_receipt?: CanonicalHocBootstrapReceiptV1 | null;
  cbsem_fit?: CanonicalCbsemFitResultV1[];
  identification_diagnostics?: CanonicalIdentificationDiagnosticV1[];
  cbsem_bootstrap_receipt?: CanonicalCbsemBootstrapReceiptV1 | null;
  cbsem_bootstrap_inference?: CanonicalCbsemBootstrapParameterInferenceV1[];
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
export const GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_CAPABILITY_CELL_V1: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.mediation",
  cell_id: "qpls3.pls.general_sem_single_mediation_bootstrap",
  capability_version: "general_sem_pls_single_mediation_full_model_case_bootstrap_v1",
};
const GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_CAPABILITY_CELL_V1: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_point",
  capability_version: "general_sem_pls_multiple_two_way_moderation_point_v1",
};
const GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
  capability_version: "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
};
export const GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_three_way_moderation_point",
  capability_version: "general_sem_pls_three_way_moderation_point_v1",
};
export const GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_three_way_moderation_bootstrap",
  capability_version: "general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1",
};
const CBSEM_GENERAL_SEM_ML_CAPABILITY_CELL_V1: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.cbsem",
  cell_id: "qpls3.cbsem.general_sem_ml",
  capability_version: "cbsem_general_sem_ml_v1",
};
const CBSEM_RECURSIVE_SEM_BOOTSTRAP_CAPABILITY_CELL_V1: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.cbsem_bootstrapping",
  cell_id: "qpls3.cbsem.bootstrap.recursive_sem",
  capability_version: "cbsem_exact_recursive_sem_case_bootstrap_v1",
};
const GENERAL_SEM_PLS_BASE_CAPABILITY_CELL_V1: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.pls_algorithm",
  cell_id: "qpls3.pls.algorithm",
  capability_version: "pls_pm_v1",
};
const GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CAPABILITY_CELL_V1: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.mediation",
  cell_id: "qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap",
  capability_version: "general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1",
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

function conditionalIndirectEffectIdentityV1(targetId: string, probeValueIndex: number): string {
  return `sem_conditional_indirect_v1_${sha256HexUtf8V1(`${targetId}\0${probeValueIndex}`)}`;
}

function moderatedMediationIndexIdentityV1(targetId: string): string {
  return `sem_moderated_mediation_index_v1_${sha256HexUtf8V1(targetId)}`;
}

function generalSemF64BitsHex(value: number): string {
  const bytes = new ArrayBuffer(8);
  const view = new DataView(bytes);
  view.setFloat64(0, value, false);
  return view.getBigUint64(0, false).toString(16).padStart(16, "0");
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

function validateGeneralSemTrace(
  value: unknown,
  path: string,
  context: GeneralSemWireContext,
): CapabilityCellReferenceV2 {
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
  return cell;
}

function approximatelyEqualGeneralSem(left: number, right: number): boolean {
  return left === right
    || Math.abs(left - right) <= Number.EPSILON * 8 * Math.max(Math.abs(left), Math.abs(right), 1);
}

function canonicalGeneralSemEffectIdentitiesV1(
  specific: readonly unknown[],
  aggregate: readonly unknown[],
  interactions: readonly unknown[],
  conditionalIndirect: readonly unknown[],
  moderatedMediationIndices: readonly unknown[],
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
  for (let index = 0; index < interactions.length; index += 1) {
    const path = `general_sem_results.interaction_effects[${index}]`;
    const effect = strictWireRecord(interactions[index], path);
    identities.push({
      kind: "interaction_scientific_rescaled_gamma",
      effect_id: wireStableId(effect.effect_id, `${path}.effect_id`),
      interaction_id: wireStableId(effect.interaction_id, `${path}.interaction_id`),
      focal_relation_id: wireStableId(effect.focal_relation_id, `${path}.focal_relation_id`),
      interaction_effect_relation_id: wireStableId(
        effect.interaction_effect_relation_id,
        `${path}.interaction_effect_relation_id`,
      ),
      interaction_effect_parameter_id: wireStableId(
        effect.interaction_effect_parameter_id,
        `${path}.interaction_effect_parameter_id`,
      ),
      generated_product_column_id: wireStableId(
        effect.generated_product_column_id,
        `${path}.generated_product_column_id`,
      ),
      focal_predictor_id: wireStableId(effect.focal_predictor_id, `${path}.focal_predictor_id`),
      moderator_id: wireStableId(effect.moderator_id, `${path}.moderator_id`),
      outcome_id: wireStableId(effect.outcome_id, `${path}.outcome_id`),
      stage_one_model_scientific_sha256: wireGeneralSemSha256(
        effect.stage_one_model_scientific_sha256,
        `${path}.stage_one_model_scientific_sha256`,
      ),
      product_scale_version: wireStableId(effect.product_scale_version, `${path}.product_scale_version`),
      method_version: wireStableId(effect.method_version, `${path}.method_version`),
    });
  }
  for (let index = 0; index < conditionalIndirect.length; index += 1) {
    const path = `general_sem_results.conditional_indirect_effects[${index}]`;
    const effect = strictWireRecord(conditionalIndirect[index], path);
    identities.push({
      kind: "conditional_indirect",
      effect_id: wireStableId(effect.effect_id, `${path}.effect_id`),
      target_id: wireStableId(effect.target_id, `${path}.target_id`),
      estimand_id: wireStableId(effect.estimand_id, `${path}.estimand_id`),
      moderated_stage: wireEnum(
        effect.moderated_stage,
        ["first_stage", "second_stage"] as const,
        `${path}.moderated_stage`,
      ),
      interaction_id: wireStableId(effect.interaction_id, `${path}.interaction_id`),
      x_id: wireStableId(effect.x_id, `${path}.x_id`),
      mediator_id: wireStableId(effect.mediator_id, `${path}.mediator_id`),
      y_id: wireStableId(effect.y_id, `${path}.y_id`),
      moderator_id: wireStableId(effect.moderator_id, `${path}.moderator_id`),
      ordered_relation_ids: validateStableIdArray(
        effect.ordered_relation_ids,
        `${path}.ordered_relation_ids`,
        { minimum: 2 },
      ),
      probe_value_index: wireU32(effect.probe_value_index, `${path}.probe_value_index`),
      moderator_value_bits_hex: generalSemF64BitsHex(
        wireFinite(effect.moderator_value, `${path}.moderator_value`),
      ),
    });
  }
  for (let index = 0; index < moderatedMediationIndices.length; index += 1) {
    const path = `general_sem_results.moderated_mediation_indices[${index}]`;
    const effect = strictWireRecord(moderatedMediationIndices[index], path);
    identities.push({
      kind: "moderated_mediation_index",
      effect_id: wireStableId(effect.effect_id, `${path}.effect_id`),
      target_id: wireStableId(effect.target_id, `${path}.target_id`),
      estimand_id: wireStableId(effect.estimand_id, `${path}.estimand_id`),
      moderated_stage: wireEnum(
        effect.moderated_stage,
        ["first_stage", "second_stage"] as const,
        `${path}.moderated_stage`,
      ),
      interaction_id: wireStableId(effect.interaction_id, `${path}.interaction_id`),
      x_id: wireStableId(effect.x_id, `${path}.x_id`),
      mediator_id: wireStableId(effect.mediator_id, `${path}.mediator_id`),
      y_id: wireStableId(effect.y_id, `${path}.y_id`),
      moderator_id: wireStableId(effect.moderator_id, `${path}.moderator_id`),
      ordered_relation_ids: validateStableIdArray(
        effect.ordered_relation_ids,
        `${path}.ordered_relation_ids`,
        { minimum: 2 },
      ),
    });
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

function validateThreeWayModerationResultsV1(
  interactionValues: readonly unknown[],
  conditionalValues: readonly unknown[],
  slopeValues: readonly unknown[],
  receiptValue: unknown,
  context: GeneralSemWireContext,
): void {
  const pointCellIdentity = capabilityCellIdentity(
    GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1,
  );
  const interactions = interactionValues.map((item, index) => {
    const path = `general_sem_results.three_way_interaction_effects[${index}]`;
    const effect = exactWireRecord(item, [
      "effect_id", "trace", "interaction_id", "focal_relation_id",
      "interaction_effect_relation_id", "interaction_effect_parameter_id", "operand_ids",
      "outcome_id", "generated_product_column_id", "stage_one_model_scientific_sha256",
      "method_version", "product_scale_version", "hierarchy_policy", "hierarchy_policy_version",
      "observation_count", "unstandardized_product_mean",
      "unstandardized_product_sample_standard_deviation", "standardized_product_coefficient",
      "scientific_rescaled_delta",
    ], [], path);
    const effectId = wireStableId(effect.effect_id, `${path}.effect_id`);
    const trace = validateGeneralSemTrace(effect.trace, `${path}.trace`, context);
    if (capabilityCellIdentity(trace) !== pointCellIdentity) {
      wireFail("document.invalid", `${path}.trace.capability_cell`, `${path} must use the exact three-way point cell.`);
    }
    const interactionId = wireStableId(effect.interaction_id, `${path}.interaction_id`);
    if (effectId !== `three_way_delta:${interactionId}`) {
      wireFail("document.invalid", `${path}.effect_id`, `${path}.effect_id must be the canonical three-way delta target.`);
    }
    const focalRelationId = wireStableId(effect.focal_relation_id, `${path}.focal_relation_id`);
    for (const field of ["interaction_effect_relation_id", "interaction_effect_parameter_id", "outcome_id", "generated_product_column_id"] as const) {
      wireStableId(effect[field], `${path}.${field}`);
    }
    const operands = validateStableIdArray(effect.operand_ids, `${path}.operand_ids`, { minimum: 3 });
    if (operands.length !== 3 || new Set(operands).size !== 3) {
      wireFail("document.invalid", `${path}.operand_ids`, `${path}.operand_ids must contain exactly three unique IDs in authored order.`);
    }
    if (operands.includes(effect.outcome_id as string)) {
      wireFail("document.invalid", `${path}.outcome_id`, `${path}.outcome_id must be distinct from all interaction operands.`);
    }
    wireGeneralSemSha256(effect.stage_one_model_scientific_sha256, `${path}.stage_one_model_scientific_sha256`);
    if (effect.method_version !== GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1) {
      wireFail("document.invalid", `${path}.method_version`, `${path}.method_version does not identify the bounded three-way point estimator.`);
    }
    if (effect.product_scale_version !== GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1) {
      wireFail("document.invalid", `${path}.product_scale_version`, `${path}.product_scale_version is not the qualified two-stage product scale.`);
    }
    if (effect.hierarchy_policy !== "strong"
      || effect.hierarchy_policy_version !== GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1) {
      wireFail("document.invalid", `${path}.hierarchy_policy`, `${path} requires the exact strong-hierarchy policy.`);
    }
    if (wireU32(effect.observation_count, `${path}.observation_count`) < 2) {
      wireFail("document.invalid", `${path}.observation_count`, `${path}.observation_count must be at least two.`);
    }
    wireFinite(effect.unstandardized_product_mean, `${path}.unstandardized_product_mean`);
    if (wireFinite(effect.unstandardized_product_sample_standard_deviation, `${path}.unstandardized_product_sample_standard_deviation`) <= 0) {
      wireFail("document.invalid", `${path}.unstandardized_product_sample_standard_deviation`, `${path} requires a positive product standard deviation.`);
    }
    validateGeneralSemEstimate(effect.standardized_product_coefficient, `${path}.standardized_product_coefficient`);
    validateGeneralSemEstimate(effect.scientific_rescaled_delta, `${path}.scientific_rescaled_delta`);
    const standardized = strictWireRecord(effect.standardized_product_coefficient, `${path}.standardized_product_coefficient`);
    const delta = strictWireRecord(effect.scientific_rescaled_delta, `${path}.scientific_rescaled_delta`);
    if (!approximatelyEqualGeneralSem(
      wireFinite(standardized.estimate, `${path}.standardized_product_coefficient.estimate`)
        / (effect.unstandardized_product_sample_standard_deviation as number),
      wireFinite(delta.estimate, `${path}.scientific_rescaled_delta.estimate`),
    )) {
      wireFail("document.invalid", `${path}.scientific_rescaled_delta.estimate`, `${path}.scientific_rescaled_delta must equal the standardized-product coefficient divided by product SD.`);
    }
    return {
      effectId,
      interactionId,
      focalRelationId,
      operands,
      outcomeId: effect.outcome_id as string,
      stageOneModelScientificSha256: effect.stage_one_model_scientific_sha256 as string,
      estimate: effect.scientific_rescaled_delta,
    };
  });

  if (interactions.length > 1) {
    wireFail("document.invalid", "general_sem_results.three_way_interaction_effects", "The bounded v1 cell supports exactly one three-way interaction per model.");
  }
  const authority = interactions[0];
  if (!authority && (conditionalValues.length || slopeValues.length || receiptValue != null)) {
    wireFail("document.invalid", "general_sem_results.three_way_interaction_effects", "Three-way conditional results require one authoritative interaction-effect row.");
  }
  const ensureAuthority = (record: StrictWireRecord, path: string) => {
    if (!authority) return;
    if (record.interaction_id !== authority.interactionId
      || record.focal_relation_id !== authority.focalRelationId
      || record.first_moderator_id !== authority.operands[1]
      || record.second_moderator_id !== authority.operands[2]) {
      wireFail("document.invalid", path, `${path} contradicts the authoritative three-way interaction.`);
    }
    const trace = validateGeneralSemTrace(record.trace, `${path}.trace`, context);
    if (capabilityCellIdentity(trace) !== pointCellIdentity) {
      wireFail("document.invalid", `${path}.trace.capability_cell`, `${path} must use the exact three-way point cell.`);
    }
  };
  const probeValue = (
    kindValue: unknown,
    probeIndex: number,
    actualValue: unknown,
    path: string,
  ): CanonicalThreeWayModeratorProbeKindV1 => {
    const kind = wireEnum(
      kindValue,
      ["continuous_standardized", "binary_zero_one"] as const,
      `${path}_probe_kind`,
    );
    const expected = kind === "continuous_standardized"
      ? [-1, 0, 1][probeIndex]
      : [0, 1][probeIndex];
    const actual = wireFinite(actualValue, `${path}_value`);
    if (expected === undefined || !Object.is(actual, expected)) {
      wireFail("document.invalid", `${path}_value`, `${path} contradicts the fixed ${kind} probe grid.`);
    }
    return kind;
  };

  const conditionalIndices = new Set<number>();
  const conditionalKinds = new Set<CanonicalThreeWayModeratorProbeKindV1>();
  const conditional = conditionalValues.map((item, index) => {
    const path = `general_sem_results.three_way_conditional_interaction_effects[${index}]`;
    const effect = exactWireRecord(item, [
      "effect_id", "trace", "interaction_id", "focal_relation_id", "first_moderator_id",
      "second_moderator_id", "second_moderator_probe_kind", "second_moderator_probe_index",
      "second_moderator_value", "value",
    ], [], path);
    const effectId = wireStableId(effect.effect_id, `${path}.effect_id`);
    for (const field of ["interaction_id", "focal_relation_id", "first_moderator_id", "second_moderator_id"] as const) {
      wireStableId(effect[field], `${path}.${field}`);
    }
    ensureAuthority(effect, path);
    const probeIndex = wireU32(effect.second_moderator_probe_index, `${path}.second_moderator_probe_index`);
    if (probeIndex > 2 || conditionalIndices.has(probeIndex)) {
      wireFail("document.invalid", `${path}.second_moderator_probe_index`, `${path} uses a duplicate or unsupported probe index.`);
    }
    conditionalIndices.add(probeIndex);
    conditionalKinds.add(probeValue(
      effect.second_moderator_probe_kind,
      probeIndex,
      effect.second_moderator_value,
      `${path}.second_moderator`,
    ));
    validateGeneralSemEstimate(effect.value, `${path}.value`);
    return { effectId, estimate: effect.value };
  });
  const conditionalGridSize = conditionalKinds.has("continuous_standardized") ? 3 : 2;
  if (authority && (conditionalKinds.size !== 1
    || conditional.length !== conditionalGridSize
    || [...conditionalIndices].some((probeIndex) => probeIndex >= conditional.length))) {
    wireFail("document.invalid", "general_sem_results.three_way_conditional_interaction_effects", "Three-way conditional interaction effects must form one contiguous two- or three-probe grid.");
  }

  const slopePairs = new Set<string>();
  const firstProbeIndices = new Set<number>();
  const secondProbeIndices = new Set<number>();
  const firstProbeKinds = new Set<CanonicalThreeWayModeratorProbeKindV1>();
  const secondProbeKinds = new Set<CanonicalThreeWayModeratorProbeKindV1>();
  const slopes = slopeValues.map((item, index) => {
    const path = `general_sem_results.three_way_simple_slopes[${index}]`;
    const effect = exactWireRecord(item, [
      "effect_id", "trace", "interaction_id", "focal_relation_id", "first_moderator_id",
      "second_moderator_id", "first_moderator_probe_kind", "second_moderator_probe_kind",
      "first_probe_index", "first_moderator_value", "second_probe_index", "second_moderator_value",
      "value",
    ], [], path);
    const effectId = wireStableId(effect.effect_id, `${path}.effect_id`);
    for (const field of ["interaction_id", "focal_relation_id", "first_moderator_id", "second_moderator_id"] as const) {
      wireStableId(effect[field], `${path}.${field}`);
    }
    ensureAuthority(effect, path);
    const firstProbe = wireU32(effect.first_probe_index, `${path}.first_probe_index`);
    const secondProbe = wireU32(effect.second_probe_index, `${path}.second_probe_index`);
    if (firstProbe > 2 || secondProbe > 2 || slopePairs.has(`${firstProbe}:${secondProbe}`)) {
      wireFail("document.invalid", path, `${path} uses a duplicate or unsupported moderator-probe pair.`);
    }
    slopePairs.add(`${firstProbe}:${secondProbe}`);
    firstProbeIndices.add(firstProbe);
    secondProbeIndices.add(secondProbe);
    firstProbeKinds.add(probeValue(
      effect.first_moderator_probe_kind,
      firstProbe,
      effect.first_moderator_value,
      `${path}.first_moderator`,
    ));
    secondProbeKinds.add(probeValue(
      effect.second_moderator_probe_kind,
      secondProbe,
      effect.second_moderator_value,
      `${path}.second_moderator`,
    ));
    validateGeneralSemEstimate(effect.value, `${path}.value`);
    return { effectId, estimate: effect.value };
  });
  const firstGridSize = firstProbeKinds.has("continuous_standardized") ? 3 : 2;
  const secondGridSize = secondProbeKinds.has("continuous_standardized") ? 3 : 2;
  if (authority && (firstProbeKinds.size !== 1 || secondProbeKinds.size !== 1
    || firstProbeIndices.size !== firstGridSize
    || secondProbeIndices.size !== secondGridSize
    || slopes.length !== firstProbeIndices.size * secondProbeIndices.size
    || [...firstProbeIndices].some((probeIndex) => probeIndex >= firstProbeIndices.size)
    || [...secondProbeIndices].some((probeIndex) => probeIndex >= secondProbeIndices.size))) {
    wireFail("document.invalid", "general_sem_results.three_way_simple_slopes", "Three-way simple slopes must form one complete 2x2, 2x3, 3x2, or 3x3 probe grid.");
  }
  if (authority && [...conditionalKinds][0] !== [...secondProbeKinds][0]) {
    wireFail("document.invalid", "general_sem_results.three_way_simple_slopes", "Three-way conditional effects and simple slopes must share the same second-moderator probe kind.");
  }

  const publishedEffectIds = [
    ...interactions.map((item) => item.effectId),
    ...conditional.map((item) => item.effectId),
    ...slopes.map((item) => item.effectId),
  ];
  if (new Set(publishedEffectIds).size !== publishedEffectIds.length) {
    wireFail("document.invalid", "general_sem_results", "Three-way target IDs must be unique across delta, conditional-interaction, and simple-slope sections.");
  }

  const allTargets = [
    ...(authority ? [authority.estimate] : []),
    ...conditional.map((item) => item.estimate),
    ...slopes.map((item) => item.estimate),
  ];
  const anyInference = allTargets.some(generalSemEstimateHasInference);
  if (receiptValue == null) {
    if (anyInference) wireFail("document.invalid", "general_sem_results.three_way_moderation_bootstrap_receipt", "Three-way bootstrap fields require the shared-ledger receipt.");
    return;
  }
  if (!allTargets.length || allTargets.some((estimate) => !generalSemEstimateHasInference(estimate))) {
    wireFail("document.invalid", "general_sem_results.three_way_moderation_bootstrap_receipt", "A three-way bootstrap receipt requires inference on every published target.");
  }
  const path = "general_sem_results.three_way_moderation_bootstrap_receipt";
  const receipt = exactWireRecord(receiptValue, [
    "capability_cell", "capability_dependencies", "method_version", "point_method_version",
    "resampling_operation_version",
    "resampling_stream_version", "quantile_method_version", "standard_error_method_version",
    "summation_method_version", "p_value_method_version", "failure_policy_version",
    "sign_alignment_method_version", "product_scale_version", "probe_policy_version",
    "compiled_plan_sha256", "general_sem_config_sha256", "model_scientific_sha256",
    "stage_one_model_scientific_sha256", "source_dataset_fingerprint", "complete_case_frame_sha256",
    "usable_replicate_indices_sha256", "target_identity_set_sha256", "target_ids",
    "interval", "tail", "confidence_level", "resamples_requested", "resamples_usable",
    "minimum_usable_resamples", "seed", "workers", "complete_model_reestimated_per_replicate",
    "shared_stage_one_reestimated_per_replicate", "score_vectors_sign_aligned_before_products",
    "all_lower_order_and_three_way_products_recomputed_per_replicate",
    "joint_stage_two_reestimated_per_replicate",
    "complete_joint_point_contract_validated_per_replicate",
    "failed_replicates", "all_three_way_targets_share_one_replicate_ledger",
  ], [], path);
  const receiptCell = validateWireCapabilityCell(receipt.capability_cell, `${path}.capability_cell`);
  const receiptIdentity = capabilityCellIdentity(receiptCell);
  if (receiptIdentity !== capabilityCellIdentity(GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1)
    || !context.capabilityIds.has(receiptIdentity)) {
    wireFail("document.invalid", `${path}.capability_cell`, `${path} must use the declared exact three-way bootstrap cell.`);
  }
  const exactVersions = [
    ["method_version", GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1],
    ["point_method_version", GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1],
    ["resampling_operation_version", GENERAL_SEM_PLS_THREE_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1],
    ["resampling_stream_version", GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1],
    ["quantile_method_version", GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1],
    ["standard_error_method_version", GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1],
    ["summation_method_version", GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1],
    ["p_value_method_version", GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1],
    ["failure_policy_version", GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1],
    ["sign_alignment_method_version", GENERAL_SEM_PLS_MULTIPLE_MODERATION_SIGN_ALIGNMENT_VERSION_V1],
    ["product_scale_version", GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1],
    ["probe_policy_version", GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1],
    ["interval", "percentile_type7"],
    ["tail", "two_sided"],
  ] as const;
  for (const [field, expected] of exactVersions) {
    if (receipt[field] !== expected) wireFail("document.invalid", `${path}.${field}`, `${path}.${field} must equal ${expected}.`);
  }
  if (receipt.all_three_way_targets_share_one_replicate_ledger !== true
    || receipt.complete_model_reestimated_per_replicate !== true
    || receipt.shared_stage_one_reestimated_per_replicate !== true
    || receipt.score_vectors_sign_aligned_before_products !== true
    || receipt.all_lower_order_and_three_way_products_recomputed_per_replicate !== true
    || receipt.joint_stage_two_reestimated_per_replicate !== true
    || receipt.complete_joint_point_contract_validated_per_replicate !== true) {
    wireFail("document.invalid", path, `${path} must prove one shared full-model replicate ledger.`);
  }
  for (const field of [
    "compiled_plan_sha256", "general_sem_config_sha256", "model_scientific_sha256",
    "stage_one_model_scientific_sha256", "complete_case_frame_sha256",
    "usable_replicate_indices_sha256", "target_identity_set_sha256",
  ] as const) wireGeneralSemSha256(receipt[field], `${path}.${field}`);
  if (receipt.model_scientific_sha256 !== context.modelDigest
    || receipt.stage_one_model_scientific_sha256 !== authority?.stageOneModelScientificSha256
    || wireGeneralSemDatasetFingerprint(receipt.source_dataset_fingerprint, `${path}.source_dataset_fingerprint`) !== context.datasetFingerprint) {
    wireFail("document.invalid", path, `${path} provenance digests must match the result document.`);
  }
  const expectedEffectIds = [
    ...interactions.map((item) => item.effectId),
    ...conditional.map((item) => item.effectId),
    ...slopes.map((item) => item.effectId),
  ].sort();
  const targetIds = validateStableIdArray(receipt.target_ids, `${path}.target_ids`, { minimum: 1, canonical: true });
  if (targetIds.length !== expectedEffectIds.length
    || targetIds.some((effectId, index) => effectId !== expectedEffectIds[index])) {
    wireFail("document.invalid", `${path}.target_ids`, `${path}.target_ids must exactly cover delta, conditional-interaction, and simple-slope targets.`);
  }
  if (receipt.target_identity_set_sha256 !== generalSemSerializedSha256(targetIds)) {
    wireFail("document.invalid", `${path}.target_identity_set_sha256`, `${path}.target_identity_set_sha256 must bind the ordered target IDs.`);
  }
  const confidence = wireFinite(receipt.confidence_level, `${path}.confidence_level`);
  if (confidence <= 0 || confidence >= 1) wireFail("document.invalid", `${path}.confidence_level`, `${path}.confidence_level must be between zero and one.`);
  const requested = wireU32(receipt.resamples_requested, `${path}.resamples_requested`);
  const usable = wireU32(receipt.resamples_usable, `${path}.resamples_usable`);
  const minimum = wireU32(receipt.minimum_usable_resamples, `${path}.minimum_usable_resamples`);
  if (requested < 2 || requested > 10_000 || minimum !== Math.max(2, Math.ceil(requested * 0.9))
    || usable < minimum || usable > requested) {
    wireFail("document.invalid", path, `${path} bootstrap counts violate the bounded 90 percent usable-replicate contract.`);
  }
  const receiptWorkers = wireU32(receipt.workers, `${path}.workers`);
  if (wireGeneralSemDecimalSafeSeed(receipt.seed, `${path}.seed`) !== String(context.seed)
    || receiptWorkers < 1 || receiptWorkers > 64 || receiptWorkers !== context.workers) {
    wireFail("document.invalid", path, `${path} seed and workers must match provenance.`);
  }
  const dependencies = wireArray(receipt.capability_dependencies, `${path}.capability_dependencies`).map((item, index) => (
    capabilityCellIdentity(validateWireCapabilityCell(item, `${path}.capability_dependencies[${index}]`))
  ));
  const expectedDependencies = [
    GENERAL_SEM_PLS_BASE_CAPABILITY_CELL_V1,
    GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1,
  ].map(capabilityCellIdentity).sort();
  if (dependencies.length !== expectedDependencies.length
    || dependencies.some((identity, index) => identity !== expectedDependencies[index])
    || dependencies.some((identity) => !context.capabilityIds.has(identity))) {
    wireFail("document.invalid", `${path}.capability_dependencies`, `${path}.capability_dependencies must exactly declare the base PLS and three-way point cells.`);
  }
  const failures = wireArray(receipt.failed_replicates, `${path}.failed_replicates`);
  if (usable + failures.length !== requested) wireFail("document.invalid", path, `${path} usable plus failed counts must equal requested resamples.`);
  const failedIndices = new Set<number>();
  let previous = -1;
  failures.forEach((item, index) => {
    const failurePath = `${path}.failed_replicates[${index}]`;
    const failure = exactWireRecord(item, ["replicate_index", "reason_code", "message"], [], failurePath);
    const replicateIndex = wireU32(failure.replicate_index, `${failurePath}.replicate_index`);
    if (replicateIndex <= previous || replicateIndex >= requested) wireFail("document.invalid", `${failurePath}.replicate_index`, `${path}.failed_replicates must be ordered and in range.`);
    previous = replicateIndex;
    failedIndices.add(replicateIndex);
    wireEnum(failure.reason_code, [
      "insufficient_observations", "constant_indicator", "stage_one_rank_deficient",
      "stage_one_nonconvergence", "indeterminate_score_sign", "constant_construct_score",
      "constant_interaction_product", "rank_deficient", "joint_stage_rank_deficient",
      "isolated_construct", "estimation_nonconvergence", "target_inventory_mismatch",
      "numerical_failure",
    ] as const, `${failurePath}.reason_code`);
    wireText(failure.message, `${failurePath}.message`);
  });
  const usableIndices = Array.from({ length: requested }, (_, index) => index)
    .filter((index) => !failedIndices.has(index));
  if (receipt.usable_replicate_indices_sha256 !== generalSemSerializedSha256(usableIndices)) {
    wireFail("document.invalid", `${path}.usable_replicate_indices_sha256`, `${path}.usable_replicate_indices_sha256 contradicts the failure ledger.`);
  }
  for (const estimateValue of allTargets) {
    const estimate = strictWireRecord(estimateValue, path);
    const exceedances = wireU32(
      estimate.bootstrap_two_sided_exceedances,
      `${path}.target.bootstrap_two_sided_exceedances`,
    );
    const pValue = wireFinite(estimate.p_value, `${path}.target.p_value`);
    if (estimate.bootstrap_usable_replicates !== usable
      || exceedances > usable
      || !approximatelyEqualGeneralSem(pValue, (exceedances + 1) / (usable + 1))) {
      wireFail("document.invalid", path, `${path} must use the same usable replicate ledger for every target.`);
    }
  }
}

function validateGeneralSemInferenceReceiptV1(
  receiptValue: unknown,
  hocReceiptValue: unknown,
  specific: readonly unknown[],
  aggregate: readonly unknown[],
  jointStage: readonly unknown[],
  interactions: readonly unknown[],
  conditional: readonly unknown[],
  conditionalIndirect: readonly unknown[],
  moderatedMediationIndices: readonly unknown[],
  plots: readonly unknown[],
  hocStages: readonly unknown[],
  context: GeneralSemWireContext,
): void {
  const mediationEffects = [
    ...specific.map((effect, index) => ({
      path: `general_sem_results.specific_indirect_effects[${index}]`,
      effect: strictWireRecord(effect, `general_sem_results.specific_indirect_effects[${index}]`),
    })),
    ...aggregate.map((effect, index) => ({
      path: `general_sem_results.aggregate_effects[${index}]`,
      effect: strictWireRecord(effect, `general_sem_results.aggregate_effects[${index}]`),
    })),
  ];
  const moderationEffects = interactions.map((effect, index) => ({
    path: `general_sem_results.interaction_effects[${index}]`,
    effect: strictWireRecord(effect, `general_sem_results.interaction_effects[${index}]`),
  }));
  const moderatedMediationDerivedEffects = [
    ...conditionalIndirect.map((effect, index) => ({
      path: `general_sem_results.conditional_indirect_effects[${index}]`,
      effect: strictWireRecord(
        effect,
        `general_sem_results.conditional_indirect_effects[${index}]`,
      ),
    })),
    ...moderatedMediationIndices.map((effect, index) => ({
      path: `general_sem_results.moderated_mediation_indices[${index}]`,
      effect: strictWireRecord(
        effect,
        `general_sem_results.moderated_mediation_indices[${index}]`,
      ),
    })),
  ];
  const mediationEffectValues = mediationEffects.map(({ effect }) => effect.value);
  const moderationEffectValues = moderationEffects.map(({ effect }) => effect.scientific_rescaled_gamma);
  const moderatedMediationDerivedEffectValues = moderatedMediationDerivedEffects
    .map(({ effect }) => effect.value);
  const standardizedProductInference = interactions.some((effect, index) => {
    const record = strictWireRecord(effect, `general_sem_results.interaction_effects[${index}]`);
    return generalSemEstimateHasInference(record.standardized_product_coefficient);
  });
  const jointStageInference = jointStage.some((coefficient, index) => generalSemEstimateHasInference(
    strictWireRecord(
      coefficient,
      `general_sem_results.joint_stage_structural_coefficients[${index}]`,
    ).estimate,
  ));
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
  const interactionPlotIntervals = plots.some((plot, plotIndex) => {
    const record = strictWireRecord(plot, `general_sem_results.interaction_plots[${plotIndex}]`);
    return Array.isArray(record.series) && record.series.some((series, seriesIndex) => {
      const seriesRecord = strictWireRecord(
        series,
        `general_sem_results.interaction_plots[${plotIndex}].series[${seriesIndex}]`,
      );
      return Array.isArray(seriesRecord.points) && seriesRecord.points.some((point, pointIndex) => {
        const pointRecord = strictWireRecord(
          point,
          `general_sem_results.interaction_plots[${plotIndex}].series[${seriesIndex}].points[${pointIndex}]`,
        );
        return pointRecord.lower != null || pointRecord.upper != null;
      });
    });
  });

  if (receiptValue == null) {
    if (mediationEffectValues.some(generalSemEstimateHasInference)
      || moderationEffectValues.some(generalSemEstimateHasInference)
      || moderatedMediationDerivedEffectValues.some(generalSemEstimateHasInference)
      || standardizedProductInference
      || jointStageInference
      || conditionalInference) {
      wireFail(
        "document.invalid",
        "general_sem_results.inference_receipt",
        "general_sem_results inference fields require inference_receipt.",
      );
    }
    if (higherOrderInference && hocReceiptValue == null) {
      wireFail(
        "document.invalid",
        "general_sem_results.higher_order_inference_receipt",
        "higher-order inference fields require higher_order_inference_receipt.",
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
  ], ["capability_dependencies"], path);

  wireEnum(receipt.kind, ["case_bootstrap"] as const, `${path}.kind`);
  const capabilityCell = validateWireCapabilityCell(receipt.capability_cell, `${path}.capability_cell`);
  const capabilityIdentity = capabilityCellIdentity(capabilityCell);
  const singleMediationBootstrap = capabilityIdentity
    === capabilityCellIdentity(GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_CAPABILITY_CELL_V1);
  const mediationBootstrap = singleMediationBootstrap || capabilityIdentity
    === capabilityCellIdentity(GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1);
  const moderationBootstrap = capabilityIdentity
    === capabilityCellIdentity(GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1);
  const moderatedMediationBootstrap = capabilityIdentity
    === capabilityCellIdentity(
      GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CAPABILITY_CELL_V1,
    );
  if (!mediationBootstrap && !moderationBootstrap && !moderatedMediationBootstrap) {
    wireFail(
      "document.invalid",
      `${path}.capability_cell`,
      `${path}.capability_cell must equal an exact General SEM full-model case-bootstrap option cell.`,
    );
  }
  if (moderatedMediationBootstrap
    && (interactions.length !== 1
      || conditionalIndirect.length !== 3
      || moderatedMediationIndices.length !== 1)) {
    wireFail(
      "document.invalid",
      path,
      `${path} moderated-mediation bootstrap requires one gamma, three locked conditional indirect effects, and one index.`,
    );
  }
  if (!context.capabilityIds.has(capabilityIdentity)) {
    wireFail(
      "document.invalid",
      `${path}.capability_cell`,
      `${path}.capability_cell references an undeclared option cell.`,
    );
  }
  if (moderationBootstrap && (specific.length > 0 || aggregate.length > 0)) {
    wireFail(
      "document.invalid",
      path,
      `${path} moderation bootstrap must not contain mediation effect rows.`,
    );
  }
  if (moderatedMediationBootstrap && (specific.length > 0 || aggregate.length > 0)) {
    wireFail(
      "document.invalid",
      path,
      `${path} moderated-mediation bootstrap must not contain ordinary mediation effect rows.`,
    );
  }
  const dependencies = optionalWireArray(receipt, "capability_dependencies", path)
    .map((dependency, index) => validateWireCapabilityCell(
      dependency,
      `${path}.capability_dependencies[${index}]`,
    ));
  const dependencyIdentities = dependencies.map(capabilityCellIdentity);
  if (new Set(dependencyIdentities).size !== dependencyIdentities.length
    || dependencyIdentities.some((identity, index) => (
      index > 0 && dependencyIdentities[index - 1]! >= identity
    ))) {
    wireFail(
      "document.invalid",
      `${path}.capability_dependencies`,
      `${path}.capability_dependencies must be distinct and ordered by exact capability identity.`,
    );
  }
  if (moderatedMediationBootstrap) {
    const expectedDependencies = [
      GENERAL_SEM_PLS_BASE_CAPABILITY_CELL_V1,
      GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_CAPABILITY_CELL_V1,
    ].map(capabilityCellIdentity).sort();
    if (dependencyIdentities.length !== expectedDependencies.length
      || dependencyIdentities.some((identity, index) => identity !== expectedDependencies[index])) {
      wireFail(
        "document.invalid",
        `${path}.capability_dependencies`,
        `${path}.capability_dependencies must exactly declare the base PLS and moderation-point cells.`,
      );
    }
    for (const identity of dependencyIdentities) {
      if (!context.capabilityIds.has(identity)) {
        wireFail(
          "document.invalid",
          `${path}.capability_dependencies`,
          `${path}.capability_dependencies references an undeclared option cell.`,
        );
      }
    }
  } else if (dependencies.length > 0) {
    wireFail(
      "document.invalid",
      `${path}.capability_dependencies`,
      `${path}.capability_dependencies must be empty for single-owner v1 bootstrap receipts.`,
    );
  }
  const moderatedMediationEffects = [
    ...moderationEffects,
    ...moderatedMediationDerivedEffects,
  ];
  const moderatedMediationEffectValues = [
    ...moderationEffectValues,
    ...moderatedMediationDerivedEffectValues,
  ];
  const coveredEffects = moderatedMediationBootstrap
    ? moderatedMediationEffects
    : moderationBootstrap
      ? moderationEffects
      : mediationEffects;
  const coveredEffectValues = moderatedMediationBootstrap
    ? moderatedMediationEffectValues
    : moderationBootstrap
      ? moderationEffectValues
      : mediationEffectValues;

  const versions = [
    [
      "method_version",
      moderatedMediationBootstrap
        ? GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1
        : moderationBootstrap
          ? GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
          : singleMediationBootstrap
            ? GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1
            : GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1,
    ],
    [
      "resampling_operation_version",
      moderatedMediationBootstrap
        ? GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1
        : moderationBootstrap
          ? GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1
          : singleMediationBootstrap
            ? GENERAL_SEM_PLS_SINGLE_MEDIATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1
            : GENERAL_SEM_PLS_CASE_BOOTSTRAP_OPERATION_VERSION_V1,
    ],
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
  const expectedEffectIds = coveredEffects
    .map(({ effect, path: effectPath }) => wireStableId(
      effect.effect_id,
      `${effectPath}.effect_id`,
    ))
    .sort();
  if (effectIds.length !== expectedEffectIds.length
    || effectIds.some((effectId, index) => effectId !== expectedEffectIds[index])) {
    wireFail(
      "document.invalid",
      `${path}.effect_ids`,
      moderatedMediationBootstrap
        ? `${path}.effect_ids must exactly cover one scientific gamma, three conditional indirect effects, and one moderated-mediation index.`
        : moderationBootstrap
          ? `${path}.effect_ids must exactly cover scientific rescaled gamma interaction rows.`
          : `${path}.effect_ids must exactly cover specific and aggregate effect rows.`,
    );
  }
  const effectIdentities = canonicalGeneralSemEffectIdentitiesV1(
    specific,
    aggregate,
    interactions,
    conditionalIndirect,
    moderatedMediationIndices,
  ).filter((identity) => {
    if (moderatedMediationBootstrap) {
      return identity.kind === "interaction_scientific_rescaled_gamma"
        || identity.kind === "conditional_indirect"
        || identity.kind === "moderated_mediation_index";
    }
    if (moderationBootstrap) return identity.kind === "interaction_scientific_rescaled_gamma";
    return identity.kind !== "interaction_scientific_rescaled_gamma"
      && identity.kind !== "conditional_indirect"
      && identity.kind !== "moderated_mediation_index";
  });
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
      "stage_one_rank_deficient",
      "stage_one_nonconvergence",
      "indeterminate_score_sign",
      "constant_construct_score",
      "constant_interaction_product",
      "rank_deficient",
      "joint_stage_rank_deficient",
      "isolated_construct",
      "estimation_nonconvergence",
      "target_inventory_mismatch",
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
      moderatedMediationBootstrap
        ? `${path} requires complete inference fields for gamma, all three conditional indirect effects, and the moderated-mediation index.`
        : moderationBootstrap
          ? `${path} requires complete inference fields for every scientific rescaled gamma interaction effect.`
          : `${path} requires complete inference fields for every covered effect.`,
    );
  }
  const pointOnlyInference = standardizedProductInference
    || jointStageInference
    || conditionalInference
    || higherOrderInference;
  const uncoveredInference = pointOnlyInference
    || ((moderationBootstrap || moderatedMediationBootstrap) && interactionPlotIntervals)
    || (moderatedMediationBootstrap
      ? mediationEffectValues.some(generalSemEstimateHasInference)
      : moderationBootstrap
        ? mediationEffectValues.some(generalSemEstimateHasInference)
          || moderatedMediationDerivedEffectValues.some(generalSemEstimateHasInference)
        : moderationEffectValues.some(generalSemEstimateHasInference)
          || moderatedMediationDerivedEffectValues.some(generalSemEstimateHasInference));
  if (uncoveredInference) {
    wireFail(
      "document.invalid",
      path,
      moderatedMediationBootstrap
        ? `${path} moderated-mediation v1 permits inference only for gamma, the three locked conditional indirect effects, and the index.`
        : moderationBootstrap
          ? `${path} moderation v1 permits inference only for scientific_rescaled_gamma; standardized-product, joint-stage, conditional, plot, mediation, and higher-order estimates must remain point-only.`
          : `${path} v1 does not cover interaction, conditional, moderated-mediation, or higher-order estimate inference.`,
    );
  }
  for (const { path: effectPath, effect } of coveredEffects) {
    const effectId = wireStableId(effect.effect_id, `${effectPath}.effect_id`);
    const combinedDerived = moderatedMediationBootstrap
      && !effectPath.startsWith("general_sem_results.interaction_effects[");
    const expectedEffectCapability = capabilityCellIdentity(
      combinedDerived
        ? GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CAPABILITY_CELL_V1
        : moderationBootstrap || moderatedMediationBootstrap
          ? GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_CAPABILITY_CELL_V1
          : GENERAL_SEM_PLS_RECURSIVE_EFFECTS_CAPABILITY_CELL_V1,
    );
    const trace = strictWireRecord(effect.trace, `${effectPath}.trace`);
    const traceCapability = validateWireCapabilityCell(
      trace.capability_cell,
      `${effectPath}.trace.capability_cell`,
    );
    if (capabilityCellIdentity(traceCapability) !== expectedEffectCapability) {
      wireFail(
        "document.invalid",
        `${effectPath}.trace.capability_cell`,
        combinedDerived
          ? `${path} effect ${effectId} trace.capability_cell must equal the supplemental two-way moderated-mediation cell.`
          : moderationBootstrap || moderatedMediationBootstrap
            ? `${path} effect ${effectId} trace.capability_cell must equal the General SEM multiple two-way moderation point option cell.`
            : `${path} effect ${effectId} trace.capability_cell must equal the PLS recursive-effects option cell.`,
      );
    }
    const gammaValue = (moderationBootstrap || moderatedMediationBootstrap) && !combinedDerived;
    const valuePath = gammaValue
      ? `${effectPath}.scientific_rescaled_gamma`
      : `${effectPath}.value`;
    const value = strictWireRecord(
      gammaValue ? effect.scientific_rescaled_gamma : effect.value,
      valuePath,
    );
    const usable = optionalWireU32(value, "bootstrap_usable_replicates", valuePath);
    const exceedances = optionalWireU32(value, "bootstrap_two_sided_exceedances", valuePath);
    if (usable == null || exceedances == null) continue;
    if (usable !== resamplesUsable) {
      wireFail(
        "document.invalid",
        `${valuePath}.bootstrap_usable_replicates`,
        `${path} effect ${effectId} usable replicate count contradicts the receipt.`,
      );
    }
    if (exceedances > usable) {
      wireFail(
        "document.invalid",
        `${valuePath}.bootstrap_two_sided_exceedances`,
        `${path} effect ${effectId} exceedance count exceeds usable replicates.`,
      );
    }
    const pValue = optionalWireFinite(value, "p_value", valuePath);
    const expectedPValue = (exceedances + 1) / (usable + 1);
    if (pValue != null && !approximatelyEqualGeneralSem(pValue, expectedPValue)) {
      wireFail(
        "document.invalid",
        `${valuePath}.p_value`,
        `${path} effect ${effectId} p_value contradicts the plus-one exceedance ledger.`,
      );
    }
  }
}

function validateHocInferenceReceiptV1(
  receiptValue: unknown,
  ordinaryReceiptValue: unknown,
  hocStages: readonly unknown[],
  context: GeneralSemWireContext,
): void {
  const relations = new Map<string, { relation: StrictWireRecord; path: string }>();
  hocStages.forEach((stageValue, stageIndex) => {
    const stagePath = `general_sem_results.higher_order_stages[${stageIndex}]`;
    const stage = strictWireRecord(stageValue, stagePath);
    optionalWireArray(stage, "relation_estimates", stagePath).forEach((relationValue, relationIndex) => {
      const relationPath = `${stagePath}.relation_estimates[${relationIndex}]`;
      const relation = strictWireRecord(relationValue, relationPath);
      const relationId = wireStableId(relation.relation_id, `${relationPath}.relation_id`);
      relations.set(relationId, { relation, path: relationPath });
    });
  });
  const inferred = [...relations.values()].filter(({ relation }) => (
    generalSemEstimateHasInference(relation.value)
  ));
  if (receiptValue == null) {
    if (inferred.length > 0) {
      wireFail(
        "document.invalid",
        "general_sem_results.higher_order_inference_receipt",
        "higher-order inference fields require higher_order_inference_receipt.",
      );
    }
    return;
  }
  const path = "general_sem_results.higher_order_inference_receipt";
  if (ordinaryReceiptValue != null) {
    wireFail(
      "document.invalid",
      path,
      `${path} is mutually exclusive with inference_receipt.`,
    );
  }
  const receipt = exactWireRecord(receiptValue, [
    "schema_version", "capability_cell", "method_version", "point_method_version",
    "resampling_operation_version", "resampling_stream_version", "quantile_method_version",
    "standard_error_method_version", "summation_method_version", "p_value_method_version",
    "failure_policy_version", "sign_alignment_method_version", "target_version",
    "general_sem_config_sha256", "compiled_plan_sha256", "hoc_stage_plan_sha256",
    "model_scientific_sha256", "stage_one_model_scientific_sha256",
    "stage_two_model_scientific_sha256", "source_dataset_fingerprint",
    "complete_case_frame_sha256", "usable_replicate_indices_sha256",
    "target_identity_set_sha256", "target_ids", "target_identities", "interval", "tail",
    "confidence_level", "resamples_requested", "resamples_usable",
    "minimum_usable_resamples", "seed", "workers",
    "complete_model_reestimated_per_replicate", "stage_one_reestimated_per_replicate",
    "generated_component_values_recalculated_per_replicate",
    "stage_one_scores_sign_aligned_per_replicate", "stage_two_reestimated_per_replicate",
    "stage_two_scores_sign_aligned_per_replicate",
    "complete_point_contract_validated_per_replicate", "failed_replicates",
  ], [], path);
  if (receipt.schema_version !== 1) {
    wireFail("schema.version_unsupported", `${path}.schema_version`, `${path}.schema_version must equal 1.`);
  }
  const capability = validateWireCapabilityCell(receipt.capability_cell, `${path}.capability_cell`);
  if (capabilityCellIdentity(capability)
    !== capabilityCellIdentity(GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_CELL_V1)) {
    wireFail("document.invalid", `${path}.capability_cell`, `${path}.capability_cell is not the exact HOC bootstrap cell.`);
  }
  if (!context.capabilityIds.has(capabilityCellIdentity(capability))) {
    wireFail("document.invalid", `${path}.capability_cell`, `${path}.capability_cell is not declared by the document.`);
  }
  const pointVersion = GENERAL_SEM_PLS_HIGHER_ORDER_POINT_CELL_V1.capability_version;
  const bootstrapVersion = GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_CELL_V1.capability_version;
  if (receipt.method_version !== bootstrapVersion || receipt.point_method_version !== pointVersion
    || receipt.resampling_operation_version !== GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_OPERATION_VERSION_V1
    || receipt.resampling_stream_version !== GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1
    || receipt.quantile_method_version !== GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1
    || receipt.standard_error_method_version !== GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1
    || receipt.summation_method_version !== GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1
    || receipt.p_value_method_version !== GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1
    || receipt.failure_policy_version !== GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1
    || receipt.sign_alignment_method_version !== GENERAL_SEM_PLS_DISJOINT_HOC_SIGN_ALIGNMENT_VERSION_V1
    || receipt.target_version !== GENERAL_SEM_PLS_DISJOINT_HOC_TARGET_VERSION_V1) {
    wireFail("document.invalid", path, `${path} contains a non-v1 HOC algorithm identity.`);
  }
  for (const key of [
    "general_sem_config_sha256", "compiled_plan_sha256", "hoc_stage_plan_sha256",
    "model_scientific_sha256", "stage_one_model_scientific_sha256",
    "stage_two_model_scientific_sha256", "complete_case_frame_sha256",
    "usable_replicate_indices_sha256", "target_identity_set_sha256",
  ] as const) {
    wireGeneralSemSha256(receipt[key], `${path}.${key}`);
  }
  if (receipt.model_scientific_sha256 !== context.modelDigest) {
    wireFail("document.invalid", `${path}.model_scientific_sha256`, `${path}.model_scientific_sha256 must equal provenance.model_digest.`);
  }
  if (wireGeneralSemDatasetFingerprint(receipt.source_dataset_fingerprint, `${path}.source_dataset_fingerprint`)
    !== context.datasetFingerprint) {
    wireFail("document.invalid", `${path}.source_dataset_fingerprint`, `${path}.source_dataset_fingerprint must equal provenance.dataset_fingerprint.`);
  }
  wireEnum(receipt.interval, ["percentile_type7"] as const, `${path}.interval`);
  wireEnum(receipt.tail, ["two_sided"] as const, `${path}.tail`);
  const confidence = wireFinite(receipt.confidence_level, `${path}.confidence_level`);
  if (confidence <= 0 || confidence >= 1) {
    wireFail("document.invalid", `${path}.confidence_level`, `${path}.confidence_level must be between zero and one.`);
  }
  const requested = wireU32(receipt.resamples_requested, `${path}.resamples_requested`);
  const usable = wireU32(receipt.resamples_usable, `${path}.resamples_usable`);
  const minimum = wireU32(receipt.minimum_usable_resamples, `${path}.minimum_usable_resamples`);
  if (requested < 2 || requested > 10_000 || minimum !== Math.max(2, Math.ceil(requested * 0.9))
    || usable < minimum || usable > requested) {
    wireFail("document.invalid", path, `${path} violates the exact 90 percent usable gate.`);
  }
  const seed = wireGeneralSemDecimalSafeSeed(receipt.seed, `${path}.seed`);
  if (context.seed == null || seed !== String(context.seed)) {
    wireFail("document.invalid", `${path}.seed`, `${path}.seed must equal provenance.seed.`);
  }
  const workers = wireU32(receipt.workers, `${path}.workers`);
  if (workers < 1 || workers > 64 || workers !== context.workers) {
    wireFail("document.invalid", `${path}.workers`, `${path}.workers must equal the bounded provenance worker count.`);
  }
  for (const key of [
    "complete_model_reestimated_per_replicate", "stage_one_reestimated_per_replicate",
    "generated_component_values_recalculated_per_replicate",
    "stage_one_scores_sign_aligned_per_replicate", "stage_two_reestimated_per_replicate",
    "stage_two_scores_sign_aligned_per_replicate",
    "complete_point_contract_validated_per_replicate",
  ] as const) {
    if (receipt[key] !== true) {
      wireFail("document.invalid", `${path}.${key}`, `${path}.${key} must be true.`);
    }
  }

  const targetIds = validateStableIdArray(receipt.target_ids, `${path}.target_ids`, { minimum: 1, canonical: true });
  const identityValues = wireArray(receipt.target_identities, `${path}.target_identities`);
  const identities = identityValues.map((identityValue, index) => {
    const identityPath = `${path}.target_identities[${index}]`;
    const identity = exactWireRecord(identityValue, [
      "kind", "target_version", "target_id", "relation_id", "parameter_id", "source_id",
      "target_variable_id", "point_method_version",
    ], [], identityPath);
    const normalized: CanonicalHocBootstrapTargetIdentityV1 = {
      kind: wireEnum(identity.kind, [
        "component_loading", "component_weight", "hoc_structural_path", "extended_total_effect",
      ] as const, `${identityPath}.kind`),
      target_version: wireText(identity.target_version, `${identityPath}.target_version`),
      target_id: wireStableId(identity.target_id, `${identityPath}.target_id`),
      relation_id: wireStableId(identity.relation_id, `${identityPath}.relation_id`),
      parameter_id: wireStableId(identity.parameter_id, `${identityPath}.parameter_id`),
      source_id: wireStableId(identity.source_id, `${identityPath}.source_id`),
      target_variable_id: wireStableId(identity.target_variable_id, `${identityPath}.target_variable_id`),
      point_method_version: wireText(identity.point_method_version, `${identityPath}.point_method_version`),
    };
    if (normalized.target_version !== GENERAL_SEM_PLS_DISJOINT_HOC_TARGET_VERSION_V1
      || normalized.point_method_version !== pointVersion
      || normalized.target_id !== normalized.relation_id) {
      wireFail("document.invalid", identityPath, `${identityPath} differs from the exact HOC target contract.`);
    }
    return normalized;
  });
  if (identities.length !== targetIds.length
    || identities.some((identity, index) => identity.target_id !== targetIds[index])) {
    wireFail("document.invalid", `${path}.target_identities`, `${path}.target_identities must exactly follow target_ids.`);
  }
  if (receipt.target_identity_set_sha256 !== generalSemSerializedSha256(identities)) {
    wireFail("document.invalid", `${path}.target_identity_set_sha256`, `${path}.target_identity_set_sha256 does not match target_identities.`);
  }
  identities.forEach((identity, index) => {
    const bound = relations.get(identity.relation_id);
    const identityPath = `${path}.target_identities[${index}]`;
    if (!bound) wireFail("document.invalid", identityPath, `${identityPath} references a missing HOC relation.`);
    const relation = bound.relation;
    const relationKind = wireEnum(relation.kind, [
      "component_loading", "component_weight", "authored_structural", "authored_control",
      "technical_structural", "extended_indirect_effect", "extended_total_effect",
    ] as const, `${bound.path}.kind`);
    const expectedKind = identity.kind === "hoc_structural_path" ? "authored_structural" : identity.kind;
    if (relationKind !== expectedKind || relation.parameter_id !== identity.parameter_id
      || relation.source_id !== identity.source_id || relation.target_id !== identity.target_variable_id
      || !generalSemEstimateHasInference(relation.value)) {
      wireFail("document.invalid", identityPath, `${identityPath} differs from its typed HOC relation.`);
    }
    const estimate = strictWireRecord(relation.value, `${bound.path}.value`);
    const relationUsable = optionalWireU32(estimate, "bootstrap_usable_replicates", `${bound.path}.value`);
    const exceedances = optionalWireU32(estimate, "bootstrap_two_sided_exceedances", `${bound.path}.value`);
    const pValue = optionalWireFinite(estimate, "p_value", `${bound.path}.value`);
    if (relationUsable !== usable || exceedances == null || exceedances > usable
      || pValue == null || !approximatelyEqualGeneralSem(pValue, (exceedances + 1) / (usable + 1))) {
      wireFail("document.invalid", `${bound.path}.value`, `${bound.path}.value contradicts the shared bootstrap ledger.`);
    }
  });
  const targetSet = new Set(targetIds);
  if (inferred.some(({ relation }) => !targetSet.has(String(relation.relation_id)))) {
    wireFail("document.invalid", path, `${path} leaves inferred relations outside target_ids.`);
  }

  const failures = wireArray(receipt.failed_replicates, `${path}.failed_replicates`);
  if (usable + failures.length !== requested) {
    wireFail("document.invalid", path, `${path} requested count must equal usable plus failed replicates.`);
  }
  const failedIndices = new Set<number>();
  let previous = -1;
  failures.forEach((failureValue, index) => {
    const failurePath = `${path}.failed_replicates[${index}]`;
    const failure = exactWireRecord(failureValue, ["replicate_index", "reason_code", "message"], [], failurePath);
    const replicateIndex = wireU32(failure.replicate_index, `${failurePath}.replicate_index`);
    if (replicateIndex >= requested || replicateIndex <= previous) {
      wireFail("document.invalid", `${failurePath}.replicate_index`, `${path}.failed_replicates must be strictly ordered within the requested frame.`);
    }
    previous = replicateIndex;
    failedIndices.add(replicateIndex);
    wireEnum(failure.reason_code, [
      "insufficient_observations", "constant_indicator", "stage_one_rank_deficient",
      "isolated_construct", "stage_one_nonconvergence", "indeterminate_score_sign",
      "constant_component_score", "stage_two_rank_deficient", "stage_two_nonconvergence",
      "component_collinearity", "numerical_failure",
    ] as const, `${failurePath}.reason_code`);
    wireText(failure.message, `${failurePath}.message`);
  });
  const usableIndices = Array.from({ length: requested }, (_, index) => index)
    .filter((index) => !failedIndices.has(index));
  if (receipt.usable_replicate_indices_sha256 !== generalSemSerializedSha256(usableIndices)) {
    wireFail("document.invalid", `${path}.usable_replicate_indices_sha256`, `${path}.usable_replicate_indices_sha256 contradicts the failure ledger.`);
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

function validateCbsemEndpointV1(value: unknown, path: string): string {
  const endpoint = strictWireRecord(value, path);
  const kind = wireEnum(endpoint.kind, ["variable", "residual", "disturbance"] as const, `${path}.kind`);
  const exact = exactWireRecord(value, ["kind", "variable_id"], [], path);
  const variableId = wireStableId(exact.variable_id, `${path}.variable_id`);
  return `${kind}:${variableId}`;
}

function validateCbsemParameterRowsV1(
  values: readonly unknown[],
  context: GeneralSemWireContext,
): Map<string, { estimate: number; isFree: boolean }> {
  const pointCell = capabilityCellIdentity(CBSEM_GENERAL_SEM_ML_CAPABILITY_CELL_V1);
  const rows = new Map<string, { estimate: number; isFree: boolean }>();
  values.forEach((value, index) => {
    const path = `general_sem_results.cbsem_parameters[${index}]`;
    const row = exactWireRecord(value, [
      "parameter_id", "trace", "role", "target", "state", "estimate",
    ], ["relation_id", "standard_error", "z_value", "p_value", "standardized_estimate"], path);
    const parameterId = wireStableId(row.parameter_id, `${path}.parameter_id`);
    const traceCell = validateGeneralSemTrace(row.trace, `${path}.trace`, context);
    if (capabilityCellIdentity(traceCell) !== pointCell) {
      wireFail("document.invalid", `${path}.trace.capability_cell`, `${path}.trace.capability_cell must equal the exact CB-SEM General SEM ML point cell.`);
    }
    const role = wireEnum(row.role, ["loading", "regression", "covariance", "variance"] as const, `${path}.role`);
    if (row.relation_id != null) wireStableId(row.relation_id, `${path}.relation_id`);
    const target = strictWireRecord(row.target, `${path}.target`);
    const targetKind = wireEnum(target.kind, ["loading", "regression", "covariance", "variance"] as const, `${path}.target.kind`);
    if (targetKind !== role) wireFail("document.invalid", `${path}.target.kind`, `${path}.role and target kind must agree.`);
    if (targetKind === "loading") {
      const exact = exactWireRecord(row.target, ["kind", "factor_id", "indicator_id"], [], `${path}.target`);
      const factor = wireStableId(exact.factor_id, `${path}.target.factor_id`);
      const indicator = wireStableId(exact.indicator_id, `${path}.target.indicator_id`);
      if (factor === indicator) wireFail("document.invalid", `${path}.target`, `${path}.target loading factor and indicator must differ.`);
    } else if (targetKind === "regression") {
      const exact = exactWireRecord(row.target, ["kind", "source_id", "target_id"], [], `${path}.target`);
      const source = wireStableId(exact.source_id, `${path}.target.source_id`);
      const targetId = wireStableId(exact.target_id, `${path}.target.target_id`);
      if (source === targetId) wireFail("document.invalid", `${path}.target`, `${path}.target regression source and target must differ.`);
    } else if (targetKind === "covariance") {
      const exact = exactWireRecord(row.target, ["kind", "left", "right"], [], `${path}.target`);
      const left = validateCbsemEndpointV1(exact.left, `${path}.target.left`);
      const right = validateCbsemEndpointV1(exact.right, `${path}.target.right`);
      if (left === right) wireFail("document.invalid", `${path}.target`, `${path}.target covariance endpoints must differ.`);
    } else {
      const exact = exactWireRecord(row.target, ["kind", "endpoint"], [], `${path}.target`);
      validateCbsemEndpointV1(exact.endpoint, `${path}.target.endpoint`);
    }
    const estimate = wireFinite(row.estimate, `${path}.estimate`);
    const standardError = optionalWireFinite(row, "standard_error", path);
    const zValue = optionalWireFinite(row, "z_value", path);
    const pValue = optionalWireFinite(row, "p_value", path);
    optionalWireFinite(row, "standardized_estimate", path);
    const uncertaintyCount = [standardError, zValue, pValue].filter((item) => item != null).length;
    if (uncertaintyCount !== 0 && uncertaintyCount !== 3) {
      wireFail("document.invalid", path, `${path} standard_error, z_value, and p_value must be all absent or all present.`);
    }
    if (standardError != null && standardError < 0) wireFail("document.invalid", `${path}.standard_error`, `${path}.standard_error must be nonnegative.`);
    if (pValue != null && (pValue < 0 || pValue > 1)) wireFail("document.invalid", `${path}.p_value`, `${path}.p_value must be between 0 and 1.`);
    const state = strictWireRecord(row.state, `${path}.state`);
    const stateKind = wireEnum(state.kind, ["fixed", "free"] as const, `${path}.state.kind`);
    if (stateKind === "fixed") {
      const exact = exactWireRecord(row.state, ["kind", "value"], [], `${path}.state`);
      const fixed = wireFinite(exact.value, `${path}.state.value`);
      if (!approximatelyEqualGeneralSem(fixed, estimate) || uncertaintyCount !== 0) {
        wireFail("document.invalid", `${path}.state`, `${path}.state fixed value must equal estimate and must not publish uncertainty.`);
      }
    } else {
      const exact = exactWireRecord(row.state, ["kind"], ["equality_label", "lower", "upper"], `${path}.state`);
      if (exact.equality_label != null) wireText(exact.equality_label, `${path}.state.equality_label`);
      const lower = optionalWireFinite(exact, "lower", `${path}.state`);
      const upper = optionalWireFinite(exact, "upper", `${path}.state`);
      validateGeneralSemBounds(lower, upper, `${path}.state`);
      if ((lower != null && estimate < lower) || (upper != null && estimate > upper)) {
        wireFail("document.invalid", `${path}.estimate`, `${path}.estimate must satisfy its declared bounds.`);
      }
    }
    rows.set(parameterId, { estimate, isFree: stateKind === "free" });
  });
  return rows;
}

function validateCbsemBootstrapV1(
  receiptValue: unknown,
  inferenceValues: readonly unknown[],
  pointRows: ReadonlyMap<string, { estimate: number; isFree: boolean }>,
  context: GeneralSemWireContext,
): void {
  const path = "general_sem_results.cbsem_bootstrap_receipt";
  if (receiptValue == null) {
    if (inferenceValues.length > 0) wireFail("document.invalid", path, `${path} is required when cbsem_bootstrap_inference is present.`);
    return;
  }
  if (inferenceValues.length === 0) wireFail("document.invalid", path, `${path} requires at least one cbsem_bootstrap_inference row.`);
  const receipt = exactWireRecord(receiptValue, [
    "capability_cell", "method_version", "resampling_operation_version", "quantile_method_version",
    "compiled_plan_sha256", "base_plan_sha256", "parameter_inventory_sha256",
    "model_scientific_sha256", "general_sem_config_sha256", "recipe_analytical_sha256",
    "source_dataset_fingerprint", "complete_case_frame_sha256", "usable_replicate_indices_sha256",
    "confidence_level", "resamples_requested", "resamples_usable", "minimum_usable_resamples",
    "seed", "workers", "complete_model_reestimated_per_replicate", "failed_replicates",
  ], [], path);
  const cell = validateWireCapabilityCell(receipt.capability_cell, `${path}.capability_cell`);
  const exactCellIdentity = capabilityCellIdentity(CBSEM_RECURSIVE_SEM_BOOTSTRAP_CAPABILITY_CELL_V1);
  if (capabilityCellIdentity(cell) !== exactCellIdentity || !context.capabilityIds.has(exactCellIdentity)) {
    wireFail("document.invalid", `${path}.capability_cell`, `${path}.capability_cell must equal the declared recursive-SEM bootstrap cell.`);
  }
  if (receipt.method_version !== CBSEM_RECURSIVE_SEM_BOOTSTRAP_METHOD_VERSION_V1) wireFail("document.invalid", `${path}.method_version`, `${path}.method_version is not the frozen v1 method.`);
  if (receipt.resampling_operation_version !== CBSEM_RECURSIVE_SEM_BOOTSTRAP_OPERATION_VERSION_V1) wireFail("document.invalid", `${path}.resampling_operation_version`, `${path}.resampling_operation_version is not the frozen v1 operation.`);
  if (receipt.quantile_method_version !== GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1) wireFail("document.invalid", `${path}.quantile_method_version`, `${path}.quantile_method_version must equal type7_quantile_v1.`);
  for (const key of [
    "compiled_plan_sha256", "base_plan_sha256", "parameter_inventory_sha256",
    "model_scientific_sha256", "general_sem_config_sha256", "recipe_analytical_sha256",
    "complete_case_frame_sha256", "usable_replicate_indices_sha256",
  ] as const) wireGeneralSemSha256(receipt[key], `${path}.${key}`);
  if (receipt.model_scientific_sha256 !== context.modelDigest) wireFail("document.invalid", `${path}.model_scientific_sha256`, `${path}.model_scientific_sha256 must equal provenance.model_digest.`);
  if (receipt.recipe_analytical_sha256 !== context.recipeDigest) wireFail("document.invalid", `${path}.recipe_analytical_sha256`, `${path}.recipe_analytical_sha256 must equal provenance.recipe_digest.`);
  if (wireGeneralSemDatasetFingerprint(receipt.source_dataset_fingerprint, `${path}.source_dataset_fingerprint`) !== context.datasetFingerprint) wireFail("document.invalid", `${path}.source_dataset_fingerprint`, `${path}.source_dataset_fingerprint must equal provenance.dataset_fingerprint.`);
  const confidence = wireFinite(receipt.confidence_level, `${path}.confidence_level`);
  if (confidence !== 0.95) wireFail("document.invalid", `${path}.confidence_level`, `${path}.confidence_level must equal 0.95.`);
  const requested = wireU32(receipt.resamples_requested, `${path}.resamples_requested`);
  const usable = wireU32(receipt.resamples_usable, `${path}.resamples_usable`);
  const minimum = wireU32(receipt.minimum_usable_resamples, `${path}.minimum_usable_resamples`);
  if (requested < 500 || requested > 10_000) wireFail("document.invalid", `${path}.resamples_requested`, `${path}.resamples_requested must be between 500 and 10000.`);
  if (minimum !== Math.ceil(requested * 0.9)) wireFail("document.invalid", `${path}.minimum_usable_resamples`, `${path}.minimum_usable_resamples must equal the 90 percent usable gate.`);
  const workers = wireU32(receipt.workers, `${path}.workers`);
  if (workers < 1 || workers > 64 || workers !== context.workers) wireFail("document.invalid", `${path}.workers`, `${path}.workers must be between 1 and 64 and equal provenance.workers.`);
  const seed = wireGeneralSemDecimalSafeSeed(receipt.seed, `${path}.seed`);
  if (context.seed == null || Number(seed) !== context.seed) wireFail("document.invalid", `${path}.seed`, `${path}.seed must equal provenance.seed.`);
  if (receipt.complete_model_reestimated_per_replicate !== true) wireFail("document.invalid", `${path}.complete_model_reestimated_per_replicate`, `${path}.complete_model_reestimated_per_replicate must be true.`);
  const failures = wireArray(receipt.failed_replicates, `${path}.failed_replicates`);
  if (usable + failures.length !== requested) wireFail("document.invalid", path, `${path} requested count must equal usable plus failed replicates.`);
  let previousFailure = -1;
  failures.forEach((value, index) => {
    const failurePath = `${path}.failed_replicates[${index}]`;
    const failure = exactWireRecord(value, ["replicate_index", "reason_code", "message"], [], failurePath);
    const replicate = wireU32(failure.replicate_index, `${failurePath}.replicate_index`);
    if (replicate >= requested || replicate <= previousFailure) wireFail("document.invalid", `${failurePath}.replicate_index`, `${failurePath}.replicate_index must be unique, ordered, and in range.`);
    previousFailure = replicate;
    wireEnum(failure.reason_code, ["insufficient_observations", "nonpositive_definite_sample_covariance", "nonconvergence", "nonfinite_estimate", "parameter_inventory_mismatch", "numerical_failure"] as const, `${failurePath}.reason_code`);
    wireText(failure.message, `${failurePath}.message`);
  });
  const inferenceParameterIds = inferenceValues.map((value, index) => wireStableId(
    strictWireRecord(value, `general_sem_results.cbsem_bootstrap_inference[${index}]`).parameter_id,
    `general_sem_results.cbsem_bootstrap_inference[${index}].parameter_id`,
  ));
  if (receipt.parameter_inventory_sha256 !== generalSemSerializedSha256(inferenceParameterIds)) {
    wireFail("document.invalid", `${path}.parameter_inventory_sha256`, `${path}.parameter_inventory_sha256 must bind the ordered inference parameter IDs.`);
  }
  inferenceValues.forEach((value, index) => {
    const inferencePath = `general_sem_results.cbsem_bootstrap_inference[${index}]`;
    const inference = exactWireRecord(value, ["parameter_id", "trace", "point_estimate", "outcome"], [], inferencePath);
    const parameterId = wireStableId(inference.parameter_id, `${inferencePath}.parameter_id`);
    const traceCell = validateGeneralSemTrace(inference.trace, `${inferencePath}.trace`, context);
    if (capabilityCellIdentity(traceCell) !== exactCellIdentity) wireFail("document.invalid", `${inferencePath}.trace.capability_cell`, `${inferencePath}.trace.capability_cell must equal the recursive-SEM bootstrap cell.`);
    const point = wireFinite(inference.point_estimate, `${inferencePath}.point_estimate`);
    const pointRow = pointRows.get(parameterId);
    if (!pointRow || !approximatelyEqualGeneralSem(pointRow.estimate, point)) wireFail("document.invalid", `${inferencePath}.point_estimate`, `${inferencePath}.point_estimate must equal the point parameter estimate.`);
    const outcome = strictWireRecord(inference.outcome, `${inferencePath}.outcome`);
    const kind = wireEnum(outcome.kind, ["available", "unavailable"] as const, `${inferencePath}.outcome.kind`);
    if (kind === "available") {
      if (!pointRow!.isFree) wireFail("document.invalid", `${inferencePath}.outcome`, `${inferencePath}.outcome cannot publish inference for a fixed parameter.`);
      const available = exactWireRecord(inference.outcome, ["kind", "value"], [], `${inferencePath}.outcome`);
      validateGeneralSemEstimate(available.value, `${inferencePath}.outcome.value`);
      const estimate = strictWireRecord(available.value, `${inferencePath}.outcome.value`);
      if (!approximatelyEqualGeneralSem(wireFinite(estimate.estimate, `${inferencePath}.outcome.value.estimate`), point)
        || !generalSemEstimateHasInference(estimate) || usable < minimum) {
        wireFail("document.invalid", `${inferencePath}.outcome`, `${inferencePath}.outcome must contain available inference for the same point estimate above the usable gate.`);
      }
      const rowUsable = optionalWireU32(estimate, "bootstrap_usable_replicates", `${inferencePath}.outcome.value`);
      const exceedances = optionalWireU32(estimate, "bootstrap_two_sided_exceedances", `${inferencePath}.outcome.value`);
      const pValue = optionalWireFinite(estimate, "p_value", `${inferencePath}.outcome.value`);
      if (rowUsable !== usable || exceedances == null || exceedances > usable || pValue == null
        || !approximatelyEqualGeneralSem(pValue, (exceedances + 1) / (usable + 1))) {
        wireFail("document.invalid", `${inferencePath}.outcome.value`, `${inferencePath}.outcome.value must reconcile with the receipt and plus-one exceedance ledger.`);
      }
    } else {
      const unavailable = exactWireRecord(inference.outcome, ["kind", "reason"], [], `${inferencePath}.outcome`);
      const reason = wireEnum(unavailable.reason, ["insufficient_usable_replicates", "parameter_not_eligible"] as const, `${inferencePath}.outcome.reason`);
      if (reason === "insufficient_usable_replicates" && usable >= minimum) wireFail("document.invalid", `${inferencePath}.outcome.reason`, `${inferencePath} cannot report insufficient usable replicates after the gate passed.`);
    }
  });
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
      "joint_stage_structural_coefficients",
      "interaction_effects",
      "three_way_interaction_effects",
      "three_way_conditional_interaction_effects",
      "three_way_simple_slopes",
      "three_way_moderation_bootstrap_receipt",
      "conditional_effect_probes",
      "conditional_effects",
      "conditional_indirect_effects",
      "moderated_mediation_indices",
      "interaction_plots",
      "higher_order_stages",
      "cbsem_parameters",
      "higher_order_inference_receipt",
      "cbsem_fit",
      "identification_diagnostics",
      "cbsem_bootstrap_receipt",
      "cbsem_bootstrap_inference",
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
  const jointStageCoefficients = optionalWireArray(
    results,
    "joint_stage_structural_coefficients",
    "general_sem_results",
  );
  const interactionEffects = optionalWireArray(results, "interaction_effects", "general_sem_results");
  const threeWayInteractionEffects = optionalWireArray(
    results,
    "three_way_interaction_effects",
    "general_sem_results",
  );
  const threeWayConditionalEffects = optionalWireArray(
    results,
    "three_way_conditional_interaction_effects",
    "general_sem_results",
  );
  const threeWaySimpleSlopes = optionalWireArray(
    results,
    "three_way_simple_slopes",
    "general_sem_results",
  );
  const probes = optionalWireArray(results, "conditional_effect_probes", "general_sem_results");
  const conditional = optionalWireArray(results, "conditional_effects", "general_sem_results");
  const conditionalIndirect = optionalWireArray(
    results,
    "conditional_indirect_effects",
    "general_sem_results",
  );
  const moderatedMediationIndices = optionalWireArray(
    results,
    "moderated_mediation_indices",
    "general_sem_results",
  );
  const plots = optionalWireArray(results, "interaction_plots", "general_sem_results");
  const hocStages = optionalWireArray(results, "higher_order_stages", "general_sem_results");
  const cbsemParameters = optionalWireArray(results, "cbsem_parameters", "general_sem_results");
  const fits = optionalWireArray(results, "cbsem_fit", "general_sem_results");
  const identification = optionalWireArray(results, "identification_diagnostics", "general_sem_results");
  const cbsemBootstrapInference = optionalWireArray(
    results,
    "cbsem_bootstrap_inference",
    "general_sem_results",
  );
  const hasCbsemBootstrapReceipt = Object.prototype.hasOwnProperty.call(results, "cbsem_bootstrap_receipt")
    && results.cbsem_bootstrap_receipt != null;
  const hasThreeWayBootstrapReceipt = Object.prototype.hasOwnProperty.call(
    results,
    "three_way_moderation_bootstrap_receipt",
  ) && results.three_way_moderation_bootstrap_receipt != null;
  if ([specific, aggregate, jointStageCoefficients, interactionEffects, threeWayInteractionEffects,
    threeWayConditionalEffects, threeWaySimpleSlopes, probes, conditional,
    conditionalIndirect, moderatedMediationIndices, plots, hocStages, cbsemParameters, fits,
    identification, cbsemBootstrapInference]
    .every((collection) => collection.length === 0)
    && !hasCbsemBootstrapReceipt
    && !hasThreeWayBootstrapReceipt) {
    return wireFail("document.invalid", "general_sem_results", "general_sem_results must contain at least one typed result section.");
  }

  validateCanonicalWireIds(specific, "effect_id", "general_sem_results.specific_indirect_effects");
  validateCanonicalWireIds(aggregate, "effect_id", "general_sem_results.aggregate_effects");
  validateCanonicalWireIds(
    jointStageCoefficients,
    "relation_id",
    "general_sem_results.joint_stage_structural_coefficients",
  );
  validateCanonicalWireIds(interactionEffects, "effect_id", "general_sem_results.interaction_effects");
  validateCanonicalWireIds(
    threeWayInteractionEffects,
    "effect_id",
    "general_sem_results.three_way_interaction_effects",
  );
  validateCanonicalWireIds(
    threeWayConditionalEffects,
    "effect_id",
    "general_sem_results.three_way_conditional_interaction_effects",
  );
  validateCanonicalWireIds(
    threeWaySimpleSlopes,
    "effect_id",
    "general_sem_results.three_way_simple_slopes",
  );
  validateCanonicalWireIds(probes, "probe_id", "general_sem_results.conditional_effect_probes");
  validateCanonicalWireIds(conditional, "effect_id", "general_sem_results.conditional_effects");
  validateCanonicalWireIds(
    conditionalIndirect,
    "effect_id",
    "general_sem_results.conditional_indirect_effects",
  );
  validateCanonicalWireIds(
    moderatedMediationIndices,
    "effect_id",
    "general_sem_results.moderated_mediation_indices",
  );
  validateCanonicalWireIds(plots, "plot_id", "general_sem_results.interaction_plots");
  validateCanonicalWireIds(hocStages, "stage_id", "general_sem_results.higher_order_stages");
  validateCanonicalWireIds(cbsemParameters, "parameter_id", "general_sem_results.cbsem_parameters");
  validateCanonicalWireIds(fits, "fit_id", "general_sem_results.cbsem_fit");
  validateCanonicalWireIds(identification, "diagnostic_id", "general_sem_results.identification_diagnostics");
  validateCanonicalWireIds(
    cbsemBootstrapInference,
    "parameter_id",
    "general_sem_results.cbsem_bootstrap_inference",
  );

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

  if ((jointStageCoefficients.length === 0) !== (interactionEffects.length === 0)) {
    wireFail(
      "document.invalid",
      "general_sem_results.joint_stage_structural_coefficients",
      "general_sem_results.joint_stage_structural_coefficients and interaction_effects must both be present for the exact joint-stage moderation cell.",
    );
  }
  const threeWayJointPoint = threeWayInteractionEffects.length > 0;
  const expectedJointPointCell = threeWayJointPoint
    ? GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1
    : GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_CAPABILITY_CELL_V1;
  const expectedJointPointMethod = threeWayJointPoint
    ? GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1
    : GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1;
  const jointStageParameterIds = new Set<string>();
  const jointStageEstimatesByRelation = new Map<string, number>();
  jointStageCoefficients.forEach((item, index) => {
    const path = `general_sem_results.joint_stage_structural_coefficients[${index}]`;
    const coefficient = exactWireRecord(item, [
      "relation_id",
      "parameter_id",
      "trace",
      "source_id",
      "target_id",
      "role",
      "estimate",
      "stage",
      "method_version",
    ], [], path);
    const relationId = wireStableId(coefficient.relation_id, `${path}.relation_id`);
    const parameterId = wireStableId(coefficient.parameter_id, `${path}.parameter_id`);
    if (jointStageParameterIds.has(parameterId)) {
      wireFail("document.invalid", `${path}.parameter_id`, `${path}.parameter_id is duplicated.`);
    }
    jointStageParameterIds.add(parameterId);
    const traceCapability = validateGeneralSemTrace(coefficient.trace, `${path}.trace`, wireContext);
    if (capabilityCellIdentity(traceCapability)
      !== capabilityCellIdentity(expectedJointPointCell)) {
      wireFail(
        "document.invalid",
        `${path}.trace.capability_cell`,
        `${path}.trace.capability_cell must equal the exact joint moderation point option cell.`,
      );
    }
    const sourceId = wireStableId(coefficient.source_id, `${path}.source_id`);
    const targetId = wireStableId(coefficient.target_id, `${path}.target_id`);
    if (sourceId === targetId) {
      wireFail("document.invalid", path, `${path} requires distinct source_id and target_id.`);
    }
    wireEnum(coefficient.role, ["structural", "control"] as const, `${path}.role`);
    validateGeneralSemEstimate(coefficient.estimate, `${path}.estimate`);
    jointStageEstimatesByRelation.set(
      relationId,
      wireFinite(
        strictWireRecord(coefficient.estimate, `${path}.estimate`).estimate,
        `${path}.estimate.estimate`,
      ),
    );
    if (generalSemEstimateHasInference(coefficient.estimate)) {
      wireFail(
        "document.invalid",
        `${path}.estimate`,
        `${path}.estimate must contain point estimation only.`,
      );
    }
    wireEnum(coefficient.stage, ["joint_stage_two"] as const, `${path}.stage`);
    const methodVersion = wireStableId(coefficient.method_version, `${path}.method_version`);
    if (methodVersion !== expectedJointPointMethod) {
      wireFail(
        "document.invalid",
        `${path}.method_version`,
        `${path}.method_version must equal ${expectedJointPointMethod}.`,
      );
    }
  });

  const interactionAuthorities = new Map<string, {
    interactionId: string;
    focalRelationId: string;
    focalPredictorId: string;
    moderatorId: string;
    outcomeId: string;
    capabilityIdentity: string;
  }>();
  const interactionIds = new Set<string>();
  const interactionRelationIds = new Set<string>();
  const interactionParameterIds = new Set<string>();
  const generatedProductColumnIds = new Set<string>();
  const stageOneModelDigests = new Set<string>();
  interactionEffects.forEach((item, index) => {
    const path = `general_sem_results.interaction_effects[${index}]`;
    const effect = exactWireRecord(item, [
      "effect_id", "trace", "interaction_id", "focal_relation_id",
      "interaction_effect_relation_id", "interaction_effect_parameter_id",
      "focal_predictor_id", "moderator_id", "outcome_id", "generated_product_column_id",
      "stage_one_model_scientific_sha256", "method_version", "construction_method",
      "product_scale_version", "hierarchy_policy",
      "hierarchy_policy_version", "conditioning_policy_version", "observation_count",
      "unstandardized_product_mean", "unstandardized_product_sample_standard_deviation",
      "standardized_product_coefficient", "scientific_rescaled_gamma",
    ], [], path);
    const effectId = wireStableId(effect.effect_id, `${path}.effect_id`);
    if (effectIds.has(effectId)) {
      wireFail("document.invalid", `${path}.effect_id`, `${path}.effect_id is duplicated across effect sections.`);
    }
    effectIds.add(effectId);
    const traceCapability = validateGeneralSemTrace(effect.trace, `${path}.trace`, wireContext);
    if (capabilityCellIdentity(traceCapability)
      !== capabilityCellIdentity(expectedJointPointCell)) {
      wireFail(
        "document.invalid",
        `${path}.trace.capability_cell`,
        `${path}.trace.capability_cell must equal the exact joint moderation point option cell.`,
      );
    }
    const interactionId = wireStableId(effect.interaction_id, `${path}.interaction_id`);
    const focalRelationId = wireStableId(effect.focal_relation_id, `${path}.focal_relation_id`);
    const interactionRelationId = wireStableId(
      effect.interaction_effect_relation_id,
      `${path}.interaction_effect_relation_id`,
    );
    const interactionParameterId = wireStableId(
      effect.interaction_effect_parameter_id,
      `${path}.interaction_effect_parameter_id`,
    );
    const focalPredictorId = wireStableId(effect.focal_predictor_id, `${path}.focal_predictor_id`);
    const moderatorId = wireStableId(effect.moderator_id, `${path}.moderator_id`);
    const outcomeId = wireStableId(effect.outcome_id, `${path}.outcome_id`);
    const productColumnId = wireStableId(
      effect.generated_product_column_id,
      `${path}.generated_product_column_id`,
    );
    const stageOneModelDigest = wireGeneralSemSha256(
      effect.stage_one_model_scientific_sha256,
      `${path}.stage_one_model_scientific_sha256`,
    );
    if (stageOneModelDigest === wireContext.modelDigest) {
      wireFail(
        "document.invalid",
        `${path}.stage_one_model_scientific_sha256`,
        `${path}.stage_one_model_scientific_sha256 must identify the projected interaction-free scoring model.`,
      );
    }
    stageOneModelDigests.add(stageOneModelDigest);
    for (const key of [
      "method_version", "product_scale_version", "hierarchy_policy_version",
      "conditioning_policy_version",
    ] as const) {
      wireStableId(effect[key], `${path}.${key}`);
    }
    wireEnum(effect.construction_method, ["two_stage"] as const, `${path}.construction_method`);
    wireEnum(effect.hierarchy_policy, ["strong"] as const, `${path}.hierarchy_policy`);
    const exactPolicyVersions = {
      method_version: expectedJointPointMethod,
      product_scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
      hierarchy_policy_version: GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1,
      conditioning_policy_version: threeWayJointPoint
        ? GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1
        : GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1,
    } as const;
    for (const [key, expected] of Object.entries(exactPolicyVersions)) {
      if (effect[key] !== expected) {
        wireFail(
          "document.invalid",
          `${path}.${key}`,
          `${path}.${key} must equal ${expected}.`,
        );
      }
    }
    const observationCount = wireU32(effect.observation_count, `${path}.observation_count`);
    if (observationCount < 3) {
      wireFail("document.invalid", `${path}.observation_count`, `${path}.observation_count must be at least three.`);
    }
    wireFinite(effect.unstandardized_product_mean, `${path}.unstandardized_product_mean`);
    const productSd = wireFinite(
      effect.unstandardized_product_sample_standard_deviation,
      `${path}.unstandardized_product_sample_standard_deviation`,
    );
    if (productSd <= Number.EPSILON) {
      wireFail(
        "document.invalid",
        `${path}.unstandardized_product_sample_standard_deviation`,
        `${path}.unstandardized_product_sample_standard_deviation must be positive.`,
      );
    }
    validateGeneralSemEstimate(
      effect.standardized_product_coefficient,
      `${path}.standardized_product_coefficient`,
    );
    validateGeneralSemEstimate(effect.scientific_rescaled_gamma, `${path}.scientific_rescaled_gamma`);
    const standardized = strictWireRecord(
      effect.standardized_product_coefficient,
      `${path}.standardized_product_coefficient`,
    );
    const gamma = strictWireRecord(effect.scientific_rescaled_gamma, `${path}.scientific_rescaled_gamma`);
    const expectedGamma = wireFinite(
      standardized.estimate,
      `${path}.standardized_product_coefficient.estimate`,
    ) / productSd;
    if (!approximatelyEqualGeneralSem(
      expectedGamma,
      wireFinite(gamma.estimate, `${path}.scientific_rescaled_gamma.estimate`),
    )) {
      wireFail(
        "document.invalid",
        `${path}.scientific_rescaled_gamma.estimate`,
        `${path}.scientific_rescaled_gamma must equal the standardized-product coefficient divided by product SD.`,
      );
    }
    if (effectId !== interactionRelationId) {
      wireFail(
        "document.invalid",
        `${path}.effect_id`,
        `${path}.effect_id must equal interaction_effect_relation_id.`,
      );
    }
    if (new Set([focalPredictorId, moderatorId, outcomeId]).size !== 3) {
      wireFail("document.invalid", path, `${path} requires distinct focal, moderator, and outcome identities.`);
    }
    if (interactionIds.has(interactionId)) {
      wireFail("document.invalid", `${path}.interaction_id`, `${path}.interaction_id is duplicated.`);
    }
    if (interactionRelationIds.has(interactionRelationId)) {
      wireFail(
        "document.invalid",
        `${path}.interaction_effect_relation_id`,
        `${path}.interaction_effect_relation_id is duplicated.`,
      );
    }
    if (interactionParameterIds.has(interactionParameterId)) {
      wireFail(
        "document.invalid",
        `${path}.interaction_effect_parameter_id`,
        `${path}.interaction_effect_parameter_id is duplicated.`,
      );
    }
    if (generatedProductColumnIds.has(productColumnId)) {
      wireFail(
        "document.invalid",
        `${path}.generated_product_column_id`,
        `${path}.generated_product_column_id is duplicated.`,
      );
    }
    interactionIds.add(interactionId);
    interactionRelationIds.add(interactionRelationId);
    interactionParameterIds.add(interactionParameterId);
    generatedProductColumnIds.add(productColumnId);
    interactionAuthorities.set(effectId, {
      interactionId,
      focalRelationId,
      focalPredictorId,
      moderatorId,
      outcomeId,
      capabilityIdentity: capabilityCellIdentity(traceCapability),
    });
  });
  if (stageOneModelDigests.size > 1) {
    wireFail(
      "document.invalid",
      "general_sem_results.interaction_effects",
      "general_sem_results.interaction_effects must share one stage-one model scientific digest.",
    );
  }

  if (conditionalIndirect.length > 0 || moderatedMediationIndices.length > 0) {
    if (conditionalIndirect.length !== 3) {
      wireFail(
        "document.invalid",
        "general_sem_results.conditional_indirect_effects",
        "general_sem_results.conditional_indirect_effects must contain exactly the locked -1/0/+1 targets.",
      );
    }
    if (moderatedMediationIndices.length !== 1) {
      wireFail(
        "document.invalid",
        "general_sem_results.moderated_mediation_indices",
        "general_sem_results.moderated_mediation_indices must contain exactly one index.",
      );
    }
    if (interactionEffects.length !== 1) {
      wireFail(
        "document.invalid",
        "general_sem_results.interaction_effects",
        "general_sem_results two-way moderated mediation requires exactly one interaction effect.",
      );
    }
    const receipt = strictWireRecord(
      results.inference_receipt,
      "general_sem_results.inference_receipt",
    );
    const receiptCell = validateWireCapabilityCell(
      receipt.capability_cell,
      "general_sem_results.inference_receipt.capability_cell",
    );
    if (capabilityCellIdentity(receiptCell)
      !== capabilityCellIdentity(
        GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CAPABILITY_CELL_V1,
      )) {
      wireFail(
        "document.invalid",
        "general_sem_results.inference_receipt.capability_cell",
        "general_sem_results moderated-mediation rows require the exact combined bootstrap receipt.",
      );
    }

    const lockedProbes = [-1, 0, 1] as const;
    const probeIndices = new Set<number>();
    const conditionalAuthorities = conditionalIndirect.map((item, index) => {
      const path = `general_sem_results.conditional_indirect_effects[${index}]`;
      const effect = exactWireRecord(item, [
        "effect_id", "target_id", "estimand_id", "trace", "moderated_stage",
        "interaction_id", "x_id", "mediator_id", "y_id", "moderator_id",
        "ordered_relation_ids", "probe_value_index", "moderator_value", "value",
      ], [], path);
      const effectId = wireStableId(effect.effect_id, `${path}.effect_id`);
      if (effectIds.has(effectId)) {
        wireFail(
          "document.invalid",
          `${path}.effect_id`,
          `${path}.effect_id is duplicated across effect sections.`,
        );
      }
      effectIds.add(effectId);
      const targetId = wireStableId(effect.target_id, `${path}.target_id`);
      const estimandId = wireStableId(effect.estimand_id, `${path}.estimand_id`);
      const traceCapability = validateGeneralSemTrace(effect.trace, `${path}.trace`, wireContext);
      if (capabilityCellIdentity(traceCapability)
        !== capabilityCellIdentity(
          GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CAPABILITY_CELL_V1,
        )) {
        wireFail(
          "document.invalid",
          `${path}.trace.capability_cell`,
          `${path}.trace.capability_cell must equal the supplemental two-way moderated-mediation cell.`,
        );
      }
      const moderatedStage = wireEnum(
        effect.moderated_stage,
        ["first_stage", "second_stage"] as const,
        `${path}.moderated_stage`,
      );
      const interactionId = wireStableId(effect.interaction_id, `${path}.interaction_id`);
      const xId = wireStableId(effect.x_id, `${path}.x_id`);
      const mediatorId = wireStableId(effect.mediator_id, `${path}.mediator_id`);
      const yId = wireStableId(effect.y_id, `${path}.y_id`);
      const moderatorId = wireStableId(effect.moderator_id, `${path}.moderator_id`);
      const orderedRelationIds = validateStableIdArray(
        effect.ordered_relation_ids,
        `${path}.ordered_relation_ids`,
        { minimum: 2 },
      );
      if (orderedRelationIds.length !== 2) {
        wireFail(
          "document.invalid",
          `${path}.ordered_relation_ids`,
          `${path}.ordered_relation_ids must contain exactly two relations.`,
        );
      }
      const probeValueIndex = wireU32(effect.probe_value_index, `${path}.probe_value_index`);
      if (probeValueIndex > 2 || probeIndices.has(probeValueIndex)) {
        wireFail(
          "document.invalid",
          `${path}.probe_value_index`,
          `${path}.probe_value_index must uniquely cover 0, 1, and 2.`,
        );
      }
      probeIndices.add(probeValueIndex);
      const moderatorValue = wireFinite(effect.moderator_value, `${path}.moderator_value`);
      if (!approximatelyEqualGeneralSem(moderatorValue, lockedProbes[probeValueIndex]!)) {
        wireFail(
          "document.invalid",
          `${path}.moderator_value`,
          `${path}.moderator_value must equal the locked standardized probe.`,
        );
      }
      if (effectId !== conditionalIndirectEffectIdentityV1(targetId, probeValueIndex)) {
        wireFail(
          "document.invalid",
          `${path}.effect_id`,
          `${path}.effect_id must equal the canonical target/probe identity.`,
        );
      }
      validateGeneralSemEstimate(effect.value, `${path}.value`);
      const valueEstimate = wireFinite(
        strictWireRecord(effect.value, `${path}.value`).estimate,
        `${path}.value.estimate`,
      );
      return {
        effectId,
        targetId,
        estimandId,
        moderatedStage,
        interactionId,
        xId,
        mediatorId,
        yId,
        moderatorId,
        orderedRelationIds,
        probeValueIndex,
        moderatorValue,
        valueEstimate,
      };
    });
    if (![0, 1, 2].every((index) => probeIndices.has(index))) {
      wireFail(
        "document.invalid",
        "general_sem_results.conditional_indirect_effects",
        "general_sem_results.conditional_indirect_effects must cover probe indices 0, 1, and 2 exactly.",
      );
    }

    const indexPath = "general_sem_results.moderated_mediation_indices[0]";
    const indexEffect = exactWireRecord(moderatedMediationIndices[0], [
      "effect_id", "target_id", "estimand_id", "trace", "moderated_stage",
      "interaction_id", "x_id", "mediator_id", "y_id", "moderator_id",
      "ordered_relation_ids", "value",
    ], [], indexPath);
    const indexEffectId = wireStableId(indexEffect.effect_id, `${indexPath}.effect_id`);
    if (effectIds.has(indexEffectId)) {
      wireFail(
        "document.invalid",
        `${indexPath}.effect_id`,
        `${indexPath}.effect_id is duplicated across effect sections.`,
      );
    }
    effectIds.add(indexEffectId);
    const indexTargetId = wireStableId(indexEffect.target_id, `${indexPath}.target_id`);
    const indexEstimandId = wireStableId(indexEffect.estimand_id, `${indexPath}.estimand_id`);
    const indexTraceCapability = validateGeneralSemTrace(
      indexEffect.trace,
      `${indexPath}.trace`,
      wireContext,
    );
    if (capabilityCellIdentity(indexTraceCapability)
      !== capabilityCellIdentity(
        GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CAPABILITY_CELL_V1,
      )) {
      wireFail(
        "document.invalid",
        `${indexPath}.trace.capability_cell`,
        `${indexPath}.trace.capability_cell must equal the supplemental two-way moderated-mediation cell.`,
      );
    }
    const indexStage = wireEnum(
      indexEffect.moderated_stage,
      ["first_stage", "second_stage"] as const,
      `${indexPath}.moderated_stage`,
    );
    const indexInteractionId = wireStableId(
      indexEffect.interaction_id,
      `${indexPath}.interaction_id`,
    );
    const indexXId = wireStableId(indexEffect.x_id, `${indexPath}.x_id`);
    const indexMediatorId = wireStableId(indexEffect.mediator_id, `${indexPath}.mediator_id`);
    const indexYId = wireStableId(indexEffect.y_id, `${indexPath}.y_id`);
    const indexModeratorId = wireStableId(indexEffect.moderator_id, `${indexPath}.moderator_id`);
    const indexRelationIds = validateStableIdArray(
      indexEffect.ordered_relation_ids,
      `${indexPath}.ordered_relation_ids`,
      { minimum: 2 },
    );
    if (indexRelationIds.length !== 2) {
      wireFail(
        "document.invalid",
        `${indexPath}.ordered_relation_ids`,
        `${indexPath}.ordered_relation_ids must contain exactly two relations.`,
      );
    }
    if (indexEffectId !== moderatedMediationIndexIdentityV1(indexTargetId)) {
      wireFail(
        "document.invalid",
        `${indexPath}.effect_id`,
        `${indexPath}.effect_id must equal the canonical target index identity.`,
      );
    }
    validateGeneralSemEstimate(indexEffect.value, `${indexPath}.value`);
    const indexEstimate = wireFinite(
      strictWireRecord(indexEffect.value, `${indexPath}.value`).estimate,
      `${indexPath}.value.estimate`,
    );

    const first = conditionalAuthorities[0]!;
    const targetSignature = (authority: typeof first): string => JSON.stringify([
      authority.targetId,
      authority.estimandId,
      authority.moderatedStage,
      authority.interactionId,
      authority.xId,
      authority.mediatorId,
      authority.yId,
      authority.moderatorId,
      authority.orderedRelationIds,
    ]);
    if (conditionalAuthorities.some((authority) => (
      targetSignature(authority) !== targetSignature(first)
    ))) {
      wireFail(
        "document.invalid",
        "general_sem_results.conditional_indirect_effects",
        "general_sem_results.conditional_indirect_effects must share one exact compiled target.",
      );
    }
    if (JSON.stringify([
      indexTargetId,
      indexEstimandId,
      indexStage,
      indexInteractionId,
      indexXId,
      indexMediatorId,
      indexYId,
      indexModeratorId,
      indexRelationIds,
    ]) !== targetSignature(first)) {
      wireFail(
        "document.invalid",
        indexPath,
        "general_sem_results.moderated_mediation_indices must identify the same target as the conditional indirect effects.",
      );
    }
    if (new Set([first.xId, first.mediatorId, first.yId, first.moderatorId]).size !== 4) {
      wireFail(
        "document.invalid",
        "general_sem_results.conditional_indirect_effects",
        "general_sem_results moderated mediation requires distinct X, M, Y, and W identities.",
      );
    }
    const interaction = strictWireRecord(
      interactionEffects[0],
      "general_sem_results.interaction_effects[0]",
    );
    if (interaction.interaction_id !== first.interactionId
      || interaction.moderator_id !== first.moderatorId) {
      wireFail(
        "document.invalid",
        "general_sem_results.interaction_effects[0]",
        "general_sem_results moderated-mediation target must bind the one published interaction effect.",
      );
    }
    const firstStage = first.moderatedStage === "first_stage";
    const moderatedRelationId = first.orderedRelationIds[firstStage ? 0 : 1]!;
    const otherRelationId = first.orderedRelationIds[firstStage ? 1 : 0]!;
    const expectedFocal = firstStage ? first.xId : first.mediatorId;
    const expectedOutcome = firstStage ? first.mediatorId : first.yId;
    if (interaction.focal_relation_id !== moderatedRelationId
      || interaction.focal_predictor_id !== expectedFocal
      || interaction.outcome_id !== expectedOutcome) {
      wireFail(
        "document.invalid",
        "general_sem_results.interaction_effects[0]",
        "general_sem_results interaction effect does not match the declared moderated path stage.",
      );
    }
    const moderatedBeta = jointStageEstimatesByRelation.get(moderatedRelationId);
    const otherBeta = jointStageEstimatesByRelation.get(otherRelationId);
    if (moderatedBeta == null || otherBeta == null) {
      wireFail(
        "document.invalid",
        "general_sem_results.joint_stage_structural_coefficients",
        "general_sem_results moderated-mediation formulas require both selected path coefficients in the joint-stage ledger.",
      );
    }
    const gamma = wireFinite(
      strictWireRecord(
        interaction.scientific_rescaled_gamma,
        "general_sem_results.interaction_effects[0].scientific_rescaled_gamma",
      ).estimate,
      "general_sem_results.interaction_effects[0].scientific_rescaled_gamma.estimate",
    );
    for (const effect of conditionalAuthorities) {
      const expected = (moderatedBeta + gamma * effect.moderatorValue) * otherBeta;
      if (!approximatelyEqualGeneralSem(effect.valueEstimate, expected)) {
        wireFail(
          "document.invalid",
          `general_sem_results.conditional_indirect_effects.${effect.effectId}`,
          `general_sem_results conditional indirect effect ${effect.effectId} contradicts the bounded formula.`,
        );
      }
    }
    if (!approximatelyEqualGeneralSem(indexEstimate, gamma * otherBeta)) {
      wireFail(
        "document.invalid",
        `${indexPath}.value.estimate`,
        "general_sem_results moderated-mediation index contradicts scientific gamma times the other-stage coefficient.",
      );
    }
  }

  const probeValues = new Map<string, {
    moderatorId: string;
    values: number[];
    capabilityIdentity: string;
    frozenInteractionPolicy: boolean;
  }>();
  probes.forEach((item, index) => {
    const path = `general_sem_results.conditional_effect_probes[${index}]`;
    const probe = exactWireRecord(item, ["probe_id", "trace", "moderator_id", "values"], [], path);
    const probeId = wireStableId(probe.probe_id, `${path}.probe_id`);
    const traceCapability = validateGeneralSemTrace(probe.trace, `${path}.trace`, wireContext);
    const values = validateConditionalProbeValues(probe.values, `${path}.values`);
    const valueRecord = strictWireRecord(probe.values, `${path}.values`);
    probeValues.set(probeId, {
      moderatorId: wireStableId(probe.moderator_id, `${path}.moderator_id`),
      values,
      capabilityIdentity: capabilityCellIdentity(traceCapability),
      frozenInteractionPolicy: valueRecord.kind === "explicit"
        && values.length === 3
        && approximatelyEqualGeneralSem(values[0]!, -1)
        && approximatelyEqualGeneralSem(values[1]!, 0)
        && approximatelyEqualGeneralSem(values[2]!, 1),
    });
  });

  const conditionalSignatures = new Set<string>();
  const interactionConditionalIndices = new Map<string, number[]>();
  conditional.forEach((item, index) => {
    const path = `general_sem_results.conditional_effects[${index}]`;
    const effect = exactWireRecord(item, [
      "effect_id", "estimand_id", "trace", "interaction_id", "focal_relation_id", "probe_id",
      "moderator_id", "probe_value_index", "moderator_value", "value",
    ], ["interaction_effect_id"], path);
    const effectId = wireStableId(effect.effect_id, `${path}.effect_id`);
    if (effectIds.has(effectId)) wireFail("document.invalid", `${path}.effect_id`, `${path}.effect_id is duplicated across effect sections.`);
    effectIds.add(effectId);
    const estimandId = wireStableId(effect.estimand_id, `${path}.estimand_id`);
    const traceCapability = validateGeneralSemTrace(effect.trace, `${path}.trace`, wireContext);
    const traceCapabilityIdentity = capabilityCellIdentity(traceCapability);
    const interactionId = wireStableId(effect.interaction_id, `${path}.interaction_id`);
    const focalRelationId = wireStableId(effect.focal_relation_id, `${path}.focal_relation_id`);
    const probeId = wireStableId(effect.probe_id, `${path}.probe_id`);
    const moderatorId = wireStableId(effect.moderator_id, `${path}.moderator_id`);
    const interactionEffectId = Object.prototype.hasOwnProperty.call(effect, "interaction_effect_id")
      ? wireStableId(effect.interaction_effect_id, `${path}.interaction_effect_id`)
      : null;
    if (interactionAuthorities.size > 0 && interactionEffectId == null) {
      wireFail(
        "document.invalid",
        `${path}.interaction_effect_id`,
        `${path}.interaction_effect_id is required when interaction_effects are present.`,
      );
    }
    if (interactionEffectId != null) {
      const authority = interactionAuthorities.get(interactionEffectId);
      if (!authority) {
        wireFail(
          "document.invalid",
          `${path}.interaction_effect_id`,
          `${path}.interaction_effect_id references a missing interaction effect.`,
        );
      }
      if (authority.interactionId !== interactionId
        || authority.focalRelationId !== focalRelationId
        || authority.moderatorId !== moderatorId
        || authority.capabilityIdentity !== traceCapabilityIdentity) {
        wireFail("document.invalid", path, `${path} contradicts its interaction effect authority.`);
      }
      const indices = interactionConditionalIndices.get(interactionEffectId) ?? [];
      indices.push(wireU32(effect.probe_value_index, `${path}.probe_value_index`));
      interactionConditionalIndices.set(interactionEffectId, indices);
    }
    const probeValueIndex = wireU32(effect.probe_value_index, `${path}.probe_value_index`);
    const moderatorValue = wireFinite(effect.moderator_value, `${path}.moderator_value`);
    const probe = probeValues.get(probeId);
    if (!probe) wireFail("document.invalid", `${path}.probe_id`, `${path}.probe_id references a missing probe.`);
    if (probe.moderatorId !== moderatorId) wireFail("document.invalid", `${path}.moderator_id`, `${path}.moderator_id contradicts its probe.`);
    if (probe.capabilityIdentity !== traceCapabilityIdentity) {
      wireFail("document.invalid", `${path}.trace`, `${path}.trace contradicts its probe authority.`);
    }
    if (interactionEffectId != null && !probe.frozenInteractionPolicy) {
      wireFail(
        "document.invalid",
        `${path}.probe_id`,
        `${path}.probe_id must use the frozen standardized -1/0/+1 interaction policy.`,
      );
    }
    const expectedValue = probe.values[probeValueIndex];
    if (expectedValue === undefined || !approximatelyEqualGeneralSem(moderatorValue, expectedValue)) {
      wireFail("document.invalid", `${path}.moderator_value`, `${path}.moderator_value contradicts its probe value.`);
    }
    const signature = `${estimandId}\0${interactionId}\0${focalRelationId}\0${probeId}\0${probeValueIndex}`;
    if (conditionalSignatures.has(signature)) wireFail("document.invalid", path, `${path} duplicates another conditional scientific effect.`);
    conditionalSignatures.add(signature);
    validateGeneralSemEstimate(effect.value, `${path}.value`);
  });

  const interactionPlotCounts = new Map<string, number>();
  plots.forEach((item, index) => {
    const path = `general_sem_results.interaction_plots[${index}]`;
    const plot = exactWireRecord(item, [
      "plot_id", "trace", "interaction_id", "focal_relation_id", "focal_predictor_id",
      "moderator_id", "outcome_id", "series",
    ], ["interaction_effect_id"], path);
    wireStableId(plot.plot_id, `${path}.plot_id`);
    const traceCapability = validateGeneralSemTrace(plot.trace, `${path}.trace`, wireContext);
    const traceCapabilityIdentity = capabilityCellIdentity(traceCapability);
    const interactionId = wireStableId(plot.interaction_id, `${path}.interaction_id`);
    const focalRelationId = wireStableId(plot.focal_relation_id, `${path}.focal_relation_id`);
    const focalId = wireStableId(plot.focal_predictor_id, `${path}.focal_predictor_id`);
    const moderatorId = wireStableId(plot.moderator_id, `${path}.moderator_id`);
    const outcomeId = wireStableId(plot.outcome_id, `${path}.outcome_id`);
    const interactionEffectId = Object.prototype.hasOwnProperty.call(plot, "interaction_effect_id")
      ? wireStableId(plot.interaction_effect_id, `${path}.interaction_effect_id`)
      : null;
    if (interactionAuthorities.size > 0 && interactionEffectId == null) {
      wireFail(
        "document.invalid",
        `${path}.interaction_effect_id`,
        `${path}.interaction_effect_id is required when interaction_effects are present.`,
      );
    }
    if (interactionEffectId != null) {
      const authority = interactionAuthorities.get(interactionEffectId);
      if (!authority) {
        wireFail(
          "document.invalid",
          `${path}.interaction_effect_id`,
          `${path}.interaction_effect_id references a missing interaction effect.`,
        );
      }
      if (authority.interactionId !== interactionId
        || authority.focalRelationId !== focalRelationId
        || authority.focalPredictorId !== focalId
        || authority.moderatorId !== moderatorId
        || authority.outcomeId !== outcomeId
        || authority.capabilityIdentity !== traceCapabilityIdentity) {
        wireFail("document.invalid", path, `${path} contradicts its interaction effect authority.`);
      }
    }
    if (new Set([focalId, moderatorId, outcomeId]).size !== 3) {
      wireFail("document.invalid", path, `${path} requires distinct focal, moderator, and outcome identities.`);
    }
    const seriesValues = wireArray(plot.series, `${path}.series`);
    if (seriesValues.length === 0) wireFail("document.invalid", `${path}.series`, `${path}.series must not be empty.`);
    if (interactionEffectId != null) {
      interactionPlotCounts.set(
        interactionEffectId,
        (interactionPlotCounts.get(interactionEffectId) ?? 0) + 1,
      );
      if (seriesValues.length !== 3) {
        wireFail(
          "document.invalid",
          `${path}.series`,
          `${path}.series must contain exactly the frozen -1/0/+1 interaction probes.`,
        );
      }
    }
    validateCanonicalWireIds(seriesValues, "series_id", `${path}.series`);
    let commonGrid: number[] | null = null;
    const linkedSeriesProbeIndices = new Set<number>();
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
      if (probe.capabilityIdentity !== traceCapabilityIdentity) {
        wireFail("document.invalid", `${seriesPath}.probe_id`, `${seriesPath}.probe_id uses a different capability authority.`);
      }
      if (interactionEffectId != null && !probe.frozenInteractionPolicy) {
        wireFail(
          "document.invalid",
          `${seriesPath}.probe_id`,
          `${seriesPath}.probe_id must use the frozen standardized -1/0/+1 interaction policy.`,
        );
      }
      if (interactionEffectId != null) linkedSeriesProbeIndices.add(probeValueIndex);
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
    if (interactionEffectId != null
      && (linkedSeriesProbeIndices.size !== 3
        || ![0, 1, 2].every((probeIndex) => linkedSeriesProbeIndices.has(probeIndex)))) {
      wireFail(
        "document.invalid",
        `${path}.series`,
        `${path}.series must cover probe indices 0, 1, and 2 exactly.`,
      );
    }
  });

  if (!threeWayJointPoint) interactionAuthorities.forEach((_authority, effectId) => {
    const indices = interactionConditionalIndices.get(effectId) ?? [];
    if (indices.length !== 3
      || new Set(indices).size !== 3
      || ![0, 1, 2].every((probeIndex) => indices.includes(probeIndex))) {
      wireFail(
        "document.invalid",
        "general_sem_results.interaction_effects",
        "general_sem_results.interaction_effects must each have exactly three conditional rows at probe indices 0, 1, and 2.",
      );
    }
    if (interactionPlotCounts.get(effectId) !== 1) {
      wireFail(
        "document.invalid",
        "general_sem_results.interaction_effects",
        "general_sem_results.interaction_effects must each have exactly one cross-referenced interaction plot.",
      );
    }
  });

  const hocSignatures = new Set<string>();
  hocStages.forEach((item, index) => {
    const path = `general_sem_results.higher_order_stages[${index}]`;
    const stage = exactWireRecord(item, [
      "stage_id", "trace", "higher_order_construct_id", "stage_number", "kind",
      "input_construct_ids", "output_variable_ids",
    ], [
      "approach", "measurement_type", "generated_variable_mappings", "receipt",
      "relation_estimates",
    ], path);
    wireStableId(stage.stage_id, `${path}.stage_id`);
    validateGeneralSemTrace(stage.trace, `${path}.trace`, wireContext);
    const hocId = wireStableId(stage.higher_order_construct_id, `${path}.higher_order_construct_id`);
    const stageNumber = wireU32(stage.stage_number, `${path}.stage_number`);
    const kind = wireEnum(stage.kind, ["lower_order_score_estimation", "higher_order_estimation"] as const, `${path}.kind`);
    const signature = `${hocId}\0${stageNumber}`;
    if (hocSignatures.has(signature)) wireFail("document.invalid", path, `${path} duplicates a higher-order construct stage.`);
    hocSignatures.add(signature);
    validateStableIdArray(stage.input_construct_ids, `${path}.input_construct_ids`, { minimum: 1, canonical: true });
    validateStableIdArray(stage.output_variable_ids, `${path}.output_variable_ids`, { minimum: 1, canonical: true });
    const approach = stage.approach == null ? null : wireEnum(stage.approach, [
      "repeated_indicators", "extended_repeated_indicators", "embedded_two_stage",
      "disjoint_two_stage", "hybrid",
    ] as const, `${path}.approach`);
    const measurementType = stage.measurement_type == null ? null : wireEnum(stage.measurement_type, [
      "reflective_reflective", "reflective_formative", "formative_reflective",
      "formative_formative",
    ] as const, `${path}.measurement_type`);
    if ((approach == null) !== (measurementType == null)) {
      wireFail("document.invalid", path, `${path}.approach and measurement_type must be present together.`);
    }
    const additiveOneStageHigherOrder = stageNumber === 1
      && kind === "higher_order_estimation"
      && (approach === "repeated_indicators" || approach === "extended_repeated_indicators");
    const historicalStageShape = (stageNumber === 1 && kind === "lower_order_score_estimation")
      || (stageNumber === 2 && kind === "higher_order_estimation");
    if (!historicalStageShape && !additiveOneStageHigherOrder) {
      wireFail("document.invalid", `${path}.stage_number`, `${path}.stage_number contradicts its stage kind.`);
    }
    const mappings = optionalWireArray(stage, "generated_variable_mappings", path);
    validateCanonicalWireIds(mappings, "component_id", `${path}.generated_variable_mappings`);
    mappings.forEach((mappingValue, mappingIndex) => {
      const mappingPath = `${path}.generated_variable_mappings[${mappingIndex}]`;
      const mapping = exactWireRecord(mappingValue, [
        "component_id", "generated_score_variable_id", "generated_component_relation_id",
        "generated_component_parameter_id", "component_relation_source_id",
        "component_relation_target_id", "relation_interpretation",
      ], [], mappingPath);
      for (const key of [
        "component_id", "generated_score_variable_id", "generated_component_relation_id",
        "generated_component_parameter_id", "component_relation_source_id",
        "component_relation_target_id",
      ] as const) wireStableId(mapping[key], `${mappingPath}.${key}`);
      wireEnum(mapping.relation_interpretation, ["loading", "weight_and_collinearity"] as const, `${mappingPath}.relation_interpretation`);
    });
    if (stage.receipt != null) {
      const receiptPath = `${path}.receipt`;
      const receipt = exactWireRecord(stage.receipt, [
        "receipt_version", "stage_number", "role", "projection_identity_sha256",
        "model_scientific_sha256", "compiled_plan_sha256", "dataset_fingerprint",
        "used_observations", "omitted_observations",
      ], ["generated_score_dataset"], receiptPath);
      if (receipt.receipt_version !== GENERAL_SEM_PLS_HIGHER_ORDER_POINT_STAGE_RECEIPT_VERSION_V1
        || wireU32(receipt.stage_number, `${receiptPath}.stage_number`) !== stageNumber) {
        wireFail("document.invalid", receiptPath, `${receiptPath} differs from the exact point-stage receipt contract.`);
      }
      wireEnum(receipt.role, [
        "repeated_indicator_estimation", "extended_repeated_indicator_estimation",
        "embedded_repeated_indicator_estimation", "disjoint_lower_order_score_estimation",
        "higher_order_from_lower_order_scores",
      ] as const, `${receiptPath}.role`);
      for (const key of [
        "projection_identity_sha256", "model_scientific_sha256", "compiled_plan_sha256",
      ] as const) wireGeneralSemSha256(receipt[key], `${receiptPath}.${key}`);
      wireGeneralSemDatasetFingerprint(receipt.dataset_fingerprint, `${receiptPath}.dataset_fingerprint`);
      const used = wireU32(receipt.used_observations, `${receiptPath}.used_observations`);
      wireU32(receipt.omitted_observations, `${receiptPath}.omitted_observations`);
      if (used === 0) wireFail("document.invalid", `${receiptPath}.used_observations`, `${receiptPath}.used_observations must be positive.`);
      if (receipt.generated_score_dataset != null) {
        const scorePath = `${receiptPath}.generated_score_dataset`;
        const score = exactWireRecord(receipt.generated_score_dataset, [
          "receipt_version", "source_dataset_fingerprint", "complete_case_row_count",
          "omitted_row_count", "complete_case_rows_sha256", "generated_score_columns",
        ], [], scorePath);
        if (score.receipt_version !== GENERAL_SEM_PLS_DISJOINT_HOC_SCORE_DATASET_RECEIPT_VERSION_V1) {
          wireFail("document.invalid", `${scorePath}.receipt_version`, `${scorePath}.receipt_version is not the exact disjoint score receipt.`);
        }
        wireGeneralSemDatasetFingerprint(score.source_dataset_fingerprint, `${scorePath}.source_dataset_fingerprint`);
        const completeRows = wireU32(score.complete_case_row_count, `${scorePath}.complete_case_row_count`);
        wireU32(score.omitted_row_count, `${scorePath}.omitted_row_count`);
        wireGeneralSemSha256(score.complete_case_rows_sha256, `${scorePath}.complete_case_rows_sha256`);
        const columns = wireArray(score.generated_score_columns, `${scorePath}.generated_score_columns`);
        validateCanonicalWireIds(columns, "component_id", `${scorePath}.generated_score_columns`);
        columns.forEach((columnValue, columnIndex) => {
          const columnPath = `${scorePath}.generated_score_columns[${columnIndex}]`;
          const column = exactWireRecord(columnValue, [
            "component_id", "generated_score_variable_id", "observation_count", "values_sha256",
          ], [], columnPath);
          wireStableId(column.component_id, `${columnPath}.component_id`);
          wireStableId(column.generated_score_variable_id, `${columnPath}.generated_score_variable_id`);
          if (wireU32(column.observation_count, `${columnPath}.observation_count`) !== completeRows) {
            wireFail("document.invalid", `${columnPath}.observation_count`, `${columnPath}.observation_count contradicts the score receipt.`);
          }
          wireGeneralSemSha256(column.values_sha256, `${columnPath}.values_sha256`);
        });
      }
    }
    const relations = optionalWireArray(stage, "relation_estimates", path);
    validateCanonicalWireIds(relations, "relation_id", `${path}.relation_estimates`);
    relations.forEach((relationValue, relationIndex) => {
      const relationPath = `${path}.relation_estimates[${relationIndex}]`;
      const relation = exactWireRecord(
        relationValue,
        ["relation_id", "source_id", "target_id", "value"],
        ["parameter_id", "kind", "collinearity_vif"],
        relationPath,
      );
      wireStableId(relation.relation_id, `${relationPath}.relation_id`);
      const parameterId = relation.parameter_id == null
        ? null
        : wireStableId(relation.parameter_id, `${relationPath}.parameter_id`);
      const relationKind = relation.kind == null ? null : wireEnum(relation.kind, [
        "component_loading", "component_weight", "authored_structural", "authored_control",
        "technical_structural", "extended_indirect_effect", "extended_total_effect",
      ] as const, `${relationPath}.kind`);
      if ((parameterId == null) !== (relationKind == null)) {
        wireFail("document.invalid", relationPath, `${relationPath}.parameter_id and kind must be present together.`);
      }
      const sourceId = wireStableId(relation.source_id, `${relationPath}.source_id`);
      const targetId = wireStableId(relation.target_id, `${relationPath}.target_id`);
      if (sourceId === targetId) wireFail("document.invalid", relationPath, `${relationPath} requires distinct source_id and target_id.`);
      const vif = optionalWireFinite(relation, "collinearity_vif", relationPath);
      if (vif != null && vif <= 0) wireFail("document.invalid", `${relationPath}.collinearity_vif`, `${relationPath}.collinearity_vif must be positive.`);
      validateGeneralSemEstimate(relation.value, `${relationPath}.value`);
    });
  });

  const cbsemPointRows = validateCbsemParameterRowsV1(cbsemParameters, wireContext);

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
    const status = wireEnum(diagnostic.status, ["identified", "provisional", "underidentified", "locally_underidentified", "boundary_condition"] as const, `${path}.status`);
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
    results.higher_order_inference_receipt,
    specific,
    aggregate,
    jointStageCoefficients,
    interactionEffects,
    conditional,
    conditionalIndirect,
    moderatedMediationIndices,
    plots,
    hocStages,
    wireContext,
  );
  validateCbsemBootstrapV1(
    results.cbsem_bootstrap_receipt,
    cbsemBootstrapInference,
    cbsemPointRows,
    wireContext,
  );
  validateHocInferenceReceiptV1(
    results.higher_order_inference_receipt,
    results.inference_receipt,
    hocStages,
    wireContext,
  );
  validateThreeWayModerationResultsV1(
    threeWayInteractionEffects,
    threeWayConditionalEffects,
    threeWaySimpleSlopes,
    results.three_way_moderation_bootstrap_receipt,
    wireContext,
  );

  return value as CanonicalGeneralSemResultsV1;
}
