import type { AnalysisUiSettings } from "../types";
import {
  isNativeWorkbenchAnalysisKind,
  type NativeWorkbenchAnalysisKind,
} from "./nativeAnalysisCatalog";
import { nativeAnalysisRecipeDescriptor } from "./nativeAnalysisRecipe";
import {
  parseNativeLogisticProfile,
  type NativeLogisticProfile,
} from "./nativeLogistic";
import { nativeOlsCsvValues } from "./nativeOls";
import { NATIVE_REGRESSION_BOOTSTRAP_MAX_SELECTED_TERMS } from "./nativeRegressionBootstrapWitness";

export interface NativeCalculationRequest {
  kind: NativeWorkbenchAnalysisKind;
  settings: AnalysisUiSettings;
  logisticProfile?: NativeLogisticProfile;
}

export function createNativeCalculationRequest(
  kind: NativeWorkbenchAnalysisKind,
  settings: Readonly<AnalysisUiSettings>,
  logisticProfile?: NativeLogisticProfile,
): NativeCalculationRequest {
  const verifiedLogisticProfile = kind === "regression" && settings.regressionType === "logistic"
    ? parseNativeLogisticProfile(logisticProfile)
    : null;
  return {
    kind,
    settings: { ...settings },
    ...(verifiedLogisticProfile ? { logisticProfile: verifiedLogisticProfile } : {}),
  };
}

export function parseNativeCalculationRequest(value: unknown): NativeCalculationRequest | null {
  if (!value || typeof value !== "object") return null;
  const candidate = value as Partial<NativeCalculationRequest>;
  if (typeof candidate.kind !== "string" || !isNativeWorkbenchAnalysisKind(candidate.kind)) return null;
  if (!candidate.settings || typeof candidate.settings !== "object") return null;
  if (typeof candidate.settings.method !== "string") return null;
  const kind = candidate.kind as NativeWorkbenchAnalysisKind;
  if (candidate.settings.method !== nativeAnalysisRecipeDescriptor(kind).engineMethod) return null;
  const settings = candidate.settings as AnalysisUiSettings;
  if (kind === "regression" && settings.regressionBootstrap === true) {
    const selectedTermCount = nativeOlsCsvValues(settings.regressionPredictors).length
      + nativeOlsCsvValues(settings.regressionControls).length;
    if ((settings.regressionType !== "ols" && settings.regressionType !== "logistic")
      || selectedTermCount > NATIVE_REGRESSION_BOOTSTRAP_MAX_SELECTED_TERMS
      || !Number.isInteger(settings.bootstrapSamples)
      || settings.bootstrapSamples < 99
      || settings.bootstrapSamples > 10_000
      || settings.studentizedInnerSamples !== 0
      || settings.permutationSamples !== 0
      || settings.confidenceLevel !== 0.95
      || !Number.isInteger(settings.workers)
      || settings.workers < 1
      || settings.workers > 64) return null;
  }
  const logistic = kind === "regression" && settings.regressionType === "logistic";
  const logisticProfile = logistic ? parseNativeLogisticProfile(candidate.logisticProfile) : null;
  if (logistic && candidate.logisticProfile !== undefined && !logisticProfile) return null;
  return createNativeCalculationRequest(kind, settings, logisticProfile ?? undefined);
}
