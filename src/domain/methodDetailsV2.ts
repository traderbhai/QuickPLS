import type { AnalysisUiSettings } from "../types";
import {
  capabilityRegistryV2,
  type CapabilityOptionCellV2,
  type CapabilityRegistryRowV2,
  type QualificationLinkV2,
} from "./capabilityRegistryV2";
import {
  validateMethodDetailsV2,
  type CapabilityAvailabilityV2,
  type MethodDetailsV2,
} from "./capabilitySurfaceV2";
import {
  methodCapabilityRequirementsV2,
  type MethodCapabilityRequirementV2,
} from "./methodCapabilityRegistryV2";

export interface CapabilityMethodDetailsV2 {
  readonly capability_cell: QualificationLinkV2;
  readonly method_name: string;
  readonly family: string;
  readonly option_name: string;
  readonly availability: CapabilityAvailabilityV2;
  readonly availability_message: string;
  readonly details: MethodDetailsV2;
}

export interface MethodDetailsResolutionV2 {
  readonly status: "ready" | "unavailable";
  readonly method_id: string;
  readonly items: readonly CapabilityMethodDetailsV2[];
  readonly issues: readonly string[];
}

const QUESTION_COPY: Readonly<Record<string, string>> = Object.freeze({
  "smartpls.pls_algorithm": "How strongly the specified composite model relates its indicators and structural paths.",
  "smartpls.plsc": "How the reflective common-factor model behaves after consistent PLS correction.",
  "smartpls.wpls": "How the PLS-SEM model behaves when observations receive positive case weights.",
  "smartpls.gsca": "How a structured component model reproduces its measurement and structural relations.",
  "smartpls.cca": "Where observed composite correlations differ from correlations reproduced by the model.",
  "smartpls.cta_pls": "Whether tetrad patterns in eligible indicator blocks are consistent with the specified measurement form.",
  "smartpls.ipma": "Which predecessors combine high structural importance with lower observed performance for a selected target.",
  "smartpls.cbsem": "How well a covariance-based latent-variable model fits the data and estimates its structural parameters.",
  "smartpls.cfa": "How well a confirmatory common-factor measurement model fits the data.",
  "smartpls.cbsem_bootstrapping": "How stable CB-SEM parameter estimates are across full case-resample refits.",
  "smartpls.pls_bootstrapping": "How stable supported PLS-SEM estimates are across case-resample refits.",
  "smartpls.permutation": "Whether structural-path effects remain unusual under the selected randomization design.",
  "smartpls.micom": "Whether the measurement model is sufficiently invariant across the selected groups.",
  "smartpls.mga": "How selected PLS-SEM parameters differ between declared groups.",
  "smartpls.plspredict": "How accurately the fitted model predicts held-out indicator observations.",
  "smartpls.cvpat": "Whether model predictions improve on the selected prediction benchmark.",
  "smartpls.nca": "Whether a condition appears necessary, but not sufficient, for reaching an outcome level.",
  "smartpls.pca_core": "How selected observed variables can be summarized by orthogonal principal components.",
  "smartpls.pca_cbsem": "How selected observed variables can be summarized before covariance-based modeling.",
  "smartpls.regression": "How a numeric outcome changes with the selected observed predictors and controls.",
  "smartpls.logistic_regression": "How selected observed predictors relate to the probability of a binary outcome.",
  "smartpls.regression_bootstrapping": "How stable regression estimates are across case-resample refits.",
  "smartpls.process": "How direct, indirect, and conditional observed-variable effects combine in the drawn path model.",
  "smartpls.process_bootstrapping": "How stable indirect and conditional effects are across case-resample refits.",
  "smartpls.mediation": "How much of a relationship operates through one or more mediators.",
  "smartpls.moderation": "How a relationship changes across values of a moderator.",
  "smartpls.pls_power_analysis": "What sample size is technically indicated by the documented PLS-SEM rule, or what power a prospective design may achieve.",
  "smartpls.pls_pos": "Whether prediction-oriented segmentation finds distinct groups with different PLS-SEM behavior.",
  "smartpls.fimix_pls": "Whether a finite-mixture model identifies latent segments with distinct PLS-SEM behavior.",
});

