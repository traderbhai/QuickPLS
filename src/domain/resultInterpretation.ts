import type { AnalysisRun, AssessmentResult, MeasurementMode, PlsResult, ResultWorkspaceTab } from "../types";

import { nativeRegressionBootstrapResultProjection } from "../native/nativeResults";
import {
  nativeLegacyProcessResultProjection,
  nativeProcessResultProjection,
} from "../native/nativeProcessResults";
import {
  NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING,
  nativeStructuralPathRandomizationProjection,
} from "../native/nativeStructuralPathRandomization";

export type InterpretationSeverity = "good" | "caution" | "issue" | "info" | "unavailable";
export type InterpretationGroup = "must" | "recommended" | "optional" | "report";

export interface InterpretationFinding {
  id: string;
  severity: InterpretationSeverity;
  group: InterpretationGroup;
  tab: ResultWorkspaceTab;
  metric: string;
  value: string;
  thresholdGuide: string;
  construct?: string;
  path?: { source: string; target: string };
  indicator?: string;
  interpretation: string;
  recommendedAction: string;
  reportSentence: string;
  linkedObject?: { type: "construct" | "path" | "indicator"; id: string; source?: string; target?: string };
}

export interface SemDiagramNodeLike {
  id: string;
  data?: {
    label?: string;
    shortName?: string;
    mode?: MeasurementMode;
    indicators?: string[];
    semantic?: string;
  };
}

export interface SemDiagramEdgeLike {
  id?: string;
  source: string;
  target: string;
  data?: { role?: string };
}

export interface ResultInterpretationContext {
  run: AnalysisRun;
  nodes?: SemDiagramNodeLike[];
  edges?: SemDiagramEdgeLike[];
}

export interface ResultInterpretation {
  findings: InterpretationFinding[];
  diagramAdvice: InterpretationFinding[];
  reportParagraphs: Array<{ section: string; text: string; sourceFindingIds: string[] }>;
}

export function buildResultInterpretation(context: ResultInterpretationContext): ResultInterpretation {
  const { run } = context;
  const result = run.result;
  if (!result) {
    return {
      findings: [finding({
        id: "run.unavailable",
        severity: "unavailable",
        group: "must",
        tab: "overview",
        metric: "Result payload",
        value: "not available",
        thresholdGuide: "A completed run must contain result payloads before interpretation is possible.",
        interpretation: "This saved run does not include numerical result values.",
        recommendedAction: "Rerun the selected method and confirm the run completes successfully.",
        reportSentence: "No result interpretation was generated because the run has no completed result payload.",
      })],
      diagramAdvice: [],
      reportParagraphs: [],
    };
  }

  const process = nativeProcessResultProjection(run);
  const legacyProcess = nativeLegacyProcessResultProjection(run);
  const regressionBootstrap = nativeRegressionBootstrapResultProjection(run);
  const findings = process
    ? dedupeFindings(inferenceFindings(run))
    : legacyProcess
      ? [historicalProcessFinding(legacyProcess)]
      : regressionBootstrap
        ? dedupeFindings(inferenceFindings(run))
    : dedupeFindings([
      ...pathFindings(run, result),
      ...measurementFindings(result, run.assessment),
      ...validityFindings(run.assessment),
      ...structuralDiagnosticFindings(result, run.assessment),
      ...inferenceFindings(run),
      ...mediationModerationFindings(result),
      ...predictionFindings(result, run.assessment),
      ...methodPayloadFindings(result),
    ]);
  const diagramAdvice = process || legacyProcess || regressionBootstrap
    ? []
    : dedupeFindings(diagramAdvisorFindings(context, result));
  const allFindings = sortFindings(dedupeFindings([...findings, ...diagramAdvice]));
  return {
    findings: allFindings,
    diagramAdvice,
    reportParagraphs: reportParagraphs(run, result, allFindings),
  };
}

export function findingsForTab(interpretation: ResultInterpretation, tab: ResultWorkspaceTab): InterpretationFinding[] {
  if (tab === "overview") return interpretation.findings.slice(0, 6);
  if (tab === "interpretation") return interpretation.findings;
  return interpretation.findings.filter((item) => item.tab === tab).slice(0, 8);
}

export function findingsByGroup(findings: InterpretationFinding[]) {
  return {
    must: findings.filter((item) => item.group === "must"),
    recommended: findings.filter((item) => item.group === "recommended"),
    optional: findings.filter((item) => item.group === "optional"),
    report: findings.filter((item) => item.group === "report"),
  };
}

export function rowSpecificInterpretation(title: string, columns: string[], row: string[]): string {
  const values = Object.fromEntries(columns.map((column, index) => [column.toLowerCase(), row[index] ?? ""]));
  if (/structural path randomization/i.test(title)) {
    const probability = Number(values["raw two-sided p"]);
    return `${values.path || "This structural path"} has raw pathwise two-sided plus-one p ${Number.isFinite(probability) ? probability.toFixed(4) : "unavailable"}. This Freedman-Lane fixed-score result conditions on fixed original PLS construct scores, assumes exchangeable reduced-model residuals, and is unadjusted for multiplicity.`;
  }
  if (/path coefficients/i.test(title)) {
    const coefficient = Number(values.coefficient);
    if (Number.isFinite(coefficient)) {
      const direction = Math.abs(coefficient) < 0.05 ? "near zero" : coefficient > 0 ? "positive" : "negative";
      return `${values.path || "This path"} has coefficient ${coefficient.toFixed(4)}, so the estimate is ${direction}. Interpret it as estimate-only unless this run includes bootstrap or permutation inference.`;
    }
  }
  if (/outer loadings/i.test(title)) {
    const loading = Number(values.loading);
    if (Number.isFinite(loading)) {
      const status = loadingStatus(loading);
      return `${values.indicator || "This indicator"} loads ${loading.toFixed(4)} on ${values.construct || "its construct"}. ${loadingInterpretation(status)}.`;
    }
  }
  if (/construct reliability/i.test(title)) {
    return `${values.construct || "This construct"} shows alpha ${values["cronbach alpha"] || "N/A"}, rho_C ${values["rho_c"] || "N/A"}, and AVE ${values.ave || "N/A"}. Review any metric below the common .70 reliability or .50 AVE guide before reporting.`;
  }
  if (/inner vif/i.test(title)) {
    const vif = Number(values.vif);
    if (Number.isFinite(vif)) return `${values.predictor || "This predictor"} has VIF ${vif.toFixed(4)} for ${values.target || "the target construct"}. ${vifInterpretation(vif)}.`;
  }
  if (/cohen f/i.test(title)) {
    const f2 = Number(values["f²"] ?? values["f2"]);
    if (Number.isFinite(f2)) return `${values.path || "This predictor"} has f2 ${f2.toFixed(4)}, a ${f2Label(f2)} effect-size guide.`;
  }
  if (/mediation effects/i.test(title)) {
    return `${values.effect || "This mediation row"} reports direct ${values.direct || "N/A"}, indirect ${values.indirect || "N/A"}, total ${values.total || "N/A"}, and class ${values.class || "N/A"}. Use bootstrap intervals if available before making inference claims.`;
  }
  return `${title}: ${columns.map((column, index) => `${column} ${row[index] ?? "N/A"}`).join(", ")}. Use the metric-specific findings above for report guidance.`;
}

