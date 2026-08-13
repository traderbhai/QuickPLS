import { CheckCircle2, CircleAlert, ClipboardCheck, MonitorCog, MousePointer2, SlidersHorizontal } from "lucide-react";
import { useEffect } from "react";
import { useWorkspace } from "../store";
import type { UiPreferences, WorkspaceView } from "../types";
import { Card, CommandGroup, InlineNotice, MetricCard, PageHeader, Panel, StatusBadge, ToolbarButton, WorkspacePage } from "./Ui";

const resetUiPreferences: UiPreferences = {
  density: "compact" as const,
  tableDensity: "compact" as const,
  defaultPrecision: 4,
  showAdvancedHelp: true,
  recentPanels: ["models", "runs", "reports"],
  methodScopeDrawerOpen: false,
  showThresholdColors: true,
  focusDiagramMode: false,
  selectedExportPreset: "journal_figure" as const,
};

const workspaceViews = new Set<WorkspaceView>(["welcome", "data", "models", "analyses", "run", "runs", "groups", "reports", "trust", "settings"]);
const exportPresets = new Set<UiPreferences["selectedExportPreset"]>(["journal_figure", "journal_tables", "thesis_appendix", "reviewer_pack", "full_reproducibility_report"]);

function parseImportedPreferences(value: unknown): Partial<UiPreferences> {
  if (!value || typeof value !== "object") return {};
  const record = value as Record<string, unknown>;
  const next: Partial<UiPreferences> = {};
  if (record.density === "compact" || record.density === "comfortable") next.density = record.density;
  if (record.tableDensity === "compact" || record.tableDensity === "comfortable") next.tableDensity = record.tableDensity;
  if (typeof record.defaultPrecision === "number" && Number.isFinite(record.defaultPrecision)) {
    next.defaultPrecision = Math.min(6, Math.max(2, Math.round(record.defaultPrecision)));
  }
  if (typeof record.showAdvancedHelp === "boolean") next.showAdvancedHelp = record.showAdvancedHelp;
  if (Array.isArray(record.recentPanels)) {
    const panels = record.recentPanels.filter((panel): panel is WorkspaceView => typeof panel === "string" && workspaceViews.has(panel as WorkspaceView));
    if (panels.length > 0) next.recentPanels = panels;
  }
  if (typeof record.methodScopeDrawerOpen === "boolean") next.methodScopeDrawerOpen = record.methodScopeDrawerOpen;
  if (typeof record.showThresholdColors === "boolean") next.showThresholdColors = record.showThresholdColors;
  if (typeof record.focusDiagramMode === "boolean") next.focusDiagramMode = record.focusDiagramMode;
  if (typeof record.selectedExportPreset === "string" && exportPresets.has(record.selectedExportPreset as UiPreferences["selectedExportPreset"])) {
    next.selectedExportPreset = record.selectedExportPreset as UiPreferences["selectedExportPreset"];
  }
  return next;
}

