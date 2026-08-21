import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import { defaultDiagramLayout } from "./diagramGraph";
import {
  arrangeModelPreservingLayoutV1,
  modelEditModeratingEffectIdentityV1,
  modelEditTransactionClassV1,
  observedVariableForModelEditColumnV1,
  reconcileModelEditDiagramLayoutV1,
  strictModelEditIntentPlanV1,
  tidyConstructsPreservingLayoutV1,
} from "./modelEditCommandGatewayV1";
import { convertLegacyBasicModelV4 } from "./semModelV4";
import { parseStandardSemModelV4AuthorityRecordV1 } from "./standardSemModelV4Authority";
import type { ConstructData, Dataset } from "../types";

const nodes: Array<Node<ConstructData>> = [
  { id: "x", type: "construct", position: { x: 460, y: 240 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
  { id: "y", type: "construct", position: { x: 40, y: 40 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] } },
];
const edges: Edge[] = [{ id: "x-y", source: "x", target: "y", type: "straight" }];
const dataset: Dataset = {
  id: "data",
  name: "Data",
  columns: ["x1", "x2", "x3", "y1", "y2", "z1"],
  rows: [],
  missing: 0,
  columnMetadata: [{
    name: "x3",
    label: "Third predictor item",
    column_type: "numeric",
    scale_type: "continuous",
    missing_markers: ["NA"],
    theoretical_min: null,
    theoretical_max: null,
    value_labels: {},
  }],
};

function authority() {
  return parseStandardSemModelV4AuthorityRecordV1({
    schema_version: 1,
    model_document_sha256: "a".repeat(64),
    model: convertLegacyBasicModelV4({
      id: "strict-model",
      name: "Strict model",
      constructs: [
        { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
        { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
        { id: "z", name: "Moderator", short_name: "Z", mode: "reflective", indicators: ["z1"] },
      ],
      paths: [{ source: "x", target: "y" }],
      controls: [],
      interactions: [],
      higher_order_constructs: [],
    }, "pls_composite"),
  });
}

describe("model edit command gateway v1 domain", () => {
  it("classifies scientific and presentation commands explicitly", () => {
    expect(modelEditTransactionClassV1({ kind: "rename_construct", constructId: "x", label: "Predictor 2" })).toBe("scientific");
    expect(modelEditTransactionClassV1({ kind: "invert_measurement_model", constructId: "x" })).toBe("scientific");
    expect(modelEditTransactionClassV1({ kind: "add_path", relationId: "relation:z-y", sourceId: "z", targetId: "y" })).toBe("scientific");
    expect(modelEditTransactionClassV1({ kind: "assign_indicators", constructId: "x", columns: ["x3"] })).toBe("scientific");
    expect(modelEditTransactionClassV1({ kind: "move_construct", constructId: "x", position: { x: 10, y: 20 } })).toBe("presentation");
    expect(modelEditTransactionClassV1({ kind: "tidy_constructs", constructIds: ["x", "y"] })).toBe("presentation");
    expect(modelEditTransactionClassV1({ kind: "set_construct_pinned", constructId: "x", pinned: true })).toBe("presentation");
    expect(modelEditTransactionClassV1({ kind: "arrange_model", direction: "horizontal" })).toBe("presentation");
  });

  it("derives exact strict intents without changing construct or observed identities", () => {
    const current = authority();
    const predictorId = current.model.variables.find((variable) => variable.label === "Predictor")?.id;
    if (!predictorId) throw new Error("Expected the resident Predictor identity.");
    expect(strictModelEditIntentPlanV1({ kind: "rename_construct", constructId: predictorId, label: "  New predictor  " }, current, dataset)).toEqual({
      status: "ready",
      intent: { kind: "rename_construct", variable_id: predictorId, label: "New predictor" },
      affected: { constructIds: [predictorId], indicatorIds: [], relationshipIds: [] },
    });

    const assignment = strictModelEditIntentPlanV1({ kind: "assign_indicators", constructId: predictorId, columns: ["x3"] }, current, dataset);
    expect(assignment).toMatchObject({
      status: "ready",
      intent: {
        kind: "assign_indicators",
        construct_id: predictorId,
        indicators: [{ id: "observed:x3", source_column: "x3", label: "Third predictor item" }],
      },
      affected: { constructIds: [predictorId], indicatorIds: ["observed:x3"] },
    });
    expect(observedVariableForModelEditColumnV1(current, dataset, "x1").id).toBe("observed:x1");
    expect(strictModelEditIntentPlanV1({ kind: "rename_construct", constructId: predictorId, label: "Predictor" }, current, dataset)).toMatchObject({ status: "blocked", code: "model_edit.no_change" });
  });

  it("plans construct, measurement, path, HOC, and moderation edits through resident strict intents", () => {
    const current = authority();
    const predictorId = current.model.variables.find((variable) => variable.label === "Predictor")?.id;
    const outcomeId = current.model.variables.find((variable) => variable.label === "Outcome")?.id;
    const moderatorId = current.model.variables.find((variable) => variable.label === "Moderator")?.id;
    if (!predictorId || !outcomeId || !moderatorId) throw new Error("Expected resident construct identities.");
    const focal = current.model.relations.find((relation) => relation.kind === "structural" && relation.source === predictorId && relation.target === outcomeId);
    if (focal?.kind !== "structural") throw new Error("Expected the focal path.");

    expect(strictModelEditIntentPlanV1({
      kind: "add_construct",
      constructId: "construct:w",
      label: "Second moderator",
      columns: ["x3"],
      position: { x: 320, y: 120 },
    }, current, dataset)).toMatchObject({
      status: "ready",
      intent: { kind: "add_construct", variable_id: "construct:w", representation: { kind: "composite", weighting: { kind: "mode_a" } } },
      affected: { constructIds: ["construct:w"], indicatorIds: ["observed:x3"] },
    });
    expect(strictModelEditIntentPlanV1({ kind: "invert_measurement_model", constructId: predictorId }, current, dataset)).toMatchObject({
      status: "ready",
      intent: { kind: "set_construct_representation", variable_id: predictorId, representation: { kind: "composite", weighting: { kind: "mode_b" } } },
    });
    expect(strictModelEditIntentPlanV1({ kind: "reverse_path", relationId: focal.id }, current, dataset)).toMatchObject({
      status: "ready",
      intent: { kind: "replace_relationship", relationship_id: focal.id, definition: { kind: "structural", source: outcomeId, target: predictorId } },
    });
    expect(strictModelEditIntentPlanV1({ kind: "remove_path", relationId: focal.id }, current, dataset)).toMatchObject({
      status: "ready",
      intent: { kind: "delete_relationship", relationship_id: focal.id },
    });

    expect(strictModelEditIntentPlanV1({
      kind: "create_higher_order",
      termId: "hoc:term",
      outputId: "hoc:output",
      draft: {
        name: "Higher order",
        shortName: "HOC",
        components: [predictorId, moderatorId],
        approach: "embedded_two_stage",
        measurementType: "reflective_reflective",
        initialPath: { direction: "hoc_to_construct", constructId: outcomeId, relationshipId: "relation:hoc-y" },
      },
    }, current, dataset)).toMatchObject({
      status: "ready",
      intent: { kind: "add_higher_order", term_id: "hoc:term", output_id: "hoc:output", initial_path: { relation_id: "relation:hoc-y" } },
    });

    const target = { kind: "focal_relation" as const, relationId: focal.id };
    const identity = modelEditModeratingEffectIdentityV1(target, [predictorId, moderatorId]);
    expect(strictModelEditIntentPlanV1({
      kind: "create_moderating_effect",
      effect: { label: "X × Z", operands: [predictorId, moderatorId], target, outcomeId },
    }, current, dataset)).toMatchObject({
      status: "ready",
      intent: { kind: "add_moderating_effect_v3", operands: [predictorId, moderatorId], target },
      affected: { constructIds: expect.arrayContaining([identity.outputId, predictorId, moderatorId, outcomeId]), relationshipIds: [focal.id] },
    });
  });

  it("preserves surviving manual metadata while reconciling a scientific graph edit", () => {
    const layout = defaultDiagramLayout(nodes, edges);
    layout.constructLayouts.x = { ...layout.constructLayouts.x, pinned: true, width: 188 };
    layout.indicatorLayouts.x.x1 = { side: "right", order: 0, pinned: true };
    layout.edgeLayouts["x-y"] = { routing: "orthogonal", bendPoints: [{ x: 280, y: 80 }], labelOffset: { x: 9, y: -7 }, pinned: true };
    layout.diagramViewport = { x: 20, y: 30, zoom: 1.25 };
    layout.moderationAnchorFractions = { "term:xw": 0.42 };
    layout.moderationConnectorBendPoints = { "connector:xw": [{ x: 12, y: 18 }] };
    layout.standardSemPresentation = { schemaVersion: 1, objects: [{ kind: "caption", id: "caption:1", text: "Theory", x: 22, y: 18 }] };

    const renamed = nodes.map((node) => node.id === "x" ? { ...node, data: { ...node.data, label: "Renamed" } } : node);
    const reconciled = reconcileModelEditDiagramLayoutV1(layout, renamed, edges);
    expect(reconciled.constructLayouts.x).toMatchObject({ pinned: true, width: 188 });
    expect(reconciled.indicatorLayouts.x.x1).toEqual({ side: "right", order: 0, pinned: true });
    expect(reconciled.edgeLayouts["x-y"]).toEqual(layout.edgeLayouts["x-y"]);
    expect(reconciled.diagramViewport).toEqual(layout.diagramViewport);
    expect(reconciled.moderationAnchorFractions).toEqual(layout.moderationAnchorFractions);
    expect(reconciled.moderationConnectorBendPoints).toEqual(layout.moderationConnectorBendPoints);
    expect(reconciled.standardSemPresentation).toEqual(layout.standardSemPresentation);
  });

  it("arranges only unpinned constructs and retains all manual overrides", () => {
    const layout = defaultDiagramLayout(nodes, edges);
    layout.constructLayouts.x = { ...layout.constructLayouts.x, pinned: true };
    layout.indicatorLayouts.x.x1 = { side: "bottom", order: 0, pinned: true };
    layout.edgeLayouts["x-y"] = { routing: "curved", labelOffset: { x: 12, y: -4 }, pinned: true };
    layout.moderationAnchorFractions = { "term:xw": 0.55 };

    const arranged = arrangeModelPreservingLayoutV1(nodes, edges, layout, "horizontal");
    expect(arranged.nodes.find((node) => node.id === "x")?.position).toEqual(nodes[0]!.position);
    expect(arranged.diagramLayout.constructLayouts.x.pinned).toBe(true);
    expect(arranged.diagramLayout.indicatorLayouts.x.x1).toEqual(layout.indicatorLayouts.x.x1);
    expect(arranged.diagramLayout.edgeLayouts["x-y"]).toEqual(layout.edgeLayouts["x-y"]);
    expect(arranged.diagramLayout.moderationAnchorFractions).toEqual(layout.moderationAnchorFractions);
    expect(arranged.movedConstructIds).not.toContain("x");
  });

  it("tidies only the requested local subgraph", () => {
    const third: Node<ConstructData> = { id: "z", type: "construct", position: { x: 990, y: 610 }, data: { label: "Unrelated", shortName: "Z", mode: "reflective", indicators: ["z1"] } };
    const localNodes = [...nodes, third];
    const layout = defaultDiagramLayout(localNodes, edges);
    layout.constructLayouts.x = { ...layout.constructLayouts.x, pinned: true };
    layout.edgeLayouts["x-y"] = { routing: "curved", labelOffset: { x: 8, y: -3 }, pinned: true };
    const tidied = tidyConstructsPreservingLayoutV1(localNodes, edges, layout, ["x", "y"]);
    expect(tidied.nodes.find((node) => node.id === "x")?.position).toEqual(nodes[0]!.position);
    expect(tidied.nodes.find((node) => node.id === "z")?.position).toEqual(third.position);
    expect(tidied.diagramLayout.edgeLayouts["x-y"]).toEqual(layout.edgeLayouts["x-y"]);
    expect(tidied.movedConstructIds).not.toContain("z");
  });
});
