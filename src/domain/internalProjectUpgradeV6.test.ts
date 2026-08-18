import { describe, expect, it } from "vitest";
import {
  projectUpgradeNeedsEstimandConfirmation,
  projectUpgradePlanIsReady,
  type ProjectUpgradeInspectionV1,
  type ProjectUpgradeOutcomeV1,
  type ProjectUpgradePlanStateV1,
} from "./internalProjectUpgradeV6";

describe("internal project upgrade v6 state model", () => {
  it("distinguishes an executable plan from a confirmation prompt", () => {
    const confirmation: ProjectUpgradeOutcomeV1<ProjectUpgradePlanStateV1> = {
      status: "ok",
      value: {
        state: "confirmation_required",
        sourceArchiveSha256: "a".repeat(64),
        destinationArchivePath: "D:\\study-v6.qpls",
        prompts: [
          {
            modelId: "model-1",
            modelName: "Research model",
            choices: ["composite", "common_factor"],
          },
        ],
        sourceWillRemainUnchanged: true,
        destinationMustBeNew: true,
        historicalResultsImmutable: true,
      },
    };
    expect(projectUpgradeNeedsEstimandConfirmation(confirmation)).toBe(true);
    expect(projectUpgradePlanIsReady(confirmation)).toBe(false);

    const ready: ProjectUpgradeOutcomeV1<ProjectUpgradePlanStateV1> = {
      status: "ok",
      value: {
        state: "ready",
        planId: "plan-1",
        planSha256: "b".repeat(64),
        sourceArchiveSha256: "a".repeat(64),
        destinationArchivePath: "D:\\study-v6.qpls",
        modelCount: 1,
        recipeCount: 1,
        historicalResultCount: 1,
        sourceWillRemainUnchanged: true,
        destinationMustBeNew: true,
        historicalResultsImmutable: true,
      },
    };
    expect(projectUpgradePlanIsReady(ready)).toBe(true);
    expect(projectUpgradeNeedsEstimandConfirmation(ready)).toBe(false);
  });

  it("keeps a current schema-6 ZIP archive read-only and distinct from a standalone document", () => {
    const inspection: ProjectUpgradeInspectionV1 = {
      sourceArchivePath: "D:\\study-v6.qpls",
      sourceArchiveSha256: "a".repeat(64),
      sourceKind: "project_archive",
      schemaVersion: 6,
      access: "current_v6_archive",
      readOnly: true,
      upgradeAvailable: false,
      projectId: "00000000-0000-0000-0000-000000000001",
      projectName: "Study",
      counts: { datasets: 1, models: 1, recipes: 1, results: 1 },
      futureUnsupported: { models: 0, recipes: 0, results: 0 },
      sourceWillRemainUnchanged: true,
      destinationMustBeNew: true,
    };

    expect(inspection.access).toBe("current_v6_archive");
    expect(inspection.sourceKind).toBe("project_archive");
    expect(inspection.readOnly).toBe(true);
    expect(inspection.upgradeAvailable).toBe(false);
  });
});
