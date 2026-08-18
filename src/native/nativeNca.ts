import type { AnalysisUiSettings, Dataset } from "../types";

export const NATIVE_NCA_MIN_COMPLETE_CASES = 3;
export const NATIVE_NCA_MIN_PERMUTATIONS = 1;
export const NATIVE_NCA_MAX_PERMUTATIONS = 10_000;
export const NATIVE_NCA_DEFAULT_PERMUTATIONS = 999;

export const NATIVE_NCA_SCOPE_NOTE =
  "Numeric observed-variable CE-FDH and CR-FDH analysis with observed-range bottlenecks. Multiple conditions, latent-score NCA, cIPMA, and broader ceiling variants are not included.";

export const NATIVE_NCA_ENGINE_SCOPE_WARNING =
  "NCA v2 supports one observed numeric condition/outcome pair with CE-FDH and CR-FDH ceilings, seeded one-sided permutation evidence, and observed-range bottlenecks. Multiple conditions, latent-score NCA, cIPMA, and additional ceiling variants are not available.";

export const NATIVE_STANDALONE_ASSESSMENT_WARNING =
  "PLS assessment is not applicable to standalone raw-data analyses.";

export interface NativeNcaReadinessAssessment {
  canRun: boolean;
  blockers: string[];
  detail: string;
  completeCases: number | null;
}

function finiteNumericValue(value: string | number | null | undefined): number | null {
  if (typeof value === "number") return Number.isFinite(value) ? value : null;
  if (typeof value !== "string" || !value.trim()) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

/**
 * Numeric NCA candidates come from declared metadata when available. For
 * lightweight in-memory fixtures without metadata, every resident non-missing
 * value must be finite numeric data before a column is offered.
 */
export function nativeNcaNumericColumns(dataset: Readonly<Dataset>): string[] {
  const metadata = new Map((dataset.columnMetadata ?? []).map((column) => [column.name, column]));
  return dataset.columns.filter((column) => {
    const declared = metadata.get(column);
    if (declared) return declared.column_type === "numeric";
    const observed = dataset.rows
      .map((row) => row[column])
      .filter((value) => value !== null && value !== undefined && value !== "");
    return observed.length > 0 && observed.every((value) => finiteNumericValue(value) !== null);
  });
}

export function nativeNcaReadiness(
  dataset: Readonly<Dataset>,
  settings: Readonly<AnalysisUiSettings>,
): NativeNcaReadinessAssessment {
  const x = settings.ncaX?.trim() ?? "";
  const y = settings.ncaY?.trim() ?? "";
  const numericColumns = new Set(nativeNcaNumericColumns(dataset));
  const ceiling = settings.ncaCeiling ?? "both";
  const permutations = settings.ncaPermutationSamples ?? NATIVE_NCA_DEFAULT_PERMUTATIONS;
  const blockers = [
    !x ? "Choose a numeric condition variable (X)" : null,
    !y ? "Choose a numeric outcome variable (Y)" : null,
    x && !dataset.columns.includes(x) ? "The selected X variable is absent from the active dataset" : null,
    y && !dataset.columns.includes(y) ? "The selected Y variable is absent from the active dataset" : null,
    x && dataset.columns.includes(x) && !numericColumns.has(x) ? "The selected X variable is not numeric" : null,
    y && dataset.columns.includes(y) && !numericColumns.has(y) ? "The selected Y variable is not numeric" : null,
    x && y && x === y ? "Condition X and outcome Y must be different variables" : null,
    !["ce_fdh", "cr_fdh", "both"].includes(ceiling) ? "Choose CE-FDH, CR-FDH, or both ceiling lines" : null,
    !Number.isInteger(permutations)
      || permutations < NATIVE_NCA_MIN_PERMUTATIONS
      || permutations > NATIVE_NCA_MAX_PERMUTATIONS
      ? `NCA requires ${NATIVE_NCA_MIN_PERMUTATIONS.toLocaleString("en-US")} to ${NATIVE_NCA_MAX_PERMUTATIONS.toLocaleString("en-US")} permutations`
      : null,
  ].filter((problem): problem is string => Boolean(problem));

  const rowCount = dataset.rowCount ?? dataset.rows.length;
  const hasCompleteResidentData = dataset.rows.length >= rowCount;
  let completeCases: number | null = null;
  if (hasCompleteResidentData && x && y && numericColumns.has(x) && numericColumns.has(y) && x !== y) {
    const pairs = dataset.rows.flatMap((row) => {
      const xValue = finiteNumericValue(row[x]);
      const yValue = finiteNumericValue(row[y]);
      return xValue === null || yValue === null ? [] : [[xValue, yValue] as const];
    });
    completeCases = pairs.length;
    if (pairs.length < NATIVE_NCA_MIN_COMPLETE_CASES) {
      blockers.push(`NCA requires at least ${NATIVE_NCA_MIN_COMPLETE_CASES} complete finite X/Y rows after listwise deletion`);
    } else {
      const xValues = pairs.map(([value]) => value);
      const yValues = pairs.map(([, value]) => value);
      if (Math.min(...xValues) === Math.max(...xValues)) blockers.push("The selected X variable is constant after listwise deletion");
      if (Math.min(...yValues) === Math.max(...yValues)) blockers.push("The selected Y variable is constant after listwise deletion");
    }
  }

  const detail = blockers.length
    ? `${[...new Set(blockers)].join("; ")}.`
    : completeCases === null
      ? `Standalone NCA is ready for ${x} as X and ${y} as Y. Complete finite rows will be verified by the desktop engine.`
      : `Standalone NCA is ready for ${x} as X and ${y} as Y with ${completeCases} complete finite row${completeCases === 1 ? "" : "s"}.`;
  return { canRun: blockers.length === 0, blockers: [...new Set(blockers)], detail, completeCases };
}
