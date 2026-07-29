import { CheckCircle2, Database, FileText, FlaskConical, FolderOpen, Network, Play, Plus, Save } from "lucide-react";
import { useWorkspace } from "../store";
import { Card, PageHeader } from "./Ui";

const sampleProjects = [
  ["simple_pls", "Simple reflective PLS-SEM", "Dataset, reflective constructs, paths, run, and report."],
  ["mediation", "Mediation", "Indirect effects, total effects, and bootstrap-ready setup."],
  ["moderation", "Moderation", "Two-stage interaction workflow and simple-slope review."],
  ["formative", "Formative measurement", "Weights, formative VIF, and indicator placement."],
  ["plspredict", "PLSpredict", "Holdout / k-fold prediction and benchmark review."],
  ["micom_mga", "MICOM / MGA", "Two-group invariance and permutation group comparison."],
  ["cbsem_cfa", "CB-SEM CFA", "Reflective CFA/SEM ML validated bounded workflow."],
  ["regression", "Regression", "OLS/logistic/bounded PROCESS result workflow."],
  ["nca", "NCA", "CE-FDH/CR-FDH necessity analysis and bottleneck tables."],
] as const;

export function OnboardingWorkspace() {
  const setView = useWorkspace((state) => state.setView);
  const setOnboardingState = useWorkspace((state) => state.setOnboardingState);
  const onboarding = useWorkspace((state) => state.onboardingState);
  const nodes = useWorkspace((state) => state.nodes);
  const dataset = useWorkspace((state) => state.dataset);
  const runs = useWorkspace((state) => state.runs);
  const projectPath = useWorkspace((state) => state.projectPath);
  const nextStep = !dataset.columns.length
    ? { view: "data" as const, label: "Import data", detail: "Start by importing CSV, XLSX, SAV, covariance, or correlation data.", icon: Database }
    : !nodes.length
      ? { view: "models" as const, label: "Build model", detail: "Create constructs, assign indicators, and draw SEM paths.", icon: Network }
      : !runs.length
        ? { view: "analyses" as const, label: "Setup and run", detail: "Choose a method, verify readiness, and launch the offline engine.", icon: Play }
        : { view: "reports" as const, label: "Prepare report", detail: "Export the diagram, tables, provenance, and report notes.", icon: FileText };
  const NextIcon = nextStep.icon;
  const start = (view: Parameters<typeof setView>[0]) => {
    setOnboardingState({ dismissed: true });
    setView(view);
  };
  return <section className="workspace-page onboarding-workspace">
    <PageHeader title="Home" description="Open a project, continue the current workflow, or start from data, demo, or model design." />
    <section className="current-project-card" aria-label="Current project status">
      <div>
        <strong>{projectPath ? "Current project saved" : "Current workspace not saved yet"}</strong>
        <span>{projectPath ?? "Use Save in the top bar to enable autosave and recovery for this workspace."}</span>
      </div>
      <div className="current-project-actions">
        <button className="secondary-button" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:save-project"))}><Save size={15} />{projectPath ? "Save now" : "Save project"}</button>
        <button className="secondary-button" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:open-project"))}><FolderOpen size={15} />Open project</button>
      </div>
    </section>
    <section className="home-next-step" aria-label="Recommended next step">
      <div>
        <span>Recommended next step</span>
        <strong>{nextStep.label}</strong>
        <p>{nextStep.detail}</p>
      </div>
      <button className="run-button" onClick={() => start(nextStep.view)}><NextIcon size={16} />{nextStep.label}</button>
    </section>
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
    <ol className="home-workflow-list" aria-label="QuickPLS workflow status">
      <li className={dataset.columns.length ? "complete" : "active"}><CheckCircle2 size={15} /><div><strong>Data</strong><span>{dataset.name}: {dataset.rowCount ?? dataset.rows.length} rows, {dataset.columns.length} variables</span></div><button className="secondary-button" onClick={() => start("data")}>Inspect</button></li>
      <li className={nodes.length ? "complete" : ""}><CheckCircle2 size={15} /><div><strong>Model</strong><span>{nodes.length} construct{nodes.length === 1 ? "" : "s"} available in the SEM designer</span></div><button className="secondary-button" onClick={() => start("models")}>Edit</button></li>
      <li className={runs.length ? "complete" : nodes.length ? "active" : ""}><CheckCircle2 size={15} /><div><strong>Setup and run</strong><span>{runs.length ? `${runs.length} completed run${runs.length === 1 ? "" : "s"}` : "Choose a method and launch when ready"}</span></div><button className="secondary-button" onClick={() => start(runs.length ? "runs" : "analyses")}>{runs.length ? "Results" : "Setup"}</button></li>
      <li className={runs.length ? "active" : ""}><CheckCircle2 size={15} /><div><strong>Report</strong><span>{runs.length ? "Diagram and tables are ready for report setup" : "Run a method to unlock exports"}</span></div><button className="secondary-button" onClick={() => start("reports")}>Prepare</button></li>
    </ol>
    <section className="sample-project-gallery" aria-label="Sample project gallery">
      <header><div><strong>Sample project gallery</strong><span>Open a familiar research workflow, then inspect Data, Model, Results, and Report.</span></div><button className="secondary-button" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:open-demo-project"))}><FlaskConical size={15} />Open selected sample</button></header>
      <div className="sample-project-grid">
        {sampleProjects.map(([id, label, detail]) => <button key={id} type="button" className={onboarding.selectedDemo === id ? "sample-project-card active" : "sample-project-card"} onClick={() => setOnboardingState({ selectedDemo: id })}>
          <strong>{label}</strong>
          <span>{detail}</span>
        </button>)}
      </div>
    </section>
    <section className="guided-research-flow" aria-label="Guided start from dataset workflow">
      <strong>Start from dataset workflow</strong>
      <span>Import data {"->"} inspect quality {"->"} detect prefixes {"->"} create constructs {"->"} arrange model {"->"} validate {"->"} run {"->"} inspect checklist {"->"} export report.</span>
      <button className="secondary-button" onClick={() => start("data")}><Database size={15} />Start from dataset</button>
    </section>
  </section>;
}
