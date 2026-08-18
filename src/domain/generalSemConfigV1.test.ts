import { describe, expect, it } from "vitest";
import {
  defaultGeneralSemConfigV1,
  parseGeneralSemConfigV1,
  type GeneralSemBootstrapIntervalV1,
  type GeneralSemConfigV1,
  type GeneralSemConfigV1ErrorCode,
  type GeneralSemInferenceTailV1,
} from "./generalSemConfigV1";

function clone<T>(value: T): T {
  return structuredClone(value);
}

function comprehensiveConfig(): GeneralSemConfigV1 {
  return {
    schema_version: 1,
    requested_effect_estimands: [
      {
        kind: "specific_path",
        estimand_id: "effect:01:specific",
        // Deliberately nonlexical: relation order is scientific path content.
        ordered_relation_ids: ["relation:z", "relation:a"],
      },
      {
        kind: "total_indirect",
        estimand_id: "effect:02:total_indirect",
        source_id: "construct:x",
        target_id: "construct:y",
      },
      {
        kind: "total_effect",
        estimand_id: "effect:03:total",
        source_id: "construct:x",
        target_id: "construct:y",
      },
    ],
    conditional_effect_probes: [
      {
        probe_id: "probe:01:explicit",
        moderator_id: "construct:moderator_a",
        values: { kind: "explicit", values: [-1.5, 0, 2.25] },
      },
      {
        probe_id: "probe:02:data",
        moderator_id: "construct:moderator_b",
        values: { kind: "data_derived_mean_plus_minus_one_sd" },
      },
    ],
    inference: {
      kind: "case_bootstrap",
      resamples: 5_000,
      seed: 2_026_081_800,
      confidence_level: 0.95,
      interval: "bca",
      tail: "one_sided_upper",
    },
    output_policy: {
      max_materialized_specific_paths: 2_048,
      lazy_specific_path_materialization: true,
      when_specific_path_limit_exceeded: "return_lazy",
    },
  };
}

function expectCode(value: unknown, code: GeneralSemConfigV1ErrorCode): void {
  expect(() => parseGeneralSemConfigV1(value)).toThrowError(expect.objectContaining({ code }));
}

