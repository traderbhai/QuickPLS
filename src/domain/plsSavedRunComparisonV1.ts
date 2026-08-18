import {
  capabilityCellReferenceIdentityV2,
  type CanonicalMissingCell,
  type CanonicalResultCell,
  type CanonicalResultDocumentV2,
  type CanonicalResultRow,
  type CanonicalResultTable,
  validateCanonicalResultDocumentV2,
} from "./canonicalResultDocumentV2";

/**
 * Internal/Labs descriptive projection for two saved PLS result documents.
 *
 * This is deliberately not the analytical `pls_model_comparison_v1` method:
 * it does not refit either model, create common folds, or calculate a paired
 * between-model CVPAT statistic. It only aligns already stored canonical data.
 */
export const PLS_SAVED_RUN_COMPARISON_V1_SCHEMA_VERSION = 1 as const;
export const PLS_SAVED_RUN_COMPARISON_V1_SURFACE = "labs" as const;
export const PLS_SAVED_RUN_COMPARISON_V1_KIND = "descriptive_saved_run_projection" as const;

const PREDICTION_TABLE_ID = "plspredict_indicator_summary";
const VALIDATION_PLAN_TABLE_ID = "plspredict_validation_plan";
const CVPAT_TABLE_ID = "cvpat_benchmark_assessment";
const INFORMATION_CRITERIA_TABLE_ID = "pls_prediction_information_criteria";
const PREDICTION_CELL_ID = "qpls3.prediction.plspredict_cvpat";
const COMPARISON_CELL_ID = "qpls3.comparison.pls_models";
const SELECTION_CELL_ID = "qpls3.selection.prediction_oriented";

export type PlsSavedRunComparisonIssueCodeV1 =
  | "first_result_invalid"
  | "second_result_invalid"
  | "same_run_selected"
  | "same_model_selected"
  | "dataset_mismatch"
  | "method_mismatch"
  | "settings_mismatch"
  | "prediction_result_missing"
  | "prediction_contract_invalid"
  | "prediction_outcome_mismatch"
  | "prediction_estimand_mismatch"
  | "cross_validation_mismatch"
  | "cvpat_result_missing"
  | "cvpat_contract_mismatch"
  | "cvpat_between_model_test_unavailable"
  | "information_criteria_missing"
  | "information_criteria_invalid"
  | "information_criteria_mismatch"
  | "akaike_weights_missing"
  | "akaike_weights_invalid"
  | "additional_result_families_ignored"
  | "no_comparable_metrics";

export interface PlsSavedRunComparisonIssueV1 {
  id: string;
  code: PlsSavedRunComparisonIssueCodeV1;
  severity: "blocking" | "information";
  title: string;
  message: string;
  related_ids: string[];
  technical_details: string[];
}

export interface PlsSavedRunMetricValueV1 {
  value: number | null;
  missing_reason: CanonicalMissingCell["reason"] | null;
}

export interface PlsSavedRunPredictionMetricV1 {
  id: string;
  label: string;
  preference: "higher" | "lower" | "descriptive";
  first: PlsSavedRunMetricValueV1;
  second: PlsSavedRunMetricValueV1;
  /** Second selected run minus first selected run. Null when either value is unavailable. */
  change: number | null;
}

export interface PlsSavedRunPredictionRowV1 {
  id: string;
  construct: string;
  indicator: string;
  first_predictor_count: number;
  second_predictor_count: number;
  metrics: PlsSavedRunPredictionMetricV1[];
}

export interface PlsSavedRunCvpatRowV1 {
  id: string;
  benchmark: string;
  target_set: string;
  loss: string;
  alternative: string;
  confidence: string;
  first: PlsSavedRunCvpatSnapshotV1;
  second: PlsSavedRunCvpatSnapshotV1;
}

export interface PlsSavedRunCvpatSnapshotV1 {
  pls_mean_loss: PlsSavedRunMetricValueV1;
  benchmark_mean_loss: PlsSavedRunMetricValueV1;
  mean_loss_difference: PlsSavedRunMetricValueV1;
  standard_error: PlsSavedRunMetricValueV1;
  t_statistic: PlsSavedRunMetricValueV1;
  p_value_one_sided: PlsSavedRunMetricValueV1;
  confidence_interval_lower: PlsSavedRunMetricValueV1;
  confidence_interval_upper: PlsSavedRunMetricValueV1;
  observations: number;
  indicators: number;
  status: string;
  conclusion: string;
  reason: string;
}

export interface PlsSavedRunBicRowV1 {
  id: string;
  outcome: string;
  definition: "prediction_oriented_bic_v1";
  observations: number;
  first_parameter_count: number;
  second_parameter_count: number;
  first_bic: number;
  second_bic: number;
  /** Second selected run minus first selected run. Lower BIC is preferred. */
  bic_change: number;
  /** Exact stored weights only. BIC alone is never relabeled as an Akaike weight. */
  first_akaike_weight: number | null;
  second_akaike_weight: number | null;
  akaike_weight_source: "stored_exact" | "unavailable";
  preferred: "first" | "second" | "tie";
}

