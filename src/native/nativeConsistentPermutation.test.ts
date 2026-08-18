import { describe, expect, it } from "vitest";
import type {
  AnalysisResultEnvelope,
  AnalysisRun,
  NativeCanonicalAnalysisRecipe,
  PlsPermutationRun,
} from "../types";
import {
  NATIVE_PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING,
  NATIVE_PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING_V1,
  NATIVE_PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_METHOD_VERSION,
  NATIVE_PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_TEST,
  NATIVE_PLSC_CONSISTENT_PERMUTATION_FAILURE_LEDGER_WARNING,
  NATIVE_PLSC_CONSISTENT_PERMUTATION_FULL_REFIT_WARNING,
  NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION,
  NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_ORIENTATION,
  NATIVE_PLSC_CONSISTENT_PERMUTATION_TEST,
  NATIVE_PLSC_CONSISTENT_PERMUTATION_TEST_V1,
  nativePlscConsistentPermutationProjection,
} from "./nativeConsistentPermutation";
import { nativeRunProvenanceTable } from "./nativeExportTables";
import { nativeRunFromCanonicalResult } from "./nativeCanonicalProject";
import { nativeResultTables } from "./nativeResults";

const DIGEST = "ab".repeat(32);
const ASSIGNMENT_DIGEST = "cd".repeat(32);

function parameter(
  kind: string,
  parts: string[],
  family: NonNullable<PlsPermutationRun["parameters"][number]["family"]>,
  estimateA: number,
  estimateB: number,
): PlsPermutationRun["parameters"][number] {
  return {
    parameter: JSON.stringify([kind, parts]),
    family,
    estimate_a: estimateA,
    estimate_b: estimateB,
    original: estimateA - estimateB,
    exceedances: 10,
    p_value_two_sided: 0.11,
    permutations: 99,
  };
}

function attachSelectedTail(
  permutation: PlsPermutationRun,
  provenance: { method_version: string },
  selectedTestTail: "group_a_greater" | "group_a_less",
): void {
  const directional = permutation.directional_inference;
  if (!directional) throw new Error("fixture drift");
  permutation.selected_tail_inference = {
    method_version: NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION,
    orientation: NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_ORIENTATION,
    selected_test_tail: selectedTestTail,
    parameters: directional.parameters.map((row) => ({
      parameter: row.parameter,
      selected_exceedances: selectedTestTail === "group_a_greater"
        ? row.greater_or_equal
        : row.less_or_equal,
      selected_p_value: selectedTestTail === "group_a_greater"
        ? row.p_value_greater
        : row.p_value_less,
      permutations: row.permutations,
    })),
  };
  provenance.method_version += `+${NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION}`;
}

