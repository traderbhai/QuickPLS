export const SEM_MODEL_V4_SCHEMA_VERSION = 4 as const;

export type ObservedScaleV4 = "continuous" | "binary" | "ordinal" | "nominal" | "identifier";
export type ObservedRoleV4 = "indicator" | "structural" | "both" | "control";
export type StructuralRelationRoleV4 = "structural" | "control";
export type CompositeModeV4 = "mode_a" | "mode_b";

export type CompositeWeightNormalizationV4 = "none" | "sum_to_one" | "unit_variance";
export type CompositeWeightingV4 =
  | { kind: "mode_a" }
  | { kind: "mode_b" }
  | { kind: "unit"; normalization: CompositeWeightNormalizationV4 }
  | { kind: "custom"; weights: Record<string, number>; normalization: CompositeWeightNormalizationV4 };

export type RecodeUnmappedPolicyV4 = "keep" | "set_missing" | "reject";
export type StandardizationDenominatorV4 = "sample" | "population";
export type ObservedTransformationOperationV4 =
  | { kind: "recode"; mappings: Record<string, string>; unmapped: RecodeUnmappedPolicyV4 }
  | { kind: "mean_center" }
  | { kind: "standardize"; denominator: StandardizationDenominatorV4 }
  | { kind: "log"; base: number; offset: number }
  | { kind: "formula"; expression: string };

export interface ObservedTransformationStepV4 {
  id: string;
  input_columns: string[];
  output_column: string;
  operation: ObservedTransformationOperationV4;
}

export type FactorIdentificationV4 =
  | { kind: "marker_loading"; indicator: string }
  | { kind: "fixed_variance" }
  | { kind: "effects_coding" };

export type FactorMeanPolicyV4 =
  | { kind: "fixed_zero" }
  | { kind: "estimated"; parameter: string }
  | { kind: "reference_group"; reference_group: string; parameter: string };

export type FactorDisturbancePolicyV4 =
  | { kind: "exogenous_variance"; parameter: string }
  | { kind: "endogenous_disturbance"; parameter: string }
  | { kind: "fixed_zero"; parameter: string };

export type SemVariableV4 =
  | {
    kind: "observed";
    id: string;
    label: string;
    source_column: string;
    scale: ObservedScaleV4;
    role: ObservedRoleV4;
    categories: string[];
    value_labels: Record<string, string>;
    missing_markers: string[];
    transformation_lineage: ObservedTransformationStepV4[];
  }
  | {
    kind: "common_factor";
    id: string;
    label: string;
    identification: FactorIdentificationV4;
    mean_policy: FactorMeanPolicyV4;
    disturbance_policy: FactorDisturbancePolicyV4;
  }
  | { kind: "composite"; id: string; label: string; weighting: CompositeWeightingV4 }
  | { kind: "derived"; id: string; label: string };

export type SemEndpointV4 =
  | { kind: "variable"; id: string }
  | { kind: "residual_of"; id: string }
  | { kind: "disturbance_of"; id: string };

export type SemRelationV4 =
  | {
    kind: "measurement_effect";
    id: string;
    construct: string;
    indicator: string;
    parameter: string;
  }
  | {
    kind: "measurement_causal";
    id: string;
    indicator: string;
    composite: string;
    parameter: string;
  }
  | {
    kind: "structural";
    id: string;
    source: string;
    target: string;
    parameter: string;
    role?: StructuralRelationRoleV4;
    intercept_parameter?: string | null;
  }
  | {
    kind: "covariance";
    id: string;
    left: SemEndpointV4;
    right: SemEndpointV4;
    parameter: string;
  };

export type SemParameterTargetV4 =
  | { kind: "loading"; construct: string; indicator: string }
  | { kind: "weight"; indicator: string; composite: string }
  | { kind: "regression"; source: string; target: string }
  | { kind: "variance"; endpoint: SemEndpointV4 }
  | { kind: "covariance"; left: SemEndpointV4; right: SemEndpointV4 }
  | { kind: "intercept"; variable: string }
  | { kind: "mean"; variable: string }
  | { kind: "threshold"; variable: string; index: number };

export type SemParameterGroupOverrideSpecV4 =
  | { kind: "free"; start?: number | null; lower?: number | null; upper?: number | null }
  | { kind: "fixed"; value: number };

export interface SemParameterGroupOverrideV4 {
  group: string;
  specification: SemParameterGroupOverrideSpecV4;
}

export type SemParameterV4 =
  | {
    kind: "free";
    id: string;
    label: string;
    target: SemParameterTargetV4;
    start?: number | null;
    lower?: number | null;
    upper?: number | null;
    equality_label?: string | null;
    group_overrides?: SemParameterGroupOverrideV4[];
  }
  | { kind: "fixed"; id: string; label: string; target: SemParameterTargetV4; value: number; group_overrides?: SemParameterGroupOverrideV4[] }
  | { kind: "derived"; id: string; label: string; target: SemParameterTargetV4; expression: string; group_overrides?: SemParameterGroupOverrideV4[] };

export interface SemLinearConstraintTermV4 {
  parameter: string;
  coefficient: number;
}

export type SemConstraintV4 =
  | { kind: "equality"; id: string; parameters: string[] }
  | { kind: "bound"; id: string; parameter: string; lower?: number | null; upper?: number | null }
  | { kind: "linear"; id: string; terms: SemLinearConstraintTermV4[]; value: number };

export type InteractionMethodV4 = "product_indicator" | "two_stage" | "orthogonalizing";
export type ProductIndicatorCenteringV4 = "none" | "mean_center" | "double_mean_center";
export type ProductIndicatorStandardizationV4 = "none" | "sample_standard_deviation";
export type ProductIndicatorPairingV4 = "all_pairs";
export interface ProductIndicatorSpecificationV4 {
  centering: ProductIndicatorCenteringV4;
  standardization: ProductIndicatorStandardizationV4;
  pairing: ProductIndicatorPairingV4;
}
export type HigherOrderConstructionApproachV4 =
  | "repeated_indicators"
  | "extended_repeated_indicators"
  | "embedded_two_stage"
  | "disjoint_two_stage"
  | "hybrid";
export type HigherOrderMeasurementTypeV4 =
  | "reflective_reflective"
  | "reflective_formative"
  | "formative_reflective"
  | "formative_formative";

export type SemDerivedTermV4 =
  | {
    kind: "interaction";
    id: string;
    output: string;
    predictor: string;
    moderator: string;
    focal_relation: string;
    method: InteractionMethodV4;
    product_indicator?: ProductIndicatorSpecificationV4 | null;
  }
  | {
    kind: "higher_order";
    id: string;
    output: string;
    components: string[];
    approach: HigherOrderConstructionApproachV4;
    measurement_type: HigherOrderMeasurementTypeV4;
  }
  | { kind: "polynomial"; id: string; output: string; source: string; degree: number };

export interface SemGroupLevelV4 {
  id: string;
  value: string;
  label: string;
}

export type SemGroupV4 =
  | { kind: "single_group" }
  | { kind: "observed_groups"; grouping_variable: string; levels: SemGroupLevelV4[] };

export type MissingDataPolicyV4 =
  | "listwise_deletion"
  | "pairwise_deletion"
  | "mean_replacement"
  | "full_information_maximum_likelihood"
  | { multiple_imputation: { imputations: number } };

export type SamplingWeightNormalizationV4 = "none" | "mean_one" | "sum_to_sample_size";
export type SemCovarianceDenominatorV4 = "sample_n_minus_one" | "maximum_likelihood_n";
export type SemWeightBindingV4 =
  | { kind: "case"; variable: string }
  | { kind: "frequency"; variable: string }
  | { kind: "sampling"; variable: string; normalization: SamplingWeightNormalizationV4 };

