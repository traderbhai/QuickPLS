export const NATIVE_PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID = "qpls3.pls.sample_size_power" as const;
export const NATIVE_PLS_SAMPLE_SIZE_POWER_HISTORICAL_METHOD_VERSION = "pls_sample_size_power_v1" as const;
export const NATIVE_PLS_SAMPLE_SIZE_POWER_METHOD_VERSION = "pls_sample_size_power_v2" as const;
export const NATIVE_PLS_SAMPLE_SIZE_POWER_FAILURE_POLICY = "failed_replicates_count_as_non_rejections_v1" as const;
export const NATIVE_PLS_SAMPLE_SIZE_POWER_INTERVAL_METHOD = "wilson_score_two_sided_v1" as const;
export const NATIVE_PLS_SAMPLE_SIZE_POWER_HISTORICAL_INFERENCE = "case_bootstrap_normal_reference_two_sided" as const;
export const NATIVE_PLS_SAMPLE_SIZE_POWER_HISTORICAL_RESULT_INFERENCE = "pls_pm_case_bootstrap_normal_reference_two_sided_v1" as const;
export const NATIVE_PLS_SAMPLE_SIZE_POWER_INFERENCE = "case_bootstrap_null_centered_two_sided_plus_one" as const;
export const NATIVE_PLS_SAMPLE_SIZE_POWER_RESULT_INFERENCE = "pls_pm_case_bootstrap_null_centered_two_sided_plus_one_v2" as const;

export interface NativePlsSampleSizePowerDraft {
  scenarioIdentity: string;
  predictorConstruct: string;
  outcomeConstruct: string;
  predictorIndicatorLoadings: string;
  outcomeIndicatorLoadings: string;
  populationPath: string;
  exogenousDistribution: "standard_normal" | "";
  structuralDisturbanceDistribution: "standard_normal" | "";
  indicatorErrorDistribution: "standard_normal" | "";
  missingData: "none" | "";
  weightingScheme: "path" | "";
  preprocessing: "standardized" | "";
  tolerance: string;
  maxIterations: string;
  inference: typeof NATIVE_PLS_SAMPLE_SIZE_POWER_INFERENCE | "";
  sampleSizeGrid: string;
  alpha: string;
  targetPower: string;
  confidenceLevel: string;
  monteCarloReplicates: string;
  bootstrapReplicates: string;
  masterSeed: string;
  workers: string;
}

export interface NativePlsSampleSizePowerRecipeV1 {
  schema_version: 1;
  capability_id: typeof NATIVE_PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID;
  method_version: typeof NATIVE_PLS_SAMPLE_SIZE_POWER_HISTORICAL_METHOD_VERSION;
  scenario_identity: string;
  design: {
    predictor_construct: string;
    outcome_construct: string;
    predictor_indicator_loadings: number[];
    outcome_indicator_loadings: number[];
    population_path: number;
    exogenous_distribution: "standard_normal";
    structural_disturbance_distribution: "standard_normal";
    indicator_error_distribution: "standard_normal";
    missing_data: "none";
  };
  estimator: {
    weighting_scheme: "path";
    preprocessing: "standardized";
    tolerance: number;
    max_iterations: number;
  };
  inference: typeof NATIVE_PLS_SAMPLE_SIZE_POWER_HISTORICAL_INFERENCE;
  sample_size_grid: number[];
  alpha: number;
  target_power: number;
  confidence_level: number;
  monte_carlo_replicates: number;
  bootstrap_replicates: number;
  master_seed: number;
  workers: number;
}

export interface NativePlsSampleSizePowerRecipeV2
  extends Omit<NativePlsSampleSizePowerRecipeV1, "schema_version" | "method_version" | "inference"> {
  schema_version: 2;
  method_version: typeof NATIVE_PLS_SAMPLE_SIZE_POWER_METHOD_VERSION;
  inference: typeof NATIVE_PLS_SAMPLE_SIZE_POWER_INFERENCE;
}

export interface NativePlsSampleSizePowerWorkload {
  gridPoints: number;
  plannedDatasets: number;
  estimatedPlsFits: number;
  estimatedPlsCaseFits: number;
}

export interface NativePlsSampleSizePowerBuild {
  recipe: NativePlsSampleSizePowerRecipeV2;
  workload: NativePlsSampleSizePowerWorkload;
}

export class NativePlsSampleSizePowerBuildError extends Error {
  readonly field: keyof NativePlsSampleSizePowerDraft | "recipe";

  constructor(field: keyof NativePlsSampleSizePowerDraft | "recipe", message: string) {
    super(message);
    this.name = "NativePlsSampleSizePowerBuildError";
    this.field = field;
  }
}

