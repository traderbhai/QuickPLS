import type { AnalysisUiSettings, Dataset, DatasetRowsPage } from "../types";
import { getNativeDatasetRows } from "../services/projectService";
import { nativeOlsCsvValues, nativeOlsNumericColumns } from "./nativeOls";

export const NATIVE_LOGISTIC_MAX_TERMS = 25;
export const NATIVE_LOGISTIC_PROFILE_PAGE_SIZE = 500;
export const NATIVE_LOGISTIC_SCOPE_NOTE =
  "Binary logistic regression with an intercept, raw numeric predictors, listwise deletion, deterministic maximum-likelihood estimation, Wald inference, odds ratios, fitted probabilities, and fixed two-sided 95% confidence intervals. The outcome must be coded exactly 0/1. Multinomial, ordinal, weighted, clustered, penalized, and Firth-corrected models are not included.";
export const NATIVE_LOGISTIC_ENGINE_SCOPE_WARNING =
  "Logistic regression v2 is validated for the documented QuickPLS binary numeric complete-case scope; multinomial, ordinal, weighted, clustered, categorical auto-encoding, and Firth-corrected models remain unsupported.";
export const NATIVE_LEGACY_LOGISTIC_ENGINE_SCOPE_WARNING =
  "Logistic regression v1 is validated for the documented QuickPLS v1.2.2 binary numeric complete-case scope; multinomial, ordinal, weighted, clustered, and Firth-corrected models remain unsupported.";

export interface NativeLogisticProfile {
  datasetId: string;
  datasetFingerprint: string;
  outcome: string;
  predictors: string[];
  controls: string[];
  expectedRows: number;
  scannedRows: number;
  completeCases: number;
  omittedRows: number;
  zeroCases: number;
  oneCases: number;
  invalidOutcomeRows: number;
  constantTerms: string[];
}

export interface NativeLogisticReadinessAssessment {
  canRun: boolean;
  blockers: string[];
  detail: string;
  profileRequired: boolean;
  profile: NativeLogisticProfile | null;
}

type DatasetRow = Dataset["rows"][number];

interface LogisticProfileAccumulator {
  scannedRows: number;
  completeCases: number;
  zeroCases: number;
  oneCases: number;
  invalidOutcomeRows: number;
  termBounds: Map<string, { minimum: number; maximum: number }>;
}

