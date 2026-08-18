import {
  buildCanonicalResultSemanticExportV2,
  canonicalResultSemanticExportJsonV2,
  verifyCanonicalResultSemanticExportReadbackV2,
  type CanonicalResultSemanticExportV2,
} from "../domain/canonicalResultSemanticExportV2";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import type { AnalysisRun } from "../types";
import {
  canonicalResultDocumentFromAnalysisRunV2,
  type NativeCanonicalResultContextV2,
} from "./nativeCanonicalResultDocumentV2";

export type NativeCanonicalSemanticExportPreviewV2 =
  | {
      status: "ready";
      sourceDocumentId: string;
      projection: CanonicalResultSemanticExportV2;
      json: string;
    }
  | { status: "unavailable"; messages: string[] };

/**
 * Projects an already native-built canonical document without reconstructing
 * method tables or invoking a file writer.
 */
export function previewNativeCanonicalDocumentSemanticExportV2(
  document: CanonicalResultDocumentV2,
): NativeCanonicalSemanticExportPreviewV2 {
  const builtExport = buildCanonicalResultSemanticExportV2(document);
  if (!builtExport.ok) return { status: "unavailable", messages: builtExport.errors };

  const json = canonicalResultSemanticExportJsonV2(builtExport.projection);
  const readback = verifyCanonicalResultSemanticExportReadbackV2(document, json);
  if (!readback.passed) return { status: "unavailable", messages: readback.errors };
  return {
    status: "ready",
    sourceDocumentId: document.document_id,
    projection: builtExport.projection,
    json,
  };
}

/**
 * Internal preview boundary for the format-neutral export pipeline. It does
 * not replace any current CSV, HTML, or workbook writer and performs no file
 * system mutation.
 */
export async function previewNativeCanonicalSemanticExportV2(
  run: AnalysisRun,
  context: NativeCanonicalResultContextV2 = {},
): Promise<NativeCanonicalSemanticExportPreviewV2> {
  const builtDocument = await canonicalResultDocumentFromAnalysisRunV2(run, context);
  if (!builtDocument.ok) return { status: "unavailable", messages: builtDocument.errors };

  return previewNativeCanonicalDocumentSemanticExportV2(builtDocument.document);
}
