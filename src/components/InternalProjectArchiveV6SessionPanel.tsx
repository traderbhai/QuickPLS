import { FolderOpen, GitFork, LoaderCircle, LockKeyhole, LogOut, Save, Sparkles } from "lucide-react";
import { type FormEvent, useEffect, useState } from "react";
import type { InternalProjectArchiveV6ReadOutcomeV1 } from "../domain/internalProjectArchiveV6Read";
import {
  type InternalProjectArchiveV6SessionState,
  type InternalProjectArchiveV6SaveCopyExecutor,
  type InternalProjectArchiveV6StandardAuthorityResolver,
  useInternalProjectArchiveV6Session,
} from "../internalProjectArchiveV6SessionStore";
import {
  inspectInternalProjectArchiveV6At,
  openInternalProjectArchiveV6,
} from "../services/internalProjectArchiveV6ReadService";
import { saveInternalProjectArchiveV6Copy } from "../services/internalProjectArchiveV6SaveCopyService";
import { resolveStandardSemModelV4Authority } from "../services/standardSemModelV4AuthorityService";
import { isNativeDesktop } from "../services/projectService";
import { InlineNotice, MetricCard, Panel, StatusBadge, ToolbarButton } from "./Ui";

export interface InternalProjectArchiveV6SessionServices {
  chooseAndRead: () => Promise<InternalProjectArchiveV6ReadOutcomeV1 | null>;
  readAt: (archivePath: string) => Promise<InternalProjectArchiveV6ReadOutcomeV1>;
  chooseAndSaveCopy: InternalProjectArchiveV6SaveCopyExecutor;
  resolveStandardAuthority?: InternalProjectArchiveV6StandardAuthorityResolver;
}

const defaultServices: InternalProjectArchiveV6SessionServices = {
  chooseAndRead: openInternalProjectArchiveV6,
  readAt: inspectInternalProjectArchiveV6At,
  chooseAndSaveCopy: saveInternalProjectArchiveV6Copy,
  resolveStandardAuthority: resolveStandardSemModelV4Authority,
};

export interface InternalProjectArchiveV6SessionViewProps {
  nativeDesktop: boolean;
  archivePath: string;
  state: Pick<
    InternalProjectArchiveV6SessionState,
    "phase" | "session" | "failure" | "statusMessage" | "dirty" | "persistence" | "modelMutationPending"
    | "standardActivationPending" | "standardActivationFailure" | "standardActivationStatusMessage"
    | "saveCopyPending" | "saveCopyFailure" | "saveCopyStatusMessage"
  > & Partial<Pick<InternalProjectArchiveV6SessionState,
    "revisionForkPending" | "revisionForkFailure" | "revisionForkStatusMessage"
  >>;
  onArchivePathChange: (archivePath: string) => void;
  onBrowseAndOpen: () => void;
  onOpenAtPath: () => void;
  onSaveCopy: () => void;
  onActivateStandard: () => void;
  onForkRevision: () => void;
  onCloseStandardProject: () => void;
  onClose: () => void;
}

