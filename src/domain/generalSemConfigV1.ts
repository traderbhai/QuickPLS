export const GENERAL_SEM_CONFIG_V1_SCHEMA_VERSION = 1 as const;
export const DEFAULT_MAX_MATERIALIZED_SPECIFIC_PATHS_V1 = 10_000;

const U32_MAX = 0xffff_ffff;
const CONTROL_CHARACTER = /[\u0000-\u001f\u007f-\u009f]/u;

type WireRecord = Record<string, unknown>;

export type GeneralSemEffectEstimandV1 =
  | {
      kind: "specific_path";
      estimand_id: string;
      /**
       * Scientific path order is preserved exactly. The topology compiler,
       * rather than this request parser, validates directed path continuity.
       */
      ordered_relation_ids: string[];
    }
  | {
      kind: "total_indirect";
      estimand_id: string;
      source_id: string;
      target_id: string;
    }
  | {
      kind: "total_effect";
      estimand_id: string;
      source_id: string;
      target_id: string;
    };

export type GeneralSemConditionalProbeValuesV1 =
  | { kind: "data_derived_mean_plus_minus_one_sd" }
  | { kind: "explicit"; values: number[] };

export interface GeneralSemConditionalEffectProbeV1 {
  probe_id: string;
  moderator_id: string;
  values: GeneralSemConditionalProbeValuesV1;
}

export type GeneralSemBootstrapIntervalV1 = "percentile" | "bca";

export type GeneralSemInferenceTailV1 =
  | "two_sided"
  | "one_sided_lower"
  | "one_sided_upper";

export type GeneralSemInferenceV1 =
  | { kind: "none" }
  | {
      kind: "case_bootstrap";
      resamples: number;
      /** A nonnegative safe integer so JSON preserves the deterministic seed exactly. */
      seed: number;
      confidence_level: number;
      interval: GeneralSemBootstrapIntervalV1;
      tail: GeneralSemInferenceTailV1;
    };

export type GeneralSemSpecificPathLimitBehaviorV1 = "error" | "return_lazy";

export interface GeneralSemOutputPolicyV1 {
  max_materialized_specific_paths: number;
  lazy_specific_path_materialization: boolean;
  when_specific_path_limit_exceeded: GeneralSemSpecificPathLimitBehaviorV1;
}

export interface GeneralSemConfigV1 {
  schema_version: 1;
  requested_effect_estimands: GeneralSemEffectEstimandV1[];
  conditional_effect_probes: GeneralSemConditionalEffectProbeV1[];
  inference: GeneralSemInferenceV1;
  output_policy: GeneralSemOutputPolicyV1;
}

export type GeneralSemConfigV1ErrorCode =
  | "general_sem_config_v1.object_required"
  | "general_sem_config_v1.field_missing"
  | "general_sem_config_v1.field_unknown"
  | "general_sem_config_v1.array_required"
  | "general_sem_config_v1.schema_version"
  | "general_sem_config_v1.stable_id_required"
  | "general_sem_config_v1.stable_id_whitespace"
  | "general_sem_config_v1.stable_id_control"
  | "general_sem_config_v1.stable_id_nfc"
  | "general_sem_config_v1.request_id_duplicate"
  | "general_sem_config_v1.request_order_noncanonical"
  | "general_sem_config_v1.specific_path_too_short"
  | "general_sem_config_v1.specific_path_relation_duplicate"
  | "general_sem_config_v1.effect_estimand_duplicate"
  | "general_sem_config_v1.effect_endpoints_equal"
  | "general_sem_config_v1.explicit_values_empty"
  | "general_sem_config_v1.finite_required"
  | "general_sem_config_v1.explicit_values_noncanonical"
  | "general_sem_config_v1.u32_required"
  | "general_sem_config_v1.u64_safe_integer_required"
  | "general_sem_config_v1.enum_invalid"
  | "general_sem_config_v1.boolean_required"
  | "general_sem_config_v1.bootstrap_resamples_zero"
  | "general_sem_config_v1.confidence_level_invalid"
  | "general_sem_config_v1.max_paths_zero"
  | "general_sem_config_v1.lazy_policy_incoherent";

