import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { capabilityRegistryV2 } from "../domain/capabilityRegistryV2";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "../domain/internalProjectArchiveV6Read";
import { supportsGeneralSemV1 } from "../domain/internalProjectArchiveV6Wire";
import {
  GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1,
  type GeneralSemModeratedMediationSelectionReadyV1,
} from "../domain/generalSemModeratedMediationAuthoringV1";
import {
  GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V2_LAYOUT_KEY,
  INTERNAL_GENERAL_SEM_MODERATED_MEDIATION_REVISION_COMMAND_V2,
  InternalGeneralSemModeratedMediationRevisionWireError,
  parseGeneralSemExecutionAuthorityRevisionLineageV2,
  parseInternalGeneralSemModeratedMediationRevisionNativeOutcomeV2,
  parseInternalGeneralSemModeratedMediationRevisionRequestV2,
  type GeneralSemExecutionAuthorityRevisionLineageV2,
  type GeneralSemExecutionAuthorityRevisionReceiptV2,
  type InternalGeneralSemModeratedMediationRevisionRequestV2,
} from "../domain/internalGeneralSemModeratedMediationRevisionV2";
import type {
  GeneralSemExecutionAuthorityRevisionIdentityV1,
  GeneralSemExecutionAuthoritySourcePinV1,
  InternalGeneralSemExecutionAuthorityRevisionDiagnosticV1,
} from "../domain/internalGeneralSemExecutionAuthorityRevisionV1";
import { inspectInternalProjectArchiveV6At } from "./internalProjectArchiveV6ReadService";

const moderatedMediationRegistryMatches = capabilityRegistryV2.quickPlsCell(
  "qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap",
).filter(({ row }) => row.capability_id === "smartpls.mediation");
const moderatedMediationRegistryCell = moderatedMediationRegistryMatches.length === 1
  ? moderatedMediationRegistryMatches[0]!.cell
  : null;
const moderatedMediationExecutionAccess = moderatedMediationRegistryCell?.surface === "standard"
  ? {
    surface: "standard" as const,
    experimentalLabsEnabled: false as const,
    recipeExecutionSurface: "native_general_sem_pls_standard_v1" as const,
  }
  : moderatedMediationRegistryCell?.surface === "labs"
    ? {
      surface: "internal_labs" as const,
      experimentalLabsEnabled: true as const,
      recipeExecutionSurface: "native_general_sem_pls_labs_v1" as const,
    }
    : null;

/** The product route follows the exact shipped Registry cell on Standard or Labs. */
export const GENERAL_SEM_MODERATED_MEDIATION_PRODUCT_ROUTE_CONNECTED_V1 =
  moderatedMediationRegistryCell !== null
  && moderatedMediationRegistryCell.capability_version
    === "general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1"
  && (moderatedMediationRegistryCell.surface === "standard"
    || moderatedMediationRegistryCell.surface === "labs")
  && capabilityRegistryV2.availability(
    moderatedMediationRegistryCell.capability_id,
    moderatedMediationRegistryCell.cell_id,
    true,
  ).selectable;

export const GENERAL_SEM_MODERATED_MEDIATION_PRODUCT_ROUTE_REQUIRES_LABS_V1 =
  moderatedMediationRegistryCell?.surface === "labs";

export type InternalGeneralSemModeratedMediationRevisionDiagnosticV1 =
  InternalGeneralSemExecutionAuthorityRevisionDiagnosticV1;

export type InternalGeneralSemModeratedMediationRevisionPersistOutcomeV2 =
  | {
    status: "ok";
    value: {
      schemaVersion: 2;
      persistence: "persisted_new_revision";
      receipt: GeneralSemExecutionAuthorityRevisionReceiptV2;
      lineage: GeneralSemExecutionAuthorityRevisionLineageV2;
      snapshot: InternalProjectArchiveV6ReadSnapshotV1;
    };
  }
  | {
    status: "blocked";
    diagnostic: InternalGeneralSemModeratedMediationRevisionDiagnosticV1;
    persistedReceipt?: GeneralSemExecutionAuthorityRevisionReceiptV2;
  };

