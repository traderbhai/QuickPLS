import type { AnalysisRun } from "../types";

export interface ComparisonRow {
  metric: string;
  item: string;
  baseline: string;
  comparison: string;
  delta: string;
}

export function compareRuns(baseline: AnalysisRun | undefined, comparison: AnalysisRun | undefined): ComparisonRow[] {
  if (!baseline?.result || !comparison?.result) return [];
  return [
    ...compareMap("R2", baseline.result.r_squared, comparison.result.r_squared),
    ...comparePathCoefficients(baseline, comparison),
  ];
}

function compareMap(metric: string, left: Record<string, number>, right: Record<string, number>) {
  const keys = [...new Set([...Object.keys(left), ...Object.keys(right)])].sort();
  return keys.map((key) => numericRow(metric, key, left[key], right[key]));
}

function comparePathCoefficients(baseline: AnalysisRun, comparison: AnalysisRun) {
  const left = Object.fromEntries(baseline.result!.paths.map((path) => [`${path.source} -> ${path.target}`, path.coefficient]));
  const right = Object.fromEntries(comparison.result!.paths.map((path) => [`${path.source} -> ${path.target}`, path.coefficient]));
  return compareMap("Path coefficient", left, right);
}

function numericRow(metric: string, item: string, baseline: number | undefined, comparison: number | undefined): ComparisonRow {
  return {
    metric,
    item,
    baseline: formatValue(baseline),
    comparison: formatValue(comparison),
    delta: baseline == null || comparison == null ? "N/A" : formatValue(comparison - baseline),
  };
}

function formatValue(value: number | undefined) {
  return value == null || !Number.isFinite(value) ? "N/A" : value.toFixed(6);
}