export interface NativePlsSampleSizePowerOutcomeV1 {
  sample_size: number;
  replicate_index: number;
  stream_identity: string;
  attempted: boolean;
  successful: boolean;
  converged: boolean;
  target_estimate: number | null;
  p_value_two_sided: number | null;
  bootstrap_requested_replicates?: number | null;
  bootstrap_usable_replicates?: number | null;
  bootstrap_failed_replicates?: number | null;
  bootstrap_two_sided_exceedances?: number | null;
  rejected: boolean;
  failure_code: string | null;
  failure_message: string | null;
}

export interface NativePlsSampleSizePowerRowV1 {
  sample_size: number;
  requested_replicates: number;
  attempted_replicates: number;
  successful_replicates: number;
  failed_replicates: number;
  rejections: number;
  achieved_power: number;
  confidence_lower: number;
  confidence_upper: number;
  qualifies: boolean;
}

export type NativePlsSampleSizePowerDecisionV1 =
  | { status: "reached"; sample_size: number }
  | { status: "not_reached" };

export interface NativePlsSampleSizePowerResultV1 {
  schema_version: 1;
  capability_id: typeof NATIVE_PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID;
  method_version: typeof NATIVE_PLS_SAMPLE_SIZE_POWER_HISTORICAL_METHOD_VERSION;
  recipe_digest: string;
  stream_domain: string;
  failure_policy: typeof NATIVE_PLS_SAMPLE_SIZE_POWER_FAILURE_POLICY;
  interval_method: typeof NATIVE_PLS_SAMPLE_SIZE_POWER_INTERVAL_METHOD;
  inference_method: typeof NATIVE_PLS_SAMPLE_SIZE_POWER_HISTORICAL_RESULT_INFERENCE;
  pls_method_version: string;
  resampling_method_version: string;
  workload: {
    grid_points: number;
    planned_datasets: number;
    estimated_pls_fits: number;
    estimated_pls_case_fits: number;
  };
  rows: NativePlsSampleSizePowerRowV1[];
  outcomes: NativePlsSampleSizePowerOutcomeV1[];
  outcome_digest: string;
  decision: NativePlsSampleSizePowerDecisionV1;
  monotonicity_violations: number;
  warnings: string[];
  exclusions: string[];
}

export interface NativePlsSampleSizePowerResultV2
  extends Omit<NativePlsSampleSizePowerResultV1, "schema_version" | "method_version" | "inference_method"> {
  schema_version: 2;
  method_version: typeof NATIVE_PLS_SAMPLE_SIZE_POWER_METHOD_VERSION;
  inference_method: typeof NATIVE_PLS_SAMPLE_SIZE_POWER_RESULT_INFERENCE;
}

export interface NativePlsSampleSizePowerPresentation {
  title: string;
  decisionLabel: string;
  decisionTone: "success" | "warning";
  rows: Array<{
    sampleSize: string;
    achievedPower: string;
    interval: string;
    successful: string;
    failures: string;
    decision: string;
  }>;
  failureSummary: string;
  assumptions: string[];
  provenance: Array<[string, string]>;
  warnings: string[];
  exclusions: string[];
}

export interface NativePlsSampleSizePowerExportTable {
  name: "Power by sample size" | "Bootstrap tail accounting" | "Simulation failures" | "Design assumptions" | "Run provenance";
  columns: string[];
  rows: string[][];
}

const IDENTIFIER = /^[A-Za-z0-9_.-]{1,80}$/;

