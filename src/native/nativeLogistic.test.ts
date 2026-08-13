import { describe, expect, it, vi } from "vitest";
import type { AnalysisUiSettings, Dataset } from "../types";
import {
  NATIVE_LOGISTIC_PROFILE_PAGE_SIZE,
  nativeLogisticReadiness,
  parseNativeLogisticProfile,
  profileNativeLogisticDataset,
  residentNativeLogisticProfile,
} from "./nativeLogistic";

const settings: AnalysisUiSettings = {
  method: "regression",
  weightingScheme: "path",
  preprocessing: "unstandardized",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20_260_812,
  workers: 1,
  confidenceLevel: 0.95,
  regressionType: "logistic",
  regressionOutcome: "converted",
  regressionPredictors: "score",
  regressionControls: "age",
};

function dataset(rows: Dataset["rows"], rowCount = rows.length): Dataset {
  return {
    id: "logistic-data",
    name: "logistic.csv",
    columns: ["converted", "score", "age", "segment"],
    rows,
    rowCount,
    missing: 0,
    fingerprint: "sha256:logistic",
    kind: "raw",
    columnMetadata: ["converted", "score", "age"].map((name) => ({
      name,
      label: null,
      column_type: "numeric" as const,
      scale_type: "continuous" as const,
      missing_markers: [],
      theoretical_min: null,
      theoretical_max: null,
      value_labels: {},
    })),
  };
}

describe("native binary logistic profiling", () => {
  it("profiles exact 0/1 complete cases and retains only aggregate readiness evidence", () => {
    const input = dataset([
      { converted: 0, score: 1, age: 22 },
      { converted: 1, score: 2, age: 25 },
      { converted: "0", score: "3", age: "31" },
      { converted: 1, score: null, age: 40 },
      { converted: null, score: 5, age: 44 },
    ]);
    const profile = residentNativeLogisticProfile(input, settings);
    expect(profile).toEqual({
      datasetId: "logistic-data",
      datasetFingerprint: "sha256:logistic",
      outcome: "converted",
      predictors: ["score"],
      controls: ["age"],
      expectedRows: 5,
      scannedRows: 5,
      completeCases: 3,
      omittedRows: 2,
      zeroCases: 2,
      oneCases: 1,
      invalidOutcomeRows: 0,
      constantTerms: [],
    });
    expect(nativeLogisticReadiness(input, settings, profile)).toMatchObject({
      canRun: false,
      blockers: ["Binary logistic regression requires at least 4 complete finite rows for 2 fitted terms"],
    });
  });

  it("blocks non-binary values, single complete-case classes, and constant terms", () => {
    const input = dataset([
      { converted: 0, score: 1, age: 22 },
      { converted: 0, score: 2, age: 22 },
      { converted: 2, score: 3, age: 22 },
      { converted: 0, score: 4, age: 22 },
    ]);
    const assessment = nativeLogisticReadiness(input, settings);
    expect(assessment.canRun).toBe(false);
    expect(assessment.blockers).toEqual(expect.arrayContaining([
      "1 non-missing outcome row is not coded exactly 0 or 1",
      "The listwise-complete outcome must contain both class 0 and class 1",
      "age is constant after listwise deletion",
    ]));
  });

  it("profiles invalid coding only inside the estimator's listwise-complete sample", () => {
    const input = dataset([
      { converted: 0, score: 1, age: 20 },
      { converted: 1, score: 2, age: 21 },
      { converted: 2, score: null, age: 22 },
      { converted: 1, score: 4, age: 23 },
    ]);
    const profile = residentNativeLogisticProfile(input, settings)!;
    expect(profile).toMatchObject({
      completeCases: 3,
      omittedRows: 1,
      zeroCases: 1,
      oneCases: 2,
      invalidOutcomeRows: 0,
    });
    expect(nativeLogisticReadiness(input, settings, profile).blockers).not.toContainEqual(expect.stringContaining("not coded exactly"));
  });

  it("reads a nonresident dataset in bounded sequential pages without collecting pages", async () => {
    const rows = Array.from({ length: 1_001 }, (_, index) => ({
      converted: index % 2,
      score: index + 1,
      age: 20 + index % 60,
    }));
    const input = dataset(rows.slice(0, 100), rows.length);
    const readPage = vi.fn(async (datasetId: string, offset: number, limit: number) => ({
      datasetId,
      offset,
      limit,
      rowCount: rows.length,
      rows: rows.slice(offset, offset + limit),
    }));

    const profile = await profileNativeLogisticDataset(input, settings, readPage);
    expect(readPage.mock.calls.map(([, offset, limit]) => [offset, limit])).toEqual([
      [0, NATIVE_LOGISTIC_PROFILE_PAGE_SIZE],
      [500, NATIVE_LOGISTIC_PROFILE_PAGE_SIZE],
      [1_000, NATIVE_LOGISTIC_PROFILE_PAGE_SIZE],
    ]);
    expect(profile).toMatchObject({
      expectedRows: 1_001,
      scannedRows: 1_001,
      completeCases: 1_001,
      zeroCases: 501,
      oneCases: 500,
      invalidOutcomeRows: 0,
    });
    expect(nativeLogisticReadiness(input, settings, profile)).toMatchObject({ canRun: true, profileRequired: false });
  });

  it("fails closed if the dataset changes between profile pages", async () => {
    const input = dataset([], 600);
    await expect(profileNativeLogisticDataset(input, settings, async (datasetId, offset, limit) => ({
      datasetId,
      offset,
      limit,
      rowCount: 601,
      rows: [{ converted: 0, score: 1, age: 20 }],
    }))).rejects.toThrow("dataset changed");
  });

  it("rejects stale or malformed dispatch profile proofs", () => {
    const rows = Array.from({ length: 8 }, (_, index) => ({
      converted: index % 2,
      score: index + 1,
      age: 20 + index,
    }));
    const input = dataset(rows);
    const profile = residentNativeLogisticProfile(input, settings)!;
    expect(parseNativeLogisticProfile(profile)).toEqual(profile);

    expect(nativeLogisticReadiness({ ...input, fingerprint: "sha256:changed" }, settings, profile)).toMatchObject({
      canRun: false,
      blockers: [expect.stringContaining("Reload the complete logistic outcome profile")],
    });
    expect(nativeLogisticReadiness(input, { ...settings, regressionControls: null }, profile)).toMatchObject({
      canRun: false,
      blockers: [expect.stringContaining("Reload the complete logistic outcome profile")],
    });
    expect(nativeLogisticReadiness(input, settings, { ...profile, scannedRows: 7 })).toMatchObject({
      canRun: false,
      blockers: [expect.stringContaining("dispatch proof is invalid")],
    });
  });
});