export function copyableInterpretationText(findings: InterpretationFinding[]) {
  return findings.map((item) => `${severityLabel(item.severity)}: ${item.interpretation} Action: ${item.recommendedAction}`).join("\n");
}

function pathFindings(run: AnalysisRun, result: PlsResult): InterpretationFinding[] {
  const findings: InterpretationFinding[] = [];
  if (!result.paths.length) return findings;
  const structuralPathRandomization = nativeStructuralPathRandomizationProjection(run);
  const sorted = [...result.paths].sort((a, b) => Math.abs(b.coefficient) - Math.abs(a.coefficient));
  const strongest = sorted[0];
  findings.push(finding({
    id: `path.strongest.${strongest.source}.${strongest.target}`,
    severity: "info",
    group: "recommended",
    tab: "structural",
    metric: "Path coefficient",
    value: strongest.coefficient.toFixed(4),
    thresholdGuide: "Rank paths by absolute coefficient, then evaluate theory and inference.",
    path: { source: strongest.source, target: strongest.target },
    interpretation: `${pathName(strongest.source, strongest.target)} is the strongest direct path in this run by absolute coefficient (${strongest.coefficient.toFixed(4)}).`,
    recommendedAction: structuralPathRandomization
      ? "Review the corresponding raw pathwise randomization p value and fixed-score assumptions before reporting inference."
      : run.bootstrap
        ? "Review the corresponding inference interval before reporting the direction as supported."
        : "Enable bootstrap or structural path randomization before reporting significance for this path.",
    reportSentence: `The strongest direct path was ${pathName(strongest.source, strongest.target)} (beta = ${strongest.coefficient.toFixed(4)}).`,
    linkedObject: pathObject(strongest.source, strongest.target),
  }));
  for (const path of sorted.filter((row) => row.coefficient < 0).slice(0, 4)) {
    findings.push(finding({
      id: `path.negative.${path.source}.${path.target}`,
      severity: "caution",
      group: "recommended",
      tab: "structural",
      metric: "Path coefficient",
      value: path.coefficient.toFixed(4),
      thresholdGuide: "Negative paths are not automatically invalid, but should match theory and coding.",
      path: { source: path.source, target: path.target },
      interpretation: `${pathName(path.source, path.target)} is negative (${path.coefficient.toFixed(4)}). This may be theoretically meaningful or may indicate reverse coding/suppression.`,
      recommendedAction: "Check construct coding, predictor collinearity, and the expected theoretical direction.",
      reportSentence: `${pathName(path.source, path.target)} was negative (beta = ${path.coefficient.toFixed(4)}); interpretation should be tied to theory and coding checks.`,
      linkedObject: pathObject(path.source, path.target),
    }));
  }
  for (const path of sorted.filter((row) => Math.abs(row.coefficient) < 0.05).slice(0, 3)) {
    findings.push(finding({
      id: `path.near_zero.${path.source}.${path.target}`,
      severity: "info",
      group: "optional",
      tab: "structural",
      metric: "Path coefficient",
      value: path.coefficient.toFixed(4),
      thresholdGuide: "Near-zero is a practical screening guide, not a universal cutoff.",
      path: { source: path.source, target: path.target },
      interpretation: `${pathName(path.source, path.target)} is close to zero (${path.coefficient.toFixed(4)}), so its practical contribution may be limited in this run.`,
      recommendedAction: "Review theory and confidence intervals before deciding whether to retain or discuss this path.",
      reportSentence: `${pathName(path.source, path.target)} was close to zero (beta = ${path.coefficient.toFixed(4)}).`,
      linkedObject: pathObject(path.source, path.target),
    }));
  }
  return findings;
}

function measurementFindings(result: PlsResult, assessment?: AssessmentResult): InterpretationFinding[] {
  const findings: InterpretationFinding[] = [];
  const weak = result.outer_estimates.filter((row) => Math.abs(row.loading) < 0.4);
  const review = result.outer_estimates.filter((row) => Math.abs(row.loading) >= 0.4 && Math.abs(row.loading) < 0.708);
  const strong = result.outer_estimates.filter((row) => Math.abs(row.loading) >= 0.708);
  for (const row of weak.slice(0, 6)) {
    findings.push(finding({
      id: `loading.weak.${row.construct}.${row.indicator}`,
      severity: "issue",
      group: "must",
      tab: "measurement",
      metric: "Outer loading",
      value: row.loading.toFixed(4),
      thresholdGuide: "< .40 is commonly treated as a serious reflective indicator concern.",
      construct: row.construct,
      indicator: row.indicator,
      interpretation: `${row.indicator} has a weak loading on ${row.construct} (${row.loading.toFixed(4)}).`,
      recommendedAction: "Inspect wording, coding, missingness, and theoretical justification before reporting or retaining the indicator.",
      reportSentence: `${row.indicator} showed a weak loading on ${row.construct} (${row.loading.toFixed(4)}), requiring justification or revision.`,
      linkedObject: { type: "indicator", id: row.indicator },
    }));
  }
  for (const row of review.slice(0, 6)) {
    findings.push(finding({
      id: `loading.review.${row.construct}.${row.indicator}`,
      severity: "caution",
      group: "recommended",
      tab: "measurement",
      metric: "Outer loading",
      value: row.loading.toFixed(4),
      thresholdGuide: ".40-.708 should be reviewed with reliability, AVE, and theory.",
      construct: row.construct,
      indicator: row.indicator,
      interpretation: `${row.indicator} loads ${row.loading.toFixed(4)} on ${row.construct}, below the common .708 guide.`,
      recommendedAction: "Check whether composite reliability and AVE remain acceptable before considering indicator removal.",
      reportSentence: `${row.indicator} loaded below .708 (${row.loading.toFixed(4)}), so the indicator was reviewed with reliability and AVE evidence.`,
      linkedObject: { type: "indicator", id: row.indicator },
    }));
  }
  if (strong.length && !weak.length && !review.length) {
    const range = numberRange(strong.map((row) => Math.abs(row.loading)));
    findings.push(finding({
      id: "loading.strong.summary",
      severity: "good",
      group: "report",
      tab: "measurement",
      metric: "Outer loadings",
      value: range,
      thresholdGuide: ">= .708 is a common reflective indicator loading guide.",
      interpretation: `All ${strong.length} outer loadings meet the common .708 guide; loading magnitudes range from ${range}.`,
      recommendedAction: "Still review cross-loadings and discriminant validity before reporting the measurement model.",
      reportSentence: `All outer loadings met the .708 guide, ranging from ${range}.`,
    }));
  }
  const outerVif = assessment?.formative_indicator_vif.filter((row) => (row.vif ?? 0) >= 3.3) ?? [];
  for (const row of outerVif.slice(0, 5)) {
    findings.push(finding({
      id: `outer_vif.${row.construct}.${row.indicator}`,
      severity: (row.vif ?? 0) >= 5 ? "issue" : "caution",
      group: (row.vif ?? 0) >= 5 ? "must" : "recommended",
      tab: "measurement",
      metric: "Outer VIF",
      value: formatNumber(row.vif, 4),
      thresholdGuide: "VIF >= 5 is high; VIF >= 3.3 often deserves review.",
      construct: row.construct,
      indicator: row.indicator,
      interpretation: `${row.indicator} has formative indicator VIF ${formatNumber(row.vif, 4)} for ${row.construct}.`,
      recommendedAction: "Review formative indicator redundancy and conceptual overlap.",
      reportSentence: `${row.indicator} showed formative collinearity requiring review (VIF = ${formatNumber(row.vif, 4)}).`,
      linkedObject: { type: "indicator", id: row.indicator },
    }));
  }
  return findings;
}

