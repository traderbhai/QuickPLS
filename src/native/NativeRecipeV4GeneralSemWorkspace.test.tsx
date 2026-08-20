import type { Edge, Node } from "@xyflow/react";
import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import {
  bindGeneralSemPlsModelToDatasetV1,
  buildGeneralSemRecipeV1,
  defaultGeneralSemPlsEngineOptionsV1,
  generalSemConfigFromEngineV1,
  generalSemPlsRequestedCapabilityCellV1,
  type GeneralSemProjectBootstrapReceiptV1,
} from "../domain/internalRecipeV4GeneralSemWorkspace";
import { preflightGeneralSemPlsV1 } from "../domain/generalSemCapabilityPreflightV1";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "../domain/internalProjectArchiveV6Read";
import { convertLegacyBasicModelV4, type SemModelV4 } from "../domain/semModelV4";
import type { InternalProjectArchiveV6ReadOnlySession } from "../internalProjectArchiveV6SessionStore";
import type { ConstructData, Dataset } from "../types";
import {
  generalSemCompletionMatchesLatestAuthorityV1,
  generalSemPersistenceNextActionV1,
  generalSemResultCanAppendV1,
  generalSemStartedJobRetentionV1,
  generalSemTemporaryResultBlocksCloseV1,
  activateGeneralSemProjectArchiveV1,
  closeGeneralSemProjectV1,
  generalSemCalculationActionLabelV1,
  generalSemCanonicalModerationInventoryV1,
  NativeRecipeV4GeneralSemWorkspace,
  selectCurrentGeneralSemNativePlsDecisionV1,
  selectGeneralSemDisplayedDocumentV1,
  selectLatestGeneralSemReopenedEntryV1,
  type NativeRecipeV4GeneralSemWorkspaceServices,
} from "./NativeRecipeV4GeneralSemWorkspace";
import {
  CanonicalResultDocumentV2View,
  canonicalResultDocumentV2ExportTables,
} from "./NativeRecipeV4CbsemWorkspace";

const workspaceHarness = vi.hoisted(() => ({
  current: {} as Record<string, unknown>,
}));
const schema6Harness = vi.hoisted(() => ({
  current: { session: null } as Record<string, unknown>,
}));

vi.mock("../store", () => ({
  useWorkspace: (selector: (state: Record<string, unknown>) => unknown) => selector(workspaceHarness.current),
}));
vi.mock("../internalProjectArchiveV6SessionStore", () => ({
  useInternalProjectArchiveV6Session: (selector: (state: Record<string, unknown>) => unknown) => selector(schema6Harness.current),
}));

afterEach(() => {
  workspaceHarness.current = {};
  schema6Harness.current = { session: null };
  vi.clearAllMocks();
});