export function buildNativePlsSampleSizePowerRecipe(
  draft: Readonly<NativePlsSampleSizePowerDraft>,
): NativePlsSampleSizePowerBuild {
  const scenarioIdentity = identifier(draft.scenarioIdentity, "scenarioIdentity");
  const predictorConstruct = identifier(draft.predictorConstruct, "predictorConstruct");
  const outcomeConstruct = identifier(draft.outcomeConstruct, "outcomeConstruct");
  if (predictorConstruct === outcomeConstruct) {
    fail("outcomeConstruct", "Outcome construct must differ from the predictor construct.");
  }
  const predictorLoadings = loadingList(draft.predictorIndicatorLoadings, "predictorIndicatorLoadings");
  const outcomeLoadings = loadingList(draft.outcomeIndicatorLoadings, "outcomeIndicatorLoadings");
  const populationPath = boundedNumber(draft.populationPath, "populationPath", -0.8, 0.8);
  requiredLiteral(draft.exogenousDistribution, "standard_normal", "exogenousDistribution");
  requiredLiteral(draft.structuralDisturbanceDistribution, "standard_normal", "structuralDisturbanceDistribution");
  requiredLiteral(draft.indicatorErrorDistribution, "standard_normal", "indicatorErrorDistribution");
  requiredLiteral(draft.missingData, "none", "missingData");
  requiredLiteral(draft.weightingScheme, "path", "weightingScheme");
  requiredLiteral(draft.preprocessing, "standardized", "preprocessing");
  requiredLiteral(draft.inference, NATIVE_PLS_SAMPLE_SIZE_POWER_INFERENCE, "inference");
  const tolerance = boundedNumber(draft.tolerance, "tolerance", 1e-10, 1e-3);
  const maxIterations = boundedInteger(draft.maxIterations, "maxIterations", 100, 10_000);
  const sampleSizeGrid = integerList(draft.sampleSizeGrid, "sampleSizeGrid", 30, 5_000);
  if (sampleSizeGrid.length < 2 || sampleSizeGrid.length > 16) {
    fail("sampleSizeGrid", "Enter between 2 and 16 sample sizes.");
  }
  if (sampleSizeGrid.some((value, index) => index > 0 && value <= sampleSizeGrid[index - 1])) {
    fail("sampleSizeGrid", "Sample sizes must be unique and strictly increasing.");
  }
  const alpha = boundedNumber(draft.alpha, "alpha", 0.001, 0.1);
  const targetPower = boundedNumber(draft.targetPower, "targetPower", 0.5, 0.99);
  const confidenceLevel = boundedNumber(draft.confidenceLevel, "confidenceLevel", 0.8, 0.999);
  const monteCarloReplicates = boundedInteger(draft.monteCarloReplicates, "monteCarloReplicates", 100, 10_000);
  const bootstrapReplicates = boundedInteger(draft.bootstrapReplicates, "bootstrapReplicates", 99, 1_999);
  if (bootstrapReplicates % 2 === 0) {
    fail("bootstrapReplicates", "Bootstrap replicates must be an odd number.");
  }
  const masterSeed = boundedInteger(draft.masterSeed, "masterSeed", 0, Number.MAX_SAFE_INTEGER);
  const workers = boundedInteger(draft.workers, "workers", 1, 64);
  const [bestLower] = nativePlsPowerWilsonInterval(monteCarloReplicates, monteCarloReplicates, confidenceLevel);
  if (bestLower + Number.EPSILON < targetPower) {
    fail(
      "monteCarloReplicates",
      "This replicate count cannot make the Wilson lower confidence bound reach the requested target power.",
    );
  }
  const fitsPerDataset = 1 + bootstrapReplicates;
  const estimatedPlsFits = sampleSizeGrid.length * monteCarloReplicates * fitsPerDataset;
  const estimatedPlsCaseFits = sampleSizeGrid.reduce(
    (total, sampleSize) => total + sampleSize * monteCarloReplicates * fitsPerDataset,
    0,
  );
  if (!Number.isSafeInteger(estimatedPlsFits) || estimatedPlsFits > 250_000) {
    fail("sampleSizeGrid", "The plan exceeds the 250,000-fit desktop execution limit.");
  }
  if (!Number.isSafeInteger(estimatedPlsCaseFits) || estimatedPlsCaseFits > 100_000_000) {
    fail("sampleSizeGrid", "The plan exceeds the 100,000,000-row desktop execution limit.");
  }
  const recipe: NativePlsSampleSizePowerRecipeV2 = {
    schema_version: 2,
    capability_id: NATIVE_PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID,
    method_version: NATIVE_PLS_SAMPLE_SIZE_POWER_METHOD_VERSION,
    scenario_identity: scenarioIdentity,
    design: {
      predictor_construct: predictorConstruct,
      outcome_construct: outcomeConstruct,
      predictor_indicator_loadings: predictorLoadings,
      outcome_indicator_loadings: outcomeLoadings,
      population_path: populationPath,
      exogenous_distribution: "standard_normal",
      structural_disturbance_distribution: "standard_normal",
      indicator_error_distribution: "standard_normal",
      missing_data: "none",
    },
    estimator: {
      weighting_scheme: "path",
      preprocessing: "standardized",
      tolerance,
      max_iterations: maxIterations,
    },
    inference: NATIVE_PLS_SAMPLE_SIZE_POWER_INFERENCE,
    sample_size_grid: sampleSizeGrid,
    alpha,
    target_power: targetPower,
    confidence_level: confidenceLevel,
    monte_carlo_replicates: monteCarloReplicates,
    bootstrap_replicates: bootstrapReplicates,
    master_seed: masterSeed,
    workers,
  };
  return {
    recipe,
    workload: {
      gridPoints: sampleSizeGrid.length,
      plannedDatasets: sampleSizeGrid.length * monteCarloReplicates,
      estimatedPlsFits,
      estimatedPlsCaseFits,
    },
  };
}