export interface InternalGeneralSemModeratedMediationRevisionTransactionV2 {
  snapshot: InternalProjectArchiveV6ReadSnapshotV1;
  source: GeneralSemExecutionAuthoritySourcePinV1;
  revision: GeneralSemExecutionAuthorityRevisionIdentityV1;
  selection: GeneralSemModeratedMediationSelectionReadyV1;
  destinationArchivePath: string;
}

interface RevisionDependenciesV2 {
  invokeNative?: (command: string, args: Record<string, unknown>) => Promise<unknown>;
  inspectDestination?: typeof inspectInternalProjectArchiveV6At;
  isSourceSessionCurrent?: (snapshot: InternalProjectArchiveV6ReadSnapshotV1) => boolean;
}

export interface SaveInternalGeneralSemModeratedMediationRevisionInputV2 {
  snapshot: InternalProjectArchiveV6ReadSnapshotV1;
  source: GeneralSemExecutionAuthoritySourcePinV1;
  revision: GeneralSemExecutionAuthorityRevisionIdentityV1;
  selection: GeneralSemModeratedMediationSelectionReadyV1;
  revisionNumberHint: number;
  isSourceSessionCurrent?: (snapshot: InternalProjectArchiveV6ReadSnapshotV1) => boolean;
}

interface SaveRevisionDependenciesV2 extends RevisionDependenciesV2 {
  selectDestination?: (suggestedPath: string) => Promise<string | null>;
}

function suggestedRevisionPath(sourceArchivePath: string, revisionNumberHint: number): string {
  const suffix = `-moderated-mediation-revision-${revisionNumberHint}.qpls`;
  return /\.qpls$/i.test(sourceArchivePath)
    ? sourceArchivePath.replace(/\.qpls$/i, suffix)
    : `${sourceArchivePath}${suffix}`;
}

function blocked(
  code: string,
  message: string,
  correctiveAction: string,
  persistedReceipt?: GeneralSemExecutionAuthorityRevisionReceiptV2,
): InternalGeneralSemModeratedMediationRevisionPersistOutcomeV2 {
  return {
    status: "blocked",
    ...(persistedReceipt ? { persistedReceipt } : {}),
    diagnostic: { code, message, correctiveAction },
  };
}

function blockedAfterPersist(
  receipt: GeneralSemExecutionAuthorityRevisionReceiptV2,
  code: string,
  message: string,
): InternalGeneralSemModeratedMediationRevisionPersistOutcomeV2 {
  return blocked(
    code,
    message,
    `The new revision is safely stored at ${receipt.destinationArchivePath}. Keep the current source session active and open that exact saved revision explicitly after resolving the verification issue.`,
    receipt,
  );
}

function selectionMatchesReceipt(
  selection: GeneralSemModeratedMediationSelectionReadyV1,
  receipt: GeneralSemExecutionAuthorityRevisionReceiptV2,
): boolean {
  const target = receipt.compiledTarget;
  const selected = selection.selectedPath;
  return target.estimand_id === selected.estimandId
    && target.specific_path_identity === selected.pathId
    && target.ordered_relation_ids[0] === selected.orderedRelationIds[0]
    && target.ordered_relation_ids[1] === selected.orderedRelationIds[1]
    && target.x_id === selected.xId
    && target.mediator_id === selected.mediatorId
    && target.y_id === selected.yId
    && target.moderator_id === selected.moderatorId
    && target.interaction_id === selected.interactionId
    && target.moderated_stage === selected.moderatedStage
    && target.moderated_relation_id === selected.moderatedRelationId
    && target.other_stage_relation_id === selected.otherStageRelationId;
}

function sameWindowsPath(left: string, right: string): boolean {
  return left.toLowerCase() === right.toLowerCase();
}

