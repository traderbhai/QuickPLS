import { MarkerType, type Edge, type Node, type XYPosition } from "@xyflow/react";
import type { AnalysisRun, ConstructData, DiagramLayoutState, DiagramMode, DiagramOverlayMode, IndicatorLayout, IndicatorSide } from "../types";

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
}

export interface DiagramGraph {
  nodes: Array<Node<LatentNodeData | IndicatorNodeData>>;
  edges: Edge[];
  compatible: boolean;
  diagnostic: string | null;
}

export interface DiagramGraphOptions {
  layout?: DiagramLayoutState;
  layoutSource?: "current_canvas" | "tidy_publication";
}

const LATENT_WIDTH = 150;
const LATENT_HEIGHT = 110;
const INDICATOR_WIDTH = 96;
const INDICATOR_HEIGHT = 34;
const MEASUREMENT_GAP = 88;
const INDICATOR_ROW_GAP = 42;
const SMARTPLS_LATENT_WIDTH = 88;
const SMARTPLS_LATENT_HEIGHT = 58;
const SMARTPLS_INDICATOR_WIDTH = 78;
const SMARTPLS_INDICATOR_HEIGHT = 24;
const SMARTPLS_COLUMN_GAP = 230;
const SMARTPLS_VERTICAL_GAP = 112;
const SMARTPLS_INDICATOR_GAP = 30;
const SMARTPLS_MEASUREMENT_GAP = 70;

export const isIndicatorNodeId = (id: string) => id.startsWith("indicator::");
export const indicatorNodeId = (constructId: string, indicator: string) => `indicator::${constructId}::${encodeURIComponent(indicator)}`;
export const parseIndicatorNodeId = (id: string) => {
  const [, constructId, encoded] = id.split("::");
  if (!constructId || !encoded) return null;
  return { constructId, indicator: decodeURIComponent(encoded) };
};