export interface PlsSavedRunComparisonDocumentV1 {
  schema_version: typeof PLS_SAVED_RUN_COMPARISON_V1_SCHEMA_VERSION;
  kind: typeof PLS_SAVED_RUN_COMPARISON_V1_KIND;
  surface: typeof PLS_SAVED_RUN_COMPARISON_V1_SURFACE;
  comparison_id: string;
  source_documents: {
    first_document_id: string;
    second_document_id: string;
  };
  compatibility: {
    dataset_fingerprint: string;
    method_version: string;
    analytical_settings_digest: string;
    first_model_digest: string;
    second_model_digest: string;
    cross_validation_plan: Record<string, string | number | boolean> | null;
  };
  prediction_rows: PlsSavedRunPredictionRowV1[];
  cvpat_rows: PlsSavedRunCvpatRowV1[];
  bic_rows: PlsSavedRunBicRowV1[];
  issues: PlsSavedRunComparisonIssueV1[];
}

export type PlsSavedRunComparisonBuildV1 =
  | { status: "ready"; comparison: PlsSavedRunComparisonDocumentV1 }
  | { status: "blocked"; issues: PlsSavedRunComparisonIssueV1[] };

interface ExtractedPredictionRow {
  id: string;
  construct: string;
  indicator: string;
  predictorCount: number;
  predictorSet: string;
  observations: number;
  mapeObservations: number;
  values: Record<string, PlsSavedRunMetricValueV1>;
}

interface ExtractedPrediction {
  plan: Record<string, string | number | boolean>;
  rows: ExtractedPredictionRow[];
  contractIdentity: string;
  metricLabels: Record<string, string>;
}

interface ExtractedCvpatRow {
  id: string;
  benchmark: string;
  targetSet: string;
  loss: string;
  alternative: string;
  confidence: string;
  snapshot: PlsSavedRunCvpatSnapshotV1;
}

interface ExtractedBicRow {
  id: string;
  outcome: string;
  definition: "prediction_oriented_bic_v1";
  value: number;
  observations: number;
  parameterCount: number;
  akaikeWeight: number | null;
  akaikeWeightDefinition: string | null;
  candidateSetDigest: string | null;
  candidateCount: number | null;
}

type Extraction<T> =
  | { status: "ready"; value: T }
  | { status: "absent" }
  | { status: "invalid"; details: string[] };

interface MetricContract {
  id: string;
  columnId: string;
  fallbackLabel: string;
  preference: PlsSavedRunPredictionMetricV1["preference"];
}

const PREDICTION_METRICS: readonly MetricContract[] = [
  { id: "q_squared_predict", columnId: "q2_predict", fallbackLabel: "Q²_predict", preference: "higher" },
  { id: "pls_rmse", columnId: "pls-sem_rmse", fallbackLabel: "PLS-SEM RMSE", preference: "lower" },
  { id: "ia_rmse", columnId: "ia_rmse", fallbackLabel: "IA RMSE", preference: "lower" },
  { id: "lm_rmse", columnId: "lm_rmse", fallbackLabel: "LM RMSE", preference: "lower" },
  { id: "pls_mae", columnId: "pls-sem_mae", fallbackLabel: "PLS-SEM MAE", preference: "lower" },
  { id: "ia_mae", columnId: "ia_mae", fallbackLabel: "IA MAE", preference: "lower" },
  { id: "lm_mae", columnId: "lm_mae", fallbackLabel: "LM MAE", preference: "lower" },
  { id: "pls_mape", columnId: "pls-sem_mape", fallbackLabel: "PLS-SEM MAPE (%)", preference: "lower" },
  { id: "ia_mape", columnId: "ia_mape", fallbackLabel: "IA MAPE (%)", preference: "lower" },
  { id: "lm_mape", columnId: "lm_mape", fallbackLabel: "LM MAPE (%)", preference: "lower" },
] as const;

function issue(
  code: PlsSavedRunComparisonIssueCodeV1,
  severity: PlsSavedRunComparisonIssueV1["severity"],
  title: string,
  message: string,
  relatedIds: readonly string[] = [],
  technicalDetails: readonly string[] = [],
): PlsSavedRunComparisonIssueV1 {
  return {
    id: code,
    code,
    severity,
    title,
    message,
    related_ids: [...new Set(relatedIds)].sort(),
    technical_details: [...technicalDetails],
  };
}

function stableValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(stableValue);
  if (value && typeof value === "object") {
    const record = value as Record<string, unknown>;
    return Object.fromEntries(Object.keys(record).sort().map((key) => [key, stableValue(record[key])]));
  }
  return value;
}

function stableJson(value: unknown): string {
  return JSON.stringify(stableValue(value));
}