function completedConsistentPermutationRun(): AnalysisRun {
  const parameters = [
    parameter("plsc_rho_a", ["x"], "rho_a", 0.84, 0.81),
    parameter("plsc_rho_a", ["y"], "rho_a", 0.87, 0.83),
    parameter("plsc_construct_correlation", ["x", "y"], "construct_correlation", 0.55, 0.42),
    parameter("plsc_outer_loading", ["x", "x1"], "outer_loading", 0.82, 0.80),
    parameter("plsc_outer_loading", ["x", "x2"], "outer_loading", 0.79, 0.77),
    parameter("plsc_outer_loading", ["y", "y1"], "outer_loading", 0.86, 0.81),
    parameter("plsc_outer_loading", ["y", "y2"], "outer_loading", 0.83, 0.79),
    parameter("plsc_path", ["x", "y"], "path", 0.52, 0.36),
    parameter("plsc_r_squared", ["y"], "r_squared", 0.27, 0.13),
  ].sort((left, right) =>
    left.parameter < right.parameter ? -1 : left.parameter > right.parameter ? 1 : 0,
  );
  return {
    id: "plsc-permutation-run",
    name: "PLSc consistent permutation",
    method: "PLSc Consistent Permutation",
    createdAt: "2026-08-15T01:00:00.000Z",
    seed: 42,
    status: "completed",
    warnings: [],
    fingerprint: "sha256:plsc-permutation-fixture",
    modelSnapshot: {
      nodes: [
        { id: "x", type: "construct", position: { x: 0, y: 0 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
        { id: "y", type: "construct", position: { x: 240, y: 0 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] } },
      ],
      edges: [{ id: "x-y", source: "x", target: "y" }],
    },
    result: {
      method_version: "plsc_v2",
      converged: true,
      iterations: 8,
      used_observations: 40,
      omitted_observations: 0,
      outer_estimates: [
        { construct: "x", indicator: "x1", weight: 0.5, loading: 0.81 },
        { construct: "x", indicator: "x2", weight: 0.5, loading: 0.78 },
        { construct: "y", indicator: "y1", weight: 0.5, loading: 0.84 },
        { construct: "y", indicator: "y2", weight: 0.5, loading: 0.82 },
      ],
      paths: [{ source: "x", target: "y", coefficient: 0.44 }],
      effects: [{ source: "x", target: "y", direct: 0.44, indirect: 0, total: 0.44 }],
      plsc: {
        method_version: "plsc_v2",
        reliability_method_version: "dijkstra_henseler_rho_a_v1",
        tolerance: 1e-12,
        reliabilities: [{ construct: "x", rho_a: 0.83 }, { construct: "y", rho_a: 0.85 }],
        construct_correlations: [{ left: "x", right: "y", original: 0.4, corrected: 0.48 }],
        corrected_paths: [{ source: "x", target: "y", coefficient: 0.44 }],
        corrected_outer_loadings: [
          { construct: "x", indicator: "x1", weight: 0.5, loading: 0.81 },
          { construct: "x", indicator: "x2", weight: 0.5, loading: 0.78 },
          { construct: "y", indicator: "y1", weight: 0.5, loading: 0.84 },
          { construct: "y", indicator: "y2", weight: 0.5, loading: 0.82 },
        ],
        corrected_r_squared: { y: 0.19 },
        warnings: [],
      },
      r_squared: { y: 0.19 },
      warnings: [],
    },
    permutation: {
      method_version: "plsc_permutation_v1",
      estimator_method_version: "plsc_v2",
      scheduler_method_version: "indexed_group_label_permutation_v1",
      plan: { permutations: 99, master_seed: 42, operation: "plsc_group_label_permutation_v1" },
      test_method: NATIVE_PLSC_CONSISTENT_PERMUTATION_TEST,
      significance_level: 0.05,
      minimum_usable_fraction: 0.9,
      retry_policy: "no_retry_no_replacement_fixed_indexed_labels_v1",
      group_column: "group",
      group_a: { group: "A", observations: 20, parameter_values_sha256: DIGEST },
      group_b: { group: "B", observations: 20, parameter_values_sha256: DIGEST },
      pooled_parameter_values_sha256: DIGEST,
      usable_permutations: 99,
      failed_permutations: [],
      permutation_ledger: Array.from({ length: 99 }, (_, permutation_index) => ({
        permutation_index,
        label_assignment_sha256: ASSIGNMENT_DIGEST,
        status: "success" as const,
        parameter_values_sha256: DIGEST,
        reason_code: null,
        message: null,
      })),
      parameters,
      directional_inference: {
        method_version: NATIVE_PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_METHOD_VERSION,
        test_method: NATIVE_PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_TEST,
        parameters: parameters.map(({ parameter }) => ({
          parameter,
          greater_or_equal: 8,
          less_or_equal: 91,
          p_value_greater: 0.09,
          p_value_less: 0.92,
          permutations: 99,
        })),
      },
      warnings: [
        NATIVE_PLSC_CONSISTENT_PERMUTATION_FULL_REFIT_WARNING,
        NATIVE_PLSC_CONSISTENT_PERMUTATION_FAILURE_LEDGER_WARNING,
        NATIVE_PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING,
      ],
    },
    provenance: {
      recipe_id: "plsc-permutation-recipe",
      dataset_fingerprint: "sha256:plsc-permutation-fixture",
      method: "plsc",
      method_version: "pls_pm_v1+plsc_v2+plsc_permutation_v1+indexed_group_label_permutation_v1",
      engine_version: "test",
      seed: 42,
      settings: {
        method: "plsc",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 99,
        seed: 42,
        workers: 4,
        confidence_level: 0.95,
        preprocessing: "standardized",
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-15T00:59:00.000Z",
      completed_at: "2026-08-15T01:00:00.000Z",
    },
  };
}

function canonicalConsistentPermutationFixture(): {
  envelope: AnalysisResultEnvelope;
  recipe: NativeCanonicalAnalysisRecipe;
} {
  const run = completedConsistentPermutationRun();
  const assessment = {
    method_version: "pls_assessment_v1",
    construct_quality: [],
    cross_loadings: [],
    fornell_larcker: { constructs: [], values: [] },
    r_squared: { y: 0.19 },
    structural_quality: [],
    structural_vif: [],
    formative_indicator_vif: [],
    f_squared: [],
    warnings: [],
  };
  const recipe: NativeCanonicalAnalysisRecipe = {
    schema_version: 3,
    id: run.provenance!.recipe_id,
    created_at: "2026-08-15T00:58:00.000Z",
    dataset_fingerprint: run.provenance!.dataset_fingerprint,
    model: {
      id: "plsc-permutation-model",
      name: "PLSc consistent permutation",
      constructs: [
        { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
        { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
      ],
      paths: [{ source: "x", target: "y" }],
      controls: [],
      higher_order_constructs: [],
      interactions: [],
    },
    settings: { ...run.provenance!.settings },
    method_config: {
      kind: "plsc_permutation",
      group_column: "group",
      group_a: "A",
      group_b: "B",
    },
    metadata: {},
  };
  return {
    recipe,
    envelope: {
      schema_version: 1,
      id: run.id,
      status: "completed",
      provenance: run.provenance!,
      diagnostics: [],
      payload: {
        kind: "pls_pm_v3",
        estimation: run.result!,
        assessment,
        bootstrap: null,
        permutation: run.permutation!,
      },
    },
  };
}

describe("native PLSc consistent-permutation projection", () => {
  it("accepts the exact internal contract and renders separate experimental tables", () => {
    const run = completedConsistentPermutationRun();
    expect(nativePlscConsistentPermutationProjection(run)).toMatchObject({
      requestedPermutations: 99,
      usablePermutations: 99,
      failedPermutations: 0,
      minimumUsablePermutations: 90,
      successfulLedgerEntries: 99,
      parameterCounts: {
        path: 1,
        outer_loading: 4,
        rho_a: 2,
        construct_correlation: 1,
        r_squared: 1,
      },
    });
    const tables = nativeResultTables(run);
    expect(tables.find((table) => table.id === "plsc_permutation_accounting")?.status).toBe("experimental");
    expect(tables.find((table) => table.id === "plsc_permutation_selected_tail")).toBeUndefined();
    const paths = tables.find((table) => table.id === "plsc_permutation_paths");
    expect(paths?.rows[0][0]).toBe("Plsc path: Predictor → Outcome");
    expect(paths?.columns).toContain("p (greater)");
    expect(paths?.columns).toContain("p (less)");
    expect(paths?.rows[0]).toEqual(expect.arrayContaining(["8", "0.0900", "91", "0.9200"]));
    expect(tables.find((table) => table.id === "plsc_permutation_construct_criteria")?.warning).toContain("does not include MICOM");
    const provenance = nativeRunProvenanceTable(run);
    expect(provenance.status).toBe("experimental");
    expect(provenance.rows).toContainEqual(["Requested PLSc label assignments", "99"]);
    expect(provenance.rows).toContainEqual(["PLSc directional inference method", NATIVE_PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_METHOD_VERSION]);
    expect(provenance.rows).toContainEqual(["PLSc directional test", NATIVE_PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_TEST]);
    expect(provenance.rows.some(([label]) => label.startsWith("PLSc selected"))).toBe(false);
    expect(provenance.rows).toContainEqual(["PLSc permutation limitation", "MICOM, broader parameter/model scope, and more than two groups remain unavailable in this internal v1 result"]);
  });

  it("fails closed on ordinary-PLS attribution, malformed ledger, arithmetic, and warnings", () => {
    const cases: AnalysisRun[] = [];
    const ordinaryMarker = structuredClone(completedConsistentPermutationRun());
    ordinaryMarker.provenance!.method_version += "+freedman_lane_permutation_v1";
    cases.push(ordinaryMarker);

    const wrongIndex = structuredClone(completedConsistentPermutationRun());
    wrongIndex.permutation!.permutation_ledger![1].permutation_index = 2;
    cases.push(wrongIndex);

    const badProbability = structuredClone(completedConsistentPermutationRun());
    badProbability.permutation!.parameters[0].p_value_two_sided = 0.12;
    cases.push(badProbability);

    const missingWarning = structuredClone(completedConsistentPermutationRun());
    missingWarning.permutation!.warnings = [];
    cases.push(missingWarning);

    const missingDirectional = structuredClone(completedConsistentPermutationRun());
    delete missingDirectional.permutation!.directional_inference;
    cases.push(missingDirectional);

    const wrongDirectionalVersion = structuredClone(completedConsistentPermutationRun());
    wrongDirectionalVersion.permutation!.directional_inference!.method_version = "plsc_directional_permutation_v0";
    cases.push(wrongDirectionalVersion);

    const reorderedDirectional = structuredClone(completedConsistentPermutationRun());
    reorderedDirectional.permutation!.directional_inference!.parameters.reverse();
    cases.push(reorderedDirectional);

    const badDirectionalCount = structuredClone(completedConsistentPermutationRun());
    badDirectionalCount.permutation!.directional_inference!.parameters[0].greater_or_equal = 100;
    cases.push(badDirectionalCount);

    const badDirectionalProbability = structuredClone(completedConsistentPermutationRun());
    badDirectionalProbability.permutation!.directional_inference!.parameters[0].p_value_less = 0.91;
    cases.push(badDirectionalProbability);

    for (const run of cases) {
      expect(nativePlscConsistentPermutationProjection(run)).toBeNull();
      expect(nativeResultTables(run)).toEqual([]);
    }
  });

  it("accepts only an exact selected-tail receipt bound to directional rows and the usable denominator", () => {
    const greater = completedConsistentPermutationRun();
    attachSelectedTail(greater.permutation!, greater.provenance!, "group_a_greater");
    expect(nativePlscConsistentPermutationProjection(greater)).toMatchObject({
      selectedTailInference: {
        method_version: NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION,
        orientation: NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_ORIENTATION,
        selected_test_tail: "group_a_greater",
      },
    });
    expect(nativePlscConsistentPermutationProjection(greater)?.selectedTailInference?.parameters[0])
      .toMatchObject({ selected_exceedances: 8, selected_p_value: 0.09, permutations: 99 });
    const selectedTables = nativeResultTables(greater);
    expect(selectedTables.find((table) => table.id === "plsc_permutation_selected_tail")?.rows).toEqual(expect.arrayContaining([
      ["Method", NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION],
      ["Orientation", "Group A minus Group B"],
      ["Selected test", "group_a_greater (Group A greater than Group B)"],
      ["Usable-assignment denominator", "99"],
    ]));
    expect(selectedTables.find((table) => table.id === "plsc_permutation_selected_tail_parameters")?.rows[0])
      .toEqual(expect.arrayContaining(["8", "0.0900", "99"]));
    const selectedProvenance = nativeRunProvenanceTable(greater).rows;
    expect(selectedProvenance).toContainEqual(["PLSc selected-tail method", NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION]);
    expect(selectedProvenance).toContainEqual(["PLSc selected-tail orientation", "Group A minus Group B"]);
    expect(selectedProvenance).toContainEqual(["PLSc selected test tail", "group_a_greater"]);
    expect(selectedProvenance).toContainEqual(["PLSc selected-tail usable denominator", "99"]);
    expect(selectedProvenance).toContainEqual(expect.arrayContaining([expect.stringContaining("PLSc selected exceedances"), "8"]));
    expect(selectedProvenance).toContainEqual(expect.arrayContaining([expect.stringContaining("PLSc selected p value"), "0.09"]));

    const less = completedConsistentPermutationRun();
    attachSelectedTail(less.permutation!, less.provenance!, "group_a_less");
    expect(nativePlscConsistentPermutationProjection(less)?.selectedTailInference?.parameters[0])
      .toMatchObject({ selected_exceedances: 91, selected_p_value: 0.92, permutations: 99 });
    expect(nativeResultTables(less).find((table) => table.id === "plsc_permutation_selected_tail")?.rows)
      .toContainEqual(["Selected test", "group_a_less (Group A less than Group B)"]);
    expect(nativeResultTables(less).find((table) => table.id === "plsc_permutation_selected_tail_parameters")?.rows[0])
      .toEqual(expect.arrayContaining(["91", "0.9200", "99"]));
    expect(nativeRunProvenanceTable(less).rows).toContainEqual(["PLSc selected test tail", "group_a_less"]);

    const cases: AnalysisRun[] = [];
    const addCase = (mutate: (receipt: NonNullable<PlsPermutationRun["selected_tail_inference"]>) => void) => {
      const run = completedConsistentPermutationRun();
      attachSelectedTail(run.permutation!, run.provenance!, "group_a_greater");
      mutate(run.permutation!.selected_tail_inference!);
      cases.push(run);
    };
    addCase((receipt) => { (receipt as { method_version: string }).method_version = "plsc_permutation_selected_tail_v0"; });
    addCase((receipt) => { (receipt as { orientation: string }).orientation = "group_b_minus_group_a"; });
    addCase((receipt) => { (receipt as { selected_test_tail: string }).selected_test_tail = "two_sided"; });
    addCase((receipt) => { receipt.parameters.reverse(); });
    addCase((receipt) => { receipt.parameters[0].selected_exceedances = 9; });
    addCase((receipt) => { receipt.parameters[0].selected_p_value = 0.1; });
    addCase((receipt) => { receipt.parameters[0].permutations = 98; });

    const missingMarker = structuredClone(greater);
    missingMarker.provenance!.method_version = missingMarker.provenance!.method_version
      .split("+")
      .filter((version) => version !== NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION)
      .join("+");
    const duplicateMarker = structuredClone(greater);
    duplicateMarker.provenance!.method_version += `+${NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION}`;
    const markerWithoutReceipt = completedConsistentPermutationRun();
    markerWithoutReceipt.provenance!.method_version += `+${NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION}`;
    cases.push(missingMarker, duplicateMarker, markerWithoutReceipt);

    for (const run of cases) {
      expect(nativePlscConsistentPermutationProjection(run)).toBeNull();
    }
  });

  it("keeps the exact historical two-sided v1 payload readable without directional inference", () => {
    const historical = structuredClone(completedConsistentPermutationRun());
    historical.permutation!.test_method = NATIVE_PLSC_CONSISTENT_PERMUTATION_TEST_V1;
    delete historical.permutation!.directional_inference;
    historical.permutation!.warnings![2] = NATIVE_PLSC_CONSISTENT_PERMUTATION_BOUNDED_SCOPE_WARNING_V1;

    expect(nativePlscConsistentPermutationProjection(historical)).not.toBeNull();
    const paths = nativeResultTables(historical).find((table) => table.id === "plsc_permutation_paths");
    expect(paths?.columns).toEqual(["Parameter", "Group A", "Group B", "Difference A − B", "p (two-tailed)", "Usable assignments"]);
    expect(nativeRunProvenanceTable(historical).rows).toContainEqual([
      "PLSc permutation limitation",
      "MICOM and one-tailed inference are unavailable in this internal v1 result",
    ]);
  });

  it("reopens only with the exact typed recipe and preserves the consistent-permutation label", () => {
    const { envelope, recipe } = canonicalConsistentPermutationFixture();
    const hydrated = nativeRunFromCanonicalResult(envelope, recipe);
    expect(hydrated).toMatchObject({
      method: "PLSc Consistent Permutation",
      permutation: {
        method_version: "plsc_permutation_v1",
        test_method: NATIVE_PLSC_CONSISTENT_PERMUTATION_TEST,
        directional_inference: {
          method_version: NATIVE_PLSC_CONSISTENT_PERMUTATION_DIRECTIONAL_METHOD_VERSION,
        },
      },
    });
    expect(nativePlscConsistentPermutationProjection(hydrated)).not.toBeNull();

    const wrongGroup = structuredClone(recipe);
    if (wrongGroup.method_config?.kind !== "plsc_permutation") throw new Error("fixture drift");
    wrongGroup.method_config.group_a = "C";
    expect(nativeRunFromCanonicalResult(envelope, wrongGroup)).toBeNull();

    const tamperedEnvelope = structuredClone(envelope);
    const payload = tamperedEnvelope.payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v3" }>;
    payload.permutation!.permutation_ledger![0].permutation_index = 1;
    expect(nativeRunFromCanonicalResult(tamperedEnvelope, recipe)).toBeNull();

    const missingDirectional = structuredClone(envelope);
    const missingPayload = missingDirectional.payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v3" }>;
    delete missingPayload.permutation!.directional_inference;
    expect(nativeRunFromCanonicalResult(missingDirectional, recipe)).toBeNull();
  });

  it("requires the selected-tail receipt exactly when the typed recipe selects a one-sided test", () => {
    const { envelope, recipe } = canonicalConsistentPermutationFixture();
    if (recipe.method_config?.kind !== "plsc_permutation") throw new Error("fixture drift");
    recipe.method_config.test_tail = "group_a_greater";
    const payload = envelope.payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v3" }>;
    attachSelectedTail(payload.permutation!, envelope.provenance, "group_a_greater");

    const hydrated = nativeRunFromCanonicalResult(envelope, recipe);
    expect(hydrated?.permutation?.selected_tail_inference).toMatchObject({
      method_version: NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION,
      orientation: NATIVE_PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_ORIENTATION,
      selected_test_tail: "group_a_greater",
    });

    const defaultRecipe = structuredClone(recipe);
    if (defaultRecipe.method_config?.kind !== "plsc_permutation") throw new Error("fixture drift");
    delete defaultRecipe.method_config.test_tail;
    expect(nativeRunFromCanonicalResult(envelope, defaultRecipe)).toBeNull();

    const missingReceipt = structuredClone(envelope);
    const missingPayload = missingReceipt.payload as Extract<AnalysisResultEnvelope["payload"], { kind: "pls_pm_v3" }>;
    delete missingPayload.permutation!.selected_tail_inference;
    expect(nativeRunFromCanonicalResult(missingReceipt, recipe)).toBeNull();

    const wrongSelection = structuredClone(recipe);
    if (wrongSelection.method_config?.kind !== "plsc_permutation") throw new Error("fixture drift");
    wrongSelection.method_config.test_tail = "group_a_less";
    expect(nativeRunFromCanonicalResult(envelope, wrongSelection)).toBeNull();
  });
});
