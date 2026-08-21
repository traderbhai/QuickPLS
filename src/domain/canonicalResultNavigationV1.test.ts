import { describe, expect, it } from "vitest";
import type { CanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import {
  buildCanonicalResultNavigationV1,
  canonicalResultDocumentForItemV1,
  canonicalResultNavigationItemV1,
  filterCanonicalResultNavigationV1,
} from "./canonicalResultNavigationV1";

function documentFixture(): CanonicalResultDocumentV2 {
  const table = (id: string, title: string) => ({
    id,
    title,
    description: `${title} description`,
    columns: [{ id: "value", label: "Value", data_type: "number" as const, description: "Persisted value." }],
    rows: [{ id: `${id}_row`, cells: [{ kind: "number" as const, value: 0.5 }] }],
    footnote_ids: id === "general_sem_specific_indirect_effects" ? ["effect_note"] : [],
  });
  return {
    schema_version: 2,
    document_id: "result:test",
    title: "Unified SEM result",
    provenance: {
      run_id: "run:test",
      project_id: "project:test",
      model_id: "model:test",
      model_digest: "a".repeat(64),
      dataset_id: "dataset:test",
      dataset_fingerprint: "b".repeat(64),
      recipe_id: "recipe:test",
      recipe_digest: "c".repeat(64),
      capability_cell: {
        registry_schema_version: 2,
        capability_id: "smartpls.moderation",
        cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
        capability_version: "v1",
      },
      method_version: "general_sem_test_v1",
      engine_version: "engine_v1",
      seed: 42,
      workers: 2,
      started_at: "2026-08-21T00:00:00Z",
      completed_at: "2026-08-21T00:00:01Z",
    },
    sections: [
      { id: "measurement_model", title: "Measurement model", table_ids: ["outer_model"], chart_ids: [] },
      { id: "structural_model", title: "Structural model", table_ids: ["structural_paths"], chart_ids: [] },
      { id: "general_sem_effects", title: "Mediation effects", table_ids: ["general_sem_specific_indirect_effects"], chart_ids: [] },
      { id: "general_sem_moderation", title: "Moderation effects", table_ids: ["general_sem_interaction_effects", "general_sem_interaction_plots"], chart_ids: ["general_sem_interaction_chart_0000"] },
      { id: "general_sem_higher_order", title: "Higher-order constructs", table_ids: ["general_sem_higher_order_targets"], chart_ids: [] },
      { id: "general_sem_moderated_mediation_bootstrap", title: "Moderated-mediation bootstrap", table_ids: ["general_sem_conditional_indirect_effects", "general_sem_moderated_mediation_bootstrap_receipt"], chart_ids: [] },
      { id: "cbsem_general_sem_point", title: "CB-SEM ML estimates", table_ids: ["cbsem_general_sem_parameters", "cbsem_general_sem_fit", "cbsem_general_sem_identification"], chart_ids: [] },
    ],
    tables: [
      table("outer_model", "Outer model"),
      table("structural_paths", "Structural paths"),
      table("general_sem_specific_indirect_effects", "Specific indirect effects"),
      table("general_sem_interaction_effects", "Interaction effects"),
      table("general_sem_interaction_plots", "Interaction plot points"),
      table("general_sem_higher_order_targets", "Higher-order targets"),
      table("general_sem_conditional_indirect_effects", "Conditional indirect effects"),
      table("general_sem_moderated_mediation_bootstrap_receipt", "Moderated-mediation bootstrap receipt"),
      table("cbsem_general_sem_parameters", "CB-SEM parameters"),
      table("cbsem_general_sem_fit", "CB-SEM fit"),
      table("cbsem_general_sem_identification", "Identification"),
    ],
    charts: [{
      id: "general_sem_interaction_chart_0000",
      title: "Interaction x by w",
      description: "Conditional outcome plot.",
      kind: "line",
      series: [{ id: "low", label: "Low W", points: [{ x: -1, y: -0.2 }, { x: 1, y: 0.4 }] }],
      source_table_id: "general_sem_interaction_plots",
      display: {},
    }],
    notices: [{ id: "notice:test", code: "bounded", severity: "information", message: "Bounded method.", section_ids: [], table_ids: [] }],
    exclusions: [{ id: "exclusion:test", title: "Deferred feature", reason: "Outside this cell." }],
    footnotes: [{ id: "effect_note", text: "Effects are associational." }],
    presentation: {
      default_section_id: "general_sem_moderation",
      default_table_id: "general_sem_interaction_effects",
      precision: 4,
      missing_value_label: "—",
      chart_defaults: {},
    },
  };
}

describe("canonical result navigation V1", () => {
  it("projects only applicable user-facing result groups in stable order", () => {
    const navigation = buildCanonicalResultNavigationV1(documentFixture());

    expect(navigation.groups.map((group) => group.title)).toEqual([
      "Overview",
      "Measurement Model",
      "Structural Model",
      "Direct, Indirect and Total Effects",
      "Moderation and Conditional Effects",
      "Higher-Order Constructs",
      "Moderated Mediation",
      "CB-SEM Parameters",
      "CB-SEM Fit and Identification",
      "Bootstrap Inference",
      "Diagnostics and Run Details",
    ]);
    expect(navigation.defaultItemId).toBe("canonical:table:general_sem_interaction_effects");
    expect(navigation.groups.flatMap((group) => group.items).map((item) => item.id)).toContain(
      "canonical:chart:general_sem_interaction_chart_0000",
    );
  });

  it("searches titles, descriptions, section context and column descriptions", () => {
    const navigation = buildCanonicalResultNavigationV1(documentFixture());

    expect(filterCanonicalResultNavigationV1(navigation, "conditional outcome").groups).toEqual([
      expect.objectContaining({
        id: "moderation",
        items: [expect.objectContaining({ id: "canonical:chart:general_sem_interaction_chart_0000" })],
      }),
    ]);
    expect(filterCanonicalResultNavigationV1(navigation, "CB-SEM Parameters").groups[0].items)
      .toEqual([expect.objectContaining({ id: "canonical:table:cbsem_general_sem_parameters" })]);
    expect(filterCanonicalResultNavigationV1(navigation, "not present").groups).toEqual([]);
  });

  it("projects a selected chart with its exact source table and keeps the full document for overview", () => {
    const document = documentFixture();
    const navigation = buildCanonicalResultNavigationV1(document);
    const chartItem = canonicalResultNavigationItemV1(
      navigation,
      "canonical:chart:general_sem_interaction_chart_0000",
    );
    const chartDocument = canonicalResultDocumentForItemV1(document, chartItem)!;

    expect(chartDocument.charts.map((chart) => chart.id)).toEqual(["general_sem_interaction_chart_0000"]);
    expect(chartDocument.tables.map((table) => table.id)).toEqual(["general_sem_interaction_plots"]);
    expect(chartDocument.sections[0].chart_ids).toEqual(["general_sem_interaction_chart_0000"]);
    expect(canonicalResultDocumentForItemV1(
      document,
      canonicalResultNavigationItemV1(navigation, "canonical:overview"),
    )).toBe(document);
  });
});
