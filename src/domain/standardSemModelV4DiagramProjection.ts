import { MarkerType, type Edge, type Node } from "@xyflow/react";
import type {
  ConstructData,
  DiagramLayoutState,
  StandardSemPresentationLayoutObject,
  StandardSemPresentationLayoutV1,
} from "../types";
import { defaultDiagramLayout } from "./diagramGraph";
import {
  parseStandardSemModelV4AuthorityRecordV1,
  type StandardSemModelV4AuthorityRecordV1,
} from "./standardSemModelV4Authority";
import type { SemEndpointV4, SemModelV4, SemRelationV4, SemVariableV4 } from "./semModelV4";

export const STANDARD_SEM_MODEL_V4_DIAGRAM_LAYOUT_VERSION = 1 as const;

export interface StandardSemModelV4DiagramLayoutV1 {
  readonly schema_version: typeof STANDARD_SEM_MODEL_V4_DIAGRAM_LAYOUT_VERSION;
  readonly model_id: string;
  readonly diagram_layout: DiagramLayoutState;
}

export interface StandardSemModelV4DiagramProjectionV1 {
  nodes: Array<Node<ConstructData>>;
  edges: Edge[];
  diagramLayout: DiagramLayoutState;
}

export class StandardSemModelV4DiagramProjectionError extends Error {
  constructor(
    public readonly code: string,
    public readonly subject: string,
    message: string,
  ) {
    super(message);
    this.name = "StandardSemModelV4DiagramProjectionError";
  }
}

const fail = (code: string, subject: string, message: string): never => {
  throw new StandardSemModelV4DiagramProjectionError(code, subject, message);
};

function object(value: unknown, subject: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return fail("standard_sem_projection.object_required", subject, `${subject} must be an object.`);
  }
  return value as Record<string, unknown>;
}

function exact(value: unknown, allowed: readonly string[], required: readonly string[], subject: string) {
  const parsed = object(value, subject);
  const unknown = Object.keys(parsed).find((key) => !allowed.includes(key));
  const missing = required.find((key) => !Object.prototype.hasOwnProperty.call(parsed, key));
  if (unknown || missing) {
    return fail(
      unknown ? "standard_sem_projection.field_unknown" : "standard_sem_projection.field_missing",
      `${subject}.${unknown ?? missing}`,
      unknown ? `${subject}.${unknown} is not supported.` : `${subject}.${missing} is required.`,
    );
  }
  return parsed;
}

const finite = (value: unknown, subject: string): number => {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return fail("standard_sem_projection.number_invalid", subject, `${subject} must be finite.`);
  }
  return value;
};

const optionalFinite = (value: unknown, subject: string) => value === undefined ? undefined : finite(value, subject);
const optionalBoolean = (value: unknown, subject: string) => {
  if (value === undefined) return undefined;
  if (typeof value !== "boolean") return fail("standard_sem_projection.boolean_invalid", subject, `${subject} must be boolean.`);
  return value;
};

const text = (value: unknown, subject: string, nonempty = false) => {
  if (typeof value !== "string" || nonempty && !value.trim()) {
    return fail("standard_sem_projection.text_invalid", subject, `${subject} must be ${nonempty ? "nonempty " : ""}text.`);
  }
  return value;
};

const nullableText = (value: unknown, subject: string) => value === null ? null : text(value, subject);

function textMap(value: unknown, subject: string) {
  return Object.fromEntries(Object.entries(object(value, subject)).map(([key, item]) => [key, text(item, `${subject}.${key}`)]));
}

function exactStableId(value: unknown, subject: string) {
  if (typeof value !== "string" || !value || value !== value.trim()) {
    return fail("standard_sem_projection.stable_id_invalid", subject, `${subject} must be an exact nonempty stable ID.`);
  }
  return value;
}

