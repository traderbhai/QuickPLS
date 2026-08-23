import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  activateNativeDataset,
  adoptNativeSchema6RevisionSourceV1,
  appendInternalProjectSchema6CanonicalResultV2,
  autosaveNativeProject,
  cancelInternalProjectUpgradeV6,
  cancelInternalLabsRecipeV4CbsemJob,
  cancelInternalLabsRecipeV4PlsJob,
  cancelNativeAnalysisJob,
  cancelNativeDiagnosticBundlePreview,
  cancelNativePlsJob,
  clearNativeSchema6RevisionSourceV1,
  createNativeProject,
  dismissNativeAnalysisJob,
  dismissNativePlsJob,
  dismissInternalLabsRecipeV4CbsemJob,
  dismissInternalLabsRecipeV4PlsJob,
  exportNativeTextFile,
  exportNativeXlsxTables,
  executeInternalProjectUpgradeV6,
  getNativeAnalysisJob,
  getNativeAnalysisJobResult,
  getNativeDatasetRows,
  getInternalLabsRecipeV4CbsemJob,
  getInternalLabsRecipeV4CbsemJobResult,
  getInternalSemModelV4ScientificSha256,
  getInternalLabsRecipeV4PlsJob,
  getInternalLabsRecipeV4PlsJobResult,
  getNativePlsJob,
  getNativePlsJobResult,
  invalidateNativeGeneralSemFreshDraftAuthorityV1,
  importNativeDatasetAtPathForValidation,
  mutateNativeProjectExplorer,
  inspectInternalProjectUpgradeV6,
  openNativeDemoProject,
  profileNativeDatasetGroups,
  previewNativeDatasetTransformation,
  applyNativeDatasetTransformation,
  previewNativeDiagnosticBundle,
  planInternalProjectUpgradeV6,
  persistInternalLabsRecipeV4CbsemJobResultToSchema6,
  persistInternalLabsRecipeV4PlsJobResultToSchema6,
  recodeNativeDatasetColumn,
  readInternalProjectSchema6CanonicalResultsV2,
  runInternalLabsRecipeV4CbsemExecution,
  runInternalLabsRecipeV4PlsExecution,
  saveNativeProject,
  saveNativeDiagnosticBundle,
  startNativeAnalysisJob,
  startNativePlsJob,
  startInternalLabsRecipeV4CbsemJob,
  startInternalLabsRecipeV4PlsJob,
} from "./projectService";
import type { NativeCanonicalModelSpec, NativeModelPresentation, RecodeColumnSpec } from "../types";
import type { DatasetTransformationSpecV2 } from "../domain/datasetTransformationsV2";
import type {
  AnalysisRecipeV4,
  InternalLabsRecipeV4PlsExecutionRequestV1,
} from "../domain/internalRecipeV4PlsExecution";
import type { SemModelV4 } from "../domain/semModelV4";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import type { InternalLabsRecipeV4CbsemExecutionRequestV1 } from "../domain/internalRecipeV4CbsemExecution";
import { parseNativeSampleProjectId } from "../domain/bundledSampleCatalog";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), save: vi.fn() }));

async function sha256Text(value: string) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: mocks.save }));
vi.mock("../internalProjectArchiveV6SessionStore", () => ({
  useInternalProjectArchiveV6Session: { getState: () => ({ session: null }) },
}));
vi.mock("../store", () => ({
  useWorkspace: { getState: () => ({ generalSemPublicationPending: false }) },
}));

function legacyRecipeV4PlsResultFixture() {
  return {
    schema_version: 1,
    provenance: {
      adapter_version: "compiled_recipe_v4_pls_plan_v2_execution_v3",
      compilation_receipt: { analytical_identity_sha256: "b".repeat(64) },
      projected_recipe_schema_version: 3,
      projected_recipe_sha256: "c".repeat(64),
      dataset_id: "dataset-1",
      estimator_method_version: "pls_pm_v1",
    },
    estimation: { method_version: "pls_pm_v1", iterations: 4, converged: true },
  };
}

function legacyRecipeV4PlsCanonicalFixture(): CanonicalResultDocumentV2 {
  return {
    schema_version: 2,
    document_id: "result.contract:pls-v1",
    title: "PLS result",
    provenance: {
      run_id: "run-pls-v1",
      project_id: "project-1",
      model_id: "model-1",
      model_digest: "a".repeat(64),
      dataset_id: "dataset-1",
      dataset_fingerprint: "b".repeat(64),
      recipe_id: "recipe-1",
      recipe_digest: "c".repeat(64),
      capability_cell: {
        registry_schema_version: 2,
        capability_id: "smartpls.pls_algorithm",
        cell_id: "qpls3.pls.algorithm",
        capability_version: "pls_pm_v1",
      },
      method_version: "pls_pm_v1",
      engine_version: "compiled_recipe_v4_pls_plan_v2_execution_v3",
      seed: 42,
      workers: 1,
      started_at: "2026-08-14T00:00:00Z",
      completed_at: "2026-08-14T00:00:01Z",
    },
    sections: [],
    tables: [],
    charts: [],
    notices: [],
    exclusions: [],
    footnotes: [],
    presentation: {
      default_section_id: null,
      default_table_id: null,
      precision: 4,
      missing_value_label: "N/A",
      chart_defaults: {},
    },
  };
}

function legacyRecipeV4PlsCompletedFixture() {
  return {
    schemaVersion: 1,
    analyticalResult: legacyRecipeV4PlsResultFixture(),
    canonicalDocument: legacyRecipeV4PlsCanonicalFixture(),
  };
}