export function nativePlsSampleSizePowerRecipeFromCanonical(
  config: Extract<NativeAnalysisMethodConfig, { kind: "pls_sample_size_power" }>,
  settings: Readonly<AnalysisEngineSettingsSnapshot>,
): NativePlsSampleSizePowerRecipeV1 | NativePlsSampleSizePowerRecipeV2 {
  const current = buildNativePlsSampleSizePowerRecipe({
    scenarioIdentity: config.scenario_identity,
    predictorConstruct: config.predictor_construct,
    outcomeConstruct: config.outcome_construct,
    predictorIndicatorLoadings: config.predictor_indicator_loadings.join(","),
    outcomeIndicatorLoadings: config.outcome_indicator_loadings.join(","),
    populationPath: String(config.population_path),
    exogenousDistribution: config.exogenous_distribution,
    structuralDisturbanceDistribution: config.structural_disturbance_distribution,
    indicatorErrorDistribution: config.indicator_error_distribution,
    missingData: config.missing_data,
    weightingScheme: settings.weighting_scheme === "path" ? "path" : "",
    preprocessing: settings.preprocessing === "standardized" ? "standardized" : "",
    tolerance: String(settings.tolerance),
    maxIterations: String(settings.max_iterations),
    inference: NATIVE_PLS_SAMPLE_SIZE_POWER_INFERENCE,
    sampleSizeGrid: config.sample_size_grid.join(","),
    alpha: String(config.alpha),
    targetPower: String(config.target_power),
    confidenceLevel: String(config.interval_confidence_level),
    monteCarloReplicates: String(config.monte_carlo_replicates),
    bootstrapReplicates: String(config.bootstrap_replicates),
    masterSeed: String(settings.seed),
    workers: String(settings.workers),
  }).recipe;
  if (config.inference === NATIVE_PLS_SAMPLE_SIZE_POWER_INFERENCE) return current;
  if (config.inference === NATIVE_PLS_SAMPLE_SIZE_POWER_HISTORICAL_INFERENCE) {
    return {
      ...current,
      schema_version: 1,
      method_version: NATIVE_PLS_SAMPLE_SIZE_POWER_HISTORICAL_METHOD_VERSION,
      inference: NATIVE_PLS_SAMPLE_SIZE_POWER_HISTORICAL_INFERENCE,
    };
  }
  fail("recipe", "The PLS power inference identity is unsupported.");
}

