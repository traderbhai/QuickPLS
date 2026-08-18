import type { ResultTable } from "../domain/resultTables";
import type {
  AnalysisResultEnvelope,
  AnalysisRun,
  NativeCanonicalAnalysisRecipe,
} from "../types";

export const NATIVE_STRUCTURAL_PATH_RANDOMIZATION_METHOD_VERSION = "freedman_lane_permutation_v1";
export const NATIVE_STRUCTURAL_PATH_RANDOMIZATION_OPERATION = "pls_pm_freedman_lane_v1";
export const NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING =
  "Supported for the documented bounded scope: single-model Freedman-Lane randomization holds the original PLS construct scores fixed and reports unadjusted pathwise two-sided plus-one p values. Interpret these as conditional, approximate inference under exchangeable reduced-model residuals. Measurement-score uncertainty is not re-estimated, no multiplicity adjustment is applied, and current calibration covers homoscedastic Gaussian errors only.";

const CURRENT_PLS_METHOD_VERSION = "pls_pm_v1";
const CURRENT_PLS_MEDIATION_METHOD_VERSION = "pls_mediation_v1";
const CURRENT_PLS_MODERATION_METHOD_VERSION = "pls_two_stage_moderation_v1";
const CURRENT_PLS_ASSESSMENT_METHOD_VERSION = "pls_assessment_v8";
const MINIMUM_PERMUTATIONS = 99;
const MAXIMUM_PERMUTATIONS = 10_000;

export interface NativeStructuralPathRandomizationParameter {
  parameter: string;
  source: string;
  target: string;
  original: number;
  exceedances: number;
  permutations: number;
  pValueTwoSided: number;
}

export interface NativeStructuralPathRandomizationProjection {
  methodVersion: typeof NATIVE_STRUCTURAL_PATH_RANDOMIZATION_METHOD_VERSION;
  operation: typeof NATIVE_STRUCTURAL_PATH_RANDOMIZATION_OPERATION;
  permutations: number;
  masterSeed: number;
  parameters: NativeStructuralPathRandomizationParameter[];
}

/**
 * Projects only the current structural-score randomization contract. A partial,
 * stale, or arithmetically inconsistent artifact is never filtered into a
 * plausible-looking table: the entire projection becomes unavailable.
 */
