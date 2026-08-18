import { FileSearch, FolderOpen, LoaderCircle, LockKeyhole } from "lucide-react";
import { type FormEvent, useReducer } from "react";
import type {
  InternalProjectArchiveV6ReadCountsV1,
  InternalProjectArchiveV6ReadDiagnosticV1,
  InternalProjectArchiveV6ReadOutcomeV1,
  InternalProjectArchiveV6ReadSnapshotV1,
} from "../domain/internalProjectArchiveV6Read";
import {
  inspectInternalProjectArchiveV6At,
  openInternalProjectArchiveV6,
} from "../services/internalProjectArchiveV6ReadService";
import { isNativeDesktop } from "../services/projectService";
import { InlineNotice, MetricCard, Panel, StatusBadge, ToolbarButton } from "./Ui";

export interface InternalProjectArchiveV6InspectionSummary {
  access: "read_only";
  archivePath: string;
  archiveSha256: string;
  archiveBytes: number;
  projectName: string;
  projectId: string;
  createdAt: string;
  modifiedAt: string;
  engineVersion: string;
  counts: InternalProjectArchiveV6ReadCountsV1;
  sourceRecheckedUnchanged: true;
}

export type InternalProjectArchiveV6InspectorPhase = "idle" | "loading" | "ready" | "error";

export interface InternalProjectArchiveV6InspectorState {
  phase: InternalProjectArchiveV6InspectorPhase;
  archivePath: string;
  summary: InternalProjectArchiveV6InspectionSummary | null;
  failure: InternalProjectArchiveV6ReadDiagnosticV1 | null;
  statusMessage: string;
}

type InternalProjectArchiveV6InspectorAction =
  | { type: "path_changed"; archivePath: string }
  | { type: "started"; statusMessage: string }
  | { type: "cancelled" }
  | { type: "succeeded"; summary: InternalProjectArchiveV6InspectionSummary }
  | { type: "failed"; failure: InternalProjectArchiveV6ReadDiagnosticV1 };

export type InternalProjectArchiveV6InspectionTerminalAction = Extract<
  InternalProjectArchiveV6InspectorAction,
  { type: "cancelled" | "succeeded" | "failed" }
>;

export interface InternalProjectArchiveV6InspectorServices {
  chooseAndInspect: () => Promise<InternalProjectArchiveV6ReadOutcomeV1 | null>;
  inspectAt: (archivePath: string) => Promise<InternalProjectArchiveV6ReadOutcomeV1>;
}

const defaultServices: InternalProjectArchiveV6InspectorServices = {
  chooseAndInspect: openInternalProjectArchiveV6,
  inspectAt: inspectInternalProjectArchiveV6At,
};

export function createInternalProjectArchiveV6InspectorState(
  archivePath = "",
): InternalProjectArchiveV6InspectorState {
  return {
    phase: "idle",
    archivePath,
    summary: null,
    failure: null,
    statusMessage: "No archive has been inspected.",
  };
}

export function internalProjectArchiveV6InspectorReducer(
  state: InternalProjectArchiveV6InspectorState,
  action: InternalProjectArchiveV6InspectorAction,
): InternalProjectArchiveV6InspectorState {
  if (action.type === "path_changed") {
    return {
      phase: "idle",
      archivePath: action.archivePath,
      summary: null,
      failure: null,
      statusMessage: action.archivePath.trim()
        ? "Archive path provided. Inspect it to validate the schema-6 ZIP."
        : "No archive has been inspected.",
    };
  }
  if (action.type === "started") {
    return {
      ...state,
      phase: "loading",
      failure: null,
      statusMessage: action.statusMessage,
    };
  }
  if (action.type === "cancelled") {
    return {
      ...state,
      phase: state.summary ? "ready" : "idle",
      failure: null,
      statusMessage: state.summary
        ? "Archive selection cancelled. The previous read-only inspection remains shown."
        : "Archive selection cancelled. No archive was inspected.",
    };
  }
  if (action.type === "succeeded") {
    return {
      phase: "ready",
      archivePath: action.summary.archivePath,
      summary: action.summary,
      failure: null,
      statusMessage: "Strict schema-6 validation completed. The archive is available for read-only inspection.",
    };
  }
  return {
    phase: "error",
    archivePath: state.archivePath,
    summary: null,
    failure: action.failure,
    statusMessage: "Archive inspection failed.",
  };
}

function summarizeSnapshot(
  snapshot: InternalProjectArchiveV6ReadSnapshotV1,
): InternalProjectArchiveV6InspectionSummary {
  return {
    access: "read_only",
    archivePath: snapshot.archivePath,
    archiveSha256: snapshot.archiveSha256,
    archiveBytes: snapshot.archiveBytes,
    projectName: snapshot.manifest.name,
    projectId: snapshot.manifest.project_id,
    createdAt: snapshot.manifest.created_at,
    modifiedAt: snapshot.manifest.modified_at,
    engineVersion: snapshot.manifest.engine_version,
    counts: { ...snapshot.counts },
    sourceRecheckedUnchanged: true,
  };
}

