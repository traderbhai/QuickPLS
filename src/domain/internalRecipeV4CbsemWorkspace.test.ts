import { describe, expect, it, vi } from "vitest";
import type { Node } from "@xyflow/react";
import wave1DiagramCbsemRoundtrip from "../../validation/fixtures/wave1_diagram_cbsem_roundtrip_v1.json";
import type { CanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import type {
  InternalLabsRecipeV4CbsemExecutionRequestV1,
  InternalRecipeV4CbsemCompletedResultV1,
  InternalRecipeV4CbsemJobSnapshotV1,
} from "./internalRecipeV4CbsemExecution";
import type { ProjectUpgradeInspectionV1 } from "./internalProjectUpgradeV6";
import {
  appendInternalLabsRecipeV4CbsemResultV1,
  bindInternalRecipeV4CbsemDatasetV1,
  buildInternalLabsRecipeV4CbsemRequestV1,
  monitorInternalLabsRecipeV4CbsemJobV1,
  preflightInternalRecipeV4CbsemWorkspaceV1,
  readStoredInternalLabsRecipeV4CbsemResultsV1,
  reopenInternalLabsRecipeV4CbsemResultV1,
  schema6ArchiveIdentityFromInspectionV1,
  storedExactCaseBootstrapEntriesV1,
} from "./internalRecipeV4CbsemWorkspace";
import { convertLegacyBasicModelV4, type SemModelV4 } from "./semModelV4";
import { requireNativeWorkbenchSemModelV4 } from "./nativeWorkbenchSemModelV4Adapter";
import type { ConstructData, Dataset } from "../types";

const engine = { tolerance: 1e-7, maxIterations: 1_000, seed: 42, workers: 1, confidenceLevel: 0.95 };
const nativeScientificSha256 = "ab".repeat(32);

function baseModel(): SemModelV4 {
  return convertLegacyBasicModelV4({
    id: "model-v4",
    name: "One-factor CFA",
    constructs: [{ id: "factor", name: "Factor", short_name: "F", mode: "reflective", indicators: ["x1", "x2", "x3"] }],
    paths: [],
  }, "cbsem_common_factor");
}

function rawDataset(): Dataset {
  return {
    id: "raw-data",
    name: "Raw data",
    kind: "raw",
    columns: ["x1", "x2", "x3"],
    rows: Array.from({ length: 12 }, (_, index) => ({ x1: index, x2: index + 1, x3: index + 2 })),
    rowCount: 12,
    missing: 0,
    fingerprint: "raw-fingerprint-v1",
  };
}

function matrixDataset(kind: "covariance" | "correlation" = "covariance"): Dataset {
  return {
    id: `${kind}-data`,
    name: `${kind} data`,
    kind,
    columns: ["x2", "x1", "x3"],
    rows: [
      { x2: 1, x1: 0.4, x3: 0.2 },
      { x2: 0.4, x1: 1, x3: 0.3 },
      { x2: 0.2, x1: 0.3, x3: 1 },
    ],
    rowCount: 3,
    missing: 0,
    sampleSize: 120,
    fingerprint: `${kind}-fingerprint-v1`,
  };
}

function snapshot(state: InternalRecipeV4CbsemJobSnapshotV1["state"], completedUnits: number): InternalRecipeV4CbsemJobSnapshotV1 {
  return {
    schemaVersion: 1,
    jobId: "job-v4",
    state,
    phase: state === "completed" ? "publication" : "estimation",
    completedUnits,
    totalUnits: 3,
    message: null,
    failure: null,
    queuedAt: "2026-08-15T00:00:00Z",
    startedAt: "2026-08-15T00:00:01Z",
    completedAt: state === "completed" || state === "cancelled" ? "2026-08-15T00:00:02Z" : null,
  };
}

function canonicalDocument(projectId = "project-v6"): CanonicalResultDocumentV2 {
  return {
    schema_version: 2,
    document_id: "document-v4",
    title: "CB-SEM result",
    provenance: {
      run_id: "run-v4",
      project_id: projectId,
      model_id: "model-v4",
      model_digest: "a".repeat(64),
      dataset_id: "raw-data",
      dataset_fingerprint: "raw-fingerprint-v1",
      recipe_id: "00000000-0000-4000-8000-000000000001",
      recipe_digest: "b".repeat(64),
      capability_cell: {
        registry_schema_version: 2,
        capability_id: "smartpls.cbsem",
        cell_id: "qpls3.cbsem.ml",
        capability_version: "cbsem_ml_v1",
      },
      method_version: "cbsem_ml_exact_parameter_table_v3",
      engine_version: "test",
      seed: 42,
      workers: 1,
      started_at: "2026-08-15T00:00:00Z",
      completed_at: "2026-08-15T00:00:02Z",
    },
    sections: [],
    tables: [],
    charts: [],
    notices: [],
    exclusions: [],
    footnotes: [],
    presentation: { default_section_id: null, default_table_id: null, precision: 3, missing_value_label: "—", chart_defaults: {} },
  };
}

function completed(projectId = "project-v6"): InternalRecipeV4CbsemCompletedResultV1 {
  return {
    schemaVersion: 1,
    canonicalDocument: canonicalDocument(projectId),
    analyticalResult: {
      schema_version: 1,
      provenance: {
        adapter_version: "test",
        compilation_receipt: {
          schema_version: 1,
          recipe_id: "00000000-0000-4000-8000-000000000001",
          recipe_document_sha256: "1".repeat(64),
          recipe_analytical_sha256: "2".repeat(64),
          model_id: "model-v4",
          model_document_sha256: "3".repeat(64),
          model_scientific_sha256: "4".repeat(64),
          dataset_fingerprint: "raw-fingerprint-v1",
          compiler_target: "cbsem_plan_v2",
          compiler_version: "test",
          capability_cell: {
            registry_schema_version: 2,
            capability_id: "smartpls.cbsem",
            cell_id: "qpls3.cbsem.ml",
            capability_version: "cbsem_ml_v1",
          },
          plan_sha256: "5".repeat(64),
          analytical_identity_sha256: "6".repeat(64),
        },
        dataset_id: "raw-data",
        estimator_method_version: "test",
        moment_input_method_version: "cbsem_ml_exact_parameter_table_v3",
      },
      estimation: {} as InternalRecipeV4CbsemCompletedResultV1["analyticalResult"]["estimation"],
    },
  };
}

describe("Exact CB-SEM Recipe-v4 workspace contract", () => {
  it("exact-matches the shared Recipe-v4 request built from a real diagram model", async () => {
    const fixture = wave1DiagramCbsemRoundtrip;
    const nodes: Node<ConstructData>[] = [{
      id: "reputation",
      type: "construct",
      position: { x: 120, y: 90 },
      data: {
        label: "Corporate reputation",
        shortName: "REPUT",
        mode: "reflective",
        indicators: ["COMP1", "COMP2", "COMP3"],
      },
    }];
    const dataset: Dataset = {
      id: fixture.dataset.id,
      name: fixture.dataset.name,
      kind: "raw",
      columns: fixture.dataset.csv_text.slice(0, fixture.dataset.csv_text.indexOf("\n")).split(","),
      rows: [],
      rowCount: 12,
      missing: 0,
      fingerprint: fixture.dataset.fingerprint,
    };
    const model = requireNativeWorkbenchSemModelV4({
      model_id: "model:diagram:cbsem:reputation",
      model_name: "Diagram-origin reputation CFA",
      nodes,
      edges: [],
      diagram_layout: { diagramViewport: { x: 8, y: 12, zoom: 1.1 } },
      data_binding: {
        kind: "raw",
        dataset_id: dataset.id,
        missing_data: "listwise_deletion",
        weight: null,
        cluster_variable: null,
        strata_variable: null,
      },
      construct_estimands: {
        reputation: { kind: "common_factor", marker_indicator: "COMP1" },
      },
      covariance_semantics: {},
    });
    expect(model).toStrictEqual(fixture.expected_request.model);

    const request = await buildInternalLabsRecipeV4CbsemRequestV1({
      recipeId: fixture.expected_request.recipe.id,
      createdAt: fixture.expected_request.recipe.created_at,
      dataset,
      model,
      nativeScientificSha256: fixture.expected_request.recipe.model_binding.scientific_sha256,
      engine,
    });

    expect(request).toStrictEqual({
      ...fixture.expected_request,
      surface: "standard",
      experimentalLabsEnabled: false,
    });
    expect(request.recipe.model_binding.kind).toBe("embedded_sem_model_v4");
    if (request.recipe.model_binding.kind !== "embedded_sem_model_v4") throw new Error("expected embedded SemModelV4");
    expect(request.model).toStrictEqual(request.recipe.model_binding.model);
  });

  it("binds raw data without moving resident rows through the request", async () => {
    const dataset = rawDataset();
    const authored = baseModel();
    const authoredObserved = authored.variables.find((variable) => variable.kind === "observed");
    if (authoredObserved?.kind === "observed") authoredObserved.missing_markers = ["NA"];
    const model = bindInternalRecipeV4CbsemDatasetV1(authored, dataset, { covarianceDenominator: "sample_n_minus_one", missingDataPolicy: "listwise_deletion" });
    expect(model.variables.filter((variable) => variable.kind === "observed").every((variable) => variable.missing_markers.length === 0)).toBe(true);
    const preflight = preflightInternalRecipeV4CbsemWorkspaceV1({ experimentalLabsEnabled: true, projectName: "Study", projectPath: "D:\\Study.qpls", dataset, model, missingDataPolicy: "listwise_deletion", engine });
    expect(preflight.ready).toBe(true);

    const request = await buildInternalLabsRecipeV4CbsemRequestV1({
      recipeId: "00000000-0000-4000-8000-000000000001",
      createdAt: "2026-08-15T00:00:00Z",
      dataset,
      model,
      nativeScientificSha256,
      engine,
    });
    expect(request).toMatchObject({ surface: "standard", experimentalLabsEnabled: false, residentData: "project_resident", datasetId: "raw-data", compilerTarget: "cbsem_plan_v2" });
    expect(request.recipe.model_binding).toMatchObject({ kind: "embedded_sem_model_v4", scientific_sha256: nativeScientificSha256 });
    expect(request.recipe.settings.missing_data).toBe("listwise_deletion");
    expect(JSON.stringify(request)).not.toContain("\"rows\"");
  });

  it("binds each exact CFA bootstrap interval to the dedicated current capability cell", async () => {
    const dataset = rawDataset();
    const model = bindInternalRecipeV4CbsemDatasetV1(baseModel(), dataset, {
      covarianceDenominator: "sample_n_minus_one",
      missingDataPolicy: "listwise_deletion",
    });
    const intervals = ["percentile_type7", "analytic_studentized_type7", "bca_type7"] as const;

    for (const [index, bootstrapInterval] of intervals.entries()) {
      const bootstrapEngine = {
        ...engine,
        workers: 4,
        bootstrapSamples: 1_000,
        bootstrapInterval,
        bootstrapTestTail: "two_sided" as const,
      };
      expect(preflightInternalRecipeV4CbsemWorkspaceV1({
        experimentalLabsEnabled: false,
        projectName: "Study",
        projectPath: "D:\\Study.qpls",
        dataset,
        model,
        missingDataPolicy: "listwise_deletion",
        engine: bootstrapEngine,
      }).ready).toBe(true);

      const request = await buildInternalLabsRecipeV4CbsemRequestV1({
        recipeId: `00000000-0000-4000-8000-00000000001${index}`,
        createdAt: "2026-08-18T00:00:00Z",
        dataset,
        model,
        nativeScientificSha256,
        engine: bootstrapEngine,
      });
      expect(request.capabilityCell).toStrictEqual({
        registry_schema_version: 2,
        capability_id: "smartpls.cbsem_bootstrapping",
        cell_id: "qpls3.cbsem.bootstrap",
        capability_version: "cbsem_exact_case_bootstrap_v1",
      });
      expect(request.recipe.method_config).toMatchObject({
        kind: "cbsem",
        model_type: "cfa",
        bootstrap_samples: 1_000,
        bootstrap_v2: {
          algorithm: "case_resampling_full_ml",
          interval: bootstrapInterval,
        },
      });
    }
  });

  it("fails closed at the exact bootstrap raw/listwise/CFA and bounded-interval limits", () => {
    const dataset = rawDataset();
    const cfa = bindInternalRecipeV4CbsemDatasetV1(baseModel(), dataset, {
      covarianceDenominator: "sample_n_minus_one",
      missingDataPolicy: "mean_replacement",
    });
    const boundedEngine = {
      ...engine,
      workers: 13,
      bootstrapSamples: 1_000,
      bootstrapInterval: "bca_type7" as const,
      bootstrapTestTail: "one_sided_greater" as const,
    };
    const meanReplacement = preflightInternalRecipeV4CbsemWorkspaceV1({
      experimentalLabsEnabled: false,
      projectName: "Study",
      projectPath: "D:\\Study.qpls",
      dataset,
      model: cfa,
      missingDataPolicy: "mean_replacement",
      engine: boundedEngine,
    });
    expect(meanReplacement.issues).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: "recipe_v4.cbsem.bootstrap_listwise_required" }),
      expect.objectContaining({ code: "recipe_v4.cbsem.bootstrap_two_sided_required" }),
      expect.objectContaining({ code: "recipe_v4.cbsem.bootstrap_workers_bounded" }),
    ]));

    const matrix = matrixDataset();
    const matrixModel = bindInternalRecipeV4CbsemDatasetV1(baseModel(), matrix, {
      covarianceDenominator: "sample_n_minus_one",
      missingDataPolicy: "listwise_deletion",
    });
    expect(preflightInternalRecipeV4CbsemWorkspaceV1({
      experimentalLabsEnabled: false,
      projectName: "Study",
      projectPath: "D:\\Study.qpls",
      dataset: matrix,
      model: matrixModel,
      missingDataPolicy: "listwise_deletion",
      engine: { ...engine, bootstrapSamples: 1_000 },
    }).issues).toContainEqual(expect.objectContaining({ code: "recipe_v4.cbsem.bootstrap_raw_required" }));

    const sem = convertLegacyBasicModelV4({
      id: "recursive-sem",
      name: "Recursive SEM",
      constructs: [
        { id: "x", name: "X", short_name: "X", mode: "reflective", indicators: ["x1", "x2", "x3"] },
        { id: "y", name: "Y", short_name: "Y", mode: "reflective", indicators: ["y1", "y2", "y3"] },
      ],
      paths: [{ source: "x", target: "y" }],
    }, "cbsem_common_factor");
    const semDataset: Dataset = {
      ...dataset,
      columns: ["x1", "x2", "x3", "y1", "y2", "y3"],
      rows: dataset.rows.map((row, index) => ({ ...row, y1: index + 3, y2: index + 4, y3: index + 5 })),
    };
    const boundSem = bindInternalRecipeV4CbsemDatasetV1(sem, semDataset, {
      covarianceDenominator: "sample_n_minus_one",
      missingDataPolicy: "listwise_deletion",
    });
    expect(preflightInternalRecipeV4CbsemWorkspaceV1({
      experimentalLabsEnabled: false,
      projectName: "Study",
      projectPath: "D:\\Study.qpls",
      dataset: semDataset,
      model: boundSem,
      missingDataPolicy: "listwise_deletion",
      engine: { ...engine, bootstrapSamples: 1_000 },
    }).issues).toContainEqual(expect.objectContaining({ code: "recipe_v4.cbsem.bootstrap_cfa_required" }));
  });

  it("binds mean replacement exactly into SemModelV4 and Recipe-v4 while threshold warnings remain non-blocking", async () => {
    const dataset = {
      ...rawDataset(),
      rowCount: 20,
      rows: Array.from({ length: 20 }, (_, index) => ({ x1: index, x2: index + 1, x3: index + 2 })),
      missing: 8,
      missingByColumn: { x1: 1, x2: 3, x3: 4 },
    };
    const authored = baseModel();
    const authoredObserved = authored.variables.find((variable) => variable.kind === "observed");
    if (authoredObserved?.kind === "observed") authoredObserved.missing_markers = ["NA"];
    const model = bindInternalRecipeV4CbsemDatasetV1(authored, dataset, {
      covarianceDenominator: "sample_n_minus_one",
      missingDataPolicy: "mean_replacement",
    });
    expect(model.data_binding).toMatchObject({ kind: "raw", missing_data: "mean_replacement" });
    expect(model.variables.find((variable) => variable.kind === "observed")).toMatchObject({ missing_markers: ["NA"] });
    const preflight = preflightInternalRecipeV4CbsemWorkspaceV1({
      experimentalLabsEnabled: true,
      projectName: "Study",
      projectPath: "D:\\Study.qpls",
      dataset,
      model,
      missingDataPolicy: "mean_replacement",
      engine,
    });
    expect(preflight.ready).toBe(true);
    expect(preflight.warnings).toEqual([
      expect.objectContaining({ subject: "x1", severity: "warning", code: "recipe_v4.cbsem.variable_missing_rate_at_least_5_percent" }),
      expect.objectContaining({ subject: "x2", severity: "warning", code: "recipe_v4.cbsem.variable_missing_rate_at_least_5_percent" }),
      expect.objectContaining({ subject: "x3", severity: "high", code: "recipe_v4.cbsem.variable_missing_rate_above_15_percent" }),
    ]);

    const request = await buildInternalLabsRecipeV4CbsemRequestV1({
      recipeId: "00000000-0000-4000-8000-000000000002",
      createdAt: "2026-08-15T00:00:00Z",
      dataset,
      model,
      nativeScientificSha256,
      engine,
    });
    expect(request.recipe.settings.missing_data).toBe("mean_replacement");
    expect(request.model.data_binding).toMatchObject({ kind: "raw", missing_data: "mean_replacement" });
    expect(JSON.stringify(request)).not.toContain("\"rows\"");
  });

  it("preserves declared weight semantics during rebinding and blocks every current CB-SEM weight kind before request emission", async () => {
    const weightedDataset: Dataset = {
      ...rawDataset(),
      columns: ["x1", "x2", "x3", "survey_weight"],
      rows: rawDataset().rows.map((row, index) => ({ ...row, survey_weight: index + 1 })),
    };
    const cases = [
      [{ kind: "case" as const, variable: "observed:survey_weight" }, "case_weight_unsupported"],
      [{ kind: "frequency" as const, variable: "observed:survey_weight" }, "frequency_weight_unsupported"],
      [{ kind: "sampling" as const, variable: "observed:survey_weight", normalization: "mean_one" as const }, "sampling_weight_unsupported"],
    ] as const;

    for (const [weight, code] of cases) {
      const authored = baseModel();
      authored.variables.push({
        kind: "observed",
        id: "observed:survey_weight",
        label: "Survey weight",
        source_column: "survey_weight",
        scale: "continuous",
        role: "control",
        categories: [],
        value_labels: {},
        missing_markers: [],
        transformation_lineage: [],
      });
      if (authored.data_binding.kind !== "raw") throw new Error("expected raw fixture binding");
      authored.data_binding.weight = weight;
      const model = bindInternalRecipeV4CbsemDatasetV1(authored, weightedDataset, {
        covarianceDenominator: "sample_n_minus_one",
        missingDataPolicy: "listwise_deletion",
      });
      expect(model.data_binding).toMatchObject({ weight });
      const preflight = preflightInternalRecipeV4CbsemWorkspaceV1({
        experimentalLabsEnabled: true,
        projectName: "Study",
        projectPath: "D:\\Study.qpls",
        dataset: weightedDataset,
        model,
        missingDataPolicy: "listwise_deletion",
        engine,
      });
      expect(preflight.ready).toBe(false);
      expect(preflight.issues).toContainEqual(expect.objectContaining({
        stage: "recipe",
        code,
        subject: "survey_weight",
        correctiveAction: expect.stringContaining("no executable plan was emitted"),
      }));
      await expect(buildInternalLabsRecipeV4CbsemRequestV1({
        recipeId: "00000000-0000-4000-8000-000000000009",
        createdAt: "2026-08-15T00:00:00Z",
        dataset: weightedDataset,
        model,
        nativeScientificSha256,
        engine,
      })).rejects.toMatchObject({ code, subject: "survey_weight" });
    }

    const matrixAuthored = baseModel();
    matrixAuthored.variables.push({
      kind: "observed",
      id: "observed:survey_weight",
      label: "Survey weight",
      source_column: "survey_weight",
      scale: "continuous",
      role: "control",
      categories: [],
      value_labels: {},
      missing_markers: [],
      transformation_lineage: [],
    });
    if (matrixAuthored.data_binding.kind !== "raw") throw new Error("expected raw fixture binding");
    matrixAuthored.data_binding.weight = { kind: "case", variable: "observed:survey_weight" };
    expect(() => bindInternalRecipeV4CbsemDatasetV1(matrixAuthored, matrixDataset(), {
      covarianceDenominator: "sample_n_minus_one",
      missingDataPolicy: "listwise_deletion",
    })).toThrowError(expect.objectContaining({
      code: "case_weight_unsupported",
      subject: "survey_weight",
    }));
  });

  it("rejects a non-canonical native model digest without normalizing it", async () => {
    const dataset = rawDataset();
    const model = bindInternalRecipeV4CbsemDatasetV1(baseModel(), dataset, {
      covarianceDenominator: "sample_n_minus_one",
      missingDataPolicy: "listwise_deletion",
    });

    await expect(buildInternalLabsRecipeV4CbsemRequestV1({
      recipeId: "00000000-0000-4000-8000-000000000003",
      createdAt: "2026-08-15T00:00:00Z",
      dataset,
      model,
      nativeScientificSha256: "A".repeat(64),
      engine,
    })).rejects.toMatchObject({ code: "recipe_v4.cbsem.native_scientific_digest_invalid" });
  });

  it("fails preflight when selected and bound missing-data policies drift", () => {
    const dataset = rawDataset();
    const model = bindInternalRecipeV4CbsemDatasetV1(baseModel(), dataset, {
      covarianceDenominator: "sample_n_minus_one",
      missingDataPolicy: "mean_replacement",
    });
    const preflight = preflightInternalRecipeV4CbsemWorkspaceV1({
      experimentalLabsEnabled: true,
      projectName: "Study",
      projectPath: "D:\\Study.qpls",
      dataset,
      model,
      missingDataPolicy: "listwise_deletion",
      engine,
    });
    expect(preflight.ready).toBe(false);
    expect(preflight.issues).toContainEqual(expect.objectContaining({ code: "recipe_v4.cbsem.missing_data_policy_mismatch" }));
  });

  it("preserves matrix order and requires explicit correlation scales and denominator", () => {
    const covariance = matrixDataset("covariance");
    const covarianceModel = bindInternalRecipeV4CbsemDatasetV1(baseModel(), covariance, { covarianceDenominator: "maximum_likelihood_n", missingDataPolicy: "listwise_deletion" });
    expect(covarianceModel.data_binding).toMatchObject({
      kind: "covariance",
      variables: ["observed:x2", "observed:x1", "observed:x3"],
      sample: { sample_size: 120, covariance_denominator: "maximum_likelihood_n" },
    });

    const correlation = matrixDataset("correlation");
    const unscaled = bindInternalRecipeV4CbsemDatasetV1(baseModel(), correlation, { covarianceDenominator: "sample_n_minus_one", missingDataPolicy: "listwise_deletion" });
    expect(preflightInternalRecipeV4CbsemWorkspaceV1({ experimentalLabsEnabled: true, projectName: "Study", projectPath: "D:\\Study.qpls", dataset: correlation, model: unscaled, missingDataPolicy: "listwise_deletion", engine }).issues)
      .toEqual(expect.arrayContaining([expect.objectContaining({ code: "recipe_v4.cbsem.correlation_scales_required" })]));
    const scaled = bindInternalRecipeV4CbsemDatasetV1(baseModel(), correlation, {
      covarianceDenominator: "sample_n_minus_one",
      missingDataPolicy: "listwise_deletion",
      correlationStandardDeviations: { "observed:x1": 1.1, "observed:x2": 1.2, "observed:x3": 1.3 },
    });
    expect(preflightInternalRecipeV4CbsemWorkspaceV1({ experimentalLabsEnabled: true, projectName: "Study", projectPath: "D:\\Study.qpls", dataset: correlation, model: scaled, missingDataPolicy: "listwise_deletion", engine }).ready).toBe(true);
  });

  it("reports access, project, model, dataset, input, and recipe layers with corrective diagnostics", () => {
    const preflight = preflightInternalRecipeV4CbsemWorkspaceV1({ experimentalLabsEnabled: false, projectName: "No project open", projectPath: null, dataset: null, model: null, missingDataPolicy: "listwise_deletion", engine: { ...engine, workers: 0 } });
    expect(preflight.layers.map((layer) => layer.stage)).toEqual(["access", "project", "model", "dataset", "input", "recipe"]);
    expect(preflight.ready).toBe(false);
    expect(preflight.issues.every((diagnostic) => diagnostic.correctiveAction.length > 0)).toBe(true);
  });

  it("blocks a validly shaped but empty scientific model before native execution", () => {
    const dataset = rawDataset();
    const emptyModel = convertLegacyBasicModelV4({
      id: "empty-model-v4",
      name: "Empty model",
      constructs: [],
      paths: [],
    }, "cbsem_common_factor");
    const model = bindInternalRecipeV4CbsemDatasetV1(emptyModel, dataset, {
      covarianceDenominator: "sample_n_minus_one",
      missingDataPolicy: "listwise_deletion",
    });

    const preflight = preflightInternalRecipeV4CbsemWorkspaceV1({
      experimentalLabsEnabled: false,
      projectName: "Study",
      projectPath: "D:\\Study.qpls",
      dataset,
      model,
      missingDataPolicy: "listwise_deletion",
      engine: { ...engine, bootstrapSamples: 1_000 },
    });

    expect(preflight.ready).toBe(false);
    expect(preflight.issues).toContainEqual(expect.objectContaining({
      stage: "model",
      code: "recipe_v4.cbsem.common_factor_required",
      subject: "model",
    }));
  });

  it("blocks reciprocal structural paths before native execution", () => {
    const model = convertLegacyBasicModelV4({
      id: "feedback-model",
      name: "Feedback model",
      constructs: [
        { id: "x", name: "X", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
        { id: "y", name: "Y", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
      ],
      paths: [{ source: "x", target: "y" }],
    }, "cbsem_common_factor");
    model.relations.push({
      kind: "structural",
      id: "feedback:y-to-x",
      source: "construct:y",
      target: "construct:x",
      parameter: "feedback:y-to-x:parameter",
      intercept_parameter: null,
    });
    model.parameters.push({
      kind: "free",
      id: "feedback:y-to-x:parameter",
      label: "Y -> X",
      target: { kind: "regression", source: "construct:y", target: "construct:x" },
    });
    const x = model.variables.find((variable) => variable.id === "construct:x");
    if (x?.kind === "common_factor" && x.disturbance_policy.kind === "exogenous_variance") {
      const varianceId = x.disturbance_policy.parameter;
      x.disturbance_policy = { kind: "endogenous_disturbance", parameter: varianceId };
      const variance = model.parameters.find((candidate) => candidate.id === varianceId);
      if (variance) variance.target = { kind: "variance", endpoint: { kind: "disturbance_of", id: x.id } };
    }

    const preflight = preflightInternalRecipeV4CbsemWorkspaceV1({
      experimentalLabsEnabled: true,
      projectName: "Study",
      projectPath: "D:\\Study.qpls",
      dataset: null,
      model,
      missingDataPolicy: "listwise_deletion",
      engine,
    });

    expect(preflight.issues).toContainEqual(expect.objectContaining({
      stage: "model",
      code: "recipe_v4.cbsem.feedback_not_available",
      subject: "structural_model",
    }));
    expect(preflight.issues.find((candidate) => candidate.code === "recipe_v4.cbsem.feedback_not_available")?.correctiveAction).toContain("Remove one path");
  });

  it("blocks group-specific parameter overrides with a corrective action", () => {
    const model = baseModel();
    model.parameters[0]!.group_overrides = [{
      group: "group:a",
      specification: { kind: "fixed", value: 1 },
    }];

    const preflight = preflightInternalRecipeV4CbsemWorkspaceV1({
      experimentalLabsEnabled: true,
      projectName: "Study",
      projectPath: "D:\\Study.qpls",
      dataset: null,
      model,
      missingDataPolicy: "listwise_deletion",
      engine,
    });

    expect(preflight.issues).toContainEqual(expect.objectContaining({
      stage: "model",
      code: "recipe_v4.cbsem.parameter_group_overrides_not_available",
      subject: model.parameters[0]!.id,
      correctiveAction: "Remove the group override and use one shared parameter specification.",
    }));
  });

  it("publishes the native canonical document only after a completed terminal snapshot", async () => {
    const getResult = vi.fn(async () => completed());
    const updates: string[] = [];
    const states = [snapshot("running", 1), snapshot("completed", 3)];
    const outcome = await monitorInternalLabsRecipeV4CbsemJobV1({
      initial: snapshot("queued", 0),
      getStatus: vi.fn(async () => states.shift()!),
      getResult,
      wait: async () => undefined,
      onSnapshot: (value) => updates.push(value.state),
    });
    expect(updates).toEqual(["queued", "running", "completed"]);
    expect(outcome.status).toBe("completed");
    expect(getResult).toHaveBeenCalledTimes(1);
  });

  it("does not request a partial result after cancellation", async () => {
    const getResult = vi.fn(async () => completed());
    const outcome = await monitorInternalLabsRecipeV4CbsemJobV1({
      initial: snapshot("cancelling", 1),
      getStatus: vi.fn(async () => snapshot("cancelled", 1)),
      getResult,
      wait: async () => undefined,
    });
    expect(outcome).toMatchObject({ status: "terminal_without_result", snapshot: { state: "cancelled" } });
    expect(getResult).not.toHaveBeenCalled();
  });

  it("accepts only the current schema-6 project archive identity", () => {
    const currentArchiveInspection = {
      sourceArchivePath: "D:\\Study-v6.qpls",
      sourceArchiveSha256: "a".repeat(64),
      sourceKind: "project_archive" as const,
      schemaVersion: 6,
      access: "current_v6_archive" as const,
      readOnly: true,
      upgradeAvailable: false,
      projectId: "project-v6",
      projectName: "Study",
      counts: { datasets: 1, models: 1, recipes: 1, results: 0 },
      futureUnsupported: { models: 0, recipes: 0, results: 0 },
      sourceWillRemainUnchanged: true,
      destinationMustBeNew: true,
    } satisfies ProjectUpgradeInspectionV1;
    expect(schema6ArchiveIdentityFromInspectionV1(currentArchiveInspection)).toEqual({
      archivePath: currentArchiveInspection.sourceArchivePath,
      sourceSha256: currentArchiveInspection.sourceArchiveSha256,
      projectId: currentArchiveInspection.projectId,
    });

    expect(() => schema6ArchiveIdentityFromInspectionV1({
      ...currentArchiveInspection,
      sourceKind: "standalone_document",
      access: "current_v6_standalone",
      readOnly: false,
    })).toThrowError("The selected file is not a current schema-6 project document.");
    expect(() => schema6ArchiveIdentityFromInspectionV1({
      ...currentArchiveInspection,
      schemaVersion: 5,
      access: "historical_upgrade_copy_required",
      upgradeAvailable: true,
    })).toThrowError("The selected file is not a current schema-6 project document.");
    expect(() => schema6ArchiveIdentityFromInspectionV1({
      ...currentArchiveInspection,
      schemaVersion: 7,
      access: "future_read_only",
    })).toThrowError("The selected file is not a current schema-6 project document.");
  });

  it("passes the exact native document to schema 6 and reopens that immutable document", async () => {
    const result = completed();
    const recipe = { id: "00000000-0000-4000-8000-000000000001" } as InternalLabsRecipeV4CbsemExecutionRequestV1["recipe"];
    const archive = schema6ArchiveIdentityFromInspectionV1({
      sourceArchivePath: "D:\\Study-v6.qpls",
      sourceArchiveSha256: "a".repeat(64),
      sourceKind: "project_archive",
      schemaVersion: 6,
      access: "current_v6_archive",
      readOnly: true,
      upgradeAvailable: false,
      projectId: "project-v6",
      projectName: "Study",
      counts: { datasets: 1, models: 1, recipes: 1, results: 0 },
      futureUnsupported: { models: 0, recipes: 0, results: 0 },
      sourceWillRemainUnchanged: true,
      destinationMustBeNew: true,
    });
    let forwarded: CanonicalResultDocumentV2 | null = null;
    const append = vi.fn(async (request) => {
      forwarded = request.canonicalDocument;
      return { status: "ok" as const, value: {
        schema_version: 6 as const,
        project_id: "project-v6",
        archive_path: archive.archivePath,
        source_document_sha256: archive.sourceSha256,
        updated_document_sha256: "b".repeat(64),
        canonical_document_id: "document-v4",
        run_id: "run-v4",
        canonical_result_document_count: 1,
        source_verified_at_commit: true,
        post_write_validated: true,
        rollback_copy_removed: true,
      } };
    });
    expect((await appendInternalLabsRecipeV4CbsemResultV1(result, recipe, archive, append)).status).toBe("ok");
    expect(forwarded).toBe(result.canonicalDocument);
    expect(append).toHaveBeenCalledWith(expect.objectContaining({ recipe }));

    const entry = {
      documentId: "document-v4",
      runId: "run-v4",
      canonicalDocumentSha256: "c".repeat(64),
      immutable: true as const,
      canonicalDocumentJson: JSON.stringify(result.canonicalDocument),
      canonicalDocument: result.canonicalDocument,
    };
    const reopened = await reopenInternalLabsRecipeV4CbsemResultV1(result, { ...archive, sourceSha256: "b".repeat(64) }, vi.fn(async () => ({ status: "ok" as const, value: {
      schemaVersion: 1 as const,
      projectId: "project-v6",
      archivePath: archive.archivePath,
      sourceDocumentSha256: "b".repeat(64),
      canonicalResultDocumentCount: 1,
      documents: [entry],
      sourceRecheckedUnchanged: true as const,
    } })));
    expect(reopened.entry?.canonicalDocument).toBe(result.canonicalDocument);
  });

  it("reads only immutable exact-bootstrap documents for selection after a relaunch", async () => {
    const pointDocument = canonicalDocument();
    const exactDocument = canonicalDocument();
    exactDocument.document_id = "document-exact-bootstrap";
    exactDocument.provenance.run_id = "run-exact-bootstrap";
    exactDocument.provenance.capability_cell = {
      registry_schema_version: 2,
      capability_id: "smartpls.cbsem_bootstrapping",
      cell_id: "qpls3.cbsem.bootstrap",
      capability_version: "cbsem_exact_case_bootstrap_v1",
    };
    const entry = (document: CanonicalResultDocumentV2) => ({
      documentId: document.document_id,
      runId: document.provenance.run_id,
      canonicalDocumentSha256: "c".repeat(64),
      immutable: true as const,
      canonicalDocumentJson: JSON.stringify(document),
      canonicalDocument: document,
    });
    const pointEntry = entry(pointDocument);
    const exactEntry = entry(exactDocument);
    expect(storedExactCaseBootstrapEntriesV1([pointEntry, exactEntry])).toEqual([exactEntry]);

    const archive = {
      archivePath: "D:\\Study-v6.qpls",
      sourceSha256: "a".repeat(64),
      projectId: "project-v6",
    };
    const read = vi.fn(async () => ({ status: "ok" as const, value: {
      schemaVersion: 1 as const,
      projectId: "project-v6",
      archivePath: archive.archivePath,
      sourceDocumentSha256: archive.sourceSha256,
      canonicalResultDocumentCount: 2,
      documents: [pointEntry, exactEntry],
      sourceRecheckedUnchanged: true as const,
    } }));
    const stored = await readStoredInternalLabsRecipeV4CbsemResultsV1(archive, read);
    expect(stored.entries).toEqual([exactEntry]);
    expect(read).toHaveBeenCalledWith({
      surface: "standard_exact_cbsem",
      experimentalLabsEnabled: false,
      archivePath: archive.archivePath,
      expectedSourceSha256: archive.sourceSha256,
    });
  });

  it("blocks stored-result selection if the inspected project identity changes", async () => {
    const stored = await readStoredInternalLabsRecipeV4CbsemResultsV1({
      archivePath: "D:\\Study-v6.qpls",
      sourceSha256: "a".repeat(64),
      projectId: "project-v6",
    }, vi.fn(async () => ({ status: "ok" as const, value: {
      schemaVersion: 1 as const,
      projectId: "other-project",
      archivePath: "D:\\Study-v6.qpls",
      sourceDocumentSha256: "a".repeat(64),
      canonicalResultDocumentCount: 0,
      documents: [],
      sourceRecheckedUnchanged: true as const,
    } })));
    expect(stored).toMatchObject({
      outcome: { status: "blocked", diagnostic: { code: "schema6.cbsem.project_identity_mismatch" } },
      entries: [],
    });
  });

  it("blocks a cross-project append before calling the native writer", async () => {
    const append = vi.fn();
    const recipe = { id: "00000000-0000-4000-8000-000000000001" } as InternalLabsRecipeV4CbsemExecutionRequestV1["recipe"];
    const outcome = await appendInternalLabsRecipeV4CbsemResultV1(completed("project-a"), recipe, {
      archivePath: "D:\\Other.qpls",
      sourceSha256: "a".repeat(64),
      projectId: "project-b",
    }, append);
    expect(outcome).toMatchObject({ status: "blocked", diagnostic: { code: "schema6.cbsem.project_identity_mismatch" } });
    expect(append).not.toHaveBeenCalled();
  });
});