export function validateNativePlsSampleSizePowerResult(
  recipe: Readonly<NativePlsSampleSizePowerRecipeV1 | NativePlsSampleSizePowerRecipeV2>,
  result: Readonly<NativePlsSampleSizePowerResultV1 | NativePlsSampleSizePowerResultV2>,
): void {
  const isV2 = recipe.schema_version === 2;
  const expectedMethod = isV2
    ? NATIVE_PLS_SAMPLE_SIZE_POWER_METHOD_VERSION
    : NATIVE_PLS_SAMPLE_SIZE_POWER_HISTORICAL_METHOD_VERSION;
  const expectedInference = isV2
    ? NATIVE_PLS_SAMPLE_SIZE_POWER_RESULT_INFERENCE
    : NATIVE_PLS_SAMPLE_SIZE_POWER_HISTORICAL_RESULT_INFERENCE;
  const recipeInference = isV2
    ? NATIVE_PLS_SAMPLE_SIZE_POWER_INFERENCE
    : NATIVE_PLS_SAMPLE_SIZE_POWER_HISTORICAL_INFERENCE;
  if (
    result.schema_version !== recipe.schema_version
    || result.capability_id !== NATIVE_PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID
    || result.method_version !== expectedMethod
    || recipe.method_version !== expectedMethod
    || recipe.inference !== recipeInference
    || result.failure_policy !== NATIVE_PLS_SAMPLE_SIZE_POWER_FAILURE_POLICY
    || result.interval_method !== NATIVE_PLS_SAMPLE_SIZE_POWER_INTERVAL_METHOD
    || result.inference_method !== expectedInference
    || !/^[a-f0-9]{64}$/.test(result.recipe_digest)
    || !/^[a-f0-9]{64}$/.test(result.outcome_digest)
  ) {
    resultFailure("Stable result identity or digest is invalid.");
  }
  if (result.rows.length !== recipe.sample_size_grid.length) {
    resultFailure("Power-row count differs from the recipe grid.");
  }
  const expectedOutcomes = recipe.sample_size_grid.length * recipe.monte_carlo_replicates;
  if (result.outcomes.length !== expectedOutcomes) {
    resultFailure("Replicate ledger length differs from the declared workload.");
  }
  const expectedFits = expectedOutcomes * (1 + recipe.bootstrap_replicates);
  const expectedCaseFits = recipe.sample_size_grid.reduce(
    (total, sampleSize) => total + sampleSize * recipe.monte_carlo_replicates * (1 + recipe.bootstrap_replicates),
    0,
  );
  if (
    result.workload.grid_points !== recipe.sample_size_grid.length
    || result.workload.planned_datasets !== expectedOutcomes
    || result.workload.estimated_pls_fits !== expectedFits
    || result.workload.estimated_pls_case_fits !== expectedCaseFits
  ) {
    resultFailure("Stored workload differs from the recorded prospective recipe.");
  }
  const recomputed = recipe.sample_size_grid.map((sampleSize, gridIndex) => {
    const start = gridIndex * recipe.monte_carlo_replicates;
    const outcomes = result.outcomes.slice(start, start + recipe.monte_carlo_replicates);
    if (outcomes.some((outcome, replicateIndex) => (
      outcome.sample_size !== sampleSize
      || outcome.replicate_index !== replicateIndex
      || !/^[a-f0-9]{64}$/.test(outcome.stream_identity)
      || !outcome.attempted
      || outcome.successful !== (outcome.failure_code === null && outcome.failure_message === null)
      || outcome.rejected !== (outcome.p_value_two_sided !== null && outcome.p_value_two_sided <= recipe.alpha)
      || !nativePlsPowerTailAccountingMatches(outcome, recipe.bootstrap_replicates, isV2)
      || (outcome.successful && (!outcome.converged || !finite(outcome.target_estimate) || !probability(outcome.p_value_two_sided)))
      || (!outcome.successful && (
        outcome.converged
        || outcome.target_estimate !== null
        || outcome.p_value_two_sided !== null
        || outcome.rejected
      ))
    ))) {
      resultFailure(`Replicate ledger for sample size ${sampleSize} is inconsistent.`);
    }
    const successful = outcomes.filter((outcome) => outcome.successful).length;
    const rejections = outcomes.filter((outcome) => outcome.rejected).length;
    const [lower, upper] = nativePlsPowerWilsonInterval(rejections, recipe.monte_carlo_replicates, recipe.confidence_level);
    const stored = result.rows[gridIndex];
    const qualifies = stored.confidence_lower >= recipe.target_power;
    return {
      sample_size: sampleSize,
      requested_replicates: recipe.monte_carlo_replicates,
      attempted_replicates: outcomes.length,
      successful_replicates: successful,
      failed_replicates: recipe.monte_carlo_replicates - successful,
      rejections,
      achieved_power: rejections / recipe.monte_carlo_replicates,
      confidence_lower: lower,
      confidence_upper: upper,
      qualifies,
    } satisfies NativePlsSampleSizePowerRowV1;
  });
  recomputed.forEach((row, index) => {
    const stored = result.rows[index];
    const exactFields = [
      "sample_size",
      "requested_replicates",
      "attempted_replicates",
      "successful_replicates",
      "failed_replicates",
      "rejections",
    ] as const;
    const exactCountsMatch = exactFields.every((field) => row[field] === stored[field]);
    // Rust uses statrs' inverse normal CDF while this presentation validator uses
    // Acklam's approximation. Their Wilson bounds differ by roughly 1e-11 at the
    // supported settings, so compare those two derived display values with an
    // explicit cross-runtime tolerance. Counts, power, qualification, and the
    // final decision remain exact and are independently archive-validated in Rust.
    const derivedNumbersMatch = Math.abs(row.achieved_power - stored.achieved_power) <= 1e-12
      && Math.abs(row.confidence_lower - stored.confidence_lower) <= 1e-9
      && Math.abs(row.confidence_upper - stored.confidence_upper) <= 1e-9;
    if (
      !exactCountsMatch
      || !derivedNumbersMatch
      || stored.qualifies !== row.qualifies
    ) {
      resultFailure(`Stored power row ${index} does not reproduce from the replicate ledger.`);
    }
  });
  const firstQualified = recomputed.find((row) => row.qualifies)?.sample_size ?? null;
  if (
    (firstQualified === null && result.decision.status !== "not_reached")
    || (firstQualified !== null && (
      result.decision.status !== "reached"
      || result.decision.sample_size !== firstQualified
    ))
  ) {
    resultFailure("Conservative grid decision does not reproduce from Wilson lower bounds.");
  }
}

