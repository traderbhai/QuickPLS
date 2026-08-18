import {
  buildPlsSavedRunComparisonV1,
  type PlsSavedRunComparisonDocumentV1,
  type PlsSavedRunComparisonIssueV1,
} from "../domain/plsSavedRunComparisonV1";
import type { AnalysisRun } from "../types";
import {
  canonicalResultDocumentFromAnalysisRunV2,
  type NativeCanonicalResultContextV2,
} from "./nativeCanonicalResultDocumentV2";

export type NativePlsSavedRunComparisonV1 =
  | { status: "hidden" }
  | { status: "ready"; comparison: PlsSavedRunComparisonDocumentV1 }
  | { status: "blocked"; issues: PlsSavedRunComparisonIssueV1[] }
  | { status: "unavailable"; messages: string[] };

export interface NativePlsSavedRunComparisonOptionsV1 {
  /** This bounded surface is not callable from Standard workflows. */
  experimentalLabsEnabled: boolean;
  firstContext?: NativeCanonicalResultContextV2;
  secondContext?: NativeCanonicalResultContextV2;
}

/**
 * Adapt completed native runs into the canonical descriptive comparison.
 * Compatibility is decided by the domain contract, never by run names or UI
 * labels. No comparison definition or result is appended to schema 6 here.
 */
export async function nativePlsSavedRunComparisonV1(
  first: AnalysisRun,
  second: AnalysisRun,
  options: NativePlsSavedRunComparisonOptionsV1,
): Promise<NativePlsSavedRunComparisonV1> {
  if (!options.experimentalLabsEnabled) return { status: "hidden" };
  const [firstDocument, secondDocument] = await Promise.all([
    canonicalResultDocumentFromAnalysisRunV2(first, options.firstContext),
    canonicalResultDocumentFromAnalysisRunV2(second, options.secondContext),
  ]);
  if (!firstDocument.ok || !secondDocument.ok) {
    return {
      status: "unavailable",
      messages: [
        ...(!firstDocument.ok ? firstDocument.errors : []),
        ...(!secondDocument.ok ? secondDocument.errors : []),
      ],
    };
  }
  return buildPlsSavedRunComparisonV1(firstDocument.document, secondDocument.document);
}