function sourceSnapshotMatches(
  transaction: InternalGeneralSemModeratedMediationRevisionTransactionV2,
): boolean {
  const { snapshot, source } = transaction;
  const authority = snapshot.generalSemExecutionAuthority;
  return supportsGeneralSemV1(snapshot.project)
    && snapshot.sourceRecheckedUnchanged
    && !sameWindowsPath(snapshot.archivePath, transaction.destinationArchivePath)
    && snapshot.project.project_id === source.projectId
    && authority?.projectId === source.projectId
    && authority.modelId === source.modelId
    && authority.modelScientificSha256 === source.modelScientificSha256
    && authority.recipeId === source.recipeId
    && authority.recipeDocumentSha256 === source.recipeDocumentSha256;
}

export function buildInternalGeneralSemModeratedMediationRevisionRequestV2(
  transaction: InternalGeneralSemModeratedMediationRevisionTransactionV2,
): InternalGeneralSemModeratedMediationRevisionRequestV2 {
  if (!moderatedMediationExecutionAccess) {
    throw new InternalGeneralSemModeratedMediationRevisionWireError(
      "schema6_general_sem_revision_v2.capability_unavailable",
      "transaction",
      "The exact moderated-mediation capability cell has no executable Registry surface.",
    );
  }
  if (!sourceSnapshotMatches(transaction)) {
    throw new InternalGeneralSemModeratedMediationRevisionWireError(
      "schema6_general_sem_revision_v2.source_session_stale",
      "transaction.snapshot",
      "The active strict source snapshot differs from the pinned model or RecipeV4 authority.",
    );
  }
  const selected = transaction.selection.selectedPath;
  return parseInternalGeneralSemModeratedMediationRevisionRequestV2({
    surface: moderatedMediationExecutionAccess.surface,
    experimentalLabsEnabled: moderatedMediationExecutionAccess.experimentalLabsEnabled,
    sourceArchivePath: transaction.snapshot.archivePath,
    expectedSourceArchiveSha256: transaction.snapshot.archiveSha256,
    destinationArchivePath: transaction.destinationArchivePath,
    revision: {
      source: transaction.source,
      revision: transaction.revision,
      intent: {
        kind: "select_two_way_moderated_mediation_path",
        intent_version: 2,
        sem_generation: "general_sem_v1",
        estimand_id: selected.estimandId,
        ordered_relation_ids: [...selected.orderedRelationIds],
      },
      expectedCapabilityCell: GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1,
      recipeExecutionSurface: moderatedMediationExecutionAccess.recipeExecutionSurface,
    },
  });
}

function destinationSnapshotMatches(
  snapshot: InternalProjectArchiveV6ReadSnapshotV1,
  receipt: GeneralSemExecutionAuthorityRevisionReceiptV2,
): boolean {
  const authority = snapshot.generalSemExecutionAuthority;
  const config = authority?.recipe.general_sem_config;
  const [effect] = config?.requested_effect_estimands ?? [];
  return supportsGeneralSemV1(snapshot.project)
    && snapshot.sourceRecheckedUnchanged
    && sameWindowsPath(snapshot.archivePath, receipt.destinationArchivePath)
    && snapshot.archiveSha256 === receipt.destinationArchiveSha256
    && snapshot.archiveBytes === receipt.destinationArchiveBytes
    && snapshot.project.project_id === receipt.projectId
    && snapshot.project.name === receipt.name
    && snapshot.project.datasets.length === 1
    && snapshot.project.models.length === 1
    && snapshot.project.recipes.length === 1
    && snapshot.project.historical_recipes.length === 0
    && snapshot.project.historical_results.length === 0
    && snapshot.project.canonical_result_documents.length === 0
    && authority?.projectId === receipt.projectId
    && authority.datasetId === receipt.residentDatasetId
    && authority.datasetFingerprint === receipt.residentDatasetFingerprint
    && authority.modelId === receipt.residentModelId
    && authority.modelScientificSha256 === receipt.residentModelScientificSha256
    && authority.recipeId === receipt.residentRecipeId
    && authority.recipeDocumentSha256 === receipt.residentRecipeDocumentSha256
    && config?.requested_effect_estimands.length === 1
    && effect?.kind === "specific_path"
    && effect.estimand_id === receipt.compiledTarget.estimand_id
    && effect.ordered_relation_ids.length === 2
    && effect.ordered_relation_ids[0] === receipt.compiledTarget.ordered_relation_ids[0]
    && effect.ordered_relation_ids[1] === receipt.compiledTarget.ordered_relation_ids[1];
}