function recipeV4CbsemExecutionFixture() {
  const datasetFingerprint = "b".repeat(64);
  return {
    schema_version: 1,
    provenance: {
      adapter_version: "compiled_recipe_v4_cbsem_plan_v2_execution_v4",
      compilation_receipt: {
        schema_version: 1,
        recipe_id: "00000000-0000-4000-8000-000000000001",
        recipe_document_sha256: "1".repeat(64),
        recipe_analytical_sha256: "2".repeat(64),
        model_id: "model-v4",
        model_document_sha256: "3".repeat(64),
        model_scientific_sha256: "4".repeat(64),
        dataset_fingerprint: datasetFingerprint,
        compiler_target: "cbsem_plan_v2",
        compiler_version: "sem_model_v4_cbsem_compiler_v2",
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
      estimator_method_version: "cbsem_ml_exact_parameter_table_v3",
      moment_input_method_version: "cbsem_ml_compiled_moment_input_mean_replacement_v1",
    },
    estimation: {
      schema_version: 4,
      method_version: "cbsem_ml_compiled_moment_input_mean_replacement_v1",
      compiler_analytical_identity_sha256: "6".repeat(64),
      plan_sha256: "5".repeat(64),
      model_scientific_sha256: "4".repeat(64),
      input: {
        kind: "raw",
        dataset_id: "raw-data",
        dataset_fingerprint: datasetFingerprint,
        declared_sample_size: null,
        used_sample_size: 20,
        omitted_observations: 0,
        covariance_denominator: "maximum_likelihood_n",
        variable_ids: ["observed:x1"],
        source_columns: ["x1"],
        standard_deviations: null,
        canonical_ml_covariance_sha256: "7".repeat(64),
        missing_data_treatment: {
          method_version: "mean_replacement_v1",
          policy: "mean_replacement",
          source_dataset_id: "raw-data",
          source_dataset_fingerprint: datasetFingerprint,
          source_row_count: 20,
          retained_row_count: 20,
          omitted_row_count: 0,
          modeled_variable_count: 1,
          imputed_cell_count: 1,
          affected_case_count: 1,
          variable_warning_threshold: 0.05,
          high_missingness_threshold: 0.15,
          variables: [{
            variable_order: 0,
            variable_id: "observed:x1",
            source_column: "x1",
            canonical_missing_markers: ["NA"],
            observed_count: 19,
            missing_count: 1,
            replacement_mean: 10,
            missing_fraction: 0.05,
            warning_level: "at_least_five_percent",
          }],
          cases: [{
            row_index_zero_based: 0,
            imputed_variable_ids: ["observed:x1"],
            missing_fraction: 1,
            high_missingness_warning: true,
          }],
          missingness_sha256: "8".repeat(64),
          completed_matrix_sha256: "9".repeat(64),
          receipt_sha256: "a".repeat(64),
        },
      },
      covariance_ml: [[1]],
      parameter_ids: {},
      analysis: { method_version: "cbsem_ml_exact_parameter_table_v3" },
    },
  };
}

function recipeV4CbsemCompletedFixture() {
  const analyticalResult = recipeV4CbsemExecutionFixture();
  const canonicalDocument = JSON.parse(
    JSON.stringify(legacyRecipeV4PlsCanonicalFixture()),
  ) as CanonicalResultDocumentV2;
  canonicalDocument.provenance.capability_cell = {
    registry_schema_version: 2,
    capability_id: "smartpls.cbsem",
    cell_id: "qpls3.cbsem.ml",
    capability_version: "cbsem_ml_v1",
  };
  canonicalDocument.provenance.dataset_id = analyticalResult.estimation.input.dataset_id;
  canonicalDocument.provenance.dataset_fingerprint = analyticalResult.estimation.input.dataset_fingerprint;
  canonicalDocument.provenance.method_version = analyticalResult.provenance.estimator_method_version;
  canonicalDocument.provenance.engine_version = analyticalResult.provenance.adapter_version;
  return { schemaVersion: 1, analyticalResult, canonicalDocument };
}

describe("native dataset row paging service", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
  });

  it("requests an id-scoped offset and limit without loading the entire dataset", async () => {
    const response = {
      datasetId: "dataset-1",
      offset: 250,
      limit: 50,
      rowCount: 10_000,
      rows: [{ score: "7" }],
    };
    mocks.invoke.mockResolvedValue(response);

    await expect(getNativeDatasetRows("dataset-1", 250, 50)).resolves.toEqual(response);
    expect(mocks.invoke).toHaveBeenCalledWith("dataset_rows", {
      datasetId: "dataset-1",
      offset: 250,
      limit: 50,
    });
  });

  it("profiles groups against the full native dataset and exact model columns", async () => {
    const response = {
      datasetId: "dataset-1",
      columnName: "region",
      rowCount: 240,
      missingCount: 2,
      unsupportedCount: 0,
      truncated: false,
      groups: [
        { value: "north", label: "North", observations: 120, completeCases: 116 },
        { value: "south", label: "South", observations: 118, completeCases: 114 },
      ],
    };
    mocks.invoke.mockResolvedValue(response);

    await expect(profileNativeDatasetGroups("dataset-1", "region", ["x1", "x2", "y1", "y2"]))
      .resolves.toEqual(response);
    expect(mocks.invoke).toHaveBeenCalledWith("profile_dataset_groups", {
      datasetId: "dataset-1",
      columnName: "region",
      analysisColumns: ["x1", "x2", "y1", "y2"],
    });
  });
});