function validityFindings(assessment?: AssessmentResult): InterpretationFinding[] {
  if (!assessment) return [];
  const findings: InterpretationFinding[] = [];
  for (const row of assessment.construct_quality) {
    if (row.ave != null && row.ave < 0.5) {
      findings.push(finding({
        id: `ave.low.${row.construct}`,
        severity: "issue",
        group: "must",
        tab: "validity",
        metric: "AVE",
        value: row.ave.toFixed(4),
        thresholdGuide: "AVE below .50 suggests convergent-validity concern.",
        construct: row.construct,
        interpretation: `${row.construct} has AVE ${row.ave.toFixed(4)}, below the common .50 guide.`,
        recommendedAction: "Inspect low-loading indicators and theory before reporting convergent validity.",
        reportSentence: `${row.construct} did not meet the .50 AVE guide (AVE = ${row.ave.toFixed(4)}).`,
        linkedObject: { type: "construct", id: row.construct },
      }));
    }
    const reliabilityIssues = [
      row.cronbach_alpha != null && row.cronbach_alpha < 0.7 ? `alpha ${row.cronbach_alpha.toFixed(4)}` : null,
      row.rho_c != null && row.rho_c < 0.7 ? `rho_C ${row.rho_c.toFixed(4)}` : null,
    ].filter(Boolean);
    if (reliabilityIssues.length) {
      findings.push(finding({
        id: `reliability.low.${row.construct}`,
        severity: "caution",
        group: "must",
        tab: "validity",
        metric: "Reliability",
        value: reliabilityIssues.join(", "),
        thresholdGuide: "Alpha and composite reliability below .70 commonly require review.",
        construct: row.construct,
        interpretation: `${row.construct} has reliability metric(s) below the .70 guide: ${reliabilityIssues.join(", ")}.`,
        recommendedAction: "Review indicator quality and construct definition before reporting reliability as acceptable.",
        reportSentence: `${row.construct} showed reliability values below .70 (${reliabilityIssues.join(", ")}).`,
        linkedObject: { type: "construct", id: row.construct },
      }));
    }
  }
  for (const cell of htmtCells(assessment).filter((item) => item.value >= 0.85).slice(0, 8)) {
    findings.push(finding({
      id: `htmt.${cell.left}.${cell.right}`,
      severity: cell.value >= 0.9 ? "issue" : "caution",
      group: cell.value >= 0.9 ? "must" : "recommended",
      tab: "validity",
      metric: "HTMT+",
      value: cell.value.toFixed(4),
      thresholdGuide: "HTMT >= .90 is a common issue guide; .85-.90 deserves caution in stricter contexts.",
      construct: cell.left,
      interpretation: `HTMT+ between ${cell.left} and ${cell.right} is ${cell.value.toFixed(4)}.`,
      recommendedAction: "Inspect item wording, construct overlap, and theory before claiming discriminant validity.",
      reportSentence: `HTMT+ for ${cell.left} and ${cell.right} was ${cell.value.toFixed(4)}, requiring discriminant-validity review.`,
    }));
  }
  findings.push(...crossLoadingFindings(assessment));
  if (!findings.some((item) => item.tab === "validity")) {
    findings.push(finding({
      id: "validity.no_common_issue",
      severity: "good",
      group: "report",
      tab: "validity",
      metric: "Validity checks",
      value: "no common issue detected",
      thresholdGuide: "Guides checked: reliability .70, AVE .50, HTMT .85/.90, and cross-loading assignment.",
      interpretation: "No common reliability, AVE, HTMT, or cross-loading issue was detected in this run.",
      recommendedAction: "Report the exact values and keep the interpretation tied to the study context.",
      reportSentence: "The reliability, convergent-validity, and discriminant-validity screening checks did not flag a common threshold issue.",
    }));
  }
  return findings;
}

function structuralDiagnosticFindings(result: PlsResult, assessment?: AssessmentResult): InterpretationFinding[] {
  const findings: InterpretationFinding[] = [];
  for (const [construct, value] of Object.entries(result.r_squared)) {
    findings.push(finding({
      id: `r2.${construct}`,
      severity: value >= 0.25 ? "info" : "caution",
      group: value >= 0.25 ? "report" : "recommended",
      tab: "structural",
      metric: "R2",
      value: value.toFixed(4),
      thresholdGuide: "R2 is context-dependent; .25/.50/.75 are descriptive weak/moderate/substantial guides in many PLS-SEM texts.",
      construct,
      interpretation: `${construct} has R2 ${value.toFixed(4)}, a ${r2Label(value)} explanatory-power guide for this run.`,
      recommendedAction: value < 0.25 ? "Review theory, missing predictors, and prediction diagnostics." : "Report with predictor count and avoid treating R2 as model quality by itself.",
      reportSentence: `${construct} had R2 = ${value.toFixed(4)} (${r2Label(value)} guide).`,
      linkedObject: { type: "construct", id: construct },
    }));
  }
  for (const row of assessment?.structural_vif.filter((item) => (item.vif ?? 0) >= 3.3).slice(0, 8) ?? []) {
    const vif = row.vif ?? NaN;
    findings.push(finding({
      id: `vif.${row.predictor_construct}.${row.target_construct}`,
      severity: vif >= 5 ? "issue" : "caution",
      group: vif >= 5 ? "must" : "recommended",
      tab: "structural",
      metric: "Inner VIF",
      value: vif.toFixed(4),
      thresholdGuide: "VIF >= 5 is high; VIF >= 3.3 often deserves review.",
      path: { source: row.predictor_construct, target: row.target_construct },
      interpretation: `${pathName(row.predictor_construct, row.target_construct)} has inner VIF ${vif.toFixed(4)}.`,
      recommendedAction: "Review predictor overlap, theory, and possible redundancy before interpreting path estimates.",
      reportSentence: `${pathName(row.predictor_construct, row.target_construct)} showed collinearity requiring review (VIF = ${vif.toFixed(4)}).`,
      linkedObject: pathObject(row.predictor_construct, row.target_construct),
    }));
  }
  const strongestF2 = [...(assessment?.f_squared ?? [])].filter((row) => row.f_squared != null).sort((a, b) => (b.f_squared ?? 0) - (a.f_squared ?? 0))[0];
  if (strongestF2?.f_squared != null) {
    findings.push(finding({
      id: `f2.strongest.${strongestF2.source_construct}.${strongestF2.target_construct}`,
      severity: "info",
      group: "recommended",
      tab: "structural",
      metric: "f2",
      value: strongestF2.f_squared.toFixed(4),
      thresholdGuide: ".02/.15/.35 are common small/medium/large f2 guides.",
      path: { source: strongestF2.source_construct, target: strongestF2.target_construct },
      interpretation: `${pathName(strongestF2.source_construct, strongestF2.target_construct)} has the largest f2 in this run (${strongestF2.f_squared.toFixed(4)}, ${f2Label(strongestF2.f_squared)} guide).`,
      recommendedAction: "Use f2 alongside coefficient size, inference, and theory when describing practical relevance.",
      reportSentence: `The largest f2 was observed for ${pathName(strongestF2.source_construct, strongestF2.target_construct)} (${strongestF2.f_squared.toFixed(4)}).`,
      linkedObject: pathObject(strongestF2.source_construct, strongestF2.target_construct),
    }));
  }
  return findings;
}