export class GeneralSemConfigV1Error extends Error {
  constructor(
    public readonly code: GeneralSemConfigV1ErrorCode,
    public readonly path: string,
    message: string,
  ) {
    super(message);
    this.name = "GeneralSemConfigV1Error";
  }
}

function fail(code: GeneralSemConfigV1ErrorCode, path: string, message: string): never {
  throw new GeneralSemConfigV1Error(code, path, message);
}

function recordAt(value: unknown, path: string): WireRecord {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    return fail("general_sem_config_v1.object_required", path, `${path} must be an object.`);
  }
  return value as WireRecord;
}

function exactRecordAt(
  value: unknown,
  required: readonly string[],
  path: string,
): WireRecord {
  const record = recordAt(value, path);
  const allowed = new Set(required);
  for (const key of required) {
    if (!Object.prototype.hasOwnProperty.call(record, key)) {
      fail(
        "general_sem_config_v1.field_missing",
        `${path}.${key}`,
        `${path}.${key} is required.`,
      );
    }
  }
  for (const key of Object.keys(record)) {
    if (!allowed.has(key)) {
      fail(
        "general_sem_config_v1.field_unknown",
        `${path}.${key}`,
        `${path}.${key} is not part of the GeneralSemConfigV1 contract.`,
      );
    }
  }
  return record;
}

function arrayAt(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) {
    return fail("general_sem_config_v1.array_required", path, `${path} must be an array.`);
  }
  return value;
}

function u32At(value: unknown, path: string): number {
  if (
    !Number.isSafeInteger(value)
    || Object.is(value, -0)
    || (value as number) < 0
    || (value as number) > U32_MAX
  ) {
    return fail(
      "general_sem_config_v1.u32_required",
      path,
      `${path} must be an unsigned 32-bit integer.`,
    );
  }
  return value as number;
}

function u64SafeIntegerAt(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value) || Object.is(value, -0) || (value as number) < 0) {
    return fail(
      "general_sem_config_v1.u64_safe_integer_required",
      path,
      `${path} must be a nonnegative safe integer so JSON preserves the seed exactly.`,
    );
  }
  return value as number;
}

function finiteAt(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return fail(
      "general_sem_config_v1.finite_required",
      path,
      `${path} must be finite.`,
    );
  }
  return value;
}

function booleanAt(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") {
    return fail(
      "general_sem_config_v1.boolean_required",
      path,
      `${path} must be a boolean.`,
    );
  }
  return value;
}

function enumAt<T extends string>(value: unknown, allowed: readonly T[], path: string): T {
  if (typeof value !== "string" || !allowed.includes(value as T)) {
    return fail(
      "general_sem_config_v1.enum_invalid",
      path,
      `${path} has an unsupported value.`,
    );
  }
  return value as T;
}

function stableIdAt(value: unknown, path: string): string {
  if (typeof value !== "string" || !value.trim()) {
    return fail(
      "general_sem_config_v1.stable_id_required",
      path,
      `${path} must be a nonempty stable id.`,
    );
  }
  if (value.trim() !== value) {
    return fail(
      "general_sem_config_v1.stable_id_whitespace",
      path,
      `${path} cannot contain surrounding whitespace.`,
    );
  }
  if (CONTROL_CHARACTER.test(value)) {
    return fail(
      "general_sem_config_v1.stable_id_control",
      path,
      `${path} cannot contain control characters.`,
    );
  }
  if (value.normalize("NFC") !== value) {
    return fail(
      "general_sem_config_v1.stable_id_nfc",
      path,
      `${path} must use Unicode NFC normalization.`,
    );
  }
  return value;
}

/** Rust String ordering is lexicographic by UTF-8, which follows scalar-value order. */
function compareRustStrings(left: string, right: string): number {
  const leftScalars = Array.from(left, (scalar) => scalar.codePointAt(0) as number);
  const rightScalars = Array.from(right, (scalar) => scalar.codePointAt(0) as number);
  const length = Math.min(leftScalars.length, rightScalars.length);
  for (let index = 0; index < length; index += 1) {
    if (leftScalars[index] !== rightScalars[index]) {
      return leftScalars[index] < rightScalars[index] ? -1 : 1;
    }
  }
  return Math.sign(leftScalars.length - rightScalars.length);
}

