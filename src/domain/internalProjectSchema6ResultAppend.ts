import type { CanonicalResultDocumentV2, CapabilityCellReferenceV2 } from "./canonicalResultDocumentV2";
import type { AnalysisRecipeV4, AnalysisRecipeV4MissingDataPolicy } from "./internalRecipeV4PlsExecution";

export const INTERNAL_PROJECT_SCHEMA6_RESULT_APPEND_SURFACE = "internal_labs" as const;
export const STANDARD_GENERAL_SEM_PROJECT_SCHEMA6_RESULT_APPEND_SURFACE = "standard" as const;
export const STANDARD_EXACT_CBSEM_PROJECT_SCHEMA6_RESULT_APPEND_SURFACE = "standard_exact_cbsem" as const;

export interface InternalProjectSchema6ResultAppendRequestV1 {
  surface:
    | typeof INTERNAL_PROJECT_SCHEMA6_RESULT_APPEND_SURFACE
    | typeof STANDARD_GENERAL_SEM_PROJECT_SCHEMA6_RESULT_APPEND_SURFACE
    | typeof STANDARD_EXACT_CBSEM_PROJECT_SCHEMA6_RESULT_APPEND_SURFACE;
  experimentalLabsEnabled: boolean;
  /**
   * Exact General SEM execution selection. Every General SEM mutation requires
   * this field; omission remains only for non-General legacy and exact CB paths.
   */
  capabilityCell?: CapabilityCellReferenceV2;
  archivePath: string;
  expectedSourceSha256: string;
  recipe?: AnalysisRecipeV4<AnalysisRecipeV4MissingDataPolicy>;
  canonicalDocument: CanonicalResultDocumentV2;
}

export interface InternalProjectSchema6ResultAppendReceiptV1 {
  schema_version: 6;
  project_id: string;
  archive_path: string;
  source_document_sha256: string;
  updated_document_sha256: string;
  canonical_document_id: string;
  run_id: string;
  canonical_result_document_count: number;
  source_verified_at_commit: boolean;
  post_write_validated: boolean;
  rollback_copy_removed: boolean;
}

export type InternalProjectSchema6ResultAppendOutcomeV1 =
  | { status: "ok"; value: InternalProjectSchema6ResultAppendReceiptV1 }
  | {
    status: "blocked";
    diagnostic: { code: string; message: string; correctiveAction: string };
  };