function inferenceFindings(run: AnalysisRun): InterpretationFinding[] {
  const result: InterpretationFinding[] = [];
  const processProjection = nativeProcessResultProjection(run);
  const processBootstrap = processProjection?.bootstrap ?? null;
  if (processBootstrap) {
    result.push(finding({
      id: "inference.process_bootstrap",
      severity: "info",
      group: "report",
      tab: "inference",
      metric: "PROCESS bootstrap inference",
      value: `${processBootstrap.usable_replicates} of ${processBootstrap.requested_replicates} usable`,
      thresholdGuide: "Report PROCESS percentile and available BCa intervals with the exact seed, workers, failed-replicate disclosures, and original raw probe grid.",
      interpretation: `This graph-defined PROCESS run includes ${processBootstrap.usable_replicates} usable indexed case-bootstrap replicates of ${processBootstrap.requested_replicates} requested; it is not a point-estimates-only run.`,
      recommendedAction: "Report effect-specific PROCESS bootstrap intervals and any tagged unavailable test or BCa state from the dedicated tables.",
      reportSentence: `Graph-defined PROCESS inference used ${processBootstrap.usable_replicates} usable indexed case-bootstrap replicates of ${processBootstrap.requested_replicates} requested with seed ${processBootstrap.seed}.`,
    }));
    return result;
  }
  if (processProjection) {
    result.push(finding({
      id: "inference.process_bootstrap_missing",
      severity: "unavailable",
      group: "must",
      tab: "inference",
      metric: "PROCESS case-bootstrap intervals",
      value: "not run",
      thresholdGuide: "PROCESS v2 coefficient rows retain HC3 Student-t inference, while effect-specific percentile or BCa intervals require the dedicated PROCESS case-bootstrap layer.",
      interpretation: "This graph-defined PROCESS run has no PROCESS case-bootstrap intervals; it is not reinterpreted through generic PLS bootstrap or permutation output.",
      recommendedAction: "Enable PROCESS case bootstrapping and rerun before making bootstrap interval claims for direct, indirect, conditional, or moderated-mediation effects.",
      reportSentence: "No PROCESS case-bootstrap layer was run, so effect-specific percentile or BCa interval claims were not made.",
    }));
    return result;
  }
  const regressionBootstrap = nativeRegressionBootstrapResultProjection(run);
  if (regressionBootstrap) {
    result.push(finding({
      id: "inference.regression_bootstrap",
      severity: "info",
      group: "report",
      tab: "inference",
      metric: "Regression bootstrap inference",
      value: `${regressionBootstrap.usable_replicates} of ${regressionBootstrap.requested_replicates} usable`,
      thresholdGuide: "Report coefficient percentile and available BCa intervals with the exact regression-bootstrap seed, workers, and failed-replicate disclosures.",
      interpretation: `This standalone regression run includes ${regressionBootstrap.usable_replicates} usable indexed case-bootstrap replicates of ${regressionBootstrap.requested_replicates} requested; it is not a point-estimates-only run.`,
      recommendedAction: "Report coefficient-specific regression-bootstrap intervals and any tagged unavailable test or BCa state from the dedicated tables.",
      reportSentence: `Standalone regression inference used ${regressionBootstrap.usable_replicates} usable indexed case-bootstrap replicates of ${regressionBootstrap.requested_replicates} requested with seed ${regressionBootstrap.seed}.`,
    }));
    return result;
  }
  const structuralPathRandomization = nativeStructuralPathRandomizationProjection(run);
  if (!run.bootstrap && !structuralPathRandomization) {
    result.push(finding({
      id: "inference.missing",
      severity: "unavailable",
      group: "must",
      tab: "inference",
      metric: "Inference",
      value: "not run",
      thresholdGuide: "Path p values and confidence intervals require bootstrap or permutation outputs.",
      interpretation: "This run has point estimates only; no bootstrap or permutation inference is available.",
      recommendedAction: "Enable bootstrap or permutation in Setup and rerun before reporting significance, p values, or confidence intervals.",
      reportSentence: "No bootstrap or permutation inference was run, so significance and confidence-interval claims were not made from this run.",
    }));
    return result;
  }
  for (const parameter of run.bootstrap?.percentile.parameters.slice(0, 10) ?? []) {
    const excludesZero = intervalExcludesZero(parameter.lower, parameter.upper);
    result.push(finding({
      id: `bootstrap.percentile.${parameter.parameter}`,
      severity: excludesZero ? "good" : "caution",
      group: excludesZero ? "report" : "recommended",
      tab: "inference",
      metric: "Bootstrap percentile CI",
      value: `${parameter.lower.toFixed(4)} to ${parameter.upper.toFixed(4)}`,
      thresholdGuide: "If the confidence interval excludes zero, the direction is supported under the selected bootstrap procedure.",
      interpretation: `${parameter.parameter} has percentile CI ${parameter.lower.toFixed(4)} to ${parameter.upper.toFixed(4)}, which ${excludesZero ? "excludes" : "includes"} zero.`,
      recommendedAction: excludesZero ? "Report the interval with bootstrap settings and seed." : "Treat the direction as inconclusive unless another justified inference procedure supports it.",
      reportSentence: `${parameter.parameter} percentile bootstrap CI was [${parameter.lower.toFixed(4)}, ${parameter.upper.toFixed(4)}].`,
    }));
  }
  if (structuralPathRandomization) {
    const smallest = [...structuralPathRandomization.parameters].sort((a, b) => a.pValueTwoSided - b.pValueTwoSided)[0];
    if (smallest) {
      result.push(finding({
        id: `permutation.${smallest.parameter}`,
        severity: "caution",
        group: "report",
        tab: "inference",
        metric: "Raw two-sided structural path randomization p",
        value: smallest.pValueTwoSided.toFixed(4),
        thresholdGuide: NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING,
        path: { source: smallest.source, target: smallest.target },
        interpretation: `${pathName(smallest.source, smallest.target)} has the smallest raw pathwise two-sided plus-one p value in this run (${smallest.pValueTwoSided.toFixed(4)} across ${smallest.permutations} permutations). The test conditions on fixed original PLS construct scores, assumes exchangeable reduced-model residuals, and is unadjusted for multiplicity.`,
        recommendedAction: "Report the path, exceedance count, permutation count, seed, fixed-score estimand, residual-exchangeability assumption, and lack of multiplicity adjustment; do not describe this as measurement-model re-estimation or a group comparison.",
        reportSentence: `${pathName(smallest.source, smallest.target)} had raw pathwise two-sided randomization p = ${smallest.pValueTwoSided.toFixed(4)} (${smallest.exceedances} exceedances in ${smallest.permutations} permutations), conditional on fixed original PLS construct scores and exchangeable reduced-model residuals, without multiplicity adjustment.`,
        linkedObject: pathObject(smallest.source, smallest.target),
      }));
    }
  }
  return result;
}