export function parseStandardSemModelV4DiagramLayoutV1(input: unknown): StandardSemModelV4DiagramLayoutV1 {
  const outer = exact(input, ["schema_version", "model_id", "diagram_layout"], ["schema_version", "model_id", "diagram_layout"], "layout");
  if (outer.schema_version !== STANDARD_SEM_MODEL_V4_DIAGRAM_LAYOUT_VERSION) {
    fail("standard_sem_projection.version_unsupported", "layout.schema_version", "The Standard diagram layout version is unsupported.");
  }
  const modelId = exactStableId(outer.model_id, "layout.model_id");
  const layout = parseDiagramLayout(outer.diagram_layout);
  return deepFreeze({ schema_version: STANDARD_SEM_MODEL_V4_DIAGRAM_LAYOUT_VERSION, model_id: modelId, diagram_layout: layout });
}

export function projectStandardSemModelV4DiagramV1(
  authorityInput: StandardSemModelV4AuthorityRecordV1,
  layoutInput?: StandardSemModelV4DiagramLayoutV1,
): StandardSemModelV4DiagramProjectionV1 {
  const authority = parseStandardSemModelV4AuthorityRecordV1(authorityInput);
  const supplied = layoutInput === undefined ? undefined : parseStandardSemModelV4DiagramLayoutV1(layoutInput);
  if (supplied && supplied.model_id !== authority.model.id) {
    fail("standard_sem_projection.model_mismatch", supplied.model_id, "The layout belongs to a different canonical model.");
  }

  const model = authority.model;
  const termByOutput = new Map(model.derived_terms.map((term) => [term.output, term]));
  const presentationNodes = model.presentation.kind === "canvas"
    ? new Map(model.presentation.nodes.map((node) => [node.variable, node]))
    : new Map<string, never>();
  const projectedIds = projectedVariableIds(model);
  const projectedVariables = model.variables.filter((variable) => projectedIds.has(variable.id));
  const nodes: Array<Node<ConstructData>> = projectedVariables.map((variable, index) => {
    const data = constructData(model, variable, termByOutput);
    const stored = supplied?.diagram_layout.constructLayouts[variable.id];
    const presented = presentationNodes.get(variable.id);
    return {
      id: variable.id,
      type: "construct",
      position: stored
        ? { x: stored.x, y: stored.y }
        : presented
          ? { x: presented.x, y: presented.y }
          : { x: 120 + (index % 4) * 260, y: 120 + Math.floor(index / 4) * 200 },
      data,
    };
  });

  const parameterLabels = new Map(model.parameters.map((parameter) => [parameter.id, parameter.label]));
  const edges: Edge[] = [];
  for (const relation of model.relations) {
    if (relation.kind === "measurement_effect" || relation.kind === "measurement_causal") continue;
    if (relation.kind === "structural") {
      requireProjectedVariable(relation.source, projectedIds, relation.id);
      requireProjectedVariable(relation.target, projectedIds, relation.id);
      const control = relation.role === "control";
      edges.push({
        id: relation.id,
        source: relation.source,
        target: relation.target,
        type: "smoothstep",
        markerEnd: { type: MarkerType.ArrowClosed },
        label: parameterLabels.get(relation.parameter) ?? "Path",
        data: {
          ...(control ? { role: "control", controlLabel: parameterLabels.get(relation.parameter) ?? "Control" } : {}),
          semAuthorityObjectId: relation.id,
          standardSemV4Authority: {
            authorityObjectId: relation.id,
            relationKind: "structural",
            parameterId: relation.parameter,
            presentationOnly: false,
            readOnly: true,
          },
        },
      });
      continue;
    }
    const left = projectEndpoint(relation.left, projectedIds, relation.id);
    const right = projectEndpoint(relation.right, projectedIds, relation.id);
    edges.push(covarianceEdge(relation.id, left, right, parameterLabels.get(relation.parameter) ?? "Covariance", false, relation.parameter, relation.left, relation.right));
  }
  for (const annotation of model.annotations) {
    if (annotation.kind !== "display_only_covariance") continue;
    requireProjectedVariable(annotation.left, projectedIds, annotation.id);
    requireProjectedVariable(annotation.right, projectedIds, annotation.id);
    edges.push(covarianceEdge(
      annotation.id,
      annotation.left,
      annotation.right,
      annotation.label ?? "Covariance",
      true,
      null,
      { kind: "variable", id: annotation.left },
      { kind: "variable", id: annotation.right },
    ));
  }

  const seed = supplied?.diagram_layout ?? presentationLayoutSeed(model);
  const decorations = supplied?.diagram_layout.standardSemPresentation ?? standardSemPresentationSeed(model);
  const diagramLayout = parseDiagramLayout({
    ...defaultDiagramLayout(nodes, edges, seed),
    ...(decorations ? { standardSemPresentation: decorations } : {}),
  });
  return { nodes, edges, diagramLayout };
}