function nativePlsPowerTailAccountingMatches(
  outcome: Readonly<NativePlsSampleSizePowerOutcomeV1>,
  expectedRequested: number,
  isV2: boolean,
): boolean {
  const values = [
    outcome.bootstrap_requested_replicates,
    outcome.bootstrap_usable_replicates,
    outcome.bootstrap_failed_replicates,
    outcome.bootstrap_two_sided_exceedances,
  ];
  if (!outcome.successful || !isV2) return values.every((value) => value == null);
  const [requested, usable, failed, exceedances] = values;
  if (
    !Number.isInteger(requested)
    || !Number.isInteger(usable)
    || !Number.isInteger(failed)
    || !Number.isInteger(exceedances)
    || requested !== expectedRequested
    || (usable as number) + (failed as number) !== requested
    || (usable as number) < Math.max(2, Math.ceil(expectedRequested * 0.9))
    || (exceedances as number) < 0
    || (exceedances as number) > (usable as number)
  ) return false;
  const expectedProbability = ((exceedances as number) + 1) / ((usable as number) + 1);
  return outcome.p_value_two_sided === expectedProbability;
}

export function nativePlsSampleSizePowerPresentation(
  recipe: Readonly<NativePlsSampleSizePowerRecipeV1 | NativePlsSampleSizePowerRecipeV2>,
  result: Readonly<NativePlsSampleSizePowerResultV1 | NativePlsSampleSizePowerResultV2>,
): NativePlsSampleSizePowerPresentation {
  validateNativePlsSampleSizePowerResult(recipe, result);
  const failed = result.rows.reduce((total, row) => total + row.failed_replicates, 0);
  const requested = result.rows.reduce((total, row) => total + row.requested_replicates, 0);
  const decisionLabel = result.decision.status === "reached"
    ? `Conservative minimum on the evaluated grid: n = ${result.decision.sample_size.toLocaleString()}`
    : "Target power was not reached on the evaluated grid";
  return {
    title: "PLS-SEM sample-size and power analysis",
    decisionLabel,
    decisionTone: result.decision.status === "reached" ? "success" : "warning",
    rows: result.rows.map((row) => ({
      sampleSize: row.sample_size.toLocaleString(),
      achievedPower: formatProbability(row.achieved_power),
      interval: `${formatProbability(row.confidence_lower)} to ${formatProbability(row.confidence_upper)}`,
      successful: `${row.successful_replicates.toLocaleString()} / ${row.requested_replicates.toLocaleString()}`,
      failures: row.failed_replicates.toLocaleString(),
      decision: row.qualifies ? "Lower bound meets target" : "Below target",
    })),
    failureSummary: `${failed.toLocaleString()} of ${requested.toLocaleString()} planned replicates failed and remained in the denominator.`,
    assumptions: [
      `Target path: ${recipe.design.predictor_construct} → ${recipe.design.outcome_construct}`,
      `Population path: ${recipe.design.population_path.toFixed(4)}`,
      `Predictor loadings: ${recipe.design.predictor_indicator_loadings.map((value) => value.toFixed(4)).join(", ")}`,
      `Outcome loadings: ${recipe.design.outcome_indicator_loadings.map((value) => value.toFixed(4)).join(", ")}`,
      "Latent variables, structural disturbance, and indicator errors: standard normal; missing data: none",
      recipe.schema_version === 2
        ? `Null-centered two-sided case-bootstrap plus-one test, α = ${recipe.alpha.toFixed(4)}, ${recipe.bootstrap_replicates.toLocaleString()} indexed bootstrap replicates`
        : `Historical two-sided case-bootstrap normal-reference test, α = ${recipe.alpha.toFixed(4)}, ${recipe.bootstrap_replicates.toLocaleString()} bootstrap replicates`,
    ],
    provenance: [
      ["Capability", result.capability_id],
      ["Method version", result.method_version],
      ["Recipe digest", result.recipe_digest],
      ["Outcome digest", result.outcome_digest],
      ["Stream domain", result.stream_domain],
      ["Failure policy", result.failure_policy],
      ["Interval method", result.interval_method],
      ["PLS method", result.pls_method_version],
      ["Resampling method", result.resampling_method_version],
    ],
    warnings: [...result.warnings],
    exclusions: [...result.exclusions],
  };
}

