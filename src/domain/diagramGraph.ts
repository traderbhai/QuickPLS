import { MarkerType, type Edge, type Node, type XYPosition } from "@xyflow/react";
import type {
  AnalysisRun,
  ConstructData,
  DiagramLayoutState,
  DiagramMode,
  DiagramOverlayMode,
  EdgeRouteStyle,
  IndicatorLayout,
  IndicatorSide,
  MeasurementConnectorLayout,
} from "../types";
import {
  deriveModerationAnchorProjections,
  hiddenInteractionNodeIds,
  isModerationAnchorData,
  isModerationConnectorData,
  moderationAnchorNodeId,
  moderationConnectorEdgeId,
  type ModerationAnchorProjectionV1,
  type ModerationConnectorProjectionV1,
  type ResultOverlaySelectionV1,
} from "./moderationDiagramProjectionV1";
import {
  SEM_SIZES,
  boxCenter,
  measureDiagramQuality,
  pointAtPolylineFraction,
  polylineMidpoint,
  renderedEdgePolyline,
  routeBetweenBoxes,
  routePolylineAroundObstacles,
  semNodeBox,
  semRectsOverlap,
  smartIndicatorPosition,
  type SemPoint,
  type SemRect,
} from "./semGeometry";

export interface IndicatorNodeData extends Record<string, unknown> {
  constructId: string;
  indicator: string;
  mode: ConstructData["mode"];
  displayMode: DiagramMode;
  loading?: number;
  weight?: number;
}

export interface LatentNodeData extends ConstructData {
  displayMode: DiagramMode;
  overlayMode: DiagramOverlayMode;
  pathCount: number;
  /** Presentation-only detail level derived from the live Canvas zoom. */
  semanticZoomLevel?: "far" | "medium" | "near";
}

export type ModerationAnchorNodeData = ModerationAnchorProjectionV1;

export interface DiagramGraph {
  nodes: Array<Node<LatentNodeData | IndicatorNodeData | ModerationAnchorNodeData>>;
  edges: Edge[];
  compatible: boolean;
  diagnostic: string | null;
}

export interface DiagramGraphOptions {
  layout?: DiagramLayoutState;
  layoutSource?: "current_canvas" | "tidy_publication";
  /** Effective Canvas editability after surface, publication, and layout locks. */
  layoutEditingEnabled?: boolean;
  /** Selected HOC whose lower-order membership should be projected visually. */
  selectedHigherOrderId?: string | null;
  /** Selected presentation-only interaction anchor in the editable canvas. */
  selectedInteractionTermId?: string | null;
  /** Selected presentation-only indicator connector in the editable canvas. */
  selectedMeasurementConnector?: { constructId: string; indicator: string } | null;
  /** Result-driven read-only highlighting without altering the scientific graph. */
  resultOverlay?: ResultOverlaySelectionV1 | null;
  /** Expert-only compatibility view; generated interaction constructs are hidden by default. */
  showGeneratedInteractionTerms?: boolean;
  /** Optional presentation layout overrides, keyed by stable interaction term id. */
  moderationAnchorFractions?: Readonly<Record<string, number>>;
  /** Optional visual connector routing points, keyed by moderation connector edge id. */
  moderationConnectorBendPoints?: Readonly<Record<string, readonly XYPosition[]>>;
  /** Optional transient anchors for read-only previews that have no scientific interaction nodes. */
  moderationAnchorProjections?: readonly ModerationAnchorProjectionV1[];
}

export interface MeasurementConnectorEdgeData extends Record<string, unknown> {
  visualOnly: true;
  relationshipKind: "measurement_connector";
  constructId: string;
  indicator: string;
  routeEditable: boolean;
  perimeterRouting: "continuous";
  routing: "straight" | "default" | "smoothstep" | "polyline";
  bendPoints?: XYPosition[];
  edgeClassName: string;
}

export interface StructuralSemEdgeData extends Record<string, unknown> {
  relationshipEditable: boolean;
}

export function isMeasurementConnectorEdgeData(value: unknown): value is MeasurementConnectorEdgeData {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const data = value as Partial<MeasurementConnectorEdgeData>;
  return data.visualOnly === true
    && data.relationshipKind === "measurement_connector"
    && typeof data.constructId === "string"
    && typeof data.indicator === "string"
    && typeof data.routeEditable === "boolean";
}

const LATENT_WIDTH = 150;
const LATENT_HEIGHT = 110;
const INDICATOR_WIDTH = 96;
const INDICATOR_HEIGHT = 34;
const MEASUREMENT_GAP = 88;
const INDICATOR_ROW_GAP = 42;
const SMARTPLS_LATENT_WIDTH = SEM_SIZES.smartplsEllipse.width;
const SMARTPLS_LATENT_NODE_HEIGHT = SEM_SIZES.smartplsLatent.height;
const SMARTPLS_INDICATOR_WIDTH = SEM_SIZES.smartplsIndicator.width;
const SMARTPLS_INDICATOR_HEIGHT = SEM_SIZES.smartplsIndicator.height;
const SMARTPLS_VERTICAL_GAP = 320;

const derivedDiagramIdPart = (value: string) => encodeURIComponent(value);
const decodedDiagramIdPart = (value: string) => {
  try {
    return decodeURIComponent(value);
  } catch {
    return null;
  }
};

export const isIndicatorNodeId = (id: string) => id.startsWith("indicator::");
export const indicatorNodeId = (constructId: string, indicator: string) => `indicator::${derivedDiagramIdPart(constructId)}::${derivedDiagramIdPart(indicator)}`;
export const parseIndicatorNodeId = (id: string) => {
  const [prefix, encodedConstructId, encodedIndicator, ...extra] = id.split("::");
  if (prefix !== "indicator" || !encodedConstructId || !encodedIndicator || extra.length) return null;
  const constructId = decodedDiagramIdPart(encodedConstructId);
  const indicator = decodedDiagramIdPart(encodedIndicator);
  return constructId && indicator ? { constructId, indicator } : null;
};

export const measurementConnectorEdgeId = (constructId: string, indicator: string) =>
  `measurement::${derivedDiagramIdPart(constructId)}::${derivedDiagramIdPart(indicator)}`;

export const parseMeasurementConnectorEdgeId = (id: string) => {
  const [prefix, encodedConstructId, encodedIndicator, ...extra] = id.split("::");
  if (prefix !== "measurement" || !encodedConstructId || !encodedIndicator || extra.length) return null;
  const constructId = decodedDiagramIdPart(encodedConstructId);
  const indicator = decodedDiagramIdPart(encodedIndicator);
  return constructId && indicator ? { constructId, indicator } : null;
};

function measurementConnectorRoutingStyle(routing: EdgeRouteStyle): MeasurementConnectorEdgeData["routing"] {
  if (routing === "curved") return "default";
  if (routing === "orthogonal") return "smoothstep";
  return routing;
}