export function parseSemWeightBindingV4(
  value: unknown,
  path = "model.data_binding.weight",
): SemWeightBindingV4 {
  const binding = strictRecord(value, path);
  if (binding.kind === "case" || binding.kind === "frequency") {
    strictKeys(binding, ["kind", "variable"], path);
  } else if (binding.kind === "sampling") {
    strictKeys(binding, ["kind", "variable", "normalization"], path);
    if (binding.normalization !== "none"
      && binding.normalization !== "mean_one"
      && binding.normalization !== "sum_to_sample_size") {
      throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.normalization`);
    }
  } else {
    throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
  }
  if (typeof binding.variable !== "string" || !binding.variable.trim()) {
    throw new SemModelV4OperationError("schema.invalid_shape", `${path}.variable`);
  }
  return binding as unknown as SemWeightBindingV4;
}

export interface SemMatrixSampleMetadataV4 {
  sample_size: number;
  covariance_denominator: SemCovarianceDenominatorV4;
  effective_sample_size?: number | null;
  degrees_of_freedom?: number | null;
  group_sample_sizes?: Record<string, number>;
}

export type SemDataBindingV4 =
  | {
    kind: "raw";
    dataset_id: string;
    missing_data: MissingDataPolicyV4;
    weight?: SemWeightBindingV4 | null;
    cluster_variable?: string | null;
    strata_variable?: string | null;
  }
  | {
    kind: "covariance";
    dataset_id: string;
    variables: string[];
    means?: Record<string, number> | null;
    standard_deviations?: Record<string, number> | null;
    sample: SemMatrixSampleMetadataV4;
  }
  | {
    kind: "correlation";
    dataset_id: string;
    variables: string[];
    means?: Record<string, number> | null;
    standard_deviations?: Record<string, number> | null;
    sample: SemMatrixSampleMetadataV4;
  };

export type SemAnnotationV4 =
  | { kind: "display_only_covariance"; id: string; left: string; right: string; label?: string | null }
  | { kind: "caption"; id: string; text: string }
  | { kind: "note"; id: string; subject: string; text: string };

export interface SemCanvasNodeV4 {
  variable: string;
  x: number;
  y: number;
  style?: Record<string, string>;
}

export interface SemCanvasEdgeV4 {
  relation: string;
  routing?: string | null;
}

export type SemCanvasShapeKindV4 = "rectangle" | "rounded_rectangle" | "ellipse" | "diamond";
export interface SemCanvasShapeV4 {
  id: string;
  shape: SemCanvasShapeKindV4;
  x: number;
  y: number;
  width: number;
  height: number;
  label?: string | null;
  style?: Record<string, string>;
}

export interface SemCanvasImageV4 {
  id: string;
  asset_ref: string;
  alt_text: string;
  x: number;
  y: number;
  width: number;
  height: number;
  style?: Record<string, string>;
}

export interface SemCanvasLineV4 {
  id: string;
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  label?: string | null;
  start_marker?: string | null;
  end_marker?: string | null;
  style?: Record<string, string>;
}

export type SemPresentationV4 =
  | { kind: "none" }
  | {
    kind: "canvas";
    nodes: SemCanvasNodeV4[];
    edges: SemCanvasEdgeV4[];
    shapes: SemCanvasShapeV4[];
    images: SemCanvasImageV4[];
    lines: SemCanvasLineV4[];
    zoom?: number | null;
    pan_x?: number | null;
    pan_y?: number | null;
  };

export interface SemModelV4 {
  schema_version: typeof SEM_MODEL_V4_SCHEMA_VERSION;
  id: string;
  name: string;
  variables: SemVariableV4[];
  relations: SemRelationV4[];
  parameters: SemParameterV4[];
  constraints: SemConstraintV4[];
  derived_terms: SemDerivedTermV4[];
  group: SemGroupV4;
  data_binding: SemDataBindingV4;
  annotations: SemAnnotationV4[];
  presentation: SemPresentationV4;
}

export interface SemModelV4Issue {
  code: string;
  subject: string | null;
  message: string;
}

const issue = (code: string, subject: string | null, message: string): SemModelV4Issue => ({ code, subject, message });
const endpointKey = (endpoint: SemEndpointV4) => `${endpoint.kind}\0${endpoint.id}`;
const canonicalEndpointPair = (left: SemEndpointV4, right: SemEndpointV4): [SemEndpointV4, SemEndpointV4] =>
  endpointKey(left) <= endpointKey(right) ? [left, right] : [right, left];

function canonicalTarget(target: SemParameterTargetV4): SemParameterTargetV4 {
  if (target.kind !== "covariance") return target;
  const [left, right] = canonicalEndpointPair(target.left, target.right);
  return { ...target, left, right };
}

function targetEquals(left: SemParameterTargetV4, right: SemParameterTargetV4): boolean {
  return stableStringify(canonicalTarget(left)) === stableStringify(canonicalTarget(right));
}

type UnknownRecord = Record<string, unknown>;

function strictRecord(value: unknown, path: string): UnknownRecord {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new SemModelV4OperationError("schema.invalid_shape", path);
  return value as UnknownRecord;
}

function strictKeys(value: unknown, allowed: readonly string[], path: string): UnknownRecord {
  const record = strictRecord(value, path);
  const unknown = Object.keys(record).find((key) => !allowed.includes(key));
  if (unknown) throw new SemModelV4OperationError("schema.unknown_field", `${path}.${unknown}`);
  return record;
}

function strictArray(value: unknown, path: string, visit: (child: unknown, childPath: string) => void) {
  if (!Array.isArray(value)) throw new SemModelV4OperationError("schema.invalid_shape", path);
  value.forEach((child, index) => visit(child, `${path}[${index}]`));
}

function hasOwn(record: UnknownRecord, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(record, key);
}

function exactRecord(value: unknown, required: readonly string[], optional: readonly string[], path: string): UnknownRecord {
  const record = strictKeys(value, [...required, ...optional], path);
  const missing = required.find((key) => !hasOwn(record, key));
  if (missing) throw new SemModelV4OperationError("schema.invalid_shape", `${path}.${missing}`);
  return record;
}

function textValue(value: unknown, path: string): string {
  if (typeof value !== "string") throw new SemModelV4OperationError("schema.invalid_shape", path);
  return value;
}

function finiteValue(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || Object.is(value, -0)) {
    throw new SemModelV4OperationError("schema.invalid_shape", path);
  }
  return value;
}

function integerValue(value: unknown, maximum: number, path: string): number {
  if (!Number.isSafeInteger(value) || Object.is(value, -0) || (value as number) < 0 || (value as number) > maximum) {
    throw new SemModelV4OperationError("schema.invalid_shape", path);
  }
  return value as number;
}

function enumValue<T extends string>(value: unknown, allowed: readonly T[], path: string): T {
  if (typeof value !== "string" || !allowed.includes(value as T)) {
    throw new SemModelV4OperationError("schema.invalid_discriminator", path);
  }
  return value as T;
}

function arrayValue<T>(value: unknown, path: string, parse: (item: unknown, itemPath: string) => T): T[] {
  if (!Array.isArray(value)) throw new SemModelV4OperationError("schema.invalid_shape", path);
  return value.map((item, index) => parse(item, `${path}[${index}]`));
}

function textArrayValue(value: unknown, path: string): string[] {
  return arrayValue(value, path, textValue);
}

function textMapValue(value: unknown, path: string): Record<string, string> {
  const record = strictRecord(value, path);
  return Object.fromEntries(Object.entries(record).map(([key, item]) => [key, textValue(item, `${path}.${key}`)]));
}

function numberMapValue(value: unknown, path: string): Record<string, number> {
  const record = strictRecord(value, path);
  return Object.fromEntries(Object.entries(record).map(([key, item]) => [key, finiteValue(item, `${path}.${key}`)]));
}

function countMapValue(value: unknown, path: string): Record<string, number> {
  const record = strictRecord(value, path);
  return Object.fromEntries(Object.entries(record).map(([key, item]) => [key, integerValue(item, Number.MAX_SAFE_INTEGER, `${path}.${key}`)]));
}

function optionalValue<T>(record: UnknownRecord, key: string, path: string, parse: (value: unknown, path: string) => T): T | null {
  if (!hasOwn(record, key) || record[key] == null) return null;
  return parse(record[key], `${path}.${key}`);
}

function defaultArrayValue<T>(record: UnknownRecord, key: string, path: string, parse: (item: unknown, itemPath: string) => T): T[] {
  return hasOwn(record, key) ? arrayValue(record[key], `${path}.${key}`, parse) : [];
}

function defaultTextMapValue(record: UnknownRecord, key: string, path: string): Record<string, string> {
  return hasOwn(record, key) ? textMapValue(record[key], `${path}.${key}`) : {};
}

function parseEndpoint(value: unknown, path: string): SemEndpointV4 {
  const endpoint = exactRecord(value, ["kind", "id"], [], path);
  return {
    kind: enumValue(endpoint.kind, ["variable", "residual_of", "disturbance_of"] as const, `${path}.kind`),
    id: textValue(endpoint.id, `${path}.id`),
  };
}

function parseTarget(value: unknown, path: string): SemParameterTargetV4 {
  const candidate = strictRecord(value, path);
  switch (candidate.kind) {
    case "loading": {
      const target = exactRecord(candidate, ["kind", "construct", "indicator"], [], path);
      return { kind: "loading", construct: textValue(target.construct, `${path}.construct`), indicator: textValue(target.indicator, `${path}.indicator`) };
    }
    case "weight": {
      const target = exactRecord(candidate, ["kind", "indicator", "composite"], [], path);
      return { kind: "weight", indicator: textValue(target.indicator, `${path}.indicator`), composite: textValue(target.composite, `${path}.composite`) };
    }
    case "regression": {
      const target = exactRecord(candidate, ["kind", "source", "target"], [], path);
      return { kind: "regression", source: textValue(target.source, `${path}.source`), target: textValue(target.target, `${path}.target`) };
    }
    case "variance": {
      const target = exactRecord(candidate, ["kind", "endpoint"], [], path);
      return { kind: "variance", endpoint: parseEndpoint(target.endpoint, `${path}.endpoint`) };
    }
    case "covariance": {
      const target = exactRecord(candidate, ["kind", "left", "right"], [], path);
      return { kind: "covariance", left: parseEndpoint(target.left, `${path}.left`), right: parseEndpoint(target.right, `${path}.right`) };
    }
    case "intercept":
    case "mean": {
      const target = exactRecord(candidate, ["kind", "variable"], [], path);
      return { kind: candidate.kind, variable: textValue(target.variable, `${path}.variable`) };
    }
    case "threshold": {
      const target = exactRecord(candidate, ["kind", "variable", "index"], [], path);
      return { kind: "threshold", variable: textValue(target.variable, `${path}.variable`), index: integerValue(target.index, Number.MAX_SAFE_INTEGER, `${path}.index`) };
    }
    default:
      throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
  }
}

function parseTransformationOperation(value: unknown, path: string): ObservedTransformationOperationV4 {
  const candidate = strictRecord(value, path);
  switch (candidate.kind) {
    case "recode": {
      const operation = exactRecord(candidate, ["kind", "mappings", "unmapped"], [], path);
      return {
        kind: "recode",
        mappings: textMapValue(operation.mappings, `${path}.mappings`),
        unmapped: enumValue(operation.unmapped, ["keep", "set_missing", "reject"] as const, `${path}.unmapped`),
      };
    }
    case "mean_center":
      exactRecord(candidate, ["kind"], [], path);
      return { kind: "mean_center" };
    case "standardize": {
      const operation = exactRecord(candidate, ["kind", "denominator"], [], path);
      return { kind: "standardize", denominator: enumValue(operation.denominator, ["sample", "population"] as const, `${path}.denominator`) };
    }
    case "log": {
      const operation = exactRecord(candidate, ["kind", "base", "offset"], [], path);
      return { kind: "log", base: finiteValue(operation.base, `${path}.base`), offset: finiteValue(operation.offset, `${path}.offset`) };
    }
    case "formula": {
      const operation = exactRecord(candidate, ["kind", "expression"], [], path);
      return { kind: "formula", expression: textValue(operation.expression, `${path}.expression`) };
    }
    default:
      throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
  }
}

function parseTransformationStep(value: unknown, path: string): ObservedTransformationStepV4 {
  const step = exactRecord(value, ["id", "input_columns", "output_column", "operation"], [], path);
  return {
    id: textValue(step.id, `${path}.id`),
    input_columns: textArrayValue(step.input_columns, `${path}.input_columns`),
    output_column: textValue(step.output_column, `${path}.output_column`),
    operation: parseTransformationOperation(step.operation, `${path}.operation`),
  };
}

function parseIdentification(value: unknown, path: string): FactorIdentificationV4 {
  const candidate = strictRecord(value, path);
  if (candidate.kind === "marker_loading") {
    const identification = exactRecord(candidate, ["kind", "indicator"], [], path);
    return { kind: "marker_loading", indicator: textValue(identification.indicator, `${path}.indicator`) };
  }
  if (candidate.kind === "fixed_variance" || candidate.kind === "effects_coding") {
    exactRecord(candidate, ["kind"], [], path);
    return { kind: candidate.kind };
  }
  throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
}

function parseMeanPolicy(value: unknown, path: string): FactorMeanPolicyV4 {
  const candidate = strictRecord(value, path);
  if (candidate.kind === "fixed_zero") {
    exactRecord(candidate, ["kind"], [], path);
    return { kind: "fixed_zero" };
  }
  if (candidate.kind === "estimated") {
    const policy = exactRecord(candidate, ["kind", "parameter"], [], path);
    return { kind: "estimated", parameter: textValue(policy.parameter, `${path}.parameter`) };
  }
  if (candidate.kind === "reference_group") {
    const policy = exactRecord(candidate, ["kind", "reference_group", "parameter"], [], path);
    return {
      kind: "reference_group",
      reference_group: textValue(policy.reference_group, `${path}.reference_group`),
      parameter: textValue(policy.parameter, `${path}.parameter`),
    };
  }
  throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
}

function parseDisturbancePolicy(value: unknown, path: string): FactorDisturbancePolicyV4 {
  const policy = exactRecord(value, ["kind", "parameter"], [], path);
  return {
    kind: enumValue(policy.kind, ["exogenous_variance", "endogenous_disturbance", "fixed_zero"] as const, `${path}.kind`),
    parameter: textValue(policy.parameter, `${path}.parameter`),
  };
}

function parseWeighting(value: unknown, path: string): CompositeWeightingV4 {
  const candidate = strictRecord(value, path);
  if (candidate.kind === "mode_a" || candidate.kind === "mode_b") {
    exactRecord(candidate, ["kind"], [], path);
    return { kind: candidate.kind };
  }
  if (candidate.kind === "unit") {
    const weighting = exactRecord(candidate, ["kind", "normalization"], [], path);
    return { kind: "unit", normalization: enumValue(weighting.normalization, ["none", "sum_to_one", "unit_variance"] as const, `${path}.normalization`) };
  }
  if (candidate.kind === "custom") {
    const weighting = exactRecord(candidate, ["kind", "weights", "normalization"], [], path);
    return {
      kind: "custom",
      weights: numberMapValue(weighting.weights, `${path}.weights`),
      normalization: enumValue(weighting.normalization, ["none", "sum_to_one", "unit_variance"] as const, `${path}.normalization`),
    };
  }
  throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
}

function parseVariable(value: unknown, path: string): SemVariableV4 {
  const candidate = strictRecord(value, path);
  if (candidate.kind === "observed") {
    const variable = exactRecord(candidate, ["kind", "id", "label", "source_column", "scale", "role"], ["categories", "value_labels", "missing_markers", "transformation_lineage"], path);
    return {
      kind: "observed",
      id: textValue(variable.id, `${path}.id`),
      label: textValue(variable.label, `${path}.label`),
      source_column: textValue(variable.source_column, `${path}.source_column`),
      scale: enumValue(variable.scale, ["continuous", "binary", "ordinal", "nominal", "identifier"] as const, `${path}.scale`),
      role: enumValue(variable.role, ["indicator", "structural", "both", "control"] as const, `${path}.role`),
      categories: hasOwn(variable, "categories") ? textArrayValue(variable.categories, `${path}.categories`) : [],
      value_labels: defaultTextMapValue(variable, "value_labels", path),
      missing_markers: hasOwn(variable, "missing_markers") ? textArrayValue(variable.missing_markers, `${path}.missing_markers`) : [],
      transformation_lineage: defaultArrayValue(variable, "transformation_lineage", path, parseTransformationStep),
    };
  }
  if (candidate.kind === "common_factor") {
    const variable = exactRecord(candidate, ["kind", "id", "label", "identification", "mean_policy", "disturbance_policy"], [], path);
    return {
      kind: "common_factor",
      id: textValue(variable.id, `${path}.id`),
      label: textValue(variable.label, `${path}.label`),
      identification: parseIdentification(variable.identification, `${path}.identification`),
      mean_policy: parseMeanPolicy(variable.mean_policy, `${path}.mean_policy`),
      disturbance_policy: parseDisturbancePolicy(variable.disturbance_policy, `${path}.disturbance_policy`),
    };
  }
  if (candidate.kind === "composite") {
    const variable = exactRecord(candidate, ["kind", "id", "label", "weighting"], [], path);
    return { kind: "composite", id: textValue(variable.id, `${path}.id`), label: textValue(variable.label, `${path}.label`), weighting: parseWeighting(variable.weighting, `${path}.weighting`) };
  }
  if (candidate.kind === "derived") {
    const variable = exactRecord(candidate, ["kind", "id", "label"], [], path);
    return { kind: "derived", id: textValue(variable.id, `${path}.id`), label: textValue(variable.label, `${path}.label`) };
  }
  throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
}

function parseRelation(value: unknown, path: string): SemRelationV4 {
  const candidate = strictRecord(value, path);
  if (candidate.kind === "measurement_effect") {
    const relation = exactRecord(candidate, ["kind", "id", "construct", "indicator", "parameter"], [], path);
    return { kind: "measurement_effect", id: textValue(relation.id, `${path}.id`), construct: textValue(relation.construct, `${path}.construct`), indicator: textValue(relation.indicator, `${path}.indicator`), parameter: textValue(relation.parameter, `${path}.parameter`) };
  }
  if (candidate.kind === "measurement_causal") {
    const relation = exactRecord(candidate, ["kind", "id", "indicator", "composite", "parameter"], [], path);
    return { kind: "measurement_causal", id: textValue(relation.id, `${path}.id`), indicator: textValue(relation.indicator, `${path}.indicator`), composite: textValue(relation.composite, `${path}.composite`), parameter: textValue(relation.parameter, `${path}.parameter`) };
  }
  if (candidate.kind === "structural") {
    const relation = exactRecord(candidate, ["kind", "id", "source", "target", "parameter"], ["role", "intercept_parameter"], path);
    const role = optionalValue(relation, "role", path, (value, rolePath) => enumValue(value, ["structural", "control"] as const, rolePath));
    return {
      kind: "structural",
      id: textValue(relation.id, `${path}.id`),
      source: textValue(relation.source, `${path}.source`),
      target: textValue(relation.target, `${path}.target`),
      parameter: textValue(relation.parameter, `${path}.parameter`),
      ...(role === "control" ? { role } : {}),
      intercept_parameter: optionalValue(relation, "intercept_parameter", path, textValue),
    };
  }
  if (candidate.kind === "covariance") {
    const relation = exactRecord(candidate, ["kind", "id", "left", "right", "parameter"], [], path);
    return { kind: "covariance", id: textValue(relation.id, `${path}.id`), left: parseEndpoint(relation.left, `${path}.left`), right: parseEndpoint(relation.right, `${path}.right`), parameter: textValue(relation.parameter, `${path}.parameter`) };
  }
  throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
}

function parseGroupOverride(value: unknown, path: string): SemParameterGroupOverrideV4 {
  const override = exactRecord(value, ["group", "specification"], [], path);
  const candidate = strictRecord(override.specification, `${path}.specification`);
  let specification: SemParameterGroupOverrideSpecV4;
  if (candidate.kind === "free") {
    const spec = exactRecord(candidate, ["kind"], ["start", "lower", "upper"], `${path}.specification`);
    specification = {
      kind: "free",
      start: optionalValue(spec, "start", `${path}.specification`, finiteValue),
      lower: optionalValue(spec, "lower", `${path}.specification`, finiteValue),
      upper: optionalValue(spec, "upper", `${path}.specification`, finiteValue),
    };
  } else if (candidate.kind === "fixed") {
    const spec = exactRecord(candidate, ["kind", "value"], [], `${path}.specification`);
    specification = { kind: "fixed", value: finiteValue(spec.value, `${path}.specification.value`) };
  } else {
    throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.specification.kind`);
  }
  return { group: textValue(override.group, `${path}.group`), specification };
}