function constructData(
  model: SemModelV4,
  variable: SemVariableV4,
  termByOutput: ReadonlyMap<string, SemModelV4["derived_terms"][number]>,
): ConstructData {
  const measurementRelations = model.relations.filter((relation): relation is Extract<SemRelationV4, { kind: "measurement_effect" | "measurement_causal" }> =>
      relation.kind === "measurement_effect" && relation.construct === variable.id
      || relation.kind === "measurement_causal" && relation.composite === variable.id);
  const measurementBindings = measurementRelations.map((relation) => {
    const observed = model.variables.find((candidate): candidate is Extract<SemVariableV4, { kind: "observed" }> => candidate.kind === "observed" && candidate.id === relation.indicator);
    if (!observed) return fail("standard_sem_projection.measurement_indicator_missing", relation.id, `Measurement relation ${relation.id} has no observed variable.`);
    return {
      relationId: relation.id,
      parameterId: relation.parameter,
      observedId: observed.id,
      sourceColumn: observed.source_column,
      relationKind: relation.kind,
    };
  });
  const base: ConstructData = {
    label: variable.label,
    shortName: shortName(variable.label, variable.id),
    mode: variable.kind === "composite" && variable.weighting.kind !== "mode_a" ? "formative" : "reflective",
    indicators: measurementBindings.map((binding) => binding.sourceColumn),
    standardSemV4Authority: {
      variableId: variable.id,
      variableKind: variable.kind,
      readOnly: true,
      ...(variable.kind === "observed" ? { observedRole: variable.role } : {}),
      measurementBindings,
    },
  };
  if (variable.kind === "observed") return { ...base, semantic: "observed" };
  if (variable.kind !== "derived") return base;
  const term = termByOutput.get(variable.id);
  if (!term) return fail("standard_sem_projection.derived_definition_missing", variable.id, "A derived construct has no canonical definition.");
  if (term.kind === "polynomial") return { ...base, semantic: "polynomial", polynomial: { termId: term.id, source: term.source, degree: term.degree } };
  if (term.kind === "interaction") {
    const focal = model.relations.find((relation): relation is Extract<SemRelationV4, { kind: "structural" }> => relation.kind === "structural" && relation.id === term.focal_relation);
    if (!focal) return fail("standard_sem_projection.focal_relation_missing", term.id, "The interaction focal path is unavailable.");
    return {
      ...base,
      semantic: "interaction",
      interaction: {
        termId: term.id,
        predictor: term.predictor,
        moderator: term.moderator,
        outcome: focal.target,
        focalRelationId: term.focal_relation,
        method: "two_stage_product_score",
        canonicalMethod: term.method,
        productIndicator: term.product_indicator ? structuredClone(term.product_indicator) : null,
      },
    };
  }
  if (term.kind === "interaction_v2") {
    if (term.operands.length < 2) {
      return fail("standard_sem_projection.interaction_v2_operands_invalid", term.id, "The interaction_v2 term needs a focal predictor and at least one moderator.");
    }
    const focal = model.relations.find((relation): relation is Extract<SemRelationV4, { kind: "structural" }> =>
      relation.kind === "structural"
      && relation.role !== "control"
      && relation.id === term.focal_relation
      && relation.source === term.operands[0]);
    if (!focal) return fail("standard_sem_projection.focal_relation_missing", term.id, "The interaction_v2 focal path is unavailable.");
    return {
      ...base,
      semantic: "interaction",
      interaction: {
        kind: "interaction_v2",
        termId: term.id,
        operands: [term.operands[0]!, term.operands[1]!, ...term.operands.slice(2)],
        outcome: focal.target,
        focalRelationId: term.focal_relation,
        canonicalMethod: term.method,
        hierarchyPolicy: term.hierarchy_policy,
        productIndicator: term.product_indicator ? structuredClone(term.product_indicator) : null,
      },
    };
  }
  const method = term.approach === "repeated_indicators" || term.approach === "extended_repeated_indicators"
    ? "repeated_indicators"
    : term.approach === "hybrid" ? "hybrid" : "two_stage";
  return {
    ...base,
    mode: term.measurement_type.endsWith("_reflective") ? "reflective" : "formative",
    semantic: "higher_order",
    higherOrder: {
      id: term.id,
      components: [...term.components],
      method,
      canonicalApproach: term.approach,
      measurementType: term.measurement_type,
    },
  };
}