function defaultMeasurementConnectorBend(
  constructNode: Node,
  indicatorNode: Node,
): XYPosition[] {
  const constructCenter = boxCenter(semNodeBox(constructNode));
  const indicatorCenter = boxCenter(semNodeBox(indicatorNode));
  const dx = indicatorCenter.x - constructCenter.x;
  const dy = indicatorCenter.y - constructCenter.y;
  const length = Math.hypot(dx, dy);
  const normal = length > 0.001
    ? { x: -dy / length, y: dx / length }
    : { x: 1, y: -1 };
  return [{
    x: Math.round((constructCenter.x + indicatorCenter.x) / 2 + normal.x * 36),
    y: Math.round((constructCenter.y + indicatorCenter.y) / 2 + normal.y * 36),
  }];
}

function measurementConnectorRoute(
  layout: MeasurementConnectorLayout | undefined,
  constructNode: Node,
  indicatorNode: Node,
  reuseStoredBends: boolean,
): Pick<MeasurementConnectorEdgeData, "routing" | "bendPoints"> {
  const style = layout?.routing ?? "straight";
  if (style !== "polyline") return { routing: measurementConnectorRoutingStyle(style) };
  const storedBends = reuseStoredBends ? finiteBendPoints(layout?.bendPoints) : [];
  return {
    routing: "polyline",
    bendPoints: storedBends.length
      ? storedBends
      : defaultMeasurementConnectorBend(constructNode, indicatorNode),
  };
}