function parseParameter(value: unknown, path: string): SemParameterV4 {
  const candidate = strictRecord(value, path);
  const common = (parameter: UnknownRecord) => ({
    id: textValue(parameter.id, `${path}.id`),
    label: textValue(parameter.label, `${path}.label`),
    target: parseTarget(parameter.target, `${path}.target`),
    group_overrides: defaultArrayValue(parameter, "group_overrides", path, parseGroupOverride),
  });
  if (candidate.kind === "free") {
    const parameter = exactRecord(candidate, ["kind", "id", "label", "target"], ["start", "lower", "upper", "equality_label", "group_overrides"], path);
    return {
      kind: "free",
      ...common(parameter),
      start: optionalValue(parameter, "start", path, finiteValue),
      lower: optionalValue(parameter, "lower", path, finiteValue),
      upper: optionalValue(parameter, "upper", path, finiteValue),
      equality_label: optionalValue(parameter, "equality_label", path, textValue),
    };
  }
  if (candidate.kind === "fixed") {
    const parameter = exactRecord(candidate, ["kind", "id", "label", "target", "value"], ["group_overrides"], path);
    return { kind: "fixed", ...common(parameter), value: finiteValue(parameter.value, `${path}.value`) };
  }
  if (candidate.kind === "derived") {
    const parameter = exactRecord(candidate, ["kind", "id", "label", "target", "expression"], ["group_overrides"], path);
    return { kind: "derived", ...common(parameter), expression: textValue(parameter.expression, `${path}.expression`) };
  }
  throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
}

function parseConstraint(value: unknown, path: string): SemConstraintV4 {
  const candidate = strictRecord(value, path);
  if (candidate.kind === "equality") {
    const constraint = exactRecord(candidate, ["kind", "id", "parameters"], [], path);
    return { kind: "equality", id: textValue(constraint.id, `${path}.id`), parameters: textArrayValue(constraint.parameters, `${path}.parameters`) };
  }
  if (candidate.kind === "bound") {
    const constraint = exactRecord(candidate, ["kind", "id", "parameter"], ["lower", "upper"], path);
    return { kind: "bound", id: textValue(constraint.id, `${path}.id`), parameter: textValue(constraint.parameter, `${path}.parameter`), lower: optionalValue(constraint, "lower", path, finiteValue), upper: optionalValue(constraint, "upper", path, finiteValue) };
  }
  if (candidate.kind === "linear") {
    const constraint = exactRecord(candidate, ["kind", "id", "terms", "value"], [], path);
    return {
      kind: "linear",
      id: textValue(constraint.id, `${path}.id`),
      terms: arrayValue(constraint.terms, `${path}.terms`, (item, itemPath) => {
        const term = exactRecord(item, ["parameter", "coefficient"], [], itemPath);
        return { parameter: textValue(term.parameter, `${itemPath}.parameter`), coefficient: finiteValue(term.coefficient, `${itemPath}.coefficient`) };
      }),
      value: finiteValue(constraint.value, `${path}.value`),
    };
  }
  throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
}

function parseProductIndicator(value: unknown, path: string): ProductIndicatorSpecificationV4 {
  const specification = exactRecord(value, ["centering", "standardization", "pairing"], [], path);
  return {
    centering: enumValue(specification.centering, ["none", "mean_center", "double_mean_center"] as const, `${path}.centering`),
    standardization: enumValue(specification.standardization, ["none", "sample_standard_deviation"] as const, `${path}.standardization`),
    pairing: enumValue(specification.pairing, ["all_pairs"] as const, `${path}.pairing`),
  };
}

function parseDerivedTerm(value: unknown, path: string): SemDerivedTermV4 {
  const candidate = strictRecord(value, path);
  if (candidate.kind === "interaction") {
    const term = exactRecord(candidate, ["kind", "id", "output", "predictor", "moderator", "focal_relation", "method"], ["product_indicator"], path);
    const parsed: Extract<SemDerivedTermV4, { kind: "interaction" }> = {
      kind: "interaction",
      id: textValue(term.id, `${path}.id`),
      output: textValue(term.output, `${path}.output`),
      predictor: textValue(term.predictor, `${path}.predictor`),
      moderator: textValue(term.moderator, `${path}.moderator`),
      focal_relation: textValue(term.focal_relation, `${path}.focal_relation`),
      method: enumValue(term.method, ["product_indicator", "two_stage", "orthogonalizing"] as const, `${path}.method`),
    };
    // Rust skips a None option on serialization; missing and explicit null both
    // normalize to an omitted product_indicator property.
    if (hasOwn(term, "product_indicator") && term.product_indicator != null) {
      parsed.product_indicator = parseProductIndicator(term.product_indicator, `${path}.product_indicator`);
    }
    return parsed;
  }
  if (candidate.kind === "higher_order") {
    const term = exactRecord(candidate, ["kind", "id", "output", "components", "approach", "measurement_type"], [], path);
    return {
      kind: "higher_order",
      id: textValue(term.id, `${path}.id`),
      output: textValue(term.output, `${path}.output`),
      components: textArrayValue(term.components, `${path}.components`),
      approach: enumValue(term.approach, ["repeated_indicators", "extended_repeated_indicators", "embedded_two_stage", "disjoint_two_stage", "hybrid"] as const, `${path}.approach`),
      measurement_type: enumValue(term.measurement_type, ["reflective_reflective", "reflective_formative", "formative_reflective", "formative_formative"] as const, `${path}.measurement_type`),
    };
  }
  if (candidate.kind === "polynomial") {
    const term = exactRecord(candidate, ["kind", "id", "output", "source", "degree"], [], path);
    return { kind: "polynomial", id: textValue(term.id, `${path}.id`), output: textValue(term.output, `${path}.output`), source: textValue(term.source, `${path}.source`), degree: integerValue(term.degree, 0xff, `${path}.degree`) };
  }
  throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
}

function parseGroup(value: unknown, path: string): SemGroupV4 {
  const candidate = strictRecord(value, path);
  if (candidate.kind === "single_group") {
    exactRecord(candidate, ["kind"], [], path);
    return { kind: "single_group" };
  }
  if (candidate.kind === "observed_groups") {
    const group = exactRecord(candidate, ["kind", "grouping_variable", "levels"], [], path);
    return {
      kind: "observed_groups",
      grouping_variable: textValue(group.grouping_variable, `${path}.grouping_variable`),
      levels: arrayValue(group.levels, `${path}.levels`, (item, itemPath) => {
        const level = exactRecord(item, ["id", "value", "label"], [], itemPath);
        return { id: textValue(level.id, `${itemPath}.id`), value: textValue(level.value, `${itemPath}.value`), label: textValue(level.label, `${itemPath}.label`) };
      }),
    };
  }
  throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
}

function parseMissingData(value: unknown, path: string): MissingDataPolicyV4 {
  if (typeof value === "string") {
    return enumValue(value, ["listwise_deletion", "pairwise_deletion", "mean_replacement", "full_information_maximum_likelihood"] as const, path);
  }
  const outer = exactRecord(value, ["multiple_imputation"], [], path);
  const inner = exactRecord(outer.multiple_imputation, ["imputations"], [], `${path}.multiple_imputation`);
  return { multiple_imputation: { imputations: integerValue(inner.imputations, 0xffff, `${path}.multiple_imputation.imputations`) } };
}

function parseWeightBinding(value: unknown, path: string): SemWeightBindingV4 {
  const candidate = strictRecord(value, path);
  if (candidate.kind === "case" || candidate.kind === "frequency") {
    const binding = exactRecord(candidate, ["kind", "variable"], [], path);
    return { kind: candidate.kind, variable: textValue(binding.variable, `${path}.variable`) };
  }
  if (candidate.kind === "sampling") {
    const binding = exactRecord(candidate, ["kind", "variable", "normalization"], [], path);
    return { kind: "sampling", variable: textValue(binding.variable, `${path}.variable`), normalization: enumValue(binding.normalization, ["none", "mean_one", "sum_to_sample_size"] as const, `${path}.normalization`) };
  }
  throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
}

function parseSample(value: unknown, path: string): SemMatrixSampleMetadataV4 {
  const sample = exactRecord(value, ["sample_size", "covariance_denominator"], ["effective_sample_size", "degrees_of_freedom", "group_sample_sizes"], path);
  return {
    sample_size: integerValue(sample.sample_size, Number.MAX_SAFE_INTEGER, `${path}.sample_size`),
    covariance_denominator: enumValue(sample.covariance_denominator, ["sample_n_minus_one", "maximum_likelihood_n"] as const, `${path}.covariance_denominator`),
    effective_sample_size: optionalValue(sample, "effective_sample_size", path, finiteValue),
    degrees_of_freedom: optionalValue(sample, "degrees_of_freedom", path, (item, itemPath) => integerValue(item, Number.MAX_SAFE_INTEGER, itemPath)),
    group_sample_sizes: hasOwn(sample, "group_sample_sizes") ? countMapValue(sample.group_sample_sizes, `${path}.group_sample_sizes`) : {},
  };
}

function parseDataBinding(value: unknown, path: string): SemDataBindingV4 {
  const candidate = strictRecord(value, path);
  if (candidate.kind === "raw") {
    const binding = exactRecord(candidate, ["kind", "dataset_id", "missing_data"], ["weight", "cluster_variable", "strata_variable"], path);
    return {
      kind: "raw",
      dataset_id: textValue(binding.dataset_id, `${path}.dataset_id`),
      missing_data: parseMissingData(binding.missing_data, `${path}.missing_data`),
      weight: optionalValue(binding, "weight", path, parseWeightBinding),
      cluster_variable: optionalValue(binding, "cluster_variable", path, textValue),
      strata_variable: optionalValue(binding, "strata_variable", path, textValue),
    };
  }
  if (candidate.kind === "covariance" || candidate.kind === "correlation") {
    const binding = exactRecord(candidate, ["kind", "dataset_id", "variables", "sample"], ["means", "standard_deviations"], path);
    return {
      kind: candidate.kind,
      dataset_id: textValue(binding.dataset_id, `${path}.dataset_id`),
      variables: textArrayValue(binding.variables, `${path}.variables`),
      means: optionalValue(binding, "means", path, numberMapValue),
      standard_deviations: optionalValue(binding, "standard_deviations", path, numberMapValue),
      sample: parseSample(binding.sample, `${path}.sample`),
    };
  }
  throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
}