export function nativeStructuralPathRandomizationProjection(
  run: AnalysisRun | null | undefined,
): NativeStructuralPathRandomizationProjection | null {
  if (!run || run.status !== "completed" || !run.result || !run.permutation || !run.provenance) return null;
  const { result, permutation, provenance } = run;
  if (!isRecord(result) || !isRecord(permutation) || !isRecord(provenance)) return null;
  const plan = permutation.plan;
  const settings = provenance.settings;
  const expectedPaths = result.paths;
  const rawParameters = permutation.parameters;
  const hasModeration = result.moderation != null;
  if (!isRecord(plan)
    || !isRecord(settings)
    || !Array.isArray(expectedPaths)
    || !Array.isArray(rawParameters)
    || (hasModeration && !isRecord(result.moderation))) return null;
  if (!hasExactKeys(permutation, ["method_version", "plan", "parameters"])
    || !hasExactKeys(plan, ["permutations", "master_seed", "operation"])) return null;
  const expectedMethodVersion = [
    CURRENT_PLS_METHOD_VERSION,
    CURRENT_PLS_MEDIATION_METHOD_VERSION,
    ...(hasModeration ? [CURRENT_PLS_MODERATION_METHOD_VERSION] : []),
    CURRENT_PLS_ASSESSMENT_METHOD_VERSION,
    NATIVE_STRUCTURAL_PATH_RANDOMIZATION_METHOD_VERSION,
  ].join("+");
  if (result.method_version !== CURRENT_PLS_METHOD_VERSION
    || provenance.method !== "pls_pm"
    || settings.method !== "pls_pm"
    || provenance.method_version !== expectedMethodVersion
    || permutation.method_version !== NATIVE_STRUCTURAL_PATH_RANDOMIZATION_METHOD_VERSION
    || plan.operation !== NATIVE_STRUCTURAL_PATH_RANDOMIZATION_OPERATION
    || !isIntegerInRange(plan.permutations, MINIMUM_PERMUTATIONS, MAXIMUM_PERMUTATIONS)
    || !isIntegerInRange(plan.master_seed, 0, 4_294_967_295)
    || provenance.seed !== plan.master_seed
    || run.seed !== plan.master_seed
    || settings.seed !== plan.master_seed
    || settings.bootstrap_samples !== 0
    || settings.studentized_inner_samples !== 0
    || settings.permutation_samples !== plan.permutations
    || run.bootstrap != null
    || !isIntegerInRange(settings.workers, 1, 64)) return null;

  if (!expectedPaths.length || expectedPaths.length !== rawParameters.length) return null;
  const seenPaths = new Set<string>();
  const parameters: NativeStructuralPathRandomizationParameter[] = [];
  for (let index = 0; index < expectedPaths.length; index += 1) {
    const path = expectedPaths[index];
    const parameter = rawParameters[index];
    if (!isRecord(path) || !isRecord(parameter)
      || !hasExactKeys(parameter, ["parameter", "original", "exceedances", "p_value_two_sided", "permutations"])
      || !hasText(path.source)
      || !hasText(path.target)
      || !Number.isFinite(path.coefficient)) return null;
    const identity = canonicalPathIdentity(path.source, path.target);
    if (seenPaths.has(identity) || parameter.parameter !== identity) return null;
    seenPaths.add(identity);
    if (!Number.isFinite(parameter.original)
      || !Object.is(parameter.original, path.coefficient)
      || !isIntegerInRange(parameter.exceedances, 0, plan.permutations)
      || parameter.permutations !== plan.permutations
      || !Number.isFinite(parameter.p_value_two_sided)
      || parameter.p_value_two_sided < 0
      || parameter.p_value_two_sided > 1) return null;
    const expectedProbability = (parameter.exceedances + 1) / (plan.permutations + 1);
    if (!Object.is(parameter.p_value_two_sided, expectedProbability)) return null;
    parameters.push({
      parameter: identity,
      source: path.source,
      target: path.target,
      original: parameter.original,
      exceedances: parameter.exceedances,
      permutations: parameter.permutations,
      pValueTwoSided: parameter.p_value_two_sided,
    });
  }

  return {
    methodVersion: NATIVE_STRUCTURAL_PATH_RANDOMIZATION_METHOD_VERSION,
    operation: NATIVE_STRUCTURAL_PATH_RANDOMIZATION_OPERATION,
    permutations: plan.permutations,
    masterSeed: plan.master_seed,
    parameters,
  };
}

/** Current schema-v3 recipe/schema-v1 result-envelope binding used during archive hydration. */
export function nativeStructuralPathRandomizationRecipeMatches(
  recipe: NativeCanonicalAnalysisRecipe,
  envelope: AnalysisResultEnvelope,
  projection: NativeStructuralPathRandomizationProjection,
): boolean {
  if (!isRecord(recipe)
    || !isRecord(envelope)
    || !isRecord(recipe.model)
    || !Array.isArray(recipe.model.constructs)
    || !Array.isArray(recipe.model.paths)
    || !recipe.model.constructs.every(isRecord)
    || !recipe.model.paths.every(isRecord)
    || !isRecord(recipe.settings)
    || !isRecord(envelope.provenance)
    || !isRecord(envelope.provenance.settings)
    || !isRecord(envelope.payload)) return false;
  const config = recipe.method_config;
  const recipeSettings = recipe.settings;
  const resultSettings = envelope.provenance.settings;
  const canonicalPaths = recipe.model.constructs.flatMap((construct) => (
    recipe.model.paths.filter((path) => path.target === construct.id)
  ));
  return recipe.schema_version === 3
    && isRecord(config)
    && config.kind === "pls_permutation"
    && Object.keys(config).length === 1
    && envelope.schema_version === 1
    && envelope.payload.kind === "pls_pm_v3"
    && envelope.payload.bootstrap == null
    && envelope.payload.permutation != null
    && envelope.provenance.recipe_id === recipe.id
    && envelope.provenance.dataset_fingerprint === recipe.dataset_fingerprint
    && recipeSettings.method === "pls_pm"
    && recipeSettings.bootstrap_samples === 0
    && recipeSettings.studentized_inner_samples === 0
    && recipeSettings.permutation_samples === projection.permutations
    && recipeSettings.seed === projection.masterSeed
    && isIntegerInRange(recipeSettings.workers, 1, 64)
    && canonicalPaths.length === projection.parameters.length
    && canonicalPaths.every((path, index) => (
      path.source === projection.parameters[index]?.source
      && path.target === projection.parameters[index]?.target
    ))
    && sameEngineSettings(recipeSettings, resultSettings);
}