const OUTPUT_COPY: Readonly<Record<string, string>> = Object.freeze({
  "smartpls.pls_algorithm": "Construct scores, paths, loadings, weights, explained variance, effects, and available quality measures.",
  "smartpls.cbsem": "Parameter estimates, standard errors, fit measures, residual diagnostics, and convergence information.",
  "smartpls.cfa": "Loadings, factor correlations, residuals, reliability information, fit measures, and convergence information.",
  "smartpls.plspredict": "Fold-level and aggregate prediction errors, Q-squared prediction results, and benchmark comparisons.",
  "smartpls.cvpat": "Paired prediction-loss differences and the available benchmark assessment.",
  "smartpls.nca": "Ceiling-line effects, permutation results, and bottleneck tables for the selected condition and outcome.",
  "smartpls.ipma": "Importance and performance tables and the corresponding map for each selected target.",
});

const SCOPE_COPY: Readonly<Record<string, string>> = Object.freeze({
  "smartpls.pls_algorithm": "Recursive composite PLS path models using the calculation options offered in the current workspace.",
  "smartpls.pls_power_analysis": "Prospective Monte Carlo power for exactly two ordinary reflective constructs joined by one predictor-to-outcome path under the declared Gaussian design without missing values.",
  "smartpls.permutation": "Either two-group MICOM and permutation MGA, or single-model direct-path randomization using fixed converged PLS scores.",
  "smartpls.cta_pls": "Descriptive sample-covariance tetrad diagnostics for eligible PLS blocks with at least four numeric indicators and the same complete cases as the associated PLS run.",
  "smartpls.micom": "MICOM Steps 1–3 for exactly two selected groups under one shared no-retry permutation plan.",
  "smartpls.mga": "Two-group permutation differences for paths, outer loadings, and outer weights under the shared MICOM plan.",
  "smartpls.cbsem": "Raw-data, single-group, continuous reflective maximum-likelihood CFA or recursive SEM with listwise deletion and marker identification.",
  "smartpls.pca_cbsem": "The standalone raw-data PCA workflow used before covariance-based modeling.",
});

const SETTINGS_COPY: Readonly<Record<string, string>> = Object.freeze({
  "smartpls.pls_algorithm": "Choose the available weighting and preprocessing options. The run uses deterministic execution and records every setting.",
  "smartpls.pls_power_analysis": "Declare the scenario, population path, loadings, sample-size grid, Monte Carlo replications, indexed bootstrap plan, seed, and workers.",
  "smartpls.wpls": "Select a positive finite case-weight variable together with the available PLS weighting and preprocessing options.",
  "smartpls.plsc": "Use reflective constructs with at least two indicators, path or factor weighting, and listwise preprocessing.",
  "smartpls.pca_core": "Select 2–50 numeric variables and choose a fixed component count, cumulative-variance target, or Kaiser retention rule.",
  "smartpls.pca_cbsem": "Select 2–50 numeric variables and choose a fixed component count, cumulative-variance target, or Kaiser retention rule.",
  "smartpls.pls_bootstrapping": "Choose resamples, confidence level, seed, workers, and optional studentized inner samples. Percentile and conditional BCa results use the recorded indexed plan.",
  "smartpls.consistent_bootstrapping": "Choose 1,000–10,000 indexed full-PLSc refits, confidence level, seed, and workers. The run reports normal-reference, percentile, and conditional BCa results.",
  "smartpls.permutation": "Choose the permitted permutation count, seed, workers, and the group or single-model design required by the selected option.",
  "smartpls.plspredict": "The fixed design uses balanced seeded 10-fold cross-validation with 10 repeats, IA and LM benchmarks, and a secondary modulo-4 holdout check.",
  "smartpls.cvpat": "The fixed design compares one fitted model with IA and LM benchmarks using a one-sided test at 95% confidence.",
  "smartpls.nca": "Select one numeric condition and outcome, ceiling method, permutation count, bottleneck levels, seed, and workers.",
  "smartpls.regression": "Select one numeric outcome plus predictors and optional controls. Inference uses HC3 standard errors and fixed two-sided 95% intervals.",
  "smartpls.logistic_regression": "Select an exactly 0/1 numeric outcome plus predictors and optional controls. Inference uses maximum-likelihood standard errors, Wald tests, and fixed two-sided 95% intervals.",
  "smartpls.regression_bootstrapping": "Choose 1,000–10,000 case resamples, seed, and workers. Percentile intervals are primary and BCa is reported when the required delete-one fits are usable.",
  "smartpls.process": "Draw the supported graph directly, select its observed-variable roles, and use the fixed HC3, Student-t, and 95% inference settings.",
  "smartpls.process_bootstrapping": "Choose 1,000–10,000 complete-case resamples, seed, and workers. Percentile intervals are primary and BCa requires all required delete-one fits.",
  "smartpls.cbsem": "The estimator is maximum likelihood with the first loading fixed to 1, no mean structure, unstandardized output, and listwise deletion.",
  "smartpls.cfa": "The estimator is maximum likelihood with the first loading fixed to 1, no mean structure, unstandardized output, and listwise deletion.",
  "smartpls.cbsem_bootstrapping": "Choose 500–10,000 indexed no-retry case refits and one fixed 95% interval design: percentile Type-7, analytic-studentized Type-7, or complete-delete-one BCa Type-7.",
});

