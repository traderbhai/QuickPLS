import { AnalysisCatalog } from "./components/AnalysisCatalog";
import { DataWorkspace } from "./components/DataWorkspace";
import { Explorer } from "./components/Explorer";
import { GroupsWorkspace } from "./components/GroupsWorkspace";
import { Inspector } from "./components/Inspector";
import { ModelCanvas } from "./components/ModelCanvas";
import { NavRail } from "./components/NavRail";
import { ReportsWorkspace } from "./components/ReportsWorkspace";
import { RunHistory } from "./components/RunHistory";
import { StatusBar } from "./components/StatusBar";
import { TopBar } from "./components/TopBar";
import { useWorkspace } from "./store";
import { useEffect } from "react";
import { autosaveNativeProject, isNativeDesktop } from "./services/projectService";

export function App() {
  const view = useWorkspace((state) => state.view);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const runs = useWorkspace((state) => state.runs);
  const analysisSettings = useWorkspace((state) => state.analysisSettings);
  const dataset = useWorkspace((state) => state.dataset);
  const projectPath = useWorkspace((state) => state.projectPath);
  useEffect(() => {
    if (!projectPath || !isNativeDesktop()) return;
    const timer = window.setTimeout(() => { void autosaveNativeProject(projectPath, { nodes, edges, runs, analysisSettings, activeDatasetId: dataset.id }).catch(() => undefined); }, 5000);
    return () => window.clearTimeout(timer);
  }, [projectPath, nodes, edges, runs, analysisSettings, dataset]);
  return <div className="app-shell">
    <TopBar />
    <div className="workspace-shell">
      <NavRail />
      {view === "models" ? <><Explorer /><ModelCanvas /><Inspector /></> : <div className="page-host">{view === "data" ? <DataWorkspace /> : view === "analyses" ? <AnalysisCatalog /> : view === "runs" ? <RunHistory /> : view === "groups" ? <GroupsWorkspace /> : <ReportsWorkspace />}</div>}
    </div>
    <StatusBar />
  </div>;
}