export function buildDiagramGraph(
  modelNodes: Array<Node<ConstructData>>,
  modelEdges: Edge[],
  mode: DiagramMode,
  overlayMode: DiagramOverlayMode,
  run?: AnalysisRun,
  options: DiagramGraphOptions = {},
): DiagramGraph {
  const structuralEdges = modelEdges.filter((edge) => edge.data?.role !== "covariance");
  const covarianceEdges = modelEdges.filter((edge) => edge.data?.role === "covariance");
  const paperStyle = mode === "sem" || mode === "publication" || mode === "smartpls_result";
  const lockedResultMode = mode === "smartpls_result";
  const smartplsPlacement = (mode === "publication" || mode === "smartpls_result") && options.layoutSource !== "current_canvas" ? smartplsLayout(modelNodes, structuralEdges) : null;
  const structuralShape = structuralShapeMaps(modelNodes, structuralEdges);
  const result = run?.status === "completed" ? run.result : undefined;
  const compatible = result ? resultMatchesModel(modelNodes, structuralEdges, result) : true;
  const resultForOverlay = result && compatible ? result : undefined;
  const loadingByConstruct = new Map<string, Map<string, { loading: number; weight: number }>>();
  for (const estimate of resultForOverlay?.outer_estimates ?? []) {
    const current = loadingByConstruct.get(estimate.construct) ?? new Map<string, { loading: number; weight: number }>();
    current.set(estimate.indicator, { loading: estimate.loading, weight: estimate.weight });
    loadingByConstruct.set(estimate.construct, current);
  }
  const pathCoefficients = new Map((resultForOverlay?.paths ?? []).map((path) => [`${path.source}\u0000${path.target}`, path.coefficient]));
  const visualNodes: DiagramGraph["nodes"] = modelNodes.map((node) => {
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
    return {
      ...edge,
      type: paperStyle ? "straight" : edge.type ?? "smoothstep",
      label: resultForOverlay && coefficient !== undefined && (paperStyle || overlayMode === "paths_r2" || overlayMode === "significance")
        ? coefficient.toFixed(3)
        : paperStyle ? ""
          : edge.data?.role === "control" ? "Control" : edge.label ?? "Path",
      markerEnd: { type: MarkerType.ArrowClosed, width: paperStyle ? 10 : 16, height: paperStyle ? 10 : 16 },
      className: edge.data?.role === "control" ? "control-edge" : paperStyle ? "smartpls-structural-edge structural-edge" : "structural-edge",
      selectable: !lockedResultMode,
    };
  });

  if (mode !== "compact") {
    for (const node of modelNodes) {
      const latentPosition = visualNodes.find((visualNode) => visualNode.id === node.id)?.position ?? node.position;
      const placement = smartplsPlacement?.indicators.get(node.id)
        ?? indicatorPositionsForConstruct(node, latentPosition, paperStyle, structuralShape, options.layout);
      node.data.indicators.forEach((indicator, index) => {
        const estimate = loadingByConstruct.get(node.id)?.get(indicator);
        visualNodes.push({
          id: indicatorNodeId(node.id, indicator),
          type: "indicator",
          position: placement[index] ?? latentPosition,
          draggable: !lockedResultMode,
          selectable: true,
          data: { constructId: node.id, indicator, mode: node.data.mode, displayMode: mode, loading: estimate?.loading, weight: estimate?.weight },
        });
        const reflective = node.data.mode === "reflective";
        visualEdges.push({
          id: `measurement::${node.id}::${indicator}`,
          source: reflective ? node.id : indicatorNodeId(node.id, indicator),
          target: reflective ? indicatorNodeId(node.id, indicator) : node.id,
          type: "straight",
          label: resultForOverlay && (paperStyle || overlayMode === "loadings")
            ? (reflective ? estimate?.loading : estimate?.weight)?.toFixed(3) ?? ""
            : paperStyle ? ""
              : reflective ? "loading" : "weight",
          markerEnd: { type: MarkerType.ArrowClosed, width: paperStyle ? 8 : 14, height: paperStyle ? 8 : 14 },
          className: reflective ? `${paperStyle ? "smartpls-measurement-edge " : ""}measurement-edge reflective` : `${paperStyle ? "smartpls-measurement-edge " : ""}measurement-edge formative`,
          selectable: false,
          data: { visualOnly: true },
        });
      });
    }
    for (const edge of covarianceEdges) {
      visualEdges.push({
        ...edge,
        type: "default",
        label: edge.label ?? "Covariance",
        markerStart: { type: MarkerType.ArrowClosed, width: 12, height: 12 },
        markerEnd: { type: MarkerType.ArrowClosed, width: 12, height: 12 },
        className: "covariance-edge",
      });
    }
  }

  return {
    nodes: visualNodes,
    edges: visualEdges,
    compatible,
    diagnostic: result && !compatible
      ? "Selected run does not match the current model. Numeric overlays are hidden."
      : lockedResultMode && !resultForOverlay
        ? "Run or select a compatible result to show estimates."
        : null,
  };
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

export function defaultDiagramLayout(modelNodes: Array<Node<ConstructData>>, modelEdges: Edge[], existing?: Partial<DiagramLayoutState>): DiagramLayoutState {
  const structuralEdges = modelEdges.filter((edge) => edge.data?.role !== "covariance");
  const shape = structuralShapeMaps(modelNodes, structuralEdges);
  const constructLayouts: DiagramLayoutState["constructLayouts"] = {};
  const indicatorLayouts: DiagramLayoutState["indicatorLayouts"] = {};
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
        ? { side: previous.side, x: previous.x, y: previous.y, order: previous.order ?? index, pinned: previous.pinned }
        : { side: indicatorSide(node.id, shape, false), order: index };
    });
    indicatorLayouts[node.id] = currentIndicators;
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
    diagramViewport: existing?.diagramViewport,
    diagramTheme: existing?.diagramTheme === "quickpls_color" || existing?.diagramTheme === "high_contrast" ? existing.diagramTheme : "academic_grayscale",
  };
}

function smartplsLayout(modelNodes: Array<Node<ConstructData>>, structuralEdges: Edge[]) {
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
  for (const nodes of byLevel.values()) nodes.sort((left, right) => left.position.y - right.position.y || left.id.localeCompare(right.id));

  const latents = new Map<string, XYPosition>();
  const indicators = new Map<string, XYPosition[]>();
  const maxStack = Math.max(...[...byLevel.values()].map((nodes) => nodes.length), 1);
  const canvasTop = 80;
  const maxLevel = Math.max(...level.values(), 0);
  for (const [currentLevel, columnNodes] of byLevel) {
    const columnHeight = (columnNodes.length - 1) * SMARTPLS_VERTICAL_GAP;
    const globalHeight = (maxStack - 1) * SMARTPLS_VERTICAL_GAP;
    const startY = canvasTop + Math.max(0, (globalHeight - columnHeight) / 2);
    columnNodes.forEach((node, index) => {
      const position = {
        x: 170 + currentLevel * SMARTPLS_COLUMN_GAP,
        y: startY + index * SMARTPLS_VERTICAL_GAP,
      };
      latents.set(node.id, position);
      indicators.set(node.id, smartplsIndicatorPositions(position, node.data.indicators.length, indicatorSide(node.id, shape, currentLevel === maxLevel)));
    });
  }
  return { latents, indicators };
}