export function nativePlsSampleSizePowerExportTables(
  recipe: Readonly<NativePlsSampleSizePowerRecipeV1 | NativePlsSampleSizePowerRecipeV2>,
  result: Readonly<NativePlsSampleSizePowerResultV1 | NativePlsSampleSizePowerResultV2>,
): NativePlsSampleSizePowerExportTable[] {
  validateNativePlsSampleSizePowerResult(recipe, result);
  const decision = result.decision.status === "reached"
    ? String(result.decision.sample_size)
    : "not_reached_on_evaluated_grid";
  const tables: NativePlsSampleSizePowerExportTable[] = [
    {
      name: "Power by sample size",
      columns: [
        "sample_size", "requested_replicates", "attempted_replicates", "successful_replicates",
        "failed_replicates", "rejections", "achieved_power", "confidence_lower", "confidence_upper", "qualifies",
      ],
      rows: result.rows.map((row) => [
        String(row.sample_size), String(row.requested_replicates), String(row.attempted_replicates),
        String(row.successful_replicates), String(row.failed_replicates), String(row.rejections),
        fixed(row.achieved_power), fixed(row.confidence_lower), fixed(row.confidence_upper), String(row.qualifies),
      ]),
    },
    ...(result.schema_version === 2 ? [{
      name: "Bootstrap tail accounting" as const,
      columns: [
        "sample_size", "successful_outer_replicates", "bootstrap_requested_total",
        "bootstrap_usable_total", "bootstrap_failed_total", "two_sided_exceedances_total",
        "minimum_usable_per_outer_replicate", "maximum_usable_per_outer_replicate",
      ],
      rows: result.rows.map((summary) => {
        const outcomes = result.outcomes.filter((outcome) =>
          outcome.sample_size === summary.sample_size && outcome.successful,
        );
        const requested = outcomes.map((outcome) => outcome.bootstrap_requested_replicates!);
        const usable = outcomes.map((outcome) => outcome.bootstrap_usable_replicates!);
        const failed = outcomes.map((outcome) => outcome.bootstrap_failed_replicates!);
        const exceedances = outcomes.map((outcome) => outcome.bootstrap_two_sided_exceedances!);
        return [
          String(summary.sample_size),
          String(outcomes.length),
          String(requested.reduce((total, value) => total + value, 0)),
          String(usable.reduce((total, value) => total + value, 0)),
          String(failed.reduce((total, value) => total + value, 0)),
          String(exceedances.reduce((total, value) => total + value, 0)),
          usable.length ? String(Math.min(...usable)) : "",
          usable.length ? String(Math.max(...usable)) : "",
        ];
      }),
    }] : []),
    {
      name: "Simulation failures",
      columns: ["sample_size", "replicate_index", "stream_identity", "failure_code", "failure_message"],
      rows: result.outcomes.filter((outcome) => !outcome.successful).map((outcome) => [
        String(outcome.sample_size), String(outcome.replicate_index), outcome.stream_identity,
        outcome.failure_code ?? "", outcome.failure_message ?? "",
      ]),
    },
    {
      name: "Design assumptions",
      columns: ["assumption", "value"],
      rows: [
        ["scenario_identity", recipe.scenario_identity],
        ["target_path", `${recipe.design.predictor_construct} -> ${recipe.design.outcome_construct}`],
        ["population_path", fixed(recipe.design.population_path)],
        ["predictor_indicator_loadings", recipe.design.predictor_indicator_loadings.map(fixed).join(";")],
        ["outcome_indicator_loadings", recipe.design.outcome_indicator_loadings.map(fixed).join(";")],
        ["distribution", "standard_normal"],
        ["missing_data", "none"],
        ["alpha", fixed(recipe.alpha)],
        ["target_power", fixed(recipe.target_power)],
        ["confidence_level", fixed(recipe.confidence_level)],
        ["monte_carlo_replicates", String(recipe.monte_carlo_replicates)],
        ["bootstrap_replicates", String(recipe.bootstrap_replicates)],
      ],
    },
    {
      name: "Run provenance",
      columns: ["field", "value"],
      rows: [
        ["capability_id", result.capability_id],
        ["method_version", result.method_version],
        ["recipe_digest", result.recipe_digest],
        ["outcome_digest", result.outcome_digest],
        ["stream_domain", result.stream_domain],
        ["failure_policy", result.failure_policy],
        ["interval_method", result.interval_method],
        ["inference_method", result.inference_method],
        ["pls_method_version", result.pls_method_version],
        ["resampling_method_version", result.resampling_method_version],
        ["grid_decision", decision],
      ],
    },
  ];
  return tables;
}

export function nativePlsPowerWilsonInterval(
  successes: number,
  trials: number,
  confidenceLevel: number,
): [number, number] {
  if (
    !Number.isInteger(successes)
    || !Number.isInteger(trials)
    || trials <= 0
    || successes < 0
    || successes > trials
    || !Number.isFinite(confidenceLevel)
    || confidenceLevel <= 0
    || confidenceLevel >= 1
  ) {
    throw new Error("Invalid Wilson interval inputs.");
  }
  const proportion = successes / trials;
  const z = inverseStandardNormal(1 - (1 - confidenceLevel) / 2);
  const squared = z * z;
  const denominator = 1 + squared / trials;
  const center = (proportion + squared / (2 * trials)) / denominator;
  const halfWidth = z * Math.sqrt(
    proportion * (1 - proportion) / trials + squared / (4 * trials * trials),
  ) / denominator;
  return [Math.max(0, center - halfWidth), Math.min(1, center + halfWidth)];
}