describe("native generic analysis job service", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
  });

  it("uses method-neutral desktop commands without changing payload shapes", async () => {
    const recipe = { method: "pls_sem", settings: { seed: 20260812 } };
    mocks.invoke.mockResolvedValue({ id: "job-1", state: "queued" });

    await startNativeAnalysisJob(recipe);
    await getNativeAnalysisJob("job-1");
    await cancelNativeAnalysisJob("job-1");
    await dismissNativeAnalysisJob("job-1");
    await getNativeAnalysisJobResult("job-1");

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "start_analysis_job", { recipe });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "analysis_job_status", { jobId: "job-1" });
    expect(mocks.invoke).toHaveBeenNthCalledWith(3, "cancel_analysis_job", { jobId: "job-1" });
    expect(mocks.invoke).toHaveBeenNthCalledWith(4, "dismiss_analysis_job", { jobId: "job-1" });
    expect(mocks.invoke).toHaveBeenNthCalledWith(5, "analysis_job_result", { jobId: "job-1" });
  });

  it("keeps legacy PLS service names as delegates to the generic command contract", async () => {
    const recipe = { method: "ols_regression" };
    mocks.invoke.mockResolvedValue({ id: "job-2", state: "running" });

    await startNativePlsJob(recipe);
    await getNativePlsJob("job-2");
    await cancelNativePlsJob("job-2");
    await dismissNativePlsJob("job-2");
    await getNativePlsJobResult("job-2");

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "start_analysis_job", { recipe });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "analysis_job_status", { jobId: "job-2" });
    expect(mocks.invoke).toHaveBeenNthCalledWith(3, "cancel_analysis_job", { jobId: "job-2" });
    expect(mocks.invoke).toHaveBeenNthCalledWith(4, "dismiss_analysis_job", { jobId: "job-2" });
    expect(mocks.invoke).toHaveBeenNthCalledWith(5, "analysis_job_result", { jobId: "job-2" });
  });
});

describe("internal Labs recipe-v4 PLS execution service", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
  });

  it("passes the exact typed resident-data request and returns the ephemeral receipt", async () => {
    const model = {
      schema_version: 4,
      id: "model-v4",
      name: "Internal fixture",
    } as SemModelV4;
    const recipe = {
      schema_version: 4,
      id: "00000000-0000-0000-0000-000000000004",
      created_at: "2026-08-14T00:00:00Z",
      dataset_fingerprint: "dataset-sha256",
      model_binding: {
        kind: "embedded_sem_model_v4",
        model,
        scientific_sha256: "a".repeat(64),
      },
      estimand_confirmation: "confirmed_composite",
      settings: {},
      method_config: { kind: "pls_algorithm" },
      metadata: {},
    } as AnalysisRecipeV4;
    const request: InternalLabsRecipeV4PlsExecutionRequestV1 = {
      surface: "internal_labs",
      experimentalLabsEnabled: true,
      residentData: "project_resident",
      datasetId: "dataset-1",
      datasetFingerprint: "dataset-sha256",
      recipe,
      model,
      compilerTarget: "pls_plan_v2",
      capabilityCell: {
        registry_schema_version: 2,
        capability_id: "smartpls.pls_algorithm",
        cell_id: "qpls3.pls.algorithm",
        capability_version: "pls_pm_v1",
      },
    };
    const response = legacyRecipeV4PlsResultFixture();
    mocks.invoke.mockResolvedValue(response);

    await expect(runInternalLabsRecipeV4PlsExecution(request)).resolves.toEqual(response);
    expect(mocks.invoke).toHaveBeenCalledWith(
      "run_internal_labs_recipe_v4_pls_execution",
      { request },
    );
  });

  it("exposes start, status, cancel, result, and terminal dismissal as separate job calls", async () => {
    const model = {
      schema_version: 4,
      id: "model-v4",
      name: "Internal fixture",
    } as SemModelV4;
    const request = {
      surface: "internal_labs",
      experimentalLabsEnabled: true,
      residentData: "project_resident",
      datasetId: "dataset-1",
      datasetFingerprint: "dataset-sha256",
      recipe: {
        schema_version: 4,
        id: "00000000-0000-0000-0000-000000000004",
        created_at: "2026-08-14T00:00:00Z",
        dataset_fingerprint: "dataset-sha256",
        model_binding: {
          kind: "embedded_sem_model_v4",
          model,
          scientific_sha256: "a".repeat(64),
        },
        estimand_confirmation: "confirmed_composite",
        settings: {},
        method_config: { kind: "pls_algorithm" },
        metadata: {},
      } as AnalysisRecipeV4,
      model,
      compilerTarget: "pls_plan_v2",
      capabilityCell: {
        registry_schema_version: 2,
        capability_id: "smartpls.pls_algorithm",
        cell_id: "qpls3.pls.algorithm",
        capability_version: "pls_pm_v1",
      },
    } satisfies InternalLabsRecipeV4PlsExecutionRequestV1;
    mocks.invoke
      .mockResolvedValueOnce({ schemaVersion: 1, jobId: "job-v4" })
      .mockResolvedValueOnce({ schemaVersion: 1, jobId: "job-v4" })
      .mockResolvedValueOnce({ schemaVersion: 1, jobId: "job-v4" })
      .mockResolvedValueOnce(legacyRecipeV4PlsCompletedFixture())
      .mockResolvedValueOnce(undefined);

    await startInternalLabsRecipeV4PlsJob(request);
    await getInternalLabsRecipeV4PlsJob("job-v4");
    await cancelInternalLabsRecipeV4PlsJob("job-v4");
    await getInternalLabsRecipeV4PlsJobResult("job-v4");
    await dismissInternalLabsRecipeV4PlsJob("job-v4");

    expect(mocks.invoke).toHaveBeenNthCalledWith(
      1,
      "start_internal_labs_recipe_v4_pls_job",
      { request },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      2,
      "internal_labs_recipe_v4_pls_job_status",
      { jobId: "job-v4" },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      3,
      "cancel_internal_labs_recipe_v4_pls_job",
      { jobId: "job-v4" },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      4,
      "internal_labs_recipe_v4_pls_job_result",
      { jobId: "job-v4" },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      5,
      "dismiss_internal_labs_recipe_v4_pls_job",
      { jobId: "job-v4" },
    );
  });
});