export function buildDiagramGraph(
  modelNodes: Array<Node<ConstructData>>,
  modelEdges: Edge[],
  mode: DiagramMode,
  overlayMode: DiagramOverlayMode,
  run?: AnalysisRun,
  options: DiagramGraphOptions = {},
): DiagramGraph {
  const scientificStructuralEdges = modelEdges.filter((edge) => edge.data?.role !== "covariance");
  const generatedInteractionIds = hiddenInteractionNodeIds(modelNodes, modelEdges);
  const displayModelNodes = options.showGeneratedInteractionTerms
    ? modelNodes
    : modelNodes.filter((node) => !generatedInteractionIds.has(node.id));
  const structuralEdges = scientificStructuralEdges.filter((edge) => options.showGeneratedInteractionTerms
    || (!generatedInteractionIds.has(edge.source) && !generatedInteractionIds.has(edge.target)));
  const covarianceEdges = modelEdges
    .filter((edge) => edge.data?.role === "covariance")
    .filter((edge) => options.showGeneratedInteractionTerms
      || (!generatedInteractionIds.has(edge.source) && !generatedInteractionIds.has(edge.target)));
  const paperStyle = mode === "sem" || mode === "publication" || mode === "smartpls_result";
  const lockedResultMode = mode === "smartpls_result" || mode === "publication";
  const smartplsPlacement = (mode === "publication" || mode === "smartpls_result") && options.layoutSource !== "current_canvas" ? smartplsLayout(displayModelNodes, structuralEdges, options.layout) : null;
  const layoutEditingEnabled = options.layoutEditingEnabled !== false
    && !lockedResultMode
    && !Boolean(options.layout?.layoutLocked);
  const structuralShape = structuralShapeMaps(displayModelNodes, structuralEdges);
  const result = run?.status === "completed" ? run.result : undefined;
  const compatible = result ? resultMatchesModel(modelNodes, scientificStructuralEdges, result) : true;
  const resultForOverlay = result && compatible ? result : undefined;
  const outerEstimatesForOverlay = resultForOverlay?.plsc
    ? resultForOverlay.plsc.corrected_outer_loadings ?? []
    : resultForOverlay?.outer_estimates ?? [];
  const loadingByConstruct = new Map<string, Map<string, { loading: number; weight: number }>>();
  for (const estimate of outerEstimatesForOverlay) {
    const current = loadingByConstruct.get(estimate.construct) ?? new Map<string, { loading: number; weight: number }>();
    current.set(estimate.indicator, { loading: estimate.loading, weight: estimate.weight });
    loadingByConstruct.set(estimate.construct, current);
  }
  const pathCoefficients = new Map((resultForOverlay?.paths ?? []).map((path) => [`${path.source}\u0000${path.target}`, path.coefficient]));
  const visualNodes: DiagramGraph["nodes"] = displayModelNodes.map((node) => {
    const estimates = [...(loadingByConstruct.get(node.id)?.entries() ?? [])];
    const layoutPosition = options.layout?.constructLayouts[node.id];
    return ({
    ...node,
    type: mode === "compact" ? "construct" : "latent",
    position: smartplsPlacement?.latents.get(node.id) ?? (layoutPosition ? { x: layoutPosition.x, y: layoutPosition.y } : node.position),
    draggable: !lockedResultMode,
    data: {
      ...node.data,
      displayMode: mode,
      overlayMode,
      resultLoadings: resultForOverlay ? Object.fromEntries(estimates.map(([indicator, estimate]) => [indicator, estimate.loading])) : undefined,
      resultR2: resultForOverlay?.r_squared[node.id],
      pathCount: structuralEdges.filter((edge) => edge.source === node.id || edge.target === node.id).length,
    } satisfies LatentNodeData,
  });
  });
  const visualEdges: Edge[] = structuralEdges.map((edge) => {
    const coefficient = pathCoefficients.get(`${edge.source}\u0000${edge.target}`);
    const sourceNode = visualNodes.find((node) => node.id === edge.source);
    const targetNode = visualNodes.find((node) => node.id === edge.target);
    const route = paperStyle && sourceNode && targetNode ? routeSides(sourceNode, targetNode) : null;
    const routeLayout = structuralRouting(edge, paperStyle, options.layout);
    return {
      ...edge,
      type: paperStyle ? "semEdge" : edge.type ?? "smoothstep",
      sourceHandle: route ? handleId("source", route.source) : edge.sourceHandle,
      targetHandle: route ? handleId("target", route.target) : edge.targetHandle,
      label: resultForOverlay && coefficient !== undefined && (paperStyle || overlayMode === "paths_r2" || overlayMode === "significance")
        ? coefficient.toFixed(3)
        : paperStyle ? (mode === "sem" ? edge.data?.role === "control" ? "Control" : edge.label ?? "Path" : "")
          : edge.data?.role === "control" ? "Control" : edge.label ?? "Path",
      markerEnd: { type: MarkerType.ArrowClosed, width: paperStyle ? 16 : 16, height: paperStyle ? 16 : 16, color: paperStyle ? "#222" : undefined },
      className: edge.data?.role === "control" ? "control-edge" : paperStyle ? "smartpls-structural-edge structural-edge" : "structural-edge",
      selectable: layoutEditingEnabled,
      focusable: layoutEditingEnabled,
      reconnectable: layoutEditingEnabled,
      deletable: layoutEditingEnabled,
      data: {
        ...edge.data,
        relationshipEditable: layoutEditingEnabled,
        perimeterRouting: "continuous",
        routing: routeLayout.routing,
        ...(routeLayout.bendPoints?.length ? { bendPoints: routeLayout.bendPoints } : {}),
        labelOffset: options.layout?.edgeLayouts[edge.id]?.labelOffset,
        edgeClassName: edge.data?.role === "control" ? "control-edge" : paperStyle ? "smartpls-structural-edge structural-edge" : "structural-edge",
      } satisfies StructuralSemEdgeData,
    };
  });

  const moderationAnchors = options.moderationAnchorProjections
    ? [...options.moderationAnchorProjections]
    : deriveModerationAnchorProjections(
      modelNodes,
      modelEdges,
      options.moderationAnchorFractions ?? options.layout?.moderationAnchorFractions,
    );
  for (const anchor of moderationAnchors) {
    const focalEdge = visualEdges.find((edge) => edge.id === anchor.focalRelationId);
    const sourceNode = focalEdge ? visualNodes.find((node) => node.id === focalEdge.source) : undefined;
    const targetNode = focalEdge ? visualNodes.find((node) => node.id === focalEdge.target) : undefined;
    if (!focalEdge || !sourceNode || !targetNode) continue;
    const focalPoint = pointAtPolylineFraction(renderedEdgePolyline(focalEdge, sourceNode, targetNode), anchor.fraction);
    const position = { x: focalPoint.x - 11, y: focalPoint.y - 11 };
    const anchorId = moderationAnchorNodeId(anchor.interactionTermId);
    const highlightedByResult = Boolean(options.resultOverlay?.interactionTermIds.includes(anchor.interactionTermId));
    const selected = options.selectedInteractionTermId === anchor.interactionTermId || highlightedByResult;
    visualNodes.push({
      id: anchorId,
      type: "moderationAnchor",
      position,
      data: { ...anchor, editable: !lockedResultMode },
      draggable: !lockedResultMode,
      connectable: !lockedResultMode && anchor.order === 2,
      deletable: false,
      selectable: true,
      focusable: false,
      selected,
      className: highlightedByResult ? "result-overlay-highlight moderation-result-highlight" : undefined,
      zIndex: 8,
    });
    const focalClass = [
      String(focalEdge.className ?? ""),
      "moderated-focal-edge",
      highlightedByResult ? "result-overlay-edge-highlight" : "",
    ].filter(Boolean).join(" ");
    focalEdge.className = focalClass;
    focalEdge.data = {
      ...focalEdge.data,
      moderationTermIds: [
        ...new Set([
          ...((Array.isArray(focalEdge.data?.moderationTermIds) ? focalEdge.data.moderationTermIds : []) as string[]),
          anchor.interactionTermId,
        ]),
      ],
      edgeClassName: focalClass,
    };
    const anchorCenter = { x: position.x + 11, y: position.y + 11 };
    for (const moderatorId of anchor.moderatorIds) {
      const moderatorNode = visualNodes.find((node) => node.id === moderatorId);
      if (!moderatorNode) continue;
      const moderatorCenter = boxCenter(semNodeBox(moderatorNode));
      const sides = moderationConnectorSides(moderatorCenter, anchorCenter);
      const sourceSide = mode === "compact"
        ? (Math.abs(anchorCenter.x - moderatorCenter.x) >= Math.abs(anchorCenter.y - moderatorCenter.y) ? "right" : "bottom")
        : sides.source;
      const connectorClass = [
        "moderation-connector-edge",
        anchor.order === 3 ? "three-way" : "two-way",
        selected ? "selected" : "",
        highlightedByResult ? "result-overlay-edge-highlight" : "",
      ].filter(Boolean).join(" ");
      const connectorId = moderationConnectorEdgeId(anchor.interactionTermId, moderatorId);
      const bendPoints = (options.moderationConnectorBendPoints
        ?? options.layout?.moderationConnectorBendPoints)?.[connectorId]
        ?.filter((point) => Number.isFinite(point.x) && Number.isFinite(point.y))
        .map((point) => ({ x: point.x, y: point.y }));
      const connectorData: ModerationConnectorProjectionV1 = {
        visualOnly: true,
        relationshipKind: "moderation_connector",
        interactionTermId: anchor.interactionTermId,
        focalRelationId: anchor.focalRelationId,
        moderatorId,
        order: anchor.order,
        edgeClassName: connectorClass,
        routing: bendPoints?.length ? "polyline" : "straight",
        ...(bendPoints?.length ? { bendPoints } : {}),
      };
      visualEdges.push({
        id: connectorId,
        source: moderatorId,
        target: anchorId,
        sourceHandle: handleId("source", sourceSide),
        targetHandle: handleId("target", sides.target),
        type: "semEdge",
        markerEnd: { type: MarkerType.ArrowClosed, width: 11, height: 11 },
        selectable: true,
        deletable: false,
        reconnectable: false,
        focusable: false,
        className: connectorClass,
        interactionWidth: 26,
        data: connectorData,
        zIndex: 7,
      });
    }
  }

  if (options.resultOverlay) {
    const highlightedNodes = new Set(options.resultOverlay.nodeIds);
    const highlightedRelations = new Set(options.resultOverlay.relationIds);
    const relationHighlighted = (edge: Edge): boolean => {
      const authorityRelationId = (edge.data as {
        standardSemV4Authority?: { authorityObjectId?: string };
      } | undefined)?.standardSemV4Authority?.authorityObjectId;
      return highlightedRelations.has(edge.id)
        || (authorityRelationId != null && highlightedRelations.has(authorityRelationId));
    };
    for (const edge of visualEdges) {
      if (!relationHighlighted(edge)) continue;
      highlightedNodes.add(edge.source);
      highlightedNodes.add(edge.target);
    }
    for (const node of visualNodes) {
      if (!highlightedNodes.has(node.id)) continue;
      node.className = [node.className, "result-overlay-highlight"].filter(Boolean).join(" ");
    }
    for (const edge of visualEdges) {
      if (!relationHighlighted(edge)) continue;
      const className = [edge.className, "result-overlay-edge-highlight"].filter(Boolean).join(" ");
      edge.className = className;
      edge.data = { ...edge.data, edgeClassName: className };
    }
  }

  const selectedHigherOrder = options.selectedHigherOrderId
    ? displayModelNodes.find((node) => node.id === options.selectedHigherOrderId
      && node.data.semantic === "higher_order"
      && node.data.higherOrder)
    : undefined;
  if (selectedHigherOrder?.data.higherOrder) {
    for (const componentId of selectedHigherOrder.data.higherOrder.components) {
      const componentNode = visualNodes.find((node) => node.id === componentId);
      if (!componentNode) continue;
      componentNode.className = [componentNode.className, "hoc-component-highlight"].filter(Boolean).join(" ");
      visualEdges.push({
        id: `hoc-membership::${selectedHigherOrder.id}::${componentId}`,
        source: componentId,
        target: selectedHigherOrder.id,
        type: "straight",
        selectable: false,
        deletable: false,
        focusable: false,
        reconnectable: false,
        animated: false,
        style: {
          stroke: "#7a8590",
          strokeDasharray: "5 4",
          strokeWidth: 1.25,
          opacity: 0.78,
        },
        data: {
          visualOnly: true,
          relationshipKind: "higher_order_membership",
          edgeClassName: "hoc-membership-edge",
        },
      });
    }
  }

  if (mode !== "compact") {
    for (const node of displayModelNodes) {
      const latentPosition = visualNodes.find((visualNode) => visualNode.id === node.id)?.position ?? node.position;
      const automaticPlacement = smartplsPlacement?.indicators.get(node.id);
      const placement = automaticPlacement
        ? automaticPlacement.map((point, index) => {
            const saved = options.layout?.indicatorLayouts[node.id]?.[node.data.indicators[index] ?? ""];
            return saved?.side === "free" && Number.isFinite(saved.x) && Number.isFinite(saved.y)
              ? { x: Number(saved.x), y: Number(saved.y) }
              : point;
          })
        : indicatorPositionsForConstruct(node, latentPosition, paperStyle, structuralShape, options.layout);
      node.data.indicators.forEach((indicator, index) => {
        const estimate = loadingByConstruct.get(node.id)?.get(indicator);
        const indicatorPosition = placement[index] ?? latentPosition;
        const reflective = node.data.mode === "reflective";
        const latentForRoute = visualNodes.find((visualNode) => visualNode.id === node.id);
        const indicatorForRoute = {
          id: indicatorNodeId(node.id, indicator),
          type: "indicator",
          position: indicatorPosition,
        } as Node<LatentNodeData | IndicatorNodeData>;
        const route = paperStyle && latentForRoute ? routeSides(latentForRoute, indicatorForRoute) : null;
        const constructForRoute = latentForRoute ?? {
          id: node.id,
          type: "latent",
          position: latentPosition,
        } as Node<LatentNodeData | IndicatorNodeData>;
        const connectorRoute = measurementConnectorRoute(
          options.layout?.measurementConnectorLayouts?.[node.id]?.[indicator],
          constructForRoute,
          indicatorForRoute,
          !smartplsPlacement,
        );
        const routeEditable = layoutEditingEnabled;
        const selected = routeEditable
          && options.selectedMeasurementConnector?.constructId === node.id
          && options.selectedMeasurementConnector.indicator === indicator;
        const edgeClassName = reflective
          ? `${paperStyle ? "smartpls-measurement-edge " : ""}measurement-edge reflective`
          : `${paperStyle ? "smartpls-measurement-edge " : ""}measurement-edge formative`;
        visualNodes.push({
          id: indicatorNodeId(node.id, indicator),
          type: "indicator",
          position: indicatorPosition,
          draggable: !lockedResultMode,
          selectable: true,
          data: { constructId: node.id, indicator, mode: node.data.mode, displayMode: mode, loading: estimate?.loading, weight: estimate?.weight },
        });
        visualEdges.push({
          id: measurementConnectorEdgeId(node.id, indicator),
          source: reflective ? node.id : indicatorNodeId(node.id, indicator),
          target: reflective ? indicatorNodeId(node.id, indicator) : node.id,
          sourceHandle: route ? handleId("source", reflective ? route.source : route.target) : undefined,
          targetHandle: route ? handleId("target", reflective ? route.target : route.source) : undefined,
          type: paperStyle ? "semEdge" : "straight",
          label: resultForOverlay && (paperStyle || overlayMode === "loadings")
            ? (reflective ? estimate?.loading : estimate?.weight)?.toFixed(3) ?? ""
            : paperStyle ? ""
              : reflective ? "loading" : "weight",
          markerEnd: { type: MarkerType.ArrowClosed, width: paperStyle ? 13 : 14, height: paperStyle ? 13 : 14, color: paperStyle ? "#222" : undefined },
          className: edgeClassName,
          selectable: routeEditable,
          focusable: routeEditable,
          selected,
          deletable: false,
          reconnectable: false,
          interactionWidth: routeEditable ? 24 : 12,
          data: {
            visualOnly: true,
            relationshipKind: "measurement_connector",
            constructId: node.id,
            indicator,
            routeEditable,
            perimeterRouting: "continuous",
            routing: connectorRoute.routing,
            ...(connectorRoute.bendPoints?.length ? { bendPoints: connectorRoute.bendPoints } : {}),
            edgeClassName,
          } satisfies MeasurementConnectorEdgeData,
        });
      });
    }
    for (const edge of covarianceEdges) {
      visualEdges.push({
        ...edge,
        type: paperStyle ? "semEdge" : "default",
        label: edge.label ?? "Covariance",
        markerStart: { type: MarkerType.ArrowClosed, width: 12, height: 12 },
        markerEnd: { type: MarkerType.ArrowClosed, width: 12, height: 12 },
        className: "covariance-edge",
        selectable: layoutEditingEnabled,
        focusable: layoutEditingEnabled,
        reconnectable: layoutEditingEnabled,
        deletable: layoutEditingEnabled,
        data: {
          ...edge.data,
          relationshipEditable: layoutEditingEnabled,
          routing: "default",
          labelOffset: options.layout?.edgeLayouts[edge.id]?.labelOffset,
          edgeClassName: "covariance-edge",
        } satisfies StructuralSemEdgeData,
      });
    }
  }

  const routedEdges = paperStyle ? applyAutomaticEdgeRoutes(visualEdges, options.layout) : visualEdges;
  repositionModerationAnchors(routedEdges, visualNodes);
  const connectorRoutedEdges = paperStyle ? applyAutomaticModerationConnectorRoutes(routedEdges, visualNodes) : routedEdges;
  const edgesWithLabelOffsets = applyAutomaticEdgeLabelOffsets(connectorRoutedEdges, visualNodes);

  return {
    nodes: visualNodes,
    edges: edgesWithLabelOffsets,
    compatible,
    diagnostic: result && !compatible
      ? "Selected run does not match the current model. Numeric overlays are hidden."
      : lockedResultMode && !resultForOverlay
        ? "Run or select a compatible result to show estimates."
        : null,
  };
}