function projectEndpoint(endpoint: SemEndpointV4, projectedIds: ReadonlySet<string>, subject: string) {
  requireProjectedVariable(endpoint.id, projectedIds, subject);
  return endpoint.id;
}

function requireProjectedVariable(id: string, projectedIds: ReadonlySet<string>, subject: string) {
  if (!projectedIds.has(id)) fail("standard_sem_projection.endpoint_missing", subject, `Endpoint ${id} is not present in the deterministic projection.`);
}

function covarianceEdge(
  id: string,
  source: string,
  target: string,
  label: string,
  presentationOnly: boolean,
  parameterId: string | null,
  leftEndpoint: SemEndpointV4,
  rightEndpoint: SemEndpointV4,
): Edge {
  return {
    id,
    source,
    target,
    type: "default",
    label,
    data: {
      role: "covariance",
      semAuthorityObjectId: id,
      presentationOnly,
      standardSemV4Authority: {
        authorityObjectId: id,
        relationKind: presentationOnly ? "display_only_covariance" : "covariance",
        parameterId,
        leftEndpoint: structuredClone(leftEndpoint),
        rightEndpoint: structuredClone(rightEndpoint),
        presentationOnly,
        readOnly: true,
      },
    },
  };
}

function projectedVariableIds(model: SemModelV4) {
  const ids = new Set(model.variables.filter((variable) => variable.kind !== "observed" || variable.role !== "indicator").map((variable) => variable.id));
  for (const relation of model.relations) {
    if (relation.kind === "structural") {
      ids.add(relation.source);
      ids.add(relation.target);
    } else if (relation.kind === "covariance") {
      ids.add(relation.left.id);
      ids.add(relation.right.id);
    }
  }
  for (const annotation of model.annotations) if (annotation.kind === "display_only_covariance") {
    ids.add(annotation.left);
    ids.add(annotation.right);
  }
  if (model.presentation.kind === "canvas") for (const node of model.presentation.nodes) ids.add(node.variable);
  return ids;
}

function shortName(label: string, id: string) {
  const normalized = label.normalize("NFKD").replace(/[^A-Za-z0-9]+/g, "").toUpperCase();
  return (normalized || id.replace(/[^A-Za-z0-9]+/g, "").toUpperCase() || "SEM").slice(0, 8);
}

function presentationLayoutSeed(model: SemModelV4): Partial<DiagramLayoutState> | undefined {
  if (model.presentation.kind !== "canvas") return undefined;
  const constructLayouts: DiagramLayoutState["constructLayouts"] = {};
  for (const node of model.presentation.nodes) constructLayouts[node.variable] = { x: node.x, y: node.y };
  const edgeLayouts: DiagramLayoutState["edgeLayouts"] = {};
  for (const edge of model.presentation.edges) {
    const routing = edge.routing === "straight" || edge.routing === "curved" || edge.routing === "orthogonal" ? edge.routing : undefined;
    if (routing) edgeLayouts[edge.relation] = { routing };
  }
  return {
    diagramVersion: "sem_designer_v1",
    constructLayouts,
    indicatorLayouts: {},
    edgeLayouts,
    diagramViewport: model.presentation.zoom === undefined || model.presentation.zoom === null
      ? undefined
      : { x: model.presentation.pan_x ?? 0, y: model.presentation.pan_y ?? 0, zoom: model.presentation.zoom },
    diagramTheme: "smartpls_like",
    showGrid: true,
    layoutLocked: false,
  };
}

