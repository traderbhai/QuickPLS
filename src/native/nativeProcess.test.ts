import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings, Dataset, NativeProcessGraphRelationshipConfig } from "../types";
import {
  nativeProcessGraphAssessment,
  nativeProcessReadiness,
  parseNativeProcessProfile,
  profileNativeProcessDataset,
  residentNativeProcessProfile,
} from "./nativeProcess";

export const compositeProcessGraph: NativeProcessGraphRelationshipConfig = {
  model: "graph",
  focal_predictor: "X",
  paths: [
    { from: "X", to: "Y" },
    { from: "X", to: "M1" },
    { from: "M1", to: "M2" },
    { from: "M2", to: "Y" },
    { from: "X", to: "M3" },
    { from: "M3", to: "Y" },
    { from: "X", to: "M4" },
    { from: "M4", to: "Y" },
  ],
  moderators: [
    { variable: "W", scale: "continuous" },
    { variable: "B", scale: "binary_0_1" },
  ],
  moderations: [
    { from: "X", to: "Y", moderator: "W", conditioning_moderator: "B" },
    { from: "X", to: "M3", moderator: "W" },
    { from: "M4", to: "Y", moderator: "B" },
  ],
  continuous_product_centering: "equation_complete_case_mean_v1",
};

export const compositeProcessSettings: AnalysisUiSettings = {
  method: "regression",
  weightingScheme: "path",
  preprocessing: "unstandardized",
  bootstrapSamples: 10_000,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20_260_812,
  workers: 4,
  confidenceLevel: 0.95,
  caseWeightColumn: null,
  regressionType: "process",
  regressionOutcome: "Y",
  regressionPredictors: "X,M1,M2,M3,M4,W,B",
  regressionControls: "C",
  regressionBootstrap: true,
  robustSe: "hc3",
  processGraph: compositeProcessGraph,
};

export function compositeProcessDataset(rows = 40): Dataset {
  const columns = ["Y", "X", "M1", "M2", "M3", "M4", "W", "B", "C"];
  return {
    id: "process-data",
    name: "process.csv",
    columns,
    rows: Array.from({ length: rows }, (_, index) => ({
      X: index / 5 - 4,
      W: (index % 9) - 4,
      B: index % 2,
      C: (index % 7) / 3,
      M1: index / 7 + (index % 3),
      M2: index / 9 + (index % 4),
      M3: index / 11 + (index % 5),
      M4: index / 13 + (index % 6),
      Y: index / 2 + (index % 8),
    })),
    rowCount: rows,
    missing: 0,
    fingerprint: "sha256:process-v2",
    kind: "raw",
    columnMetadata: columns.map((name) => ({
      name,
      label: null,
      column_type: "numeric" as const,
      role: "unassigned" as const,
      scale_type: "continuous" as const,
      missing_markers: [],
      theoretical_min: null,
      theoretical_max: null,
      value_labels: {},
    })),
  };
}