function mediationModerationFindings(result: PlsResult): InterpretationFinding[] {
  const findings: InterpretationFinding[] = [];
  for (const row of result.mediation?.estimates ?? []) {
    findings.push(finding({
      id: `mediation.${row.source}.${row.target}`,
      severity: row.classification === "no_effect" ? "info" : row.variance_accounted_for != null && row.variance_accounted_for > 1 ? "caution" : "info",
      group: "recommended",
      tab: "structural",
      metric: "Mediation effect",
      value: `indirect ${row.indirect.toFixed(4)}`,
      thresholdGuide: "VAF and mediation classes are descriptive; bootstrap inference is needed for indirect-effect claims.",
      path: { source: row.source, target: row.target },
      interpretation: `${pathName(row.source, row.target)} has indirect effect ${row.indirect.toFixed(4)} and class ${formatCode(row.classification)}.`,
      recommendedAction: "Inspect bootstrap intervals for indirect effects before reporting mediation support.",
      reportSentence: `${pathName(row.source, row.target)} showed ${formatCode(row.classification)} mediation with indirect effect ${row.indirect.toFixed(4)}.`,
      linkedObject: pathObject(row.source, row.target),
    }));
  }
  for (const row of result.moderation?.estimates ?? []) {
    findings.push(finding({
      id: `moderation.${row.product_construct}.${row.outcome}`,
      severity: "info",
      group: "recommended",
      tab: "structural",
      metric: "Moderation effect",
      value: row.interaction_effect.toFixed(4),
      thresholdGuide: "Interpret interaction effects with simple slopes and bootstrap inference where available.",
      path: { source: row.product_construct, target: row.outcome },
      interpretation: `${row.interaction} has interaction effect ${row.interaction_effect.toFixed(4)} on ${row.outcome}.`,
      recommendedAction: "Review simple slopes and confidence intervals before describing moderation.",
      reportSentence: `${row.interaction} was estimated as a moderation term for ${row.outcome} (interaction coefficient = ${row.interaction_effect.toFixed(4)}).`,
      linkedObject: pathObject(row.product_construct, row.outcome),
    }));
  }
  return findings;
}

function predictionFindings(result: PlsResult, assessment?: AssessmentResult): InterpretationFinding[] {
  const findings: InterpretationFinding[] = [];
  for (const row of assessment?.blindfolding?.constructs ?? []) {
    if (row.q_squared == null) continue;
    findings.push(finding({
      id: `q2.${row.construct}`,
      severity: row.q_squared > 0 ? "good" : "caution",
      group: "recommended",
      tab: "prediction",
      metric: "Q2",
      value: row.q_squared.toFixed(4),
      thresholdGuide: "Q2 above zero is directional predictive-relevance evidence, not proof of overall model quality.",
      construct: row.construct,
      interpretation: `${row.construct} has blindfolding Q2 ${row.q_squared.toFixed(4)}.`,
      recommendedAction: row.q_squared > 0 ? "Report with omission distance and prediction settings." : "Review predictor set and consider PLSpredict if prediction is a research objective.",
      reportSentence: `${row.construct} had Q2 = ${row.q_squared.toFixed(4)} under blindfolding.`,
      linkedObject: { type: "construct", id: row.construct },
    }));
  }
  if (result.predict?.targets.length) {
    const better = result.predict.targets.filter((row) => row.q_squared_predict != null && row.q_squared_predict > 0);
    findings.push(finding({
      id: "plspredict.summary",
      severity: better.length ? "info" : "caution",
      group: "recommended",
      tab: "prediction",
      metric: "PLSpredict",
      value: `${better.length}/${result.predict.targets.length} targets with Q2 predict > 0`,
      thresholdGuide: "Compare PLS prediction errors against benchmark models and CVPAT where available.",
      interpretation: `${better.length} of ${result.predict.targets.length} PLSpredict target(s) have Q2 predict above zero.`,
      recommendedAction: "Inspect RMSE/MAE against LM and benchmark columns before claiming predictive relevance.",
      reportSentence: `PLSpredict reported ${better.length} of ${result.predict.targets.length} target(s) with Q2 predict above zero.`,
    }));
  }
  return findings;
}

function methodPayloadFindings(result: PlsResult): InterpretationFinding[] {
  const findings: InterpretationFinding[] = [];
  if (result.micom) findings.push(genericPayloadFinding("micom", "MICOM", "groups", "Review configural, compositional, mean, and variance invariance before group comparisons."));
  if (result.mga || result.mga_permutation) findings.push(genericPayloadFinding("mga", "MGA", "groups", "Interpret group path differences only after checking MICOM and permutation settings."));
  if (result.fimix) findings.push(genericPayloadFinding("fimix", "Experimental FIMIX-style diagnostic", "groups", "Treat inverse-distance memberships and pseudo-likelihood criteria as diagnostics, not posterior probabilities or full finite-mixture EM/FIMIX-PLS results."));
  if (result.segmentation) findings.push(genericPayloadFinding("pls_pos", "Experimental PLS-POS-style diagnostic", "groups", "Review objective history, segment size, and path stability only within the deterministic score-space routine; this is not a full published PLS-POS implementation."));
  if (result.ipma) findings.push(genericPayloadFinding("ipma", "IPMA", "groups", "Prioritize high-importance, lower-performance constructs or indicators for managerial interpretation."));
  if (result.regression) findings.push(genericPayloadFinding("regression", result.regression.regression_type === "logistic" ? "Logistic regression" : "Regression", "diagnostics", "Report coefficients with standard errors, intervals, fit metrics, and any analysis requirements or cautions."));
  if (result.nca) findings.push(genericPayloadFinding("nca", "NCA", "diagnostics", "Interpret necessity effect sizes and bottleneck rows only for numeric X/Y CE-FDH and CR-FDH analyses using observed ranges."));
  if (result.cbsem) findings.push(genericPayloadFinding("cbsem", "CB-SEM/CFA", "diagnostics", "Review ML convergence, fit indices, standardized solution, residuals, and modification-index diagnostics."));
  if (result.gsca) findings.push(genericPayloadFinding("gsca", "GSCA", "diagnostics", "Review component estimates, fit diagnostics, and the reported limitations."));
  if (result.pca) findings.push(genericPayloadFinding("pca", "PCA", "diagnostics", "Interpret retained components through eigenvalues, explained variance, and loadings."));
  return findings;
}

