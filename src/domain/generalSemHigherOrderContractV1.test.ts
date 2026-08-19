import { describe, expect, it } from "vitest";
import type {
  HigherOrderConstructionApproachV4,
  HigherOrderMeasurementTypeV4,
  SemModelV4,
} from "./semModelV4";
import {
  GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_CELL_V1,
  GENERAL_SEM_PLS_HIGHER_ORDER_POINT_CELL_V1,
  generalSemHocApproachTypeSupportedV1,
  preflightGeneralSemHocContractV1,
} from "./generalSemHigherOrderContractV1";

function hocModel(input: {
  approach?: HigherOrderConstructionApproachV4;
  measurementType?: HigherOrderMeasurementTypeV4;
  endogenous?: boolean;
  locMode?: "mode_a" | "mode_b";
} = {}): SemModelV4 {
  const output = "derived:hoc";
  return {
    schema_version: 4,
    id: "model:hoc",
    name: "HOC contract fixture",
    variables: [
      { kind: "composite", id: "construct:a", label: "A", weighting: { kind: input.locMode ?? "mode_a" } },
      { kind: "composite", id: "construct:b", label: "B", weighting: { kind: input.locMode ?? "mode_a" } },
      { kind: "composite", id: "construct:x", label: "X", weighting: { kind: "mode_a" } },
      { kind: "composite", id: "construct:y", label: "Y", weighting: { kind: "mode_a" } },
      { kind: "derived", id: output, label: "HOC" },
    ],
    relations: [
      ...(input.endogenous ? [{
        kind: "structural" as const,
        id: "relation:x_hoc",
        source: "construct:x",
        target: output,
        parameter: "parameter:x_hoc",
        role: "structural" as const,
        intercept_parameter: null,
      }] : []),
      {
        kind: "structural",
        id: "relation:hoc_y",
        source: output,
        target: "construct:y",
        parameter: "parameter:hoc_y",
        role: "structural",
        intercept_parameter: null,
      },
    ],
    parameters: [],
    constraints: [],
    derived_terms: [{
      kind: "higher_order",
      id: "term:hoc",
      output,
      components: ["construct:a", "construct:b"],
      approach: input.approach ?? "disjoint_two_stage",
      measurement_type: input.measurementType ?? "reflective_reflective",
    }],
    group: { kind: "single_group" },
    data_binding: {
      kind: "raw",
      dataset_id: "dataset:hoc",
      missing_data: "listwise_deletion",
      weight: null,
      cluster_variable: null,
      strata_variable: null,
    },
    annotations: [],
    presentation: { kind: "none" },
  };
}

describe("General SEM HOC compiler contract v1", () => {
  it("mirrors the exact approach, type, and topology matrix", () => {
    const types: HigherOrderMeasurementTypeV4[] = [
      "reflective_reflective",
      "reflective_formative",
      "formative_reflective",
      "formative_formative",
    ];
    for (const endogenous of [false, true]) {
      for (const type of types) {
        expect(generalSemHocApproachTypeSupportedV1("embedded_two_stage", type, endogenous)).toBe(true);
        expect(generalSemHocApproachTypeSupportedV1("disjoint_two_stage", type, endogenous)).toBe(true);
        expect(generalSemHocApproachTypeSupportedV1("hybrid", type, endogenous)).toBe(false);
      }
      expect(generalSemHocApproachTypeSupportedV1("repeated_indicators", "reflective_reflective", endogenous)).toBe(true);
      expect(generalSemHocApproachTypeSupportedV1("repeated_indicators", "formative_reflective", endogenous)).toBe(true);
      expect(generalSemHocApproachTypeSupportedV1("repeated_indicators", "reflective_formative", endogenous)).toBe(!endogenous);
      expect(generalSemHocApproachTypeSupportedV1("extended_repeated_indicators", "reflective_formative", endogenous)).toBe(endogenous);
      expect(generalSemHocApproachTypeSupportedV1("extended_repeated_indicators", "reflective_reflective", endogenous)).toBe(false);
    }
  });

  it("reserves exact point/bootstrap cells but stays blocked until the runner exists", () => {
    const point = preflightGeneralSemHocContractV1(hocModel(), false);
    expect(point.contractCompiles).toBe(true);
    expect(point.capabilityCells).toEqual([GENERAL_SEM_PLS_HIGHER_ORDER_POINT_CELL_V1]);
    expect(point.diagnostics.map((item) => item.code)).toEqual([
      "sem.capability.pls.higher_order_runtime_not_connected",
    ]);

    const bootstrap = preflightGeneralSemHocContractV1(hocModel(), true);
    expect(bootstrap.capabilityCells).toEqual([
      GENERAL_SEM_PLS_HIGHER_ORDER_POINT_CELL_V1,
      GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_CELL_V1,
    ]);
    expect(bootstrap.evidence.map((item) => item.evidence_id)).toContain(
      "compiler:recipe_v4_to_compiled_pls_plan_v3_higher_order_full_model_case_bootstrap_v1",
    );
  });

  it("returns corrective diagnostics for unsupported topology and LOC modes", () => {
    const topology = preflightGeneralSemHocContractV1(hocModel({
      approach: "repeated_indicators",
      measurementType: "reflective_formative",
      endogenous: true,
    }), false);
    expect(topology.contractCompiles).toBe(false);
    expect(topology.diagnostics.map((item) => item.code)).toContain(
      "sem.capability.pls.higher_order_approach_type_topology_not_executable",
    );

    const mode = preflightGeneralSemHocContractV1(hocModel({
      measurementType: "formative_reflective",
      locMode: "mode_a",
    }), false);
    expect(mode.contractCompiles).toBe(false);
    expect(mode.diagnostics.filter((item) => (
      item.code === "sem.capability.pls.higher_order_measurement_mode_not_executable"
    ))).toHaveLength(2);
  });
});
