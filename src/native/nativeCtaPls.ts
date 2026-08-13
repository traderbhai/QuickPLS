import type { Edge, Node } from "@xyflow/react";
import type { AnalysisUiSettings, ConstructData, Dataset } from "../types";

export const NATIVE_CTA_PLS_METHOD_VERSION = "cta_pls_tetrad_v1" as const;
export const NATIVE_CTA_PLS_COVARIANCE_VERSION = "sample_covariance_of_preprocessed_indicators_v1" as const;
export const NATIVE_CTA_PLS_RESULT_WARNING =
  "CTA-PLS tetrad bootstrap/permutation inference is outside the validated QuickPLS v1.2.3 descriptive scope.";
export const NATIVE_CTA_PLS_ESTIMATION_WARNING =
  "CTA-PLS tetrad diagnostics are validated for the documented QuickPLS v1.2.3 descriptive tetrad scope; bootstrap/permutation tetrad decision rules remain unsupported.";
export const NATIVE_CTA_PLS_SCOPE_NOTE =
  "Descriptive sample-covariance tetrads only. QuickPLS reports all three pairings for every four-indicator subset; it does not classify blocks or calculate bootstrap, permutation, asymptotic, or vanishing-tetrad decisions.";

export const NATIVE_CTA_PLS_PAIRINGS = [
  "ab_cd_minus_ac_bd",
  "ac_bd_minus_ad_bc",
  "ad_bc_minus_ab_cd",
] as const;

export interface NativeCtaPlsEligibleBlock {
  constructId: string;
  constructLabel: string;
  indicators: string[];
  quadruples: number;
  tetrads: number;
}

export interface NativeCtaPlsSetupAssessment {
  canRun: boolean;
  detail: string;
  blockers: string[];
  eligibleBlocks: NativeCtaPlsEligibleBlock[];
  completeCases: number | null;
}

export function nativeCtaPlsCombinationCount(indicatorCount: number): number {
  if (!Number.isInteger(indicatorCount) || indicatorCount < 4) return 0;
  return indicatorCount * (indicatorCount - 1) * (indicatorCount - 2) * (indicatorCount - 3) / 24;
}

export function nativeCtaPlsEligibleBlocks(
  nodes: readonly Node<ConstructData>[],
): NativeCtaPlsEligibleBlock[] {
  return nodes
    .filter((node) => !node.data.semantic && node.data.indicators.length >= 4)
    .map((node) => {
      const quadruples = nativeCtaPlsCombinationCount(node.data.indicators.length);
      return {
        constructId: node.id,
        constructLabel: node.data.label.trim() || node.id,
        indicators: [...node.data.indicators],
        quadruples,
        tetrads: quadruples * NATIVE_CTA_PLS_PAIRINGS.length,
      };
    });
}

