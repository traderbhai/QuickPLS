import { CheckCircle2, CircleAlert, ClipboardCheck, FileArchive, MonitorCog, MousePointer2, SlidersHorizontal } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import {
  cancelNativeDiagnosticBundlePreview,
  isNativeDesktop,
  previewNativeDiagnosticBundle,
  saveNativeDiagnosticBundle,
  type DiagnosticBundlePreview,
  type DiagnosticStagedContents,
} from "../services/projectService";
import { useWorkspace } from "../store";
import type { UiPreferences, WorkspaceView } from "../types";
import { InternalProjectArchiveV6SessionPanel } from "./InternalProjectArchiveV6SessionPanel";
import { Card, CommandGroup, InlineNotice, MetricCard, PageHeader, Panel, StatusBadge, ToolbarButton, WorkspacePage } from "./Ui";

const resetUiPreferences: UiPreferences = {
  density: "compact" as const,
  tableDensity: "compact" as const,
  defaultPrecision: 4,
  showAdvancedHelp: true,
  experimentalLabsEnabled: false,
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
  if (typeof record.experimentalLabsEnabled === "boolean") next.experimentalLabsEnabled = record.experimentalLabsEnabled;
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

export function DiagnosticStagedContentsPreview({ contents }: { contents: DiagnosticStagedContents }) {
  const systemRows: Array<[string, string]> = [
    ["Metadata schema", String(contents.system.schemaVersion)],
    ["QuickPLS version", contents.system.quickplsVersion],
    ["Release channel", contents.system.releaseChannel],
    ["Source revision", contents.system.sourceRevision],
    ["Operating system", contents.system.osFamily],
    ["Architecture", contents.system.architecture],
    ["Desktop runtime", contents.system.desktopRuntime],
    ["Locale", contents.system.locale],
    ["WebView2 version", contents.system.webview2Version],
    ["User data included", contents.system.userDataIncluded ? "yes" : "no"],
    ["Network accessed", contents.system.networkAccessed ? "yes" : "no"],
  ];
  return <section aria-label="Redacted staged diagnostic contents">
    <h3>Redacted staged contents</h3>
    <p>These are the exact metadata fields, event rows, and payload descriptors staged for the ZIP.</p>

    <h4>System and build metadata</h4>
    <dl>
      {systemRows.map(([label, value]) => <div key={label}><dt>{label}</dt><dd>{value}</dd></div>)}
    </dl>

    <div role="region" aria-label="Redacted diagnostic event rows" tabIndex={0}>
      <table>
        <caption>Redacted diagnostic event rows</caption>
        <thead><tr><th scope="col">Sequence</th><th scope="col">Timestamp</th><th scope="col">Severity</th><th scope="col">Stable code</th></tr></thead>
        <tbody>{contents.events.map((event) => <tr key={`${event.sequence}-${event.code}`}>
          <td>{event.sequence}</td><td>{event.timestamp}</td><td>{event.severity}</td><td>{event.code}</td>
        </tr>)}</tbody>
      </table>
    </div>

    <div role="region" aria-label="Diagnostic manifest payload descriptors" tabIndex={0}>
      <table>
        <caption>Manifest payload descriptors</caption>
        <thead><tr><th scope="col">Entry</th><th scope="col">Bytes</th><th scope="col">SHA-256</th></tr></thead>
        <tbody>{contents.manifest.entries.map((entry) => <tr key={entry.name}>
          <td>{entry.name}</td><td>{entry.bytes}</td><td>{entry.sha256}</td>
        </tr>)}</tbody>
      </table>
    </div>
    <p>
      Manifest schema {contents.manifest.schemaVersion}; created {contents.manifest.createdAt}; policy {contents.manifest.policyVersion}; {contents.manifest.archiveLimits.compression} ZIP;
      maximum {contents.manifest.archiveLimits.maximumArchiveBytes} archive bytes;
      local only: {contents.manifest.localOnly ? "yes" : "no"}; network accessed: {contents.manifest.networkAccessed ? "yes" : "no"}.
    </p>
  </section>;
}

export function DiagnosticBundlePanel() {
  const nativeDesktop = isNativeDesktop();
  const [diagnosticPreview, setDiagnosticPreview] = useState<DiagnosticBundlePreview | null>(null);
  const diagnosticPreviewIdRef = useRef<string | null>(null);
  const [diagnosticStatus, setDiagnosticStatus] = useState<"idle" | "previewing" | "ready" | "saving" | "saved" | "cancelled" | "error">("idle");
  const [diagnosticMessage, setDiagnosticMessage] = useState("No diagnostic bundle has been staged.");

  const previewDiagnostics = async () => {
    if (!nativeDesktop) return;
    setDiagnosticStatus("previewing");
    setDiagnosticMessage("Preparing a redacted local preview...");
    try {
      const preview = await previewNativeDiagnosticBundle(diagnosticPreview?.previewId ?? null);
      diagnosticPreviewIdRef.current = preview.previewId;
      setDiagnosticPreview(preview);
      setDiagnosticStatus("ready");
      setDiagnosticMessage("Preview ready. Review the included and excluded categories before saving.");
    } catch {
      setDiagnosticPreview(null);
      setDiagnosticStatus("error");
      setDiagnosticMessage("Diagnostic preview failed. Try again before choosing a destination.");
    }
  };

  const cancelDiagnosticPreview = async () => {
    const previewId = diagnosticPreview?.previewId;
    diagnosticPreviewIdRef.current = null;
    setDiagnosticPreview(null);
    setDiagnosticStatus("cancelled");
    setDiagnosticMessage("Diagnostic preview cancelled. No file was created.");
    if (!previewId) return;
    try {
      await cancelNativeDiagnosticBundlePreview(previewId);
    } catch {
      setDiagnosticStatus("error");
      setDiagnosticMessage("The preview was cleared locally, but the native diagnostic state could not be confirmed.");
    }
  };

  const saveDiagnostics = async () => {
    if (!diagnosticPreview) return;
    setDiagnosticStatus("saving");
    setDiagnosticMessage("Waiting for a new ZIP destination...");
    try {
      const saved = await saveNativeDiagnosticBundle(diagnosticPreview.previewId);
      diagnosticPreviewIdRef.current = null;
      setDiagnosticPreview(null);
      if (!saved) {
        setDiagnosticStatus("cancelled");
        setDiagnosticMessage("Save cancelled. No diagnostic ZIP was created.");
        return;
      }
      setDiagnosticStatus("saved");
      setDiagnosticMessage(`Diagnostic bundle saved locally (${Math.max(1, Math.ceil(saved.bytes / 1024))} KiB). QuickPLS did not upload it.`);
    } catch {
      diagnosticPreviewIdRef.current = null;
      setDiagnosticPreview(null);
      setDiagnosticStatus("error");
      setDiagnosticMessage("Diagnostic save failed. Create a fresh preview and choose another new ZIP destination.");
    }
  };

  useEffect(() => () => {
    const previewId = diagnosticPreviewIdRef.current;
    diagnosticPreviewIdRef.current = null;
    if (previewId) {
      void cancelNativeDiagnosticBundlePreview(previewId).catch(() => undefined);
    }
  }, []);

  return <div data-diagnostic-bundle-panel="live">
    <Panel
      title="Diagnostics and support"
      description="Create a redacted, local-only support ZIP. QuickPLS never uploads or attaches it automatically."
      actions={<FileArchive size={18} aria-hidden="true" />}
      className="settings-diagnostics"
    >
      {!nativeDesktop ? <InlineNotice tone="info" title="Native desktop required">
        Diagnostic preview and saving are available only in the installed QuickPLS desktop application.
      </InlineNotice> : null}

      {diagnosticPreview ? <>
        <div className="qpls2-design-system-grid">
          <MetricCard label="Archive entries" value={diagnosticPreview.entryCount} detail="Fixed allowlist; no arbitrary files." tone="info" />
          <MetricCard label="Session events" value={diagnosticPreview.eventCount} detail="Stable codes only; no project labels." tone="info" />
          <MetricCard
            label="Redactions"
            value={Object.values(diagnosticPreview.redactionCounts).reduce((total, count) => total + count, 0)}
            detail="Applied before this preview was produced."
            tone="success"
          />
          <MetricCard
            label="Estimated size"
            value={`${Math.max(1, Math.ceil(diagnosticPreview.estimatedUncompressedBytes / 1024))} KiB`}
            detail="Maximum uncompressed staging size."
            tone="info"
          />
        </div>
        <div className="qpls2-design-samples">
          <InlineNotice tone="success" title="Included in this preview">
            {diagnosticPreview.includedCategories.join("; ")}.
          </InlineNotice>
          <InlineNotice tone="warning" title="Always excluded">
            {diagnosticPreview.excludedCategories.join("; ")}.
          </InlineNotice>
        </div>
        <DiagnosticStagedContentsPreview contents={diagnosticPreview.stagedContents} />
      </> : null}

      <div className="qpls2-design-toolbar-demo">
        <CommandGroup label="Diagnostic bundle">
          <ToolbarButton
            type="button"
            onClick={previewDiagnostics}
            disabled={!nativeDesktop || diagnosticStatus === "previewing" || diagnosticStatus === "saving"}
            reason={!nativeDesktop ? "Open Settings in the native desktop application." : "A diagnostic action is already running."}
          >
            <FileArchive size={15} aria-hidden="true" /> {diagnosticPreview ? "Refresh preview" : "Preview bundle"}
          </ToolbarButton>
          <ToolbarButton
            type="button"
            onClick={saveDiagnostics}
            disabled={!diagnosticPreview || diagnosticStatus === "saving"}
            reason="Preview and review the redacted contents before saving."
          >
            Save new ZIP
          </ToolbarButton>
          <ToolbarButton
            type="button"
            onClick={cancelDiagnosticPreview}
            disabled={!diagnosticPreview || diagnosticStatus === "saving"}
            reason="There is no staged diagnostic preview to cancel."
          >
            Cancel preview
          </ToolbarButton>
        </CommandGroup>
      </div>

      <div
        role={diagnosticStatus === "error" ? "alert" : "status"}
        aria-live="polite"
        aria-atomic="true"
        className={`qpls2-inline-notice ${diagnosticStatus === "error" ? "danger" : diagnosticStatus === "saved" || diagnosticStatus === "ready" ? "success" : "info"}`}
      >
        <div><strong>Diagnostic status</strong><span>{diagnosticMessage}</span></div>
      </div>
    </Panel>
  </div>;
}

export function SettingsSchema6LabsSurface({ enabled }: { enabled: boolean }) {
  return <InternalProjectArchiveV6SessionPanel experimentalLabsEnabled={enabled} />;
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
        <label className="checkbox-row" aria-describedby="experimental-labs-description">
          <input
            type="checkbox"
            checked={uiPreferences.experimentalLabsEnabled}
            onChange={(event) => setUiPreferences({ experimentalLabsEnabled: event.target.checked })}
          />
          Enable Experimental Labs
        </label>
        <p id="experimental-labs-description" className="settings-field-description">
          Disabled by default. Labs methods remain separate from Standard analyses and product-parity claims.
        </p>
      </div>
    </Panel>

    <SettingsSchema6LabsSurface enabled={uiPreferences.experimentalLabsEnabled} />

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
        <MetricCard label="Availability language" value="Plain" detail="Supported, Experimental, Needs setup, Not available." tone="success" />
      </div>

      <div className="qpls2-design-samples">
        <InlineNotice
          tone="success"
          title="Supported setup wording"
          action={<StatusBadge status="validated">Supported setup</StatusBadge>}
        >
          Method copy states the concrete model and data requirements.
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
          <ToolbarButton><CheckCircle2 size={15} /> Supported setup</ToolbarButton>
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

    <DiagnosticBundlePanel />

    <div className="qpls2-hero-grid">
      <Card title="Offline first" description="QuickPLS does not require account login, activation, telemetry, cloud sync, or remote computation." tone="validated" />
      <Card title="Versioned artifacts" description="Each desktop build creates a fresh installer, portable executable, and checksum file." />
      <Card title="Numerical boundary" description="Settings here do not change formulas, estimators, numerical tolerances, or serialized result values." />
    </div>
  </WorkspacePage>;
}
