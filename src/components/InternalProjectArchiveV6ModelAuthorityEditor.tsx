import { FileJson, LoaderCircle, ShieldCheck } from "lucide-react";
import { useState } from "react";
import type { InternalProjectArchiveV6ModelMutationV1 } from "../domain/internalProjectArchiveV6ModelMutation";
import type { ProjectModelRecordV6Wire } from "../domain/internalProjectArchiveV6Wire";
import {
  parseSemModelV4AuthoringDraft,
  type SemModelV4,
} from "../domain/semModelV4";
import {
  applySemModelV4AuthorityOperationBatchV1,
  parseSemModelV4AuthorityOperationBatchJsonV1,
  SEM_MODEL_V4_AUTHORITY_OPERATION_UNSUPPORTED_ACTIONS,
} from "../domain/semModelV4AuthorityOperations";
import {
  type InternalProjectArchiveV6ReadOnlySession,
  useInternalProjectArchiveV6Session,
} from "../internalProjectArchiveV6SessionStore";
import { InlineNotice, StatusBadge, ToolbarButton } from "./Ui";

type DraftModelRecord = ProjectModelRecordV6Wire & {
  payload: Extract<ProjectModelRecordV6Wire["payload"], { kind: "sem_model_v4_draft" }>;
};
type AuthorityModelRecord = ProjectModelRecordV6Wire & {
  payload: Extract<ProjectModelRecordV6Wire["payload"], { kind: "sem_model_v4" | "sem_model_v4_draft" }>;
};

export type InternalProjectArchiveV6JsonMutationMode = "insert" | "replace" | "operations";

export class InternalProjectArchiveV6ModelJsonError extends Error {
  constructor(
    public readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = "InternalProjectArchiveV6ModelJsonError";
  }
}

/** JSON decoding followed by the existing exact-key SemModelV4 draft decoder. */
export function parseInternalProjectArchiveV6SemModelJson(
  json: string,
): SemModelV4 {
  if (!json.trim()) {
    throw new InternalProjectArchiveV6ModelJsonError(
      "schema6_model_editor.json_required",
      "Paste one exact SemModelV4 JSON object.",
    );
  }
  let decoded: unknown;
  try {
    decoded = JSON.parse(json);
  } catch {
    throw new InternalProjectArchiveV6ModelJsonError(
      "schema6_model_editor.json_invalid",
      "The pasted value is not valid JSON.",
    );
  }
  return parseSemModelV4AuthoringDraft(decoded);
}

export function isDraftModelRecord(
  record: ProjectModelRecordV6Wire,
): record is DraftModelRecord {
  return record.payload.kind === "sem_model_v4_draft";
}

function isAuthorityModelRecord(
  record: ProjectModelRecordV6Wire,
): record is AuthorityModelRecord {
  return record.payload.kind === "sem_model_v4"
    || record.payload.kind === "sem_model_v4_draft";
}

export function buildInternalProjectArchiveV6JsonMutation(
  mode: InternalProjectArchiveV6JsonMutationMode,
  json: string,
  selectedDraft?: DraftModelRecord,
): InternalProjectArchiveV6ModelMutationV1 {
  if (mode === "operations") {
    if (!selectedDraft) {
      throw new InternalProjectArchiveV6ModelJsonError(
        "schema6_model_editor.draft_required",
        "Select one current draft before applying canonical operations.",
      );
    }
    const batch = parseSemModelV4AuthorityOperationBatchJsonV1(json);
    const replacement = applySemModelV4AuthorityOperationBatchV1(
      selectedDraft.payload.model,
      batch,
    ).model;
    return {
      kind: "replace_draft",
      modelId: selectedDraft.model_id,
      expectedModelDocumentSha256: selectedDraft.payload.model_document_sha256,
      replacement,
    };
  }
  const model = parseInternalProjectArchiveV6SemModelJson(json);
  if (mode === "insert") return { kind: "insert_draft", draft: model };
  if (!selectedDraft) {
    throw new InternalProjectArchiveV6ModelJsonError(
      "schema6_model_editor.draft_required",
      "Select one current draft before replacing it.",
    );
  }
  if (model.id !== selectedDraft.model_id) {
    throw new InternalProjectArchiveV6ModelJsonError(
      "schema6_model_editor.model_id_mismatch",
      "Replacement JSON must keep the selected draft model id exactly.",
    );
  }
  return {
    kind: "replace_draft",
    modelId: selectedDraft.model_id,
    expectedModelDocumentSha256: selectedDraft.payload.model_document_sha256,
    replacement: model,
  };
}

export function buildInternalProjectArchiveV6Promotion(
  selectedDraft?: DraftModelRecord,
): InternalProjectArchiveV6ModelMutationV1 {
  if (!selectedDraft) {
    throw new InternalProjectArchiveV6ModelJsonError(
      "schema6_model_editor.draft_required",
      "Select one current draft before promoting it.",
    );
  }
  return {
    kind: "promote_draft",
    modelId: selectedDraft.model_id,
    expectedModelDocumentSha256: selectedDraft.payload.model_document_sha256,
  };
}

