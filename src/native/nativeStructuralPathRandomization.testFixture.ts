import { completedSamplePlsRun } from "../data/smokeRun";
import type {
  AnalysisEngineSettingsSnapshot,
  AnalysisResultEnvelope,
  AnalysisRun,
  NativeCanonicalAnalysisRecipe,
} from "../types";
import {
  NATIVE_STRUCTURAL_PATH_RANDOMIZATION_METHOD_VERSION,
  NATIVE_STRUCTURAL_PATH_RANDOMIZATION_OPERATION,
} from "./nativeStructuralPathRandomization";

const SEED = 7;
const PERMUTATIONS = 999;
const DATASET_FINGERPRINT = "sha256:structural-path-randomization-test-fixture";

function settings(): AnalysisEngineSettingsSnapshot {
  return {
    method: "pls_pm",
    weighting_scheme: "path",
    tolerance: 1e-7,
    max_iterations: 3000,
    bootstrap_samples: 0,
    studentized_inner_samples: 0,
    permutation_samples: PERMUTATIONS,
    seed: SEED,
    workers: 2,
    confidence_level: 0.95,
    preprocessing: "standardized",
    missing_data: "listwise_deletion",
    case_weight_column: null,
  };
}

export function completedStructuralPathRandomizationRun(): AnalysisRun {
  const base = completedSamplePlsRun();
  const result = { ...base.result!, method_version: "pls_pm_v1" };
  const engineSettings = settings();
  return {
    ...base,
    id: "structural-path-randomization-result",
    name: "Structural Path Randomization run",
    method: "Structural Path Randomization",
    seed: SEED,
    warnings: [],
    fingerprint: DATASET_FINGERPRINT.slice(0, 12),
    result,
    bootstrap: undefined,
    permutation: {
      method_version: NATIVE_STRUCTURAL_PATH_RANDOMIZATION_METHOD_VERSION,
      plan: {
        permutations: PERMUTATIONS,
        master_seed: SEED,
        operation: NATIVE_STRUCTURAL_PATH_RANDOMIZATION_OPERATION,
      },
      parameters: result.paths.map((path, index) => {
        const exceedances = 9 + index;
        return {
          parameter: JSON.stringify(["path", [path.source, path.target]]),
          original: path.coefficient,
          exceedances,
          p_value_two_sided: (exceedances + 1) / (PERMUTATIONS + 1),
          permutations: PERMUTATIONS,
        };
      }),
    },
    provenance: {
      recipe_id: "structural-path-randomization-recipe",
      dataset_fingerprint: DATASET_FINGERPRINT,
      method: "pls_pm",
      method_version: `pls_pm_v1+pls_mediation_v1+pls_assessment_v7+${NATIVE_STRUCTURAL_PATH_RANDOMIZATION_METHOD_VERSION}`,
      engine_version: "test",
      seed: SEED,
      settings: engineSettings,
      started_at: "2026-08-13T00:00:00.000Z",
      completed_at: "2026-08-13T00:00:01.000Z",
    },
  };
}

export function structuralPathRandomizationCanonicalFixture(): {
  recipe: NativeCanonicalAnalysisRecipe;
  envelope: AnalysisResultEnvelope;
} {
  const run = completedStructuralPathRandomizationRun();
  const result = run.result!;
  const provenance = run.provenance!;
  const constructIds = [...new Set(result.paths.flatMap((path) => [path.source, path.target]))];
  const recipe: NativeCanonicalAnalysisRecipe = {
    schema_version: 3,
    id: provenance.recipe_id,
    created_at: provenance.started_at,
    dataset_fingerprint: provenance.dataset_fingerprint,
    model: {
      id: "structural-path-randomization-model",
      name: "Structural path randomization model",
      constructs: constructIds.map((id) => ({
        id,
        name: id,
        short_name: id,
        mode: "reflective",
        indicators: [],
      })),
      paths: result.paths.map((path) => ({ source: path.source, target: path.target })),
      controls: [],
      higher_order_constructs: [],
      interactions: [],
    },
    settings: { ...provenance.settings },
    method_config: { kind: "pls_permutation" },
    metadata: {},
  };
  const envelope: AnalysisResultEnvelope = {
    schema_version: 1,
    id: run.id,
    status: "completed",
    provenance,
    diagnostics: [],
    payload: {
      kind: "pls_pm_v3",
      estimation: result,
      assessment: run.assessment!,
      bootstrap: null,
      permutation: run.permutation!,
    },
  };
  return { recipe, envelope };
}