function failureFromUnknown(error: unknown): InternalProjectArchiveV6ReadDiagnosticV1 {
  const record = error && typeof error === "object" ? error as Record<string, unknown> : null;
  return {
    code: record && typeof record.code === "string"
      ? record.code
      : "schema6_archive_read.ui_failed",
    message: error instanceof Error && error.message
      ? error.message
      : "QuickPLS could not inspect the selected archive.",
    correctiveAction: record && typeof record.correctiveAction === "string"
      ? record.correctiveAction
      : "Confirm this is a trusted schema-6 .qpls ZIP, then choose it again in the native desktop app.",
  };
}

export async function resolveInternalProjectArchiveV6Inspection(
  load: () => Promise<InternalProjectArchiveV6ReadOutcomeV1 | null>,
): Promise<InternalProjectArchiveV6InspectionTerminalAction> {
  try {
    const outcome = await load();
    if (outcome === null) return { type: "cancelled" };
    if (outcome.status === "blocked") {
      return { type: "failed", failure: outcome.diagnostic };
    }
    return { type: "succeeded", summary: summarizeSnapshot(outcome.value) };
  } catch (error) {
    return { type: "failed", failure: failureFromUnknown(error) };
  }
}

export interface InternalProjectArchiveV6InspectorViewProps {
  nativeDesktop: boolean;
  state: InternalProjectArchiveV6InspectorState;
  onArchivePathChange: (archivePath: string) => void;
  onBrowse: () => void;
  onInspect: () => void;
}