describe("internal Labs recipe-v4 CB-SEM execution service", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
  });

  const request = (): InternalLabsRecipeV4CbsemExecutionRequestV1 => {
    const model = {
      schema_version: 4,
      id: "cbsem-model-v4",
      name: "CB-SEM internal fixture",
    } as SemModelV4;
    return {
      surface: "internal_labs",
      experimentalLabsEnabled: true,
      residentData: "project_resident",
      datasetId: "matrix-dataset-1",
      datasetFingerprint: "dataset-sha256",
      recipe: {
        schema_version: 4,
        id: "00000000-0000-0000-0000-00000000cb51",
        created_at: "2026-08-15T00:00:00Z",
        dataset_fingerprint: "dataset-sha256",
        model_binding: {
          kind: "embedded_sem_model_v4",
          model,
          scientific_sha256: "a".repeat(64),
        },
        estimand_confirmation: "confirmed_common_factor",
        settings: {},
        method_config: { kind: "cbsem" },
        metadata: {},
      } as AnalysisRecipeV4,
      model,
      compilerTarget: "cbsem_plan_v2",
      capabilityCell: {
        registry_schema_version: 2,
        capability_id: "smartpls.cbsem",
        cell_id: "qpls3.cbsem.ml",
        capability_version: "cbsem_ml_v1",
      },
    };
  };

  it("uses the native SemModelV4 digest authority and rejects malformed native output", async () => {
    const model = request().model;
    const nativeDigest = "ab".repeat(32);
    mocks.invoke.mockResolvedValueOnce(nativeDigest);

    await expect(getInternalSemModelV4ScientificSha256(model)).resolves.toBe(nativeDigest);
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      1,
      "internal_sem_model_v4_scientific_sha256",
      { model },
    );

    mocks.invoke.mockResolvedValueOnce("A".repeat(64));
    await expect(getInternalSemModelV4ScientificSha256(model)).rejects.toThrow(/exact lowercase SHA-256/);
  });

  it("passes the exact capability-bound request and exposes the cancellable lifecycle", async () => {
    const executionRequest = request();
    const execution = recipeV4CbsemExecutionFixture();
    const completed = recipeV4CbsemCompletedFixture();
    const snapshot = { schemaVersion: 1, jobId: "job-cbsem-v4" };
    mocks.invoke
      .mockResolvedValueOnce(execution)
      .mockResolvedValueOnce(snapshot)
      .mockResolvedValueOnce(snapshot)
      .mockResolvedValueOnce(snapshot)
      .mockResolvedValueOnce(completed)
      .mockResolvedValueOnce(undefined);

    await expect(runInternalLabsRecipeV4CbsemExecution(executionRequest)).resolves.toEqual(execution);
    await startInternalLabsRecipeV4CbsemJob(executionRequest);
    await getInternalLabsRecipeV4CbsemJob("job-cbsem-v4");
    await cancelInternalLabsRecipeV4CbsemJob("job-cbsem-v4");
    await getInternalLabsRecipeV4CbsemJobResult("job-cbsem-v4");
    await dismissInternalLabsRecipeV4CbsemJob("job-cbsem-v4");

    expect(mocks.invoke).toHaveBeenNthCalledWith(
      1,
      "run_internal_labs_recipe_v4_cbsem_execution",
      { request: executionRequest },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      2,
      "start_internal_labs_recipe_v4_cbsem_job",
      { request: executionRequest },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      3,
      "internal_labs_recipe_v4_cbsem_job_status",
      { jobId: "job-cbsem-v4" },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      4,
      "cancel_internal_labs_recipe_v4_cbsem_job",
      { jobId: "job-cbsem-v4" },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      5,
      "internal_labs_recipe_v4_cbsem_job_result",
      { jobId: "job-cbsem-v4" },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      6,
      "dismiss_internal_labs_recipe_v4_cbsem_job",
      { jobId: "job-cbsem-v4" },
    );
  });

  it("rejects untrusted direct and completed CB-SEM payloads before they reach consumers", async () => {
    const executionRequest = request();
    const direct = recipeV4CbsemExecutionFixture() as ReturnType<typeof recipeV4CbsemExecutionFixture> & { unexpected?: boolean };
    direct.unexpected = true;
    mocks.invoke.mockResolvedValueOnce(direct);
    await expect(runInternalLabsRecipeV4CbsemExecution(executionRequest))
      .rejects.toThrow(/unknown unexpected/);

    const completed = recipeV4CbsemCompletedFixture() as ReturnType<typeof recipeV4CbsemCompletedFixture> & { unexpected?: boolean };
    completed.unexpected = true;
    mocks.invoke.mockResolvedValueOnce(completed);
    await expect(getInternalLabsRecipeV4CbsemJobResult("job-cbsem-v4"))
      .rejects.toThrow(/unknown unexpected/);
  });

  it("passes the native-built canonical document unchanged to the schema-6 writer", async () => {
    const completed = recipeV4CbsemCompletedFixture();
    const canonicalDocument = completed.canonicalDocument;
    const appendOutcome = { status: "ok", value: { schema_version: 6 } };
    mocks.invoke.mockResolvedValueOnce(completed).mockResolvedValueOnce(appendOutcome);

    await expect(
      persistInternalLabsRecipeV4CbsemJobResultToSchema6(
        "job-cbsem-v4",
        "D:\\study-v6.json",
        "d".repeat(64),
      ),
    ).resolves.toEqual({ completed, appendOutcome });
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      1,
      "internal_labs_recipe_v4_cbsem_job_result",
      { jobId: "job-cbsem-v4" },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      2,
      "append_internal_project_schema6_canonical_result_v2",
      {
        request: {
          surface: "internal_labs",
          experimentalLabsEnabled: true,
          archivePath: "D:\\study-v6.json",
          expectedSourceSha256: "d".repeat(64),
          canonicalDocument,
        },
      },
    );
  });
});