function applyAutomaticEdgeLabelOffsets(edges: Edge[], nodes: DiagramGraph["nodes"]): Edge[] {
  const nodeRects: SemRect[] = nodes.map((node) => semNodeBox(node));
  const occupied: SemRect[] = [];
  return edges.map((edge) => {
    const label = typeof edge.label === "string" ? edge.label : "";
    if (!label) return edge;
    const measurementConnector = isMeasurementConnectorEdgeData(edge.data);
    const existing = edge.data?.labelOffset;
    if (!measurementConnector && existing && typeof existing === "object") return edge;
    const source = nodes.find((node) => node.id === edge.source);
    const target = nodes.find((node) => node.id === edge.target);
    if (!source || !target) return edge;
    const mid = polylineMidpoint(renderedEdgePolyline(edge, source, target));
    const width = Math.min(190, Math.max(34, label.length * 7 + 14));
    const height = 20;
    // Loading and weight badges belong to their generated measurement route.
    // SemEdge already derives an on-path anchor for every supported routing
    // style, so a perpendicular offset would detach the value from the
    // connector it describes. Reserve its centered rectangle so labels that
    // are allocated later (for example covariance labels) still avoid it.
    if (measurementConnector) {
      occupied.push({ x: mid.x - width / 2, y: mid.y - height / 2, width, height });
      return edge;
    }
    const candidates = [
      { x: 0, y: -18 },
      { x: 0, y: 18 },
      { x: 22, y: 0 },
      { x: -22, y: 0 },
      { x: 24, y: -18 },
      { x: -24, y: -18 },
      { x: 24, y: 18 },
      { x: -24, y: 18 },
    ];
    const chosen = candidates.find((offset) => {
      const rect = { x: mid.x + offset.x - width / 2, y: mid.y + offset.y - height / 2, width, height };
      return !nodeRects.some((nodeRect) => semRectsOverlap(rect, nodeRect))
        && !occupied.some((labelRect) => semRectsOverlap(rect, labelRect));
    }) ?? { x: 0, y: -18 - occupied.length * 4 };
    occupied.push({ x: mid.x + chosen.x - width / 2, y: mid.y + chosen.y - height / 2, width, height });
    return { ...edge, data: { ...edge.data, labelOffset: chosen } };
  });
}

