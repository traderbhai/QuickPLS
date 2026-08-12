import type { AnalysisUiSettings } from "../types";

export type NativeCalculationMode = "pls" | "bootstrap" | "permutation" | "predict";

export const NATIVE_PREDICTION_METHOD_LABEL = "PLSpredict / CVPAT";
export const NATIVE_LEGACY_PREDICTION_METHOD_LABEL = "Legacy construct-score prediction (v1)";
export const NATIVE_PREDICTION_SCOPE_DESCRIPTION = "Assess endogenous-indicator prediction with fixed seeded 10-fold × 10-repeat cross-validation and IA/LM benchmark tests.";
export const NATIVE_PREDICTION_PLAN_DESCRIPTION = "Complete cases; seeded balanced 10-fold × 10-repeat cross-validation; deterministic modulo-4 holdout retained as a secondary check";
export const NATIVE_PREDICTION_TARGET_DESCRIPTION = "Endogenous indicators are primary; construct-score metrics are supplementary";
export const NATIVE_PREDICTION_BENCHMARK_DESCRIPTION = "Indicator average (IA) and Linear model (LM, where estimable)";
export const NATIVE_PREDICTION_CVPAT_DESCRIPTION = "Single fitted model versus IA/LM benchmarks; one-sided test, 95% confidence; not a comparison of saved models";

export const CURRENT_PLS_PREDICT_METHOD_VERSION = "plspredict_indicator_v2";
export const CURRENT_PLS_PREDICT_REPEATED_METHOD_VERSION = "plspredict_repeated_kfold_indicator_v2";
export const CURRENT_CVPAT_METHOD_VERSION = "cvpat_indicator_benchmarks_v2";
export const LEGACY_PLS_PREDICT_METHOD_VERSION = "plspredict_holdout_v1";
export const LEGACY_PLS_PREDICT_REPEATED_METHOD_VERSION = "plspredict_repeated_kfold_v1";
export const NATIVE_PREDICTION_FOLDS = 10;
export const NATIVE_PREDICTION_REPEATS = 10;
export const NATIVE_PREDICTION_CONFIDENCE_LEVEL = 0.95;
export const NATIVE_PREDICTION_MIN_COMPLETE_CASES = 20;

const DEFAULT_BOOTSTRAP_SAMPLES = 10_000;
const DEFAULT_PERMUTATION_SAMPLES = 999;

function boundedSamples(value: number, fallback: number, minimum: number): number {
  if (!Number.isFinite(value) || value <= 0) return fallback;
  return Math.min(10_000, Math.max(minimum, Math.trunc(value)));
}

export function nativeCalculationModeForSettings(settings: Readonly<AnalysisUiSettings>): NativeCalculationMode {
  if (settings.method === "predict") return "predict";
  if (settings.permutationSamples > 0) return "permutation";
  if (settings.method === "bootstrap" || settings.bootstrapSamples > 0) return "bootstrap";
  return "pls";
}

export function nativeCalculationSettingsForMode(
  settings: Readonly<AnalysisUiSettings>,
  mode: NativeCalculationMode,
): AnalysisUiSettings {
  return {
    ...settings,
    method: mode === "predict" ? "predict" : "pls_pm",
    // The compact prediction workflow is indicator-level PLSpredict / CVPAT.
    // Clear legacy segmentation state that the dialog does not disclose.
    groupMethods: mode === "predict" ? null : settings.groupMethods,
    bootstrapSamples: mode === "bootstrap"
      ? boundedSamples(settings.bootstrapSamples, DEFAULT_BOOTSTRAP_SAMPLES, 100)
      : 0,
    studentizedInnerSamples: mode === "bootstrap" ? settings.studentizedInnerSamples : 0,
    permutationSamples: mode === "permutation"
      ? boundedSamples(settings.permutationSamples, DEFAULT_PERMUTATION_SAMPLES, 99)
      : 0,
    confidenceLevel: mode === "predict" ? NATIVE_PREDICTION_CONFIDENCE_LEVEL : settings.confidenceLevel,
    workers: mode === "bootstrap" || mode === "permutation" ? settings.workers : 1,
  };
}

export function nativeCalculationMethodName(mode: NativeCalculationMode): string {
  if (mode === "predict") return NATIVE_PREDICTION_METHOD_LABEL;
  if (mode === "bootstrap") return "PLS-SEM Bootstrapping";
  if (mode === "permutation") return "Structural Path Randomization";
  return "PLS-SEM Algorithm";
}

export function nativeCalculationStartLabel(mode: NativeCalculationMode, retry: boolean): string {
  const verb = retry ? "Retry" : "Start";
  if (mode === "predict") return `${verb} prediction`;
  if (mode === "bootstrap") return `${verb} bootstrapping`;
  if (mode === "permutation") return `${verb} path randomization`;
  return `${verb} calculation`;
}
