import { CheckCircle2, Database, FileText, FlaskConical, FolderOpen, Keyboard, Network, Play, Plus, Save, ShieldCheck } from "lucide-react";
import { useWorkspace } from "../store";
import { Card, InlineNotice, MetricCard, PageHeader, Panel, WorkspacePage } from "./Ui";

const sampleProjects = [
  ["corporate_reputation", "Corporate reputation", "Four-construct PLS-SEM model with a completed bootstrap-backed run."],
  ["simple_pls", "Simple reflective PLS-SEM", "Dataset, reflective constructs, paths, run, and report."],
  ["mediation", "Mediation", "Direct, indirect, and total effects from a completed three-construct run."],
] as const;

export function OnboardingWorkspace() {
  const setView = useWorkspace((state) => state.setView);
  const setOnboardingState = useWorkspace((state) => state.setOnboardingState);
  const setShortcutOverlayOpen = useWorkspace((state) => state.setShortcutOverlayOpen);
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
  const recentProjects = onboarding.recentProjectCards.length
    ? onboarding.recentProjectCards.map((project, index) => ({
      name: project.split(/[\\/]/).pop() ?? project,
      path: project,
      lastOpened: index === 0 ? "Last used" : "Earlier",
      status: "Available",
    }))
    : [];
  const start = (view: Parameters<typeof setView>[0]) => {
    setOnboardingState({ dismissed: true });
    setView(view);
  };
  return <WorkspacePage className="home-v2-workspace home-v211-workspace home-v217-workspace home-v223-workspace" data-v217-mockup-screen="home" data-v223-project-manager="start-center">
    <PageHeader
      kicker="Project command center"
      title="Home"
      description="Continue a research project, import data, build the SEM diagram, run a supported analysis, and prepare outputs from one desktop workspace."
      actions={<>
        <button className="qpls2-secondary-action" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:open-project"))}><FolderOpen size={15} />Open project</button>
        <button className="qpls2-primary-action" onClick={() => start(nextStep.view)}><NextIcon size={16} />{nextStep.label}</button>
      </>}
    />
    <section className="home-v2-hero" aria-label="QuickPLS project launcher">
      <div className="home-v2-current">
        <span className="page-kicker">Current workspace</span>
        <h2>{projectPath ? projectPath.split(/[\\/]/).pop() : "Unsaved QuickPLS project"}</h2>
        <p>{projectPath ?? "Save the project to enable durable autosave, recovery, and repeatable project handoff."}</p>
        <div className="home-v2-action-row">
          <button className="qpls2-primary-action" onClick={() => start(nextStep.view)}><NextIcon size={16} />{nextStep.label}</button>
          <button className="qpls2-secondary-action" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:save-project"))}><Save size={15} />{projectPath ? "Save now" : "Save project"}</button>
          <button className="qpls2-secondary-action" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:open-project"))}><FolderOpen size={15} />Open project</button>
        </div>
      </div>
      <aside className="home-v2-next">
        <span>Recommended next step</span>
        <strong>{nextStep.label}</strong>
        <p>{nextStep.detail}</p>
        <small><ShieldCheck size={13} /> Offline desktop workflow. Review Method Details before reporting results.</small>
      </aside>
    </section>
    <Panel title="Project launcher" description="Common project actions stay visible in the first desktop viewport." className="home-v211-command-panel home-v217-launcher home-v223-start-center">
    <div className="home-v2-command-grid" aria-label="Primary project actions">
      <Card title="Start new model" description="Open the SEM designer and create constructs, indicators, and paths.">
        <button className="qpls2-primary-action" onClick={() => start("models")}><Plus size={16} />Build model</button>
      </Card>
      <Card title="Import dataset" description="Bring in raw data, covariance, or correlation inputs before calculation.">
        <button className="qpls2-secondary-action" onClick={() => start("data")}><Database size={16} />Open data</button>
      </Card>
      <Card title="Open demo project" description="Load a bundled workflow with data, model, saved run, and report surfaces.">
        <button className="qpls2-secondary-action" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:open-demo-project"))}><FlaskConical size={16} />Open demo</button>
      </Card>
      <Card title="Continue project" description={onboarding.recentProjectCards.length ? onboarding.recentProjectCards[0] : "Select a .qpls project from disk."}>
        <button className="qpls2-secondary-action" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:open-project"))}><FolderOpen size={16} />Open project</button>
      </Card>
    </div>
    </Panel>
    <section className="home-v223-manager-grid" aria-label="Project manager">
      <Panel title="Project summary" description="Current workspace state." className="home-v223-detail-panel">
        <div className="home-v211-metrics home-v217-summary" aria-label="Workspace facts">
          <MetricCard label="Rows" value={dataset.rowCount ?? dataset.rows.length} detail={dataset.columns.length ? dataset.name : "No data imported"} tone={dataset.columns.length ? "success" : "warning"} />
          <MetricCard label="Constructs" value={nodes.length} detail={nodes.length ? "SEM model available" : "Open Model to create constructs"} tone={nodes.length ? "success" : "info"} />
          <MetricCard label="Runs" value={runs.length} detail={runs.length ? "Ready for report setup" : "Run after setup is ready"} tone={runs.length ? "success" : "info"} />
        </div>
      </Panel>
      <Panel title="Recent projects" description="Desktop project list." actions={<button className="qpls2-secondary-action" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:open-project"))}><FolderOpen size={15} />Browse</button>} className="home-v223-detail-panel">
        <div className="home-v223-recent-list" role="list">
          {recentProjects.length ? recentProjects.map((project) => <button key={project.path} type="button" role="listitem" className="home-v223-recent-row" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:open-project"))}>
            <FileText size={15} />
            <span><strong>{project.name}</strong><small>{project.path}</small></span>
            <em>{project.lastOpened}</em>
          </button>) : <div className="home-v223-empty-row"><FileText size={15} /><span>No recent project recorded. Use Open Project to choose a .qpls file.</span></div>}
        </div>
      </Panel>
      <Panel title="Recovery and autosave" description="Desktop project recovery is checked when opening a saved .qpls project." className="home-v223-detail-panel">
        {projectPath ? <InlineNotice tone="info" title="Autosave enabled" action={<button className="qpls2-secondary-action" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:save-project"))}>Save now</button>}>
          This saved project can use the desktop autosave and backup recovery flow.
        </InlineNotice> : <div className="home-v223-empty-row"><Save size={15} /><span>Save the project to enable autosave and recovery checks.</span></div>}
      </Panel>
      <Panel title="Quick links" description="Reference surfaces without leaving the desktop workflow." className="home-v223-detail-panel">
        <div className="home-v223-link-list">
          <button type="button" onClick={() => start("trust")}><ShieldCheck size={15} />Trust Center</button>
          <button type="button" onClick={() => start("analyses")}><Network size={15} />Method guide</button>
          <button type="button" onClick={() => setShortcutOverlayOpen(true)}><Keyboard size={15} />Keyboard shortcuts</button>
        </div>
      </Panel>
    </section>
    <ol className="home-workflow-list home-v2-workflow" aria-label="QuickPLS workflow status">
      <li className={dataset.columns.length ? "complete" : "active"}><CheckCircle2 size={15} /><div><strong>Data</strong><span>{dataset.name}: {dataset.rowCount ?? dataset.rows.length} rows, {dataset.columns.length} variables</span></div><button className="secondary-button" onClick={() => start("data")}>Inspect</button></li>
      <li className={nodes.length ? "complete" : ""}><CheckCircle2 size={15} /><div><strong>Model</strong><span>{nodes.length} construct{nodes.length === 1 ? "" : "s"} available in the SEM designer</span></div><button className="secondary-button" onClick={() => start("models")}>Edit</button></li>
      <li className={runs.length ? "complete" : nodes.length ? "active" : ""}><CheckCircle2 size={15} /><div><strong>Setup and run</strong><span>{runs.length ? `${runs.length} completed run${runs.length === 1 ? "" : "s"}` : "Choose a method and launch when ready"}</span></div><button className="secondary-button" onClick={() => start(runs.length ? "runs" : "analyses")}>{runs.length ? "Results" : "Setup"}</button></li>
      <li className={runs.length ? "active" : ""}><CheckCircle2 size={15} /><div><strong>Report</strong><span>{runs.length ? "Diagram and tables are ready for report setup" : "Run a method to unlock exports"}</span></div><button className="secondary-button" onClick={() => start("reports")}>Prepare</button></li>
    </ol>
    <section className="sample-project-gallery home-v2-samples" aria-label="Sample project gallery">
      <header><div><strong>Sample project gallery</strong><span>Open one of three complete bundled workflows, then inspect Data, Model, Results, and Report.</span></div><button className="secondary-button" onClick={() => window.dispatchEvent(new CustomEvent("quickpls:open-demo-project", { detail: { sampleId: onboarding.selectedDemo } }))}><FlaskConical size={15} />Open selected sample</button></header>
      <div className="sample-project-grid">
        {sampleProjects.map(([id, label, detail]) => <button key={id} type="button" className={onboarding.selectedDemo === id ? "sample-project-card active" : "sample-project-card"} onClick={() => setOnboardingState({ selectedDemo: id })}>
          <strong>{label}</strong>
          <span>{detail}</span>
        </button>)}
      </div>
    </section>
    <InlineNotice
      tone="info"
      title="Start from dataset workflow"
      action={<button className="qpls2-secondary-action" onClick={() => start("data")}><Database size={15} />Start from dataset</button>}
    >
      Import data {"->"} inspect quality {"->"} detect prefixes {"->"} create constructs {"->"} arrange model {"->"} validate {"->"} run {"->"} inspect checklist {"->"} export report.
    </InlineNotice>
    <InlineNotice
      tone="warning"
      title="Reviewing a private dataset?"
      action={<button className="qpls2-secondary-action" onClick={() => start("trust")}><ShieldCheck size={15} />Open protocol</button>}
    >
      Use the real dataset review protocol before taking screenshots or notes. Keep raw files, private projects, and value-revealing exports out of the repository.
    </InlineNotice>
  </WorkspacePage>;
}
