import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "../domain/internalProjectArchiveV6Read";
import { supportsGeneralSemV1 } from "../domain/internalProjectArchiveV6Wire";
import {
  INTERNAL_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_COMMAND_V1,
  INTERNAL_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_SURFACE_V1,
  parseInternalGeneralSemExecutionAuthorityRevisionNativeOutcomeV1,
  parseInternalGeneralSemExecutionAuthorityRevisionRequestV1,
  type GeneralSemExecutionAuthorityRevisionIdentityV1,
  type GeneralSemExecutionAuthorityRevisionReceiptV1,
  type GeneralSemExecutionAuthoritySourcePinV1,
  type InternalGeneralSemExecutionAuthorityRevisionDiagnosticV1,
  type InternalGeneralSemExecutionAuthorityRevisionRequestV1,
} from "../domain/internalGeneralSemExecutionAuthorityRevisionV1";
import type { AddGeneralSemInteractionV2EditorIntentV1 } from "../domain/standardSemModelV4Authority";
import { inspectInternalProjectArchiveV6At } from "./internalProjectArchiveV6ReadService";

export type InternalGeneralSemExecutionAuthorityRevisionPersistOutcomeV1 =
  | {
    status: "ok";
    value: {
      schemaVersion: 1;
      persistence: "persisted_new_revision";
      receipt: GeneralSemExecutionAuthorityRevisionReceiptV1;
      snapshot: InternalProjectArchiveV6ReadSnapshotV1;
    };
  }
  | {
    status: "blocked";
    diagnostic: InternalGeneralSemExecutionAuthorityRevisionDiagnosticV1;
    persistedReceipt?: GeneralSemExecutionAuthorityRevisionReceiptV1;
  };

function suggestedRevisionPath(sourceArchivePath: string, revisionNumberHint: number): string {
  const suffix = `-sem-revision-${revisionNumberHint}.qpls`;
  return /\.qpls$/i.test(sourceArchivePath)
    ? sourceArchivePath.replace(/\.qpls$/i, suffix)
    : `${sourceArchivePath}${suffix}`;
}

function blockedAfterPersist(
  receipt: GeneralSemExecutionAuthorityRevisionReceiptV1,
  code: string,
  message: string,
): InternalGeneralSemExecutionAuthorityRevisionPersistOutcomeV1 {
  return {
    status: "blocked",
    persistedReceipt: receipt,
    diagnostic: {
      code,
      message,
      correctiveAction: `The new revision is safely stored at ${receipt.destinationArchivePath}. Keep the current source active and open that saved revision explicitly after resolving the inspection issue.`,
    },
  };
}

export async function reviseInternalGeneralSemExecutionAuthorityAtV1(
  requestInput: InternalGeneralSemExecutionAuthorityRevisionRequestV1,
  dependencies: {
    invokeNative?: typeof invoke;
    inspectDestination?: typeof inspectInternalProjectArchiveV6At;
  } = {},
): Promise<InternalGeneralSemExecutionAuthorityRevisionPersistOutcomeV1> {
  const request = parseInternalGeneralSemExecutionAuthorityRevisionRequestV1(requestInput);
  const invokeNative = dependencies.invokeNative ?? invoke;
  const inspectDestination = dependencies.inspectDestination ?? inspectInternalProjectArchiveV6At;
  const response = await invokeNative<unknown>(
    INTERNAL_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_COMMAND_V1,
    { request },
  );
  const nativeOutcome = parseInternalGeneralSemExecutionAuthorityRevisionNativeOutcomeV1(response, request);
  if (nativeOutcome.status === "blocked") return nativeOutcome;
  const receipt = nativeOutcome.value.receipt;
  let inspected;
  try {
    inspected = await inspectDestination(receipt.destinationArchivePath);
  } catch (error) {
    return blockedAfterPersist(
      receipt,
      "schema6_general_sem_revision.destination_inspection_failed",
      error instanceof Error ? error.message : "The saved revision could not be inspected through the strict reader.",
    );
  }
  if (inspected.status === "blocked") {
    return blockedAfterPersist(
      receipt,
      "schema6_general_sem_revision.destination_inspection_blocked",
      inspected.diagnostic.message,
    );
  }
  const snapshot = inspected.value;
  const authority = snapshot.generalSemExecutionAuthority;
  const lineage = snapshot.project.layouts.general_sem_execution_authority_revision_v1 as Record<string, unknown> | undefined;
  const authorityMatches = Boolean(
    supportsGeneralSemV1(snapshot.project)
    && snapshot.archivePath === receipt.destinationArchivePath
    && snapshot.archiveSha256 === receipt.destinationArchiveSha256
    && snapshot.archiveBytes === receipt.destinationArchiveBytes
    && snapshot.project.project_id === receipt.projectId
    && snapshot.project.name === receipt.name
    && snapshot.project.datasets.length === 1
    && snapshot.project.models.length === 1
    && snapshot.project.recipes.length === 1
    && snapshot.project.historical_recipes.length === 0
    && snapshot.project.historical_results.length === 0
    && authority?.projectId === receipt.projectId
    && authority.datasetId === receipt.residentDatasetId
    && authority.datasetFingerprint === receipt.residentDatasetFingerprint
    && authority.modelId === receipt.residentModelId
    && authority.modelScientificSha256 === receipt.residentModelScientificSha256
    && authority.recipeId === receipt.residentRecipeId
    && authority.recipeDocumentSha256 === receipt.residentRecipeDocumentSha256
    && lineage?.schemaVersion === 1
    && lineage.revisionNumber === receipt.revisionNumber
    && (lineage.revised as Record<string, unknown> | undefined)?.projectId === receipt.projectId
    && (lineage.revised as Record<string, unknown> | undefined)?.modelDocumentSha256 === receipt.residentModelDocumentSha256
    && (lineage.compilation as Record<string, unknown> | undefined)?.compiledArtifactIdentitySha256 === receipt.compiledArtifactIdentitySha256
  );
  if (!authorityMatches) {
    return blockedAfterPersist(
      receipt,
      "schema6_general_sem_revision.destination_authority_mismatch",
      "The strict destination snapshot differs from the native model, RecipeV4, compiler, or lineage receipt.",
    );
  }
  return {
    status: "ok",
    value: { schemaVersion: 1, persistence: "persisted_new_revision", receipt, snapshot },
  };
}

export async function saveGeneralSemExecutionAuthorityRevisionV1(input: {
  snapshot: InternalProjectArchiveV6ReadSnapshotV1;
  source: GeneralSemExecutionAuthoritySourcePinV1;
  revision: GeneralSemExecutionAuthorityRevisionIdentityV1;
  intent: AddGeneralSemInteractionV2EditorIntentV1;
  revisionNumberHint?: number;
}) {
  const destinationArchivePath = await save({
    defaultPath: suggestedRevisionPath(input.snapshot.archivePath, input.revisionNumberHint ?? 1),
    filters: [{ name: "QuickPLS General SEM revision", extensions: ["qpls"] }],
  });
  if (!destinationArchivePath) return null;
  return reviseInternalGeneralSemExecutionAuthorityAtV1({
    surface: INTERNAL_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_SURFACE_V1,
    experimentalLabsEnabled: true,
    sourceArchivePath: input.snapshot.archivePath,
    expectedSourceArchiveSha256: input.snapshot.archiveSha256,
    destinationArchivePath,
    revision: { source: input.source, revision: input.revision, intent: input.intent },
  });
}