function finiteNumber(value: string | number | null | undefined): number | null {
  if (typeof value === "number") return Number.isFinite(value) ? value : null;
  if (typeof value !== "string" || !value.trim()) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function createAccumulator(terms: readonly string[]): LogisticProfileAccumulator {
  return {
    scannedRows: 0,
    completeCases: 0,
    zeroCases: 0,
    oneCases: 0,
    invalidOutcomeRows: 0,
    termBounds: new Map(terms.map((term) => [term, { minimum: Number.POSITIVE_INFINITY, maximum: Number.NEGATIVE_INFINITY }])),
  };
}

function scanRows(
  accumulator: LogisticProfileAccumulator,
  rows: readonly DatasetRow[],
  outcome: string,
  terms: readonly string[],
) {
  for (const row of rows) {
    accumulator.scannedRows += 1;
    const rawOutcome = row[outcome];
    const outcomeValue = finiteNumber(rawOutcome);
    const validOutcome = outcomeValue === 0 || outcomeValue === 1;
    const termValues = terms.map((term) => finiteNumber(row[term]));
    if (outcomeValue === null || termValues.some((value) => value === null)) continue;

    accumulator.completeCases += 1;
    if (outcomeValue === 0) accumulator.zeroCases += 1;
    else if (outcomeValue === 1) accumulator.oneCases += 1;
    else accumulator.invalidOutcomeRows += 1;
    terms.forEach((term, index) => {
      const value = termValues[index]!;
      const bounds = accumulator.termBounds.get(term)!;
      bounds.minimum = Math.min(bounds.minimum, value);
      bounds.maximum = Math.max(bounds.maximum, value);
    });
  }
}

function finishProfile(
  dataset: Readonly<Dataset>,
  outcome: string,
  predictors: readonly string[],
  controls: readonly string[],
  expectedRows: number,
  accumulator: LogisticProfileAccumulator,
): NativeLogisticProfile {
  return {
    datasetId: dataset.id,
    datasetFingerprint: dataset.fingerprint?.trim() ?? "",
    outcome,
    predictors: [...predictors],
    controls: [...controls],
    expectedRows,
    scannedRows: accumulator.scannedRows,
    completeCases: accumulator.completeCases,
    omittedRows: Math.max(0, expectedRows - accumulator.completeCases),
    zeroCases: accumulator.zeroCases,
    oneCases: accumulator.oneCases,
    invalidOutcomeRows: accumulator.invalidOutcomeRows,
    constantTerms: [...accumulator.termBounds.entries()]
      .filter(([, bounds]) => Number.isFinite(bounds.minimum) && bounds.minimum === bounds.maximum)
      .map(([term]) => term),
  };
}

function selectedVariables(settings: Readonly<AnalysisUiSettings>) {
  const outcome = settings.regressionOutcome?.trim() ?? "";
  const predictors = nativeOlsCsvValues(settings.regressionPredictors);
  const controls = nativeOlsCsvValues(settings.regressionControls);
  return { outcome, predictors, controls, terms: [...predictors, ...controls] };
}

export function residentNativeLogisticProfile(
  dataset: Readonly<Dataset>,
  settings: Readonly<AnalysisUiSettings>,
): NativeLogisticProfile | null {
  const rowCount = dataset.rowCount ?? dataset.rows.length;
  if (dataset.rows.length < rowCount) return null;
  const { outcome, predictors, controls, terms } = selectedVariables(settings);
  if (!outcome || !terms.length) return null;
  const accumulator = createAccumulator(terms);
  scanRows(accumulator, dataset.rows.slice(0, rowCount), outcome, terms);
  return finishProfile(dataset, outcome, predictors, controls, rowCount, accumulator);
}

/**
 * Profiles the complete desktop dataset in bounded sequential pages. Only
 * aggregate counts and per-term bounds survive each page; case rows are never
 * accumulated in frontend memory.
 */
export async function profileNativeLogisticDataset(
  dataset: Readonly<Dataset>,
  settings: Readonly<AnalysisUiSettings>,
  readPage: (datasetId: string, offset: number, limit: number) => Promise<DatasetRowsPage> = getNativeDatasetRows,
  isCancelled: () => boolean = () => false,
): Promise<NativeLogisticProfile> {
  const resident = residentNativeLogisticProfile(dataset, settings);
  if (resident) return resident;

  const { outcome, predictors, controls, terms } = selectedVariables(settings);
  if (!outcome || !terms.length) throw new Error("Choose a logistic outcome and at least one predictor before profiling.");
  const expectedRows = dataset.rowCount ?? dataset.rows.length;
  const accumulator = createAccumulator(terms);
  let offset = 0;
  while (offset < expectedRows) {
    if (isCancelled()) throw new Error("Logistic outcome profiling was cancelled.");
    const page = await readPage(dataset.id, offset, NATIVE_LOGISTIC_PROFILE_PAGE_SIZE);
    if (page.datasetId !== dataset.id || page.offset !== offset || page.rowCount !== expectedRows) {
      throw new Error("The dataset changed while its logistic outcome was being profiled.");
    }
    if (page.rows.length === 0 || page.rows.length > NATIVE_LOGISTIC_PROFILE_PAGE_SIZE) {
      throw new Error("The desktop row service returned an invalid logistic-profile page.");
    }
    scanRows(accumulator, page.rows, outcome, terms);
    offset += page.rows.length;
  }
  if (accumulator.scannedRows !== expectedRows) {
    throw new Error(`Expected ${expectedRows} rows but profiled ${accumulator.scannedRows}.`);
  }
  return finishProfile(dataset, outcome, predictors, controls, expectedRows, accumulator);
}

function nonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

function textArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string" && item.trim().length > 0);
}

function sameStrings(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((item, index) => item === right[index]);
}

/** Parses the aggregate proof carried from the setup dialog to dispatch. */
export function parseNativeLogisticProfile(value: unknown): NativeLogisticProfile | null {
  if (!value || typeof value !== "object") return null;
  const candidate = value as Partial<NativeLogisticProfile>;
  if (typeof candidate.datasetId !== "string" || !candidate.datasetId.trim()
    || typeof candidate.datasetFingerprint !== "string" || !candidate.datasetFingerprint.trim()
    || typeof candidate.outcome !== "string" || !candidate.outcome.trim()
    || !textArray(candidate.predictors) || candidate.predictors.length < 1
    || !textArray(candidate.controls)
    || !textArray(candidate.constantTerms)
    || !nonNegativeInteger(candidate.expectedRows)
    || !nonNegativeInteger(candidate.scannedRows)
    || !nonNegativeInteger(candidate.completeCases)
    || !nonNegativeInteger(candidate.omittedRows)
    || !nonNegativeInteger(candidate.zeroCases)
    || !nonNegativeInteger(candidate.oneCases)
    || !nonNegativeInteger(candidate.invalidOutcomeRows)) return null;
  const terms = [...candidate.predictors, ...candidate.controls];
  if (candidate.scannedRows !== candidate.expectedRows
    || candidate.completeCases + candidate.omittedRows !== candidate.expectedRows
    || candidate.zeroCases + candidate.oneCases + candidate.invalidOutcomeRows !== candidate.completeCases
    || new Set([candidate.outcome, ...terms]).size !== terms.length + 1
    || new Set(candidate.constantTerms).size !== candidate.constantTerms.length
    || candidate.constantTerms.some((term) => !terms.includes(term))) return null;
  return {
    datasetId: candidate.datasetId,
    datasetFingerprint: candidate.datasetFingerprint,
    outcome: candidate.outcome,
    predictors: [...candidate.predictors],
    controls: [...candidate.controls],
    expectedRows: candidate.expectedRows,
    scannedRows: candidate.scannedRows,
    completeCases: candidate.completeCases,
    omittedRows: candidate.omittedRows,
    zeroCases: candidate.zeroCases,
    oneCases: candidate.oneCases,
    invalidOutcomeRows: candidate.invalidOutcomeRows,
    constantTerms: [...candidate.constantTerms],
  };
}