describe("internal schema-6 canonical result append service", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
  });

  it("passes the exact digest-bound internal request without rewriting fields", async () => {
    const request = {
      surface: "internal_labs" as const,
      experimentalLabsEnabled: true as const,
      archivePath: "D:\\study-v6.json",
      expectedSourceSha256: "a".repeat(64),
      canonicalDocument: { schema_version: 2 } as CanonicalResultDocumentV2,
    };
    const response = { status: "ok", value: { schema_version: 6 } };
    mocks.invoke.mockResolvedValue(response);

    await expect(appendInternalProjectSchema6CanonicalResultV2(request)).resolves.toEqual(response);
    expect(mocks.invoke).toHaveBeenCalledWith(
      "append_internal_project_schema6_canonical_result_v2",
      { request },
    );
  });

  it("persists the native-built job document without rebuilding scientific tables in TypeScript", async () => {
    const completed = legacyRecipeV4PlsCompletedFixture();
    const canonicalDocument = completed.canonicalDocument;
    const appendOutcome = { status: "ok", value: { schema_version: 6 } };
    mocks.invoke
      .mockResolvedValueOnce(completed)
      .mockResolvedValueOnce(appendOutcome);

    await expect(
      persistInternalLabsRecipeV4PlsJobResultToSchema6(
        "job-v4",
        "D:\\study-v6.json",
        "a".repeat(64),
      ),
    ).resolves.toEqual({ completed, appendOutcome });
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      1,
      "internal_labs_recipe_v4_pls_job_result",
      { jobId: "job-v4" },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      2,
      "append_internal_project_schema6_canonical_result_v2",
      {
        request: {
          surface: "internal_labs",
          experimentalLabsEnabled: true,
          archivePath: "D:\\study-v6.json",
          expectedSourceSha256: "a".repeat(64),
          canonicalDocument,
        },
      },
    );
  });
});

describe("internal schema-6 canonical result read service", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
  });

  it("passes the exact read-only digest-bound request", async () => {
    const request = {
      surface: "internal_labs" as const,
      experimentalLabsEnabled: true as const,
      archivePath: "D:\\study-v6.json",
      expectedSourceSha256: "b".repeat(64),
    };
    const response = {
      status: "ok",
      value: {
        schemaVersion: 1,
        projectId: "project-1",
        archivePath: request.archivePath,
        sourceDocumentSha256: request.expectedSourceSha256,
        canonicalResultDocumentCount: 0,
        documents: [],
        sourceRecheckedUnchanged: true,
      },
    };
    mocks.invoke.mockResolvedValue(response);

    await expect(readInternalProjectSchema6CanonicalResultsV2(request)).resolves.toEqual(response);
    expect(mocks.invoke).toHaveBeenCalledWith(
      "read_internal_project_schema6_canonical_results_v2",
      { request },
    );
  });
});

describe("internal schema-6 project upgrade service", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
  });

  it("keeps inspect and plan calls in the internal Labs surface", async () => {
    mocks.invoke.mockResolvedValue({ status: "ok", value: { state: "ready" } });

    await inspectInternalProjectUpgradeV6("D:\\study.qpls");
    await planInternalProjectUpgradeV6({
      sourceArchivePath: "D:\\study.qpls",
      destinationArchivePath: "D:\\study-v6.qpls",
      expectedSourceArchiveSha256: "a".repeat(64),
    });

    expect(mocks.invoke).toHaveBeenNthCalledWith(
      1,
      "inspect_internal_project_upgrade_v6",
      {
        request: {
          surface: "internal_labs",
          experimentalLabsEnabled: true,
          sourceArchivePath: "D:\\study.qpls",
        },
      },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      2,
      "plan_internal_project_upgrade_v6",
      {
        request: {
          surface: "internal_labs",
          experimentalLabsEnabled: true,
          sourceArchivePath: "D:\\study.qpls",
          destinationArchivePath: "D:\\study-v6.qpls",
          expectedSourceArchiveSha256: "a".repeat(64),
          legacyDisplayCovariances: {},
          estimandConfirmations: {},
        },
      },
    );
  });

  it("binds execute and cancel to the exact ephemeral plan identity", async () => {
    mocks.invoke.mockResolvedValue({ status: "ok", value: {} });

    await executeInternalProjectUpgradeV6("plan-1", "b".repeat(64));
    await cancelInternalProjectUpgradeV6("plan-2", "c".repeat(64));

    expect(mocks.invoke).toHaveBeenNthCalledWith(
      1,
      "execute_internal_project_upgrade_v6",
      {
        request: {
          surface: "internal_labs",
          experimentalLabsEnabled: true,
          planId: "plan-1",
          expectedPlanSha256: "b".repeat(64),
          confirmNewDestination: true,
        },
      },
    );
    expect(mocks.invoke).toHaveBeenNthCalledWith(
      2,
      "cancel_internal_project_upgrade_v6",
      {
        request: {
          surface: "internal_labs",
          experimentalLabsEnabled: true,
          planId: "plan-2",
          expectedPlanSha256: "c".repeat(64),
        },
      },
    );
  });
});