function dataset(): Dataset {
  const columns = ["x1", "x2", "m11", "m12", "m21", "m22", "y1", "y2"];
  return {
    id: "dataset:general-sem-ui",
    name: "General SEM observations",
    kind: "raw",
    columns,
    rows: Array.from({ length: 24 }, (_, index) => Object.fromEntries(
      columns.map((column, columnIndex) => [column, index + columnIndex / 10]),
    )),
    rowCount: 24,
    missing: 0,
    fingerprint: "a".repeat(64),
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

function tableBackedModerationDocument(): CanonicalResultDocumentV2 {
  const cell = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.moderation",
    cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_point",
    capability_version: "general_sem_pls_multiple_two_way_moderation_point_v1",
  };
  return {
    schema_version: 2,
    document_id: "result:moderation:reopened",
    title: "Reopened moderation point result",
    provenance: {
      run_id: "run:moderation:reopened",
      project_id: "00000000-0000-4000-8000-000000000001",
      model_id: "model:general-sem-ui",
      model_digest: "e".repeat(64),
      dataset_id: "dataset:general-sem-ui",
      dataset_fingerprint: "a".repeat(64),
      recipe_id: "00000000-0000-4000-8000-000000000020",
      recipe_digest: "d".repeat(64),
      capability_cell: cell,
      method_version: "qpls.general-sem-pls.multiple-two-way.point.v1",
      engine_version: "test",
      seed: 42,
      workers: 1,
      started_at: "2026-08-19T00:00:00Z",
      completed_at: "2026-08-19T00:00:01Z",
    },
    capability_cells: [cell],
    sections: [{
      id: "general_sem_moderation",
      title: "Moderation effects",
      description: "Persisted native canonical plot points.",
      table_ids: ["general_sem_interaction_plots"],
      chart_ids: [],
      capability_cells: [cell],
    }],
    tables: [{
      id: "general_sem_interaction_plots",
      title: "Interaction plot points",
      description: "Every point from the typed canonical interaction plots.",
      columns: [
        { id: "interaction_id", label: "Interaction", description: "Interaction identity.", data_type: "text", role: "label" },
        { id: "moderator_value", label: "Moderator value", description: "Standardized moderator value.", data_type: "number" },
        { id: "focal_value", label: "Focal value", description: "Standardized focal value.", data_type: "number" },
        { id: "predicted_value", label: "Predicted outcome", description: "Canonical prediction.", data_type: "number" },
      ],
      rows: [{
        id: "interaction_plot_point_0000",
        cells: [
          { kind: "text", value: "interaction:x:m1" },
          { kind: "number", value: -1, display: "-1.0000" },
          { kind: "number", value: 0.5, display: "0.5000" },
          { kind: "number", value: 0.625, display: "0.6250" },
        ],
      }],
      footnote_ids: [],
      capability_cells: [cell],
    }],
    charts: [],
    notices: [],
    exclusions: [],
    footnotes: [],
    presentation: {
      default_section_id: "general_sem_moderation",
      default_table_id: "general_sem_interaction_plots",
      precision: 4,
      missing_value_label: "—",
      chart_defaults: {},
    },
    general_sem_results: {
      schema_version: 1,
      interaction_effects: [{ interaction_id: "interaction:x:m1" }],
      conditional_effects: [{ effect_id: "conditional:x:m1:0" }],
      interaction_plots: [{
        interaction_id: "interaction:x:m1",
        series: [{ points: [{ focal_value: 0.5, predicted_value: 0.625 }] }],
      }],
    },
  } as unknown as CanonicalResultDocumentV2;
}

function node(id: "x" | "m1" | "m2" | "y", indicators: string[]): Node<ConstructData> {
  return {
    id,
    type: "construct",
    position: { x: id === "x" ? 40 : id === "y" ? 640 : 320, y: id === "m1" ? 40 : id === "m2" ? 240 : 140 },
    data: {
      label: id.toUpperCase(),
      shortName: id.toUpperCase(),
      mode: "reflective",
      indicators,
      semModelV4: {
        version: 1,
        construct: { kind: "composite" },
      },
    },
  };
}

const nodes = [
  node("x", ["x1", "x2"]),
  node("m1", ["m11", "m12"]),
  node("m2", ["m21", "m22"]),
  node("y", ["y1", "y2"]),
];

const edges: Edge[] = [
  { id: "path:x:m1", source: "x", target: "m1" },
  { id: "path:m1:y", source: "m1", target: "y" },
  { id: "path:x:m2", source: "x", target: "m2" },
  { id: "path:m2:y", source: "m2", target: "y" },
  { id: "path:x:y", source: "x", target: "y" },
];

const services = {
  scientificDigest: vi.fn(),
  bootstrapArchive: vi.fn(),
  inspectArchive: vi.fn(),
  nativePreflight: vi.fn(),
  start: vi.fn(),
  status: vi.fn(),
  cancel: vi.fn(),
  dismiss: vi.fn(),
  result: vi.fn(),
  append: vi.fn(),
  read: vi.fn(),
  invalidateDraft: vi.fn(),
  exportXlsx: vi.fn(),
  selectDestination: vi.fn(),
} as unknown as NativeRecipeV4GeneralSemWorkspaceServices;

function workspaceState(resident: Dataset): Record<string, unknown> {
  return {
    projectId: "00000000-0000-4000-8000-000000000001",
    projectName: "Mediation study",
    projectPath: "D:\\Mediation-Study.qpls",
    activeModelId: "model:general-sem-ui",
    standardSemModelV4Authorities: {},
    nodes,
    edges,
    diagramLayout: {
      diagramVersion: "sem_designer_v1",
      constructLayouts: {},
      indicatorLayouts: {},
      edgeLayouts: {},
      diagramViewport: { x: 0, y: 0, zoom: 1 },
      diagramTheme: "quickpls_color",
      showGrid: true,
      layoutLocked: false,
    },
    dataset: resident,
    datasetCatalog: [resident],
    analysisSettings: {
      tolerance: 1e-7,
      maxIterations: 1_000,
      seed: 42,
      workers: 1,
      confidenceLevel: 0.95,
      bootstrapSamples: 500,
    },
  };
}

function setReadyWorkspace(): void {
  const resident = dataset();
  const strictModel = bindGeneralSemPlsModelToDatasetV1(convertLegacyBasicModelV4({
    id: "model:general-sem-ui",
    name: "Parallel mediation",
    constructs: [
      { id: "x", name: "X", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
      { id: "m1", name: "M1", short_name: "M1", mode: "reflective", indicators: ["m11", "m12"] },
      { id: "m2", name: "M2", short_name: "M2", mode: "reflective", indicators: ["m21", "m22"] },
      { id: "y", name: "Y", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
    ],
    paths: [
      { source: "x", target: "m1" },
      { source: "m1", target: "y" },
      { source: "x", target: "m2" },
      { source: "m2", target: "y" },
      { source: "x", target: "y" },
    ],
  }, "pls_composite"), resident);
  workspaceHarness.current = {
    ...workspaceState(resident),
    standardSemModelV4Authorities: {
      "model:general-sem-ui": {
        schema_version: 1,
        model_document_sha256: "c".repeat(64),
        model: strictModel,
      },
    },
  };
  setMarkedGeneralSemSession("model:general-sem-ui");
}

function setReadyDraftWorkspace(): void {
  const resident = dataset();
  workspaceHarness.current = {
    ...workspaceState(resident),
    generalSemProjectDraftMode: {
      schemaVersion: 1,
      semGeneration: "general_sem_v1",
      sourceProjectId: "00000000-0000-4000-8000-000000000001",
    },
  };
  schema6Harness.current = { session: null };
}

function addTwoWayInteraction(
  value: SemModelV4,
  id: string,
  focalPredictor: string,
  moderator: string,
): void {
  const focal = value.relations.find((relation) => relation.kind === "structural"
    && relation.source === focalPredictor
    && relation.target === "construct:y");
  if (!focal) throw new Error(`Missing focal relation for ${id}`);
  const output = `derived:${id}`;
  const relationId = `relation:${id}:effect`;
  const parameterId = `parameter:${id}:effect`;
  value.variables.push({ kind: "derived", id: output, label: id });
  value.relations.push({
    kind: "structural",
    id: relationId,
    source: output,
    target: "construct:y",
    parameter: parameterId,
    intercept_parameter: null,
  });
  value.parameters.push({
    kind: "free",
    id: parameterId,
    label: `${id} effect`,
    target: { kind: "regression", source: output, target: "construct:y" },
    group_overrides: [],
  });
  value.derived_terms.push({
    kind: "interaction_v2",
    id,
    output,
    operands: [focalPredictor, moderator],
    focal_relation: focal.id,
    method: "two_stage",
    hierarchy_policy: "strong",
  });
}

function setReadyModerationWorkspace(): SemModelV4 {
  const resident = dataset();
  const strictModel = bindGeneralSemPlsModelToDatasetV1(convertLegacyBasicModelV4({
    id: "model:general-sem-ui",
    name: "Same-path simultaneous moderation",
    constructs: [
      { id: "x", name: "X", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
      { id: "m1", name: "W", short_name: "W", mode: "reflective", indicators: ["m11", "m12"] },
      { id: "m2", name: "Z", short_name: "Z", mode: "reflective", indicators: ["m21", "m22"] },
      { id: "y", name: "Y", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
    ],
    paths: [
      { source: "x", target: "y" },
      { source: "m1", target: "y" },
      { source: "m2", target: "y" },
    ],
  }, "pls_composite"), resident);
  addTwoWayInteraction(strictModel, "interaction:x:m1", "construct:x", "construct:m1");
  addTwoWayInteraction(strictModel, "interaction:x:m2", "construct:x", "construct:m2");
  workspaceHarness.current = {
    ...workspaceState(resident),
    standardSemModelV4Authorities: {
      "model:general-sem-ui": {
        schema_version: 1,
        model_document_sha256: "c".repeat(64),
        model: strictModel,
      },
    },
  };
  setMarkedGeneralSemSession("model:general-sem-ui");
  return strictModel;
}

function setMarkedGeneralSemSession(modelId: string): void {
  const resident = workspaceHarness.current.dataset as Dataset;
  const authority = (workspaceHarness.current.standardSemModelV4Authorities as Record<string, { model: ReturnType<typeof convertLegacyBasicModelV4> }>)[modelId];
  const engine = defaultGeneralSemPlsEngineOptionsV1();
  const config = generalSemConfigFromEngineV1(engine);
  const recipeModel = bindGeneralSemPlsModelToDatasetV1(authority.model, resident);
  const recipe = buildGeneralSemRecipeV1({
    recipeId: "00000000-0000-4000-8000-000000000020",
    createdAt: "2026-08-19T00:00:00Z",
    dataset: resident,
    model: recipeModel,
    nativeScientificSha256: "e".repeat(64),
    config,
    engine,
    capabilityCell: generalSemPlsRequestedCapabilityCellV1(recipeModel, config),
    experimentalLabsEnabled: true,
  });
  const project = {
    schema_version: 6,
    project_id: "00000000-0000-4000-8000-000000000001",
    name: "Mediation study",
    created_at: "2026-08-19T00:00:00Z",
    modified_at: "2026-08-19T00:00:00Z",
    origin: { kind: "new_project" },
    sem_generation: "general_sem_v1",
    datasets: [{ id: resident.id, fingerprint: resident.fingerprint }],
    models: [{
      model_id: modelId,
      payload: {
        kind: "sem_model_v4",
        model: recipeModel,
        scientific_sha256: "e".repeat(64),
      },
    }],
    recipes: [recipe],
    canonical_result_documents: [],
  };
  const snapshot = {
    schemaVersion: 1,
    archivePath: "D:\\Mediation-Study.qpls",
    archiveSha256: "f".repeat(64),
    archiveBytes: 4096,
    project,
    generalSemExecutionAuthority: {
      schemaVersion: 1,
      projectId: project.project_id,
      datasetId: resident.id,
      datasetFingerprint: resident.fingerprint,
      modelId,
      modelScientificSha256: "e".repeat(64),
      recipeId: recipe.id,
      recipeDocumentSha256: "d".repeat(64),
      recipe,
    },
  } as unknown as InternalProjectArchiveV6ReadSnapshotV1;
  schema6Harness.current = {
    session: {
      snapshot,
      project,
      standardActivation: {
        modelIds: [modelId],
        sourceArchiveSha256: "f".repeat(64),
      },
    },
  };
}

describe("General SEM native workspace accessibility", () => {
  it("rejects an asynchronously completed result after the active authority changes", () => {
    expect(generalSemCompletionMatchesLatestAuthorityV1("authority:old", "authority:new")).toBe(false);
    expect(generalSemCompletionMatchesLatestAuthorityV1("authority:current", "authority:current")).toBe(true);
    expect(generalSemCompletionMatchesLatestAuthorityV1(null, "authority:current")).toBe(false);
  });

  it("drops an exact native moderation decision as soon as its authority key becomes stale", () => {
    const model = setReadyModerationWorkspace();
    const config = generalSemConfigFromEngineV1(defaultGeneralSemPlsEngineOptionsV1());
    const decision = preflightGeneralSemPlsV1(model, config);
    const preflight = { authorityKey: "authority:current", decision };

    expect(selectCurrentGeneralSemNativePlsDecisionV1(preflight, "authority:current"))
      .toBe(decision);
    expect(selectCurrentGeneralSemNativePlsDecisionV1(preflight, "authority:changed"))
      .toBeNull();
    expect(selectCurrentGeneralSemNativePlsDecisionV1(null, "authority:current"))
      .toBeNull();
  });

  it("counts only persisted canonical moderation effects, slopes, plots, and plot points", () => {
    const canonical = {
      general_sem_results: {
        schema_version: 1,
        interaction_effects: [{ interaction_id: "interaction:x:m1" }, { interaction_id: "interaction:x:m2" }],
        conditional_effects: Array.from({ length: 6 }, (_, index) => ({ effect_id: `slope:${index}` })),
        interaction_plots: [
          { series: [{ points: [{}, {}, {}] }, { points: [{}, {}, {}] }, { points: [{}, {}, {}] }] },
          { series: [{ points: [{}, {}] }, { points: [{}, {}] }, { points: [{}, {}] }] },
        ],
      },
    } as never;

    expect(generalSemCanonicalModerationInventoryV1(canonical)).toEqual({
      interactionEffectCount: 2,
      gammaInferenceCount: 0,
      conditionalSlopeCount: 6,
      interactionPlotCount: 2,
      interactionPlotPointCount: 15,
      bootstrapResamplesRequested: null,
      bootstrapResamplesUsable: null,
      conditionalIndirectCount: 0,
      moderatedMediationIndexCount: 0,
      combinedModeratedMediation: false,
    });
    const inferred = structuredClone(canonical) as unknown as CanonicalResultDocumentV2;
    inferred.general_sem_results!.interaction_effects![0]!.scientific_rescaled_gamma = {
      estimate: 0.4,
      standard_error: 0.1,
    };
    inferred.general_sem_results!.interaction_effects![1]!.scientific_rescaled_gamma = {
      estimate: -0.2,
      standard_error: 0.08,
    };
    inferred.general_sem_results!.inference_receipt = {
      capability_cell: {
        registry_schema_version: 2,
        capability_id: "smartpls.moderation",
        cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
        capability_version: "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
      },
      resamples_requested: 500,
      resamples_usable: 492,
    } as never;
    expect(generalSemCanonicalModerationInventoryV1(inferred)).toMatchObject({
      gammaInferenceCount: 2,
      bootstrapResamplesRequested: 500,
      bootstrapResamplesUsable: 492,
      combinedModeratedMediation: false,
    });
    expect(generalSemCanonicalModerationInventoryV1(null)).toBeNull();
    expect(selectGeneralSemDisplayedDocumentV1(canonical, null, false, true)).toBe(canonical);
    expect(selectGeneralSemDisplayedDocumentV1(canonical, null, false, false)).toBeNull();
  });

  it("renders and exports reopened table-backed interaction plot points without recomputation", () => {
    const document = tableBackedModerationDocument();
    const reopened = selectGeneralSemDisplayedDocumentV1(document, null, false, true);
    if (!reopened) throw new Error("Expected reopened moderation document");

    const html = renderToStaticMarkup(<CanonicalResultDocumentV2View
      document={reopened}
      reopened
      compilationReceipt={null}
    />);
    expect(html).toContain('data-canonical-table-id="general_sem_interaction_plots"');
    expect(html).toContain("Interaction plot points");
    expect(html).toContain("0.6250");

    const exported = canonicalResultDocumentV2ExportTables(reopened)
      .find((table) => table.id === "general_sem_interaction_plots");
    expect(exported?.rows).toStrictEqual([["interaction:x:m1", "-1.0000", "0.5000", "0.6250"]]);
  });

  it("retains a started native job and global blocker until terminal recovery is known", () => {
    expect(generalSemStartedJobRetentionV1({
      started: true,
      terminalKnown: false,
      activeJobId: "job:general-sem",
    })).toBe("retain");
    expect(generalSemStartedJobRetentionV1({
      started: true,
      terminalKnown: true,
      activeJobId: "job:general-sem",
    })).toBe("release");
    expect(generalSemStartedJobRetentionV1({
      started: false,
      terminalKnown: false,
      activeJobId: null,
    })).toBe("release");

    const source = readFileSync("src/native/NativeRecipeV4GeneralSemWorkspace.tsx", "utf8");
    const start = source.slice(source.indexOf("const start = async"), source.indexOf("const cancel = async"));
    expect(source).toContain("validateGeneralSemPlsCompletedExecutionV1(outcome.completed, nativePlsExecution)");
    expect(start).toContain("retainRecoverableJobFailure(error)");
    expect(start).toContain('=== "release") activeJobIdRef.current = null');
    expect(source).toContain("Retry job recovery");
    expect(source).toContain("Abandon unrecovered job");
  });

  it("suppresses every result fallback after persistence integrity fails", () => {
    expect(selectGeneralSemDisplayedDocumentV1("reopened", "completed", false)).toBe("reopened");
    expect(selectGeneralSemDisplayedDocumentV1(null, "completed", false)).toBe("completed");
    expect(selectGeneralSemDisplayedDocumentV1("reopened", "completed", true)).toBeNull();
    expect(selectGeneralSemDisplayedDocumentV1(null, "completed", true)).toBeNull();
    expect(selectGeneralSemDisplayedDocumentV1("reopened", "completed", false, false)).toBeNull();
    expect(generalSemPersistenceNextActionV1(false, false)).toBe("append");
    expect(generalSemPersistenceNextActionV1(true, true)).toBe("verify_reanchor");
    expect(generalSemPersistenceNextActionV1(true, false)).toBe("strict_readback");
    expect(generalSemResultCanAppendV1({
      completed: true,
      authorityCurrent: true,
      sessionDirty: false,
      operationPending: false,
      appendSucceeded: false,
      resultIntegrityInvalid: false,
    })).toBe(true);
    expect(generalSemResultCanAppendV1({
      completed: true,
      authorityCurrent: false,
      sessionDirty: false,
      operationPending: false,
      appendSucceeded: false,
      resultIntegrityInvalid: false,
    })).toBe(false);
    expect(generalSemResultCanAppendV1({
      completed: true,
      authorityCurrent: true,
      sessionDirty: true,
      operationPending: false,
      appendSucceeded: false,
      resultIntegrityInvalid: false,
    })).toBe(false);
    expect(generalSemTemporaryResultBlocksCloseV1({
      completed: true,
      appendSucceeded: true,
      reopened: false,
      resultIntegrityInvalid: false,
    })).toBe(true);
    expect(generalSemTemporaryResultBlocksCloseV1({
      completed: true,
      appendSucceeded: true,
      reopened: true,
      resultIntegrityInvalid: false,
    })).toBe(false);
  });

  it("chooses the latest matching strict readback by instant and releases a clean project", () => {
    const authorityReceipt = {
      schemaVersion: 1,
      archiveSchemaVersion: 6,
      projectId: "00000000-0000-4000-8000-000000000001",
      name: "Mediation study",
      createdAt: "2026-08-19T00:00:00Z",
      destinationArchivePath: "D:\\Mediation-Study.qpls",
      destinationArchiveSha256: "f".repeat(64),
      destinationArchiveBytes: 4096,
      strictReopenValidated: true,
      residentDatasetId: "dataset:general-sem-ui",
      residentDatasetFingerprint: "a".repeat(64),
      residentModelId: "model:general-sem-ui",
      residentModelScientificSha256: "e".repeat(64),
      residentRecipeId: "00000000-0000-4000-8000-000000000020",
      residentRecipeDocumentSha256: "d".repeat(64),
    } satisfies GeneralSemProjectBootstrapReceiptV1;
    const entry = (documentId: string, completedAt: string) => ({
      documentId,
      canonicalDocument: {
        general_sem_results: { schema_version: 1 },
        provenance: {
          project_id: authorityReceipt.projectId,
          dataset_id: authorityReceipt.residentDatasetId,
          dataset_fingerprint: authorityReceipt.residentDatasetFingerprint,
          model_id: authorityReceipt.residentModelId,
          model_digest: authorityReceipt.residentModelScientificSha256,
          recipe_id: authorityReceipt.residentRecipeId,
          completed_at: completedAt,
        },
      },
    }) as never;
    expect(selectLatestGeneralSemReopenedEntryV1([
      entry("result:later", "2026-08-19T07:00:00+05:30"),
      entry("result:earlier", "2026-08-19T01:00:00Z"),
    ], authorityReceipt)?.documentId).toBe("result:later");

    const close = vi.fn(() => "closed" as const);
    expect(closeGeneralSemProjectV1({ close, readFailure: () => null })).toEqual({ status: "closed" });
    expect(close).toHaveBeenCalledOnce();
  });

  it("renders a ready fresh-project Labs flow without opening legacy adaptation", () => {
    setReadyDraftWorkspace();
    const html = renderToStaticMarkup(<NativeRecipeV4GeneralSemWorkspace
      modelName="Parallel mediation"
      experimentalLabsEnabled
      projectActivationConnected
      services={services}
    />);

    expect(html).toContain('id="nd-model-general-sem-labs-panel"');
    expect(html).toContain('role="tabpanel"');
    expect(html).toContain('aria-labelledby="nd-model-general-sem-labs-tab"');
    expect(html).toContain("General SEM in QuickPLS");
    expect(html).toContain("One QuickPLS canvas · explicit General SEM project authority · Registry-authorized PLS-SEM estimation");
    expect(html).not.toContain("PLS-first Experimental Labs");
    expect(html).toContain('role="note"');
    expect(html).toContain("This is a fresh General SEM draft inside QuickPLS.");
    expect(html).toContain("Only this newly created canvas may be adapted; ordinary projects are never converted.");
    expect(html).toContain("Fresh General SEM draft");
    expect(html).toContain('id="nd-general-sem-bootstrap"');
    expect(html).toContain('for="nd-general-sem-bootstrap"');
    expect(html).toContain('id="nd-general-sem-seed"');
    expect(html).toContain('for="nd-general-sem-seed"');
    expect(html).toContain('id="nd-general-sem-workers"');
    expect(html).toContain('for="nd-general-sem-workers"');
    expect(html).toContain('id="nd-general-sem-preflight"');
    expect(html).toContain('tabindex="-1"');
    expect(html).toContain("Ready for QuickPLS engine verification");
    expect(html).toContain("Safe QuickPLS project file");
    expect(html).toContain("Pending final engine verification");
    expect(html).toContain('role="status"');
    expect(html).toContain('aria-live="polite"');
    expect(html).toMatch(/<button(?=[^>]*class="primary")(?![^>]*disabled="")[^>]*>[^<]*(?:<svg[\s\S]*?<\/svg>)?Save and activate project…<\/button>/);
    expect(html).toMatch(/<button(?=[^>]*class="primary")(?=[^>]*disabled="")[^>]*>[\s\S]*?Calculate PLS effects<\/button>/);
    expect(html).toMatch(/<button(?=[^>]*class="danger")(?=[^>]*disabled="")[^>]*>[\s\S]*?Cancel<\/button>/);
    expect(html).not.toContain('role="alert"');
    expect(Object.values(services).every((service) => !vi.isMockFunction(service) || service.mock.calls.length === 0)).toBe(true);
  });

  it("renders simultaneous moderation with an accessible optional gamma-bootstrap contract", () => {
    setReadyModerationWorkspace();
    const html = renderToStaticMarkup(<NativeRecipeV4GeneralSemWorkspace
      modelName="Same-path simultaneous moderation"
      experimentalLabsEnabled
      projectActivationConnected
      services={services}
    />);

    expect(html).toContain("Optional full-model case bootstrap is available for scientific rescaled interaction gamma.");
    expect(html).toContain("fixed -1/0/+1 slopes, and interaction plots remain point-only.");
    expect(html).toContain('id="nd-general-sem-moderation-inference-note"');
    expect(html).toMatch(/id="nd-general-sem-bootstrap"(?=[^>]*disabled="")(?=[^>]*aria-describedby="nd-general-sem-moderation-inference-note")/);
    expect(html).toContain("Calculate moderation point estimates");
    expect(generalSemCalculationActionLabelV1(true, true)).toBe("Calculate moderation bootstrap");
    expect(generalSemCalculationActionLabelV1(true, false)).toBe("Calculate moderation point estimates");
    expect(generalSemCalculationActionLabelV1(false, true)).toBe("Calculate PLS effects");
    expect(generalSemCalculationActionLabelV1(false, false, true)).toBe("Calculate HOC point estimates");
    expect(generalSemCalculationActionLabelV1(false, true, true)).toBe("Calculate HOC bootstrap");
    expect(html).not.toContain("sem.capability.pls.derived_shape_not_executable");
    expect(html).toContain("Ready for QuickPLS engine verification");

    const source = readFileSync("src/native/NativeRecipeV4GeneralSemWorkspace.tsx", "utf8");
    const inputGuard = source.slice(
      source.indexOf("const moderationBootstrapInputDisabled"),
      source.indexOf("return <section", source.indexOf("const moderationBootstrapInputDisabled")),
    );
    expect(inputGuard).not.toContain("interactionPlan");
    expect(source).not.toContain("moderationBootstrapTurnOffRequired");
    expect(source).not.toContain("if (interactionPlan && event.target.checked) return;");
  });

  it("fails closed without adapting an ordinary project even when its legacy canvas is complete", () => {
    const resident = dataset();
    workspaceHarness.current = workspaceState(resident);
    const html = renderToStaticMarkup(<NativeRecipeV4GeneralSemWorkspace
      modelName="Parallel mediation"
      experimentalLabsEnabled
      projectActivationConnected
      services={services}
    />);

    expect(html).toContain("1 issue");
    expect(html).toContain("This ordinary QuickPLS project is intentionally isolated from General SEM project mode.");
    expect(html).toContain("QuickPLS will not copy, adapt, or relabel its canvas as general_sem_v1.");
    expect(html).toContain("The open canvas is neither a fresh General SEM draft nor an activated general_sem_v1 authority.");
    expect(html).toContain("general_sem.project_mode.required");
    expect(html).toMatch(/<button(?=[^>]*class="primary")(?=[^>]*disabled="")(?=[^>]*title="Start a new General SEM project to create its marked authority; existing projects cannot enter this path\.")[^>]*>[\s\S]*?Save and activate project…<\/button>/);
    expect(html).toContain("General SEM project and model authority");
    expect(html).toContain("Safe QuickPLS project file");
    expect(html).toContain("QuickPLS engine preflight");
    expect(html).not.toContain('role="alert"');
    expect(Object.values(services).every((service) => !vi.isMockFunction(service) || service.mock.calls.length === 0)).toBe(true);
  });

  it("keeps an activated marked authority blocked when the build does not opt into the activation bridge", () => {
    setReadyWorkspace();
    const html = renderToStaticMarkup(<NativeRecipeV4GeneralSemWorkspace
      modelName="Parallel mediation"
      experimentalLabsEnabled
      services={services}
    />);

    expect(html).toContain("This canvas is bound to the activated, newly created general_sem_v1 authority.");
    expect(html).toContain("Primary General SEM project activation is not connected in this build.");
    expect(html).toContain("general_sem.project_mode.primary_activation_pending");
    expect(html).toMatch(/<button(?=[^>]*class="primary")(?=[^>]*disabled="")[^>]*>[\s\S]*?Save and activate project…<\/button>/);
    expect(Object.values(services).every((service) => !vi.isMockFunction(service) || service.mock.calls.length === 0)).toBe(true);
  });

  it("prefers the active strict SemModelV4 authority over a simpler canvas reconstruction", () => {
    const resident = dataset();
    const strictModel = convertLegacyBasicModelV4({
      id: "model:general-sem-ui",
      name: "Strict authority",
      constructs: [
        { id: "x", name: "X", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
        { id: "m1", name: "M1", short_name: "M1", mode: "reflective", indicators: ["m11", "m12"] },
        { id: "m2", name: "M2", short_name: "M2", mode: "reflective", indicators: ["m21", "m22"] },
        { id: "y", name: "Y", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
      ],
      paths: [
        { source: "x", target: "m1" },
        { source: "m1", target: "y" },
        { source: "x", target: "m2" },
        { source: "m2", target: "y" },
      ],
    }, "cbsem_common_factor");
    workspaceHarness.current = {
      ...workspaceState(resident),
      standardSemModelV4Authorities: {
        "model:general-sem-ui": {
          schema_version: 1,
          model_document_sha256: "d".repeat(64),
          model: strictModel,
        },
      },
    };
    setMarkedGeneralSemSession("model:general-sem-ui");

    const html = renderToStaticMarkup(<NativeRecipeV4GeneralSemWorkspace
      modelName="Parallel mediation"
      experimentalLabsEnabled
      projectActivationConnected
      services={services}
    />);

    expect(html).toContain("The current General SEM PLS executor requires structural constructs to be composites; the authored model contains common factors.");
    expect(html).toContain("sem.capability.pls.common_factor_not_executable");
    expect(html).toMatch(/<button(?=[^>]*class="primary")(?=[^>]*disabled="")[^>]*>[\s\S]*?Save and activate project…<\/button>/);
  });

  it("opens, verifies, and promotes the exact populated archive as the active same-app authority", async () => {
    const receipt = {
      schemaVersion: 1,
      archiveSchemaVersion: 6,
      projectId: "70000001-0000-4000-8000-000000000001",
      name: "Activated General SEM",
      createdAt: "2026-08-19T00:00:00Z",
      destinationArchivePath: "D:\\Activated-General-SEM.qpls",
      destinationArchiveSha256: "a".repeat(64),
      destinationArchiveBytes: 4096,
      strictReopenValidated: true,
      residentDatasetId: "00000000-0000-4000-8000-000000000010",
      residentDatasetFingerprint: "b".repeat(64),
      residentModelId: "model:general-sem-ui",
      residentModelScientificSha256: "c".repeat(64),
      residentRecipeId: "00000000-0000-4000-8000-000000000020",
      residentRecipeDocumentSha256: "d".repeat(64),
    } satisfies GeneralSemProjectBootstrapReceiptV1;
    const snapshot = {
      archiveSha256: receipt.destinationArchiveSha256,
      project: {
        project_id: receipt.projectId,
        origin: { kind: "new_project" },
        sem_generation: "general_sem_v1",
      },
    } as unknown as InternalProjectArchiveV6ReadSnapshotV1;
    const setProjectMeta = vi.fn();
    const clearGeneralSemProjectDraftMode = vi.fn();

    await activateGeneralSemProjectArchiveV1(snapshot, receipt, {
      openSnapshot: vi.fn(async () => "activated"),
      activateStandardAuthorities: vi.fn(async () => "activated"),
      rollbackActivation: vi.fn(),
      readSession: () => ({
        snapshot,
        project: snapshot.project,
        standardActivation: {
          modelIds: [receipt.residentModelId],
          sourceArchiveSha256: receipt.destinationArchiveSha256,
        },
      } as InternalProjectArchiveV6ReadOnlySession),
      readWorkspace: () => ({
        activeModelId: receipt.residentModelId,
        standardSemModelV4Authorities: { [receipt.residentModelId]: {} as never },
        standardSemModelV4Persistence: {
          [receipt.residentModelId]: { scientificSha256: receipt.residentModelScientificSha256 } as never,
        },
        setProjectMeta,
        clearGeneralSemProjectDraftMode,
      }),
    });

    expect(setProjectMeta).toHaveBeenCalledWith(receipt.name, receipt.destinationArchivePath, receipt.projectId);
    expect(clearGeneralSemProjectDraftMode).toHaveBeenCalledOnce();
  });

  it("rolls back a partially opened schema-6 session when activation fails", async () => {
    const rollbackActivation = vi.fn();
    const receipt = {
      schemaVersion: 1,
      archiveSchemaVersion: 6,
      projectId: "70000001-0000-4000-8000-000000000001",
      name: "Failed activation",
      createdAt: "2026-08-19T00:00:00Z",
      destinationArchivePath: "D:\\Failed-Activation.qpls",
      destinationArchiveSha256: "a".repeat(64),
      destinationArchiveBytes: 4096,
      strictReopenValidated: true,
      residentDatasetId: "00000000-0000-4000-8000-000000000010",
      residentDatasetFingerprint: "b".repeat(64),
      residentModelId: "model:general-sem-ui",
      residentModelScientificSha256: "c".repeat(64),
      residentRecipeId: "00000000-0000-4000-8000-000000000020",
      residentRecipeDocumentSha256: "d".repeat(64),
    } satisfies GeneralSemProjectBootstrapReceiptV1;
    const snapshot = {
      archiveSha256: receipt.destinationArchiveSha256,
      project: { project_id: receipt.projectId, origin: { kind: "new_project" }, sem_generation: "general_sem_v1" },
    } as unknown as InternalProjectArchiveV6ReadSnapshotV1;

    await expect(activateGeneralSemProjectArchiveV1(snapshot, receipt, {
      openSnapshot: vi.fn(async () => "activated"),
      activateStandardAuthorities: vi.fn(async () => "blocked"),
      rollbackActivation,
      readSession: () => null,
      readWorkspace: () => ({
        activeModelId: null,
        standardSemModelV4Authorities: {},
        standardSemModelV4Persistence: {},
        setProjectMeta: vi.fn(),
        clearGeneralSemProjectDraftMode: vi.fn(),
      }),
    })).rejects.toThrow("could not become the active QuickPLS canvas authority");
    expect(rollbackActivation).toHaveBeenCalledOnce();
  });
});
