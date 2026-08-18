import { useEffect, useState } from "react";
import {
  reduceStandardSemModelV4AuthorityV1,
  StandardSemModelV4AuthorityError,
  type StandardSemModelV4AuthorityRecordV1,
  type StandardSemModelV4EditorIntentV1,
} from "../domain/standardSemModelV4Authority";
import { parseSemModelV4AuthoringDraft, type SemModelV4 } from "../domain/semModelV4";
import type { StandardSemModelV4AuthorityCommitResult } from "../store";

export type StandardSemModelV4AdvancedCommit = (
  intent: StandardSemModelV4EditorIntentV1,
) => Promise<StandardSemModelV4AuthorityCommitResult>;

export type StandardSemModelV4AdvancedFeedback = {
  tone: "pending" | "committed" | "blocked" | "stale" | "rejected";
  message: string;
};

export function parseStandardSemModelV4AdvancedDocument(
  source: string,
  authority: StandardSemModelV4AuthorityRecordV1,
): SemModelV4 {
  let decoded: unknown;
  try {
    decoded = JSON.parse(source) as unknown;
  } catch (error) {
    const detail = error instanceof Error ? error.message : "The text is not valid JSON.";
    throw new Error(`Invalid JSON: ${detail}`);
  }

  const model = parseSemModelV4AuthoringDraft(decoded);
  return reduceStandardSemModelV4AuthorityV1(authority, {
    kind: "replace_complete_model",
    model,
  }).model;
}

export async function commitStandardSemModelV4AdvancedDocument(
  source: string,
  authority: StandardSemModelV4AuthorityRecordV1,
  commit: StandardSemModelV4AdvancedCommit,
) {
  const model = parseStandardSemModelV4AdvancedDocument(source, authority);
  return commit({ kind: "replace_complete_model", model });
}

export function standardSemModelV4AdvancedFeedbackFor(
  result: StandardSemModelV4AuthorityCommitResult,
): StandardSemModelV4AdvancedFeedback {
  if (result.status === "committed") {
    return {
      tone: "committed",
      message: "Committed to the strict Standard model authority. Native readiness was recalculated. Use Save validated new copy… in the schema-6 session to persist this edit.",
    };
  }
  if (result.status === "blocked") {
    return {
      tone: "blocked",
      message: `Blocked: ${result.diagnostic.message} ${result.diagnostic.correctiveAction}`,
    };
  }
  if (result.status === "stale") {
    return {
      tone: "stale",
      message: "Stale edit ignored because the active model authority changed. Reload the current document before retrying.",
    };
  }
  const detail = result.error instanceof Error ? result.error.message : String(result.error);
  return { tone: "rejected", message: `Rejected: ${detail}` };
}

export interface StandardSemModelV4AdvancedEditorProps {
  authority: StandardSemModelV4AuthorityRecordV1;
  commit: StandardSemModelV4AdvancedCommit;
}

export function StandardSemModelV4AdvancedEditor({
  authority,
  commit,
}: StandardSemModelV4AdvancedEditorProps) {
  const canonicalDocument = JSON.stringify(authority.model, null, 2);
  const [draft, setDraft] = useState(canonicalDocument);
  const [feedback, setFeedback] = useState<StandardSemModelV4AdvancedFeedback | null>(null);
  const [pending, setPending] = useState(false);

  useEffect(() => {
    setDraft(canonicalDocument);
    setFeedback(null);
    setPending(false);
  }, [authority.model_document_sha256, canonicalDocument]);

  const submit = async () => {
    if (pending) return;
    setPending(true);
    setFeedback({ tone: "pending", message: "Validating and committing the complete strict Standard model…" });
    try {
      const result = await commitStandardSemModelV4AdvancedDocument(draft, authority, commit);
      setFeedback(standardSemModelV4AdvancedFeedbackFor(result));
    } catch (error) {
      const detail = error instanceof StandardSemModelV4AuthorityError
        ? `${error.message} ${error.corrective_action}`
        : error instanceof Error ? error.message : String(error);
      setFeedback({ tone: "rejected", message: `Rejected: ${detail}` });
    } finally {
      setPending(false);
    }
  };

  const reset = () => {
    setDraft(canonicalDocument);
    setFeedback(null);
  };

  const feedbackRole = feedback?.tone === "committed" || feedback?.tone === "pending" ? "status" : "alert";

  return <details className="nd-standard-advanced-editor">
    <summary>Advanced canonical document</summary>
    <section aria-labelledby="nd-standard-advanced-heading" aria-busy={pending}>
      <strong id="nd-standard-advanced-heading">Complete SemModelV4 JSON</strong>
      <p id="nd-standard-advanced-help" className="nd-property-note">
        Expert fallback for canonical scientific fields that do not yet have dedicated controls. The stable model ID and data_binding.dataset_id must remain unchanged. Keep annotations and presentation unchanged; use the canvas presentation layer for captions, notes, shapes, images, lines, positions, routing, and viewport settings. The document is strictly decoded, and one native authority commit decides the result. A committed model may still be a draft; resolve its readiness issues before calculation.
      </p>
      <p id="nd-standard-advanced-save-help" className="nd-property-note">
        This editor does not overwrite the source project. After committing, use Save validated new copy… in the schema-6 session to persist the current authority.
      </p>
      <label htmlFor="nd-standard-advanced-document">Canonical model document</label>
      <textarea
        id="nd-standard-advanced-document"
        aria-describedby="nd-standard-advanced-help nd-standard-advanced-save-help"
        rows={18}
        spellCheck={false}
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
      />
      <div className="nd-property-actions">
        <button type="button" disabled={pending} onClick={() => void submit()}>Validate and commit</button>
        <button type="button" disabled={pending || draft === canonicalDocument} onClick={reset}>Reset to current authority</button>
      </div>
      {feedback ? <p
        className={`nd-authority-feedback ${feedback.tone}`}
        role={feedbackRole}
        aria-live="polite"
        aria-atomic="true"
      >{feedback.message}</p> : null}
    </section>
  </details>;
}