describe("native canonical project services", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
  });

  it("normalizes additive canonical collections from legacy desktop snapshots", async () => {
    mocks.invoke.mockResolvedValue({
      name: "Legacy",
      path: null,
      readOnly: false,
      recovered: false,
      datasets: [],
      workspace: null,
    });

    await expect(createNativeProject("Legacy")).resolves.toMatchObject({
      sourceArchiveVersion: 0,
      migrationPending: false,
      compatibilityNotices: [],
      futureUnsupported: { models: 0, recipes: 0, results: 0 },
      saveWarning: null,
      models: [],
      recipes: [],
      results: [],
      activeModelId: null,
      modelPresentations: {},
      savedReports: [],
      datasetVersions: [],
    });
    expect(mocks.invoke).toHaveBeenCalledWith("new_project", { name: "Legacy" });
  });

  it("requests a backend-owned fresh General SEM draft using the strict projectMode wire", async () => {
    mocks.invoke.mockResolvedValue({
      projectId: "60000002-0000-4000-8000-000000000123",
      name: "General SEM",
      path: null,
      readOnly: false,
      recovered: false,
      datasets: [],
      datasetVersions: [],
      workspace: null,
    });

    await createNativeProject("General SEM", "general_sem_v1");

    expect(mocks.invoke).toHaveBeenCalledWith("new_project", {
      name: "General SEM",
      projectMode: "general_sem_v1",
    });
  });

  it("exposes a one-way invalidation seam for strict schema-6 project activation", async () => {
    mocks.invoke.mockResolvedValue(undefined);

    await invalidateNativeGeneralSemFreshDraftAuthorityV1();

    expect(mocks.invoke).toHaveBeenCalledWith(
      "invalidate_general_sem_fresh_draft_authority_v1",
    );
  });

  it("separates one-shot draft revocation from an explicit native schema-6 release", async () => {
    mocks.invoke.mockResolvedValue(undefined);

    await clearNativeSchema6RevisionSourceV1();

    expect(mocks.invoke).toHaveBeenCalledWith(
      "clear_internal_project_archive_v6_native_revision_source_v1",
    );
  });

  it("binds native schema-6 revision adoption to the complete strict inspection identity", async () => {
    const request = {
      archivePath: "D:\\QuickPLS\\validation\\exact-general-sem.qpls",
      expectedArchiveSha256: "a".repeat(64),
      expectedArchiveBytes: 42_000,
      expectedProjectId: "60000002-0000-4000-8000-000000000123",
      expectedDatasetId: "60000002-0000-4000-8000-000000000124",
      expectedDatasetFingerprint: `v2:${"b".repeat(64)}`,
      expectedModelId: "model:general-sem:exact",
      expectedModelScientificSha256: "c".repeat(64),
      expectedRecipeId: "60000002-0000-4000-8000-000000000125",
      expectedRecipeDocumentSha256: "d".repeat(64),
    };
    const snapshot = {
      archivePath: request.archivePath,
      archiveSha256: request.expectedArchiveSha256,
      archiveBytes: request.expectedArchiveBytes,
      generalSemExecutionAuthority: {
        projectId: request.expectedProjectId,
        datasetId: request.expectedDatasetId,
        datasetFingerprint: request.expectedDatasetFingerprint,
        modelId: request.expectedModelId,
        modelScientificSha256: request.expectedModelScientificSha256,
        recipeId: request.expectedRecipeId,
        recipeDocumentSha256: request.expectedRecipeDocumentSha256,
      },
    };
    mocks.invoke.mockResolvedValue({
      schemaVersion: 1,
      ...request,
      archiveSha256: request.expectedArchiveSha256,
      archiveBytes: request.expectedArchiveBytes,
      projectId: request.expectedProjectId,
      datasetId: request.expectedDatasetId,
      datasetFingerprint: request.expectedDatasetFingerprint,
      modelId: request.expectedModelId,
      modelScientificSha256: request.expectedModelScientificSha256,
      recipeId: request.expectedRecipeId,
      recipeDocumentSha256: request.expectedRecipeDocumentSha256,
      readOnly: true,
      autosaveRecoveryUsed: false,
      sourceRecheckedUnchanged: true,
    });

    await adoptNativeSchema6RevisionSourceV1(snapshot as never);

    expect(mocks.invoke).toHaveBeenCalledWith(
      "adopt_internal_project_archive_v6_native_revision_source_v1",
      { request },
    );
  });

  it("imports an exact named-evidence CSV through the backend dataset authority", async () => {
    mocks.invoke.mockResolvedValue({
      id: "25500000-0000-4550-8550-000000000001",
      name: "named-sem-evidence.csv",
      columns: ["x1", "x2"],
      rows: null,
      rowCount: 360,
      missing: 0,
      fingerprint: `v2:${"a".repeat(64)}`,
      kind: "raw",
    });

    await expect(importNativeDatasetAtPathForValidation("D:\\QuickPLS\\validation\\fixtures\\v255\\named-sem-evidence.csv"))
      .resolves.toMatchObject({
        id: "25500000-0000-4550-8550-000000000001",
        rows: [],
        rowCount: 360,
      });
    expect(mocks.invoke).toHaveBeenCalledWith("import_dataset", {
      path: "D:\\QuickPLS\\validation\\fixtures\\v255\\named-sem-evidence.csv",
      dataKind: "raw",
      sampleSize: undefined,
      missingMarkers: [],
    });
  });

  it("rejects a non-absolute or non-CSV named-evidence source before native import", async () => {
    await expect(importNativeDatasetAtPathForValidation("validation/fixture.txt"))
      .rejects.toThrow(/absolute Windows CSV path/);
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it("rejects malformed lineage returned across the untrusted project snapshot boundary", async () => {
    mocks.invoke.mockResolvedValue({
      name: "Malformed",
      path: null,
      readOnly: false,
      recovered: false,
      datasets: [],
      datasetVersions: null,
      workspace: null,
    });

    await expect(createNativeProject("Malformed")).rejects.toMatchObject({
      code: "data_lineage.records_invalid",
    });
  });

  it("opens the exact selected bundled sample instead of silently falling back", async () => {
    mocks.invoke.mockResolvedValue({
      name: "Mediation Sample",
      path: null,
      readOnly: false,
      recovered: false,
      datasets: [],
      datasetVersions: [],
      models: [],
      recipes: [],
      results: [],
      activeModelId: null,
      workspace: null,
    });

    const mediationSampleId = parseNativeSampleProjectId("mediation");
    expect(mediationSampleId).not.toBeNull();
    await openNativeDemoProject(mediationSampleId!);

    expect(mocks.invoke).toHaveBeenCalledWith("open_demo_project", { sampleId: "mediation" });
  });

  it("sends the typed active model with both explicit saves and recovery saves", async () => {
    const model: NativeCanonicalModelSpec = {
      id: "model-1",
      name: "Model",
      constructs: [],
      paths: [],
      controls: [],
      higher_order_constructs: [],
      interactions: [],
    };
    const workspace = { nodes: [], edges: [], activeModelId: model.id };
    const modelPresentation: NativeModelPresentation = { nodes: [], edges: [] };
    mocks.invoke.mockResolvedValueOnce({
      name: "Study",
      path: "D:/projects/study.qpls",
      readOnly: false,
      recovered: false,
      datasets: [],
      datasetVersions: [],
      models: [model],
      recipes: [],
      results: [],
      activeModelId: model.id,
      workspace,
    }).mockResolvedValueOnce(undefined);

    await saveNativeProject("D:/projects/study.qpls", workspace, model, modelPresentation);
    await autosaveNativeProject("D:/projects/study.qpls", workspace, model, modelPresentation);

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "save_active_project", {
      path: "D:/projects/study.qpls",
      workspace,
      model,
      modelPresentation,
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "autosave_active_project", {
      path: "D:/projects/study.qpls",
      workspace,
      model,
      modelPresentation,
    });
  });

  it("sends one typed explorer mutation request and normalizes its snapshot", async () => {
    const request = {
      mutation: { kind: "rename_model" as const, modelId: "model-1", name: "Revised model" },
      currentModel: null,
      currentPresentation: null,
      path: "D:/projects/study.qpls",
    };
    mocks.invoke.mockResolvedValue({
      name: "Study",
      path: request.path,
      readOnly: false,
      recovered: false,
      datasets: [],
      datasetVersions: [],
      models: [],
      recipes: [],
      results: [],
      activeModelId: null,
      workspace: null,
    });

    await expect(mutateNativeProjectExplorer(request)).resolves.toMatchObject({
      modelPresentations: {},
      savedReports: [],
    });
    expect(mocks.invoke).toHaveBeenCalledWith("mutate_project_explorer", { request });
  });
});