function finiteBendPoints(value: unknown): SemPoint[] {
  if (!Array.isArray(value)) return [];
  return value.flatMap((point): SemPoint[] => {
    if (!point || typeof point !== "object") return [];
    const x = Number((point as { x?: unknown }).x);
    const y = Number((point as { y?: unknown }).y);
    return Number.isFinite(x) && Number.isFinite(y) ? [{ x, y }] : [];
  });
}

function applyAutomaticEdgeRoutes(edges: Edge[], layout?: DiagramLayoutState): Edge[] {
  return edges.map((edge) => {
    const className = String(edge.className ?? edge.data?.edgeClassName ?? "");
    const structural = className.includes("structural-edge") || className.includes("control-edge");
    if (!structural) return edge;
    const savedRoute = layout?.edgeLayouts[edge.id];
    if (savedRoute?.pinned) return edge;
    return { ...edge, data: { ...edge.data, routing: "straight", bendPoints: undefined } };
  });
}

function repositionModerationAnchors(edges: Edge[], nodes: DiagramGraph["nodes"]): void {
  for (const anchorNode of nodes) {
    if (!isModerationAnchorData(anchorNode.data)) continue;
    const anchor = anchorNode.data;
    const focal = edges.find((edge) => edge.id === anchor.focalRelationId);
    const source = focal ? nodes.find((node) => node.id === focal.source) : undefined;
    const target = focal ? nodes.find((node) => node.id === focal.target) : undefined;
    if (!focal || !source || !target) continue;
    const point = pointAtPolylineFraction(renderedEdgePolyline(focal, source, target), anchor.fraction);
    anchorNode.position = { x: point.x - 11, y: point.y - 11 };
  }
}

function applyAutomaticModerationConnectorRoutes(edges: Edge[], nodes: DiagramGraph["nodes"]): Edge[] {
  return edges.map((edge) => {
    if (!isModerationConnectorData(edge.data)) return edge;
    const storedBends = finiteBendPoints(edge.data.bendPoints);
    const source = nodes.find((node) => node.id === edge.source);
    const target = nodes.find((node) => node.id === edge.target);
    if (!source || !target) return edge;
    const sides = moderationConnectorSides(boxCenter(semNodeBox(source)), boxCenter(semNodeBox(target)));
    const handledEdge = { ...edge, sourceHandle: handleId("source", sides.source), targetHandle: handleId("target", sides.target) };
    if (storedBends.length) return handledEdge;
    const route = renderedEdgePolyline(handledEdge, source, target);
    const obstacles = nodes.filter((node) => node.id !== edge.source && node.id !== edge.target).map((node) => semNodeBox(node));
    const bends = routePolylineAroundObstacles(route[0]!, route[route.length - 1]!, obstacles, 10);
    return {
      ...handledEdge,
      ...(bends.length ? { data: { ...edge.data, routing: "polyline", bendPoints: bends } } : {}),
    };
  });
}

export function indicatorPositions(position: XYPosition, count: number): XYPosition[] {
  if (count === 0) return [];
  const leftCount = Math.ceil(count / 2);
  return Array.from({ length: count }, (_, index) => {
    const leftSide = index < leftCount;
    const sideIndex = leftSide ? index : index - leftCount;
    const sideCount = leftSide ? leftCount : count - leftCount;
    const stackHeight = Math.max(0, sideCount - 1) * INDICATOR_ROW_GAP;
    return {
      x: position.x + LATENT_WIDTH / 2 + (leftSide ? -MEASUREMENT_GAP - INDICATOR_WIDTH : MEASUREMENT_GAP),
      y: position.y + LATENT_HEIGHT / 2 - INDICATOR_HEIGHT / 2 - stackHeight / 2 + sideIndex * INDICATOR_ROW_GAP,
    };
  });
}

function compactIndicatorPositions(
  position: XYPosition,
  count: number,
  side: "left" | "right" | "top" | "bottom",
): XYPosition[] {
  if (count === 0) return [];
  if (side === "left" || side === "right") {
    const stackHeight = Math.max(0, count - 1) * INDICATOR_ROW_GAP;
    return Array.from({ length: count }, (_, index) => ({
      x: position.x + LATENT_WIDTH / 2 + (side === "left" ? -MEASUREMENT_GAP - INDICATOR_WIDTH : MEASUREMENT_GAP),
      y: position.y + LATENT_HEIGHT / 2 - INDICATOR_HEIGHT / 2 - stackHeight / 2 + index * INDICATOR_ROW_GAP,
    }));
  }
  const columnGap = INDICATOR_WIDTH + 14;
  const stackWidth = INDICATOR_WIDTH + Math.max(0, count - 1) * columnGap;
  return Array.from({ length: count }, (_, index) => ({
    x: position.x + LATENT_WIDTH / 2 - stackWidth / 2 + index * columnGap,
    y: side === "top"
      ? position.y - MEASUREMENT_GAP - INDICATOR_HEIGHT
      : position.y + LATENT_HEIGHT + MEASUREMENT_GAP,
  }));
}

