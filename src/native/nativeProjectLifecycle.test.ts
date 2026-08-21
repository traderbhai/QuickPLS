import { describe, expect, it } from "vitest";
import type { NativeProjectSignatureInput } from "./nativeProjectLifecycle";
import {
  hasUnsavedNativeProjectChanges,
  nativeSchema6BoundWorkspaceReplacementBlocker,
  nativeLegacyProjectOperationBlocker,
  nativeProjectSignature,
  nativeSavedProjectSignature,
} from "./nativeProjectLifecycle";

const importVersion = {
  datasetId: "dataset-1",
  parentDatasetId: null,
  operation: "import" as const,
  createdAt: "2026-08-10T00:00:00Z",
  summary: "Imported study.csv",
  sourceColumn: null,
  targetColumn: null,
};

const base: NativeProjectSignatureInput = {
  dataset: {
    id: "dataset-1",
    name: "study.csv",
    columns: ["x1", "y1"],
    rows: [{ x1: 1, y1: 2 }],
    rowCount: 1,
    missing: 0,
    fingerprint: "sha256:study",
    kind: "raw",
  },
  datasetCatalog: [{
    id: "dataset-1",
    name: "study.csv",
    columns: ["x1", "y1"],
    rows: [{ x1: 1, y1: 2 }],
    rowCount: 1,
    missing: 0,
    fingerprint: "sha256:study",
    kind: "raw",
  }],
  datasetVersions: [importVersion],
  projectModels: [{
    id: "model-1",
    name: "Structural model",
    constructs: [{ id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1"] }],
    paths: [],
    controls: [],
    higher_order_constructs: [],
    interactions: [],
  }],
  activeModelId: "model-1",
  modelPresentations: {},
  savedReports: [],
  nodes: [{
    id: "x",
    type: "construct",
    position: { x: 10, y: 20 },
    data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1"] },
  }],
  edges: [],
  runs: [],
  analysisSettings: {
    method: "pls_pm",
    bootstrapSamples: 0,
    studentizedInnerSamples: 0,
    permutationSamples: 0,
    seed: 20260810,
    workers: 1,
    confidenceLevel: 0.95,
  },
  diagramMode: "sem",
  diagramOverlaySettings: {
    selectedRunId: null,
    mode: "model",
    precision: 3,
    showLoadings: true,
    showPathCoefficients: true,
    showPValues: false,
    showTValues: false,
    showRSquared: true,
    showWarnings: true,
    showWatermark: true,
  },
  publicationDiagramSettings: {
    mode: "smartpls_result",
    precision: 3,
    overlayMode: "paths_r2",
    aspectRatio: "wide",
    palette: "grayscale",
    layoutSource: "current_canvas",
    showLoadings: true,
    showPathCoefficients: true,
    showRSquared: true,
    showValidationWatermark: true,
    showUnsupportedWarning: true,
    showRunProvenance: true,
  },
  diagramLayout: {
    diagramVersion: "sem_designer_v1",
    constructLayouts: {},
    indicatorLayouts: {},
    edgeLayouts: {},
    diagramViewport: { x: 0, y: 0, zoom: 1 },
    diagramTheme: "smartpls_like",
    showGrid: true,
    layoutLocked: false,
  },
};

describe("native project lifecycle", () => {
  it("blocks even clean workspace replacement while a schema-6 Standard source remains bound", () => {
    expect(nativeSchema6BoundWorkspaceReplacementBlocker(false, false)).toBeNull();
    expect(nativeSchema6BoundWorkspaceReplacementBlocker(true, false)).toContain("Close the calculation-ready project");
    expect(nativeSchema6BoundWorkspaceReplacementBlocker(true, true)).toContain("Save a validated new copy");
  });

  it("fails legacy lifecycle work closed when strict Standard authority is active", () => {
    const operations: string[] = [];
    const gate = {
      standardSemModelV4OperationBlocker: (operation: "schema5_save" | "schema5_autosave" | "calculation" | "legacy_graph_serialization") => {
        operations.push(operation);
        return `blocked:${operation}`;
      },
    };

    expect(nativeLegacyProjectOperationBlocker(gate, "schema5_save")).toBe("blocked:schema5_save");
    expect(operations).toEqual(["schema5_save", "legacy_graph_serialization"]);
  });

  it("preserves legacy lifecycle behavior when no strict Standard authority is active", () => {
    const operations: string[] = [];
    const gate = {
      standardSemModelV4OperationBlocker: (operation: "schema5_save" | "schema5_autosave" | "calculation" | "legacy_graph_serialization") => {
        operations.push(operation);
        return null;
      },
    };

    expect(nativeLegacyProjectOperationBlocker(gate, "calculation")).toBeNull();
    expect(operations).toEqual(["calculation", "legacy_graph_serialization"]);
  });

  it("does not mark transient React Flow selection or drag state as a project change", () => {
    const selected = {
      ...base,
      nodes: [{ ...base.nodes[0], selected: true, dragging: true, measured: { width: 120, height: 80 } }],
    };

    expect(nativeProjectSignature(selected)).toBe(nativeProjectSignature(base));
  });

  it("signs an interaction_v2 draft from its scientific graph without legacy recipe serialization", () => {
    const moderation = {
      ...base,
      nodes: [
        ...base.nodes,
        {
          id: "w",
          type: "construct",
          position: { x: 10, y: 160 },
          data: { label: "Moderator", shortName: "W", mode: "reflective" as const, indicators: ["w1"] },
        },
        {
          id: "y",
          type: "construct",
          position: { x: 280, y: 20 },
          data: { label: "Outcome", shortName: "Y", mode: "reflective" as const, indicators: ["y1"] },
        },
        {
          id: "interaction-x-w-y",
          type: "construct",
          position: { x: 140, y: 90 },
          data: {
            label: "X x W",
            shortName: "XW",
            mode: "formative" as const,
            indicators: [],
            semantic: "interaction" as const,
            interaction: {
              kind: "interaction_v2" as const,
              termId: "term:x-w-y",
              operands: ["x", "w"],
              outcome: "y",
              focalRelationId: "x-y",
              canonicalMethod: "two_stage" as const,
              hierarchyPolicy: "strong" as const,
              productIndicator: null,
            },
          },
        },
      ],
      edges: [
        { id: "x-y", source: "x", target: "y" },
        { id: "w-y", source: "w", target: "y", data: { technicalGenerated: true } },
        { id: "xw-y", source: "interaction-x-w-y", target: "y" },
      ],
    } satisfies NativeProjectSignatureInput;

    const signed = nativeProjectSignature(moderation);
    expect(signed).toContain('"activeScientificGraph"');
    expect(nativeProjectSignature({
      ...moderation,
      nodes: moderation.nodes.map((node) => node.id === "interaction-x-w-y"
        ? { ...node, data: { ...node.data, label: "X x W revised" } }
        : node),
    })).not.toBe(signed);
  });

  it("detects model, method, dataset, result, and diagram changes", () => {
    const clean = nativeProjectSignature(base);
    const changed = [
      { ...base, nodes: [{ ...base.nodes[0], position: { x: 11, y: 20 } }] },
      { ...base, analysisSettings: { ...base.analysisSettings, bootstrapSamples: 10_000 } },
      { ...base, dataset: { ...base.dataset, fingerprint: "sha256:replacement" } },
      { ...base, runs: [{ id: "run-1", name: "PLS run", method: "PLS-SEM", createdAt: "2026-08-10T00:00:00Z", seed: 1, status: "completed" as const, warnings: [], fingerprint: "study" }] },
      { ...base, diagramLayout: { ...base.diagramLayout, showGrid: false } },
    ];

    for (const state of changed) expect(nativeProjectSignature(state)).not.toBe(clean);
  });

  it("stays dirty after adding a derived dataset and reactivating the clean dataset", () => {
    const clean = nativeProjectSignature(base);
    const derived = {
      ...base.dataset,
      id: "dataset-2",
      name: "study - x1 recode",
      columns: [...base.dataset.columns, "x1_recode"],
      fingerprint: "sha256:derived",
    };
    const recodeVersion = {
      datasetId: derived.id,
      parentDatasetId: base.dataset.id,
      operation: "recode" as const,
      createdAt: "2026-08-10T00:01:00Z",
      summary: "Recoded x1 into x1_recode",
      sourceColumn: "x1",
      targetColumn: "x1_recode",
    };

    const derivedState: NativeProjectSignatureInput = {
      ...base,
      dataset: derived,
      datasetCatalog: [base.dataset, derived],
      datasetVersions: [importVersion, recodeVersion],
    };
    const reactivatedCleanDataset = { ...derivedState, dataset: base.dataset };

    expect(nativeProjectSignature(derivedState)).not.toBe(clean);
    expect(nativeProjectSignature(reactivatedCleanDataset)).not.toBe(clean);
  });

  it("detects catalog metadata and lineage changes", () => {
    const clean = nativeProjectSignature(base);
    const metadataChanged: NativeProjectSignatureInput = {
      ...base,
      datasetCatalog: [{
        ...base.datasetCatalog[0],
        columnMetadata: [{
          name: "x1",
          label: "Revised predictor label",
          column_type: "numeric",
          scale_type: "continuous",
          missing_markers: [],
          theoretical_min: null,
          theoretical_max: null,
          value_labels: {},
        }],
      }],
    };
    const lineageChanged: NativeProjectSignatureInput = {
      ...base,
      datasetVersions: [{ ...importVersion, summary: "Imported with revised provenance" }],
    };

    expect(nativeProjectSignature(metadataChanged)).not.toBe(clean);
    expect(nativeProjectSignature(lineageChanged)).not.toBe(clean);
  });

  it("binds the complete reconstructable transformation receipt into dirty state", () => {
    const outputId = "00000000-0000-4000-8000-000000000002";
    const sourceId = "00000000-0000-4000-8000-000000000001";
    const receipt = {
      schema_version: 2 as const,
      engine: "qpls.dataset_transform.v2" as const,
      operation_id: "dataset_transform:0123456789abcdef01234567",
      source_dataset_id: sourceId,
      source_dataset_fingerprint: "v2:" + "a".repeat(64),
      output_dataset_id: outputId,
      output_dataset_fingerprint: "v2:" + "b".repeat(64),
      created_at: "2026-08-15T00:00:00.000Z",
      spec_sha256: "c".repeat(64),
      spec: {
        kind: "reverse_scale" as const,
        source_column: "x1",
        target_column: "x1_reversed",
        scale_min: 1,
        scale_max: 7,
      },
      input_columns: ["x1"],
      output_columns: ["x1_reversed"],
      source_row_count: 100,
      output_missing_count: 2,
    };
    const state: NativeProjectSignatureInput = {
      ...base,
      datasetVersions: [{
        datasetId: outputId,
        parentDatasetId: sourceId,
        operation: "transform",
        createdAt: receipt.created_at,
        summary: "Derived x1_reversed",
        sourceColumn: "x1",
        targetColumn: "x1_reversed",
        transformation: receipt,
      }],
    };
    const changed: NativeProjectSignatureInput = {
      ...state,
      datasetVersions: [{
        ...state.datasetVersions[0],
        transformation: { ...receipt, output_missing_count: 3 },
      }],
    };

    expect(nativeProjectSignature(changed)).not.toBe(nativeProjectSignature(state));
  });

  it("detects model catalog, inactive presentation, and saved-report changes but ignores explorer selection", () => {
    const clean = nativeProjectSignature(base);
    const catalogChanged: NativeProjectSignatureInput = {
      ...base,
      projectModels: [{ ...base.projectModels[0], name: "Renamed model" }],
    };
    const presentationChanged: NativeProjectSignatureInput = {
      ...base,
      modelPresentations: {
        "model-2": { nodes: [{ ...base.nodes[0], position: { x: 900, y: 20 } }], edges: [] },
      },
    };
    const reportChanged: NativeProjectSignatureInput = {
      ...base,
      savedReports: [{ resultId: "run-1", name: "Reviewer report", savedAt: "2026-08-11T01:00:00Z" }],
    };
    const semAuthoringChanged: NativeProjectSignatureInput = {
      ...base,
      nodes: base.nodes.map((node, index) => index === 0 ? {
        ...node,
        data: { ...node.data, semModelV4: { version: 1, construct: { kind: "composite" } } },
      } : node),
    };
    const selectionOnly = {
      ...base,
      explorerSelection: { kind: "reports", resultId: "ignored" },
    } as NativeProjectSignatureInput & { explorerSelection: unknown };

    expect(nativeProjectSignature(catalogChanged)).not.toBe(clean);
    expect(nativeProjectSignature(presentationChanged)).not.toBe(clean);
    expect(nativeProjectSignature(reportChanged)).not.toBe(clean);
    expect(nativeProjectSignature(semAuthoringChanged)).not.toBe(clean);
    expect(nativeProjectSignature(selectionOnly)).toBe(clean);
  });

  it("treats a model switch as navigation while retaining edits made before the switch", () => {
    const secondModel = { ...base.projectModels[0], id: "model-2", name: "Second model" };
    const secondNodes = base.nodes.map((node) => ({ ...node, position: { x: 600, y: 120 } }));
    const beforeSwitch: NativeProjectSignatureInput = {
      ...base,
      projectModels: [base.projectModels[0], secondModel],
      activeModelId: "model-1",
      modelPresentations: {
        "model-2": { nodes: secondNodes, edges: base.edges, diagramLayout: base.diagramLayout },
      },
    };
    const afterSwitch: NativeProjectSignatureInput = {
      ...beforeSwitch,
      activeModelId: "model-2",
      nodes: secondNodes,
      modelPresentations: {
        "model-1": { nodes: base.nodes, edges: base.edges, diagramLayout: base.diagramLayout },
      },
    };
    const editedBeforeSwitch: NativeProjectSignatureInput = {
      ...afterSwitch,
      modelPresentations: {
        "model-1": {
          nodes: base.nodes.map((node) => ({ ...node, position: { x: 111, y: node.position.y } })),
          edges: base.edges,
          diagramLayout: base.diagramLayout,
        },
      },
    };

    expect(nativeProjectSignature(afterSwitch)).toBe(nativeProjectSignature(beforeSwitch));
    expect(nativeProjectSignature(editedBeforeSwitch)).not.toBe(nativeProjectSignature(beforeSwitch));
  });

  it("requires an open project and an established clean baseline before reporting dirty state", () => {
    const current = nativeProjectSignature(base);

    expect(hasUnsavedNativeProjectChanges(false, "different", current)).toBe(false);
    expect(hasUnsavedNativeProjectChanges(true, null, current)).toBe(false);
    expect(hasUnsavedNativeProjectChanges(true, current, current)).toBe(false);
    expect(hasUnsavedNativeProjectChanges(true, "different", current)).toBe(true);
  });

  it("uses strict authority anchors and pending native work instead of the projected graph signature", () => {
    const unchangedProjection = nativeProjectSignature(base);

    expect(hasUnsavedNativeProjectChanges(true, unchangedProjection, unchangedProjection, {
      active: true,
      dirty: true,
      operationPending: false,
    })).toBe(true);
    expect(hasUnsavedNativeProjectChanges(true, "stale-legacy-projection", unchangedProjection, {
      active: true,
      dirty: false,
      operationPending: false,
    })).toBe(false);
    expect(hasUnsavedNativeProjectChanges(false, null, unchangedProjection, {
      active: false,
      dirty: false,
      operationPending: true,
    })).toBe(true);
  });

  it("accepts backend-revealed lineage as part of the saved baseline without hiding later workspace edits", () => {
    const version = {
      ...base.dataset,
      id: "dataset-2",
      name: "study - recode",
      fingerprint: "sha256:study-recode",
      columns: [...base.dataset.columns, "x1_recode"],
    };
    const lineage = [{
      datasetId: version.id,
      parentDatasetId: base.dataset.id,
      operation: "recode" as const,
      createdAt: "2026-08-10T08:00:00Z",
      summary: "Recoded x1 as x1_recode",
      sourceColumn: "x1",
      targetColumn: "x1_recode",
    }];
    const saved = nativeSavedProjectSignature(base, [base.dataset, version], lineage, base.dataset.id);
    const authoritative = nativeProjectSignature({
      ...base,
      datasetCatalog: [base.dataset, version],
      datasetVersions: lineage,
    });

    expect(saved).toBe(authoritative);
    expect(saved).not.toBe(nativeProjectSignature(base));
    expect(nativeProjectSignature({ ...base, datasetCatalog: [base.dataset, version], datasetVersions: lineage, nodes: [{ ...base.nodes[0], position: { x: 99, y: 20 } }] })).not.toBe(saved);
  });

  it("accepts the backend explorer catalogue and presentations as the saved baseline", () => {
    const explorer = {
      projectModels: [{ ...base.projectModels[0], name: "Saved model" }],
      activeModelId: "model-1",
      modelPresentations: { "model-1": { nodes: base.nodes, edges: base.edges, diagramLayout: base.diagramLayout } },
      savedReports: [{ resultId: "result-1", name: "Saved report", savedAt: "2026-08-11T02:00:00Z" }],
    };
    const saved = nativeSavedProjectSignature(
      base,
      base.datasetCatalog,
      base.datasetVersions,
      base.dataset.id,
      explorer,
    );

    expect(saved).toBe(nativeProjectSignature({ ...base, ...explorer }));
  });

  it("keeps an explorer mutation dirty through recovery save and clears it only after explicit save/reopen", () => {
    const cleanBaseline = nativeProjectSignature(base);
    const explorer = {
      projectModels: [{ ...base.projectModels[0], name: "Renamed after open" }],
      activeModelId: base.activeModelId,
      modelPresentations: {
        "model-1": { nodes: base.nodes, edges: base.edges, diagramLayout: base.diagramLayout },
      },
      savedReports: [{ resultId: "result-1", name: "Reviewer report", savedAt: "2026-08-11T03:00:00Z" }],
    };
    const mutated = { ...base, ...explorer };
    const mutatedSignature = nativeProjectSignature(mutated);

    expect(hasUnsavedNativeProjectChanges(true, cleanBaseline, mutatedSignature)).toBe(true);
    // A recovery save writes safety data but deliberately retains the explicit-save baseline.
    expect(hasUnsavedNativeProjectChanges(true, cleanBaseline, mutatedSignature)).toBe(true);

    const explicitSaveBaseline = nativeSavedProjectSignature(
      base,
      base.datasetCatalog,
      base.datasetVersions,
      base.dataset.id,
      explorer,
    );
    expect(explicitSaveBaseline).toBe(mutatedSignature);
    expect(hasUnsavedNativeProjectChanges(true, explicitSaveBaseline, nativeProjectSignature(mutated))).toBe(false);
  });
});
