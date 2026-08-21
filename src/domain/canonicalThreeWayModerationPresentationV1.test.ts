import { describe, expect, it } from "vitest";
import type { CanonicalResultDocumentV2, CapabilityCellReferenceV2 } from "./canonicalResultDocumentV2";
import type { CanonicalGeneralSemResultsV1 } from "./canonicalGeneralSemResultsV1";
import {
  CANONICAL_THREE_WAY_BOOTSTRAP_RECEIPT_TABLE_ID_V1,
  CANONICAL_THREE_WAY_CONDITIONAL_TABLE_ID_V1,
  CANONICAL_THREE_WAY_INTERACTION_TABLE_ID_V1,
  CANONICAL_THREE_WAY_SECTION_ID_V1,
  CANONICAL_THREE_WAY_SIMPLE_SLOPE_CHART_ID_V1,
  CANONICAL_THREE_WAY_SIMPLE_SLOPE_TABLE_ID_V1,
  canonicalThreeWayModerationPresentationV1,
} from "./canonicalThreeWayModerationPresentationV1";

const pointCell: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_three_way_moderation_point",
  capability_version: "general_sem_pls_three_way_moderation_point_v1",
};
const bootstrapCell: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_three_way_moderation_bootstrap",
  capability_version: "general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1",
};

function fixture(): CanonicalResultDocumentV2 {
  const estimate = (value: number) => ({
    estimate: value,
    bootstrap_mean: value + 0.01,
    bootstrap_bias: 0.01,
    standard_error: 0.05,
    lower: value - 0.1,
    upper: value + 0.1,
    p_value: 0.02,
    bootstrap_usable_replicates: 499,
    bootstrap_two_sided_exceedances: 9,
  });
  const generalSemResults = {
    schema_version: 1,
    three_way_interaction_effects: [{
      effect_id: "three_way_delta:term:x_w_z",
      trace: { model_id: "model:test", capability_cell: pointCell },
      interaction_id: "term:x_w_z",
      focal_relation_id: "relation:x_y",
      interaction_effect_relation_id: "relation:x_w_z_y",
      interaction_effect_parameter_id: "parameter:x_w_z_y",
      operand_ids: ["construct:x", "construct:w", "construct:z"],
      outcome_id: "construct:y",
      generated_product_column_id: "column:x_w_z",
      stage_one_model_scientific_sha256: "d".repeat(64),
      method_version: "qpls.general-sem-pls.three-way.point.v1",
      product_scale_version: "qpls.general-sem-pls.two-stage-product.sample-standardized.v1",
      hierarchy_policy: "strong",
      hierarchy_policy_version: "qpls.general-sem-pls.interaction-hierarchy.strong.v1",
      observation_count: 120,
      unstandardized_product_mean: 0,
      unstandardized_product_sample_standard_deviation: 1,
      standardized_product_coefficient: estimate(0.2),
      scientific_rescaled_delta: estimate(0.2),
    }],
    three_way_conditional_interaction_effects: [-1, 0, 1].map((probe, index) => ({
      effect_id: `conditional_interaction:${index}`,
      trace: { model_id: "model:test", capability_cell: pointCell },
      interaction_id: "term:x_w_z",
      focal_relation_id: "relation:x_y",
      first_moderator_id: "construct:w",
      second_moderator_id: "construct:z",
      second_moderator_probe_kind: "continuous_standardized",
      second_moderator_probe_index: index,
      second_moderator_value: probe,
      value: estimate(0.2 + probe * 0.1),
    })),
    three_way_simple_slopes: [-1, 0, 1].flatMap((secondProbe, secondIndex) => (
      [-1, 0, 1].map((firstProbe, firstIndex) => ({
        effect_id: `simple_slope:${secondIndex}:${firstIndex}`,
        trace: { model_id: "model:test", capability_cell: pointCell },
        interaction_id: "term:x_w_z",
        focal_relation_id: "relation:x_y",
        first_moderator_id: "construct:w",
        second_moderator_id: "construct:z",
        first_moderator_probe_kind: "continuous_standardized",
        second_moderator_probe_kind: "continuous_standardized",
        first_probe_index: firstIndex,
        first_moderator_value: firstProbe,
        second_probe_index: secondIndex,
        second_moderator_value: secondProbe,
        value: estimate(0.3 + firstProbe * 0.1 + secondProbe * 0.05),
      }))
    )),
    three_way_moderation_bootstrap_receipt: {
      capability_cell: bootstrapCell,
      method_version: "qpls.general-sem-pls.three-way.full-model-case-bootstrap.v1",
      resamples_requested: 500,
      resamples_usable: 499,
      confidence_level: 0.95,
      seed: "17",
      workers: 2,
      all_three_way_targets_share_one_replicate_ledger: true,
      complete_model_reestimated_per_replicate: true,
      shared_stage_one_reestimated_per_replicate: true,
      score_vectors_sign_aligned_before_products: true,
      all_lower_order_and_three_way_products_recomputed_per_replicate: true,
      joint_stage_two_reestimated_per_replicate: true,
      complete_joint_point_contract_validated_per_replicate: true,
    },
  } as unknown as CanonicalGeneralSemResultsV1;
  return {
    schema_version: 2,
    document_id: "result:three-way",
    title: "Three-way moderation result",
    provenance: {
      run_id: "run:test",
      project_id: "project:test",
      model_id: "model:test",
      model_digest: "a".repeat(64),
      dataset_id: "dataset:test",
      dataset_fingerprint: "b".repeat(64),
      recipe_id: "recipe:test",
      recipe_digest: "c".repeat(64),
      capability_cell: bootstrapCell,
      method_version: "qpls.general-sem-pls.three-way.full-model-case-bootstrap.v1",
      engine_version: "engine:v1",
      seed: 17,
      workers: 2,
      started_at: "2026-08-21T00:00:00Z",
      completed_at: "2026-08-21T00:00:01Z",
    },
    capability_cells: [bootstrapCell, pointCell],
    general_sem_results: generalSemResults,
    sections: [],
    tables: [],
    charts: [],
    notices: [],
    exclusions: [],
    footnotes: [],
    presentation: {
      default_section_id: null,
      default_table_id: null,
      precision: 4,
      missing_value_label: "—",
      chart_defaults: {},
    },
  };
}

