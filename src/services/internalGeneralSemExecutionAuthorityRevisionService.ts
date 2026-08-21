import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "../domain/internalProjectArchiveV6Read";
import { supportsGeneralSemV1 } from "../domain/internalProjectArchiveV6Wire";
import {
  INTERNAL_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_COMMAND_V1,
  GENERAL_SEM_PLS_LABS_REVISION_RECIPE_EXECUTION_SURFACE_V1,
  GENERAL_SEM_PLS_STANDARD_REVISION_RECIPE_EXECUTION_SURFACE_V1,
  parseInternalGeneralSemExecutionAuthorityRevisionNativeOutcomeV1,
  parseInternalGeneralSemExecutionAuthorityRevisionRequestV1,
  type GeneralSemExecutionAuthorityRevisionIdentityV1,
  type GeneralSemExecutionAuthorityRevisionEditorIntentV1,
  type GeneralSemExecutionAuthorityRevisionReceiptV1,
  type GeneralSemExecutionAuthoritySourcePinV1,
  type InternalGeneralSemExecutionAuthorityRevisionDiagnosticV1,
  type InternalGeneralSemExecutionAuthorityRevisionRequestV1,
} from "../domain/internalGeneralSemExecutionAuthorityRevisionV1";
import { parseGeneralSemConfigV1 } from "../domain/generalSemConfigV1";
import {
  GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_HIGHER_ORDER_POINT_CAPABILITY_CELL_V1,
  selectGeneralSemExecutionAccessV1,
  type GeneralSemCapabilityRegistryReaderV1,
} from "../domain/internalRecipeV4GeneralSemWorkspace";
import { GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1 } from "../domain/generalSemCapabilityPreflightV1";
import { capabilityRegistryV2 } from "../domain/capabilityRegistryV2";
import type { SemModelV4 } from "../domain/semModelV4";
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
    && authority.recipe.metadata.execution_surface === request.revision.recipeExecutionSurface
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

export function selectGeneralSemRevisionExecutionV1(input: {
  snapshot: InternalProjectArchiveV6ReadSnapshotV1;
  intent?: GeneralSemExecutionAuthorityRevisionEditorIntentV1;
  experimentalLabsEnabled: boolean;
  capabilityRegistry?: GeneralSemCapabilityRegistryReaderV1;
}) {
  const authority = input.snapshot.generalSemExecutionAuthority;
  if (!authority?.recipe.general_sem_config) {
    throw new Error("The strict General SEM revision source has no resident GeneralSemConfigV1 authority.");
  }
  const config = parseGeneralSemConfigV1(authority.recipe.general_sem_config);
  const modelRecord = input.snapshot.project.models.find((candidate) => (
    candidate.model_id === authority.modelId
  ));
  if (!modelRecord || modelRecord.payload.kind !== "sem_model_v4") {
    throw new Error("The strict General SEM revision source has no resident executable SemModelV4 authority.");
  }
  const model = modelRecord.payload.model;
  const higherOrderIntent = input.intent?.kind === "add_higher_order"
    || input.intent?.kind === "replace_higher_order";
  const removedHigherOrderIdentity = input.intent?.kind === "remove_higher_order"
    ? { termId: input.intent.term_id, outputId: input.intent.output_id }
    : null;
  const projectedInteractionOrders = model.derived_terms.flatMap((term) => (
    term.kind === "interaction_v2"
      && !(input.intent?.kind === "replace_moderating_effect" || input.intent?.kind === "remove_moderating_effect"
        ? term.id === input.intent.term_id
        : false)
      ? [term.operands.length]
      : []
  ));
  if (input.intent?.kind === "add_general_sem_interaction_v2") projectedInteractionOrders.push(2);
  if (input.intent?.kind === "add_moderating_effect_v3"
    || input.intent?.kind === "replace_moderating_effect") {
    projectedInteractionOrders.push(input.intent.operands.length);
  }
  const hasHigherOrder = higherOrderIntent
    || model.derived_terms.some((term) => (
      term.kind === "higher_order"
      && !(removedHigherOrderIdentity
        && term.id === removedHigherOrderIdentity.termId
        && term.output === removedHigherOrderIdentity.outputId)
    ));
  const hasThreeWay = projectedInteractionOrders.includes(3);
  const hasTwoWay = projectedInteractionOrders.includes(2);
  const indirectPathCount = countScientificIndirectPathsV1(model);
  const expectedCapabilityCell = hasHigherOrder
    ? config.inference.kind === "case_bootstrap"
      ? GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_CAPABILITY_CELL_V1
      : GENERAL_SEM_PLS_HIGHER_ORDER_POINT_CAPABILITY_CELL_V1
    : hasThreeWay
      ? config.inference.kind === "case_bootstrap"
        ? GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1
        : GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1
    : hasTwoWay
      ? config.inference.kind === "case_bootstrap"
        ? config.requested_effect_estimands.length === 1
          ? GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1
          : GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1
        : GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1
    : config.inference.kind === "case_bootstrap"
      ? indirectPathCount === 1
        ? GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_CAPABILITY_CELL_V1
        : GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1
      : GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1;
  const access = selectGeneralSemExecutionAccessV1({
    capabilityCell: expectedCapabilityCell,
    experimentalLabsEnabled: input.experimentalLabsEnabled,
    registry: input.capabilityRegistry ?? capabilityRegistryV2,
  });
  const recipeExecutionSurface = access.surface === "standard"
    ? GENERAL_SEM_PLS_STANDARD_REVISION_RECIPE_EXECUTION_SURFACE_V1
    : GENERAL_SEM_PLS_LABS_REVISION_RECIPE_EXECUTION_SURFACE_V1;
  return { access, expectedCapabilityCell, recipeExecutionSurface } as const;
}