export function SettingsWorkspace() {
  const uiPreferences = useWorkspace((state) => state.uiPreferences);
  const setUiPreferences = useWorkspace((state) => state.setUiPreferences);

  useEffect(() => {
    const apply = () => window.dispatchEvent(new CustomEvent("quickpls:status-message", { detail: { message: "Settings applied locally.", tone: "success" } }));
    const reset = () => {
      setUiPreferences(resetUiPreferences);
      window.dispatchEvent(new CustomEvent("quickpls:status-message", { detail: { message: "Interface preferences reset to QuickPLS defaults.", tone: "success" } }));
    };
    const exportPreferences = () => {
      const url = URL.createObjectURL(new Blob([JSON.stringify(uiPreferences, null, 2)], { type: "application/json" }));
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = "quickpls-ui-preferences.json";
      anchor.click();
      setTimeout(() => URL.revokeObjectURL(url), 0);
    };
    const importPreferences = () => {
      const input = document.createElement("input");
      input.type = "file";
      input.accept = "application/json,.json";
      input.onchange = async () => {
        const file = input.files?.[0];
        if (!file) return;
        try {
          const imported = parseImportedPreferences(JSON.parse(await file.text()));
          if (Object.keys(imported).length === 0) {
            throw new Error("No supported QuickPLS UI preference keys were found.");
          }
          setUiPreferences(imported);
          window.dispatchEvent(new CustomEvent("quickpls:status-message", { detail: { message: `Imported interface preferences from ${file.name}.`, tone: "success" } }));
        } catch (error) {
          window.dispatchEvent(new CustomEvent("quickpls:status-message", { detail: { message: error instanceof Error ? error.message : "Preference import failed.", tone: "error" } }));
        }
      };
      input.click();
    };
    window.addEventListener("quickpls:settings-apply", apply);
    window.addEventListener("quickpls:settings-ok", apply);
    window.addEventListener("quickpls:settings-cancel", apply);
    window.addEventListener("quickpls:settings-reset", reset);
    window.addEventListener("quickpls:settings-import", importPreferences);
    window.addEventListener("quickpls:settings-export", exportPreferences);
    return () => {
      window.removeEventListener("quickpls:settings-apply", apply);
      window.removeEventListener("quickpls:settings-ok", apply);
      window.removeEventListener("quickpls:settings-cancel", apply);
      window.removeEventListener("quickpls:settings-reset", reset);
      window.removeEventListener("quickpls:settings-import", importPreferences);
      window.removeEventListener("quickpls:settings-export", exportPreferences);
    };
  }, [setUiPreferences, uiPreferences]);

  return <WorkspacePage className="settings-workspace settings-v2-workspace settings-v214-workspace settings-v219-workspace" data-v219-mockup-screen="settings">
    <PageHeader
      title="Settings"
      description="Tune desktop density, table behavior, result precision, threshold colors, and v2 workspace preferences."
      kicker="QuickPLS 2.0"
      actions={<StatusBadge status="info">Local preferences</StatusBadge>}
    />

    <Panel
      title="Desktop experience"
      description="These settings affect only the interface, not statistical results or project fingerprints."
      actions={<MonitorCog size={18} aria-hidden="true" />}
    >
      <div className="settings-grid qpls2-settings-grid">
        <label>
          App density
          <select value={uiPreferences.density} onChange={(event) => setUiPreferences({ density: event.target.value === "comfortable" ? "comfortable" : "compact" })}>
            <option value="compact">Compact desktop</option>
            <option value="comfortable">Comfortable</option>
          </select>
        </label>
        <label>
          Table density
          <select value={uiPreferences.tableDensity} onChange={(event) => setUiPreferences({ tableDensity: event.target.value === "comfortable" ? "comfortable" : "compact" })}>
            <option value="compact">Compact tables</option>
            <option value="comfortable">Comfortable tables</option>
          </select>
        </label>
        <label>
          Default precision
          <input type="number" min={2} max={6} value={uiPreferences.defaultPrecision} onChange={(event) => setUiPreferences({ defaultPrecision: Number(event.target.value) })} />
        </label>
        <label className="checkbox-row">
          <input type="checkbox" checked={uiPreferences.showThresholdColors} onChange={(event) => setUiPreferences({ showThresholdColors: event.target.checked })} />
          Show threshold guidance colors
        </label>
      </div>
    </Panel>

    <Panel
      title="Design system foundation"
      description="Reusable primitives now define the v2 shell, panels, cards, notices, status chips, command groups, metric cards, and toolbar buttons."
      actions={<StatusBadge status="validated">Mockup contract</StatusBadge>}
      className="settings-v2-design-system"
    >
      <div className="qpls2-design-system-grid">
        <MetricCard label="Control height" value="34 px" detail="Consistent button, select, and input sizing." tone="info" />
        <MetricCard label="Panel radius" value="4 px" detail="Low-radius desktop surfaces." tone="success" />
        <MetricCard label="Page gutter" value="28 px" detail="Shared workspace alignment." tone="info" />
        <MetricCard label="Status language" value="Scoped" detail="Validated, Experimental, Unsupported, Needs setup." tone="success" />
      </div>

      <div className="qpls2-design-samples">
        <InlineNotice
          tone="success"
          title="Validated scope wording"
          action={<StatusBadge status="validated">Validated scope</StatusBadge>}
        >
          Method status copy stays bounded to documented QuickPLS scope.
        </InlineNotice>
        <InlineNotice
          tone="warning"
          title="Disabled reasons stay local"
          action={<ToolbarButton disabled reason="Import a raw dataset before running.">Run selected method</ToolbarButton>}
        >
          Primary disabled actions must explain the exact blocker beside the control.
        </InlineNotice>
      </div>

      <div className="qpls2-design-toolbar-demo">
        <CommandGroup label="Canvas">
          <ToolbarButton active><MousePointer2 size={15} /> Select</ToolbarButton>
          <ToolbarButton><SlidersHorizontal size={15} /> Arrange</ToolbarButton>
        </CommandGroup>
        <CommandGroup label="Status">
          <ToolbarButton><CheckCircle2 size={15} /> Validated scope</ToolbarButton>
          <ToolbarButton><CircleAlert size={15} /> Needs setup</ToolbarButton>
        </CommandGroup>
      </div>
    </Panel>

    <Panel
      title="Real dataset review"
      description="Private researcher datasets should be reviewed through the v2.12 protocol, not checked into the repository."
      actions={<ClipboardCheck size={18} aria-hidden="true" />}
      className="settings-v2-real-dataset-review"
      data-real-dataset-protocol-entrypoint="settings"
    >
      <div className="qpls2-design-system-grid">
        <MetricCard label="Raw private data" value="Never commit" detail="Use local-only review and dataset aliases." tone="warning" />
        <MetricCard label="Screenshots" value="Redact first" detail="Do not store value-revealing screenshots." tone="warning" />
        <MetricCard label="Issue notes" value="Anonymized" detail="Use the issue-register template for product feedback." tone="success" />
        <MetricCard label="Automated gates" value="Fixtures only" detail="Bundled and generated data remain the gate inputs." tone="info" />
      </div>
    </Panel>

    <div className="qpls2-hero-grid">
      <Card title="Offline first" description="QuickPLS does not require account login, activation, telemetry, cloud sync, or remote computation." tone="validated" />
      <Card title="Versioned artifacts" description="Each desktop build creates a fresh installer, portable executable, and checksum file." />
      <Card title="Numerical boundary" description="Settings here do not change formulas, estimators, validation tolerances, or serialized result values." />
    </div>
  </WorkspacePage>;
}
