import { AnalysisCatalog } from "./components/AnalysisCatalog";
import { DataWorkspace } from "./components/DataWorkspace";
import { Explorer } from "./components/Explorer";
import { GroupsWorkspace } from "./components/GroupsWorkspace";
import { Inspector } from "./components/Inspector";
import { ModelCanvas } from "./components/ModelCanvas";
import { NavRail } from "./components/NavRail";
import { ReportsWorkspace } from "./components/ReportsWorkspace";
import { RunHistory } from "./components/RunHistory";
import { RunWorkspace } from "./components/RunWorkspace";
import { StatusBar } from "./components/StatusBar";
import { TopBar } from "./components/TopBar";
import { WorkflowStrip } from "./components/WorkflowStrip";
import { completedSamplePlsRun } from "./data/smokeRun";
import { sampleDataset } from "./data/sample";
import { useWorkspace } from "./store";
import { useEffect } from "react";
import { autosaveNativeProject, isNativeDesktop } from "./services/projectService";
import type { ConstructData, WorkspaceView } from "./types";
import type { Edge, Node } from "@xyflow/react";

declare global {
  interface Window {
    __QUICKPLS_SMOKE__?: {
      addCompletedRun: () => void;
      loadDiagramFixture: (fixture: string) => void;
      setView: (nextView: string) => void;
    };
  }
}

function diagramFixture(name: string): { nodes: Array<Node<ConstructData>>; edges: Edge[] } {
  const count = name === "large" ? 20 : name === "medium" ? 8 : 5;
  const indicatorsPerConstruct = name === "large" ? 4 : name === "medium" ? 4 : 3;
  const nodes = Array.from({ length: count }, (_, index): Node<ConstructData> => {
    const level = index < Math.ceil(count / 3) ? 0 : index < Math.ceil((count * 2) / 3) ? 1 : 2;
    const withinLevel = level === 0 ? index : level === 1 ? index - Math.ceil(count / 3) : index - Math.ceil((count * 2) / 3);
    const id = `v111_c${index + 1}`;
    return {
      id,
      type: "construct",
      position: { x: 90 + level * 280, y: 80 + withinLevel * 118 },
      data: {
        label: `Construct ${index + 1}`,
        shortName: `C${index + 1}`,
        mode: name === "formative" && index % 2 === 0 ? "formative" : "reflective",
        indicators: Array.from({ length: indicatorsPerConstruct }, (_, item) => `C${index + 1}I${item + 1}`),
      },
    };
  });
  const edges: Edge[] = [];
  const firstColumn = nodes.slice(0, Math.ceil(count / 3));
  const middleColumn = nodes.slice(Math.ceil(count / 3), Math.ceil((count * 2) / 3));
  const finalColumn = nodes.slice(Math.ceil((count * 2) / 3));
  firstColumn.forEach((source, index) => {
    const target = middleColumn[index % Math.max(1, middleColumn.length)];
    if (target) edges.push({ id: `${source.id}-${target.id}`, source: source.id, target: target.id, label: "Path", type: "smoothstep" });
  });
  middleColumn.forEach((source, index) => {
    const target = finalColumn[(finalColumn.length - 1 - index + finalColumn.length) % Math.max(1, finalColumn.length)];
    if (target) edges.push({ id: `${source.id}-${target.id}`, source: source.id, target: target.id, label: "Path", type: "smoothstep" });
  });
  if (name === "mediation" && nodes.length >= 3) {
    return {
      nodes: nodes.slice(0, 3).map((node, index) => ({ ...node, position: { x: 100 + index * 280, y: 210 } })),
      edges: [
        { id: "v111_c1-v111_c2", source: "v111_c1", target: "v111_c2", label: "Path", type: "smoothstep" },
        { id: "v111_c2-v111_c3", source: "v111_c2", target: "v111_c3", label: "Path", type: "smoothstep" },
        { id: "v111_c1-v111_c3", source: "v111_c1", target: "v111_c3", label: "Path", type: "smoothstep" },
      ],
    };
  }
  return { nodes, edges };
}

export function App() {
  const view = useWorkspace((state) => state.view);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const runs = useWorkspace((state) => state.runs);
  const analysisSettings = useWorkspace((state) => state.analysisSettings);
  const diagramMode = useWorkspace((state) => state.diagramMode);
  const diagramOverlaySettings = useWorkspace((state) => state.diagramOverlaySettings);
  const publicationDiagramSettings = useWorkspace((state) => state.publicationDiagramSettings);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const dataset = useWorkspace((state) => state.dataset);
  const projectPath = useWorkspace((state) => state.projectPath);
  useEffect(() => {
    if (!new URLSearchParams(window.location.search).has("quickpls_smoke")) return;
    const smokeApi = {
      addCompletedRun: () => useWorkspace.getState().addRun(completedSamplePlsRun()),
      loadDiagramFixture: (fixture: string) => {
        const { nodes, edges } = diagramFixture(fixture);
        useWorkspace.getState().loadProject({
          nodes,
          edges,
          dataset: {
            ...sampleDataset,
            id: `v111-${fixture}`,
            name: `v1.1.1 ${fixture} fixture`,
            columns: nodes.flatMap((node) => node.data.indicators),
          },
          runs: [],
          diagramMode: "sem",
        });
      },
      setView: (nextView: string) => {
        if (["data", "models", "analyses", "run", "runs", "groups", "reports"].includes(nextView)) {
          useWorkspace.getState().setView(nextView as WorkspaceView);
        }
      },
    };
    window.__QUICKPLS_SMOKE__ = smokeApi;
    return () => { delete window.__QUICKPLS_SMOKE__; };
  }, []);
  useEffect(() => {
    if (!projectPath || !isNativeDesktop()) return;
    const timer = window.setTimeout(() => { void autosaveNativeProject(projectPath, { nodes, edges, runs, analysisSettings, diagramMode, diagramOverlaySettings, publicationDiagramSettings, diagramLayout, activeDatasetId: dataset.id }).catch(() => undefined); }, 5000);
    return () => window.clearTimeout(timer);
  }, [projectPath, nodes, edges, runs, analysisSettings, diagramMode, diagramOverlaySettings, publicationDiagramSettings, diagramLayout, dataset]);
  return <div className="app-shell">
    <TopBar />
    <div className="workspace-shell">
      <NavRail />
      {view === "models" ? <><Explorer /><ModelCanvas /><Inspector /></> : <div className="page-host"><WorkflowStrip />{view === "data" ? <DataWorkspace /> : view === "analyses" ? <AnalysisCatalog /> : view === "run" ? <RunWorkspace /> : view === "runs" ? <RunHistory /> : view === "groups" ? <GroupsWorkspace /> : <ReportsWorkspace />}</div>}
    </div>
    <StatusBar />
  </div>;
}
