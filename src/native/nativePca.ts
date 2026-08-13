import type { AnalysisUiSettings, Dataset } from "../types";
import { nativeNcaNumericColumns } from "./nativeNca";

export const NATIVE_PCA_MIN_VARIABLES = 2;
export const NATIVE_PCA_MAX_VARIABLES = 50;
export const NATIVE_PCA_MIN_COMPLETE_CASES = 3;

export const NATIVE_PCA_SCOPE_NOTE =
  "Correlation-matrix PCA of 2 to 50 selected numeric variables with listwise deletion, deterministic component orientation, and no rotation or inferential resampling.";

export const NATIVE_PCA_ENGINE_SCOPE_WARNING =
  "Standalone PCA v1 is validated for the documented QuickPLS v1.2 supported scope; unsupported shapes remain blocked.";

export function nativePcaNumericColumns(dataset: Readonly<Dataset>): string[] {
  return nativeNcaNumericColumns(dataset);
}

export function nativePcaSelectedVariables(settings: Readonly<AnalysisUiSettings>): string[] {
  return (settings.pcaVariables ?? "")
    .split(",")
    .map((variable) => variable.trim())
    .filter(Boolean);
}

function finiteNumber(value: string | number | null | undefined): number | null {
  if (typeof value === "number") return Number.isFinite(value) ? value : null;
  if (typeof value !== "string" || !value.trim()) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

export interface NativePcaReadinessAssessment {
  canRun: boolean;
  blockers: string[];
  detail: string;
  completeCases: number | null;
}

export function nativePcaReadiness(
  dataset: Readonly<Dataset>,
  settings: Readonly<AnalysisUiSettings>,
): NativePcaReadinessAssessment {
  const selected = nativePcaSelectedVariables(settings);
  const unique = new Set(selected);
  const numeric = new Set(nativePcaNumericColumns(dataset));
  const rule = settings.pcaComponentRule ?? "kaiser";
  const fixedComponents = settings.pcaComponents ?? 2;
  const threshold = settings.pcaVarianceThreshold ?? 0.80;
  const blockers = [
    selected.length < NATIVE_PCA_MIN_VARIABLES
      ? `Select at least ${NATIVE_PCA_MIN_VARIABLES} numeric variables`
      : null,
    selected.length > NATIVE_PCA_MAX_VARIABLES
      ? `Select no more than ${NATIVE_PCA_MAX_VARIABLES} variables`
      : null,
    unique.size !== selected.length ? "Each PCA variable may be selected only once" : null,
    ...selected.map((variable) => !dataset.columns.includes(variable)
      ? `The selected variable ${variable} is absent from the active dataset`
      : !numeric.has(variable)
        ? `The selected variable ${variable} is not numeric`
        : null),
    !["kaiser", "fixed", "variance_threshold"].includes(rule)
      ? "Choose Kaiser, fixed-count, or cumulative-variance retention"
      : null,
    rule === "fixed" && (
      !Number.isInteger(fixedComponents)
      || fixedComponents < 1
      || fixedComponents > Math.min(selected.length || 1, NATIVE_PCA_MAX_VARIABLES)
    )
      ? "Fixed components must be an integer from 1 through the number of selected variables"
      : null,
    rule === "variance_threshold" && (
      !Number.isFinite(threshold) || threshold < 0.01 || threshold > 0.999
    )
      ? "Cumulative variance threshold must be from 1% to 99.9%"
      : null,
  ].filter((problem): problem is string => Boolean(problem));

  const rowCount = dataset.rowCount ?? dataset.rows.length;
  const residentComplete = dataset.rows.length >= rowCount;
  let completeCases: number | null = null;
  if (residentComplete
    && selected.length >= NATIVE_PCA_MIN_VARIABLES
    && selected.length <= NATIVE_PCA_MAX_VARIABLES
    && unique.size === selected.length
    && selected.every((variable) => numeric.has(variable))) {
    const completeRows = dataset.rows.filter((row) =>
      selected.every((variable) => finiteNumber(row[variable]) !== null));
    completeCases = completeRows.length;
    if (completeCases < NATIVE_PCA_MIN_COMPLETE_CASES) {
      blockers.push(`PCA requires at least ${NATIVE_PCA_MIN_COMPLETE_CASES} complete finite rows after listwise deletion`);
    }
    for (const variable of selected) {
      const values = completeRows.map((row) => finiteNumber(row[variable])!);
      if (values.length && Math.min(...values) === Math.max(...values)) {
        blockers.push(`${variable} is constant after listwise deletion`);
      }
    }
    if (rule === "fixed" && fixedComponents > Math.min(selected.length, Math.max(1, completeCases - 1))) {
      blockers.push("Fixed components cannot exceed min(selected variables, complete cases minus one)");
    }
  }

  const uniqueBlockers = [...new Set(blockers)];
  const detail = uniqueBlockers.length
    ? `${uniqueBlockers.join("; ")}.`
    : completeCases === null
      ? `Standalone PCA is ready for ${selected.length} variables. Complete finite rows and nonconstant columns will be verified by the desktop engine.`
      : `Standalone PCA is ready for ${selected.length} variables and ${completeCases} complete finite rows.`;
  return { canRun: uniqueBlockers.length === 0, blockers: uniqueBlockers, detail, completeCases };
}