export function defaultDiagramLayout(modelNodes: Array<Node<ConstructData>>, modelEdges: Edge[], existing?: Partial<DiagramLayoutState>): DiagramLayoutState {
  const structuralEdges = modelEdges.filter((edge) => edge.data?.role !== "covariance");
  const shape = structuralShapeMaps(modelNodes, structuralEdges);
  const constructLayouts: DiagramLayoutState["constructLayouts"] = {};
  const indicatorLayouts: DiagramLayoutState["indicatorLayouts"] = {};
  const measurementConnectorLayouts: DiagramLayoutState["measurementConnectorLayouts"] = {};
  for (const node of modelNodes) {
    constructLayouts[node.id] = {
      x: existing?.constructLayouts?.[node.id]?.x ?? node.position.x,
      y: existing?.constructLayouts?.[node.id]?.y ?? node.position.y,
      width: existing?.constructLayouts?.[node.id]?.width,
      height: existing?.constructLayouts?.[node.id]?.height,
      pinned: existing?.constructLayouts?.[node.id]?.pinned,
    };
    const currentIndicators: Record<string, IndicatorLayout> = {};
    node.data.indicators.forEach((indicator, index) => {
      const previous = existing?.indicatorLayouts?.[node.id]?.[indicator];
      currentIndicators[indicator] = previous
        ? previous.pinned
          ? { side: previous.side, x: previous.x, y: previous.y, order: previous.order ?? index, pinned: previous.pinned }
          : { side: indicatorSide(node.id, shape, false), order: previous.order ?? index, pinned: false }
        : { side: indicatorSide(node.id, shape, false), order: index };
    });
    indicatorLayouts[node.id] = currentIndicators;
    const currentConnectors: Record<string, MeasurementConnectorLayout> = {};
    for (const indicator of node.data.indicators) {
      const previous = existing?.measurementConnectorLayouts?.[node.id]?.[indicator];
      if (!previous || !["straight", "curved", "orthogonal", "polyline"].includes(previous.routing)) continue;
      const bendPoints = finiteBendPoints(previous.bendPoints);
      currentConnectors[indicator] = {
        routing: previous.routing,
        ...(bendPoints.length ? { bendPoints } : {}),
      };
    }
    if (Object.keys(currentConnectors).length) measurementConnectorLayouts[node.id] = currentConnectors;
  }
  const edgeLayouts: DiagramLayoutState["edgeLayouts"] = {};
  for (const edge of modelEdges) {
    const previous = existing?.edgeLayouts?.[edge.id];
    edgeLayouts[edge.id] = previous
      ? { routing: previous.routing, bendPoints: previous.bendPoints, labelOffset: previous.labelOffset, pinned: previous.pinned }
      : { routing: edge.type === "straight" ? "straight" : edge.type === "default" ? "curved" : "orthogonal" };
  }
  return {
    diagramVersion: "sem_designer_v1",
    constructLayouts,
    indicatorLayouts,
    edgeLayouts,
    measurementConnectorLayouts,
    diagramViewport: existing?.diagramViewport,
    diagramTheme: existing?.diagramTheme === "academic_grayscale" || existing?.diagramTheme === "quickpls_color" || existing?.diagramTheme === "high_contrast" || existing?.diagramTheme === "journal_mono" || existing?.diagramTheme === "smartpls_like" ? existing.diagramTheme : "smartpls_like",
    showGrid: existing?.showGrid ?? true,
    layoutLocked: existing?.layoutLocked ?? false,
    ...(existing?.moderationAnchorFractions
      ? { moderationAnchorFractions: { ...existing.moderationAnchorFractions } }
      : {}),
    ...(existing?.moderationConnectorBendPoints
      ? {
          moderationConnectorBendPoints: Object.fromEntries(
            Object.entries(existing.moderationConnectorBendPoints)
              .map(([id, points]) => [id, points.map((point) => ({ ...point }))]),
          ),
        }
      : {}),
    ...(existing?.standardSemPresentation
      ? {
          standardSemPresentation: {
            schemaVersion: 1,
            objects: existing.standardSemPresentation.objects.map((object) => ({
              ...object,
              ...(object.kind === "shape" || object.kind === "image" || object.kind === "line"
                ? { style: { ...object.style } }
                : {}),
            })),
          },
        }
      : {}),
  };
}

interface RelativeEnvelope extends SemRect {
  indicatorPositions: XYPosition[];
}

function presentationEnvelopeForConstruct(
  node: Node<ConstructData>,
  shape: ReturnType<typeof structuralShapeMaps>,
  finalLevel: boolean,
  columnIndex: number,
  columnSize: number,
  layout?: DiagramLayoutState,
): RelativeEnvelope {
  const saved = layout?.indicatorLayouts[node.id];
  const defaultSide = indicatorSide(node.id, shape, finalLevel, columnIndex, columnSize);
  const positions: Array<XYPosition | undefined> = new Array(node.data.indicators.length);
  const bySide = new Map<Exclude<IndicatorSide, "free">, Array<{ indicator: string; index: number; order: number }>>();
  node.data.indicators.forEach((indicator, index) => {
    const item = saved?.[indicator];
    if (item?.side === "free" && Number.isFinite(item.x) && Number.isFinite(item.y)) {
      positions[index] = { x: Number(item.x) - node.position.x, y: Number(item.y) - node.position.y };
      return;
    }
    const side = item?.side && item.side !== "free" ? item.side : defaultSide;
    bySide.set(side, [...(bySide.get(side) ?? []), { indicator, index, order: item?.order ?? index }]);
  });
  for (const [side, entries] of bySide) {
    const ordered = [...entries].sort((left, right) => left.order - right.order || left.indicator.localeCompare(right.indicator));
    const generated = smartplsIndicatorPositions({ x: 0, y: 0 }, ordered.length, side);
    ordered.forEach((entry, index) => { positions[entry.index] = generated[index]!; });
  }
  const rects: SemRect[] = [
    { x: 0, y: 0, width: SMARTPLS_LATENT_WIDTH, height: SMARTPLS_LATENT_NODE_HEIGHT },
    ...positions.flatMap((position, index): SemRect[] => {
      const indicator = node.data.indicators[index];
      const savedPosition = indicator ? saved?.[indicator] : undefined;
      if (!position || (savedPosition?.side === "free" && Number.isFinite(savedPosition.x) && Number.isFinite(savedPosition.y))) return [];
      return [{ x: position.x, y: position.y, width: SMARTPLS_INDICATOR_WIDTH, height: SMARTPLS_INDICATOR_HEIGHT }];
    }),
  ];
  const left = Math.min(...rects.map((rect) => rect.x));
  const top = Math.min(...rects.map((rect) => rect.y));
  const right = Math.max(...rects.map((rect) => rect.x + rect.width));
  const bottom = Math.max(...rects.map((rect) => rect.y + rect.height));
  return { x: left, y: top, width: right - left, height: bottom - top, indicatorPositions: positions.map((point) => point ?? { x: 0, y: 0 }) };
}