function validateCanonicalRequestOrder(
  previousId: string | undefined,
  currentId: string,
  path: string,
): void {
  if (previousId !== undefined && compareRustStrings(previousId, currentId) >= 0) {
    fail(
      "general_sem_config_v1.request_order_noncanonical",
      path,
      `${path} must follow ${previousId} in strict stable-id order.`,
    );
  }
}

function parseEffectEstimand(value: unknown, path: string): GeneralSemEffectEstimandV1 {
  const candidate = recordAt(value, path);
  if (candidate.kind === "specific_path") {
    const estimand = exactRecordAt(
      candidate,
      ["kind", "estimand_id", "ordered_relation_ids"],
      path,
    );
    const estimandId = stableIdAt(estimand.estimand_id, `${path}.estimand_id`);
    const orderedRelationIds = arrayAt(
      estimand.ordered_relation_ids,
      `${path}.ordered_relation_ids`,
    ).map((relationId, index) => stableIdAt(
      relationId,
      `${path}.ordered_relation_ids[${index}]`,
    ));
    if (orderedRelationIds.length < 2) {
      fail(
        "general_sem_config_v1.specific_path_too_short",
        `${path}.ordered_relation_ids`,
        `${path} requires at least two ordered relation ids.`,
      );
    }
    const relationIds = new Set<string>();
    orderedRelationIds.forEach((relationId, index) => {
      if (relationIds.has(relationId)) {
        fail(
          "general_sem_config_v1.specific_path_relation_duplicate",
          `${path}.ordered_relation_ids[${index}]`,
          `${path} repeats relation id ${relationId}.`,
        );
      }
      relationIds.add(relationId);
    });
    return {
      kind: "specific_path",
      estimand_id: estimandId,
      ordered_relation_ids: orderedRelationIds,
    };
  }
  if (candidate.kind === "total_indirect" || candidate.kind === "total_effect") {
    const estimand = exactRecordAt(
      candidate,
      ["kind", "estimand_id", "source_id", "target_id"],
      path,
    );
    const estimandId = stableIdAt(estimand.estimand_id, `${path}.estimand_id`);
    const sourceId = stableIdAt(estimand.source_id, `${path}.source_id`);
    const targetId = stableIdAt(estimand.target_id, `${path}.target_id`);
    if (sourceId === targetId) {
      fail(
        "general_sem_config_v1.effect_endpoints_equal",
        path,
        `${path} requires distinct source and target ids.`,
      );
    }
    return {
      kind: candidate.kind,
      estimand_id: estimandId,
      source_id: sourceId,
      target_id: targetId,
    };
  }
  return fail(
    "general_sem_config_v1.enum_invalid",
    `${path}.kind`,
    `${path}.kind has an unsupported effect-estimand kind.`,
  );
}

function effectEstimandSignature(estimand: GeneralSemEffectEstimandV1): string {
  return estimand.kind === "specific_path"
    ? JSON.stringify([estimand.kind, ...estimand.ordered_relation_ids])
    : JSON.stringify([estimand.kind, estimand.source_id, estimand.target_id]);
}

function parseConditionalProbeValues(
  value: unknown,
  path: string,
): GeneralSemConditionalProbeValuesV1 {
  const candidate = recordAt(value, path);
  if (candidate.kind === "data_derived_mean_plus_minus_one_sd") {
    exactRecordAt(candidate, ["kind"], path);
    return { kind: "data_derived_mean_plus_minus_one_sd" };
  }
  if (candidate.kind === "explicit") {
    const explicit = exactRecordAt(candidate, ["kind", "values"], path);
    const values = arrayAt(explicit.values, `${path}.values`).map((item, index) =>
      finiteAt(item, `${path}.values[${index}]`));
    if (values.length === 0) {
      fail(
        "general_sem_config_v1.explicit_values_empty",
        `${path}.values`,
        `${path}.values requires at least one value.`,
      );
    }
    for (let index = 1; index < values.length; index += 1) {
      if (!(values[index - 1] < values[index])) {
        fail(
          "general_sem_config_v1.explicit_values_noncanonical",
          `${path}.values[${index}]`,
          `${path}.values must be finite and strictly increasing.`,
        );
      }
    }
    return { kind: "explicit", values };
  }
  return fail(
    "general_sem_config_v1.enum_invalid",
    `${path}.kind`,
    `${path}.kind has an unsupported conditional-probe value kind.`,
  );
}