export async function reviseInternalGeneralSemModeratedMediationAtV2(
  transaction: InternalGeneralSemModeratedMediationRevisionTransactionV2,
  dependencies: RevisionDependenciesV2 = {},
): Promise<InternalGeneralSemModeratedMediationRevisionPersistOutcomeV2> {
  let request: InternalGeneralSemModeratedMediationRevisionRequestV2;
  try {
    request = buildInternalGeneralSemModeratedMediationRevisionRequestV2(transaction);
  } catch (error) {
    return blocked(
      error instanceof InternalGeneralSemModeratedMediationRevisionWireError
        ? error.code
        : "schema6_general_sem_revision_v2.request_invalid",
      error instanceof Error ? error.message : "The moderated-mediation revision request is invalid.",
      "Keep the source unchanged, refresh the strict session, and rebuild the exact selected-path revision.",
    );
  }
  const isSourceSessionCurrent = dependencies.isSourceSessionCurrent ?? (() => true);
  if (!isSourceSessionCurrent(transaction.snapshot)) {
    return blocked(
      "schema6_general_sem_revision_v2.source_session_stale",
      "The active source session changed before native persistence began.",
      "Keep the source unchanged, refresh its strict snapshot, and choose a new revision destination.",
    );
  }
  const invokeNative = dependencies.invokeNative
    ?? ((command, args) => invoke<unknown>(command, args));
  let response: unknown;
  try {
    response = await invokeNative(
      INTERNAL_GENERAL_SEM_MODERATED_MEDIATION_REVISION_COMMAND_V2,
      { request },
    );
  } catch (error) {
    return blocked(
      "schema6_general_sem_revision_v2.native_transport_failed",
      error instanceof Error ? error.message : "The native revision-v2 transport failed.",
      "Keep the source active. Inspect the chosen destination before retrying so an already-published revision is never overwritten.",
    );
  }
  let nativeOutcome;
  try {
    nativeOutcome = parseInternalGeneralSemModeratedMediationRevisionNativeOutcomeV2(response, request);
  } catch (error) {
    return blocked(
      error instanceof InternalGeneralSemModeratedMediationRevisionWireError
        ? error.code
        : "schema6_general_sem_revision_v2.response_invalid",
      error instanceof Error ? error.message : "The native revision-v2 response is invalid.",
      "Keep the source active and inspect the selected destination before retrying the no-replace transaction.",
    );
  }
  if (nativeOutcome.status === "blocked") return nativeOutcome;
  const receipt = nativeOutcome.value.receipt;
  if (!selectionMatchesReceipt(transaction.selection, receipt)) {
    return blockedAfterPersist(
      receipt,
      "schema6_general_sem_revision_v2.compiled_target_mismatch",
      "The persisted compiled target differs from the exact selected first- or second-stage path.",
    );
  }
  if (!isSourceSessionCurrent(transaction.snapshot)) {
    return blockedAfterPersist(
      receipt,
      "schema6_general_sem_revision_v2.source_session_changed_after_persist",
      "The source session changed after the new archive was persisted, so QuickPLS did not replace or activate any authority.",
    );
  }
  const inspectDestination = dependencies.inspectDestination ?? inspectInternalProjectArchiveV6At;
  let inspected;
  try {
    inspected = await inspectDestination(receipt.destinationArchivePath);
  } catch (error) {
    return blockedAfterPersist(
      receipt,
      "schema6_general_sem_revision_v2.destination_inspection_failed",
      error instanceof Error ? error.message : "The saved revision could not be inspected through the strict reader.",
    );
  }
  if (inspected.status === "blocked") {
    return blockedAfterPersist(
      receipt,
      "schema6_general_sem_revision_v2.destination_inspection_blocked",
      inspected.diagnostic.message,
    );
  }
  if (!destinationSnapshotMatches(inspected.value, receipt)) {
    return blockedAfterPersist(
      receipt,
      "schema6_general_sem_revision_v2.destination_authority_mismatch",
      "The strict destination snapshot differs from the native model, RecipeV4, archive, or selected-path receipt.",
    );
  }
  let lineage: GeneralSemExecutionAuthorityRevisionLineageV2;
  try {
    lineage = parseGeneralSemExecutionAuthorityRevisionLineageV2(
      inspected.value.project.layouts[GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V2_LAYOUT_KEY],
      request,
      receipt,
    );
  } catch (error) {
    return blockedAfterPersist(
      receipt,
      error instanceof InternalGeneralSemModeratedMediationRevisionWireError
        ? error.code
        : "schema6_general_sem_revision_v2.lineage_invalid",
      error instanceof Error ? error.message : "The saved revision-v2 lineage is invalid.",
    );
  }
  if (!isSourceSessionCurrent(transaction.snapshot)) {
    return blockedAfterPersist(
      receipt,
      "schema6_general_sem_revision_v2.source_session_changed_after_verification",
      "The source session changed during destination verification, so QuickPLS left both archives inactive and unchanged.",
    );
  }
  return {
    status: "ok",
    value: {
      schemaVersion: 2,
      persistence: "persisted_new_revision",
      receipt,
      lineage,
      snapshot: inspected.value,
    },
  };
}