function editorFailure(error: unknown): { code: string; message: string; correctiveAction?: string } {
  const record = error && typeof error === "object"
    ? error as Record<string, unknown>
    : null;
  const correctiveAction = record && typeof record.correctiveAction === "string"
    ? record.correctiveAction
    : undefined;
  return {
    code: record && typeof record.code === "string"
      ? record.code
      : "schema6_model_editor.input_invalid",
    message: error instanceof Error && error.message
      ? error.message
      : "The model-authority request is invalid.",
    ...(correctiveAction ? { correctiveAction } : {}),
  };
}

export interface InternalProjectArchiveV6ModelAuthorityEditorViewProps {
  session: InternalProjectArchiveV6ReadOnlySession;
  mode: InternalProjectArchiveV6JsonMutationMode;
  json: string;
  selectedDraftId: string;
  pending: boolean;
  dirty: boolean;
  persistence: "not_persisted" | "persisted_new_copy" | null;
  statusMessage: string;
  failure: { code: string; message: string; correctiveAction?: string } | null;
  onModeChange: (mode: InternalProjectArchiveV6JsonMutationMode) => void;
  onJsonChange: (json: string) => void;
  onSelectedDraftChange: (modelId: string) => void;
  onApplyJson: () => void;
  onPromote: () => void;
}

export function InternalProjectArchiveV6ModelAuthorityEditorView({
  session,
  mode,
  json,
  selectedDraftId,
  pending,
  dirty,
  persistence,
  statusMessage,
  failure,
  onModeChange,
  onJsonChange,
  onSelectedDraftChange,
  onApplyJson,
  onPromote,
}: InternalProjectArchiveV6ModelAuthorityEditorViewProps) {
  const authorityRecords = session.project.models.filter(isAuthorityModelRecord);
  const drafts = authorityRecords.filter(isDraftModelRecord);
  const effectiveDraftId = selectedDraftId || drafts[0]?.model_id || "";

  return <section
    aria-labelledby="internal-schema6-model-authority-heading"
    className="internal-schema6-model-authority-editor"
    data-schema6-model-authority-editor="ephemeral"
  >
    <header>
      <div>
        <h4 id="internal-schema6-model-authority-heading">Ephemeral model-authority editor</h4>
        <p>Inspect ready and draft SemModelV4 records, or apply one in-memory draft transition.</p>
      </div>
      <StatusBadge status="experimental">Persistence · {dirty
        ? "not_persisted"
        : persistence ?? "source_snapshot"}</StatusBadge>
    </header>

    {dirty ? <InlineNotice tone="warning" title="Unsaved ephemeral changes">
      This session document is dirty and not_persisted. Save it only with the new-destination Save validated new copy action.
    </InlineNotice> : <InlineNotice tone="info" title="Archive document unchanged">
      {persistence === "persisted_new_copy"
        ? "The current detached document is based on the validated new copy. Further model changes remain ephemeral until another Save copy."
        : "No ephemeral model change has been applied. Standard save, autosave, and recovery remain unavailable."}
    </InlineNotice>}

    <div role="region" aria-label="Schema-6 model authority records" tabIndex={0}>
      <table>
        <caption>Ready and draft model records in the ephemeral session document</caption>
        <thead><tr>
          <th scope="col">Model</th>
          <th scope="col">Authority</th>
          <th scope="col">Exact digest</th>
        </tr></thead>
        <tbody>
          {authorityRecords.map((record) => <tr key={record.model_id}>
            <td><strong>{record.payload.model.name}</strong><br /><code>{record.model_id}</code></td>
            <td>{record.payload.kind === "sem_model_v4_draft" ? "Draft" : "Ready"}</td>
            <td><code>{record.payload.kind === "sem_model_v4_draft"
              ? record.payload.model_document_sha256
              : record.payload.scientific_sha256}</code></td>
          </tr>)}
          {authorityRecords.length === 0 ? <tr><td colSpan={3}>No ready or draft SemModelV4 records.</td></tr> : null}
        </tbody>
      </table>
    </div>

    <div className="desktop-dialog-form-grid">
      <label htmlFor="internal-schema6-model-json-mode">
        JSON operation
        <select
          id="internal-schema6-model-json-mode"
          value={mode}
          disabled={pending}
          onChange={(event) => onModeChange(
            event.target.value === "replace"
              ? "replace"
              : event.target.value === "operations"
                ? "operations"
                : "insert",
          )}
        >
          <option value="insert">Insert new draft</option>
          <option value="replace">Replace selected draft</option>
          <option value="operations">Apply canonical operation batch</option>
        </select>
      </label>
      <label htmlFor="internal-schema6-model-draft">
        Current draft authority
        <select
          id="internal-schema6-model-draft"
          value={effectiveDraftId}
          disabled={pending || drafts.length === 0}
          onChange={(event) => onSelectedDraftChange(event.target.value)}
        >
          {drafts.length === 0 ? <option value="">No drafts available</option> : null}
          {drafts.map((record) => <option key={record.model_id} value={record.model_id}>
            {record.payload.model.name} · {record.model_id}
          </option>)}
        </select>
      </label>
      <label htmlFor="internal-schema6-model-json" style={{ gridColumn: "1 / -1" }}>
        {mode === "operations" ? "Exact canonical authority-operation batch JSON" : "Exact SemModelV4 JSON"}
        <textarea
          id="internal-schema6-model-json"
          rows={12}
          value={json}
          disabled={pending}
          spellCheck={false}
          aria-describedby="internal-schema6-model-json-help"
          onChange={(event) => onJsonChange(event.target.value)}
        />
      </label>
    </div>
    <p id="internal-schema6-model-json-help">
      {mode === "operations"
        ? "The batch is applied atomically to the selected canonical SemModelV4 authority. Its exact model id, bounded operations, nested model objects, and final authoring integrity are validated before the detached draft changes."
        : "Unknown fields, missing fields, invalid discriminators, and authoring-integrity violations are rejected by the strict SemModelV4 decoder."}
    </p>

    {mode === "operations" ? <details>
      <summary>Bounded operation lane and fail-closed exclusions</summary>
      <p>Supported operations append or replace variables and parameters, append relations, constraints, and derived terms, or set the group and data binding.</p>
      <ul>{SEM_MODEL_V4_AUTHORITY_OPERATION_UNSUPPORTED_ACTIONS.map((entry) => <li key={entry.action}>
        <code>{entry.action}</code>: {entry.correctiveAction}
      </li>)}</ul>
    </details> : null}

    <div className="qpls2-command-row">
      <ToolbarButton type="button" disabled={pending || !json.trim()} onClick={onApplyJson}>
        <FileJson size={15} aria-hidden="true" /> {mode === "insert"
          ? "Insert draft JSON"
          : mode === "operations"
            ? "Apply operation batch"
            : "Replace selected draft"}
      </ToolbarButton>
      <ToolbarButton type="button" disabled={pending || !effectiveDraftId} onClick={onPromote}>
        <ShieldCheck size={15} aria-hidden="true" /> Promote selected draft
      </ToolbarButton>
    </div>

    <p role="status" aria-live="polite" aria-atomic="true">
      {pending ? <><LoaderCircle size={15} aria-hidden="true" /> </> : null}{statusMessage}
    </p>

    {failure ? <div role="alert">
      <InlineNotice tone="danger" title="Model change not applied">
        {failure.message} {failure.correctiveAction ?? "Correct the exact JSON or refresh the draft authority, then retry."}
      </InlineNotice>
      <p><strong>Diagnostic code:</strong> <code>{failure.code}</code></p>
    </div> : null}
  </section>;
}