function parseAnnotation(value: unknown, path: string): SemAnnotationV4 {
  const candidate = strictRecord(value, path);
  if (candidate.kind === "display_only_covariance") {
    const annotation = exactRecord(candidate, ["kind", "id", "left", "right"], ["label"], path);
    return { kind: "display_only_covariance", id: textValue(annotation.id, `${path}.id`), left: textValue(annotation.left, `${path}.left`), right: textValue(annotation.right, `${path}.right`), label: optionalValue(annotation, "label", path, textValue) };
  }
  if (candidate.kind === "caption") {
    const annotation = exactRecord(candidate, ["kind", "id", "text"], [], path);
    return { kind: "caption", id: textValue(annotation.id, `${path}.id`), text: textValue(annotation.text, `${path}.text`) };
  }
  if (candidate.kind === "note") {
    const annotation = exactRecord(candidate, ["kind", "id", "subject", "text"], [], path);
    return { kind: "note", id: textValue(annotation.id, `${path}.id`), subject: textValue(annotation.subject, `${path}.subject`), text: textValue(annotation.text, `${path}.text`) };
  }
  throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
}

function parsePresentation(value: unknown, path: string): SemPresentationV4 {
  const candidate = strictRecord(value, path);
  if (candidate.kind === "none") {
    exactRecord(candidate, ["kind"], [], path);
    return { kind: "none" };
  }
  if (candidate.kind !== "canvas") throw new SemModelV4OperationError("schema.invalid_discriminator", `${path}.kind`);
  const presentation = exactRecord(candidate, ["kind", "nodes", "edges"], ["shapes", "images", "lines", "zoom", "pan_x", "pan_y"], path);
  return {
    kind: "canvas",
    nodes: arrayValue(presentation.nodes, `${path}.nodes`, (item, itemPath) => {
      const node = exactRecord(item, ["variable", "x", "y"], ["style"], itemPath);
      return { variable: textValue(node.variable, `${itemPath}.variable`), x: finiteValue(node.x, `${itemPath}.x`), y: finiteValue(node.y, `${itemPath}.y`), style: defaultTextMapValue(node, "style", itemPath) };
    }),
    edges: arrayValue(presentation.edges, `${path}.edges`, (item, itemPath) => {
      const edge = exactRecord(item, ["relation"], ["routing"], itemPath);
      return { relation: textValue(edge.relation, `${itemPath}.relation`), routing: optionalValue(edge, "routing", itemPath, textValue) };
    }),
    shapes: defaultArrayValue(presentation, "shapes", path, (item, itemPath) => {
      const shape = exactRecord(item, ["id", "shape", "x", "y", "width", "height"], ["label", "style"], itemPath);
      return { id: textValue(shape.id, `${itemPath}.id`), shape: enumValue(shape.shape, ["rectangle", "rounded_rectangle", "ellipse", "diamond"] as const, `${itemPath}.shape`), x: finiteValue(shape.x, `${itemPath}.x`), y: finiteValue(shape.y, `${itemPath}.y`), width: finiteValue(shape.width, `${itemPath}.width`), height: finiteValue(shape.height, `${itemPath}.height`), label: optionalValue(shape, "label", itemPath, textValue), style: defaultTextMapValue(shape, "style", itemPath) };
    }),
    images: defaultArrayValue(presentation, "images", path, (item, itemPath) => {
      const image = exactRecord(item, ["id", "asset_ref", "alt_text", "x", "y", "width", "height"], ["style"], itemPath);
      return { id: textValue(image.id, `${itemPath}.id`), asset_ref: textValue(image.asset_ref, `${itemPath}.asset_ref`), alt_text: textValue(image.alt_text, `${itemPath}.alt_text`), x: finiteValue(image.x, `${itemPath}.x`), y: finiteValue(image.y, `${itemPath}.y`), width: finiteValue(image.width, `${itemPath}.width`), height: finiteValue(image.height, `${itemPath}.height`), style: defaultTextMapValue(image, "style", itemPath) };
    }),
    lines: defaultArrayValue(presentation, "lines", path, (item, itemPath) => {
      const line = exactRecord(item, ["id", "x1", "y1", "x2", "y2"], ["label", "start_marker", "end_marker", "style"], itemPath);
      return { id: textValue(line.id, `${itemPath}.id`), x1: finiteValue(line.x1, `${itemPath}.x1`), y1: finiteValue(line.y1, `${itemPath}.y1`), x2: finiteValue(line.x2, `${itemPath}.x2`), y2: finiteValue(line.y2, `${itemPath}.y2`), label: optionalValue(line, "label", itemPath, textValue), start_marker: optionalValue(line, "start_marker", itemPath, textValue), end_marker: optionalValue(line, "end_marker", itemPath, textValue), style: defaultTextMapValue(line, "style", itemPath) };
    }),
    zoom: optionalValue(presentation, "zoom", path, finiteValue),
    pan_x: optionalValue(presentation, "pan_x", path, finiteValue),
    pan_y: optionalValue(presentation, "pan_y", path, finiteValue),
  };
}

function parseSemModelV4WireShape(value: unknown): SemModelV4 {
  const model = exactRecord(value, ["schema_version", "id", "name", "variables", "relations", "parameters", "constraints", "derived_terms", "group", "data_binding"], ["annotations", "presentation"], "model");
  const typed: SemModelV4 = {
    schema_version: integerValue(model.schema_version, 0xffff_ffff, "model.schema_version") as typeof SEM_MODEL_V4_SCHEMA_VERSION,
    id: textValue(model.id, "model.id"),
    name: textValue(model.name, "model.name"),
    variables: arrayValue(model.variables, "model.variables", parseVariable),
    relations: arrayValue(model.relations, "model.relations", parseRelation),
    parameters: arrayValue(model.parameters, "model.parameters", parseParameter),
    constraints: arrayValue(model.constraints, "model.constraints", parseConstraint),
    derived_terms: arrayValue(model.derived_terms, "model.derived_terms", parseDerivedTerm),
    group: parseGroup(model.group, "model.group"),
    data_binding: parseDataBinding(model.data_binding, "model.data_binding"),
    annotations: defaultArrayValue(model, "annotations", "model", parseAnnotation),
    presentation: hasOwn(model, "presentation") ? parsePresentation(model.presentation, "model.presentation") : { kind: "none" },
  };
  try {
    // Exercise the complete validator once so malformed primitive wire values
    // fail as schema errors even when a draft intentionally has unresolved
    // scientific references.
    validateSemModelV4(typed);
  } catch {
    throw new SemModelV4OperationError("schema.invalid_shape", "model");
  }
  return typed;
}

function validateSemModelV4AuthoringIdentity(model: SemModelV4): SemModelV4Issue[] {
  const issues: SemModelV4Issue[] = [];
  if (model.schema_version !== SEM_MODEL_V4_SCHEMA_VERSION) {
    issues.push(issue("schema.version", String(model.schema_version), "SEM model schema_version must be 4"));
  }
  if (typeof model.id !== "string" || !model.id.trim()) {
    issues.push(issue("model.id.empty", null, "Model id cannot be empty"));
  }
  if (typeof model.name !== "string" || !model.name.trim()) {
    issues.push(issue("model.name.empty", null, "Model name cannot be empty"));
  }

  const allObjects: Array<[string, unknown]> = [
    ...model.variables.map((value) => ["variable", value.id] as [string, unknown]),
    ...model.relations.map((value) => ["relation", value.id] as [string, unknown]),
    ...model.parameters.map((value) => ["parameter", value.id] as [string, unknown]),
    ...model.constraints.map((value) => ["constraint", value.id] as [string, unknown]),
    ...model.derived_terms.map((value) => ["derived_term", value.id] as [string, unknown]),
    ...model.annotations.map((value) => ["annotation", value.id] as [string, unknown]),
  ];
  const objectIds = new Map<string, string>();
  for (const [kind, candidate] of allObjects) {
    if (typeof candidate !== "string" || !candidate.trim()) {
      issues.push(issue("object.id.empty", kind, "Object id cannot be empty"));
    } else if (objectIds.has(candidate)) {
      issues.push(issue("object.id.duplicate", candidate, `Object id is shared by ${objectIds.get(candidate)} and ${kind}`));
    } else {
      objectIds.set(candidate, kind);
    }
  }
  return issues;
}

const AUTHORING_INTEGRITY_POLICY_CODES = new Set([
  "identification.factor.causal_measurement",
  "factor.mean_policy.parameter_invalid",
  "factor.mean_policy.group_overrides_invalid",
  "factor.mean_policy.reference_group_invalid",
  "factor.disturbance_policy.invalid",
  "identification.composite.custom_weights_invalid",
]);

/**
 * Mirrors Rust `SemModelV4::validate_authoring_integrity`: drafts may omit
 * not-yet-authored science, but every typed object already present must have
 * coherent references, shapes, values, and non-readiness policy semantics.
 * Passing this boundary never makes a draft executable.
 */
export function validateSemModelV4AuthoringIntegrity(model: SemModelV4): SemModelV4Issue[] {
  const issues = validateSemModelV4AuthoringStructure(model);

  for (const variable of model.variables) {
    if (variable.kind !== "common_factor" || variable.identification.kind !== "marker_loading") continue;
    const markerIndicator = variable.identification.indicator;
    const hasMarkerEffect = model.relations.some((relation) => (
      relation.kind === "measurement_effect"
      && relation.construct === variable.id
      && relation.indicator === markerIndicator
    ));
    if (!hasMarkerEffect) {
      issues.push(issue(
        "authoring.marker.reference_invalid",
        variable.id,
        "Marker-loading identification must reference an effect indicator of the factor",
      ));
    }
  }

  const policyIssues: SemModelV4Issue[] = [];
  const parameters = new Map(model.parameters.map((parameter) => [parameter.id, parameter]));
  validateIdentification(model, parameters, policyIssues);
  issues.push(...policyIssues.filter((value) => AUTHORING_INTEGRITY_POLICY_CODES.has(value.code)));
  return issues;
}

/** Strictly parses a structurally typed but potentially incomplete authoring draft. */
export function parseSemModelV4AuthoringDraft(value: unknown): SemModelV4 {
  const typed = parseSemModelV4WireShape(value);
  const issues = validateSemModelV4AuthoringIntegrity(typed);
  if (issues.length) throw new SemModelV4OperationError("model.invalid", "model", issues);
  return typed;
}

export function parseSemModelV4(value: unknown): SemModelV4 {
  const typed = parseSemModelV4WireShape(value);
  let issues: SemModelV4Issue[];
  try {
    issues = validateSemModelV4(typed);
  } catch {
    throw new SemModelV4OperationError("schema.invalid_shape", "model");
  }
  if (issues.length) throw new SemModelV4OperationError("model.invalid", "model", issues);
  return typed;
}

