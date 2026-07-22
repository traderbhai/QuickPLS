import { Database, FileText, FlaskConical, FolderOpen, Network, Play, Plus } from "lucide-react";
import { useWorkspace } from "../store";
import { Card, PageHeader, StatusBadge } from "./Ui";

export function OnboardingWorkspace() {
  const setView = useWorkspace((state) => state.setView);
  const setOnboardingState = useWorkspace((state) => state.setOnboardingState);
  const onboarding = useWorkspace((state) => state.onboardingState);
  const nodes = useWorkspace((state) => state.nodes);
  const dataset = useWorkspace((state) => state.dataset);
  const runs = useWorkspace((state) => state.runs);
  const start = (view: Parameters<typeof setView>[0]) => {
    setOnboardingState({ dismissed: true });
    setView(view);
  };
  return <section className="workspace-page onboarding-workspace">
    <PageHeader title="Start QuickPLS" description="Open a project, import data, build a SEM diagram, run a validated method, and export a publication-ready report." actions={<StatusBadge status="validated">desktop-first workflow</StatusBadge>} />
    <div className="onboarding-grid">
      <Card title="Start new project" description="Start from the current workspace and build a diagram.">
        <button className="run-button" onClick={() => start("models")}><Plus size={16} />Build model</button>
      </Card>
      <Card title="Import dataset" description="CSV, XLSX, SAV, covariance, and correlation imports are handled through the Data workspace.">
        <button className="secondary-button" onClick={() => start("data")}><Database size={16} />Open data</button>
      </Card>
      <Card title="Open demo project" description="Use the bundled corporate reputation fixture to see the full workflow.">
        <button className="secondary-button" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:open-demo-project"))}><FlaskConical size={16} />Open demo</button>
      </Card>
      <Card title="Continue recent project" description={onboarding.recentProjectCards.length ? onboarding.recentProjectCards[0] : "Use Open in the top bar to select a .qpls project."}>
        <button className="secondary-button" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:open-project"))}><FolderOpen size={16} />Open existing project</button>
      </Card>
    </div>
    <div className="workflow-cards">
      <Card title="1. Data" description={`${dataset.name}: ${dataset.rowCount ?? dataset.rows.length} rows, ${dataset.columns.length} variables`}><button className="secondary-button" onClick={() => start("data")}><Database size={15} />Inspect columns</button></Card>
      <Card title="2. Model" description={`${nodes.length} constructs are available in the SEM designer.`}><button className="secondary-button" onClick={() => start("models")}><Network size={15} />Edit diagram</button></Card>
      <Card title="3. Run" description="Check method readiness before launching the offline engine."><button className="secondary-button" onClick={() => start("analyses")}><Play size={15} />Setup analysis</button></Card>
      <Card title="4. Report" description={runs.length ? `${runs.length} saved run(s) ready for reports.` : "Run a method to unlock exports."}><button className="secondary-button" onClick={() => start("reports")}><FileText size={15} />Prepare report</button></Card>
    </div>
  </section>;
}