function smartplsLayout(modelNodes: Array<Node<ConstructData>>, structuralEdges: Edge[], layout?: DiagramLayoutState) {
  const shape = structuralShapeMaps(modelNodes, structuralEdges);

  const level = new Map<string, number>();
  const visit = (id: string, seen = new Set<string>()): number => {
    if (level.has(id)) return level.get(id)!;
    if (seen.has(id)) return 0;
    const currentParents = shape.parents.get(id) ?? [];
    const value = currentParents.length === 0 ? 0 : 1 + Math.max(...currentParents.map((parent) => visit(parent, new Set([...seen, id]))));
    level.set(id, value);
    return value;
  };
  for (const node of modelNodes) visit(node.id);

  const byLevel = new Map<number, Array<Node<ConstructData>>>();
  for (const node of modelNodes) {
    const currentLevel = level.get(node.id) ?? 0;
    byLevel.set(currentLevel, [...(byLevel.get(currentLevel) ?? []), node]);
  }
  const orderedLevels = orderSmartplsLevels(byLevel, shape);

  const envelopes = new Map<string, RelativeEnvelope>();
  const maxLevel = Math.max(...level.values(), 0);
  for (const [currentLevel, columnNodes] of orderedLevels) {
    columnNodes.forEach((node, index) => envelopes.set(node.id, presentationEnvelopeForConstruct(
      node,
      shape,
      currentLevel === maxLevel,
      index,
      columnNodes.length,
      layout,
    )));
  }
  const horizontalLane = 130;
  const verticalLane = 92;
  const columnMetrics = new Map([...orderedLevels].map(([currentLevel, columnNodes]) => {
    const columnEnvelopes = columnNodes.map((node) => envelopes.get(node.id)!);
    return [currentLevel, {
      left: Math.min(...columnEnvelopes.map((envelope) => envelope.x)),
      right: Math.max(...columnEnvelopes.map((envelope) => envelope.x + envelope.width)),
      height: columnEnvelopes.reduce((sum, envelope) => sum + envelope.height, 0) + Math.max(0, columnEnvelopes.length - 1) * verticalLane,
    }] as const;
  }));
  const globalHeight = Math.max(...[...columnMetrics.values()].map((metric) => metric.height), 1);
  const levelX = new Map<number, number>();
  let horizontalCursor = 80;
  for (const currentLevel of [...orderedLevels.keys()].sort((left, right) => left - right)) {
    const metric = columnMetrics.get(currentLevel)!;
    levelX.set(currentLevel, horizontalCursor - metric.left);
    horizontalCursor += metric.right - metric.left + horizontalLane;
  }
  const latents = new Map<string, XYPosition>();
  const indicators = new Map<string, XYPosition[]>();
  for (const [currentLevel, columnNodes] of [...orderedLevels.entries()].sort(([a], [b]) => a - b)) {
    const columnHeight = columnMetrics.get(currentLevel)!.height;
    let verticalCursor = 80 + Math.max(0, (globalHeight - columnHeight) / 2);
    columnNodes.forEach((node, index) => {
      const envelope = envelopes.get(node.id)!;
      const position = {
        x: levelX.get(currentLevel)!,
        y: verticalCursor - envelope.y,
      };
      latents.set(node.id, position);
      indicators.set(node.id, envelope.indicatorPositions.map((point) => ({ x: position.x + point.x, y: position.y + point.y })));
      verticalCursor += envelope.height + verticalLane;
    });
  }
  return { latents, indicators };
}

function orderSmartplsLevels(
  byLevel: Map<number, Array<Node<ConstructData>>>,
  shape: ReturnType<typeof structuralShapeMaps>,
) {
  const levels = [...byLevel.keys()].sort((a, b) => a - b);
  let ordered = new Map(levels.map((level) => [
    level,
    [...(byLevel.get(level) ?? [])].sort((left, right) => left.position.y - right.position.y || left.id.localeCompare(right.id)),
  ]));

  for (let sweep = 0; sweep < 4; sweep += 1) {
    ordered = sweepSmartplsLevels(ordered, levels, shape.parents, "parents");
    ordered = sweepSmartplsLevels(ordered, [...levels].reverse(), shape.children, "children");
  }

  return ordered;
}

function sweepSmartplsLevels(
  ordered: Map<number, Array<Node<ConstructData>>>,
  levels: number[],
  neighbors: Map<string, string[]>,
  relation: "parents" | "children",
) {
  const next = new Map(ordered);
  for (const level of levels) {
    const levelNodes = next.get(level) ?? [];
    const neighborLevel = relation === "parents" ? level - 1 : level + 1;
    const neighborOrder = new Map((next.get(neighborLevel) ?? []).map((node, index) => [node.id, index]));
    if (neighborOrder.size === 0) continue;
    next.set(level, [...levelNodes].sort((left, right) => {
      const leftScore = smartplsBarycenter(left, neighbors, neighborOrder);
      const rightScore = smartplsBarycenter(right, neighbors, neighborOrder);
      return leftScore - rightScore || left.position.y - right.position.y || left.id.localeCompare(right.id);
    }));
  }
  return next;
}

function smartplsBarycenter(node: Node<ConstructData>, neighbors: Map<string, string[]>, neighborOrder: Map<string, number>) {
  const indexes = (neighbors.get(node.id) ?? [])
    .map((id) => neighborOrder.get(id))
    .filter((index): index is number => typeof index === "number");
  if (indexes.length === 0) return node.position.y / SMARTPLS_VERTICAL_GAP;
  return indexes.reduce((sum, index) => sum + index, 0) / indexes.length;
}

export function layoutSmartplsModel(modelNodes: Array<Node<ConstructData>>, modelEdges: Edge[], layout?: DiagramLayoutState): Array<Node<ConstructData>> {
  const hiddenInteractions = hiddenInteractionNodeIds(modelNodes, modelEdges);
  const visibleNodes = modelNodes.filter((node) => !hiddenInteractions.has(node.id));
  const movableEnvelopeNodes = visibleNodes.map((node) => ({
    ...node,
    data: {
      ...node.data,
      // Free indicators remain at persisted absolute Canvas coordinates when
      // constructs move; exclude them from construct-relative arrangement.
      indicators: node.data.indicators.filter((indicator) => {
        const item = layout?.indicatorLayouts[node.id]?.[indicator];
        return item?.side !== "free" || !Number.isFinite(item.x) || !Number.isFinite(item.y);
      }),
    },
  }));
  const structuralEdges = modelEdges.filter((edge) => edge.data?.role !== "covariance"
    && !hiddenInteractions.has(edge.source)
    && !hiddenInteractions.has(edge.target));
  const placement = smartplsLayout(movableEnvelopeNodes, structuralEdges, layout);
  return modelNodes.map((node) => ({ ...node, position: placement.latents.get(node.id) ?? node.position }));
}

function structuralShapeMaps(modelNodes: Array<Node<ConstructData>>, structuralEdges: Edge[]) {
  const nodeIds = new Set(modelNodes.map((node) => node.id));
  const incoming = new Map(modelNodes.map((node) => [node.id, 0]));
  const outgoing = new Map(modelNodes.map((node) => [node.id, 0]));
  const parents = new Map(modelNodes.map((node) => [node.id, [] as string[]]));
  const children = new Map(modelNodes.map((node) => [node.id, [] as string[]]));
  for (const edge of structuralEdges) {
    if (!nodeIds.has(edge.source) || !nodeIds.has(edge.target)) continue;
    outgoing.set(edge.source, (outgoing.get(edge.source) ?? 0) + 1);
    incoming.set(edge.target, (incoming.get(edge.target) ?? 0) + 1);
    parents.get(edge.target)?.push(edge.source);
    children.get(edge.source)?.push(edge.target);
  }
  return { incoming, outgoing, parents, children };
}