function validateSemModelV4AuthoringStructure(model: SemModelV4): SemModelV4Issue[] {
  const issues = validateSemModelV4AuthoringIdentity(model);

  const variables = new Map(model.variables.map((variable) => [variable.id, variable]));
  const parameters = new Map(model.parameters.map((parameter) => [parameter.id, parameter]));
  for (const variable of model.variables) {
    if (!variable.label.trim()) issues.push(issue("variable.label.empty", variable.id, "Variable label cannot be empty"));
    if (variable.kind === "observed") {
      if (!variable.source_column.trim()) issues.push(issue("observed.source_column.empty", variable.id, "Source column cannot be empty"));
      if (variable.scale === "binary" && variable.categories.length !== 2) issues.push(issue("observed.categories.binary_invalid", variable.id, "Binary variables require exactly two categories"));
      if (["ordinal", "nominal"].includes(variable.scale) && variable.categories.length < 2) issues.push(issue("observed.categories.insufficient", variable.id, "Ordinal and nominal variables require at least two categories"));
      if (["continuous", "identifier"].includes(variable.scale) && variable.categories.length) issues.push(issue("observed.categories.scale_invalid", variable.id, "Continuous and identifier variables cannot declare categories"));
      if (new Set(variable.categories).size !== variable.categories.length || variable.categories.some((category) => !category.trim())) issues.push(issue("observed.categories.duplicate_or_empty", variable.id, "Categories must be non-empty and unique"));
      const categories = new Set(variable.categories);
      if (Object.entries(variable.value_labels).some(([value, label]) => !value.trim() || !label.trim() || !categories.has(value))) issues.push(issue("observed.value_labels.invalid", variable.id, "Value labels must reference declared categories"));
      if (new Set(variable.missing_markers).size !== variable.missing_markers.length || variable.missing_markers.some((marker) => !marker.trim() || categories.has(marker))) issues.push(issue("observed.missing_markers.invalid", variable.id, "Missing markers must be non-empty, unique, and distinct from categories"));
      const stepIds = new Set<string>();
      const outputs = new Set<string>();
      for (const step of variable.transformation_lineage) {
        if (!step.id.trim() || stepIds.has(step.id) || !step.input_columns.length || step.input_columns.some((column) => !column.trim()) || !step.output_column.trim() || outputs.has(step.output_column)) issues.push(issue("observed.transformation.step_invalid", variable.id, "Transformation steps require unique ids and outputs plus non-empty inputs"));
        stepIds.add(step.id);
        outputs.add(step.output_column);
        if (step.operation.kind === "recode" && (!Object.keys(step.operation.mappings).length || Object.entries(step.operation.mappings).some(([from, to]) => !from.trim() || !to.trim()))) issues.push(issue("observed.transformation.recode_invalid", step.id, "Recode mappings must be non-empty"));
        if (step.operation.kind === "log" && (!Number.isFinite(step.operation.base) || step.operation.base <= 0 || step.operation.base === 1 || !Number.isFinite(step.operation.offset))) issues.push(issue("observed.transformation.log_invalid", step.id, "Log configuration is invalid"));
        if (step.operation.kind === "formula" && !step.operation.expression.trim()) issues.push(issue("observed.transformation.formula_empty", step.id, "Formula expression cannot be empty"));
      }
      if (variable.transformation_lineage.at(-1)?.output_column !== undefined && variable.transformation_lineage.at(-1)?.output_column !== variable.source_column) issues.push(issue("observed.transformation.output_mismatch", variable.id, "Final transformation output must match source_column"));
    }
  }

  const structuralPairs = new Set<string>();
  const covariancePairs = new Set<string>();
  for (const relation of model.relations) {
    const parameter = parameters.get(relation.parameter);
    if (!parameter) issues.push(issue("relation.parameter.unknown", relation.id, `Unknown parameter ${relation.parameter}`));
    if (relation.kind === "measurement_effect") {
      const construct = variables.get(relation.construct);
      if (construct?.kind !== "common_factor" && construct?.kind !== "composite") {
        issues.push(issue("measurement.effect.construct.invalid", relation.id, "Effect measurement source must be a common factor or composite"));
      }
      if (variables.get(relation.indicator)?.kind !== "observed") {
        issues.push(issue("measurement.indicator.invalid", relation.id, "Measurement indicator must be observed"));
      }
      const expected: SemParameterTargetV4 = { kind: "loading", construct: relation.construct, indicator: relation.indicator };
      if (parameter && !targetEquals(parameter.target, expected)) {
        issues.push(issue("relation.parameter.target_mismatch", relation.id, "Loading target does not match relation"));
      }
    } else if (relation.kind === "measurement_causal") {
      if (variables.get(relation.indicator)?.kind !== "observed") {
        issues.push(issue("measurement.indicator.invalid", relation.id, "Measurement indicator must be observed"));
      }
      if (variables.get(relation.composite)?.kind !== "composite") {
        issues.push(issue("measurement.causal.composite.invalid", relation.id, "Causal measurement target must be a composite"));
      }
      const expected: SemParameterTargetV4 = { kind: "weight", indicator: relation.indicator, composite: relation.composite };
      if (parameter && !targetEquals(parameter.target, expected)) {
        issues.push(issue("relation.parameter.target_mismatch", relation.id, "Weight target does not match relation"));
      }
    } else if (relation.kind === "structural") {
      if (!variables.has(relation.source) || !variables.has(relation.target)) {
        issues.push(issue("structural.variable.unknown", relation.id, "Structural relation references an unknown variable"));
      }
      if (relation.source === relation.target) issues.push(issue("structural.self", relation.id, "Structural self-loop is invalid"));
      const key = `${relation.source}\0${relation.target}`;
      if (structuralPairs.has(key)) issues.push(issue("structural.duplicate", relation.id, "Structural relation is duplicated"));
      structuralPairs.add(key);
      const expected: SemParameterTargetV4 = { kind: "regression", source: relation.source, target: relation.target };
      if (parameter && !targetEquals(parameter.target, expected)) {
        issues.push(issue("relation.parameter.target_mismatch", relation.id, "Regression target does not match relation"));
      }
      if (relation.intercept_parameter) {
        const intercept = parameters.get(relation.intercept_parameter);
        if (!intercept) issues.push(issue("structural.intercept.unknown", relation.id, "Unknown intercept parameter"));
        else if (!targetEquals(intercept.target, { kind: "intercept", variable: relation.target })) {
          issues.push(issue("structural.intercept.target_mismatch", relation.id, "Intercept target does not match outcome"));
        }
      }
    } else {
      validateEndpoint(relation.left, relation.id, variables, issues);
      validateEndpoint(relation.right, relation.id, variables, issues);
      const [left, right] = canonicalEndpointPair(relation.left, relation.right);
      const key = `${endpointKey(left)}\u0001${endpointKey(right)}`;
      if (endpointKey(left) === endpointKey(right)) issues.push(issue("covariance.self", relation.id, "Use a variance rather than covariance self-loop"));
      if (covariancePairs.has(key)) issues.push(issue("covariance.duplicate", relation.id, "Scientific covariance is duplicated"));
      covariancePairs.add(key);
      const expected: SemParameterTargetV4 = { kind: "covariance", left, right };
      if (parameter && !targetEquals(parameter.target, expected)) {
        issues.push(issue("relation.parameter.target_mismatch", relation.id, "Covariance target does not match relation"));
      }
    }
  }

  for (const variable of model.variables) if (variable.kind === "observed") {
    const asIndicator = model.relations.some((relation) => (relation.kind === "measurement_effect" || relation.kind === "measurement_causal") && relation.indicator === variable.id);
    const asSource = model.relations.some((relation) => relation.kind === "structural" && relation.source === variable.id);
    const asTarget = model.relations.some((relation) => relation.kind === "structural" && relation.target === variable.id);
    const invalidRole = variable.role === "indicator" ? asSource || asTarget
      : variable.role === "structural" ? asIndicator
        : variable.role === "control" ? asIndicator || asTarget
          : false;
    if (invalidRole) issues.push(issue("observed.role.usage_invalid", variable.id, "Observed-variable role is inconsistent with its model use"));
    if (variable.scale === "identifier" && (asIndicator || asSource || asTarget)) issues.push(issue("observed.identifier.model_use_invalid", variable.id, "Identifier variables cannot participate in measurement or structural relations"));
  }

  for (const parameter of model.parameters) validateParameter(parameter, variables, model.group, issues);
  validateConstraints(model.constraints, parameters, issues);
  validateDerivedTerms(model, variables, issues);
  validateGroup(model.group, variables, issues);
  validateDataBinding(model.data_binding, variables, model.group, issues);
  validateAnnotations(model.annotations, variables, issues);
  validatePresentation(model.presentation, variables, new Set(model.relations.map((relation) => relation.id)), issues);
  return issues;
}

export function validateSemModelV4(model: SemModelV4): SemModelV4Issue[] {
  const issues = validateSemModelV4AuthoringStructure(model);
  const parameters = new Map(model.parameters.map((parameter) => [parameter.id, parameter]));
  validateIdentification(model, parameters, issues);
  return issues;
}

function validateEndpoint(endpoint: SemEndpointV4, subject: string, variables: Map<string, SemVariableV4>, issues: SemModelV4Issue[]) {
  const variable = variables.get(endpoint.id);
  if (!variable) {
    issues.push(issue("endpoint.variable.unknown", subject, `Unknown endpoint variable ${endpoint.id}`));
  } else if (endpoint.kind === "residual_of" && variable.kind !== "observed") {
    issues.push(issue("endpoint.residual.invalid", subject, "Residual endpoint must reference an observed variable"));
  } else if (endpoint.kind === "disturbance_of" && variable.kind === "observed") {
    issues.push(issue("endpoint.disturbance.invalid", subject, "Disturbance endpoint must reference a latent, composite, or derived variable"));
  }
}

function validateParameter(parameter: SemParameterV4, variables: Map<string, SemVariableV4>, group: SemGroupV4, issues: SemModelV4Issue[]) {
  const finite = (value: number | null | undefined) => value == null || Number.isFinite(value);
  if (parameter.kind === "free") {
    if (![parameter.start, parameter.lower, parameter.upper].every(finite)) {
      issues.push(issue("parameter.value.non_finite", parameter.id, "Parameter start and bounds must be finite"));
    }
    if (parameter.lower != null && parameter.upper != null && parameter.lower > parameter.upper) {
      issues.push(issue("parameter.bounds.invalid", parameter.id, "Lower bound cannot exceed upper bound"));
    }
  } else if (parameter.kind === "fixed" && !Number.isFinite(parameter.value)) {
    issues.push(issue("parameter.value.non_finite", parameter.id, "Fixed value must be finite"));
  }
  const validGroups = group.kind === "observed_groups" ? new Set(group.levels.map((level) => level.id)) : null;
  const overriddenGroups = new Set<string>();
  for (const override of parameter.group_overrides ?? []) {
    if (!override.group.trim() || overriddenGroups.has(override.group) || !validGroups?.has(override.group)) issues.push(issue("parameter.group_override.group_invalid", parameter.id, "Group override must uniquely reference a declared group"));
    overriddenGroups.add(override.group);
    if (override.specification.kind === "fixed") {
      if (!Number.isFinite(override.specification.value)) issues.push(issue("parameter.group_override.value_invalid", parameter.id, "Fixed group override must be finite"));
    } else {
      const { start, lower, upper } = override.specification;
      if (![start, lower, upper].every(finite) || (lower != null && upper != null && lower > upper)) issues.push(issue("parameter.group_override.value_invalid", parameter.id, "Free group override bounds are invalid"));
    }
  }
  const target = parameter.target;
  if (target.kind === "loading") {
    const construct = variables.get(target.construct);
    if ((construct?.kind !== "common_factor" && construct?.kind !== "composite") || variables.get(target.indicator)?.kind !== "observed") {
      issues.push(issue("parameter.loading.target_invalid", parameter.id, "Loading target is invalid"));
    }
  } else if (target.kind === "weight") {
    if (variables.get(target.indicator)?.kind !== "observed" || variables.get(target.composite)?.kind !== "composite") {
      issues.push(issue("parameter.weight.target_invalid", parameter.id, "Weight target is invalid"));
    }
  } else if (target.kind === "regression") {
    if (!variables.has(target.source) || !variables.has(target.target)) issues.push(issue("parameter.regression.target_invalid", parameter.id, "Regression target is invalid"));
  } else if (target.kind === "variance") {
    validateEndpoint(target.endpoint, parameter.id, variables, issues);
  } else if (target.kind === "covariance") {
    validateEndpoint(target.left, parameter.id, variables, issues);
    validateEndpoint(target.right, parameter.id, variables, issues);
  } else if (!variables.has(target.variable)) {
    issues.push(issue("parameter.location.target_invalid", parameter.id, "Location target is invalid"));
  }
}

function validateConstraints(constraints: SemConstraintV4[], parameters: Map<string, SemParameterV4>, issues: SemModelV4Issue[]) {
  for (const constraint of constraints) {
    if (constraint.kind === "equality") {
      const unique = new Set(constraint.parameters);
      if (unique.size < 2 || unique.size !== constraint.parameters.length) {
        issues.push(issue("constraint.equality.invalid", constraint.id, "Equality constraint requires at least two unique parameters"));
      }
      for (const parameter of constraint.parameters) if (!parameters.has(parameter)) issues.push(issue("constraint.parameter.unknown", constraint.id, `Unknown parameter ${parameter}`));
    } else if (constraint.kind === "bound") {
      if (!parameters.has(constraint.parameter)) issues.push(issue("constraint.parameter.unknown", constraint.id, `Unknown parameter ${constraint.parameter}`));
      if (constraint.lower == null && constraint.upper == null) issues.push(issue("constraint.bound.empty", constraint.id, "Bound requires a lower or upper value"));
      if (constraint.lower != null && constraint.upper != null && constraint.lower > constraint.upper) issues.push(issue("constraint.bound.invalid", constraint.id, "Lower bound cannot exceed upper bound"));
    } else {
      if (!constraint.terms.length || !Number.isFinite(constraint.value)) issues.push(issue("constraint.linear.invalid", constraint.id, "Linear constraint requires finite terms and value"));
      const seen = new Set<string>();
      for (const term of constraint.terms) {
        if (!parameters.has(term.parameter)) issues.push(issue("constraint.parameter.unknown", constraint.id, `Unknown parameter ${term.parameter}`));
        if (!Number.isFinite(term.coefficient) || seen.has(term.parameter)) issues.push(issue("constraint.linear.term_invalid", constraint.id, "Linear terms must be finite and unique"));
        seen.add(term.parameter);
      }
    }
  }
}

