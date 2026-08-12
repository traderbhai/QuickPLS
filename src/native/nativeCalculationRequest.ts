import type { AnalysisUiSettings } from "../types";
import {
  isNativeWorkbenchAnalysisKind,
  type NativeWorkbenchAnalysisKind,
} from "./nativeAnalysisCatalog";
import { nativeAnalysisRecipeDescriptor } from "./nativeAnalysisRecipe";

export interface NativeCalculationRequest {
  kind: NativeWorkbenchAnalysisKind;
  settings: AnalysisUiSettings;
}

export function createNativeCalculationRequest(
  kind: NativeWorkbenchAnalysisKind,
  settings: Readonly<AnalysisUiSettings>,
): NativeCalculationRequest {
  return {
    kind,
    settings: { ...settings },
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
  return createNativeCalculationRequest(kind, candidate.settings as AnalysisUiSettings);
}