function loadingList(value: string, field: keyof NativePlsSampleSizePowerDraft): number[] {
  const result = numberList(value, field);
  if (result.length < 3 || result.length > 10 || result.some((loading) => loading < 0.5 || loading > 0.95)) {
    fail(field, "Enter 3 to 10 finite loadings from 0.50 through 0.95.");
  }
  return result;
}

function integerList(
  value: string,
  field: keyof NativePlsSampleSizePowerDraft,
  minimum: number,
  maximum: number,
): number[] {
  const result = numberList(value, field);
  if (result.some((item) => !Number.isInteger(item) || item < minimum || item > maximum)) {
    fail(field, `Every value must be an integer from ${minimum} through ${maximum}.`);
  }
  return result;
}

function numberList(value: string, field: keyof NativePlsSampleSizePowerDraft): number[] {
  const tokens = value.split(/[;,\s]+/).filter(Boolean);
  if (tokens.length === 0) fail(field, "Enter at least one numeric value.");
  const result = tokens.map(Number);
  if (result.some((item) => !Number.isFinite(item))) fail(field, "Every value must be finite and numeric.");
  return result;
}

function identifier(value: string, field: keyof NativePlsSampleSizePowerDraft): string {
  const candidate = value.trim();
  if (!IDENTIFIER.test(candidate)) {
    fail(field, "Use 1 to 80 ASCII letters, digits, dots, underscores, or hyphens.");
  }
  return candidate;
}

function boundedNumber(
  value: string,
  field: keyof NativePlsSampleSizePowerDraft,
  minimum: number,
  maximum: number,
): number {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed < minimum || parsed > maximum) {
    fail(field, `Enter a finite value from ${minimum} through ${maximum}.`);
  }
  return parsed;
}

function boundedInteger(
  value: string,
  field: keyof NativePlsSampleSizePowerDraft,
  minimum: number,
  maximum: number,
): number {
  const parsed = boundedNumber(value, field, minimum, maximum);
  if (!Number.isSafeInteger(parsed)) fail(field, "Enter a whole number.");
  return parsed;
}

function requiredLiteral<T extends string>(
  value: T | "",
  expected: T,
  field: keyof NativePlsSampleSizePowerDraft,
): void {
  if (value !== expected) fail(field, `Select the supported ${expected} option.`);
}

function finite(value: number | null): value is number {
  return value !== null && Number.isFinite(value);
}

function probability(value: number | null): value is number {
  return finite(value) && value >= 0 && value <= 1;
}

function fixed(value: number): string {
  return value.toFixed(12);
}

function formatProbability(value: number): string {
  return value.toFixed(4);
}

function fail(field: keyof NativePlsSampleSizePowerDraft | "recipe", message: string): never {
  throw new NativePlsSampleSizePowerBuildError(field, message);
}

function resultFailure(message: string): never {
  throw new NativePlsSampleSizePowerBuildError("recipe", message);
}

// Acklam's rational inverse-normal approximation, accurate well beyond the
// precision required for Wilson intervals at the supported confidence levels.
function inverseStandardNormal(probabilityValue: number): number {
  if (!(probabilityValue > 0 && probabilityValue < 1)) throw new Error("Probability must be inside (0, 1).");
  const a = [-3.969683028665376e1, 2.209460984245205e2, -2.759285104469687e2, 1.38357751867269e2, -3.066479806614716e1, 2.506628277459239];
  const b = [-5.447609879822406e1, 1.615858368580409e2, -1.556989798598866e2, 6.680131188771972e1, -1.328068155288572e1];
  const c = [-7.784894002430293e-3, -3.223964580411365e-1, -2.400758277161838, -2.549732539343734, 4.374664141464968, 2.938163982698783];
  const d = [7.784695709041462e-3, 3.224671290700398e-1, 2.445134137142996, 3.754408661907416];
  const lower = 0.02425;
  const upper = 1 - lower;
  if (probabilityValue < lower) {
    const q = Math.sqrt(-2 * Math.log(probabilityValue));
    return (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
      / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1);
  }
  if (probabilityValue <= upper) {
    const q = probabilityValue - 0.5;
    const r = q * q;
    return (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q
      / (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1);
  }
  const q = Math.sqrt(-2 * Math.log(1 - probabilityValue));
  return -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5])
    / ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1);
}
import type { AnalysisEngineSettingsSnapshot, NativeAnalysisMethodConfig } from "../types";
