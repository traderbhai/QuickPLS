import type { AnalysisUiSettings, MethodDefinition } from "../types";
import {
  methodCapabilityAvailabilityV2,
  type MethodCapabilityAvailabilityOptionsV2,
  type MethodCapabilityAvailabilityV2,
} from "./methodCapabilityRegistryV2";

export interface AnalysisCatalogCapabilityEntryV2 {
  readonly method: MethodDefinition;
  readonly settings: AnalysisUiSettings;
  readonly availability: MethodCapabilityAvailabilityV2;
}

export interface AnalysisCatalogCapabilityCountsV2 {
  readonly standard: number;
  readonly experimental: number;
  readonly hidden: number;
}

/**
 * Registry-backed projection for the historical workspace catalogue.
 *
 * `MethodDefinition.status` is deliberately ignored: it predates option-cell
 * coverage and cannot authorize Standard or Experimental visibility.
 */
export function analysisCatalogCapabilityEntriesV2(
  definitions: readonly MethodDefinition[],
  settings: Readonly<AnalysisUiSettings>,
  options: MethodCapabilityAvailabilityOptionsV2,
): readonly AnalysisCatalogCapabilityEntryV2[] {
  const seen = new Set<string>();
  return Object.freeze(definitions.map((method) => {
    if (seen.has(method.id)) {
      throw new Error(`Duplicate analysis method definition: ${method.id}`);
    }
    seen.add(method.id);
    const candidate = Object.freeze({
      ...settings,
      method: method.id,
    }) as AnalysisUiSettings;
    return Object.freeze({
      method,
      settings: candidate,
      availability: methodCapabilityAvailabilityV2(candidate, options),
    });
  }));
}

export function visibleAnalysisCatalogCapabilityEntriesV2(
  definitions: readonly MethodDefinition[],
  settings: Readonly<AnalysisUiSettings>,
  options: MethodCapabilityAvailabilityOptionsV2,
): readonly AnalysisCatalogCapabilityEntryV2[] {
  return Object.freeze(analysisCatalogCapabilityEntriesV2(definitions, settings, options)
    .filter((entry) => entry.availability.selectable));
}

export function analysisCatalogCapabilityCountsV2(
  entries: readonly AnalysisCatalogCapabilityEntryV2[],
): AnalysisCatalogCapabilityCountsV2 {
  const counts = { standard: 0, experimental: 0, hidden: 0 };
  for (const entry of entries) counts[entry.availability.tier] += 1;
  return Object.freeze(counts);
}
