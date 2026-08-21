import { describe, expect, it } from "vitest";
import type { CanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import {
  authoredCanonicalResultPresentation,
  authoredResultTablePresentation,
  createAuthoredResultIdentityResolver,
  type AuthoredResultIdentityModel,
} from "./authoredResultIdentity";

function model(): AuthoredResultIdentityModel {
  return {
    nodes: [
      { id: "construct:sat", position: { x: 0, y: 0 }, data: { label: "Satisfaction", shortName: "SAT", mode: "reflective", indicators: ["SAT1"] } },
      { id: "construct:loy", position: { x: 200, y: 0 }, data: { label: "Loyalty", shortName: "LOY", mode: "reflective", indicators: ["LOY1"] } },
      { id: "construct:trust", position: { x: 100, y: 100 }, data: { label: "Trust", shortName: "TRU", mode: "reflective", indicators: ["TRU1"] } },
      { id: "construct:trust_alt", position: { x: 100, y: 180 }, data: { label: "Trust", shortName: "ALT", mode: "reflective", indicators: ["ALT1"] } },
      {
        id: "__qpls_interaction_sat_trust",
        position: { x: 100, y: 240 },
        data: {
          label: "Generated interaction",
          shortName: "INT",
          mode: "reflective",
          indicators: [],
          semantic: "interaction",
          interaction: {
            predictor: "construct:sat",
            moderator: "construct:trust",
            outcome: "construct:loy",
            method: "two_stage_product_score",
            termId: "term:sat_by_trust",
          },
        },
      },
      {
        id: "__qpls_hoc_standing",
        position: { x: 300, y: 180 },
        data: {
          label: "Corporate standing",
          shortName: "STAND",
          mode: "reflective",
          indicators: [],
          semantic: "higher_order",
          higherOrder: {
            id: "__qpls_hoc_standing",
            components: ["construct:sat", "construct:trust"],
            method: "two_stage",
          },
        },
      },
    ],
    edges: [{ id: "relation:sat_loy", source: "construct:sat", target: "construct:loy" }],
  } as AuthoredResultIdentityModel;
}

function canonicalDocument(): CanonicalResultDocumentV2 {
  return {
    schema_version: 2,
    document_id: "result:identity",
    title: "Identity projection",
    provenance: {
      run_id: "run:identity",
      project_id: "project:identity",
      model_id: "model:identity",
      model_digest: "a".repeat(64),
      dataset_id: "dataset:identity",
      dataset_fingerprint: "b".repeat(64),
      recipe_id: "recipe:identity",
      recipe_digest: "c".repeat(64),
      capability_cell: { registry_schema_version: 2, capability_id: "smartpls.moderation", cell_id: "point", capability_version: "v1" },
      method_version: "method:v1",
      engine_version: "engine:v1",
      seed: 17,
      workers: 1,
      started_at: "2026-08-21T00:00:00Z",
      completed_at: "2026-08-21T00:00:01Z",
    },
    sections: [{ id: "effects", title: "Effects", table_ids: ["effects"], chart_ids: ["hoc_chart"] }],
    tables: [{
      id: "effects",
      title: "Effects",
      columns: [
        { id: "relation_id", label: "Relation ID", data_type: "text", description: "Saved relation." },
        { id: "interaction_id", label: "Interaction ID", data_type: "text", description: "Saved interaction." },
        { id: "estimate", label: "Estimate", data_type: "number", description: "Estimate.", role: "estimate" },
      ],
      rows: [{ id: "row:stable", cells: [
        { kind: "text", value: "relation:sat_loy" },
        { kind: "text", value: "term:sat_by_trust" },
        { kind: "number", value: 0.42 },
      ] }],
      footnote_ids: [],
    }, {
      id: "run_details",
      title: "Run details",
      columns: [{ id: "parameter_id", label: "Parameter ID", data_type: "text", description: "Raw parameter." }],
      rows: [{ id: "raw", cells: [{ kind: "text", value: "__qpls_interaction_sat_trust" }] }],
      footnote_ids: [],
    }],
    charts: [{
      id: "hoc_chart",
      title: "Higher-order paths",
      description: "Derived from canonical source table effects.",
      kind: "bar",
      source_table_id: "effects",
      series: [{ id: "series", label: "Estimate", points: [{ x: 0, y: 0.42, label: "hoc_path:construct:sat_to_construct:loy" }] }],
      display: { x_axis_label: "Target index", y_axis_label: "Estimate" },
    }],
    notices: [],
    exclusions: [],
    footnotes: [],
    presentation: { default_section_id: "effects", default_table_id: "effects", precision: 4, missing_value_label: "—", chart_defaults: {} },
  };
}

describe("authored result identity resolver", () => {
  it("resolves constructs, indicators, relations, mediation, moderation and higher-order identities", () => {
    const identity = createAuthoredResultIdentityResolver(model());

    expect(identity.construct("construct:sat")).toBe("Satisfaction");
    expect(identity.construct("construct:trust")).toBe("Trust (TRU)");
    expect(identity.construct("construct:trust_alt")).toBe("Trust (ALT)");
    expect(identity.indicator("__qpls_hoc___qpls_hoc_standing_construct:sat")).toBe("Satisfaction score for Corporate standing");
    expect(identity.relation("relation:sat_loy")).toBe("Satisfaction → Loyalty");
    expect(identity.mediation(["construct:sat", "construct:trust", "construct:loy"])).toBe("Satisfaction → Trust (TRU) → Loyalty");
    expect(identity.interaction("term:sat_by_trust")).toBe("Satisfaction × Trust (TRU) → Loyalty");
    expect(identity.higherOrder("__qpls_hoc_standing", "construct:sat")).toBe("Satisfaction component of Corporate standing");
    expect(identity.text("construct:sat = -1; construct:loy = 1")).toBe("Satisfaction = −1 SD; Loyalty = +1 SD");
    expect(identity.text("Simple slopes by construct:trust for construct:loy")).toBe("Simple slopes by Trust (TRU) for Loyalty");
  });

  it("uses readable historical fallbacks without exposing generated identifiers", () => {
    const identity = createAuthoredResultIdentityResolver();

    expect(identity.construct("__qpls_hoc_6cabc123")).toBe("Higher-order construct");
    expect(identity.interaction("__qpls_interaction_6cabc123")).toBe("Moderating effect");
    expect(identity.canonicalTarget("sha256:" + "f".repeat(64))).toBe("Saved result target");
  });

  it("projects visible canonical cells and charts while preserving stable IDs and raw Run Details", () => {
    const original = canonicalDocument();
    const before = structuredClone(original);
    const projected = authoredCanonicalResultPresentation(original, createAuthoredResultIdentityResolver(model()));

    expect(original).toEqual(before);
    expect(projected.document_id).toBe(original.document_id);
    expect(projected.tables[0].rows[0].id).toBe("row:stable");
    expect(projected.tables[0].columns.map((column) => column.label)).toEqual(["Relationship", "Moderating effect", "Estimate"]);
    expect(projected.tables[0].rows[0].cells).toEqual([
      { kind: "text", value: "Satisfaction → Loyalty" },
      { kind: "text", value: "Satisfaction × Trust (TRU) → Loyalty" },
      { kind: "number", value: 0.42 },
    ]);
    expect(projected.tables[1]).toEqual(original.tables[1]);
    expect(projected.charts[0].series[0].points[0].label).toBe("Satisfaction → Loyalty");
    expect(projected.charts[0].display.x_axis_label).toBe("Higher-order relationship");
    expect(projected.provenance).toEqual(original.provenance);
  });

  it("projects legacy process identities but leaves diagnostic tables unchanged", () => {
    const identity = createAuthoredResultIdentityResolver();
    const source = {
      id: "process_simple_slopes",
      title: "Simple slopes",
      status: "validated" as const,
      warning: null,
      columns: ["Effect ID", "Moderation", "Solved moderator", "Estimate"],
      rows: [["slope:moderation:sat->loy@trust", "moderation:sat->loy@trust", "trust", "0.42"]],
    };

    expect(authoredResultTablePresentation(source, identity)).toMatchObject({
      columns: ["Effect", "Moderation", "Solved moderator", "Estimate"],
      rows: [["Conditional effect", "Sat × Trust → Loy", "Trust", "0.42"]],
    });
    const details = { ...source, id: "process_run_details", title: "Run details" };
    expect(authoredResultTablePresentation(details, identity)).toBe(details);
  });
});
