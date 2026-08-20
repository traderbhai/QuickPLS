import type { GeneralSemModeratedMediationSelectionReadyV1 } from "../domain/generalSemModeratedMediationAuthoringV1";

/**
 * Product admission remains false until revision-v2 native persistence and
 * Registry promotion are independently connected. Keeping this constant here
 * prevents a prepared UI from becoming an executable feature by accident.
 */
export const GENERAL_SEM_MODERATED_MEDIATION_PRODUCT_ROUTE_CONNECTED_V1 = false as const;

export interface InternalGeneralSemModeratedMediationRevisionDiagnosticV1 {
  readonly code: "general_sem.moderated_mediation.native_revision_not_connected";
  readonly message: string;
  readonly correctiveAction: string;
}

export type InternalGeneralSemModeratedMediationRevisionOutcomeV1 = {
  readonly status: "blocked";
  readonly diagnostic: InternalGeneralSemModeratedMediationRevisionDiagnosticV1;
};

/**
 * Fail-closed service seam for the future source-preserving model + Recipe
 * save-as revision. This function deliberately performs no native invocation,
 * file selection, archive write, or active-authority replacement.
 */
export async function saveInternalGeneralSemModeratedMediationRevisionV1(
  selection: GeneralSemModeratedMediationSelectionReadyV1,
): Promise<InternalGeneralSemModeratedMediationRevisionOutcomeV1> {
  void selection;
  return {
    status: "blocked",
    diagnostic: {
      code: "general_sem.moderated_mediation.native_revision_not_connected",
      message: "The moderated-mediation model-and-Recipe revision is prepared but not connected to native persistence.",
      correctiveAction: "Keep the source archive and authored diagram unchanged until revision-v2 persistence and the exact supplemental cell are admitted together.",
    },
  };
}