const CAUTION_COPY: Readonly<Record<string, string>> = Object.freeze({
  "smartpls.pls_power_analysis": "This is prospective design analysis, not retrospective observed power or a heuristic sample-size rule.",
  "smartpls.wpls": "Every included case weight must be finite and greater than zero.",
  "smartpls.pca_core": "Rotation and inferential resampling are not part of this PCA workflow.",
  "smartpls.pca_cbsem": "Rotation and inferential resampling are not part of this PCA workflow.",
  "smartpls.permutation": "This is not an unrestricted generic permutation procedure.",
  "smartpls.cca": "The output is descriptive; it does not classify the measurement model or provide inferential decisions.",
  "smartpls.cta_pls": "The output is descriptive; inferential decisions and automatic measurement classification are not provided.",
  "smartpls.micom": "The workflow is limited to exactly two groups and does not provide consistent-MICOM variants.",
  "smartpls.mga": "The workflow is limited to exactly two groups under the shared fixed permutation plan.",
  "smartpls.plspredict": "Prediction targets are endogenous indicators; construct-score metrics are supplementary and saved-model comparison is not included.",
  "smartpls.cvpat": "CVPAT compares one fitted model with IA and LM benchmarks; it does not compare saved models.",
  "smartpls.ipma": "Performance uses 0–100 observed-range scaling of standardized composite scores; the target and unrelated constructs are omitted.",
  "smartpls.higher_order_models": "General SEM supports one non-nested HOC through the exact repeated, extended-repeated, embedded two-stage, and disjoint two-stage approach/type matrix. Hybrid, HOC interactions, multiple or nested HOCs, groups, weights, feedback, PLSc, and permutation remain excluded.",
  "smartpls.gsca": "Bootstrap inference is not part of the GSCA point-estimate workflow.",
  "smartpls.logistic_regression": "The outcome must be coded exactly 0/1. Multinomial, ordinal, weighted, clustered, penalized, and Firth-corrected variants are not included.",
  "smartpls.nca": "The workflow analyzes one observed condition/outcome pair and should not be interpreted as evidence of sufficiency.",
  "smartpls.process": "Numbered PROCESS templates are not executed; author the supported continuous-outcome graph directly.",
  "smartpls.process_bootstrapping": "Studentized intervals, one-tailed tests, and custom alpha are not included.",
  "smartpls.regression": "Categorical encoding, weights, clusters, logistic regression, and PROCESS graphs are separate workflows.",
  "smartpls.regression_bootstrapping": "Studentized intervals, one-tailed tests, and custom alpha are not included.",
  "smartpls.cbsem": "Mean structures, robust or ordinal estimators, FIML, and invariance testing are separate or unavailable workflows.",
  "smartpls.cfa": "Mean structures, robust or ordinal estimators, FIML, and invariance testing are separate or unavailable workflows.",
  "smartpls.cbsem_bootstrapping": "Studentized and BCa options are CFA-only and enforce the displayed sample, variable, parameter, delete-one, and worker limits. Failed indexed refits are retained without retry.",
});