/** Native Save As entrypoint for the exact Registry-authorized route. */
export async function saveInternalGeneralSemModeratedMediationRevisionV1(
  input: SaveInternalGeneralSemModeratedMediationRevisionInputV2,
  dependencies: SaveRevisionDependenciesV2 = {},
): Promise<InternalGeneralSemModeratedMediationRevisionPersistOutcomeV2 | null> {
  if (!GENERAL_SEM_MODERATED_MEDIATION_PRODUCT_ROUTE_CONNECTED_V1) {
    return blocked(
      "general_sem.moderated_mediation.registry_cell_unavailable",
      "The exact moderated-mediation capability cell is not selectable in the shipped Registry.",
      "Keep the source archive unchanged and restore the exact Registry-authorized cell before retrying.",
    );
  }
  const selectDestination = dependencies.selectDestination ?? (async (suggestedPath: string) => {
    const selected = await save({
      defaultPath: suggestedPath,
      filters: [{ name: "QuickPLS General SEM project", extensions: ["qpls"] }],
    });
    return typeof selected === "string" ? selected : null;
  });
  const destinationArchivePath = await selectDestination(suggestedRevisionPath(
    input.snapshot.archivePath,
    input.revisionNumberHint,
  ));
  if (destinationArchivePath === null) return null;
  const isSourceSessionCurrent = input.isSourceSessionCurrent
    ?? dependencies.isSourceSessionCurrent;
  return reviseInternalGeneralSemModeratedMediationAtV2({
    snapshot: input.snapshot,
    source: input.source,
    revision: input.revision,
    selection: input.selection,
    destinationArchivePath,
  }, {
    ...(dependencies.invokeNative ? { invokeNative: dependencies.invokeNative } : {}),
    ...(dependencies.inspectDestination ? { inspectDestination: dependencies.inspectDestination } : {}),
    ...(isSourceSessionCurrent ? { isSourceSessionCurrent } : {}),
  });
}