function parseConditionalEffectProbe(
  value: unknown,
  path: string,
): GeneralSemConditionalEffectProbeV1 {
  const probe = exactRecordAt(value, ["probe_id", "moderator_id", "values"], path);
  return {
    probe_id: stableIdAt(probe.probe_id, `${path}.probe_id`),
    moderator_id: stableIdAt(probe.moderator_id, `${path}.moderator_id`),
    values: parseConditionalProbeValues(probe.values, `${path}.values`),
  };
}

function parseInference(value: unknown, path: string): GeneralSemInferenceV1 {
  const candidate = recordAt(value, path);
  if (candidate.kind === "none") {
    exactRecordAt(candidate, ["kind"], path);
    return { kind: "none" };
  }
  if (candidate.kind === "case_bootstrap") {
    const inference = exactRecordAt(
      candidate,
      ["kind", "resamples", "seed", "confidence_level", "interval", "tail"],
      path,
    );
    const resamples = u32At(inference.resamples, `${path}.resamples`);
    if (resamples === 0) {
      fail(
        "general_sem_config_v1.bootstrap_resamples_zero",
        `${path}.resamples`,
        `${path}.resamples must be greater than zero.`,
      );
    }
    const confidenceLevel = finiteAt(inference.confidence_level, `${path}.confidence_level`);
    if (confidenceLevel <= 0 || confidenceLevel >= 1) {
      fail(
        "general_sem_config_v1.confidence_level_invalid",
        `${path}.confidence_level`,
        `${path}.confidence_level must be strictly between zero and one.`,
      );
    }
    return {
      kind: "case_bootstrap",
      resamples,
      seed: u64SafeIntegerAt(inference.seed, `${path}.seed`),
      confidence_level: confidenceLevel,
      interval: enumAt(
        inference.interval,
        ["percentile", "bca"] as const,
        `${path}.interval`,
      ),
      tail: enumAt(
        inference.tail,
        ["two_sided", "one_sided_lower", "one_sided_upper"] as const,
        `${path}.tail`,
      ),
    };
  }
  return fail(
    "general_sem_config_v1.enum_invalid",
    `${path}.kind`,
    `${path}.kind has an unsupported inference kind.`,
  );
}

function parseOutputPolicy(value: unknown, path: string): GeneralSemOutputPolicyV1 {
  const policy = exactRecordAt(
    value,
    [
      "max_materialized_specific_paths",
      "lazy_specific_path_materialization",
      "when_specific_path_limit_exceeded",
    ],
    path,
  );
  const maxPaths = u32At(
    policy.max_materialized_specific_paths,
    `${path}.max_materialized_specific_paths`,
  );
  if (maxPaths === 0) {
    fail(
      "general_sem_config_v1.max_paths_zero",
      `${path}.max_materialized_specific_paths`,
      `${path}.max_materialized_specific_paths must be greater than zero.`,
    );
  }
  const lazy = booleanAt(
    policy.lazy_specific_path_materialization,
    `${path}.lazy_specific_path_materialization`,
  );
  const limitBehavior = enumAt(
    policy.when_specific_path_limit_exceeded,
    ["error", "return_lazy"] as const,
    `${path}.when_specific_path_limit_exceeded`,
  );
  if (limitBehavior === "return_lazy" && !lazy) {
    fail(
      "general_sem_config_v1.lazy_policy_incoherent",
      path,
      `${path} cannot return lazy output when lazy materialization is disabled.`,
    );
  }
  return {
    max_materialized_specific_paths: maxPaths,
    lazy_specific_path_materialization: lazy,
    when_specific_path_limit_exceeded: limitBehavior,
  };
}