function standardSemPresentationSeed(model: SemModelV4): StandardSemPresentationLayoutV1 | undefined {
  const objects: StandardSemPresentationLayoutObject[] = [];
  let annotationIndex = 0;
  for (const annotation of model.annotations) {
    if (annotation.kind === "caption") {
      objects.push({ kind: "caption", id: annotation.id, text: annotation.text, x: 40, y: 40 + annotationIndex++ * 72 });
    } else if (annotation.kind === "note") {
      objects.push({ kind: "note", id: annotation.id, subject: annotation.subject, text: annotation.text, x: 40, y: 40 + annotationIndex++ * 72 });
    }
  }
  if (model.presentation.kind === "canvas") {
    objects.push(
      ...model.presentation.shapes.map((shape) => ({
        kind: "shape" as const,
        id: shape.id,
        shape: shape.shape,
        x: shape.x,
        y: shape.y,
        width: shape.width,
        height: shape.height,
        label: shape.label ?? null,
        style: { ...(shape.style ?? {}) },
      })),
      ...model.presentation.images.map((image) => ({
        kind: "image" as const,
        id: image.id,
        assetRef: image.asset_ref,
        altText: image.alt_text,
        x: image.x,
        y: image.y,
        width: image.width,
        height: image.height,
        style: { ...(image.style ?? {}) },
      })),
      ...model.presentation.lines.map((line) => ({
        kind: "line" as const,
        id: line.id,
        x1: line.x1,
        y1: line.y1,
        x2: line.x2,
        y2: line.y2,
        label: line.label ?? null,
        startMarker: line.start_marker ?? null,
        endMarker: line.end_marker ?? null,
        style: { ...(line.style ?? {}) },
      })),
    );
  }
  return objects.length ? { schemaVersion: 1, objects } : undefined;
}