describe("native dataset version services", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
  });

  it("activates a backend-owned version by dataset id and normalizes its row stub", async () => {
    mocks.invoke.mockResolvedValue({ id: "version-2", name: "Version 2", columns: ["score"], missing: 0 });

    await expect(activateNativeDataset("version-2")).resolves.toMatchObject({ id: "version-2", rows: [] });
    expect(mocks.invoke).toHaveBeenCalledWith("activate_dataset", { datasetId: "version-2" });
  });

  it("sends the exact camelCase recode payload and retains the authoritative version record", async () => {
    const sourceId = "00000000-0000-4000-8000-000000000001";
    const outputId = "00000000-0000-4000-8000-000000000002";
    const spec: RecodeColumnSpec = {
      sourceColumn: "segment",
      targetColumn: "segment_binary",
      targetLabel: "Segment binary",
      targetType: "numeric",
      targetScale: "binary",
      mappings: [{ source: "A", target: "1" }, { source: "B", target: null }],
      unmapped: "error",
    };
    const response = {
      dataset: { id: outputId, name: "Recode", columns: ["segment", "segment_binary"], missing: 1 },
      version: {
        datasetId: outputId,
        parentDatasetId: sourceId,
        operation: "recode",
        createdAt: "2026-08-10T12:00:00Z",
        summary: "Recoded segment into segment_binary",
        sourceColumn: "segment",
        targetColumn: "segment_binary",
      },
    };
    mocks.invoke.mockResolvedValue(response);

    await expect(recodeNativeDatasetColumn(sourceId, spec)).resolves.toEqual({
      ...response,
      dataset: { ...response.dataset, rows: [] },
    });
    expect(mocks.invoke).toHaveBeenCalledWith("recode_dataset_column", { datasetId: sourceId, spec });
  });

  it("previews and commits a typed immutable dataset transformation", async () => {
    const sourceId = "00000000-0000-4000-8000-000000000001";
    const outputId = "00000000-0000-4000-8000-000000000002";
    const sourceFingerprint = `v2:${"a".repeat(64)}`;
    const outputFingerprint = `v2:${"b".repeat(64)}`;
    const spec: DatasetTransformationSpecV2 = {
      kind: "reverse_scale",
      source_column: "score",
      target_column: "score_reversed",
      scale_min: 1,
      scale_max: 5,
    };
    const preview = {
      schema_version: 2,
      source_dataset_id: sourceId,
      target_column: "score_reversed",
      input_columns: ["score"],
      inspected_rows: 2,
      total_rows: 2,
      output_missing_count: 0,
      rows: [],
      issues: [],
    };
    mocks.invoke.mockResolvedValueOnce(preview);
    await expect(previewNativeDatasetTransformation(sourceId, spec)).resolves.toEqual(preview);
    expect(mocks.invoke).toHaveBeenLastCalledWith("preview_dataset_transformation", {
      datasetId: sourceId,
      spec,
    });

    const specSha256 = "c".repeat(64);
    const operationDigest = await sha256Text(`${sourceFingerprint}\u0000${specSha256}\u0000${outputId}`);
    const response = {
      dataset: { id: outputId, name: "Study - derived", columns: ["score", "score_reversed"], missing: 0 },
      version: {
        datasetId: outputId,
        parentDatasetId: sourceId,
        operation: "transform",
        createdAt: "2026-08-14T12:00:00Z",
        summary: "Derived score_reversed from score",
        sourceColumn: "score",
        targetColumn: "score_reversed",
        transformation: {
          schema_version: 2,
          engine: "qpls.dataset_transform.v2",
          operation_id: `dataset_transform:${operationDigest.slice(0, 24)}`,
          source_dataset_id: sourceId,
          source_dataset_fingerprint: sourceFingerprint,
          output_dataset_id: outputId,
          output_dataset_fingerprint: outputFingerprint,
          created_at: "2026-08-14T12:00:00Z",
          spec_sha256: specSha256,
          spec,
          input_columns: ["score"],
          output_columns: ["score_reversed"],
          source_row_count: 2,
          output_missing_count: 0,
        },
      },
    };
    mocks.invoke.mockResolvedValueOnce(response);
    await expect(applyNativeDatasetTransformation(sourceId, spec, "Study - derived")).resolves.toEqual({
      ...response,
      dataset: { ...response.dataset, rows: [] },
    });
    expect(mocks.invoke).toHaveBeenLastCalledWith("apply_dataset_transformation", {
      datasetId: sourceId,
      spec,
      outputDatasetName: "Study - derived",
    });
  });
});