function validateDerivedTerms(model: SemModelV4, variables: Map<string, SemVariableV4>, issues: SemModelV4Issue[]) {
  const outputs = new Set<string>();
  for (const term of model.derived_terms) {
    if (outputs.has(term.output)) issues.push(issue("derived.output.duplicate", term.output, "Derived output has multiple definitions"));
    outputs.add(term.output);
    if (variables.get(term.output)?.kind !== "derived") issues.push(issue("derived.output.invalid", term.id, "Derived output must reference a derived variable"));
    if (term.kind === "interaction") {
      if (new Set([term.output, term.predictor, term.moderator]).size !== 3) issues.push(issue("derived.interaction.roles_invalid", term.id, "Interaction roles must be distinct"));
      for (const input of [term.predictor, term.moderator]) if (!variables.has(input)) issues.push(issue("derived.input.unknown", term.id, `Unknown input ${input}`));
      if (term.method === "product_indicator" && !term.product_indicator) {
        issues.push(issue("derived.interaction.product_indicator_spec_required", term.id, "Product-indicator interactions require explicit construction settings"));
      } else if (term.method !== "product_indicator" && term.product_indicator) {
        issues.push(issue("derived.interaction.product_indicator_spec_forbidden", term.id, "Product-indicator construction settings are valid only for the product-indicator method"));
      }
      const focal = model.relations.find((relation) => relation.id === term.focal_relation);
      if (!focal) issues.push(issue("derived.interaction.focal_relation_unknown", term.id, `Unknown focal relation ${term.focal_relation}`));
      else if (focal.kind !== "structural" || focal.source !== term.predictor) issues.push(issue("derived.interaction.focal_relation_invalid", term.id, "Focal relation must be a structural path sourced by the predictor"));
      else if (!model.relations.some((relation) => relation.kind === "structural" && relation.source === term.output && relation.target === focal.target)) issues.push(issue("derived.interaction.effect_path_missing", term.id, "Interaction output must target the focal outcome"));
    } else if (term.kind === "higher_order") {
      const unique = new Set(term.components);
      if (unique.size < 2 || unique.size !== term.components.length || unique.has(term.output)) issues.push(issue("derived.higher_order.components_invalid", term.id, "Higher-order components must be unique and non-self"));
      for (const input of term.components) if (!variables.has(input)) issues.push(issue("derived.input.unknown", term.id, `Unknown component ${input}`));
    } else if (term.degree < 2 || term.source === term.output || !variables.has(term.source)) {
      issues.push(issue("derived.polynomial.invalid", term.id, "Polynomial definition is invalid"));
    }
  }
  for (const variable of model.variables) if (variable.kind === "derived" && !outputs.has(variable.id)) issues.push(issue("derived.definition.missing", variable.id, "Derived variable requires one definition"));
}

function validateGroup(group: SemGroupV4, variables: Map<string, SemVariableV4>, issues: SemModelV4Issue[]) {
  if (group.kind === "single_group") return;
  if (variables.get(group.grouping_variable)?.kind !== "observed") issues.push(issue("group.variable.invalid", group.grouping_variable, "Grouping variable must be observed"));
  if (group.levels.length < 2) issues.push(issue("group.levels.insufficient", group.grouping_variable, "At least two group levels are required"));
  const ids = new Set<string>();
  const values = new Set<string>();
  for (const level of group.levels) {
    if (!level.id.trim() || !level.value.trim() || ids.has(level.id) || values.has(level.value)) issues.push(issue("group.level.duplicate_or_empty", level.id, "Group level ids and values must be non-empty and unique"));
    ids.add(level.id);
    values.add(level.value);
  }
}

function validateDataBinding(binding: SemDataBindingV4, variables: Map<string, SemVariableV4>, group: SemGroupV4, issues: SemModelV4Issue[]) {
  if (!binding.dataset_id.trim()) issues.push(issue("data.dataset.empty", null, "Dataset id cannot be empty"));
  if (binding.kind !== "raw") {
    if (!Number.isInteger(binding.sample.sample_size) || binding.sample.sample_size < 2) issues.push(issue("data.matrix.invalid", binding.dataset_id, "Matrix sample size must be at least two"));
    if (binding.sample.covariance_denominator !== "sample_n_minus_one" && binding.sample.covariance_denominator !== "maximum_likelihood_n") issues.push(issue("data.matrix.denominator_invalid", binding.dataset_id, "Matrix covariance denominator must be declared explicitly"));
    const matrixVariables = new Set(binding.variables);
    if (binding.variables.length < 2 || matrixVariables.size !== binding.variables.length || binding.variables.some((id) => variables.get(id)?.kind !== "observed")) issues.push(issue("data.matrix.variables_invalid", binding.dataset_id, "Matrix variables must be unique observed-variable ids"));
    for (const [name, values, positive] of [["means", binding.means, false], ["standard_deviations", binding.standard_deviations, true]] as const) if (values) {
      const keys = Object.keys(values);
      if (keys.length !== matrixVariables.size || keys.some((key) => !matrixVariables.has(key)) || Object.values(values).some((value) => !Number.isFinite(value) || (positive && value <= 0))) issues.push(issue("data.matrix.moments_invalid", binding.dataset_id, `Matrix ${name} are invalid`));
    }
    if (binding.sample.effective_sample_size != null && (!Number.isFinite(binding.sample.effective_sample_size) || binding.sample.effective_sample_size <= 0) || binding.sample.degrees_of_freedom != null && (!Number.isInteger(binding.sample.degrees_of_freedom) || binding.sample.degrees_of_freedom <= 0)) issues.push(issue("data.matrix.sample_metadata_invalid", binding.dataset_id, "Matrix sample metadata must be positive"));
    const groupSizes = binding.sample.group_sample_sizes ?? {};
    const validGroups = group.kind === "observed_groups" ? new Set(group.levels.map((level) => level.id)) : null;
    if (Object.keys(groupSizes).length && (!validGroups || Object.keys(groupSizes).some((id) => !validGroups.has(id)) || Object.values(groupSizes).some((size) => !Number.isInteger(size) || size <= 0) || Object.values(groupSizes).reduce((sum, size) => sum + size, 0) !== binding.sample.sample_size)) issues.push(issue("data.matrix.group_sample_sizes_invalid", binding.dataset_id, "Group sample sizes are invalid"));
    return;
  }
  if (typeof binding.missing_data === "object" && binding.missing_data.multiple_imputation.imputations < 2) issues.push(issue("data.imputation.count_invalid", null, "Multiple imputation requires at least two imputations"));
  if (binding.weight) {
    const weight = binding.weight as SemWeightBindingV4;
    if (weight.kind !== "case" && weight.kind !== "frequency" && weight.kind !== "sampling") {
      issues.push(issue("data.weight.kind_invalid", null, "Weight kind must be case, frequency, or sampling"));
    } else {
      if (weight.kind === "sampling"
        && weight.normalization !== "none"
        && weight.normalization !== "mean_one"
        && weight.normalization !== "sum_to_sample_size") {
        issues.push(issue("data.weight.normalization_invalid", weight.variable, "Sampling-weight normalization must be declared explicitly"));
      }
      const variable = variables.get(weight.variable);
      if (variable?.kind !== "observed" || variable.scale !== "continuous" || variable.role !== "control") issues.push(issue("data.weight.variable_invalid", weight.variable, "Weights require a continuous observed control variable"));
    }
  }
  for (const id of [binding.cluster_variable, binding.strata_variable]) if (id && variables.get(id)?.kind !== "observed") issues.push(issue("data.binding.variable_invalid", id, "Data role must reference an observed variable"));
}

function validateAnnotations(annotations: SemAnnotationV4[], variables: Map<string, SemVariableV4>, issues: SemModelV4Issue[]) {
  for (const annotation of annotations) if (annotation.kind === "display_only_covariance" && (annotation.left === annotation.right || !variables.has(annotation.left) || !variables.has(annotation.right))) issues.push(issue("annotation.covariance.invalid", annotation.id, "Display covariance must reference two distinct known variables"));
}

function validatePresentation(presentation: SemPresentationV4, variables: Map<string, SemVariableV4>, relations: Set<string>, issues: SemModelV4Issue[]) {
  if (presentation.kind === "none") return;
  const nodes = new Set<string>();
  for (const node of presentation.nodes) {
    if (!variables.has(node.variable) || nodes.has(node.variable)) issues.push(issue("presentation.node.invalid", node.variable, "Canvas node must reference a unique known variable"));
    if (!Number.isFinite(node.x) || !Number.isFinite(node.y)) issues.push(issue("presentation.coordinate.non_finite", node.variable, "Canvas coordinates must be finite"));
    nodes.add(node.variable);
  }
  const edges = new Set<string>();
  for (const edge of presentation.edges) {
    if (!relations.has(edge.relation) || edges.has(edge.relation)) issues.push(issue("presentation.edge.invalid", edge.relation, "Canvas edge must reference a unique known relation"));
    edges.add(edge.relation);
  }
  const decorations = new Set<string>();
  for (const shape of presentation.shapes) {
    if (!shape.id.trim() || decorations.has(shape.id) || ![shape.x, shape.y, shape.width, shape.height].every(Number.isFinite) || shape.width <= 0 || shape.height <= 0) issues.push(issue("presentation.shape.invalid", shape.id, "Canvas shape is invalid"));
    decorations.add(shape.id);
  }
  for (const image of presentation.images) {
    if (!image.id.trim() || decorations.has(image.id) || !image.asset_ref.trim() || !image.alt_text.trim() || ![image.x, image.y, image.width, image.height].every(Number.isFinite) || image.width <= 0 || image.height <= 0) issues.push(issue("presentation.image.invalid", image.id, "Canvas image is invalid"));
    decorations.add(image.id);
  }
  for (const line of presentation.lines) {
    if (!line.id.trim() || decorations.has(line.id) || ![line.x1, line.y1, line.x2, line.y2].every(Number.isFinite) || line.x1 === line.x2 && line.y1 === line.y2) issues.push(issue("presentation.line.invalid", line.id, "Canvas line is invalid"));
    decorations.add(line.id);
  }
  if ([presentation.zoom, presentation.pan_x, presentation.pan_y].some((value) => value != null && !Number.isFinite(value))) issues.push(issue("presentation.viewport.non_finite", null, "Viewport values must be finite"));
}

function validateIdentification(model: SemModelV4, parameters: Map<string, SemParameterV4>, issues: SemModelV4Issue[]) {
  for (const variable of model.variables) {
    if (variable.kind === "common_factor") {
      const effects = model.relations.filter((relation): relation is Extract<SemRelationV4, { kind: "measurement_effect" }> => relation.kind === "measurement_effect" && relation.construct === variable.id);
      if (effects.length < 2) issues.push(issue("identification.factor.indicators_insufficient", variable.id, "Common factor requires at least two effect indicators"));
      if (model.relations.some((relation) => relation.kind === "measurement_causal" && relation.composite === variable.id)) {
        issues.push(issue("identification.factor.causal_measurement", variable.id, "A common factor cannot use causal-indicator measurement relations"));
      }
      if (variable.identification.kind === "marker_loading") {
        const markerIndicator = variable.identification.indicator;
        const marker = effects.find((relation) => relation.indicator === markerIndicator);
        const parameter = marker ? parameters.get(marker.parameter) : null;
        if (parameter?.kind !== "fixed" || Math.abs(parameter.value - 1) > 1e-12) issues.push(issue("identification.marker.invalid", variable.id, "Marker loading must be fixed to one"));
      } else if (variable.identification.kind === "fixed_variance") {
        const fixed = model.parameters.some((parameter) => parameter.kind === "fixed" && parameter.value === 1 && targetEquals(parameter.target, { kind: "variance", endpoint: { kind: "variable", id: variable.id } }));
        if (!fixed) issues.push(issue("identification.fixed_variance.missing", variable.id, "Factor variance must be fixed to one"));
      } else {
        const ids = new Set(effects.map((relation) => relation.parameter));
        const found = effects.length >= 3 && model.constraints.some((constraint) => constraint.kind === "linear" && constraint.terms.length === ids.size && constraint.terms.every((term) => ids.has(term.parameter) && term.coefficient === 1) && constraint.value === ids.size);
        if (!found) issues.push(issue("identification.effects_coding.invalid", variable.id, "Effects coding requires at least three loadings and an explicit sum constraint"));
      }
      if (variable.mean_policy.kind !== "fixed_zero") {
        const parameter = parameters.get(variable.mean_policy.parameter);
        if (!parameter || !targetEquals(parameter.target, { kind: "mean", variable: variable.id }) || variable.mean_policy.kind === "estimated" && parameter.kind !== "free") issues.push(issue("factor.mean_policy.parameter_invalid", variable.id, "Factor mean policy requires a matching mean parameter"));
        if (variable.mean_policy.kind === "reference_group") {
          const referenceGroup = variable.mean_policy.reference_group;
          if (model.group.kind !== "observed_groups" || !model.group.levels.some((level) => level.id === referenceGroup)) issues.push(issue("factor.mean_policy.reference_group_invalid", variable.id, "Reference group is not declared"));
          if (model.group.kind === "observed_groups" && parameter) {
            const overrides = new Map((parameter.group_overrides ?? []).map((override) => [override.group, override.specification]));
            const reference = overrides.get(referenceGroup);
            if (overrides.size !== model.group.levels.length || model.group.levels.some((level) => !overrides.has(level.id)) || reference?.kind !== "fixed" || Math.abs(reference.value) > 1e-12) issues.push(issue("factor.mean_policy.group_overrides_invalid", variable.id, "Reference-group means require explicit overrides and zero fixed in the reference group"));
          }
        }
      }
      const endogenous = model.relations.some((relation) => relation.kind === "structural" && relation.target === variable.id);
      const disturbanceParameter = parameters.get(variable.disturbance_policy.parameter);
      const expectedEndpoint: SemEndpointV4 = variable.disturbance_policy.kind === "exogenous_variance"
        ? { kind: "variable", id: variable.id }
        : { kind: "disturbance_of", id: variable.id };
      const invalidDirection = variable.disturbance_policy.kind === "exogenous_variance" ? endogenous
        : variable.disturbance_policy.kind === "endogenous_disturbance" ? !endogenous
          : false;
      const invalidZero = variable.disturbance_policy.kind === "fixed_zero" && (disturbanceParameter?.kind !== "fixed" || Math.abs(disturbanceParameter.value) > 1e-12);
      if (!disturbanceParameter || !targetEquals(disturbanceParameter.target, { kind: "variance", endpoint: variable.disturbance_policy.kind === "fixed_zero" && !endogenous ? { kind: "variable", id: variable.id } : expectedEndpoint }) || invalidDirection || invalidZero) issues.push(issue("factor.disturbance_policy.invalid", variable.id, "Disturbance policy does not match endogeneity and variance parameter"));
    } else if (variable.kind === "composite") {
      const effects = model.relations.filter((relation) => relation.kind === "measurement_effect" && relation.construct === variable.id).length;
      const causal = model.relations.filter((relation) => relation.kind === "measurement_causal" && relation.composite === variable.id).length;
      const valid = variable.weighting.kind === "mode_a" ? effects > 0 && causal === 0 : causal > 0 && effects === 0;
      if (!valid) issues.push(issue("identification.composite.measurement_invalid", variable.id, "Composite measurement direction does not match weighting"));
      if (variable.weighting.kind === "custom") {
        const indicators = new Set(model.relations.flatMap((relation) => relation.kind === "measurement_causal" && relation.composite === variable.id ? [relation.indicator] : []));
        const keys = Object.keys(variable.weighting.weights);
        if (keys.length !== indicators.size || keys.some((key) => !indicators.has(key)) || Object.values(variable.weighting.weights).some((weight) => !Number.isFinite(weight)) || Object.values(variable.weighting.weights).every((weight) => Math.abs(weight) <= Number.EPSILON)) issues.push(issue("identification.composite.custom_weights_invalid", variable.id, "Custom weights must cover all causal indicators and cannot all be zero"));
      }
    }
  }
}