export function defaultGeneralSemConfigV1(): GeneralSemConfigV1 {
  return {
    schema_version: GENERAL_SEM_CONFIG_V1_SCHEMA_VERSION,
    requested_effect_estimands: [],
    conditional_effect_probes: [],
    inference: { kind: "none" },
    output_policy: {
      max_materialized_specific_paths: DEFAULT_MAX_MATERIALIZED_SPECIFIC_PATHS_V1,
      lazy_specific_path_materialization: false,
      when_specific_path_limit_exceeded: "error",
    },
  };
}

/** Strictly parses the JSON-compatible mirror of Rust GeneralSemConfigV1. */
export function parseGeneralSemConfigV1(
  input: unknown,
  path = "general_sem_config",
): GeneralSemConfigV1 {
  const config = exactRecordAt(
    input,
    [
      "schema_version",
      "requested_effect_estimands",
      "conditional_effect_probes",
      "inference",
      "output_policy",
    ],
    path,
  );
  const schemaVersion = u32At(config.schema_version, `${path}.schema_version`);
  if (schemaVersion !== GENERAL_SEM_CONFIG_V1_SCHEMA_VERSION) {
    fail(
      "general_sem_config_v1.schema_version",
      `${path}.schema_version`,
      "GeneralSemConfigV1 requires schema_version 1.",
    );
  }

  const requestIds = new Set<string>();
  const estimandSignatures = new Set<string>();
  const requestedEffectEstimands: GeneralSemEffectEstimandV1[] = [];
  let previousEstimandId: string | undefined;
  for (const [index, value] of arrayAt(
    config.requested_effect_estimands,
    `${path}.requested_effect_estimands`,
  ).entries()) {
    const estimandPath = `${path}.requested_effect_estimands[${index}]`;
    const estimand = parseEffectEstimand(value, estimandPath);
    if (requestIds.has(estimand.estimand_id)) {
      fail(
        "general_sem_config_v1.request_id_duplicate",
        `${estimandPath}.estimand_id`,
        `Request id ${estimand.estimand_id} is duplicated.`,
      );
    }
    requestIds.add(estimand.estimand_id);
    validateCanonicalRequestOrder(
      previousEstimandId,
      estimand.estimand_id,
      `${estimandPath}.estimand_id`,
    );
    previousEstimandId = estimand.estimand_id;
    const signature = effectEstimandSignature(estimand);
    if (estimandSignatures.has(signature)) {
      fail(
        "general_sem_config_v1.effect_estimand_duplicate",
        `${estimandPath}.estimand_id`,
        `Effect estimand ${estimand.estimand_id} duplicates another scientific request.`,
      );
    }
    estimandSignatures.add(signature);
    requestedEffectEstimands.push(estimand);
  }

  const conditionalEffectProbes: GeneralSemConditionalEffectProbeV1[] = [];
  let previousProbeId: string | undefined;
  for (const [index, value] of arrayAt(
    config.conditional_effect_probes,
    `${path}.conditional_effect_probes`,
  ).entries()) {
    const probePath = `${path}.conditional_effect_probes[${index}]`;
    const probe = parseConditionalEffectProbe(value, probePath);
    if (requestIds.has(probe.probe_id)) {
      fail(
        "general_sem_config_v1.request_id_duplicate",
        `${probePath}.probe_id`,
        `Request id ${probe.probe_id} is duplicated.`,
      );
    }
    requestIds.add(probe.probe_id);
    validateCanonicalRequestOrder(previousProbeId, probe.probe_id, `${probePath}.probe_id`);
    previousProbeId = probe.probe_id;
    conditionalEffectProbes.push(probe);
  }

  return {
    schema_version: GENERAL_SEM_CONFIG_V1_SCHEMA_VERSION,
    requested_effect_estimands: requestedEffectEstimands,
    conditional_effect_probes: conditionalEffectProbes,
    inference: parseInference(config.inference, `${path}.inference`),
    output_policy: parseOutputPolicy(config.output_policy, `${path}.output_policy`),
  };
}
