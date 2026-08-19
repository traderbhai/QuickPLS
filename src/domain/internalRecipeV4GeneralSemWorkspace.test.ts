import { describe, expect, it, vi } from "vitest";
import type { CanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import { validateCanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import {
  bindGeneralSemPlsModelToDatasetV1,
  buildGeneralSemRecipeV1,
  defaultGeneralSemPlsEngineOptionsV1,
  GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1,
  generalSemConfigFromEngineV1,
  generalSemJobRequestFromReceiptV1,
  monitorGeneralSemPlsJobV1,
  parseGeneralSemPlsCompletedResultV1,
  parseGeneralSemPlsJobSnapshotV1,
  parseGeneralSemProjectBootstrapOutcomeV1,
  preflightGeneralSemWorkspaceV1,
  rehydrateGeneralSemExecutionAuthorityV1,
  type GeneralSemPlsCompletedResultV1,
  type GeneralSemPlsJobSnapshotV1,
  type GeneralSemProjectBootstrapReceiptV1,
} from "./internalRecipeV4GeneralSemWorkspace";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "./internalProjectArchiveV6Read";
import { convertLegacyBasicModelV4, type SemModelV4 } from "./semModelV4";
import { sha256HexBytesV1, sha256HexUtf8V1 } from "./sha256V1";
import type { Dataset } from "../types";

const PROJECT_ID = "00000000-0000-4000-8000-000000000001";
const RECIPE_ID = "00000000-0000-4000-8000-000000000002";
const DIGEST_A = "a".repeat(64);
const DIGEST_B = "b".repeat(64);
const DIGEST_C = "c".repeat(64);

function rawDataset(): Dataset {
  const columns = ["x1", "x2", "m11", "m12", "m21", "m22", "y1", "y2"];
  return {
    id: "dataset:general-sem",
    name: "General SEM observations",
    kind: "raw",
    columns,
    rows: Array.from({ length: 24 }, (_, index) => Object.fromEntries(
      columns.map((column, columnIndex) => [column, index + columnIndex / 10]),
    )),
    rowCount: 24,
    missing: 0,
    fingerprint: DIGEST_B,
    columnMetadata: columns.map((name) => ({
      name,
      label: null,
      column_type: "numeric",
      role: "unassigned",
      scale_type: "continuous",
      missing_markers: [],
      theoretical_min: null,
      theoretical_max: null,
      value_labels: {},
    })),
  };
}

function multipleMediationModel(): SemModelV4 {
  return convertLegacyBasicModelV4({
    id: "model:general-sem",
    name: "Parallel mediation",
    constructs: ["x", "m1", "m2", "y"].map((id) => ({
      id,
      name: id.toUpperCase(),
      short_name: id.toUpperCase(),
      mode: "reflective" as const,
      indicators: id === "m1" ? ["m11", "m12"] : id === "m2" ? ["m21", "m22"] : [`${id}1`, `${id}2`],
    })),
    paths: [
      { source: "x", target: "m1" },
      { source: "m1", target: "y" },
      { source: "x", target: "m2" },
      { source: "m2", target: "y" },
      { source: "x", target: "y" },
    ],
  }, "pls_composite");
}

function receipt(): GeneralSemProjectBootstrapReceiptV1 {
  return {
    schemaVersion: 1,
    archiveSchemaVersion: 6,
    projectId: PROJECT_ID,
    name: "General SEM calculation",
    createdAt: "2026-08-19T00:00:00Z",
    destinationArchivePath: "D:\\General-Sem.qpls",
    destinationArchiveSha256: DIGEST_A,
    destinationArchiveBytes: 4096,
    strictReopenValidated: true,
    residentDatasetId: "dataset:general-sem",
    residentDatasetFingerprint: DIGEST_B,
    residentModelId: "model:general-sem",
    residentModelScientificSha256: DIGEST_C,
    residentRecipeId: RECIPE_ID,
    residentRecipeDocumentSha256: "d".repeat(64),
  };
}

function snapshot(
  state: GeneralSemPlsJobSnapshotV1["state"],
  completedUnits: number,
): GeneralSemPlsJobSnapshotV1 {
  return {
    schemaVersion: 1,
    jobId: "job:general-sem",
    state,
    phase: state === "completed" ? "publication" : "estimation",
    completedUnits,
    totalUnits: 3,
    message: null,
    failure: state === "failed" ? {
      schemaVersion: 1,
      stage: "estimation",
      subject: "model:general-sem",
      code: "general_sem.estimation.failed",
      message: "The estimator did not converge.",
      correctiveAction: "Review the model and retry.",
      issues: [],
    } : null,
    queuedAt: "2026-08-19T00:00:00Z",
    startedAt: state === "queued" ? null : "2026-08-19T00:00:01Z",
    completedAt: ["completed", "failed", "cancelled"].includes(state)
      ? "2026-08-19T00:00:02Z"
      : null,
  };
}

function canonicalDocument(projectId = PROJECT_ID): CanonicalResultDocumentV2 {
  const capabilityCell = GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1;
  return {
    schema_version: 2,
    document_id: "result.general-sem:1",
    title: "General SEM result",
    provenance: {
      run_id: "run:general-sem:1",
      project_id: projectId,
      model_id: "model:general-sem",
      model_digest: DIGEST_C,
      dataset_id: "dataset:general-sem",
      dataset_fingerprint: DIGEST_B,
      recipe_id: RECIPE_ID,
      // The canonical provenance carries the analytical Recipe-v4 digest,
      // while archiveIdentity carries the full recipe-document digest.
      recipe_digest: "e".repeat(64),
      capability_cell: capabilityCell,
      method_version: "general_sem_pls_point_v1",
      engine_version: "test",
      seed: 42,
      workers: 1,
      started_at: "2026-08-19T00:00:00Z",
      completed_at: "2026-08-19T00:00:02Z",
    },
    capability_cells: [capabilityCell],
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
      missing_value_label: "—",
      chart_defaults: {},
    },
    general_sem_results: {
      schema_version: 1,
      identification_diagnostics: [{
        diagnostic_id: "identification:model:general-sem",
        trace: { model_id: "model:general-sem", capability_cell: capabilityCell },
        scope: "model",
        subject_id: "model:general-sem",
        status: "identified",
        code: "identified",
        message: "The recursive model passed the current identification checks.",
        degrees_of_freedom: 1,
      }],
    },
  };
}

function rustSpecificPathIdentityV1(relationIds: readonly string[]): string {
  const encoder = new TextEncoder();
  const domain = encoder.encode("qpls.compiled-sem-topology-v1.specific-directed-path\0");
  const encodedIds = relationIds.map((relationId) => encoder.encode(relationId));
  const totalLength = domain.length + encodedIds.reduce((total, bytes) => total + 8 + bytes.length, 0);
  const identityInput = new Uint8Array(totalLength);
  identityInput.set(domain);
  let offset = domain.length;
  for (const bytes of encodedIds) {
    const lengthView = new DataView(identityInput.buffer, offset, 8);
    const length = BigInt(bytes.length);
    lengthView.setUint32(0, Number(length >> 32n), false);
    lengthView.setUint32(4, Number(length & 0xffff_ffffn), false);
    offset += 8;
    identityInput.set(bytes, offset);
    offset += bytes.length;
  }
  return `sem_specific_path_v1_${sha256HexBytesV1(identityInput)}`;
}

function rustShapedBootstrapCanonicalDocument(): CanonicalResultDocumentV2 {
  const pointCell = GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1;
  const bootstrapCell = GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1;
  const baseCell = {
    registry_schema_version: 2,
    capability_id: "smartpls.pls_algorithm",
    cell_id: "qpls3.pls.algorithm",
    capability_version: "pls_pm_v1",
  } as const;
  const orderedRelationIds = ["relation.x.m1", "relation.m1.y"];
  const effectId = rustSpecificPathIdentityV1(orderedRelationIds);
  const effectIdentity = {
    kind: "specific_indirect" as const,
    effect_id: effectId,
    estimand_id: "estimand.x.y.m1",
    source_id: "x",
    target_id: "y",
    ordered_relation_ids: orderedRelationIds,
  };
  const usableReplicateIndices = [0, 1];

  return {
    ...canonicalDocument(),
    title: "PLS-SEM multiple mediation with full-model bootstrap",
    provenance: {
      ...canonicalDocument().provenance,
      capability_cell: pointCell,
      method_version: "general_sem_pls_full_model_case_bootstrap_v1",
      seed: 42,
      workers: 1,
    },
    // Rust sorts exact option-cell identities before serialization.
    capability_cells: [bootstrapCell, pointCell, baseCell],
    general_sem_results: {
      schema_version: 1,
      inference_receipt: {
        kind: "case_bootstrap",
        capability_cell: bootstrapCell,
        method_version: "general_sem_pls_full_model_case_bootstrap_v1",
        resampling_operation_version: "general_sem_pls_case_bootstrap_v1",
        resampling_stream_version: "indexed_case_resampling_v1",
        quantile_method_version: "type7_quantile_v1",
        standard_error_method_version: "sample_standard_error_b_minus_1_v1",
        summation_method_version: "neumaier_compensated_sum_v1",
        p_value_method_version: "null_centered_plus_one_v1",
        failure_policy_version: "minimum_usable_fraction_0_9_v1",
        compilation_artifact_identity_sha256: "1".repeat(64),
        compiled_plan_sha256: "2".repeat(64),
        general_sem_config_sha256: "3".repeat(64),
        recipe_analytical_sha256: "e".repeat(64),
        model_scientific_sha256: DIGEST_C,
        source_dataset_fingerprint: DIGEST_B,
        complete_case_frame_sha256: "4".repeat(64),
        usable_replicate_indices_sha256: sha256HexUtf8V1(JSON.stringify(usableReplicateIndices)),
        effect_identity_set_sha256: sha256HexUtf8V1(JSON.stringify([effectIdentity])),
        effect_ids: [effectId],
        interval: "percentile_type7",
        tail: "two_sided",
        confidence_level: 0.95,
        resamples_requested: 2,
        resamples_usable: 2,
        minimum_usable_resamples: 2,
        seed: "42",
        workers: 1,
        complete_model_reestimated_per_replicate: true,
        failed_replicates: [],
      },
      specific_indirect_effects: [{
        effect_id: effectId,
        estimand_id: effectIdentity.estimand_id,
        trace: { model_id: "model:general-sem", capability_cell: pointCell },
        source_id: effectIdentity.source_id,
        target_id: effectIdentity.target_id,
        ordered_relation_ids: orderedRelationIds,
        value: {
          estimate: 0.25,
          bootstrap_mean: 0.375,
          bootstrap_bias: 0.125,
          standard_error: 0.125,
          lower: 0.125,
          upper: 0.5,
          p_value: 1 / 3,
          bootstrap_usable_replicates: 2,
          bootstrap_two_sided_exceedances: 0,
        },
      }],
    },
  };
}

function completedResult(projectId = PROJECT_ID): GeneralSemPlsCompletedResultV1 {
  return {
    schemaVersion: 1,
    archiveIdentity: {
      archivePath: "D:\\General-Sem.qpls",
      archiveSha256: DIGEST_A,
      projectId: PROJECT_ID,
      datasetId: "dataset:general-sem",
      datasetFingerprint: DIGEST_B,
      modelId: "model:general-sem",
      modelScientificSha256: DIGEST_C,
      recipeId: RECIPE_ID,
      recipeDocumentSha256: "d".repeat(64),
    },
    analyticalResult: { schema_version: 1 },
    canonicalDocument: canonicalDocument(projectId),
  };
}

describe("General SEM Recipe-v4 workspace contract", () => {
  it("selects the exact point and multiple-mediation bootstrap cells from the frozen inference config", () => {
    const point = generalSemConfigFromEngineV1(defaultGeneralSemPlsEngineOptionsV1());
    const bootstrapEngine = {
      ...defaultGeneralSemPlsEngineOptionsV1(),
      inference: "percentile_case_bootstrap" as const,
      bootstrapSamples: 1_000,
      seed: 20260819,
      workers: 4,
      confidenceLevel: 0.9,
    };
    const bootstrap = generalSemConfigFromEngineV1(bootstrapEngine);

    expect(point.inference).toEqual({ kind: "none" });
    expect(generalSemJobRequestFromReceiptV1(receipt(), point).capabilityCell)
      .toStrictEqual(GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1);
    expect(bootstrap.inference).toStrictEqual({
      kind: "case_bootstrap",
      resamples: 1_000,
      seed: 20260819,
      confidence_level: 0.9,
      interval: "percentile",
      tail: "two_sided",
    });
    expect(generalSemJobRequestFromReceiptV1(receipt(), bootstrap).capabilityCell)
      .toStrictEqual(GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1);
    expect(generalSemJobRequestFromReceiptV1(receipt(), bootstrap, "f".repeat(64)).expectedArchiveSha256)
      .toBe("f".repeat(64));
  });

  it("binds one resident raw dataset and emits a project-model Recipe-v4 authority without case rows", () => {
    const dataset = rawDataset();
    const source = multipleMediationModel();
    const authoredMissingMarker = "-999";
    const model = bindGeneralSemPlsModelToDatasetV1({
      ...source,
      variables: source.variables.map((variable) => variable.kind === "observed" && variable.source_column === "x1"
        ? { ...variable, missing_markers: [authoredMissingMarker] }
        : variable),
    }, dataset);
    expect(model.variables.find((variable) => variable.kind === "observed" && variable.source_column === "x1"))
      .toMatchObject({ missing_markers: [authoredMissingMarker] });
    const engine = {
      ...defaultGeneralSemPlsEngineOptionsV1(),
      inference: "percentile_case_bootstrap" as const,
      bootstrapSamples: 500,
      workers: 2,
    };
    const config = generalSemConfigFromEngineV1(engine);
    const recipe = buildGeneralSemRecipeV1({
      recipeId: RECIPE_ID,
      createdAt: "2026-08-19T00:00:00Z",
      dataset,
      model,
      nativeScientificSha256: DIGEST_C,
      config,
      engine,
    });

    expect(model.data_binding).toStrictEqual({
      kind: "raw",
      dataset_id: dataset.id,
      missing_data: "listwise_deletion",
      weight: null,
      cluster_variable: null,
      strata_variable: null,
    });
    expect(recipe).toMatchObject({
      schema_version: 4,
      id: RECIPE_ID,
      dataset_fingerprint: DIGEST_B,
      model_binding: {
        kind: "project_sem_model_v4_reference",
        model_id: model.id,
        scientific_sha256: DIGEST_C,
      },
      settings: {
        method: "pls_pm",
        bootstrap_samples: 500,
        workers: 2,
        missing_data: "listwise_deletion",
      },
      method_config: { kind: "pls_algorithm" },
      general_sem_config: config,
      metadata: {
        execution_surface: "native_general_sem_pls_labs_v1",
        general_sem_generation: "general_sem_v1",
      },
    });
    expect(JSON.stringify(recipe)).not.toContain("\"rows\"");
  });

  it("rehydrates exact resident config and native recipe-document identity after restart", () => {
    const dataset = rawDataset();
    const model = bindGeneralSemPlsModelToDatasetV1(multipleMediationModel(), dataset);
    const engine = {
      ...defaultGeneralSemPlsEngineOptionsV1(),
      inference: "percentile_case_bootstrap" as const,
      bootstrapSamples: 777,
      seed: 20260819,
      workers: 3,
      confidenceLevel: 0.9,
      maxMaterializedSpecificPaths: 321,
    };
    const config = generalSemConfigFromEngineV1(engine, [{
      kind: "total_indirect",
      estimand_id: "effect:x:y",
      source_id: "x",
      target_id: "y",
    }]);
    const recipe = buildGeneralSemRecipeV1({
      recipeId: RECIPE_ID,
      createdAt: "2026-08-19T00:00:00Z",
      dataset,
      model,
      nativeScientificSha256: DIGEST_C,
      config,
      engine,
    });
    const nativeRecipeDocumentSha256 = "d".repeat(64);
    const restored = rehydrateGeneralSemExecutionAuthorityV1({
      archivePath: "D:\\General-Sem.qpls",
      archiveSha256: DIGEST_A,
      archiveBytes: 4096,
      project: {
        project_id: PROJECT_ID,
        name: "General SEM calculation",
        created_at: "2026-08-19T00:00:00Z",
      },
      generalSemExecutionAuthority: {
        schemaVersion: 1,
        projectId: PROJECT_ID,
        datasetId: dataset.id,
        datasetFingerprint: dataset.fingerprint!,
        modelId: model.id,
        modelScientificSha256: DIGEST_C,
        recipeId: RECIPE_ID,
        recipeDocumentSha256: nativeRecipeDocumentSha256,
        recipe,
      },
    } as InternalProjectArchiveV6ReadSnapshotV1);

    expect(restored.config).toStrictEqual(config);
    expect(restored.engine).toStrictEqual(engine);
    expect(restored.receipt.residentRecipeDocumentSha256).toBe(nativeRecipeDocumentSha256);
  });

  it("fails local preflight when resident observed-column descriptors are missing or noncontinuous", () => {
    const dataset = rawDataset();
    const model = bindGeneralSemPlsModelToDatasetV1(multipleMediationModel(), dataset);
    const engine = defaultGeneralSemPlsEngineOptionsV1();
    const config = generalSemConfigFromEngineV1(engine);
    const run = (candidateDataset: Dataset | null) => preflightGeneralSemWorkspaceV1({
      experimentalLabsEnabled: true,
      sourceProjectId: PROJECT_ID,
      dataset: candidateDataset,
      model,
      config,
      engine,
    });

    expect(run(dataset).ready).toBe(true);

    const cases: Array<[Dataset | null, string]> = [
      [null, "general_sem.dataset.required"],
      [{ ...dataset, columnMetadata: undefined }, "general_sem.dataset.continuous_numeric_required"],
      [{
        ...dataset,
        columns: dataset.columns.filter((column) => column !== "x1"),
      }, "general_sem.dataset.observed_column_missing"],
      [{
        ...dataset,
        columnMetadata: dataset.columnMetadata?.map((column) => column.name === "x1"
          ? { ...column, scale_type: "ordinal" as const }
          : column),
      }, "general_sem.dataset.continuous_numeric_required"],
    ];

    for (const [candidateDataset, expectedCode] of cases) {
      const decision = run(candidateDataset);
      expect(decision.ready).toBe(false);
      expect(decision.issues.map((item) => item.code)).toContain(expectedCode);
    }
  });

  it("fails closed on model digest and resident-dataset authority mismatches", () => {
    const dataset = rawDataset();
    const model = bindGeneralSemPlsModelToDatasetV1(multipleMediationModel(), dataset);
    const engine = defaultGeneralSemPlsEngineOptionsV1();
    const config = generalSemConfigFromEngineV1(engine);
    const common = {
      recipeId: RECIPE_ID,
      createdAt: "2026-08-19T00:00:00Z",
      dataset,
      model,
      config,
      engine,
    };

    expect(() => buildGeneralSemRecipeV1({ ...common, nativeScientificSha256: "ABC" }))
      .toThrowError(expect.objectContaining({ code: "general_sem.model.native_digest_invalid" }));
    expect(() => buildGeneralSemRecipeV1({
      ...common,
      dataset: { ...dataset, id: "dataset:tampered" },
      nativeScientificSha256: DIGEST_C,
    })).toThrowError(expect.objectContaining({ code: "general_sem.dataset.binding_mismatch" }));
  });

  it("parses strict schema-6 receipts and rejects digest or strict-reopen tampering", () => {
    const wire = { status: "ok", value: { schemaVersion: 1, receipt: receipt() } };
    expect(parseGeneralSemProjectBootstrapOutcomeV1(wire)).toStrictEqual(wire);

    const digestTamper = structuredClone(wire);
    digestTamper.value.receipt.destinationArchiveSha256 = DIGEST_A.toUpperCase();
    expect(() => parseGeneralSemProjectBootstrapOutcomeV1(digestTamper))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.digest_invalid" }));

    const reopenTamper = structuredClone(wire) as unknown as {
      status: string;
      value: { schemaVersion: number; receipt: Record<string, unknown> };
    };
    reopenTamper.value.receipt.strictReopenValidated = false;
    expect(() => parseGeneralSemProjectBootstrapOutcomeV1(reopenTamper))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.bootstrap_contract_invalid" }));
  });

  it("rejects malformed job snapshots and canonical/archive identity tampering", () => {
    expect(parseGeneralSemPlsJobSnapshotV1(snapshot("running", 1)))
      .toStrictEqual(snapshot("running", 1));
    expect(() => parseGeneralSemPlsJobSnapshotV1({ ...snapshot("running", 1), state: "publishing" }))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.job_snapshot_invalid" }));
    const invalidFailureStage = structuredClone(snapshot("failed", 1)) as unknown as {
      failure: Record<string, unknown>;
    };
    invalidFailureStage.failure.stage = "publishing";
    expect(() => parseGeneralSemPlsJobSnapshotV1(invalidFailureStage)).toThrow();
    expect(() => parseGeneralSemPlsJobSnapshotV1({
      ...snapshot("running", 1),
      completedUnits: 4,
      totalUnits: 3,
    })).toThrow();

    expect(parseGeneralSemPlsCompletedResultV1(completedResult()))
      .toStrictEqual(completedResult());
    expect(() => parseGeneralSemPlsCompletedResultV1(completedResult("00000000-0000-4000-8000-000000000099")))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.result_authority_mismatch" }));
    const datasetFingerprintTamper = structuredClone(completedResult());
    datasetFingerprintTamper.canonicalDocument.provenance.dataset_fingerprint = "f".repeat(64);
    expect(() => parseGeneralSemPlsCompletedResultV1(datasetFingerprintTamper))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.result_authority_mismatch" }));
    const modelDigestTamper = structuredClone(completedResult());
    modelDigestTamper.canonicalDocument.provenance.model_digest = "f".repeat(64);
    expect(() => parseGeneralSemPlsCompletedResultV1(modelDigestTamper))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.result_authority_mismatch" }));
    const archiveDigestTamper = structuredClone(completedResult()) as unknown as Record<string, unknown>;
    (archiveDigestTamper.archiveIdentity as Record<string, unknown>).archiveSha256 = "not-a-digest";
    expect(() => parseGeneralSemPlsCompletedResultV1(archiveDigestTamper))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.digest_invalid" }));
  });

  it("accepts the Rust-shaped exact-cell bootstrap result and rejects generic resampling ownership", () => {
    const canonical = rustShapedBootstrapCanonicalDocument();
    expect(validateCanonicalResultDocumentV2(canonical)).toEqual({ passed: true, errors: [] });
    expect(parseGeneralSemPlsCompletedResultV1({
      ...completedResult(),
      canonicalDocument: canonical,
    })).toMatchObject({ canonicalDocument: canonical });

    const genericCellTamper = structuredClone(canonical);
    genericCellTamper.general_sem_results!.inference_receipt!.capability_cell = {
      registry_schema_version: 2,
      capability_id: "smartpls.pls_bootstrapping",
      cell_id: "qpls3.inference.bootstrap",
      capability_version: "indexed_resampling_v4",
    };
    genericCellTamper.capability_cells = [
      GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1,
      GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1,
      {
        registry_schema_version: 2,
        capability_id: "smartpls.pls_algorithm",
        cell_id: "qpls3.pls.algorithm",
        capability_version: "pls_pm_v1",
      },
      genericCellTamper.general_sem_results!.inference_receipt!.capability_cell,
    ];
    const validation = validateCanonicalResultDocumentV2(genericCellTamper);
    expect(validation.passed).toBe(false);
    expect(validation.errors.join("\n")).toContain(
      "must equal the General SEM multiple-mediation full-model case-bootstrap option cell",
    );
    expect(() => parseGeneralSemPlsCompletedResultV1({
      ...completedResult(),
      canonicalDocument: genericCellTamper,
    })).toThrowError(expect.objectContaining({ code: "general_sem.wire.canonical_invalid" }));
  });

  it("stops immediately when monitoring is cancelled and never requests a result", async () => {
    const controller = new AbortController();
    controller.abort();
    const getStatus = vi.fn();
    const getResult = vi.fn();
    const wait = vi.fn();

    await expect(monitorGeneralSemPlsJobV1({
      initial: snapshot("queued", 0),
      getStatus,
      getResult,
      wait,
      signal: controller.signal,
    })).resolves.toEqual({ status: "aborted", snapshot: snapshot("queued", 0) });
    expect(wait).not.toHaveBeenCalled();
    expect(getStatus).not.toHaveBeenCalled();
    expect(getResult).not.toHaveBeenCalled();
  });

  it("returns failed or cancelled terminal snapshots without publishing a partial result", async () => {
    const failed = snapshot("failed", 1);
    const getResult = vi.fn();
    const onSnapshot = vi.fn();
    const outcome = await monitorGeneralSemPlsJobV1({
      initial: snapshot("running", 0),
      getStatus: vi.fn().mockResolvedValue(failed),
      getResult,
      onSnapshot,
      wait: async () => undefined,
    });

    expect(outcome).toEqual({ status: "terminal_without_result", snapshot: failed });
    expect(onSnapshot.mock.calls.map(([value]) => value.state)).toEqual(["running", "failed"]);
    expect(getResult).not.toHaveBeenCalled();
  });

  it("reads a completed result exactly once after the terminal completed snapshot", async () => {
    const running = snapshot("running", 1);
    const completedSnapshot = snapshot("completed", 3);
    const completed = completedResult();
    const statuses = [running, completedSnapshot];
    const getStatus = vi.fn().mockImplementation(async () => statuses.shift());
    const getResult = vi.fn().mockResolvedValue(completed);

    await expect(monitorGeneralSemPlsJobV1({
      initial: snapshot("queued", 0),
      getStatus,
      getResult,
      wait: async () => undefined,
    })).resolves.toEqual({ status: "completed", snapshot: completedSnapshot, completed });
    expect(getStatus).toHaveBeenCalledTimes(2);
    expect(getResult).toHaveBeenCalledOnce();
    expect(getResult).toHaveBeenCalledWith("job:general-sem");
  });

  it("propagates post-start status and one-shot result retrieval failures to the lifecycle owner", async () => {
    const statusFailure = new Error("status transport unavailable");
    const statusResult = vi.fn();
    await expect(monitorGeneralSemPlsJobV1({
      initial: snapshot("running", 0),
      getStatus: vi.fn().mockRejectedValue(statusFailure),
      getResult: statusResult,
      wait: async () => undefined,
    })).rejects.toBe(statusFailure);
    expect(statusResult).not.toHaveBeenCalled();

    const resultFailure = new Error("one-shot result parser rejected");
    const getResult = vi.fn().mockRejectedValue(resultFailure);
    await expect(monitorGeneralSemPlsJobV1({
      initial: snapshot("completed", 3),
      getStatus: vi.fn(),
      getResult,
    })).rejects.toBe(resultFailure);
    expect(getResult).toHaveBeenCalledOnce();
    expect(getResult).toHaveBeenCalledWith("job:general-sem");
  });
});