export function nativeLogisticReadiness(
  dataset: Readonly<Dataset>,
  settings: Readonly<AnalysisUiSettings>,
  suppliedProfile: NativeLogisticProfile | null = null,
): NativeLogisticReadinessAssessment {
  const { outcome, predictors, controls, terms } = selectedVariables(settings);
  const variables = outcome ? [outcome, ...terms] : terms;
  const numeric = new Set(nativeOlsNumericColumns(dataset));
  const blockers = [
    !outcome ? "Choose one numeric outcome coded exactly 0/1" : null,
    predictors.length < 1 ? "Choose at least one numeric predictor" : null,
    terms.length > NATIVE_LOGISTIC_MAX_TERMS ? `Choose no more than ${NATIVE_LOGISTIC_MAX_TERMS} predictors and controls combined` : null,
    new Set(variables).size !== variables.length ? "Outcome, predictors, and controls must be distinct variables" : null,
    ...variables.map((variable) => !dataset.columns.includes(variable)
      ? `The selected variable ${variable} is absent from the active dataset`
      : !numeric.has(variable)
        ? `The selected variable ${variable} is not numeric`
        : null),
    !dataset.fingerprint?.trim() ? "Import or reopen a fingerprinted dataset before binary logistic regression" : null,
  ].filter((problem): problem is string => Boolean(problem));

  const parsedProfile = suppliedProfile ? parseNativeLogisticProfile(suppliedProfile) : null;
  const profile = parsedProfile ?? (suppliedProfile || blockers.length ? null : residentNativeLogisticProfile(dataset, settings));
  if (profile) {
    const expectedRows = dataset.rowCount ?? dataset.rows.length;
    if (profile.datasetId !== dataset.id
      || profile.datasetFingerprint !== dataset.fingerprint
      || profile.outcome !== outcome
      || !sameStrings(profile.predictors, predictors)
      || !sameStrings(profile.controls, controls)
      || profile.expectedRows !== expectedRows
      || profile.scannedRows !== expectedRows) {
      blockers.push("Reload the complete logistic outcome profile for the current dataset and variable selection");
    } else {
      if (profile.invalidOutcomeRows > 0) {
        blockers.push(`${profile.invalidOutcomeRows} non-missing outcome row${profile.invalidOutcomeRows === 1 ? " is" : "s are"} not coded exactly 0 or 1`);
      }
      if (profile.zeroCases === 0 || profile.oneCases === 0) {
        blockers.push("The listwise-complete outcome must contain both class 0 and class 1");
      }
      const minimum = terms.length + 2;
      if (profile.completeCases < minimum) {
        blockers.push(`Binary logistic regression requires at least ${minimum} complete finite rows for ${terms.length} fitted term${terms.length === 1 ? "" : "s"}`);
      }
      if (profile.constantTerms.length) {
        blockers.push(`${profile.constantTerms.join(", ")} ${profile.constantTerms.length === 1 ? "is constant" : "are constant"} after listwise deletion`);
      }
    }
  } else if (suppliedProfile) {
    blockers.push("Reload the complete logistic outcome profile because its dispatch proof is invalid");
  }

  const uniqueBlockers = [...new Set(blockers)];
  const profileRequired = uniqueBlockers.length === 0 && profile === null;
  const detail = uniqueBlockers.length
    ? `${uniqueBlockers.join("; ")}.`
    : profileRequired
      ? `The binary logistic setup for ${outcome} is structurally ready. The complete dataset must be profiled before the calculation starts.`
      : `Binary logistic regression is ready for ${outcome} with ${profile!.completeCases} complete cases: ${profile!.zeroCases} class 0 and ${profile!.oneCases} class 1.`;
  return { canRun: uniqueBlockers.length === 0, blockers: uniqueBlockers, detail, profileRequired, profile };
}