export function hasStructuralFeedbackV4(model: SemModelV4): boolean {
  const ids = new Set(model.variables.map((variable) => variable.id));
  const indegree = new Map([...ids].map((id) => [id, 0]));
  const outgoing = new Map<string, string[]>();
  for (const relation of model.relations) if (relation.kind === "structural" && ids.has(relation.source) && ids.has(relation.target) && relation.source !== relation.target) {
    indegree.set(relation.target, (indegree.get(relation.target) ?? 0) + 1);
    outgoing.set(relation.source, [...(outgoing.get(relation.source) ?? []), relation.target]);
  }
  const ready = [...indegree].filter(([, degree]) => degree === 0).map(([id]) => id);
  let visited = 0;
  while (ready.length) {
    const source = ready.pop()!;
    visited += 1;
    for (const target of outgoing.get(source) ?? []) {
      const degree = (indegree.get(target) ?? 0) - 1;
      indegree.set(target, degree);
      if (degree === 0) ready.push(target);
    }
  }
  return visited !== ids.size;
}

const UTF8_ENCODER_V1 = new TextEncoder();

/** Matches Rust String/str ordering by comparing the encoded UTF-8 bytes. */
export function compareUtf8StringsV1(left: string, right: string): number {
  if (left === right) return 0;
  const leftBytes = UTF8_ENCODER_V1.encode(left);
  const rightBytes = UTF8_ENCODER_V1.encode(right);
  const sharedLength = Math.min(leftBytes.length, rightBytes.length);
  for (let index = 0; index < sharedLength; index += 1) {
    const difference = leftBytes[index]! - rightBytes[index]!;
    if (difference !== 0) return difference;
  }
  return leftBytes.length - rightBytes.length;
}

export function canonicalizeSemModelV4(model: SemModelV4): SemModelV4 {
  const canonical = JSON.parse(JSON.stringify(model)) as SemModelV4;
  for (const variable of canonical.variables) if (variable.kind === "observed") variable.missing_markers.sort(compareUtf8StringsV1);
  canonical.variables.sort((left, right) => left.id.localeCompare(right.id));
  for (const relation of canonical.relations) {
    if (relation.kind === "covariance") [relation.left, relation.right] = canonicalEndpointPair(relation.left, relation.right);
    else if (relation.kind === "structural" && relation.role === "structural") delete relation.role;
  }
  canonical.relations.sort((left, right) => left.id.localeCompare(right.id));
  for (const parameter of canonical.parameters) {
    parameter.target = canonicalTarget(parameter.target);
    parameter.group_overrides?.sort((left, right) => left.group.localeCompare(right.group));
  }
  canonical.parameters.sort((left, right) => left.id.localeCompare(right.id));
  for (const constraint of canonical.constraints) {
    if (constraint.kind === "equality") constraint.parameters.sort();
    else if (constraint.kind === "linear") constraint.terms.sort((left, right) => left.parameter.localeCompare(right.parameter));
  }
  canonical.constraints.sort((left, right) => left.id.localeCompare(right.id));
  for (const term of canonical.derived_terms) if (term.kind === "higher_order") term.components.sort();
  canonical.derived_terms.sort((left, right) => left.id.localeCompare(right.id));
  if (canonical.group.kind === "observed_groups") canonical.group.levels.sort((left, right) => left.id.localeCompare(right.id));
  for (const annotation of canonical.annotations) if (annotation.kind === "display_only_covariance" && annotation.left > annotation.right) [annotation.left, annotation.right] = [annotation.right, annotation.left];
  canonical.annotations.sort((left, right) => left.id.localeCompare(right.id));
  if (canonical.presentation.kind === "canvas") {
    canonical.presentation.nodes.sort((left, right) => left.variable.localeCompare(right.variable));
    canonical.presentation.edges.sort((left, right) => left.relation.localeCompare(right.relation));
    canonical.presentation.shapes.sort((left, right) => left.id.localeCompare(right.id));
    canonical.presentation.images.sort((left, right) => left.id.localeCompare(right.id));
    canonical.presentation.lines.sort((left, right) => left.id.localeCompare(right.id));
  }
  return canonical;
}

export function scientificSemModelV4HashInput(model: SemModelV4): string {
  const issues = validateSemModelV4(model);
  if (issues.length) throw new SemModelV4OperationError("model.invalid", "model", issues);
  const canonical = canonicalizeSemModelV4(model);
  canonical.annotations = [];
  canonical.presentation = { kind: "none" };
  return stableStringify(canonical);
}