export function InternalProjectArchiveV6InspectorView({
  nativeDesktop,
  state,
  onArchivePathChange,
  onBrowse,
  onInspect,
}: InternalProjectArchiveV6InspectorViewProps) {
  const loading = state.phase === "loading";
  const controlsDisabled = loading || !nativeDesktop;
  const summary = state.summary;

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!controlsDisabled && state.archivePath.trim()) onInspect();
  };

  return <Panel
    title="Schema-6 archive inspection"
    description="Internal/Labs inspection of one local .qpls ZIP through the strict native reader."
    actions={<StatusBadge status="experimental">Labs · read-only</StatusBadge>}
    tone="warning"
    className="internal-schema6-archive-inspector"
  >
    <div
      role="region"
      aria-labelledby="internal-schema6-archive-inspector-heading"
      aria-busy={loading}
      data-internal-schema6-archive-inspector="read-only"
      data-inspection-state={state.phase}
    >
      <h3 id="internal-schema6-archive-inspector-heading">Inspect without opening the project</h3>

      <div id="internal-schema6-read-only-limits" role="note" aria-label="Read-only inspection limits">
        <InlineNotice
          tone="warning"
          title="Inspection only"
          action={<LockKeyhole size={18} aria-hidden="true" />}
        >
          The inspected archive is not active, cannot be edited, and cannot be saved from this view. It never replaces the current project.
        </InlineNotice>
      </div>

      {!nativeDesktop ? <div role="note">
        <InlineNotice tone="warning" title="Native desktop required">
          Choose and inspect schema-6 archives in the installed QuickPLS desktop app. No browser fallback activates or reads a project.
        </InlineNotice>
      </div> : null}

      <form aria-labelledby="internal-schema6-source-heading" onSubmit={submit}>
        <h4 id="internal-schema6-source-heading">Local archive source</h4>
        <div className="desktop-dialog-form-grid">
          <label htmlFor="internal-schema6-archive-path" style={{ gridColumn: "1 / -1" }}>
            Archive path
            <input
              id="internal-schema6-archive-path"
              type="text"
              value={state.archivePath}
              placeholder="D:\\projects\\study-v6.qpls"
              autoComplete="off"
              spellCheck={false}
              disabled={controlsDisabled}
              aria-describedby="internal-schema6-archive-path-help internal-schema6-read-only-limits"
              onChange={(event) => onArchivePathChange(event.target.value)}
            />
          </label>
        </div>
        <p id="internal-schema6-archive-path-help">
          Provide an existing path or use the native file chooser. Only the dedicated strict schema-6 ZIP inspection command is called.
        </p>
        <div className="qpls2-command-row">
          <ToolbarButton type="button" disabled={controlsDisabled} onClick={onBrowse}>
            <FolderOpen size={15} aria-hidden="true" /> Choose and inspect…
          </ToolbarButton>
          <ToolbarButton
            type="submit"
            disabled={controlsDisabled || !state.archivePath.trim()}
          >
            <FileSearch size={15} aria-hidden="true" /> Inspect provided path
          </ToolbarButton>
        </div>
      </form>

      <p role="status" aria-live="polite" aria-atomic="true">
        {state.statusMessage}
      </p>

      {loading ? <div role="status" aria-live="polite" aria-label="Archive inspection in progress">
        <InlineNotice
          tone="info"
          title="Validating schema-6 ZIP"
          action={<LoaderCircle size={18} aria-hidden="true" />}
        >
          QuickPLS is checking the manifest, digests, project document, and resident dataset summaries locally.
        </InlineNotice>
      </div> : null}

      {state.failure ? <div role="alert" aria-labelledby="internal-schema6-error-heading">
        <InlineNotice tone="danger" title="Archive not inspected">
          {state.failure.message} {state.failure.correctiveAction}
        </InlineNotice>
        <p id="internal-schema6-error-heading"><strong>Diagnostic code:</strong> <code>{state.failure.code}</code></p>
      </div> : null}

      {state.phase === "ready" && summary ? <>
        <section className="method-confidence-panel" aria-labelledby="internal-schema6-identity-heading">
          <header>
            <h4 id="internal-schema6-identity-heading">Inspected project identity</h4>
            <StatusBadge status="info">Read-only snapshot</StatusBadge>
          </header>
          <dl>
            <div><dt>Project name</dt><dd>{summary.projectName || "(empty name)"}</dd></div>
            <div><dt>Project ID</dt><dd>{summary.projectId}</dd></div>
            <div><dt>Archive schema</dt><dd>6</dd></div>
            <div><dt>Native engine</dt><dd>{summary.engineVersion}</dd></div>
            <div><dt>Created</dt><dd>{summary.createdAt}</dd></div>
            <div><dt>Modified</dt><dd>{summary.modifiedAt}</dd></div>
            <div><dt>Archive path</dt><dd>{summary.archivePath}</dd></div>
            <div><dt>Archive bytes</dt><dd>{summary.archiveBytes.toLocaleString("en-US")}</dd></div>
            <div><dt>Archive SHA-256</dt><dd>{summary.archiveSha256}</dd></div>
            <div><dt>Source rechecked</dt><dd>{summary.sourceRecheckedUnchanged ? "Unchanged" : "Not confirmed"}</dd></div>
          </dl>
        </section>

        <section aria-labelledby="internal-schema6-counts-heading">
          <h4 id="internal-schema6-counts-heading">Exact validated content counts</h4>
          <div className="qpls2-design-system-grid">
            <MetricCard label="Datasets" value={summary.counts.datasets} detail="Schema-6 dataset descriptors" tone="info" />
            <MetricCard label="Models" value={summary.counts.models} detail="Schema-6 model records" tone="info" />
            <MetricCard label="Recipes" value={summary.counts.recipes} detail="Current recipe records" tone="info" />
            <MetricCard label="Historical recipes" value={summary.counts.historicalRecipes} detail="Immutable historical records" tone="info" />
            <MetricCard label="Historical results" value={summary.counts.historicalResults} detail="Immutable historical records" tone="info" />
            <MetricCard label="Canonical result documents" value={summary.counts.canonicalResultDocuments} detail="Attached canonical documents" tone="info" />
          </div>
        </section>

        <div role="note">
          <InlineNotice tone="success" title="Read-only inspection complete">
            No active project, workspace selection, editable model, save target, autosave state, or recovery state was changed.
          </InlineNotice>
        </div>
      </> : null}
    </div>
  </Panel>;
}

export function InternalProjectArchiveV6Inspector({
  experimentalLabsEnabled,
  nativeDesktopOverride,
  services = defaultServices,
}: {
  experimentalLabsEnabled: boolean;
  nativeDesktopOverride?: boolean;
  services?: InternalProjectArchiveV6InspectorServices;
}) {
  const [state, dispatch] = useReducer(
    internalProjectArchiveV6InspectorReducer,
    undefined,
    () => createInternalProjectArchiveV6InspectorState(),
  );
  const nativeDesktop = nativeDesktopOverride ?? isNativeDesktop();

  if (!experimentalLabsEnabled) return null;

  const runInspection = async (
    load: () => Promise<InternalProjectArchiveV6ReadOutcomeV1 | null>,
    statusMessage: string,
  ) => {
    dispatch({ type: "started", statusMessage });
    dispatch(await resolveInternalProjectArchiveV6Inspection(load));
  };

  return <InternalProjectArchiveV6InspectorView
    nativeDesktop={nativeDesktop}
    state={state}
    onArchivePathChange={(archivePath) => dispatch({ type: "path_changed", archivePath })}
    onBrowse={() => {
      if (!nativeDesktop || state.phase === "loading") return;
      void runInspection(services.chooseAndInspect, "Waiting for a local schema-6 archive, then validating it strictly…");
    }}
    onInspect={() => {
      const archivePath = state.archivePath.trim();
      if (!nativeDesktop || state.phase === "loading" || !archivePath) return;
      void runInspection(
        () => services.inspectAt(archivePath),
        "Validating the provided schema-6 archive path locally…",
      );
    }}
  />;
}
