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
  const logistic = kind === "regression" && settings.regressionType === "logistic";
  const logisticProfile = logistic ? parseNativeLogisticProfile(candidate.logisticProfile) : null;
  if (logistic && candidate.logisticProfile !== undefined && !logisticProfile) return null;
  return createNativeCalculationRequest(kind, settings, logisticProfile ?? undefined);
}