function diagramAdvisorFindings(context: ResultInterpretationContext, result: PlsResult): InterpretationFinding[] {
  const nodes = context.nodes ?? [];
  const edges = (context.edges ?? []).filter((edge) => edge.data?.role !== "covariance");
  const findings: InterpretationFinding[] = [];
  const incoming = new Map<string, number>();
  const outgoing = new Map<string, number>();
  for (const edge of edges) {
    outgoing.set(edge.source, (outgoing.get(edge.source) ?? 0) + 1);
    incoming.set(edge.target, (incoming.get(edge.target) ?? 0) + 1);
  }
  for (const [target, count] of incoming) {
    if (count >= 3) {
      findings.push(finding({
        id: `advisor.multiple_predictors.${target}`,
        severity: "info",
        group: "recommended",
        tab: "interpretation",
        metric: "SEM diagram advisor",
        value: `${count} predictors`,
        thresholdGuide: "Multiple predictors increase the importance of collinearity and effect-size checks.",
        construct: target,
        interpretation: `${target} has ${count} incoming structural paths in the SEM diagram.`,
        recommendedAction: "Review inner VIF and f2 for each predictor targeting this construct.",
        reportSentence: `${target} was modeled with ${count} direct predictors; collinearity and effect-size diagnostics were reviewed.`,
        linkedObject: { type: "construct", id: target },
      }));
    }
  }
  if (hasMediationShape(edges)) {
    findings.push(finding({
      id: "advisor.mediation_shape",
      severity: "info",
      group: "recommended",
      tab: "interpretation",
      metric: "SEM diagram advisor",
      value: "mediation-like path chain",
      thresholdGuide: "A path chain suggests indirect effects may be theoretically relevant.",
      interpretation: "The diagram contains at least one predictor -> mediator -> outcome chain.",
      recommendedAction: result.mediation?.estimates.length ? "Inspect the mediation effects table and bootstrap intervals." : "Enable or inspect indirect-effect decomposition before reporting mediation-like hypotheses.",
      reportSentence: "The model included an indirect-effect path chain, so mediation-related evidence was reviewed.",
    }));
  }
  if (nodes.some((node) => node.data?.semantic === "interaction")) {
    findings.push(finding({
      id: "advisor.moderation_shape",
      severity: "info",
      group: "recommended",
      tab: "interpretation",
      metric: "SEM diagram advisor",
      value: "interaction construct present",
      thresholdGuide: "Interactions should be interpreted with simple slopes and inference where available.",
      interpretation: "The SEM diagram includes an interaction construct.",
      recommendedAction: "Inspect moderation effects and simple slopes before reporting conditional effects.",
      reportSentence: "The model included an interaction term, so moderation evidence and simple slopes were reviewed.",
    }));
  }
  const formative = nodes.filter((node) => node.data?.mode === "formative");
  if (formative.length) {
    findings.push(finding({
      id: "advisor.formative_blocks",
      severity: "info",
      group: "recommended",
      tab: "interpretation",
      metric: "SEM diagram advisor",
      value: `${formative.length} formative construct(s)`,
      thresholdGuide: "Formative blocks use weights and VIF, not reflective loading/reliability rules.",
      interpretation: `The diagram has ${formative.length} formative construct(s).`,
      recommendedAction: "Use indicator weights and collinearity diagnostics for formative blocks.",
      reportSentence: `${formative.length} formative construct(s) were interpreted with formative measurement diagnostics.`,
    }));
  }
  if (!context.run.bootstrap && result.paths.length) {
    findings.push(finding({
      id: "advisor.enable_bootstrap",
      severity: "caution",
      group: "must",
      tab: "interpretation",
      metric: "SEM diagram advisor",
      value: "bootstrap off",
      thresholdGuide: "Inference claims need resampling or another documented inference procedure.",
      interpretation: "The diagram has structural paths, but the selected run has no bootstrap output.",
      recommendedAction: "Enable bootstrap before reporting significance for paths, loadings, or indirect effects.",
      reportSentence: "Bootstrap was not run, so the interpretation is limited to point estimates.",
    }));
  }
  if (!result.predict && result.paths.length) {
    findings.push(finding({
      id: "advisor.plspredict",
      severity: "info",
      group: "optional",
      tab: "interpretation",
      metric: "SEM diagram advisor",
      value: "prediction not run",
      thresholdGuide: "Prediction checks are optional unless prediction is a research objective.",
      interpretation: "No PLSpredict payload is present for this structural model.",
      recommendedAction: "Run PLSpredict if prediction is part of the research objective.",
      reportSentence: "Prediction-oriented assessment was not included unless required by the study objective.",
    }));
  }
  return findings;
}

function reportParagraphs(run: AnalysisRun, result: PlsResult, findings: InterpretationFinding[]) {
  const process = nativeProcessResultProjection(run);
  if (process) return processReportParagraphs(run, process, findings);
  const legacyProcess = nativeLegacyProcessResultProjection(run);
  if (legacyProcess) return historicalProcessReportParagraphs(run, legacyProcess);
  const regressionBootstrap = nativeRegressionBootstrapResultProjection(run);
  if (regressionBootstrap) return regressionBootstrapReportParagraphs(run, regressionBootstrap, findings);
  const structuralPathRandomization = nativeStructuralPathRandomizationProjection(run);
  const strongestPath = [...result.paths].sort((a, b) => Math.abs(b.coefficient) - Math.abs(a.coefficient))[0];
  const r2Rows = Object.entries(result.r_squared).sort((a, b) => b[1] - a[1]);
  const loadingValues = result.outer_estimates.map((row) => Math.abs(row.loading));
  const issues = findings.filter((item) => item.severity === "issue");
  const cautions = findings.filter((item) => item.severity === "caution" || item.severity === "unavailable");
  return [
    {
      section: "Model and provenance",
      text: `${run.name} was estimated with ${run.method} using ${result.used_observations} observations, seed ${run.seed}, and fingerprint ${run.fingerprint}. Analysis details: ${scopeText(run)}.`,
      sourceFindingIds: [],
    },
    {
      section: "Measurement interpretation",
      text: loadingValues.length
        ? `Outer loading magnitudes ranged from ${numberRange(loadingValues)}. ${findings.some((item) => item.id.startsWith("loading.weak")) ? "At least one indicator was below .40 and requires justification or revision." : findings.some((item) => item.id.startsWith("loading.review")) ? "Some indicators were below .708 and should be interpreted with reliability, AVE, and theory." : "All available loadings met the common .708 guide."}`
        : "No loading interpretation was generated because no outer estimates were available.",
      sourceFindingIds: findings.filter((item) => item.id.startsWith("loading.")).map((item) => item.id),
    },
    {
      section: "Structural model interpretation",
      text: `${strongestPath ? `The strongest direct path was ${pathName(strongestPath.source, strongestPath.target)} (beta = ${strongestPath.coefficient.toFixed(4)}). ` : ""}${r2Rows.length ? `The highest R2 was ${r2Rows[0][1].toFixed(4)} for ${r2Rows[0][0]}.` : "No endogenous R2 values were available."}`,
      sourceFindingIds: findings.filter((item) => item.tab === "structural").map((item) => item.id),
    },
    {
      section: "Inference caveat",
      text: structuralPathRandomization
        ? `${NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING} Report each raw p value with its exceedance count, ${structuralPathRandomization.permutations} permutations, and seed ${structuralPathRandomization.masterSeed}.`
        : run.bootstrap
          ? "Inference should be reported with the configured bootstrap procedure, confidence level, seed, and any failed or unavailable intervals."
          : "This run does not include bootstrap or current structural path randomization inference; avoid p-value or confidence-interval claims from this run.",
      sourceFindingIds: findings.filter((item) => item.tab === "inference").map((item) => item.id),
    },
    {
      section: "Reporting checks",
      text: issues.length || cautions.length
        ? `Before reporting, address ${issues.length} issue(s) and review ${cautions.length} caution/unavailable finding(s).`
        : "No issue-level findings were detected by the deterministic interpretation rules; report exact values and study-specific justification.",
      sourceFindingIds: [...issues, ...cautions].map((item) => item.id),
    },
  ];
}

