export const INTERNAL_PROJECT_UPGRADE_V6_SURFACE = "internal_labs" as const;

export type ProjectUpgradeOutcomeV1<T> =
  | { status: "ok"; value: T }
  | { status: "blocked"; diagnostic: ProjectUpgradeDiagnosticV1 };

export interface ProjectUpgradeDiagnosticV1 {
  code: string;
  message: string;
  correctiveAction: string;
  objectId?: string;
}

export interface ProjectUpgradeItemCountsV1 {
  datasets: number;
  models: number;
  recipes: number;
  results: number;
}

export interface ProjectUpgradeInspectionV1 {
  sourceArchivePath: string;
  sourceArchiveSha256: string;
  sourceKind: "project_archive" | "standalone_document";
  schemaVersion: number;
  access:
    | "historical_upgrade_copy_required"
    | "current_v6_archive"
    | "current_v6_standalone"
    | "future_read_only";
  readOnly: boolean;
  upgradeAvailable: boolean;
  projectId?: string;
  projectName?: string;
  counts: ProjectUpgradeItemCountsV1;
  futureUnsupported: {
    models: number;
    recipes: number;
    results: number;
  };
  sourceWillRemainUnchanged: true;
  destinationMustBeNew: true;
}

export interface ProjectUpgradeLegacyDisplayCovarianceV1 {
  id: string;
  leftConstruct: string;
  rightConstruct: string;
  label?: string | null;
}

export type ProjectUpgradeEstimandChoiceV1 = "composite" | "common_factor";

export interface ProjectUpgradePlanInputV1 {
  sourceArchivePath: string;
  destinationArchivePath: string;
  expectedSourceArchiveSha256: string;
  legacyDisplayCovariances?: Record<
    string,
    ProjectUpgradeLegacyDisplayCovarianceV1[]
  >;
  estimandConfirmations?: Record<string, ProjectUpgradeEstimandChoiceV1>;
}

export type ProjectUpgradePlanStateV1 =
  | {
      state: "confirmation_required";
      sourceArchiveSha256: string;
      destinationArchivePath: string;
      prompts: Array<{
        modelId: string;
        modelName: string;
        choices: ["composite", "common_factor"];
        conversionBlocker?: string;
      }>;
      sourceWillRemainUnchanged: true;
      destinationMustBeNew: true;
      historicalResultsImmutable: true;
    }
  | {
      state: "ready";
      planId: string;
      planSha256: string;
      sourceArchiveSha256: string;
      destinationArchivePath: string;
      modelCount: number;
      recipeCount: number;
      historicalResultCount: number;
      sourceWillRemainUnchanged: true;
      destinationMustBeNew: true;
      historicalResultsImmutable: true;
    };

export interface ProjectUpgradeExecutionV1 {
  planId: string;
  receipt: {
    write: {
      schemaVersion: number;
      projectId: string;
      destinationArchivePath: string;
      documentSha256: string;
      byteLength: number;
      postWriteValidated: boolean;
    };
    sourceArchivePath: string;
    sourceArchiveSha256: string;
    sourceVerifiedUnchanged: boolean;
    historicalResultsImmutable: boolean;
  };
  destinationWritten: true;
}

export interface ProjectUpgradeCancellationV1 {
  planId: string;
  destinationArchivePath: string;
  cancelled: true;
  destinationWritten: false;
}

export const projectUpgradePlanIsReady = (
  outcome: ProjectUpgradeOutcomeV1<ProjectUpgradePlanStateV1>,
): outcome is {
  status: "ok";
  value: Extract<ProjectUpgradePlanStateV1, { state: "ready" }>;
} => outcome.status === "ok" && outcome.value.state === "ready";

export const projectUpgradeNeedsEstimandConfirmation = (
  outcome: ProjectUpgradeOutcomeV1<ProjectUpgradePlanStateV1>,
): outcome is {
  status: "ok";
  value: Extract<ProjectUpgradePlanStateV1, { state: "confirmation_required" }>;
} =>
  outcome.status === "ok" && outcome.value.state === "confirmation_required";
