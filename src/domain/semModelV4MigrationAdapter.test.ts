import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { ConstructData, NativeCanonicalModelSpec, NativeModelPresentation } from "../types";
import { scientificSemModelV4HashInput } from "./semModelV4";
import {
  authorPresentationCovarianceV4,
  authorScientificCovarianceV4,
  compileMigratedCbsemPlanV2,
  compileMigratedPlsPlanV2,
  confirmLegacyEstimandSemModelV4,
  convertPresentationCovarianceToScientificV4,
  currentDatasetToSemDataBindingV4,
  migrateCurrentQuickPlsGraphToSemModelV4,
  roundTripCurrentQuickPlsGraphV4,
  SemModelV4MigrationAdapterError,
  semConstructVariableIdV4,
  semObservedVariableIdV4,
} from "./semModelV4MigrationAdapter";

function model(): NativeCanonicalModelSpec {
  return {
    id: "model-current",
    name: "Current model",
    constructs: [
      { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
      { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x2", "x1"] },
      { id: "z", name: "Context", short_name: "Z", mode: "reflective", indicators: ["z1", "z2"] },
    ],
    paths: [
      { source: "x", target: "y" },
      { source: "z", target: "y" },
    ],
    controls: [],
    higher_order_constructs: [],
    interactions: [],
  };
}

function node(id: string, x: number, y: number): Node<ConstructData> {
  const construct = model().constructs.find((candidate) => candidate.id === id)!;
  return {
    id,
    type: "construct",
    position: { x, y },
    data: {
      label: construct.name,
      shortName: construct.short_name,
      mode: construct.mode,
      indicators: [...construct.indicators],
    },
  };
}

function presentation(): NativeModelPresentation {
  return {
    nodes: [node("z", 20, 30), node("x", 100, 40), node("y", 300, 100)],
    edges: [
      { id: "path-x-y", source: "x", target: "y", type: "smoothstep", label: "Path" },
      { id: "display-cov-x-y", source: "x", target: "y", type: "smoothstep", label: "Visual covariance", data: { role: "covariance" } },
      { id: "path-z-y", source: "z", target: "y", type: "straight", label: "Path" },
    ],
    diagramLayout: {
      diagramVersion: "sem_designer_v1",
      constructLayouts: {},
      indicatorLayouts: {},
      edgeLayouts: { "path-x-y": { routing: "curved" } },
      diagramViewport: { x: 12, y: 14, zoom: 1.25 },
      diagramTheme: "academic_grayscale",
      showGrid: true,
      layoutLocked: false,
    },
  };
}

function rawBinding(source = model()) {
  return currentDatasetToSemDataBindingV4({
    id: "dataset-current",
    columns: ["case_id", "z2", "x1", "y2", "x2", "y1", "z1"],
    kind: "raw",
    rowCount: 250,
  }, source);
}

describe("SemModelV4 current-graph migration adapter", () => {
  it("migrates PLS constructs as composites without mutating or reordering the current graph", () => {
    const sourceModel = model();
    sourceModel.constructs[2].mode = "formative";
    const sourcePresentation = presentation();
    const sourceBefore = JSON.stringify({ sourceModel, sourcePresentation });

    const migrated = migrateCurrentQuickPlsGraphToSemModelV4({
      model: sourceModel,
      presentation: sourcePresentation,
      data_binding: rawBinding(sourceModel),
      method_intent: "pls_sem",
    });

    expect(JSON.stringify({ sourceModel, sourcePresentation })).toBe(sourceBefore);
    expect(migrated.kind).toBe("sem_model_v4");
    if (migrated.kind !== "sem_model_v4") throw new Error("expected confirmed model");
    expect(migrated.model.id).toBe("model-current");
    expect(migrated.model.data_binding.dataset_id).toBe("dataset-current");
    expect(migrated.source_graph.model.constructs.map((construct) => construct.id)).toEqual(["y", "x", "z"]);
    expect(migrated.source_graph.model.constructs[1].indicators).toEqual(["x2", "x1"]);
    expect(migrated.model.variables.find((variable) => variable.id === semConstructVariableIdV4("x"))?.kind).toBe("composite");
    expect(migrated.model.variables.find((variable) => variable.id === semConstructVariableIdV4("z"))).toMatchObject({
      kind: "composite",
      weighting: { kind: "mode_b" },
    });
    expect(migrated.model.annotations).toEqual([expect.objectContaining({
      kind: "display_only_covariance",
      id: "display-cov-x-y",
    })]);
    expect(compileMigratedPlsPlanV2(migrated).blocks).toHaveLength(3);
    expect(Object.isFrozen(migrated)).toBe(true);
  });

  it("migrates reflective CB-SEM constructs as common factors and retains the declared data binding", () => {
    const migrated = migrateCurrentQuickPlsGraphToSemModelV4({
      model: model(),
      presentation: presentation(),
      data_binding: rawBinding(),
      method_intent: "cbsem",
    });
    expect(migrated.kind).toBe("sem_model_v4");
    if (migrated.kind !== "sem_model_v4") throw new Error("expected confirmed model");
    expect(migrated.model.variables.filter((variable) => variable.kind === "common_factor")).toHaveLength(3);
    expect(compileMigratedCbsemPlanV2(migrated).covariances).toEqual([]);
    expect(migrated.model.presentation).toMatchObject({
      kind: "canvas",
      zoom: 1.25,
      pan_x: 12,
      pan_y: 14,
    });
  });

  it("keeps method-neutral models explicitly pending until factor/composite confirmation", () => {
    const pending = migrateCurrentQuickPlsGraphToSemModelV4({
      model: model(),
      presentation: presentation(),
      data_binding: rawBinding(),
      method_intent: "method_neutral",
    });
    expect(pending).toMatchObject({
      kind: "legacy_estimand_unspecified",
      automatic_conversion_blocker: null,
    });
    expect(() => compileMigratedPlsPlanV2(pending)).toThrowError(expect.objectContaining({
      code: "migration.estimand_confirmation_required",
    }));

    const confirmed = confirmLegacyEstimandSemModelV4(pending, "pls_composite");
    expect(confirmed.kind).toBe("sem_model_v4");
    expect(compileMigratedPlsPlanV2(confirmed).paths).toHaveLength(2);
    expect(pending.kind).toBe("legacy_estimand_unspecified");
  });

  it("retains a legacy covariance as presentation-only until explicit scientific conversion", () => {
    const migrated = migrateCurrentQuickPlsGraphToSemModelV4({
      model: model(),
      presentation: presentation(),
      data_binding: rawBinding(),
      method_intent: "cbsem",
    });
    if (migrated.kind !== "sem_model_v4") throw new Error("expected confirmed model");
    const scientificBefore = scientificSemModelV4HashInput(migrated.model);
    expect(compileMigratedCbsemPlanV2(migrated).covariances).toEqual([]);

    const converted = convertPresentationCovarianceToScientificV4(migrated, "display-cov-x-y");
    const plan = compileMigratedCbsemPlanV2(converted);
    expect(plan.covariances).toHaveLength(1);
    expect(scientificSemModelV4HashInput(converted.model)).not.toBe(scientificBefore);
    expect(converted.model.annotations).toContainEqual(expect.objectContaining({ id: "display-cov-x-y" }));
    expect(converted.covariance_lineage).toContainEqual(expect.objectContaining({
      source_edge_id: "display-cov-x-y",
      annotation_id: "display-cov-x-y",
      scientific_relation_id: plan.covariances[0].relation_id,
      scientific_parameter_id: plan.covariances[0].parameter_id,
      operation: "convert_to_model_covariance_v1",
    }));
    expect(convertPresentationCovarianceToScientificV4(converted, "display-cov-x-y")).toBe(converted);
    expect(roundTripCurrentQuickPlsGraphV4(converted).presentation).toEqual(presentation());
  });

  it("authors presentation and scientific covariance through separate APIs", () => {
    const migrated = migrateCurrentQuickPlsGraphToSemModelV4({
      model: model(),
      presentation: presentation(),
      data_binding: rawBinding(),
      method_intent: "cbsem",
    });
    if (migrated.kind !== "sem_model_v4") throw new Error("expected confirmed model");
    const scientificBefore = scientificSemModelV4HashInput(migrated.model);
    const presentationOnly = authorPresentationCovarianceV4(migrated, {
      id: "display-cov-y-z",
      left_construct: "y",
      right_construct: "z",
      label: "Layout cue",
    });
    if (presentationOnly.kind !== "sem_model_v4") throw new Error("expected confirmed model");
    expect(scientificSemModelV4HashInput(presentationOnly.model)).toBe(scientificBefore);
    expect(compileMigratedCbsemPlanV2(presentationOnly).covariances).toEqual([]);
    expect(roundTripCurrentQuickPlsGraphV4(presentationOnly).presentation.edges?.at(-1)).toMatchObject({
      id: "display-cov-y-z",
      data: { role: "covariance" },
    });

    const scientific = authorScientificCovarianceV4(presentationOnly, {
      id: "residual-x1-y1",
      left: { kind: "residual_of", id: semObservedVariableIdV4("x1") },
      right: { kind: "residual_of", id: semObservedVariableIdV4("y1") },
      label: "Residual x1 with y1",
    });
    expect(compileMigratedCbsemPlanV2(scientific).covariances).toHaveLength(1);
    expect(roundTripCurrentQuickPlsGraphV4(scientific).scientific_covariances).toHaveLength(1);
  });

  it("maps matrix variables in declared dataset order", () => {
    const binding = currentDatasetToSemDataBindingV4({
      id: "matrix-data",
      columns: ["z2", "x1", "y2", "x2", "y1", "z1"],
      kind: "covariance",
      sampleSize: 300,
    }, model());
    expect(binding).toEqual({
      kind: "covariance",
      dataset_id: "matrix-data",
      variables: ["observed:z2", "observed:x1", "observed:y2", "observed:x2", "observed:y1", "observed:z1"],
      means: null,
      standard_deviations: null,
      sample: {
        sample_size: 300,
        covariance_denominator: "sample_n_minus_one",
        group_sample_sizes: {},
      },
    });
  });

  it("preserves an explicitly supplied missing-data policy without pretending the initial compiler supports it", () => {
    const binding = { ...rawBinding(), missing_data: "mean_replacement" as const };
    const migrated = migrateCurrentQuickPlsGraphToSemModelV4({
      model: model(),
      presentation: presentation(),
      data_binding: binding,
      method_intent: "pls_sem",
    });
    expect(migrated.kind).toBe("sem_model_v4");
    if (migrated.kind !== "sem_model_v4") throw new Error("expected confirmed model");
    expect(migrated.model.data_binding).toEqual(binding);
    expect(() => compileMigratedPlsPlanV2(migrated)).toThrowError(expect.objectContaining({
      code: "pls.data_binding_unsupported",
    }));
  });

  it("fails closed for unknown, duplicate, and role-ambiguous presentation edges", () => {
    const cases: Array<{ edge: Edge; code: string }> = [
      { edge: { id: "unknown-role", source: "x", target: "y", data: { role: "mystery" } }, code: "migration.edge_role_unknown" },
      { edge: { id: "unknown-target", source: "x", target: "missing", data: { role: "covariance" } }, code: "migration.presentation_edge_target_unknown" },
      { edge: { id: "wrong-control", source: "x", target: "y", data: { role: "control" } }, code: "migration.presentation_path_role_mismatch" },
      { edge: { id: "measurement::x::evil", source: "x", target: "missing" }, code: "migration.measurement_edge_ambiguous" },
    ];
    for (const candidate of cases) {
      const invalid = presentation();
      invalid.edges = [candidate.edge];
      expect(() => migrateCurrentQuickPlsGraphToSemModelV4({
        model: model(),
        presentation: invalid,
        data_binding: rawBinding(),
        method_intent: "pls_sem",
      })).toThrowError(expect.objectContaining({ code: candidate.code }));
    }

    const duplicate = presentation();
    duplicate.edges = [
      { id: "cov-a", source: "x", target: "y", data: { role: "covariance" } },
      { id: "cov-b", source: "y", target: "x", data: { role: "covariance" } },
    ];
    expect(() => migrateCurrentQuickPlsGraphToSemModelV4({
      model: model(),
      presentation: duplicate,
      data_binding: rawBinding(),
      method_intent: "cbsem",
    })).toThrowError(expect.objectContaining({ code: "migration.display_covariance_duplicate" }));
  });

  it("stores automatic conversion blockers instead of guessing advanced semantics", () => {
    const advanced = model();
    advanced.controls = [{ source: "x", target: "y", label: "Control" }];
    const advancedPresentation = presentation();
    advancedPresentation.edges = advancedPresentation.edges?.map((edge) => edge.id === "path-x-y"
      ? { ...edge, data: { role: "control", controlLabel: "Control" } }
      : edge);
    const pending = migrateCurrentQuickPlsGraphToSemModelV4({
      model: advanced,
      presentation: advancedPresentation,
      data_binding: rawBinding(advanced),
      method_intent: "pls_sem",
    });
    expect(pending).toMatchObject({
      kind: "legacy_estimand_unspecified",
      automatic_conversion_blocker: { code: "migration.advanced_semantics" },
    });
    expect(() => confirmLegacyEstimandSemModelV4(pending, "pls_composite")).toThrowError();
  });

  it("uses a typed adapter error for a missing matrix sample size", () => {
    try {
      currentDatasetToSemDataBindingV4({
        id: "matrix-data",
        columns: ["y1", "y2", "x1", "x2", "z1", "z2"],
        kind: "correlation",
        rowCount: 6,
      }, model());
      throw new Error("expected migration to fail");
    } catch (error) {
      expect(error).toBeInstanceOf(SemModelV4MigrationAdapterError);
      expect((error as SemModelV4MigrationAdapterError).code).toBe("migration.matrix_sample_size_invalid");
    }
  });
});