export function layoutSmartplsModel(modelNodes: Array<Node<ConstructData>>, modelEdges: Edge[]): Array<Node<ConstructData>> {
  const structuralEdges = modelEdges.filter((edge) => edge.data?.role !== "covariance");
  const placement = smartplsLayout(modelNodes, structuralEdges);
  return modelNodes.map((node) => ({ ...node, position: placement.latents.get(node.id) ?? node.position }));
}

function structuralShapeMaps(modelNodes: Array<Node<ConstructData>>, structuralEdges: Edge[]) {
  const nodeIds = new Set(modelNodes.map((node) => node.id));
  const incoming = new Map(modelNodes.map((node) => [node.id, 0]));
  const outgoing = new Map(modelNodes.map((node) => [node.id, 0]));
  const parents = new Map(modelNodes.map((node) => [node.id, [] as string[]]));
  for (const edge of structuralEdges) {
    if (!nodeIds.has(edge.source) || !nodeIds.has(edge.target)) continue;
    outgoing.set(edge.source, (outgoing.get(edge.source) ?? 0) + 1);
    incoming.set(edge.target, (incoming.get(edge.target) ?? 0) + 1);
    parents.get(edge.target)?.push(edge.source);
  }
  return { incoming, outgoing, parents };
}

function indicatorSide(id: string, shape: ReturnType<typeof structuralShapeMaps>, finalLevel: boolean): "left" | "right" | "top" | "bottom" {
  const incomingCount = shape.incoming.get(id) ?? 0;
  const outgoingCount = shape.outgoing.get(id) ?? 0;
  if (incomingCount === 0) return "left";
  if (finalLevel || outgoingCount === 0) return "right";
  return "bottom";
}

function smartplsIndicatorPositions(position: XYPosition, count: number, side: "left" | "right" | "top" | "bottom"): XYPosition[] {
  if (count === 0) return [];
  if (side === "top" || side === "bottom") {
    const stackWidth = Math.max(0, count - 1) * (SMARTPLS_INDICATOR_WIDTH + 8);
    const y = side === "top"
      ? position.y - SMARTPLS_MEASUREMENT_GAP
      : position.y + SMARTPLS_LATENT_HEIGHT + 52;
    return Array.from({ length: count }, (_, index) => ({
      x: position.x + SMARTPLS_LATENT_WIDTH / 2 - SMARTPLS_INDICATOR_WIDTH / 2 - stackWidth / 2 + index * (SMARTPLS_INDICATOR_WIDTH + 8),
      y,
    }));
  }
  const stackHeight = Math.max(0, count - 1) * SMARTPLS_INDICATOR_GAP;
  const x = side === "left"
    ? position.x - SMARTPLS_MEASUREMENT_GAP - SMARTPLS_INDICATOR_WIDTH
    : position.x + SMARTPLS_LATENT_WIDTH + SMARTPLS_MEASUREMENT_GAP;
  const centerY = position.y + SMARTPLS_LATENT_HEIGHT / 2 - SMARTPLS_INDICATOR_HEIGHT / 2;
  return Array.from({ length: count }, (_, index) => ({ x, y: centerY - stackHeight / 2 + index * SMARTPLS_INDICATOR_GAP }));
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
    const generated = paperStyle ? smartplsIndicatorPositions(position, ordered.length, side === "free" ? "left" : side) : indicatorPositions(position, ordered.length);
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
  const resultIndicators = new Map<string, Set<string>>();
  for (const estimate of result.outer_estimates) {
    if (!nodeIds.has(estimate.construct)) return false;
    const indicators = resultIndicators.get(estimate.construct) ?? new Set<string>();
    indicators.add(estimate.indicator);
    resultIndicators.set(estimate.construct, indicators);
  }
  return nodes.every((node) => {
    const indicators = resultIndicators.get(node.id);
    return indicators !== undefined
      && indicators.size === node.data.indicators.length
      && node.data.indicators.every((indicator) => indicators.has(indicator));
  });
}