function stableStringify(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(stableStringify).join(",")}]`;
  if (value && typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>)
      .filter(([, child]) => child !== undefined)
      .sort(([left], [right]) => left.localeCompare(right));
    return `{${entries.map(([key, child]) => `${JSON.stringify(key)}:${stableStringify(child)}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

export class SemModelV4OperationError extends Error {
  constructor(
    public readonly code: string,
    public readonly subject: string,
    public readonly issues: readonly SemModelV4Issue[] = [],
  ) {
    super(`${code}: ${subject}`);
    this.name = "SemModelV4OperationError";
  }
}

export interface CompiledPlsPlanV2 {
  readonly model_id: string;
  readonly scientific_hash_input: string;
  readonly dataset_id: string;
  readonly blocks: readonly {
    readonly construct_id: string;
    readonly mode: CompositeModeV4;
    readonly fixed_scoring?:
      | { readonly kind: "unit"; readonly normalization: CompositeWeightNormalizationV4 }
      | { readonly kind: "custom"; readonly weights: Readonly<Record<string, number>>; readonly normalization: CompositeWeightNormalizationV4 };
    readonly indicators: readonly { readonly variable_id: string; readonly source_column: string; readonly parameter_id: string }[];
  }[];
  readonly paths: readonly { readonly relation_id: string; readonly source: string; readonly target: string; readonly parameter_id: string }[];
}

export interface CompiledCbsemPlanV2 {
  readonly model_id: string;
  readonly scientific_hash_input: string;
  readonly input: Readonly<SemDataBindingV4>;
  readonly observed_variables: readonly string[];
  readonly factors: readonly string[];
  readonly loadings: readonly { readonly relation_id: string; readonly factor: string; readonly indicator: string; readonly parameter_id: string }[];
  readonly regressions: readonly { readonly relation_id: string; readonly source: string; readonly target: string; readonly parameter_id: string }[];
  readonly covariances: readonly { readonly relation_id: string; readonly left: Readonly<SemEndpointV4>; readonly right: Readonly<SemEndpointV4>; readonly parameter_id: string }[];
  readonly parameters: readonly Readonly<SemParameterV4>[];
  readonly constraints: readonly Readonly<SemConstraintV4>[];
  readonly has_feedback: boolean;
}

export function compilePlsPlanV2(model: SemModelV4): CompiledPlsPlanV2 {
  assertValid(model);
  if (hasStructuralFeedbackV4(model)) throw new SemModelV4OperationError("pls.feedback_unsupported", "model");
  if (model.group.kind !== "single_group") throw new SemModelV4OperationError("pls.multigroup_unsupported", "group");
  if (model.constraints.length) throw new SemModelV4OperationError("pls.constraints_unsupported", model.constraints[0].id);
  if (model.derived_terms.length) throw new SemModelV4OperationError("pls.derived_terms_unsupported", model.derived_terms[0].id);
  const groupOverride = model.parameters.find((parameter) => parameter.group_overrides?.length);
  if (groupOverride) throw new SemModelV4OperationError("pls.parameter_group_overrides_unsupported", groupOverride.id);
  if (model.data_binding.kind !== "raw" || model.data_binding.missing_data !== "listwise_deletion" || model.data_binding.weight || model.data_binding.cluster_variable || model.data_binding.strata_variable) throw new SemModelV4OperationError("pls.data_binding_unsupported", "data_binding");
  for (const variable of model.variables) {
    if (variable.kind === "common_factor" || variable.kind === "derived") throw new SemModelV4OperationError("pls.variable_unsupported", variable.id);
    if (variable.kind === "observed" && variable.scale !== "continuous") throw new SemModelV4OperationError("pls.scale_unsupported", variable.id);
    if (variable.kind === "observed" && (variable.missing_markers.length || variable.transformation_lineage.length)) throw new SemModelV4OperationError("pls.observed_metadata_unsupported", variable.id);
  }
  const covariance = model.relations.find((relation) => relation.kind === "covariance");
  if (covariance) throw new SemModelV4OperationError("pls.covariance_unsupported", covariance.id);
  const variables = new Map(model.variables.map((variable) => [variable.id, variable]));
  const blocks = model.variables.filter((variable): variable is Extract<SemVariableV4, { kind: "composite" }> => variable.kind === "composite").map((variable) => {
    const mode: CompositeModeV4 = variable.weighting.kind === "mode_a" ? "mode_a"
      : "mode_b";
    const fixed_scoring = variable.weighting.kind === "unit"
      ? { kind: "unit" as const, normalization: variable.weighting.normalization }
      : variable.weighting.kind === "custom"
        ? {
            kind: "custom" as const,
            weights: Object.fromEntries(Object.entries(variable.weighting.weights).sort(([left], [right]) => left.localeCompare(right))),
            normalization: variable.weighting.normalization,
          }
        : undefined;
    return {
      construct_id: variable.id,
      mode,
      ...(fixed_scoring ? { fixed_scoring } : {}),
      indicators: model.relations.flatMap((relation) => {
      let indicatorId: string;
      if (mode === "mode_a" && relation.kind === "measurement_effect" && relation.construct === variable.id) {
        indicatorId = relation.indicator;
      } else if (mode === "mode_b" && relation.kind === "measurement_causal" && relation.composite === variable.id) {
        indicatorId = relation.indicator;
      } else {
        return [];
      }
      const indicator = variables.get(indicatorId);
      if (indicator?.kind !== "observed") throw new SemModelV4OperationError("pls.indicator_invalid", indicatorId);
      return [{ variable_id: indicatorId, source_column: indicator.source_column, parameter_id: relation.parameter }];
    }).sort((left, right) => left.variable_id.localeCompare(right.variable_id)),
    };
  }).sort((left, right) => left.construct_id.localeCompare(right.construct_id));
  const paths = model.relations.filter((relation): relation is Extract<SemRelationV4, { kind: "structural" }> => relation.kind === "structural").map((relation) => {
    if (relation.intercept_parameter) throw new SemModelV4OperationError("pls.intercept_unsupported", relation.id);
    if (variables.get(relation.source)?.kind !== "composite" || variables.get(relation.target)?.kind !== "composite") throw new SemModelV4OperationError("pls.path_endpoint_unsupported", relation.id);
    return { relation_id: relation.id, source: relation.source, target: relation.target, parameter_id: relation.parameter };
  }).sort((left, right) => left.relation_id.localeCompare(right.relation_id));
  return deepFreeze({ model_id: model.id, scientific_hash_input: scientificSemModelV4HashInput(model), dataset_id: model.data_binding.dataset_id, blocks, paths });
}

export function compileCbsemPlanV2(model: SemModelV4): CompiledCbsemPlanV2 {
  assertValid(model);
  if (model.group.kind !== "single_group") throw new SemModelV4OperationError("cbsem.multigroup_unsupported", "group");
  if (model.derived_terms.length) throw new SemModelV4OperationError("cbsem.derived_terms_unsupported", model.derived_terms[0].id);
  const groupOverride = model.parameters.find((parameter) => parameter.group_overrides?.length);
  if (groupOverride) throw new SemModelV4OperationError("cbsem.parameter_group_overrides_unsupported", groupOverride.id);
  if (model.data_binding.kind === "raw" && (
    (model.data_binding.missing_data !== "listwise_deletion" && model.data_binding.missing_data !== "mean_replacement")
    || model.data_binding.weight
    || model.data_binding.cluster_variable
    || model.data_binding.strata_variable
  )) throw new SemModelV4OperationError("cbsem.data_binding_unsupported", "data_binding");
  if (model.data_binding.kind !== "raw" && (model.data_binding.means || model.data_binding.standard_deviations || model.data_binding.sample.effective_sample_size != null || model.data_binding.sample.degrees_of_freedom != null || Object.keys(model.data_binding.sample.group_sample_sizes ?? {}).length)) throw new SemModelV4OperationError("cbsem.matrix_metadata_unsupported", "data_binding");
  for (const variable of model.variables) {
    if (variable.kind === "composite" || variable.kind === "derived") throw new SemModelV4OperationError("cbsem.variable_unsupported", variable.id);
    if (variable.kind === "observed" && variable.scale !== "continuous") throw new SemModelV4OperationError("cbsem.scale_unsupported", variable.id);
    if (variable.kind === "observed" && (
      variable.transformation_lineage.length
      || (variable.missing_markers.length && !(model.data_binding.kind === "raw" && model.data_binding.missing_data === "mean_replacement"))
    )) throw new SemModelV4OperationError("cbsem.observed_metadata_unsupported", variable.id);
    if (variable.kind === "common_factor" && (variable.mean_policy.kind !== "fixed_zero" || variable.disturbance_policy.kind === "fixed_zero")) throw new SemModelV4OperationError("cbsem.factor_policy_unsupported", variable.id);
  }
  for (const parameter of model.parameters) if (["weight", "intercept", "mean", "threshold"].includes(parameter.target.kind)) throw new SemModelV4OperationError("cbsem.parameter_unsupported", parameter.id);
  const varianceEndpoints = new Set(model.parameters.flatMap((parameter) => parameter.target.kind === "variance" ? [endpointKey(parameter.target.endpoint)] : []));
  for (const variable of model.variables) if (variable.kind === "common_factor" && !varianceEndpoints.has(endpointKey({ kind: "variable", id: variable.id })) && !varianceEndpoints.has(endpointKey({ kind: "disturbance_of", id: variable.id }))) throw new SemModelV4OperationError("cbsem.variance_missing", variable.id);
  const measured = new Set(model.relations.flatMap((relation) => relation.kind === "measurement_effect" ? [relation.indicator] : []));
  for (const indicator of measured) if (!varianceEndpoints.has(endpointKey({ kind: "residual_of", id: indicator }))) throw new SemModelV4OperationError("cbsem.residual_variance_missing", indicator);
  const loadings = model.relations.flatMap((relation) => relation.kind === "measurement_effect" ? [{ relation_id: relation.id, factor: relation.construct, indicator: relation.indicator, parameter_id: relation.parameter }] : []).sort((left, right) => left.relation_id.localeCompare(right.relation_id));
  if (model.relations.some((relation) => relation.kind === "measurement_causal")) throw new SemModelV4OperationError("cbsem.causal_measurement_unsupported", "model");
  const regressions = model.relations.flatMap((relation) => relation.kind === "structural" ? [{ relation_id: relation.id, source: relation.source, target: relation.target, parameter_id: relation.parameter }] : []).sort((left, right) => left.relation_id.localeCompare(right.relation_id));
  const covariances = model.relations.flatMap((relation) => relation.kind === "covariance" ? [{ relation_id: relation.id, left: relation.left, right: relation.right, parameter_id: relation.parameter }] : []).sort((left, right) => left.relation_id.localeCompare(right.relation_id));
  const canonical = canonicalizeSemModelV4(model);
  return deepFreeze({
    model_id: model.id,
    scientific_hash_input: scientificSemModelV4HashInput(model),
    input: canonical.data_binding,
    observed_variables: canonical.variables.filter((variable) => variable.kind === "observed").map((variable) => variable.id),
    factors: canonical.variables.filter((variable) => variable.kind === "common_factor").map((variable) => variable.id),
    loadings,
    regressions,
    covariances,
    parameters: canonical.parameters,
    constraints: canonical.constraints,
    has_feedback: hasStructuralFeedbackV4(model),
  });
}

function assertValid(model: SemModelV4) {
  const issues = validateSemModelV4(model);
  if (issues.length) throw new SemModelV4OperationError("model.invalid", "model", issues);
}

function deepFreeze<T>(value: T): T {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value as Record<string, unknown>)) deepFreeze(child);
  }
  return value;
}

export type LegacyBasicModelInterpretationV4 = "unspecified" | "pls_composite" | "cbsem_common_factor";

export interface LegacyBasicModelV4Input {
  id: string;
  name: string;
  constructs: Array<{
    id: string;
    name: string;
    short_name: string;
    mode: "reflective" | "formative";
    indicators: string[];
  }>;
  paths: Array<{ source: string; target: string }>;
  controls?: unknown[];
  higher_order_constructs?: unknown[];
  interactions?: unknown[];
}

export interface LegacyDisplayCovarianceV4 {
  id: string;
  left_construct: string;
  right_construct: string;
  label?: string | null;
}

export function convertLegacyBasicModelV4(
  legacy: LegacyBasicModelV4Input,
  interpretation: LegacyBasicModelInterpretationV4,
  displayCovariances: readonly LegacyDisplayCovarianceV4[] = [],
): SemModelV4 {
  if (interpretation === "unspecified") throw new SemModelV4OperationError("migration.interpretation_required", "legacy_model");
  if (legacy.controls?.length || legacy.higher_order_constructs?.length || legacy.interactions?.length) throw new SemModelV4OperationError("migration.advanced_semantics", "legacy_model");
  if (interpretation === "cbsem_common_factor") {
    const formative = legacy.constructs.find((construct) => construct.mode === "formative");
    if (formative) throw new SemModelV4OperationError("migration.formative_cbsem", formative.id);
  }
  const legacyIndicatorOwners = new Map<string, string>();
  for (const construct of legacy.constructs) for (const indicator of construct.indicators) {
    const firstOwner = legacyIndicatorOwners.get(indicator);
    if (firstOwner) throw new SemModelV4OperationError("migration.duplicate_indicator", `${indicator}:${firstOwner}:${construct.id}`);
    legacyIndicatorOwners.set(indicator, construct.id);
  }
  const constructIds = new Map(legacy.constructs.map((construct) => [construct.id, `construct:${construct.id}`]));
  const endogenousConstructs = new Set(legacy.paths.map((path) => path.target));
  const indicators = [...new Set(legacy.constructs.flatMap((construct) => construct.indicators))].sort();
  const indicatorIds = new Map(indicators.map((indicator) => [indicator, `observed:${indicator}`]));
  const variables: SemVariableV4[] = indicators.map((indicator) => ({
    kind: "observed",
    id: indicatorIds.get(indicator)!,
    label: indicator,
    source_column: indicator,
    scale: "continuous",
    role: "indicator",
    categories: [],
    value_labels: {},
    missing_markers: [],
    transformation_lineage: [],
  }));
  const relations: SemRelationV4[] = [];
  const parameters: SemParameterV4[] = [];
  for (const construct of legacy.constructs) {
    const constructId = constructIds.get(construct.id)!;
    const varianceId = stableSemId("variance", [construct.id]);
    if (interpretation === "pls_composite") variables.push({ kind: "composite", id: constructId, label: construct.name, weighting: { kind: construct.mode === "reflective" ? "mode_a" : "mode_b" } });
    else variables.push({
      kind: "common_factor",
      id: constructId,
      label: construct.name,
      identification: { kind: "marker_loading", indicator: indicatorIds.get(construct.indicators[0]) ?? "missing-marker" },
      mean_policy: { kind: "fixed_zero" },
      disturbance_policy: endogenousConstructs.has(construct.id)
        ? { kind: "endogenous_disturbance", parameter: varianceId }
        : { kind: "exogenous_variance", parameter: varianceId },
    });
    for (const [index, indicator] of construct.indicators.entries()) {
      const indicatorId = indicatorIds.get(indicator)!;
      const relationId = stableSemId("measurement", [construct.id, indicator]);
      const parameterId = stableSemId("parameter", [construct.id, indicator]);
      const effect = interpretation === "cbsem_common_factor" || construct.mode === "reflective";
      relations.push(effect
        ? { kind: "measurement_effect", id: relationId, construct: constructId, indicator: indicatorId, parameter: parameterId }
        : { kind: "measurement_causal", id: relationId, indicator: indicatorId, composite: constructId, parameter: parameterId });
      const target: SemParameterTargetV4 = effect
        ? { kind: "loading", construct: constructId, indicator: indicatorId }
        : { kind: "weight", indicator: indicatorId, composite: constructId };
      parameters.push(interpretation === "cbsem_common_factor" && index === 0
        ? { kind: "fixed", id: parameterId, label: `${construct.short_name} -> ${indicator}`, target, value: 1, group_overrides: [] }
        : { kind: "free", id: parameterId, label: `${construct.short_name} -> ${indicator}`, target, start: interpretation === "cbsem_common_factor" ? 0.7 : null, group_overrides: [] });
    }
    if (interpretation === "cbsem_common_factor") parameters.push({ kind: "free", id: varianceId, label: `Var(${construct.short_name})`, target: { kind: "variance", endpoint: { kind: endogenousConstructs.has(construct.id) ? "disturbance_of" : "variable", id: constructId } }, start: 1, lower: 0, group_overrides: [] });
  }
  if (interpretation === "cbsem_common_factor") for (const indicator of indicators) parameters.push({ kind: "free", id: stableSemId("residual_variance", [indicator]), label: `Residual variance(${indicator})`, target: { kind: "variance", endpoint: { kind: "residual_of", id: indicatorIds.get(indicator)! } }, start: 0.5, lower: 0, group_overrides: [] });
  for (const path of legacy.paths) {
    const source = constructIds.get(path.source);
    const target = constructIds.get(path.target);
    if (!source) throw new SemModelV4OperationError("migration.unknown_structural_construct", path.source);
    if (!target) throw new SemModelV4OperationError("migration.unknown_structural_construct", path.target);
    const parameter = stableSemId("regression", [path.source, path.target]);
    relations.push({ kind: "structural", id: stableSemId("structural", [path.source, path.target]), source, target, parameter, intercept_parameter: null });
    parameters.push({ kind: "free", id: parameter, label: `${path.source} -> ${path.target}`, target: { kind: "regression", source, target }, group_overrides: [] });
  }
  const annotations: SemAnnotationV4[] = displayCovariances.map((covariance) => {
    const left = constructIds.get(covariance.left_construct);
    const right = constructIds.get(covariance.right_construct);
    if (!left || !right) throw new SemModelV4OperationError("migration.display_covariance_unknown", covariance.id);
    return { kind: "display_only_covariance", id: covariance.id, left, right, label: covariance.label };
  });
  const model = canonicalizeSemModelV4({
    schema_version: SEM_MODEL_V4_SCHEMA_VERSION,
    id: legacy.id,
    name: legacy.name,
    variables,
    relations,
    parameters,
    constraints: [],
    derived_terms: [],
    group: { kind: "single_group" },
    data_binding: { kind: "raw", dataset_id: "legacy-unbound", missing_data: "listwise_deletion", weight: null, cluster_variable: null, strata_variable: null },
    annotations,
    presentation: { kind: "none" },
  });
  assertValid(model);
  return model;
}

function stableSemId(prefix: string, parts: readonly string[]): string {
  const encoder = new TextEncoder();
  const encoded = parts.map((part) => [...encoder.encode(part)].map((byte) => byte.toString(16).padStart(2, "0")).join("")).join("_");
  return `${prefix}_${encoded}`;
}