function parseDiagramLayout(value: unknown): DiagramLayoutState {
  const layout = exact(
    value,
    ["diagramVersion", "constructLayouts", "indicatorLayouts", "edgeLayouts", "diagramViewport", "diagramTheme", "showGrid", "layoutLocked", "standardSemPresentation"],
    ["diagramVersion", "constructLayouts", "indicatorLayouts", "edgeLayouts", "diagramTheme", "showGrid", "layoutLocked"],
    "layout.diagram_layout",
  );
  if (layout.diagramVersion !== "sem_designer_v1") fail("standard_sem_projection.diagram_version_invalid", "layout.diagram_layout.diagramVersion", "The diagram version is unsupported.");
  const constructLayouts: DiagramLayoutState["constructLayouts"] = {};
  for (const [id, raw] of Object.entries(object(layout.constructLayouts, "layout.diagram_layout.constructLayouts"))) {
    exactStableId(id, `layout.diagram_layout.constructLayouts.${id}`);
    const item = exact(raw, ["x", "y", "width", "height", "pinned"], ["x", "y"], `layout.diagram_layout.constructLayouts.${id}`);
    constructLayouts[id] = { x: finite(item.x, `${id}.x`), y: finite(item.y, `${id}.y`), width: optionalFinite(item.width, `${id}.width`), height: optionalFinite(item.height, `${id}.height`), pinned: optionalBoolean(item.pinned, `${id}.pinned`) };
  }
  const indicatorLayouts: DiagramLayoutState["indicatorLayouts"] = {};
  for (const [constructId, rawIndicators] of Object.entries(object(layout.indicatorLayouts, "layout.diagram_layout.indicatorLayouts"))) {
    exactStableId(constructId, `layout.diagram_layout.indicatorLayouts.${constructId}`);
    indicatorLayouts[constructId] = {};
    for (const [indicatorId, raw] of Object.entries(object(rawIndicators, `layout.diagram_layout.indicatorLayouts.${constructId}`))) {
      const item = exact(raw, ["side", "x", "y", "order", "pinned"], ["side", "order"], `layout.diagram_layout.indicatorLayouts.${constructId}.${indicatorId}`);
      if (!["left", "right", "top", "bottom", "free"].includes(String(item.side))) fail("standard_sem_projection.indicator_side_invalid", indicatorId, "The indicator side is invalid.");
      const order = finite(item.order, `${indicatorId}.order`);
      if (!Number.isInteger(order) || order < 0) fail("standard_sem_projection.indicator_order_invalid", indicatorId, "Indicator order must be a nonnegative integer.");
      indicatorLayouts[constructId][indicatorId] = { side: item.side as DiagramLayoutState["indicatorLayouts"][string][string]["side"], x: optionalFinite(item.x, `${indicatorId}.x`), y: optionalFinite(item.y, `${indicatorId}.y`), order, pinned: optionalBoolean(item.pinned, `${indicatorId}.pinned`) ?? false };
    }
  }
  const edgeLayouts: DiagramLayoutState["edgeLayouts"] = {};
  for (const [id, raw] of Object.entries(object(layout.edgeLayouts, "layout.diagram_layout.edgeLayouts"))) {
    const item = exact(raw, ["routing", "bendPoints", "labelOffset", "pinned"], ["routing"], `layout.diagram_layout.edgeLayouts.${id}`);
    if (!["straight", "curved", "orthogonal"].includes(String(item.routing))) fail("standard_sem_projection.routing_invalid", id, "The edge routing is invalid.");
    const points = item.bendPoints === undefined ? undefined : parsePoints(item.bendPoints, `${id}.bendPoints`);
    const labelOffset = item.labelOffset === undefined ? undefined : parsePoint(item.labelOffset, `${id}.labelOffset`);
    edgeLayouts[id] = { routing: item.routing as DiagramLayoutState["edgeLayouts"][string]["routing"], bendPoints: points, labelOffset, pinned: optionalBoolean(item.pinned, `${id}.pinned`) };
  }
  const viewport = layout.diagramViewport === undefined ? undefined : exact(layout.diagramViewport, ["x", "y", "zoom"], ["x", "y", "zoom"], "layout.diagram_layout.diagramViewport");
  const themes = ["academic_grayscale", "smartpls_like", "quickpls_color", "journal_mono", "high_contrast"] as const;
  if (!themes.includes(layout.diagramTheme as typeof themes[number])) fail("standard_sem_projection.theme_invalid", "layout.diagram_layout.diagramTheme", "The diagram theme is invalid.");
  if (typeof layout.showGrid !== "boolean" || typeof layout.layoutLocked !== "boolean") fail("standard_sem_projection.boolean_invalid", "layout.diagram_layout", "Diagram flags must be boolean.");
  const showGrid = layout.showGrid as boolean;
  const layoutLocked = layout.layoutLocked as boolean;
  return {
    diagramVersion: "sem_designer_v1",
    constructLayouts,
    indicatorLayouts,
    edgeLayouts,
    diagramViewport: viewport ? { x: finite(viewport.x, "viewport.x"), y: finite(viewport.y, "viewport.y"), zoom: finite(viewport.zoom, "viewport.zoom") } : undefined,
    diagramTheme: layout.diagramTheme as typeof themes[number],
    showGrid,
    layoutLocked,
    ...(layout.standardSemPresentation === undefined
      ? {}
      : { standardSemPresentation: parseStandardSemPresentationLayout(layout.standardSemPresentation) }),
  };
}