function regressionBootstrapReportParagraphs(
  run: AnalysisRun,
  bootstrap: NonNullable<ReturnType<typeof nativeRegressionBootstrapResultProjection>>,
  findings: InterpretationFinding[],
) {
  const regression = run.result!.regression!;
  const nonIntercept = regression.coefficients.filter((coefficient) => coefficient.term !== "intercept");
  const strongest = [...nonIntercept].sort((left, right) => Math.abs(right.estimate) - Math.abs(left.estimate))[0];
  const fitDisclosure = regression.regression_type === "logistic"
    ? `McFadden pseudo-R-squared ${regression.fit?.pseudo_r_squared?.toFixed(4) ?? "not available"}`
    : `R-squared ${regression.fit?.r_squared?.toFixed(4) ?? "not available"}`;
  return [
    {
      section: "Model and provenance",
      text: `${run.name} used standalone ${regression.regression_type === "logistic" ? "binary logistic" : "OLS"} regression for outcome ${regression.outcome} with ${regression.observations} observations, seed ${run.seed}, and fingerprint ${run.fingerprint}.`,
      sourceFindingIds: [],
    },
    {
      section: "Regression model",
      text: `${fitDisclosure}.${strongest ? ` The largest non-intercept coefficient by absolute value was ${strongest.term} (${strongest.estimate.toFixed(4)}).` : ""}`,
      sourceFindingIds: [],
    },
    {
      section: "Inference caveat",
      text: `This standalone regression run includes ${bootstrap.usable_replicates} usable indexed case-bootstrap replicates of ${bootstrap.requested_replicates} requested. Report coefficient percentile and available BCa intervals with seed ${bootstrap.seed}, worker count ${bootstrap.workers}, and ${bootstrap.failed_replicates.length} failed replicate(s).`,
      sourceFindingIds: findings.filter((item) => item.tab === "inference").map((item) => item.id),
    },
    {
      section: "Reporting checks",
      text: "Report the regression type, coefficient scale, fit statistic, resampling settings, and all tagged unavailable intervals; do not reinterpret standalone regression through PLS measurement-model rules.",
      sourceFindingIds: findings.map((item) => item.id),
    },
  ];
}

function historicalProcessFinding(
  process: NonNullable<ReturnType<typeof nativeLegacyProcessResultProjection>>,
): InterpretationFinding {
  return finding({
    id: "process_v1.historical_read_only",
    severity: "unavailable",
    group: "must",
    tab: "overview",
    metric: "Historical PROCESS v1 interpretation",
    value: "read-only",
    thresholdGuide: "Historical regression_process_v1 output remains readable under its original label and is never reinterpreted as current graph-defined PROCESS output.",
    interpretation: `This ${process.model.replaceAll("_", " ")} result is a historical PROCESS v1 archive. QuickPLS displays its recorded values without applying generic PLS or current PROCESS v2 interpretation rules.`,
    recommendedAction: "Create and run a graph-defined PROCESS v2 analysis when a current interpretation is required.",
    reportSentence: "Historical PROCESS v1 output was retained as a readable, read-only archive and was not reinterpreted under current methods.",
  });
}

function historicalProcessReportParagraphs(
  run: AnalysisRun,
  process: NonNullable<ReturnType<typeof nativeLegacyProcessResultProjection>>,
) {
  return [{
    section: "Historical archive disclosure",
    text: `${run.name} contains historical ${process.methodVersion} ${process.model.replaceAll("_", " ")} output. Recorded effect and slope rows remain readable under their original version label, but they are not reinterpreted as generic PLS or current PROCESS v2 results.`,
    sourceFindingIds: ["process_v1.historical_read_only"],
  }];
}

function processReportParagraphs(
  run: AnalysisRun,
  process: NonNullable<ReturnType<typeof nativeProcessResultProjection>>,
  findings: InterpretationFinding[],
) {
  const issues = findings.filter((item) => item.severity === "issue");
  const cautions = findings.filter((item) => item.severity === "caution" || item.severity === "unavailable");
  const equationSummary = process.graph.equations
    .map((equation) => `${equation.outcome} R-squared ${equation.fit.r_squared.toFixed(4)}`)
    .join("; ");
  const direct = process.graph.reference_effects.find((effect) => effect.kind === "direct");
  const totalIndirect = process.graph.reference_effects.find((effect) => effect.kind === "total_indirect");
  return [
    {
      section: "Model and provenance",
      text: `${run.name} used the graph-defined PROCESS v2 workflow with ${process.observations} global complete cases, ${process.omittedObservations} omitted rows, ${process.graph.equations.length} OLS equation(s), HC3 covariance, seed ${run.seed}, and fingerprint ${run.fingerprint}.`,
      sourceFindingIds: [],
    },
    {
      section: "Graph-defined path analysis",
      text: `${equationSummary || "No supported equation-fit rows were available."}${direct ? ` Reference direct effect ${direct.effect_id} was ${direct.estimate.toFixed(4)}.` : ""}${totalIndirect ? ` Reference total indirect effect was ${totalIndirect.estimate.toFixed(4)}.` : ""} Reference effects are evaluated at the persisted original-sample moderator conditions disclosed in the result tables.`,
      sourceFindingIds: findings.filter((item) => item.tab === "structural").map((item) => item.id),
    },
    {
      section: "Inference caveat",
      text: process.bootstrap
        ? `This graph-defined PROCESS run includes ${process.bootstrap.usable_replicates} usable indexed case-bootstrap replicates of ${process.bootstrap.requested_replicates} requested. Report dedicated percentile and available BCa intervals with seed ${process.bootstrap.seed}, worker count ${process.bootstrap.workers}, and any failed or unavailable states.`
        : "This graph-defined PROCESS run includes equation-specific HC3 Student-t coefficient inference and persisted simple-slope or Johnson-Neyman diagnostics where applicable, but no PROCESS case-bootstrap intervals.",
      sourceFindingIds: findings.filter((item) => item.tab === "inference").map((item) => item.id),
    },
    {
      section: "Reporting checks",
      text: issues.length || cautions.length
        ? `Before reporting, address ${issues.length} issue(s) and review ${cautions.length} caution/unavailable finding(s).`
        : "No issue-level findings were detected by the deterministic PROCESS interpretation rules; report exact values, reference conditions, and study-specific justification.",
      sourceFindingIds: [...issues, ...cautions].map((item) => item.id),
    },
  ];
}

function genericPayloadFinding(id: string, label: string, tab: ResultWorkspaceTab, action: string): InterpretationFinding {
  return finding({
    id: `payload.${id}`,
    severity: "info",
    group: "optional",
    tab,
    metric: label,
    value: "available",
    thresholdGuide: "Use the method-specific requirements and warnings.",
    interpretation: `${label} output is present in this run.`,
    recommendedAction: action,
    reportSentence: `${label} results were reviewed according to the listed requirements.`,
  });
}

