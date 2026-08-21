import type { Edge, Node } from "@xyflow/react";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type {
  AnalysisRun,
  AnalysisUiSettings,
  CbsemAnalysis,
  ColumnMetadata,
  ConstructData,
  Dataset,
  RunMonitorState,
} from "../types";
import NativeCalculationDialog from "./NativeCalculationDialog";
import { nativeAnalysisCatalogItem } from "./nativeAnalysisCatalog";
import { buildNativeAnalysisRecipe, NativeAnalysisRecipeBuildError } from "./nativeAnalysisRecipe";
import { completedCbsemRun } from "./nativeCbsem.testFixture";
import { nativeRunProvenanceTable, nativeRunSettingApplicability } from "./nativeExportTables";
import { nativePlsReadiness } from "./nativePlsReadiness";
import { buildNativeResultNavigation, nativeCbsemResultProjection } from "./nativeResults";

const settings: AnalysisUiSettings = {
  method: "cbsem",
  weightingScheme: "path",
  preprocessing: "standardized",
  tolerance: 1e-7,
  maxIterations: 3_000,
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20_260_814,
  workers: 4,
  confidenceLevel: 0.95,
  caseWeightColumn: null,
  cbsemModelType: "sem",
  cbsemMeanStructure: false,
  cbsemStandardization: "std_all",
  cbsemGroupColumn: null,
  cbsemInvarianceSteps: null,
  cbsemBootstrapSamples: 1_000,
};