function countScientificIndirectPathsV1(model: SemModelV4): number {
  const variables = new Set(model.variables.flatMap((variable) => {
    if (variable.kind === "common_factor" || variable.kind === "composite") return [variable.id];
    if (variable.kind === "observed" && (variable.role === "structural" || variable.role === "both")) {
      return [variable.id];
    }
    return [];
  }));
  const generatedHierarchyRelationIds = new Set(model.annotations.flatMap((annotation) => (
    annotation.kind === "note"
      && annotation.id.startsWith("general-sem:v1:interaction-generated:")
      ? [annotation.subject]
      : []
  )));
  const outgoing = new Map<string, Array<{ target: string }>>();
  for (const relation of model.relations) {
    if (relation.kind !== "structural"
      || (relation.role ?? "structural") !== "structural"
      || generatedHierarchyRelationIds.has(relation.id)
      || !variables.has(relation.source)
      || !variables.has(relation.target)) continue;
    const bucket = outgoing.get(relation.source) ?? [];
    bucket.push({ target: relation.target });
    outgoing.set(relation.source, bucket);
  }
  let count = 0;
  const visit = (source: string, visited: ReadonlySet<string>, depth: number): void => {
    for (const relation of outgoing.get(source) ?? []) {
      if (visited.has(relation.target)) continue;
      const nextDepth = depth + 1;
      if (nextDepth >= 2) count += 1;
      visit(relation.target, new Set([...visited, relation.target]), nextDepth);
    }
  };
  for (const source of outgoing.keys()) visit(source, new Set([source]), 0);
  return count;
}

export async function saveGeneralSemExecutionAuthorityRevisionV1(input: {
  snapshot: InternalProjectArchiveV6ReadSnapshotV1;
  source: GeneralSemExecutionAuthoritySourcePinV1;
  revision: GeneralSemExecutionAuthorityRevisionIdentityV1;
  intent: GeneralSemExecutionAuthorityRevisionEditorIntentV1;
  revisionNumberHint?: number;
  experimentalLabsEnabled: boolean;
  capabilityRegistry?: GeneralSemCapabilityRegistryReaderV1;
}) {
  const { access, expectedCapabilityCell, recipeExecutionSurface } =
    selectGeneralSemRevisionExecutionV1({
    snapshot: input.snapshot,
    intent: input.intent,
    experimentalLabsEnabled: input.experimentalLabsEnabled,
    capabilityRegistry: input.capabilityRegistry,
  });
  const destinationArchivePath = await save({
    defaultPath: suggestedRevisionPath(input.snapshot.archivePath, input.revisionNumberHint ?? 1),
    filters: [{ name: "QuickPLS General SEM revision", extensions: ["qpls"] }],
  });
  if (!destinationArchivePath) return null;
  return reviseInternalGeneralSemExecutionAuthorityAtV1({
    ...access,
    sourceArchivePath: input.snapshot.archivePath,
    expectedSourceArchiveSha256: input.snapshot.archiveSha256,
    destinationArchivePath,
    revision: {
      source: input.source,
      revision: input.revision,
      intent: input.intent,
      expectedCapabilityCell,
      recipeExecutionSurface,
    },
  });
}
