import { describe, expect, it } from "vitest";
import { completedSamplePlsRun } from "../data/smokeRun";
import { buildResultInterpretation, rowSpecificInterpretation } from "../domain/resultInterpretation";
import { runExportTables } from "../domain/resultTables";
import type { AnalysisRun } from "../types";
import { nativeRunFromCanonicalResult } from "./nativeCanonicalProject";
import { nativeRunProvenanceTable } from "./nativeExportTables";
import { nativeResultTables } from "./nativeResults";
import {
  NATIVE_STRUCTURAL_PATH_RANDOMIZATION_METHOD_VERSION,
  NATIVE_STRUCTURAL_PATH_RANDOMIZATION_OPERATION,
  NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING,
  nativeStructuralPathRandomizationProjection,
  nativeStructuralPathRandomizationTable,
} from "./nativeStructuralPathRandomization";
import {
  completedStructuralPathRandomizationRun,
  structuralPathRandomizationCanonicalFixture,
} from "./nativeStructuralPathRandomization.testFixture";

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

describe("current structural path randomization frontend contract", () => {
  it("projects the exact method, plan, manifest, and plus-one arithmetic into candidate rows", () => {
    const projection = nativeStructuralPathRandomizationProjection(completedStructuralPathRandomizationRun());

    expect(projection).toMatchObject({
      methodVersion: NATIVE_STRUCTURAL_PATH_RANDOMIZATION_METHOD_VERSION,
      operation: NATIVE_STRUCTURAL_PATH_RANDOMIZATION_OPERATION,
      permutations: 999,
      masterSeed: 7,
    });
    expect(projection?.parameters).toHaveLength(5);
    expect(projection?.parameters[0]).toEqual({
      parameter: '["path",["competence","satisfaction"]]',
      source: "competence",
      target: "satisfaction",
      original: 0.403,
      exceedances: 9,
      permutations: 999,
      pValueTwoSided: 0.01,
    });

    const table = nativeStructuralPathRandomizationTable(projection!);
    expect(table).toMatchObject({
      id: "permutation",
      title: "Structural path randomization",
      status: "experimental",
      warning: NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING,
      columns: ["Path", "Original", "Exceedances", "Permutations", "Raw two-sided p"],
    });
    expect(table.rows[0]).toEqual(["competence -> satisfaction", "0.403000", "9", "999", "0.01"]);
    expect(JSON.stringify({ projection, table })).not.toMatch(/[\xC3\u00E2]/);
  });

  it("fails the entire projection closed for every current-contract inconsistency", () => {
    const mutations: Array<(run: AnalysisRun) => void> = [
      (run) => { run.result!.method_version = "pls_pm_v2"; },
      (run) => { run.provenance!.method_version += "+unexpected_v1"; },
      (run) => { (run.permutation as unknown as Record<string, unknown>).unexpected = true; },
      (run) => { (run.permutation!.plan as unknown as Record<string, unknown>).unexpected = true; },
      (run) => { (run.permutation!.parameters[0] as unknown as Record<string, unknown>).unexpected = true; },
      (run) => { run.permutation!.method_version = "freedman_lane_permutation_v0"; },
      (run) => { run.permutation!.plan.operation = "pls_permutation"; },
      (run) => { run.permutation!.plan.master_seed += 1; },
      (run) => { run.provenance!.settings.permutation_samples -= 1; },
      (run) => { run.provenance!.settings.bootstrap_samples = 99; },
      (run) => { run.bootstrap = completedSamplePlsRun().bootstrap; },
      (run) => { run.permutation!.parameters.reverse(); },
      (run) => { run.permutation!.parameters[1].parameter = run.permutation!.parameters[0].parameter; },
      (run) => { run.permutation!.parameters[0].original += Number.EPSILON; },
      (run) => { run.permutation!.parameters[0].exceedances = 1_000; },
      (run) => { run.permutation!.parameters[0].permutations = 998; },
      (run) => { run.permutation!.parameters[0].p_value_two_sided += Number.EPSILON; },
      (run) => { run.permutation!.parameters.pop(); },
      (run) => { run.permutation!.plan = null as unknown as NonNullable<AnalysisRun["permutation"]>["plan"]; },
      (run) => { run.permutation!.parameters = {} as unknown as NonNullable<AnalysisRun["permutation"]>["parameters"]; },
      (run) => { run.result!.paths = null as unknown as NonNullable<AnalysisRun["result"]>["paths"]; },
      (run) => { run.provenance!.settings = null as unknown as NonNullable<AnalysisRun["provenance"]>["settings"]; },
      (run) => { run.result!.moderation = "malformed" as unknown as NonNullable<AnalysisRun["result"]>["moderation"]; },
    ];

    for (const mutate of mutations) {
      const run = clone(completedStructuralPathRandomizationRun());
      mutate(run);
      expect(() => nativeStructuralPathRandomizationProjection(run)).not.toThrow();
      expect(nativeStructuralPathRandomizationProjection(run)).toBeNull();
    }
  });

  it("uses only the strict projection in native results, reports, and exports", () => {
    const run = completedStructuralPathRandomizationRun();
    const nativeTable = nativeResultTables(run).find((table) => table.id === "permutation");
    expect(nativeTable).toMatchObject({
      status: "experimental",
      warning: NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING,
      columns: ["Path", "Original", "Exceedances", "Permutations", "Raw two-sided p"],
    });

    const exportTables = runExportTables(run);
    expect(exportTables.find((table) => table.id === "permutation")).toEqual(nativeTable);
    expect(exportTables[0]).toMatchObject({
      id: "run_provenance",
      status: "experimental",
      warning: NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING,
    });
    expect(exportTables[0].rows).toEqual(expect.arrayContaining([
      ["Randomization method", NATIVE_STRUCTURAL_PATH_RANDOMIZATION_METHOD_VERSION],
      ["Randomization operation", NATIVE_STRUCTURAL_PATH_RANDOMIZATION_OPERATION],
      ["Randomized structural paths", "5"],
      ["Randomization estimand", "Structural path coefficients conditional on fixed original PLS construct scores"],
      ["Pathwise probability", "Conditional/approximate two-sided plus-one probability under exchangeable reduced-model residuals; no multiplicity adjustment"],
      ["Qualification status", "Internal candidate/experimental product label; method-specific qualification evidence is tracked separately"],
    ]));
    expect(nativeRunProvenanceTable(run)).toMatchObject({
      status: "experimental",
      warning: NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING,
    });

    const invalid = clone(run);
    invalid.permutation!.parameters[0].p_value_two_sided = 0.0101;
    expect(nativeResultTables(invalid).some((table) => table.id === "permutation")).toBe(false);
    expect(runExportTables(invalid).some((table) => table.id === "permutation")).toBe(false);
    expect(nativeRunProvenanceTable(invalid).rows.some(([field]) => field === "Randomization method")).toBe(false);
  });

  it("interprets valid randomization as candidate fixed-score, unadjusted pathwise inference", () => {
    const run = completedStructuralPathRandomizationRun();
    const interpretation = buildResultInterpretation({ run });
    const randomization = interpretation.findings.find((finding) => finding.id.startsWith("permutation."));

    expect(randomization).toMatchObject({
      severity: "caution",
      metric: "Raw two-sided structural path randomization p",
      path: { source: "competence", target: "satisfaction" },
    });
    expect(randomization?.interpretation).toContain("fixed original PLS construct scores");
    expect(randomization?.interpretation).toContain("exchangeable reduced-model residuals");
    expect(randomization?.interpretation).toContain("unadjusted for multiplicity");
    expect(interpretation.reportParagraphs.find((paragraph) => paragraph.section === "Inference caveat")?.text)
      .toContain(NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING);
    expect(rowSpecificInterpretation(
      "Structural path randomization",
      ["Path", "Original", "Exceedances", "Permutations", "Raw two-sided p"],
      ["competence -> satisfaction", "0.403000", "9", "999", "0.01"],
    )).toContain("candidate Freedman-Lane result");

    const invalid = clone(run);
    invalid.permutation!.parameters[0].exceedances = 1_000;
    const invalidInterpretation = buildResultInterpretation({ run: invalid });
    expect(invalidInterpretation.findings.some((finding) => finding.id.startsWith("permutation."))).toBe(false);
    expect(invalidInterpretation.findings.some((finding) => finding.id === "inference.missing")).toBe(true);
  });

  it("hydrates only a schema-v3 recipe and genuine schema-v1 result envelope bound to the exact current contract", () => {
    const current = structuralPathRandomizationCanonicalFixture();
    expect(current.envelope.schema_version).toBe(1);
    expect(nativeRunFromCanonicalResult(current.envelope, current.recipe)).not.toBeNull();

    for (const staleSchemaVersion of [0, 2, 3, 4, 5]) {
      const staleEnvelope = clone(current);
      staleEnvelope.envelope.schema_version = staleSchemaVersion;
      expect(nativeRunFromCanonicalResult(staleEnvelope.envelope, staleEnvelope.recipe)).toBeNull();
    }

    const staleOperation = clone(current);
    const stalePayload = staleOperation.envelope.payload;
    if (stalePayload.kind === "pls_pm_v3" && stalePayload.permutation) {
      stalePayload.permutation.plan.operation = "pls_permutation";
    }
    expect(nativeRunFromCanonicalResult(staleOperation.envelope, staleOperation.recipe)).toBeNull();

    const wrongRecipeKind = clone(current);
    wrongRecipeKind.recipe.method_config = { kind: "pls_algorithm" };
    expect(nativeRunFromCanonicalResult(wrongRecipeKind.envelope, wrongRecipeKind.recipe)).toBeNull();

    const wrongPathOrder = clone(current);
    wrongPathOrder.recipe.model.paths.reverse();
    expect(nativeRunFromCanonicalResult(wrongPathOrder.envelope, wrongPathOrder.recipe)).toBeNull();

    const wrongArithmetic = clone(current);
    const arithmeticPayload = wrongArithmetic.envelope.payload;
    if (arithmeticPayload.kind === "pls_pm_v3" && arithmeticPayload.permutation) {
      arithmeticPayload.permutation.parameters[0].p_value_two_sided += Number.EPSILON;
    }
    expect(nativeRunFromCanonicalResult(wrongArithmetic.envelope, wrongArithmetic.recipe)).toBeNull();
  });
});