describe("native graph-defined PROCESS", () => {
  it("accepts one graph covering serial, parallel, first-stage, second-stage, and mixed three-way moderation", () => {
    const assessment = nativeProcessGraphAssessment(compositeProcessSettings);
    expect(assessment).toMatchObject({
      canRun: true,
      focalPredictor: "X",
      outcome: "Y",
      mediators: ["M1", "M2", "M3", "M4"],
      predictors: ["X", "M1", "M2", "M3", "M4", "W", "B"],
      controls: ["C"],
    });
    expect(assessment.equationTermCounts.find((equation) => equation.outcome === "Y")?.terms).toBe(12);
  });

  it("enforces the exact eight-predictor and one-control release boundaries", () => {
    const eight = structuredClone(compositeProcessSettings);
    eight.processGraph!.paths.push({ from: "X", to: "M5" }, { from: "M5", to: "Y" });
    eight.regressionPredictors = null;
    const atPredictorLimit = nativeProcessGraphAssessment(eight);
    expect(atPredictorLimit.predictors).toHaveLength(8);
    expect(atPredictorLimit.blockers).not.toContain(
      "Use no more than 8 predictors across the focal predictor, mediators, and declared moderators",
    );

    const nine = structuredClone(eight);
    nine.processGraph!.paths.push({ from: "X", to: "M6" }, { from: "M6", to: "Y" });
    const abovePredictorLimit = nativeProcessGraphAssessment(nine);
    expect(abovePredictorLimit.predictors).toHaveLength(9);
    expect(abovePredictorLimit.blockers).toContain(
      "Use no more than 8 predictors across the focal predictor, mediators, and declared moderators",
    );

    expect(nativeProcessGraphAssessment(compositeProcessSettings).blockers).not.toContain("Use no more than one control variable");
    const twoControls = { ...compositeProcessSettings, regressionControls: "C,D" };
    expect(nativeProcessGraphAssessment(twoControls).blockers).toContain("Use no more than one control variable");
  });

  it("counts colon-containing interaction tuples injectively for equation readiness", () => {
    const settings = structuredClone(compositeProcessSettings);
    settings.processGraph = {
      model: "graph",
      focal_predictor: "X",
      paths: [
        { from: "X", to: "A" },
        { from: "A", to: "Y" },
        { from: "X", to: "A:B" },
        { from: "A:B", to: "Y" },
      ],
      moderators: [
        { variable: "B:C", scale: "continuous" },
        { variable: "C", scale: "continuous" },
      ],
      moderations: [
        { from: "A", to: "Y", moderator: "B:C" },
        { from: "A:B", to: "Y", moderator: "C" },
      ],
      continuous_product_centering: "equation_complete_case_mean_v1",
    };
    settings.regressionPredictors = null;
    settings.regressionControls = null;
    expect(nativeProcessGraphAssessment(settings).equationTermCounts)
      .toContainEqual({ outcome: "Y", terms: 6 });
  });

  it("fails closed for cycles, detached mediators, excess moderators, and unsupported mediated interactions", () => {
    const cycle = structuredClone(compositeProcessSettings);
    cycle.processGraph!.paths.push({ from: "M2", to: "X" });
    expect(nativeProcessGraphAssessment(cycle).blockers).toContain("The focal predictor cannot have an incoming path");

    const detached = structuredClone(compositeProcessSettings);
    detached.processGraph!.paths.push({ from: "D", to: "E" });
    detached.regressionPredictors = null;
    expect(nativeProcessGraphAssessment(detached).blockers).toContain("Every mediator must lie on a directed focal-predictor-to-outcome path");

    const excessModerators = structuredClone(compositeProcessSettings);
    excessModerators.processGraph!.moderators.push({ variable: "Z", scale: "continuous" });
    excessModerators.regressionPredictors = null;
    expect(nativeProcessGraphAssessment(excessModerators).blockers).toContain("Declare no more than 2 moderator variables");

    const threeWayMediated = structuredClone(compositeProcessSettings);
    threeWayMediated.processGraph!.moderations[1].conditioning_moderator = "B";
    expect(nativeProcessGraphAssessment(threeWayMediated).blockers).toContain("Two-moderator interactions are supported only on the direct focal-predictor-to-outcome path");

    const twoStages = structuredClone(compositeProcessSettings);
    twoStages.processGraph!.moderations.push({ from: "M3", to: "Y", moderator: "B" });
    expect(nativeProcessGraphAssessment(twoStages).blockers).toContain("Indirect path X -> M3 -> Y can moderate only one stage");

    const unusedModerator = structuredClone(compositeProcessSettings);
    unusedModerator.processGraph!.moderations = unusedModerator.processGraph!.moderations
      .filter((moderation) => moderation.moderator !== "B")
      .map((moderation) => ({ ...moderation, conditioning_moderator: undefined }));
    expect(nativeProcessGraphAssessment(unusedModerator).blockers)
      .toContain("Every declared moderator must be used as a primary or conditioning moderator");
  });

  it("matches Rust code-point ordering and blocks reserved scientific-ID names", () => {
    const ordered = structuredClone(compositeProcessSettings);
    const mediators = ["a", "\u{1F600}", "\uE000", "A"];
    ordered.processGraph = {
      model: "graph",
      focal_predictor: "X",
      paths: mediators.flatMap((mediator) => [
        { from: "X", to: mediator },
        { from: mediator, to: "Y" },
      ]),
      moderators: [],
      moderations: [],
      continuous_product_centering: "equation_complete_case_mean_v1",
    };
    ordered.regressionPredictors = null;
    ordered.regressionControls = null;
    expect(nativeProcessGraphAssessment(ordered).mediators).toEqual(["A", "a", "\uE000", "\u{1F600}"]);

    for (const reserved of ["A->B", "W@1", "X*Z", "A,B", "A=B", "A|B", "A\0B", "A\u0085B"]) {
      const settings = { ...compositeProcessSettings, regressionOutcome: reserved };
      expect(nativeProcessGraphAssessment(settings).blockers).toContain(
        "PROCESS variable names cannot contain control characters, ->, @, |, *, comma, or equals because these tokens are reserved for stable scientific identities",
      );
    }
  });

  it("profiles the global listwise sample and enforces exact binary moderator coding", () => {
    const dataset = compositeProcessDataset();
    const profile = residentNativeProcessProfile(dataset, compositeProcessSettings);
    expect(profile).toMatchObject({
      expectedRows: 40,
      scannedRows: 40,
      completeCases: 40,
      omittedRows: 0,
      binaryModerators: ["B"],
      invalidBinaryRows: { B: 0 },
      binaryEquationOutcomes: [],
      constantVariables: [],
    });
    expect(parseNativeProcessProfile(profile)).toEqual(profile);
    expect(nativeProcessReadiness(dataset, compositeProcessSettings, profile)).toMatchObject({
      canRun: true,
      profileRequired: false,
      completeCases: 40,
    });

    const invalid = compositeProcessDataset();
    invalid.rows[4].B = 2;
    const invalidProfile = residentNativeProcessProfile(invalid, compositeProcessSettings);
    expect(nativeProcessReadiness(invalid, compositeProcessSettings, invalidProfile).blockers)
      .toContain("B has 1 listwise-complete value outside exact 0/1 coding");

    for (const endogenous of ["M1", "Y"]) {
      const binaryEndogenous = compositeProcessDataset();
      binaryEndogenous.rows.forEach((row, index) => { row[endogenous] = index % 2; });
      const binaryProfile = residentNativeProcessProfile(binaryEndogenous, compositeProcessSettings);
      expect(binaryProfile?.binaryEquationOutcomes).toContain(endogenous);
      expect(nativeProcessReadiness(binaryEndogenous, compositeProcessSettings, binaryProfile).blockers)
        .toContain(`PROCESS v2 requires continuous endogenous equation outcomes; ${endogenous} is exactly coded 0/1 in the original complete sample`);
    }
  });

  it("profiles nonresident desktop rows in bounded ordered pages and rejects stale proofs", async () => {
    const full = compositeProcessDataset(1_025);
    const shell = { ...full, rows: full.rows.slice(0, 20) };
    const calls: Array<[number, number]> = [];
    const profile = await profileNativeProcessDataset(shell, compositeProcessSettings, async (datasetId, offset, limit) => {
      calls.push([offset, limit]);
      return {
        datasetId,
        offset,
        limit,
        rowCount: full.rowCount!,
        rows: full.rows.slice(offset, offset + limit),
      };
    });
    expect(calls).toEqual([[0, 500], [500, 500], [1_000, 500]]);
    expect(profile).toMatchObject({ scannedRows: 1_025, completeCases: 1_025 });
    expect(nativeProcessReadiness(shell, compositeProcessSettings, profile).canRun).toBe(true);
    expect(nativeProcessReadiness(
      shell,
      { ...compositeProcessSettings, regressionControls: null },
      profile,
    ).blockers).toContain("Reload the complete PROCESS profile for the current dataset and graph");
  });

  it("requires enough complete cases for the largest hierarchy-preserving equation", () => {
    const dataset = compositeProcessDataset(12);
    const profile = residentNativeProcessProfile(dataset, compositeProcessSettings);
    const readiness = nativeProcessReadiness(dataset, compositeProcessSettings, profile);
    expect(readiness.canRun).toBe(false);
    expect(readiness.blockers).toContain("PROCESS v2 requires at least 14 complete finite rows for the largest 12-term equation");
  });
});