function crossLoadingFindings(assessment: AssessmentResult): InterpretationFinding[] {
  const byIndicator = new Map<string, AssessmentResult["cross_loadings"]>();
  for (const row of assessment.cross_loadings) {
    byIndicator.set(row.indicator, [...(byIndicator.get(row.indicator) ?? []), row]);
  }
  const findings: InterpretationFinding[] = [];
  for (const [indicator, rows] of byIndicator) {
    const assigned = rows.find((row) => row.construct === row.assigned_construct);
    const best = [...rows].sort((a, b) => Math.abs(b.loading) - Math.abs(a.loading))[0];
    if (assigned && best && best.construct !== assigned.assigned_construct && Math.abs(best.loading) > Math.abs(assigned.loading) + 1e-9) {
      findings.push(finding({
        id: `cross_loading.${indicator}`,
        severity: "issue",
        group: "must",
        tab: "validity",
        metric: "Cross-loading",
        value: `${best.construct} ${best.loading.toFixed(4)} > assigned ${assigned.loading.toFixed(4)}`,
        thresholdGuide: "An indicator should usually load highest on its assigned construct.",
        construct: assigned.assigned_construct,
        indicator,
        interpretation: `${indicator} loads higher on ${best.construct} (${best.loading.toFixed(4)}) than on assigned construct ${assigned.assigned_construct} (${assigned.loading.toFixed(4)}).`,
        recommendedAction: "Inspect item wording and discriminant validity before reporting the measurement model.",
        reportSentence: `${indicator} showed a cross-loading concern: ${best.construct} ${best.loading.toFixed(4)} versus assigned ${assigned.assigned_construct} ${assigned.loading.toFixed(4)}.`,
        linkedObject: { type: "indicator", id: indicator },
      }));
    }
  }
  return findings.slice(0, 8);
}

function htmtCells(assessment: AssessmentResult) {
  const artifact = assessment.htmt_plus ?? assessment.htmt;
  if (!artifact) return [];
  if ("cells" in artifact) {
    return artifact.cells.flatMap((row, rowIndex) => row.map((cell, columnIndex) => ({
      left: artifact.constructs[rowIndex],
      right: artifact.constructs[columnIndex],
      value: cell.value ?? NaN,
      diagonal: rowIndex === columnIndex,
      rowIndex,
      columnIndex,
    }))).filter((cell) => !cell.diagonal && cell.rowIndex < cell.columnIndex && Number.isFinite(cell.value));
  }
  return artifact.values.flatMap((row, rowIndex) => row.map((value, columnIndex) => ({
    left: artifact.constructs[rowIndex],
    right: artifact.constructs[columnIndex],
    value: value ?? NaN,
    diagonal: rowIndex === columnIndex,
    rowIndex,
    columnIndex,
  }))).filter((cell) => !cell.diagonal && cell.rowIndex < cell.columnIndex && Number.isFinite(cell.value));
}

function sortFindings(findings: InterpretationFinding[]) {
  return dedupeFindings(findings)
    .sort((a, b) => severityRank(a.severity) - severityRank(b.severity) || groupRank(a.group) - groupRank(b.group) || a.metric.localeCompare(b.metric));
}

function dedupeFindings(findings: InterpretationFinding[]) {
  const seen = new Set<string>();
  return findings
    .filter((item) => {
      const key = canonicalFindingKey(item);
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
}

function canonicalFindingKey(item: InterpretationFinding) {
  const htmtPair = htmtPairKey(item);
  if (htmtPair) return `${item.tab}|${item.metric}|${htmtPair}|${item.severity}`;
  const objectKey = item.path
    ? `path:${item.path.source}->${item.path.target}`
    : item.indicator
      ? `indicator:${item.indicator}`
      : item.construct
        ? `construct:${item.construct}`
        : item.linkedObject
          ? `${item.linkedObject.type}:${item.linkedObject.id}`
          : item.id.replace(/\.\d+$/, "");
  return `${item.tab}|${item.metric}|${objectKey}|${item.value}|${item.recommendedAction}`;
}

function htmtPairKey(item: InterpretationFinding) {
  if (!/^HTMT/i.test(item.metric)) return null;
  const idPair = item.id.match(/htmt[^.]*\.([^.]+)\.([^.]+)/i);
  if (idPair) return [idPair[1], idPair[2]].sort().join("|");
  const textPair = item.interpretation.match(/between (.+?) and (.+?) is /i);
  if (textPair) return [textPair[1], textPair[2]].sort().join("|");
  return null;
}

function finding(item: InterpretationFinding): InterpretationFinding {
  return item;
}

function pathObject(source: string, target: string): InterpretationFinding["linkedObject"] {
  return { type: "path", id: `${source}->${target}`, source, target };
}

function pathName(source: string, target: string) {
  return `${source} -> ${target}`;
}

function numberRange(values: number[]) {
  const finite = values.filter(Number.isFinite);
  if (!finite.length) return "not available";
  return `${Math.min(...finite).toFixed(4)} to ${Math.max(...finite).toFixed(4)}`;
}

function formatNumber(value: number | null | undefined, digits: number) {
  return value == null || !Number.isFinite(value) ? "N/A" : value.toFixed(digits);
}

function intervalExcludesZero(lower: number | null | undefined, upper: number | null | undefined) {
  return lower != null && upper != null && ((lower > 0 && upper > 0) || (lower < 0 && upper < 0));
}

function loadingStatus(value: number) {
  const abs = Math.abs(value);
  if (abs >= 0.708) return "strong";
  if (abs >= 0.4) return "review";
  return "weak";
}

function loadingInterpretation(status: string) {
  if (status === "strong") return "This meets the common .708 reflective loading guide";
  if (status === "review") return "This is below .708 and should be reviewed with reliability, AVE, and theory";
  return "This is below .40 and is a serious indicator concern unless strongly justified";
}

function vifInterpretation(value: number) {
  if (value >= 5) return "This is high and should be addressed before reporting the path as stable";
  if (value >= 3.3) return "This deserves collinearity review";
  return "This is below the common review thresholds";
}

function r2Label(value: number) {
  if (value >= 0.75) return "substantial";
  if (value >= 0.5) return "moderate";
  if (value >= 0.25) return "weak-to-moderate";
  return "weak";
}

function f2Label(value: number) {
  if (value >= 0.35) return "large";
  if (value >= 0.15) return "medium";
  if (value >= 0.02) return "small";
  return "very small";
}

function formatCode(code: string) {
  return code.replaceAll("_", " ");
}

function scopeText(run: AnalysisRun) {
  if (nativeStructuralPathRandomizationProjection(run)) {
    return "Supported single-model fixed-score structural path randomization with conditional, approximate inference under exchangeable reduced-model residuals; see Method Details for assumptions and limitations";
  }
  return (run.warnings[0] ?? "See Method Details for requirements and known limitations.").replace(/QuickPLS v\d+\.\d+\.\d+ supported scope/g, "the supported setup for this method"); // customer-copy-lint: allow-internal
}

function severityLabel(severity: InterpretationSeverity) {
  if (severity === "good") return "Good";
  if (severity === "caution") return "Caution";
  if (severity === "issue") return "Issue";
  if (severity === "unavailable") return "Unavailable";
  return "Info";
}

function severityRank(severity: InterpretationSeverity) {
  return ({ issue: 0, unavailable: 1, caution: 2, info: 3, good: 4 } as Record<InterpretationSeverity, number>)[severity];
}

function groupRank(group: InterpretationGroup) {
  return ({ must: 0, recommended: 1, optional: 2, report: 3 } as Record<InterpretationGroup, number>)[group];
}

function hasMediationShape(edges: SemDiagramEdgeLike[]) {
  for (const first of edges) {
    for (const second of edges) {
      if (first.target === second.source && first.source !== second.target) return true;
    }
  }
  return false;
}
