import type { Edge, Node } from "@xyflow/react";
import { buildDiagramGraph } from "./diagramGraph";
import type { AnalysisRun, ConstructData, DiagramLayoutState, PublicationDiagramSettings } from "../types";

const PADDING = 42;
const FALLBACK_SETTINGS: PublicationDiagramSettings = {
  mode: "smartpls_result",
  precision: 3,
  overlayMode: "paths_r2",
  aspectRatio: "wide",
  palette: "grayscale",
  layoutSource: "current_canvas",
  showLoadings: true,
  showPathCoefficients: true,
  showRSquared: true,
  showValidationWatermark: true,
  showUnsupportedWarning: true,
  showRunProvenance: true,
};
const QUICKPLS_SIZE = { latentWidth: 150, latentHeight: 110, indicatorWidth: 96, indicatorHeight: 34 };
const SMARTPLS_SIZE = { latentWidth: 88, latentHeight: 82, ellipseWidth: 88, ellipseHeight: 58, indicatorWidth: 78, indicatorHeight: 24 };

export function publicationDiagramSvg(nodes: Array<Node<ConstructData>>, edges: Edge[], run?: AnalysisRun, settings: Partial<PublicationDiagramSettings> = {}, layout?: DiagramLayoutState): string {
  const options = normalizeSettings(settings);
  const graph = buildDiagramGraph(nodes, edges, options.mode, options.overlayMode, run, { layout, layoutSource: options.layoutSource });
  const smartpls = options.mode === "smartpls_result";
  const bounds = diagramBounds(graph.nodes, smartpls);
  const width = Math.max(smartpls ? 720 : 640, bounds.width + PADDING * 2);
  const height = Math.max(smartpls ? 430 : 420, bounds.height + PADDING * 2 + 34);
  const palette = paletteClass(options.palette, smartpls);
  const nodeMarkup = graph.nodes.map((node) => node.type === "indicator" ? renderIndicator(node, bounds, options.precision, smartpls) : renderLatent(node, bounds, smartpls, options)).join("\n");
  const edgeMarkup = graph.edges.map((edge) => renderEdge(edge, graph.nodes, bounds, smartpls, options)).join("\n");
  const title = run ? `${run.name} publication diagram` : "QuickPLS model diagram";
  const warning = run && options.showValidationWatermark
    ? `<text x="${PADDING}" y="${height - 15}" class="warning">Validated for documented QuickPLS v1.0.0 supported scope; unsupported shapes remain blocked.</text>`
    : "";
  const provenance = run && options.showRunProvenance
    ? `<text x="${width - PADDING}" y="${height - 15}" text-anchor="end" class="caption">Run ${escapeXml(run.id)} | ${escapeXml(run.createdAt)}</text>`
    : "";
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" role="img" aria-label="${escapeXml(title)}" class="${palette}">
<defs>
<marker id="arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0,0 L8,4 L0,8 z" class="arrow"/></marker>
<marker id="arrow-start" markerWidth="8" markerHeight="8" refX="1" refY="4" orient="auto"><path d="M8,0 L0,4 L8,8 z" class="arrow"/></marker>
<style>
.bg{fill:#fff}.caption{font:10px Arial,sans-serif;fill:#526169}.warning{font:10px Arial,sans-serif;fill:#666}
.latent{fill:#fff;stroke:#2d777a;stroke-width:1.8}.latent.formative{stroke:#a16d0b}.latent-title{font:700 12px Arial,sans-serif;fill:#172126}.latent-meta{font:10px Arial,sans-serif;fill:#56656b}.r2{font:700 10px Arial,sans-serif;fill:#32630d}.indicator{fill:#fff8df;stroke:#c49116;stroke-width:1.2}.indicator.formative{fill:#eaf8f8;stroke:#0b8f92}.indicator-text{font:700 9px Arial,sans-serif;fill:#253137}.indicator-stat{font:700 8px Arial,sans-serif;fill:#32630d}
.smartpls-latent{fill:#7d7d7d;stroke:#777;stroke-width:1}.smartpls-latent-label{font:10px Arial,sans-serif;fill:#222}.smartpls-r2{font:700 10px Arial,sans-serif;fill:#fff}.smartpls-indicator{fill:#eee;stroke:#d2d2d2;stroke-width:1}.smartpls-indicator-text{font:700 9px Arial,sans-serif;fill:#222}
.edge{stroke:#465961;stroke-width:1.45;fill:none}.measurement{stroke:#9c7a20;stroke-width:1.05;fill:none}.measurement.formative{stroke:#0c777b}.covariance{stroke:#8d5798;stroke-width:1.25;stroke-dasharray:4 3;fill:none}.edge-label{font:700 10px Arial,sans-serif;fill:#172126}.label-bg{fill:#fff;stroke:#dce2e5}.arrow{fill:#465961}
.smartpls .edge,.smartpls .measurement{stroke:#333;stroke-width:1.05}.smartpls .edge-label{font:500 9px Arial,sans-serif;fill:#222}.smartpls .label-bg{fill:#fff;stroke:none;fill-opacity:.85}.smartpls .arrow{fill:#333}
.mono .latent,.mono .indicator{stroke:#333}.mono .indicator{fill:#fff}.mono .edge,.mono .measurement,.mono .covariance{stroke:#333}.mono .arrow{fill:#333}.high-contrast .smartpls-latent{fill:#222;stroke:#111}.high-contrast .smartpls-indicator{fill:#fff;stroke:#111}.quickpls-color .smartpls-latent{fill:#4f9fa2;stroke:#1f6e72}.quickpls-color .smartpls-indicator{fill:#fff8df;stroke:#c49116}
</style>
</defs>
<rect class="bg" width="100%" height="100%"/>
<text x="${PADDING}" y="25" class="caption">${escapeXml(title)}</text>
${edgeMarkup}
${nodeMarkup}
${warning}
${provenance}
</svg>`;
}

function normalizeSettings(settings: Partial<PublicationDiagramSettings>): PublicationDiagramSettings {
  const palette = settings.palette === "monochrome" ? "grayscale" : settings.palette ?? FALLBACK_SETTINGS.palette;
  return {
    ...FALLBACK_SETTINGS,
    ...settings,
    palette,
    mode: settings.mode ?? FALLBACK_SETTINGS.mode,
    precision: Math.min(6, Math.max(0, Math.trunc(settings.precision ?? FALLBACK_SETTINGS.precision))),
  };
}

function paletteClass(palette: PublicationDiagramSettings["palette"], smartpls: boolean) {
  if (smartpls) {
    if (palette === "high_contrast") return "smartpls high-contrast";
    if (palette === "quickpls_color" || palette === "color") return "smartpls quickpls-color";
    return "smartpls mono";
  }
  return palette === "monochrome" || palette === "grayscale" ? "mono" : "color";
}

function diagramBounds(nodes: Array<Node>, smartpls: boolean) {
  if (nodes.length === 0) return { minX: 0, minY: 0, width: 640, height: 420 };
  const dimensions = (node: Node) => node.type === "indicator"
    ? (smartpls ? { width: SMARTPLS_SIZE.indicatorWidth, height: SMARTPLS_SIZE.indicatorHeight } : { width: QUICKPLS_SIZE.indicatorWidth, height: QUICKPLS_SIZE.indicatorHeight })
    : (smartpls ? { width: SMARTPLS_SIZE.latentWidth, height: SMARTPLS_SIZE.latentHeight } : { width: QUICKPLS_SIZE.latentWidth, height: QUICKPLS_SIZE.latentHeight });
  const minX = Math.min(...nodes.map((node) => node.position.x));
  const minY = Math.min(...nodes.map((node) => node.position.y));
  const maxX = Math.max(...nodes.map((node) => node.position.x + dimensions(node).width));
  const maxY = Math.max(...nodes.map((node) => node.position.y + dimensions(node).height));
  return { minX, minY, width: maxX - minX, height: maxY - minY };
}

function project(position: { x: number; y: number }, bounds: { minX: number; minY: number }) {
  return { x: position.x - bounds.minX + PADDING, y: position.y - bounds.minY + PADDING };
}

function renderLatent(node: Node, bounds: { minX: number; minY: number }, smartpls: boolean, options: PublicationDiagramSettings) {
  const position = project(node.position, bounds);
  const data = node.data as Record<string, unknown>;
  const label = String(data.label ?? node.id);
  if (smartpls) {
    const r2 = options.showRSquared && typeof data.resultR2 === "number" ? `<text x="${position.x + 44}" y="${position.y + 33}" text-anchor="middle" class="smartpls-r2">R&#178; ${data.resultR2.toFixed(options.precision)}</text>` : "";
    return `<ellipse class="smartpls-latent" cx="${position.x + 44}" cy="${position.y + 29}" rx="44" ry="29"/>
${r2}
<text x="${position.x + 44}" y="${position.y + 74}" text-anchor="middle" class="smartpls-latent-label">${escapeXml(label)}</text>`;
  }
  const shortName = String(data.shortName ?? node.id);
  const mode = data.mode === "formative" ? "formative" : "reflective";
  const r2 = options.showRSquared && typeof data.resultR2 === "number" ? `<text x="${position.x + 75}" y="${position.y + 83}" text-anchor="middle" class="r2">R&#178; ${data.resultR2.toFixed(options.precision)}</text>` : "";
  return `<ellipse class="latent ${mode}" cx="${position.x + 75}" cy="${position.y + 55}" rx="74" ry="54"/>
<text x="${position.x + 75}" y="${position.y + 49}" text-anchor="middle" class="latent-title">${escapeXml(label)}</text>
<text x="${position.x + 75}" y="${position.y + 65}" text-anchor="middle" class="latent-meta">[${escapeXml(shortName)}]</text>
${r2}`;
}

function renderIndicator(node: Node, bounds: { minX: number; minY: number }, precision: number, smartpls: boolean) {
  const position = project(node.position, bounds);
  const data = node.data as Record<string, unknown>;
  const indicator = String(data.indicator ?? node.id);
  if (smartpls) {
    return `<rect class="smartpls-indicator" x="${position.x}" y="${position.y}" width="78" height="24"/>
<text x="${position.x + 39}" y="${position.y + 16}" text-anchor="middle" class="smartpls-indicator-text">${escapeXml(indicator)}</text>`;
  }
  const mode = data.mode === "formative" ? "formative" : "reflective";
  const value = typeof data.loading === "number" ? data.loading : typeof data.weight === "number" ? data.weight : null;
  const stat = value === null ? "" : `<text x="${position.x + 88}" y="${position.y + 23}" text-anchor="end" class="indicator-stat">${value.toFixed(precision)}</text>`;
  return `<rect class="indicator ${mode}" x="${position.x}" y="${position.y}" width="96" height="34" rx="4"/>
<text x="${position.x + 8}" y="${position.y + 21}" class="indicator-text">${escapeXml(indicator)}</text>
${stat}`;
}

function renderEdge(edge: Edge, nodes: Array<Node>, bounds: { minX: number; minY: number }, smartpls: boolean, options: PublicationDiagramSettings) {
  const source = nodes.find((node) => node.id === edge.source);
  const target = nodes.find((node) => node.id === edge.target);
  if (!source || !target) return "";
  const start = anchor(source, target, bounds, smartpls);
  const end = anchor(target, source, bounds, smartpls);
  const rawLabel = typeof edge.label === "string" ? edge.label : "";
  const measurement = String(edge.className ?? "").includes("measurement-edge");
  const structural = String(edge.className ?? "").includes("structural-edge");
  const label = measurement && !options.showLoadings ? "" : structural && !options.showPathCoefficients ? "" : rawLabel;
  const className = measurement ? `measurement ${String(edge.className).includes("formative") ? "formative" : ""}`
    : String(edge.className ?? "").includes("covariance-edge") ? "covariance"
      : "edge";
  const marker = className === "covariance" ? `marker-start="url(#arrow-start)" marker-end="url(#arrow)"` : `marker-end="url(#arrow)"`;
  const offset = edge.data?.labelOffset && typeof edge.data.labelOffset === "object"
    ? edge.data.labelOffset as { x?: number; y?: number }
    : {};
  const mid = {
    x: (start.x + end.x) / 2 + Number(offset.x ?? 0),
    y: (start.y + end.y) / 2 - (smartpls ? 2 : 5) + Number(offset.y ?? 0),
  };
  const d = className === "covariance"
    ? `M${start.x},${start.y} Q${mid.x},${mid.y - 45} ${end.x},${end.y}`
    : `M${start.x},${start.y} L${end.x},${end.y}`;
  const labelWidth = Math.max(30, label.length * (smartpls ? 5 : 6) + 10);
  const labelMarkup = label ? `<rect class="label-bg" x="${mid.x - labelWidth / 2}" y="${mid.y - 12}" width="${labelWidth}" height="15" rx="2"/><text x="${mid.x}" y="${mid.y}" text-anchor="middle" class="edge-label">${escapeXml(label)}</text>` : "";
  return `<path class="${className}" d="${d}" ${marker}/>
${labelMarkup}`;
}

function anchor(node: Node, other: Node, bounds: { minX: number; minY: number }, smartpls: boolean) {
  const size = node.type === "indicator"
    ? (smartpls ? { width: SMARTPLS_SIZE.indicatorWidth, height: SMARTPLS_SIZE.indicatorHeight } : { width: QUICKPLS_SIZE.indicatorWidth, height: QUICKPLS_SIZE.indicatorHeight })
    : (smartpls ? { width: SMARTPLS_SIZE.ellipseWidth, height: SMARTPLS_SIZE.ellipseHeight } : { width: QUICKPLS_SIZE.latentWidth, height: QUICKPLS_SIZE.latentHeight });
  const offsetY = smartpls && node.type !== "indicator" ? 0 : 0;
  const position = project(node.position, bounds);
  const center = { x: position.x + size.width / 2, y: position.y + offsetY + size.height / 2 };
  const otherSize = other.type === "indicator"
    ? (smartpls ? { width: SMARTPLS_SIZE.indicatorWidth, height: SMARTPLS_SIZE.indicatorHeight } : { width: QUICKPLS_SIZE.indicatorWidth, height: QUICKPLS_SIZE.indicatorHeight })
    : (smartpls ? { width: SMARTPLS_SIZE.ellipseWidth, height: SMARTPLS_SIZE.ellipseHeight } : { width: QUICKPLS_SIZE.latentWidth, height: QUICKPLS_SIZE.latentHeight });
  const otherPosition = project(other.position, bounds);
  const otherCenter = { x: otherPosition.x + otherSize.width / 2, y: otherPosition.y + otherSize.height / 2 };
  const dx = otherCenter.x - center.x;
  const dy = otherCenter.y - center.y;
  if (Math.abs(dx) > Math.abs(dy)) return { x: center.x + Math.sign(dx) * size.width / 2, y: center.y };
  return { x: center.x, y: center.y + Math.sign(dy) * size.height / 2 };
}

function escapeXml(value: string) {
  return value.replace(/[&<>"']/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;", "'": "&apos;" }[character]!));
}