describe("GeneralSemConfigV1", () => {
  it("mirrors the Rust default and round-trips canonical requests without reordering paths", () => {
    const defaultConfig = defaultGeneralSemConfigV1();
    expect(parseGeneralSemConfigV1(JSON.parse(JSON.stringify(defaultConfig)))).toEqual(defaultConfig);
    expect(defaultConfig).toEqual({
      schema_version: 1,
      requested_effect_estimands: [],
      conditional_effect_probes: [],
      inference: { kind: "none" },
      output_policy: {
        max_materialized_specific_paths: 10_000,
        lazy_specific_path_materialization: false,
        when_specific_path_limit_exceeded: "error",
      },
    });

    const config = comprehensiveConfig();
    const encoded = JSON.stringify(config);
    const parsed = parseGeneralSemConfigV1(JSON.parse(encoded));
    expect(parsed).toEqual(config);
    expect(JSON.stringify(parsed)).toBe(encoded);
    expect(parsed.requested_effect_estimands[0]).toMatchObject({
      ordered_relation_ids: ["relation:z", "relation:a"],
    });
  });

  it("rejects missing and unknown fields at every strict tagged boundary", () => {
    expectCode(
      { ...defaultGeneralSemConfigV1(), unexpected: true },
      "general_sem_config_v1.field_unknown",
    );

    const missing = clone(defaultGeneralSemConfigV1()) as Partial<GeneralSemConfigV1>;
    delete missing.inference;
    expectCode(missing, "general_sem_config_v1.field_missing");

    const nested = comprehensiveConfig();
    (nested.inference as unknown as Record<string, unknown>).unexpected = true;
    expectCode(nested, "general_sem_config_v1.field_unknown");

    const tagged = comprehensiveConfig();
    (tagged.conditional_effect_probes[1].values as unknown as Record<string, unknown>).authored = true;
    expectCode(tagged, "general_sem_config_v1.field_unknown");

    const truncate = comprehensiveConfig();
    (truncate.output_policy as unknown as Record<string, unknown>)
      .when_specific_path_limit_exceeded = "truncate";
    expectCode(truncate, "general_sem_config_v1.enum_invalid");
  });

  it("enforces schema 1 and stable, globally unique, canonically ordered request ids", () => {
    const wrongSchema = comprehensiveConfig();
    (wrongSchema as unknown as { schema_version: number }).schema_version = 2;
    expectCode(wrongSchema, "general_sem_config_v1.schema_version");

    const padded = comprehensiveConfig();
    padded.requested_effect_estimands[0].estimand_id = " effect:01:specific";
    expectCode(padded, "general_sem_config_v1.stable_id_whitespace");

    const control = comprehensiveConfig();
    control.conditional_effect_probes[0].moderator_id = "construct:\u0007moderator";
    expectCode(control, "general_sem_config_v1.stable_id_control");

    const nonNfc = comprehensiveConfig();
    nonNfc.conditional_effect_probes[0].moderator_id = "construct:e\u0301";
    expectCode(nonNfc, "general_sem_config_v1.stable_id_nfc");

    const duplicateAcrossCollections = comprehensiveConfig();
    duplicateAcrossCollections.conditional_effect_probes[0].probe_id = "effect:01:specific";
    expectCode(duplicateAcrossCollections, "general_sem_config_v1.request_id_duplicate");

    const outOfOrder = comprehensiveConfig();
    outOfOrder.requested_effect_estimands.reverse();
    expectCode(outOfOrder, "general_sem_config_v1.request_order_noncanonical");

    const rustUnicodeOrder = defaultGeneralSemConfigV1();
    rustUnicodeOrder.requested_effect_estimands = [
      {
        kind: "total_effect",
        estimand_id: "effect:\uE000",
        source_id: "construct:a",
        target_id: "construct:b",
      },
      {
        kind: "total_effect",
        estimand_id: "effect:\u{1F600}",
        source_id: "construct:a",
        target_id: "construct:c",
      },
    ];
    expect(parseGeneralSemConfigV1(rustUnicodeOrder)).toEqual(rustUnicodeOrder);
  });

  it("enforces identifiable unique specific paths and unique scientific estimands", () => {
    for (const relationIds of [[], ["relation:a"]]) {
      const tooShort = comprehensiveConfig();
      if (tooShort.requested_effect_estimands[0].kind === "specific_path") {
        tooShort.requested_effect_estimands[0].ordered_relation_ids = relationIds;
      }
      expectCode(tooShort, "general_sem_config_v1.specific_path_too_short");
    }

    const repeatedRelation = comprehensiveConfig();
    if (repeatedRelation.requested_effect_estimands[0].kind === "specific_path") {
      repeatedRelation.requested_effect_estimands[0].ordered_relation_ids = [
        "relation:a",
        "relation:a",
      ];
    }
    expectCode(repeatedRelation, "general_sem_config_v1.specific_path_relation_duplicate");

    const duplicatePath = comprehensiveConfig();
    duplicatePath.requested_effect_estimands.splice(1, 0, {
      kind: "specific_path",
      estimand_id: "effect:01a:same_path",
      ordered_relation_ids: ["relation:z", "relation:a"],
    });
    expectCode(duplicatePath, "general_sem_config_v1.effect_estimand_duplicate");

    const duplicateTotal = comprehensiveConfig();
    duplicateTotal.requested_effect_estimands.splice(2, 0, {
      kind: "total_indirect",
      estimand_id: "effect:02a:same_total_indirect",
      source_id: "construct:x",
      target_id: "construct:y",
    });
    expectCode(duplicateTotal, "general_sem_config_v1.effect_estimand_duplicate");

    const equalEndpoints = comprehensiveConfig();
    const total = equalEndpoints.requested_effect_estimands[1];
    if (total.kind === "total_indirect") total.target_id = total.source_id;
    expectCode(equalEndpoints, "general_sem_config_v1.effect_endpoints_equal");
  });

  it("requires explicit probe values to be nonempty, finite, and strictly increasing", () => {
    const invalidCases: Array<[number[], GeneralSemConfigV1ErrorCode]> = [
      [[], "general_sem_config_v1.explicit_values_empty"],
      [[0, Number.POSITIVE_INFINITY], "general_sem_config_v1.finite_required"],
      [[0, 0], "general_sem_config_v1.explicit_values_noncanonical"],
      [[1, -1], "general_sem_config_v1.explicit_values_noncanonical"],
      [[-0, 0], "general_sem_config_v1.explicit_values_noncanonical"],
    ];
    for (const [values, code] of invalidCases) {
      const config = comprehensiveConfig();
      config.conditional_effect_probes[0].values = { kind: "explicit", values };
      expectCode(config, code);
    }

    const oneValue = comprehensiveConfig();
    oneValue.conditional_effect_probes[0].values = { kind: "explicit", values: [-0] };
    expect(parseGeneralSemConfigV1(oneValue).conditional_effect_probes[0].values)
      .toEqual({ kind: "explicit", values: [-0] });
  });

  it("requires explicit deterministic bootstrap inference and accepts every interval-tail mode", () => {
    const intervals: GeneralSemBootstrapIntervalV1[] = ["percentile", "bca"];
    const tails: GeneralSemInferenceTailV1[] = [
      "two_sided",
      "one_sided_lower",
      "one_sided_upper",
    ];
    for (const interval of intervals) {
      for (const tail of tails) {
        const config = comprehensiveConfig();
        config.inference = {
          kind: "case_bootstrap",
          resamples: 1,
          seed: 0,
          confidence_level: 0.95,
          interval,
          tail,
        };
        expect(parseGeneralSemConfigV1(config).inference).toEqual(config.inference);
      }
    }

    const zeroResamples = comprehensiveConfig();
    if (zeroResamples.inference.kind === "case_bootstrap") {
      zeroResamples.inference.resamples = 0;
    }
    expectCode(zeroResamples, "general_sem_config_v1.bootstrap_resamples_zero");

    for (const confidenceLevel of [Number.NaN, Number.NEGATIVE_INFINITY, 0, 1, 1.01]) {
      const config = comprehensiveConfig();
      if (config.inference.kind === "case_bootstrap") {
        config.inference.confidence_level = confidenceLevel;
      }
      expectCode(
        config,
        Number.isFinite(confidenceLevel)
          ? "general_sem_config_v1.confidence_level_invalid"
          : "general_sem_config_v1.finite_required",
      );
    }

    const unsafeSeed = comprehensiveConfig();
    if (unsafeSeed.inference.kind === "case_bootstrap") {
      unsafeSeed.inference.seed = Number.MAX_SAFE_INTEGER + 1;
    }
    expectCode(unsafeSeed, "general_sem_config_v1.u64_safe_integer_required");

    const maxResamples = comprehensiveConfig();
    if (maxResamples.inference.kind === "case_bootstrap") {
      maxResamples.inference.resamples = 0xffff_ffff;
      maxResamples.inference.seed = Number.MAX_SAFE_INTEGER;
    }
    expect(parseGeneralSemConfigV1(maxResamples).inference).toEqual(maxResamples.inference);
  });

  it("requires a positive materialization limit and an explicit nontruncating limit policy", () => {
    const zero = comprehensiveConfig();
    zero.output_policy.max_materialized_specific_paths = 0;
    expectCode(zero, "general_sem_config_v1.max_paths_zero");

    const incoherent = comprehensiveConfig();
    incoherent.output_policy.lazy_specific_path_materialization = false;
    expectCode(incoherent, "general_sem_config_v1.lazy_policy_incoherent");

    const failClosed = comprehensiveConfig();
    failClosed.output_policy = {
      max_materialized_specific_paths: 1,
      lazy_specific_path_materialization: false,
      when_specific_path_limit_exceeded: "error",
    };
    expect(parseGeneralSemConfigV1(failClosed).output_policy).toEqual(failClosed.output_policy);

    const lazy = comprehensiveConfig();
    lazy.output_policy = {
      max_materialized_specific_paths: 0xffff_ffff,
      lazy_specific_path_materialization: true,
      when_specific_path_limit_exceeded: "return_lazy",
    };
    expect(parseGeneralSemConfigV1(lazy).output_policy).toEqual(lazy.output_policy);
  });
});