describe("canonical three-way moderation presentation V1", () => {
  it("projects strict typed rows into accessible tables and a two-dimensional chart without mutation", () => {
    const document = fixture();
    const before = structuredClone(document);

    const projected = canonicalThreeWayModerationPresentationV1(document);

    expect(document).toEqual(before);
    expect(projected).not.toBe(document);
    expect(projected.tables.map((table) => table.id)).toEqual([
      CANONICAL_THREE_WAY_INTERACTION_TABLE_ID_V1,
      CANONICAL_THREE_WAY_CONDITIONAL_TABLE_ID_V1,
      CANONICAL_THREE_WAY_SIMPLE_SLOPE_TABLE_ID_V1,
      CANONICAL_THREE_WAY_BOOTSTRAP_RECEIPT_TABLE_ID_V1,
    ]);
    expect(projected.sections).toContainEqual(expect.objectContaining({
      id: CANONICAL_THREE_WAY_SECTION_ID_V1,
      table_ids: projected.tables.map((table) => table.id),
      chart_ids: [CANONICAL_THREE_WAY_SIMPLE_SLOPE_CHART_ID_V1],
    }));
    expect(projected.charts[0]).toMatchObject({
      kind: "line",
      source_table_id: CANONICAL_THREE_WAY_SIMPLE_SLOPE_TABLE_ID_V1,
      display: {
        show_legend: true,
        x_axis_label: "construct:w",
        y_axis_label: "Simple slope of focal predictor",
      },
    });
    expect(projected.charts[0].series.map((series) => series.label)).toEqual([
      "construct:z = −1 SD",
      "construct:z = Mean",
      "construct:z = +1 SD",
    ]);
    expect(projected.charts[0].series.map((series) => series.points.map((point) => point.x))).toEqual([
      ["−1 SD", "Mean", "+1 SD"],
      ["−1 SD", "Mean", "+1 SD"],
      ["−1 SD", "Mean", "+1 SD"],
    ]);
    expect(projected.charts[0].series).toHaveLength(3);
    expect(projected.charts[0].series.every((series) => series.points.length === 3)).toBe(true);
    const slopeTable = projected.tables.find((table) => table.id === CANONICAL_THREE_WAY_SIMPLE_SLOPE_TABLE_ID_V1)!;
    expect(slopeTable.rows).toHaveLength(9);
    expect(slopeTable.rows[0]!.cells[3]).toEqual({ kind: "number", value: -1 });
    expect(slopeTable.rows[0]!.cells[6]).toEqual({ kind: "number", value: -1 });
  });

  it("overlays an existing same-ID chart with typed continuous labels without changing its exact table or source document", () => {
    const document = fixture();
    document.charts = [{
      id: CANONICAL_THREE_WAY_SIMPLE_SLOPE_CHART_ID_V1,
      title: "Simple slopes across moderator probes",
      description: "Persisted chart.",
      kind: "line",
      series: [{ id: "persisted", label: "construct:z = -1.0000", points: [{ x: -1, y: 0.1 }] }],
      source_table_id: CANONICAL_THREE_WAY_SIMPLE_SLOPE_TABLE_ID_V1,
      display: { x_axis_label: "construct:w", y_axis_label: "Simple slope of focal predictor" },
    }];
    const before = structuredClone(document);

    const projected = canonicalThreeWayModerationPresentationV1(document);

    expect(document).toEqual(before);
    expect(projected.charts).toHaveLength(1);
    expect(projected.charts[0].series[0]).toMatchObject({
      label: "construct:z = −1 SD",
      points: [
        expect.objectContaining({ x: "−1 SD" }),
        expect.objectContaining({ x: "Mean" }),
        expect.objectContaining({ x: "+1 SD" }),
      ],
    });
  });

  it("keeps binary moderator probes as actual 0/1 presentation categories", () => {
    const document = fixture();
    const results = document.general_sem_results!;
    results.three_way_conditional_interaction_effects = results.three_way_conditional_interaction_effects!.slice(0, 2).map((row, index) => ({
      ...row,
      second_moderator_probe_kind: "binary_zero_one",
      second_moderator_value: index,
    }));
    results.three_way_simple_slopes = results.three_way_simple_slopes!
      .filter((row) => row.first_probe_index < 2 && row.second_probe_index < 2)
      .map((row) => ({
        ...row,
        first_moderator_probe_kind: "binary_zero_one",
        second_moderator_probe_kind: "binary_zero_one",
        first_moderator_value: row.first_probe_index,
        second_moderator_value: row.second_probe_index,
      }));

    const projected = canonicalThreeWayModerationPresentationV1(document);

    expect(projected.charts[0].series.map((series) => series.label)).toEqual([
      "construct:z (binary) = 0",
      "construct:z (binary) = 1",
    ]);
    expect(projected.charts[0].series.map((series) => series.points.map((point) => point.x))).toEqual([
      ["0", "1"],
      ["0", "1"],
    ]);
    const slopeTable = projected.tables.find((table) => table.id === CANONICAL_THREE_WAY_SIMPLE_SLOPE_TABLE_ID_V1)!;
    expect(slopeTable.rows[0]!.cells[3]).toEqual({ kind: "number", value: 0 });
    expect(slopeTable.rows[0]!.cells[6]).toEqual({ kind: "number", value: 0 });
  });

  it("is idempotent when a canonical producer already supplied every resource", () => {
    const first = canonicalThreeWayModerationPresentationV1(fixture());
    const second = canonicalThreeWayModerationPresentationV1(first);

    expect(second).toBe(first);
  });
});
