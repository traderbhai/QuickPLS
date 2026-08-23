import { describe, expect, it } from "vitest";
import { convertLegacyBasicModelV4, scientificSemModelV4HashInput, type LegacyBasicModelV4Input, type SemVariableV4 } from "./semModelV4";
import { reduceStandardSemModelV4AuthorityV1, type StandardSemModelV4AuthorityRecordV1 } from "./standardSemModelV4Authority";
import {
  parseStandardSemModelV4DiagramLayoutV1,
  projectStandardSemModelV4DiagramV1,
} from "./standardSemModelV4DiagramProjection";

const legacy: LegacyBasicModelV4Input = {
  id: "standard-model",
  name: "Standard model",
  constructs: [
    { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
    { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
  ],
  paths: [{ source: "x", target: "y" }],
  controls: [],
  higher_order_constructs: [],
  interactions: [],
};

const authority = (): StandardSemModelV4AuthorityRecordV1 => ({
  schema_version: 1,
  model_document_sha256: "a".repeat(64),
  model: convertLegacyBasicModelV4(legacy, "cbsem_common_factor"),
});

const observed = (id: string, role: Extract<SemVariableV4, { kind: "observed" }>["role"] = "indicator"): Extract<SemVariableV4, { kind: "observed" }> => ({
  kind: "observed",
  id,
  label: id,
  source_column: id,
  scale: "continuous",
  role,
  categories: [],
  value_labels: {},
  missing_markers: [],
  transformation_lineage: [],
});

const nextAuthority = (model: StandardSemModelV4AuthorityRecordV1["model"], digit: string): StandardSemModelV4AuthorityRecordV1 => ({
  schema_version: 1,
  model_document_sha256: digit.repeat(64),
  model,
});

describe("StandardSemModelV4 diagram projection", () => {
  it("projects canonical constructs and paths without introducing scientific metadata", () => {
    const source = authority();
    const before = JSON.stringify(source);
    const projected = projectStandardSemModelV4DiagramV1(source);
    expect(JSON.stringify(source)).toBe(before);
    expect(projected.nodes.map((node) => node.id)).toEqual(["construct:x", "construct:y"]);
    expect(projected.nodes[0].data).toMatchObject({ label: "Predictor", mode: "reflective", indicators: ["x1", "x2"] });
    expect(projected.nodes[0].data).not.toHaveProperty("semModelV4");
    expect(projected.nodes[0].data.standardSemV4Authority).toMatchObject({ variableId: "construct:x", variableKind: "common_factor", readOnly: true });
    expect(projected.edges).toHaveLength(1);
    expect(projected.edges[0]).toMatchObject({ source: "construct:x", target: "construct:y" });
    expect(projected.diagramLayout.constructLayouts).toHaveProperty("construct:x");
  });

  it("keeps scientific and presentation-only covariance provenance distinct", () => {
    const scientific = reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "add_relationship",
      relationship_id: "covariance:x-y",
      definition: {
        kind: "covariance",
        left: { kind: "variable", id: "construct:x" },
        right: { kind: "disturbance_of", id: "construct:y" },
        label: "Scientific covariance",
      },
    });
    const withPresentation = reduceStandardSemModelV4AuthorityV1({ schema_version: 1, model_document_sha256: "b".repeat(64), model: scientific.model }, {
      kind: "add_relationship",
      relationship_id: "display:x-y",
      definition: { kind: "presentation_only_covariance", left: "construct:x", right: "construct:y", label: "Display covariance" },
    });
    const projected = projectStandardSemModelV4DiagramV1({ schema_version: 1, model_document_sha256: "c".repeat(64), model: withPresentation.model });
    expect(projected.edges.find((edge) => edge.id === "covariance:x-y")?.data).toMatchObject({ role: "covariance", presentationOnly: false });
    expect(projected.edges.find((edge) => edge.id === "display:x-y")?.data).toMatchObject({ role: "covariance", presentationOnly: true });
  });

  it("projects supported higher-order semantics from authority only", () => {
    const candidate = reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "add_higher_order",
      term_id: "higher-order:xy",
      output_id: "derived:xy",
      label: "Higher order XY",
      components: ["construct:x", "construct:y"],
      approach: "embedded_two_stage",
      measurement_type: "reflective_reflective",
    });
    const projected = projectStandardSemModelV4DiagramV1({ schema_version: 1, model_document_sha256: "b".repeat(64), model: candidate.model });
    expect(projected.nodes.find((node) => node.id === "derived:xy")?.data).toMatchObject({
      semantic: "higher_order",
      higherOrder: {
        id: "higher-order:xy",
        components: ["construct:x", "construct:y"],
        method: "two_stage",
        canonicalApproach: "embedded_two_stage",
        measurementType: "reflective_reflective",
      },
    });

    const approaches = ["repeated_indicators", "extended_repeated_indicators", "embedded_two_stage", "disjoint_two_stage", "hybrid"] as const;
    const measurementTypes = ["reflective_reflective", "reflective_formative", "formative_reflective", "formative_formative"] as const;
    for (const approach of approaches) for (const measurementType of measurementTypes) {
      const matrixCandidate = reduceStandardSemModelV4AuthorityV1(authority(), {
        kind: "add_higher_order",
        term_id: `higher-order:${approach}:${measurementType}`,
        output_id: `derived:${approach}:${measurementType}`,
        label: "Higher-order matrix case",
        components: ["construct:x", "construct:y"],
        approach,
        measurement_type: measurementType,
      });
      const matrixProjection = projectStandardSemModelV4DiagramV1(nextAuthority(matrixCandidate.model, "b"));
      expect(matrixProjection.nodes.find((node) => node.id === `derived:${approach}:${measurementType}`)?.data.higherOrder)
        .toMatchObject({ canonicalApproach: approach, measurementType });
    }
  });

  it("projects observed controls, cross-loadings, and exact residual covariance endpoints", () => {
    const crossed = reduceStandardSemModelV4AuthorityV1(authority(), { kind: "add_cross_loading", construct_id: "construct:y", observed_id: "observed:x1" });
    const withControl = reduceStandardSemModelV4AuthorityV1(nextAuthority(crossed.model, "b"), { kind: "add_observed_variable", variable: observed("observed:control", "control") });
    const withControlPath = reduceStandardSemModelV4AuthorityV1(nextAuthority(withControl.model, "c"), {
      kind: "add_relationship",
      relationship_id: "control:c-y",
      definition: { kind: "control", source: "observed:control", target: "construct:y", label: "Control C" },
    });
    const withResidualCovariance = reduceStandardSemModelV4AuthorityV1(nextAuthority(withControlPath.model, "d"), {
      kind: "add_relationship",
      relationship_id: "residual:x1-y1",
      definition: {
        kind: "covariance",
        left: { kind: "residual_of", id: "observed:x1" },
        right: { kind: "residual_of", id: "observed:y1" },
        label: "Residual covariance",
      },
    });
    const withObservedCovariance = reduceStandardSemModelV4AuthorityV1(nextAuthority(withResidualCovariance.model, "e"), {
      kind: "add_relationship",
      relationship_id: "observed-covariance:c-x",
      definition: {
        kind: "covariance",
        left: { kind: "variable", id: "observed:control" },
        right: { kind: "variable", id: "construct:x" },
        label: "Observed-latent covariance",
      },
    });
    const withMixedCovariance = reduceStandardSemModelV4AuthorityV1(nextAuthority(withObservedCovariance.model, "f"), {
      kind: "add_relationship",
      relationship_id: "mixed-covariance:x2-y",
      definition: {
        kind: "covariance",
        left: { kind: "residual_of", id: "observed:x2" },
        right: { kind: "disturbance_of", id: "construct:y" },
        label: "Residual-disturbance covariance",
      },
    });
    const projected = projectStandardSemModelV4DiagramV1(nextAuthority(withMixedCovariance.model, "1"));
    expect(projected.nodes.map((node) => node.id)).toEqual(expect.arrayContaining(["observed:control", "observed:x1", "observed:y1"]));
    expect(projected.nodes.find((node) => node.id === "construct:y")?.data.standardSemV4Authority?.measurementBindings)
      .toContainEqual(expect.objectContaining({ observedId: "observed:x1", relationKind: "measurement_effect" }));
    expect(projected.edges.find((edge) => edge.id === "control:c-y")?.data).toMatchObject({ role: "control" });
    expect(projected.edges.find((edge) => edge.id === "residual:x1-y1")).toMatchObject({ source: "observed:x1", target: "observed:y1" });
    expect(projected.edges.find((edge) => edge.id === "residual:x1-y1")?.data?.standardSemV4Authority).toMatchObject({
      relationKind: "covariance",
      leftEndpoint: { kind: "residual_of", id: "observed:x1" },
      rightEndpoint: { kind: "residual_of", id: "observed:y1" },
    });
    expect(projected.edges.find((edge) => edge.id === "observed-covariance:c-x")?.data?.standardSemV4Authority).toMatchObject({
      leftEndpoint: { kind: "variable", id: "construct:x" },
      rightEndpoint: { kind: "variable", id: "observed:control" },
    });
    expect(projected.edges.find((edge) => edge.id === "mixed-covariance:x2-y")?.data?.standardSemV4Authority).toMatchObject({
      leftEndpoint: { kind: "disturbance_of", id: "construct:y" },
      rightEndpoint: { kind: "residual_of", id: "observed:x2" },
    });
  });

  it("projects control styling from the canonical relation role for latent and observed sources", () => {
    const structural = authority().model.relations.find((relation) => relation.kind === "structural");
    if (!structural || structural.kind !== "structural") throw new Error("Expected a structural relationship.");
    const latentControl = reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "replace_relationship",
      relationship_id: structural.id,
      definition: { kind: "control", source: structural.source, target: structural.target, label: "Latent control" },
    });
    const latentEdge = projectStandardSemModelV4DiagramV1(nextAuthority(latentControl.model, "b"))
      .edges.find((edge) => edge.id === structural.id);
    expect(latentEdge?.data).toMatchObject({ role: "control", controlLabel: "Latent control" });

    const withObserved = reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "add_observed_variable",
      variable: observed("observed:control", "control"),
    });
    const observedStructural = reduceStandardSemModelV4AuthorityV1(nextAuthority(withObserved.model, "c"), {
      kind: "add_relationship",
      relationship_id: "structural:c-y",
      definition: { kind: "structural", source: "observed:control", target: "construct:y", label: "Observed predictor" },
    });
    const observedEdge = projectStandardSemModelV4DiagramV1(nextAuthority(observedStructural.model, "d"))
      .edges.find((edge) => edge.id === "structural:c-y");
    expect(observedEdge?.data?.role).toBeUndefined();
  });

  it("projects every legacy interaction construction without changing its historical canvas shape", () => {
    const withModerator = reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "add_construct",
      variable_id: "construct:z",
      label: "Moderator",
      representation: { kind: "composite", weighting: { kind: "mode_a" } },
      indicators: [observed("observed:z1")],
    });
    const focal = withModerator.model.relations.find((relation) => relation.kind === "structural" && relation.source === "construct:x" && relation.target === "construct:y");
    if (!focal) throw new Error("Expected focal path.");
    const cases = [
      {
        method: "product_indicator" as const,
        product_indicator: { centering: "mean_center" as const, standardization: "none" as const, pairing: "all_pairs" as const },
      },
      { method: "orthogonalizing" as const },
      { method: "two_stage" as const },
    ];
    for (const [index, item] of cases.entries()) {
      const candidate = reduceStandardSemModelV4AuthorityV1(nextAuthority(withModerator.model, String(index + 2)), {
        kind: "add_interaction",
        term_id: `interaction:${item.method}`,
        output_id: `derived:${item.method}`,
        label: `${item.method} interaction`,
        predictor: "construct:x",
        moderator: "construct:z",
        focal_relation: focal.id,
        outcome: "construct:y",
        method: item.method,
        ...(item.method === "product_indicator" ? { product_indicator: item.product_indicator } : {}),
      });
      const interaction = projectStandardSemModelV4DiagramV1(nextAuthority(candidate.model, String(index + 5)))
        .nodes.find((node) => node.id === `derived:${item.method}`)?.data.interaction;
      expect(interaction).toMatchObject({
        termId: `interaction:${item.method}`,
        predictor: "construct:x",
        moderator: "construct:z",
        outcome: "construct:y",
        method: "two_stage_product_score",
        canonicalMethod: item.method,
        productIndicator: item.method === "product_indicator" ? item.product_indicator : null,
      });
      expect(interaction).not.toHaveProperty("kind");
      expect(interaction).not.toHaveProperty("operands");
      expect(interaction).not.toHaveProperty("hierarchyPolicy");
    }

    const polynomial = reduceStandardSemModelV4AuthorityV1(nextAuthority(withModerator.model, "e"), {
      kind: "add_polynomial",
      term_id: "polynomial:x3",
      output_id: "derived:x3",
      label: "X cubed",
      source: "construct:x",
      degree: 3,
    });
    expect(projectStandardSemModelV4DiagramV1(nextAuthority(polynomial.model, "f")).nodes
      .find((node) => node.id === "derived:x3")?.data)
      .toMatchObject({ semantic: "polynomial", polynomial: { termId: "polynomial:x3", source: "construct:x", degree: 3 } });
  });

  it("projects two-way interaction_v2 metadata without a legacy-method downgrade", () => {
    const withModerator = reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "add_construct",
      variable_id: "construct:z",
      label: "Moderator",
      representation: { kind: "composite", weighting: { kind: "mode_a" } },
      indicators: [observed("observed:z1")],
    });
    const focal = withModerator.model.relations.find((relation) =>
      relation.kind === "structural"
      && relation.source === "construct:x"
      && relation.target === "construct:y");
    if (!focal) throw new Error("Expected focal path.");
    const seeded = reduceStandardSemModelV4AuthorityV1(nextAuthority(withModerator.model, "b"), {
      kind: "add_interaction",
      term_id: "interaction-v2:x-z",
      output_id: "derived:interaction-v2:x-z",
      label: "X by Z V2",
      predictor: "construct:x",
      moderator: "construct:z",
      focal_relation: focal.id,
      outcome: "construct:y",
      method: "product_indicator",
      product_indicator: { centering: "double_mean_center", standardization: "sample_standard_deviation", pairing: "all_pairs" },
    });
    const model = structuredClone(seeded.model);
    const index = model.derived_terms.findIndex((term) => term.id === "interaction-v2:x-z");
    const term = model.derived_terms[index];
    if (term?.kind !== "interaction") throw new Error("Expected interaction seed.");
    model.derived_terms[index] = {
      kind: "interaction_v2",
      id: term.id,
      output: term.output,
      operands: [term.predictor, term.moderator],
      focal_relation: term.focal_relation,
      method: term.method,
      hierarchy_policy: "strong",
      product_indicator: term.product_indicator,
    };

    const interaction = projectStandardSemModelV4DiagramV1(nextAuthority(model, "c"))
      .nodes.find((node) => node.id === term.output)?.data.interaction;
    expect(interaction).toEqual({
      kind: "interaction_v2",
      termId: term.id,
      operands: ["construct:x", "construct:z"],
      outcome: "construct:y",
      focalRelationId: term.focal_relation,
      canonicalMethod: "product_indicator",
      hierarchyPolicy: "strong",
      productIndicator: { centering: "double_mean_center", standardization: "sample_standard_deviation", pairing: "all_pairs" },
    });
    expect(interaction).not.toHaveProperty("method");
  });

  it("projects three-way interaction_v2 operand order and hierarchy losslessly", () => {
    const withModerator = reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "add_construct",
      variable_id: "construct:z",
      label: "First moderator",
      representation: { kind: "composite", weighting: { kind: "mode_a" } },
      indicators: [observed("observed:z1")],
    });
    const withSecondModerator = reduceStandardSemModelV4AuthorityV1(nextAuthority(withModerator.model, "b"), {
      kind: "add_construct",
      variable_id: "construct:w",
      label: "Second moderator",
      representation: { kind: "composite", weighting: { kind: "mode_a" } },
      indicators: [observed("observed:w1")],
    });
    const focal = withSecondModerator.model.relations.find((relation) =>
      relation.kind === "structural"
      && relation.source === "construct:x"
      && relation.target === "construct:y");
    if (!focal) throw new Error("Expected focal path.");
    const seeded = reduceStandardSemModelV4AuthorityV1(nextAuthority(withSecondModerator.model, "c"), {
      kind: "add_interaction",
      term_id: "interaction-v2:x-z-w",
      output_id: "derived:interaction-v2:x-z-w",
      label: "X by Z by W",
      predictor: "construct:x",
      moderator: "construct:z",
      focal_relation: focal.id,
      outcome: "construct:y",
      method: "two_stage",
    });
    const withSecondMainEffect = reduceStandardSemModelV4AuthorityV1(nextAuthority(seeded.model, "d"), {
      kind: "add_relationship",
      relationship_id: "structural:w-y",
      definition: { kind: "structural", source: "construct:w", target: "construct:y", label: "W to Y" },
    });
    const model = structuredClone(withSecondMainEffect.model);
    const index = model.derived_terms.findIndex((term) => term.id === "interaction-v2:x-z-w");
    const term = model.derived_terms[index];
    if (term?.kind !== "interaction") throw new Error("Expected interaction seed.");
    model.derived_terms[index] = {
      kind: "interaction_v2",
      id: term.id,
      output: term.output,
      operands: [term.predictor, term.moderator, "construct:w"],
      focal_relation: term.focal_relation,
      method: term.method,
      hierarchy_policy: "weak",
    };

    expect(projectStandardSemModelV4DiagramV1(nextAuthority(model, "e")).nodes
      .find((node) => node.id === term.output)?.data.interaction)
      .toEqual({
        kind: "interaction_v2",
        termId: term.id,
        operands: ["construct:x", "construct:z", "construct:w"],
        outcome: "construct:y",
        focalRelationId: term.focal_relation,
        canonicalMethod: "two_stage",
        hierarchyPolicy: "weak",
        productIndicator: null,
      });
  });

  it("strictly parses presentation-only layout and rejects identity or numeric drift", () => {
    const projected = projectStandardSemModelV4DiagramV1(authority());
    const relationshipId = Object.keys(projected.diagramLayout.edgeLayouts)[0]!;
    projected.diagramLayout.edgeLayouts[relationshipId] = {
      routing: "polyline",
      bendPoints: [{ x: 310, y: 74 }, { x: 410, y: 166 }],
      pinned: true,
    };
    projected.diagramLayout.measurementConnectorLayouts["construct:x"] = {
      x1: { routing: "polyline", bendPoints: [{ x: 170, y: 54 }] },
      x2: { routing: "curved" },
    };
    const parsed = parseStandardSemModelV4DiagramLayoutV1({
      schema_version: 1,
      model_id: "standard-model",
      diagram_layout: projected.diagramLayout,
    });
    expect(Object.isFrozen(parsed.diagram_layout)).toBe(true);
    expect(parsed.diagram_layout.edgeLayouts[relationshipId]).toEqual({
      routing: "polyline",
      bendPoints: [{ x: 310, y: 74 }, { x: 410, y: 166 }],
      labelOffset: undefined,
      pinned: true,
    });
    expect(parsed.diagram_layout.measurementConnectorLayouts).toEqual({
      "construct:x": {
        x1: { routing: "polyline", bendPoints: [{ x: 170, y: 54 }] },
        x2: { routing: "curved" },
      },
    });
    expect(projectStandardSemModelV4DiagramV1(authority(), parsed)).toEqual(projected);
    expect(() => parseStandardSemModelV4DiagramLayoutV1({ ...parsed, model_id: " standard-model " }))
      .toThrowError(expect.objectContaining({ code: "standard_sem_projection.stable_id_invalid" }));
    const invalid = structuredClone(parsed) as unknown as { diagram_layout: { constructLayouts: Record<string, { x: number }> } };
    invalid.diagram_layout.constructLayouts["construct:x"].x = Number.NaN;
    expect(() => parseStandardSemModelV4DiagramLayoutV1(invalid))
      .toThrowError(expect.objectContaining({ code: "standard_sem_projection.number_invalid" }));
  });

  it("accepts older strict layouts without measurement connector metadata", () => {
    const projected = projectStandardSemModelV4DiagramV1(authority());
    const legacyLayout = structuredClone(projected.diagramLayout) as unknown as Record<string, unknown>;
    delete legacyLayout.measurementConnectorLayouts;
    const parsed = parseStandardSemModelV4DiagramLayoutV1({
      schema_version: 1,
      model_id: authority().model.id,
      diagram_layout: legacyLayout,
    });
    expect(parsed.diagram_layout.measurementConnectorLayouts).toEqual({});
  });

  it("seeds captions, notes, shapes, images, and lines into layout without changing scientific authority", () => {
    const source = authority();
    const decorated = {
      ...source,
      model: {
        ...source.model,
        annotations: [
          ...source.model.annotations,
          { kind: "caption" as const, id: "caption:1", text: "Figure caption" },
          { kind: "note" as const, id: "note:1", subject: "Review", text: "Presentation note" },
        ],
        presentation: {
          kind: "canvas" as const,
          nodes: [],
          edges: [],
          shapes: [{ id: "shape:1", shape: "diamond" as const, x: 300, y: 40, width: 120, height: 80, label: "Decision", style: { border: "double" } }],
          images: [{ id: "image:1", asset_ref: "asset:logo", alt_text: "Study logo", x: 460, y: 40, width: 160, height: 90, style: {} }],
          lines: [{ id: "line:1", x1: 300, y1: 180, x2: 520, y2: 180, label: "Section", start_marker: null, end_marker: "arrow", style: { stroke: "dashed" } }],
        },
      },
    };
    const scientificBefore = scientificSemModelV4HashInput(decorated.model);
    const authorityBefore = JSON.stringify(decorated);
    const projected = projectStandardSemModelV4DiagramV1(decorated);
    expect(projected.diagramLayout.standardSemPresentation?.objects.map((object) => object.kind))
      .toEqual(["caption", "note", "shape", "image", "line"]);

    const edited = parseStandardSemModelV4DiagramLayoutV1({
      schema_version: 1,
      model_id: decorated.model.id,
      diagram_layout: {
        ...projected.diagramLayout,
        standardSemPresentation: {
          schemaVersion: 1,
          objects: projected.diagramLayout.standardSemPresentation!.objects.map((object) =>
            object.kind === "caption" ? { ...object, text: "Edited caption", x: object.x + 20 } : object),
        },
      },
    });
    expect(projectStandardSemModelV4DiagramV1(decorated, edited).diagramLayout.standardSemPresentation)
      .toEqual(edited.diagram_layout.standardSemPresentation);
    expect(JSON.stringify(decorated)).toBe(authorityBefore);
    expect(scientificSemModelV4HashInput(decorated.model)).toBe(scientificBefore);
  });

  it("keeps moderation hierarchy annotations internal while preserving authored presentation notes", () => {
    const source = authority();
    const decorated = {
      ...source,
      model: {
        ...source.model,
        annotations: [
          ...source.model.annotations,
          { kind: "caption" as const, id: "caption:authored", text: "Authored caption" },
          { kind: "note" as const, id: "note:authored", subject: "Method", text: "Authored note" },
          { kind: "note" as const, id: "general-sem:v1:interaction-generated:relation%3Alegacy", subject: "relation:legacy", text: "Internal legacy origin" },
          { kind: "note" as const, id: "general-sem:v1:interaction-dependency:term%3Alegacy:relation%3Alegacy", subject: "relation:legacy", text: "Internal legacy dependency" },
          { kind: "note" as const, id: "general-sem:v1:interaction-generated:_72656c6174696f6e", subject: "relation:canonical", text: "Internal canonical origin" },
          { kind: "note" as const, id: "general-sem:v1:interaction-dependency:_7465726d_72656c6174696f6e", subject: "relation:canonical", text: "Internal canonical dependency" },
        ],
      },
    };

    const projected = projectStandardSemModelV4DiagramV1(decorated);
    expect(projected.diagramLayout.standardSemPresentation?.objects).toEqual([
      { kind: "caption", id: "caption:authored", text: "Authored caption", x: 40, y: 40 },
      { kind: "note", id: "note:authored", subject: "Method", text: "Authored note", x: 40, y: 112 },
    ]);

    const reopenedLayout = parseStandardSemModelV4DiagramLayoutV1({
      schema_version: 1,
      model_id: decorated.model.id,
      diagram_layout: {
        ...projected.diagramLayout,
        standardSemPresentation: {
          schemaVersion: 1,
          objects: [
            ...projected.diagramLayout.standardSemPresentation!.objects,
            { kind: "note", id: "general-sem:v1:interaction-generated:relation%3Aleaked", subject: "relation:leaked", text: "Previously leaked note", x: 40, y: 184 },
            { kind: "note", id: "general-sem:v1:interaction-dependency:_7465726d_72656c6174696f6e", subject: "relation:leaked", text: "Previously leaked dependency", x: 40, y: 256 },
          ],
        },
      },
    });
    expect(projectStandardSemModelV4DiagramV1(decorated, reopenedLayout).diagramLayout.standardSemPresentation?.objects)
      .toEqual(projected.diagramLayout.standardSemPresentation?.objects);
  });

  it("rejects a layout belonging to another model", () => {
    const projected = projectStandardSemModelV4DiagramV1(authority());
    const wrong = parseStandardSemModelV4DiagramLayoutV1({ schema_version: 1, model_id: "other-model", diagram_layout: projected.diagramLayout });
    expect(() => projectStandardSemModelV4DiagramV1(authority(), wrong))
      .toThrowError(expect.objectContaining({ code: "standard_sem_projection.model_mismatch" }));
  });
});
