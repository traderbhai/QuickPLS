import type { Edge, Node } from "@xyflow/react";
import type { AnalysisRun, ConstructData } from "../types";

const NODE_WIDTH = 170;
const NODE_HEIGHT = 118;
const PADDING = 54;

export function publicationDiagramSvg(nodes: Array<Node<ConstructData>>, edges: Edge[], run?: AnalysisRun): string {
  const bounds = diagramBounds(nodes);
  const result = run?.result;
  const pathCoefficients = new Map(result?.paths.map((path) => [`${path.source}\u0000${path.target}`, path.coefficient]) ?? []);
  const loadings = new Map<string, Map<string, number>>();
  for (const estimate of result?.outer_estimates ?? []) {
    const current = loadings.get(estimate.construct) ?? new Map<string, number>();
    current.set(estimate.indicator, estimate.loading);
    loadings.set(estimate.construct, current);
  }
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const edgeMarkup = edges.map((edge) => renderEdge(edge, byId, bounds, pathCoefficients)).join("\n");
  const nodeMarkup = nodes.map((node) => renderNode(node, bounds, result?.r_squared[node.id], loadings.get(node.id))).join("\n");
  const title = run ? `${run.name} publication diagram` : "QuickPLS model diagram";
  const warning = run?.result && run.result.method_version !== "pls_pm_v1"
    ? `<text x="${PADDING}" y="${bounds.height - 14}" class="warning">Validated for documented QuickPLS v0.9.0-rc.1 supported scope; unsupported shapes remain blocked.</text>`
    : "";
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${bounds.width}" height="${bounds.height}" viewBox="0 0 ${bounds.width} ${bounds.height}" role="img" aria-label="${escapeXml(title)}">
<style>
.bg{fill:#fbfdfd}.node{fill:#fff;stroke:#567078;stroke-width:1.4}.node-title{font:700 13px Arial,sans-serif;fill:#172126}.node-meta{font:10px Arial,sans-serif;fill:#53646b}.badge{fill:#e8f5e4;stroke:#7dbd54}.badge-text{font:700 10px Arial,sans-serif;fill:#32630d}.indicator{font:9px Arial,sans-serif;fill:#314148}.loading{font:700 9px Arial,sans-serif;fill:#0c7376}.edge{stroke:#51646c;stroke-width:1.6;fill:none}.edge-label{font:700 11px Arial,sans-serif;fill:#172126}.caption{font:11px Arial,sans-serif;fill:#526169}.warning{font:10px Arial,sans-serif;fill:#8a5a00}
</style>
<defs><marker id="arrow" markerWidth="9" markerHeight="9" refX="7" refY="3" orient="auto" markerUnits="strokeWidth"><path d="M0,0 L0,6 L7,3 z" fill="#51646c"/></marker></defs>
<rect class="bg" width="100%" height="100%"/>
<text x="${PADDING}" y="28" class="caption">${escapeXml(title)}</text>
${edgeMarkup}
${nodeMarkup}
${warning}
</svg>`;
}

function diagramBounds(nodes: Array<Node<ConstructData>>) {
  if (nodes.length === 0) return { minX: 0, minY: 0, width: 640, height: 420 };
  const minX = Math.min(...nodes.map((node) => node.position.x));
  const minY = Math.min(...nodes.map((node) => node.position.y));
  const maxX = Math.max(...nodes.map((node) => node.position.x + NODE_WIDTH));
  const maxY = Math.max(...nodes.map((node) => node.position.y + NODE_HEIGHT));
  return {
    minX,
    minY,
    width: Math.ceil(maxX - minX + PADDING * 2),
    height: Math.ceil(maxY - minY + PADDING * 2 + 22),
  };
}

function project(point: { x: number; y: number }, bounds: { minX: number; minY: number }) {
  return { x: point.x - bounds.minX + PADDING, y: point.y - bounds.minY + PADDING };
}

function renderEdge(edge: Edge, nodes: Map<string, Node<ConstructData>>, bounds: { minX: number; minY: number }, coefficients: Map<string, number>) {
  const source = nodes.get(edge.source);
  const target = nodes.get(edge.target);
  if (!source || !target) return "";
  const start = project({ x: source.position.x + NODE_WIDTH, y: source.position.y + NODE_HEIGHT / 2 }, bounds);
  const end = project({ x: target.position.x, y: target.position.y + NODE_HEIGHT / 2 }, bounds);
  const mid = { x: (start.x + end.x) / 2, y: (start.y + end.y) / 2 - 8 };
  const coefficient = coefficients.get(`${edge.source}\u0000${edge.target}`);
  const label = coefficient == null ? edge.data?.role === "control" ? "Control" : "Path" : `${edge.data?.role === "control" ? "C " : ""}${coefficient.toFixed(3)}`;
  return `<path class="edge" d="M${start.x},${start.y} L${end.x},${end.y}" marker-end="url(#arrow)"/>
<rect x="${mid.x - 29}" y="${mid.y - 13}" width="58" height="18" rx="3" fill="#fbfdfd" stroke="#d7e0e3"/>
<text x="${mid.x}" y="${mid.y}" text-anchor="middle" class="edge-label">${escapeXml(label)}</text>`;
}

function renderNode(node: Node<ConstructData>, bounds: { minX: number; minY: number }, rSquared: number | undefined, loadings: Map<string, number> | undefined) {
  const position = project(node.position, bounds);
  const badge = rSquared == null ? "Model" : `R2 ${rSquared.toFixed(3)}`;
  const indicators = node.data.indicators.slice(0, 5).map((indicator, index) => {
    const y = position.y + 72 + index * 13;
    const loading = loadings?.get(indicator);
    return `<text x="${position.x + 12}" y="${y}" class="indicator">${escapeXml(indicator)}</text>${loading == null ? "" : `<text x="${position.x + NODE_WIDTH - 12}" y="${y}" text-anchor="end" class="loading">${loading.toFixed(3)}</text>`}`;
  }).join("\n");
  const more = node.data.indicators.length > 5 ? `<text x="${position.x + 12}" y="${position.y + 72 + 5 * 13}" class="indicator">+${node.data.indicators.length - 5} indicators</text>` : "";
  return `<g>
<rect class="node" x="${position.x}" y="${position.y}" width="${NODE_WIDTH}" height="${NODE_HEIGHT}" rx="6"/>
<rect class="badge" x="${position.x + NODE_WIDTH - 62}" y="${position.y + 10}" width="50" height="24" rx="12"/>
<text x="${position.x + NODE_WIDTH - 37}" y="${position.y + 26}" text-anchor="middle" class="badge-text">${escapeXml(badge)}</text>
<text x="${position.x + 12}" y="${position.y + 24}" class="node-title">${escapeXml(node.data.label)}</text>
<text x="${position.x + 12}" y="${position.y + 42}" class="node-meta">[${escapeXml(node.data.shortName)}] ${node.data.mode === "reflective" ? "Mode A" : "Mode B"}</text>
${indicators}
${more}
</g>`;
}

function escapeXml(value: string) {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll("\"", "&quot;");
}