describe("native result export service", () => {
  const request = {
    defaultPath: "quickpls-results.csv",
    filterName: "CSV tables",
    extension: "csv" as const,
    contents: "metric,value\npath_coefficient,0.42\n",
  };

  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
  });

  it("uses the native Save dialog before writing a text export", async () => {
    const path = "D:/exports/quickpls-results.csv";
    mocks.save.mockResolvedValue(path);
    mocks.invoke.mockResolvedValue(undefined);

    await expect(exportNativeTextFile(request)).resolves.toBe(path);
    expect(mocks.save).toHaveBeenCalledWith({
      defaultPath: request.defaultPath,
      filters: [{ name: request.filterName, extensions: [request.extension] }],
    });
    expect(mocks.invoke).toHaveBeenCalledWith("export_text_file", {
      path,
      contents: request.contents,
    });
  });

  it("treats cancellation as a neutral result without invoking a writer", async () => {
    mocks.save.mockResolvedValue(null);

    await expect(exportNativeTextFile(request)).resolves.toBeNull();
    await expect(exportNativeXlsxTables([])).resolves.toBeNull();
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it("propagates native write failures for the dialog to report", async () => {
    mocks.save.mockResolvedValue("D:/exports/quickpls-results.csv");
    mocks.invoke.mockRejectedValue(new Error("disk is full"));

    await expect(exportNativeTextFile(request)).rejects.toThrow("disk is full");
  });
});

describe("native diagnostic bundle service", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
  });

  it("previews through the native backend before opening any destination dialog", async () => {
    const preview = {
      previewId: "preview-1",
      createdAt: "2026-08-13T09:00:00.000Z",
      includedCategories: ["QuickPLS build and release identity"],
      excludedCategories: ["Dataset rows, values, and variable names"],
      redactionCounts: {
        windowsPaths: 0,
        emailAddresses: 0,
        urlQueriesOrFragments: 0,
        bearerTokens: 0,
      },
      entryCount: 3,
      eventCount: 2,
      estimatedUncompressedBytes: 1024,
      localOnly: true,
      networkActivity: "none" as const,
      stagedContents: {
        system: {
          schemaVersion: 1,
          quickplsVersion: "2.46.0",
          releaseChannel: "internal",
          sourceRevision: "not_provided",
          osFamily: "windows",
          architecture: "x86_64",
          desktopRuntime: "Tauri 2",
          locale: "not_collected",
          webview2Version: "not_collected",
          userDataIncluded: false,
          networkAccessed: false,
        },
        events: [{ timestamp: "2026-08-13T09:00:00.000Z", sequence: 1, severity: "info", code: "desktop.session.started" }],
        manifest: {
          schemaVersion: 1,
          policyVersion: "quickpls-diagnostics-v1",
          createdAt: "2026-08-13T09:00:00.000Z",
          quickplsVersion: "2.46.0",
          entries: [{ name: "metadata/system.json", sha256: "b".repeat(64), bytes: 512 }],
          redactionCounts: {
            windowsPaths: 0,
            emailAddresses: 0,
            urlQueriesOrFragments: 0,
            bearerTokens: 0,
          },
          redactionTotal: 0,
          archiveLimits: {
            maximumEntries: 3,
            maximumEntryBytes: 262144,
            maximumUncompressedBytes: 524288,
            maximumArchiveBytes: 532480,
            compression: "stored" as const,
          },
          localOnly: true,
          networkAccessed: false,
        },
      },
    };
    mocks.invoke.mockResolvedValue(preview);

    await expect(previewNativeDiagnosticBundle()).resolves.toEqual(preview);

    expect(mocks.invoke).toHaveBeenCalledWith("preview_diagnostic_bundle", { replacesPreviewId: null });
    expect(mocks.save).not.toHaveBeenCalled();
  });

  it("names the exact prior preview ID when requesting an atomic refresh", async () => {
    mocks.invoke.mockResolvedValue({ previewId: "preview-2" });

    await previewNativeDiagnosticBundle("preview-1");

    expect(mocks.invoke).toHaveBeenCalledWith("preview_diagnostic_bundle", {
      replacesPreviewId: "preview-1",
    });
  });

  it("saves an already-previewed bundle to a new ZIP selected by the user", async () => {
    mocks.save.mockResolvedValue("D:/support/quickpls-diagnostic-bundle.zip");
    mocks.invoke.mockResolvedValue({ bytes: 2048, archiveSha256: "a".repeat(64) });

    await expect(saveNativeDiagnosticBundle("preview-1")).resolves.toEqual({
      bytes: 2048,
      archiveSha256: "a".repeat(64),
    });

    expect(mocks.save).toHaveBeenCalledWith({
      defaultPath: "quickpls-diagnostic-bundle.zip",
      filters: [{ name: "QuickPLS diagnostic bundle", extensions: ["zip"] }],
    });
    expect(mocks.invoke).toHaveBeenCalledWith("save_diagnostic_bundle", {
      path: "D:/support/quickpls-diagnostic-bundle.zip",
      previewId: "preview-1",
    });
  });

  it("cancels the staged preview without invoking a writer when the Save dialog is dismissed", async () => {
    mocks.save.mockResolvedValue(null);
    mocks.invoke.mockResolvedValue(undefined);

    await expect(saveNativeDiagnosticBundle("preview-1")).resolves.toBeNull();

    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(mocks.invoke).toHaveBeenCalledWith("cancel_diagnostic_bundle_preview", {
      previewId: "preview-1",
    });
    expect(mocks.invoke).not.toHaveBeenCalledWith("save_diagnostic_bundle", expect.anything());
  });

  it("exposes an explicit preview cancellation command", async () => {
    mocks.invoke.mockResolvedValue(undefined);

    await expect(cancelNativeDiagnosticBundlePreview("preview-1")).resolves.toBeUndefined();

    expect(mocks.invoke).toHaveBeenCalledWith("cancel_diagnostic_bundle_preview", {
      previewId: "preview-1",
    });
  });
});