export function nativeCtaPlsSetupAssessment(
  dataset: Dataset,
  nodes: readonly Node<ConstructData>[],
  settings: Readonly<AnalysisUiSettings>,
  edges: readonly Edge[] = [],
): NativeCtaPlsSetupAssessment {
  const eligibleBlocks = nativeCtaPlsEligibleBlocks(nodes);
  const eligibleIndicators = [...new Set(eligibleBlocks.flatMap((block) => block.indicators))];
  const allModelIndicators = [...new Set(nodes.flatMap((node) => node.data.indicators))];
  const specialConstructs = nodes.filter((node) => Boolean(node.data.semantic));
  const nonStructuralEdges = edges.filter((edge) => {
    const role = (edge.data as { role?: string } | undefined)?.role;
    return role === "control" || role === "covariance";
  });
  const missingColumns = eligibleIndicators.filter((indicator) => !dataset.columns.includes(indicator));
  const nonNumericMetadata = new Set((dataset.columnMetadata ?? [])
    .filter((column) => column.column_type !== "numeric")
    .map((column) => column.name));
  const nonNumericIndicators = eligibleIndicators.filter((indicator) => nonNumericMetadata.has(indicator));
  const observations = dataset.rowCount ?? dataset.rows.length;
  const fullRowsResident = dataset.rows.length >= observations;
  const completeRows = fullRowsResident
    ? dataset.rows.filter((row) => allModelIndicators.every((indicator) => finiteDatasetNumber(row[indicator]) !== null))
    : null;
  const completeCases = completeRows?.length ?? null;
  const constantIndicators = completeRows && completeRows.length >= 2
    ? eligibleIndicators.filter((indicator) => sampleVariance(completeRows.map((row) => finiteDatasetNumber(row[indicator])!)) <= 0)
    : [];

  const blockers = [
    eligibleBlocks.length === 0
      ? "CTA-PLS requires at least one ordinary construct with four or more assigned indicators"
      : null,
    specialConstructs.length > 0
      ? "The bounded native CTA-PLS workflow does not support interaction or higher-order constructs"
      : null,
    nonStructuralEdges.length > 0
      ? "The bounded native CTA-PLS workflow supports structural paths only; control and covariance edges are excluded"
      : null,
    (settings.weightingScheme ?? "path") === "pca"
      ? "CTA-PLS requires path or factor weighting"
      : null,
    settings.caseWeightColumn?.trim()
      ? "CTA-PLS does not support case weights in the bounded descriptive scope"
      : null,
    settings.bootstrapSamples > 0 || settings.studentizedInnerSamples > 0 || settings.permutationSamples > 0
      ? "CTA-PLS descriptive tetrads must be run separately from resampling inference"
      : null,
    missingColumns.length
      ? `CTA-PLS indicator${missingColumns.length === 1 ? "" : "s"} ${missingColumns.join(", ")} ${missingColumns.length === 1 ? "is" : "are"} absent from the active dataset`
      : null,
    nonNumericIndicators.length
      ? `CTA-PLS indicator${nonNumericIndicators.length === 1 ? "" : "s"} ${nonNumericIndicators.join(", ")} ${nonNumericIndicators.length === 1 ? "is" : "are"} not numeric`
      : null,
    completeCases !== null && completeCases < 3
      ? `CTA-PLS requires at least three listwise-complete observations; ${completeCases} remain`
      : null,
    constantIndicators.length
      ? `CTA-PLS indicator${constantIndicators.length === 1 ? "" : "s"} ${constantIndicators.join(", ")} ${constantIndicators.length === 1 ? "has" : "have"} zero variance after listwise deletion`
      : null,
  ].filter((problem): problem is string => Boolean(problem));

  const quadruples = eligibleBlocks.reduce((sum, block) => sum + block.quadruples, 0);
  const tetrads = eligibleBlocks.reduce((sum, block) => sum + block.tetrads, 0);
  const observationDetail = completeCases === null
    ? "The engine will verify complete cases and indicator variance against the full fingerprinted dataset."
    : `${completeCases} listwise-complete observations are available.`;
  return {
    canRun: blockers.length === 0,
    blockers,
    eligibleBlocks,
    completeCases,
    detail: blockers.length
      ? `${blockers.join("; ")}.`
      : `${eligibleBlocks.length} eligible block${eligibleBlocks.length === 1 ? "" : "s"} produce ${quadruples} four-indicator subset${quadruples === 1 ? "" : "s"} and ${tetrads} descriptive tetrads. ${observationDetail}`,
  };
}

function finiteDatasetNumber(value: string | number | null | undefined): number | null {
  if (value === null || value === undefined || value === "") return null;
  const numeric = typeof value === "number" ? value : Number(value);
  return Number.isFinite(numeric) ? numeric : null;
}

function sampleVariance(values: readonly number[]): number {
  if (values.length < 2) return 0;
  const mean = values.reduce((sum, value) => sum + value, 0) / values.length;
  return values.reduce((sum, value) => sum + (value - mean) ** 2, 0) / (values.length - 1);
}