export function InternalProjectArchiveV6ModelAuthorityEditor({
  session,
}: {
  session: InternalProjectArchiveV6ReadOnlySession;
}) {
  const [mode, setMode] = useState<InternalProjectArchiveV6JsonMutationMode>("insert");
  const [json, setJson] = useState("");
  const [selectedDraftId, setSelectedDraftId] = useState("");
  const [inputFailure, setInputFailure] = useState<{ code: string; message: string; correctiveAction?: string } | null>(null);
  const pending = useInternalProjectArchiveV6Session((state) => state.modelMutationPending);
  const dirty = useInternalProjectArchiveV6Session((state) => state.dirty);
  const persistence = useInternalProjectArchiveV6Session((state) => state.persistence);
  const serviceFailure = useInternalProjectArchiveV6Session((state) => state.modelMutationFailure);
  const statusMessage = useInternalProjectArchiveV6Session((state) => state.modelMutationStatusMessage);
  const mutateModel = useInternalProjectArchiveV6Session((state) => state.mutateModel);

  const drafts = session.project.models.filter(isDraftModelRecord);
  const selectedDraft = drafts.find((record) => record.model_id === selectedDraftId)
    ?? drafts[0];

  const apply = async (mutation: InternalProjectArchiveV6ModelMutationV1) => {
    setInputFailure(null);
    const result = await mutateModel(mutation);
    if (result === "applied") setJson("");
  };

  const applyJson = () => {
    try {
      void apply(buildInternalProjectArchiveV6JsonMutation(mode, json, selectedDraft));
    } catch (error) {
      setInputFailure(editorFailure(error));
    }
  };

  const promote = () => {
    try {
      void apply(buildInternalProjectArchiveV6Promotion(selectedDraft));
    } catch (error) {
      setInputFailure(editorFailure(error));
    }
  };

  return <InternalProjectArchiveV6ModelAuthorityEditorView
    session={session}
    mode={mode}
    json={json}
    selectedDraftId={selectedDraft?.model_id ?? ""}
    pending={pending}
    dirty={dirty}
    persistence={persistence}
    statusMessage={statusMessage}
    failure={inputFailure ?? serviceFailure}
    onModeChange={(nextMode) => {
      setMode(nextMode);
      setInputFailure(null);
    }}
    onJsonChange={(nextJson) => {
      setJson(nextJson);
      setInputFailure(null);
    }}
    onSelectedDraftChange={(modelId) => {
      setSelectedDraftId(modelId);
      setInputFailure(null);
    }}
    onApplyJson={applyJson}
    onPromote={promote}
  />;
}