export function nativeStructuralPathRandomizationTable(
  projection: NativeStructuralPathRandomizationProjection,
  constructLabel: (constructId: string) => string = (constructId) => constructId,
  excludedParameters: ReadonlySet<string> = new Set(),
): ResultTable {
  return {
    id: "permutation",
    title: "Structural path randomization",
    status: "validated",
    warning: NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING,
    columns: ["Path", "Original", "Exceedances", "Permutations", "Raw two-sided p"],
    rows: projection.parameters
      .filter((parameter) => !excludedParameters.has(parameter.parameter))
      .map((parameter) => [
        `${constructLabel(parameter.source)} -> ${constructLabel(parameter.target)}`,
        formatNumber(parameter.original),
        String(parameter.exceedances),
        String(parameter.permutations),
        String(parameter.pValueTwoSided),
      ]),
  };
}

export function isStructuralPathRandomizationIdentityPresent(
  recipe: NativeCanonicalAnalysisRecipe,
  envelope: AnalysisResultEnvelope,
): boolean {
  const config = isRecord(recipe?.method_config) ? recipe.method_config : null;
  const recipeSettings = isRecord(recipe?.settings) ? recipe.settings : null;
  const provenance = isRecord(envelope?.provenance) ? envelope.provenance : null;
  const payload = isRecord(envelope?.payload) ? envelope.payload : null;
  return config?.kind === "pls_permutation"
    || (typeof recipeSettings?.permutation_samples === "number" && recipeSettings.permutation_samples > 0)
    || (typeof provenance?.method_version === "string"
      && provenance.method_version.split("+").includes(NATIVE_STRUCTURAL_PATH_RANDOMIZATION_METHOD_VERSION))
    || (payload?.kind === "pls_pm_v3" && payload.permutation != null);
}

function canonicalPathIdentity(source: string, target: string): string {
  return JSON.stringify(["path", [source, target]]);
}

function hasText(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function isRecord(value: unknown): value is Record<string, any> {
  return value != null && typeof value === "object" && !Array.isArray(value);
}

function isIntegerInRange(value: unknown, minimum: number, maximum: number): value is number {
  return Number.isSafeInteger(value) && Number(value) >= minimum && Number(value) <= maximum;
}

function hasExactKeys(value: unknown, expectedKeys: readonly string[]): boolean {
  if (value == null || typeof value !== "object" || Array.isArray(value)) return false;
  const keys = Object.keys(value);
  return keys.length === expectedKeys.length && expectedKeys.every((key) => keys.includes(key));
}

function formatNumber(value: number): string {
  return value.toFixed(6).replace(/^-0\.000000$/, "0.000000");
}

function sameEngineSettings(
  left: NativeCanonicalAnalysisRecipe["settings"],
  right: NativeCanonicalAnalysisRecipe["settings"],
): boolean {
  return left.method === right.method
    && left.weighting_scheme === right.weighting_scheme
    && left.tolerance === right.tolerance
    && left.max_iterations === right.max_iterations
    && left.bootstrap_samples === right.bootstrap_samples
    && left.studentized_inner_samples === right.studentized_inner_samples
    && left.permutation_samples === right.permutation_samples
    && left.seed === right.seed
    && left.workers === right.workers
    && left.confidence_level === right.confidence_level
    && left.preprocessing === right.preprocessing
    && left.missing_data === right.missing_data
    && left.case_weight_column === right.case_weight_column;
}