const PREDICATE_COPY: Readonly<Record<string, string>> = Object.freeze({
  "model_family:pls_sem": "A PLS-SEM model",
  "model_family:cbsem": "A covariance-based SEM model",
  "model_family:cfa": "A confirmatory factor model",
  "input:raw_numeric": "Raw numeric observations",
  "input:covariance": "A covariance matrix with its sample size",
  "input:correlation": "A correlation matrix with its sample size",
  "missing_data:method_defined": "A missing-data choice offered for this method",
  "case_weights:required": "A positive case-weight column",
  "groups:required": "A declared grouping variable",
});

function sentenceCase(value: string): string {
  const readable = value.replaceAll("_", " ").replace(":", ": ").trim();
  return readable ? `${readable[0].toUpperCase()}${readable.slice(1)}` : value;
}

function predicateText(token: string): string {
  return PREDICATE_COPY[token] ?? sentenceCase(token);
}

function unique(values: readonly string[]): string[] {
  return [...new Set(values.filter((value) => value.trim().length > 0))];
}

function requirementsText(row: CapabilityRegistryRowV2, cell: CapabilityOptionCellV2): string {
  const model = unique(cell.supported_model_predicate.official.map(predicateText));
  const data = unique(cell.supported_data_predicate.official.map(predicateText));
  const parts = [
    model.length > 0 ? `Model: ${model.join("; ")}.` : "",
    data.length > 0 ? `Data: ${data.join("; ")}.` : "",
  ].filter(Boolean);
  const requirements = parts.join(" ") || `Use ${row.official_method} only with the model and data choices offered in Calculate.`;
  const supportedUse = SCOPE_COPY[row.capability_id] ?? row.scope_statement
    .replace(/^Release-qualified\s+/i, "")
    .replace(/^Scoped Standard\s+/i, "")
    .replace(/\bbounded\b/gi, "documented")
    .trim();
  return `${requirements} Supported use: ${supportedUse}`;
}

function availabilityMessage(availability: CapabilityAvailabilityV2): string {
  if (availability.visibility === "supported") return "Available in Standard.";
  if (availability.visibility === "experimental") return "Available in Experimental Labs.";
  switch (availability.reason) {
    case "labs_disabled":
      return "Turn on Experimental Labs in Preferences to use this option.";
    case "not_executable":
    case "incomplete_standard_cell":
      return "This option is not available for calculation in the current build.";
    case "legacy_only":
    case "intentionally_excluded":
      return "This historical method is available only when reopening earlier results.";
    default:
      return "This option is not available from the current workspace.";
  }
}

function cautionText(
  row: CapabilityRegistryRowV2,
  availability: CapabilityAvailabilityV2,
): string {
  const useRequirement = `Use ${row.official_method} only when the listed model and data requirements match your study.`;
  const methodCaution = CAUTION_COPY[row.capability_id];
  if (availability.visibility === "experimental") {
    return `This option is Experimental. Independently check the result before final reporting. ${useRequirement}${methodCaution ? ` ${methodCaution}` : ""}`;
  }
  return `${useRequirement}${methodCaution ? ` ${methodCaution}` : ""}`;
}

function optionName(row: CapabilityRegistryRowV2): string {
  return row.official_method;
}