export function InternalProjectArchiveV6SessionView({
  nativeDesktop,
  archivePath,
  state,
  onArchivePathChange,
  onBrowseAndOpen,
  onOpenAtPath,
  onSaveCopy,
  onActivateStandard,
  onForkRevision,
  onCloseStandardProject,
  onClose,
}: InternalProjectArchiveV6SessionViewProps) {
  const opening = state.phase === "opening";
  const controlsDisabled = opening || state.standardActivationPending || state.revisionForkPending || state.saveCopyPending || !nativeDesktop;
  const session = state.session;
  const snapshot = state.session?.snapshot ?? null;

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!controlsDisabled && archivePath.trim()) onOpenAtPath();
  };

  return <Panel
    title="Schema-6 read-only memory session"
    description="Internal/Labs opening of one strict schema-6 ZIP into an isolated, non-persistent frontend session."
    actions={<StatusBadge status="experimental">Labs · isolated</StatusBadge>}
    tone="warning"
    className="internal-schema6-session-panel"
  >
    <div
      role="region"
      aria-labelledby="internal-schema6-session-heading"
      aria-busy={opening || state.standardActivationPending || state.saveCopyPending}
      data-internal-schema6-session={state.session ? "active" : state.phase}
    >
      <h3 id="internal-schema6-session-heading">Open safely, then activate explicitly</h3>

      <InlineNotice
        tone="warning"
        title="Read-only Labs boundary"
        action={<LockKeyhole size={18} aria-hidden="true" />}
      >
        Opening never changes Standard. The explicit activation action installs native-resolved ready/draft models as the
        sole Standard science authority. Schema-5 save, autosave, calculation, and recovery remain unavailable; persistence
        uses only a validated new-destination schema-6 copy.
      </InlineNotice>

      {!nativeDesktop ? <InlineNotice tone="warning" title="Native desktop required">
        Open schema-6 sessions only in the installed desktop app. No browser fallback reads or activates an archive.
      </InlineNotice> : null}

      <form aria-labelledby="internal-schema6-session-source-heading" onSubmit={submit}>
        <h4 id="internal-schema6-session-source-heading">Local archive source</h4>
        <div className="desktop-dialog-form-grid">
          <label htmlFor="internal-schema6-session-path" style={{ gridColumn: "1 / -1" }}>
            Archive path
            <input
              id="internal-schema6-session-path"
              type="text"
              value={archivePath}
              placeholder="D:\\projects\\study-v6.qpls"
              autoComplete="off"
              spellCheck={false}
              disabled={controlsDisabled}
              aria-describedby="internal-schema6-session-path-help"
              onChange={(event) => onArchivePathChange(event.target.value)}
            />
          </label>
        </div>
        <p id="internal-schema6-session-path-help">
          The strict native reader validates the ZIP and returns a detached project document plus resident-dataset summaries.
        </p>
        <div className="qpls2-command-row">
          <ToolbarButton type="button" disabled={controlsDisabled} onClick={onBrowseAndOpen}>
            <FolderOpen size={15} aria-hidden="true" /> Choose and open read-only…
          </ToolbarButton>
          <ToolbarButton
            type="submit"
            disabled={controlsDisabled || !archivePath.trim()}
          >
            <FolderOpen size={15} aria-hidden="true" /> Open provided path read-only
          </ToolbarButton>
          <ToolbarButton
            type="button"
            disabled={state.saveCopyPending || Boolean(state.session?.standardActivation) || (!state.session && !opening && !state.standardActivationPending)}
            title={state.session?.standardActivation
              ? "The schema-6 source session must remain open while Standard authority is active."
              : undefined}
            onClick={onClose}
          >
            <LogOut size={15} aria-hidden="true" /> {state.saveCopyPending
              ? "Saving new copy…"
              : opening || state.standardActivationPending
                ? "Cancel and close session"
                : "Close read-only session"}
          </ToolbarButton>
        </div>
      </form>

      <p role="status" aria-live="polite" aria-atomic="true">{state.statusMessage}</p>

      {opening ? <InlineNotice
        tone="info"
        title="Opening through the strict reader"
        action={<LoaderCircle size={18} aria-hidden="true" />}
      >
        The current session remains isolated while the archive is validated locally.
      </InlineNotice> : null}

      {state.failure ? <div role="alert">
        <InlineNotice tone="danger" title="Archive not opened">
          {state.failure.message} {state.failure.correctiveAction}
        </InlineNotice>
        <p><strong>Diagnostic code:</strong> <code>{state.failure.code}</code></p>
      </div> : null}

      {snapshot ? <section aria-labelledby="internal-schema6-active-session-heading">
        <header>
          <h4 id="internal-schema6-active-session-heading">Active only in the Labs memory session</h4>
          <StatusBadge status="info">Read-only · schema 6</StatusBadge>
        </header>
        <dl>
          <div><dt>Project name</dt><dd>{snapshot.manifest.name || "(empty name)"}</dd></div>
          <div><dt>Project ID</dt><dd>{snapshot.manifest.project_id}</dd></div>
          <div><dt>Archive path</dt><dd>{snapshot.archivePath}</dd></div>
          <div><dt>Archive SHA-256</dt><dd>{snapshot.archiveSha256}</dd></div>
          <div><dt>Source rechecked</dt><dd>{snapshot.sourceRecheckedUnchanged ? "Unchanged" : "Not confirmed"}</dd></div>
        </dl>
        <div className="qpls2-design-system-grid">
          <MetricCard label="Datasets" value={snapshot.counts.datasets} detail="Validated resident summaries" tone="info" />
          <MetricCard label="Models" value={session?.project.models.length ?? snapshot.counts.models} detail="Current ephemeral document" tone="info" />
          <MetricCard label="Recipes" value={snapshot.counts.recipes} detail="Read-only current records" tone="info" />
          <MetricCard label="Saved results" value={snapshot.counts.historicalResults + snapshot.counts.canonicalResultDocuments} detail="Historical plus canonical documents" tone="info" />
        </div>
        <InlineNotice tone="success" title="Isolated session active">
          The full schema-6 project document is retained only in this Labs store. Dataset values are not exposed to the frontend;
          only summaries validated while resident in the native reader are retained.
        </InlineNotice>
        <div className="qpls2-command-row">
          <ToolbarButton
            type="button"
            disabled={controlsDisabled || Boolean(session?.standardActivation)}
            onClick={onActivateStandard}
          >
            {state.standardActivationPending
              ? <LoaderCircle size={15} aria-hidden="true" />
              : <Sparkles size={15} aria-hidden="true" />}
            {session?.standardActivation
              ? "Activated in Standard"
              : state.standardActivationPending
                ? "Resolving native authority…"
                : "Activate ready/draft models in Standard"}
          </ToolbarButton>
        </div>
        <p role="status" aria-live="polite" aria-atomic="true">{state.standardActivationStatusMessage}</p>
        {state.standardActivationFailure ? <div role="alert">
          <InlineNotice tone="danger" title="Standard activation blocked">
            {state.standardActivationFailure.message} {state.standardActivationFailure.correctiveAction}
          </InlineNotice>
          <p><strong>Diagnostic code:</strong> <code>{state.standardActivationFailure.code}</code></p>
        </div> : null}
        {session?.standardActivation ? <>
          <div className="qpls2-command-row">
            <ToolbarButton
              type="button"
              disabled={controlsDisabled || state.dirty}
              title={state.dirty
                ? "Fork-before-edit requires the selected RecipeV4-bound model to be clean."
                : "Create and activate a new unbound model identity; existing recipes and results stay unchanged."}
              onClick={onForkRevision}
            >
              {state.revisionForkPending
                ? <LoaderCircle size={15} aria-hidden="true" />
                : <GitFork size={15} aria-hidden="true" />}
              {state.revisionForkPending ? "Creating model revision…" : "Edit active model as new revision"}
            </ToolbarButton>
            <span>The old RecipeV4 and canonical results remain bound to the original model ID and digest.</span>
          </div>
          <p role="status" aria-live="polite" aria-atomic="true">{state.revisionForkStatusMessage}</p>
          {state.revisionForkFailure ? <div role="alert">
            <InlineNotice tone="danger" title="Model revision not created">
              {state.revisionForkFailure.message} {state.revisionForkFailure.correctiveAction}
            </InlineNotice>
            <p><strong>Diagnostic code:</strong> <code>{state.revisionForkFailure.code}</code></p>
          </div> : null}
        </> : null}
        <div className="qpls2-command-row">
          <ToolbarButton
            type="button"
            disabled={controlsDisabled || !session?.standardActivation || !state.dirty}
            onClick={onSaveCopy}
          >
            {state.saveCopyPending
              ? <LoaderCircle size={15} aria-hidden="true" />
              : <Save size={15} aria-hidden="true" />}
            {state.saveCopyPending ? "Saving validated new copy…" : "Save validated new copy…"}
          </ToolbarButton>
        </div>
        <p role="status" aria-live="polite" aria-atomic="true">{state.saveCopyStatusMessage}</p>
        {state.saveCopyFailure ? <div role="alert">
          <InlineNotice tone="danger" title="Copy not saved">
            {state.saveCopyFailure.message} {state.saveCopyFailure.correctiveAction}
          </InlineNotice>
          <p><strong>Diagnostic code:</strong> <code>{state.saveCopyFailure.code}</code></p>
        </div> : null}
        {session?.standardActivation ? <div className="qpls2-command-row">
          <ToolbarButton
            type="button"
            disabled={controlsDisabled || state.dirty || state.persistence !== "persisted_new_copy"}
            title={state.dirty
              ? "Save a validated new copy before closing the Standard project."
              : state.persistence !== "persisted_new_copy"
                ? "A validated new copy is required before closing the bound Standard project."
                : "Close Standard and release its schema-6 source binding."}
            onClick={onCloseStandardProject}
          >
            <LogOut size={15} aria-hidden="true" /> Close Standard project
          </ToolbarButton>
          <span>Closes the strict Standard workspace and its source session together. Reopen the saved copy to continue.</span>
        </div> : null}
      </section> : null}

    </div>
  </Panel>;
}