function indicatorSide(id: string, shape: ReturnType<typeof structuralShapeMaps>, finalLevel: boolean, columnIndex = 0, columnSize = 1): "left" | "right" | "top" | "bottom" {
  const incomingCount = shape.incoming.get(id) ?? 0;
  const outgoingCount = shape.outgoing.get(id) ?? 0;
  if (incomingCount === 0) return "left";
  if (finalLevel || outgoingCount === 0) return "right";
  if (columnSize === 1) return "top";
  if (columnIndex === 0) return "bottom";
  if (columnIndex === columnSize - 1) return "top";
  return columnIndex % 2 === 0 ? "top" : "bottom";
}

function smartplsIndicatorPositions(position: XYPosition, count: number, side: "left" | "right" | "top" | "bottom"): XYPosition[] {
  return smartIndicatorPosition(position, count, side);
}

function handleId(kind: "source" | "target", side: "left" | "right" | "top" | "bottom") {
  return `${kind}-${side}`;
}

function structuralRouting(edge: Edge, paperStyle: boolean, layout?: DiagramLayoutState): { routing: string; bendPoints?: XYPosition[] } {
  if (!paperStyle) return { routing: edge.type ?? "smoothstep" };
  const saved = layout?.edgeLayouts[edge.id];
  if (!saved?.pinned) return { routing: "straight" };
  if (saved.routing === "orthogonal") return { routing: "smoothstep" };
  if (saved.routing === "curved") return { routing: "default" };
  if (saved.routing === "polyline" && saved.bendPoints?.length) return { routing: "polyline", bendPoints: saved.bendPoints.map((point) => ({ ...point })) };
  return { routing: "straight" };
}

function moderationConnectorSides(
  source: XYPosition,
  target: XYPosition,
): { source: "left" | "right" | "top" | "bottom"; target: "left" | "right" | "top" | "bottom" } {
  const dx = target.x - source.x;
  const dy = target.y - source.y;
  if (Math.abs(dx) >= Math.abs(dy)) {
    return dx >= 0 ? { source: "right", target: "left" } : { source: "left", target: "right" };
  }
  return dy >= 0 ? { source: "bottom", target: "top" } : { source: "top", target: "bottom" };
}

function routeSides(sourceNode: Node, targetNode: Node): { source: "left" | "right" | "top" | "bottom"; target: "left" | "right" | "top" | "bottom" } {
  const route = routeBetweenBoxes(semNodeBox(sourceNode), semNodeBox(targetNode));
  return { source: route.source, target: route.target };
}

function indicatorPositionsForConstruct(
  node: Node<ConstructData>,
  position: XYPosition,
  paperStyle: boolean,
  shape: ReturnType<typeof structuralShapeMaps>,
  layout?: DiagramLayoutState,
): XYPosition[] {
  const defaults = paperStyle ? smartplsIndicatorPositions(position, node.data.indicators.length, indicatorSide(node.id, shape, false)) : indicatorPositions(position, node.data.indicators.length);
  const saved = layout?.indicatorLayouts[node.id];
  if (!saved) return defaults;
  const bySide = new Map<IndicatorSide, Array<{ indicator: string; index: number; layout: IndicatorLayout }>>();
  node.data.indicators.forEach((indicator, index) => {
    const current = saved[indicator];
    if (current?.side === "free" && typeof current.x === "number" && typeof current.y === "number") return;
    const side = current?.side && current.side !== "free" ? current.side : indicatorSide(node.id, shape, false);
    bySide.set(side, [...(bySide.get(side) ?? []), { indicator, index, layout: current ?? { side, order: index } }]);
  });
  const next = [...defaults];
  for (const [side, entries] of bySide) {
    const ordered = [...entries].sort((left, right) => (left.layout.order ?? left.index) - (right.layout.order ?? right.index) || left.indicator.localeCompare(right.indicator));
    const exactSide = side === "free" ? "left" : side;
    const generated = paperStyle
      ? smartplsIndicatorPositions(position, ordered.length, exactSide)
      : compactIndicatorPositions(position, ordered.length, exactSide);
    ordered.forEach((entry, sideIndex) => { next[entry.index] = generated[sideIndex]; });
  }
  node.data.indicators.forEach((indicator, index) => {
    const current = saved[indicator];
    if (current?.side === "free" && typeof current.x === "number" && typeof current.y === "number") {
      next[index] = { x: current.x, y: current.y };
    }
  });
  return next;
}

export function modelFingerprint(nodes: Array<Node<ConstructData>>, edges: Edge[]) {
  return JSON.stringify({
    nodes: nodes.map((node) => ({ id: node.id, indicators: [...node.data.indicators].sort() })).sort((a, b) => a.id.localeCompare(b.id)),
    edges: edges.filter((edge) => edge.data?.role !== "covariance").map((edge) => [edge.source, edge.target]).sort(),
  });
}

function resultMatchesModel(nodes: Array<Node<ConstructData>>, edges: Edge[], result: NonNullable<AnalysisRun["result"]>) {
  const nodeIds = new Set(nodes.map((node) => node.id));
  const currentPaths = new Set(edges.map((edge) => `${edge.source}\u0000${edge.target}`));
  const resultPaths = new Set(result.paths.map((path) => `${path.source}\u0000${path.target}`));
  if (currentPaths.size !== resultPaths.size || [...currentPaths].some((path) => !resultPaths.has(path))) return false;
  const generatedHigherOrderIndicators = new Map(nodes.flatMap((node) => {
    const declaration = node.data.semantic === "higher_order" ? node.data.higherOrder : undefined;
    if (!declaration) return [];
    const higherOrderIds = new Set([node.id, declaration.id]);
    return [[node.id, new Set([...higherOrderIds].flatMap((higherOrderId) => (
      declaration.components.map((componentId) => `__qpls_hoc_${higherOrderId}_${componentId}`)
    )))]] as const;
  }));
  const resultIndicators = new Map<string, Set<string>>();
  for (const estimate of result.outer_estimates) {
    if (!nodeIds.has(estimate.construct)) return false;
    if (generatedHigherOrderIndicators.get(estimate.construct)?.has(estimate.indicator)) continue;
    const indicators = resultIndicators.get(estimate.construct) ?? new Set<string>();
    indicators.add(estimate.indicator);
    resultIndicators.set(estimate.construct, indicators);
  }
  return nodes.every((node) => {
    const indicators = resultIndicators.get(node.id);
    if (!indicators) {
      return node.data.semantic === "higher_order" && node.data.indicators.length === 0;
    }
    return indicators.size === node.data.indicators.length
      && node.data.indicators.every((indicator) => indicators.has(indicator));
  });
}

export { measureDiagramQuality };