function buildDetails(
  requirement: MethodCapabilityRequirementV2,
  row: CapabilityRegistryRowV2,
  cell: CapabilityOptionCellV2,
  availability: CapabilityAvailabilityV2,
): MethodDetailsV2 {
  const references = unique([
    cell.documentation_reference,
    ...cell.settings_schema.references,
    ...cell.result_schema.references,
  ]).filter((reference) => reference.startsWith("https://"));
  const details: MethodDetailsV2 = {
    what_it_answers: QUESTION_COPY[row.capability_id]
      ?? `What ${row.official_method} estimates or assesses for the specified model and data.`,
    when_to_use: `Use it when ${row.official_method} directly matches the research question and every requirement below is satisfied.`,
    required_model_and_data: requirementsText(row, cell),
    settings_and_defaults: SETTINGS_COPY[row.capability_id]
      ?? "Calculate shows the available settings and their defaults. The completed run records every selected value.",
    outputs: OUTPUT_COPY[row.capability_id]
      ?? `The completed run contains the tables, charts, diagnostics, and exportable values available for ${row.official_method}.`,
    assumptions_and_cautions: cautionText(row, availability),
    interpretation_guidance: "Interpret estimates together with their uncertainty, diagnostics, assumptions, and the research design. Use Run Details when reporting how the result was produced.",
    method_references: references,
    advanced_technical_details: "The calculation recipe records the exact method revision, data fingerprint, settings, random seed, and worker configuration so the run can be reproduced.",
  };
  const errors = validateMethodDetailsV2(details);
  if (errors.length > 0) throw new Error(`Invalid Method Details for ${requirement.capability_id}: ${errors.join("; ")}`);
  return Object.freeze(details);
}

export function methodDetailsForSettingsV2(
  settings: Readonly<AnalysisUiSettings>,
  experimentalLabsEnabled: boolean,
): MethodDetailsResolutionV2 {
  let requirements: readonly MethodCapabilityRequirementV2[];
  try {
    requirements = methodCapabilityRequirementsV2(settings);
  } catch (error) {
    return Object.freeze({
      status: "unavailable",
      method_id: String(settings.method),
      items: Object.freeze([]),
      issues: Object.freeze([error instanceof Error ? error.message : "The selected method could not be resolved."]),
    });
  }

  return methodDetailsForRequirementsV2(String(settings.method), requirements, experimentalLabsEnabled);
}

export function methodDetailsForRequirementsV2(
  methodId: string,
  requirements: readonly MethodCapabilityRequirementV2[],
  experimentalLabsEnabled: boolean,
): MethodDetailsResolutionV2 {
  if (requirements.length === 0) {
    return Object.freeze({
      status: "unavailable",
      method_id: methodId,
      items: Object.freeze([]),
      issues: Object.freeze(["Method information is unavailable for this run."]),
    });
  }

  const items: CapabilityMethodDetailsV2[] = [];
  const issues: string[] = [];
  for (const requirement of requirements) {
    const match = capabilityRegistryV2.quickPlsCell(requirement.cell_id).find((candidate) => (
      candidate.row.capability_id === requirement.capability_id
      && candidate.cell.capability_id === requirement.capability_id
    ));
    if (!match) {
      issues.push(`Method information is unavailable for ${requirement.option}.`);
      continue;
    }
    const availability = capabilityRegistryV2.availability(
      requirement.capability_id,
      requirement.cell_id,
      experimentalLabsEnabled,
    );
    items.push(Object.freeze({
      capability_cell: match.link,
      method_name: match.row.official_method,
      family: match.row.official_family,
      option_name: optionName(match.row),
      availability,
      availability_message: availabilityMessage(availability),
      details: buildDetails(requirement, match.row, match.cell, availability),
    }));
  }
  return Object.freeze({
    status: issues.length === 0 && items.length > 0 ? "ready" : "unavailable",
    method_id: methodId,
    items: Object.freeze(items),
    issues: Object.freeze(issues),
  });
}
