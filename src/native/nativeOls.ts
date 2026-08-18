import type { AnalysisUiSettings, Dataset } from "../types";
import { nativeNcaNumericColumns } from "./nativeNca";
import { NATIVE_REGRESSION_BOOTSTRAP_MAX_SELECTED_TERMS } from "./nativeRegressionBootstrapWitness";

export const NATIVE_OLS_MAX_TERMS = 25;
export const NATIVE_OLS_SCOPE_NOTE =
  "Raw numeric ordinary least squares with an intercept, listwise deletion, HC3 robust standard errors, and fixed two-sided 95% confidence intervals. Optional regression case-resampling reports percentile-primary and conditional BCa inference. Categorical encoding, weights, clusters, generic PLS resampling, logistic regression, and PROCESS models are not included.";
export const NATIVE_OLS_ENGINE_SCOPE_WARNING =
  "OLS regression v1 requires numeric complete-case variables and HC3 robust standard errors; incompatible configurations are blocked before calculation.";

export function nativeOlsNumericColumns(dataset: Readonly<Dataset>): string[] {
  return nativeNcaNumericColumns(dataset);
}

export function nativeOlsCsvValues(value: string | null | undefined): string[] {
  return (value ?? "").split(",").map((item) => item.trim()).filter(Boolean);
}

function finiteNumber(value: string | number | null | undefined): number | null {
  if (typeof value === "number") return Number.isFinite(value) ? value : null;
  if (typeof value !== "string" || !value.trim()) return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

export interface NativeOlsReadinessAssessment {
  canRun: boolean;
  blockers: string[];
  detail: string;
  completeCases: number | null;
}

export function nativeOlsReadiness(
  dataset: Readonly<Dataset>,
  settings: Readonly<AnalysisUiSettings>,
): NativeOlsReadinessAssessment {
  const outcome = settings.regressionOutcome?.trim() ?? "";
  const predictors = nativeOlsCsvValues(settings.regressionPredictors);
  const controls = nativeOlsCsvValues(settings.regressionControls);
  const terms = [...predictors, ...controls];
  const maximumTerms = settings.regressionBootstrap === true
    ? NATIVE_REGRESSION_BOOTSTRAP_MAX_SELECTED_TERMS
    : NATIVE_OLS_MAX_TERMS;
  const variables = outcome ? [outcome, ...terms] : terms;
  const numeric = new Set(nativeOlsNumericColumns(dataset));
  const blockers = [
    !outcome ? "Choose one numeric outcome variable" : null,
    predictors.length < 1 ? "Choose at least one numeric predictor" : null,
    terms.length > maximumTerms
      ? settings.regressionBootstrap === true
        ? `Regression bootstrap supports at most ${maximumTerms} predictors and controls (${maximumTerms + 1} coefficient terms including the intercept)`
        : `Choose no more than ${maximumTerms} predictors and controls combined`
      : null,
    new Set(variables).size !== variables.length ? "Outcome, predictors, and controls must be distinct variables" : null,
    ...variables.map((variable) => !dataset.columns.includes(variable)
      ? `The selected variable ${variable} is absent from the active dataset`
      : !numeric.has(variable)
        ? `The selected variable ${variable} is not numeric`
        : null),
    settings.robustSe && settings.robustSe !== "hc3" ? "OLS regression v1 requires HC3 robust standard errors" : null,
  ].filter((problem): problem is string => Boolean(problem));

  const rowCount = dataset.rowCount ?? dataset.rows.length;
  const residentComplete = dataset.rows.length >= rowCount;
  let completeCases: number | null = null;
  if (residentComplete
    && outcome
    && predictors.length
    && terms.length <= maximumTerms
    && new Set(variables).size === variables.length
    && variables.every((variable) => numeric.has(variable))) {
    const completeRows = dataset.rows.filter((row) =>
      variables.every((variable) => finiteNumber(row[variable]) !== null));
    completeCases = completeRows.length;
    const minimum = terms.length + 2;
    if (completeCases < minimum) {
      blockers.push(`OLS requires at least ${minimum} complete finite rows for ${terms.length} fitted term${terms.length === 1 ? "" : "s"}`);
    }
    for (const variable of variables) {
      const values = completeRows.map((row) => finiteNumber(row[variable])!);
      if (values.length && Math.min(...values) === Math.max(...values)) {
        blockers.push(`${variable} is constant after listwise deletion`);
      }
    }
  }

  const uniqueBlockers = [...new Set(blockers)];
  const detail = uniqueBlockers.length
    ? `${uniqueBlockers.join("; ")}.`
    : completeCases === null
      ? `Standalone OLS is ready for ${outcome} with ${terms.length} fitted term${terms.length === 1 ? "" : "s"}. Complete rows and full-rank design will be verified by the desktop engine.`
      : `Standalone OLS is ready for ${outcome} with ${terms.length} fitted term${terms.length === 1 ? "" : "s"} and ${completeCases} complete finite rows.`;
  return { canRun: uniqueBlockers.length === 0, blockers: uniqueBlockers, detail, completeCases };
}