function parseStandardSemPresentationLayout(value: unknown): StandardSemPresentationLayoutV1 {
  const presentation = exact(value, ["schemaVersion", "objects"], ["schemaVersion", "objects"], "layout.diagram_layout.standardSemPresentation");
  if (presentation.schemaVersion !== 1 || !Array.isArray(presentation.objects)) {
    return fail("standard_sem_projection.presentation_invalid", "layout.diagram_layout.standardSemPresentation", "The Standard presentation layout must use schema version 1 and an object array.");
  }
  const seen = new Set<string>();
  const objects = presentation.objects.map((raw, index): StandardSemPresentationLayoutObject => {
    const subject = `layout.diagram_layout.standardSemPresentation.objects[${index}]`;
    const candidate = object(raw, subject);
    const id = exactStableId(candidate.id, `${subject}.id`);
    if (seen.has(id)) return fail("standard_sem_projection.presentation_id_duplicate", `${subject}.id`, "Presentation object IDs must be unique.");
    seen.add(id);
    const x = (key: string) => finite(candidate[key], `${subject}.${key}`);
    if (candidate.kind === "caption") {
      exact(candidate, ["kind", "id", "text", "x", "y"], ["kind", "id", "text", "x", "y"], subject);
      return { kind: "caption", id, text: text(candidate.text, `${subject}.text`), x: x("x"), y: x("y") };
    }
    if (candidate.kind === "note") {
      exact(candidate, ["kind", "id", "subject", "text", "x", "y"], ["kind", "id", "subject", "text", "x", "y"], subject);
      return { kind: "note", id, subject: text(candidate.subject, `${subject}.subject`), text: text(candidate.text, `${subject}.text`), x: x("x"), y: x("y") };
    }
    if (candidate.kind === "shape") {
      exact(candidate, ["kind", "id", "shape", "x", "y", "width", "height", "label", "style"], ["kind", "id", "shape", "x", "y", "width", "height", "label", "style"], subject);
      if (!["rectangle", "rounded_rectangle", "ellipse", "diamond"].includes(String(candidate.shape))) return fail("standard_sem_projection.presentation_shape_invalid", `${subject}.shape`, "The presentation shape is unsupported.");
      const width = x("width"); const height = x("height");
      if (width <= 0 || height <= 0) return fail("standard_sem_projection.presentation_size_invalid", subject, "Presentation dimensions must be positive.");
      return { kind: "shape", id, shape: candidate.shape as Extract<StandardSemPresentationLayoutObject, { kind: "shape" }>["shape"], x: x("x"), y: x("y"), width, height, label: nullableText(candidate.label, `${subject}.label`), style: textMap(candidate.style, `${subject}.style`) };
    }
    if (candidate.kind === "image") {
      exact(candidate, ["kind", "id", "assetRef", "altText", "x", "y", "width", "height", "style"], ["kind", "id", "assetRef", "altText", "x", "y", "width", "height", "style"], subject);
      const width = x("width"); const height = x("height");
      if (width <= 0 || height <= 0) return fail("standard_sem_projection.presentation_size_invalid", subject, "Presentation dimensions must be positive.");
      return { kind: "image", id, assetRef: text(candidate.assetRef, `${subject}.assetRef`, true), altText: text(candidate.altText, `${subject}.altText`, true), x: x("x"), y: x("y"), width, height, style: textMap(candidate.style, `${subject}.style`) };
    }
    if (candidate.kind === "line") {
      exact(candidate, ["kind", "id", "x1", "y1", "x2", "y2", "label", "startMarker", "endMarker", "style"], ["kind", "id", "x1", "y1", "x2", "y2", "label", "startMarker", "endMarker", "style"], subject);
      const x1 = x("x1"); const y1 = x("y1"); const x2 = x("x2"); const y2 = x("y2");
      if (x1 === x2 && y1 === y2) return fail("standard_sem_projection.presentation_line_invalid", subject, "A presentation line must have two distinct endpoints.");
      return { kind: "line", id, x1, y1, x2, y2, label: nullableText(candidate.label, `${subject}.label`), startMarker: nullableText(candidate.startMarker, `${subject}.startMarker`), endMarker: nullableText(candidate.endMarker, `${subject}.endMarker`), style: textMap(candidate.style, `${subject}.style`) };
    }
    return fail("standard_sem_projection.presentation_kind_invalid", `${subject}.kind`, "The presentation object kind is unsupported.");
  });
  return { schemaVersion: 1, objects };
}

function parsePoints(value: unknown, subject: string) {
  if (!Array.isArray(value)) return fail("standard_sem_projection.array_required", subject, `${subject} must be an array.`);
  return value.map((point, index) => parsePoint(point, `${subject}[${index}]`));
}

function parsePoint(value: unknown, subject: string) {
  const point = exact(value, ["x", "y"], ["x", "y"], subject);
  return { x: finite(point.x, `${subject}.x`), y: finite(point.y, `${subject}.y`) };
}

function deepFreeze<T>(value: T): T {
  if (!value || typeof value !== "object" || Object.isFrozen(value)) return value;
  Object.freeze(value);
  Object.values(value as Record<string, unknown>).forEach(deepFreeze);
  return value;
}