function compareText(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

function stableId(value: string): string {
  const normalized = value
    .normalize("NFKD")
    .toLowerCase()
    .replace(/[^a-z0-9_.:-]+/g, "_")
    .replace(/^[_:.-]+|[_:.-]+$/g, "")
    .replace(/_+/g, "_");
  return normalized || "unnamed";
}

function findTable(document: CanonicalResultDocumentV2, id: string): CanonicalResultTable | null {
  return document.tables.find((table) => table.id === id) ?? null;
}

function hasAttributedCapability(
  table: CanonicalResultTable,
  capabilityId: string,
  cellId: string,
  capabilityVersion: string,
): boolean {
  return Boolean(table.capability_cells?.some((reference) => (
    reference.registry_schema_version === 2
    && reference.capability_id === capabilityId
    && reference.cell_id === cellId
    && reference.capability_version === capabilityVersion
  )));
}

function hasInformationCriterionAttribution(table: CanonicalResultTable): boolean {
  return hasAttributedCapability(
    table,
    "smartpls.pls_model_comparison",
    COMPARISON_CELL_ID,
    "pls_model_comparison_v1",
  ) || hasAttributedCapability(
    table,
    "smartpls.prediction_oriented_model_selection",
    SELECTION_CELL_ID,
    "prediction_oriented_model_selection_v1",
  );
}

function columnIndex(table: CanonicalResultTable, id: string, dataType: "number" | "text" | "boolean"): number {
  return table.columns.findIndex((column) => column.id === id && column.data_type === dataType);
}

function cellAt(table: CanonicalResultTable, row: CanonicalResultRow, id: string, dataType: "number" | "text" | "boolean"): CanonicalResultCell | null {
  const index = columnIndex(table, id, dataType);
  return index < 0 ? null : row.cells[index] ?? null;
}

function textCell(table: CanonicalResultTable, row: CanonicalResultRow, id: string): string | null {
  const cell = cellAt(table, row, id, "text");
  return cell?.kind === "text" && cell.value.trim() ? cell.value : null;
}

function optionalTextCell(table: CanonicalResultTable, row: CanonicalResultRow, id: string): string | null {
  const index = table.columns.findIndex((column) => column.id === id);
  const cell = index < 0 ? null : row.cells[index] ?? null;
  return cell?.kind === "text" ? cell.value : cell?.kind === "missing" ? null : null;
}

function numberCell(table: CanonicalResultTable, row: CanonicalResultRow, id: string): number | null {
  const cell = cellAt(table, row, id, "number");
  return cell?.kind === "number" && Number.isFinite(cell.value) ? cell.value : null;
}

function metricCell(table: CanonicalResultTable, row: CanonicalResultRow, id: string): PlsSavedRunMetricValueV1 | null {
  // The current generic canonical bridge types an all-missing numeric column
  // as text. The cell itself still carries an explicit missing reason, so use
  // the exact label and typed cell rather than pretending a value is present.
  const index = table.columns.findIndex((column) => column.id === id);
  const cell = index < 0 ? null : row.cells[index] ?? null;
  if (cell?.kind === "number" && Number.isFinite(cell.value)) return { value: cell.value, missing_reason: null };
  if (cell?.kind === "missing") return { value: null, missing_reason: cell.reason };
  return null;
}

function primitiveCell(table: CanonicalResultTable, row: CanonicalResultRow, id: string): string | number | boolean | null {
  const index = table.columns.findIndex((column) => column.id === id);
  const cell = index < 0 ? null : row.cells[index] ?? null;
  if (!cell || cell.kind === "missing") return null;
  return cell.value;
}

interface ColumnContract {
  id: string;
  dataType: "number" | "text" | "boolean";
  allowAllMissingNumeric?: boolean;
}

function exactColumnContract(table: CanonicalResultTable, contract: readonly ColumnContract[]): string | null {
  const selected = contract.map((expected) => {
    const index = table.columns.findIndex((candidate) => candidate.id === expected.id);
    if (index < 0) return null;
    const column = table.columns[index];
    if (column.data_type === expected.dataType) return `${column.id}:${expected.dataType}`;
    if (
      expected.allowAllMissingNumeric
      && expected.dataType === "number"
      && table.rows.every((row) => row.cells[index]?.kind === "missing")
    ) return `${column.id}:number_or_all_missing`;
    return null;
  });
  return selected.some((value) => value === null) ? null : selected.join("|");
}

function positiveInteger(value: number | null): value is number {
  return value !== null && Number.isSafeInteger(value) && value > 0;
}

function nonnegativeInteger(value: number | null): value is number {
  return value !== null && Number.isSafeInteger(value) && value >= 0;
}

function extractPrediction(document: CanonicalResultDocumentV2): Extraction<ExtractedPrediction> {
  const table = findTable(document, PREDICTION_TABLE_ID);
  const planTable = findTable(document, VALIDATION_PLAN_TABLE_ID);
  if (!table && !planTable) return { status: "absent" };
  const details: string[] = [];
  if (!table) details.push(`Missing ${PREDICTION_TABLE_ID}.`);
  if (!planTable) details.push(`Missing ${VALIDATION_PLAN_TABLE_ID}.`);
  if (!table || !planTable) return { status: "invalid", details };
  if (
    !hasAttributedCapability(table, "smartpls.plspredict", PREDICTION_CELL_ID, "plspredict_indicator_v2")
    || !hasAttributedCapability(planTable, "smartpls.plspredict", PREDICTION_CELL_ID, "plspredict_indicator_v2")
  ) {
    details.push("Prediction tables are not attributed to the exact PLSpredict/CVPAT option cell.");
  }
  const predictionColumns: ColumnContract[] = [
    { id: "construct", dataType: "text" },
    { id: "indicator", dataType: "text" },
    { id: "predictor_set", dataType: "text" },
    { id: "predictors", dataType: "number" },
    { id: "observations", dataType: "number" },
    ...PREDICTION_METRICS.map((metric) => ({ id: metric.columnId, dataType: "number" as const, allowAllMissingNumeric: true })),
    { id: "mape_observations", dataType: "number" },
    { id: "lm_benchmark", dataType: "text" },
  ];
  const predictionContract = exactColumnContract(table, predictionColumns);
  if (!predictionContract) details.push("Indicator prediction columns do not match the current typed contract.");
  const planColumns: ColumnContract[] = [
    { id: "procedure", dataType: "text" },
    { id: "complete_cases", dataType: "number" },
    { id: "folds", dataType: "number" },
    { id: "repeats", dataType: "number" },
    { id: "assignment", dataType: "text" },
    { id: "assignment_digest", dataType: "text" },
    { id: "seed", dataType: "number" },
    { id: "test_predictions", dataType: "number" },
  ];
  const planContract = exactColumnContract(planTable, planColumns);
  if (!planContract || planTable.rows.length !== 1) details.push("Cross-validation design must contain exactly one current typed plan row.");
  if (details.length > 0) return { status: "invalid", details };

  const planRow = planTable.rows[0];
  const planEntries = planColumns.map((column) => [column.id, primitiveCell(planTable, planRow, column.id)] as const);
  if (planEntries.some(([, value]) => value === null)) {
    return { status: "invalid", details: ["Cross-validation design contains a missing or mistyped value."] };
  }
  const plan = Object.fromEntries(planEntries) as Record<string, string | number | boolean>;
  if (
    typeof plan.procedure !== "string"
    || typeof plan.assignment !== "string"
    || typeof plan.assignment_digest !== "string"
    || !/^sha256:[0-9a-f]{64}$/.test(plan.assignment_digest)
    || !positiveInteger(typeof plan.complete_cases === "number" ? plan.complete_cases : null)
    || !positiveInteger(typeof plan.folds === "number" ? plan.folds : null)
    || !positiveInteger(typeof plan.repeats === "number" ? plan.repeats : null)
    || !nonnegativeInteger(typeof plan.seed === "number" ? plan.seed : null)
    || !positiveInteger(typeof plan.test_predictions === "number" ? plan.test_predictions : null)
  ) return { status: "invalid", details: ["Cross-validation design contains an invalid count, seed, or assignment digest."] };
  const rows: ExtractedPredictionRow[] = [];
  const seen = new Set<string>();
  for (const row of table.rows) {
    const construct = textCell(table, row, "construct");
    const indicator = textCell(table, row, "indicator");
    const predictorCount = numberCell(table, row, "predictors");
    const observations = numberCell(table, row, "observations");
    const predictorSet = textCell(table, row, "predictor_set");
    const mapeObservations = numberCell(table, row, "mape_observations");
    const lmBenchmark = textCell(table, row, "lm_benchmark");
    if (!construct || !indicator || !nonnegativeInteger(predictorCount) || !positiveInteger(observations) || !predictorSet || !nonnegativeInteger(mapeObservations) || !lmBenchmark) {
      details.push(`Prediction row ${row.id} is missing its typed outcome or design fields.`);
      continue;
    }
    const id = `${stableId(construct)}:${stableId(indicator)}`;
    if (seen.has(id)) {
      details.push(`Prediction outcome ${construct} / ${indicator} is duplicated.`);
      continue;
    }
    seen.add(id);
    const values: Record<string, PlsSavedRunMetricValueV1> = {};
    for (const metric of PREDICTION_METRICS) {
      const value = metricCell(table, row, metric.columnId);
      if (!value) details.push(`Prediction row ${row.id} has an invalid ${metric.columnId} cell.`);
      else values[metric.id] = value;
    }
    values.observations = { value: observations, missing_reason: null };
    values.mape_observations = { value: mapeObservations, missing_reason: null };
    rows.push({ id, construct, indicator, predictorCount, predictorSet, observations, mapeObservations, values });
  }
  if (rows.length === 0) details.push("Indicator prediction table contains no typed outcomes.");
  if (details.length > 0) return { status: "invalid", details };
  rows.sort((left, right) => compareText(left.id, right.id));
  return {
    status: "ready",
    value: {
      plan,
      rows,
      contractIdentity: `${predictionContract}\u0000${planContract}`,
      metricLabels: Object.fromEntries(PREDICTION_METRICS.map((metric) => [
        metric.id,
        table.columns.find((column) => column.id === metric.columnId)?.label ?? metric.fallbackLabel,
      ])),
    },
  };
}

function extractCvpat(document: CanonicalResultDocumentV2): Extraction<ExtractedCvpatRow[]> {
  const table = findTable(document, CVPAT_TABLE_ID);
  if (!table) return { status: "absent" };
  const details: string[] = [];
  if (!hasAttributedCapability(table, "smartpls.cvpat", PREDICTION_CELL_ID, "plspredict_indicator_v2")) {
    details.push("CVPAT table is not attributed to the exact PLSpredict/CVPAT option cell.");
  }
  const columns: ColumnContract[] = [
    { id: "benchmark", dataType: "text" },
    { id: "target_set", dataType: "text" },
    { id: "loss", dataType: "text" },
    { id: "alternative", dataType: "text" },
    { id: "confidence", dataType: "text" },
    { id: "pls-sem_mean_loss", dataType: "number", allowAllMissingNumeric: true },
    { id: "benchmark_mean_loss", dataType: "number", allowAllMissingNumeric: true },
    { id: "mean_loss_difference_pls-sem_benchmark", dataType: "number", allowAllMissingNumeric: true },
    { id: "se", dataType: "number", allowAllMissingNumeric: true },
    { id: "t", dataType: "number", allowAllMissingNumeric: true },
    { id: "p_one-sided", dataType: "number", allowAllMissingNumeric: true },
    { id: "95_ci_lower", dataType: "number", allowAllMissingNumeric: true },
    { id: "95_ci_upper", dataType: "number", allowAllMissingNumeric: true },
    { id: "complete_cases", dataType: "number" },
    { id: "indicators", dataType: "number" },
    { id: "status", dataType: "text" },
    { id: "supported_conclusion", dataType: "text" },
    { id: "reason", dataType: "text" },
  ];
  if (!exactColumnContract(table, columns)) details.push("CVPAT columns do not match the current typed benchmark-assessment contract.");
  const rows: ExtractedCvpatRow[] = [];
  const seen = new Set<string>();
  for (const row of table.rows) {
    const benchmark = textCell(table, row, "benchmark");
    const targetSet = textCell(table, row, "target_set");
    const loss = textCell(table, row, "loss");
    const alternative = textCell(table, row, "alternative");
    const confidence = textCell(table, row, "confidence");
    const observations = numberCell(table, row, "complete_cases");
    const indicators = numberCell(table, row, "indicators");
    const status = textCell(table, row, "status");
    const conclusion = textCell(table, row, "supported_conclusion");
    const reason = optionalTextCell(table, row, "reason") ?? "";
    const metric = (id: string) => metricCell(table, row, id);
    const metrics = {
      pls_mean_loss: metric("pls-sem_mean_loss"),
      benchmark_mean_loss: metric("benchmark_mean_loss"),
      mean_loss_difference: metric("mean_loss_difference_pls-sem_benchmark"),
      standard_error: metric("se"),
      t_statistic: metric("t"),
      p_value_one_sided: metric("p_one-sided"),
      confidence_interval_lower: metric("95_ci_lower"),
      confidence_interval_upper: metric("95_ci_upper"),
    };
    if (!benchmark || !targetSet || !loss || !alternative || !confidence || !positiveInteger(observations) || !positiveInteger(indicators) || !status || !conclusion || Object.values(metrics).some((value) => value === null)) {
      details.push(`CVPAT row ${row.id} contains missing or mistyped contract fields.`);
      continue;
    }
    const id = stableId(benchmark);
    if (seen.has(id)) {
      details.push(`CVPAT benchmark ${benchmark} is duplicated.`);
      continue;
    }
    seen.add(id);
    rows.push({
      id,
      benchmark,
      targetSet,
      loss,
      alternative,
      confidence,
      snapshot: {
        ...(metrics as Record<keyof typeof metrics, PlsSavedRunMetricValueV1>),
        observations,
        indicators,
        status,
        conclusion,
        reason,
      },
    });
  }
  if (rows.length === 0) details.push("CVPAT table contains no typed benchmark rows.");
  return details.length > 0
    ? { status: "invalid", details }
    : { status: "ready", value: rows.sort((left, right) => compareText(left.id, right.id)) };
}

function extractBic(document: CanonicalResultDocumentV2): Extraction<ExtractedBicRow[]> {
  const table = findTable(document, INFORMATION_CRITERIA_TABLE_ID);
  if (!table) return { status: "absent" };
  const details: string[] = [];
  if (!hasInformationCriterionAttribution(table)) {
    details.push("Information criteria are not attributed to an exact model-comparison or prediction-selection option cell.");
  }
  const columns: ColumnContract[] = [
    { id: "outcome", dataType: "text" },
    { id: "bic", dataType: "number" },
    { id: "bic_definition", dataType: "text" },
    { id: "observations", dataType: "number" },
    { id: "parameter_count", dataType: "number" },
    { id: "akaike_weight", dataType: "number", allowAllMissingNumeric: true },
    { id: "akaike_weight_definition", dataType: "text" },
    { id: "candidate_set_digest", dataType: "text" },
    { id: "candidate_count", dataType: "number", allowAllMissingNumeric: true },
  ];
  if (!exactColumnContract(table, columns)) details.push("Information-criterion columns do not match the exact typed contract.");
  const rows: ExtractedBicRow[] = [];
  const seen = new Set<string>();
  for (const row of table.rows) {
    const outcome = textCell(table, row, "outcome");
    const definition = textCell(table, row, "bic_definition");
    const value = numberCell(table, row, "bic");
    const observations = numberCell(table, row, "observations");
    const parameterCount = numberCell(table, row, "parameter_count");
    const weight = metricCell(table, row, "akaike_weight");
    const akaikeWeightDefinition = optionalTextCell(table, row, "akaike_weight_definition");
    const candidateSetDigest = optionalTextCell(table, row, "candidate_set_digest");
    const candidateCountCell = metricCell(table, row, "candidate_count");
    const candidateCount = candidateCountCell?.value ?? null;
    if (!outcome || definition !== "prediction_oriented_bic_v1" || value === null || !positiveInteger(observations) || !nonnegativeInteger(parameterCount) || !weight || !candidateCountCell) {
      details.push(`Information-criterion row ${row.id} is not an exact prediction-oriented BIC row.`);
      continue;
    }
    const allWeightFieldsMissing = weight.value === null
      && akaikeWeightDefinition === null
      && candidateSetDigest === null
      && candidateCount === null;
    const completeWeightFields = weight.value !== null
      && weight.value >= 0
      && weight.value <= 1
      && akaikeWeightDefinition === "akaike_weight_v1"
      && /^sha256:[0-9a-f]{64}$/.test(candidateSetDigest ?? "")
      && candidateCount === 2;
    if (!allWeightFieldsMissing && !completeWeightFields) {
      details.push(`Information-criterion row ${row.id} has a partial or invalid stored Akaike-weight contract.`);
      continue;
    }
    const id = stableId(outcome);
    if (seen.has(id)) {
      details.push(`BIC outcome ${outcome} is duplicated.`);
      continue;
    }
    seen.add(id);
    rows.push({
      id,
      outcome,
      definition,
      value,
      observations,
      parameterCount,
      akaikeWeight: weight.value,
      akaikeWeightDefinition,
      candidateSetDigest,
      candidateCount,
    });
  }
  if (rows.length === 0) details.push("Information-criterion table contains no exact prediction-oriented BIC rows.");
  return details.length > 0
    ? { status: "invalid", details }
    : { status: "ready", value: rows.sort((left, right) => compareText(left.id, right.id)) };
}

function ids(values: ReadonlyArray<{ id: string }>): string[] {
  return values.map((value) => value.id);
}

function sameValues(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function sameMetric(left: PlsSavedRunMetricValueV1, right: PlsSavedRunMetricValueV1): boolean {
  return left.value === right.value && left.missing_reason === right.missing_reason;
}

function predictionEstimandsMatch(first: ExtractedPrediction, second: ExtractedPrediction): boolean {
  const secondById = new Map(second.rows.map((row) => [row.id, row]));
  return first.rows.every((row) => {
    const other = secondById.get(row.id);
    return Boolean(other)
      && row.predictorSet === other!.predictorSet
      && row.observations === other!.observations
      && row.mapeObservations === other!.mapeObservations
      && ["ia_rmse", "ia_mae", "ia_mape"].every((metricId) => sameMetric(row.values[metricId], other!.values[metricId]));
  });
}

function change(first: PlsSavedRunMetricValueV1, second: PlsSavedRunMetricValueV1): number | null {
  if (first.value === null || second.value === null) return null;
  const result = second.value - first.value;
  return Object.is(result, -0) ? 0 : result;
}

function comparisonPredictionRows(first: ExtractedPrediction, second: ExtractedPrediction): PlsSavedRunPredictionRowV1[] {
  const secondById = new Map(second.rows.map((row) => [row.id, row]));
  return first.rows.map((firstRow) => {
    const secondRow = secondById.get(firstRow.id)!;
    return {
      id: firstRow.id,
      construct: firstRow.construct,
      indicator: firstRow.indicator,
      first_predictor_count: firstRow.predictorCount,
      second_predictor_count: secondRow.predictorCount,
      metrics: PREDICTION_METRICS.map((metric) => ({
        id: metric.id,
        label: first.metricLabels[metric.id] ?? metric.fallbackLabel,
        preference: metric.preference,
        first: firstRow.values[metric.id],
        second: secondRow.values[metric.id],
        change: change(firstRow.values[metric.id], secondRow.values[metric.id]),
      })),
    };
  });
}

function cvpatContract(row: ExtractedCvpatRow): string {
  return stableJson({
    id: row.id,
    targetSet: row.targetSet,
    loss: row.loss,
    alternative: row.alternative,
    confidence: row.confidence,
    observations: row.snapshot.observations,
    indicators: row.snapshot.indicators,
  });
}

function comparisonCvpatRows(first: ExtractedCvpatRow[], second: ExtractedCvpatRow[]): PlsSavedRunCvpatRowV1[] {
  const secondById = new Map(second.map((row) => [row.id, row]));
  return first.map((firstRow) => {
    const secondRow = secondById.get(firstRow.id)!;
    return {
      id: firstRow.id,
      benchmark: firstRow.benchmark,
      target_set: firstRow.targetSet,
      loss: firstRow.loss,
      alternative: firstRow.alternative,
      confidence: firstRow.confidence,
      first: firstRow.snapshot,
      second: secondRow.snapshot,
    };
  });
}

function comparisonBicRows(
  first: ExtractedBicRow[],
  second: ExtractedBicRow[],
  includeStoredWeights: boolean,
): PlsSavedRunBicRowV1[] {
  const secondById = new Map(second.map((row) => [row.id, row]));
  return first.map((firstRow) => {
    const secondRow = secondById.get(firstRow.id)!;
    return {
      id: firstRow.id,
      outcome: firstRow.outcome,
      definition: "prediction_oriented_bic_v1",
      observations: firstRow.observations,
      first_parameter_count: firstRow.parameterCount,
      second_parameter_count: secondRow.parameterCount,
      first_bic: firstRow.value,
      second_bic: secondRow.value,
      bic_change: secondRow.value - firstRow.value,
      first_akaike_weight: includeStoredWeights ? firstRow.akaikeWeight : null,
      second_akaike_weight: includeStoredWeights ? secondRow.akaikeWeight : null,
      akaike_weight_source: includeStoredWeights ? "stored_exact" : "unavailable",
      preferred: firstRow.value === secondRow.value ? "tie" : firstRow.value < secondRow.value ? "first" : "second",
    };
  });
}

function exactCapabilitySet(document: CanonicalResultDocumentV2): string[] {
  return (document.capability_cells ?? []).map(capabilityCellReferenceIdentityV2).sort();
}

/**
 * Compare two canonical saved PLS results within the bounded descriptive Labs
 * contract. Model digests must differ; current canonical recipe digests may be
 * equal because they bind method/version/analytical settings, not model shape.
 */
export function buildPlsSavedRunComparisonV1(
  first: CanonicalResultDocumentV2,
  second: CanonicalResultDocumentV2,
): PlsSavedRunComparisonBuildV1 {
  const blocking: PlsSavedRunComparisonIssueV1[] = [];
  const informational: PlsSavedRunComparisonIssueV1[] = [];
  const firstValidation = validateCanonicalResultDocumentV2(first);
  const secondValidation = validateCanonicalResultDocumentV2(second);
  if (!firstValidation.passed) blocking.push(issue(
    "first_result_invalid",
    "blocking",
    "First result is invalid",
    "Reopen or recalculate the first saved run before comparing it.",
    [first.document_id],
    firstValidation.errors,
  ));
  if (!secondValidation.passed) blocking.push(issue(
    "second_result_invalid",
    "blocking",
    "Second result is invalid",
    "Reopen or recalculate the second saved run before comparing it.",
    [second.document_id],
    secondValidation.errors,
  ));
  if (blocking.length > 0) return { status: "blocked", issues: blocking };
  if (first.document_id === second.document_id || first.provenance.run_id === second.provenance.run_id) {
    blocking.push(issue(
      "same_run_selected",
      "blocking",
      "Choose two different runs",
      "Select two separately saved runs before opening the comparison.",
      [first.document_id, second.document_id],
    ));
  }
  if (first.provenance.model_digest === second.provenance.model_digest) {
    blocking.push(issue(
      "same_model_selected",
      "blocking",
      "Choose two distinct models",
      "These runs bind the same scientific model. Select runs from two alternative model specifications.",
      [first.provenance.model_id, second.provenance.model_id],
    ));
  }
  if (first.provenance.dataset_fingerprint !== second.provenance.dataset_fingerprint) {
    blocking.push(issue(
      "dataset_mismatch",
      "blocking",
      "Data differs",
      "Choose runs calculated from the same immutable dataset.",
      [first.provenance.dataset_id, second.provenance.dataset_id],
    ));
  }
  if (first.provenance.method_version !== second.provenance.method_version) {
    blocking.push(issue(
      "method_mismatch",
      "blocking",
      "PLS method versions differ",
      "Recalculate both models with the same PLS method version.",
      [first.provenance.method_version, second.provenance.method_version],
    ));
  }
  if (first.provenance.recipe_digest !== second.provenance.recipe_digest) {
    blocking.push(issue(
      "settings_mismatch",
      "blocking",
      "Analysis settings differ",
      "Recalculate both models with the same analytical settings. Execution worker count does not affect this check.",
      [first.provenance.recipe_digest, second.provenance.recipe_digest],
    ));
  }
  if (blocking.length > 0) return { status: "blocked", issues: blocking };

  const firstPrediction = extractPrediction(first);
  const secondPrediction = extractPrediction(second);
  let predictionRows: PlsSavedRunPredictionRowV1[] = [];
  let crossValidationPlan: Record<string, string | number | boolean> | null = null;
  if (firstPrediction.status === "ready" && secondPrediction.status === "ready") {
    if (firstPrediction.value.contractIdentity !== secondPrediction.value.contractIdentity) {
      blocking.push(issue(
        "prediction_contract_invalid",
        "blocking",
        "Prediction result definitions differ",
        "Recalculate both runs with the current PLSpredict result contract.",
      ));
    }
    if (!sameValues(ids(firstPrediction.value.rows), ids(secondPrediction.value.rows))) {
      blocking.push(issue(
        "prediction_outcome_mismatch",
        "blocking",
        "Prediction outcomes differ",
        "Choose two runs with the same endogenous indicator outcomes.",
        [...ids(firstPrediction.value.rows), ...ids(secondPrediction.value.rows)],
      ));
    }
    if (
      sameValues(ids(firstPrediction.value.rows), ids(secondPrediction.value.rows))
      && !predictionEstimandsMatch(firstPrediction.value, secondPrediction.value)
    ) {
      blocking.push(issue(
        "prediction_estimand_mismatch",
        "blocking",
        "Prediction definitions differ",
        "Recalculate both runs with the same predictor definition, evaluation cases, and indicator-average benchmark.",
      ));
    }
    if (stableJson(firstPrediction.value.plan) !== stableJson(secondPrediction.value.plan)) {
      blocking.push(issue(
        "cross_validation_mismatch",
        "blocking",
        "Cross-validation designs differ",
        "Recalculate both models with the same cases, folds, repetitions, assignment digest, and seed.",
        [],
        [stableJson(firstPrediction.value.plan), stableJson(secondPrediction.value.plan)],
      ));
    }
    if (blocking.length === 0) {
      predictionRows = comparisonPredictionRows(firstPrediction.value, secondPrediction.value);
      crossValidationPlan = firstPrediction.value.plan;
    }
  } else if (firstPrediction.status === "invalid" || secondPrediction.status === "invalid") {
    blocking.push(issue(
      "prediction_contract_invalid",
      "blocking",
      "Prediction results are incomplete",
      "Recalculate the affected run with the current PLSpredict workflow.",
      [],
      [
        ...(firstPrediction.status === "invalid" ? firstPrediction.details : []),
        ...(secondPrediction.status === "invalid" ? secondPrediction.details : []),
      ],
    ));
  } else {
    informational.push(issue(
      "prediction_result_missing",
      "information",
      "PLSpredict comparison is unavailable",
      "Run the same PLSpredict design for both models to compare stored prediction metrics.",
    ));
  }
  if (blocking.length > 0) return { status: "blocked", issues: blocking };

  const firstCvpat = extractCvpat(first);
  const secondCvpat = extractCvpat(second);
  let cvpatRows: PlsSavedRunCvpatRowV1[] = [];
  if (firstCvpat.status === "ready" && secondCvpat.status === "ready") {
    const sameIds = sameValues(ids(firstCvpat.value), ids(secondCvpat.value));
    const sameContracts = sameIds && firstCvpat.value.every((row, index) => cvpatContract(row) === cvpatContract(secondCvpat.value[index]));
    if (sameContracts) {
      cvpatRows = comparisonCvpatRows(firstCvpat.value, secondCvpat.value);
      informational.push(issue(
        "cvpat_between_model_test_unavailable",
        "information",
        "CVPAT rows are single-model benchmark assessments",
        "The side-by-side values compare each model with its IA or LM benchmark; they are not a paired CVPAT test between the two models.",
      ));
    } else {
      informational.push(issue(
        "cvpat_contract_mismatch",
        "information",
        "CVPAT benchmark results are not comparable",
        "Recalculate both runs with the same target set, loss, confidence level, cases, and indicators.",
      ));
    }
  } else if (firstCvpat.status !== "absent" || secondCvpat.status !== "absent") {
    informational.push(issue(
      "cvpat_result_missing",
      "information",
      "CVPAT side-by-side results are unavailable",
      "Recalculate both models with the current CVPAT benchmark-assessment workflow.",
      [],
      [
        ...(firstCvpat.status === "invalid" ? firstCvpat.details : []),
        ...(secondCvpat.status === "invalid" ? secondCvpat.details : []),
      ],
    ));
  }

  const firstBic = extractBic(first);
  const secondBic = extractBic(second);
  let bicRows: PlsSavedRunBicRowV1[] = [];
  if (firstBic.status === "ready" && secondBic.status === "ready") {
    const sameIds = sameValues(ids(firstBic.value), ids(secondBic.value));
    const observationsMatch = sameIds && firstBic.value.every((row, index) => row.observations === secondBic.value[index].observations);
    if (sameIds && observationsMatch) {
      const weightsAbsent = firstBic.value.every((row, index) => (
        row.akaikeWeight === null && secondBic.value[index].akaikeWeight === null
      ));
      const storedWeightsComparable = firstBic.value.every((row, index) => {
        const other = secondBic.value[index];
        return row.akaikeWeight !== null
          && other.akaikeWeight !== null
          && row.akaikeWeightDefinition === "akaike_weight_v1"
          && other.akaikeWeightDefinition === "akaike_weight_v1"
          && row.candidateSetDigest === other.candidateSetDigest
          && row.candidateCount === 2
          && other.candidateCount === 2
          && Math.abs(row.akaikeWeight + other.akaikeWeight - 1) <= 1e-10;
      });
      bicRows = comparisonBicRows(firstBic.value, secondBic.value, storedWeightsComparable);
      if (weightsAbsent) {
        informational.push(issue(
          "akaike_weights_missing",
          "information",
          "Akaike weights are unavailable",
          "BIC values are shown, but BIC-only data are not relabeled as Akaike weights.",
        ));
      } else if (!storedWeightsComparable) {
        informational.push(issue(
          "akaike_weights_invalid",
          "information",
          "Stored Akaike weights are not comparable",
          "Use exact stored weights from the same two-model candidate set.",
        ));
      }
    } else {
      informational.push(issue(
        "information_criteria_mismatch",
        "information",
        "BIC results are not comparable",
        "Use prediction-oriented BIC values for the same outcomes and observations.",
        [...ids(firstBic.value), ...ids(secondBic.value)],
      ));
    }
  } else if (firstBic.status === "invalid" || secondBic.status === "invalid") {
    informational.push(issue(
      "information_criteria_invalid",
      "information",
      "BIC and Akaike weights are unavailable",
      "Recalculate exact prediction-oriented BIC results; generic fit statistics are not substituted.",
      [],
      [
        ...(firstBic.status === "invalid" ? firstBic.details : []),
        ...(secondBic.status === "invalid" ? secondBic.details : []),
      ],
    ));
  } else {
    informational.push(issue(
      "information_criteria_missing",
      "information",
      "BIC and Akaike weights are unavailable",
      "The saved canonical results do not contain exact prediction-oriented BIC values for both models.",
    ));
  }

  if (predictionRows.length === 0 && cvpatRows.length === 0 && bicRows.length === 0) {
    return {
      status: "blocked",
      issues: [issue(
        "no_comparable_metrics",
        "blocking",
        "No comparable model-selection results are stored",
        "Run compatible PLSpredict/CVPAT analyses or exact prediction-oriented BIC for both models.",
      ), ...informational],
    };
  }

  const firstCapabilities = exactCapabilitySet(first);
  const secondCapabilities = exactCapabilitySet(second);
  if (!sameValues(firstCapabilities, secondCapabilities)) {
    informational.push(issue(
      "additional_result_families_ignored",
      "information",
      "Saved runs contain different additional result families",
      "Only the explicitly aligned PLSpredict, CVPAT, and BIC rows are included in this view.",
      [...firstCapabilities, ...secondCapabilities],
    ));
  }
  return {
    status: "ready",
    comparison: {
      schema_version: PLS_SAVED_RUN_COMPARISON_V1_SCHEMA_VERSION,
      kind: PLS_SAVED_RUN_COMPARISON_V1_KIND,
      surface: PLS_SAVED_RUN_COMPARISON_V1_SURFACE,
      comparison_id: `pls_saved_run_comparison:${first.document_id}:to:${second.document_id}`,
      source_documents: {
        first_document_id: first.document_id,
        second_document_id: second.document_id,
      },
      compatibility: {
        dataset_fingerprint: first.provenance.dataset_fingerprint,
        method_version: first.provenance.method_version,
        analytical_settings_digest: first.provenance.recipe_digest,
        first_model_digest: first.provenance.model_digest,
        second_model_digest: second.provenance.model_digest,
        cross_validation_plan: crossValidationPlan,
      },
      prediction_rows: predictionRows,
      cvpat_rows: cvpatRows,
      bic_rows: bicRows,
      issues: informational,
    },
  };
}