const nodes: Array<Node<ConstructData>> = [
  { id: "x", position: { x: 0, y: 0 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
  { id: "y", position: { x: 300, y: 0 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] } },
];
const edges: Edge[] = [{ id: "x-y", source: "x", target: "y" }];
const numericMetadata = (name: string): ColumnMetadata => ({
  name,
  label: null,
  column_type: "numeric",
  scale_type: "continuous",
  missing_markers: [],
  theoretical_min: null,
  theoretical_max: null,
  value_labels: {},
});
const dataset: Dataset = {
  id: "cbsem-study",
  name: "cbsem-study.csv",
  columns: ["x1", "x2", "y1", "y2"],
  rows: Array.from({ length: 30 }, (_, index) => ({
    x1: index + 0.1,
    x2: index + 0.2,
    y1: index + 0.3,
    y2: index + 0.4,
  })),
  rowCount: 30,
  missing: 0,
  fingerprint: "sha256:cbsem-bootstrap-study",
  kind: "raw",
  columnMetadata: ["x1", "x2", "y1", "y2"].map(numericMetadata),
};

function bootstrapRun(usableReplicates = 1_000, requestedReplicates = 1_000): AnalysisRun {
  const run = completedCbsemRun("sem");
  const analysis = run.result!.cbsem!;
  const freeParameters = analysis.parameters.filter((parameter) => !parameter.fixed);
  const successfulReplicates = Array.from({ length: usableReplicates }, (_, replicateIndex) => ({
    replicate_index: replicateIndex,
    sample_indices_sha256: "a".repeat(64),
    iterations: 7,
    objective: 0.01,
    parameter_estimates: freeParameters.map((parameter, parameterIndex) => (
      parameter.estimate + ((replicateIndex % 11) - 5) * 1e-5 * (parameterIndex + 1)
    )),
  }));
  const failures = Array.from({ length: requestedReplicates - usableReplicates }, (_, index) => ({
    replicate_index: usableReplicates + index,
    sample_indices_sha256: "b".repeat(64),
    reason_code: "ml_nonconvergence",
    message: "ML refit did not converge for this preplanned primary draw.",
  }));
  const minimumUsableReplicates = Math.max(1_000, Math.ceil(0.9 * requestedReplicates));
  const available = usableReplicates >= minimumUsableReplicates;
  const unavailableMessage = `CB-SEM bootstrap inference is unavailable because ${usableReplicates} usable primary fits are below the required ${minimumUsableReplicates}; no intervals were emitted.`;
  const bootstrap: NonNullable<CbsemAnalysis["bootstrap_v2"]> = {
    method_version: "cbsem_bootstrap_v2",
    algorithm: "indexed_raw_case_refit_ml_v2",
    interval_method: "percentile_type7_v1",
    retry_policy: "no_retry_fixed_preplanned_primary_draws_v1",
    confidence_level: 0.95,
    requested_replicates: requestedReplicates,
    attempted_fits: requestedReplicates,
    usable_replicates: usableReplicates,
    failed_replicates: failures.length,
    minimum_usable_fraction: 0.9,
    minimum_usable_replicates: minimumUsableReplicates,
    max_attempts_per_replicate: 1,
    complete_case_sample_size: analysis.sample_size,
    seed: run.seed,
    stream_token: "quickpls_cbsem_ml_case_bootstrap_v2",
    inference: available
      ? { status: "available" }
      : { status: "unavailable", reason_code: "insufficient_usable_replicates", message: unavailableMessage },
    intervals: available
      ? freeParameters.map((parameter) => ({
        parameter: parameter.name,
        original: parameter.estimate,
        bootstrap_mean: parameter.estimate,
        bias: 0,
        standard_error: 0.01,
        percentile_lower: parameter.estimate - 0.02,
        percentile_upper: parameter.estimate + 0.02,
        usable_replicates: usableReplicates,
      }))
      : [],
    failures,
    validation_witness: {
      method_version: "cbsem_bootstrap_validation_witness_v2",
      dataset_fingerprint: run.provenance!.dataset_fingerprint,
      recipe_sha256: "c".repeat(64),
      base_result_sha256: "d".repeat(64),
      parameter_names: freeParameters.map((parameter) => parameter.name),
      successful_replicates: successfulReplicates,
    },
    warnings: available
      ? ["Percentile Type-7 intervals are based on usable full-ML case refits."]
      : [unavailableMessage, ...(failures.length ? [`${failures.length} failed primary draw${failures.length === 1 ? " remains" : "s remain"} in the ledger.`] : [])],
  };
  analysis.bootstrap_v2 = bootstrap;
  run.provenance!.method_version = run.provenance!.method_version.replace(
    "+pls_mediation_v1",
    "+cbsem_bootstrap_v2+pls_mediation_v1",
  );
  run.provenance!.settings.workers = 4;
  run.provenance!.settings.confidence_level = 0.95;
  return run;
}

const runMonitor: RunMonitorState = {
  status: "idle",
  phase: "",
  message: "",
  completedUnits: 0,
  totalUnits: 0,
  startedAt: null,
  completedAt: null,
  activeJobId: null,
  lastRunId: null,
  error: null,
  logs: [],
};

describe("historical native CB-SEM bootstrap v2 compatibility", () => {
  it("keeps the historical schema-3 recipe explicit but routes new inference to Exact CB-SEM", () => {
    const recipe = buildNativeAnalysisRecipe({
      kind: "cbsem",
      recipeId: "11111111-1111-4111-8111-111111111111",
      modelId: "22222222-2222-4222-8222-222222222222",
      createdAt: "2026-08-14T00:00:00.000Z",
      datasetFingerprint: dataset.fingerprint!,
      projectName: "CB-SEM bootstrap v2",
      nodes,
      edges,
      settings,
    });
    expect(recipe.method_config).toMatchObject({
      kind: "cbsem",
      estimator: "ml",
      input: "raw",
      mean_structure: false,
      bootstrap_samples: 1_000,
      bootstrap_v2: { algorithm: "case_resampling_full_ml", interval: "percentile_type7" },
    });
    expect(recipe.settings).toMatchObject({
      seed: 20_260_814,
      workers: 4,
      confidence_level: 0.95,
      bootstrap_samples: 0,
    });
    expect(recipe.metadata).toEqual({
      status: "candidate_cbsem_bootstrap_v2_unqualified_bounded_scope",
    });
    expect(() => buildNativeAnalysisRecipe({
      kind: "cbsem",
      recipeId: "33333333-3333-4333-8333-333333333333",
      modelId: "44444444-4444-4444-8444-444444444444",
      createdAt: "2026-08-14T00:00:00.000Z",
      datasetFingerprint: dataset.fingerprint!,
      projectName: "Wrong confidence",
      nodes,
      edges,
      settings: { ...settings, confidenceLevel: 0.90 },
    })).toThrowError(expect.objectContaining<Partial<NativeAnalysisRecipeBuildError>>({ field: "confidenceLevel" }));

    const readiness = nativePlsReadiness({ dataset, nodes, edges, settings, nativeDesktop: true });
    expect(readiness.canRun).toBe(false);
    expect(readiness.blockers.find((item) => item.id === "calculation")?.detail)
      .toContain("Run current exact CFA bootstrap from the Exact CB-SEM model tab");
    const wrongConfidence = nativePlsReadiness({
      dataset,
      nodes,
      edges,
      settings: { ...settings, confidenceLevel: 0.90 },
      nativeDesktop: true,
    });
    expect(wrongConfidence.canRun).toBe(false);
    expect(wrongConfidence.blockers.find((item) => item.id === "calculation")?.detail).toContain("fixed two-sided 95%");

    const markup = renderToStaticMarkup(createElement(NativeCalculationDialog, {
      kind: "cbsem",
      experimentalLabsEnabled: true,
      setKind: () => undefined,
      settings,
      setSettings: () => undefined,
      readiness,
      runMonitor,
      dataset,
      analysisColumns: dataset.columns,
      nodes,
      edges,
      start: () => undefined,
      cancel: () => undefined,
      close: () => undefined,
    }));
    expect(markup).toContain('id="nd-calculation-method-cbsem"');
    expect(markup).toContain('id="nd-calculation-cbsem-archived-bootstrap"');
    expect(markup).toContain("Run current exact CFA bootstrap from the Exact CB-SEM model tab");
    expect(markup).toContain("Clear the archived bootstrap setting before running this point-estimate setup.");
    expect(markup).toContain("Clear setting");
    expect(markup).not.toContain('id="nd-calculation-cbsem-exact-bootstrap-route"');
    expect(markup).not.toContain("Run percentile Type-7, analytic studentized Type-7, or BCa Type-7 case bootstrap");
    expect(markup).not.toContain('id="nd-calculation-cbsem-bootstrap-samples"');
    expect(markup).not.toContain('id="nd-calculation-cbsem-confidence"');
    expect(markup).not.toContain("Calculation method unavailable");
    expect(markup).toMatch(/class="primary" type="submit" disabled=""/);
    expect(markup).toContain('id="nd-calculation-workers"');
    expect(markup).not.toContain('data-limited-scope-warning="true"');
    expect(nativeAnalysisCatalogItem("cbsem").description)
      .toContain("Current exact CFA case bootstrap is available from the Exact CB-SEM model tab");
  });

  it("projects the same completed run into results and export tables and rejects analytical tampering", () => {
    const run = bootstrapRun();
    expect(nativeCbsemResultProjection(run)).not.toBeNull();
    const navigation = buildNativeResultNavigation(run);
    expect(navigation.tables.map((table) => table.id)).toEqual(expect.arrayContaining([
      "cbsem_bootstrap_intervals",
      "cbsem_bootstrap_failures",
      "cbsem_bootstrap_settings",
    ]));
    expect(navigation.tables.find((table) => table.id === "cbsem_bootstrap_intervals")?.warning).toContain("Experimental CB-SEM bootstrap output");
    expect(nativeRunSettingApplicability(run)).toEqual({
      usesSeed: true,
      usesConfidenceLevel: true,
      usesWorkers: true,
    });
    const exportTable = nativeRunProvenanceTable(run);
    expect(exportTable.status).toBe("experimental");
    expect(exportTable.rows).toEqual(expect.arrayContaining([
      ["CB-SEM bootstrap method", "cbsem_bootstrap_v2"],
      ["CB-SEM bootstrap availability", "Experimental"],
      ["Requested CB-SEM bootstrap draws", "1000"],
      ["CB-SEM bootstrap confidence", "95.0%"],
      ["Workers", "4"],
    ]));

    const tamperedConfidence = bootstrapRun();
    tamperedConfidence.result!.cbsem!.bootstrap_v2!.confidence_level = 0.90;
    expect(nativeCbsemResultProjection(tamperedConfidence)).toBeNull();
    expect(buildNativeResultNavigation(tamperedConfidence).tables.map((table) => table.id))
      .not.toContain("cbsem_bootstrap_intervals");

    const tamperedWitness = bootstrapRun();
    tamperedWitness.result!.cbsem!.bootstrap_v2!.validation_witness.successful_replicates[0]
      .sample_indices_sha256 = "not-a-digest";
    expect(nativeCbsemResultProjection(tamperedWitness)).toBeNull();
  });

  it("keeps a below-threshold completion visible with no interval and the exact failure ledger", () => {
    const run = bootstrapRun(999);
    expect(nativeCbsemResultProjection(run)).not.toBeNull();
    const navigation = buildNativeResultNavigation(run);
    const intervals = navigation.tables.find((table) => table.id === "cbsem_bootstrap_intervals");
    const failures = navigation.tables.find((table) => table.id === "cbsem_bootstrap_failures");
    const threshold = navigation.tables.find((table) => table.id === "cbsem_bootstrap_settings");
    expect(intervals?.rows).toEqual([]);
    expect(intervals?.warning).toMatch(/Experimental CB-SEM bootstrap output.*inference is unavailable/);
    expect(failures?.rows).toEqual([[
      "1000",
      "ml_nonconvergence",
      "ML refit did not converge for this preplanned primary draw.",
      "b".repeat(64),
    ]]);
    expect(threshold?.rows).toEqual(expect.arrayContaining([
      ["Inference", "Unavailable - insufficient usable full-ML fits"],
      ["Usable ML fits", "999"],
      ["Failed ML fits", "1"],
      ["Minimum usable fits", "1000"],
    ]));
  });

  it("keeps a complete 500-replicate pilot visible as unavailable and rejects boundary tampering", () => {
    const run = bootstrapRun(500, 500);
    expect(nativeCbsemResultProjection(run)).not.toBeNull();
    const navigation = buildNativeResultNavigation(run);
    expect(navigation.tables.find((table) => table.id === "cbsem_bootstrap_intervals")?.rows).toEqual([]);
    expect(navigation.tables.find((table) => table.id === "cbsem_bootstrap_failures")?.rows).toEqual([]);
    expect(navigation.tables.find((table) => table.id === "cbsem_bootstrap_settings")?.rows)
      .toEqual(expect.arrayContaining([
        ["Requested primary draws", "500"],
        ["Usable ML fits", "500"],
        ["Failed ML fits", "0"],
        ["Minimum usable fits", "1000"],
        ["Inference", "Unavailable - insufficient usable full-ML fits"],
      ]));

    const belowPilot = bootstrapRun(499, 499);
    expect(nativeCbsemResultProjection(belowPilot)).toBeNull();

    const falseAvailability = bootstrapRun(500, 500);
    falseAvailability.result!.cbsem!.bootstrap_v2!.inference = { status: "available" };
    expect(nativeCbsemResultProjection(falseAvailability)).toBeNull();

    const wrongCount = bootstrapRun(500, 500);
    wrongCount.result!.cbsem!.bootstrap_v2!.attempted_fits = 501;
    expect(nativeCbsemResultProjection(wrongCount)).toBeNull();
  });
});