export function InternalProjectArchiveV6SessionPanel({
  experimentalLabsEnabled,
  nativeDesktopOverride,
  services = defaultServices,
}: {
  experimentalLabsEnabled: boolean;
  nativeDesktopOverride?: boolean;
  services?: InternalProjectArchiveV6SessionServices;
}) {
  const [archivePath, setArchivePath] = useState("");
  const phase = useInternalProjectArchiveV6Session((state) => state.phase);
  const session = useInternalProjectArchiveV6Session((state) => state.session);
  const failure = useInternalProjectArchiveV6Session((state) => state.failure);
  const statusMessage = useInternalProjectArchiveV6Session((state) => state.statusMessage);
  const modelMutationPending = useInternalProjectArchiveV6Session((state) => state.modelMutationPending);
  const standardActivationPending = useInternalProjectArchiveV6Session((state) => state.standardActivationPending);
  const standardActivationFailure = useInternalProjectArchiveV6Session((state) => state.standardActivationFailure);
  const standardActivationStatusMessage = useInternalProjectArchiveV6Session((state) => state.standardActivationStatusMessage);
  const revisionForkPending = useInternalProjectArchiveV6Session((state) => state.revisionForkPending);
  const revisionForkFailure = useInternalProjectArchiveV6Session((state) => state.revisionForkFailure);
  const revisionForkStatusMessage = useInternalProjectArchiveV6Session((state) => state.revisionForkStatusMessage);
  const dirty = useInternalProjectArchiveV6Session((state) => state.dirty);
  const persistence = useInternalProjectArchiveV6Session((state) => state.persistence);
  const saveCopyPending = useInternalProjectArchiveV6Session((state) => state.saveCopyPending);
  const saveCopyFailure = useInternalProjectArchiveV6Session((state) => state.saveCopyFailure);
  const saveCopyStatusMessage = useInternalProjectArchiveV6Session((state) => state.saveCopyStatusMessage);
  const openSession = useInternalProjectArchiveV6Session((state) => state.open);
  const activateStandardAuthorities = useInternalProjectArchiveV6Session((state) => state.activateStandardAuthorities);
  const forkActiveRecipeBoundRevision = useInternalProjectArchiveV6Session((state) => state.forkActiveRecipeBoundRevision);
  const saveCopy = useInternalProjectArchiveV6Session((state) => state.saveCopy);
  const closeStandardProject = useInternalProjectArchiveV6Session((state) => state.closeStandardProject);
  const deactivate = useInternalProjectArchiveV6Session((state) => state.deactivate);
  const nativeDesktop = nativeDesktopOverride ?? isNativeDesktop();

  useEffect(() => {
    if (!experimentalLabsEnabled) deactivate();
  }, [deactivate, experimentalLabsEnabled]);

  if (!experimentalLabsEnabled) return null;

  const runOpen = async (
    loader: () => Promise<InternalProjectArchiveV6ReadOutcomeV1 | null>,
  ) => {
    const result = await openSession(loader);
    if (result === "activated") {
      setArchivePath(useInternalProjectArchiveV6Session.getState().session?.snapshot.archivePath ?? "");
    }
  };

  return <InternalProjectArchiveV6SessionView
    nativeDesktop={nativeDesktop}
    archivePath={archivePath}
    state={{
      phase,
      session,
      failure,
      statusMessage,
      dirty,
      persistence,
      modelMutationPending,
      standardActivationPending,
      standardActivationFailure,
      standardActivationStatusMessage,
      revisionForkPending,
      revisionForkFailure,
      revisionForkStatusMessage,
      saveCopyPending,
      saveCopyFailure,
      saveCopyStatusMessage,
    }}
    onArchivePathChange={setArchivePath}
    onBrowseAndOpen={() => {
      if (!nativeDesktop || phase === "opening") return;
      void runOpen(services.chooseAndRead);
    }}
    onOpenAtPath={() => {
      const path = archivePath.trim();
      if (!nativeDesktop || phase === "opening" || !path) return;
      void runOpen(() => services.readAt(path));
    }}
    onSaveCopy={() => {
      if (!nativeDesktop || phase !== "active" || saveCopyPending || !dirty) return;
      void saveCopy(services.chooseAndSaveCopy).then((result) => {
        if (result === "saved") {
          setArchivePath(useInternalProjectArchiveV6Session.getState().session?.snapshot.archivePath ?? "");
        }
      });
    }}
    onActivateStandard={() => {
      if (!nativeDesktop || phase !== "active" || standardActivationPending || session?.standardActivation) return;
      void activateStandardAuthorities(services.resolveStandardAuthority);
    }}
    onForkRevision={() => {
      if (!nativeDesktop || phase !== "active" || revisionForkPending || dirty) return;
      void forkActiveRecipeBoundRevision();
    }}
    onCloseStandardProject={() => {
      if (!nativeDesktop || phase !== "active") return;
      if (closeStandardProject() === "closed") setArchivePath("");
    }}
    onClose={deactivate}
  />;
}
