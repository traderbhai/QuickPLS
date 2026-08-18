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
import {
  nativeProcessGraphAssessment,
  parseNativeProcessProfile,
  type NativeProcessProfile,
} from "./nativeProcess";

export interface NativeCalculationRequest {
  kind: NativeWorkbenchAnalysisKind;
  settings: AnalysisUiSettings;
  logisticProfile?: NativeLogisticProfile;
  processProfile?: NativeProcessProfile;
}

export function createNativeCalculationRequest(
  kind: NativeWorkbenchAnalysisKind,
  settings: Readonly<AnalysisUiSettings>,
  dataProfile?: NativeLogisticProfile | NativeProcessProfile,
): NativeCalculationRequest {
  const verifiedLogisticProfile = kind === "regression" && settings.regressionType === "logistic"
    ? parseNativeLogisticProfile(dataProfile)
    : null;
  const verifiedProcessProfile = kind === "regression" && settings.regressionType === "process"
    ? parseNativeProcessProfile(dataProfile)
    : null;
  return {
    kind,
    settings: { ...settings },
    ...(verifiedLogisticProfile ? { logisticProfile: verifiedLogisticProfile } : {}),
    ...(verifiedProcessProfile ? { processProfile: verifiedProcessProfile } : {}),
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
  if (kind === "pls_permutation" && (
    !Number.isInteger(settings.permutationSamples)
    || settings.permutationSamples < 99
    || settings.permutationSamples > 10_000
    || settings.bootstrapSamples !== 0
    || settings.studentizedInnerSamples !== 0
    || !Number.isInteger(settings.workers)
    || settings.workers < 1
    || settings.workers > 64
  )) return null;
  if (kind === "mga") {
    const groupMethods = (settings.groupMethods ?? "")
      .split(",")
      .map((method) => method.trim())
      .filter(Boolean);
    const groupA = settings.groupAValue?.trim() ?? "";
    const groupB = settings.groupBValue?.trim() ?? "";
    if (!settings.groupColumn?.trim()
      || !groupA
      || !groupB
      || groupA === groupB
      || groupMethods.length !== 2
      || groupMethods[0] !== "micom"
      || groupMethods[1] !== "mga_permutation"
      || settings.micomConfiguralConfirmed !== true
      || !Number.isInteger(settings.groupPermutationSamples)
      || settings.groupPermutationSamples! < 5_000
      || settings.groupPermutationSamples! > 10_000
      || settings.weightingScheme !== "path"
      || settings.preprocessing !== "standardized"
      || settings.bootstrapSamples !== 0
      || settings.studentizedInnerSamples !== 0
      || settings.permutationSamples !== 0
      || Boolean(settings.caseWeightColumn?.trim())) return null;
  }
  const process = kind === "regression" && settings.regressionType === "process";
  if (process) {
    const graph = nativeProcessGraphAssessment(settings);
    if (!graph.canRun || !graph.graph
      || settings.preprocessing !== "unstandardized"
      || settings.confidenceLevel !== 0.95
      || settings.studentizedInnerSamples !== 0
      || settings.permutationSamples !== 0
      || (settings.regressionBootstrap === true
        ? !Number.isInteger(settings.bootstrapSamples)
          || settings.bootstrapSamples < 99
          || settings.bootstrapSamples > 10_000
          || !Number.isInteger(settings.workers)
          || settings.workers < 1
          || settings.workers > 64
        : settings.bootstrapSamples !== 0 || settings.workers !== 1)) return null;
  }
  if (kind === "regression" && settings.regressionBootstrap === true) {
    const selectedTermCount = nativeOlsCsvValues(settings.regressionPredictors).length
      + nativeOlsCsvValues(settings.regressionControls).length;
    if ((settings.regressionType !== "ols" && settings.regressionType !== "logistic" && settings.regressionType !== "process")
      || (settings.regressionType !== "process" && selectedTermCount > NATIVE_REGRESSION_BOOTSTRAP_MAX_SELECTED_TERMS)
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
  const processProfile = process ? parseNativeProcessProfile(candidate.processProfile) : null;
  if (process && candidate.processProfile !== undefined && !processProfile) return null;
  if ((!logistic && candidate.logisticProfile !== undefined)
    || (!process && candidate.processProfile !== undefined)) return null;
  return createNativeCalculationRequest(kind, settings, logisticProfile ?? processProfile ?? undefined);
}
